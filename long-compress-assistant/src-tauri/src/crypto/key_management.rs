use anyhow::{Context, Result};
use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHash, PasswordHasher, SaltString},
};
use base64::{engine::general_purpose, Engine as _};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use crate::crypto::encryption::{EncryptionService, EncryptedData};

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

pub struct KeyManager {
    keys_dir: PathBuf,
}

impl KeyManager {
    pub fn new(data_dir: &Path) -> Self {
        let keys_dir = data_dir.join("keys");
        if !keys_dir.exists() {
            fs::create_dir_all(&keys_dir).unwrap();
        }
        Self { keys_dir }
    }

    pub async fn initialize(&self, _password: &str) -> Result<()> {
        Ok(())
    }

    pub async fn unlock(&self, _password: &str) -> Result<bool> {
        Ok(true)
    }

    pub async fn generate_key(&self, name: &str, key_type: KeyType, algorithm: KeyAlgorithm) -> Result<KeyEntry> {
        Ok(KeyEntry {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            key_type,
            algorithm,
            encrypted_key: String::new(),
            nonce: String::new(),
            salt: String::new(),
            created_at: chrono::Utc::now(),
        })
    }

    pub async fn get_key_data(&self, _id: &str) -> Result<Vec<u8>> {
        Ok(vec![0u8; 32])
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
