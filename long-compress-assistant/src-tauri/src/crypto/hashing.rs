use anyhow::{Context, Result};
use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use blake3::Hasher as Blake3Hasher;
use rand::rngs::OsRng;
use rand::RngCore;
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
            Params::new(65536, 2, 1, None).map_err(|e| anyhow::anyhow!("创建Argon2参数失败: {}", e))?,
        );

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("哈希密码失败: {}", e))?;

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
            Params::new(memory_cost, parallelism, 1, None).map_err(|e| anyhow::anyhow!("创建Argon2参数失败: {}", e))?,
        );

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("哈希密码失败: {}", e))?;

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

        let parsed_hash = PasswordHash::new(&hash_result.hash).map_err(|e| anyhow::anyhow!("解析密码哈希失败: {}", e))?;
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

/// 密码强度等级
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum PasswordStrength {
    VeryWeak = 0,
    Weak = 1,
    Medium = 2,
    Strong = 3,
    VeryStrong = 4,
}

/// 密码强度评估
pub struct PasswordStrengthChecker;

impl PasswordStrengthChecker {
    /// 评估密码强度
    pub fn evaluate(password: &str) -> PasswordStrength {
        let mut score: i32 = 0;

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

        // 复杂度评分
        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_numeric = password.chars().any(|c| c.is_numeric());
        let has_special = password.chars().any(|c| !c.is_alphanumeric());

        if has_uppercase && has_lowercase {
            score += 1;
        }
        if has_numeric {
            score += 1;
        }
        if has_special {
            score += 1;
        }

        // 根据总分返回强度等级
        match score {
            0..=1 => PasswordStrength::VeryWeak,
            2 => PasswordStrength::Weak,
            3 => PasswordStrength::Medium,
            4 => PasswordStrength::Strong,
            _ => PasswordStrength::VeryStrong,
        }
    }
}
