use anyhow::{Context, Result, anyhow};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce, Key
};
use argon2::{Argon2, Algorithm, Params, Version};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tar::Builder;

/// TAR.AES 加密引擎
/// 提供 TAR 归档 + AES-256-GCM 加密的组合功能
///
/// 文件格式:
/// [Magic: 8 bytes]["TARAES01"][Salt: 32 bytes][Nonce: 12 bytes][Encrypted Data][Auth Tag: 16 bytes]
pub struct TarAesEngine;

const MAGIC: &[u8; 8] = b"TARAES01";
const SALT_SIZE: usize = 32;
const NONCE_SIZE: usize = 12;

impl TarAesEngine {
    /// 压缩并加密文件到 TAR.AES 格式
    ///
    /// # 参数
    /// * `files` - 要压缩的文件/目录列表
    /// * `output` - 输出文件路径（.tar.aes）
    /// * `password` - 加密密码
    /// * `base_dir` - 可选的基础目录，用于计算相对路径
    pub fn compress_tar_aes(
        files: &[PathBuf],
        output: &Path,
        password: &str,
        base_dir: Option<&Path>,
    ) -> Result<()> {
        // 步骤 1: 创建 TAR 归档到内存缓冲区
        let tar_data = Self::create_tar_archive(files, base_dir)
            .context("创建 TAR 归档失败")?;

        // 步骤 2: 使用 AES-256-GCM 加密
        Self::encrypt_with_aes(&tar_data, output, password)
            .context("AES 加密失败")?;

        Ok(())
    }

    /// 解密并解压 TAR.AES 文件
    ///
    /// # 参数
    /// * `archive_path` - TAR.AES 文件路径
    /// * `output_dir` - 解压目标目录
    /// * `password` - 解密密码
    pub fn decompress_tar_aes(
        archive_path: &Path,
        output_dir: &Path,
        password: &str,
    ) -> Result<()> {
        // 步骤 1: AES 解密到内存
        let tar_data = Self::decrypt_with_aes(archive_path, password)
            .context("AES 解密失败")?;

        // 步骤 2: 解压 TAR 归档
        Self::extract_tar_archive(&tar_data, output_dir)
            .context("解压 TAR 归档失败")?;

        Ok(())
    }

    /// 创建 TAR 归档到内存
    fn create_tar_archive(files: &[PathBuf], base_dir: Option<&Path>) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();
        {
            let mut archive = Builder::new(&mut buffer);

            for file_path in files {
                if file_path.is_file() {
                    let archive_path = if let Some(base) = base_dir {
                        file_path.strip_prefix(base)
                            .unwrap_or(file_path)
                    } else {
                        file_path.file_name()
                            .map(Path::new)
                            .unwrap_or(file_path)
                    };

                    archive.append_path_with_name(file_path, archive_path)
                        .with_context(|| format!("添加文件失败: {:?}", file_path))?;
                } else if file_path.is_dir() {
                    let archive_path = if let Some(base) = base_dir {
                        file_path.strip_prefix(base)
                            .unwrap_or(file_path)
                    } else {
                        file_path
                    };

                    archive.append_dir_all(archive_path, file_path)
                        .with_context(|| format!("添加目录失败: {:?}", file_path))?;
                }
            }

            archive.finish()
                .context("完成 TAR 归档失败")?;
        }

        Ok(buffer)
    }

    /// 使用 AES-256-GCM 加密数据
    fn encrypt_with_aes(data: &[u8], output: &Path, password: &str) -> Result<()> {
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

    /// 使用 AES-256-GCM 解密数据
    fn decrypt_with_aes(archive_path: &Path, password: &str) -> Result<Vec<u8>> {
        // 读取加密文件
        let mut file = File::open(archive_path)
            .with_context(|| format!("打开加密文件失败: {:?}", archive_path))?;

        // 读取 Magic
        let mut magic = [0u8; 8];
        file.read_exact(&mut magic)
            .context("读取文件头失败")?;

        if &magic != MAGIC {
            return Err(anyhow!("无效的文件格式"));
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

    /// 从内存解压 TAR 归档
    fn extract_tar_archive(data: &[u8], output_dir: &Path) -> Result<()> {
        let mut archive = tar::Archive::new(data);

        // 创建输出目录
        std::fs::create_dir_all(output_dir)
            .with_context(|| format!("创建输出目录失败: {:?}", output_dir))?;

        // 解压所有条目
        archive.unpack(output_dir)
            .context("解压归档失败")?;

        Ok(())
    }

    /// 检测文件是否为 TAR.AES 加密格式
    pub fn is_tar_aes(path: &Path) -> Result<bool> {
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
    fn test_tar_aes_roundtrip() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, b"Test content for AES encryption")?;

        let output = temp_dir.path().join("test.tar.aes");
        let extract_dir = temp_dir.path().join("extracted");

        // 压缩加密
        TarAesEngine::compress_tar_aes(
            &[test_file.clone()],
            &output,
            "test_password",
            Some(temp_dir.path()),
        )?;

        assert!(output.exists());
        assert!(TarAesEngine::is_tar_aes(&output)?);

        // 解压解密
        TarAesEngine::decompress_tar_aes(
            &output,
            &extract_dir,
            "test_password",
        )?;

        let extracted_file = extract_dir.join("test.txt");
        assert!(extracted_file.exists());

        let content = fs::read_to_string(extracted_file)?;
        assert_eq!(content, "Test content for AES encryption");

        Ok(())
    }

    #[test]
    fn test_wrong_password() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, b"Test content")?;

        let output = temp_dir.path().join("test.tar.aes");
        let extract_dir = temp_dir.path().join("extracted");

        TarAesEngine::compress_tar_aes(
            &[test_file],
            &output,
            "correct_password",
            Some(temp_dir.path()),
        )?;

        let result = TarAesEngine::decompress_tar_aes(
            &output,
            &extract_dir,
            "wrong_password",
        );

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("密码错误") || err_msg.contains("解密失败"));
        Ok(())
    }

    #[test]
    fn test_multiple_files() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        fs::write(&file1, b"Content 1")?;
        fs::write(&file2, b"Content 2")?;

        let output = temp_dir.path().join("test.tar.aes");
        let extract_dir = temp_dir.path().join("extracted");

        TarAesEngine::compress_tar_aes(
            &[file1, file2],
            &output,
            "password123",
            Some(temp_dir.path()),
        )?;

        TarAesEngine::decompress_tar_aes(&output, &extract_dir, "password123")?;

        assert!(extract_dir.join("file1.txt").exists());
        assert!(extract_dir.join("file2.txt").exists());

        Ok(())
    }
}
