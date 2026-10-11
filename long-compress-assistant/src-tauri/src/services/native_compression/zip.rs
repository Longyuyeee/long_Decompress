use super::byte_progress::{source_total_bytes, ByteProgress};
use super::CompressionRuntime;
use crate::models::compression::{CompressionOptions, TaskLogSeverity};
use crate::services::compression_entries;
use crate::services::compression_service::CompressionError;
use crate::services::progress_gate::ProgressGate;
use crate::services::split_compression::SplitCompressionService;
use anyhow::Result;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use tauri::Window;
use zip::{write::FileOptions, CompressionMethod};

fn copy_cancellable<R: CompressionRuntime, Rd: Read, Wr: Write>(
    runtime: &R,
    reader: &mut Rd,
    writer: &mut Wr,
    buffer: &mut [u8],
    mut on_read: impl FnMut(u64),
) -> Result<()> {
    loop {
        runtime.check_cancellation()?;
        let read = reader.read(buffer)?;
        if read == 0 {
            return Ok(());
        }
        writer.write_all(&buffer[..read])?;
        on_read(read as u64);
    }
}

pub(crate) fn create_encrypted_zip<R: CompressionRuntime>(
    runtime: &R,
    password: &str,
    password_encoding: crate::services::zip_password::ZipPasswordEncoding,
    level: u32,
    sources: &[String],
    output: &str,
    preserve_paths: bool,
    _legacy_fast_pack: bool,
    progress_context: Option<(&Window, &str)>,
) -> Result<u64> {
    // Resolve before creating output so unrepresentable passwords leave no file.
    let password_bytes = crate::services::zip_password::encode(password, password_encoding)?;
    let file = File::create(output)?;
    let mut writer = zip_aes::ZipWriter::new(file);
    let options = zip_aes::write::SimpleFileOptions::default()
        .compression_method(zip_aes::CompressionMethod::Deflated)
        .compression_level(Some(level.clamp(1, 9) as i64))
        .with_aes_encryption_bytes(zip_aes::AesMode::Aes256, &password_bytes);
    let entries = compression_entries::collect(sources, preserve_paths, true)?;
    let total_bytes = source_total_bytes(&entries)?;
    let mut progress = ByteProgress::new(total_bytes);
    let mut progress_gate = ProgressGate::new();
    let mut copy_buffer = vec![0u8; runtime.copy_buffer_size()];

    for entry in entries {
        runtime.check_cancellation()?;
        let archive_name = entry.archive_name.replace('\\', "/");
        if entry.is_dir {
            writer.add_directory(archive_name, options)?;
        } else {
            let mut source = File::open(entry.path)?;
            writer.start_file(archive_name, options)?;
            let current_name = entry.archive_name.clone();
            copy_cancellable(
                runtime,
                &mut source,
                &mut writer,
                &mut copy_buffer,
                |read| {
                    if let Some((ratio, processed, total)) = progress.record(read, false) {
                        if !progress_gate.allow(ratio >= 1.0) {
                            return;
                        }
                        if let Some((window, task_id)) = progress_context {
                            runtime.emit_progress(
                                window,
                                task_id,
                                ratio,
                                Some(current_name.clone()),
                                processed,
                                total,
                            );
                        }
                    }
                },
            )?;
            if let Some((ratio, processed, total)) = progress.record(0, true) {
                if let Some((window, task_id)) =
                    progress_context.filter(|_| progress_gate.allow(ratio >= 1.0))
                {
                    runtime.emit_progress(
                        window,
                        task_id,
                        ratio,
                        Some(entry.archive_name.clone()),
                        processed,
                        total,
                    );
                }
            }
        }
    }
    writer.finish()?;
    Ok(total_bytes)
}

pub(crate) fn compress<R: CompressionRuntime>(
    runtime: &R,
    window: Option<&Window>,
    task_id: &str,
    sources: &[String],
    output: &str,
    options: CompressionOptions,
) -> Result<()> {
    if !output.to_ascii_lowercase().ends_with(".zip") {
        return Err(CompressionError::CompressionFailed(
            "ZIP compression output path must end with .zip".to_string(),
        )
        .into());
    }

    if let Some(parent) = Path::new(output).parent() {
        std::fs::create_dir_all(parent)?;
    }

    if let Some(split_size) = options.split_size.filter(|size| *size > 0) {
        runtime.check_cancellation()?;
        // Reuse the real ZIP writer for directories, path layout and encryption.
        // A temporary complete archive costs disk space, not archive-sized RAM.
        struct TemporaryZip(std::path::PathBuf);
        impl Drop for TemporaryZip {
            fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
        }
        let staged = crate::services::output_publish_transaction::staged_output_path(
            Path::new(output), "split-source")?;
        let reservation = std::fs::OpenOptions::new().write(true).create_new(true).open(&staged)?;
        let temporary = TemporaryZip(staged);
        drop(reservation);
        let mut unsplit = options.clone();
        unsplit.split_size = None;
        compress(runtime, window, task_id, sources, &temporary.0.to_string_lossy(), unsplit)?;
        runtime.check_cancellation()?;
        let result = SplitCompressionService::split_archive_file(
            &temporary.0, Path::new(output), split_size, || runtime.check_cancellation(),
        )?;
        if let Some(window) = window {
            runtime.emit_log(
                window,
                task_id,
                &format!("分卷压缩完成：{} 个分卷", result.part_count),
                TaskLogSeverity::Success,
            );
            runtime.emit_progress(window, task_id, 1.0, Some(output.to_string()), 0, 0);
        }
        return Ok(());
    }

    if let Some(password) = options
        .password
        .as_deref()
        .filter(|password| !password.is_empty())
    {
        if let Some(window) = window {
            runtime.emit_log(
                window,
                task_id,
                "使用原生引擎创建 AES-256 加密 ZIP...",
                TaskLogSeverity::Info,
            );
        }
        let total_bytes = create_encrypted_zip(
            runtime,
            password,
            options.zip_password_encoding,
            options.level.clamp(1, 9),
            sources,
            output,
            options.preserve_paths.unwrap_or(true),
            options.fast_pack,
            window.map(|window| (window, task_id)),
        )?;
        if let Some(window) = window {
            if total_bytes == 0 {
                runtime.emit_progress(window, task_id, 1.0, Some(output.to_string()), 0, 0);
            }
            runtime.emit_log(
                window,
                task_id,
                "加密 ZIP 创建完成",
                TaskLogSeverity::Success,
            );
        }
        return Ok(());
    }

    let file = File::create(output)?;
    let mut writer = zip::ZipWriter::new(file);
    let zip_options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(options.level.clamp(1, 9) as i32));
    let entries =
        compression_entries::collect(sources, options.preserve_paths.unwrap_or(true), true)?;
    let total_entries = entries.len().max(1);
    let mut byte_progress = ByteProgress::new(source_total_bytes(&entries)?);
    let mut progress_gate = ProgressGate::new();
    let mut copy_buffer = vec![0u8; runtime.copy_buffer_size()];

    for (index, entry) in entries.iter().enumerate() {
        runtime.check_cancellation()?;
        if entry.is_dir {
            writer.add_directory(entry.archive_name.replace('\\', "/"), zip_options)?;
            continue;
        }
        let mut source = File::open(&entry.path)?;
        writer.start_file(&entry.archive_name, zip_options)?;
        let current_name = entry.archive_name.clone();
        copy_cancellable(
            runtime,
            &mut source,
            &mut writer,
            &mut copy_buffer,
            |read| {
                if let Some((ratio, processed, total)) = byte_progress.record(read, false) {
                    if !progress_gate.allow(ratio >= 1.0) {
                        return;
                    }
                    if let Some(window) = window {
                        runtime.emit_progress(
                            window,
                            task_id,
                            ratio,
                            Some(current_name.clone()),
                            processed,
                            total,
                        );
                    }
                }
            },
        )?;
        if let Some((ratio, processed, total)) = byte_progress.record(0, true) {
            if let Some(window) = window.filter(|_| progress_gate.allow(ratio >= 1.0)) {
                runtime.emit_progress(
                    window,
                    task_id,
                    ratio,
                    Some(entry.archive_name.clone()),
                    processed,
                    total,
                );
            }
        } else if byte_progress.total() == 0 {
            if let Some(window) = window.filter(|_| progress_gate.allow(index + 1 == total_entries))
            {
                runtime.emit_progress(
                    window,
                    task_id,
                    (index + 1) as f32 / total_entries as f32,
                    Some(entry.archive_name.clone()),
                    0,
                    0,
                );
            }
        }
    }
    writer.finish()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    #[derive(Default)]
    struct TestRuntime {
        cancelled: Arc<AtomicBool>,
        cancel_after: std::cell::Cell<Option<usize>>,
    }

    #[test]
    fn native_split_creation_preserves_directories_passwords_and_cleans_temporary_zip() {
        use crate::services::zip_password::{encode, ZipPasswordEncoding};
        for mode in [ZipPasswordEncoding::Utf8, ZipPasswordEncoding::WindowsLocal] {
            for password in [None, Some("密码 + !测试")] {
                if password.is_some_and(|value| encode(value, mode).is_err()) { continue; }
                let temp = tempfile::tempdir().unwrap();
                let folder = temp.path().join("资料");
                std::fs::create_dir_all(folder.join("空目录")).unwrap();
                std::fs::write(folder.join("内容.txt"), b"folder payload").unwrap();
                let second = temp.path().join("第二份.txt");
                std::fs::write(&second, b"second payload").unwrap();
                let output = temp.path().join("bundle.zip");
                compress(&TestRuntime::default(), None, "split-test",
                    &[folder.to_string_lossy().into_owned(), second.to_string_lossy().into_owned()],
                    &output.to_string_lossy(), CompressionOptions {
                        password: password.map(str::to_owned), zip_password_encoding: mode,
                        split_size: Some(37), preserve_paths: Some(true), ..Default::default()
                    }).unwrap();
                let first = temp.path().join("bundle.zip.001");
                crate::services::compression_verification::verify_numeric_zip(&first, password, || false).unwrap();
                let reader = crate::services::numeric_volume_reader::NumericVolumeReader::open(&first).unwrap();
                let mut archive = zip_aes::ZipArchive::new(reader).unwrap();
                for (name, expected) in [("资料/内容.txt", b"folder payload".as_slice()), ("第二份.txt", b"second payload".as_slice())] {
                    let mut entry = match password {
                        Some(value) => crate::services::zip_password::open_named_entry(&mut archive, name, value).unwrap(),
                        None => archive.by_name(name).unwrap(),
                    };
                    let mut bytes = Vec::new(); entry.read_to_end(&mut bytes).unwrap();
                    assert_eq!(bytes, expected);
                }
                assert!(archive.index_for_name("资料/空目录/").is_some());
                assert!(!output.exists());
                assert!(std::fs::read_dir(temp.path()).unwrap().all(|entry| !entry.unwrap().file_name().to_string_lossy().starts_with(".split-source-")));
            }
        }
    }

    #[test]
    fn native_split_collision_preserves_existing_volume_and_removes_temporary_archive() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        std::fs::write(&source, b"source payload").unwrap();
        let output = temp.path().join("bundle.zip");
        let first = temp.path().join("bundle.zip.001");
        std::fs::write(&first, b"existing user data").unwrap();
        let result = compress(&TestRuntime::default(), None, "split-test",
            &[source.to_string_lossy().into_owned()], &output.to_string_lossy(),
            CompressionOptions { split_size: Some(37), ..Default::default() });
        assert!(result.is_err());
        assert_eq!(std::fs::read(first).unwrap(), b"existing user data");
        assert_eq!(std::fs::read(source).unwrap(), b"source payload");
        assert!(std::fs::read_dir(temp.path()).unwrap().all(|entry| !entry.unwrap().file_name().to_string_lossy().starts_with(".split-source-")));
    }

    #[test]
    fn native_split_cancellation_leaves_no_temporary_or_partial_output() {
        for checks in [0, 3] {
            let temp = tempfile::tempdir().unwrap();
            let source = temp.path().join("source.bin");
            std::fs::write(&source, vec![42; 1024 * 1024]).unwrap();
            let runtime = TestRuntime::default();
            runtime.cancel_after.set(Some(checks));
            let result = compress(&runtime, None, "cancel-split",
                &[source.to_string_lossy().into_owned()], &temp.path().join("result.zip").to_string_lossy(),
                CompressionOptions { split_size: Some(10), ..Default::default() });
            assert!(matches!(result.unwrap_err().downcast_ref::<CompressionError>(), Some(CompressionError::Cancelled)));
            let remaining: Vec<_> = std::fs::read_dir(temp.path()).unwrap().map(|entry| entry.unwrap().path()).collect();
            assert_eq!(remaining, vec![source]);
        }
    }

    #[test]
    fn requested_password_encoding_flows_through_real_creation_and_verification() {
        use crate::services::zip_password::{encode, ZipPasswordEncoding};
        let temp = tempfile::tempdir().unwrap();
        let folder = temp.path().join("资料");
        std::fs::create_dir_all(folder.join("空目录")).unwrap();
        std::fs::write(folder.join("内容.txt"), b"real creation payload").unwrap();
        let password = "密码 + !测试";
        for encoding in [ZipPasswordEncoding::Utf8, ZipPasswordEncoding::WindowsLocal] {
            if encode(password, encoding).is_err() { continue; }
            let output = temp.path().join(format!("{encoding:?}.zip"));
            compress(&TestRuntime::default(), None, "encoding-test",
                &[folder.to_string_lossy().into_owned()], &output.to_string_lossy(),
                CompressionOptions { password: Some(password.into()), zip_password_encoding: encoding,
                    preserve_paths: Some(true), level: 6, ..Default::default() }).unwrap();
            assert!(crate::services::compression_verification::verify_native(
                crate::services::compression_format::CompressionRoute::Zip,
                &output, Some(password), || false).unwrap());
            assert!(crate::services::universal_engine::UniversalCliEngine::try_zip_password(&output, password).unwrap());
            assert!(!crate::services::universal_engine::UniversalCliEngine::try_zip_password(&output, "wrong").unwrap());
            if encoding == ZipPasswordEncoding::WindowsLocal {
                let engine = crate::utils::archive_tools::find_7z_command().expect("7-Zip interoperability engine");
                let destination = temp.path().join("independent");
                // Only this synthetic test password goes to argv, never a user's password.
                let result = crate::utils::process::command(engine).arg("x").arg("-y")
                    .arg(format!("-p{password}")).arg(format!("-o{}", destination.display()))
                    .arg(&output).output().unwrap();
                assert!(result.status.success(), "independent extraction failed");
                assert_eq!(std::fs::read(destination.join("资料/内容.txt")).unwrap(), b"real creation payload");
                assert!(destination.join("资料/空目录").is_dir());
            }
        }
        let old: CompressionOptions = serde_json::from_value(serde_json::json!({"level":6})).unwrap();
        assert_eq!(old.zip_password_encoding, ZipPasswordEncoding::Utf8);
        assert!(serde_json::from_value::<CompressionOptions>(serde_json::json!({"level":6,"zip_password_encoding":"unknown"})).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn unrepresentable_local_password_leaves_no_archive() {
        use crate::services::zip_password::{encode, ZipPasswordEncoding};
        let password = "🗝️";
        if encode(password, ZipPasswordEncoding::WindowsLocal).is_ok() { return; }
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        std::fs::write(&source, b"keep").unwrap();
        let output = temp.path().join("bad.zip");
        let result = compress(&TestRuntime::default(), None, "encoding-test",
            &[source.to_string_lossy().into_owned()], &output.to_string_lossy(),
            CompressionOptions { password: Some(password.into()),
                zip_password_encoding: ZipPasswordEncoding::WindowsLocal, ..Default::default() });
        assert!(result.is_err());
        assert!(!output.exists());
        assert_eq!(std::fs::read(source).unwrap(), b"keep");
    }

    #[test]
    fn retired_fast_pack_is_ignored_and_zip_payloads_remain_intact() {
        let legacy: CompressionOptions =
            serde_json::from_value(serde_json::json!({ "level": 6, "fast_pack": true })).unwrap();
        assert!(!legacy.fast_pack);
        assert!(serde_json::to_value(&legacy).unwrap().get("fast_pack").is_none());
        let temp = tempfile::tempdir().unwrap();
        let mut state = 0x12345678u32;
        let random: Vec<u8> = (0..1024 * 1024)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as u8
            })
            .collect();
        let fixtures = [
            ("random.txt", random),
            ("text.jpg", vec![b'A'; 1024 * 1024]),
            ("small.bin", vec![1, 2, 3, 4]),
        ];
        let sources: Vec<String> = fixtures
            .iter()
            .map(|(name, bytes)| {
                let path = temp.path().join(name);
                std::fs::write(&path, bytes).unwrap();
                path.to_string_lossy().into_owned()
            })
            .collect();
        for encrypted in [false, true] {
            for fast_pack in [false, true] {
                let output = temp.path().join(format!("{encrypted}-{fast_pack}.zip"));
                compress(
                    &TestRuntime::default(),
                    None,
                    "fast-pack",
                    &sources,
                    output.to_str().unwrap(),
                    CompressionOptions {
                        fast_pack,
                        level: 6,
                        preserve_paths: Some(false),
                        password: encrypted.then(|| "test-password".into()),
                        ..Default::default()
                    },
                )
                .unwrap();
                let mut archive = zip_aes::ZipArchive::new(File::open(&output).unwrap()).unwrap();
                assert_eq!(archive.len(), fixtures.len());
                for (name, expected) in &fixtures {
                    let mut entry = if encrypted {
                        archive.by_name_decrypt(name, b"test-password").unwrap()
                    } else {
                        archive.by_name(name).unwrap()
                    };
                    let expected_method = zip_aes::CompressionMethod::Deflated;
                    assert_eq!(entry.compression(), expected_method);
                    let mut actual = Vec::new();
                    entry.read_to_end(&mut actual).unwrap();
                    assert_eq!(&actual, expected);
                }
                if encrypted {
                    assert!(archive
                        .by_name_decrypt("random.txt", b"wrong-password")
                        .is_err());
                }
            }
        }
    }

    impl CompressionRuntime for TestRuntime {
        fn check_cancellation(&self) -> Result<()> {
            if let Some(remaining) = self.cancel_after.get() {
                if remaining == 0 { self.cancelled.store(true, Ordering::SeqCst); }
                else { self.cancel_after.set(Some(remaining - 1)); }
            }
            if self.cancelled.load(Ordering::Relaxed) {
                Err(CompressionError::Cancelled.into())
            } else {
                Ok(())
            }
        }

        fn cancellation_flag(&self) -> Arc<AtomicBool> {
            self.cancelled.clone()
        }

        fn copy_buffer_size(&self) -> usize {
            256 * 1024
        }

        fn emit_log(
            &self,
            _window: &Window,
            _task_id: &str,
            _message: &str,
            _severity: TaskLogSeverity,
        ) {
        }

        fn emit_progress(
            &self,
            _window: &Window,
            _task_id: &str,
            _progress: f32,
            _current_file: Option<String>,
            _processed_bytes: u64,
            _total_bytes: u64,
        ) {
        }
    }

    #[test]
    fn plain_zip_round_trip_preserves_payload() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("payload.txt");
        let archive = temp.path().join("payload.zip");
        std::fs::write(&source, b"native zip payload").expect("write source");

        compress(
            &TestRuntime::default(),
            None,
            "zip-task",
            &[source.to_string_lossy().to_string()],
            archive.to_string_lossy().as_ref(),
            CompressionOptions::default(),
        )
        .expect("create zip");

        let mut archive =
            zip::ZipArchive::new(File::open(archive).expect("open zip")).expect("read zip");
        let mut payload = String::new();
        archive
            .by_name("payload.txt")
            .expect("payload entry")
            .read_to_string(&mut payload)
            .expect("read payload");
        assert_eq!(payload, "native zip payload");
    }

    #[test]
    fn cancellation_stops_before_creating_entries() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("payload.txt");
        let archive = temp.path().join("cancelled.zip");
        std::fs::write(&source, b"cancelled payload").expect("write source");
        let runtime = TestRuntime::default();
        runtime.cancelled.store(true, Ordering::Relaxed);

        let error = compress(
            &runtime,
            None,
            "cancel-task",
            &[source.to_string_lossy().to_string()],
            archive.to_string_lossy().as_ref(),
            CompressionOptions::default(),
        )
        .expect_err("cancelled zip must fail");
        assert!(matches!(
            error.downcast_ref::<CompressionError>(),
            Some(CompressionError::Cancelled)
        ));
    }

    #[test]
    fn source_total_bytes_ignores_directories_and_counts_payloads() {
        let temp = tempfile::tempdir().expect("temp dir");
        let folder = temp.path().join("payloads");
        std::fs::create_dir_all(&folder).expect("create folder");
        std::fs::write(folder.join("alpha.bin"), vec![1u8; 8192]).expect("alpha");
        std::fs::write(folder.join("beta.bin"), vec![2u8; 4096]).expect("beta");
        let entries =
            compression_entries::collect(&[folder.to_string_lossy().to_string()], true, true)
                .expect("collect entries");

        assert_eq!(source_total_bytes(&entries).expect("total bytes"), 12_288);
    }
}
