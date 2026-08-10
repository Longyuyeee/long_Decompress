use crate::models::compression::{ArchiveBrowseResult, ArchiveEntryInfo};
use crate::services::archive_format::ArchiveFormat;
use crate::services::universal_engine::UniversalCliEngine;
use anyhow::{Context, Result};
use std::fs::File;
use std::io::Read;
use std::path::Path;

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

fn browse_zip(path: &Path) -> Result<ArchiveBrowseResult> {
    let mut archive = zip_aes::ZipArchive::new(File::open(path)?)?;
    let mut entries = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
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

fn browse_7z(path: &Path, password: Option<&str>) -> Result<ArchiveBrowseResult> {
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
    let entries = archive
        .files
        .into_iter()
        .map(|entry| {
            let path = entry.name.replace('\\', "/");
            let modified = entry.has_last_modified_date.then(|| {
                let value: std::time::SystemTime = entry.last_modified_date.into();
                chrono::DateTime::<chrono::Utc>::from(value).to_rfc3339()
            });
            ArchiveEntryInfo {
                name: entry_name(&path),
                path,
                size: entry.size,
                compressed_size: Some(entry.compressed_size),
                modified,
                crc: entry.has_crc.then(|| format!("{:08X}", entry.crc)),
                encrypted,
                is_dir: entry.is_directory,
            }
        })
        .collect();
    Ok(summarize("7Z", entries))
}

fn browse_rar(path: &Path, password: Option<&str>) -> Result<ArchiveBrowseResult> {
    let archive = match password {
        Some(password) => unrar::Archive::with_password(path, password),
        None => unrar::Archive::new(path),
    };
    let entries = archive
        .open_for_listing()?
        .map(|entry| {
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
        .collect::<std::result::Result<Vec<_>, unrar::error::UnrarError>>()?;
    Ok(summarize("RAR", entries))
}

fn browse_tar_reader<R: Read>(reader: R, format: &str) -> Result<ArchiveBrowseResult> {
    let mut archive = tar::Archive::new(reader);
    let mut entries = Vec::new();
    for entry in archive.entries()? {
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

fn browse_tar_family(path: &Path) -> Result<Option<ArchiveBrowseResult>> {
    let name = path.file_name().and_then(|value| value.to_str()).unwrap_or("").to_ascii_lowercase();
    if name.ends_with(".tar") || name.ends_with(".ova") {
        return browse_tar_reader(File::open(path)?, "TAR").map(Some);
    }
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") || name.ends_with(".tpz") {
        return browse_tar_reader(flate2::read::GzDecoder::new(File::open(path)?), "TAR.GZ").map(Some);
    }
    if name.ends_with(".tar.bz2") || name.ends_with(".tbz") || name.ends_with(".tbz2") {
        return browse_tar_reader(bzip2::read::BzDecoder::new(File::open(path)?), "TAR.BZ2").map(Some);
    }
    if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        return browse_tar_reader(xz2::read::XzDecoder::new(File::open(path)?), "TAR.XZ").map(Some);
    }
    if name.ends_with(".tar.zst") || name.ends_with(".tzst") {
        return browse_tar_reader(zstd::stream::read::Decoder::new(File::open(path)?)?, "TAR.ZST").map(Some);
    }
    Ok(None)
}

pub async fn browse_archive(path: &Path, password: Option<&str>) -> Result<ArchiveBrowseResult> {
    if !path.is_file() {
        anyhow::bail!(
            "Archive does not exist or is not a file: {}",
            path.display()
        );
    }
    if let Some(result) = browse_tar_family(path)? {
        return Ok(result);
    }
    match detect_format(path)? {
        ArchiveFormat::Zip => browse_zip(path).context("Unable to read ZIP metadata"),
        ArchiveFormat::SevenZip => browse_7z(path, password),
        ArchiveFormat::Rar => browse_rar(path, password).context("Unable to read RAR metadata"),
        format => {
            if password.is_some() {
                anyhow::bail!("This archive format cannot be browsed with a password without exposing it to a command line");
            }
            UniversalCliEngine::list_metadata(path, format!("{format:?}")).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::browse_archive;
    use std::io::Write;

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
}
