//! PKware ZIP traditional encryption.

/// Length of the encryption check header preceding encrypted entry data.
pub const ENCR_HEADER_LEN: usize = 12;

/// Standard CRC32 lookup table (polynomial 0xEDB88320).
const CRC32_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0u32;
    while i < 256 {
        let mut crc = i;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i as usize] = crc;
        i += 1;
    }
    table
};

/// Traditional PKWARE ZIP cipher state.
pub struct ZipCrypto {
    key: [u32; 3],
}

impl ZipCrypto {
    /// Initialize the cipher keys from a password (CP949 bytes for ALZ).
    pub fn new(password: &[u8]) -> Self {
        let mut c = ZipCrypto {
            key: [305419896, 591751049, 878082192],
        };
        for &b in password {
            c.update_keys(b);
        }
        c
    }

    fn crc32_byte(crc: u32, b: u8) -> u32 {
        CRC32_TABLE[((crc ^ b as u32) & 0xff) as usize] ^ (crc >> 8)
    }

    fn update_keys(&mut self, c: u8) {
        self.key[0] = Self::crc32_byte(self.key[0], c);
        self.key[1] = self.key[1].wrapping_add(self.key[0] & 0xff);
        self.key[1] = self.key[1].wrapping_mul(134775813).wrapping_add(1);
        self.key[2] = Self::crc32_byte(self.key[2], (self.key[1] >> 24) as u8);
    }

    fn decrypt_byte(&self) -> u8 {
        let temp = (self.key[2] | 2) as u16;
        ((temp.wrapping_mul(temp ^ 1)) >> 8) as u8
    }

    /// Validate the 12-byte encryption header.
    /// Returns true if password is correct.
    pub fn check_header(
        &mut self,
        enc_header: &[u8; ENCR_HEADER_LEN],
        file_crc: u32,
        file_time_date: u32,
        is_data_descr: bool,
    ) -> bool {
        let mut last_byte = 0u8;
        for &b in enc_header.iter() {
            let c = b ^ self.decrypt_byte();
            self.update_keys(c);
            last_byte = c;
        }

        if is_data_descr {
            (file_time_date >> 8) as u8 == last_byte
        } else {
            (file_crc >> 24) as u8 == last_byte
        }
    }

    /// Decrypt data in place.
    pub fn decrypt(&mut self, data: &mut [u8]) {
        for b in data.iter_mut() {
            let temp = *b ^ self.decrypt_byte();
            self.update_keys(temp);
            *b = temp;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_keys() {
        let c = ZipCrypto::new(b"");
        assert_eq!(c.key, [305419896, 591751049, 878082192]);
    }

    #[test]
    fn test_key_update_deterministic() {
        let c1 = ZipCrypto::new(b"password");
        let c2 = ZipCrypto::new(b"password");
        assert_eq!(c1.key, c2.key);
    }

    #[test]
    fn test_different_passwords_different_keys() {
        let c1 = ZipCrypto::new(b"abc");
        let c2 = ZipCrypto::new(b"xyz");
        assert_ne!(c1.key, c2.key);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let data = b"hello world";
        let mut encrypted = *data;

        let mut c = ZipCrypto::new(b"secret");
        for b in encrypted.iter_mut() {
            let plain = *b;
            *b = plain ^ c.decrypt_byte();
            c.update_keys(plain);
        }

        let mut c = ZipCrypto::new(b"secret");
        c.decrypt(&mut encrypted);
        assert_eq!(&encrypted, data);
    }

    #[test]
    fn test_crc32_table_spot_check() {
        // CRC32 of 0x00 with polynomial 0xEDB88320
        assert_eq!(CRC32_TABLE[0], 0x00000000);
        assert_eq!(CRC32_TABLE[1], 0x77073096);
        assert_eq!(CRC32_TABLE[255], 0x2D02EF8D);
    }
}
