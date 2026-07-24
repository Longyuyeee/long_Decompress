use anyhow::{Context, Result};
use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHasher, SaltString},
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyType {
    Master,
    Data,
    Symmetric,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyAlgorithm {
    Aes256Gcm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEntry {
    pub id: String,
    pub name: String,
    pub key_type: KeyType,
    pub algorithm: KeyAlgorithm,
    pub encrypted_key: String,
    pub nonce: String,
    pub salt: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[allow(dead_code)]
pub struct KeyManager {
    keys_dir: PathBuf,
}

impl KeyManager {
    pub fn new(data_dir: &Path) -> Result<Self> {
        let keys_dir = data_dir.join("keys");
        fs::create_dir_all(&keys_dir)
            .with_context(|| format!("创建密钥目录失败: {}", keys_dir.display()))?;
        Ok(Self { keys_dir })
    }

    pub async fn initialize(&self, _password: &str) -> Result<()> {
        Err(anyhow::anyhow!("KeyManager::initialize 尚未实现 — 请使用 EncryptedPasswordService::unlock"))
    }

    pub async fn unlock(&self, _password: &str) -> Result<bool> {
        Err(anyhow::anyhow!("KeyManager::unlock 尚未实现 — 请使用 EncryptedPasswordService::unlock"))
    }

    pub async fn generate_key(&self, _name: &str, _key_type: KeyType, _algorithm: KeyAlgorithm) -> Result<KeyEntry> {
        Err(anyhow::anyhow!("KeyManager::generate_key 尚未实现"))
    }

    pub async fn get_key_data(&self, _id: &str) -> Result<Vec<u8>> {
        Err(anyhow::anyhow!("KeyManager::get_key_data 尚未实现 — 请使用 EncryptionService"))
    }

    pub async fn list_keys(&self) -> Result<Vec<KeyEntry>> {
        Ok(Vec::new())
    }

    pub fn derive_master_key(&self, password: &str, salt_bytes: &[u8]) -> Result<Vec<u8>> {
        let salt = SaltString::encode_b64(salt_bytes).map_err(|e| anyhow::anyhow!("编码盐值失败: {}", e))?;

        let argon2 = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(65536, 2, 1, Some(32)).map_err(|e| anyhow::anyhow!("创建Argon2参数失败: {}", e))?,
        );

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("哈希密码失败: {}", e))?;

        let hash = password_hash.hash.context("获取哈希字节失败")?;
        Ok(hash.as_bytes().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_manager_reports_an_unusable_data_directory() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let file_path = temp.path().join("not-a-directory");
        fs::write(&file_path, b"occupied")?;

        let error = KeyManager::new(&file_path)
            .err()
            .context("a file-backed data path must be rejected")?;

        assert!(error.to_string().contains("创建密钥目录失败"));
        Ok(())
    }
}
