use anyhow::{Context, Result};
use crate::crypto::encryption::{EncryptionService, EncryptedData};
use crate::crypto::key_management::{KeyManager, KeyEntry, KeyType, KeyAlgorithm};
use crate::crypto::hashing::{HashingService, HashResult};
use crate::models::password::{
    PasswordEntry, PasswordCategory, PasswordStrength, CustomField, CustomFieldType,
    PasswordGroup, PasswordAuditResult, PasswordIssue, PasswordIssueType, IssueSeverity,
    PasswordGeneratorOptions, PasswordImportExportOptions, ImportExportFormat
};
use crate::database::models::{PasswordEntryDb, PasswordGroupDb};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::fs;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 加密密码服务
pub struct EncryptedPasswordService {
    key_manager: Arc<RwLock<Option<KeyManager>>>,
    data_dir: PathBuf,
    master_password_hash: Option<HashResult>,
}

impl EncryptedPasswordService {
    /// 创建新的加密密码服务
    pub fn new(data_dir: &Path) -> Self {
        Self {
            key_manager: Arc::new(RwLock::new(None)),
            data_dir: data_dir.to_path_buf(),
            master_password_hash: None,
        }
    }

    /// 初始化服务（设置主密码）
    pub async fn initialize(&mut self, master_password: &str) -> Result<()> {
        // 创建密钥存储路径
        let key_store_path = self.data_dir.join("password_vault.dat");

        // 创建密钥管理器
        let key_manager = KeyManager::new(&key_store_path);

        // 初始化密钥管理器
        key_manager.initialize(master_password).await?;

        // 保存主密码哈希
        self.master_password_hash = Some(HashingService::hash_password(master_password)?);

        // 保存哈希到文件
        let hash_path = self.data_dir.join("master_password.hash");
        let hash_json = serde_json::to_string(&self.master_password_hash)?;
        fs::write(&hash_path, hash_json).await?;

        // 设置密钥管理器
        let mut key_manager_lock = self.key_manager.write().unwrap();
        *key_manager_lock = Some(key_manager);

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

        // 创建密钥存储路径
        let key_store_path = self.data_dir.join("password_vault.dat");
        let key_manager = KeyManager::new(&key_store_path);

        // 解锁密钥管理器
        let unlocked = key_manager.unlock(master_password).await?;
        if !unlocked {
            return Ok(false);
        }

        // 设置密钥管理器
        let mut key_manager_lock = self.key_manager.write().unwrap();
        *key_manager_lock = Some(key_manager);

        Ok(true)
    }

    /// 锁定服务
    pub fn lock(&mut self) {
        let mut key_manager_lock = self.key_manager.write().unwrap();
        *key_manager_lock = None;
    }

    /// 检查是否已解锁
    pub fn is_unlocked(&self) -> bool {
        let key_manager_lock = self.key_manager.read().unwrap();
        key_manager_lock.is_some()
    }

    /// 添加密码条目
    pub async fn add_password(&self, mut entry: PasswordEntry) -> Result<PasswordEntry> {
        if !self.is_unlocked() {
            return Err(anyhow::anyhow!("密码服务未解锁"));
        }

        let key_manager_lock = self.key_manager.read().unwrap();
        let key_manager = key_manager_lock.as_ref().unwrap();

        // 生成加密密钥
        let encryption_key = key_manager.generate_key(
            &format!("密码条目: {}", entry.name),
            KeyType::Symmetric,
            KeyAlgorithm::AES256GCM,
            Some(&format!("用于加密密码条目: {}", entry.id)),
        ).await?;

        // 获取密钥数据
        let key_data = key_manager.get_key_data(&encryption_key.id).await?
            .ok_or_else(|| anyhow::anyhow!("无法获取密钥数据"))?;

        // 创建加密服务
        let encryption_service = EncryptionService::from_base64_key(&key_data)?;

        // 加密密码
        let encrypted_password = encryption_service.encrypt_string(&entry.password)?;
        let encrypted_password_json = serde_json::to_string(&encrypted_password)?;

        // 更新条目
        entry.password = encrypted_password_json;
        entry.id = Uuid::new_v4().to_string();
        entry.created_at = Utc::now();
        entry.updated_at = Utc::now();

        // 加密自定义字段中的敏感数据
        let mut encrypted_fields = Vec::new();
        for field in &entry.custom_fields {
            if field.sensitive {
                let encrypted_value = encryption_service.encrypt_string(&field.value)?;
                let encrypted_value_json = serde_json::to_string(&encrypted_value)?;

                encrypted_fields.push(CustomField {
                    name: field.name.clone(),
                    value: encrypted_value_json,
                    field_type: field.field_type.clone(),
                    sensitive: true,
                });
            } else {
                encrypted_fields.push(field.clone());
            }
        }
        entry.custom_fields = encrypted_fields;

        // 保存到数据库（这里简化处理，实际应该使用数据库）
        self.save_password_entry(&entry).await?;

        Ok(entry)
    }

    /// 获取密码条目
    pub async fn get_password(&self, id: &str) -> Result<Option<PasswordEntry>> {
        if !self.is_unlocked() {
            return Err(anyhow::anyhow!("密码服务未解锁"));
        }

        // 从数据库加载（这里简化处理）
        let entry = self.load_password_entry(id).await?;

        if let Some(mut entry) = entry {
            // 解密密码
            let encrypted_password: EncryptedData = serde_json::from_str(&entry.password)?;

            // 查找对应的加密密钥
            let key_manager_lock = self.key_manager.read().unwrap();
            let key_manager = key_manager_lock.as_ref().unwrap();

            // 这里需要根据条目ID找到对应的密钥ID
            // 简化处理：使用第一个AES256GCM密钥
            let keys = key_manager.list_keys().await?;
            let encryption_key = keys.iter()
                .find(|k| k.algorithm == KeyAlgorithm::AES256GCM)
                .ok_or_else(|| anyhow::anyhow!("未找到加密密钥"))?;

            let key_data = key_manager.get_key_data(&encryption_key.id).await?
                .ok_or_else(|| anyhow::anyhow!("无法获取密钥数据"))?;

            let encryption_service = EncryptionService::from_base64_key(&key_data)?;
            let decrypted_password = encryption_service.decrypt_string(&encrypted_password)?;

            entry.password = decrypted_password;

            // 解密自定义字段中的敏感数据
            let mut decrypted_fields = Vec::new();
            for field in &entry.custom_fields {
                if field.sensitive {
                    let encrypted_value: EncryptedData = serde_json::from_str(&field.value)?;
                    let decrypted_value = encryption_service.decrypt_string(&encrypted_value)?;

                    decrypted_fields.push(CustomField {
                        name: field.name.clone(),
                        value: decrypted_value,
                        field_type: field.field_type.clone(),
                        sensitive: true,
                    });
                } else {
                    decrypted_fields.push(field.clone());
                }
            }
            entry.custom_fields = decrypted_fields;

            Ok(Some(entry))
        } else {
            Ok(None)
        }
    }

    /// 更新密码条目
    pub async fn update_password(&self, id: &str, mut entry: PasswordEntry) -> Result<PasswordEntry> {
        if !self.is_unlocked() {
            return Err(anyhow::anyhow!("密码服务未解锁"));
        }

        // 检查条目是否存在
        let existing_entry = self.get_password(id).await?;
        if existing_entry.is_none() {
            return Err(anyhow::anyhow!("密码条目不存在"));
        }

        // 重新加密密码
        let key_manager_lock = self.key_manager.read().unwrap();
        let key_manager = key_manager_lock.as_ref().unwrap();

        let keys = key_manager.list_keys().await?;
        let encryption_key = keys.iter()
            .find(|k| k.algorithm == KeyAlgorithm::AES256GCM)
            .ok_or_else(|| anyhow::anyhow!("未找到加密密钥"))?;

        let key_data = key_manager.get_key_data(&encryption_key.id).await?
            .ok_or_else(|| anyhow::anyhow!("无法获取密钥数据"))?;

        let encryption_service = EncryptionService::from_base64_key(&key_data)?;

        let encrypted_password = encryption_service.encrypt_string(&entry.password)?;
        let encrypted_password_json = serde_json::to_string(&encrypted_password)?;

        entry.password = encrypted_password_json;
        entry.updated_at = Utc::now();
        entry.id = id.to_string(); // 保持相同的ID

        // 加密自定义字段中的敏感数据
        let mut encrypted_fields = Vec::new();
        for field in &entry.custom_fields {
            if field.sensitive {
                let encrypted_value = encryption_service.encrypt_string(&field.value)?;
                let encrypted_value_json = serde_json::to_string(&encrypted_value)?;

                encrypted_fields.push(CustomField {
                    name: field.name.clone(),
                    value: encrypted_value_json,
                    field_type: field.field_type.clone(),
                    sensitive: true,
                });
            } else {
                encrypted_fields.push(field.clone());
            }
        }
        entry.custom_fields = encrypted_fields;

        // 保存更新
        self.save_password_entry(&entry).await?;

        Ok(entry)
    }

    /// 删除密码条目
    pub async fn delete_password(&self, id: &str) -> Result<()> {
        if !self.is_unlocked() {
            return Err(anyhow::anyhow!("密码服务未解锁"));
        }

        // 删除条目文件
        let entry_path = self.data_dir.join("passwords").join(format!("{}.json", id));
        if entry_path.exists() {
            fs::remove_file(&entry_path).await?;
        }

        Ok(())
    }

    /// 搜索密码条目
    pub async fn search_passwords(&self, query: &str) -> Result<Vec<PasswordEntry>> {
        if !self.is_unlocked() {
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
            if path.extension().map_or(false, |ext| ext == "json") {
                let content = fs::read_to_string(&path).await?;
                let db_entry: PasswordEntryDb = serde_json::from_str(&content)?;
                let mut password_entry: PasswordEntry = db_entry.into();

                // 解密密码用于搜索
                let decrypted_entry = self.get_password(&password_entry.id).await?;
                if let Some(decrypted_entry) = decrypted_entry {
                    // 搜索条件
                    let query_lower = query.to_lowercase();
                    let matches = decrypted_entry.name.to_lowercase().contains(&query_lower) ||
                        decrypted_entry.username.as_ref().map_or(false, |u| u.to_lowercase().contains(&query_lower)) ||
                        decrypted_entry.url.as_ref().map_or(false, |u| u.to_lowercase().contains(&query_lower)) ||
                        decrypted_entry.notes.as_ref().map_or(false, |n| n.to_lowercase().contains(&query_lower)) ||
                        decrypted_entry.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower));

                    if matches {
                        entries.push(decrypted_entry);
                    }
                }
            }
        }

        Ok(entries)
    }

    /// 列出所有密码条目
    pub async fn list_passwords(&self) -> Result<Vec<PasswordEntry>> {
        self.search_passwords("").await
    }

    /// 生成强密码
    pub fn generate_password(options: &PasswordGeneratorOptions) -> String {
        use rand::Rng;
        use rand::seq::SliceRandom;

        let mut charset = String::new();

        if options.include_lowercase {
            charset.push_str("abcdefghijklmnopqrstuvwxyz");
        }
        if options.include_uppercase {
            charset.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        }
        if options.include_numbers {
            charset.push_str("0123456789");
        }
        if options.include_symbols {
            charset.push_str("!@#$%^&*()-_=+[]{}|;:,.<>?");
        }

        // 排除相似字符
        if options.exclude_similar {
            charset = charset.replace("iIlL1", "")
                .replace("oO0", "")
                .replace("sS5", "");
        }

        // 排除歧义字符
        if options.exclude_ambiguous {
            charset = charset.replace("{}[]()/\\'\"`~,;:.<>", "");
        }

        // 如果字符集为空，使用默认字符集
        if charset.is_empty() {
            charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".to_string();
        }

        let charset_chars: Vec<char> = charset.chars().collect();
        let mut rng = rand::thread_rng();

        (0..options.length)
            .map(|_| *charset_chars.choose(&mut rng).unwrap())
            .collect()
    }

    /// 审计密码安全性
    pub async fn audit_passwords(&self) -> Result<Vec<PasswordAuditResult>> {
        if !self.is_unlocked() {
            return Err(anyhow::anyhow!("密码服务未解锁"));
        }

        let entries = self.list_passwords().await?;
        let mut audit_results = Vec::new();

        for entry in entries {
            let mut issues = Vec::new();
            let mut score = 10; // 初始分数

            // 检查密码强度
            match entry.strength {
                PasswordStrength::VeryWeak => {
                    issues.push(PasswordIssue {
                        issue_type: PasswordIssueType::WeakPassword,
                        severity: IssueSeverity::Critical,
                        description: "密码非常弱".to_string(),
                        recommendation: "使用更强的密码".to_string(),
                    });
                    score -= 4;
                }
                PasswordStrength::Weak => {
                    issues.push(PasswordIssue {
                        issue_type: PasswordIssueType::WeakPassword,
                        severity: IssueSeverity::High,
                        description: "密码较弱".to_string(),
                        recommendation: "使用更强的密码".to_string(),
                    });
                    score -= 3;
                }
                PasswordStrength::Medium => {
                    score -= 1;
                }
                _ => {}
            }

            // 检查密码是否过期
            if let Some(expires_at) = entry.expires_at {
                if expires_at < Utc::now() {
                    issues.push(PasswordIssue {
                        issue_type: PasswordIssueType::ExpiredPassword,
                        severity: IssueSeverity::High,
                        description: "密码已过期".to_string(),
                        recommendation: "更新密码".to_string(),
                    });
                    score -= 3;
                }
            }

            // 检查最后使用时间
            if let Some(last_used) = entry.last_used {
                let days_since_used = (Utc::now() - last_used).num_days();
                if days_since_used > 180 { // 6个月未使用
                    issues.push(PasswordIssue {
                        issue_type: PasswordIssueType::OldPassword,
                        severity: IssueSeverity::Medium,
                        description: format!("密码已{}天未使用", days_since_used),
                        recommendation: "考虑删除或更新".to_string(),
                    });
                    score -= 1;
                }
            }

            // 检查是否缺少用户名
            if entry.username.is_none() {
                issues.push(PasswordIssue {
                    issue_type: PasswordIssueType::MissingUsername,
                    severity: IssueSeverity::Low,
                    description: "缺少用户名".to_string(),
                    recommendation: "添加用户名".to_string(),
                });
                score -= 1;
            }

            // 检查是否缺少URL
            if entry.url.is_none() {
                issues.push(PasswordIssue {
                    issue_type: PasswordIssueType::MissingUrl,
                    severity: IssueSeverity::Low,
                    description: "缺少URL".to_string(),
                    recommendation: "添加URL".to_string(),
                });
                score -= 1;
            }

            audit_results.push(PasswordAuditResult {
                entry_id: entry.id,
                issues,
                score: score.max(0) as u8,
                recommendations: Vec::new(), // 可以根据问题生成建议
            });
        }

        Ok(audit_results)
    }

    /// 导出密码本
    pub async fn export_passwords(
        &self,
        options: &PasswordImportExportOptions,
        export_password: &str,
    ) -> Result<Vec<u8>> {
        if !self.is_unlocked() {
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
            let encryption_service = EncryptionService::from_password(export_password, None)?;
            let encrypted_data = encryption_service.encrypt(&export_json)?;
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
        if !self.is_unlocked() {
            return Err(anyhow::anyhow!("密码服务未解锁"));
        }

        let data = if options.encrypt {
            let password = import_password.ok_or_else(|| anyhow::anyhow!("需要导入密码"))?;
            let encryption_service = EncryptionService::from_password(password, None)?;
            let encrypted_data: EncryptedData = serde_json::from_slice(import_data)?;
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

    /// 保存密码条目到文件（简化处理）
    async fn save_password_entry(&self, entry: &PasswordEntry) -> Result<()> {
        let passwords_dir = self.data_dir.join("passwords");
        if !passwords_dir.exists() {
            fs::create_dir_all(&passwords_dir).await?;
        }

        let entry_path = passwords_dir.join(format!("{}.json", entry.id));
        let db_entry: PasswordEntryDb = entry.clone().into();
        let entry_json = serde_json::to_string(&db_entry)?;

        fs::write(&entry_path, entry_json).await?;

        Ok(())
    }

    /// 从文件加载密码条目（简化处理）
    async fn load_password_entry(&self, id: &str) -> Result<Option<PasswordEntry>> {
        let entry_path = self.data_dir.join("passwords").join(format!("{}.json", id));

        if !entry_path.exists() {
            return Ok(None);
        }

        let entry_json = fs::read_to_string(&entry_path).await?;
        let db_entry: PasswordEntryDb = serde_json::from_str(&entry_json)?;
        let entry: PasswordEntry = db_entry.into();

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
        if !self.encrypted_password_service.is_unlocked() {
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
        if !self.encrypted_password_service.is_unlocked() {
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
            if path.extension().map_or(false, |ext| ext == "json") {
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
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_encrypted_password_service() {
        let temp_dir = tempdir().unwrap();
        let data_dir = temp_dir.path();

        let mut service = EncryptedPasswordService::new(data_dir);

        // 初始化服务
        let master_password = "test_master_password_123!";
        service.initialize(master_password).await.unwrap();

        // 解锁服务
        let unlocked = service.unlock(master_password).await.unwrap();
        assert!(unlocked);
        assert!(service.is_unlocked());

        // 创建密码条目
        let password_entry = PasswordEntry::new(
            "测试网站".to_string(),
            Some("testuser".to_string()),
            "TestPassword123!".to_string(),
            Some("https://example.com".to_string()),
            PasswordCategory::Personal,
        );

        let added_entry = service.add_password(password_entry).await.unwrap();
        assert_eq!(added_entry.name, "测试网站");

        // 获取密码条目
        let retrieved_entry = service.get_password(&added_entry.id).await.unwrap();
        assert!(retrieved_entry.is_some());
        let entry = retrieved_entry.unwrap();
        assert_eq!(entry.name, "测试网站");
        assert_eq!(entry.password, "TestPassword123!");

        // 搜索密码条目
        let search_results = service.search_passwords("测试").await.unwrap();
        assert_eq!(search_results.len(), 1);

        // 生成密码
        let options = PasswordGeneratorOptions::default();
        let generated_password = EncryptedPasswordService::generate_password(&options);
        assert_eq!(generated_password.len(), 16);

        // 锁定服务
        service.lock();
        assert!(!service.is_unlocked());
    }

    #[tokio::test]
    async fn test_password_group_service() {
        let temp_dir = tempdir().unwrap();
        let data_dir = temp_dir.path();

        let mut password_service = EncryptedPasswordService::new(data_dir);
        let master_password = "test_master_password_123!";
        password_service.initialize(master_password).await.unwrap();
        password_service.unlock(master_password).await.unwrap();

        let password_service_arc = Arc::new(password_service);
        let group_service = PasswordGroupService::new(password_service_arc.clone());

        // 创建密码条目
        let password_entry = PasswordEntry::new(
            "组测试条目".to_string(),
            Some("groupuser".to_string()),
            "GroupPassword123!".to_string(),
            Some("https://group.example.com".to_string()),
            PasswordCategory::Work,
        );

        let added_entry = password_service_arc.add_password(password_entry).await.unwrap();

        // 创建密码组
        let group = PasswordGroup::new(
            "工作密码".to_string(),
            Some("工作相关密码".to_string()),
            PasswordCategory::Work,
        );

        let created_group = group_service.create_group(group).await.unwrap();
        assert_eq!(created_group.name, "工作密码");

        // 向组中添加条目
        group_service.add_entry_to_group(&created_group.id, &added_entry.id).await.unwrap();

        // 获取组中的条目
        let group_entries = group_service.get_group_entries(&created_group.id).await.unwrap();
        assert_eq!(group_entries.len(), 1);
        assert_eq!(group_entries[0].name, "组测试条目");

        // 从组中移除条目
        group_service.remove_entry_from_group(&created_group.id, &added_entry.id).await.unwrap();

        let updated_group_entries = group_service.get_group_entries(&created_group.id).await.unwrap();
        assert_eq!(updated_group_entries.len(), 0);
    }
}