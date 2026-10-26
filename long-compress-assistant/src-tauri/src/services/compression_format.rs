use std::path::Path;

use anyhow::Result;

use crate::models::compression::CompressionOptions;

use super::compression_service::CompressionError;

#[derive(Clone, Copy, Debug)]
pub struct CompressionFormatCapability {
    pub format: &'static str,
    pub extensions: &'static [&'static str],
    pub can_compress: bool,
    pub can_extract: bool,
    pub supports_password_compress: bool,
    pub supports_password_extract: bool,
    pub single_file_only: bool,
    pub supports_split: bool,
    pub requires_7za: bool,
    pub requires_winrar: bool,
}

pub const COMPRESSION_FORMAT_CAPABILITIES: &[CompressionFormatCapability] = &[
    CompressionFormatCapability { format:"tar.lzo", extensions:&["tar.lzo"], can_compress:false, can_extract:true, supports_password_compress:false, supports_password_extract:false, single_file_only:false, supports_split:false, requires_7za:false, requires_winrar:false },
    CompressionFormatCapability { format:"lzo", extensions:&["lzo"], can_compress:false, can_extract:true, supports_password_compress:false, supports_password_extract:false, single_file_only:false, supports_split:false, requires_7za:false, requires_winrar:false },
    CompressionFormatCapability { format: "swm", extensions: &["swm"], can_compress: true, can_extract: true, supports_password_compress: false, supports_password_extract: false, single_file_only: false, supports_split: true, requires_7za: true, requires_winrar: false },
    CompressionFormatCapability { format: "tar.lzma.aes", extensions: &["tar.lzma.aes"], can_compress: true, can_extract: true, supports_password_compress: true, supports_password_extract: true, single_file_only: false, supports_split: true, requires_7za: false, requires_winrar: false },
    CompressionFormatCapability { format: "lzma.aes", extensions: &["lzma.aes"], can_compress: true, can_extract: true, supports_password_compress: true, supports_password_extract: true, single_file_only: true, supports_split: true, requires_7za: false, requires_winrar: false },
    CompressionFormatCapability { format: "tar.lzma", extensions: &["tar.lzma"], can_compress: true, can_extract: true, supports_password_compress: false, supports_password_extract: false, single_file_only: false, supports_split: true, requires_7za: false, requires_winrar: false },
    CompressionFormatCapability { format: "zpaq", extensions: &["zpaq"], can_compress: true, can_extract: true, supports_password_compress: true, supports_password_extract: true, single_file_only: false, supports_split: true, requires_7za: false, requires_winrar: false },
    CompressionFormatCapability { format: "tar.lz4.aes", extensions: &["tar.lz4.aes"], can_compress: true, can_extract: true, supports_password_compress: true, supports_password_extract: true, single_file_only: false, supports_split: true, requires_7za: false, requires_winrar: false },
    CompressionFormatCapability { format: "tar.br.aes", extensions: &["tar.br.aes"], can_compress: true, can_extract: true, supports_password_compress: true, supports_password_extract: true, single_file_only: false, supports_split: true, requires_7za: false, requires_winrar: false },
    CompressionFormatCapability { format: "tar.lz.aes", extensions: &["tar.lz.aes", "tlz.aes"], can_compress: true, can_extract: true, supports_password_compress: true, supports_password_extract: true, single_file_only: false, supports_split: true, requires_7za: false, requires_winrar: false },
    CompressionFormatCapability { format: "lz4.aes", extensions: &["lz4.aes"], can_compress: true, can_extract: true, supports_password_compress: true, supports_password_extract: true, single_file_only: true, supports_split: true, requires_7za: false, requires_winrar: false },
    CompressionFormatCapability { format: "br.aes", extensions: &["br.aes"], can_compress: true, can_extract: true, supports_password_compress: true, supports_password_extract: true, single_file_only: true, supports_split: true, requires_7za: false, requires_winrar: false },
    CompressionFormatCapability { format: "lz.aes", extensions: &["lz.aes"], can_compress: true, can_extract: true, supports_password_compress: true, supports_password_extract: true, single_file_only: true, supports_split: true, requires_7za: false, requires_winrar: false },
    CompressionFormatCapability {
        format: "tar.aes",
        extensions: &["tar.aes"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: true,
        single_file_only: false,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "tar.gz.aes",
        extensions: &["tar.gz.aes", "tgz.aes"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: true,
        single_file_only: false,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "tar.bz2.aes",
        extensions: &["tar.bz2.aes", "tbz2.aes"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: true,
        single_file_only: false,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "tar.xz.aes",
        extensions: &["tar.xz.aes", "txz.aes"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: true,
        single_file_only: false,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "tar.zst.aes",
        extensions: &["tar.zst.aes", "tzst.aes"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: true,
        single_file_only: false,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "gz.aes",
        extensions: &["gz.aes", "gzip.aes"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: true,
        single_file_only: true,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "bz2.aes",
        extensions: &["bz2.aes", "bzip2.aes"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: true,
        single_file_only: true,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "xz.aes",
        extensions: &["xz.aes"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: true,
        single_file_only: true,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "zst.aes",
        extensions: &["zst.aes", "zstd.aes"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: true,
        single_file_only: true,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "tar.bz2",
        extensions: &["tar.bz2", "tbz2", "tbz"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: false,
        single_file_only: false,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "tar.gz",
        extensions: &["tar.gz", "tgz", "tpz"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: false,
        single_file_only: false,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "tar.xz",
        extensions: &["tar.xz", "txz"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: false,
        single_file_only: false,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "tar.zst",
        extensions: &["tar.zst", "tzst"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: false,
        single_file_only: false,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "zip",
        extensions: &["zip"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: true,
        single_file_only: false,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "zipx", extensions: &["zipx"], can_compress: true, can_extract: true,
        supports_password_compress: true, supports_password_extract: true,
        single_file_only: false, supports_split: true, requires_7za: false, requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "7z",
        extensions: &["7z"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: true,
        single_file_only: false,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "rar",
        extensions: &["rar"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: true,
        single_file_only: false,
        supports_split: true,
        requires_7za: false,
        requires_winrar: true,
    },
    CompressionFormatCapability { format: "esd", extensions: &["esd"], can_compress: true, can_extract: true, supports_password_compress: false, supports_password_extract: false, single_file_only: false, supports_split: true, requires_7za: true, requires_winrar: false },
    CompressionFormatCapability {
        format: "wim",
        extensions: &["wim"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: false,
        supports_password_extract: false,
        single_file_only: false,
        supports_split: true,
        requires_7za: true,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "tar",
        extensions: &["tar", "ova"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: false,
        single_file_only: false,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "gz",
        extensions: &["gz", "gzip"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: false,
        single_file_only: true,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "bz2",
        extensions: &["bz2", "bzip2"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: false,
        single_file_only: true,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "xz",
        extensions: &["xz"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: false,
        single_file_only: true,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "zst",
        extensions: &["zst", "zstd"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: false,
        single_file_only: true,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability {
        format: "lzma",
        extensions: &["lzma"],
        can_compress: true,
        can_extract: true,
        supports_password_compress: true,
        supports_password_extract: false,
        single_file_only: true,
        supports_split: true,
        requires_7za: false,
        requires_winrar: false,
    },
    CompressionFormatCapability { format: "tar.lz4", extensions: &["tar.lz4"], can_compress: true, can_extract: true, supports_password_compress: true, supports_password_extract: false, single_file_only: false, supports_split: true, requires_7za: false, requires_winrar: false },
    CompressionFormatCapability { format: "tar.br", extensions: &["tar.br"], can_compress: true, can_extract: true, supports_password_compress: true, supports_password_extract: false, single_file_only: false, supports_split: true, requires_7za: false, requires_winrar: false },
    CompressionFormatCapability { format: "lz4", extensions: &["lz4"], can_compress: true, can_extract: true, supports_password_compress: true, supports_password_extract: false, single_file_only: true, supports_split: true, requires_7za: false, requires_winrar: false },
    CompressionFormatCapability { format: "br", extensions: &["br"], can_compress: true, can_extract: true, supports_password_compress: true, supports_password_extract: false, single_file_only: true, supports_split: true, requires_7za: false, requires_winrar: false },
    CompressionFormatCapability { format: "tar.lz", extensions: &["tar.lz", "tlz"], can_compress: true, can_extract: true, supports_password_compress: false, supports_password_extract: false, single_file_only: false, supports_split: true, requires_7za: false, requires_winrar: false },
    CompressionFormatCapability { format: "lz", extensions: &["lz"], can_compress: true, can_extract: true, supports_password_compress: false, supports_password_extract: false, single_file_only: true, supports_split: true, requires_7za: false, requires_winrar: false },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CompressionRoute {
    LzmaAes,
    TarLzmaAes,
    TarLzma,
    Zpaq,
    Lz4Aes,
    BrotliAes,
    LzipAes,
    TarLz4Aes,
    TarBrotliAes,
    TarLzipAes,
    TarAes,
    TarGzipAes,
    TarBzip2Aes,
    TarXzAes,
    TarZstdAes,
    GzipAes,
    Bzip2Aes,
    XzAes,
    ZstdAes,
    Zip,
    Tar,
    TarGzip,
    TarBzip2,
    TarXz,
    SevenZip,
    Rar,
    Wim,
    Gzip,
    Bzip2,
    Xz,
    Zstd,
    TarZstd,
    Lzma,
    Lz4,
    Brotli,
    TarLz4,
    TarBrotli,
    Lzip,
    TarLzip,
}

pub fn normalize_compression_format(format: &str) -> String {
    let normalized = format.trim().trim_start_matches('.').to_ascii_lowercase();
    COMPRESSION_FORMAT_CAPABILITIES
        .iter()
        .find(|capability| {
            capability.format == normalized
                || capability
                    .extensions
                    .iter()
                    .any(|extension| *extension == normalized)
        })
        .map(|capability| capability.format.to_string())
        .unwrap_or(normalized)
}

pub fn infer_compression_format(output_path: &str, explicit_format: Option<&str>) -> String {
    if let Some(format) = explicit_format
        .map(str::trim)
        .filter(|format| !format.is_empty())
    {
        return normalize_compression_format(format);
    }

    let output_lower = output_path.to_ascii_lowercase();
    COMPRESSION_FORMAT_CAPABILITIES
        .iter()
        .find(|capability| {
            capability
                .extensions
                .iter()
                .any(|extension| output_lower.ends_with(&format!(".{extension}")))
        })
        .map(|capability| capability.format.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn compression_format_capabilities() -> &'static [CompressionFormatCapability] {
    COMPRESSION_FORMAT_CAPABILITIES
}

pub fn find_compression_format_capability(
    format: &str,
) -> Option<&'static CompressionFormatCapability> {
    let normalized = normalize_compression_format(format);
    COMPRESSION_FORMAT_CAPABILITIES
        .iter()
        .find(|capability| capability.format == normalized)
}

pub fn has_native_password_container(format: &str) -> bool {
    matches!(
        normalize_compression_format(format).as_str(),
        "zip"
            | "lzma.aes"
            | "tar.lzma.aes"
            | "zpaq"
            | "zipx"
            | "7z"
            | "rar"
            | "lz4.aes"
            | "br.aes"
            | "lz.aes"
            | "tar.lz4.aes"
            | "tar.br.aes"
            | "tar.lz.aes"
            | "tar.aes"
            | "tar.gz.aes"
            | "tar.bz2.aes"
            | "tar.xz.aes"
            | "tar.zst.aes"
            | "gz.aes"
            | "bz2.aes"
            | "xz.aes"
            | "zst.aes"
    )
}

pub fn validate_compression_request(
    source_files: &[String],
    output_path: &str,
    options: &CompressionOptions,
) -> Result<String> {
    let requested_format = infer_compression_format(output_path, options.format.as_deref());
    let capability = find_compression_format_capability(&requested_format);
    anyhow::ensure!(!capability.is_some_and(|value| !value.can_compress), "该格式当前仅支持读取，请选择 ZIP 或 7Z 作为目标格式");
    let has_password = options
        .password
        .as_deref()
        .is_some_and(|password| !password.is_empty());
    let split_requested = options.split_size.is_some_and(|size| size > 0);

    if requested_format == "swm" {
        anyhow::ensure!(options.split_size.is_some_and(|s| s >= 65536), "原生 SWM 需要设置至少 64 KiB 的目标卷大小");
        anyhow::ensure!(matches!(options.wim_method, super::wim_writer::WimMethod::Store | super::wim_writer::WimMethod::Xpress | super::wim_writer::WimMethod::Lzx), "原生 SWM 仅支持仅存储、XPRESS、LZX，不支持 LZMS 或固实");
        anyhow::ensure!(output_path.to_ascii_lowercase().ends_with(".swm"), "原生 SWM 输出需要以 .swm 结尾");
    }
    if options.encrypt_filenames && (!matches!(requested_format.as_str(), "7z" | "rar") || !has_password) {
        return Err(CompressionError::CompressionFailed(
            "文件名加密仅支持 7Z/RAR，且必须设置压缩密码。".to_string(),
        ).into());
    }

    if split_requested && !capability.is_some_and(|capability| capability.supports_split) {
        return Err(CompressionError::CompressionFailed(format!(
            "{} does not support split archive creation in the active engine.",
            requested_format
        ))
        .into());
    }

    if options.create_solid_archive && !matches!(requested_format.as_str(), "7z" | "rar") {
        return Err(CompressionError::CompressionFailed(
            "Solid compression is only supported for 7Z/RAR archives.".to_string(),
        )
        .into());
    }

    if has_password && !capability.is_some_and(|capability| capability.supports_password_compress) {
        return Err(CompressionError::UnsupportedEncryption.into());
    }

    if has_password && !has_native_password_container(&requested_format) {
        if !output_path.to_ascii_lowercase().ends_with(".7z") {
            return Err(CompressionError::CompressionFailed(format!(
                "{} does not support native encryption. Use a .7z output path or choose an .aes format.",
                requested_format
            ))
            .into());
        }
        return Ok("7z".to_string());
    }

    if capability.is_some_and(|capability| capability.single_file_only) {
        let single_regular_file = source_files.len() == 1 && Path::new(&source_files[0]).is_file();
        if !single_regular_file {
            return Err(CompressionError::CompressionFailed(format!(
                "{} compression only supports one regular file. Please use a TAR-based format for folders or multiple files.",
                requested_format
            ))
            .into());
        }
    }

    Ok(requested_format)
}

pub(crate) fn compression_route(format: &str) -> Option<CompressionRoute> {
    match normalize_compression_format(format).as_str() {
        "lzma.aes" => Some(CompressionRoute::LzmaAes),
        "tar.lzma.aes" => Some(CompressionRoute::TarLzmaAes),
        "tar.lzma" => Some(CompressionRoute::TarLzma),
        "zpaq" => Some(CompressionRoute::Zpaq),
        "lz4.aes" => Some(CompressionRoute::Lz4Aes),
        "br.aes" => Some(CompressionRoute::BrotliAes),
        "lz.aes" => Some(CompressionRoute::LzipAes),
        "tar.lz4.aes" => Some(CompressionRoute::TarLz4Aes),
        "tar.br.aes" => Some(CompressionRoute::TarBrotliAes),
        "tar.lz.aes" => Some(CompressionRoute::TarLzipAes),
        "tar.aes" => Some(CompressionRoute::TarAes),
        "tar.gz.aes" => Some(CompressionRoute::TarGzipAes),
        "tar.bz2.aes" => Some(CompressionRoute::TarBzip2Aes),
        "tar.xz.aes" => Some(CompressionRoute::TarXzAes),
        "tar.zst.aes" => Some(CompressionRoute::TarZstdAes),
        "gz.aes" => Some(CompressionRoute::GzipAes),
        "bz2.aes" => Some(CompressionRoute::Bzip2Aes),
        "xz.aes" => Some(CompressionRoute::XzAes),
        "zst.aes" => Some(CompressionRoute::ZstdAes),
        "zip" | "zipx" => Some(CompressionRoute::Zip),
        "tar" => Some(CompressionRoute::Tar),
        "tar.gz" => Some(CompressionRoute::TarGzip),
        "tar.bz2" => Some(CompressionRoute::TarBzip2),
        "tar.xz" => Some(CompressionRoute::TarXz),
        "7z" => Some(CompressionRoute::SevenZip),
        "rar" => Some(CompressionRoute::Rar),
        "wim" | "esd" | "swm" => Some(CompressionRoute::Wim),
        "gz" => Some(CompressionRoute::Gzip),
        "bz2" => Some(CompressionRoute::Bzip2),
        "xz" => Some(CompressionRoute::Xz),
        "zst" => Some(CompressionRoute::Zstd),
        "tar.zst" => Some(CompressionRoute::TarZstd),
        "lzma" => Some(CompressionRoute::Lzma),
        "lz4" => Some(CompressionRoute::Lz4),
        "br" => Some(CompressionRoute::Brotli),
        "tar.lz4" => Some(CompressionRoute::TarLz4),
        "tar.br" => Some(CompressionRoute::TarBrotli),
        "lz" => Some(CompressionRoute::Lzip),
        "tar.lz" => Some(CompressionRoute::TarLzip),
        _ => None,
    }
}

pub fn is_tar_wrapped_archive(path: &Path, format: &str) -> bool {
    let mut file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if let Some((base, number)) = file_name.rsplit_once('.') {
        if number.len() >= 3 && number.bytes().all(|byte| byte.is_ascii_digit()) { file_name = base.to_string(); }
    }
    let suffixes: &[&str] = match normalize_compression_format(format).as_str() {
        "tar.lzo" => &[".tar.lzo"],
        "tar.lzma" => &[".tar.lzma"],
        "tar.gz" => &[".tar.gz", ".tgz", ".tpz"],
        "tar.bz2" => &[".tar.bz2", ".tbz", ".tbz2"],
        "tar.xz" => &[".tar.xz", ".txz"],
        "tar.zst" => &[".tar.zst", ".tzst"],
        "tar.lz4" => &[".tar.lz4"],
        "tar.br" => &[".tar.br"],
        "tar.lz" => &[".tar.lz", ".tlz"],
        _ => return false,
    };
    suffixes.iter().any(|suffix| file_name.ends_with(suffix))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;
    use crate::services::compression_service::CompressionService;

    fn options(format: Option<&str>) -> CompressionOptions {
        CompressionOptions {
            format: format.map(str::to_string),
            ..Default::default()
        }
    }

    #[test]
    fn every_declared_extension_normalizes_to_its_canonical_format() {
        for capability in COMPRESSION_FORMAT_CAPABILITIES {
            assert_eq!(
                normalize_compression_format(capability.format),
                capability.format
            );
            for extension in capability.extensions {
                assert_eq!(
                    normalize_compression_format(extension),
                    capability.format,
                    "{extension} should normalize to {}",
                    capability.format
                );
            }
        }
    }

    #[test]
    fn longest_extensions_win_when_inferring_from_output_paths() {
        assert_eq!(
            infer_compression_format("C:/output/archive.tar.gz.aes", None),
            "tar.gz.aes"
        );
        assert_eq!(
            infer_compression_format("C:/output/archive.tar.zst", None),
            "tar.zst"
        );
        assert_eq!(
            infer_compression_format("C:/output/archive.ZIPX", None),
            "zipx"
        );
        assert_eq!(
            infer_compression_format("C:/output/archive.unknown", None),
            "unknown"
        );
    }

    #[test]
    fn aliases_share_the_same_execution_route() {
        assert_eq!(
            compression_route("tgz.aes"),
            Some(CompressionRoute::TarGzipAes)
        );
        assert_eq!(compression_route("zipx"), Some(CompressionRoute::Zip));
        assert_eq!(compression_route("zstd"), Some(CompressionRoute::Zstd));
        assert_eq!(compression_route("unsupported"), None);
    }

    #[test]
    fn every_compressible_capability_has_exactly_one_execution_route() {
        for capability in COMPRESSION_FORMAT_CAPABILITIES {
            if capability.can_compress {
                assert!(
                    compression_route(capability.format).is_some(),
                    "{} is declared compressible but has no execution route",
                    capability.format
                );
            }
        }
    }

    #[test]
    fn recognizes_every_tar_wrapper_alias() {
        for (file_name, format) in [
            ("archive.tar.gz", "tar.gz"),
            ("archive.tgz", "tar.gz"),
            ("archive.tpz", "tar.gz"),
            ("archive.tar.bz2", "tar.bz2"),
            ("archive.tbz", "tar.bz2"),
            ("archive.tbz2", "tar.bz2"),
            ("archive.tar.xz", "tar.xz"),
            ("archive.txz", "tar.xz"),
            ("archive.tar.zst", "tar.zst"),
            ("archive.tzst", "tar.zst"),
        ] {
            assert!(
                is_tar_wrapped_archive(Path::new(file_name), format),
                "{file_name} should be recognized as a TAR wrapper"
            );
        }
        assert!(!is_tar_wrapped_archive(Path::new("archive.gz"), "gz"));
    }

    #[test]
    fn validation_enforces_single_file_and_explicit_encryption_fallbacks() {
        let temp = tempdir().expect("temp dir");
        let source = temp.path().join("source.txt");
        fs::write(&source, b"source").expect("source");
        let source = source.to_string_lossy().to_string();

        assert_eq!(
            validate_compression_request(
                std::slice::from_ref(&source),
                "archive.zipx",
                &options(Some("zipx")),
            )
            .expect("zipx creation"),
            "zipx"
        );

        let multi_source = vec![source.clone(), source.clone()];
        assert!(
            validate_compression_request(&multi_source, "archive.gz", &options(Some("gzip")),)
                .is_err()
        );

        let mut encrypted_tar = options(Some("tgz"));
        encrypted_tar.password = Some("secret".to_string());
        assert!(validate_compression_request(
            std::slice::from_ref(&source),
            "archive.tar.gz",
            &encrypted_tar,
        )
        .is_err());
        assert_eq!(
            validate_compression_request(&[source], "archive.7z", &encrypted_tar)
                .expect("explicit 7z fallback"),
            "7z"
        );
    }

    #[test]
    fn compatibility_facade_exposes_the_extracted_rules() {
        assert_eq!(
            CompressionService::infer_compression_format("archive.tgz", None),
            "tar.gz"
        );
        assert_eq!(
            CompressionService::find_compression_format_capability("gzip")
                .expect("gzip capability")
                .format,
            "gz"
        );
    }

    #[test]
    fn validation_rejects_unimplemented_split_and_solid_combinations() {
        let temp = tempdir().expect("temp dir");
        let source = temp.path().join("source.txt");
        fs::write(&source, b"source").expect("source");
        let source = source.to_string_lossy().to_string();

        let mut split_7z = options(Some("7z"));
        split_7z.split_size = Some(1024);
        assert!(validate_compression_request(
            std::slice::from_ref(&source),
            "archive.7z",
            &split_7z,
        )
        .is_ok());

        let mut solid_zip = options(Some("zip"));
        solid_zip.create_solid_archive = true;
        assert!(validate_compression_request(&[source], "archive.zip", &solid_zip).is_err());
    }

    #[test]
    fn filename_encryption_requires_explicit_seven_zip_password() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source.txt");
        fs::write(&source, b"source").unwrap();
        let sources = vec![source.to_string_lossy().into_owned()];
        for (format, password, valid) in [
            ("7z", None, false), ("7z", Some(""), false),
            ("zip", Some("password"), false), ("tar.aes", Some("password"), false),
            ("7z", Some("中文密码 + !测试"), true), ("7z", Some(" "), true),
        ] {
            let mut request = options(Some(format));
            request.encrypt_filenames = true;
            request.password = password.map(str::to_owned);
            let result = validate_compression_request(&sources, &format!("archive.{format}"), &request);
            assert_eq!(result.is_ok(), valid, "format={format}");
            if !valid { assert!(result.unwrap_err().to_string().contains("文件名加密仅支持 7Z")); }
        }
        let legacy: CompressionOptions = serde_json::from_value(serde_json::json!({"level":6})).unwrap();
        assert!(!legacy.encrypt_filenames);
        assert!(!CompressionOptions::default().encrypt_filenames);
        let mut request = legacy;
        request.encrypt_filenames = true;
        let restored: CompressionOptions = serde_json::from_value(serde_json::to_value(request).unwrap()).unwrap();
        assert!(restored.encrypt_filenames);
    }
}
