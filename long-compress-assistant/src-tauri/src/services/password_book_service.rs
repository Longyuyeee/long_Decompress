use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use uuid::Uuid;

use crate::crypto::encryption::{EncryptionService, EncryptedData};
use crate::database::connection::get_connection;
use crate::models::password::{
    PasswordCategory, PasswordStrength, CustomField
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordBookEntry {
    pub id: String,
    pub name: String,
    pub username: Option<String>,
    pub password: String, // 加密存储的密码
    pub url: Option<String>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub category: PasswordCategory,
    pub strength: PasswordStrength,
    pub key_id: String, // 加密密钥ID
    pub encryption_algorithm: String,
    pub encryption_version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub favorite: bool,
    pub archived: bool,
    pub deleted: bool,
    pub custom_fields: Vec<CustomField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPasswordRequest {
    pub name: String,
    pub username: Option<String>,
    pub password: String, // 明文密码
    pub url: Option<String>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub category: PasswordCategory,
    pub expires_at: Option<DateTime<Utc>>,
    pub custom_fields: Vec<CustomField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePasswordRequest {
    pub id: String,
    pub name: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>, // 明文密码
    pub url: Option<String>,
    pub notes: Option<String>,
    pub tags: Option<Vec<String>>,
    pub category: Option<PasswordCategory>,
    pub expires_at: Option<DateTime<Utc>>,
    pub favorite: Option<bool>,
    pub custom_fields: Option<Vec<CustomField>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordValidationResult {
    pub is_valid: bool,
    pub issues: Vec<String>,
    pub score: u8,
    pub strength: PasswordStrength,
}

pub struct PasswordBookService {
    master_key: Option<EncryptionService>,
    session_key: Option<EncryptionService>,
}

impl PasswordBookService {
    pub fn new() -> Self {
        Self {
            master_key: None,
            session_key: None,
        }
    }

    /// 设置主密钥（从密码派生）
    pub async fn set_master_key(&mut self, password: &str) -> Result<()> {
        let key_service = EncryptionService::from_password(password, None)
            .context("创建主密钥失败")?;

        self.master_key = Some(key_service);
        Ok(())
    }

    /// 验证主密码
    pub async fn verify_master_password(&self, _password: &str) -> Result<bool> {
        // 这里应该实现密码验证逻辑
        // 暂时返回true用于测试
        Ok(true)
    }

    /// 创建会话密钥
    pub async fn create_session_key(&mut self) -> Result<()> {
        let key_service = EncryptionService::new_random();
        self.session_key = Some(key_service);
        Ok(())
    }

    /// 添加密码条目
    pub async fn add_password(&self, request: AddPasswordRequest) -> Result<PasswordBookEntry> {
        let db = get_connection().await?;
        let pool = db.pool();

        // 验证密码强度
        let validation = self.validate_password(&request.password).await?;
        if !validation.is_valid {
            return Err(anyhow::anyhow!("密码不符合要求: {:?}", validation.issues));
        }

        // 获取加密服务
        let encryption_service = self.get_encryption_service()
            .context("加密服务未初始化")?;

        // 加密密码
        let encrypted_password = encryption_service.encrypt_string(&request.password)
            .context("加密密码失败")?;

        // 创建密码条目
        let now = Utc::now();
        let entry = PasswordBookEntry {
            id: Uuid::new_v4().to_string(),
            name: request.name,
            username: request.username,
            password: serde_json::to_string(&encrypted_password)
                .context("序列化加密密码失败")?,
            url: request.url,
            notes: request.notes,
            tags: request.tags,
            category: request.category,
            strength: validation.strength,
            key_id: "master_key".to_string(), // 实际应该使用密钥ID
            encryption_algorithm: "AES256GCM".to_string(),
            encryption_version: 1,
            created_at: now,
            updated_at: now,
            last_used: None,
            expires_at: request.expires_at,
            favorite: false,
            archived: false,
            deleted: false,
            custom_fields: request.custom_fields,
        };

        // 保存到数据库
        self.save_password_entry(pool, &entry).await?;

        Ok(entry)
    }

    /// 更新密码条目
    pub async fn update_password(&self, request: UpdatePasswordRequest) -> Result<PasswordBookEntry> {
        let db = get_connection().await?;
        let pool = db.pool();

        // 获取现有条目
        let mut entry = self.get_password_entry(pool, &request.id).await?
            .ok_or_else(|| anyhow::anyhow!("密码条目不存在: {}", request.id))?;

        // 更新字段
        if let Some(name) = request.name {
            entry.name = name;
        }
        if let Some(username) = request.username {
            entry.username = Some(username);
        }
        if let Some(password) = request.password {
            // 验证新密码强度
            let validation = self.validate_password(&password).await?;
            if !validation.is_valid {
                return Err(anyhow::anyhow!("新密码不符合要求: {:?}", validation.issues));
            }

            // 获取加密服务
            let encryption_service = self.get_encryption_service()
                .context("加密服务未初始化")?;

            // 加密新密码
            let encrypted_password = encryption_service.encrypt_string(&password)
                .context("加密密码失败")?;

            entry.password = serde_json::to_string(&encrypted_password)
                .context("序列化加密密码失败")?;
            entry.strength = validation.strength;
        }
        if let Some(url) = request.url {
            entry.url = Some(url);
        }
        if let Some(notes) = request.notes {
            entry.notes = Some(notes);
        }
        if let Some(tags) = request.tags {
            entry.tags = tags;
        }
        if let Some(category) = request.category {
            entry.category = category;
        }
        if let Some(expires_at) = request.expires_at {
            entry.expires_at = Some(expires_at);
        }
        if let Some(favorite) = request.favorite {
            entry.favorite = favorite;
        }
        if let Some(custom_fields) = request.custom_fields {
            entry.custom_fields = custom_fields;
        }

        entry.updated_at = Utc::now();

        // 更新数据库
        self.update_password_entry(pool, &entry).await?;

        Ok(entry)
    }

    /// 获取密码条目（包含解密密码）
    pub async fn get_password(&self, id: &str, include_password: bool) -> Result<Option<PasswordBookEntry>> {
        let db = get_connection().await?;
        let pool = db.pool();

        let mut entry = self.get_password_entry(pool, id).await?;

        if let Some(entry) = entry.as_mut() {
            // 更新最后使用时间
            entry.last_used = Some(Utc::now());
            self.update_password_entry(pool, entry).await?;

            if include_password {
                // 解密密码
                let encrypted_password: EncryptedData = serde_json::from_str(&entry.password)
                    .context("解析加密密码失败")?;

                let encryption_service = self.get_encryption_service()
                    .context("加密服务未初始化")?;

                let decrypted_password = encryption_service.decrypt_string(&encrypted_password)
                    .context("解密密码失败")?;

                // 注意：这里返回的条目包含解密后的密码
                // 在实际应用中，应该小心处理解密后的密码
                entry.password = decrypted_password;
            }
        }

        Ok(entry)
    }

    /// 删除密码条目（软删除）
    pub async fn delete_password(&self, id: &str) -> Result<()> {
        let db = get_connection().await?;
        let pool = db.pool();

        let mut entry = self.get_password_entry(pool, id).await?
            .ok_or_else(|| anyhow::anyhow!("密码条目不存在: {}", id))?;

        entry.deleted = true;
        entry.updated_at = Utc::now();

        self.update_password_entry(pool, &entry).await?;

        Ok(())
    }

    /// 归档密码条目
    pub async fn archive_password(&self, id: &str) -> Result<()> {
        let db = get_connection().await?;
        let pool = db.pool();

        let mut entry = self.get_password_entry(pool, id).await?
            .ok_or_else(|| anyhow::anyhow!("密码条目不存在: {}", id))?;

        entry.archived = true;
        entry.updated_at = Utc::now();

        self.update_password_entry(pool, &entry).await?;

        Ok(())
    }

    /// 验证密码强度
    pub async fn validate_password(&self, password: &str) -> Result<PasswordValidationResult> {
        let mut issues = Vec::new();
        let mut score: i32 = 0;

        // 长度检查
        if password.len() < 8 {
            issues.push("密码长度至少8个字符".to_string());
        } else {
            score += 1;
        }

        if password.len() >= 12 {
            score += 1;
        }
        if password.len() >= 16 {
            score += 1;
        }

        // 字符类型检查
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_digit = password.chars().any(|c| c.is_digit(10));
        let has_symbol = password.chars().any(|c| !c.is_alphanumeric());

        if !has_lowercase {
            issues.push("密码应包含小写字母".to_string());
        } else {
            score += 1;
        }

        if !has_uppercase {
            issues.push("密码应包含大写字母".to_string());
        } else {
            score += 1;
        }

        if !has_digit {
            issues.push("密码应包含数字".to_string());
        } else {
            score += 1;
        }

        if !has_symbol {
            issues.push("密码应包含特殊字符".to_string());
        } else {
            score += 1;
        }

        // 检查常见密码
        let common_passwords = ["password", "123456", "qwerty", "admin", "welcome", "letmein"];
        if common_passwords.iter().any(|&p| password.to_lowercase().contains(p)) {
            issues.push("密码过于常见".to_string());
            score = score.saturating_sub(2);
        }

        // 检查序列
        if Self::contains_sequence(password) {
            issues.push("密码包含连续字符序列".to_string());
            score = score.saturating_sub(1);
        }

        // 检查重复字符
        if Self::contains_repetition(password) {
            issues.push("密码包含重复字符".to_string());
            score = score.saturating_sub(1);
        }

        let strength = match score {
            0..=3 => PasswordStrength::VeryWeak,
            4..=5 => PasswordStrength::Weak,
            6..=7 => PasswordStrength::Medium,
            8..=9 => PasswordStrength::Strong,
            _ => PasswordStrength::VeryStrong,
        };

        let is_valid = issues.is_empty() || score >= 6; // 允许中等强度以上的密码

        Ok(PasswordValidationResult {
            is_valid,
            issues,
            score: score.min(10) as u8,
            strength,
        })
    }

    /// 检查是否包含序列
    pub(crate) fn contains_sequence(password: &str) -> bool {
        if password.len() < 3 {
            return false;
        }

        let chars: Vec<char> = password.chars().collect();
        for i in 0..chars.len() - 2 {
            let c1 = chars[i] as u32;
            let c2 = chars[i + 1] as u32;
            let c3 = chars[i + 2] as u32;

            // 检查连续递增或递减
            if (c2 == c1 + 1 && c3 == c2 + 1) || (c2 == c1 - 1 && c3 == c2 - 1) {
                return true;
            }
        }

        false
    }

    /// 检查是否包含重复字符
    pub(crate) fn contains_repetition(password: &str) -> bool {
        if password.len() < 3 {
            return false;
        }

        let chars: Vec<char> = password.chars().collect();
        for i in 0..chars.len() - 2 {
            if chars[i] == chars[i + 1] && chars[i] == chars[i + 2] {
                return true;
            }
        }

        false
    }

    /// 获取加密服务
    fn get_encryption_service(&self) -> Option<&EncryptionService> {
        self.session_key.as_ref().or(self.master_key.as_ref())
    }

    /// 保存密码条目到数据库
    async fn save_password_entry(&self, pool: &SqlitePool, entry: &PasswordBookEntry) -> Result<()> {
        // 序列化JSON字段
        let tags_json = serde_json::to_string(&entry.tags)
            .context("序列化标签失败")?;
        let custom_fields_json = serde_json::to_string(&entry.custom_fields)
            .context("序列化自定义字段失败")?;

        // 转换分类和强度为字符串
        let category_str = match entry.category {
            PasswordCategory::Personal => "Personal",
            PasswordCategory::Work => "Work",
            PasswordCategory::Finance => "Finance",
            PasswordCategory::Social => "Social",
            PasswordCategory::Shopping => "Shopping",
            PasswordCategory::Entertainment => "Entertainment",
            PasswordCategory::Education => "Education",
            PasswordCategory::Travel => "Travel",
            PasswordCategory::Health => "Health",
            PasswordCategory::Other => "Other",
        };

        let strength_str = match entry.strength {
            PasswordStrength::VeryWeak => "VeryWeak",
            PasswordStrength::Weak => "Weak",
            PasswordStrength::Medium => "Medium",
            PasswordStrength::Strong => "Strong",
            PasswordStrength::VeryStrong => "VeryStrong",
        };

        sqlx::query(
            r#"
            INSERT INTO password_entries (
                id, name, username, password, url, notes, tags, category, strength,
                key_id, encryption_algorithm, encryption_version,
                created_at, updated_at, last_used, expires_at, favorite, archived, deleted, custom_fields
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&entry.id)
        .bind(&entry.name)
        .bind(&entry.username)
        .bind(&entry.password)
        .bind(&entry.url)
        .bind(&entry.notes)
        .bind(&tags_json)
        .bind(category_str)
        .bind(strength_str)
        .bind(&entry.key_id)
        .bind(&entry.encryption_algorithm)
        .bind(entry.encryption_version)
        .bind(entry.created_at)
        .bind(entry.updated_at)
        .bind(entry.last_used)
        .bind(entry.expires_at)
        .bind(entry.favorite)
        .bind(entry.archived)
        .bind(entry.deleted)
        .bind(&custom_fields_json)
        .execute(pool)
        .await
        .context("保存密码条目失败")?;

        Ok(())
    }

    /// 从数据库获取密码条目
    async fn get_password_entry(&self, pool: &SqlitePool, id: &str) -> Result<Option<PasswordBookEntry>> {
        use crate::database::models::PasswordEntryDb;

        let db_entry: Option<PasswordEntryDb> = sqlx::query_as(
            r#"
            SELECT * FROM password_entries
            WHERE id = ? AND deleted = FALSE
            "#
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("查询密码条目失败")?;

        if let Some(db_entry) = db_entry {
            // 这里需要将 PasswordEntryDb 转换为 PasswordBookEntry
            // 由于 PasswordBookEntry 与 PasswordEntry 基本相同，我将重用转换逻辑
            let entry: crate::models::password::PasswordEntry = db_entry.into();
            Ok(Some(self.password_entry_to_book_entry(entry)))
        } else {
            Ok(None)
        }
    }

    fn password_entry_to_book_entry(&self, entry: crate::models::password::PasswordEntry) -> PasswordBookEntry {
        PasswordBookEntry {
            id: entry.id,
            name: entry.name,
            username: entry.username,
            password: entry.password,
            url: entry.url,
            notes: entry.notes,
            tags: entry.tags,
            category: entry.category,
            strength: entry.strength,
            key_id: "master_key".to_string(), // 简化
            encryption_algorithm: "AES256GCM".to_string(),
            encryption_version: 1,
            created_at: entry.created_at,
            updated_at: entry.updated_at,
            last_used: entry.last_used,
            expires_at: entry.expires_at,
            favorite: entry.favorite,
            archived: false,
            deleted: false,
            custom_fields: entry.custom_fields,
        }
    }

    /// 更新数据库中的密码条目
    async fn update_password_entry(&self, pool: &SqlitePool, entry: &PasswordBookEntry) -> Result<()> {
        // 序列化JSON字段
        let tags_json = serde_json::to_string(&entry.tags)
            .context("序列化标签失败")?;
        let custom_fields_json = serde_json::to_string(&entry.custom_fields)
            .context("序列化自定义字段失败")?;

        // 转换分类和强度为字符串
        let category_str = match entry.category {
            PasswordCategory::Personal => "Personal",
            PasswordCategory::Work => "Work",
            PasswordCategory::Finance => "Finance",
            PasswordCategory::Social => "Social",
            PasswordCategory::Shopping => "Shopping",
            PasswordCategory::Entertainment => "Entertainment",
            PasswordCategory::Education => "Education",
            PasswordCategory::Travel => "Travel",
            PasswordCategory::Health => "Health",
            PasswordCategory::Other => "Other",
        };

        let strength_str = match entry.strength {
            PasswordStrength::VeryWeak => "VeryWeak",
            PasswordStrength::Weak => "Weak",
            PasswordStrength::Medium => "Medium",
            PasswordStrength::Strong => "Strong",
            PasswordStrength::VeryStrong => "VeryStrong",
        };

        sqlx::query(
            r#"
            UPDATE password_entries SET
                name = ?, username = ?, password = ?, url = ?, notes = ?, tags = ?, category = ?, strength = ?,
                key_id = ?, encryption_algorithm = ?, encryption_version = ?,
                updated_at = ?, last_used = ?, expires_at = ?, favorite = ?, archived = ?, deleted = ?, custom_fields = ?
            WHERE id = ?
            "#
        )
        .bind(&entry.name)
        .bind(&entry.username)
        .bind(&entry.password)
        .bind(&entry.url)
        .bind(&entry.notes)
        .bind(&tags_json)
        .bind(category_str)
        .bind(strength_str)
        .bind(&entry.key_id)
        .bind(&entry.encryption_algorithm)
        .bind(entry.encryption_version)
        .bind(entry.updated_at)
        .bind(entry.last_used)
        .bind(entry.expires_at)
        .bind(entry.favorite)
        .bind(entry.archived)
        .bind(entry.deleted)
        .bind(&custom_fields_json)
        .bind(&entry.id)
        .execute(pool)
        .await
        .context("更新密码条目失败")?;

        Ok(())
    }

    /// 将数据库行转换为密码条目
    fn row_to_password_entry(&self, row: (String, String, Option<String>, String, Option<String>, Option<String>, String, String, String, String, String, i32, DateTime<Utc>, DateTime<Utc>, Option<DateTime<Utc>>, Option<DateTime<Utc>>, bool, bool, bool, String)) -> Result<PasswordBookEntry> {
        let (
            id, name, username, password, url, notes, tags_json, category_str, strength_str,
            key_id, encryption_algorithm, encryption_version,
            created_at, updated_at, last_used, expires_at, favorite, archived, deleted, custom_fields_json
        ) = row;

        // 反序列化JSON字段
        let tags: Vec<String> = serde_json::from_str(&tags_json)
            .unwrap_or_else(|_| Vec::new());
        let custom_fields: Vec<CustomField> = serde_json::from_str(&custom_fields_json)
            .unwrap_or_else(|_| Vec::new());

        // 转换分类
        let category = match category_str.as_str() {
            "Personal" => PasswordCategory::Personal,
            "Work" => PasswordCategory::Work,
            "Finance" => PasswordCategory::Finance,
            "Social" => PasswordCategory::Social,
            "Shopping" => PasswordCategory::Shopping,
            "Entertainment" => PasswordCategory::Entertainment,
            "Education" => PasswordCategory::Education,
            "Travel" => PasswordCategory::Travel,
            "Health" => PasswordCategory::Health,
            "Other" => PasswordCategory::Other,
            _ => PasswordCategory::Other,
        };

        // 转换强度
        let strength = match strength_str.as_str() {
            "VeryWeak" => PasswordStrength::VeryWeak,
            "Weak" => PasswordStrength::Weak,
            "Medium" => PasswordStrength::Medium,
            "Strong" => PasswordStrength::Strong,
            "VeryStrong" => PasswordStrength::VeryStrong,
            _ => PasswordStrength::Weak,
        };

        Ok(PasswordBookEntry {
            id,
            name,
            username,
            password,
            url,
            notes,
            tags,
            category,
            strength,
            key_id,
            encryption_algorithm,
            encryption_version,
            created_at,
            updated_at,
            last_used,
            expires_at,
            favorite,
            archived,
            deleted,
            custom_fields,
        })
    }

    /// 搜索密码条目
    pub async fn search_passwords(&self, query: &str, filters: Option<SearchFilters>) -> Result<Vec<PasswordBookEntry>> {
        let db = get_connection().await?;
        let pool = db.pool();

        let filters = filters.unwrap_or_default();
        let query_lower = query.to_lowercase();

        // 构建SQL查询
        let mut sql = String::from(
            r#"
            SELECT
                id, name, username, password, url, notes, tags, category, strength,
                key_id, encryption_algorithm, encryption_version,
                created_at, updated_at, last_used, expires_at, favorite, archived, deleted, custom_fields
            FROM password_entries
            WHERE deleted = FALSE
            "#
        );

        let mut conditions = Vec::new();
        let mut params: Vec<String> = Vec::new();

        // 添加搜索条件
        if !query_lower.is_empty() {
            conditions.push("(
                LOWER(name) LIKE ? OR
                LOWER(username) LIKE ? OR
                LOWER(url) LIKE ? OR
                LOWER(notes) LIKE ? OR
                tags LIKE ?
            )".to_string());

            let search_pattern = format!("%{}%", query_lower);
            for _ in 0..4 {
                params.push(search_pattern.clone());
            }
            params.push(format!("%\"{}\"%", query_lower)); // 在JSON数组中搜索标签
        }

        // 添加分类过滤
        if !filters.categories.is_empty() {
            let category_placeholders: Vec<String> = filters.categories.iter()
                .map(|_| "?".to_string())
                .collect();
            conditions.push(format!("category IN ({})", category_placeholders.join(", ")));

            for category in &filters.categories {
                let category_str = match category {
                    PasswordCategory::Personal => "Personal",
                    PasswordCategory::Work => "Work",
                    PasswordCategory::Finance => "Finance",
                    PasswordCategory::Social => "Social",
                    PasswordCategory::Shopping => "Shopping",
                    PasswordCategory::Entertainment => "Entertainment",
                    PasswordCategory::Education => "Education",
                    PasswordCategory::Travel => "Travel",
                    PasswordCategory::Health => "Health",
                    PasswordCategory::Other => "Other",
                };
                params.push(category_str.to_string());
            }
        }

        // 添加强度过滤
        if !filters.strengths.is_empty() {
            let strength_placeholders: Vec<String> = filters.strengths.iter()
                .map(|_| "?".to_string())
                .collect();
            conditions.push(format!("strength IN ({})", strength_placeholders.join(", ")));

            for strength in &filters.strengths {
                let strength_str = match strength {
                    PasswordStrength::VeryWeak => "VeryWeak",
                    PasswordStrength::Weak => "Weak",
                    PasswordStrength::Medium => "Medium",
                    PasswordStrength::Strong => "Strong",
                    PasswordStrength::VeryStrong => "VeryStrong",
                };
                params.push(strength_str.to_string());
            }
        }

        // 添加收藏过滤
        if let Some(favorite) = filters.favorite {
            conditions.push("favorite = ?".to_string());
            params.push(favorite.to_string());
        }

        // 添加归档过滤
        if let Some(archived) = filters.archived {
            conditions.push("archived = ?".to_string());
            params.push(archived.to_string());
        }

        // 添加过期过滤
        if let Some(expired) = filters.expired {
            let now = Utc::now();
            if expired {
                conditions.push("expires_at IS NOT NULL AND expires_at < ?".to_string());
                params.push(now.to_rfc3339());
            } else {
                conditions.push("(expires_at IS NULL OR expires_at >= ?)".to_string());
                params.push(now.to_rfc3339());
            }
        }

        // 添加标签过滤
        if !filters.tags.is_empty() {
            for tag in &filters.tags {
                conditions.push("tags LIKE ?".to_string());
                params.push(format!("%\"{}\"%", tag));
            }
        }

        // 添加日期范围过滤
        if let Some(start_date) = filters.created_after {
            conditions.push("created_at >= ?".to_string());
            params.push(start_date.to_rfc3339());
        }
        if let Some(end_date) = filters.created_before {
            conditions.push("created_at <= ?".to_string());
            params.push(end_date.to_rfc3339());
        }

        // 组合所有条件
        if !conditions.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&conditions.join(" AND "));
        }

        // 添加排序
        sql.push_str(&format!(" ORDER BY {}", filters.sort_by.to_sql()));

        // 添加分页
        if let Some(limit) = filters.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
            if let Some(offset) = filters.offset {
                sql.push_str(&format!(" OFFSET {}", offset));
            }
        }

        // 执行查询
        let mut query_builder = sqlx::query(&sql);

        for param in params {
            query_builder = query_builder.bind(param);
        }

        let rows = query_builder
            .fetch_all(pool)
            .await
            .context("执行搜索查询失败")?;

        // 转换结果
        let mut results = Vec::new();
        for row in rows {
            let entry = self.row_to_password_entry_from_sqlx(row)?;
            results.push(entry);
        }

        Ok(results)
    }

    /// 获取所有密码条目（用于列表显示）
    pub async fn get_all_passwords(&self, include_password: bool) -> Result<Vec<PasswordBookEntry>> {
        let db = get_connection().await?;
        let pool = db.pool();

        let rows = sqlx::query(
            r#"
            SELECT
                id, name, username, password, url, notes, tags, category, strength,
                key_id, encryption_algorithm, encryption_version,
                created_at, updated_at, last_used, expires_at, favorite, archived, deleted, custom_fields
            FROM password_entries
            WHERE deleted = FALSE
            ORDER BY updated_at DESC
            "#
        )
        .fetch_all(pool)
        .await
        .context("获取所有密码条目失败")?;

        let mut results = Vec::new();
        for row in rows {
            let mut entry = self.row_to_password_entry_from_sqlx(row)?;

            if include_password {
                // 解密密码
                let encrypted_password: EncryptedData = serde_json::from_str(&entry.password)
                    .context("解析加密密码失败")?;

                let encryption_service = self.get_encryption_service()
                    .context("加密服务未初始化")?;

                let decrypted_password = encryption_service.decrypt_string(&encrypted_password)
                    .context("解密密码失败")?;

                entry.password = decrypted_password;
            }

            results.push(entry);
        }

        Ok(results)
    }

    /// 获取收藏的密码条目
    pub async fn get_favorite_passwords(&self) -> Result<Vec<PasswordBookEntry>> {
        let db = get_connection().await?;
        let pool = db.pool();

        let rows = sqlx::query(
            r#"
            SELECT
                id, name, username, password, url, notes, tags, category, strength,
                key_id, encryption_algorithm, encryption_version,
                created_at, updated_at, last_used, expires_at, favorite, archived, deleted, custom_fields
            FROM password_entries
            WHERE deleted = FALSE AND favorite = TRUE
            ORDER BY updated_at DESC
            "#
        )
        .fetch_all(pool)
        .await
        .context("获取收藏密码条目失败")?;

        let mut results = Vec::new();
        for row in rows {
            let entry = self.row_to_password_entry_from_sqlx(row)?;
            results.push(entry);
        }

        Ok(results)
    }

    /// 获取最近使用的密码条目
    pub async fn get_recently_used_passwords(&self, limit: i32) -> Result<Vec<PasswordBookEntry>> {
        let db = get_connection().await?;
        let pool = db.pool();

        let rows = sqlx::query(
            r#"
            SELECT
                id, name, username, password, url, notes, tags, category, strength,
                key_id, encryption_algorithm, encryption_version,
                created_at, updated_at, last_used, expires_at, favorite, archived, deleted, custom_fields
            FROM password_entries
            WHERE deleted = FALSE AND last_used IS NOT NULL
            ORDER BY last_used DESC
            LIMIT ?
            "#
        )
        .bind(limit)
        .fetch_all(pool)
        .await
        .context("获取最近使用的密码条目失败")?;

        let mut results = Vec::new();
        for row in rows {
            let entry = self.row_to_password_entry_from_sqlx(row)?;
            results.push(entry);
        }

        Ok(results)
    }

    /// 获取即将过期的密码条目
    pub async fn get_expiring_passwords(&self, days_before: i32) -> Result<Vec<PasswordBookEntry>> {
        let db = get_connection().await?;
        let pool = db.pool();

        let now = Utc::now();
        let warning_date = now + chrono::Duration::days(days_before as i64);

        let rows = sqlx::query(
            r#"
            SELECT
                id, name, username, password, url, notes, tags, category, strength,
                key_id, encryption_algorithm, encryption_version,
                created_at, updated_at, last_used, expires_at, favorite, archived, deleted, custom_fields
            FROM password_entries
            WHERE deleted = FALSE AND expires_at IS NOT NULL
                AND expires_at >= ? AND expires_at <= ?
            ORDER BY expires_at ASC
            "#
        )
        .bind(now.to_rfc3339())
        .bind(warning_date.to_rfc3339())
        .fetch_all(pool)
        .await
        .context("获取即将过期的密码条目失败")?;

        let mut results = Vec::new();
        for row in rows {
            let entry = self.row_to_password_entry_from_sqlx(row)?;
            results.push(entry);
        }

        Ok(results)
    }

    /// 获取密码统计信息
    pub async fn get_password_statistics(&self) -> Result<PasswordStatistics> {
        let db = get_connection().await?;
        let pool = db.pool();

        // 总条目数
        let total_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM password_entries WHERE deleted = FALSE"
        )
        .fetch_one(pool)
        .await
        .context("获取总条目数失败")?;

        // 收藏条目数
        let favorite_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM password_entries WHERE deleted = FALSE AND favorite = TRUE"
        )
        .fetch_one(pool)
        .await
        .context("获取收藏条目数失败")?;

        // 归档条目数
        let archived_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM password_entries WHERE deleted = FALSE AND archived = TRUE"
        )
        .fetch_one(pool)
        .await
        .context("获取归档条目数失败")?;

        // 过期条目数
        let now = Utc::now();
        let expired_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM password_entries WHERE deleted = FALSE AND expires_at IS NOT NULL AND expires_at < ?"
        )
        .bind(now.to_rfc3339())
        .fetch_one(pool)
        .await
        .context("获取过期条目数失败")?;

        // 按分类统计
        let category_stats: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT category, COUNT(*) as count
            FROM password_entries
            WHERE deleted = FALSE
            GROUP BY category
            ORDER BY count DESC
            "#
        )
        .fetch_all(pool)
        .await
        .context("获取分类统计失败")?;

        // 按强度统计
        let strength_stats: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT strength, COUNT(*) as count
            FROM password_entries
            WHERE deleted = FALSE
            GROUP BY strength
            ORDER BY count DESC
            "#
        )
        .fetch_all(pool)
        .await
        .context("获取强度统计失败")?;

        Ok(PasswordStatistics {
            total_count: total_count.0 as u64,
            favorite_count: favorite_count.0 as u64,
            archived_count: archived_count.0 as u64,
            expired_count: expired_count.0 as u64,
            category_stats: category_stats.into_iter()
                .map(|(category, count)| (category, count as u64))
                .collect(),
            strength_stats: strength_stats.into_iter()
                .map(|(strength, count)| (strength, count as u64))
                .collect(),
        })
    }

    /// 从sqlx::Row转换为密码条目
    fn row_to_password_entry_from_sqlx(&self, row: sqlx::sqlite::SqliteRow) -> Result<PasswordBookEntry> {
        let id: String = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let username: Option<String> = row.try_get("username")?;
        let password: String = row.try_get("password")?;
        let url: Option<String> = row.try_get("url")?;
        let notes: Option<String> = row.try_get("notes")?;
        let tags_json: String = row.try_get("tags")?;
        let category_str: String = row.try_get("category")?;
        let strength_str: String = row.try_get("strength")?;
        let key_id: String = row.try_get("key_id")?;
        let encryption_algorithm: String = row.try_get("encryption_algorithm")?;
        let encryption_version: i32 = row.try_get("encryption_version")?;
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let updated_at: DateTime<Utc> = row.try_get("updated_at")?;
        let last_used: Option<DateTime<Utc>> = row.try_get("last_used")?;
        let expires_at: Option<DateTime<Utc>> = row.try_get("expires_at")?;
        let favorite: bool = row.try_get("favorite")?;
        let archived: bool = row.try_get("archived")?;
        let deleted: bool = row.try_get("deleted")?;
        let custom_fields_json: String = row.try_get("custom_fields")?;

        // 反序列化JSON字段
        let tags: Vec<String> = serde_json::from_str(&tags_json)
            .unwrap_or_else(|_| Vec::new());
        let custom_fields: Vec<CustomField> = serde_json::from_str(&custom_fields_json)
            .unwrap_or_else(|_| Vec::new());

        // 转换分类
        let category = match category_str.as_str() {
            "Personal" => PasswordCategory::Personal,
            "Work" => PasswordCategory::Work,
            "Finance" => PasswordCategory::Finance,
            "Social" => PasswordCategory::Social,
            "Shopping" => PasswordCategory::Shopping,
            "Entertainment" => PasswordCategory::Entertainment,
            "Education" => PasswordCategory::Education,
            "Travel" => PasswordCategory::Travel,
            "Health" => PasswordCategory::Health,
            "Other" => PasswordCategory::Other,
            _ => PasswordCategory::Other,
        };

        // 转换强度
        let strength = match strength_str.as_str() {
            "VeryWeak" => PasswordStrength::VeryWeak,
            "Weak" => PasswordStrength::Weak,
            "Medium" => PasswordStrength::Medium,
            "Strong" => PasswordStrength::Strong,
            "VeryStrong" => PasswordStrength::VeryStrong,
            _ => PasswordStrength::Weak,
        };

        Ok(PasswordBookEntry {
            id,
            name,
            username,
            password,
            url,
            notes,
            tags,
            category,
            strength,
            key_id,
            encryption_algorithm,
            encryption_version,
            created_at,
            updated_at,
            last_used,
            expires_at,
            favorite,
            archived,
            deleted,
            custom_fields,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchFilters {
    pub categories: Vec<PasswordCategory>,
    pub strengths: Vec<PasswordStrength>,
    pub tags: Vec<String>,
    pub favorite: Option<bool>,
    pub archived: Option<bool>,
    pub expired: Option<bool>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub sort_by: SortOption,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortOption {
    NameAsc,
    NameDesc,
    UpdatedAtAsc,
    UpdatedAtDesc,
    CreatedAtAsc,
    CreatedAtDesc,
    LastUsedAsc,
    LastUsedDesc,
    ExpiresAtAsc,
    ExpiresAtDesc,
}

impl Default for SortOption {
    fn default() -> Self {
        Self::UpdatedAtDesc
    }
}

impl SortOption {
    fn to_sql(&self) -> String {
        match self {
            Self::NameAsc => "name ASC",
            Self::NameDesc => "name DESC",
            Self::UpdatedAtAsc => "updated_at ASC",
            Self::UpdatedAtDesc => "updated_at DESC",
            Self::CreatedAtAsc => "created_at ASC",
            Self::CreatedAtDesc => "created_at DESC",
            Self::LastUsedAsc => "last_used ASC",
            Self::LastUsedDesc => "last_used DESC",
            Self::ExpiresAtAsc => "expires_at ASC",
            Self::ExpiresAtDesc => "expires_at DESC",
        }.to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordStatistics {
    pub total_count: u64,
    pub favorite_count: u64,
    pub archived_count: u64,
    pub expired_count: u64,
    pub category_stats: Vec<(String, u64)>,
    pub strength_stats: Vec<(String, u64)>,
}

impl Default for PasswordBookService {
    fn default() -> Self {
        Self::new()
    }
}