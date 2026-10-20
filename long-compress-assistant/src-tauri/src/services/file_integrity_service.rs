use anyhow::Result;
use std::path::Path;
use std::fs::File;
use std::io::{Read, BufReader};
use sha2::{Sha256, Digest};

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
        Self::calculate_checksum_with_progress(path, algorithm, &mut |_, _| true)?
            .ok_or_else(|| anyhow::anyhow!("校验已停止"))
    }

    /// Returning false stops before the next read and never finalizes a partial digest.
    pub fn calculate_checksum_with_progress(path: &Path, algorithm: ChecksumAlgorithm, progress: &mut dyn FnMut(u64, u64) -> bool) -> Result<Option<ChecksumResult>> {
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        anyhow::ensure!(metadata.is_file(), "路径不是普通文件");
        let file_size = metadata.len();
        let modified = metadata.modified().ok();
        let mut reader = BufReader::new(file);
        let mut crc = crc32fast::Hasher::new();
        let mut md5 = md5::Context::new();
        let mut sha = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        let mut bytes = 0;
        if !progress(bytes, file_size) { return Ok(None); }
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 { break; }
            match algorithm {
                ChecksumAlgorithm::CRC32 => crc.update(&buffer[..count]),
                ChecksumAlgorithm::MD5 => md5.consume(&buffer[..count]),
                ChecksumAlgorithm::SHA256 => sha.update(&buffer[..count]),
            }
            bytes += count as u64;
            if !progress(bytes, file_size) { return Ok(None); }
        }
        let after = reader.get_ref().metadata()?;
        anyhow::ensure!(bytes == file_size && after.len() == file_size && after.modified().ok() == modified, "读取期间文件发生变化，请重新检查");
        let checksum = match algorithm {
            ChecksumAlgorithm::CRC32 => format!("{:08x}", crc.finalize()),
            ChecksumAlgorithm::MD5 => format!("{:x}", md5.compute()),
            ChecksumAlgorithm::SHA256 => format!("{:x}", sha.finalize()),
        };
        Ok(Some(ChecksumResult { algorithm, checksum, file_size }))
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

            match algorithm {
                ChecksumAlgorithm::CRC32 => writeln!(output, "{} {}", file_name, result.checksum)?,
                _ => writeln!(output, "{}  {}", result.checksum, file_name)?,
            }
        }

        Ok(())
    }

    /// Verify every row, including explicit missing/unreadable/invalid causes.
    pub fn verify_checksum_file(checksum_file: &Path, base_dir: &Path) -> Result<super::checksum_manifest::ChecksumVerificationReport> {
        Self::verify_checksum_entries(checksum_file, base_dir, None)
    }

    pub fn verify_checksum_entries(checksum_file: &Path, base_dir: &Path, only_lines: Option<&[usize]>) -> Result<super::checksum_manifest::ChecksumVerificationReport> {
        super::checksum_manifest::verify(checksum_file, base_dir, only_lines)
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
