use anyhow::{Context, Result};
use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use blake3::Hasher as Blake3Hasher;
use rand::rngs::OsRng;
use sha2::{Sha256, Sha512, Digest};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum HashAlgorithm {
    Argon2id,
    Blake3,
    SHA256,
    SHA512,
}

impl HashAlgorithm {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Argon2id => "Argon2id",
            Self::Blake3 => "BLAKE3",
            Self::SHA256 => "SHA-256",
            Self::SHA512 => "SHA-512",
        }
    }

    pub fn is_password_hash(&self) -> bool {
        matches!(self, Self::Argon2id)
    }

    pub fn is_fast_hash(&self) -> bool {
        matches!(self, Self::Blake3 | Self::SHA256 | Self::SHA512)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashResult {
    pub algorithm: HashAlgorithm,
    pub hash: String,
    pub salt: Option<String>,
    pub iterations: Option<u32>,
    pub memory_cost: Option<u32>,
    pub parallelism: Option<u32>,
}

pub struct HashingService;

impl HashingService {
    /// 使用Argon2id哈希密码
    pub fn hash_password(password: &str) -> Result<HashResult> {
        let salt = SaltString::generate(&mut OsRng);

        let argon2 = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(65536, 2, 1, None).context("创建Argon2参数失败")?,
        );

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .context("哈希密码失败")?;

        Ok(HashResult {
            algorithm: HashAlgorithm::Argon2id,
            hash: password_hash.to_string(),
            salt: Some(salt.to_string()),
            iterations: Some(65536),
            memory_cost: Some(65536),
            parallelism: Some(2),
        })
    }

    /// 使用自定义参数哈希密码
    pub fn hash_password_with_params(
        password: &str,
        iterations: u32,
        memory_cost: u32,
        parallelism: u32,
    ) -> Result<HashResult> {
        let salt = SaltString::generate(&mut OsRng);

        let argon2 = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(memory_cost, parallelism, 1, None).context("创建Argon2参数失败")?,
        );

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .context("哈希密码失败")?;

        Ok(HashResult {
            algorithm: HashAlgorithm::Argon2id,
            hash: password_hash.to_string(),
            salt: Some(salt.to_string()),
            iterations: Some(iterations),
            memory_cost: Some(memory_cost),
            parallelism: Some(parallelism),
        })
    }

    /// 验证密码
    pub fn verify_password(password: &str, hash_result: &HashResult) -> Result<bool> {
        if hash_result.algorithm != HashAlgorithm::Argon2id {
            return Err(anyhow::anyhow!("不支持的哈希算法"));
        }

        let parsed_hash = PasswordHash::new(&hash_result.hash).context("解析密码哈希失败")?;
        let argon2 = Argon2::default();

        Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
    }

    /// 使用BLAKE3哈希数据
    pub fn hash_blake3(data: &[u8]) -> HashResult {
        let mut hasher = Blake3Hasher::new();
        hasher.update(data);
        let hash = hasher.finalize();

        HashResult {
            algorithm: HashAlgorithm::Blake3,
            hash: general_purpose::STANDARD.encode(hash.as_bytes()),
            salt: None,
            iterations: None,
            memory_cost: None,
            parallelism: None,
        }
    }

    /// 使用SHA-256哈希数据
    pub fn hash_sha256(data: &[u8]) -> HashResult {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();

        HashResult {
            algorithm: HashAlgorithm::SHA256,
            hash: general_purpose::STANDARD.encode(hash.as_slice()),
            salt: None,
            iterations: None,
            memory_cost: None,
            parallelism: None,
        }
    }

    /// 使用SHA-512哈希数据
    pub fn hash_sha512(data: &[u8]) -> HashResult {
        let mut hasher = Sha512::new();
        hasher.update(data);
        let hash = hasher.finalize();

        HashResult {
            algorithm: HashAlgorithm::SHA512,
            hash: general_purpose::STANDARD.encode(hash.as_slice()),
            salt: None,
            iterations: None,
            memory_cost: None,
            parallelism: None,
        }
    }

    /// 使用指定算法哈希数据
    pub fn hash_data(data: &[u8], algorithm: HashAlgorithm) -> HashResult {
        match algorithm {
            HashAlgorithm::Blake3 => Self::hash_blake3(data),
            HashAlgorithm::SHA256 => Self::hash_sha256(data),
            HashAlgorithm::SHA512 => Self::hash_sha512(data),
            HashAlgorithm::Argon2id => {
                // Argon2不适合普通数据哈希，回退到BLAKE3
                Self::hash_blake3(data)
            }
        }
    }

    /// 验证数据哈希
    pub fn verify_data(data: &[u8], hash_result: &HashResult) -> bool {
        let new_hash = Self::hash_data(data, hash_result.algorithm);
        new_hash.hash == hash_result.hash
    }

    /// 计算文件的BLAKE3哈希
    pub fn hash_file_blake3(file_path: &str) -> Result<HashResult> {
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(file_path).context("打开文件失败")?;
        let mut hasher = Blake3Hasher::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = file.read(&mut buffer).context("读取文件失败")?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let hash = hasher.finalize();

        Ok(HashResult {
            algorithm: HashAlgorithm::Blake3,
            hash: general_purpose::STANDARD.encode(hash.as_bytes()),
            salt: None,
            iterations: None,
            memory_cost: None,
            parallelism: None,
        })
    }

    /// 计算文件的SHA-256哈希
    pub fn hash_file_sha256(file_path: &str) -> Result<HashResult> {
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(file_path).context("打开文件失败")?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = file.read(&mut buffer).context("读取文件失败")?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let hash = hasher.finalize();

        Ok(HashResult {
            algorithm: HashAlgorithm::SHA256,
            hash: general_purpose::STANDARD.encode(hash.as_slice()),
            salt: None,
            iterations: None,
            memory_cost: None,
            parallelism: None,
        })
    }

    /// 验证文件完整性
    pub fn verify_file_integrity(file_path: &str, expected_hash: &HashResult) -> Result<bool> {
        let actual_hash = match expected_hash.algorithm {
            HashAlgorithm::Blake3 => Self::hash_file_blake3(file_path)?,
            HashAlgorithm::SHA256 => Self::hash_file_sha256(file_path)?,
            HashAlgorithm::SHA512 => {
                // SHA-512文件哈希实现类似
                Self::hash_file_sha256(file_path)? // 暂时使用SHA-256
            }
            HashAlgorithm::Argon2id => {
                return Err(anyhow::anyhow!("Argon2不适合文件哈希"));
            }
        };

        Ok(actual_hash.hash == expected_hash.hash)
    }

    /// 生成随机盐值
    pub fn generate_salt() -> String {
        let salt = SaltString::generate(&mut OsRng);
        salt.to_string()
    }

    /// 生成随机令牌
    pub fn generate_token(length: usize) -> String {
        use rand::Rng;
        use rand::distributions::Alphanumeric;

        let rng = rand::thread_rng();
        rng.sample_iter(&Alphanumeric)
            .take(length)
            .map(char::from)
            .collect()
    }

    /// 生成安全随机数
    pub fn generate_random_bytes(length: usize) -> Vec<u8> {
        let mut bytes = vec![0u8; length];
        OsRng.fill_bytes(&mut bytes);
        bytes
    }
}

/// 密码强度评估
pub struct PasswordStrengthChecker;

impl PasswordStrengthChecker {
    /// 评估密码强度
    pub fn evaluate(password: &str) -> PasswordStrength {
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
        let common_passwords = [
            "password", "123456", "qwerty", "admin", "welcome",
            "password123", "12345678", "123456789", "1234567890",
            "abc123", "letmein", "monkey", "dragon", "baseball",
        ];

        let password_lower = password.to_lowercase();
        if common_passwords.iter().any(|&p| password_lower.contains(p)) {
            score = score.saturating_sub(2);
        }

        // 检查序列和重复
        if Self::has_sequential_chars(password) {
            score = score.saturating_sub(1);
        }
        if Self::has_repeating_chars(password) {
            score = score.saturating_sub(1);
        }

        // 根据分数确定强度
        match score {
            0..=3 => PasswordStrength::VeryWeak,
            4..=5 => PasswordStrength::Weak,
            6..=7 => PasswordStrength::Medium,
            8..=9 => PasswordStrength::Strong,
            _ => PasswordStrength::VeryStrong,
        }
    }

    /// 检查是否有连续字符
    fn has_sequential_chars(password: &str) -> bool {
        if password.len() < 3 {
            return false;
        }

        let chars: Vec<char> = password.chars().collect();
        for i in 0..chars.len() - 2 {
            let c1 = chars[i] as u8;
            let c2 = chars[i + 1] as u8;
            let c3 = chars[i + 2] as u8;

            // 检查数字序列
            if c1.is_ascii_digit() && c2.is_ascii_digit() && c3.is_ascii_digit() {
                if (c1 + 1 == c2 && c2 + 1 == c3) || (c1 - 1 == c2 && c2 - 1 == c3) {
                    return true;
                }
            }

            // 检查字母序列
            if c1.is_ascii_alphabetic() && c2.is_ascii_alphabetic() && c3.is_ascii_alphabetic() {
                let c1_lower = c1.to_ascii_lowercase();
                let c2_lower = c2.to_ascii_lowercase();
                let c3_lower = c3.to_ascii_lowercase();

                if (c1_lower + 1 == c2_lower && c2_lower + 1 == c3_lower) ||
                   (c1_lower - 1 == c2_lower && c2_lower - 1 == c3_lower) {
                    return true;
                }
            }
        }

        false
    }

    /// 检查是否有重复字符
    fn has_repeating_chars(password: &str) -> bool {
        if password.len() < 3 {
            return false;
        }

        let chars: Vec<char> = password.chars().collect();
        for i in 0..chars.len() - 2 {
            if chars[i] == chars[i + 1] && chars[i + 1] == chars[i + 2] {
                return true;
            }
        }

        false
    }

    /// 获取密码建议
    pub fn get_suggestions(password: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        if password.len() < 8 {
            suggestions.push("密码长度至少应为8个字符".to_string());
        }

        if !password.chars().any(|c| c.is_lowercase()) {
            suggestions.push("添加小写字母".to_string());
        }

        if !password.chars().any(|c| c.is_uppercase()) {
            suggestions.push("添加大写字母".to_string());
        }

        if !password.chars().any(|c| c.is_digit(10)) {
            suggestions.push("添加数字".to_string());
        }

        if !password.chars().any(|c| !c.is_alphanumeric()) {
            suggestions.push("添加特殊字符（如!@#$%）".to_string());
        }

        if Self::has_sequential_chars(password) {
            suggestions.push("避免使用连续字符".to_string());
        }

        if Self::has_repeating_chars(password) {
            suggestions.push("避免使用重复字符".to_string());
        }

        suggestions
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PasswordStrength {
    VeryWeak,
    Weak,
    Medium,
    Strong,
    VeryStrong,
}

impl PasswordStrength {
    pub fn name(&self) -> &'static str {
        match self {
            Self::VeryWeak => "非常弱",
            Self::Weak => "弱",
            Self::Medium => "中等",
            Self::Strong => "强",
            Self::VeryStrong => "非常强",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            Self::VeryWeak => "#ff4444",
            Self::Weak => "#ff8800",
            Self::Medium => "#ffbb33",
            Self::Strong => "#00c851",
            Self::VeryStrong => "#007e33",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let password = "my_secret_password";
        let hash_result = HashingService::hash_password(password).unwrap();

        assert_eq!(hash_result.algorithm, HashAlgorithm::Argon2id);
        assert!(hash_result.salt.is_some());
        assert!(hash_result.iterations.is_some());

        let is_valid = HashingService::verify_password(password, &hash_result).unwrap();
        assert!(is_valid);

        let wrong_password = "wrong_password";
        let is_wrong_valid = HashingService::verify_password(wrong_password, &hash_result).unwrap();
        assert!(!is_wrong_valid);
    }

    #[test]
    fn test_data_hashing() {
        let data = b"test data";

        let blake3_hash = HashingService::hash_blake3(data);
        assert_eq!(blake3_hash.algorithm, HashAlgorithm::Blake3);

        let sha256_hash = HashingService::hash_sha256(data);
        assert_eq!(sha256_hash.algorithm, HashAlgorithm::SHA256);

        let sha512_hash = HashingService::hash_sha512(data);
        assert_eq!(sha512_hash.algorithm, HashAlgorithm::SHA512);

        // 验证哈希
        assert!(HashingService::verify_data(data, &blake3_hash));
        assert!(HashingService::verify_data(data, &sha256_hash));
        assert!(HashingService::verify_data(data, &sha512_hash));

        let wrong_data = b"wrong data";
        assert!(!HashingService::verify_data(wrong_data, &blake3_hash));
    }

    #[test]
    fn test_password_strength() {
        assert_eq!(
            PasswordStrengthChecker::evaluate("123"),
            PasswordStrength::VeryWeak
        );

        assert_eq!(
            PasswordStrengthChecker::evaluate("password123"),
            PasswordStrength::Weak
        );

        assert_eq!(
            PasswordStrengthChecker::evaluate("Password123"),
            PasswordStrength::Medium
        );

        assert_eq!(
            PasswordStrengthChecker::evaluate("Password123!"),
            PasswordStrength::Strong
        );

        assert_eq!(
            PasswordStrengthChecker::evaluate("V3ry$tr0ngP@ssw0rd!"),
            PasswordStrength::VeryStrong
        );
    }
}