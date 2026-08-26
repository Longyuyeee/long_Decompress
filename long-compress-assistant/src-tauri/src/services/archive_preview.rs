use crate::services::archive_format::ArchiveFormat;
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
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
}

impl<R> ScanLimitedReader<R> {
    fn new(inner: R, max_bytes: u64) -> Self {
        Self {
            inner,
            remaining: max_bytes,
        }
    }
}

impl<R: Read> Read for ScanLimitedReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
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
        Ok(read)
    }
}

fn read_bounded(reader: &mut dyn Read, expected_bytes: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(MAX_PREVIEW_BYTES + 1)
        .read_to_end(&mut bytes)
        .context("Unable to read the selected image entry")?;
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

fn read_zip_entry(path: &Path, entry_path: &str, password: Option<&str>) -> Result<Vec<u8>> {
    let mut archive = zip_aes::ZipArchive::new(File::open(path)?)?;
    let mut entry = match password.filter(|value| !value.is_empty()) {
        Some(password) => archive.by_name_decrypt(entry_path, password.as_bytes())?,
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
    read_bounded(&mut entry, expected_bytes)
}

fn read_zip_text_entry(path: &Path, entry_path: &str, password: Option<&str>) -> Result<TextBytes> {
    let mut archive = zip_aes::ZipArchive::new(File::open(path)?)?;
    let mut entry = match password.filter(|value| !value.is_empty()) {
        Some(password) => archive.by_name_decrypt(entry_path, password.as_bytes())?,
        None => archive.by_name(entry_path)?,
    };
    if entry.is_dir() {
        anyhow::bail!("Directories cannot be previewed");
    }
    let total_size = entry.size();
    read_text_prefix(&mut entry, total_size)
}

fn read_tar_entry<R: Read>(reader: R, entry_path: &str) -> Result<Vec<u8>> {
    let reader = ScanLimitedReader::new(reader, MAX_TAR_PREVIEW_SCAN_BYTES);
    let mut archive = tar::Archive::new(reader);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().replace('\\', "/");
        if path != entry_path {
            continue;
        }
        if !entry.header().entry_type().is_file() {
            anyhow::bail!("Only regular image files can be previewed");
        }
        let expected_bytes = entry.size();
        if expected_bytes > MAX_PREVIEW_BYTES {
            anyhow::bail!(
                "Image preview is limited to {} MiB after decompression",
                MAX_PREVIEW_BYTES / 1024 / 1024
            );
        }
        return read_bounded(&mut entry, expected_bytes);
    }
    anyhow::bail!("The selected image entry was not found in the archive")
}

fn read_tar_text_entry<R: Read>(reader: R, entry_path: &str) -> Result<TextBytes> {
    let reader = ScanLimitedReader::new(reader, MAX_TAR_PREVIEW_SCAN_BYTES);
    let mut archive = tar::Archive::new(reader);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().replace('\\', "/");
        if path != entry_path {
            continue;
        }
        if !entry.header().entry_type().is_file() {
            anyhow::bail!("Only regular text files can be previewed");
        }
        let total_size = entry.size();
        return read_text_prefix(&mut entry, total_size);
    }
    anyhow::bail!("The selected text entry was not found in the archive")
}

fn preview_format(path: &Path) -> Result<&'static str> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if name.ends_with(".tar") || name.ends_with(".ova") {
        return Ok("TAR");
    }
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") || name.ends_with(".tpz") {
        return Ok("TAR.GZ");
    }
    if name.ends_with(".tar.bz2") || name.ends_with(".tbz") || name.ends_with(".tbz2") {
        return Ok("TAR.BZ2");
    }
    if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        return Ok("TAR.XZ");
    }
    if name.ends_with(".tar.zst") || name.ends_with(".tzst") {
        return Ok("TAR.ZST");
    }

    let mut header = [0u8; 560];
    let mut file = File::open(path)?;
    let read = file.read(&mut header)?;
    if ArchiveFormat::from_magic(&header[..read]) == ArchiveFormat::Zip {
        return Ok("ZIP");
    }
    anyhow::bail!(
        "Safe bounded preview is currently available for ZIP and TAR-family archives only"
    )
}

fn read_tar_family(path: &Path, format: &str, entry_path: &str) -> Result<Vec<u8>> {
    match format {
        "TAR" => read_tar_entry(File::open(path)?, entry_path),
        "TAR.GZ" => read_tar_entry(flate2::read::GzDecoder::new(File::open(path)?), entry_path),
        "TAR.BZ2" => read_tar_entry(bzip2::read::BzDecoder::new(File::open(path)?), entry_path),
        "TAR.XZ" => read_tar_entry(xz2::read::XzDecoder::new(File::open(path)?), entry_path),
        "TAR.ZST" => read_tar_entry(
            zstd::stream::read::Decoder::new(File::open(path)?)?,
            entry_path,
        ),
        _ => anyhow::bail!(
            "Safe bounded preview is currently available for ZIP and TAR-family archives only"
        ),
    }
}

fn read_tar_family_text(path: &Path, format: &str, entry_path: &str) -> Result<TextBytes> {
    match format {
        "TAR" => read_tar_text_entry(File::open(path)?, entry_path),
        "TAR.GZ" => {
            read_tar_text_entry(flate2::read::GzDecoder::new(File::open(path)?), entry_path)
        }
        "TAR.BZ2" => {
            read_tar_text_entry(bzip2::read::BzDecoder::new(File::open(path)?), entry_path)
        }
        "TAR.XZ" => read_tar_text_entry(xz2::read::XzDecoder::new(File::open(path)?), entry_path),
        "TAR.ZST" => read_tar_text_entry(
            zstd::stream::read::Decoder::new(File::open(path)?)?,
            entry_path,
        ),
        _ => anyhow::bail!(
            "Safe bounded text preview is currently available for ZIP and TAR-family archives only"
        ),
    }
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
) -> Result<ArchiveImagePreview> {
    let entry_path = normalized_entry_path(entry_path)?;
    let format = preview_format(archive_path)?;
    let bytes = match format {
        "ZIP" => read_zip_entry(archive_path, &entry_path, password)?,
        format if format.starts_with("TAR") => read_tar_family(archive_path, format, &entry_path)?,
        _ => anyhow::bail!(
            "Safe bounded preview is currently available for ZIP and TAR-family archives only"
        ),
    };
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
    let archive_path = archive_path.to_path_buf();
    let entry_path = entry_path.to_string();
    let password = password.map(str::to_string);
    tauri::async_runtime::spawn_blocking(move || {
        preview_archive_image_sync(&archive_path, &entry_path, password.as_deref())
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
    for pair in bytes.chunks_exact(2) {
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

fn preview_archive_text_sync(
    archive_path: &Path,
    entry_path: &str,
    password: Option<&str>,
) -> Result<ArchiveTextPreview> {
    let entry_path = normalized_entry_path(entry_path)?;
    let format = preview_format(archive_path)?;
    let text_bytes = match format {
        "ZIP" => read_zip_text_entry(archive_path, &entry_path, password)?,
        format if format.starts_with("TAR") => {
            read_tar_family_text(archive_path, format, &entry_path)?
        }
        _ => anyhow::bail!(
            "Safe bounded text preview is currently available for ZIP and TAR-family archives only"
        ),
    };
    let (content, encoding, consumed) = decode_text_prefix(&text_bytes.bytes)?;
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
    let archive_path = archive_path.to_path_buf();
    let entry_path = entry_path.to_string();
    let password = password.map(str::to_string);
    tauri::async_runtime::spawn_blocking(move || {
        preview_archive_text_sync(&archive_path, &entry_path, password.as_deref())
    })
    .await
    .context("Archive text preview worker failed")?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

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
    async fn refuses_routes_without_a_provable_bounded_reader() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("images.7z");
        let source = temp.path().join("preview.png");
        std::fs::write(&source, PNG_1X1).unwrap();
        sevenz_rust::compress_to_path(&source, &archive).unwrap();

        let entry_name = source.file_name().unwrap().to_string_lossy();
        let error = preview_archive_image(&archive, &entry_name, None)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("ZIP and TAR-family"));
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
    async fn refuses_text_routes_without_a_provable_bounded_reader() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("text.7z");
        let source = temp.path().join("readme.txt");
        std::fs::write(&source, b"bounded route required").unwrap();
        sevenz_rust::compress_to_path(&source, &archive).unwrap();

        let error = preview_archive_text(&archive, "readme.txt", None)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("ZIP and TAR-family"));
    }
}
