//! ZIP password byte encoding. UTF-8 remains the default; local compatibility
//! must be selected explicitly and must never replace unrepresentable characters.
use anyhow::Result;
#[cfg(not(windows))]
use anyhow::bail;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZipEncryption {
    #[default]
    Aes256,
    Zipcrypto,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZipPasswordEncoding {
    #[default]
    Utf8,
    WindowsLocal,
    Cp932,
    Cp936,
    Cp950,
    Cp437,
    Cp850,
}

impl ZipPasswordEncoding {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Utf8 => "utf8",
            Self::WindowsLocal => "windows_local",
            Self::Cp932 => "cp932",
            Self::Cp936 => "cp936",
            Self::Cp950 => "cp950",
            Self::Cp437 => "cp437",
            Self::Cp850 => "cp850",
        }
    }

    pub fn from_setting(value: &str) -> Option<Self> {
        match value {
            "utf8" => Some(Self::Utf8),
            "windows_local" => Some(Self::WindowsLocal),
            "cp932" => Some(Self::Cp932),
            "cp936" => Some(Self::Cp936),
            "cp950" => Some(Self::Cp950),
            "cp437" => Some(Self::Cp437),
            "cp850" => Some(Self::Cp850),
            _ => None,
        }
    }
}

/// A bad encoding choice is not an incorrect password or damaged archive.
/// Keep the typed cause when exposing it through the ZIP reader's I/O error.
#[derive(Debug)]
pub struct ZipPasswordEncodingError {
    pub encoding: ZipPasswordEncoding,
    source: anyhow::Error,
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct UnrepresentablePassword(String);

impl ZipPasswordEncodingError {
    pub(crate) fn is_unrepresentable(&self) -> bool {
        self.source.downcast_ref::<UnrepresentablePassword>().is_some()
    }
}

impl std::fmt::Display for ZipPasswordEncodingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.source.fmt(formatter)
    }
}

impl std::error::Error for ZipPasswordEncodingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

pub(crate) fn encoding_error(error: &anyhow::Error) -> Option<&ZipPasswordEncodingError> {
    error.chain().find_map(|cause| {
        cause.downcast_ref::<ZipPasswordEncodingError>().or_else(|| {
            cause.downcast_ref::<std::io::Error>()?.get_ref()?.downcast_ref::<ZipPasswordEncodingError>()
        })
    })
}

pub fn encode(password: &str, encoding: ZipPasswordEncoding) -> Result<Vec<u8>> {
    match encoding {
        ZipPasswordEncoding::Utf8 => Ok(password.as_bytes().to_vec()),
        ZipPasswordEncoding::WindowsLocal => encode_local(password),
        ZipPasswordEncoding::Cp932 => encode_fixed(password, 932),
        ZipPasswordEncoding::Cp936 => encode_fixed(password, 936),
        ZipPasswordEncoding::Cp950 => encode_fixed(password, 950),
        ZipPasswordEncoding::Cp437 => encode_fixed(password, 437),
        ZipPasswordEncoding::Cp850 => encode_fixed(password, 850),
    }
}

fn decrypt_compatible<'a, R: std::io::Read + std::io::Seek>(
    archive: &'a mut zip_aes::ZipArchive<R>, index: usize, password: &[u8],
) -> zip_aes::result::ZipResult<zip_aes::read::ZipFile<'a, R>> {
    archive.by_index_with_options(index, zip_aes::ZipReadOptions::new()
        .password(Some(password)).zipcrypto_crc_with_descriptor(true))
}

/// Read historical UTF-8 archives first, then try the current Windows encoding
/// for an explicit password mismatch. If both short encryption headers accept
/// the password, disambiguate with full integrity validation before choosing.
/// A failed second validation must never make corrupt content acceptable.
pub fn open_entry<'a, R: std::io::Read + std::io::Seek>(
    archive: &'a mut zip_aes::ZipArchive<R>,
    index: usize,
    password: &str,
) -> zip_aes::result::ZipResult<zip_aes::read::ZipFile<'a, R>> {
    open_entry_with_encoding(archive, index, password, None)
}

/// None preserves the historical UTF-8/local fallback. An explicit choice
/// uses only those password bytes; it never falls back to another code page.
pub fn open_entry_with_encoding<'a, R: std::io::Read + std::io::Seek>(
    archive: &'a mut zip_aes::ZipArchive<R>,
    index: usize,
    password: &str,
    encoding: Option<ZipPasswordEncoding>,
) -> zip_aes::result::ZipResult<zip_aes::read::ZipFile<'a, R>> {
    let Some(encoding) = encoding else {
        return open_entry_automatic(archive, index, password);
    };
    // A supplied password also reaches public entries in mixed archives.
    // Their bytes must not depend on whether this password is representable.
    if !archive.by_index_raw(index)?.encrypted() {
        return archive.by_index(index);
    }
    let bytes = encode(password, encoding).map_err(|error| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput,
            ZipPasswordEncodingError { encoding, source: error })
    })?;
    decrypt_compatible(archive, index, &bytes)
}

fn open_entry_automatic<'a, R: std::io::Read + std::io::Seek>(
    archive: &'a mut zip_aes::ZipArchive<R>,
    index: usize,
    password: &str,
) -> zip_aes::result::ZipResult<zip_aes::read::ZipFile<'a, R>> {
    let local = if password.is_ascii() { None } else {
        encode(password, ZipPasswordEncoding::WindowsLocal).ok()
            .filter(|bytes| bytes != password.as_bytes())
    };
    let Some(local) = local else {
        return decrypt_compatible(archive, index, password.as_bytes());
    };
    // In a mixed archive a supplied password also reaches plain entries. They
    // accept any password bytes; that is not an ambiguous encryption header.
    if !archive.by_index_raw(index)?.encrypted() {
        return archive.by_index(index);
    }
    let mismatch = match decrypt_compatible(archive, index, password.as_bytes()) {
        Ok(_) => false,
        Err(zip_aes::result::ZipError::InvalidPassword) => true,
        Err(error) => return Err(error),
    };
    if mismatch {
        return decrypt_compatible(archive, index, &local);
    }
    // ZipCrypto's short header can accept incorrect UTF-8 bytes even when the
    // real password was encoded in ACP. Most entries need only header checks;
    // stream content only in the ambiguous double-match case.
    let local_also_matches = decrypt_compatible(archive, index, &local).is_ok();
    if local_also_matches {
        let utf8_integrity = {
            let mut reader = decrypt_compatible(archive, index, password.as_bytes())?;
            std::io::copy(&mut reader, &mut std::io::sink())
        };
        if let Err(utf8_error) = utf8_integrity {
            let local_integrity = {
                let mut reader = decrypt_compatible(archive, index, &local)?;
                std::io::copy(&mut reader, &mut std::io::sink())
            };
            if local_integrity.is_err() { return Err(utf8_error.into()); }
            return decrypt_compatible(archive, index, &local);
        }
    }
    decrypt_compatible(archive, index, password.as_bytes())
}

pub fn open_named_entry<'a, R: std::io::Read + std::io::Seek>(
    archive: &'a mut zip_aes::ZipArchive<R>, name: &str, password: &str,
) -> zip_aes::result::ZipResult<zip_aes::read::ZipFile<'a, R>> {
    open_named_entry_with_encoding(archive, name, password, None)
}

pub fn open_named_entry_with_encoding<'a, R: std::io::Read + std::io::Seek>(
    archive: &'a mut zip_aes::ZipArchive<R>, name: &str, password: &str,
    encoding: Option<ZipPasswordEncoding>,
) -> zip_aes::result::ZipResult<zip_aes::read::ZipFile<'a, R>> {
    let index = archive.index_for_name(name).ok_or(zip_aes::result::ZipError::FileNotFound)?;
    open_entry_with_encoding(archive, index, password, encoding)
}

#[cfg(not(windows))]
fn encode_fixed(_password: &str, code_page: u32) -> Result<Vec<u8>> {
    bail!("Windows ZIP password code page {code_page} is only available on Windows")
}

#[cfg(windows)]
fn encode_fixed(password: &str, code_page: u32) -> Result<Vec<u8>> {
    encode_windows_code_page(password, code_page, true)
}

#[cfg(not(windows))]
fn encode_local(_password: &str) -> Result<Vec<u8>> {
    bail!("Windows ZIP password compatibility is only available on Windows")
}

#[cfg(windows)]
fn encode_local(password: &str) -> Result<Vec<u8>> {
    use windows_sys::Win32::Globalization::GetACP;
    // SAFETY: GetACP takes no pointers and only reads the process code page.
    let code_page = unsafe { GetACP() };
    // Keep historical local mode unchanged. Fixed pages additionally reject
    // aliases whose byte round-trip would change the entered Unicode text.
    encode_windows_code_page(password, code_page, false)
}

#[cfg(windows)]
fn encode_windows_code_page(password: &str, code_page: u32, require_round_trip: bool) -> Result<Vec<u8>> {
    use windows_sys::Win32::Globalization::{WideCharToMultiByte, MultiByteToWideChar,
        WC_NO_BEST_FIT_CHARS, MB_ERR_INVALID_CHARS};
    if code_page == 65001 || password.is_empty() {
        return Ok(password.as_bytes().to_vec());
    }
    let wide: Vec<u16> = password.encode_utf16().collect();
    let length = i32::try_from(wide.len())?;
    let mut substituted = 0;
    // SAFETY: input points to length UTF-16 code units, a null output with zero
    // capacity requests the buffer length, and substituted is a writable BOOL.
    let size = unsafe {
        WideCharToMultiByte(code_page, WC_NO_BEST_FIT_CHARS, wide.as_ptr(), length,
            std::ptr::null_mut(), 0, std::ptr::null(), &mut substituted)
    };
    if size <= 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if substituted != 0 {
        if !require_round_trip {
            return Err(UnrepresentablePassword(format!("ZIP password contains characters unavailable in Windows code page {code_page}; confirm the password text and ZIP password encoding; characters will not be replaced")).into());
        }
        return Err(UnrepresentablePassword(format!("ZIP 密码含有 Windows 代码页 {code_page} 无法表示的字符；请确认密码文本与 ZIP 密码编码，不会替换字符")).into());
    }
    let mut bytes = vec![0; size as usize];
    // SAFETY: input remains valid; output has exactly the size requested above.
    let written = unsafe {
        WideCharToMultiByte(code_page, WC_NO_BEST_FIT_CHARS, wide.as_ptr(), length,
            bytes.as_mut_ptr(), size, std::ptr::null(), &mut substituted)
    };
    if written <= 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if substituted != 0 {
        if !require_round_trip {
            return Err(UnrepresentablePassword("ZIP password cannot be encoded without character replacement".into()).into());
        }
        return Err(UnrepresentablePassword(format!("ZIP 密码无法无损使用 Windows 代码页 {code_page}，不会替换字符")).into());
    }
    bytes.truncate(written as usize);
    if require_round_trip {
        // Fixed Windows pages can contain aliases (for example yen/backslash
        // in CP932). Never accept one by silently changing the input text.
        let byte_len = i32::try_from(bytes.len())?;
        // SAFETY: bytes is byte_len readable bytes; null/zero queries length.
        let count = unsafe { MultiByteToWideChar(code_page, MB_ERR_INVALID_CHARS,
            bytes.as_ptr(), byte_len, std::ptr::null_mut(), 0) };
        if count <= 0 { return Err(std::io::Error::last_os_error().into()); }
        let mut restored = vec![0u16; count as usize];
        // SAFETY: restored has the queried UTF-16 capacity; input is unchanged.
        let restored_count = unsafe { MultiByteToWideChar(code_page, MB_ERR_INVALID_CHARS,
            bytes.as_ptr(), byte_len, restored.as_mut_ptr(), count) };
        if restored_count <= 0 { return Err(std::io::Error::last_os_error().into()); }
        restored.truncate(restored_count as usize);
        if restored != wide {
            return Err(UnrepresentablePassword(format!("ZIP 密码无法无损使用 Windows 代码页 {code_page}，不会替换字符")).into());
        }
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn unrepresentable_password_guidance_also_applies_to_existing_archives() {
        let error = encode("🔒", ZipPasswordEncoding::Cp932).unwrap_err().to_string();
        assert!(error.contains("932") && error.contains("不会替换字符"));
        assert!(!error.contains("7Z"), "reading an existing archive cannot change its format");
    }

    #[test]
    fn candidate_encoding_error_distinguishes_lossy_text_from_system_failures() {
        for unrepresentable in [false, true] {
            let cause: anyhow::Error = if unrepresentable { UnrepresentablePassword("cannot encode".into()).into() }
                else { std::io::Error::new(std::io::ErrorKind::Other, "codepage conversion system failure").into() };
            let error = anyhow::Error::new(zip_aes::result::ZipError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput, ZipPasswordEncodingError { encoding: ZipPasswordEncoding::Cp932, source: cause })))
                .context("password inspection");
            let encoding = encoding_error(&error).unwrap();
            assert_eq!(encoding.encoding, ZipPasswordEncoding::Cp932);
            assert_eq!(encoding.is_unrepresentable(), unrepresentable);
        }
    }

    #[cfg(windows)]
    fn legacy_fixture_cases() -> [(u32, &'static str, &'static [u8]); 3] {
        [
            (932, "日本語パスワード+!1416", b"\x93\xfa\x96\x7b\x8c\xea\x83\x70\x83\x58\x83\x8f\x81\x5b\x83\x68+!1416"),
            (936, "简体密码测试+!1416", b"\xbc\xf2\xcc\xe5\xc3\xdc\xc2\xeb\xb2\xe2\xca\xd4+!1416"),
            (950, "繁體密碼測試+!1416", b"\xc1\x63\xc5\xe9\xb1\x4b\xbd\x58\xb4\xfa\xb8\xd5+!1416"),
        ]
    }

    #[cfg(windows)]
    #[test]
    fn real_infozip_fixtures_show_the_current_machine_codepage_boundary() {
        use std::io::{Cursor, Read};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/archives/legacy-zip-password-1416");
        let expected = std::fs::read(root.join("payload.txt")).unwrap();
        for (codepage, password, known_bytes) in legacy_fixture_cases() {
            let bytes = std::fs::read(root.join(format!("cp{codepage}.zip"))).unwrap();
            let mut archive = zip_aes::ZipArchive::new(Cursor::new(bytes.clone())).unwrap();
            let mut content = Vec::new();
            archive.by_index_decrypt(0, known_bytes).unwrap().read_to_end(&mut content).unwrap();
            assert_eq!(content, expected, "the original external archive must be fully valid");

            let mut archive = zip_aes::ZipArchive::new(Cursor::new(bytes)).unwrap();
            let accepted = open_entry(&mut archive, 0, password).is_ok_and(|mut entry| {
                let mut content = Vec::new();
                entry.read_to_end(&mut content).is_ok() && content == expected
            });
            let current = unsafe { windows_sys::Win32::Globalization::GetACP() };
            assert_eq!(accepted, current == codepage,
                "legacy automatic mode must retain its current UTF-8/local-only behavior: ACP={current}, fixture={codepage}");
        }
    }

    #[test]
    fn encoding_setting_names_round_trip_without_accepting_unknown_values() {
        for name in ["utf8", "windows_local", "cp932", "cp936", "cp950"] {
            let encoding = ZipPasswordEncoding::from_setting(name).unwrap();
            assert_eq!(encoding.as_str(), name);
            assert_eq!(serde_json::to_value(encoding).unwrap(), name);
            assert_eq!(serde_json::from_value::<ZipPasswordEncoding>(serde_json::json!(name)).unwrap(), encoding);
        }
        for name in ["auto", "CP932", "cp0", "unknown", ""] {
            assert!(ZipPasswordEncoding::from_setting(name).is_none());
            assert!(serde_json::from_value::<ZipPasswordEncoding>(serde_json::json!(name)).is_err());
        }
        assert_eq!(ZipPasswordEncoding::default(), ZipPasswordEncoding::Utf8);
    }

    #[cfg(windows)]
    #[test]
    fn explicit_fixed_codepages_read_complete_real_infozip_archives() {
        use std::io::{Cursor, Read};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/archives/legacy-zip-password-1416");
        let expected = std::fs::read(root.join("payload.txt")).unwrap();
        for (page, password, known_bytes) in legacy_fixture_cases() {
            let encoding = ZipPasswordEncoding::from_setting(&format!("cp{page}")).unwrap();
            assert_eq!(encode(password, encoding).unwrap(), known_bytes);
            let bytes = std::fs::read(root.join(format!("cp{page}.zip"))).unwrap();
            let mut archive = zip_aes::ZipArchive::new(Cursor::new(bytes.clone())).unwrap();
            let mut content = Vec::new();
            open_named_entry_with_encoding(&mut archive, "payload.txt", password, Some(encoding))
                .unwrap().read_to_end(&mut content).unwrap();
            assert_eq!(content, expected);

            for (wrong_password, wrong_encoding) in [(password, ZipPasswordEncoding::Utf8), ("incorrect", encoding)] {
                let mut archive = zip_aes::ZipArchive::new(Cursor::new(bytes.clone())).unwrap();
                let accepted = open_entry_with_encoding(&mut archive, 0, wrong_password, Some(wrong_encoding))
                    .is_ok_and(|mut entry| entry.read_to_end(&mut Vec::new()).is_ok());
                assert!(!accepted, "an explicit selection must not fall back: cp{page}");
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn fixed_codepages_reject_unrepresentable_and_alias_characters() {
        for encoding in [ZipPasswordEncoding::Cp932, ZipPasswordEncoding::Cp936, ZipPasswordEncoding::Cp950] {
            assert_eq!(encode("ASCII + !", encoding).unwrap(), b"ASCII + !");
            assert_eq!(encode("", encoding).unwrap(), b"");
            assert!(encode("密码🔑", encoding).is_err(), "{encoding:?}");
        }
        assert!(encode("¥", ZipPasswordEncoding::Cp932).is_err(), "yen must not silently become backslash");
        assert!(encode("‾", ZipPasswordEncoding::Cp932).is_err(), "overline must not silently become tilde");
    }

    #[cfg(windows)]
    #[test]
    fn explicit_unrepresentable_password_is_not_reported_as_incorrect_password() {
        use std::io::Cursor;
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/archives/legacy-zip-password-1416/cp932.zip");
        let mut archive = zip_aes::ZipArchive::new(Cursor::new(std::fs::read(path).unwrap())).unwrap();
        let error = open_entry_with_encoding(&mut archive, 0, "日本語🔑", Some(ZipPasswordEncoding::Cp932))
            .map(|_| ()).unwrap_err();
        assert!(matches!(error, zip_aes::result::ZipError::Io(ref io) if io.kind() == std::io::ErrorKind::InvalidInput), "{error:?}");
        let zip_aes::result::ZipError::Io(ref io) = error else { unreachable!() };
        assert_eq!(io.get_ref().unwrap().downcast_ref::<ZipPasswordEncodingError>().unwrap().encoding,
            ZipPasswordEncoding::Cp932);
        assert!(error.to_string().contains("932"));
        assert!(matches!(open_entry_with_encoding(&mut archive, 99, "🔑", Some(ZipPasswordEncoding::Cp932)),
            Err(zip_aes::result::ZipError::FileNotFound)));
        assert!(matches!(open_named_entry_with_encoding(&mut archive, "missing", "🔑", Some(ZipPasswordEncoding::Cp950)),
            Err(zip_aes::result::ZipError::FileNotFound)));
    }

    #[cfg(windows)]
    #[test]
    fn explicit_encoding_preserves_plain_entry_crc_and_damaged_ciphertext_failures() {
        use std::io::{Cursor, Read, Write};
        let mut writer = zip_aes::ZipWriter::new(Cursor::new(Vec::new()));
        writer.start_file("public.txt", zip_aes::write::SimpleFileOptions::default()
            .compression_method(zip_aes::CompressionMethod::Stored)).unwrap();
        writer.write_all(b"exact public bytes").unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        let mut archive = zip_aes::ZipArchive::new(Cursor::new(bytes.clone())).unwrap();
        let mut content = Vec::new();
        open_entry_with_encoding(&mut archive, 0, "🔑", Some(ZipPasswordEncoding::Cp950))
            .unwrap().read_to_end(&mut content).unwrap();
        assert_eq!(content, b"exact public bytes");
        let mut damaged = bytes;
        damaged[archive.by_index_raw(0).unwrap().data_start().unwrap() as usize] ^= 1;
        let mut archive = zip_aes::ZipArchive::new(Cursor::new(damaged)).unwrap();
        let mut public = open_entry_with_encoding(&mut archive, 0, "🔑", Some(ZipPasswordEncoding::Cp950)).unwrap();
        assert!(public.read_to_end(&mut Vec::new()).is_err(), "plain content CRC must still be checked");

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/archives/legacy-zip-password-1416");
        for (page, password, _) in legacy_fixture_cases() {
            let mut bytes = std::fs::read(root.join(format!("cp{page}.zip"))).unwrap();
            let mut archive = zip_aes::ZipArchive::new(Cursor::new(bytes.clone())).unwrap();
            let start = archive.by_index_raw(0).unwrap().data_start().unwrap() as usize;
            bytes[start + 15] ^= 0x40; // after the encryption header, not a password mismatch
            let mut archive = zip_aes::ZipArchive::new(Cursor::new(bytes)).unwrap();
            let encoding = ZipPasswordEncoding::from_setting(&format!("cp{page}")).unwrap();
            let mut entry = open_entry_with_encoding(&mut archive, 0, password, Some(encoding)).unwrap();
            assert!(entry.read_to_end(&mut Vec::new()).is_err(), "cp{page}: damaged content must not be accepted");
        }
    }

    #[test]
    fn legacy_descriptor_crc_header_still_requires_complete_payload_integrity() {
        use std::io::{Cursor, Read, Write};
        use zip_aes::unstable::write::FileOptionsExt;
        let payload = (0..1000).map(|index| format!("original complete content, including the final bytes {index}").into_bytes())
            .find(|bytes| (1..=191).contains(&(crc32fast::hash(bytes) >> 24))).unwrap();
        let check = (crc32fast::hash(&payload) >> 24) as u8;
        let mut writer = zip_aes::ZipWriter::new(Cursor::new(Vec::new()));
        writer.start_file("资料.txt", zip_aes::write::SimpleFileOptions::default()
            .last_modified_time(zip_aes::DateTime::from_date_and_time(2001, 1, 1, check >> 3, (check & 7) << 3, 0).unwrap())
            .compression_method(zip_aes::CompressionMethod::Stored)
            .with_deprecated_encryption(b"correct").unwrap()).unwrap();
        writer.write_all(&payload).unwrap();
        let mut bytes = writer.finish().unwrap().into_inner();
        let end = bytes.len() - 22;
        let central = u32::from_le_bytes(bytes[end+16..end+20].try_into().unwrap()) as usize;
        let crc = u32::from_le_bytes(bytes[central+16..central+20].try_into().unwrap());
        assert_eq!((crc >> 24) as u8, check);
        assert_ne!(bytes[6] & 8, 0, "writer already emits a real data descriptor");
        // The writer uses the timestamp convention. Its chosen high byte was
        // equal to the payload CRC high byte; now change only the metadata time
        // to distinguish CRC from time, leaving encryption and content intact.
        bytes[10..12].copy_from_slice(&0u16.to_le_bytes());
        bytes[central+12..central+14].copy_from_slice(&0u16.to_le_bytes());
        let mut archive = zip_aes::ZipArchive::new(Cursor::new(bytes.clone())).unwrap();
        // The dependency remains strict by default; only the shared application
        // reader explicitly accepts the observed legacy header convention.
        let strict = archive.by_index_decrypt(0, b"correct").map(|_| ());
        assert!(matches!(strict, Err(zip_aes::result::ZipError::InvalidPassword)), "strict reader result: {strict:?}");
        let mut actual = Vec::new();
        open_entry(&mut archive, 0, "correct").unwrap().read_to_end(&mut actual).unwrap();
        assert_eq!(actual, payload);
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("legacy.zip");
        std::fs::write(&path, &bytes).unwrap();
        assert!(crate::services::universal_engine::UniversalCliEngine::try_zip_password(&path, "correct").unwrap());
        assert!(!crate::services::universal_engine::UniversalCliEngine::try_zip_password(&path, "wrong").is_ok_and(|accepted| accepted));
        let data_start = archive.by_index_raw(0).unwrap().data_start().unwrap() as usize;
        bytes[data_start + 15] ^= 0x40;
        std::fs::write(&path, bytes).unwrap();
        let error = crate::services::universal_engine::UniversalCliEngine::try_zip_password(&path, "correct").unwrap_err();
        assert!(error.to_string().contains("ambiguous-data"), "corrupt content is not a verified password: {error}");
    }

    #[test]
    fn reads_utf8_and_local_archives_without_changing_the_password() {
        use std::io::{Cursor, Read, Write};
        for password in ["ASCII + !", "密码 + !测试", "密码🗝️"] {
            for encoding in [ZipPasswordEncoding::Utf8, ZipPasswordEncoding::WindowsLocal] {
                let Ok(bytes) = encode(password, encoding) else { continue };
                let mut writer = zip_aes::ZipWriter::new(Cursor::new(Vec::new()));
                writer.start_file("资料.txt", zip_aes::write::SimpleFileOptions::default()
                    .with_aes_encryption_bytes(zip_aes::AesMode::Aes256, &bytes)).unwrap();
                writer.write_all(b"exact original payload").unwrap();
                let data = writer.finish().unwrap();
                let fixture = tempfile::tempdir().unwrap();
                let path = fixture.path().join("fixture.zip");
                std::fs::write(&path, data.get_ref()).unwrap();
                assert!(crate::services::universal_engine::UniversalCliEngine::try_zip_password(&path, password).unwrap());
                assert!(!crate::services::universal_engine::UniversalCliEngine::try_zip_password(&path, "wrong").unwrap());
                assert!(crate::services::compression_verification::verify_native(
                    crate::services::compression_format::CompressionRoute::Zip,
                    &path, Some(password), || false,
                ).unwrap());
                let mut archive = zip_aes::ZipArchive::new(data).unwrap();
                let mut actual = String::new();
                open_entry(&mut archive, 0, password).unwrap().read_to_string(&mut actual).unwrap();
                assert_eq!(actual, "exact original payload");
                assert!(open_named_entry(&mut archive, "资料.txt", password).is_ok());
                assert!(matches!(open_named_entry(&mut archive, "missing", password), Err(zip_aes::result::ZipError::FileNotFound)));
                assert!(matches!(open_entry(&mut archive, 0, "wrong"), Err(zip_aes::result::ZipError::InvalidPassword)));
                assert!(matches!(open_entry(&mut archive, 99, password), Err(zip_aes::result::ZipError::FileNotFound)));
            }
        }
    }

    #[test]
    fn utf8_is_default_and_preserves_every_character() {
        assert_eq!(ZipPasswordEncoding::default(), ZipPasswordEncoding::Utf8);
        for value in ["", "ASCII + !", "密码🗝️", "é\0尾"] {
            assert_eq!(encode(value, ZipPasswordEncoding::default()).unwrap(), value.as_bytes());
        }
        assert!(serde_json::from_str::<ZipPasswordEncoding>("\"unknown\"").is_err());
    }

    #[cfg(windows)]
    #[test]
    fn plain_entries_are_not_fully_read_during_encoding_selection() {
        use std::io::{Cursor, Read, Write};
        let mut writer = zip_aes::ZipWriter::new(Cursor::new(Vec::new()));
        writer.start_file("plain", zip_aes::write::SimpleFileOptions::default()
            .compression_method(zip_aes::CompressionMethod::Stored)).unwrap();
        writer.write_all(b"nonempty payload").unwrap();
        let data = writer.finish().unwrap();
        let mut archive = zip_aes::ZipArchive::new(data.clone()).unwrap();
        let start = archive.by_index_raw(0).unwrap().data_start().unwrap() as usize;
        let mut damaged = data.into_inner(); damaged[start] ^= 1;
        let mut archive = zip_aes::ZipArchive::new(Cursor::new(damaged)).unwrap();
        // Opening an unencrypted entry does not pre-read its whole payload;
        // the subsequent real read must still reject the broken CRC.
        let mut entry = open_entry(&mut archive, 0, "中文密码").unwrap();
        assert!(entry.read_to_end(&mut Vec::new()).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn zipcrypto_short_header_collision_does_not_hide_valid_local_password() {
        use std::io::{Cursor, Read, Write};
        use zip_aes::unstable::write::FileOptionsExt;
        // ZipCrypto has only a short password check. Find a real collision,
        // then require full content validation to choose the correct encoding.
        for index in 0..8192 {
            let password = format!("中文密码{index}");
            let Ok(local) = encode(&password, ZipPasswordEncoding::WindowsLocal) else { return };
            if local == password.as_bytes() { return; }
            let mut writer = zip_aes::ZipWriter::new(Cursor::new(Vec::new()));
            let options = zip_aes::write::SimpleFileOptions::default()
                .compression_method(zip_aes::CompressionMethod::Stored)
                .with_deprecated_encryption(&local).unwrap();
            writer.start_file("payload.txt", options).unwrap();
            writer.write_all(b"exact nonempty original payload").unwrap();
            let data = writer.finish().unwrap();
            let mut archive = zip_aes::ZipArchive::new(data.clone()).unwrap();
            if archive.by_index_decrypt(0, password.as_bytes()).is_err() { continue; }
            let mut contents = Vec::new();
            open_entry(&mut archive, 0, &password).unwrap().read_to_end(&mut contents).unwrap();
            assert_eq!(contents, b"exact nonempty original payload");
            let data_start = archive.by_index_raw(0).unwrap().data_start().unwrap() as usize;
            let mut damaged = data.into_inner();
            damaged[data_start + 15] ^= 0x40;
            let mut damaged = zip_aes::ZipArchive::new(Cursor::new(damaged)).unwrap();
            let accepted = open_entry(&mut damaged, 0, &password).is_ok_and(|mut entry| {
                std::io::copy(&mut entry, &mut std::io::sink()).is_ok()
            });
            assert!(!accepted, "Encoding fallback must not accept damaged content");
            return;
        }
        panic!("Failed to construct a ZipCrypto short-header collision");
    }

    #[cfg(windows)]
    #[test]
    fn local_encoding_preserves_ascii_and_rejects_lossy_cp936_passwords() {
        assert_eq!(encode("ASCII + !", ZipPasswordEncoding::WindowsLocal).unwrap(), b"ASCII + !");
        assert_eq!(encode("", ZipPasswordEncoding::WindowsLocal).unwrap(), b"");
        if unsafe { windows_sys::Win32::Globalization::GetACP() } == 936 {
            let (expected, _, errors) = encoding_rs::GBK.encode("密码 + !测试");
            assert!(!errors);
            assert_eq!(encode("密码 + !测试", ZipPasswordEncoding::WindowsLocal).unwrap(), expected.as_ref());
            assert!(encode("密码🗝️", ZipPasswordEncoding::WindowsLocal).is_err());
        }
    }
}
