use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::fs;
use regex::Regex;

/// 分卷文件识别服务
/// 自动检测和识别各种分卷压缩格式
#[derive(Debug, Clone)]
pub struct SplitArchiveDetector;

/// 分卷格式类型
#[derive(Debug, Clone, PartialEq)]
pub enum SplitFormat {
    /// ZIP 分卷 (.zip, .z01, .z02, ...)
    ZipSplit,
    /// RAR 分卷 (.rar, .r00, .r01, ... 或 .part1.rar, .part2.rar, ...)
    RarSplit,
    /// 7Z 分卷 (.7z.001, .7z.002, ...)
    SevenZipSplit,
    /// 通用数字分卷 (.001, .002, .003, ...)
    GenericNumeric,
    /// 通用 part 分卷 (.part1, .part2, ...)
    GenericPart,
}

/// 分卷文件信息
#[derive(Debug, Clone)]
pub struct SplitArchiveInfo {
    /// 格式类型
    pub format: SplitFormat,
    /// 基础名称（不含扩展名）
    pub base_name: String,
    /// 所有分卷文件路径
    pub parts: Vec<PathBuf>,
    /// 第一个分卷（用于解压入口）
    pub first_part: PathBuf,
    /// 总分卷数
    pub total_parts: usize,
    /// 所有分卷的总大小
    pub total_size: u64,
}

impl SplitArchiveDetector {
    /// 检测文件是否为分卷压缩包
    pub fn is_split_archive(path: &Path) -> bool {
        if !path.exists() || !path.is_file() {
            return false;
        }

        let file_name = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => return false,
        };

        // 检测各种分卷格式
        Self::is_zip_split(&file_name) ||
        Self::is_rar_split(&file_name) ||
        Self::is_7z_split(&file_name) ||
        Self::is_generic_numeric_split(&file_name) ||
        Self::is_generic_part_split(&file_name)
    }

    /// 检测完整的分卷信息
    pub fn detect_split_archive(path: &Path) -> Result<Option<SplitArchiveInfo>> {
        if !Self::is_split_archive(path) {
            return Ok(None);
        }

        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("无效的文件名"))?;

        let parent_dir = path.parent()
            .ok_or_else(|| anyhow!("无法获取父目录"))?;

        // 根据格式类型检测
        if Self::is_zip_split(file_name) {
            Self::detect_zip_split(path, parent_dir)
        } else if Self::is_rar_split(file_name) {
            Self::detect_rar_split(path, parent_dir)
        } else if Self::is_7z_split(file_name) {
            Self::detect_7z_split(path, parent_dir)
        } else if Self::is_generic_numeric_split(file_name) {
            Self::detect_generic_numeric_split(path, parent_dir)
        } else if Self::is_generic_part_split(file_name) {
            Self::detect_generic_part_split(path, parent_dir)
        } else {
            Ok(None)
        }
    }

    /// 检测 ZIP 分卷 (.zip, .z01, .z02, ...)
    fn is_zip_split(file_name: &str) -> bool {
        let lower = file_name.to_lowercase();
        lower.ends_with(".zip") ||
        Regex::new(r"\.z\d{2}$").unwrap().is_match(&lower)
    }

    fn detect_zip_split(path: &Path, parent_dir: &Path) -> Result<Option<SplitArchiveInfo>> {
        let file_name = path.file_name().unwrap().to_str().unwrap();
        let lower = file_name.to_lowercase();

        // 提取基础名称
        let base_name = if lower.ends_with(".zip") {
            file_name.trim_end_matches(".zip").trim_end_matches(".ZIP").to_string()
        } else {
            // .z01, .z02 格式
            Regex::new(r"\.z\d{2}$").unwrap()
                .replace(&file_name, "")
                .to_string()
        };

        // 查找所有分卷
        let mut parts = Vec::new();

        // 主文件 .zip
        let main_file = parent_dir.join(format!("{}.zip", base_name));
        if main_file.exists() {
            parts.push(main_file);
        }

        // 分卷文件 .z01, .z02, ...
        for i in 1..=999 {
            let part_file = parent_dir.join(format!("{}.z{:02}", base_name, i));
            if part_file.exists() {
                parts.push(part_file);
            } else {
                break;
            }
        }

        if parts.is_empty() {
            return Ok(None);
        }

        let total_size = parts.iter()
            .filter_map(|p| fs::metadata(p).ok())
            .map(|m| m.len())
            .sum();

        Ok(Some(SplitArchiveInfo {
            format: SplitFormat::ZipSplit,
            base_name,
            first_part: parts.first().unwrap().clone(),
            total_parts: parts.len(),
            parts,
            total_size,
        }))
    }

    /// 检测 RAR 分卷
    fn is_rar_split(file_name: &str) -> bool {
        let lower = file_name.to_lowercase();
        // .rar, .r00, .r01 或 .part1.rar, .part2.rar
        lower.ends_with(".rar") ||
        Regex::new(r"\.r\d{2}$").unwrap().is_match(&lower) ||
        Regex::new(r"\.part\d+\.rar$").unwrap().is_match(&lower)
    }

    fn detect_rar_split(path: &Path, parent_dir: &Path) -> Result<Option<SplitArchiveInfo>> {
        let file_name = path.file_name().unwrap().to_str().unwrap();
        let lower = file_name.to_lowercase();

        let (base_name, is_part_format) = if let Some(caps) = Regex::new(r"(.+)\.part\d+\.rar$").unwrap().captures(&lower) {
            // .part1.rar 格式
            (caps.get(1).unwrap().as_str().to_string(), true)
        } else if lower.ends_with(".rar") {
            (file_name.trim_end_matches(".rar").trim_end_matches(".RAR").to_string(), false)
        } else {
            // .r00, .r01 格式
            (Regex::new(r"\.r\d{2}$").unwrap().replace(&file_name, "").to_string(), false)
        };

        let mut parts = Vec::new();

        if is_part_format {
            // .part1.rar, .part2.rar 格式
            for i in 1..=999 {
                let part_file = parent_dir.join(format!("{}.part{}.rar", base_name, i));
                if part_file.exists() {
                    parts.push(part_file);
                } else {
                    break;
                }
            }
        } else {
            // .rar, .r00, .r01 格式
            let main_file = parent_dir.join(format!("{}.rar", base_name));
            if main_file.exists() {
                parts.push(main_file);
            }

            for i in 0..=999 {
                let part_file = parent_dir.join(format!("{}.r{:02}", base_name, i));
                if part_file.exists() {
                    parts.push(part_file);
                } else {
                    break;
                }
            }
        }

        if parts.is_empty() {
            return Ok(None);
        }

        let total_size = parts.iter()
            .filter_map(|p| fs::metadata(p).ok())
            .map(|m| m.len())
            .sum();

        Ok(Some(SplitArchiveInfo {
            format: SplitFormat::RarSplit,
            base_name,
            first_part: parts.first().unwrap().clone(),
            total_parts: parts.len(),
            parts,
            total_size,
        }))
    }

    /// 检测 7Z 分卷 (.7z.001, .7z.002, ...)
    fn is_7z_split(file_name: &str) -> bool {
        let lower = file_name.to_lowercase();
        Regex::new(r"\.7z\.\d{3}$").unwrap().is_match(&lower)
    }

    fn detect_7z_split(path: &Path, parent_dir: &Path) -> Result<Option<SplitArchiveInfo>> {
        let file_name = path.file_name().unwrap().to_str().unwrap();

        // 提取基础名称
        let base_name = Regex::new(r"\.7z\.\d{3}$").unwrap()
            .replace(&file_name, "")
            .to_string();

        let mut parts = Vec::new();

        // 查找 .7z.001, .7z.002, ...
        for i in 1..=999 {
            let part_file = parent_dir.join(format!("{}.7z.{:03}", base_name, i));
            if part_file.exists() {
                parts.push(part_file);
            } else {
                break;
            }
        }

        if parts.is_empty() {
            return Ok(None);
        }

        let total_size = parts.iter()
            .filter_map(|p| fs::metadata(p).ok())
            .map(|m| m.len())
            .sum();

        Ok(Some(SplitArchiveInfo {
            format: SplitFormat::SevenZipSplit,
            base_name,
            first_part: parts.first().unwrap().clone(),
            total_parts: parts.len(),
            parts,
            total_size,
        }))
    }

    /// 检测通用数字分卷 (.001, .002, ...)
    fn is_generic_numeric_split(file_name: &str) -> bool {
        Regex::new(r"\.\d{3}$").unwrap().is_match(file_name)
    }

    fn detect_generic_numeric_split(path: &Path, parent_dir: &Path) -> Result<Option<SplitArchiveInfo>> {
        let file_name = path.file_name().unwrap().to_str().unwrap();

        let base_name = Regex::new(r"\.\d{3}$").unwrap()
            .replace(&file_name, "")
            .to_string();

        let mut parts = Vec::new();

        for i in 1..=999 {
            let part_file = parent_dir.join(format!("{}.{:03}", base_name, i));
            if part_file.exists() {
                parts.push(part_file);
            } else {
                break;
            }
        }

        if parts.is_empty() {
            return Ok(None);
        }

        let total_size = parts.iter()
            .filter_map(|p| fs::metadata(p).ok())
            .map(|m| m.len())
            .sum();

        Ok(Some(SplitArchiveInfo {
            format: SplitFormat::GenericNumeric,
            base_name,
            first_part: parts.first().unwrap().clone(),
            total_parts: parts.len(),
            parts,
            total_size,
        }))
    }

    /// 检测通用 part 分卷 (.part1, .part2, ...)
    fn is_generic_part_split(file_name: &str) -> bool {
        Regex::new(r"\.part\d+$").unwrap().is_match(&file_name.to_lowercase())
    }

    fn detect_generic_part_split(path: &Path, parent_dir: &Path) -> Result<Option<SplitArchiveInfo>> {
        let file_name = path.file_name().unwrap().to_str().unwrap();

        let base_name = Regex::new(r"\.part\d+$").unwrap()
            .replace(&file_name.to_lowercase(), "")
            .to_string();

        let mut parts = Vec::new();

        for i in 1..=999 {
            let part_file = parent_dir.join(format!("{}.part{}", base_name, i));
            if part_file.exists() {
                parts.push(part_file);
            } else {
                break;
            }
        }

        if parts.is_empty() {
            return Ok(None);
        }

        let total_size = parts.iter()
            .filter_map(|p| fs::metadata(p).ok())
            .map(|m| m.len())
            .sum();

        Ok(Some(SplitArchiveInfo {
            format: SplitFormat::GenericPart,
            base_name,
            first_part: parts.first().unwrap().clone(),
            total_parts: parts.len(),
            parts,
            total_size,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_zip_split() {
        assert!(SplitArchiveDetector::is_zip_split("archive.zip"));
        assert!(SplitArchiveDetector::is_zip_split("archive.z01"));
        assert!(SplitArchiveDetector::is_zip_split("archive.z99"));
        assert!(!SplitArchiveDetector::is_zip_split("archive.rar"));
    }

    #[test]
    fn test_is_rar_split() {
        assert!(SplitArchiveDetector::is_rar_split("archive.rar"));
        assert!(SplitArchiveDetector::is_rar_split("archive.r00"));
        assert!(SplitArchiveDetector::is_rar_split("archive.part1.rar"));
        assert!(!SplitArchiveDetector::is_rar_split("archive.zip"));
    }

    #[test]
    fn test_is_7z_split() {
        assert!(SplitArchiveDetector::is_7z_split("archive.7z.001"));
        assert!(SplitArchiveDetector::is_7z_split("archive.7z.999"));
        assert!(!SplitArchiveDetector::is_7z_split("archive.7z"));
    }
}
