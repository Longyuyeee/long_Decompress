use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::fs;
use walkdir::WalkDir;
use thiserror::Error;
use std::io::{Read, Write};

#[derive(Debug, Error)]
pub enum FileServiceError {
    #[error("文件不存在: {0}")]
    FileNotFound(String),

    #[error("目录不存在: {0}")]
    DirectoryNotFound(String),

    #[error("路径不是目录: {0}")]
    NotADirectory(String),

    #[error("路径不是文件: {0}")]
    NotAFile(String),

    #[error("权限不足: {0}")]
    PermissionDenied(String),

    #[error("磁盘空间不足: {0}")]
    DiskSpaceFull(String),

    #[error("文件已存在: {0}")]
    FileAlreadyExists(String),

    #[error("目录不为空: {0}")]
    DirectoryNotEmpty(String),

    #[error("无效的文件名: {0}")]
    InvalidFileName(String),

    #[error("无效的文件路径: {0}")]
    InvalidFilePath(String),

    #[error("文件操作超时: {0}")]
    OperationTimeout(String),

    #[error("文件系统错误: {0}")]
    FileSystemError(String),

    #[error("文件编码错误: {0}")]
    EncodingError(String),

    #[error("文件哈希计算失败: {0}")]
    HashCalculationFailed(String),

    #[error("文件搜索失败: {0}")]
    SearchFailed(String),

    #[error("文件比较失败: {0}")]
    ComparisonFailed(String),

    #[error("批量操作失败: {0}")]
    BatchOperationFailed(String),

    #[error("监控操作失败: {0}")]
    MonitoringFailed(String),

    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("未知错误: {0}")]
    Unknown(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub modified: Option<SystemTime>,
    pub created: Option<SystemTime>,
    pub extension: Option<String>,
    pub permissions: Option<String>,
    pub owner: Option<String>,
    pub group: Option<String>,
    pub inode: Option<u64>,
    pub device: Option<u64>,
    pub links: Option<u64>,
    pub blocks: Option<u64>,
    pub block_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileFilter {
    pub name_pattern: Option<String>,
    pub extension: Option<Vec<String>>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub is_dir: Option<bool>,
    pub is_file: Option<bool>,
    pub is_hidden: Option<bool>,
    pub is_symlink: Option<bool>,
    pub modified_after: Option<SystemTime>,
    pub modified_before: Option<SystemTime>,
    pub created_after: Option<SystemTime>,
    pub created_before: Option<SystemTime>,
    pub owner: Option<String>,
    pub group: Option<String>,
}

impl FileFilter {
    pub fn matches(&self, file: &FileInfo) -> bool {
        // 名称模式匹配
        if let Some(pattern) = &self.name_pattern {
            if !wildmatch::WildMatch::new(pattern).matches(&file.name) {
                return false;
            }
        }

        // 扩展名匹配
        if let Some(extensions) = &self.extension {
            if let Some(file_ext) = &file.extension {
                if !extensions.iter().any(|ext| ext.eq_ignore_ascii_case(file_ext)) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // 大小范围匹配
        if let Some(min_size) = self.min_size {
            if file.size < min_size {
                return false;
            }
        }
        if let Some(max_size) = self.max_size {
            if file.size > max_size {
                return false;
            }
        }

        // 文件类型匹配
        if let Some(is_dir) = self.is_dir {
            if file.is_dir != is_dir {
                return false;
            }
        }
        if let Some(is_file) = self.is_file {
            if file.is_dir == is_file {
                return false;
            }
        }

        // 隐藏文件匹配
        if let Some(is_hidden) = self.is_hidden {
            let is_hidden_file = file.name.starts_with('.');
            if is_hidden_file != is_hidden {
                return false;
            }
        }

        // 时间范围匹配
        if let Some(modified_after) = self.modified_after {
            if let Some(file_modified) = file.modified {
                if file_modified < modified_after {
                    return false;
                }
            } else {
                return false;
            }
        }
        if let Some(modified_before) = self.modified_before {
            if let Some(file_modified) = file.modified {
                if file_modified > modified_before {
                    return false;
                }
            } else {
                return false;
            }
        }

        // 创建时间范围匹配
        if let Some(created_after) = self.created_after {
            if let Some(file_created) = file.created {
                if file_created < created_after {
                    return false;
                }
            } else {
                return false;
            }
        }
        if let Some(created_before) = self.created_before {
            if let Some(file_created) = file.created {
                if file_created > created_before {
                    return false;
                }
            } else {
                return false;
            }
        }

        // 所有者和组匹配
        if let Some(owner) = &self.owner {
            if let Some(file_owner) = &file.owner {
                if file_owner != owner {
                    return false;
                }
            } else {
                return false;
            }
        }
        if let Some(group) = &self.group {
            if let Some(file_group) = &file.group {
                if file_group != group {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

impl Default for FileFilter {
    fn default() -> Self {
        Self {
            name_pattern: None,
            extension: None,
            min_size: None,
            max_size: None,
            is_dir: None,
            is_file: None,
            is_hidden: None,
            is_symlink: None,
            modified_after: None,
            modified_before: None,
            created_after: None,
            created_before: None,
            owner: None,
            group: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileServiceConfig {
    /// 最大递归深度
    pub max_recursion_depth: usize,

    /// 批量操作的最大文件数
    pub max_batch_files: usize,

    /// 文件操作超时时间（秒）
    pub operation_timeout_seconds: u64,

    /// 文件哈希计算的缓冲区大小（字节）
    pub hash_buffer_size: usize,

    /// 文件搜索的最大结果数
    pub max_search_results: usize,

    /// 是否启用文件监控
    pub enable_file_monitoring: bool,

    /// 文件监控的轮询间隔（毫秒）
    pub monitoring_poll_interval_ms: u64,

    /// 文件预览的最大大小（字节）
    pub max_preview_size: usize,

    /// 是否启用文件缓存
    pub enable_file_cache: bool,

    /// 文件缓存的最大大小（字节）
    pub max_cache_size: usize,

    /// 文件缓存的过期时间（秒）
    pub cache_expiration_seconds: u64,
}

impl Default for FileServiceConfig {
    fn default() -> Self {
        Self {
            max_recursion_depth: 10,
            max_batch_files: 1000,
            operation_timeout_seconds: 30,
            hash_buffer_size: 8192,
            max_search_results: 1000,
            enable_file_monitoring: false,
            monitoring_poll_interval_ms: 1000,
            max_preview_size: 1024 * 1024, // 1MB
            enable_file_cache: true,
            max_cache_size: 100 * 1024 * 1024, // 100MB
            cache_expiration_seconds: 300,
        }
    }
}

pub struct FileService {
    config: FileServiceConfig,
}

impl FileService {
    /// 创建新的文件服务实例
    pub fn new(config: FileServiceConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建文件服务实例
    pub fn default() -> Self {
        Self {
            config: FileServiceConfig::default(),
        }
    }

    /// 获取配置
    pub fn config(&self) -> &FileServiceConfig {
        &self.config
    }

    /// 更新配置
    pub fn update_config(&mut self, config: FileServiceConfig) {
        self.config = config;
    }

    /// 列出目录中的文件
    pub async fn list_files(&self, dir_path: &str, recursive: bool) -> Result<Vec<FileInfo>, FileServiceError> {
        let path = Path::new(dir_path);

        if !path.exists() {
            return Err(FileServiceError::DirectoryNotFound(dir_path.to_string()));
        }

        if !path.is_dir() {
            return Err(FileServiceError::NotADirectory(dir_path.to_string()));
        }

        let mut files = Vec::new();

        if recursive {
            // 递归遍历
            for entry in WalkDir::new(path)
                .max_depth(self.config.max_recursion_depth)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let file_info = self.get_file_info_from_entry(&entry)?;
                files.push(file_info);
            }
        } else {
            // 非递归遍历
            let mut entries = fs::read_dir(path).await
                .map_err(|e| FileServiceError::IoError(e))?;

            while let Some(entry) = entries.next_entry().await
                .map_err(|e| FileServiceError::IoError(e))? {
                let metadata = entry.metadata().await
                    .map_err(|e| FileServiceError::IoError(e))?;
                let file_info = self.get_file_info_from_metadata(&entry, &metadata)?;
                files.push(file_info);
            }
        }

        // 按名称排序
        files.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(files)
    }

    /// 列出目录中的文件（带分页）
    pub async fn list_files_paged(
        &self,
        dir_path: &str,
        recursive: bool,
        page: usize,
        page_size: usize,
    ) -> Result<(Vec<FileInfo>, usize), FileServiceError> {
        let all_files = self.list_files(dir_path, recursive).await?;
        let total_files = all_files.len();

        let start = page * page_size;
        let end = std::cmp::min(start + page_size, total_files);

        let paged_files = if start < total_files {
            all_files[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok((paged_files, total_files))
    }

    /// 列出目录中的文件（带过滤）
    pub async fn list_files_filtered(
        &self,
        dir_path: &str,
        recursive: bool,
        filter: FileFilter,
    ) -> Result<Vec<FileInfo>, FileServiceError> {
        let all_files = self.list_files(dir_path, recursive).await?;

        let filtered_files = all_files.into_iter()
            .filter(|file| filter.matches(file))
            .collect();

        Ok(filtered_files)
    }

    /// 获取文件信息
    pub async fn get_file_info(&self, file_path: &str) -> Result<FileInfo, FileServiceError> {
        let path = Path::new(file_path);

        if !path.exists() {
            return Err(FileServiceError::FileNotFound(file_path.to_string()));
        }

        let metadata = fs::metadata(path).await
            .map_err(|e| FileServiceError::IoError(e))?;

        // 获取详细的文件信息
        let (permissions, owner, group, inode, device, links, blocks, block_size) =
            self.get_detailed_file_info(path).await?;

        Ok(FileInfo {
            name: path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string(),
            path: path.to_string_lossy().to_string(),
            size: metadata.len(),
            is_dir: metadata.is_dir(),
            modified: metadata.modified().ok(),
            created: metadata.created().ok(),
            extension: path.extension()
                .and_then(|ext| ext.to_str())
                .map(|s| s.to_string()),
            permissions,
            owner,
            group,
            inode,
            device,
            links,
            blocks,
            block_size,
        })
    }

    /// 获取详细的文件信息
    async fn get_detailed_file_info(&self, path: &Path) -> Result<(
        Option<String>,
        Option<String>,
        Option<String>,
        Option<u64>,
        Option<u64>,
        Option<u64>,
        Option<u64>,
        Option<u64>,
    ), FileServiceError> {
        use std::os::unix::fs::MetadataExt;

        let metadata = std::fs::metadata(path)
            .map_err(|e| FileServiceError::IoError(e))?;

        // 权限
        let permissions = Some(format!("{:o}", metadata.permissions().mode() & 0o777));

        // 所有者和组（在Unix系统上）
        #[cfg(unix)]
        let (owner, group) = {
            use std::os::unix::fs::MetadataExt;
            let uid = metadata.uid();
            let gid = metadata.gid();

            // 这里可以添加从uid/gid到用户名/组名的转换
            // 暂时使用数字ID
            (Some(uid.to_string()), Some(gid.to_string()))
        };

        #[cfg(not(unix))]
        let (owner, group) = (None, None);

        // 文件系统信息
        let inode = Some(metadata.ino());
        let device = Some(metadata.dev());
        let links = Some(metadata.nlink());
        let blocks = Some(metadata.blocks());
        let block_size = Some(metadata.blksize());

        Ok((permissions, owner, group, inode, device, links, blocks, block_size))
    }

    /// 从walkdir条目获取文件信息
    fn get_file_info_from_entry(&self, entry: &walkdir::DirEntry) -> Result<FileInfo, FileServiceError> {
        let metadata = entry.metadata()
            .map_err(|e| FileServiceError::IoError(e))?;
        let path = entry.path();

        // 获取详细的文件信息
        let (permissions, owner, group, inode, device, links, blocks, block_size) =
            self.get_detailed_file_info(path).unwrap_or((None, None, None, None, None, None, None, None));

        Ok(FileInfo {
            name: path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string(),
            path: path.to_string_lossy().to_string(),
            size: metadata.len(),
            is_dir: metadata.is_dir(),
            modified: metadata.modified().ok(),
            created: metadata.created().ok(),
            extension: path.extension()
                .and_then(|ext| ext.to_str())
                .map(|s| s.to_string()),
            permissions,
            owner,
            group,
            inode,
            device,
            links,
            blocks,
            block_size,
        })
    }

    /// 从tokio目录条目获取文件信息
    fn get_file_info_from_metadata(
        &self,
        entry: &tokio::fs::DirEntry,
        metadata: &std::fs::Metadata,
    ) -> Result<FileInfo, FileServiceError> {
        let path = entry.path();

        // 获取详细的文件信息
        let (permissions, owner, group, inode, device, links, blocks, block_size) =
            self.get_detailed_file_info(&path).unwrap_or((None, None, None, None, None, None, None, None));

        Ok(FileInfo {
            name: entry.file_name()
                .to_string_lossy()
                .to_string(),
            path: path.to_string_lossy().to_string(),
            size: metadata.len(),
            is_dir: metadata.is_dir(),
            modified: metadata.modified().ok(),
            created: metadata.created().ok(),
            extension: path.extension()
                .and_then(|ext| ext.to_str())
                .map(|s| s.to_string()),
            permissions,
            owner,
            group,
            inode,
            device,
            links,
            blocks,
            block_size,
        })
    }

    /// 检查文件是否为压缩文件
    pub fn is_compressed_file(file_path: &str) -> bool {
        let path = Path::new(file_path);
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        matches!(
            extension.as_str(),
            "zip" | "rar" | "7z" | "tar" | "gz" | "tgz" | "bz2" | "tbz2" | "xz" | "txz" | "tar.gz" | "tar.bz2" | "tar.xz"
        )
    }

    /// 计算文件哈希值
    pub async fn calculate_file_hash(&self, file_path: &str, algorithm: HashAlgorithm) -> Result<String, FileServiceError> {
        use std::io::Read;
        use blake3::Hasher as Blake3Hasher;
        use sha2::{Sha256, Sha512, Digest};
        use md5::Md5;

        let path = Path::new(file_path);
        if !path.exists() {
            return Err(FileServiceError::FileNotFound(file_path.to_string()));
        }

        if !path.is_file() {
            return Err(FileServiceError::NotAFile(file_path.to_string()));
        }

        let mut file = std::fs::File::open(path)
            .map_err(|e| FileServiceError::IoError(e))?;

        let mut buffer = vec![0; self.config.hash_buffer_size];
        let mut hasher: Box<dyn std::io::Write> = match algorithm {
            HashAlgorithm::MD5 => Box::new(Md5::new()),
            HashAlgorithm::SHA256 => Box::new(Sha256::new()),
            HashAlgorithm::SHA512 => Box::new(Sha512::new()),
            HashAlgorithm::Blake3 => Box::new(Blake3Hasher::new()),
        };

        loop {
            let bytes_read = file.read(&mut buffer)
                .map_err(|e| FileServiceError::IoError(e))?;

            if bytes_read == 0 {
                break;
            }

            hasher.write_all(&buffer[..bytes_read])
                .map_err(|e| FileServiceError::IoError(e))?;
        }

        let hash = match algorithm {
            HashAlgorithm::MD5 => {
                let hasher = hasher.downcast::<Md5>().unwrap();
                format!("{:x}", hasher.finalize())
            }
            HashAlgorithm::SHA256 => {
                let hasher = hasher.downcast::<Sha256>().unwrap();
                format!("{:x}", hasher.finalize())
            }
            HashAlgorithm::SHA512 => {
                let hasher = hasher.downcast::<Sha512>().unwrap();
                format!("{:x}", hasher.finalize())
            }
            HashAlgorithm::Blake3 => {
                let hasher = hasher.downcast::<Blake3Hasher>().unwrap();
                format!("{}", hasher.finalize())
            }
        };

        Ok(hash)
    }

    /// 搜索文件
    pub async fn search_files(
        &self,
        root_dir: &str,
        search_query: &str,
        recursive: bool,
    ) -> Result<Vec<FileInfo>, FileServiceError> {
        let all_files = self.list_files(root_dir, recursive).await?;

        let results = all_files.into_iter()
            .filter(|file| {
                file.name.to_lowercase().contains(&search_query.to_lowercase()) ||
                file.path.to_lowercase().contains(&search_query.to_lowercase())
            })
            .take(self.config.max_search_results)
            .collect();

        Ok(results)
    }

    /// 批量复制文件
    pub async fn batch_copy_files(
        &self,
        source_files: &[String],
        destination_dir: &str,
    ) -> Result<BatchOperationResult, FileServiceError> {
        if source_files.len() > self.config.max_batch_files {
            return Err(FileServiceError::BatchOperationFailed(
                format!("文件数量超过最大限制: {}", self.config.max_batch_files)
            ));
        }

        let dest_path = Path::new(destination_dir);
        if !dest_path.exists() {
            fs::create_dir_all(dest_path).await
                .map_err(|e| FileServiceError::IoError(e))?;
        }

        let mut results = BatchOperationResult::new();
        let mut tasks = Vec::new();

        for source_file in source_files {
            let source_file = source_file.clone();
            let dest_dir = destination_dir.to_string();

            let task = tokio::spawn(async move {
                let source_path = Path::new(&source_file);
                let dest_path = Path::new(&dest_dir).join(
                    source_path.file_name().unwrap_or_default()
                );

                match Self::copy_file_internal(&source_file, &dest_path.to_string_lossy()).await {
                    Ok(_) => BatchOperationItem {
                        path: source_file,
                        success: true,
                        error: None,
                    },
                    Err(e) => BatchOperationItem {
                        path: source_file,
                        success: false,
                        error: Some(e.to_string()),
                    },
                }
            });

            tasks.push(task);
        }

        for task in tasks {
            let result = task.await
                .map_err(|e| FileServiceError::BatchOperationFailed(e.to_string()))?;
            results.add_item(result);
        }

        Ok(results)
    }

    /// 内部复制文件方法
    async fn copy_file_internal(src: &str, dst: &str) -> Result<(), FileServiceError> {
        let src_path = Path::new(src);
        let dst_path = Path::new(dst);

        if !src_path.exists() {
            return Err(FileServiceError::FileNotFound(src.to_string()));
        }

        // 确保目标目录存在
        if let Some(parent) = dst_path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| FileServiceError::IoError(e))?;
        }

        fs::copy(src_path, dst_path).await
            .map_err(|e| FileServiceError::IoError(e))?;

        Ok(())
    }

    /// 获取文件大小的人类可读格式
    pub fn format_file_size(size: u64) -> String {
        const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];

        if size == 0 {
            return "0 B".to_string();
        }

        let base = 1024_f64;
        let size_f64 = size as f64;
        let exponent = (size_f64.log10() / base.log10()).floor() as i32;
        let unit_index = exponent.min(5).max(0) as usize;

        let formatted_size = size_f64 / base.powi(exponent);

        format!("{:.2} {}", formatted_size, UNITS[unit_index])
    }

    /// 获取目录大小
    pub async fn get_directory_size(&self, dir_path: &str) -> Result<u64, FileServiceError> {
        let path = Path::new(dir_path);

        if !path.exists() {
            return Err(FileServiceError::DirectoryNotFound(dir_path.to_string()));
        }

        if !path.is_dir() {
            return Err(FileServiceError::NotADirectory(dir_path.to_string()));
        }

        let mut total_size = 0;

        for entry in WalkDir::new(path)
            .max_depth(self.config.max_recursion_depth)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                total_size += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }

        Ok(total_size)
    }

    /// 复制文件
    pub async fn copy_file(&self, src: &str, dst: &str) -> Result<(), FileServiceError> {
        Self::copy_file_internal(src, dst).await
    }

    /// 移动文件
    pub async fn move_file(&self, src: &str, dst: &str) -> Result<(), FileServiceError> {
        let src_path = Path::new(src);
        let dst_path = Path::new(dst);

        if !src_path.exists() {
            return Err(FileServiceError::FileNotFound(src.to_string()));
        }

        // 确保目标目录存在
        if let Some(parent) = dst_path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| FileServiceError::IoError(e))?;
        }

        fs::rename(src_path, dst_path).await
            .map_err(|e| FileServiceError::IoError(e))?;

        Ok(())
    }

    /// 删除文件或目录
    pub async fn delete_path(&self, path: &str) -> Result<(), FileServiceError> {
        let path = Path::new(path);

        if !path.exists() {
            return Err(FileServiceError::FileNotFound(path.to_string_lossy().to_string()));
        }

        if path.is_dir() {
            // 检查目录是否为空
            let mut entries = fs::read_dir(path).await
                .map_err(|e| FileServiceError::IoError(e))?;

            if entries.next_entry().await.map_err(|e| FileServiceError::IoError(e))?.is_some() {
                return Err(FileServiceError::DirectoryNotEmpty(path.to_string_lossy().to_string()));
            }

            fs::remove_dir(path).await
                .map_err(|e| FileServiceError::IoError(e))?;
        } else {
            fs::remove_file(path).await
                .map_err(|e| FileServiceError::IoError(e))?;
        }

        Ok(())
    }

    /// 强制删除文件或目录（包括非空目录）
    pub async fn delete_path_force(&self, path: &str) -> Result<(), FileServiceError> {
        let path = Path::new(path);

        if !path.exists() {
            return Err(FileServiceError::FileNotFound(path.to_string_lossy().to_string()));
        }

        if path.is_dir() {
            fs::remove_dir_all(path).await
                .map_err(|e| FileServiceError::IoError(e))?;
        } else {
            fs::remove_file(path).await
                .map_err(|e| FileServiceError::IoError(e))?;
        }

        Ok(())
    }

    /// 创建目录
    pub async fn create_directory(&self, dir_path: &str, recursive: bool) -> Result<(), FileServiceError> {
        let path = Path::new(dir_path);

        if path.exists() {
            return Err(FileServiceError::FileAlreadyExists(dir_path.to_string()));
        }

        if recursive {
            fs::create_dir_all(path).await
                .map_err(|e| FileServiceError::IoError(e))?;
        } else {
            fs::create_dir(path).await
                .map_err(|e| FileServiceError::IoError(e))?;
        }

        Ok(())
    }

    /// 重命名文件或目录
    pub async fn rename_path(&self, old_path: &str, new_path: &str) -> Result<(), FileServiceError> {
        let old_path = Path::new(old_path);
        let new_path = Path::new(new_path);

        if !old_path.exists() {
            return Err(FileServiceError::FileNotFound(old_path.to_string_lossy().to_string()));
        }

        if new_path.exists() {
            return Err(FileServiceError::FileAlreadyExists(new_path.to_string_lossy().to_string()));
        }

        // 确保新路径的父目录存在
        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| FileServiceError::IoError(e))?;
        }

        fs::rename(old_path, new_path).await
            .map_err(|e| FileServiceError::IoError(e))?;

        Ok(())
    }

    /// 检查文件权限
    pub async fn check_permissions(&self, path: &str, permission: FilePermission) -> Result<bool, FileServiceError> {
        let path = Path::new(path);

        if !path.exists() {
            return Err(FileServiceError::FileNotFound(path.to_string_lossy().to_string()));
        }

        let metadata = fs::metadata(path).await
            .map_err(|e| FileServiceError::IoError(e))?;

        let permissions = metadata.permissions();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = permissions.mode();

            match permission {
                FilePermission::Read => Ok(mode & 0o444 != 0),
                FilePermission::Write => Ok(mode & 0o222 != 0),
                FilePermission::Execute => Ok(mode & 0o111 != 0),
            }
        }

        #[cfg(not(unix))]
        {
            // 在非Unix系统上，使用简单检查
            match permission {
                FilePermission::Read => Ok(true), // 假设可读
                FilePermission::Write => Ok(!permissions.readonly()),
                FilePermission::Execute => Ok(false), // 在Windows上难以检查
            }
        }
    }

    /// 比较两个文件
    pub async fn compare_files(&self, file1: &str, file2: &str) -> Result<FileComparisonResult, FileServiceError> {
        let info1 = self.get_file_info(file1).await?;
        let info2 = self.get_file_info(file2).await?;

        let mut result = FileComparisonResult {
            files_identical: true,
            size_difference: info1.size as i64 - info2.size as i64,
            hash_difference: false,
            content_differences: Vec::new(),
            metadata_differences: Vec::new(),
        };

        // 检查大小差异
        if info1.size != info2.size {
            result.files_identical = false;
        }

        // 检查哈希差异
        let hash1 = self.calculate_file_hash(file1, HashAlgorithm::SHA256).await?;
        let hash2 = self.calculate_file_hash(file2, HashAlgorithm::SHA256).await?;

        if hash1 != hash2 {
            result.files_identical = false;
            result.hash_difference = true;
        }

        // 检查元数据差异
        if info1.modified != info2.modified {
            result.metadata_differences.push(MetadataDifference {
                field: "modified".to_string(),
                expected: format!("{:?}", info1.modified),
                actual: format!("{:?}", info2.modified),
            });
            result.files_identical = false;
        }

        if info1.created != info2.created {
            result.metadata_differences.push(MetadataDifference {
                field: "created".to_string(),
                expected: format!("{:?}", info1.created),
                actual: format!("{:?}", info2.modified),
            });
            result.files_identical = false;
        }

        Ok(result)
    }

    /// 获取文件预览
    pub async fn get_file_preview(&self, file_path: &str) -> Result<FilePreview, FileServiceError> {
        let path = Path::new(file_path);

        if !path.exists() {
            return Err(FileServiceError::FileNotFound(file_path.to_string()));
        }

        if !path.is_file() {
            return Err(FileServiceError::NotAFile(file_path.to_string()));
        }

        let metadata = fs::metadata(path).await
            .map_err(|e| FileServiceError::IoError(e))?;

        if metadata.len() > self.config.max_preview_size as u64 {
            // 文件太大，只预览前一部分
            let mut file = std::fs::File::open(path)
                .map_err(|e| FileServiceError::IoError(e))?;

            let mut buffer = vec![0; self.config.max_preview_size];
            let bytes_read = file.read(&mut buffer)
                .map_err(|e| FileServiceError::IoError(e))?;

            let content = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();

            Ok(FilePreview {
                content,
                encoding: "utf-8".to_string(),
                truncated: true,
                line_count: content.lines().count(),
                byte_count: bytes_read,
            })
        } else {
            // 读取整个文件
            let content = fs::read_to_string(path).await
                .map_err(|e| FileServiceError::IoError(e))?;

            Ok(FilePreview {
                content,
                encoding: "utf-8".to_string(),
                truncated: false,
                line_count: content.lines().count(),
                byte_count: content.len(),
            })
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilePermission {
    Read,
    Write,
    Execute,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HashAlgorithm {
    MD5,
    SHA256,
    SHA512,
    Blake3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationResult {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub items: Vec<BatchOperationItem>,
}

impl BatchOperationResult {
    pub fn new() -> Self {
        Self {
            total: 0,
            successful: 0,
            failed: 0,
            items: Vec::new(),
        }
    }

    pub fn add_item(&mut self, item: BatchOperationItem) {
        self.total += 1;
        if item.success {
            self.successful += 1;
        } else {
            self.failed += 1;
        }
        self.items.push(item);
    }

    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.successful as f64 / self.total as f64 * 100.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationItem {
    pub path: String,
    pub success: bool,
    pub error: Option<String>,
}

/// 文件比较结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileComparisonResult {
    pub files_identical: bool,
    pub size_difference: i64,
    pub hash_difference: bool,
    pub content_differences: Vec<ContentDifference>,
    pub metadata_differences: Vec<MetadataDifference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentDifference {
    pub offset: u64,
    pub expected: u8,
    pub actual: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataDifference {
    pub field: String,
    pub expected: String,
    pub actual: String,
}

/// 文件预览
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePreview {
    pub content: String,
    pub encoding: String,
    pub truncated: bool,
    pub line_count: usize,
    pub byte_count: usize,
}

/// 文件系统监控事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemEvent {
    pub event_type: FileSystemEventType,
    pub path: String,
    pub timestamp: SystemTime,
    pub metadata: Option<FileInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileSystemEventType {
    Created,
    Modified,
    Deleted,
    Renamed,
    Accessed,
}