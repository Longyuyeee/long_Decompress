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

fn skippable_corruption(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| cause.downcast_ref::<std::io::Error>().is_some_and(|error|
        matches!(error.kind(), std::io::ErrorKind::InvalidData | std::io::ErrorKind::UnexpectedEof)))
}

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
    encoding: Option<crate::services::zip_password::ZipPasswordEncoding>,
    selection: &crate::services::zip_selection::ZipSelection,
    mut check_control: impl FnMut() -> Result<()>,
) -> Result<()> {
    check_control()?;
    // Selection already checked metadata/methods and identified every selected
    // encrypted entry. Opening plain entries again initialized codecs and did
    // extra seeks without validating any password or reading their CRCs.
    for &index in &selection.encrypted_indices {
        check_control()?;
        match password {
            Some(password) => match crate::services::zip_password::open_entry_with_encoding(archive, index, password, encoding) {
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
    validate_password_access(&mut archive, password, options.zip_password_encoding, &selection, || runtime.check_cancellation())?;
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
                let source_modified = crate::services::zip_read_time::modified(&entry);
                (file_name, outpath, is_dir, entry.size(), source_modified)
            };

            let mut created_file = false;
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
                    crate::services::zip_password::open_entry_with_encoding(&mut archive, index, password, options.zip_password_encoding)?
                } else {
                    archive.by_index(index)?
                };
                let mut outfile = File::create(&outpath)?;
                created_file = true;
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
                if options.skip_corrupted && skippable_corruption(&error) {
                    if created_file {
                        std::fs::remove_file(&outpath).map_err(|cleanup|
                            anyhow::anyhow!("无法清理未完成的 ZIP 条目「{file_name}」：{cleanup}；原始错误：{error}"))?;
                    }
                    if let Some(window) = window {
                        runtime.emit_log(
                            window,
                            task_id,
                            &format!("已跳过损坏条目「{file_name}」，未保留不完整文件：{error}"),
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

    fn validate_all_passwords<R: std::io::Read + std::io::Seek>(
        archive: &mut ZipArchive<R>, password: Option<&str>,
    ) -> anyhow::Result<()> {
        let selection = crate::services::zip_selection::ZipSelection::read(
            archive, &crate::models::compression::DecompressOptions::default(), || Ok(()),
        )?;
        validate_password_access(archive, password, None, &selection, || Ok(()))
    }

    #[test]
    fn password_preflight_scans_entries_beyond_the_old_probe_limit() {
        let error = validate_all_passwords(&mut mixed_encryption_archive(), None)
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
            validate_all_passwords(&mut mixed_encryption_archive(), Some("wrong-password"))
                .expect_err("wrong password must fail on the late encrypted entry");
        assert!(
            matches!(
                error.downcast_ref::<CompressionError>(),
                Some(CompressionError::InvalidPassword)
            ),
            "{error:?}"
        );

        validate_all_passwords(&mut mixed_encryption_archive(), Some("correct-password"))
            .expect("correct password must unlock every entry");
    }

    #[test]
    fn password_preflight_checks_later_distinct_password_before_creating_output() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("两个密码.zip");
        let mut writer = ModernZipWriter::new(std::fs::File::create(&source).unwrap());
        for (name, password) in [("甲方.txt", "alpha"), ("乙方.txt", "beta")] {
            writer.start_file(name, SimpleFileOptions::default()
                .with_aes_encryption(zip_aes::AesMode::Aes256, password)).unwrap();
            writer.write_all(b"distinct AES passwords").unwrap();
        }
        writer.finish().unwrap();
        let output = temp.path().join("out");
        let error = super::extract_with_optional_window(
            &crate::services::compression_service::CompressionService::for_testing(),
            None, "mixed-passwords", source.to_str().unwrap(), output.to_str().unwrap(), Some("alpha"),
            &crate::models::compression::DecompressOptions::default(),
        ).unwrap_err();
        assert!(matches!(error.downcast_ref::<CompressionError>(), Some(CompressionError::InvalidPassword)), "{error:?}");
        assert!(!output.exists(), "later selected password mismatch must fail before writing even public entries");
    }

    #[test]
    fn password_preflight_cancellation_is_not_a_password_error() {
        let mut archive = mixed_encryption_archive();
        let selection = crate::services::zip_selection::ZipSelection::read(
            &mut archive, &crate::models::compression::DecompressOptions::default(), || Ok(()),
        ).unwrap();
        let mut checks = 0;
        let error = validate_password_access(&mut archive, Some("correct-password"), None, &selection, || {
            checks += 1;
            if checks == 2 { return Err(CompressionError::Cancelled.into()); }
            Ok(())
        }).unwrap_err();
        assert!(matches!(error.downcast_ref::<CompressionError>(), Some(CompressionError::Cancelled)));
        assert_eq!(checks, 2);
    }

    // Frozen implementation from f2a22b2, for same-process A/B measurements
    // only. Production always uses validate_password_access above.
    fn legacy_password_preflight<R: std::io::Read + std::io::Seek>(
        archive: &mut ZipArchive<R>, password: Option<&str>, indices: &[usize],
    ) -> anyhow::Result<()> {
        for &index in indices {
            let required = match archive.by_index(index) {
                Ok(_) => false,
                Err(zip_aes::result::ZipError::UnsupportedArchive(detail))
                    if detail == zip_aes::result::ZipError::PASSWORD_REQUIRED => true,
                Err(error) => return Err(error.into()),
            };
            if !required { continue; }
            match password {
                Some(password) => match crate::services::zip_password::open_entry(archive, index, password) {
                    Ok(mut reader) => { reader.read(&mut [0u8; 4])?; }
                    Err(zip_aes::result::ZipError::InvalidPassword) => return Err(CompressionError::InvalidPassword.into()),
                    Err(error) => return Err(error.into()),
                },
                None => return Err(CompressionError::PasswordRequired.into()),
            }
        }
        Ok(())
    }

    #[derive(Default)]
    struct ReadCounts {
        bytes: std::cell::Cell<u64>,
        reads: std::cell::Cell<u64>,
        seeks: std::cell::Cell<u64>,
    }
    struct CountedInput<R> {
        reader: R,
        counts: std::rc::Rc<ReadCounts>,
    }
    impl<R: Read> Read for CountedInput<R> {
        fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
            let read = self.reader.read(bytes)?;
            self.counts.reads.set(self.counts.reads.get() + 1);
            self.counts.bytes.set(self.counts.bytes.get() + read as u64);
            Ok(read)
        }
    }
    impl<R: std::io::Seek> std::io::Seek for CountedInput<R> {
        fn seek(&mut self, position: std::io::SeekFrom) -> std::io::Result<u64> {
            self.counts.seeks.set(self.counts.seeks.get() + 1);
            self.reader.seek(position)
        }
    }

    #[test]
    fn password_preflight_does_not_reopen_selected_plain_entries() {
        let counts = std::rc::Rc::new(ReadCounts::default());
        let mut archive = ZipArchive::new(CountedInput {
            reader: Cursor::new(mixed_encryption_archive().into_inner().into_inner()), counts: counts.clone(),
        }).unwrap();
        let options = crate::models::compression::DecompressOptions {
            file_filter: Some("public-*.txt".into()), ..Default::default()
        };
        let selection = crate::services::zip_selection::ZipSelection::read(&mut archive, &options, || Ok(())).unwrap();
        assert_eq!(selection.indices.len(), 8);
        assert!(!selection.password_required);
        let before = (counts.bytes.get(), counts.reads.get(), counts.seeks.get());
        validate_password_access(&mut archive, None, None, &selection, || Ok(())).unwrap();
        assert_eq!((counts.bytes.get(), counts.reads.get(), counts.seeks.get()), before,
            "password preflight must not initialize codecs/seek/read plain selected entries again");
    }

    #[test]
    #[ignore = "explicit disk-backed password preflight A/B, not extraction or physical I/O timing"]
    fn zip_password_preflight_measurement() {
        use sha2::{Digest, Sha256};
        use std::time::Instant;
        let temp = tempfile::tempdir().unwrap();
        for encrypted in [false, true] {
            let source = temp.path().join(if encrypted { "mixed.zip" } else { "plain.zip" });
            let mut writer = ZipWriter::new(std::fs::File::create(&source).unwrap());
            for index in 0..5000 {
                writer.start_file(format!("资料/小文件-{index:05}.txt"), FileOptions::default()).unwrap();
                writer.write_all(&[42; 4096]).unwrap();
            }
            if encrypted {
                writer.start_file("资料/最后的密码文件.txt", FileOptions::default().with_deprecated_encryption(b"correct-password")).unwrap();
                writer.write_all(b"the late encrypted entry must still be checked").unwrap();
            }
            writer.finish().unwrap();
            let fixture_sha256 = format!("{:x}", Sha256::digest(std::fs::read(&source).unwrap()));
            // Alternate execution order, including warm-up. Every observation
            // uses the same file and fresh archive/selection, never a global cache.
            for round in 0..6 {
                for legacy in if round % 2 == 0 { [true, false] } else { [false, true] } {
                    let counts = std::rc::Rc::new(ReadCounts::default());
                    let mut archive = ZipArchive::new(CountedInput {
                        reader: std::fs::File::open(&source).unwrap(), counts: counts.clone(),
                    }).unwrap();
                    let selection = crate::services::zip_selection::ZipSelection::read(
                        &mut archive, &crate::models::compression::DecompressOptions::default(), || Ok(()),
                    ).unwrap();
                    let before = (counts.bytes.get(), counts.reads.get(), counts.seeks.get());
                    let started = Instant::now();
                    if legacy {
                        legacy_password_preflight(&mut archive, Some("correct-password"), &selection.indices).unwrap();
                    } else {
                        validate_password_access(&mut archive, Some("correct-password"), None, &selection, || Ok(())).unwrap();
                    }
                    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
                    if round > 0 {
                        println!("ZIP_PREFLIGHT_JSON {}", serde_json::json!({
                            "scenario": if encrypted { "5000_plain_plus_one_zipcrypto" } else { "5000_plain" },
                            "legacy": legacy, "round": round, "fixture_sha256": fixture_sha256,
                            "elapsed_ms": elapsed_ms, "logical_read_bytes": counts.bytes.get() - before.0,
                            "read_calls": counts.reads.get() - before.1, "seek_calls": counts.seeks.get() - before.2,
                        }));
                    }
                }
            }
        }
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
    fn selected_zip_skip_corruption_discards_incomplete_file_and_keeps_good_entries() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("部分损坏.zip");
        let mut writer = ModernZipWriter::new(std::fs::File::create(&source).unwrap());
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        writer.start_file("bad.txt", options).unwrap(); writer.write_all(&vec![42; 100_000]).unwrap();
        writer.start_file("good.txt", options).unwrap(); writer.write_all(b"good").unwrap(); writer.finish().unwrap();
        let offset = ZipArchive::new(std::fs::File::open(&source).unwrap()).unwrap().by_index_raw(0).unwrap().data_start().unwrap() as usize;
        let mut bytes = std::fs::read(&source).unwrap(); bytes[offset+23] ^= 1; std::fs::write(&source, bytes).unwrap();
        let output = temp.path().join("out");
        super::extract_with_optional_window(&crate::services::compression_service::CompressionService::for_testing(), None, "skip-corrupt", source.to_str().unwrap(), output.to_str().unwrap(), None,
            &crate::models::compression::DecompressOptions { skip_corrupted: true, ..Default::default() }).unwrap();
        assert_eq!(std::fs::read(output.join("good.txt")).unwrap(), b"good");
        assert!(!output.join("bad.txt").exists(), "跳过损坏文件不能发布已写出的半截内容");
    }

    #[test]
    fn selected_zip_skip_corruption_never_hides_cancellation_or_storage_errors() {
        assert!(super::skippable_corruption(&std::io::Error::new(std::io::ErrorKind::InvalidData, "bad CRC").into()));
        assert!(super::skippable_corruption(&std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "truncated").into()));
        for kind in [std::io::ErrorKind::PermissionDenied, std::io::ErrorKind::StorageFull, std::io::ErrorKind::Interrupted] {
            assert!(!super::skippable_corruption(&std::io::Error::new(kind, "not corrupt data").into()));
        }
        assert!(!super::skippable_corruption(&CompressionError::Cancelled.into()));
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
