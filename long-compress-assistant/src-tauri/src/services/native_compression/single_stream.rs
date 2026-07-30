use super::CompressionRuntime;
use crate::models::compression::{CompressionOptions, TaskLogSeverity};
use crate::services::compression_service::CompressionError;
use anyhow::Result;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use tauri::Window;

const PROGRESS_EMIT_INTERVAL_BYTES: u64 = 4 * 1024 * 1024;

fn validate_source<'a>(
    sources: &'a [String],
    output: &str,
    extensions: &[&str],
) -> Result<&'a Path> {
    if sources.len() != 1 {
        return Err(CompressionError::CompressionFailed(format!(
            "{} compression only supports one regular file.",
            extensions.join("/")
        ))
        .into());
    }
    let source = Path::new(&sources[0]);
    if !source.is_file() {
        return Err(CompressionError::CompressionFailed(format!(
            "{} compression only supports one regular file.",
            extensions.join("/")
        ))
        .into());
    }
    let output_lower = output.to_ascii_lowercase();
    if !extensions
        .iter()
        .any(|extension| output_lower.ends_with(extension))
    {
        return Err(CompressionError::CompressionFailed(format!(
            "Output path must end with one of: {}",
            extensions.join(", ")
        ))
        .into());
    }
    if let Some(parent) = Path::new(output).parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(source)
}

fn copy_cancellable<R: CompressionRuntime, Rd: Read, Wr: Write>(
    runtime: &R,
    reader: &mut Rd,
    writer: &mut Wr,
) -> Result<()> {
    let mut buffer = vec![0u8; runtime.copy_buffer_size()];
    loop {
        runtime.check_cancellation()?;
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            return Ok(());
        }
        writer.write_all(&buffer[..read])?;
    }
}

pub(crate) fn compress_gzip<R: CompressionRuntime>(
    runtime: &R,
    window: Option<&Window>,
    task_id: &str,
    sources: &[String],
    output: &str,
    options: CompressionOptions,
) -> Result<()> {
    let source = validate_source(sources, output, &[".gz"])?;
    let mut input = File::open(source)?;
    let file = File::create(output)?;
    let mut encoder =
        flate2::write::GzEncoder::new(file, flate2::Compression::new(options.level.clamp(1, 9)));
    copy_cancellable(runtime, &mut input, &mut encoder)?;
    encoder.finish()?;
    emit_file_complete(runtime, window, task_id, source);
    Ok(())
}

pub(crate) fn compress_bzip2<R: CompressionRuntime>(
    runtime: &R,
    window: Option<&Window>,
    task_id: &str,
    sources: &[String],
    output: &str,
    options: CompressionOptions,
) -> Result<()> {
    let source = validate_source(sources, output, &[".bz2"])?;
    let mut input = File::open(source)?;
    let file = File::create(output)?;
    let mut encoder =
        bzip2::write::BzEncoder::new(file, bzip2::Compression::new(options.level.clamp(1, 9)));
    copy_cancellable(runtime, &mut input, &mut encoder)?;
    encoder.finish()?;
    emit_file_complete(runtime, window, task_id, source);
    Ok(())
}

pub(crate) fn compress_xz<R: CompressionRuntime>(
    runtime: &R,
    window: Option<&Window>,
    task_id: &str,
    sources: &[String],
    output: &str,
    options: CompressionOptions,
) -> Result<()> {
    let source = validate_source(sources, output, &[".xz"])?;
    let mut input = File::open(source)?;
    let file = File::create(output)?;
    let mut encoder = xz2::write::XzEncoder::new(file, options.level.clamp(1, 9));
    copy_cancellable(runtime, &mut input, &mut encoder)?;
    encoder.finish()?;
    emit_file_complete(runtime, window, task_id, source);
    Ok(())
}

pub(crate) fn compress_zstd<R: CompressionRuntime>(
    runtime: &R,
    window: Option<&Window>,
    task_id: &str,
    sources: &[String],
    output: &str,
    options: CompressionOptions,
) -> Result<()> {
    let source = validate_source(sources, output, &[".zst", ".zstd"])?;
    if let Some(window) = window {
        runtime.emit_log(
            window,
            task_id,
            "使用原生 Zstd 压缩...",
            TaskLogSeverity::Info,
        );
    }
    let mut input = File::open(source)?;
    let file = File::create(output)?;
    let mut encoder = zstd::stream::write::Encoder::new(file, options.level.clamp(1, 21) as i32)?;
    copy_cancellable(runtime, &mut input, &mut encoder)?;
    encoder.finish()?;
    if let Some(window) = window {
        runtime.emit_progress(window, task_id, 1.0, Some(output.to_string()), 0, 0);
        runtime.emit_log(window, task_id, "Zstd 压缩完成", TaskLogSeverity::Success);
    }
    Ok(())
}

pub(crate) fn compress_lzma<R: CompressionRuntime>(
    runtime: &R,
    window: Option<&Window>,
    task_id: &str,
    sources: &[String],
    output: &str,
    options: CompressionOptions,
) -> Result<()> {
    let source = validate_source(sources, output, &[".lzma"])?;
    if let Some(window) = window {
        runtime.emit_log(
            window,
            task_id,
            "正在使用原生 LZMA 编码器压缩...",
            TaskLogSeverity::Info,
        );
    }
    let lzma_options =
        xz2::stream::LzmaOptions::new_preset(options.level.clamp(1, 9)).map_err(|error| {
            CompressionError::CompressionFailed(format!(
                "Unable to configure LZMA encoder: {error}"
            ))
        })?;
    let stream = xz2::stream::Stream::new_lzma_encoder(&lzma_options).map_err(|error| {
        CompressionError::CompressionFailed(format!("Unable to initialize LZMA encoder: {error}"))
    })?;
    let input_file = File::open(source)?;
    let total_bytes = input_file.metadata()?.len();
    let output_file = File::create(output)?;
    let mut encoder = xz2::write::XzEncoder::new_stream(output_file, stream);
    let mut reader = input_file;
    let mut buffer = vec![0u8; runtime.copy_buffer_size()];
    let mut processed_bytes = 0u64;
    let mut last_emitted = 0u64;
    loop {
        runtime.check_cancellation()?;
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        encoder.write_all(&buffer[..read])?;
        processed_bytes = processed_bytes.saturating_add(read as u64);
        if processed_bytes.saturating_sub(last_emitted) >= PROGRESS_EMIT_INTERVAL_BYTES {
            last_emitted = processed_bytes;
            if let Some(window) = window {
                let progress = if total_bytes == 0 {
                    1.0
                } else {
                    processed_bytes as f32 / total_bytes as f32
                };
                runtime.emit_progress(
                    window,
                    task_id,
                    progress.min(0.99),
                    Some(source.to_string_lossy().into_owned()),
                    processed_bytes,
                    total_bytes,
                );
            }
        }
    }
    runtime.check_cancellation()?;
    encoder.finish()?;
    if let Some(window) = window {
        runtime.emit_progress(window, task_id, 1.0, Some(output.to_string()), 0, 0);
        runtime.emit_log(window, task_id, "LZMA 压缩完成", TaskLogSeverity::Success);
    }
    Ok(())
}

fn emit_file_complete<R: CompressionRuntime>(
    runtime: &R,
    window: Option<&Window>,
    task_id: &str,
    source: &Path,
) {
    if let Some(window) = window {
        runtime.emit_progress(
            window,
            task_id,
            1.0,
            source
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_string),
            0,
            0,
        );
    }
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
    fn zstd_round_trip_preserves_payload() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("payload.txt");
        let archive = temp.path().join("payload.zst");
        std::fs::write(&source, b"native zstd payload").expect("write source");

        compress_zstd(
            &TestRuntime::default(),
            None,
            "zstd-task",
            &[source.to_string_lossy().to_string()],
            archive.to_string_lossy().as_ref(),
            CompressionOptions::default(),
        )
        .expect("create zstd");
        let payload = zstd::stream::decode_all(File::open(archive).expect("open archive"))
            .expect("decode zstd");
        assert_eq!(payload, b"native zstd payload");
    }

    #[test]
    fn gzip_cancellation_is_preserved_as_cancelled() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("payload.txt");
        let archive = temp.path().join("cancelled.gz");
        std::fs::write(&source, b"cancelled stream payload").expect("write source");
        let runtime = TestRuntime::default();
        runtime.cancelled.store(true, Ordering::Relaxed);

        let error = compress_gzip(
            &runtime,
            None,
            "cancelled-gzip-task",
            &[source.to_string_lossy().to_string()],
            archive.to_string_lossy().as_ref(),
            CompressionOptions::default(),
        )
        .expect_err("cancelled gzip must fail");
        assert!(matches!(
            error.downcast_ref::<CompressionError>(),
            Some(CompressionError::Cancelled)
        ));
    }
}
