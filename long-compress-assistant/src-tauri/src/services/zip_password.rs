//! ZIP password byte encoding. UTF-8 remains the default; local compatibility
//! must be selected explicitly and must never replace unrepresentable characters.
use anyhow::{bail, Result};
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
}

pub fn encode(password: &str, encoding: ZipPasswordEncoding) -> Result<Vec<u8>> {
    match encoding {
        ZipPasswordEncoding::Utf8 => Ok(password.as_bytes().to_vec()),
        ZipPasswordEncoding::WindowsLocal => encode_local(password),
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
    let index = archive.index_for_name(name).ok_or(zip_aes::result::ZipError::FileNotFound)?;
    open_entry(archive, index, password)
}

#[cfg(not(windows))]
fn encode_local(_password: &str) -> Result<Vec<u8>> {
    bail!("Windows ZIP password compatibility is only available on Windows")
}

#[cfg(windows)]
fn encode_local(password: &str) -> Result<Vec<u8>> {
    use windows_sys::Win32::Globalization::{GetACP, WideCharToMultiByte, WC_NO_BEST_FIT_CHARS};
    // SAFETY: GetACP takes no pointers and only reads the process code page.
    let code_page = unsafe { GetACP() };
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
        bail!("ZIP password contains characters unavailable in Windows code page {code_page}; use UTF-8 or 7Z");
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
        bail!("ZIP password cannot be encoded without character replacement");
    }
    bytes.truncate(written as usize);
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn real_infozip_fixtures_show_the_current_machine_codepage_boundary() {
        use std::io::{Cursor, Read};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/archives/legacy-zip-password-1416");
        let expected = std::fs::read(root.join("payload.txt")).unwrap();
        let cases: [(u32, &str, &[u8]); 3] = [
            (932, "日本語パスワード+!1416", b"\x93\xfa\x96\x7b\x8c\xea\x83\x70\x83\x58\x83\x8f\x81\x5b\x83\x68+!1416"),
            (936, "简体密码测试+!1416", b"\xbc\xf2\xcc\xe5\xc3\xdc\xc2\xeb\xb2\xe2\xca\xd4+!1416"),
            (950, "繁體密碼測試+!1416", b"\xc1\x63\xc5\xe9\xb1\x4b\xbd\x58\xb4\xfa\xb8\xd5+!1416"),
        ];
        for (codepage, password, known_bytes) in cases {
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
