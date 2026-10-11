//! ZIP password byte encoding. UTF-8 remains the default; local compatibility
//! must be selected explicitly and must never replace unrepresentable characters.
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

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

/// Read historical UTF-8 archives first, then try the current Windows encoding
/// only for an explicit password mismatch. I/O, corruption and unsupported
/// methods are never swallowed as a reason to try another encoding.
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
        return archive.by_index_decrypt(index, password.as_bytes());
    };
    let mismatch = match archive.by_index_decrypt(index, password.as_bytes()) {
        Ok(_) => false,
        Err(zip_aes::result::ZipError::InvalidPassword) => true,
        Err(error) => return Err(error),
    };
    if mismatch {
        return archive.by_index_decrypt(index, &local);
    }
    archive.by_index_decrypt(index, password.as_bytes())
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
