use super::ExtractionRuntime;
use crate::models::compression::{DecompressOptions, TaskLogSeverity};
use crate::services::extraction_transaction;
use anyhow::Result;
use std::io::Read;
use std::path::Path;
use tauri::Window;

struct CheckedReader<'a, R: ExtractionRuntime> {
    runtime: &'a R, reader: Box<dyn Read + Send>, output: &'a Path, read: u64,
}
impl<R: ExtractionRuntime> Read for CheckedReader<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.runtime.check_cancellation().map_err(|error| std::io::Error::other(error.to_string()))?;
        let count = self.reader.read(buffer)?;
        self.read = self.read.checked_add(count as u64).ok_or_else(|| std::io::Error::other("TAR size overflow"))?;
        if self.read > extraction_transaction::MAX_EXTRACTED_BYTES { return Err(std::io::Error::other("TAR 展开体积超出支持范围")); }
        if self.read % (16 * 1024 * 1024) < count as u64 {
            extraction_transaction::validate_staging_disk_reserve(self.output).map_err(|error| std::io::Error::other(error.to_string()))?;
        }
        Ok(count)
    }
}

pub(crate) fn extract<R: ExtractionRuntime>(
    runtime: &R,
    window: &Window,
    task_id: &str,
    file: &str,
    output: &Path,
    decoder: Option<Box<dyn Read + Send>>,
    options: &DecompressOptions,
) -> Result<()> {
    extract_inner(runtime, Some(window), task_id, file, output, decoder, options)
}

pub(crate) fn extract_inner<R: ExtractionRuntime>(runtime: &R, window: Option<&Window>, task_id: &str, file: &str, output: &Path, decoder: Option<Box<dyn Read + Send>>, options: &DecompressOptions) -> Result<()> {
    let reader: Box<dyn Read + Send> = match decoder {
        Some(decoder) => decoder,
        None => Box::new(crate::services::numeric_volume_reader::ZipInputReader::open(Path::new(file))?),
    };
    let file_filter = extraction_transaction::compile_file_filter(options.file_filter.as_deref());
    let reader = CheckedReader { runtime, reader, output, read: 0 };
    let mut archive = tar::Archive::new(reader);
    let mut expanded_bytes = 0u64;
    let mut buffer = vec![0u8; runtime.copy_buffer_size().clamp(64 * 1024, 1024 * 1024)];

    for entry in archive.entries()? {
        runtime.check_cancellation()?;
        let entry_result = (|| -> Result<()> {
            let mut entry = crate::services::tar_compatibility::LogicalEntry::new(entry?)?;
            if !extraction_transaction::matches_selected_entries(entry.path(), &options.selected_entries) {
                return Ok(());
            }
            let relative =
                match runtime.normalized_archive_path(entry.path(), options.preserve_paths) {
                    Some(path) => path,
                    None => return Ok(()),
                };
            if !extraction_transaction::matches_compiled_file_filter(&relative, &file_filter) {
                return Ok(());
            }

            if entry.header().entry_type().is_dir() {
                std::fs::create_dir_all(output.join(relative))?;
                return Ok(());
            }
            entry.require_file_or_dir()?;
            expanded_bytes = expanded_bytes.checked_add(entry.size()).ok_or_else(|| anyhow::anyhow!("TAR 逻辑展开大小溢出"))?;
            anyhow::ensure!(expanded_bytes <= extraction_transaction::MAX_EXTRACTED_BYTES,
                "[archive-inspection:unsupported] TAR 逻辑展开体积超出支持范围");

            let target =
                extraction_transaction::resolve_extract_path(&output.join(relative), options)?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let modified = entry.header().mtime().ok();
            let mut writer = std::fs::File::create(&target)?;
            let mut copied = 0u64;
            loop {
                runtime.check_cancellation()?;
                let count = entry.read(&mut buffer)?;
                if count == 0 { break; }
                runtime.write_extracted_chunk(&mut writer, &buffer[..count])?;
                copied += count as u64;
                if copied % (16 * 1024 * 1024) < count as u64 {
                    extraction_transaction::validate_staging_disk_reserve(output)?;
                }
            }
            if options.preserve_timestamps {
                if let Some(time) = modified.and_then(|seconds| std::time::UNIX_EPOCH.checked_add(std::time::Duration::from_secs(seconds))) {
                    writer.set_modified(time)?;
                }
            }
            Ok(())
        })();

        if let Err(error) = entry_result {
            // Preserve cancellation even when tar adds an unpack context.
            runtime.check_cancellation()?;
            // Explicit tolerance may skip corrupt data, never unsupported
            // entries or output/read I/O failures and call them full success.
            let data_corruption = error.chain().filter_map(|cause|cause.downcast_ref::<std::io::Error>())
                .any(|e|matches!(e.kind(),std::io::ErrorKind::InvalidData|std::io::ErrorKind::UnexpectedEof));
            if options.skip_corrupted && data_corruption {
                if let Some(window) = window { runtime.emit_log(
                    window,
                    task_id,
                    &format!("Skipped tar entry: {error}"),
                    TaskLogSeverity::Warning,
                ); }
                continue;
            }
            return Err(error);
        }
    }
    // TAR's zero blocks are not the end of a compressed stream: consume its
    // footer so truncation/checksum failures cannot publish partial success.
    crate::services::tar_compatibility::finish_stream(archive.into_inner(), || runtime.check_cancellation())
        .map_err(crate::services::tar_compatibility::inspection_error)?;
    Ok(())
}

pub(crate) fn extract_gzip<R: ExtractionRuntime>(
    runtime: &R,
    window: &Window,
    task_id: &str,
    file: &str,
    output: &Path,
    options: &DecompressOptions,
) -> Result<()> {
    let decoder = crate::services::codec_stream::decoder(Path::new(file), "gz")?;
    extract(
        runtime,
        window,
        task_id,
        file,
        output,
        Some(decoder),
        options,
    )
}

pub(crate) fn extract_bzip2<R: ExtractionRuntime>(
    runtime: &R,
    window: &Window,
    task_id: &str,
    file: &str,
    output: &Path,
    options: &DecompressOptions,
) -> Result<()> {
    let decoder = crate::services::codec_stream::decoder(Path::new(file), "bz2")?;
    extract(
        runtime,
        window,
        task_id,
        file,
        output,
        Some(decoder),
        options,
    )
}

pub(crate) fn extract_xz<R: ExtractionRuntime>(
    runtime: &R,
    window: &Window,
    task_id: &str,
    file: &str,
    output: &Path,
    options: &DecompressOptions,
) -> Result<()> {
    let decoder = crate::services::codec_stream::decoder(Path::new(file), "xz")?;
    extract(
        runtime,
        window,
        task_id,
        file,
        output,
        Some(decoder),
        options,
    )
}

pub(crate) fn extract_zstandard<R: ExtractionRuntime>(
    runtime: &R,
    window: &Window,
    task_id: &str,
    file: &str,
    output: &Path,
    options: &DecompressOptions,
) -> Result<()> {
    let decoder = crate::services::codec_stream::decoder(Path::new(file), "zst")?;
    extract(
        runtime,
        window,
        task_id,
        file,
        output,
        Some(decoder),
        options,
    )
}
