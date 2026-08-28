use anyhow::{Context, Result};

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{GetLastError, LocalFree},
    Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN,
        CRYPT_INTEGER_BLOB,
    },
};

/// Protect bytes for the current Windows user without showing credential UI.
#[cfg(windows)]
pub fn protect_for_current_user(plaintext: &[u8]) -> Result<Vec<u8>> {
    let input_len = u32::try_from(plaintext.len()).context("DPAPI 输入过大")?;
    let input = CRYPT_INTEGER_BLOB {
        cbData: input_len,
        pbData: plaintext.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };

    let succeeded = unsafe {
        CryptProtectData(
            &input,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if succeeded == 0 {
        let code = unsafe { GetLastError() };
        return Err(anyhow::anyhow!("Windows DPAPI 保护失败（错误码 {}）", code));
    }

    let protected = unsafe {
        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(output.pbData.cast());
        bytes
    };
    Ok(protected)
}

/// Unprotect bytes that were bound to the current Windows user by DPAPI.
#[cfg(windows)]
pub fn unprotect_for_current_user(protected: &[u8]) -> Result<Vec<u8>> {
    let input_len = u32::try_from(protected.len()).context("DPAPI 密文过大")?;
    let input = CRYPT_INTEGER_BLOB {
        cbData: input_len,
        pbData: protected.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };

    let succeeded = unsafe {
        CryptUnprotectData(
            &input,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if succeeded == 0 {
        let code = unsafe { GetLastError() };
        return Err(anyhow::anyhow!("Windows DPAPI 解锁失败（错误码 {}）", code));
    }

    let plaintext = unsafe {
        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(output.pbData.cast());
        bytes
    };
    Ok(plaintext)
}

#[cfg(not(windows))]
pub fn protect_for_current_user(_plaintext: &[u8]) -> Result<Vec<u8>> {
    Err(anyhow::anyhow!("当前用户级数据保护仅支持 Windows"))
}

#[cfg(not(windows))]
pub fn unprotect_for_current_user(_protected: &[u8]) -> Result<Vec<u8>> {
    Err(anyhow::anyhow!("当前用户级数据保护仅支持 Windows"))
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn real_windows_dpapi_round_trip_and_tamper_rejection() {
        let plaintext = b"Long archive password vault real DPAPI fixture";
        let protected = protect_for_current_user(plaintext).expect("protect with real Windows DPAPI");

        assert_ne!(protected, plaintext);
        assert_eq!(unprotect_for_current_user(&protected).unwrap(), plaintext);

        let mut tampered = protected;
        let index = tampered.len() / 2;
        tampered[index] ^= 0x5a;
        assert!(unprotect_for_current_user(&tampered).is_err());
    }
}
