/// ZipCrypto (32-bit variant) as used in EGG archives.
///
/// The key difference from standard PKZIP: DecryptByte uses full 32-bit
/// multiply instead of 16-bit truncation.
const CRC32_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut j = 0;
        while j < 8 {
            if c & 1 != 0 {
                c = 0xEDB88320 ^ (c >> 1);
            } else {
                c >>= 1;
            }
            j += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
};

pub trait Decryptor {
    fn decrypt(&mut self, data: &mut [u8]);
}

pub struct ZipCrypto {
    key: [u32; 3],
}

impl ZipCrypto {
    pub fn new(password: &[u8]) -> Self {
        let mut c = ZipCrypto {
            key: [0x12345678, 0x23456789, 0x34567890],
        };
        for &b in password {
            c.update_keys(b);
        }
        c
    }

    fn crc32_byte(crc: u32, b: u8) -> u32 {
        CRC32_TABLE[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8)
    }

    fn update_keys(&mut self, c: u8) {
        self.key[0] = Self::crc32_byte(self.key[0], c);
        self.key[1] = self.key[1].wrapping_add(self.key[0] & 0xFF);
        self.key[1] = self.key[1].wrapping_mul(134775813).wrapping_add(1);
        self.key[2] = Self::crc32_byte(self.key[2], (self.key[1] >> 24) as u8);
    }

    fn decrypt_byte(&self) -> u8 {
        // 32-bit variant: no truncation to u16
        let temp = self.key[2] | 2;
        (temp.wrapping_mul(temp ^ 1) >> 8) as u8
    }

    /// Verify password by decrypting the 12-byte header.
    /// Returns true if password matches. On success, keys are in the correct
    /// state for data decryption.
    pub fn check_password(&mut self, verify_data: &[u8; 12], stored_crc: u32) -> bool {
        let mut last_byte = 0u8;
        for &b in verify_data.iter() {
            let c = b ^ self.decrypt_byte();
            self.update_keys(c);
            last_byte = c;
        }
        last_byte == (stored_crc >> 24) as u8
    }

    /// Encrypt data in-place.
    /// encrypt: cipher = plain ^ keystream; update_keys(plain)
    pub fn encrypt(&mut self, data: &mut [u8]) {
        for b in data.iter_mut() {
            let plain = *b;
            *b = plain ^ self.decrypt_byte();
            self.update_keys(plain);
        }
    }
}

impl Decryptor for ZipCrypto {
    /// Decrypt data in-place.
    /// decrypt: plain = cipher ^ keystream; update_keys(plain)
    fn decrypt(&mut self, data: &mut [u8]) {
        for b in data.iter_mut() {
            let temp = *b ^ self.decrypt_byte();
            self.update_keys(temp);
            *b = temp;
        }
    }
}
