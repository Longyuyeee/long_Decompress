use anyhow::{Result, anyhow};
use std::path::Path;
use std::fs::File;
use std::io::{Read, BufReader};
use sha2::{Sha256, Digest};
use md5::Md5;

/// 文件完整性校验服务
/// 支持 CRC32, MD5, SHA256 校验算法
pub struct FileIntegrityService;

/// 校验算法类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChecksumAlgorithm {
    CRC32,
    MD5,
    SHA256,
}

/// 校验结果
#[derive(Debug, Clone)]
pub struct ChecksumResult {
    pub algorithm: ChecksumAlgorithm,
    pub checksum: String,
    pub file_size: u64,
}

impl FileIntegrityService {
    /// 计算文件校验和
    pub fn calculate_checksum(path: &Path, algorithm: ChecksumAlgorithm) -> Result<ChecksumResult> {
        let file = File::open(path)?;
        let file_size = file.metadata()?.len();
        let mut reader = BufReader::new(file);

        let checksum = match algorithm {
            ChecksumAlgorithm::CRC32 => Self::calculate_crc32(&mut reader)?,
            ChecksumAlgorithm::MD5 => Self::calculate_md5(&mut reader)?,
            ChecksumAlgorithm::SHA256 => Self::calculate_sha256(&mut reader)?,
        };

        Ok(ChecksumResult {
            algorithm,
            checksum,
            file_size,
        })
    }

    /// 验证文件校验和
    pub fn verify_checksum(path: &Path, expected: &str, algorithm: ChecksumAlgorithm) -> Result<bool> {
        let result = Self::calculate_checksum(path, algorithm)?;
        Ok(result.checksum.eq_ignore_ascii_case(expected))
    }

    /// 计算多个算法的校验和
    pub fn calculate_all_checksums(path: &Path) -> Result<Vec<ChecksumResult>> {
        let mut results = Vec::new();

        for algorithm in &[ChecksumAlgorithm::CRC32, ChecksumAlgorithm::MD5, ChecksumAlgorithm::SHA256] {
            match Self::calculate_checksum(path, *algorithm) {
                Ok(result) => results.push(result),
                Err(e) => log::warn!("计算 {:?} 校验和失败: {}", algorithm, e),
            }
        }

        Ok(results)
    }

    /// 计算 CRC32 校验和
    fn calculate_crc32<R: Read>(reader: &mut R) -> Result<String> {
        let mut hasher = crc32fast::Hasher::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:08x}", hasher.finalize()))
    }

    /// 计算 MD5 校验和
    fn calculate_md5<R: Read>(reader: &mut R) -> Result<String> {
        let mut hasher = Md5::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    /// 计算 SHA256 校验和
    fn calculate_sha256<R: Read>(reader: &mut R) -> Result<String> {
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    /// 生成校验文件（类似 md5sum 格式）
    pub fn generate_checksum_file(
        files: &[&Path],
        algorithm: ChecksumAlgorithm,
        output_path: &Path,
    ) -> Result<()> {
        use std::io::Write;

        let mut output = File::create(output_path)?;

        for file in files {
            let result = Self::calculate_checksum(file, algorithm)?;
            let file_name = file.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            writeln!(output, "{}  {}", result.checksum, file_name)?;
        }

        Ok(())
    }

    /// 验证校验文件
    pub fn verify_checksum_file(checksum_file: &Path, base_dir: &Path) -> Result<Vec<(String, bool)>> {
        use std::io::BufRead;

        let file = File::open(checksum_file)?;
        let reader = BufReader::new(file);
        let mut results = Vec::new();

        // 自动检测算法（根据校验和长度）
        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.split_whitespace().collect();

            if parts.len() < 2 {
                continue;
            }

            let expected_checksum = parts[0];
            let file_name = parts[1];
            let file_path = base_dir.join(file_name);

            if !file_path.exists() {
                results.push((file_name.to_string(), false));
                continue;
            }

            // 根据校验和长度推断算法
            let algorithm = match expected_checksum.len() {
                8 => ChecksumAlgorithm::CRC32,
                32 => ChecksumAlgorithm::MD5,
                64 => ChecksumAlgorithm::SHA256,
                _ => {
                    log::warn!("未知的校验和长度: {}", expected_checksum.len());
                    continue;
                }
            };

            let is_valid = Self::verify_checksum(&file_path, expected_checksum, algorithm)
                .unwrap_or(false);

            results.push((file_name.to_string(), is_valid));
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_calculate_crc32() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path)?;
        file.write_all(b"Hello, World!")?;
        drop(file);

        let result = FileIntegrityService::calculate_checksum(&file_path, ChecksumAlgorithm::CRC32)?;
        assert_eq!(result.checksum.len(), 8);
        assert_eq!(result.file_size, 13);
        Ok(())
    }

    #[test]
    fn test_calculate_md5() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path)?;
        file.write_all(b"Hello, World!")?;
        drop(file);

        let result = FileIntegrityService::calculate_checksum(&file_path, ChecksumAlgorithm::MD5)?;
        assert_eq!(result.checksum.len(), 32);
        assert_eq!(result.checksum, "65a8e27d8879283831b664bd8b7f0ad4");
        Ok(())
    }

    #[test]
    fn test_calculate_sha256() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path)?;
        file.write_all(b"Hello, World!")?;
        drop(file);

        let result = FileIntegrityService::calculate_checksum(&file_path, ChecksumAlgorithm::SHA256)?;
        assert_eq!(result.checksum.len(), 64);
        Ok(())
    }

    #[test]
    fn test_verify_checksum() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path)?;
        file.write_all(b"Hello, World!")?;
        drop(file);

        let is_valid = FileIntegrityService::verify_checksum(
            &file_path,
            "65a8e27d8879283831b664bd8b7f0ad4",
            ChecksumAlgorithm::MD5,
        )?;

        assert!(is_valid);
        Ok(())
    }
}
