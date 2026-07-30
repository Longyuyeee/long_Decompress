use super::CompressionRuntime;
use crate::models::compression::{CompressionOptions, TaskLogSeverity};
use crate::services::compression_entries;
use crate::services::compression_service::CompressionError;
use anyhow::Result;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tauri::Window;

fn validate_output(output: &str, extensions: &[&str]) -> Result<()> {
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
    Ok(())
}

fn write_entries<R: CompressionRuntime, W: Write>(
    runtime: &R,
    window: Option<&Window>,
    task_id: &str,
    sources: &[String],
    options: &CompressionOptions,
    builder: &mut tar::Builder<W>,
) -> Result<()> {
    let entries =
        compression_entries::collect(sources, options.preserve_paths.unwrap_or(true), true)?;
    let total = entries.len().max(1);
    for (index, entry) in entries.iter().enumerate() {
        runtime.check_cancellation()?;
        if entry.is_dir {
            builder.append_dir(&entry.archive_name, &entry.path)?;
        } else {
            builder.append_path_with_name(&entry.path, &entry.archive_name)?;
        }
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
    Ok(())
}

pub(crate) fn compress_tar<R: CompressionRuntime>(
    runtime: &R,
    window: Option<&Window>,
    task_id: &str,
    sources: &[String],
    output: &str,
    options: CompressionOptions,
) -> Result<()> {
    validate_output(output, &[".tar"])?;
    let file = File::create(output)?;
    let mut builder = tar::Builder::new(file);
    write_entries(runtime, window, task_id, sources, &options, &mut builder)?;
    builder.finish()?;
    Ok(())
}

pub(crate) fn compress_gzip<R: CompressionRuntime>(
    runtime: &R,
    window: Option<&Window>,
    task_id: &str,
    sources: &[String],
    output: &str,
    options: CompressionOptions,
) -> Result<()> {
    validate_output(output, &[".tar.gz", ".tgz"])?;
    let file = File::create(output)?;
    let encoder =
        flate2::write::GzEncoder::new(file, flate2::Compression::new(options.level.clamp(1, 9)));
    let mut builder = tar::Builder::new(encoder);
    write_entries(runtime, window, task_id, sources, &options, &mut builder)?;
    builder.into_inner()?.finish()?;
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
    validate_output(output, &[".tar.bz2", ".tbz", ".tbz2"])?;
    let file = File::create(output)?;
    let encoder =
        bzip2::write::BzEncoder::new(file, bzip2::Compression::new(options.level.clamp(1, 9)));
    let mut builder = tar::Builder::new(encoder);
    write_entries(runtime, window, task_id, sources, &options, &mut builder)?;
    builder.into_inner()?.finish()?;
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
    validate_output(output, &[".tar.xz", ".txz"])?;
    let file = File::create(output)?;
    let encoder = xz2::write::XzEncoder::new(file, options.level.clamp(1, 9));
    let mut builder = tar::Builder::new(encoder);
    write_entries(runtime, window, task_id, sources, &options, &mut builder)?;
    builder.into_inner()?.finish()?;
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
    validate_output(output, &[".tar.zst", ".tzst"])?;
    if let Some(window) = window {
        runtime.emit_log(
            window,
            task_id,
            "使用原生 tar.zst 压缩...",
            TaskLogSeverity::Info,
        );
    }
    let file = File::create(output)?;
    let encoder = zstd::stream::write::Encoder::new(file, options.level.clamp(1, 21) as i32)?;
    let mut builder = tar::Builder::new(encoder);
    write_entries(runtime, window, task_id, sources, &options, &mut builder)?;
    builder.into_inner()?.finish()?;
    if let Some(window) = window {
        runtime.emit_progress(window, task_id, 1.0, Some(output.to_string()), 0, 0);
        runtime.emit_log(
            window,
            task_id,
            "tar.zst 压缩完成",
            TaskLogSeverity::Success,
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{atomic::AtomicBool, Arc};

    #[derive(Default)]
    struct TestRuntime {
        cancelled: Arc<AtomicBool>,
    }

    impl CompressionRuntime for TestRuntime {
        fn check_cancellation(&self) -> Result<()> {
            Ok(())
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
    fn tar_gzip_round_trip_preserves_payload() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("payload.txt");
        let archive = temp.path().join("payload.tar.gz");
        let extracted = temp.path().join("extracted");
        std::fs::write(&source, b"native tar gzip payload").expect("write source");

        compress_gzip(
            &TestRuntime::default(),
            None,
            "tar-gzip-task",
            &[source.to_string_lossy().to_string()],
            archive.to_string_lossy().as_ref(),
            CompressionOptions::default(),
        )
        .expect("create tar.gz");
        let decoder = flate2::read::GzDecoder::new(File::open(archive).expect("open archive"));
        tar::Archive::new(decoder)
            .unpack(&extracted)
            .expect("extract archive");

        assert_eq!(
            std::fs::read(extracted.join("payload.txt")).expect("read payload"),
            b"native tar gzip payload"
        );
    }
}
