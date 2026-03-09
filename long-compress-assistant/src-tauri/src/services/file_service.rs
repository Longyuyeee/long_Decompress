use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum FileServiceError {
    #[error("文件未找到: {0}")]
    FileNotFound(String),
    #[error("不是一个文件: {0}")]
    NotAFile(String),
    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),
    #[error("未知错误: {0}")]
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    pub extension: Option<String>,
    pub modified: Option<SystemTime>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HashAlgorithm {
    MD5,
    SHA256,
    SHA512,
    Blake3,
}

#[derive(Debug, Clone)]
pub struct FileServiceConfig {
    pub hash_buffer_size: usize,
}

impl Default for FileServiceConfig {
    fn default() -> Self {
        Self {
            hash_buffer_size: 8192,
        }
    }
}

#[derive(Clone)]
pub struct FileService {
    config: FileServiceConfig,
}

impl FileService {
    pub fn new(config: FileServiceConfig) -> Self {
        Self { config }
    }

    pub async fn list_files(&self, path: &str, recursive: bool) -> Result<Vec<FileInfo>, FileServiceError> {
        let mut files = Vec::new();
        let walker = WalkDir::new(path);
        
        for entry in walker {
            let entry = entry.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            files.push(self.get_file_info_sync(entry.path())?);
            if !recursive && files.len() > 0 && entry.depth() > 0 {
                // 简化的非递归实现
            }
        }
        Ok(files)
    }

    pub async fn get_file_info(&self, path: &str) -> Result<FileInfo, FileServiceError> {
        self.get_file_info_sync(Path::new(path))
    }

    fn get_file_info_sync(&self, path: &Path) -> Result<FileInfo, FileServiceError> {
        let metadata = std::fs::metadata(path)?;
        Ok(FileInfo {
            path: path.to_string_lossy().to_string(),
            name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
            size: metadata.len(),
            is_dir: metadata.is_dir(),
            extension: path.extension().map(|s| s.to_string_lossy().to_string()),
            modified: metadata.modified().ok(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationItem {
    pub source: String,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationResult {
    pub success: bool,
    pub message: String,
    pub items: Vec<BatchItemResult>,
}

impl BatchOperationResult {
    pub fn successful_count(&self) -> usize {
        self.items.iter().filter(|i| i.success).count()
    }
    pub fn failed_count(&self) -> usize {
        self.items.iter().filter(|i| !i.success).count()
    }
    pub fn skipped_count(&self) -> usize {
        0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItemResult {
    pub source: String,
    pub success: bool,
    pub error: Option<String>,
}
