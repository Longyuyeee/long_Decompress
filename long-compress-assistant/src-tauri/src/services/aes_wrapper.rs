use crate::services::aes_stream_v2::{AesStreamKind, AesStreamV2, AES_MAGIC_V2};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use anyhow::{anyhow, Context, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use zeroize::{Zeroize, Zeroizing};

/// 通用 AES 加密包装器。
///
/// 新文件使用分块的 AESENC02；AESENC01 仅保留受限的只读兼容。
pub struct AesWrapper;

const LEGACY_MAGIC: &[u8; 8] = b"AESENC01";
const SALT_SIZE: usize = 32;
const NONCE_SIZE: usize = 12;
const MAX_LEGACY_AES_BYTES: u64 = 512 * 1024 * 1024;

impl AesWrapper {
    pub fn encrypt_file(input: &Path, output: &Path, password: &str) -> Result<()> {
        AesStreamV2::encrypt_file(input, output, password, AesStreamKind::Generic)
    }

    pub fn decrypt_file(input: &Path, output: &Path, password: &str) -> Result<()> {
        if AesStreamV2::is_kind(input, AesStreamKind::Generic)? {
            return AesStreamV2::decrypt_file(input, output, password, AesStreamKind::Generic);
        }

        let decrypted_data = Self::decrypt_legacy_data(input, password)?;
        let mut output_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)
            .with_context(|| format!("创建输出文件失败: {:?}", output))?;
        output_file
            .write_all(&decrypted_data)
            .context("写入输出文件失败")?;
        output_file.sync_all()?;
        Ok(())
    }

    pub fn verify_password(input: &Path, password: &str) -> Result<bool> {
        if AesStreamV2::is_kind(input, AesStreamKind::Generic)? {
            return AesStreamV2::verify_password(input, password, AesStreamKind::Generic);
        }
        if !Self::has_magic(input, LEGACY_MAGIC)? {
            return Ok(false);
        }
        Ok(Self::decrypt_legacy_data(input, password).is_ok())
    }

    fn decrypt_legacy_data(input: &Path, password: &str) -> Result<Zeroizing<Vec<u8>>> {
        if input.metadata()?.len() > MAX_LEGACY_AES_BYTES + 128 {
            return Err(anyhow!(
                "Legacy AES container exceeds the safe 512 MiB in-memory limit"
            ));
        }
        let mut file =
            File::open(input).with_context(|| format!("打开加密文件失败: {:?}", input))?;

        let mut magic = [0u8; 8];
        file.read_exact(&mut magic).context("读取文件头失败")?;
        if &magic != LEGACY_MAGIC {
            return Err(anyhow!("无效的加密文件格式"));
        }

        let mut salt = [0u8; SALT_SIZE];
        file.read_exact(&mut salt).context("读取盐值失败")?;
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        file.read_exact(&mut nonce_bytes)
            .context("读取 nonce 失败")?;

        let mut ciphertext = Vec::new();
        file.read_to_end(&mut ciphertext)
            .context("读取旧版加密数据失败")?;
        let key = Self::derive_legacy_key(password, &salt)?;
        let cipher = Aes256Gcm::new(&key);
        cipher
            .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
            .map(Zeroizing::new)
            .map_err(|_| anyhow!("解密失败: 密码错误或文件已损坏"))
    }

    fn derive_legacy_key(password: &str, salt: &[u8]) -> Result<Key<Aes256Gcm>> {
        let mut key_bytes = [0u8; 32];
        let params = Params::new(65_536, 3, 1, Some(32))
            .map_err(|error| anyhow!("创建 Argon2 参数失败: {error:?}"))?;
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        argon2
            .hash_password_into(password.as_bytes(), salt, &mut key_bytes)
            .map_err(|error| anyhow!("密钥派生失败: {error:?}"))?;
        let key = *Key::<Aes256Gcm>::from_slice(&key_bytes);
        key_bytes.zeroize();
        Ok(key)
    }

    fn has_magic(path: &Path, expected: &[u8; 8]) -> Result<bool> {
        let mut file = File::open(path)?;
        let mut magic = [0u8; 8];
        if file.read_exact(&mut magic).is_err() {
            return Ok(false);
        }
        Ok(&magic == expected)
    }

    pub fn is_aes_encrypted(path: &Path) -> Result<bool> {
        let mut file = File::open(path)?;
        let mut magic = [0u8; 8];
        if file.read_exact(&mut magic).is_err() {
            return Ok(false);
        }
        Ok(&magic == LEGACY_MAGIC || &magic == AES_MAGIC_V2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;
    use std::fs;
    use tempfile::TempDir;

    fn write_legacy_fixture(data: &[u8], output: &Path, password: &str) -> Result<()> {
        let mut salt = [0u8; SALT_SIZE];
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut salt);
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let key = AesWrapper::derive_legacy_key(password, &salt)?;
        let cipher = Aes256Gcm::new(&key);
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), data)
            .map_err(|_| anyhow!("legacy fixture encryption failed"))?;
        let mut file = File::create(output)?;
        file.write_all(LEGACY_MAGIC)?;
        file.write_all(&salt)?;
        file.write_all(&nonce_bytes)?;
        file.write_all(&ciphertext)?;
        Ok(())
    }

    #[test]
    fn new_files_use_v2_and_roundtrip() -> Result<()> {
        let temp = TempDir::new()?;
        let input = temp.path().join("input.txt");
        let encrypted = temp.path().join("encrypted.aes");
        let decrypted = temp.path().join("decrypted.txt");
        fs::write(&input, b"Test data for encryption")?;

        AesWrapper::encrypt_file(&input, &encrypted, "test_password")?;
        assert!(AesWrapper::is_aes_encrypted(&encrypted)?);
        assert_eq!(&fs::read(&encrypted)?[..8], AES_MAGIC_V2);
        AesWrapper::decrypt_file(&encrypted, &decrypted, "test_password")?;
        assert_eq!(fs::read(decrypted)?, b"Test data for encryption");
        Ok(())
    }

    #[test]
    fn legacy_v1_remains_readable() -> Result<()> {
        let temp = TempDir::new()?;
        let encrypted = temp.path().join("legacy.aes");
        let decrypted = temp.path().join("decrypted.txt");
        write_legacy_fixture(b"legacy data", &encrypted, "password")?;

        assert!(AesWrapper::is_aes_encrypted(&encrypted)?);
        assert!(AesWrapper::verify_password(&encrypted, "password")?);
        AesWrapper::decrypt_file(&encrypted, &decrypted, "password")?;
        assert_eq!(fs::read(decrypted)?, b"legacy data");
        Ok(())
    }

    #[test]
    fn wrong_password_does_not_create_output() -> Result<()> {
        let temp = TempDir::new()?;
        let input = temp.path().join("input.txt");
        let encrypted = temp.path().join("encrypted.aes");
        let decrypted = temp.path().join("decrypted.txt");
        fs::write(&input, b"Secret data")?;
        AesWrapper::encrypt_file(&input, &encrypted, "correct_password")?;

        let error = AesWrapper::decrypt_file(&encrypted, &decrypted, "wrong_password").unwrap_err();
        assert!(error.to_string().contains("密码错误"));
        assert!(!decrypted.exists());
        Ok(())
    }
}
