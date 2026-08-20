use super::byte_progress::{source_total_bytes, ByteProgress};
use super::CompressionRuntime;
use crate::models::compression::{CompressionOptions, TaskLogSeverity};
use crate::services::compression_entries;
use crate::services::compression_service::CompressionError;
use anyhow::Result;
use std::fs::File;
use std::io::{self, Read, Write};
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

struct ProgressReader<'a, R: CompressionRuntime> {
    runtime: &'a R,
    source: File,
    progress: &'a mut ByteProgress,
    window: Option<&'a Window>,
    task_id: &'a str,
    current_file: String,
}

impl<R: CompressionRuntime> Read for ProgressReader<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.runtime
            .check_cancellation()
            .map_err(|error| io::Error::other(error.to_string()))?;
        let read = self.source.read(buffer)?;
        if let Some((ratio, processed, total)) = self.progress.record(read as u64, false) {
            if let Some(window) = self.window {
                self.runtime.emit_progress(
                    window,
                    self.task_id,
                    ratio,
                    Some(self.current_file.clone()),
                    processed,
                    total,
                );
            }
        }
        Ok(read)
    }
}

fn emit_forced_progress<R: CompressionRuntime>(
    runtime: &R,
    window: Option<&Window>,
    task_id: &str,
    current_file: String,
    progress: &mut ByteProgress,
) {
    let Some((ratio, processed, total)) = progress.record(0, true) else {
        return;
    };
    if let Some(window) = window {
        runtime.emit_progress(window, task_id, ratio, Some(current_file), processed, total);
    }
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
    let total_entries = entries.len().max(1);
    let mut progress = ByteProgress::new(source_total_bytes(&entries)?);
    for (index, entry) in entries.iter().enumerate() {
        runtime.check_cancellation()?;
        if entry.is_dir {
            builder.append_dir(&entry.archive_name, &entry.path)?;
            if progress.total() == 0 {
                if let Some(window) = window {
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
        } else {
            let metadata = entry.path.metadata()?;
            let mut header = tar::Header::new_gnu();
            header.set_metadata(&metadata);
            header.set_size(metadata.len());
            let reader = ProgressReader {
                runtime,
                source: File::open(&entry.path)?,
                progress: &mut progress,
                window,
                task_id,
                current_file: entry.archive_name.clone(),
            };
            if let Err(error) = builder.append_data(&mut header, &entry.archive_name, reader) {
                runtime.check_cancellation()?;
                return Err(error.into());
            }
            emit_forced_progress(
                runtime,
                window,
                task_id,
                entry.archive_name.clone(),
                &mut progress,
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
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
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

    struct CancelDuringReadRuntime {
        checks: AtomicUsize,
        cancel_at: usize,
        cancelled: Arc<AtomicBool>,
    }

    impl CompressionRuntime for CancelDuringReadRuntime {
        fn check_cancellation(&self) -> Result<()> {
            if self.checks.fetch_add(1, Ordering::Relaxed) >= self.cancel_at {
                self.cancelled.store(true, Ordering::Relaxed);
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

    #[test]
    fn tar_preserves_long_paths_and_real_payload_size() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source_root = temp.path().join("source-root");
        let first = "a".repeat(58);
        let second = "b".repeat(58);
        let nested = source_root.join(&first).join(&second);
        std::fs::create_dir_all(&nested).expect("create long path");
        let source = nested.join("payload.bin");
        let payload = vec![0x5au8; 1024 * 1024 + 17];
        std::fs::write(&source, &payload).expect("write payload");
        let archive = temp.path().join("long-path.tar");

        compress_tar(
            &TestRuntime::default(),
            None,
            "tar-long-path",
            &[source_root.to_string_lossy().to_string()],
            archive.to_string_lossy().as_ref(),
            CompressionOptions::default(),
        )
        .expect("create tar");

        let mut tar = tar::Archive::new(File::open(archive).expect("open tar"));
        let entry = tar
            .entries()
            .expect("entries")
            .filter_map(|entry| entry.ok())
            .find(|entry| entry.header().entry_type().is_file())
            .expect("payload entry");
        assert_eq!(
            entry.header().size().expect("entry size"),
            payload.len() as u64
        );
        assert_eq!(
            entry.path().expect("entry path").to_string_lossy(),
            format!("source-root/{first}/{second}/payload.bin")
        );
    }

    #[test]
    fn cancellation_is_checked_while_tar_reads_a_large_file() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("large.bin");
        let archive = temp.path().join("cancelled.tar");
        std::fs::write(&source, vec![0x41u8; 8 * 1024 * 1024]).expect("write source");
        let runtime = CancelDuringReadRuntime {
            checks: AtomicUsize::new(0),
            cancel_at: 3,
            cancelled: Arc::new(AtomicBool::new(false)),
        };

        let error = compress_tar(
            &runtime,
            None,
            "tar-cancel",
            &[source.to_string_lossy().to_string()],
            archive.to_string_lossy().as_ref(),
            CompressionOptions::default(),
        )
        .expect_err("tar compression must observe cancellation during file reads");
        assert!(matches!(
            error.downcast_ref::<CompressionError>(),
            Some(CompressionError::Cancelled)
        ));
        assert!(runtime.checks.load(Ordering::Relaxed) >= 4);
    }

    #[test]
    fn source_total_ignores_directories_and_counts_all_tar_payloads() {
        let temp = tempfile::tempdir().expect("temp dir");
        let folder = temp.path().join("payloads");
        std::fs::create_dir_all(folder.join("empty")).expect("create folders");
        std::fs::write(folder.join("alpha.bin"), vec![1u8; 8192]).expect("alpha");
        std::fs::write(folder.join("beta.bin"), vec![2u8; 4096]).expect("beta");
        let entries =
            compression_entries::collect(&[folder.to_string_lossy().to_string()], true, true)
                .expect("collect entries");
        assert_eq!(source_total_bytes(&entries).expect("total bytes"), 12_288);
    }
}
