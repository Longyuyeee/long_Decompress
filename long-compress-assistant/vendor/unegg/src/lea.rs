/// LEA (Lightweight Encryption Algorithm) block cipher.
/// Implements LEA-128, LEA-192, LEA-256 in CTR mode for EGG archives.
/// Based on KS X 3246 specification.
use pbkdf2::pbkdf2_hmac;
use sha1::Sha1;

use crate::crypto::Decryptor;
use crate::error::{EggError, EggResult};

/// Delta constants from sqrt(766995) where 76/69/95 = 'L','E','A' in ASCII.
const DELTA: [u32; 8] = [
    0xc3efe9db, 0x44626b02, 0x79e27c8a, 0x78df30ec, 0x715ea49e, 0xc785da0a, 0xe04ef22a, 0xe5c40957,
];

fn rol(x: u32, n: u32) -> u32 {
    if n == 0 { x } else { x.rotate_left(n) }
}

fn ror(x: u32, n: u32) -> u32 {
    if n == 0 { x } else { x.rotate_right(n) }
}

/// A single round key: 6 words.
type RoundKey = [u32; 6];

struct LeaCipher {
    round_keys: Vec<RoundKey>,
}

impl LeaCipher {
    fn new_128(key: &[u8; 16]) -> Self {
        let mut t = [0u32; 4];
        for i in 0..4 {
            t[i] = u32::from_le_bytes(key[i * 4..(i + 1) * 4].try_into().unwrap());
        }

        let mut rk = Vec::with_capacity(24);
        for i in 0..24u32 {
            let d = DELTA[(i % 4) as usize];
            t[0] = rol(t[0].wrapping_add(rol(d, i)), 1);
            t[1] = rol(t[1].wrapping_add(rol(d, i.wrapping_add(1))), 3);
            t[2] = rol(t[2].wrapping_add(rol(d, i.wrapping_add(2))), 6);
            t[3] = rol(t[3].wrapping_add(rol(d, i.wrapping_add(3))), 11);
            rk.push([t[0], t[1], t[2], t[1], t[3], t[1]]);
        }

        LeaCipher { round_keys: rk }
    }

    #[allow(dead_code)]
    fn new_192(key: &[u8; 24]) -> Self {
        let mut t = [0u32; 6];
        for i in 0..6 {
            t[i] = u32::from_le_bytes(key[i * 4..(i + 1) * 4].try_into().unwrap());
        }

        let mut rk = Vec::with_capacity(28);
        for i in 0..28u32 {
            let d = DELTA[(i % 6) as usize];
            t[0] = rol(t[0].wrapping_add(rol(d, i)), 1);
            t[1] = rol(t[1].wrapping_add(rol(d, i.wrapping_add(1))), 3);
            t[2] = rol(t[2].wrapping_add(rol(d, i.wrapping_add(2))), 6);
            t[3] = rol(t[3].wrapping_add(rol(d, i.wrapping_add(3))), 11);
            t[4] = rol(t[4].wrapping_add(rol(d, i.wrapping_add(4))), 13);
            t[5] = rol(t[5].wrapping_add(rol(d, i.wrapping_add(5))), 17);
            rk.push([t[0], t[1], t[2], t[3], t[4], t[5]]);
        }

        LeaCipher { round_keys: rk }
    }

    fn new_256(key: &[u8; 32]) -> Self {
        let mut t = [0u32; 8];
        for i in 0..8 {
            t[i] = u32::from_le_bytes(key[i * 4..(i + 1) * 4].try_into().unwrap());
        }

        let mut rk = Vec::with_capacity(32);
        for i in 0..32u32 {
            let d = DELTA[(i % 8) as usize];
            let base = (6u32.wrapping_mul(i)) as usize;
            t[(base) % 8] = rol(t[(base) % 8].wrapping_add(rol(d, i)), 1);
            t[(base + 1) % 8] = rol(t[(base + 1) % 8].wrapping_add(rol(d, i.wrapping_add(1))), 3);
            t[(base + 2) % 8] = rol(t[(base + 2) % 8].wrapping_add(rol(d, i.wrapping_add(2))), 6);
            t[(base + 3) % 8] = rol(
                t[(base + 3) % 8].wrapping_add(rol(d, i.wrapping_add(3))),
                11,
            );
            t[(base + 4) % 8] = rol(
                t[(base + 4) % 8].wrapping_add(rol(d, i.wrapping_add(4))),
                13,
            );
            t[(base + 5) % 8] = rol(
                t[(base + 5) % 8].wrapping_add(rol(d, i.wrapping_add(5))),
                17,
            );
            rk.push([
                t[(base) % 8],
                t[(base + 1) % 8],
                t[(base + 2) % 8],
                t[(base + 3) % 8],
                t[(base + 4) % 8],
                t[(base + 5) % 8],
            ]);
        }

        LeaCipher { round_keys: rk }
    }

    /// Encrypt a single 16-byte block in place.
    fn encrypt_block(&self, block: &mut [u8; 16]) {
        let mut x = [0u32; 4];
        for i in 0..4 {
            x[i] = u32::from_le_bytes(block[i * 4..(i + 1) * 4].try_into().unwrap());
        }

        for rk in &self.round_keys {
            let tmp0 = rol((x[0] ^ rk[0]).wrapping_add(x[1] ^ rk[1]), 9);
            let tmp1 = ror((x[1] ^ rk[2]).wrapping_add(x[2] ^ rk[3]), 5);
            let tmp2 = ror((x[2] ^ rk[4]).wrapping_add(x[3] ^ rk[5]), 3);
            let tmp3 = x[0];
            x = [tmp0, tmp1, tmp2, tmp3];
        }

        for i in 0..4 {
            block[i * 4..(i + 1) * 4].copy_from_slice(&x[i].to_le_bytes());
        }
    }
}

/// CTR mode counter: 16-byte big-endian increment.
fn ctr128_inc(counter: &mut [u8; 16]) {
    for i in (0..16).rev() {
        counter[i] = counter[i].wrapping_add(1);
        if counter[i] != 0 {
            break;
        }
    }
}

pub struct LeaCtrDecryptor {
    cipher: LeaCipher,
    counter: [u8; 16],
    keystream: [u8; 16],
    ks_offset: usize, // how many bytes of current keystream block consumed
    auth_key: Vec<u8>,
}

impl LeaCtrDecryptor {
    /// Create LEA-CTR decryptor.
    /// mode: 1 = LEA-128, 3 = LEA-256
    pub fn new(
        mode: u8,
        password: &[u8],
        salt: &[u8],
        stored_verifier: &[u8; 2],
    ) -> EggResult<Self> {
        let key_size = match mode {
            1 => 16,
            3 => 32,
            _ => return Err(EggError::UnsupportedEncryption(mode)),
        };

        let dk_len = key_size * 2 + 2;
        let mut derived = vec![0u8; dk_len];
        pbkdf2_hmac::<Sha1>(password, salt, 1000, &mut derived);

        let enc_key = &derived[..key_size];
        let auth_key = derived[key_size..key_size * 2].to_vec();
        let verifier = &derived[key_size * 2..key_size * 2 + 2];

        if verifier != stored_verifier {
            return Err(EggError::InvalidPassword);
        }

        let cipher = match mode {
            1 => LeaCipher::new_128(enc_key.try_into().unwrap()),
            3 => LeaCipher::new_256(enc_key.try_into().unwrap()),
            _ => unreachable!(),
        };

        // Zero IV, generate first keystream block
        let counter = [0u8; 16];
        let decryptor = LeaCtrDecryptor {
            cipher,
            counter,
            keystream: [0u8; 16],
            ks_offset: 16, // forces generation on first use
            auth_key,
        };

        Ok(decryptor)
    }

    /// PBKDF2-derived HMAC-SHA1 authentication key (WinZip AE-2 style).
    pub fn auth_key(&self) -> &[u8] {
        &self.auth_key
    }

    fn next_keystream_block(&mut self) {
        self.keystream = self.counter;
        self.cipher.encrypt_block(&mut self.keystream);
        ctr128_inc(&mut self.counter);
        self.ks_offset = 0;
    }
}

impl Decryptor for LeaCtrDecryptor {
    fn decrypt(&mut self, data: &mut [u8]) {
        for b in data.iter_mut() {
            if self.ks_offset >= 16 {
                self.next_keystream_block();
            }
            *b ^= self.keystream[self.ks_offset];
            self.ks_offset += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lea128_encrypt() {
        // Test vector from KS X 3246 Appendix A
        let key: [u8; 16] = [
            0x0f, 0x1e, 0x2d, 0x3c, 0x4b, 0x5a, 0x69, 0x78, 0x87, 0x96, 0xa5, 0xb4, 0xc3, 0xd2,
            0xe1, 0xf0,
        ];
        let plaintext: [u8; 16] = [
            0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d,
            0x1e, 0x1f,
        ];
        let expected: [u8; 16] = [
            0x9f, 0xc8, 0x4e, 0x35, 0x28, 0xc6, 0xc6, 0x18, 0x55, 0x32, 0xc7, 0xa7, 0x04, 0x64,
            0x8b, 0xfd,
        ];

        let cipher = LeaCipher::new_128(&key);
        let mut block = plaintext;
        cipher.encrypt_block(&mut block);
        assert_eq!(block, expected);
    }

    #[test]
    fn test_lea192_encrypt() {
        // Correct test vector from KS X 3246 / Wikipedia
        let key: [u8; 24] = [
            0x0f, 0x1e, 0x2d, 0x3c, 0x4b, 0x5a, 0x69, 0x78, 0x87, 0x96, 0xa5, 0xb4, 0xc3, 0xd2,
            0xe1, 0xf0, 0xf0, 0xe1, 0xd2, 0xc3, 0xb4, 0xa5, 0x96, 0x87,
        ];
        let plaintext: [u8; 16] = [
            0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d,
            0x2e, 0x2f,
        ];
        let expected: [u8; 16] = [
            0x6f, 0xb9, 0x5e, 0x32, 0x5a, 0xad, 0x1b, 0x87, 0x8c, 0xdc, 0xf5, 0x35, 0x76, 0x74,
            0xc6, 0xf2,
        ];

        let cipher = LeaCipher::new_192(&key);
        let mut block = plaintext;
        cipher.encrypt_block(&mut block);
        assert_eq!(block, expected);
    }

    #[test]
    fn test_lea256_encrypt() {
        // Correct test vector from KS X 3246 / Wikipedia
        let key: [u8; 32] = [
            0x0f, 0x1e, 0x2d, 0x3c, 0x4b, 0x5a, 0x69, 0x78, 0x87, 0x96, 0xa5, 0xb4, 0xc3, 0xd2,
            0xe1, 0xf0, 0xf0, 0xe1, 0xd2, 0xc3, 0xb4, 0xa5, 0x96, 0x87, 0x78, 0x69, 0x5a, 0x4b,
            0x3c, 0x2d, 0x1e, 0x0f,
        ];
        let plaintext: [u8; 16] = [
            0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d,
            0x3e, 0x3f,
        ];
        let expected: [u8; 16] = [
            0xd6, 0x51, 0xaf, 0xf6, 0x47, 0xb1, 0x89, 0xc1, 0x3a, 0x89, 0x00, 0xca, 0x27, 0xf9,
            0xe1, 0x97,
        ];

        let cipher = LeaCipher::new_256(&key);
        let mut block = plaintext;
        cipher.encrypt_block(&mut block);
        assert_eq!(block, expected);
    }

    #[test]
    fn test_lea128_round_keys() {
        // Verify first few round keys match spec
        let key: [u8; 16] = [
            0x0f, 0x1e, 0x2d, 0x3c, 0x4b, 0x5a, 0x69, 0x78, 0x87, 0x96, 0xa5, 0xb4, 0xc3, 0xd2,
            0xe1, 0xf0,
        ];
        let cipher = LeaCipher::new_128(&key);

        assert_eq!(
            cipher.round_keys[0],
            [
                0x003a0fd4, 0x02497010, 0x194f7db1, 0x02497010, 0x090d0883, 0x02497010
            ]
        );
        assert_eq!(
            cipher.round_keys[1],
            [
                0x11fdcbb1, 0x9e98e0c8, 0x18b570cf, 0x9e98e0c8, 0x9dc53a79, 0x9e98e0c8
            ]
        );
        assert_eq!(
            cipher.round_keys[2],
            [
                0xf30f7bb5, 0x6d6628db, 0xb74e5dad, 0x6d6628db, 0xa65e46d0, 0x6d6628db
            ]
        );
        assert_eq!(
            cipher.round_keys[3],
            [
                0x74120631, 0xdac9bd17, 0xcd1ecf34, 0xdac9bd17, 0x540f76f1, 0xdac9bd17
            ]
        );
    }

    #[test]
    fn test_lea128_intermediate_state() {
        // Verify X_1 after first round
        let key: [u8; 16] = [
            0x0f, 0x1e, 0x2d, 0x3c, 0x4b, 0x5a, 0x69, 0x78, 0x87, 0x96, 0xa5, 0xb4, 0xc3, 0xd2,
            0xe1, 0xf0,
        ];

        // X_0 = [0x13121110, 0x17161514, 0x1b1a1918, 0x1f1e1d1c] (plaintext as words)
        let x0 = [0x13121110u32, 0x17161514, 0x1b1a1918, 0x1f1e1d1c];
        let cipher = LeaCipher::new_128(&key);
        let rk = &cipher.round_keys[0];

        // After round 0: X_1 = [0x0f079051, 0x693d668d, 0xe5edcfd4, 0x13121110]
        let tmp0 = rol((x0[0] ^ rk[0]).wrapping_add(x0[1] ^ rk[1]), 9);
        let tmp1 = ror((x0[1] ^ rk[2]).wrapping_add(x0[2] ^ rk[3]), 5);
        let tmp2 = ror((x0[2] ^ rk[4]).wrapping_add(x0[3] ^ rk[5]), 3);
        let tmp3 = x0[0];

        assert_eq!(tmp0, 0x0f079051);
        assert_eq!(tmp1, 0x693d668d);
        assert_eq!(tmp2, 0xe5edcfd4);
        assert_eq!(tmp3, 0x13121110);
    }
}
