use encoding_rs::{EUC_KR, SHIFT_JIS};

/// Decode filename bytes. Without the area-code flag the name is UTF-8; with it,
/// `locale_code` selects a legacy code page (932=Shift-JIS, 949=EUC-KR).
pub fn decode_filename(use_area_code: bool, locale_code: Option<u16>, data: &[u8]) -> crate::error::EggResult<String> {
    if !use_area_code {
        return String::from_utf8(data.to_vec()).map_err(|_| crate::error::EggError::CorruptedFile);
    }

    let locale = locale_code.unwrap_or(0);
    let encoding = match locale {
        932 => SHIFT_JIS,
        949 | 0 => EUC_KR,
        936 => encoding_rs::GBK,
        950 => encoding_rs::BIG5,
        _ => return Err(crate::error::EggError::UnsupportedFeature(format!("filename code page {locale}"))),
    };

    let (decoded, invalid) = encoding.decode_without_bom_handling(data);
    if invalid { return Err(crate::error::EggError::CorruptedFile); }
    Ok(decoded.into_owned())
}

/// Normalize path separators to forward slash, strip leading slashes, and drop
/// control characters (NUL, terminal escapes, newlines) that a crafted archive
/// could smuggle into a filename.
pub fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_does_not_repair_invalid_names() {
        assert_eq!(normalize_path("a\\b\\c"), "a/b/c");
        assert_eq!(normalize_path("/leading"), "/leading");
        // NUL and a terminal escape sequence are dropped.
        assert_eq!(normalize_path("ev\0il\x1b[2J"), "ev\0il\x1b[2J");
    }
}
