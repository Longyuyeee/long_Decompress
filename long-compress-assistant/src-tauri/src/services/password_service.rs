use anyhow::{Context, Result};
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{PasswordHash, SaltString};
use argon2::Algorithm;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordEntry {
    pub id: String,
    pub name: String,
    pub username: Option<String>,
    pub password: String,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub struct PasswordService {
    // 内存中的密码存储（实际应用中应该使用数据库）
    passwords: HashMap<String, PasswordEntry>,
    master_password_hash: Option<String>,
}

impl PasswordService {
    pub fn new() -> Self {
        Self {
            passwords: HashMap::new(),
            master_password_hash: None,
        }
    }

    /// 设置主密码
    pub fn set_master_password(&mut self, password: &str) -> Result<()> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::new(
            Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::default(),
        );

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .context("哈希密码失败")?
            .to_string();

        self.master_password_hash = Some(password_hash);
        Ok(())
    }

    /// 验证主密码
    pub fn verify_master_password(&self, password: &str) -> Result<bool> {
        match &self.master_password_hash {
            Some(hash_str) => {
                let parsed_hash = PasswordHash::new(hash_str).context("解析密码哈希失败")?;
                let argon2 = Argon2::default();
                Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
            }
            None => Ok(false), // 没有设置主密码
        }
    }

    /// 添加密码条目
    pub fn add_password(&mut self, entry: PasswordEntry) -> Result<()> {
        // 在实际应用中，这里应该加密密码
        self.passwords.insert(entry.id.clone(), entry);
        Ok(())
    }

    /// 查找密码条目
    pub fn find_password(&self, id: &str) -> Option<&PasswordEntry> {
        self.passwords.get(id)
    }

    /// 搜索密码条目
    pub fn search_passwords(&self, query: &str) -> Vec<&PasswordEntry> {
        let query_lower = query.to_lowercase();

        self.passwords.values()
            .filter(|entry| {
                entry.name.to_lowercase().contains(&query_lower) ||
                entry.username.as_ref().map_or(false, |u| u.to_lowercase().contains(&query_lower)) ||
                entry.url.as_ref().map_or(false, |u| u.to_lowercase().contains(&query_lower)) ||
                entry.notes.as_ref().map_or(false, |n| n.to_lowercase().contains(&query_lower)) ||
                entry.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// 更新密码条目
    pub fn update_password(&mut self, id: &str, entry: PasswordEntry) -> Result<()> {
        if !self.passwords.contains_key(id) {
            return Err(anyhow::anyhow!("密码条目不存在: {}", id));
        }

        self.passwords.insert(id.to_string(), entry);
        Ok(())
    }

    /// 删除密码条目
    pub fn delete_password(&mut self, id: &str) -> Result<()> {
        if self.passwords.remove(id).is_none() {
            return Err(anyhow::anyhow!("密码条目不存在: {}", id));
        }

        Ok(())
    }

    /// 获取所有密码条目
    pub fn get_all_passwords(&self) -> Vec<&PasswordEntry> {
        self.passwords.values().collect()
    }

    /// 生成强密码
    pub fn generate_password(
        length: usize,
        include_uppercase: bool,
        include_lowercase: bool,
        include_numbers: bool,
        include_symbols: bool,
    ) -> String {
        use rand::Rng;
        use rand::seq::SliceRandom;

        let mut charset = String::new();

        if include_lowercase {
            charset.push_str("abcdefghijklmnopqrstuvwxyz");
        }
        if include_uppercase {
            charset.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        }
        if include_numbers {
            charset.push_str("0123456789");
        }
        if include_symbols {
            charset.push_str("!@#$%^&*()-_=+[]{}|;:,.<>?");
        }

        // 如果字符集为空，使用默认字符集
        if charset.is_empty() {
            charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".to_string();
        }

        let charset_chars: Vec<char> = charset.chars().collect();
        let mut rng = rand::thread_rng();

        (0..length)
            .map(|_| *charset_chars.choose(&mut rng).unwrap())
            .collect()
    }

    /// 评估密码强度
    pub fn evaluate_password_strength(password: &str) -> PasswordStrength {
        let mut score = 0;

        // 长度评分
        if password.len() >= 8 {
            score += 1;
        }
        if password.len() >= 12 {
            score += 1;
        }
        if password.len() >= 16 {
            score += 1;
        }

        // 字符类型评分
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_digit = password.chars().any(|c| c.is_digit(10));
        let has_symbol = password.chars().any(|c| !c.is_alphanumeric());

        if has_lowercase { score += 1; }
        if has_uppercase { score += 1; }
        if has_digit { score += 1; }
        if has_symbol { score += 1; }

        // 避免常见模式
        let common_passwords = ["password", "123456", "qwerty", "admin", "welcome"];
        if common_passwords.iter().any(|&p| password.to_lowercase().contains(p)) {
            score = score.saturating_sub(2);
        }

        match score {
            0..=3 => PasswordStrength::Weak,
            4..=6 => PasswordStrength::Medium,
            7..=8 => PasswordStrength::Strong,
            _ => PasswordStrength::VeryStrong,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum PasswordStrength {
    Weak,
    Medium,
    Strong,
    VeryStrong,
}

impl PasswordStrength {
    pub fn to_string(&self) -> &'static str {
        match self {
            PasswordStrength::Weak => "弱",
            PasswordStrength::Medium => "中等",
            PasswordStrength::Strong => "强",
            PasswordStrength::VeryStrong => "非常强",
        }
    }
}