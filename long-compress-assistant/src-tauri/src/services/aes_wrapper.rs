use anyhow::{Context, Result, anyhow};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce, Key
};
use argon2::{Argon2, Algorithm, Params, Version};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// 通用 AES 加密包装器
/// 可以加密任何已压缩的数据流
pub struct AesWrapper;

const MAGIC: &[u8; 8] = b"AESENC01";
const SALT_SIZE: usize = 32;
const NONCE_SIZE: usize = 12;

impl AesWrapper {
    /// 加密文件
    ///
    /// # 参数
    /// * `input` - 输入文件路径（已压缩）
    /// * `output` - 输出文件路径（加密后）
    /// * `password` - 加密密码
    pub fn encrypt_file(input: &Path, output: &Path, password: &str) -> Result<()> {
        // 读取输入文件
        let mut input_file = File::open(input)
            .with_context(|| format!("打开输入文件失败: {:?}", input))?;

        let mut data = Vec::new();
        input_file.read_to_end(&mut data)
            .context("读取输入文件失败")?;

        // 加密
        Self::encrypt_data(&data, output, password)
    }

    /// 解密文件
    ///
    /// # 参数
    /// * `input` - 输入文件路径（已加密）
    /// * `output` - 输出文件路径（解密后）
    /// * `password` - 解密密码
    pub fn decrypt_file(input: &Path, output: &Path, password: &str) -> Result<()> {
        // 解密到内存
        let decrypted_data = Self::decrypt_data(input, password)?;

        // 写入输出文件
        let mut output_file = File::create(output)
            .with_context(|| format!("创建输出文件失败: {:?}", output))?;

        output_file.write_all(&decrypted_data)
            .context("写入输出文件失败")?;

        Ok(())
    }

    /// 加密数据到文件
    pub fn encrypt_data(data: &[u8], output: &Path, password: &str) -> Result<()> {
        // 生成随机盐
        let mut salt = [0u8; SALT_SIZE];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut salt);

        // 从密码派生密钥
        let key = Self::derive_key(password, &salt)?;

        // 生成随机 nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // 创建加密器
        let cipher = Aes256Gcm::new(&key);

        // 加密数据
        let ciphertext = cipher.encrypt(nonce, data)
            .map_err(|e| anyhow!("加密失败: {:?}", e))?;

        // 写入文件：Magic + Salt + Nonce + Encrypted Data
        let mut output_file = File::create(output)
            .with_context(|| format!("创建输出文件失败: {:?}", output))?;

        output_file.write_all(MAGIC)?;
        output_file.write_all(&salt)?;
        output_file.write_all(&nonce_bytes)?;
        output_file.write_all(&ciphertext)?;

        Ok(())
    }

    /// 从文件解密数据
    pub fn decrypt_data(input: &Path, password: &str) -> Result<Vec<u8>> {
        // 读取加密文件
        let mut file = File::open(input)
            .with_context(|| format!("打开加密文件失败: {:?}", input))?;

        // 读取 Magic
        let mut magic = [0u8; 8];
        file.read_exact(&mut magic)
            .context("读取文件头失败")?;

        if &magic != MAGIC {
            return Err(anyhow!("无效的加密文件格式"));
        }

        // 读取 Salt
        let mut salt = [0u8; SALT_SIZE];
        file.read_exact(&mut salt)
            .context("读取盐值失败")?;

        // 读取 Nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        file.read_exact(&mut nonce_bytes)
            .context("读取 nonce 失败")?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        // 读取加密数据
        let mut ciphertext = Vec::new();
        file.read_to_end(&mut ciphertext)
            .context("读取加密数据失败")?;

        // 从密码派生密钥
        let key = Self::derive_key(password, &salt)?;

        // 创建解密器
        let cipher = Aes256Gcm::new(&key);

        // 解密数据
        let plaintext = cipher.decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| anyhow!("解密失败: 密码错误或文件已损坏"))?;

        Ok(plaintext)
    }

    /// 从密码派生密钥（使用 Argon2）
    fn derive_key(password: &str, salt: &[u8]) -> Result<Key<Aes256Gcm>> {
        let mut key_bytes = [0u8; 32];

        let params = Params::new(65536, 3, 1, Some(32))
            .map_err(|e| anyhow!("创建 Argon2 参数失败: {:?}", e))?;

        let argon2 = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            params,
        );

        argon2.hash_password_into(password.as_bytes(), salt, &mut key_bytes)
            .map_err(|e| anyhow!("密钥派生失败: {:?}", e))?;

        Ok(*Key::<Aes256Gcm>::from_slice(&key_bytes))
    }

    /// 检测文件是否为 AES 加密格式
    pub fn is_aes_encrypted(path: &Path) -> Result<bool> {
        let mut file = File::open(path)?;
        let mut magic = [0u8; 8];

        if file.read_exact(&mut magic).is_err() {
            return Ok(false);
        }

        Ok(&magic == MAGIC)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_encrypt_decrypt_roundtrip() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let input_file = temp_dir.path().join("input.txt");
        let encrypted_file = temp_dir.path().join("encrypted.aes");
        let decrypted_file = temp_dir.path().join("decrypted.txt");

        let test_data = b"Test data for encryption";
        fs::write(&input_file, test_data)?;

        // 加密
        AesWrapper::encrypt_file(&input_file, &encrypted_file, "test_password")?;
        assert!(encrypted_file.exists());
        assert!(AesWrapper::is_aes_encrypted(&encrypted_file)?);

        // 解密
        AesWrapper::decrypt_file(&encrypted_file, &decrypted_file, "test_password")?;

        let decrypted_data = fs::read(decrypted_file)?;
        assert_eq!(decrypted_data, test_data);

        Ok(())
    }

    #[test]
    fn test_wrong_password() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let input_file = temp_dir.path().join("input.txt");
        let encrypted_file = temp_dir.path().join("encrypted.aes");
        let decrypted_file = temp_dir.path().join("decrypted.txt");

        fs::write(&input_file, b"Secret data")?;

        AesWrapper::encrypt_file(&input_file, &encrypted_file, "correct_password")?;

        let result = AesWrapper::decrypt_file(&encrypted_file, &decrypted_file, "wrong_password");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("密码错误"));
        Ok(())
    }
}
