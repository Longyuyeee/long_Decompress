use anyhow::{Context, Result};
use crate::crypto::encryption::{EncryptionService, EncryptedData};
use crate::crypto::key_management::KeyManager;
use base64::Engine as _;
use crate::crypto::hashing::{HashingService, HashResult};
use crate::models::password::{PasswordEntry, PasswordGroup};
use crate::services::password_strength_service::{
    PasswordImportExportOptions
};
use crate::database::models::{PasswordEntryDb, PasswordGroupDb};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::fs;
use uuid::Uuid;
use chrono::{DateTime, Local, Utc};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

const DPAPI_KEY_PREFIX: &str = "long-dpapi:v1:";
const PASSWORD_CIPHERTEXT_PREFIX: &str = "long-vault:v2:";

/// 加密密码服务
pub struct EncryptedPasswordService {
    pub key_manager: Arc<RwLock<Option<KeyManager>>>,
    protection_key: Arc<RwLock<Option<Vec<u8>>>>,
    pub data_dir: PathBuf,
    pub master_password_hash: Option<HashResult>,
}

impl Clone for EncryptedPasswordService {
    fn clone(&self) -> Self {
        Self {
            key_manager: self.key_manager.clone(),
            protection_key: self.protection_key.clone(),
            data_dir: self.data_dir.clone(),
            master_password_hash: self.master_password_hash.clone(),
        }
    }
}

impl EncryptedPasswordService {
    /// 创建新的加密密码服务
    pub fn new(data_dir: &Path) -> Self {
        Self {
            key_manager: Arc::new(RwLock::new(None)),
            protection_key: Arc::new(RwLock::new(None)),
            data_dir: data_dir.to_path_buf(),
            master_password_hash: None,
        }
    }

    /// 获取或创建每安装实例的随机主密钥。
    /// Windows 使用当前用户 DPAPI 保护磁盘密钥；旧版明文密钥在成功读取后原地迁移。
    pub async fn get_or_create_master_key(data_dir: &Path) -> Result<String> {
        let key_path = data_dir.join("installation.key");
        if key_path.exists() {
            let stored = fs::read_to_string(&key_path).await?;
            let stored = stored.trim();
            if stored.is_empty() {
                return Err(anyhow::anyhow!("安装密钥为空，拒绝生成新密钥以避免覆盖既有密码数据"));
            }

            #[cfg(windows)]
            {
                if let Some(encoded) = stored.strip_prefix(DPAPI_KEY_PREFIX) {
                    let protected = base64::engine::general_purpose::STANDARD
                        .decode(encoded)
                        .context("安装密钥的 DPAPI 数据无效")?;
                    let plaintext = crate::crypto::os_protection::unprotect_for_current_user(&protected)?;
                    return String::from_utf8(plaintext).context("DPAPI 安装密钥不是有效 UTF-8");
                }

                // v1.1.14 及更早版本把随机安装密钥明文保存在此文件。
                // 只有 DPAPI 保护和原子替换都成功后才完成迁移。
                Self::store_master_key(&key_path, stored).await?;
                return Ok(stored.to_string());
            }

            #[cfg(not(windows))]
            return Ok(stored.to_string());
        }

        let random_key = Self::generate_installation_key();
        fs::create_dir_all(data_dir).await?;
        Self::store_master_key(&key_path, &random_key).await?;
        Self::restrict_key_file_permissions(&key_path)?;
        Ok(random_key)
    }

    async fn store_master_key(key_path: &Path, plaintext: &str) -> Result<()> {
        #[cfg(windows)]
        let stored = {
            let protected = crate::crypto::os_protection::protect_for_current_user(plaintext.as_bytes())?;
            format!(
                "{}{}",
                DPAPI_KEY_PREFIX,
                base64::engine::general_purpose::STANDARD.encode(protected)
            )
        };

        #[cfg(not(windows))]
        let stored = plaintext.to_string();

        Self::write_atomically(key_path, stored.as_bytes()).await
    }

    async fn write_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
        let parent = path.parent().context("目标文件缺少父目录")?;
        fs::create_dir_all(parent).await?;
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .context("目标文件名无效")?;
        let temporary = parent.join(format!(".{}.{}.tmp", file_name, Uuid::new_v4()));
        fs::write(&temporary, bytes).await?;

        #[cfg(windows)]
        let replace_result = {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Storage::FileSystem::{
                MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
            };

            let from: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
            let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let result = unsafe {
                MoveFileExW(
                    from.as_ptr(),
                    to.as_ptr(),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
            };
            if result == 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        };

        #[cfg(not(windows))]
        let replace_result = fs::rename(&temporary, path).await;

        if let Err(error) = replace_result {
            let _ = fs::remove_file(&temporary).await;
            return Err(error).with_context(|| format!("原子替换失败: {}", path.display()));
        }
        Ok(())
    }

    /// 生成随机安装密钥 (32 字节的 base64 编码)
    fn generate_installation_key() -> String {
        use rand::Rng;
        use base64::Engine;
        let random_bytes: [u8; 32] = rand::thread_rng().gen();
        base64::engine::general_purpose::STANDARD.encode(random_bytes)
    }

    fn derive_data_protection_key(master_key: &str) -> Vec<u8> {
        if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(master_key) {
            if decoded.len() == 32 {
                return decoded;
            }
        }

        // Compatibility for tests and pre-random-key installations. The production installation
        // key is already 256 bits of randomness; this branch only normalizes legacy input to 32 bytes.
        let mut digest = Sha256::new();
        digest.update(b"LongDecompress.PasswordVault.v2\0");
        digest.update(master_key.as_bytes());
        digest.finalize().to_vec()
    }

    /// 在 Unix 上设置文件权限为 0600 (仅所有者可读写)
    #[cfg(unix)]
    fn restrict_key_file_permissions(path: &Path) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(path, perms)?;
        Ok(())
    }

    /// 在 Windows 上，默认继承目录权限，但可以通过 ACL 限制 (skip for now)
    #[cfg(not(unix))]
    fn restrict_key_file_permissions(_path: &Path) -> Result<()> {
        Ok(())
    }

    /// 初始化服务（设置主密码）
    pub async fn initialize(&mut self, master_password: &str) -> Result<()> {
        fs::create_dir_all(&self.data_dir).await?;
        let key_manager = KeyManager::new(&self.data_dir)?;

        // 保存主密码哈希
        self.master_password_hash = Some(HashingService::hash_password(master_password)?);

        // 保存哈希到文件
        let hash_path = self.data_dir.join("master_password.hash");
        let hash_json = serde_json::to_string(&self.master_password_hash)?;
        fs::write(&hash_path, hash_json).await?;

        // 设置密钥管理器
        let mut key_manager_lock = self.key_manager.write().await;
        *key_manager_lock = Some(key_manager);
        let mut protection_key = self.protection_key.write().await;
        *protection_key = Some(Self::derive_data_protection_key(master_password));

        Ok(())
    }

    /// 解锁服务
    pub async fn unlock(&mut self, master_password: &str) -> Result<bool> {
        // 验证主密码
        let hash_path = self.data_dir.join("master_password.hash");
        if !hash_path.exists() {
            return Ok(false);
        }

        let hash_json = fs::read_to_string(&hash_path).await?;
        let stored_hash: HashResult = serde_json::from_str(&hash_json)?;

        let is_valid = HashingService::verify_password(master_password, &stored_hash)?;
        if !is_valid {
            return Ok(false);
        }

        let key_manager = KeyManager::new(&self.data_dir)?;

        // 设置密钥管理器
        let mut key_manager_lock = self.key_manager.write().await;
        *key_manager_lock = Some(key_manager);
        let mut protection_key = self.protection_key.write().await;
        *protection_key = Some(Self::derive_data_protection_key(master_password));

        Ok(true)
    }

    /// 锁定服务
    pub async fn lock(&mut self) {
        let mut key_manager_lock = self.key_manager.write().await;
        *key_manager_lock = None;
        let mut protection_key = self.protection_key.write().await;
        if let Some(mut key) = protection_key.take() {
            key.zeroize();
        }
    }

    /// 检查是否已解锁
    pub fn is_unlocked_sync(&self) -> bool {
        // 注意：这里需要慎用，因为 tokio::sync::RwLock 没有同步的 try_read 保证
        // 但在 Tauri 命令中我们通常是 async 的
        false 
    }

    pub async fn is_unlocked(&self) -> bool {
        self.protection_key.read().await.is_some()
    }

    /// 添加密码条目
    pub async fn add_password(&self, mut entry: PasswordEntry) -> Result<PasswordEntry> {
        if !self.is_unlocked().await {
            return Err(anyhow::anyhow!("密码服务未解锁"));
        }

        entry.id = Uuid::new_v4().to_string();
        entry.created_at = Utc::now();
        entry.updated_at = Utc::now();

        self.save_password_entry(&entry).await?;
        Ok(entry)
    }

    /// 更新密码条目
    pub async fn update_password(&self, id: &str, mut entry: PasswordEntry) -> Result<PasswordEntry> {
        if !self.is_unlocked().await {
            return Err(anyhow::anyhow!("密码服务未解锁"));
        }

        entry.updated_at = Utc::now();
        entry.id = id.to_string();

        self.save_password_entry(&entry).await?;
        Ok(entry)
    }

    /// 增加密码使用次数
    pub async fn increment_use_count(&self, id: &str) -> Result<PasswordEntry> {
        if !self.is_unlocked().await {
            return Err(anyhow::anyhow!("密码服务未解锁"));
        }

        // 获取当前条目
        let mut entry = self.get_password(id).await?
            .ok_or_else(|| anyhow::anyhow!("密码条目不存在: {}", id))?;

        // 增加计数并更新时间
        entry.use_count += 1;
        let now = Utc::now();
        entry.last_used = Some(now);
        entry.updated_at = now;

        // 记录历史
        // 日趋势面向用户展示，按机器本地自然日归档，避免 UTC 跨日后
        // “刚刚使用”落入昨天的统计桶。
        let date_str = Local::now().format("%Y-%m-%d").to_string();
        let count = entry.usage_history.entry(date_str).or_insert(0);
        *count += 1;

        // 保存更新
        self.save_password_entry(&entry).await?;

        Ok(entry)
    }

    /// 删除密码条目
    pub async fn delete_password(&self, id: &str) -> Result<()> {
        if !self.is_unlocked().await {
            return Err(anyhow::anyhow!("密码服务未解锁"));
        }

        // 删除条目文件
        let entry_path = self.data_dir.join("passwords").join(format!("{}.json", id));
        if entry_path.exists() {
            fs::remove_file(&entry_path).await?;
        }

        Ok(())
    }

    /// 清空所有密码条目
    pub async fn clear_all_passwords(&self) -> Result<()> {
        if !self.is_unlocked().await {
            return Err(anyhow::anyhow!("密码服务未解锁"));
        }

        let passwords_dir = self.data_dir.join("passwords");
        if passwords_dir.exists() {
            fs::remove_dir_all(&passwords_dir).await.context("无法删除密码目录")?;
        }
        
        // 重新创建空的密码目录
        fs::create_dir_all(&passwords_dir).await.context("无法重新创建密码目录")?;

        Ok(())
    }

    /// 搜索密码条目
    pub async fn search_passwords(&self, query: &str) -> Result<Vec<PasswordEntry>> {
        if !self.is_unlocked().await {
            return Err(anyhow::anyhow!("密码服务未解锁"));
        }

        // 这里简化处理，实际应该从数据库搜索
        let passwords_dir = self.data_dir.join("passwords");
        if !passwords_dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        let mut dir = fs::read_dir(&passwords_dir).await?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let id = path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .context("密码条目文件名无效")?;
                let decrypted_entry = self
                    .load_password_entry(id)
                    .await?
                    .context("密码条目在读取期间消失")?;

                // 搜索条件
                let query_lower = query.to_lowercase();
                let matches = decrypted_entry.name.to_lowercase().contains(&query_lower) ||
                    decrypted_entry.username.as_ref().is_some_and(|u| u.to_lowercase().contains(&query_lower)) ||
                    decrypted_entry.url.as_ref().is_some_and(|u| u.to_lowercase().contains(&query_lower)) ||
                    decrypted_entry.notes.as_ref().is_some_and(|n| n.to_lowercase().contains(&query_lower)) ||
                    decrypted_entry.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower));

                if matches {
                    entries.push(decrypted_entry);
                }
            }
        }

        Ok(entries)
    }

    /// 列出所有密码条目
    pub async fn list_passwords(&self) -> Result<Vec<PasswordEntry>> {
        self.search_passwords("").await
    }

    /// 获取密码条目并在 Rust 内存中解密密码正文。
    pub async fn get_password(&self, id: &str) -> Result<Option<PasswordEntry>> {
        if !self.is_unlocked().await {
            return Err(anyhow::anyhow!("密码服务未解锁"));
        }

        self.load_password_entry(id).await
    }

    /// 导出密码本
    pub async fn export_passwords(
        &self,
        options: &PasswordImportExportOptions,
        export_password: &str,
    ) -> Result<Vec<u8>> {
        if !self.is_unlocked().await {
            return Err(anyhow::anyhow!("密码服务未解锁"));
        }

        let entries = self.list_passwords().await?;

        // 创建导出数据
        let export_data = ExportData {
            version: "1.0".to_string(),
            export_date: Utc::now(),
            entry_count: entries.len(),
            entries: if options.include_passwords {
                entries
            } else {
                // 不包含密码，只包含元数据
                entries.into_iter().map(|mut entry| {
                    entry.password = "".to_string(); // 清空密码
                    entry
                }).collect()
            },
            metadata: if options.include_metadata {
                Some(HashMap::from([
                    ("export_format".to_string(), format!("{:?}", options.format)),
                    ("export_date".to_string(), Utc::now().to_rfc3339()),
                ]))
            } else {
                None
            },
        };

        let export_json = serde_json::to_vec(&export_data)?;

        if options.encrypt {
            let (encryption_service, salt) = EncryptionService::from_password(export_password, None)?;
            let mut encrypted_data = encryption_service.encrypt(&export_json)?;
            // 将盐值保存到导出数据中，以便导入时可以还原密钥
            encrypted_data.salt = Some(base64::engine::general_purpose::STANDARD.encode(&salt));
            Ok(serde_json::to_vec(&encrypted_data)?)
        } else {
            Ok(export_json)
        }
    }

    /// 导入密码本
    pub async fn import_passwords(
        &self,
        import_data: &[u8],
        options: &PasswordImportExportOptions,
        import_password: Option<&str>,
    ) -> Result<usize> {
        if !self.is_unlocked().await {
            return Err(anyhow::anyhow!("密码服务未解锁"));
        }

        let data = if options.encrypt {
            let password = import_password.ok_or_else(|| anyhow::anyhow!("需要导入密码"))?;
            let encrypted_data: EncryptedData = serde_json::from_slice(import_data)?;
            // 从导出数据中恢复盐值，若无则使用空盐值（向后兼容）
            let salt_bytes = encrypted_data.salt.as_deref().map(|s| {
                base64::engine::general_purpose::STANDARD.decode(s).unwrap_or_default()
            });
            let encryption_service = if let Some(ref salt) = salt_bytes {
                EncryptionService::from_password_with_salt(password, salt)?
            } else {
                let (svc, _) = EncryptionService::from_password(password, None)?;
                svc
            };
            encryption_service.decrypt(&encrypted_data)?
        } else {
            import_data.to_vec()
        };

        let export_data: ExportData = serde_json::from_slice(&data)?;

        let mut imported_count = 0;
        for entry in export_data.entries {
            // 检查是否已存在
            let existing_entry = self.get_password(&entry.id).await?;
            if existing_entry.is_none() {
                self.add_password(entry).await?;
                imported_count += 1;
            }
        }

        Ok(imported_count)
    }

    async fn encrypt_password_for_storage(&self, password: &str) -> Result<String> {
        let mut key = self
            .protection_key
            .read()
            .await
            .as_ref()
            .cloned()
            .context("密码服务未解锁")?;
        let result = (|| {
            let encryption_service = EncryptionService::new(key.clone());
            let encrypted = encryption_service.encrypt_string(password)?;
            let payload = serde_json::to_vec(&encrypted)?;
            Ok(format!(
                "{}{}",
                PASSWORD_CIPHERTEXT_PREFIX,
                base64::engine::general_purpose::STANDARD.encode(payload)
            ))
        })();
        key.zeroize();
        result
    }

    async fn decrypt_password_from_storage(&self, stored: &str) -> Result<(String, bool)> {
        let Some(encoded) = stored.strip_prefix(PASSWORD_CIPHERTEXT_PREFIX) else {
            return Ok((stored.to_string(), true));
        };

        let mut key = self
            .protection_key
            .read()
            .await
            .as_ref()
            .cloned()
            .context("密码服务未解锁")?;
        let result = (|| {
            let payload = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .context("密码条目密文编码无效")?;
            let encrypted: EncryptedData = serde_json::from_slice(&payload)
                .context("密码条目密文结构无效")?;
            let encryption_service = EncryptionService::new(key.clone());
            encryption_service.decrypt_string(&encrypted)
        })();
        key.zeroize();
        result.map(|password| (password, false))
    }

    /// 保存密码条目。磁盘 JSON 只保存 AES-GCM 密文，不保存密码正文。
    async fn save_password_entry(&self, entry: &PasswordEntry) -> Result<()> {
        let passwords_dir = self.data_dir.join("passwords");
        if !passwords_dir.exists() {
            fs::create_dir_all(&passwords_dir).await?;
        }

        let entry_path = passwords_dir.join(format!("{}.json", entry.id));
        let mut db_entry: PasswordEntryDb = entry.clone().into();
        db_entry.password = self.encrypt_password_for_storage(&entry.password).await?;
        db_entry.key_id = "installation-key".to_string();
        db_entry.encryption_algorithm = if cfg!(windows) {
            "AES256GCM+WindowsDPAPI"
        } else {
            "AES256GCM+UserFilePermissions"
        }
        .to_string();
        db_entry.encryption_version = 2;
        let entry_json = serde_json::to_string(&db_entry)?;

        Self::write_atomically(&entry_path, entry_json.as_bytes()).await?;

        Ok(())
    }

    /// 从文件加载密码条目。旧版明文只在成功读取后迁移为 v2 密文。
    async fn load_password_entry(&self, id: &str) -> Result<Option<PasswordEntry>> {
        let entry_path = self.data_dir.join("passwords").join(format!("{}.json", id));

        if !entry_path.exists() {
            return Ok(None);
        }

        let entry_json = fs::read_to_string(&entry_path).await?;
        let mut db_entry: PasswordEntryDb = serde_json::from_str(&entry_json)?;
        let (password, needs_migration) = self
            .decrypt_password_from_storage(&db_entry.password)
            .await
            .with_context(|| format!("无法解锁密码条目 {}", id))?;
        db_entry.password = password;
        let entry: PasswordEntry = db_entry.into();

        if needs_migration {
            self.save_password_entry(&entry)
                .await
                .with_context(|| format!("迁移旧密码条目失败: {}", id))?;
        }

        Ok(Some(entry))
    }
}

/// 导出数据格式
#[derive(Debug, Serialize, Deserialize)]
struct ExportData {
    version: String,
    export_date: DateTime<Utc>,
    entry_count: usize,
    entries: Vec<PasswordEntry>,
    metadata: Option<HashMap<String, String>>,
}

/// 密码组管理
pub struct PasswordGroupService {
    encrypted_password_service: Arc<EncryptedPasswordService>,
}

impl PasswordGroupService {
    pub fn new(encrypted_password_service: Arc<EncryptedPasswordService>) -> Self {
        Self {
            encrypted_password_service,
        }
    }

    /// 创建密码组
    pub async fn create_group(&self, mut group: PasswordGroup) -> Result<PasswordGroup> {
        if !self.encrypted_password_service.is_unlocked().await {
            return Err(anyhow::anyhow!("密码服务未解锁"));
        }

        group.id = Uuid::new_v4().to_string();
        group.created_at = Utc::now();
        group.updated_at = Utc::now();

        self.save_password_group(&group).await?;

        Ok(group)
    }

    /// 获取密码组
    pub async fn get_group(&self, id: &str) -> Result<Option<PasswordGroup>> {
        self.load_password_group(id).await
    }

    /// 更新密码组
    pub async fn update_group(&self, id: &str, mut group: PasswordGroup) -> Result<PasswordGroup> {
        if !self.encrypted_password_service.is_unlocked().await {
            return Err(anyhow::anyhow!("密码服务未解锁"));
        }

        group.id = id.to_string();
        group.updated_at = Utc::now();

        self.save_password_group(&group).await?;

        Ok(group)
    }

    /// 删除密码组
    pub async fn delete_group(&self, id: &str) -> Result<()> {
        let group_path = self.encrypted_password_service.data_dir
            .join("groups")
            .join(format!("{}.json", id));

        if group_path.exists() {
            fs::remove_file(&group_path).await?;
        }

        Ok(())
    }

    /// 列出所有密码组
    pub async fn list_groups(&self) -> Result<Vec<PasswordGroup>> {
        let groups_dir = self.encrypted_password_service.data_dir.join("groups");
        if !groups_dir.exists() {
            return Ok(Vec::new());
        }

        let mut groups = Vec::new();
        let mut dir = fs::read_dir(&groups_dir).await?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let content = fs::read_to_string(&path).await?;
                let db_group: PasswordGroupDb = serde_json::from_str(&content)?;
                let group: PasswordGroup = db_group.into();
                groups.push(group);
            }
        }

        Ok(groups)
    }

    /// 向组中添加密码条目
    pub async fn add_entry_to_group(&self, group_id: &str, entry_id: &str) -> Result<()> {
        let mut group = match self.get_group(group_id).await? {
            Some(g) => g,
            None => return Err(anyhow::anyhow!("密码组不存在")),
        };

        // 检查条目是否存在
        let entry = self.encrypted_password_service.get_password(entry_id).await?;
        if entry.is_none() {
            return Err(anyhow::anyhow!("密码条目不存在"));
        }

        group.add_entry(entry_id.to_string());
        self.update_group(group_id, group).await?;

        Ok(())
    }

    /// 从组中移除密码条目
    pub async fn remove_entry_from_group(&self, group_id: &str, entry_id: &str) -> Result<()> {
        let mut group = match self.get_group(group_id).await? {
            Some(g) => g,
            None => return Err(anyhow::anyhow!("密码组不存在")),
        };

        group.remove_entry(entry_id);
        self.update_group(group_id, group).await?;

        Ok(())
    }

    /// 获取组中的所有密码条目
    pub async fn get_group_entries(&self, group_id: &str) -> Result<Vec<PasswordEntry>> {
        let group = match self.get_group(group_id).await? {
            Some(g) => g,
            None => return Err(anyhow::anyhow!("密码组不存在")),
        };

        let mut entries = Vec::new();
        for entry_id in &group.entry_ids {
            if let Some(entry) = self.encrypted_password_service.get_password(entry_id).await? {
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    /// 保存密码组到文件
    async fn save_password_group(&self, group: &PasswordGroup) -> Result<()> {
        let groups_dir = self.encrypted_password_service.data_dir.join("groups");
        if !groups_dir.exists() {
            fs::create_dir_all(&groups_dir).await?;
        }

        let group_path = groups_dir.join(format!("{}.json", group.id));
        let db_group: PasswordGroupDb = group.clone().into();
        let group_json = serde_json::to_string(&db_group)?;

        fs::write(&group_path, group_json).await?;

        Ok(())
    }

    /// 从文件加载密码组
    async fn load_password_group(&self, id: &str) -> Result<Option<PasswordGroup>> {
        let group_path = self.encrypted_password_service.data_dir
            .join("groups")
            .join(format!("{}.json", id));

        if !group_path.exists() {
            return Ok(None);
        }

        let group_json = fs::read_to_string(&group_path).await?;
        let db_group: PasswordGroupDb = serde_json::from_str(&group_json)?;
        let group: PasswordGroup = db_group.into();

        Ok(Some(group))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::password::PasswordCategory;

    #[tokio::test]
    async fn legacy_plaintext_entry_is_migrated_without_changing_the_password() {
        let temp = tempfile::tempdir().unwrap();
        let mut service = EncryptedPasswordService::new(temp.path());
        service.initialize("real-installation-key").await.unwrap();

        let entry = PasswordEntry::new(
            "真实迁移样本".to_string(),
            "archive-secret-迁移".to_string(),
            PasswordCategory::Other,
        );
        let entry_path = temp
            .path()
            .join("passwords")
            .join(format!("{}.json", entry.id));
        fs::create_dir_all(entry_path.parent().unwrap()).await.unwrap();
        let legacy: PasswordEntryDb = entry.clone().into();
        fs::write(&entry_path, serde_json::to_vec(&legacy).unwrap())
            .await
            .unwrap();

        let loaded = service.get_password(&entry.id).await.unwrap().unwrap();
        assert_eq!(loaded.password, entry.password);

        let migrated = fs::read_to_string(&entry_path).await.unwrap();
        assert!(!migrated.contains(&entry.password));
        assert!(migrated.contains(PASSWORD_CIPHERTEXT_PREFIX));
        assert!(migrated.contains("AES256GCM+WindowsDPAPI") || migrated.contains("AES256GCM+UserFilePermissions"));

        service.lock().await;
        let mut reopened = EncryptedPasswordService::new(temp.path());
        assert!(reopened.unlock("real-installation-key").await.unwrap());
        assert_eq!(
            reopened.get_password(&entry.id).await.unwrap().unwrap().password,
            entry.password
        );
    }

    #[tokio::test]
    async fn damaged_ciphertext_is_rejected_without_overwriting_the_record() {
        let temp = tempfile::tempdir().unwrap();
        let mut service = EncryptedPasswordService::new(temp.path());
        service.initialize("real-installation-key").await.unwrap();
        let saved = service
            .add_password(PasswordEntry::new(
                "损坏密文样本".to_string(),
                "must-not-leak".to_string(),
                PasswordCategory::Other,
            ))
            .await
            .unwrap();
        let path = temp.path().join("passwords").join(format!("{}.json", saved.id));
        let mut stored: PasswordEntryDb = serde_json::from_slice(&fs::read(&path).await.unwrap()).unwrap();
        stored.password.push('A');
        let damaged = serde_json::to_vec(&stored).unwrap();
        fs::write(&path, &damaged).await.unwrap();

        assert!(service.get_password(&saved.id).await.is_err());
        assert_eq!(fs::read(&path).await.unwrap(), damaged);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn legacy_installation_key_is_really_migrated_to_windows_dpapi() {
        let temp = tempfile::tempdir().unwrap();
        let key_path = temp.path().join("installation.key");
        let legacy = "legacy-installation-key-real-fixture";
        fs::write(&key_path, legacy).await.unwrap();

        let first = EncryptedPasswordService::get_or_create_master_key(temp.path())
            .await
            .unwrap();
        let stored = fs::read_to_string(&key_path).await.unwrap();
        assert_eq!(first, legacy);
        assert!(stored.starts_with(DPAPI_KEY_PREFIX));
        assert!(!stored.contains(legacy));

        let second = EncryptedPasswordService::get_or_create_master_key(temp.path())
            .await
            .unwrap();
        assert_eq!(second, legacy);
    }
}
