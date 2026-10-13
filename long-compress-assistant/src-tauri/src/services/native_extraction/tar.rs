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

    for entry in archive.entries()? {
        runtime.check_cancellation()?;
        let entry_result = (|| -> Result<()> {
            let mut entry = entry?;
            let relative =
                match runtime.normalized_archive_path(&entry.path()?, options.preserve_paths) {
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
            if !entry.header().entry_type().is_file() {
                if let Some(window) = window { runtime.emit_log(
                    window,
                    task_id,
                    "Skipped non-regular TAR entry (links and device entries are not extracted)",
                    TaskLogSeverity::Warning,
                ); }
                return Ok(());
            }

            let target =
                extraction_transaction::resolve_extract_path(&output.join(relative), options)?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            entry.unpack(&target)?;
            Ok(())
        })();

        if let Err(error) = entry_result {
            // Preserve cancellation even when tar adds an unpack context.
            runtime.check_cancellation()?;
            if options.skip_corrupted {
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
    let mut tail = archive.into_inner();
    let mut buffer = [0u8; 64 * 1024];
    while tail.read(&mut buffer)? > 0 {}
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
    let decoder = flate2::read::GzDecoder::new(crate::services::numeric_volume_reader::ZipInputReader::open(Path::new(file))?);
    extract(
        runtime,
        window,
        task_id,
        file,
        output,
        Some(Box::new(decoder)),
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
    let decoder = bzip2::read::BzDecoder::new(crate::services::numeric_volume_reader::ZipInputReader::open(Path::new(file))?);
    extract(
        runtime,
        window,
        task_id,
        file,
        output,
        Some(Box::new(decoder)),
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
    let decoder = xz2::read::XzDecoder::new(crate::services::numeric_volume_reader::ZipInputReader::open(Path::new(file))?);
    extract(
        runtime,
        window,
        task_id,
        file,
        output,
        Some(Box::new(decoder)),
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
    let decoder = zstd::stream::read::Decoder::new(crate::services::numeric_volume_reader::ZipInputReader::open(Path::new(file))?)?;
    extract(
        runtime,
        window,
        task_id,
        file,
        output,
        Some(Box::new(decoder)),
        options,
    )
}
