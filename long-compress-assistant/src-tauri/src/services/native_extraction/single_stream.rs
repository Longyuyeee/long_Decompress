use super::ExtractionRuntime;
use crate::models::compression::{DecompressOptions, TaskLogSeverity};
use crate::services::compression_service::CompressionError;
use crate::services::extraction_transaction;
use anyhow::Result;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::Window;

const DISK_RESERVE_CHECK_INTERVAL: u64 = 16 * 1024 * 1024;
const PROGRESS_EMIT_INTERVAL: u64 = 4 * 1024 * 1024;

pub(crate) fn output_name(path: &Path, suffixes: &[&str]) -> String {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("output");
    let lower_name = file_name.to_lowercase();
    for suffix in suffixes {
        if lower_name.ends_with(suffix) && file_name.len() > suffix.len() {
            return file_name[..file_name.len() - suffix.len()].to_string();
        }
    }
    path.file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("output")
        .to_string()
}

pub(crate) fn extract<R: ExtractionRuntime, T: Read>(
    runtime: &R,
    window: &Window,
    task_id: &str,
    mut reader: T,
    output: &Path,
    output_name: String,
    options: &DecompressOptions,
) -> Result<()> {
    let relative = PathBuf::from(output_name);
    let file_filter = extraction_transaction::compile_file_filter(options.file_filter.as_deref());
    if !extraction_transaction::matches_compiled_file_filter(&relative, &file_filter) {
        runtime.emit_log(
            window,
            task_id,
            "Single-file archive skipped by current file filter.",
            TaskLogSeverity::Warning,
        );
        return Ok(());
    }

    let target = extraction_transaction::resolve_extract_path(&output.join(relative), options)?;
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut outfile = File::create(&target)?;
    let mut buffer = vec![0u8; runtime.copy_buffer_size()];
    let mut processed = 0u64;
    let mut last_emitted = 0u64;
    loop {
        runtime.check_cancellation()?;
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        processed = processed.checked_add(read as u64).ok_or_else(|| {
            CompressionError::ExtractionFailed(
                "Extracted stream size overflowed the supported range".to_string(),
            )
        })?;
        if processed > extraction_transaction::MAX_EXTRACTED_BYTES {
            return Err(CompressionError::ExtractionFailed(format!(
                "Extracted stream exceeds the safety limit ({} bytes)",
                extraction_transaction::MAX_EXTRACTED_BYTES
            ))
            .into());
        }
        if processed % DISK_RESERVE_CHECK_INTERVAL < read as u64 {
            extraction_transaction::validate_staging_disk_reserve(output)?;
        }
        outfile.write_all(&buffer[..read])?;
        if processed.saturating_sub(last_emitted) >= PROGRESS_EMIT_INTERVAL {
            last_emitted = processed;
            runtime.emit_progress(window, task_id, 0.5, None, processed, 0);
        }
    }
    outfile.flush()?;

    runtime.emit_log(
        window,
        task_id,
        &format!("Extracted single-file stream to {}", target.display()),
        TaskLogSeverity::Success,
    );
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
        decoder,
        output,
        output_name(Path::new(file), &[".gz"]),
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
        decoder,
        output,
        output_name(Path::new(file), &[".bz2"]),
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
        decoder,
        output,
        output_name(Path::new(file), &[".xz"]),
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
        decoder,
        output,
        output_name(Path::new(file), &[".zst", ".zstd"]),
        options,
    )
}

#[cfg(test)]
mod tests {
    use super::output_name;
    use std::path::Path;

    #[test]
    fn removes_known_suffix_case_insensitively() {
        assert_eq!(output_name(Path::new("backup.GZ"), &[".gz"]), "backup");
        assert_eq!(
            output_name(Path::new("database.ZSTD"), &[".zst", ".zstd"]),
            "database"
        );
    }

    #[test]
    fn preserves_inner_dots_and_has_a_safe_fallback() {
        assert_eq!(
            output_name(Path::new("release.bundle.xz"), &[".xz"]),
            "release.bundle"
        );
        assert_eq!(output_name(Path::new(".gz"), &[".gz"]), ".gz");
    }
}
