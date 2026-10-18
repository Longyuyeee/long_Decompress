/// AES-128/256 CTR mode encryption for EGG archives.
/// Uses PBKDF2-HMAC-SHA1 key derivation.
/// WinZip AE-2 compatible: counter starts at 1, little-endian increment.
use aes::cipher::{BlockCipherEncrypt, KeyInit};
use pbkdf2::pbkdf2_hmac;
use sha1::Sha1;

use crate::crypto::Decryptor;
use crate::error::{EggError, EggResult};

enum AesCipher {
    Aes128(Box<aes::Aes128>),
    Aes256(Box<aes::Aes256>),
}

pub struct AesCtrDecryptor {
    cipher: AesCipher,
    counter: [u8; 16],
    keystream: [u8; 16],
    ks_offset: usize,
    auth_key: Vec<u8>,
}

/// WinZip AE-2 CTR counter: little-endian increment.
fn ctr128_inc_le(counter: &mut [u8; 16]) {
    for byte in counter.iter_mut() {
        *byte = byte.wrapping_add(1);
        if *byte != 0 {
            break;
        }
    }
}

impl AesCtrDecryptor {
    /// Create a new AES-CTR decryptor for the given encryption method.
    /// mode: 1 = AES-128, 3 = AES-256
    pub fn new(
        mode: u8,
        password: &[u8],
        salt: &[u8],
        stored_verifier: &[u8; 2],
    ) -> EggResult<Self> {
        let key_size = match mode {
            1 => 16, // AES-128
            3 => 32, // AES-256
            _ => return Err(EggError::UnsupportedEncryption(mode)),
        };

        // PBKDF2-HMAC-SHA1: derive key_size*2 + 2 bytes
        let dk_len = key_size * 2 + 2;
        let mut derived = vec![0u8; dk_len];
        pbkdf2_hmac::<Sha1>(password, salt, 1000, &mut derived);

        let enc_key = &derived[..key_size];
        let auth_key = derived[key_size..key_size * 2].to_vec();
        let verifier = &derived[key_size * 2..key_size * 2 + 2];

        // Password verification
        if verifier != stored_verifier {
            return Err(EggError::InvalidPassword);
        }

        let cipher = match mode {
            1 => AesCipher::Aes128(Box::new(
                aes::Aes128::new_from_slice(enc_key).map_err(|_| EggError::CorruptedFile)?,
            )),
            3 => AesCipher::Aes256(Box::new(
                aes::Aes256::new_from_slice(enc_key).map_err(|_| EggError::CorruptedFile)?,
            )),
            _ => unreachable!(),
        };

        // WinZip AE-2: counter starts at 1 (little-endian)
        let mut counter = [0u8; 16];
        counter[0] = 1;

        Ok(AesCtrDecryptor {
            cipher,
            counter,
            keystream: [0u8; 16],
            ks_offset: 16, // forces generation on first use
            auth_key,
        })
    }

    /// PBKDF2-derived HMAC-SHA1 authentication key (WinZip AE-2).
    pub fn auth_key(&self) -> &[u8] {
        &self.auth_key
    }

    fn next_keystream_block(&mut self) {
        let mut block = aes::Block::default();
        block.copy_from_slice(&self.counter);
        match &self.cipher {
            AesCipher::Aes128(c) => c.encrypt_block(&mut block),
            AesCipher::Aes256(c) => c.encrypt_block(&mut block),
        }
        self.keystream.copy_from_slice(&block);
        ctr128_inc_le(&mut self.counter);
        self.ks_offset = 0;
    }
}

impl Decryptor for AesCtrDecryptor {
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
