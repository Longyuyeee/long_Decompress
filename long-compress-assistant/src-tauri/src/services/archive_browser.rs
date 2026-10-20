use crate::models::compression::{ArchiveBrowseResult, ArchiveEntryInfo};
use crate::services::archive_format::ArchiveFormat;
use crate::services::universal_engine::UniversalCliEngine;
use anyhow::{Context, Result};

use std::io::{Read, Seek, SeekFrom};
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
        zpaq_history: None,
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

pub(crate) fn detect_format(path: &Path) -> Result<ArchiveFormat> {
    let source = super::split_archive_detector::SplitArchiveDetector::complete_source(path)?;
    if source != path { return detect_format(&source); }
    if super::egg_volumes::is_candidate(path) { super::egg_volumes::complete(path)?; return Ok(ArchiveFormat::Egg); }
    if super::alz_volumes::is_candidate(path) { super::alz_volumes::complete(path)?; return Ok(ArchiveFormat::Alz); }
    if super::sfx_archive::is_executable_path(path) {
        if let Some((format, _)) = super::sfx_archive::detect(path)? { return Ok(format); }
        anyhow::bail!("[archive-inspection:unsupported] 不是可读取的 ZIP、7Z 或 RAR 自解压归档，或归档头已损坏；不会执行此程序");
    }
    let file = super::numeric_volume_reader::ZipInputReader::open(path)?;
    if matches!(&file, super::numeric_volume_reader::ZipInputReader::StandardZip(_)) {
        return Ok(ArchiveFormat::Zip);
    }
    let mut header = [0u8; 560];
    let mut bytes = Vec::with_capacity(header.len());
    file.take(header.len() as u64).read_to_end(&mut bytes)?;
    let read = bytes.len();
    header[..read].copy_from_slice(&bytes);
    let magic = ArchiveFormat::from_magic(&header[..read]);
    if magic != ArchiveFormat::Unknown {
        return Ok(magic);
    }
    let logical = if super::numeric_volume_reader::ZipInputReader::is_numeric_first(path) { Path::new(path.file_stem().unwrap()) } else { path };
    Ok(logical
        .extension()
        .and_then(|value| value.to_str())
        .map(ArchiveFormat::from_extension)
        .unwrap_or(ArchiveFormat::Unknown))
}

fn browse_zip(path: &Path, cancelled: &AtomicBool) -> Result<ArchiveBrowseResult> {
    let mut archive = zip_aes::ZipArchive::new(super::numeric_volume_reader::ZipInputReader::open(path)?)?;
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
        entries.push(ArchiveEntryInfo { size_known: None,
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
    let mut file = super::numeric_volume_reader::ZipInputReader::open_seven_zip(path)?;
    let length = file.seek(SeekFrom::End(0))?;
    // Explicitly supplied passwords bypass password resolution. Apply the same
    // CRC/extent preflight here so truncated volumes cannot become password errors.
    super::native_extraction::seven_zip::validate_archive_extent(&mut file, length)?;
    file.seek(SeekFrom::Start(0))?;
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
            Ok(ArchiveEntryInfo { size_known: None,
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
    let archive = archive.open_for_listing()?;
    super::sfx_volumes::validate_native_volume(path, !matches!(archive.volume_info(), unrar::VolumeInfo::None))?;
    let encrypted_headers = archive.has_encrypted_headers();
    let password_proven = std::cell::Cell::new(false);
    let entries = archive
        .map(|entry| -> Result<ArchiveEntryInfo> {
            ensure_not_cancelled(cancelled)?;
            let entry = entry.map_err(|error| {
                if let Some(message) = super::rar_support::RarSupportService::native_failure_message(&error, password_proven.get()) { anyhow::anyhow!(message) }
                else { anyhow::Error::new(error) }
            })?;
            password_proven.set(password_proven.get() || encrypted_headers);
            let path = entry.filename.to_string_lossy().replace('\\', "/");
            Ok(ArchiveEntryInfo { size_known: None,
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
        let entry = super::tar_compatibility::LogicalEntry::new(entry?)?;
        if entry.path().as_os_str().is_empty() && entry.header().entry_type().is_dir() { continue; }
        let path = entry.path().to_string_lossy().replace('\\', "/");
        let header = entry.header();
        let is_dir = header.entry_type().is_dir();
        let modified = header.mtime().ok().and_then(|seconds| {
            chrono::DateTime::<chrono::Utc>::from_timestamp(seconds as i64, 0)
                .map(|value| value.to_rfc3339())
        });
        entries.push(ArchiveEntryInfo { size_known: None,
            name: entry_name(&path),
            path,
            size: if is_dir { 0 } else { entry.size() },
            compressed_size: None,
            modified,
            crc: None,
            encrypted: false,
            is_dir,
        });
    }
    // TAR stops at zero blocks; read the enclosing stream's trailer as well.
    super::tar_compatibility::finish_stream(archive.into_inner(), || ensure_not_cancelled(cancelled))?;
    Ok(summarize(format, entries))
}

fn browse_tar_family(
    path: &Path,
    cancelled: &AtomicBool,
) -> Result<Option<ArchiveBrowseResult>> {
    let logical = if super::numeric_volume_reader::ZipInputReader::is_numeric_first(path) { Path::new(path.file_stem().unwrap()) } else { path };
    let name = logical.file_name().and_then(|value| value.to_str()).unwrap_or("").to_ascii_lowercase();
    if name.ends_with(".tar") || name.ends_with(".ova") {
        return browse_tar_reader(super::numeric_volume_reader::ZipInputReader::open(path)?, "TAR", cancelled).map(Some);
    }
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") || name.ends_with(".tpz") {
        return browse_tar_reader(
            crate::services::codec_stream::decoder(path, "gz")?,
            "TAR.GZ",
            cancelled,
        )
        .map(Some);
    }
    if name.ends_with(".tar.bz2") || name.ends_with(".tbz") || name.ends_with(".tbz2") {
        return browse_tar_reader(
            crate::services::codec_stream::decoder(path, "bz2")?,
            "TAR.BZ2",
            cancelled,
        )
        .map(Some);
    }
    if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        return browse_tar_reader(
            crate::services::codec_stream::decoder(path, "xz")?,
            "TAR.XZ",
            cancelled,
        )
        .map(Some);
    }
    if name.ends_with(".tar.zst") || name.ends_with(".tzst") {
        return browse_tar_reader(
            crate::services::codec_stream::decoder(path, "zst")?,
            "TAR.ZST",
            cancelled,
        )
        .map(Some);
    }
    for codec in ["lz4", "br", "lz", "lzma"] {
        if super::compression_format::is_tar_wrapped_archive(path, &format!("tar.{codec}")) {
            let label = if codec == "lz" { "TAR.LZIP".to_string() } else { format!("TAR.{}", codec.to_uppercase()) };
            return browse_tar_reader(super::codec_stream::decoder(path, codec)?, &label, cancelled).map(Some);
        }
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
    browse_archive_cancellable_with_encoding(path, password, cancelled, None).await
}

pub async fn browse_archive_cancellable_with_encoding(
    path: &Path, password: Option<&str>, cancelled: Arc<AtomicBool>,
    encoding: Option<super::zip_password::ZipPasswordEncoding>,
) -> Result<ArchiveBrowseResult> {
    browse_archive_cancellable_at(path, password, cancelled, encoding, None).await
}

pub async fn browse_archive_cancellable_at(
    path: &Path, password: Option<&str>, cancelled: Arc<AtomicBool>,
    encoding: Option<super::zip_password::ZipPasswordEncoding>,
    snapshot: Option<crate::models::compression::ZpaqSnapshot>,
) -> Result<ArchiveBrowseResult> {
    if !path.is_file() {
        anyhow::bail!(
            "Archive does not exist or is not a file: {}",
            path.display()
        );
    }
    ensure_not_cancelled(&cancelled)?;
    let path = super::split_archive_detector::SplitArchiveDetector::complete_source(path)?;
    let format = detect_format(&path)?;
    let is_tar_family = super::tar_compatibility::integrity_route(&path, &format).is_some();
    let password = password.map(str::to_string);
    let operation_cancelled = cancelled.clone();
    let operation = async move {
        if let Some(snapshot) = snapshot {
            anyhow::ensure!(format == ArchiveFormat::Zpaq, "所选快照只适用于 ZPAQ 日志归档，请重新选择来源");
            return super::zpaq_engine::browse_at(&path, password.as_deref(), Some(&snapshot), operation_cancelled).await;
        }
        if format == ArchiveFormat::AesEncrypted {
            let aes_path = path.clone();
            let aes_password = password.clone();
            let aes_cancelled = operation_cancelled.clone();
            let inner = tauri::async_runtime::spawn_blocking(move ||
                super::aes_container::AuthenticatedInner::open(&aes_path, aes_password.as_deref(), || ensure_not_cancelled(&aes_cancelled))
            ).await??;
            let mut result = Box::pin(browse_archive_cancellable_with_encoding(
                &inner.path, None, operation_cancelled, encoding)).await?;
            result.format = format!("LONG AES.{}", result.format);
            result.encrypted = true;
            for entry in &mut result.entries { entry.encrypted = true; }
            result.total_compressed_size = super::split_archive_detector::SplitArchiveDetector::detect_split_archive(&path)?
                .map(|group| group.total_size).unwrap_or(std::fs::metadata(&path)?.len());
            return Ok(result);
        }
        if format == ArchiveFormat::Alz {
            return super::alz_engine::browse_with_encoding(&path, encoding, operation_cancelled).await;
        }
        if format == ArchiveFormat::Egg {
            return super::egg_engine::browse(&path, operation_cancelled).await;
        }
        if format == ArchiveFormat::Zpaq {
            return super::zpaq_engine::browse(&path, password.as_deref(), operation_cancelled).await;
        }
        if is_tar_family || super::codec_stream::plain_format(&format).is_some()
            || matches!(
                format,
                ArchiveFormat::Zip | ArchiveFormat::SevenZip | ArchiveFormat::Rar | ArchiveFormat::Tar | ArchiveFormat::Lz4 | ArchiveFormat::Brotli | ArchiveFormat::Lzip
            )
        {
            let native_cancelled = operation_cancelled.clone();
            return tauri::async_runtime::spawn_blocking(move || {
                if is_tar_family && format != ArchiveFormat::Tar {
                    if let Some(result) = browse_tar_family(&path, &native_cancelled)? {
                        return Ok(result);
                    }
                }
                if let Some((codec, label, suffixes)) = super::codec_stream::plain_format(&format) {
                    let mut reader = super::codec_stream::decoder(&path, codec)?;
                    let mut buffer = [0u8; 64 * 1024];
                    let mut size = 0u64;
                    let mut size_known = true;
                    loop {
                        ensure_not_cancelled(&native_cancelled)?;
                        let count = reader.read(&mut buffer)?;
                        if count == 0 { break; }
                        size = size.checked_add(count as u64).context("Stream size overflow")?;
                        // Listing one large compressed file must not require full decoding.
                        if size > 8 * 1024 * 1024 { size = 0; size_known = false; break; }
                    }
                    let name = super::native_extraction::single_stream::output_name(&path, suffixes);
                    let compressed_size = super::split_archive_detector::SplitArchiveDetector::detect_split_archive(&path)?
                        .map(|group| group.total_size).unwrap_or(std::fs::metadata(&path)?.len());
                    return Ok(summarize(label, vec![ArchiveEntryInfo { size_known: Some(size_known), name: name.clone(), path: name,
                        size, compressed_size: Some(compressed_size), modified: None, crc: None, encrypted: false, is_dir: false }]));
                }
                match format {
                    ArchiveFormat::Tar => browse_tar_reader(super::numeric_volume_reader::ZipInputReader::open(&path)?, "TAR", &native_cancelled),
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
                    ArchiveFormat::Lz4 | ArchiveFormat::Brotli | ArchiveFormat::Lzip => {
                        let codec = match format { ArchiveFormat::Lz4 => "lz4", ArchiveFormat::Lzip => "lz", _ => "br" };
                        let mut reader = super::codec_stream::decoder(&path, codec)?;
                        let mut buffer = [0u8; 64 * 1024];
                        let mut size = 0u64;
                        loop {
                            ensure_not_cancelled(&native_cancelled)?;
                            let count = reader.read(&mut buffer)?;
                            if count == 0 { break; }
                            size = size.checked_add(count as u64).context("Stream size overflow")?;
                            anyhow::ensure!(size <= super::extraction_transaction::MAX_EXTRACTED_BYTES, "Stream is too large");
                        }
                        let name = super::native_extraction::single_stream::output_name(&path, &[&format!(".{codec}")]);
                        let label = if codec == "lz" { "LZIP".to_string() } else { codec.to_uppercase() };
                        Ok(summarize(&label, vec![ArchiveEntryInfo { size_known: None, name: name.clone(), path: name, size, compressed_size: None, modified: None, crc: None, encrypted: false, is_dir: false }]))
                    }
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

pub async fn browse_archive_with_encoding(path: &Path, password: Option<&str>, encoding: Option<super::zip_password::ZipPasswordEncoding>) -> Result<ArchiveBrowseResult> {
    browse_archive_cancellable_with_encoding(path, password, Arc::new(AtomicBool::new(false)), encoding).await
}

#[cfg(test)]
mod tests {
    use super::{browse_archive, browse_archive_cancellable};
    use std::io::Write;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    #[tokio::test]
    async fn genuine_haozip_forum_seven_zip_browses_every_original_member() {
        let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/archives/haozip-forum-7z-1417");
        for stem in ["论坛7Z内容密码", "论坛7Z文件名密码"] {
            for index in 1..=3 {
                let member = fixtures.join(format!("{stem}.haozip{index:02}.7z"));
                let result = browse_archive(&member, Some("ForumTest-1411")).await.unwrap();
                assert_eq!(result.format, "7Z");
                assert_eq!(result.total_files, 2);
                assert_eq!(result.total_directories, 2);
                assert_eq!(result.total_uncompressed_size, 163_995);
                assert!(result.entries.iter().any(|entry| entry.path.trim_end_matches('/') == "中文目录/空目录" && entry.is_dir));
            }
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn seven_zip_browser_checks_missing_tail_before_supplied_password() {
        // Independent ordinary 7-Zip volumes, NOT a genuine HaoZip forum fixture.
        // The same browser preflight must be correct before adding another reader.
        let encoder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/archive-engine/7z.exe");
        for mode in ["plain", "content", "header"] {
            let temp = tempfile::tempdir().unwrap();
            let source = temp.path().join("payload.bin");
            let payload: Vec<u8> = (0..163_871).map(|index| (index % 251) as u8).collect();
            std::fs::write(&source, &payload).unwrap();
            let archive = temp.path().join("收到的资料.7z");
            let mut command = crate::utils::process::command(&encoder);
            command.args(["a", "-t7z", "-mx=0", "-v64k", "-y"]);
            if mode != "plain" {
                command.args(["-pknown-test-password", if mode == "header" { "-mhe=on" } else { "-mhe=off" }]);
            }
            let result = command.arg(&archive).arg(&source).output().unwrap();
            assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
            let first = temp.path().join("收到的资料.7z.001");
            let last = temp.path().join("收到的资料.7z.003");
            let original_tail = std::fs::read(&last).unwrap();
            let supplied = (mode != "plain").then_some("known-test-password");
            assert_eq!(browse_archive(&first, supplied).await.unwrap().total_files, 1);
            std::fs::remove_file(&last).unwrap();
            for password in [None, Some("wrong"), Some("known-test-password")] {
                let error = browse_archive(&first, password).await.unwrap_err().to_string();
                assert!(error.contains("[archive-inspection:incomplete]"), "mode={mode}, password={password:?}: {error}");
                assert!(error.contains("可能缺少尾卷"), "{error}");
            }
            std::fs::write(&last, &original_tail).unwrap();
            assert_eq!(browse_archive(&first, supplied).await.unwrap().total_files, 1);
            std::fs::write(&last, &original_tail[..original_tail.len() - 1]).unwrap();
            for password in [None, Some("wrong"), Some("known-test-password")] {
                let error = browse_archive(&first, password).await.unwrap_err().to_string();
                assert!(error.contains("[archive-inspection:incomplete]"), "{mode}: {error}");
            }
            std::fs::write(&last, &original_tail).unwrap();
            let original_first = std::fs::read(&first).unwrap();
            let mut corrupt_first = original_first.clone();
            corrupt_first[12] ^= 1;
            std::fs::write(&first, &corrupt_first).unwrap();
            for password in [None, Some("wrong"), Some("known-test-password")] {
                let error = browse_archive(&first, password).await.unwrap_err().to_string();
                assert!(error.contains("[archive-inspection:damaged]"), "{mode}: {error}");
                assert!(error.contains("归档头校验失败"), "{error}");
            }
            std::fs::write(&first, &original_first).unwrap();
            assert_eq!(browse_archive(&first, supplied).await.unwrap().total_files, 1);
            assert_eq!(std::fs::read(&source).unwrap(), payload);
        }
    }

    #[tokio::test]
    async fn browses_zip_sfx_payload_instead_of_executable_resources() {
        let temp = tempfile::tempdir().unwrap();
        let plain = temp.path().join("plain.zip");
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&plain).unwrap());
        writer.start_file("说明.txt", zip::write::FileOptions::default()).unwrap();
        writer.write_all(b"read without executing").unwrap();
        writer.finish().unwrap();
        // Offset regression fixture, not a claim of an externally generated SFX.
        let mut bytes = vec![0u8; 4096]; bytes[..2].copy_from_slice(b"MZ");
        bytes.extend(std::fs::read(&plain).unwrap());
        let executable = temp.path().join("收到的自解压包.exe");
        std::fs::write(&executable, bytes).unwrap();
        let result = browse_archive(&executable, None).await.unwrap();
        assert_eq!(result.format, "ZIP");
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].path, "说明.txt");
    }

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
