use anyhow::{Context, Result};
use crate::crypto::encryption::EncryptionService;
use crate::crypto::hashing::{HashingService, HashAlgorithm};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEntry {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub key_type: KeyType,
    pub algorithm: KeyAlgorithm,
    pub key_data: String, // 加密存储的密钥数据
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyType {
    Symmetric,
    AsymmetricPublic,
    AsymmetricPrivate,
    Master,
    Session,
    Derived,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyAlgorithm {
    AES256GCM,
    RSA2048,
    RSA4096,
    ECDSAP256,
    ECDSAP384,
    ED25519,
    X25519,
}

impl KeyAlgorithm {
    pub fn name(&self) -> &'static str {
        match self {
            Self::AES256GCM => "AES-256-GCM",
            Self::RSA2048 => "RSA-2048",
            Self::RSA4096 => "RSA-4096",
            Self::ECDSAP256 => "ECDSA P-256",
            Self::ECDSAP384 => "ECDSA P-384",
            Self::ED25519 => "Ed25519",
            Self::X25519 => "X25519",
        }
    }

    pub fn is_symmetric(&self) -> bool {
        matches!(self, Self::AES256GCM)
    }

    pub fn is_asymmetric(&self) -> bool {
        !self.is_symmetric()
    }

    pub fn key_size_bits(&self) -> u32 {
        match self {
            Self::AES256GCM => 256,
            Self::RSA2048 => 2048,
            Self::RSA4096 => 4096,
            Self::ECDSAP256 => 256,
            Self::ECDSAP384 => 384,
            Self::ED25519 => 256,
            Self::X25519 => 256,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyUsagePolicy {
    pub key_id: String,
    pub allowed_operations: Vec<KeyOperation>,
    pub max_usage_count: Option<u32>,
    pub valid_from: chrono::DateTime<chrono::Utc>,
    pub valid_until: Option<chrono::DateTime<chrono::Utc>>,
    pub ip_restrictions: Vec<String>,
    pub user_restrictions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyOperation {
    Encrypt,
    Decrypt,
    Sign,
    Verify,
    Derive,
    Wrap,
    Unwrap,
}

pub struct KeyManager {
    master_key: Arc<RwLock<Option<EncryptionService>>>,
    key_store: Arc<RwLock<HashMap<String, KeyEntry>>>,
    key_store_path: PathBuf,
}

impl KeyManager {
    /// 创建新的密钥管理器
    pub fn new(key_store_path: &Path) -> Self {
        Self {
            master_key: Arc::new(RwLock::new(None)),
            key_store: Arc::new(RwLock::new(HashMap::new())),
            key_store_path: key_store_path.to_path_buf(),
        }
    }

    /// 初始化密钥管理器（设置主密钥）
    pub async fn initialize(&self, master_password: &str) -> Result<()> {
        // 生成主密钥
        let master_key_service = EncryptionService::from_password(master_password, None)?;

        // 保存主密钥哈希（用于验证）
        let master_key_hash = HashingService::hash_password(master_password)?;
        let hash_json = serde_json::to_string(&master_key_hash)?;

        // 创建密钥存储目录
        if let Some(parent) = self.key_store_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // 保存主密钥哈希到文件
        let hash_path = self.key_store_path.with_extension("hash");
        fs::write(&hash_path, hash_json).await?;

        // 设置主密钥
        let mut master_key_lock = self.master_key.write().unwrap();
        *master_key_lock = Some(master_key_service);

        Ok(())
    }

    /// 解锁密钥管理器
    pub async fn unlock(&self, master_password: &str) -> Result<bool> {
        // 读取主密钥哈希
        let hash_path = self.key_store_path.with_extension("hash");
        if !hash_path.exists() {
            return Ok(false);
        }

        let hash_json = fs::read_to_string(&hash_path).await?;
        let master_key_hash: crate::crypto::hashing::HashResult = serde_json::from_str(&hash_json)?;

        // 验证密码
        let is_valid = HashingService::verify_password(master_password, &master_key_hash)?;
        if !is_valid {
            return Ok(false);
        }

        // 创建主密钥服务
        let master_key_service = EncryptionService::from_password(master_password, None)?;

        // 设置主密钥
        let mut master_key_lock = self.master_key.write().unwrap();
        *master_key_lock = Some(master_key_service);

        // 加载密钥存储
        self.load_key_store().await?;

        Ok(true)
    }

    /// 锁定密钥管理器
    pub fn lock(&self) {
        let mut master_key_lock = self.master_key.write().unwrap();
        *master_key_lock = None;

        let mut key_store_lock = self.key_store.write().unwrap();
        key_store_lock.clear();
    }

    /// 检查是否已解锁
    pub fn is_unlocked(&self) -> bool {
        let master_key_lock = self.master_key.read().unwrap();
        master_key_lock.is_some()
    }

    /// 生成新密钥
    pub async fn generate_key(
        &self,
        name: &str,
        key_type: KeyType,
        algorithm: KeyAlgorithm,
        description: Option<&str>,
    ) -> Result<KeyEntry> {
        if !self.is_unlocked() {
            return Err(anyhow::anyhow!("密钥管理器未解锁"));
        }

        let master_key_lock = self.master_key.read().unwrap();
        let master_key = master_key_lock.as_ref().unwrap();

        // 生成密钥数据（这里简化处理，实际需要根据算法生成）
        let key_data = match algorithm {
            KeyAlgorithm::AES256GCM => {
                let key_service = EncryptionService::new_random();
                key_service.get_key_base64()
            }
            _ => {
                // 对于非对称密钥，生成随机数据作为示例
                let random_bytes = HashingService::generate_random_bytes(32);
                base64::engine::general_purpose::STANDARD.encode(random_bytes)
            }
        };

        // 加密密钥数据
        let encrypted_key_data = master_key.encrypt_string(&key_data)?;

        let now = chrono::Utc::now();
        let key_entry = KeyEntry {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            key_type,
            algorithm,
            key_data: serde_json::to_string(&encrypted_key_data)?,
            created_at: now,
            updated_at: now,
            expires_at: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
        };

        // 保存密钥
        self.save_key(&key_entry).await?;

        Ok(key_entry)
    }

    /// 获取密钥
    pub async fn get_key(&self, key_id: &str) -> Result<Option<KeyEntry>> {
        if !self.is_unlocked() {
            return Err(anyhow::anyhow!("密钥管理器未解锁"));
        }

        let key_store_lock = self.key_store.read().unwrap();
        Ok(key_store_lock.get(key_id).cloned())
    }

    /// 获取密钥的明文数据
    pub async fn get_key_data(&self, key_id: &str) -> Result<Option<String>> {
        if !self.is_unlocked() {
            return Err(anyhow::anyhow!("密钥管理器未解锁"));
        }

        let key_entry = match self.get_key(key_id).await? {
            Some(entry) => entry,
            None => return Ok(None),
        };

        let master_key_lock = self.master_key.read().unwrap();
        let master_key = master_key_lock.as_ref().unwrap();

        // 解密密钥数据
        let encrypted_key_data: crate::crypto::encryption::EncryptedData =
            serde_json::from_str(&key_entry.key_data)?;

        let key_data = master_key.decrypt_string(&encrypted_key_data)?;

        Ok(Some(key_data))
    }

    /// 列出所有密钥
    pub async fn list_keys(&self) -> Result<Vec<KeyEntry>> {
        if !self.is_unlocked() {
            return Err(anyhow::anyhow!("密钥管理器未解锁"));
        }

        let key_store_lock = self.key_store.read().unwrap();
        let keys: Vec<KeyEntry> = key_store_lock.values().cloned().collect();

        // 按更新时间排序
        let mut sorted_keys = keys;
        sorted_keys.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(sorted_keys)
    }

    /// 搜索密钥
    pub async fn search_keys(
        &self,
        query: &str,
        key_type: Option<KeyType>,
        algorithm: Option<KeyAlgorithm>,
    ) -> Result<Vec<KeyEntry>> {
        if !self.is_unlocked() {
            return Err(anyhow::anyhow!("密钥管理器未解锁"));
        }

        let key_store_lock = self.key_store.read().unwrap();
        let query_lower = query.to_lowercase();

        let filtered_keys: Vec<KeyEntry> = key_store_lock
            .values()
            .filter(|key| {
                // 搜索条件
                let matches_query = query.is_empty() ||
                    key.name.to_lowercase().contains(&query_lower) ||
                    key.description.as_ref().map_or(false, |d| d.to_lowercase().contains(&query_lower)) ||
                    key.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower));

                let matches_type = key_type.as_ref().map_or(true, |t| &key.key_type == t);
                let matches_algorithm = algorithm.as_ref().map_or(true, |a| &key.algorithm == a);

                matches_query && matches_type && matches_algorithm
            })
            .cloned()
            .collect();

        // 按名称排序
        let mut sorted_keys = filtered_keys;
        sorted_keys.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(sorted_keys)
    }

    /// 更新密钥
    pub async fn update_key(&self, key_entry: &KeyEntry) -> Result<()> {
        if !self.is_unlocked() {
            return Err(anyhow::anyhow!("密钥管理器未解锁"));
        }

        let mut updated_entry = key_entry.clone();
        updated_entry.updated_at = chrono::Utc::now();

        self.save_key(&updated_entry).await
    }

    /// 删除密钥
    pub async fn delete_key(&self, key_id: &str) -> Result<()> {
        if !self.is_unlocked() {
            return Err(anyhow::anyhow!("密钥管理器未解锁"));
        }

        let mut key_store_lock = self.key_store.write().unwrap();
        key_store_lock.remove(key_id);

        self.save_key_store().await
    }

    /// 导出密钥（安全格式）
    pub async fn export_key(&self, key_id: &str, export_password: &str) -> Result<Vec<u8>> {
        if !self.is_unlocked() {
            return Err(anyhow::anyhow!("密钥管理器未解锁"));
        }

        let key_entry = match self.get_key(key_id).await? {
            Some(entry) => entry,
            None => return Err(anyhow::anyhow!("密钥不存在")),
        };

        // 创建导出加密服务
        let export_key_service = EncryptionService::from_password(export_password, None)?;

        // 加密密钥条目
        let encrypted_key = export_key_service.encrypt_json(&key_entry)?;

        // 序列化为JSON
        let export_data = serde_json::to_vec(&encrypted_key)?;

        Ok(export_data)
    }

    /// 导入密钥
    pub async fn import_key(
        &self,
        export_data: &[u8],
        export_password: &str,
    ) -> Result<KeyEntry> {
        if !self.is_unlocked() {
            return Err(anyhow::anyhow!("密钥管理器未解锁"));
        }

        // 创建导出加密服务
        let export_key_service = EncryptionService::from_password(export_password, None)?;

        // 解析导出数据
        let encrypted_key: crate::crypto::encryption::EncryptedData =
            serde_json::from_slice(export_data)?;

        // 解密密钥条目
        let key_entry: KeyEntry = export_key_service.decrypt_json(&encrypted_key)?;

        // 检查密钥ID是否已存在
        let existing_key = self.get_key(&key_entry.id).await?;
        if existing_key.is_some() {
            return Err(anyhow::anyhow!("密钥已存在"));
        }

        // 重新加密密钥数据（使用主密钥）
        let master_key_lock = self.master_key.read().unwrap();
        let master_key = master_key_lock.as_ref().unwrap();

        // 解密原始密钥数据
        let original_encrypted_data: crate::crypto::encryption::EncryptedData =
            serde_json::from_str(&key_entry.key_data)?;
        let original_key_data = export_key_service.decrypt_string(&original_encrypted_data)?;

        // 使用主密钥重新加密
        let reencrypted_key_data = master_key.encrypt_string(&original_key_data)?;

        let mut imported_entry = key_entry.clone();
        imported_entry.key_data = serde_json::to_string(&reencrypted_key_data)?;
        imported_entry.updated_at = chrono::Utc::now();

        // 保存导入的密钥
        self.save_key(&imported_entry).await?;

        Ok(imported_entry)
    }

    /// 轮换密钥（生成新版本）
    pub async fn rotate_key(&self, key_id: &str) -> Result<KeyEntry> {
        if !self.is_unlocked() {
            return Err(anyhow::anyhow!("密钥管理器未解锁"));
        }

        let old_key = match self.get_key(key_id).await? {
            Some(key) => key,
            None => return Err(anyhow::anyhow!("密钥不存在")),
        };

        // 生成新密钥
        let new_key = self.generate_key(
            &format!("{} (轮换)", old_key.name),
            old_key.key_type.clone(),
            old_key.algorithm.clone(),
            old_key.description.as_deref(),
        ).await?;

        // 标记旧密钥为已轮换
        let mut updated_old_key = old_key.clone();
        updated_old_key.metadata.insert("rotated_to".to_string(), new_key.id.clone());
        updated_old_key.metadata.insert("rotated_at".to_string(), chrono::Utc::now().to_rfc3339());

        self.update_key(&updated_old_key).await?;

        Ok(new_key)
    }

    /// 备份密钥存储
    pub async fn backup(&self, backup_path: &Path, backup_password: &str) -> Result<()> {
        if !self.is_unlocked() {
            return Err(anyhow::anyhow!("密钥管理器未解锁"));
        }

        // 读取密钥存储文件
        let key_store_data = fs::read(&self.key_store_path).await?;

        // 加密备份数据
        let backup_key_service = EncryptionService::from_password(backup_password, None)?;
        let encrypted_backup = backup_key_service.encrypt(&key_store_data)?;

        // 写入备份文件
        let backup_json = serde_json::to_vec(&encrypted_backup)?;
        fs::write(backup_path, backup_json).await?;

        Ok(())
    }

    /// 恢复密钥存储
    pub async fn restore(&self, backup_path: &Path, backup_password: &str) -> Result<()> {
        // 读取备份文件
        let backup_json = fs::read(backup_path).await?;
        let encrypted_backup: crate::crypto::encryption::EncryptedData =
            serde_json::from_slice(&backup_json)?;

        // 解密备份数据
        let backup_key_service = EncryptionService::from_password(backup_password, None)?;
        let key_store_data = backup_key_service.decrypt(&encrypted_backup)?;

        // 写入密钥存储文件
        fs::write(&self.key_store_path, key_store_data).await?;

        // 重新加载密钥存储
        self.load_key_store().await?;

        Ok(())
    }

    /// 保存密钥到存储
    async fn save_key(&self, key_entry: &KeyEntry) -> Result<()> {
        let mut key_store_lock = self.key_store.write().unwrap();
        key_store_lock.insert(key_entry.id.clone(), key_entry.clone());

        self.save_key_store().await
    }

    /// 保存密钥存储到文件
    async fn save_key_store(&self) -> Result<()> {
        if !self.is_unlocked() {
            return Err(anyhow::anyhow!("密钥管理器未解锁"));
        }

        let key_store_lock = self.key_store.read().unwrap();
        let key_store_json = serde_json::to_string(&*key_store_lock)?;

        let master_key_lock = self.master_key.read().unwrap();
        let master_key = master_key_lock.as_ref().unwrap();

        // 加密密钥存储
        let encrypted_store = master_key.encrypt_string(&key_store_json)?;
        let encrypted_json = serde_json::to_vec(&encrypted_store)?;

        // 写入文件
        fs::write(&self.key_store_path, encrypted_json).await?;

        Ok(())
    }

    /// 从文件加载密钥存储
    async fn load_key_store(&self) -> Result<()> {
        if !self.key_store_path.exists() {
            return Ok(());
        }

        let master_key_lock = self.master_key.read().unwrap();
        let master_key = match master_key_lock.as_ref() {
            Some(key) => key,
            None => return Err(anyhow::anyhow!("主密钥未设置")),
        };

        // 读取加密的密钥存储
        let encrypted_json = fs::read(&self.key_store_path).await?;
        let encrypted_store: crate::crypto::encryption::EncryptedData =
            serde_json::from_slice(&encrypted_json)?;

        // 解密密钥存储
        let key_store_json = master_key.decrypt_string(&encrypted_store)?;
        let key_store: HashMap<String, KeyEntry> = serde_json::from_str(&key_store_json)?;

        // 更新内存中的密钥存储
        let mut key_store_lock = self.key_store.write().unwrap();
        *key_store_lock = key_store;

        Ok(())
    }
}

/// 密钥派生函数
pub struct KeyDerivation;

impl KeyDerivation {
    /// 从密码派生密钥
    pub fn derive_from_password(
        password: &str,
        salt: &[u8],
        algorithm: HashAlgorithm,
        output_length: usize,
    ) -> Result<Vec<u8>> {
        match algorithm {
            HashAlgorithm::Argon2id => {
                use argon2::{Algorithm, Argon2, Params, Version};
                use argon2::password_hash::{PasswordHasher, SaltString};

                let salt_str = SaltString::encode_b64(salt).context("编码盐值失败")?;
                let argon2 = Argon2::new(
                    Algorithm::Argon2id,
                    Version::V0x13,
                    Params::new(65536, 2, 1, Some(output_length)).context("创建Argon2参数失败")?,
                );

                let password_hash = argon2
                    .hash_password(password.as_bytes(), &salt_str)
                    .context("哈希密码失败")?;

                let hash_bytes = password_hash.hash.context("获取哈希字节失败")?;
                Ok(hash_bytes.as_bytes()[..output_length].to_vec())
            }
            HashAlgorithm::Blake3 => {
                let mut hasher = blake3::Hasher::new();
                hasher.update(password.as_bytes());
                hasher.update(salt);
                let hash = hasher.finalize();
                Ok(hash.as_bytes()[..output_length].to_vec())
            }
            HashAlgorithm::SHA256 => {
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(password.as_bytes());
                hasher.update(salt);
                let hash = hasher.finalize();
                Ok(hash.as_slice()[..output_length.min(32)].to_vec())
            }
            HashAlgorithm::SHA512 => {
                use sha2::{Sha512, Digest};
                let mut hasher = Sha512::new();
                hasher.update(password.as_bytes());
                hasher.update(salt);
                let hash = hasher.finalize();
                Ok(hash.as_slice()[..output_length.min(64)].to_vec())
            }
        }
    }

    /// 派生密钥并创建加密服务
    pub fn derive_encryption_service(
        password: &str,
        salt: &[u8],
        algorithm: HashAlgorithm,
    ) -> Result<EncryptionService> {
        let key_bytes = Self::derive_from_password(password, salt, algorithm, 32)?;

        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes[..32]);

        Ok(EncryptionService { key })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_key_manager_basic() {
        let temp_dir = tempdir().unwrap();
        let key_store_path = temp_dir.path().join("keystore.dat");

        let key_manager = KeyManager::new(&key_store_path);

        // 初始化
        let master_password = "master_password_123";
        key_manager.initialize(master_password).await.unwrap();

        // 解锁
        let unlocked = key_manager.unlock(master_password).await.unwrap();
        assert!(unlocked);
        assert!(key_manager.is_unlocked());

        // 生成密钥
        let key_entry = key_manager
            .generate_key(
                "测试密钥",
                KeyType::Symmetric,
                KeyAlgorithm::AES256GCM,
                Some("测试用密钥"),
            )
            .await
            .unwrap();

        assert_eq!(key_entry.name, "测试密钥");
        assert_eq!(key_entry.key_type, KeyType::Symmetric);
        assert_eq!(key_entry.algorithm, KeyAlgorithm::AES256GCM);

        // 获取密钥
        let retrieved_key = key_manager.get_key(&key_entry.id).await.unwrap();
        assert!(retrieved_key.is_some());
        assert_eq!(retrieved_key.unwrap().name, "测试密钥");

        // 获取密钥数据
        let key_data = key_manager.get_key_data(&key_entry.id).await.unwrap();
        assert!(key_data.is_some());

        // 列出密钥
        let keys = key_manager.list_keys().await.unwrap();
        assert_eq!(keys.len(), 1);

        // 锁定
        key_manager.lock();
        assert!(!key_manager.is_unlocked());
    }

    #[test]
    fn test_key_derivation() {
        let password = "test_password";
        let salt = b"test_salt";

        let derived_key = KeyDerivation::derive_from_password(
            password,
            salt,
            HashAlgorithm::Blake3,
            32,
        ).unwrap();

        assert_eq!(derived_key.len(), 32);

        let encryption_service = KeyDerivation::derive_encryption_service(
            password,
            salt,
            HashAlgorithm::Blake3,
        ).unwrap();

        // 测试加密解密
        let plaintext = "测试数据";
        let encrypted = encryption_service.encrypt_string(plaintext).unwrap();
        let decrypted = encryption_service.decrypt_string(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }
}