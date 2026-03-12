use anyhow::{Context, Result};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHasher, SaltString},
};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};

const KEY_SIZE: usize = 32;
const NONCE_SIZE: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    pub ciphertext: String,
    pub nonce: String,
    pub salt: Option<String>,
}

pub struct EncryptionService {
    key: Vec<u8>,
}

impl EncryptionService {
    pub fn new(key: Vec<u8>) -> Self {
        Self { key }
    }

    pub fn new_random() -> Self {
        use rand::RngCore;
        let mut key = vec![0u8; KEY_SIZE];
        rand::thread_rng().fill_bytes(&mut key);
        Self { key }
    }

    pub fn from_base64_key(base64_key: &str) -> Result<Self> {
        let key = general_purpose::STANDARD.decode(base64_key).context("解码密钥失败")?;
        Ok(Self::new(key))
    }

    pub fn from_password(password: &str, salt: Option<&[u8]>) -> Result<Self> {
        let salt_bytes = salt.unwrap_or(b"default_salt_123");
        let key = Self::derive_key(password, salt_bytes)?;
        Ok(Self::new(key))
    }

    /// 派生密钥
    pub fn derive_key(password: &str, salt_bytes: &[u8]) -> Result<Vec<u8>> {
        let salt = SaltString::encode_b64(salt_bytes).map_err(|e| anyhow::anyhow!("编码盐值失败: {}", e))?;

        let argon2 = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(65536, 2, 1, Some(KEY_SIZE)).map_err(|e| anyhow::anyhow!("创建Argon2参数失败: {}", e))?,
        );

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("哈希密码失败: {}", e))?;

        let hash_bytes = password_hash.hash.context("获取哈希字节失败")?;
        Ok(hash_bytes.as_bytes().to_vec())
    }

    /// 加密数据
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedData> {
        let cipher = Aes256Gcm::new_from_slice(&self.key).context("创建加密器失败")?;
        
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| anyhow::anyhow!("加密数据失败: {}", e))?;

        Ok(EncryptedData {
            ciphertext: general_purpose::STANDARD.encode(ciphertext),
            nonce: general_purpose::STANDARD.encode(nonce_bytes),
            salt: None,
        })
    }

    /// 解密数据
    pub fn decrypt(&self, encrypted_data: &EncryptedData) -> Result<Vec<u8>> {
        let ciphertext = general_purpose::STANDARD.decode(&encrypted_data.ciphertext).context("解码密文失败")?;
        let nonce_bytes = general_purpose::STANDARD.decode(&encrypted_data.nonce).context("解码随机数失败")?;

        let cipher = Aes256Gcm::new_from_slice(&self.key).context("创建解密器失败")?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| anyhow::anyhow!("解密数据失败: {}", e))?;

        Ok(plaintext)
    }

    pub fn encrypt_string(&self, plaintext: &str) -> Result<EncryptedData> {
        self.encrypt(plaintext.as_bytes())
    }

    pub fn decrypt_string(&self, encrypted_data: &EncryptedData) -> Result<String> {
        let decrypted = self.decrypt(encrypted_data)?;
        String::from_utf8(decrypted).context("密文不是有效的UTF-8字符串")
    }
}
