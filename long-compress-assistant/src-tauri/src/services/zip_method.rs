//! ZIP's optional output method. ZIPX creation uses XZ explicitly.
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ZipMethod {
    #[default]
    Deflate,
    Zstd,
    Bzip2,
    Ppmd,
    Lzma,
}

pub(crate) fn file_options(zipx: bool, method: ZipMethod, level: u32) -> zip_aes::write::SimpleFileOptions {
    let (method, level) = if zipx {
        (zip_aes::CompressionMethod::Xz, Some(level.min(9) as i64))
    } else if method == ZipMethod::Lzma {
        (zip_aes::CompressionMethod::Lzma, Some(level.clamp(1, 9) as i64))
    } else if method == ZipMethod::Ppmd {
        (zip_aes::CompressionMethod::Ppmd, Some(level.clamp(1, 9) as i64))
    } else if method == ZipMethod::Bzip2 {
        (zip_aes::CompressionMethod::Bzip2, Some(level.clamp(1, 9) as i64))
    } else if method == ZipMethod::Zstd {
        // The existing 0–9 strength control stays consistent across all entry points.
        (zip_aes::CompressionMethod::Zstd, Some(level.clamp(1, 9) as i64))
    } else if level == 0 {
        (zip_aes::CompressionMethod::Stored, None)
    } else {
        (zip_aes::CompressionMethod::Deflated, Some(level.min(9) as i64))
    };
    zip_aes::write::SimpleFileOptions::default().compression_method(method).compression_level(level)
}
