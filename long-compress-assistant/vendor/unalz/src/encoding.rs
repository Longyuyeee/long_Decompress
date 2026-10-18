//! CP949/EUC-KR filename and password conversion.

/// Convert CP949/EUC-KR encoded filename bytes to a UTF-8 string.
pub fn cp949_to_utf8(bytes: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }

    let (cow, _encoding_used, _had_errors) = encoding_rs::EUC_KR.decode(bytes);
    cow.into_owned()
}

/// Encode a password string to CP949/EUC-KR bytes for ZIP key derivation.
/// ALZ producers are ANSI-codepage Korean Windows apps and derive the
/// traditional-ZIP keys from the CP949 byte representation of the password,
/// not UTF-8. ASCII passwords encode identically in both, so this only
/// changes behavior for non-ASCII passwords.
pub fn password_to_cp949(password: &str) -> Vec<u8> {
    let (cow, _encoding_used, _had_errors) = encoding_rs::EUC_KR.encode(password);
    cow.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utf8_passthrough() {
        assert_eq!(cp949_to_utf8(b"hello.txt"), "hello.txt");
        assert_eq!(cp949_to_utf8("테스트.txt".as_bytes()), "테스트.txt");
    }

    #[test]
    fn test_cp949_decode() {
        // "한글" in CP949: 0xC7, 0xD1, 0xB1, 0xDB
        let cp949 = b"\xc7\xd1\xb1\xdb";
        assert_eq!(cp949_to_utf8(cp949), "한글");
    }

    #[test]
    fn test_empty() {
        assert_eq!(cp949_to_utf8(b""), "");
    }

    #[test]
    fn test_password_to_cp949() {
        // ASCII passwords are unchanged (CP949 is an ASCII superset).
        assert_eq!(password_to_cp949("test1234"), b"test1234");
        // "한글" round-trips to its CP949 bytes.
        assert_eq!(password_to_cp949("한글"), b"\xc7\xd1\xb1\xdb");
    }
}
