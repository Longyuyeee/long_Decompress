use super::ExtractionRuntime;
use crate::models::compression::{DecompressOptions, TaskLogSeverity};
use crate::services::compression_service::CompressionError;
use crate::services::extraction_transaction;
use crate::services::progress_gate::ProgressGate;
use crate::utils::io_utils::ProgressReader;
use anyhow::Result;
use chrono::{Local, TimeZone};
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::Path;
use std::sync::Arc;
use tauri::Window;
use zip_aes::result::ZipError;
use zip_aes::ZipArchive;

const PROGRESS_EMIT_INTERVAL_BYTES: u64 = 4 * 1024 * 1024;

pub(crate) fn system_time(
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
) -> Option<std::time::SystemTime> {
    let naive = chrono::NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32)?
        .and_hms_opt(hour as u32, minute as u32, second.min(59) as u32)?;
    Local
        .from_local_datetime(&naive)
        .single()
        .or_else(|| Local.from_local_datetime(&naive).earliest())
        .map(std::time::SystemTime::from)
}

fn validate_password_access<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    password: Option<&str>,
    indices: &[usize],
) -> Result<()> {
    for &index in indices {
        let requires_password = match archive.by_index(index) {
            Ok(_) => false,
            Err(ZipError::UnsupportedArchive(detail)) if detail == ZipError::PASSWORD_REQUIRED => {
                true
            }
            Err(error) => return Err(error.into()),
        };
        if !requires_password {
            continue;
        }

        match password {
            Some(password) => match crate::services::zip_password::open_entry(archive, index, password) {
                Ok(mut reader) => {
                    let mut probe = [0u8; 4];
                    let _read = reader.read(&mut probe)?;
                }
                Err(ZipError::InvalidPassword) => {
                    return Err(CompressionError::InvalidPassword.into())
                }
                Err(error) => return Err(error.into()),
            },
            None => return Err(CompressionError::PasswordRequired.into()),
        }
    }
    Ok(())
}

pub(crate) fn extract<R: ExtractionRuntime + Sync>(
    runtime: &R,
    window: &Window,
    task_id: &str,
    file: &str,
    output: &str,
    password: Option<&str>,
    options: &DecompressOptions,
) -> Result<()> {
    extract_with_optional_window(
        runtime,
        Some(window),
        task_id,
        file,
        output,
        password,
        options,
    )
}

// Keep one engine for desktop use and headless regression measurements. None
// suppresses only window events, not archive parsing, cancellation or file I/O.
pub(crate) fn extract_with_optional_window<R: ExtractionRuntime + Sync>(
    runtime: &R,
    window: Option<&Window>,
    task_id: &str,
    file: &str,
    output: &str,
    password: Option<&str>,
    options: &DecompressOptions,
) -> Result<()> {
    let _worker_slot = super::zip_parallel::WorkerSlot::acquire(runtime)?;
    let mut archive = ZipArchive::new(crate::services::numeric_volume_reader::ZipInputReader::open(Path::new(file))?)?;
    let selection = crate::services::zip_selection::ZipSelection::read(&mut archive, options, || runtime.check_cancellation())?;
    validate_password_access(&mut archive, password, &selection.indices)?;
    if super::zip_parallel::try_extract(
        runtime,
        window,
        task_id,
        file,
        output,
        &mut archive,
        options,
    )? {
        return Ok(());
    }

    let total_files = selection.indices.len();
    let mut buffer_handle: Option<crate::services::io_buffer_pool::IOBufferHandle> = None;
    let mut last_parent = None;
    let mut progress_gate = ProgressGate::new();

    let extraction_result = (|| -> Result<()> {
        for (position, &index) in selection.indices.iter().enumerate() {
            runtime.check_cancellation()?;
            let (file_name, outpath, is_dir, source_size, source_modified) = {
                let entry = archive.by_index_raw(index)?;
                let file_name = entry.name().to_string();
                let is_dir = entry.is_dir();
                let relative = match runtime
                    .normalized_archive_path(&entry.mangled_name(), true)
                {
                    Some(path) => path,
                    None => continue,
                };
                // Keep archive paths in staging. Selection and filtering use
                // those paths; flattening happens once at transactional commit.
                let target = Path::new(output).join(relative);
                let outpath = if is_dir {
                    target
                } else {
                    extraction_transaction::resolve_extract_path(&target, options)?
                };
                let source_modified = entry.last_modified().and_then(|modified| {
                    system_time(
                        modified.year(),
                        modified.month(),
                        modified.day(),
                        modified.hour(),
                        modified.minute(),
                        modified.second(),
                    )
                });
                (file_name, outpath, is_dir, entry.size(), source_modified)
            };

            let entry_result = (|| -> Result<()> {
                if is_dir {
                    std::fs::create_dir_all(&outpath)?;
                    return Ok(());
                }
                if let Some(parent) = outpath.parent() {
                    if last_parent.as_deref() != Some(parent) {
                        std::fs::create_dir_all(parent)?;
                        last_parent = Some(parent.to_path_buf());
                    }
                }

                let reader = if let Some(password) = password {
                    crate::services::zip_password::open_entry(&mut archive, index, password)?
                } else {
                    archive.by_index(index)?
                };
                let mut outfile = File::create(&outpath)?;
                let buffer_size = runtime.buffer_pool().recommend_buffer_size(source_size);
                if buffer_handle
                    .as_ref()
                    .is_none_or(|handle| handle.capacity() < buffer_size)
                {
                    if let Some(handle) = buffer_handle.take() {
                        tauri::async_runtime::block_on(handle.release());
                    }
                    buffer_handle = Some(tauri::async_runtime::block_on(
                        runtime.buffer_pool().acquire(Some(buffer_size)),
                    ));
                }
                let mut progress_reader =
                    ProgressReader::new(reader, source_size, Arc::new(|_, _| {}));
                let mut last_emitted = 0u64;

                let copy_result = (|| -> Result<()> {
                    let buffer = buffer_handle
                        .as_mut()
                        .expect("acquired copy buffer")
                        .buffer_mut()
                        .as_mut_slice();
                    loop {
                        runtime.check_cancellation()?;
                        let read = progress_reader.read(buffer)?;
                        if read == 0 {
                            break;
                        }
                        outfile.write_all(&buffer[..read])?;
                        let processed = progress_reader.current_pos();
                        if processed < source_size
                            && processed.saturating_sub(last_emitted)
                                >= PROGRESS_EMIT_INTERVAL_BYTES
                        {
                            last_emitted = processed;
                            let entry_progress = if source_size == 0 {
                                1.0
                            } else {
                                processed as f32 / source_size as f32
                            };
                            let file_progress = (position as f32 / total_files as f32)
                                + (entry_progress / total_files as f32);
                            if let Some(window) = window.filter(|_| progress_gate.allow(false)) {
                                runtime.emit_progress(
                                    window,
                                    task_id,
                                    file_progress,
                                    Some(file_name.clone()),
                                    processed,
                                    source_size,
                                );
                            }
                        }
                    }
                    outfile.flush()?;
                    Ok(())
                })();

                copy_result?;

                if options.preserve_timestamps || options.extract_only_newer {
                    if let Some(source_modified) = source_modified {
                        filetime::set_file_mtime(
                            &outpath,
                            filetime::FileTime::from_system_time(source_modified),
                        )?;
                    }
                }
                Ok(())
            })();

            if let Err(error) = entry_result {
                if options.skip_corrupted {
                    if let Some(window) = window {
                        runtime.emit_log(
                            window,
                            task_id,
                            &format!("Skipped entry {file_name}: {error}"),
                            TaskLogSeverity::Warning,
                        );
                    }
                    continue;
                }
                return Err(error);
            }
            if let Some(window) = window.filter(|_| progress_gate.allow(position + 1 == total_files)) {
                runtime.emit_progress(
                    window,
                    task_id,
                    (position + 1) as f32 / total_files as f32,
                    Some(file_name),
                    source_size,
                    source_size,
                );
            }
        }
        Ok(())
    })();
    // Release on success, cancellation and every fallible entry path. Reuse is
    // bounded to one buffer per archive, not one allocation per input file.
    if let Some(handle) = buffer_handle.take() {
        tauri::async_runtime::block_on(handle.release());
    }
    extraction_result
}

#[cfg(test)]
mod tests {
    use super::{system_time, validate_password_access};
    use crate::services::compression_service::CompressionError;
    use std::io::{Cursor, Read, Write};
    use zip::unstable::write::FileOptionsExt;
    use zip::write::FileOptions;
    use zip::ZipWriter;
    use zip_aes::write::SimpleFileOptions;
    use zip_aes::ZipArchive;
    use zip_aes::{CompressionMethod, ZipWriter as ModernZipWriter};

    #[test]
    fn zip_timestamps_reject_invalid_dates() {
        assert!(system_time(2026, 2, 30, 12, 0, 0).is_none());
        assert!(system_time(2026, 7, 30, 24, 0, 0).is_none());
    }

    #[test]
    fn zip_timestamps_clamp_leap_seconds() {
        assert!(system_time(2026, 7, 30, 12, 30, 60).is_some());
    }

    #[test]
    fn modern_zip_reader_preserves_legacy_compression_method_coverage() {
        for method in [CompressionMethod::Bzip2, CompressionMethod::Zstd] {
            let mut writer = ModernZipWriter::new(Cursor::new(Vec::new()));
            writer
                .start_file(
                    "资料/内容.txt",
                    SimpleFileOptions::default().compression_method(method),
                )
                .expect("start compressed entry");
            writer
                .write_all(b"compression compatibility payload")
                .unwrap();
            let cursor = writer.finish().expect("finish archive");

            let mut archive = ZipArchive::new(Cursor::new(cursor.into_inner())).unwrap();
            let mut entry = archive.by_index(0).expect("read compressed entry");
            assert_eq!(entry.name(), "资料/内容.txt");
            let mut payload = Vec::new();
            entry.read_to_end(&mut payload).unwrap();
            assert_eq!(payload, b"compression compatibility payload");
        }
    }

    fn mixed_encryption_archive() -> ZipArchive<Cursor<Vec<u8>>> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for index in 0..8 {
            writer
                .start_file(format!("public-{index}.txt"), FileOptions::default())
                .expect("start public entry");
            writer.write_all(b"public").expect("write public entry");
        }
        writer
            .start_file(
                "secret.txt",
                FileOptions::default().with_deprecated_encryption(b"correct-password"),
            )
            .expect("start encrypted entry");
        writer.write_all(b"secret").expect("write encrypted entry");

        let cursor = writer.finish().expect("finish mixed archive");
        ZipArchive::new(Cursor::new(cursor.into_inner())).expect("read mixed archive")
    }

    #[test]
    fn password_preflight_scans_entries_beyond_the_old_probe_limit() {
        let error = validate_password_access(&mut mixed_encryption_archive(), None, &(0..9).collect::<Vec<_>>())
            .expect_err("late encrypted entry must require a password");
        assert!(
            matches!(
                error.downcast_ref::<CompressionError>(),
                Some(CompressionError::PasswordRequired)
            ),
            "{error:?}"
        );
    }

    #[test]
    fn password_preflight_validates_every_entry_in_mixed_archives() {
        let error =
            validate_password_access(&mut mixed_encryption_archive(), Some("wrong-password"), &(0..9).collect::<Vec<_>>())
                .expect_err("wrong password must fail on the late encrypted entry");
        assert!(
            matches!(
                error.downcast_ref::<CompressionError>(),
                Some(CompressionError::InvalidPassword)
            ),
            "{error:?}"
        );

        validate_password_access(&mut mixed_encryption_archive(), Some("correct-password"), &(0..9).collect::<Vec<_>>())
            .expect("correct password must unlock every entry");
    }

    fn selection_fixture(path: &std::path::Path) {
        let mut writer = ZipWriter::new(std::fs::File::create(path).unwrap());
        for (name, password, payload) in [
            ("资料/公开说明.txt", None, "公开内容"),
            ("资料/甲方/合同.bin", Some("alpha"), "甲方内容"),
            ("资料/乙方/合同.bin", Some("beta"), "乙方内容"),
        ] {
            let options = match password {
                Some(password) => FileOptions::default().with_deprecated_encryption(password.as_bytes()),
                None => FileOptions::default(),
            };
            writer.start_file(name, options).unwrap();
            writer.write_all(payload.as_bytes()).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn selected_zip_public_file_ignores_unselected_encryption() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("混合.zip");
        selection_fixture(&source);
        let output = temp.path().join("out");
        let options = crate::models::compression::DecompressOptions {
            selected_entries: vec!["资料/公开说明.txt".into()],
            ..Default::default()
        };
        super::extract_with_optional_window(
            &crate::services::compression_service::CompressionService::for_testing(),
            None, "public", source.to_str().unwrap(), output.to_str().unwrap(), None, &options,
        ).expect("只要公开说明，不应要求其他文件的密码");
        assert_eq!(std::fs::read_to_string(output.join("资料/公开说明.txt")).unwrap(), "公开内容");
        assert!(!output.join("资料/甲方").exists());
        assert!(!output.join("资料/乙方").exists());
    }

    #[test]
    fn selected_zip_password_ignores_other_password_entry() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("混合.zip");
        selection_fixture(&source);
        let output = temp.path().join("out");
        let options = crate::models::compression::DecompressOptions {
            selected_entries: vec!["资料/甲方".into()],
            ..Default::default()
        };
        super::extract_with_optional_window(
            &crate::services::compression_service::CompressionService::for_testing(),
            None, "alpha", source.to_str().unwrap(), output.to_str().unwrap(), Some("alpha"), &options,
        ).expect("只要甲方文件夹，乙方的不同密码不应阻止操作");
        assert_eq!(std::fs::read_to_string(output.join("资料/甲方/合同.bin")).unwrap(), "甲方内容");
        assert!(!output.join("资料/公开说明.txt").exists());
        assert!(!output.join("资料/乙方").exists());
    }
}
