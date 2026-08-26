use crate::models::compression::{ArchiveBrowseResult, ArchiveEntryInfo};
use crate::services::archive_format::ArchiveFormat;
use crate::services::universal_engine::UniversalCliEngine;
use anyhow::{Context, Result};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const ARCHIVE_BROWSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

fn ensure_not_cancelled(cancelled: &AtomicBool) -> Result<()> {
    if cancelled.load(Ordering::Relaxed) {
        anyhow::bail!("ARCHIVE_BROWSE_CANCELLED");
    }
    Ok(())
}

fn entry_name(path: &str) -> String {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .to_string()
}

fn summarize(format: &str, entries: Vec<ArchiveEntryInfo>) -> ArchiveBrowseResult {
    ArchiveBrowseResult {
        format: format.to_string(),
        total_files: entries.iter().filter(|entry| !entry.is_dir).count(),
        total_directories: entries.iter().filter(|entry| entry.is_dir).count(),
        total_uncompressed_size: entries
            .iter()
            .filter(|entry| !entry.is_dir)
            .map(|entry| entry.size)
            .sum(),
        total_compressed_size: entries
            .iter()
            .filter_map(|entry| entry.compressed_size)
            .sum(),
        encrypted: entries.iter().any(|entry| entry.encrypted),
        entries,
    }
}

fn detect_format(path: &Path) -> Result<ArchiveFormat> {
    let mut file = File::open(path)?;
    let mut header = [0u8; 560];
    let read = file.read(&mut header)?;
    let magic = ArchiveFormat::from_magic(&header[..read]);
    if magic != ArchiveFormat::Unknown {
        return Ok(magic);
    }
    Ok(path
        .extension()
        .and_then(|value| value.to_str())
        .map(ArchiveFormat::from_extension)
        .unwrap_or(ArchiveFormat::Unknown))
}

fn browse_zip(path: &Path, cancelled: &AtomicBool) -> Result<ArchiveBrowseResult> {
    let mut archive = zip_aes::ZipArchive::new(File::open(path)?)?;
    let mut entries = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        ensure_not_cancelled(cancelled)?;
        let entry = archive.by_index_raw(index)?;
        let modified = entry.last_modified().map(|date| {
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                date.year(),
                date.month(),
                date.day(),
                date.hour(),
                date.minute(),
                date.second()
            )
        });
        let entry_path = entry.name().replace('\\', "/");
        entries.push(ArchiveEntryInfo {
            name: entry_name(&entry_path),
            path: entry_path,
            size: entry.size(),
            compressed_size: Some(entry.compressed_size()),
            modified,
            crc: Some(format!("{:08X}", entry.crc32())),
            encrypted: entry.encrypted(),
            is_dir: entry.is_dir(),
        });
    }
    Ok(summarize("ZIP", entries))
}

fn browse_7z(
    path: &Path,
    password: Option<&str>,
    cancelled: &AtomicBool,
) -> Result<ArchiveBrowseResult> {
    ensure_not_cancelled(cancelled)?;
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    let password = password.map(sevenz_rust::Password::from);
    let archive = sevenz_rust::Archive::read(
        &mut file,
        length,
        password
            .as_ref()
            .map(|value| value.as_slice())
            .unwrap_or(&[]),
    )
    .map_err(|error| anyhow::anyhow!("Unable to read 7Z metadata: {error}"))?;
    let encrypted = archive.folders.iter().any(|folder| {
        folder.coders.iter().any(|coder| {
            coder.decompression_method_id() == sevenz_rust::SevenZMethod::ID_AES256SHA256
        })
    });
    ensure_not_cancelled(cancelled)?;
    let entries = archive
        .files
        .into_iter()
        .map(|entry| {
            ensure_not_cancelled(cancelled)?;
            let path = entry.name.replace('\\', "/");
            let modified = entry.has_last_modified_date.then(|| {
                let value: std::time::SystemTime = entry.last_modified_date.into();
                chrono::DateTime::<chrono::Utc>::from(value).to_rfc3339()
            });
            Ok(ArchiveEntryInfo {
                name: entry_name(&path),
                path,
                size: entry.size,
                compressed_size: Some(entry.compressed_size),
                modified,
                crc: entry.has_crc.then(|| format!("{:08X}", entry.crc)),
                encrypted,
                is_dir: entry.is_directory,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(summarize("7Z", entries))
}

fn browse_rar(
    path: &Path,
    password: Option<&str>,
    cancelled: &AtomicBool,
) -> Result<ArchiveBrowseResult> {
    let archive = match password {
        Some(password) => unrar::Archive::with_password(path, password),
        None => unrar::Archive::new(path),
    };
    let entries = archive
        .open_for_listing()?
        .map(|entry| -> Result<ArchiveEntryInfo> {
            ensure_not_cancelled(cancelled)?;
            let entry = entry?;
            let path = entry.filename.to_string_lossy().replace('\\', "/");
            Ok(ArchiveEntryInfo {
                name: entry_name(&path),
                path,
                size: entry.unpacked_size,
                compressed_size: None,
                modified: None,
                crc: Some(format!("{:08X}", entry.file_crc)),
                encrypted: entry.is_encrypted(),
                is_dir: entry.is_directory(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(summarize("RAR", entries))
}

fn browse_tar_reader<R: Read>(
    reader: R,
    format: &str,
    cancelled: &AtomicBool,
) -> Result<ArchiveBrowseResult> {
    let mut archive = tar::Archive::new(reader);
    let mut entries = Vec::new();
    for entry in archive.entries()? {
        ensure_not_cancelled(cancelled)?;
        let entry = entry?;
        let path = entry.path()?.to_string_lossy().replace('\\', "/");
        let header = entry.header();
        let is_dir = header.entry_type().is_dir();
        let modified = header.mtime().ok().and_then(|seconds| {
            chrono::DateTime::<chrono::Utc>::from_timestamp(seconds as i64, 0)
                .map(|value| value.to_rfc3339())
        });
        entries.push(ArchiveEntryInfo {
            name: entry_name(&path),
            path,
            size: if is_dir { 0 } else { header.size().unwrap_or(0) },
            compressed_size: None,
            modified,
            crc: None,
            encrypted: false,
            is_dir,
        });
    }
    Ok(summarize(format, entries))
}

fn browse_tar_family(
    path: &Path,
    cancelled: &AtomicBool,
) -> Result<Option<ArchiveBrowseResult>> {
    let name = path.file_name().and_then(|value| value.to_str()).unwrap_or("").to_ascii_lowercase();
    if name.ends_with(".tar") || name.ends_with(".ova") {
        return browse_tar_reader(File::open(path)?, "TAR", cancelled).map(Some);
    }
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") || name.ends_with(".tpz") {
        return browse_tar_reader(
            flate2::read::GzDecoder::new(File::open(path)?),
            "TAR.GZ",
            cancelled,
        )
        .map(Some);
    }
    if name.ends_with(".tar.bz2") || name.ends_with(".tbz") || name.ends_with(".tbz2") {
        return browse_tar_reader(
            bzip2::read::BzDecoder::new(File::open(path)?),
            "TAR.BZ2",
            cancelled,
        )
        .map(Some);
    }
    if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        return browse_tar_reader(
            xz2::read::XzDecoder::new(File::open(path)?),
            "TAR.XZ",
            cancelled,
        )
        .map(Some);
    }
    if name.ends_with(".tar.zst") || name.ends_with(".tzst") {
        return browse_tar_reader(
            zstd::stream::read::Decoder::new(File::open(path)?)?,
            "TAR.ZST",
            cancelled,
        )
        .map(Some);
    }
    Ok(None)
}

async fn wait_for_cancel(cancelled: Arc<AtomicBool>) {
    while !cancelled.load(Ordering::Relaxed) {
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
}

pub async fn browse_archive_cancellable(
    path: &Path,
    password: Option<&str>,
    cancelled: Arc<AtomicBool>,
) -> Result<ArchiveBrowseResult> {
    if !path.is_file() {
        anyhow::bail!(
            "Archive does not exist or is not a file: {}",
            path.display()
        );
    }
    ensure_not_cancelled(&cancelled)?;
    let path = path.to_path_buf();
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let is_tar_family = [
        ".tar", ".ova", ".tar.gz", ".tgz", ".tpz", ".tar.bz2", ".tbz", ".tbz2",
        ".tar.xz", ".txz", ".tar.zst", ".tzst",
    ]
    .iter()
    .any(|suffix| name.ends_with(suffix));
    let format = detect_format(&path)?;
    let password = password.map(str::to_string);
    let operation_cancelled = cancelled.clone();
    let operation = async move {
        if is_tar_family
            || matches!(
                format,
                ArchiveFormat::Zip | ArchiveFormat::SevenZip | ArchiveFormat::Rar
            )
        {
            let native_cancelled = operation_cancelled.clone();
            return tauri::async_runtime::spawn_blocking(move || {
                if let Some(result) = browse_tar_family(&path, &native_cancelled)? {
                    return Ok(result);
                }
                match format {
                    ArchiveFormat::Zip => browse_zip(&path, &native_cancelled)
                        .context("Unable to read ZIP metadata"),
                    ArchiveFormat::SevenZip => {
                        browse_7z(&path, password.as_deref(), &native_cancelled)
                    }
                    ArchiveFormat::Rar => browse_rar(
                        &path,
                        password.as_deref(),
                        &native_cancelled,
                    )
                    .context("Unable to read RAR metadata"),
                    _ => anyhow::bail!("Unsupported native archive route"),
                }
            })
            .await
            .map_err(|error| anyhow::anyhow!("Archive metadata worker failed: {error}"))?;
        }
        if password.is_some() {
            anyhow::bail!("This archive format cannot be browsed with a password without exposing it to a command line");
        }
        UniversalCliEngine::list_metadata_cancellable(
            &path,
            format!("{format:?}"),
            operation_cancelled,
        )
        .await
    };

    tokio::select! {
        result = operation => result,
        _ = wait_for_cancel(cancelled.clone()) => anyhow::bail!("ARCHIVE_BROWSE_CANCELLED"),
        _ = tokio::time::sleep(ARCHIVE_BROWSE_TIMEOUT) => {
            cancelled.store(true, Ordering::Relaxed);
            anyhow::bail!("ARCHIVE_BROWSE_TIMEOUT");
        }
    }
}

pub async fn browse_archive(path: &Path, password: Option<&str>) -> Result<ArchiveBrowseResult> {
    browse_archive_cancellable(path, password, Arc::new(AtomicBool::new(false))).await
}

#[cfg(test)]
mod tests {
    use super::{browse_archive, browse_archive_cancellable};
    use std::io::Write;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    #[tokio::test]
    async fn browses_real_zip_metadata_without_extracting() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("browse.zip");
        let file = std::fs::File::create(&path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        writer.add_directory("docs/", zip::write::FileOptions::default()).unwrap();
        writer.start_file("docs/readme.txt", zip::write::FileOptions::default()).unwrap();
        writer.write_all(b"archive browser").unwrap();
        writer.finish().unwrap();

        let result = browse_archive(&path, None).await.unwrap();
        assert_eq!(result.format, "ZIP");
        assert_eq!(result.total_files, 1);
        assert_eq!(result.total_directories, 1);
        assert_eq!(result.total_uncompressed_size, 15);
        let file = result.entries.iter().find(|entry| entry.path == "docs/readme.txt").unwrap();
        assert_eq!(file.name, "readme.txt");
        assert_eq!(file.crc.as_deref().map(str::len), Some(8));
        assert!(!file.encrypted);
    }

    #[tokio::test]
    async fn cancelled_real_archive_read_returns_without_parsing_entries() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("cancelled.zip");
        let file = std::fs::File::create(&path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        writer
            .start_file("payload.txt", zip::write::FileOptions::default())
            .unwrap();
        writer.write_all(b"real archive cancellation fixture").unwrap();
        writer.finish().unwrap();

        let cancelled = Arc::new(AtomicBool::new(true));
        let error = browse_archive_cancellable(&path, None, cancelled)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("ARCHIVE_BROWSE_CANCELLED"));
    }
}
