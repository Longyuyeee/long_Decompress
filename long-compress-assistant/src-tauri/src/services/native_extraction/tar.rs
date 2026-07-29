use super::ExtractionRuntime;
use crate::models::compression::{DecompressOptions, TaskLogSeverity};
use crate::services::extraction_transaction;
use anyhow::Result;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tauri::Window;

pub(crate) fn extract<R: ExtractionRuntime>(
    runtime: &R,
    window: &Window,
    task_id: &str,
    file: &str,
    output: &Path,
    decoder: Option<Box<dyn Read + Send>>,
    options: &DecompressOptions,
) -> Result<()> {
    let reader: Box<dyn Read + Send> = match decoder {
        Some(decoder) => decoder,
        None => Box::new(File::open(file)?),
    };
    let file_filter = extraction_transaction::compile_file_filter(options.file_filter.as_deref());
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
                runtime.emit_log(
                    window,
                    task_id,
                    "Skipped non-regular TAR entry (links and device entries are not extracted)",
                    TaskLogSeverity::Warning,
                );
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
            if options.skip_corrupted {
                runtime.emit_log(
                    window,
                    task_id,
                    &format!("Skipped tar entry: {error}"),
                    TaskLogSeverity::Warning,
                );
                continue;
            }
            return Err(error);
        }
    }
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
    let decoder = flate2::read::GzDecoder::new(File::open(file)?);
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
    let decoder = bzip2::read::BzDecoder::new(File::open(file)?);
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
    let decoder = xz2::read::XzDecoder::new(File::open(file)?);
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
    let decoder = zstd::stream::read::Decoder::new(File::open(file)?)?;
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
