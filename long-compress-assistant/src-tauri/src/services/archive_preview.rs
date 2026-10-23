use super::image_reader::Control as ImageControl;
use crate::services::archive_format::ArchiveFormat;
use crate::services::zip_password::ZipPasswordEncoding;
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

pub const MAX_PREVIEW_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_PREVIEW_PIXELS: u64 = 16_000_000;
pub const MAX_PREVIEW_DIMENSION: u32 = 8192;
pub const MAX_TAR_PREVIEW_SCAN_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_TEXT_PREVIEW_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveImagePreview {
    pub entry_path: String,
    pub mime_type: String,
    pub data_url: String,
    pub byte_size: u64,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveTextPreview {
    pub entry_path: String,
    pub content: String,
    pub encoding: String,
    pub byte_size: u64,
    pub total_size: u64,
    pub truncated: bool,
    pub line_count: usize,
}

struct TextBytes {
    bytes: Vec<u8>,
    total_size: u64,
}

fn normalized_entry_path(value: &str) -> Result<String> {
    let normalized = value.replace('\\', "/");
    if normalized.is_empty() || normalized.contains('\0') {
        anyhow::bail!("Preview entry path is invalid");
    }
    Ok(normalized)
}

struct ScanLimitedReader<R> {
    inner: R,
    remaining: u64,
    control: Option<(ImageControl,std::sync::Arc<std::sync::atomic::AtomicBool>)>,
}

impl<R> ScanLimitedReader<R> {
    fn new(inner: R, max_bytes: u64) -> Self {
        Self {
            inner,
            remaining: max_bytes,
            control: None,
        }
    }
}

impl<R: Read> Read for ScanLimitedReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if let Some((control,_))=&self.control {control.check().map_err(|e|io::Error::other(e.to_string()))?;}
        if self.remaining == 0 {
            let mut overflow = [0u8; 1];
            return match self.inner.read(&mut overflow)? {
                0 => Ok(0),
                _ => Err(io::Error::other(
                    "TAR image preview exceeded the bounded scan budget",
                )),
            };
        }
        let capacity =
            usize::try_from(self.remaining.min(buffer.len() as u64)).unwrap_or(buffer.len());
        let read = self.inner.read(&mut buffer[..capacity])?;
        self.remaining -= read as u64;
        if let Some((control,scanning))=&self.control {if scanning.load(std::sync::atomic::Ordering::Acquire){control.update("locating",MAX_TAR_PREVIEW_SCAN_BYTES-self.remaining,MAX_TAR_PREVIEW_SCAN_BYTES);}}
        Ok(read)
    }
}

#[cfg(test)]
fn read_bounded(reader: &mut dyn Read, expected_bytes: u64) -> Result<Vec<u8>> {
    read_bounded_controlled(reader,expected_bytes,&ImageControl::default())
}
fn read_bounded_controlled(reader:&mut dyn Read,expected_bytes:u64,control:&ImageControl)->Result<Vec<u8>>{
    let mut bytes=Vec::with_capacity(expected_bytes.min(MAX_PREVIEW_BYTES) as usize);let mut buffer=[0u8;65536];
    loop {control.check()?;let remaining=(MAX_PREVIEW_BYTES+1).saturating_sub(bytes.len() as u64) as usize;if remaining==0{break}
        let n=reader.read(&mut buffer[..remaining.min(65536)]).context("Unable to read the selected image entry")?;if n==0{break}bytes.extend_from_slice(&buffer[..n]);control.update("reading",bytes.len() as u64,expected_bytes);
    }
    if bytes.len() as u64 > MAX_PREVIEW_BYTES {
        anyhow::bail!(
            "Image preview is limited to {} MiB after decompression",
            MAX_PREVIEW_BYTES / 1024 / 1024
        );
    }
    if bytes.len() as u64 != expected_bytes {
        anyhow::bail!(
            "Image entry was truncated (expected {expected_bytes} bytes, read {})",
            bytes.len()
        );
    }
    Ok(bytes)
}

fn read_text_prefix(reader: &mut dyn Read, total_size: u64) -> Result<TextBytes> {
    let mut bytes = Vec::with_capacity(total_size.min(MAX_TEXT_PREVIEW_BYTES) as usize);
    reader
        .take(MAX_TEXT_PREVIEW_BYTES)
        .read_to_end(&mut bytes)
        .context("Unable to read the selected text entry")?;
    if total_size <= MAX_TEXT_PREVIEW_BYTES && bytes.len() as u64 != total_size {
        anyhow::bail!(
            "Text entry was truncated (expected {total_size} bytes, read {})",
            bytes.len()
        );
    }
    Ok(TextBytes { bytes, total_size })
}

fn read_zip_entry(path: &Path, entry_path: &str, password: Option<&str>, encoding: Option<ZipPasswordEncoding>,control:&ImageControl) -> Result<Vec<u8>> {
    let mut archive = zip_aes::ZipArchive::new(super::numeric_volume_reader::ZipInputReader::open(path)?)?;
    let mut entry = match password.filter(|value| !value.is_empty()) {
        Some(password) => crate::services::zip_password::open_named_entry_with_encoding(&mut archive, entry_path, password, encoding)?,
        None => archive.by_name(entry_path)?,
    };
    if entry.is_dir() {
        anyhow::bail!("Directories cannot be previewed");
    }
    let expected_bytes = entry.size();
    if expected_bytes > MAX_PREVIEW_BYTES {
        anyhow::bail!(
            "Image preview is limited to {} MiB after decompression",
            MAX_PREVIEW_BYTES / 1024 / 1024
        );
    }
    read_bounded_controlled(&mut entry, expected_bytes,control)
}

fn read_zip_text_entry(path: &Path, entry_path: &str, password: Option<&str>, encoding: Option<ZipPasswordEncoding>) -> Result<TextBytes> {
    let mut archive = zip_aes::ZipArchive::new(super::numeric_volume_reader::ZipInputReader::open(path)?)?;
    let mut entry = match password.filter(|value| !value.is_empty()) {
        Some(password) => crate::services::zip_password::open_named_entry_with_encoding(&mut archive, entry_path, password, encoding)?,
        None => archive.by_name(entry_path)?,
    };
    if entry.is_dir() {
        anyhow::bail!("Directories cannot be previewed");
    }
    let total_size = entry.size();
    read_text_prefix(&mut entry, total_size)
}

fn read_tar_entry<R: Read>(reader: R, entry_path: &str,control:&ImageControl) -> Result<Vec<u8>> {
    let scanning=std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let mut reader = ScanLimitedReader::new(reader, MAX_TAR_PREVIEW_SCAN_BYTES);reader.control=Some((control.clone(),scanning.clone()));
    let mut archive = tar::Archive::new(reader);
    for entry in archive.entries()? {
        let mut entry = super::tar_compatibility::LogicalEntry::new(entry?)?;
        let path = entry.path().to_string_lossy().replace('\\', "/");
        if path != entry_path.trim_start_matches("./") {
            continue;
        }
        if !entry.is_file() {
            anyhow::bail!("Only regular image files can be previewed");
        }
        let expected_bytes = entry.size();
        if expected_bytes > MAX_PREVIEW_BYTES {
            anyhow::bail!(
                "Image preview is limited to {} MiB after decompression",
                MAX_PREVIEW_BYTES / 1024 / 1024
            );
        }
        scanning.store(false,std::sync::atomic::Ordering::Release);
        return read_bounded_controlled(&mut entry, expected_bytes,control);
    }
    anyhow::bail!("The selected image entry was not found in the archive")
}

fn read_tar_text_entry<R: Read>(reader: R, entry_path: &str) -> Result<TextBytes> {
    let reader = ScanLimitedReader::new(reader, MAX_TAR_PREVIEW_SCAN_BYTES);
    let mut archive = tar::Archive::new(reader);
    for entry in archive.entries()? {
        let mut entry = super::tar_compatibility::LogicalEntry::new(entry?)?;
        let path = entry.path().to_string_lossy().replace('\\', "/");
        if path != entry_path.trim_start_matches("./") {
            continue;
        }
        if !entry.is_file() {
            anyhow::bail!("Only regular text files can be previewed");
        }
        let total_size = entry.size();
        return read_text_prefix(&mut entry, total_size);
    }
    anyhow::bail!("The selected text entry was not found in the archive")
}

fn preview_format(path: &Path) -> Result<&'static str> {
    use super::compression_format::CompressionRoute as Route;
    let format = super::archive_browser::detect_format(path)?;
    if format == ArchiveFormat::Zip { return Ok("ZIP"); }
    if format == ArchiveFormat::Zpaq { return Ok("ZPAQ"); }
    if format == ArchiveFormat::Wim { return Ok("WIM"); }
    if format == ArchiveFormat::Alz { return Ok("ALZ"); }
    if format == ArchiveFormat::Egg { return Ok("EGG"); }
    match super::tar_compatibility::integrity_route(path, &format) {
        Some(Route::Tar) => Ok("TAR"),
        Some(Route::TarGzip) => Ok("TAR.GZ"),
        Some(Route::TarBzip2) => Ok("TAR.BZ2"),
        Some(Route::TarXz) => Ok("TAR.XZ"),
        Some(Route::TarZstd) => Ok("TAR.ZST"),
        Some(Route::TarLz4) => Ok("TAR.LZ4"),
        Some(Route::TarBrotli) => Ok("TAR.BR"),
        Some(Route::TarLzip) => Ok("TAR.LZIP"),
        Some(Route::TarLzma) => Ok("TAR.LZMA"),
        _ => super::codec_stream::plain_format(&format).map(|(_,label,_)| label)
            .context("此格式暂不支持内部预览，请先提取后打开"),
    }
}

fn read_tar_family(path: &Path, format: &str, entry_path: &str,control:&ImageControl) -> Result<Vec<u8>> {
    match format {
        "TAR.LZ4" => read_tar_entry(super::codec_stream::decoder(path, "lz4")?, entry_path,control),
        "TAR.BR" => read_tar_entry(super::codec_stream::decoder(path, "br")?, entry_path,control),
        "TAR.LZIP" => read_tar_entry(super::codec_stream::decoder(path, "lz")?, entry_path,control),
        "TAR.LZMA" => read_tar_entry(super::codec_stream::decoder(path, "lzma")?, entry_path,control),
        "TAR" => read_tar_entry(super::numeric_volume_reader::ZipInputReader::open(path)?, entry_path,control),
        "TAR.GZ" => read_tar_entry(crate::services::codec_stream::decoder(path, "gz")?, entry_path,control),
        "TAR.BZ2" => read_tar_entry(crate::services::codec_stream::decoder(path, "bz2")?, entry_path,control),
        "TAR.XZ" => read_tar_entry(crate::services::codec_stream::decoder(path, "xz")?, entry_path,control),
        "TAR.ZST" => read_tar_entry(
            crate::services::codec_stream::decoder(path, "zst")?,
            entry_path,control,
        ),
        _ => anyhow::bail!(
            "Safe bounded preview is currently available for ZIP and TAR-family archives only"
        ),
    }
}

fn read_tar_family_text(path: &Path, format: &str, entry_path: &str) -> Result<TextBytes> {
    match format {
        "TAR.LZ4" => read_tar_text_entry(super::codec_stream::decoder(path, "lz4")?, entry_path),
        "TAR.BR" => read_tar_text_entry(super::codec_stream::decoder(path, "br")?, entry_path),
        "TAR.LZIP" => read_tar_text_entry(super::codec_stream::decoder(path, "lz")?, entry_path),
        "TAR.LZMA" => read_tar_text_entry(super::codec_stream::decoder(path, "lzma")?, entry_path),
        "TAR" => read_tar_text_entry(super::numeric_volume_reader::ZipInputReader::open(path)?, entry_path),
        "TAR.GZ" => {
            read_tar_text_entry(crate::services::codec_stream::decoder(path, "gz")?, entry_path)
        }
        "TAR.BZ2" => {
            read_tar_text_entry(crate::services::codec_stream::decoder(path, "bz2")?, entry_path)
        }
        "TAR.XZ" => read_tar_text_entry(crate::services::codec_stream::decoder(path, "xz")?, entry_path),
        "TAR.ZST" => read_tar_text_entry(
            crate::services::codec_stream::decoder(path, "zst")?,
            entry_path,
        ),
        _ => anyhow::bail!(
            "Safe bounded text preview is currently available for ZIP and TAR-family archives only"
        ),
    }
}

// Single compressed files have no entry directory or reliable declared total.
// Scan at most the existing preview budget, retaining only the displayed prefix.
fn read_plain_stream(path: &Path, format: &str, entry: &str, max_total: u64, keep: u64) -> Result<TextBytes> {
    read_plain_stream_controlled(path,format,entry,max_total,keep,None)
}
fn read_plain_stream_controlled(path: &Path, format: &str, entry: &str, max_total: u64, keep: u64,control:Option<&ImageControl>) -> Result<TextBytes> {
    let detected = if format == "LZIP" { ArchiveFormat::Lzip } else { ArchiveFormat::from_extension(&format.to_ascii_lowercase()) };
    let (codec, _, suffixes) = super::codec_stream::plain_format(&detected).context("Unsupported compressed preview")?;
    let name = super::native_extraction::single_stream::output_name(path, suffixes);
    anyhow::ensure!(entry == name, "The selected entry was not found in the compressed file");
    let mut reader = super::codec_stream::decoder(path, codec)?;
    let mut bytes = Vec::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        if let Some(control)=control {control.check()?;}
        let count = reader.read(&mut buffer)?;
        if count == 0 { break; }
        total += count as u64;
        if let Some(control)=control {control.update("reading",total,0);}
        anyhow::ensure!(total <= max_total, "此压缩流超出内部预览的读取范围，请先解压后打开；未将部分预览当作完整文件");
        let retained = (keep.saturating_sub(bytes.len() as u64) as usize).min(count);
        bytes.extend_from_slice(&buffer[..retained]);
    }
    Ok(TextBytes { bytes, total_size: total })
}

fn be_u32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn le_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn le_u24(bytes: &[u8]) -> u32 {
    bytes[0] as u32 | ((bytes[1] as u32) << 8) | ((bytes[2] as u32) << 16)
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 4 || bytes[..2] != [0xFF, 0xD8] {
        return None;
    }
    let mut index = 2usize;
    while index + 3 < bytes.len() {
        while index < bytes.len() && bytes[index] == 0xFF {
            index += 1;
        }
        if index >= bytes.len() {
            return None;
        }
        let marker = bytes[index];
        index += 1;
        if marker == 0xD9 || marker == 0xDA {
            return None;
        }
        if matches!(marker, 0x01 | 0xD0..=0xD7) {
            continue;
        }
        if index + 2 > bytes.len() {
            return None;
        }
        let length = u16::from_be_bytes([bytes[index], bytes[index + 1]]) as usize;
        if length < 2 || index + length > bytes.len() {
            return None;
        }
        if matches!(marker, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF) && length >= 7 {
            let height = u16::from_be_bytes([bytes[index + 3], bytes[index + 4]]) as u32;
            let width = u16::from_be_bytes([bytes[index + 5], bytes[index + 6]]) as u32;
            return Some((width, height));
        }
        index += length;
    }
    None
}

fn image_identity(bytes: &[u8]) -> Result<(&'static str, u32, u32)> {
    let identity = if bytes.len() >= 24
        && bytes[..8] == [137, 80, 78, 71, 13, 10, 26, 10]
        && &bytes[12..16] == b"IHDR"
    {
        Some(("image/png", be_u32(&bytes[16..20]), be_u32(&bytes[20..24])))
    } else if bytes.len() >= 10 && matches!(&bytes[..6], b"GIF87a" | b"GIF89a") {
        Some((
            "image/gif",
            le_u16(&bytes[6..8]) as u32,
            le_u16(&bytes[8..10]) as u32,
        ))
    } else if bytes.len() >= 26 && &bytes[..2] == b"BM" {
        let width = i32::from_le_bytes(bytes[18..22].try_into().unwrap()).unsigned_abs();
        let height = i32::from_le_bytes(bytes[22..26].try_into().unwrap()).unsigned_abs();
        Some(("image/bmp", width, height))
    } else if let Some((width, height)) = jpeg_dimensions(bytes) {
        Some(("image/jpeg", width, height))
    } else if bytes.len() >= 30 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        match &bytes[12..16] {
            b"VP8X" => Some((
                "image/webp",
                le_u24(&bytes[24..27]) + 1,
                le_u24(&bytes[27..30]) + 1,
            )),
            b"VP8 " if bytes.len() >= 30 && bytes[23..26] == [0x9D, 0x01, 0x2A] => Some((
                "image/webp",
                le_u16(&bytes[26..28]) as u32 & 0x3FFF,
                le_u16(&bytes[28..30]) as u32 & 0x3FFF,
            )),
            b"VP8L" if bytes.len() >= 25 && bytes[20] == 0x2F => Some((
                "image/webp",
                1 + bytes[21] as u32 + (((bytes[22] & 0x3F) as u32) << 8),
                1 + ((bytes[22] as u32) >> 6)
                    + ((bytes[23] as u32) << 2)
                    + (((bytes[24] & 0x0F) as u32) << 10),
            )),
            _ => None,
        }
    } else {
        None
    };
    let (mime, width, height) = identity.ok_or_else(|| {
        anyhow::anyhow!(
            "The selected entry is not a supported raster image (PNG, JPEG, GIF, WebP or BMP)"
        )
    })?;
    if width == 0 || height == 0 {
        anyhow::bail!("Image dimensions are invalid");
    }
    let pixels = u64::from(width) * u64::from(height);
    if width > MAX_PREVIEW_DIMENSION
        || height > MAX_PREVIEW_DIMENSION
        || pixels > MAX_PREVIEW_PIXELS
    {
        anyhow::bail!(
            "Image dimensions exceed the preview safety limit ({} × {} pixels, {} megapixels)",
            MAX_PREVIEW_DIMENSION,
            MAX_PREVIEW_DIMENSION,
            MAX_PREVIEW_PIXELS / 1_000_000
        );
    }
    Ok((mime, width, height))
}

fn preview_archive_image_sync(
    archive_path: &Path,
    entry_path: &str,
    password: Option<&str>,
    encoding: Option<ZipPasswordEncoding>,
    snapshot: Option<crate::models::compression::ZpaqSnapshot>,
    control:&ImageControl,
) -> Result<ArchiveImagePreview> {
    anyhow::ensure!(snapshot.is_none() || super::archive_browser::detect_format(archive_path)? == ArchiveFormat::Zpaq, "所选快照只适用于 ZPAQ 日志归档");
    if super::archive_browser::detect_format(archive_path)? == ArchiveFormat::AesEncrypted {
        let inner = super::aes_container::AuthenticatedInner::open(archive_path, password, || control.check())?;
        return preview_archive_image_sync(&inner.path, entry_path, None, encoding, snapshot,control);
    }
    let entry_path = normalized_entry_path(entry_path)?;
    let format = if super::archive_browser::detect_format(archive_path)?==ArchiveFormat::SevenZip {"7Z"} else {preview_format(archive_path)?};
    let bytes = match format {
        "7Z" => super::sevenz_preview::read_image(archive_path,&entry_path,password,control)?,
        "ZIP" => read_zip_entry(archive_path, &entry_path, password, encoding,control)?,
        "EGG" => tauri::async_runtime::block_on(super::egg_engine::preview_image_entry(archive_path, &entry_path, password, encoding, control.flag.clone()))?,
        "ALZ" => tauri::async_runtime::block_on(super::alz_engine::preview_image_entry(archive_path, &entry_path, password, encoding, control.flag.clone()))?,
        "WIM" => tauri::async_runtime::block_on(super::wim_preview::preview_entry(archive_path, &entry_path, password, MAX_PREVIEW_BYTES, MAX_PREVIEW_BYTES, control.flag.clone()))?.0,
        "ZPAQ" => tauri::async_runtime::block_on(super::zpaq_engine::preview_entry_controlled_at(archive_path, &entry_path, password, MAX_PREVIEW_BYTES, MAX_PREVIEW_BYTES, snapshot.as_ref(),control.flag.clone()))?.0,
        format if format.starts_with("TAR") => read_tar_family(archive_path, format, &entry_path,control)?,
        "GZIP" | "BZIP2" | "XZ" | "ZSTD" | "LZMA" | "LZIP" | "LZ4" | "BR" => read_plain_stream_controlled(archive_path, format, &entry_path, MAX_PREVIEW_BYTES, MAX_PREVIEW_BYTES,Some(control))?.bytes,
        _ => anyhow::bail!(
            "Safe bounded preview is currently available for ZIP and TAR-family archives only"
        ),
    };
    control.check()?;control.update("decoding",bytes.len() as u64,bytes.len() as u64);
    let (mime_type, width, height) = image_identity(&bytes)?;
    Ok(ArchiveImagePreview {
        entry_path,
        mime_type: mime_type.to_string(),
        data_url: format!(
            "data:{mime_type};base64,{}",
            general_purpose::STANDARD.encode(&bytes)
        ),
        byte_size: bytes.len() as u64,
        width,
        height,
    })
}

pub async fn preview_archive_image(
    archive_path: &Path,
    entry_path: &str,
    password: Option<&str>,
) -> Result<ArchiveImagePreview> {
    preview_archive_image_with_encoding(archive_path, entry_path, password, None).await
}

pub async fn preview_archive_image_with_encoding(
    archive_path: &Path,
    entry_path: &str,
    password: Option<&str>,
    encoding: Option<ZipPasswordEncoding>,
) -> Result<ArchiveImagePreview> {
    preview_archive_image_at(archive_path, entry_path, password, encoding, None).await
}

pub async fn preview_archive_image_at(
    archive_path: &Path, entry_path: &str, password: Option<&str>,
    encoding: Option<ZipPasswordEncoding>, snapshot: Option<crate::models::compression::ZpaqSnapshot>,
) -> Result<ArchiveImagePreview> {
    preview_archive_image_controlled_at(archive_path,entry_path,password,encoding,snapshot,ImageControl::default()).await
}
pub async fn preview_archive_image_controlled_at(archive_path:&Path,entry_path:&str,password:Option<&str>,encoding:Option<ZipPasswordEncoding>,snapshot:Option<crate::models::compression::ZpaqSnapshot>,control:ImageControl)->Result<ArchiveImagePreview>{
    preview_archive_image_bound_at(archive_path, entry_path, password, encoding, snapshot, control, None).await
}
pub async fn preview_archive_image_bound_at(archive_path:&Path,entry_path:&str,password:Option<&str>,encoding:Option<ZipPasswordEncoding>,snapshot:Option<crate::models::compression::ZpaqSnapshot>,control:ImageControl,expected_revision:Option<String>)->Result<ArchiveImagePreview>{
    let archive_path = archive_path.to_path_buf();
    let entry_path = entry_path.to_string();
    let password = password.map(str::to_string);
    tauri::async_runtime::spawn_blocking(move || {
        control.check()?;
        if let Some(revision) = expected_revision.as_deref() { super::image_reader::check_source(&archive_path, revision)?; }
        let source = super::split_archive_detector::SplitArchiveDetector::complete_source(&archive_path)?;
        let revision = if matches!(super::archive_browser::detect_format(&source)?,ArchiveFormat::Alz|ArchiveFormat::Egg|ArchiveFormat::SevenZip) {
            Some(match expected_revision { Some(value) => value, None => super::image_reader::revision(&source)? })
        } else { None };
        control.update("opening",0,0);control.check()?;
        let result = preview_archive_image_sync(&source, &entry_path, password.as_deref(), encoding, snapshot,&control)?;
        control.check()?;
        if let Some(revision) = revision { super::image_reader::check_source(&source, &revision)?; }
        Ok(result)
    })
    .await
    .context("Archive image preview worker failed")?
}

fn looks_binary(bytes: &[u8]) -> bool {
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        return false;
    }
    if bytes.contains(&0) {
        return true;
    }
    let controls = bytes
        .iter()
        .filter(|byte| matches!(byte, 0x01..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F | 0x7F))
        .count();
    controls > bytes.len().max(1) / 20
}

fn decode_utf16(bytes: &[u8], little_endian: bool) -> Option<String> {
    let mut values = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks(2) {
        if pair.len() != 2 {
            break;
        }
        values.push(if little_endian {
            u16::from_le_bytes([pair[0], pair[1]])
        } else {
            u16::from_be_bytes([pair[0], pair[1]])
        });
    }
    String::from_utf16(&values).ok()
}

fn decode_text_prefix(bytes: &[u8]) -> Result<(String, String, usize)> {
    if looks_binary(bytes) {
        anyhow::bail!("The selected entry appears to be binary and will not be rendered as text");
    }
    if let Some(payload) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        let text = std::str::from_utf8(payload).context("Invalid UTF-8 text after BOM")?;
        return Ok((text.to_string(), "UTF-8 BOM".to_string(), bytes.len()));
    }
    if let Some(payload) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        let usable = payload.len() - payload.len() % 2;
        let text = decode_utf16(&payload[..usable], true).context("Invalid UTF-16LE text")?;
        return Ok((text, "UTF-16LE".to_string(), usable + 2));
    }
    if let Some(payload) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let usable = payload.len() - payload.len() % 2;
        let text = decode_utf16(&payload[..usable], false).context("Invalid UTF-16BE text")?;
        return Ok((text, "UTF-16BE".to_string(), usable + 2));
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok((text.to_string(), "UTF-8".to_string(), bytes.len()));
    }

    for (encoding, label) in [
        (encoding_rs::GBK, "GBK"),
        (encoding_rs::BIG5, "Big5"),
        (encoding_rs::WINDOWS_1252, "Windows-1252"),
    ] {
        for trim in 0..=4.min(bytes.len()) {
            let candidate = &bytes[..bytes.len() - trim];
            if let Some(text) =
                encoding.decode_without_bom_handling_and_without_replacement(candidate)
            {
                return Ok((text.into_owned(), label.to_string(), candidate.len()));
            }
        }
    }
    anyhow::bail!("The selected entry does not use a supported text encoding")
}

/// File-manager previews share the archive viewer's encodings and image decoder.
/// Only a prefix is read, so opening a large log never loads the complete file.
pub(crate) fn preview_local_text(path: &Path) -> Result<ArchiveTextPreview> {
    let file = std::fs::File::open(path).context("无法读取此文本文件，它可能已被移动或正被其他程序占用")?;
    let metadata = file.metadata()?;
    anyhow::ensure!(metadata.is_file(), "请选择文本文件");
    let mut bytes = Vec::new();
    file.take(MAX_TEXT_PREVIEW_BYTES).read_to_end(&mut bytes)?;
    if metadata.len() > bytes.len() as u64 {
        // Do not mistake a UTF-8 character split at the display boundary for GBK.
        let bom = usize::from(bytes.starts_with(&[0xef,0xbb,0xbf])) * 3;
        if let Err(error) = std::str::from_utf8(&bytes[bom..]) {
            if error.error_len().is_none() { bytes.truncate(bom + error.valid_up_to()); }
        }
        // A UTF-16 prefix may end between the two halves of a surrogate pair.
        if bytes.len() >= 4 && (bytes.starts_with(&[0xff,0xfe]) || bytes.starts_with(&[0xfe,0xff])) {
            let n = bytes.len();
            let last = if bytes[0] == 0xff {u16::from_le_bytes([bytes[n-2],bytes[n-1]])} else {u16::from_be_bytes([bytes[n-2],bytes[n-1]])};
            if (0xd800..=0xdbff).contains(&last) { bytes.truncate(n-2); }
        }
    }
    let (content, encoding, consumed) = decode_text_prefix(&bytes)?;
    let line_count = content.lines().count();
    Ok(ArchiveTextPreview { entry_path:path.to_string_lossy().into_owned(), content, encoding,
        byte_size:consumed as u64, total_size:metadata.len(), truncated:metadata.len()>consumed as u64, line_count })
}

pub(crate) fn preview_local_image(path: &Path) -> Result<ArchiveImagePreview> {
    let file = std::fs::File::open(path).context("无法读取此图片，它可能已被移动")?;
    let metadata = file.metadata()?;
    anyhow::ensure!(metadata.is_file(), "请选择图片文件");
    anyhow::ensure!(metadata.len() <= MAX_PREVIEW_BYTES, "图片超过 8 MB，请在资源管理器中使用图片应用查看");
    let mut bytes = Vec::new();
    file.take(MAX_PREVIEW_BYTES+1).read_to_end(&mut bytes)?;
    anyhow::ensure!(bytes.len() as u64 <= MAX_PREVIEW_BYTES, "图片超过预览大小");
    let (mime, width, height) = image_identity(&bytes)?;
    Ok(ArchiveImagePreview {entry_path:path.to_string_lossy().into_owned(),mime_type:mime.into(),
        data_url:format!("data:{mime};base64,{}",general_purpose::STANDARD.encode(&bytes)),byte_size:bytes.len() as u64,width,height})
}

fn preview_archive_text_sync(
    archive_path: &Path,
    entry_path: &str,
    password: Option<&str>,
    password_encoding: Option<ZipPasswordEncoding>,
    snapshot: Option<crate::models::compression::ZpaqSnapshot>,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<ArchiveTextPreview> {
    anyhow::ensure!(snapshot.is_none() || super::archive_browser::detect_format(archive_path)? == ArchiveFormat::Zpaq, "所选快照只适用于 ZPAQ 日志归档");
    if super::archive_browser::detect_format(archive_path)? == ArchiveFormat::AesEncrypted {
        let inner = super::aes_container::AuthenticatedInner::open(archive_path, password, || Ok(()))?;
        return preview_archive_text_sync(&inner.path, entry_path, None, password_encoding, snapshot, cancelled);
    }
    let entry_path = normalized_entry_path(entry_path)?;
    let format = match super::archive_browser::detect_format(archive_path)? {
        ArchiveFormat::Alz=>"ALZ",ArchiveFormat::Egg=>"EGG",ArchiveFormat::SevenZip=>"7Z",_=>preview_format(archive_path)?
    };
    let mut text_bytes = match format {
        "7Z" => { let (bytes,total_size)=super::sevenz_preview::read(archive_path,&entry_path,password,cancelled)?; TextBytes{bytes,total_size} },
        "EGG"=>{
            let (bytes,total_size)=tauri::async_runtime::block_on(super::egg_engine::preview_text_entry(archive_path,&entry_path,password,password_encoding,cancelled))?;
            TextBytes{bytes,total_size}
        }
        "ALZ" => {
            let (bytes, total_size) = tauri::async_runtime::block_on(super::alz_engine::preview_text_entry(archive_path, &entry_path, password, password_encoding, cancelled))?;
            TextBytes { bytes, total_size }
        }
        "ZIP" => read_zip_text_entry(archive_path, &entry_path, password, password_encoding)?,
        "WIM" => {
            let (bytes, total_size) = tauri::async_runtime::block_on(super::wim_preview::preview_entry(archive_path, &entry_path, password, MAX_TAR_PREVIEW_SCAN_BYTES, MAX_TEXT_PREVIEW_BYTES, std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false))))?;
            TextBytes { bytes, total_size }
        }
        "ZPAQ" => {
            let (bytes, total_size) = tauri::async_runtime::block_on(super::zpaq_engine::preview_entry_at(archive_path, &entry_path, password, MAX_TAR_PREVIEW_SCAN_BYTES, MAX_TEXT_PREVIEW_BYTES, snapshot.as_ref()))?;
            TextBytes { bytes, total_size }
        }
        format if format.starts_with("TAR") => {
            read_tar_family_text(archive_path, format, &entry_path)?
        }
        "GZIP" | "BZIP2" | "XZ" | "ZSTD" | "LZMA" | "LZIP" | "LZ4" | "BR" => read_plain_stream(archive_path, format, &entry_path, MAX_TAR_PREVIEW_SCAN_BYTES, MAX_TEXT_PREVIEW_BYTES)?,
        _ => anyhow::bail!(
            "Safe bounded text preview is currently available for ZIP and TAR-family archives only"
        ),
    };
    if matches!(format,"ALZ"|"EGG") && text_bytes.total_size>text_bytes.bytes.len() as u64 {
        let bytes=&mut text_bytes.bytes;
        let bom=usize::from(bytes.starts_with(&[0xef,0xbb,0xbf]))*3;
        if let Err(error)=std::str::from_utf8(&bytes[bom..]) { if error.error_len().is_none(){bytes.truncate(bom+error.valid_up_to());} }
        if bytes.len()>=4 && (bytes.starts_with(&[0xff,0xfe])||bytes.starts_with(&[0xfe,0xff])) {
            let n=bytes.len();let unit=if bytes[0]==0xff{u16::from_le_bytes([bytes[n-2],bytes[n-1]])}else{u16::from_be_bytes([bytes[n-2],bytes[n-1]])};
            if (0xd800..=0xdbff).contains(&unit){bytes.truncate(n-2);}
        }
    }
    let (content, encoding, consumed) = decode_text_prefix(&text_bytes.bytes)?;
    anyhow::ensure!(format!="EGG"||text_bytes.total_size>text_bytes.bytes.len() as u64||consumed==text_bytes.bytes.len(),"EGG 文本编码末尾不完整，请先提取后打开");
    anyhow::ensure!(format!="7Z" || consumed==text_bytes.bytes.len(),"7Z 文本编码末尾不完整，请先提取后打开");
    let truncated = text_bytes.total_size > consumed as u64;
    let line_count = if content.is_empty() {
        0
    } else {
        content.lines().count()
    };
    Ok(ArchiveTextPreview {
        entry_path,
        content,
        encoding,
        byte_size: consumed as u64,
        total_size: text_bytes.total_size,
        truncated,
        line_count,
    })
}

pub async fn preview_archive_text(
    archive_path: &Path,
    entry_path: &str,
    password: Option<&str>,
) -> Result<ArchiveTextPreview> {
    preview_archive_text_with_encoding(archive_path, entry_path, password, None).await
}

pub async fn preview_archive_text_with_encoding(
    archive_path: &Path,
    entry_path: &str,
    password: Option<&str>,
    encoding: Option<ZipPasswordEncoding>,
) -> Result<ArchiveTextPreview> {
    preview_archive_text_at(archive_path, entry_path, password, encoding, None).await
}

pub async fn preview_archive_text_at(
    archive_path: &Path, entry_path: &str, password: Option<&str>,
    encoding: Option<ZipPasswordEncoding>, snapshot: Option<crate::models::compression::ZpaqSnapshot>,
) -> Result<ArchiveTextPreview> {
    preview_archive_text_controlled_at(archive_path, entry_path, password, encoding, snapshot,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)), None).await
}

pub async fn preview_archive_text_controlled_at(
    archive_path: &Path, entry_path: &str, password: Option<&str>, encoding: Option<ZipPasswordEncoding>,
    snapshot: Option<crate::models::compression::ZpaqSnapshot>, cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    expected_revision: Option<String>,
) -> Result<ArchiveTextPreview> {
    let archive_path = archive_path.to_path_buf();
    let entry_path = entry_path.to_string();
    let password = password.map(str::to_string);
    tauri::async_runtime::spawn_blocking(move || {
        anyhow::ensure!(!cancelled.load(std::sync::atomic::Ordering::Acquire), "文本预览已取消");
        if let Some(revision) = expected_revision.as_deref() { super::image_reader::check_source(&archive_path, revision)?; }
        let source = super::split_archive_detector::SplitArchiveDetector::complete_source(&archive_path)?;
        let revision = if matches!(super::archive_browser::detect_format(&source)?,ArchiveFormat::Alz|ArchiveFormat::Egg|ArchiveFormat::SevenZip) {
            let revision = match expected_revision { Some(value) => value, None => super::image_reader::revision(&source)? };
            super::image_reader::check_source(&source, &revision)?;
            Some(revision)
        } else { None };
        let result = preview_archive_text_sync(&source, &entry_path, password.as_deref(), encoding, snapshot, cancelled.clone())?;
        anyhow::ensure!(!cancelled.load(std::sync::atomic::Ordering::Acquire), "文本预览已取消");
        if let Some(revision) = revision { super::image_reader::check_source(&source, &revision)?; }
        Ok(result)
    }).await.context("Archive text preview worker failed")?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[cfg(windows)]
    #[tokio::test]
    async fn fixed_password_encoding_previews_original_infozip_text_and_numeric_slices() {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/legacy-zip-password-1416");
        let expected = std::fs::read_to_string(fixtures.join("payload.txt")).unwrap();
        for (encoding, password) in [
            (ZipPasswordEncoding::Cp932, "日本語パスワード+!1416"),
            (ZipPasswordEncoding::Cp936, "简体密码测试+!1416"),
            (ZipPasswordEncoding::Cp950, "繁體密碼測試+!1416"),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let original = fixtures.join(format!("{}.zip", encoding.as_str()));
            let before = std::fs::read(&original).unwrap();
            let split = crate::services::split_compression::SplitCompressionService::split_archive_file(
                &original, &temp.path().join("slices.zip"), 37, || Ok(())).unwrap();
            let files_before = std::fs::read_dir(temp.path()).unwrap().count();
            for source in [&original, &split.part_files[0]] {
                let preview = preview_archive_text_with_encoding(source, "payload.txt", Some(password), Some(encoding)).await.unwrap();
                assert_eq!(preview.content, expected);
                assert_eq!(preview.total_size, expected.len() as u64);
                assert!(!preview.truncated);
                assert!(preview_archive_text_with_encoding(source, "payload.txt", Some(password), Some(ZipPasswordEncoding::Utf8)).await.is_err());
                let error = preview_archive_text_with_encoding(source, "payload.txt", Some("🔑"), Some(encoding)).await.unwrap_err();
                assert!(crate::services::zip_password::encoding_error(&error).unwrap().is_unrepresentable());
            }
            assert_eq!(std::fs::read(&original).unwrap(), before);
            assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), files_before, "preview must not materialize entries");
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn fixed_password_encoding_image_and_text_previews_keep_entry_scope_and_bounds() {
        for (encoding, password) in [
            (ZipPasswordEncoding::Cp932, "日本語パスワード+!1416"),
            (ZipPasswordEncoding::Cp936, "简体密码测试+!1416"),
            (ZipPasswordEncoding::Cp950, "繁體密碼測試+!1416"),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let source = temp.path().join("preview.zip");
            let bytes = crate::services::zip_password::encode(password, encoding).unwrap();
            let mut writer = zip_aes::ZipWriter::new(File::create(&source).unwrap());
            let options = zip_aes::write::SimpleFileOptions::default()
                .with_aes_encryption_bytes(zip_aes::AesMode::Aes256, &bytes);
            writer.start_file("图片.png", options).unwrap(); writer.write_all(PNG_1X1).unwrap();
            writer.start_file("large.txt", options).unwrap(); writer.write_all(&vec![b'x'; MAX_TEXT_PREVIEW_BYTES as usize + 12]).unwrap();
            writer.start_file("other.txt", zip_aes::write::SimpleFileOptions::default()
                .with_aes_encryption(zip_aes::AesMode::Aes256, "different password")).unwrap();
            writer.write_all(b"not selected").unwrap(); writer.finish().unwrap();
            let preview = preview_archive_image_with_encoding(&source, "图片.png", Some(password), Some(encoding)).await.unwrap();
            assert_eq!((preview.width, preview.height, preview.byte_size), (1, 1, PNG_1X1.len() as u64));
            let text = preview_archive_text_with_encoding(&source, "large.txt", Some(password), Some(encoding)).await.unwrap();
            assert!(text.truncated);
            assert_eq!(text.byte_size, MAX_TEXT_PREVIEW_BYTES);
            assert!(preview_archive_image_with_encoding(&source, "图片.png", Some(password), Some(ZipPasswordEncoding::Utf8)).await.is_err());
            assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
        }
    }

    const PNG_1X1: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 8, 215, 99, 248, 207, 192, 240, 31,
        0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

    #[test]
    fn tar_scan_reader_accepts_exact_budget_and_rejects_the_next_byte() {
        let mut exact = ScanLimitedReader::new(std::io::Cursor::new(vec![1u8; 4]), 4);
        let mut exact_bytes = Vec::new();
        exact.read_to_end(&mut exact_bytes).unwrap();
        assert_eq!(exact_bytes.len(), 4);

        let mut overflow = ScanLimitedReader::new(std::io::Cursor::new(vec![1u8; 5]), 4);
        let mut overflow_bytes = Vec::new();
        let error = overflow.read_to_end(&mut overflow_bytes).unwrap_err();
        assert!(error.to_string().contains("bounded scan budget"));
        assert_eq!(overflow_bytes.len(), 4);
    }

    #[test]
    fn bounded_image_read_rejects_truncated_payloads() {
        let mut source = std::io::Cursor::new(vec![1u8; 3]);
        let error = read_bounded(&mut source, 4).unwrap_err();
        assert!(error.to_string().contains("truncated"));
    }

    fn write_zip(path: &Path, entry_name: &str, contents: &[u8]) {
        let mut writer = zip::ZipWriter::new(File::create(path).unwrap());
        writer
            .start_file(entry_name, zip::write::FileOptions::default())
            .unwrap();
        writer.write_all(contents).unwrap();
        writer.finish().unwrap();
    }

    #[tokio::test]
    async fn previews_real_zip_png_without_writing_to_disk() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("images.zip");
        write_zip(&archive, "art/像素.png", PNG_1X1);

        let preview = preview_archive_image(&archive, "art/像素.png", None)
            .await
            .unwrap();
        assert_eq!((preview.width, preview.height), (1, 1));
        assert_eq!(preview.mime_type, "image/png");
        assert!(preview.data_url.starts_with("data:image/png;base64,"));
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[tokio::test]
    async fn previews_real_tar_gz_png() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("images.tar.gz");
        let encoder = flate2::write::GzEncoder::new(
            File::create(&archive).unwrap(),
            flate2::Compression::default(),
        );
        let mut tar = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(PNG_1X1.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, "preview.png", PNG_1X1)
            .unwrap();
        tar.into_inner().unwrap().finish().unwrap();

        let preview = preview_archive_image(&archive, "preview.png", None)
            .await
            .unwrap();
        assert_eq!(preview.mime_type, "image/png");
        assert_eq!(preview.byte_size, PNG_1X1.len() as u64);
    }

    #[tokio::test]
    async fn rejects_extension_spoofing_and_active_svg_content() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("unsafe.zip");
        write_zip(
            &archive,
            "looks-safe.png",
            br#"<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script></svg>"#,
        );
        let error = preview_archive_image(&archive, "looks-safe.png", None)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("supported raster image"));
    }

    #[tokio::test]
    async fn rejects_images_over_the_pixel_budget_before_rendering() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("large-pixels.zip");
        let mut header = PNG_1X1.to_vec();
        header[16..20].copy_from_slice(&5000u32.to_be_bytes());
        header[20..24].copy_from_slice(&5000u32.to_be_bytes());
        write_zip(&archive, "large.png", &header);
        let error = preview_archive_image(&archive, "large.png", None)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("safety limit"));
    }

    #[tokio::test]
    async fn sevenz1618_accepts_the_now_bounded_native_image_route() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("images.7z");
        let source = temp.path().join("preview.png");
        std::fs::write(&source, PNG_1X1).unwrap();
        sevenz_rust::compress_to_path(&source, &archive).unwrap();

        let entry_name = source.file_name().unwrap().to_string_lossy();
        let value = preview_archive_image(&archive,&entry_name,None).await.unwrap();
        assert_eq!((value.width,value.height),(1,1));
    }

    #[tokio::test]
    async fn previews_real_zip_utf8_text_without_writing_to_disk() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("text.zip");
        write_zip(&archive, "docs/说明.txt", "第一行\n第二行\n".as_bytes());

        let preview = preview_archive_text(&archive, "docs/说明.txt", None)
            .await
            .unwrap();
        assert_eq!(preview.content, "第一行\n第二行\n");
        assert_eq!(preview.encoding, "UTF-8");
        assert_eq!(preview.line_count, 2);
        assert!(!preview.truncated);
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[tokio::test]
    async fn previews_zip_sfx_text_using_the_detected_payload_format() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("original.zip");
        write_zip(&archive, "说明.txt", "不执行程序即可读取".as_bytes());
        let mut bytes = b"MZ offset regression fixture".to_vec();
        bytes.extend(std::fs::read(&archive).unwrap());
        let sfx = temp.path().join("资料.exe"); std::fs::write(&sfx, &bytes).unwrap();
        let preview = preview_archive_text(&sfx, "说明.txt", None).await.unwrap();
        assert_eq!(preview.content, "不执行程序即可读取");
        assert_eq!(std::fs::read(&sfx).unwrap(), bytes);
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 2);
    }

    #[tokio::test]
    async fn previews_real_tar_gz_gbk_text() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("text.tar.gz");
        let (encoded, _, had_errors) = encoding_rs::GBK.encode("中文本地编码\n第二行");
        assert!(!had_errors);
        let encoder = flate2::write::GzEncoder::new(
            File::create(&archive).unwrap(),
            flate2::Compression::default(),
        );
        let mut tar = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(encoded.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, "legacy.txt", encoded.as_ref())
            .unwrap();
        tar.into_inner().unwrap().finish().unwrap();

        let preview = preview_archive_text(&archive, "legacy.txt", None)
            .await
            .unwrap();
        assert_eq!(preview.content, "中文本地编码\n第二行");
        assert_eq!(preview.encoding, "GBK");
        assert_eq!(preview.line_count, 2);
    }

    #[tokio::test]
    async fn truncates_real_oversized_text_at_the_bounded_prefix() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("large-text.zip");
        let contents = vec![b'x'; MAX_TEXT_PREVIEW_BYTES as usize + 4096];
        write_zip(&archive, "large.log", &contents);

        let preview = preview_archive_text(&archive, "large.log", None)
            .await
            .unwrap();
        assert_eq!(preview.byte_size, MAX_TEXT_PREVIEW_BYTES);
        assert_eq!(preview.content.len(), MAX_TEXT_PREVIEW_BYTES as usize);
        assert_eq!(preview.total_size, contents.len() as u64);
        assert!(preview.truncated);
    }

    #[tokio::test]
    async fn rejects_binary_payloads_even_with_a_text_extension() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("binary.zip");
        write_zip(&archive, "spoofed.txt", &[0, 1, 2, 3, 255, 0, 8]);

        let error = preview_archive_text(&archive, "spoofed.txt", None)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("appears to be binary"));
    }

    #[tokio::test]
    async fn sevenz1617_accepts_the_now_bounded_native_text_route() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("text.7z");
        let source = temp.path().join("readme.txt");
        std::fs::write(&source, b"bounded route required").unwrap();
        sevenz_rust::compress_to_path(&source, &archive).unwrap();

        let value = preview_archive_text(&archive, "readme.txt", None).await.unwrap();
        assert_eq!(value.content,"bounded route required");
        assert!(!value.truncated);
    }
}

#[cfg(test)]
mod local_file_preview_tests {
    use super::*;
    #[test]
    fn local_file_preview_reads_common_text_encodings_and_empty_files() {
        let dir=tempfile::tempdir().unwrap();
        for (name,bytes,expected,encoding) in [
            ("utf8.txt","项目说明\n第二行".as_bytes().to_vec(),"项目说明\n第二行","UTF-8"),
            ("gbk.log",vec![0xcb,0xb5,0xc3,0xf7],"说明","GBK"),
            ("utf16.txt",vec![0xff,0xfe,0x2d,0x4e,0x87,0x65],"中文","UTF-16LE"),
            ("empty.txt",vec![],"","UTF-8"),
        ] {
            let path=dir.path().join(name);std::fs::write(&path,&bytes).unwrap();
            let preview=preview_local_text(&path).unwrap();assert_eq!(preview.content,expected);assert_eq!(preview.encoding,encoding);assert!(!preview.truncated);assert_eq!(preview.total_size,bytes.len() as u64);
        }
    }
    #[test]
    fn local_file_preview_keeps_utf8_at_the_large_log_boundary() {
        let dir=tempfile::tempdir().unwrap();let path=dir.path().join("large.log");
        let mut bytes=vec![b'a';MAX_TEXT_PREVIEW_BYTES as usize-1];bytes.extend_from_slice("中文".as_bytes());std::fs::write(&path,&bytes).unwrap();
        let preview=preview_local_text(&path).unwrap();assert!(preview.truncated);assert_eq!(preview.encoding,"UTF-8");assert_eq!(preview.byte_size,MAX_TEXT_PREVIEW_BYTES-1);assert!(!preview.content.contains('�'));assert_eq!(preview.total_size,bytes.len() as u64);
    }
}

#[cfg(test)]
mod alz_text169_tests {
    use super::*;
    fn fixtures() -> std::path::PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives") }
    #[tokio::test]
    async fn alz_text169_complete_small_texts_and_exact_selection() {
        let root = fixtures();
        let manifest:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("alz-text-169/manifest.json")).unwrap()).unwrap();
        for (name, content, encoding) in [
            ("store",manifest["text"].as_str().unwrap().to_owned(),"UTF-8"),
            ("deflate",manifest["text"].as_str().unwrap().repeat(120),"UTF-8"),
            ("legacy","旧编码日志\n第二行".into(),"GBK"),
            ("utf16","UTF16 配置\nvalue=中文".into(),"UTF-16 LE"),
            ("empty",String::new(),"UTF-8"),
            ("at-limit","x".repeat(1024*1024),"UTF-8"),
        ] {
            let path=root.join(format!("alz-text-169/{name}.alz"));
            let result=preview_archive_text(&path,"docs/readme.txt",None).await.unwrap();
            assert_eq!(result.content,content,"{name}");assert!(!result.truncated);
            if name != "utf16" {assert_eq!(result.encoding,encoding);}
        }
        let original=preview_archive_text(&root.join("alz-152/bzip2.alz"),"a.txt",None).await.unwrap();
        assert_eq!(original.content,"ABCDEFG the quick brown fox 0123456789\n".repeat(60));
        // Actual original ALZip password/native Deflate groups retained by v1.5.12.
        for group in ["m1-key-single","m2-key-volumes"] {
            let extension=if group.ends_with("volumes"){ "a00" }else{ "alz" };
            let path=root.join(format!("alz-1512/originals/{group}/{group}.{extension}"));
            let result=preview_archive_text_with_encoding(&path,"文档/99说明.txt",Some("中文密码1512!"),Some(ZipPasswordEncoding::Cp936)).await.unwrap();
            assert_eq!(result.content,"原厂 ALZ 普通压缩流；完整内容与结束位置都必须核对。\r\n".repeat(500));
        }
    }
    #[tokio::test]
    async fn alz_text169_limits_bad_content_and_original_errors_stay_distinct() {
        let root=fixtures();
        for (name,reason) in [("crc-damage","CRC"),("binary","binary")] {
            let result=preview_archive_text(&root.join(format!("alz-text-169/{name}.alz")),"docs/readme.txt",None).await.unwrap_err();
            assert!(format!("{result:#}").to_lowercase().contains(&reason.to_lowercase()),"{result:#}");
        }
        // v1.6.11 intentionally supports large text as an explicitly partial prefix.
        let large=preview_archive_text(&root.join("alz-text-169/too-large.alz"),"docs/readme.txt",None).await.unwrap();
        assert!(large.truncated && large.byte_size==1024*1024);
        let error=preview_archive_text(&root.join("alz-bzip2-165/plain-missing-end-single.alz"),"a.txt",None).await.unwrap_err();
        assert!(format!("{error:#}").contains("缺少结束标记"));
        let path=root.join("alz-1512/originals/m1-key-single/m1-key-single.alz");
        let error=preview_archive_text_with_encoding(&path,"文档/99说明.txt",Some("wrong"),Some(ZipPasswordEncoding::Cp936)).await.unwrap_err();
        assert!(format!("{error:#}").contains("[alz-password]"));
    }
    #[tokio::test]
    async fn alz_text169_cancel_and_stale_source_never_return_text() {
        let root=fixtures();let dir=tempfile::tempdir().unwrap();let path=dir.path().join("source.alz");
        std::fs::copy(root.join("alz-text-169/store.alz"),&path).unwrap();
        let expected=super::super::image_reader::revision(&path).unwrap();
        let cancelled=std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let error=preview_archive_text_controlled_at(&path,"docs/readme.txt",None,None,None,cancelled,Some(expected.clone())).await.unwrap_err();
        assert!(format!("{error:#}").contains("取消"));
        use std::io::Write;
        std::fs::OpenOptions::new().append(true).open(&path).unwrap().write_all(b"changed").unwrap();
        let error=preview_archive_text_controlled_at(&path,"docs/readme.txt",None,None,None,std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),Some(expected)).await.unwrap_err();
        assert!(format!("{error:#}").contains("IMAGE_READER_CHANGED"));
    }
}

#[cfg(test)]
mod alz_image1610_tests {
    use super::*;
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    fn root() -> std::path::PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/alz-image-1610") }
    #[tokio::test]
    async fn alz_image1610_complete_images_only_read_the_selected_entry() {
        use sha2::{Digest,Sha256};
        let root=root();
        let manifest:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap();
        for (file,password) in [("store.alz",None),("deflate.alz",None),("bzip2.alz",None),("password.alz",Some("图片密码1610!")),("password-volumes.a00",Some("图片密码1610!"))] {
            for (entry,width,height) in [("相册/01-封面.png",320,240),("相册/02-网格.bmp",256,256)] {
                let preview=preview_archive_image_with_encoding(&root.join(file),entry,password,Some(ZipPasswordEncoding::Utf8)).await.unwrap();
                assert_eq!((preview.width,preview.height),(width,height),"{file}");
                let bytes=general_purpose::STANDARD.decode(preview.data_url.split_once(',').unwrap().1).unwrap();
                let expected=manifest["images"].as_array().unwrap().iter().find(|item|item["path"]==entry).unwrap();
                assert_eq!(bytes.len() as u64,expected["bytes"].as_u64().unwrap());
                assert_eq!(format!("{:x}",Sha256::digest(&bytes)),expected["sha256"].as_str().unwrap());
            }
        }
        let exact=preview_archive_image(&root.join("at-limit.alz"),"preview.bmp",None).await.unwrap();
        assert_eq!(exact.byte_size,8*1024*1024);
    }
    #[tokio::test]
    async fn alz_image1610_bad_entries_never_become_finished_images() {
        for (file,reason) in [("crc","CRC"),("missing-end","结束"),("not-image","not a supported raster"),("oversize","8 MiB"),("dimensions","dimensions exceed")] {
            let error=preview_archive_image(&root().join(format!("{file}.alz")),"preview.png",None).await.unwrap_err();
            assert!(format!("{error:#}").contains(reason),"{file}: {error:#}");
        }
        let error=preview_archive_image_with_encoding(&root().join("password-volumes.a00"),"相册/01-封面.png",Some("wrong"),Some(ZipPasswordEncoding::Utf8)).await.unwrap_err();
        assert!(format!("{error:#}").contains("[alz-password]"));
    }
    #[tokio::test]
    async fn alz_image1610_reader_cancellation_and_source_change_stop_delivery() {
        let dir=tempfile::tempdir().unwrap();let path=dir.path().join("source.alz");
        std::fs::copy(root().join("ui.alz"),&path).unwrap();
        let revision=super::super::image_reader::revision(&path).unwrap();
        let flag=Arc::new(AtomicBool::new(false));let signal=flag.clone();
        let control=ImageControl{flag,observer:Arc::new(move|event|if event.phase=="opening"{signal.store(true,Ordering::Release)})};
        let error=super::super::image_reader::read(&path,"gallery/01-cover.png",None,None,None,&revision,control).await.unwrap_err();
        assert!(format!("{error:#}").contains("IMAGE_READER_CANCELLED"));
        let changed=path.clone();
        let control=ImageControl{observer:Arc::new(move|event|if event.phase=="decoding"{std::fs::write(&changed,b"changed source").unwrap()}),..Default::default()};
        let error=super::super::image_reader::read(&path,"gallery/01-cover.png",None,None,None,&revision,control).await.unwrap_err();
        assert!(format!("{error:#}").contains("IMAGE_READER_CHANGED"));
        let error=preview_archive_image_bound_at(&path,"gallery/01-cover.png",None,None,None,Default::default(),Some(revision)).await.unwrap_err();
        assert!(format!("{error:#}").contains("IMAGE_READER_CHANGED"));
    }
}

#[cfg(test)]
mod alz_prefix1611_tests {
    use super::*;
    use std::sync::{Arc, atomic::AtomicBool};
    fn fixtures()->std::path::PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives")}
    #[tokio::test]
    async fn alz_prefix1611_methods_native_password_and_character_boundaries(){
        let root=fixtures().join("alz-prefix-1611");
        for name in ["store.alz","deflate.alz","bzip2.alz","password.alz","password-volumes.a00"] {
            let key=name.starts_with("password").then_some("日志密码1611!");
            let value=preview_archive_text_with_encoding(&root.join(name),"logs/session.txt",key,Some(ZipPasswordEncoding::Utf8)).await.unwrap();
            assert!(value.truncated && value.byte_size<=1024*1024 && value.byte_size>1024*1024-4,"{name}");
            assert!(value.content.starts_with("INFO 2026-10-23"));assert!(!value.content.contains('�'));assert_eq!(value.encoding,"UTF-8");
        }
        for (name,size,encoding) in [("utf8-boundary",1024*1024-1,"UTF-8"),("utf16-boundary",1024*1024-2,"UTF-16LE"),("gbk-boundary",1024*1024-1,"GBK")] {
            let value=preview_archive_text(&root.join(format!("{name}.alz")),"logs/session.txt",None).await.unwrap();
            assert!(value.truncated && !value.content.contains('�'));assert_eq!(value.byte_size,size,"{name}");assert_eq!(value.encoding,encoding,"{name}");
        }
    }
    #[tokio::test]
    async fn alz_prefix1611_partial_content_never_relaxes_full_integrity_or_other_previews(){
        let path=fixtures().join("alz-prefix-1611/bad-crc-later.alz");
        let value=preview_archive_text(&path,"logs/session.txt",None).await.unwrap();assert!(value.truncated);
        let options=crate::models::compression::DecompressOptions{selected_entries:vec!["logs/session.txt".into()],..Default::default()};
        let error=super::super::alz_engine::verify(&path,None,&options,Arc::new(AtomicBool::new(false))).await.unwrap_err();assert!(format!("{error:#}").contains("CRC"));
        let error=preview_archive_text(&fixtures().join("alz-prefix-1611/short-before-prefix.alz"),"logs/session.txt",None).await.unwrap_err();assert!(format!("{error:#}").contains("声明大小不符"));
        let text=preview_archive_text(&fixtures().join("alz-text-169/store.alz"),"docs/readme.txt",None).await.unwrap();assert!(!text.truncated && text.content.contains("包内说明"));
        let image=preview_archive_image_with_encoding(&fixtures().join("alz-image-1610/password.alz"),"相册/01-封面.png",Some("图片密码1610!"),Some(ZipPasswordEncoding::Utf8)).await.unwrap();assert_eq!((image.width,image.height),(320,240));
    }
    #[tokio::test]
    async fn alz_prefix1611_cancel_and_replaced_source_do_not_deliver_a_prefix(){
        let dir=tempfile::tempdir().unwrap();let path=dir.path().join("logs.alz");std::fs::copy(fixtures().join("alz-prefix-1611/deflate.alz"),&path).unwrap();
        let version=super::super::image_reader::revision(&path).unwrap();
        let error=preview_archive_text_controlled_at(&path,"logs/session.txt",None,None,None,Arc::new(AtomicBool::new(true)),Some(version.clone())).await.unwrap_err();assert!(format!("{error:#}").contains("取消"));
        std::fs::write(&path,b"replaced source").unwrap();
        let error=preview_archive_text_controlled_at(&path,"logs/session.txt",None,None,None,Arc::new(AtomicBool::new(false)),Some(version)).await.unwrap_err();assert!(format!("{error:#}").contains("IMAGE_READER_CHANGED"));
    }
    #[tokio::test]
    async fn alz_prefix1611_vault_uses_only_the_prefix_without_marking_full_password_success(){
        use crate::services::{compression_service::{CompressionService,CompressionServiceConfig},encrypted_password_service::EncryptedPasswordService,io_buffer_pool::IOBufferPool,rar_support::RarSupportService,universal_engine::UniversalCliEngine};
        use crate::models::password::{PasswordEntry,PasswordCategory};
        let dir=tempfile::tempdir().unwrap();let mut vault=EncryptedPasswordService::new(dir.path());vault.initialize("public-owned-test-key").await.unwrap();
        for (name,key) in [("a wrong","wrong"),("b public","日志密码1611!")] {vault.add_password(PasswordEntry::new(name.into(),key.into(),PasswordCategory::Other)).await.unwrap();}
        let service=CompressionService::new(CompressionServiceConfig::default(),Arc::new(IOBufferPool::default()),Arc::new(RarSupportService::new()),Arc::new(UniversalCliEngine::new()),Arc::new(vault));
        let path=fixtures().join("alz-prefix-1611/password-volumes.a00");
        let value=service.preview_alz_text_prefix_with_vault(path.to_str().unwrap(),"logs/session.txt",None,Some(ZipPasswordEncoding::Utf8),true,None).await.unwrap();assert!(value.truncated && value.content.starts_with("INFO"));
        assert!(service.encrypted_password_service.list_passwords().await.unwrap().iter().all(|e|e.use_count==0));
    }
}

#[cfg(test)]
mod egg_text1612_tests{
    use super::*;
    use std::sync::{Arc,atomic::AtomicBool};
    fn root()->std::path::PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives")}
    #[tokio::test]
    async fn egg_text1612_original_methods_keys_and_native_volumes_match_frozen_payloads(){
        let sources=[("egg-153/upstream/store.egg","hello.txt",None,None),("egg-153/upstream/deflate5.egg","text.txt",None,None),("egg-153/upstream/lzma5.egg","text.txt",None,None),("egg-153/upstream/enc_zip20_bzip2.egg","text.txt",Some("test1234"),None)];
        for (file,entry,key,encoding) in sources{
            let preview=preview_archive_text_with_encoding(&root().join(file),entry,key,encoding).await.unwrap();assert!(!preview.truncated&&preview.byte_size==preview.total_size&&!preview.content.is_empty());
        }
        for cipher in ["zipcrypto","aes256","lea256"]{
            let name=format!("ordinary-{cipher}-m3-volumes");let path=root().join(format!("egg-1511/originals/{name}/{name}.vol3.egg"));
            let value=preview_archive_text_with_encoding(&path,"文档/99最后.txt",Some("中文密码1511!"),Some(ZipPasswordEncoding::Cp936)).await.unwrap();
            use sha2::Digest;assert_eq!(format!("{:x}",sha2::Sha256::digest(value.content.as_bytes())),"c729bf41ca7329485476070dc259d161e10863fa0beeed825f725375b7a2bbe3");assert!(!value.truncated);
        }
        for name in ["empty","exact-limit","plain"]{let value=preview_archive_text(&root().join(format!("egg-text-1612/{name}.egg")),"readme.txt",None).await.unwrap();assert!(!value.truncated);if name=="exact-limit"{assert_eq!(value.byte_size,1048576);}}
    }
    #[tokio::test]
    async fn egg_text1612_bad_content_solid_limits_and_wrong_keys_never_display_as_complete(){
        for (name,reason) in [("crc-damage","CRC"),("trailing-deflate","校验失败"),("short-deflate","校验失败"),("binary","binary"),("utf16-odd","编码末尾不完整")]{
            let error=preview_archive_text(&root().join(format!("egg-text-1612/{name}.egg")),"readme.txt",None).await.unwrap_err();assert!(format!("{error:#}").contains(reason),"{name}: {error:#}");
        }
        let value=preview_archive_text(&root().join("egg-153/upstream/solid.egg"),"hello.txt",None).await.unwrap();assert!(!value.truncated && value.total_size==45);
        let error=preview_archive_text(&root().join("../../../test-results/egg-text-1612/input-limit.egg"),"readme.txt",None).await.unwrap_err();assert!(format!("{error:#}").contains("64 MiB"),"{error:#}");
        let path=root().join("egg-153/upstream/enc_zip20_bzip2.egg");let error=preview_archive_text(&path,"text.txt",Some("wrong")).await.unwrap_err();assert!(format!("{error:#}").contains("[egg-password]"));
    }
    #[tokio::test]
    async fn egg_text1612_cancel_replacement_and_missing_member_preserve_the_original_scope(){
        let dir=tempfile::tempdir().unwrap();let path=dir.path().join("text.egg");std::fs::copy(root().join("egg-text-1612/plain.egg"),&path).unwrap();let version=super::super::image_reader::revision(&path).unwrap();
        let error=preview_archive_text_controlled_at(&path,"readme.txt",None,None,None,Arc::new(AtomicBool::new(true)),Some(version.clone())).await.unwrap_err();assert!(format!("{error:#}").contains("取消"));
        std::fs::write(&path,b"changed source").unwrap();let error=preview_archive_text_controlled_at(&path,"readme.txt",None,None,None,Arc::new(AtomicBool::new(false)),Some(version)).await.unwrap_err();assert!(format!("{error:#}").contains("IMAGE_READER_CHANGED"));
        for n in 1..=3{let name=format!("split_store.vol{n}.egg");std::fs::copy(root().join("egg-153/upstream").join(&name),dir.path().join(&name)).unwrap();}
        let member=dir.path().join("split_store.vol3.egg");let version=super::super::image_reader::revision(&member).unwrap();std::fs::remove_file(dir.path().join("split_store.vol2.egg")).unwrap();
        let error=preview_archive_text_controlled_at(&member,"hello.txt",None,None,None,Arc::new(AtomicBool::new(false)),Some(version)).await.unwrap_err();assert!(format!("{error:#}").contains("IMAGE_READER_CHANGED"));
    }
    #[tokio::test]
    async fn egg_text1612_vault_checks_selected_text_without_validating_unrelated_data(){
        use crate::services::{compression_service::{CompressionService,CompressionServiceConfig},encrypted_password_service::EncryptedPasswordService,io_buffer_pool::IOBufferPool,rar_support::RarSupportService,universal_engine::UniversalCliEngine};
        use crate::models::password::{PasswordEntry,PasswordCategory};
        let dir=tempfile::tempdir().unwrap();let mut vault=EncryptedPasswordService::new(dir.path());vault.initialize("public-owned-test-key").await.unwrap();
        for (name,key) in [("a wrong","wrong"),("b original","中文密码1511!")]{vault.add_password(PasswordEntry::new(name.into(),key.into(),PasswordCategory::Other)).await.unwrap();}
        let service=CompressionService::new(CompressionServiceConfig::default(),Arc::new(IOBufferPool::default()),Arc::new(RarSupportService::new()),Arc::new(UniversalCliEngine::new()),Arc::new(vault));
        let path=root().join("egg-1511/originals/ordinary-lea256-m3-volumes/ordinary-lea256-m3-volumes.vol3.egg");
        let value=service.preview_egg_text_with_vault(path.to_str().unwrap(),"文档/99最后.txt",None,Some(ZipPasswordEncoding::Cp936),None).await.unwrap();assert_eq!(value.byte_size,22500);assert!(!value.truncated);
        let path=root().join("egg-text-1612/plain.egg");let value=service.preview_egg_text_with_vault(path.to_str().unwrap(),"readme.txt",None,None,None).await.unwrap();assert!(value.content.contains("保持双栏"));
    }
}

#[cfg(test)]
mod egg_image1613_tests{
    use super::*;use std::sync::{Arc,atomic::AtomicBool};
    fn fixtures()->std::path::PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives")}
    fn root()->std::path::PathBuf{fixtures().join("egg-image-1613")}
    #[tokio::test]
    async fn egg_image1613_methods_keys_and_native_members_match_exact_owned_pixels(){
        let m:serde_json::Value=serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap();
        for name in ["store.egg","deflate.egg","bzip2.egg","lzma.egg","zipcrypto.egg","aes256.egg","password-volumes.vol2.egg"]{
            let encrypted=name.starts_with("zipcrypto")||name.starts_with("aes256")||name.starts_with("password");let entry=if encrypted{"相册/01-封面.png"}else{"gallery/01-cover.png"};let key=encrypted.then_some("图片密码1613!");
            let value=preview_archive_image_with_encoding(&root().join(name),entry,key,Some(ZipPasswordEncoding::Cp936)).await.unwrap();
            let bytes=base64::Engine::decode(&base64::engine::general_purpose::STANDARD,value.data_url.split(',').nth(1).unwrap()).unwrap();use sha2::Digest;
            assert_eq!(format!("{:x}",sha2::Sha256::digest(&bytes)),m["images"][0]["sha256"].as_str().unwrap(),"{name}");assert_eq!((value.width,value.height),(320,240));
        }
        let exact=preview_archive_image(&root().join("at-limit.egg"),"preview.bmp",None).await.unwrap();assert_eq!(exact.byte_size,8*1024*1024);assert_eq!((exact.width,exact.height),(256,256));
        let old=preview_archive_text(&fixtures().join("egg-text-1612/plain.egg"),"readme.txt",None).await.unwrap();assert!(!old.truncated&&old.content.contains("保持双栏"));
    }
    #[tokio::test]
    async fn egg_image1613_incomplete_damaged_solid_and_limits_remain_actionable(){
        for (file,reason) in [("crc","CRC"),("truncated","校验失败"),("trailing","校验失败"),("oversize","8 MiB"),("not-image","not a supported raster"),("dimensions","dimensions exceed")]{
            let error=preview_archive_image(&root().join(format!("{file}.egg")),"preview.png",None).await.unwrap_err();assert!(format!("{error:#}").contains(reason),"{file}: {error:#}");
        }
        let input=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/egg-image-1613/input-limit.egg");let error=preview_archive_image(&input,"preview.png",None).await.unwrap_err();assert!(format!("{error:#}").contains("64 MiB"));
        let error=preview_archive_image_with_encoding(&root().join("aes256.egg"),"相册/01-封面.png",Some("wrong"),Some(ZipPasswordEncoding::Cp936)).await.unwrap_err();assert!(format!("{error:#}").contains("[egg-password]"));
    }
    #[tokio::test]
    async fn egg_image1613_cancel_and_source_change_after_decode_never_publish_an_image(){
        let dir=tempfile::tempdir().unwrap();let path=dir.path().join("pictures.egg");std::fs::copy(root().join("deflate.egg"),&path).unwrap();let revision=super::super::image_reader::revision(&path).unwrap();
        let control=super::super::image_reader::Control{flag:Arc::new(AtomicBool::new(true)),..Default::default()};let error=preview_archive_image_bound_at(&path,"gallery/01-cover.png",None,None,None,control,Some(revision.clone())).await.unwrap_err();assert!(format!("{error:#}").contains("CANCELLED"));
        let changed=path.clone();let control=super::super::image_reader::Control{observer:Arc::new(move|event|{if event.phase=="decoding"{std::fs::write(&changed,b"replaced after payload read").unwrap();}}),..Default::default()};
        let error=preview_archive_image_bound_at(&path,"gallery/01-cover.png",None,None,None,control,Some(revision.clone())).await.unwrap_err();assert!(format!("{error:#}").contains("IMAGE_READER_CHANGED"));
        let error=preview_archive_text_controlled_at(&path,"readme.txt",None,None,None,Arc::new(AtomicBool::new(false)),Some(revision)).await.unwrap_err();assert!(format!("{error:#}").contains("IMAGE_READER_CHANGED"));
    }
    #[tokio::test]
    async fn egg_image1613_vault_confirms_only_selected_bounded_image(){
        use crate::services::{compression_service::{CompressionService,CompressionServiceConfig},encrypted_password_service::EncryptedPasswordService,io_buffer_pool::IOBufferPool,rar_support::RarSupportService,universal_engine::UniversalCliEngine};use crate::models::password::{PasswordEntry,PasswordCategory};
        let dir=tempfile::tempdir().unwrap();let mut vault=EncryptedPasswordService::new(dir.path());vault.initialize("public-owned-test-key").await.unwrap();
        for (name,key) in [("a wrong","wrong"),("b controlled","图片密码1613!")]{vault.add_password(PasswordEntry::new(name.into(),key.into(),PasswordCategory::Other)).await.unwrap();}
        let control=super::super::image_reader::Control::default();let mut service=CompressionService::new(CompressionServiceConfig::default(),Arc::new(IOBufferPool::default()),Arc::new(RarSupportService::new()),Arc::new(UniversalCliEngine::new()),Arc::new(vault));service.cancellation_flag=control.flag.clone();
        let path=root().join("password-volumes.vol2.egg");let value=service.preview_egg_image_with_vault(path.to_str().unwrap(),"相册/01-封面.png",None,Some(ZipPasswordEncoding::Cp936),None,control).await.unwrap();assert_eq!((value.width,value.height),(320,240));assert!(service.encrypted_password_service.list_passwords().await.unwrap().iter().all(|e|e.use_count==0));
        let path=root().join("solid.egg");let image=service.preview_egg_image_with_vault(path.to_str().unwrap(),"preview.png",None,None,None,Default::default()).await.unwrap();assert_eq!(image.width,320);
    }
}


#[cfg(test)]
mod egg_solid_text1614_tests {
    use super::*;
    use std::sync::{Arc, atomic::AtomicBool};
    use sha2::Digest;
    fn fixtures()->std::path::PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives") }
    fn controlled(name:&str)->std::path::PathBuf { fixtures().join(format!("egg-solid-text-1614/{name}.egg")) }
    #[tokio::test]
    async fn egg_solid_text1614_original_shared_deflate_lzma_and_empty_content() {
        for (file,entry,expected) in [
            ("egg-153/upstream/solid.egg","hello.txt","b47cc0f104b62d4c7c30bcd68fd8e67613e287dc4ad8c310ef10cbadea9c4380"),
            ("egg-154/original-alzip/solid-deflate.egg","文档/说明 文件.txt","fa0a640e110667fc47056120b2dd20a7faa377f49afb19af557ad1463f912021"),
            ("egg-154/original-alzip/solid-high.egg","文档/说明 文件.txt","fa0a640e110667fc47056120b2dd20a7faa377f49afb19af557ad1463f912021")
        ] {
            let value=preview_archive_text(&fixtures().join(file),entry,None).await.unwrap();
            assert!(!value.truncated && value.byte_size==value.total_size);
            assert_eq!(format!("{:x}",sha2::Sha256::digest(value.content.as_bytes())),expected);
        }
        let empty=preview_archive_text(&fixtures().join("egg-153/upstream/solid.egg"),"empty.txt",None).await.unwrap();
        assert!(empty.content.is_empty() && !empty.truncated && empty.total_size==0);
        let exact=preview_archive_text(&controlled("exact-limit"),"readme.txt",None).await.unwrap();
        assert!(!exact.truncated && exact.total_size==1024*1024);
    }
    #[tokio::test]
    async fn egg_solid_text1614_later_dependencies_and_limits_block_early_text() {
        for (name,reason) in [("shared-crc","CRC"),("shared-output-limit","64 MiB"),("selected-limit","1 MiB"),("unknown-dependency","Deflate / LZMA"),("encrypted-dependency","加密固实"),("truncated","校验失败"),("trailing","校验失败")] {
            let error=preview_archive_text(&controlled(name),"readme.txt",Some("irrelevant manual key")).await.unwrap_err();
            assert!(format!("{error:#}").contains(reason),"{name}: {error:#}");
        }
    }
    #[tokio::test]
    async fn egg_solid_text1614_controlled_native_group_cancel_change_and_missing_dependency() {
        let dir=tempfile::tempdir().unwrap();
        for n in 1..=3 { let name=format!("controlled-native.vol{n}");std::fs::copy(controlled(&name),dir.path().join(format!("{name}.egg"))).unwrap(); }
        let source=dir.path().join("controlled-native.vol2.egg");
        let version=super::super::image_reader::revision(&source).unwrap();
        let value=preview_archive_text_controlled_at(&source,"hello.txt",None,None,None,Arc::new(AtomicBool::new(false)),Some(version.clone())).await.unwrap();
        assert!(!value.truncated && value.total_size==45);
        let error=preview_archive_text_controlled_at(&source,"hello.txt",None,None,None,Arc::new(AtomicBool::new(true)),Some(version.clone())).await.unwrap_err();
        assert!(format!("{error:#}").contains("取消"));
        let absent=dir.path().join("controlled-native.vol3.egg");std::fs::remove_file(absent).unwrap();
        let error=preview_archive_text_controlled_at(&source,"hello.txt",None,None,None,Arc::new(AtomicBool::new(false)),Some(version)).await.unwrap_err();
        assert!(format!("{error:#}").contains("IMAGE_READER_CHANGED"));
        let error=preview_archive_text(&source,"hello.txt",None).await.unwrap_err();
        assert!(format!("{error:#}").contains("incomplete"));
    }
    #[tokio::test]
    async fn egg_solid_text1614_plain_and_encrypted_shared_paths_never_need_a_vault() {
        use crate::services::{compression_service::{CompressionService,CompressionServiceConfig},encrypted_password_service::EncryptedPasswordService,io_buffer_pool::IOBufferPool,rar_support::RarSupportService,universal_engine::UniversalCliEngine};
        let dir=tempfile::tempdir().unwrap();
        // Deliberately locked / uninitialized: plain solid preview must not require it.
        let vault=EncryptedPasswordService::new(dir.path());
        let service=CompressionService::new(CompressionServiceConfig::default(),Arc::new(IOBufferPool::default()),Arc::new(RarSupportService::new()),Arc::new(UniversalCliEngine::new()),Arc::new(vault));
        let value=service.preview_egg_text_with_vault(controlled("ui").to_str().unwrap(),"readme.txt",None,None,None).await.unwrap();
        assert!(!value.truncated && value.content.starts_with("固实包内说明"));
        let error=service.preview_egg_text_with_vault(controlled("encrypted-dependency").to_str().unwrap(),"readme.txt",None,None,None).await.unwrap_err();
        assert!(format!("{error:#}").contains("加密固实"));
        let error=service.preview_egg_text_with_vault(controlled("shared-output-limit").to_str().unwrap(),"readme.txt",None,None,None).await.unwrap_err();
        assert!(format!("{error:#}").contains("64 MiB"));
    }
}


#[cfg(test)]
mod egg_solid_image1615_tests {
    use super::*;
    use std::sync::{Arc,atomic::{AtomicBool,Ordering}};
    use sha2::Digest;
    fn root()->std::path::PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/egg-solid-image-1615") }
    fn source(name:&str)->std::path::PathBuf { root().join(format!("{name}.egg")) }
    fn manifest()->serde_json::Value { serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap() }
    fn image_sha(value:&ArchiveImagePreview)->String {
        let bytes=base64::Engine::decode(&base64::engine::general_purpose::STANDARD,value.data_url.split(',').nth(1).unwrap()).unwrap();
        format!("{:x}",sha2::Sha256::digest(bytes))
    }
    #[tokio::test]
    async fn egg_solid_image1615_shared_deflate_lzma_native_pages_match_exact_owned_pixels() {
        let m=manifest();
        for name in ["deflate","lzma","controlled-native.vol2"] {
            let p=source(name);
            for entry in m["images"].as_array().unwrap() {
                let value=preview_archive_image(&p,entry["path"].as_str().unwrap(),None).await.unwrap();
                assert_eq!(value.byte_size,entry["bytes"].as_u64().unwrap());
                assert_eq!(image_sha(&value),entry["sha256"].as_str().unwrap(),"{name}");
            }
        }
        let value=preview_archive_image(&source("exact-limit"),"preview.bmp",None).await.unwrap();
        assert_eq!(value.byte_size,8*1024*1024);assert_eq!(image_sha(&value),m["exactLimitSha256"].as_str().unwrap());
        // The image capture limit must not enlarge text capture.
        let error=preview_archive_text(&source("ui"),"gallery/02-grid.bmp",None).await.unwrap_err();
        assert!(format!("{error:#}").contains("1 MiB"));
    }
    #[tokio::test]
    async fn egg_solid_image1615_tail_crc_dependency_limits_and_encryption_never_display_partial_images() {
        for (name,entry,reason) in [
            ("shared-crc","gallery/01-cover.png","CRC"),
            ("shared-output-limit","gallery/01-cover.png","64 MiB"),
            ("encrypted-dependency","gallery/01-cover.png","加密固实图片"),
            ("unknown-dependency","gallery/01-cover.png","Deflate / LZMA"),
            ("truncated","gallery/01-cover.png","校验失败"),
            ("trailing","gallery/01-cover.png","校验失败"),
            ("oversize","preview.bmp","8 MiB"),
            ("dimensions","preview.png","dimensions exceed"),
            ("ui","gallery/00-broken.png","not a supported raster")
        ] {
            let error=preview_archive_image(&source(name),entry,None).await.unwrap_err();
            assert!(format!("{error:#}").contains(reason),"{name}: {error:#}");
        }
    }
    #[tokio::test]
    async fn egg_solid_image1615_cancel_after_shared_read_and_source_replacement_discard_the_page() {
        let p=source("ui");let revision=super::super::image_reader::revision(&p).unwrap();
        let flag=Arc::new(AtomicBool::new(false));let signal=flag.clone();
        let control=ImageControl{flag,observer:Arc::new(move|event|if event.phase=="decoding"{signal.store(true,Ordering::Release)})};
        let error=super::super::image_reader::read(&p,"gallery/02-grid.bmp",None,None,None,&revision,control).await.unwrap_err();
        assert!(format!("{error:#}").contains("IMAGE_READER_CANCELLED"));
        let dir=tempfile::tempdir().unwrap();let p=dir.path().join("changed.egg");std::fs::copy(source("lzma"),&p).unwrap();
        let revision=super::super::image_reader::revision(&p).unwrap();let changed=p.clone();
        let control=ImageControl{observer:Arc::new(move|event|if event.phase=="decoding"{std::fs::write(&changed,b"source replaced after shared stream read").unwrap()}),..Default::default()};
        let error=super::super::image_reader::read(&p,"gallery/01-cover.png",None,None,None,&revision,control).await.unwrap_err();
        assert!(format!("{error:#}").contains("IMAGE_READER_CHANGED"));
    }
    #[tokio::test]
    async fn egg_solid_image1615_locked_vault_middle_member_and_exact_extraction_keep_the_same_selection() {
        use crate::services::{compression_service::{CompressionService,CompressionServiceConfig},encrypted_password_service::EncryptedPasswordService,io_buffer_pool::IOBufferPool,rar_support::RarSupportService,universal_engine::UniversalCliEngine};
        let dir=tempfile::tempdir().unwrap();let vault=EncryptedPasswordService::new(dir.path());
        let service=CompressionService::new(CompressionServiceConfig::default(),Arc::new(IOBufferPool::default()),Arc::new(RarSupportService::new()),Arc::new(UniversalCliEngine::new()),Arc::new(vault));
        let p=source("controlled-native.vol2");let entry="gallery/02-grid.bmp";
        let value=service.preview_egg_image_with_vault(p.to_str().unwrap(),entry,None,None,None,Default::default()).await.unwrap();
        assert_eq!(image_sha(&value),manifest()["images"][1]["sha256"].as_str().unwrap());
        let error=service.preview_egg_image_with_vault(source("encrypted-dependency").to_str().unwrap(),"gallery/01-cover.png",None,None,None,Default::default()).await.unwrap_err();
        assert!(format!("{error:#}").contains("加密固实图片"));
        let opts=crate::models::compression::DecompressOptions{selected_entries:vec![entry.into()],..Default::default()};
        let out=tempfile::tempdir().unwrap();
        super::super::egg_engine::extract(&p,out.path(),None,&opts,Arc::new(AtomicBool::new(false))).await.unwrap();
        assert_eq!(format!("{:x}",sha2::Sha256::digest(std::fs::read(out.path().join(entry)).unwrap())),manifest()["images"][1]["sha256"].as_str().unwrap());
        assert!(!out.path().join("gallery/01-cover.png").exists() && !out.path().join("tail.bin").exists());
        for n in [1,3] {std::fs::copy(source(&format!("controlled-native.vol{n}")),dir.path().join(format!("controlled-native.vol{n}.egg"))).unwrap();}
        let error=service.preview_egg_image_with_vault(dir.path().join("controlled-native.vol3.egg").to_str().unwrap(),"gallery/01-cover.png",None,None,None,Default::default()).await.unwrap_err();
        assert!(format!("{error:#}").contains("incomplete"));
    }
}


#[cfg(test)]
mod egg_prefix1616_tests {
    use super::*;
    use std::sync::{Arc,atomic::AtomicBool};
    fn fixtures()->std::path::PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives")}
    fn source(name:&str)->std::path::PathBuf{fixtures().join(format!("egg-prefix-1616/{name}.egg"))}
    #[tokio::test]
    async fn egg_prefix1616_methods_keys_native_and_character_boundaries_are_explicitly_partial(){
        for name in ["store","deflate","bzip2","lzma","zipcrypto","aes256","password-volumes.vol2"]{
            let key=matches!(name,"zipcrypto"|"aes256"|"password-volumes.vol2").then_some("日志密码1616!");
            let value=preview_archive_text_with_encoding(&source(name),"logs/session.txt",key,Some(ZipPasswordEncoding::Cp936)).await.unwrap();
            assert!(value.truncated && value.byte_size<=1024*1024 && value.byte_size>1024*1024-4 && value.total_size>value.byte_size);
            assert!(value.content.starts_with("INFO 2026-10-23") && !value.content.contains('�'));
        }
        for (name,bytes,encoding) in [("utf8-boundary",1048575,"UTF-8"),("utf16-boundary",1048574,"UTF-16LE"),("gbk-boundary",1048575,"GBK")]{
            let value=preview_archive_text(&source(name),"logs/session.txt",None).await.unwrap();
            assert!(value.truncated && !value.content.contains('�'));assert_eq!(value.byte_size,bytes,"{name}");assert_eq!(value.encoding,encoding,"{name}");
        }
    }
    #[tokio::test]
    async fn egg_prefix1616_partial_bytes_never_replace_complete_integrity_or_small_content(){
        let p=source("bad-crc-later");let value=preview_archive_text(&p,"logs/session.txt",None).await.unwrap();assert!(value.truncated);
        let opts=crate::models::compression::DecompressOptions{selected_entries:vec!["logs/session.txt".into()],..Default::default()};
        let error=super::super::egg_engine::verify(&p,None,&opts,Arc::new(AtomicBool::new(false))).await.unwrap_err();assert!(format!("{error:#}").contains("CRC"));
        for (name,reason) in [("short-before-prefix","大小或 CRC"),("bad-first-block","CRC"),("unknown-method","目前支持"),("binary","binary")]{
            let error=preview_archive_text(&source(name),"logs/session.txt",None).await.unwrap_err();assert!(format!("{error:#}").contains(reason),"{name}: {error:#}");
        }
        let small=preview_archive_text(&fixtures().join("egg-153/upstream/solid.egg"),"hello.txt",None).await.unwrap();assert!(!small.truncated && small.total_size==45);
        let error=preview_archive_text(&fixtures().join("egg-solid-image-1615/ui.egg"),"gallery/02-grid.bmp",None).await.unwrap_err();assert!(format!("{error:#}").contains("1 MiB"));
    }
    #[tokio::test]
    async fn egg_prefix1616_cancel_stale_group_and_missing_members_discard_the_prefix(){
        let dir=tempfile::tempdir().unwrap();
        for n in 1..=3{let name=format!("password-volumes.vol{n}.egg");std::fs::copy(source(&format!("password-volumes.vol{n}")),dir.path().join(name)).unwrap();}
        let p=dir.path().join("password-volumes.vol2.egg");let revision=super::super::image_reader::revision(&p).unwrap();
        let error=preview_archive_text_controlled_at(&p,"logs/session.txt",None,None,None,Arc::new(AtomicBool::new(true)),Some(revision.clone())).await.unwrap_err();assert!(format!("{error:#}").contains("取消"));
        std::fs::remove_file(dir.path().join("password-volumes.vol3.egg")).unwrap();
        let error=preview_archive_text_controlled_at(&p,"logs/session.txt",None,None,None,Arc::new(AtomicBool::new(false)),Some(revision)).await.unwrap_err();assert!(format!("{error:#}").contains("IMAGE_READER_CHANGED"));
        let error=preview_archive_text(&p,"logs/session.txt",None).await.unwrap_err();assert!(format!("{error:#}").contains("incomplete"));
    }
    #[tokio::test]
    async fn egg_prefix1616_vault_uses_only_the_selected_prefix_without_marking_complete_password_success(){
        use crate::services::{compression_service::{CompressionService,CompressionServiceConfig},encrypted_password_service::EncryptedPasswordService,io_buffer_pool::IOBufferPool,rar_support::RarSupportService,universal_engine::UniversalCliEngine};
        use crate::models::password::{PasswordEntry,PasswordCategory};
        let dir=tempfile::tempdir().unwrap();let mut vault=EncryptedPasswordService::new(dir.path());vault.initialize("owned-public-test-key").await.unwrap();
        for(name,key)in[("a wrong","wrong"),("b public","日志密码1616!")]{vault.add_password(PasswordEntry::new(name.into(),key.into(),PasswordCategory::Other)).await.unwrap();}
        let service=CompressionService::new(CompressionServiceConfig::default(),Arc::new(IOBufferPool::default()),Arc::new(RarSupportService::new()),Arc::new(UniversalCliEngine::new()),Arc::new(vault));
        let p=source("password-volumes.vol2");let value=service.preview_egg_text_with_vault(p.to_str().unwrap(),"logs/session.txt",None,Some(ZipPasswordEncoding::Cp936),None).await.unwrap();
        assert!(value.truncated && value.content.starts_with("INFO"));
        assert!(service.encrypted_password_service.list_passwords().await.unwrap().iter().all(|entry|entry.use_count==0));
        let error=service.preview_egg_text_with_vault(p.to_str().unwrap(),"logs/session.txt",Some("wrong manual"),Some(ZipPasswordEncoding::Cp936),None).await.unwrap_err();assert!(format!("{error:#}").contains("密码"));
    }
}
