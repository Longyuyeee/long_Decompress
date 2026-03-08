use anyhow::{Context, Result};
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::str;

const NONCE_SIZE: usize = 12; // 96 bits for AES-GCM
const KEY_SIZE: usize = 32; // 256 bits for AES-256

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    pub ciphertext: String,
    pub nonce: String,
    pub salt: Option<String>,
}

pub struct EncryptionService {
    key: [u8; KEY_SIZE],
}

impl EncryptionService {
    /// 从密码派生密钥创建加密服务
    pub fn from_password(password: &str, salt: Option<&[u8]>) -> Result<Self> {
        use argon2::{
            Algorithm, Argon2, Params, Version,
            password_hash::{PasswordHasher, SaltString},
        };

        let salt = if let Some(salt_bytes) = salt {
            SaltString::encode_b64(salt_bytes).context("编码盐值失败")?
        } else {
            SaltString::generate(&mut OsRng)
        };

        let argon2 = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(65536, 2, 1, Some(KEY_SIZE)).context("创建Argon2参数失败")?,
        );

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .context("哈希密码失败")?;

        let hash_bytes = password_hash.hash.context("获取哈希字节失败")?;
        let mut key = [0u8; KEY_SIZE];
        key.copy_from_slice(&hash_bytes.as_bytes()[..KEY_SIZE]);

        Ok(Self { key })
    }

    /// 使用随机密钥创建加密服务
    pub fn new_random() -> Self {
        let mut key = [0u8; KEY_SIZE];
        OsRng.fill_bytes(&mut key);

        Self { key }
    }

    /// 加密数据
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedData> {
        let cipher = Aes256Gcm::new_from_slice(&self.key).context("创建加密器失败")?;

        // 生成随机nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // 加密数据
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .context("加密数据失败")?;

        Ok(EncryptedData {
            ciphertext: general_purpose::STANDARD.encode(ciphertext),
            nonce: general_purpose::STANDARD.encode(nonce_bytes),
            salt: None,
        })
    }

    /// 解密数据
    pub fn decrypt(&self, encrypted: &EncryptedData) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new_from_slice(&self.key).context("创建解密器失败")?;

        // 解码nonce和密文
        let nonce_bytes = general_purpose::STANDARD
            .decode(&encrypted.nonce)
            .context("解码nonce失败")?;
        let ciphertext = general_purpose::STANDARD
            .decode(&encrypted.ciphertext)
            .context("解码密文失败")?;

        let nonce = Nonce::from_slice(&nonce_bytes);

        // 解密数据
        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .context("解密数据失败")?;

        Ok(plaintext)
    }

    /// 加密字符串
    pub fn encrypt_string(&self, text: &str) -> Result<EncryptedData> {
        self.encrypt(text.as_bytes())
    }

    /// 解密字符串
    pub fn decrypt_string(&self, encrypted: &EncryptedData) -> Result<String> {
        let bytes = self.decrypt(encrypted)?;
        String::from_utf8(bytes).context("解密后的数据不是有效的UTF-8字符串")
    }

    /// 加密JSON数据
    pub fn encrypt_json<T: Serialize>(&self, data: &T) -> Result<EncryptedData> {
        let json = serde_json::to_vec(data).context("序列化JSON失败")?;
        self.encrypt(&json)
    }

    /// 解密JSON数据
    pub fn decrypt_json<T: for<'de> Deserialize<'de>>(&self, encrypted: &EncryptedData) -> Result<T> {
        let bytes = self.decrypt(encrypted)?;
        serde_json::from_slice(&bytes).context("反序列化JSON失败")
    }

    /// 获取密钥（base64编码）
    pub fn get_key_base64(&self) -> String {
        general_purpose::STANDARD.encode(&self.key)
    }

    /// 从base64编码的密钥创建加密服务
    pub fn from_base64_key(key_base64: &str) -> Result<Self> {
        let key_bytes = general_purpose::STANDARD
            .decode(key_base64)
            .context("解码密钥失败")?;

        if key_bytes.len() != KEY_SIZE {
            return Err(anyhow::anyhow!("密钥长度不正确"));
        }

        let mut key = [0u8; KEY_SIZE];
        key.copy_from_slice(&key_bytes);

        Ok(Self { key })
    }
}

/// 文件加密功能
pub struct FileEncryption;

impl FileEncryption {
    /// 加密文件
    pub fn encrypt_file(
        input_path: &str,
        output_path: &str,
        password: &str,
    ) -> Result<EncryptedData> {
        use std::fs;
        use std::io::Read;

        // 读取文件内容
        let mut file = fs::File::open(input_path).context("打开输入文件失败")?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).context("读取文件失败")?;

        // 创建加密服务并加密数据
        let encryption_service = EncryptionService::from_password(password, None)?;
        let encrypted_data = encryption_service.encrypt(&buffer)?;

        // 将加密数据写入输出文件
        let encrypted_json = serde_json::to_vec(&encrypted_data).context("序列化加密数据失败")?;
        fs::write(output_path, encrypted_json).context("写入输出文件失败")?;

        Ok(encrypted_data)
    }

    /// 解密文件
    pub fn decrypt_file(
        input_path: &str,
        output_path: &str,
        password: &str,
    ) -> Result<Vec<u8>> {
        use std::fs;

        // 读取加密文件
        let encrypted_json = fs::read(input_path).context("读取加密文件失败")?;
        let encrypted_data: EncryptedData =
            serde_json::from_slice(&encrypted_json).context("解析加密数据失败")?;

        // 创建加密服务并解密数据
        let encryption_service = EncryptionService::from_password(password, None)?;
        let decrypted_data = encryption_service.decrypt(&encrypted_data)?;

        // 将解密数据写入输出文件
        fs::write(output_path, &decrypted_data).context("写入输出文件失败")?;

        Ok(decrypted_data)
    }

    /// 检查文件是否加密
    pub fn is_encrypted_file(file_path: &str) -> Result<bool> {
        use std::fs;

        let content = fs::read(file_path).context("读取文件失败")?;

        // 尝试解析为EncryptedData
        match serde_json::from_slice::<EncryptedData>(&content) {
            Ok(encrypted_data) => {
                // 检查必要的字段是否存在
                Ok(!encrypted_data.ciphertext.is_empty() && !encrypted_data.nonce.is_empty())
            }
            Err(_) => Ok(false),
        }
    }
}

/// 内存安全工具
pub struct MemorySafe;

impl MemorySafe {
    /// 安全地清空内存中的敏感数据
    pub fn zeroize_bytes(bytes: &mut [u8]) {
        use zeroize::Zeroize;
        bytes.zeroize();
    }

    /// 安全地清空字符串
    pub fn zeroize_string(s: &mut String) {
        unsafe {
            let bytes = s.as_bytes_mut();
            Self::zeroize_bytes(bytes);
        }
        s.clear();
    }

    /// 创建安全缓冲区（使用后自动清零）
    pub fn secure_buffer(size: usize) -> SecureBuffer {
        SecureBuffer::new(size)
    }
}

/// 安全缓冲区（使用后自动清零）
pub struct SecureBuffer {
    data: Vec<u8>,
}

impl SecureBuffer {
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0u8; size],
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl Drop for SecureBuffer {
    fn drop(&mut self) {
        MemorySafe::zeroize_bytes(&mut self.data);
    }
}

impl std::ops::Deref for SecureBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl std::ops::DerefMut for SecureBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() {
        let service = EncryptionService::new_random();
        let plaintext = "这是一个测试消息";

        let encrypted = service.encrypt_string(plaintext).unwrap();
        let decrypted = service.decrypt_string(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_password_based_encryption() {
        let password = "my_secret_password";
        let plaintext = "敏感数据";

        let service = EncryptionService::from_password(password, None).unwrap();
        let encrypted = service.encrypt_string(plaintext).unwrap();
        let decrypted = service.decrypt_string(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_json_encryption() {
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct TestData {
            name: String,
            value: i32,
            secret: bool,
        }

        let service = EncryptionService::new_random();
        let data = TestData {
            name: "测试".to_string(),
            value: 42,
            secret: true,
        };

        let encrypted = service.encrypt_json(&data).unwrap();
        let decrypted: TestData = service.decrypt_json(&encrypted).unwrap();

        assert_eq!(data, decrypted);
    }
}