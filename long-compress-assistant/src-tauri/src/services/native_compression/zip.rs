use super::CompressionRuntime;
use crate::models::compression::{CompressionOptions, TaskLogSeverity};
use crate::services::compression_entries;
use crate::services::compression_service::CompressionError;
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
) -> Result<()> {
    loop {
        runtime.check_cancellation()?;
        let read = reader.read(buffer)?;
        if read == 0 {
            return Ok(());
        }
        writer.write_all(&buffer[..read])?;
    }
}

pub(crate) fn create_encrypted_zip<R: CompressionRuntime>(
    runtime: &R,
    password: &str,
    level: u32,
    sources: &[String],
    output: &str,
    preserve_paths: bool,
) -> Result<()> {
    let file = File::create(output)?;
    let mut writer = zip_aes::ZipWriter::new(file);
    let options = zip_aes::write::SimpleFileOptions::default()
        .compression_method(zip_aes::CompressionMethod::Deflated)
        .compression_level(Some(level.clamp(1, 9) as i64))
        .with_aes_encryption(zip_aes::AesMode::Aes256, password);
    let entries = compression_entries::collect(sources, preserve_paths, true)?;
    let mut copy_buffer = vec![0u8; runtime.copy_buffer_size()];

    for entry in entries {
        runtime.check_cancellation()?;
        let archive_name = entry.archive_name.replace('\\', "/");
        if entry.is_dir {
            writer.add_directory(archive_name, options)?;
        } else {
            writer.start_file(archive_name, options)?;
            let mut source = File::open(entry.path)?;
            copy_cancellable(runtime, &mut source, &mut writer, &mut copy_buffer)?;
        }
    }
    writer.finish()?;
    Ok(())
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

    if options.split_size.is_some_and(|size| size > 0) {
        let split_service = SplitCompressionService::new();
        let handle = tokio::runtime::Handle::current();
        let result = handle.block_on(async {
            split_service
                .compress_to_split_zips_cancellable(
                    sources,
                    Path::new(output),
                    options.clone(),
                    runtime.cancellation_flag(),
                )
                .await
        })?;
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
        create_encrypted_zip(
            runtime,
            password,
            options.level.clamp(1, 9),
            sources,
            output,
            options.preserve_paths.unwrap_or(true),
        )?;
        if let Some(window) = window {
            runtime.emit_progress(window, task_id, 1.0, Some(output.to_string()), 0, 0);
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
        compression_entries::collect(sources, options.preserve_paths.unwrap_or(true), false)?;
    let total = entries.len().max(1);
    let mut copy_buffer = vec![0u8; runtime.copy_buffer_size()];

    for (index, entry) in entries.iter().enumerate() {
        runtime.check_cancellation()?;
        writer.start_file(&entry.archive_name, zip_options)?;
        let mut source = File::open(&entry.path)?;
        copy_cancellable(runtime, &mut source, &mut writer, &mut copy_buffer)?;
        if let Some(window) = window {
            runtime.emit_progress(
                window,
                task_id,
                (index + 1) as f32 / total as f32,
                Some(entry.archive_name.clone()),
                0,
                0,
            );
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
    }

    impl CompressionRuntime for TestRuntime {
        fn check_cancellation(&self) -> Result<()> {
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
}
