use super::ExtractionRuntime;
use crate::models::compression::{DecompressOptions, TaskLogSeverity};
use crate::services::compression_service::CompressionError;
use crate::services::extraction_transaction;
use crate::utils::io_utils::{ProgressReader, SmartFileReader};
use anyhow::Result;
use chrono::{Local, TimeZone};
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::Path;
use std::sync::Arc;
use tauri::Window;
use zip::result::ZipError;
use zip::ZipArchive;

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
) -> Result<()> {
    for index in 0..archive.len() {
        let requires_password = match archive.by_index(index) {
            Ok(_) => false,
            Err(ZipError::UnsupportedArchive(detail))
                if detail == ZipError::PASSWORD_REQUIRED =>
            {
                true
            }
            Err(error) => return Err(error.into()),
        };
        if !requires_password {
            continue;
        }

        match password {
            Some(password) => match archive.by_index_decrypt(index, password.as_bytes()) {
                Ok(Ok(mut reader)) => {
                    let mut probe = [0u8; 4];
                    let _read = reader.read(&mut probe)?;
                }
                Ok(Err(_)) => return Err(CompressionError::InvalidPassword.into()),
                Err(error) => return Err(error.into()),
            },
            None => return Err(CompressionError::PasswordRequired.into()),
        }
    }
    Ok(())
}

pub(crate) fn extract<R: ExtractionRuntime>(
    runtime: &R,
    window: &Window,
    task_id: &str,
    file: &str,
    output: &str,
    password: Option<&str>,
    options: &DecompressOptions,
) -> Result<()> {
    let mut archive = ZipArchive::new(SmartFileReader::open(file)?)?;
    validate_password_access(&mut archive, password)?;

    let total_files = archive.len();
    let file_filter =
        extraction_transaction::compile_file_filter(options.file_filter.as_deref());

    for index in 0..total_files {
        runtime.check_cancellation()?;
        let (file_name, outpath, is_dir, source_size, source_modified) = {
            let entry = archive.by_index(index)?;
            let file_name = entry.name().to_string();
            let is_dir = entry.is_dir();
            let relative = match runtime
                .normalized_archive_path(&entry.mangled_name(), options.preserve_paths)
            {
                Some(path) => path,
                None => continue,
            };
            if !extraction_transaction::matches_compiled_file_filter(&relative, &file_filter) {
                continue;
            }
            let target = Path::new(output).join(relative);
            let outpath = if is_dir {
                target
            } else {
                extraction_transaction::resolve_extract_path(&target, options)?
            };
            let modified = entry.last_modified();
            let source_modified = system_time(
                modified.year(),
                modified.month(),
                modified.day(),
                modified.hour(),
                modified.minute(),
                modified.second(),
            );
            (
                file_name,
                outpath,
                is_dir,
                entry.size(),
                source_modified,
            )
        };

        let entry_result = (|| -> Result<()> {
            if is_dir {
                std::fs::create_dir_all(&outpath)?;
                return Ok(());
            }
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let reader = if let Some(password) = password {
                archive.by_index_decrypt(index, password.as_bytes())??
            } else {
                archive.by_index(index)?
            };
            let mut outfile = File::create(&outpath)?;
            let buffer_size = runtime.buffer_pool().recommend_buffer_size(source_size);
            let mut handle =
                tauri::async_runtime::block_on(runtime.buffer_pool().acquire(Some(buffer_size)));
            let mut progress_reader =
                ProgressReader::new(reader, source_size, Arc::new(|_, _| {}));
            let mut last_emitted = 0u64;

            let copy_result = (|| -> Result<()> {
                let buffer = handle.buffer_mut().as_mut_slice();
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
                        let file_progress = (index as f32 / total_files as f32)
                            + (entry_progress / total_files as f32);
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
                outfile.flush()?;
                Ok(())
            })();

            tauri::async_runtime::block_on(handle.release());
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
                runtime.emit_log(
                    window,
                    task_id,
                    &format!("Skipped entry {file_name}: {error}"),
                    TaskLogSeverity::Warning,
                );
                continue;
            }
            return Err(error);
        }
        runtime.emit_progress(
            window,
            task_id,
            (index + 1) as f32 / total_files as f32,
            Some(file_name),
            source_size,
            source_size,
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{system_time, validate_password_access};
    use crate::services::compression_service::CompressionError;
    use std::io::{Cursor, Write};
    use zip::unstable::write::FileOptionsExt;
    use zip::write::FileOptions;
    use zip::{ZipArchive, ZipWriter};

    #[test]
    fn zip_timestamps_reject_invalid_dates() {
        assert!(system_time(2026, 2, 30, 12, 0, 0).is_none());
        assert!(system_time(2026, 7, 30, 24, 0, 0).is_none());
    }

    #[test]
    fn zip_timestamps_clamp_leap_seconds() {
        assert!(system_time(2026, 7, 30, 12, 30, 60).is_some());
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
        let error = validate_password_access(&mut mixed_encryption_archive(), None)
            .expect_err("late encrypted entry must require a password");
        assert!(matches!(
            error.downcast_ref::<CompressionError>(),
            Some(CompressionError::PasswordRequired)
        ), "{error:?}");
    }

    #[test]
    fn password_preflight_validates_every_entry_in_mixed_archives() {
        let error =
            validate_password_access(&mut mixed_encryption_archive(), Some("wrong-password"))
                .expect_err("wrong password must fail on the late encrypted entry");
        assert!(matches!(
            error.downcast_ref::<CompressionError>(),
            Some(CompressionError::InvalidPassword)
        ), "{error:?}");

        validate_password_access(
            &mut mixed_encryption_archive(),
            Some("correct-password"),
        )
        .expect("correct password must unlock every entry");
    }
}
