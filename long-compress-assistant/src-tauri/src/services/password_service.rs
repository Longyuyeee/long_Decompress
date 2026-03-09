use anyhow::{Context, Result};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use crate::models::password::PasswordStrength;

pub struct PasswordService;

impl PasswordService {
    pub fn verify_password(password: &str, hash_str: &str) -> Result<bool> {
        let parsed_hash = PasswordHash::new(hash_str).map_err(|e| anyhow::anyhow!("解析密码哈希失败: {}", e))?;
        let argon2 = Argon2::default();
        Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
    }

    pub fn evaluate_strength(password: &str) -> PasswordStrength {
        let mut score: i32 = 0;
        if password.len() >= 8 { score += 1; }
        if password.len() >= 12 { score += 1; }
        if password.chars().any(|c| c.is_uppercase()) && password.chars().any(|c| c.is_lowercase()) { score += 1; }
        if password.chars().any(|c| c.is_numeric()) { score += 1; }
        if password.chars().any(|c| !c.is_alphanumeric()) { score += 1; }

        match score {
            0..=1 => PasswordStrength::VeryWeak,
            2 => PasswordStrength::Weak,
            3 => PasswordStrength::Medium,
            4 => PasswordStrength::Strong,
            _ => PasswordStrength::VeryStrong,
        }
    }
}
