use crate::models::compression::CompressionOptions;
use crate::services::io_buffer_pool::{IOBufferPool, IOBufferPoolConfig, IOBufferHandle};
use crate::services::parallel_extraction::{ParallelExtractor, copy_file_with_buffer_pool};
use crate::services::password_attempt_service::{PasswordAttemptService, PasswordAttemptStrategy, PasswordAttemptResult};
use crate::services::rar_support::{RarSupportService, RarError};
use crate::services::split_compression::{SplitCompressionService, SplitCompressionResult};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;
use zip::ZipArchive;
use std::io::{Read, Write};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::{Archive, Builder};
use std::fs::File;
use sevenz_rust;
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum CompressionError {
    #[error("文件不存在: {0}")]
    FileNotFound(String),

    #[error("目录不存在: {0}")]
    DirectoryNotFound(String),

    #[error("无效的文件名: {0}")]
    InvalidFileName(String),

    #[error("无效的文件路径: {0}")]
    InvalidFilePath(String),

    #[error("不支持的文件格式: {0}")]
    UnsupportedFormat(String),

    #[error("压缩失败: {0}")]
    CompressionFailed(String),

    #[error("解压失败: {0}")]
    ExtractionFailed(String),

    #[error("密码错误或缺失")]
    PasswordError,

    #[error("磁盘空间不足")]
    DiskSpaceFull,

    #[error("权限不足: {0}")]
    PermissionDenied(String),

    #[error("操作超时")]
    OperationTimeout,

    #[error("文件损坏: {0}")]
    FileCorrupted(String),

    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("ZIP库错误: {0}")]
    ZipError(#[from] zip::result::ZipError),

    #[error("文件已存在: {0}")]
    FileAlreadyExists(String),

    #[error("未知错误: {0}")]
    Unknown(String),
}
use rayon::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// 压缩进度回调
pub type ProgressCallback = Box<dyn Fn(CompressionProgress) + Send + Sync>;

/// 压缩进度信息
#[derive(Debug, Clone)]
pub struct CompressionProgress {
    pub current_file: String,
    pub current_file_index: usize,
    pub total_files: usize,
    pub current_file_progress: f32,
    pub total_progress: f32,
    pub bytes_processed: u64,
    pub total_bytes: u64,
}

/// 压缩服务配置
#[derive(Debug, Clone)]
pub struct CompressionServiceConfig {
    /// 最大并发文件数
    pub max_concurrent_files: usize,

    /// 缓冲区大小（字节）- 已弃用，使用缓冲区池
    pub buffer_size: usize,

    /// 操作超时时间（秒）
    pub operation_timeout_seconds: u64,

    /// 是否启用进度回调
    pub enable_progress_callback: bool,

    /// 是否验证压缩文件
    pub verify_compressed_file: bool,

    /// 是否保留文件权限
    pub preserve_permissions: bool,

    /// 是否保留文件时间戳
    pub preserve_timestamps: bool,

    /// 缓冲区池配置
    pub buffer_pool_config: IOBufferPoolConfig,
}

impl Default for CompressionServiceConfig {
    fn default() -> Self {
        Self {
            max_concurrent_files: 4,
            buffer_size: 8192, // 保持向后兼容
            operation_timeout_seconds: 300,
            enable_progress_callback: true,
            verify_compressed_file: true,
            preserve_permissions: true,
            preserve_timestamps: true,
            buffer_pool_config: IOBufferPoolConfig::default(),
        }
    }
}

pub struct CompressionService {
    config: CompressionServiceConfig,
    buffer_pool: IOBufferPool,
    cancellation_flag: Arc<AtomicBool>,
}

impl CompressionService {
    /// 创建新的压缩服务实例
    pub fn new(config: CompressionServiceConfig) -> Self {
        let buffer_pool = IOBufferPool::new(config.buffer_pool_config.clone());
        let cancellation_flag = Arc::new(AtomicBool::new(false));
        Self { config, buffer_pool, cancellation_flag }
    }

    /// 使用默认配置创建压缩服务实例
    pub fn default() -> Self {
        let config = CompressionServiceConfig::default();
        let buffer_pool = IOBufferPool::new(config.buffer_pool_config.clone());
        let cancellation_flag = Arc::new(AtomicBool::new(false));
        Self { config, buffer_pool, cancellation_flag }
    }

    /// 获取配置
    pub fn config(&self) -> &CompressionServiceConfig {
        &self.config
    }

    /// 更新配置
    pub fn update_config(&mut self, config: CompressionServiceConfig) {
        self.config = config;
    }

    /// 请求取消当前操作
    pub fn request_cancellation(&self) {
        self.cancellation_flag.store(true, Ordering::SeqCst);
    }

    /// 重置取消标志
    pub fn reset_cancellation(&self) {
        self.cancellation_flag.store(false, Ordering::SeqCst);
    }

    /// 检查是否已请求取消
    pub fn is_cancellation_requested(&self) -> bool {
        self.cancellation_flag.load(Ordering::SeqCst)
    }

    /// 检查取消并返回错误（如果已请求取消）
    fn check_cancellation(&self) -> Result<(), CompressionError> {
        if self.is_cancellation_requested() {
            Err(CompressionError::OperationTimeout)
        } else {
            Ok(())
        }
    }
    /// 解压压缩文件
    ///
    /// # 参数
    /// * `file_path` - 要解压的文件路径
    /// * `output_dir` - 输出目录（可选，默认为文件所在目录）
    /// * `password` - 解压密码（可选，用于加密的压缩文件）
    ///
    /// # 返回值
    /// * `Ok(String)` - 成功时返回解压输出目录的路径
    /// * `Err(anyhow::Error)` - 失败时返回错误信息
    ///
    /// # 支持的格式
    /// * ZIP (.zip) - 支持AES-256加密
    /// * RAR (.rar) - 需要系统安装unrar或7z工具
    /// * 7Z (.7z) - 支持密码保护
    /// * TAR (.tar)
    /// * GZIP (.gz, .tar.gz)
    /// * BZIP2 (.bz2, .tar.bz2)
    /// * XZ (.xz, .tar.xz)
    ///
    /// # 示例
    /// ```
    /// use crate::services::compression_service::CompressionService;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     // 解压普通ZIP文件
    ///     let output = CompressionService::extract(
    ///         "archive.zip",
    ///         Some("output_dir"),
    ///         None,
    ///     ).await?;
    ///
    ///     // 解压加密ZIP文件
    ///     let output = CompressionService::extract(
    ///         "encrypted.zip",
    ///         Some("output_dir"),
    ///         Some("mypassword"),
    ///     ).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn extract(
        file_path: &str,
        output_dir: Option<&str>,
        password: Option<&str>,
    ) -> Result<String> {
        let path = Path::new(file_path);
        let output_path = if let Some(dir) = output_dir {
            PathBuf::from(dir)
        } else {
            path.parent()
                .unwrap_or_else(|| Path::new("."))
                .join(path.file_stem().unwrap_or_default())
        };

        // 创建输出目录
        fs::create_dir_all(&output_path)
            .await
            .context("创建输出目录失败")?;

        // 根据文件扩展名选择解压方法
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "zip" => Self::extract_zip_optimized(path, &output_path, password).await,
            "rar" => Self::extract_rar(path, &output_path, password).await,
            "gz" | "tgz" | "tar.gz" => Self::extract_tar_gz(path, &output_path).await,
            "tar" => Self::extract_tar(path, &output_path).await,
            "bz2" | "tbz2" | "tar.bz2" => Self::extract_tar_bz2(path, &output_path).await,
            "xz" | "txz" | "tar.xz" => Self::extract_tar_xz(path, &output_path).await,
            "7z" => Self::extract_7z(path, &output_path, password).await,
            _ => Err(anyhow::anyhow!("不支持的文件格式: {}", extension)),
        }?;

        Ok(output_path.to_string_lossy().to_string())
    }

    /// 解压文件（带密码尝试，集成密码本系统）
    ///
    /// 此方法会自动从密码本中尝试密码来解压加密的ZIP文件。
    /// 支持多种密码尝试策略，可以智能地从密码本中选择密码进行尝试。
    ///
    /// # 参数
    /// * `file_path` - 要解压的ZIP文件路径
    /// * `output_dir` - 输出目录（可选）
    /// * `password_attempt_service` - 密码尝试服务实例
    /// * `strategy` - 密码尝试策略
    ///
    /// # 密码尝试策略
    /// * `All` - 尝试密码本中的所有密码
    /// * `Recent(limit)` - 尝试最近使用的N个密码
    /// * `Category(category)` - 尝试特定分类的密码
    /// * `NameMatch(pattern)` - 尝试名称匹配模式的密码
    /// * `Custom(passwords)` - 尝试自定义密码列表
    ///
    /// # 返回值
    /// * `Ok(PasswordAttemptResult)` - 包含尝试结果、使用的密码、尝试次数等信息
    /// * `Err(anyhow::Error)` - 解压过程发生错误
    ///
    /// # 示例
    /// ```
    /// use crate::services::compression_service::CompressionService;
    /// use crate::services::password_attempt_service::{PasswordAttemptService, PasswordAttemptStrategy};
    /// use crate::services::password_query_service::PasswordQueryService;
    /// use std::sync::Arc;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let query_service = Arc::new(PasswordQueryService::new());
    ///     let attempt_service = PasswordAttemptService::new(query_service);
    ///
    ///     // 尝试所有密码
    ///     let result = CompressionService::extract_with_password_attempt(
    ///         "encrypted.zip",
    ///         Some("output_dir"),
    ///         &attempt_service,
    ///         PasswordAttemptStrategy::All,
    ///     ).await?;
    ///
    ///     if result.success {
    ///         println!("解压成功! 使用的密码: {:?}", result.password);
    ///         println!("尝试了 {} 次，共 {} 个密码", result.attempts, result.total_passwords);
    ///     } else {
    ///         println!("解压失败: {:?}", result.error_message);
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn extract_with_password_attempt(
        file_path: &str,
        output_dir: Option<&str>,
        password_attempt_service: &PasswordAttemptService,
        strategy: PasswordAttemptStrategy,
    ) -> Result<PasswordAttemptResult> {
        let path = Path::new(file_path);

        // 验证文件存在
        if !path.exists() {
            return Ok(PasswordAttemptResult {
                success: false,
                password: None,
                attempts: 0,
                total_passwords: 0,
                matched_entry: None,
                error_message: Some(format!("文件不存在: {}", file_path)),
            });
        }

        // 验证是ZIP文件
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        if extension != "zip" {
            return Ok(PasswordAttemptResult {
                success: false,
                password: None,
                attempts: 0,
                total_passwords: 0,
                matched_entry: None,
                error_message: Some(format!("不支持的文件格式: {}, 仅支持ZIP文件", extension)),
            });
        }

        // 准备输出目录
        let output_path = if let Some(dir) = output_dir {
            PathBuf::from(dir)
        } else {
            path.parent()
                .unwrap_or_else(|| Path::new("."))
                .join(path.file_stem().unwrap_or_default())
        };

        // 创建输出目录
        if let Err(e) = fs::create_dir_all(&output_path).await {
            return Ok(PasswordAttemptResult {
                success: false,
                password: None,
                attempts: 0,
                total_passwords: 0,
                matched_entry: None,
                error_message: Some(format!("创建输出目录失败: {}", e)),
            });
        }

        // 使用密码尝试服务解压
        let result = password_attempt_service
            .attempt_extract_with_passwords(
                file_path,
                &output_path.to_string_lossy(),
                strategy,
            )
            .await?;

        Ok(result)
    }

    /// 压缩文件（兼容旧版本）
    pub async fn compress(
        files: &[String],
        output_path: &str,
        options: CompressionOptions,
    ) -> Result<(), CompressionError> {
        let service = CompressionService::default();
        service.compress_enhanced(files, output_path, options, None).await
    }

    /// 压缩文件（增强版）
    pub async fn compress_enhanced(
        &self,
        files: &[String],
        output_path: &str,
        options: CompressionOptions,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<(), CompressionError> {
        let output_path = Path::new(output_path);
        let extension = output_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "zip" => self.compress_zip_enhanced(files, output_path, options, progress_callback).await,
            "gz" | "tgz" | "tar.gz" => self.compress_tar_gz_enhanced(files, output_path, progress_callback).await,
            "tar" => self.compress_tar_enhanced(files, output_path, progress_callback).await,
            "7z" => self.compress_7z_enhanced(files, output_path, options, progress_callback).await,
            _ => Err(CompressionError::UnsupportedFormat(extension)),
        }
    }

    /// 解压ZIP文件（优化版，支持并行处理）
    async fn extract_zip_optimized(
        zip_path: &Path,
        output_dir: &Path,
        password: Option<&str>,
    ) -> Result<()> {
        // 对于小文件，使用原始版本
        let metadata = std::fs::metadata(zip_path).context("获取文件元数据失败")?;
        if metadata.len() < 10 * 1024 * 1024 { // 小于10MB
            return Self::extract_zip_original(zip_path, output_dir, password).await;
        }

        // 对于大文件，使用并行版本
        Self::extract_zip_parallel(zip_path, output_dir, password).await
    }

    /// 解压ZIP文件（原始版本，保持向后兼容）
    async fn extract_zip_original(
        zip_path: &Path,
        output_dir: &Path,
        password: Option<&str>,
    ) -> Result<()> {
        Self::extract_zip_with_password_check(zip_path, output_dir, password).await?;
        Ok(())
    }

    /// 解压ZIP文件并检查密码（返回是否成功）
    pub async fn extract_zip_with_password_check(
        zip_path: &Path,
        output_dir: &Path,
        password: Option<&str>,
    ) -> Result<bool> {
        let file = File::open(zip_path).context("打开ZIP文件失败")?;
        let mut archive = ZipArchive::new(file).context("读取ZIP归档失败")?;

        for i in 0..archive.len() {
            // 尝试使用密码打开文件（如果提供）
            let mut zip_file = if let Some(pwd) = password {
                match archive.by_index_decrypt(i, pwd.as_bytes()) {
                    Ok(decrypted_file) => decrypted_file,
                    Err(zip::result::ZipError::InvalidPassword) => {
                        // 密码错误，返回false
                        return Ok(false);
                    }
                    Err(e) => {
                        // 如果解密失败但不是密码错误，可能文件未加密
                        // 回退到无密码打开
                        archive.by_index(i).context("获取ZIP文件条目失败")?
                    }
                }
            } else {
                archive.by_index(i).context("获取ZIP文件条目失败")?
            };

            let outpath = output_dir.join(zip_file.mangled_name());

            if zip_file.name().ends_with('/') {
                // 创建目录
                fs::create_dir_all(&outpath)
                    .await
                    .context("创建目录失败")?;
            } else {
                // 创建父目录
                if let Some(parent) = outpath.parent() {
                    fs::create_dir_all(parent)
                        .await
                        .context("创建父目录失败")?;
                }

                // 写入文件
                let mut outfile = File::create(&outpath).context("创建输出文件失败")?;
                std::io::copy(&mut zip_file, &mut outfile).context("复制文件内容失败")?;
            }
        }

        // 所有文件都成功解压，返回true
        Ok(true)
    }

    /// 解压tar.gz文件
    async fn extract_tar_gz(tar_gz_path: &Path, output_dir: &Path) -> Result<()> {
        let file = File::open(tar_gz_path).context("打开tar.gz文件失败")?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);

        archive.unpack(output_dir).context("解压tar.gz文件失败")?;
        Ok(())
    }

    /// 解压tar文件
    async fn extract_tar(tar_path: &Path, output_dir: &Path) -> Result<()> {
        let file = File::open(tar_path).context("打开tar文件失败")?;
        let mut archive = Archive::new(file);

        archive.unpack(output_dir).context("解压tar文件失败")?;
        Ok(())
    }

    /// 解压tar.bz2文件
    async fn extract_tar_bz2(tar_bz2_path: &Path, output_dir: &Path) -> Result<()> {
        let file = File::open(tar_bz2_path).context("打开tar.bz2文件失败")?;
        let decoder = bzip2::read::BzDecoder::new(file);
        let mut archive = Archive::new(decoder);

        archive.unpack(output_dir).context("解压tar.bz2文件失败")?;
        Ok(())
    }

    /// 解压tar.xz文件
    async fn extract_tar_xz(tar_xz_path: &Path, output_dir: &Path) -> Result<()> {
        let file = File::open(tar_xz_path).context("打开tar.xz文件失败")?;
        let decoder = xz2::read::XzDecoder::new(file);
        let mut archive = Archive::new(decoder);

        archive.unpack(output_dir).context("解压tar.xz文件失败")?;
        Ok(())
    }

    /// 压缩为ZIP文件（增强版）
    pub async fn compress_zip_enhanced(
        &self,
        files: &[String],
        output_path: &Path,
        options: CompressionOptions,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<(), CompressionError> {
        log::info!("开始压缩ZIP文件: {:?} -> {:?}", files, output_path);

        // 检查是否需要分卷压缩
        if options.split_size.is_some() {
            log::debug!("使用分卷压缩模式");
            return self.compress_zip_with_split(files, output_path, options, progress_callback).await;
        }

        // 验证输入文件
        self.validate_input_files(files).await?;

        // 检查输出路径
        if output_path.exists() {
            if !options.overwrite_existing {
                return Err(CompressionError::FileAlreadyExists(
                    output_path.to_string_lossy().to_string(),
                ));
            }
            log::warn!("覆盖已存在的文件: {:?}", output_path);
        }

        // 确保输出目录存在
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| CompressionError::IoError(e))?;
        }

        // 计算总大小（用于进度报告）
        let total_size = self.calculate_total_size(files).await?;
        log::debug!("总文件大小: {} 字节", total_size);

        // 检查磁盘空间（估计需要2倍空间用于压缩过程）
        let estimated_required_space = total_size * 2;
        if let Some(parent) = output_path.parent() {
            if let Err(e) = self.check_disk_space(parent, estimated_required_space) {
                log::warn!("磁盘空间检查失败: {}, 继续执行", e);
                // 不阻止操作，只是记录警告
            }
        }

        // 创建ZIP文件
        let file = File::create(output_path)
            .map_err(|e| CompressionError::IoError(e))?;
        let mut zip = zip::ZipWriter::new(file);

        // 配置压缩选项
        let zip_options = self.create_zip_options(&options)?;

        let mut bytes_processed = 0u64;
        let mut current_file_index = 0;

        // 处理每个文件
        for file_path in files {
            current_file_index += 1;
            let path = Path::new(file_path);

            if path.is_file() {
                // 处理单个文件
                self.add_file_to_zip(
                    &mut zip,
                    path,
                    &zip_options,
                    current_file_index,
                    files.len(),
                    &mut bytes_processed,
                    total_size,
                    &progress_callback,
                ).await?;
            } else if path.is_dir() {
                // 处理目录（递归）
                self.add_directory_to_zip(
                    &mut zip,
                    path,
                    &zip_options,
                    current_file_index,
                    files.len(),
                    &mut bytes_processed,
                    total_size,
                    &progress_callback,
                ).await?;
            }
        }

        // 完成ZIP文件
        zip.finish()
            .map_err(|e| CompressionError::ZipError(e))?;

        // 验证压缩文件（如果启用）
        if self.config.verify_compressed_file {
            self.verify_zip_file(output_path, files.len()).await?;
        }

        log::info!("ZIP压缩完成: {:?}", output_path);
        Ok(())
    }

    /// 使用分卷压缩ZIP文件
    async fn compress_zip_with_split(
        &self,
        files: &[String],
        output_path: &Path,
        options: CompressionOptions,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<(), CompressionError> {
        log::info!("开始分卷压缩ZIP文件: {:?} -> {:?}", files, output_path);

        // 创建分卷压缩服务
        let split_service = SplitCompressionService::new();

        // 调用分卷压缩服务
        match split_service.compress_to_split_zips(files, output_path, options).await {
            Ok(result) => {
                log::info!("分卷压缩成功，创建了 {} 个分卷文件", result.part_count);

                // 调用进度回调（如果提供）
                if let Some(callback) = progress_callback {
                    callback(100.0, "分卷压缩完成".to_string()).await;
                }

                Ok(())
            }
            Err(e) => {
                log::error!("分卷压缩失败: {}", e);
                Err(CompressionError::CompressionFailed(format!("分卷压缩失败: {}", e)))
            }
        }
    }

    /// 验证输入文件
    async fn validate_input_files(&self, files: &[String]) -> Result<(), CompressionError> {
        if files.is_empty() {
            return Err(CompressionError::InvalidFilePath("文件列表为空".to_string()));
        }

        for file_path in files {
            let path = Path::new(file_path);
            if !path.exists() {
                return Err(CompressionError::FileNotFound(file_path.to_string()));
            }
        }

        Ok(())
    }

    /// 计算总文件大小
    async fn calculate_total_size(&self, files: &[String]) -> Result<u64, CompressionError> {
        let mut total_size = 0u64;

        for file_path in files {
            let path = Path::new(file_path);

            if path.is_file() {
                let metadata = tokio::fs::metadata(path).await
                    .map_err(|e| CompressionError::IoError(e))?;
                total_size += metadata.len();
            } else if path.is_dir() {
                // 递归计算目录大小
                for entry in WalkDir::new(path) {
                    let entry = entry.map_err(|e| CompressionError::IoError(e.into()))?;
                    if entry.file_type().is_file() {
                        total_size += entry.metadata()
                            .map(|m| m.len())
                            .unwrap_or(0);
                    }
                }
            }
        }

        Ok(total_size)
    }

    /// 创建ZIP选项
    fn create_zip_options(&self, options: &CompressionOptions) -> Result<zip::write::FileOptions, CompressionError> {
        let mut zip_options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(options.compression_level as i32);

        // 添加密码（如果提供）- zip 1.2支持AES加密
        if let Some(password) = &options.password {
            zip_options = zip_options.with_aes_encryption(
                zip::AesMode::Aes256,
                password.as_bytes(),
            )
            .map_err(|e| CompressionError::ZipError(e))?;
        }

        // 设置其他选项
        if self.config.preserve_permissions {
            // 在Unix系统上保留权限
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                zip_options = zip_options.unix_permissions(0o644);
            }
        }

        Ok(zip_options)
    }

    /// 添加文件到ZIP
    async fn add_file_to_zip(
        &self,
        zip: &mut zip::ZipWriter<File>,
        file_path: &Path,
        options: &zip::write::FileOptions,
        current_index: usize,
        total_files: usize,
        bytes_processed: &mut u64,
        total_bytes: u64,
        progress_callback: &Option<ProgressCallback>,
    ) -> Result<(), CompressionError> {
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| CompressionError::InvalidFileName(
                file_path.to_string_lossy().to_string()
            ))?;

        log::debug!("添加文件到ZIP: {}", file_name);

        // 开始写入文件
        zip.start_file(file_name, *options)
            .map_err(|e| CompressionError::ZipError(e))?;

        // 打开源文件
        let mut source_file = File::open(file_path)
            .map_err(|e| CompressionError::IoError(e))?;

        // 获取文件大小
        let file_size = source_file.metadata()
            .map_err(|e| CompressionError::IoError(e))?
            .len();

        // 根据文件大小获取推荐的缓冲区大小
        let recommended_buffer_size = self.buffer_pool.recommend_buffer_size(file_size);

        // 从缓冲区池获取缓冲区
        let mut buffer_handle = self.buffer_pool.acquire(Some(recommended_buffer_size)).await;
        let buffer = buffer_handle.buffer_mut();

        let mut file_bytes_processed = 0u64;

        loop {
            // 检查取消
            self.check_cancellation()?;

            let bytes_read = source_file.read(buffer.as_mut_slice())
                .map_err(|e| CompressionError::IoError(e))?;

            if bytes_read == 0 {
                break;
            }

            buffer.set_size(bytes_read);
            zip.write_all(buffer.as_slice())
                .map_err(|e| CompressionError::IoError(e))?;

            file_bytes_processed += bytes_read as u64;
            *bytes_processed += bytes_read as u64;

            // 更新进度
            if let Some(callback) = progress_callback {
                let progress = CompressionProgress {
                    current_file: file_name.to_string(),
                    current_file_index: current_index,
                    total_files,
                    current_file_progress: if file_size > 0 {
                        (file_bytes_processed as f32 / file_size as f32) * 100.0
                    } else {
                        0.0
                    },
                    total_progress: if total_bytes > 0 {
                        (*bytes_processed as f32 / total_bytes as f32) * 100.0
                    } else {
                        0.0
                    },
                    bytes_processed: *bytes_processed,
                    total_bytes,
                };
                callback(progress);
            }
        }

        Ok(())
    }

    /// 添加目录到ZIP
    async fn add_directory_to_zip(
        &self,
        zip: &mut zip::ZipWriter<File>,
        dir_path: &Path,
        options: &zip::write::FileOptions,
        current_index: usize,
        total_files: usize,
        bytes_processed: &mut u64,
        total_bytes: u64,
        progress_callback: &Option<ProgressCallback>,
    ) -> Result<(), CompressionError> {
        let dir_name = dir_path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| CompressionError::InvalidFileName(
                dir_path.to_string_lossy().to_string()
            ))?;

        log::debug!("添加目录到ZIP: {}", dir_name);

        // 添加目录条目
        let dir_entry_name = format!("{}/", dir_name);
        zip.add_directory(&dir_entry_name, *options)
            .map_err(|e| CompressionError::ZipError(e))?;

        // 递归处理目录内容
        for entry in WalkDir::new(dir_path) {
            let entry = entry.map_err(|e| CompressionError::IoError(e.into()))?;
            let entry_path = entry.path();

            if entry.file_type().is_file() {
                // 计算相对路径
                let relative_path = entry_path.strip_prefix(dir_path)
                    .map_err(|_| CompressionError::InvalidFilePath(
                        entry_path.to_string_lossy().to_string()
                    ))?;

                let zip_entry_name = format!("{}/{}", dir_name, relative_path.to_string_lossy());

                // 开始写入文件
                zip.start_file(&zip_entry_name, *options)
                    .map_err(|e| CompressionError::ZipError(e))?;

                // 复制文件内容
                let mut source_file = File::open(entry_path)
                    .map_err(|e| CompressionError::IoError(e))?;

                let file_size = source_file.metadata()
                    .map_err(|e| CompressionError::IoError(e))?
                    .len();

                // 根据文件大小获取推荐的缓冲区大小
                let recommended_buffer_size = self.buffer_pool.recommend_buffer_size(file_size);

                // 从缓冲区池获取缓冲区
                let mut buffer_handle = self.buffer_pool.acquire(Some(recommended_buffer_size)).await;
                let buffer = buffer_handle.buffer_mut();

                let mut file_bytes_processed = 0u64;

                loop {
                    let bytes_read = source_file.read(buffer.as_mut_slice())
                        .map_err(|e| CompressionError::IoError(e))?;

                    if bytes_read == 0 {
                        break;
                    }

                    buffer.set_size(bytes_read);
                    zip.write_all(buffer.as_slice())
                        .map_err(|e| CompressionError::IoError(e))?;

                    file_bytes_processed += bytes_read as u64;
                    *bytes_processed += bytes_read as u64;

                    // 更新进度
                    if let Some(callback) = progress_callback {
                        let progress = CompressionProgress {
                            current_file: entry_path.to_string_lossy().to_string(),
                            current_file_index: current_index,
                            total_files,
                            current_file_progress: if file_size > 0 {
                                (file_bytes_processed as f32 / file_size as f32) * 100.0
                            } else {
                                0.0
                            },
                            total_progress: if total_bytes > 0 {
                                (*bytes_processed as f32 / total_bytes as f32) * 100.0
                            } else {
                                0.0
                            },
                            bytes_processed: *bytes_processed,
                            total_bytes,
                        };
                        callback(progress);
                    }
                }
            }
        }

        Ok(())
    }

    /// 检查磁盘空间
    fn check_disk_space(&self, path: &Path, required_size: u64) -> Result<(), CompressionError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let metadata = std::fs::metadata(path)
                .map_err(|e| CompressionError::IoError(e))?;
            let available_space = metadata.blocks() * metadata.blksize() as u64;

            if available_space < required_size * 2 { // 保留2倍空间作为缓冲
                return Err(CompressionError::DiskSpaceFull);
            }
        }

        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            let metadata = std::fs::metadata(path)
                .map_err(|e| CompressionError::IoError(e))?;
            // Windows上需要更复杂的磁盘空间检查
            // 暂时跳过，在实际应用中应该实现
        }

        Ok(())
    }

    /// 验证ZIP文件
    async fn verify_zip_file(&self, zip_path: &Path, expected_file_count: usize) -> Result<(), CompressionError> {
        log::debug!("验证ZIP文件: {:?}", zip_path);

        let file = File::open(zip_path)
            .map_err(|e| CompressionError::IoError(e))?;

        let archive = zip::ZipArchive::new(file)
            .map_err(|e| CompressionError::ZipError(e))?;

        let actual_file_count = archive.len();

        if actual_file_count < expected_file_count {
            return Err(CompressionError::CompressionFailed(format!(
                "ZIP文件验证失败: 期望 {} 个文件，实际 {} 个文件",
                expected_file_count, actual_file_count
            )));
        }

        // 检查文件完整性
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| CompressionError::ZipError(e))?;

            // 尝试读取文件内容以验证完整性
            let mut buffer = vec![0u8; 1024];
            while let Ok(bytes_read) = file.read(&mut buffer) {
                if bytes_read == 0 {
                    break;
                }
            }
        }

        log::debug!("ZIP文件验证通过");
        Ok(())
    }

    /// 压缩为tar.gz文件
    async fn compress_tar_gz(files: &[String], output_path: &Path) -> Result<()> {
        let tar_gz_file = File::create(output_path).context("创建tar.gz文件失败")?;
        let encoder = GzEncoder::new(tar_gz_file, Compression::default());
        let mut tar = Builder::new(encoder);

        for file_path in files {
            let path = Path::new(file_path);
            if !path.exists() {
                return Err(anyhow::anyhow!("文件不存在: {}", file_path));
            }

            if path.is_file() {
                let mut file = File::open(path).context("打开源文件失败")?;
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| anyhow::anyhow!("无效的文件名: {}", file_path))?;

                tar.append_file(name, &mut file).context("添加文件到tar失败")?;
            } else if path.is_dir() {
                tar.append_dir_all(path.file_name().unwrap_or_default(), path)
                    .context("添加目录到tar失败")?;
            }
        }

        tar.finish().context("完成tar文件写入失败")?;
        Ok(())
    }

    /// 压缩为tar文件
    async fn compress_tar(files: &[String], output_path: &Path) -> Result<()> {
        let tar_file = File::create(output_path).context("创建tar文件失败")?;
        let mut tar = Builder::new(tar_file);

        for file_path in files {
            let path = Path::new(file_path);
            if !path.exists() {
                return Err(anyhow::anyhow!("文件不存在: {}", file_path));
            }

            if path.is_file() {
                let mut file = File::open(path).context("打开源文件失败")?;
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| anyhow::anyhow!("无效的文件名: {}", file_path))?;

                tar.append_file(name, &mut file).context("添加文件到tar失败")?;
            } else if path.is_dir() {
                tar.append_dir_all(path.file_name().unwrap_or_default(), path)
                    .context("添加目录到tar失败")?;
            }
        }

        tar.finish().context("完成tar文件写入失败")?;
        Ok(())
    }

    /// 压缩为7z文件（增强版）
    async fn compress_7z_enhanced(
        &self,
        files: &[String],
        output_path: &Path,
        options: CompressionOptions,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<(), CompressionError> {
        log::info!("开始压缩7z文件: {:?} -> {:?}", files, output_path);

        // 验证输入文件
        self.validate_input_files(files).await?;

        // 检查输出路径
        if output_path.exists() {
            if !options.overwrite_existing {
                return Err(CompressionError::FileAlreadyExists(
                    output_path.to_string_lossy().to_string(),
                ));
            }
            log::warn!("覆盖已存在的文件: {:?}", output_path);
        }

        // 确保输出目录存在
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| CompressionError::IoError(e))?;
        }

        // 计算总大小（用于进度报告）
        let total_size = self.calculate_total_size(files).await?;
        log::debug!("总文件大小: {} 字节", total_size);

        // 构建压缩配置
        let mut config = sevenz_rust::compress::CompressConfig::default();

        // 设置压缩级别（1-9，9为最高压缩）
        config.compression_level = options.compression_level.clamp(1, 9) as u32;

        // 设置压缩方法（LZMA2是7z的默认方法）
        config.compression_method = sevenz_rust::CompressionMethod::LZMA2;

        // 设置多线程（如果支持）
        config.multithread = options.use_multithreading;

        // 创建7z归档写入器
        let mut archive = sevenz_rust::SevenZWriter::create(output_path, config)
            .map_err(|e| CompressionError::CompressionFailed(e.to_string()))?;

        let mut bytes_processed = 0u64;
        let mut current_file_index = 0;

        // 处理每个文件
        for file_path in files {
            current_file_index += 1;
            let path = Path::new(file_path);

            // 更新进度
            if let Some(callback) = &progress_callback {
                let progress = CompressionProgress {
                    current_file: file_path.clone(),
                    current_file_index,
                    total_files: files.len(),
                    current_file_progress: 0.0,
                    total_progress: (bytes_processed as f32 / total_size as f32) * 100.0,
                    bytes_processed,
                    total_bytes: total_size,
                };
                callback(progress);
            }

            if path.is_file() {
                // 添加单个文件
                archive.push_archive_entry_by_path(
                    path,
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown"),
                    false,
                )
                .map_err(|e| CompressionError::CompressionFailed(e.to_string()))?;

                // 更新已处理的字节数
                if let Ok(metadata) = std::fs::metadata(path) {
                    bytes_processed += metadata.len();
                }
            } else if path.is_dir() {
                // 添加目录及其内容
                for entry in walkdir::WalkDir::new(path) {
                    let entry = entry.map_err(|e| CompressionError::IoError(e.into()))?;
                    let entry_path = entry.path();

                    if entry_path.is_file() {
                        let relative_path = entry_path.strip_prefix(path)
                            .map_err(|e| CompressionError::InvalidFilePath(e.to_string()))?;

                        archive.push_archive_entry_by_path(
                            entry_path,
                            relative_path.to_string_lossy().as_ref(),
                            false,
                        )
                        .map_err(|e| CompressionError::CompressionFailed(e.to_string()))?;

                        // 更新已处理的字节数
                        if let Ok(metadata) = std::fs::metadata(entry_path) {
                            bytes_processed += metadata.len();
                        }

                        // 更新进度
                        if let Some(callback) = &progress_callback {
                            let progress = CompressionProgress {
                                current_file: entry_path.to_string_lossy().to_string(),
                                current_file_index,
                                total_files: files.len(),
                                current_file_progress: 0.0,
                                total_progress: (bytes_processed as f32 / total_size as f32) * 100.0,
                                bytes_processed,
                                total_bytes: total_size,
                            };
                            callback(progress);
                        }
                    }
                }
            }
        }

        // 完成压缩
        archive.finish()
            .map_err(|e| CompressionError::CompressionFailed(e.to_string()))?;

        // 最终进度更新
        if let Some(callback) = &progress_callback {
            let progress = CompressionProgress {
                current_file: "完成".to_string(),
                current_file_index: files.len(),
                total_files: files.len(),
                current_file_progress: 100.0,
                total_progress: 100.0,
                bytes_processed: total_size,
                total_bytes: total_size,
            };
            callback(progress);
        }

        log::info!("7z文件压缩完成: {:?}", output_path);
        Ok(())
    }

    /// 压缩为tar.gz文件（增强版）
    async fn compress_tar_gz_enhanced(
        &self,
        files: &[String],
        output_path: &Path,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<(), CompressionError> {
        log::info!("开始压缩tar.gz文件: {:?} -> {:?}", files, output_path);

        // 验证输入文件
        self.validate_input_files(files).await?;

        // 检查输出路径
        if output_path.exists() {
            log::warn!("覆盖已存在的文件: {:?}", output_path);
        }

        // 确保输出目录存在
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| CompressionError::IoError(e))?;
        }

        // 计算总大小（用于进度报告）
        let total_size = self.calculate_total_size(files).await?;
        log::debug!("总文件大小: {} 字节", total_size);

        // 创建tar.gz文件
        let tar_gz_file = File::create(output_path)
            .map_err(|e| CompressionError::IoError(e))?;
        let encoder = GzEncoder::new(tar_gz_file, Compression::default());
        let mut tar = Builder::new(encoder);

        let mut bytes_processed = 0u64;
        let mut current_file_index = 0;

        // 处理每个文件
        for file_path in files {
            current_file_index += 1;
            let path = Path::new(file_path);

            if path.is_file() {
                // 处理单个文件
                self.add_file_to_tar(
                    &mut tar,
                    path,
                    current_file_index,
                    files.len(),
                    &mut bytes_processed,
                    total_size,
                    &progress_callback,
                ).await?;
            } else if path.is_dir() {
                // 处理目录（递归）
                self.add_directory_to_tar(
                    &mut tar,
                    path,
                    current_file_index,
                    files.len(),
                    &mut bytes_processed,
                    total_size,
                    &progress_callback,
                ).await?;
            }
        }

        // 完成tar文件
        tar.finish()
            .map_err(|e| CompressionError::IoError(e))?;

        log::info!("tar.gz压缩完成: {:?}", output_path);
        Ok(())
    }

    /// 压缩为tar文件（增强版）
    async fn compress_tar_enhanced(
        &self,
        files: &[String],
        output_path: &Path,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<(), CompressionError> {
        log::info!("开始压缩tar文件: {:?} -> {:?}", files, output_path);

        // 验证输入文件
        self.validate_input_files(files).await?;

        // 检查输出路径
        if output_path.exists() {
            log::warn!("覆盖已存在的文件: {:?}", output_path);
        }

        // 确保输出目录存在
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| CompressionError::IoError(e))?;
        }

        // 计算总大小（用于进度报告）
        let total_size = self.calculate_total_size(files).await?;
        log::debug!("总文件大小: {} 字节", total_size);

        // 创建tar文件
        let tar_file = File::create(output_path)
            .map_err(|e| CompressionError::IoError(e))?;
        let mut tar = Builder::new(tar_file);

        let mut bytes_processed = 0u64;
        let mut current_file_index = 0;

        // 处理每个文件
        for file_path in files {
            current_file_index += 1;
            let path = Path::new(file_path);

            if path.is_file() {
                // 处理单个文件
                self.add_file_to_tar(
                    &mut tar,
                    path,
                    current_file_index,
                    files.len(),
                    &mut bytes_processed,
                    total_size,
                    &progress_callback,
                ).await?;
            } else if path.is_dir() {
                // 处理目录（递归）
                self.add_directory_to_tar(
                    &mut tar,
                    path,
                    current_file_index,
                    files.len(),
                    &mut bytes_processed,
                    total_size,
                    &progress_callback,
                ).await?;
            }
        }

        // 完成tar文件
        tar.finish()
            .map_err(|e| CompressionError::IoError(e))?;

        log::info!("tar压缩完成: {:?}", output_path);
        Ok(())
    }

    /// 添加文件到tar
    async fn add_file_to_tar(
        &self,
        tar: &mut Builder<impl Write>,
        file_path: &Path,
        current_index: usize,
        total_files: usize,
        bytes_processed: &mut u64,
        total_bytes: u64,
        progress_callback: &Option<ProgressCallback>,
    ) -> Result<(), CompressionError> {
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| CompressionError::InvalidFileName(
                file_path.to_string_lossy().to_string()
            ))?;

        log::debug!("添加文件到tar: {}", file_name);

        // 打开源文件
        let mut source_file = File::open(file_path)
            .map_err(|e| CompressionError::IoError(e))?;

        // 获取文件大小
        let file_size = source_file.metadata()
            .map_err(|e| CompressionError::IoError(e))?
            .len();

        // 添加文件到tar
        tar.append_file(file_name, &mut source_file)
            .map_err(|e| CompressionError::IoError(e))?;

        *bytes_processed += file_size;

        // 更新进度
        if let Some(callback) = progress_callback {
            let progress = CompressionProgress {
                current_file: file_name.to_string(),
                current_file_index: current_index,
                total_files,
                current_file_progress: 100.0, // tar添加文件是原子操作
                total_progress: if total_bytes > 0 {
                    (*bytes_processed as f32 / total_bytes as f32) * 100.0
                } else {
                    0.0
                },
                bytes_processed: *bytes_processed,
                total_bytes,
            };
            callback(progress);
        }

        Ok(())
    }

    /// 添加目录到tar
    async fn add_directory_to_tar(
        &self,
        tar: &mut Builder<impl Write>,
        dir_path: &Path,
        current_index: usize,
        total_files: usize,
        bytes_processed: &mut u64,
        total_bytes: u64,
        progress_callback: &Option<ProgressCallback>,
    ) -> Result<(), CompressionError> {
        let dir_name = dir_path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| CompressionError::InvalidFileName(
                dir_path.to_string_lossy().to_string()
            ))?;

        log::debug!("添加目录到tar: {}", dir_name);

        // 添加目录到tar
        tar.append_dir_all(dir_name, dir_path)
            .map_err(|e| CompressionError::IoError(e))?;

        // 计算目录大小
        let mut dir_size = 0u64;
        for entry in WalkDir::new(dir_path) {
            let entry = entry.map_err(|e| CompressionError::IoError(e.into()))?;
            if entry.file_type().is_file() {
                dir_size += entry.metadata()
                    .map(|m| m.len())
                    .unwrap_or(0);
            }
        }

        *bytes_processed += dir_size;

        // 更新进度
        if let Some(callback) = progress_callback {
            let progress = CompressionProgress {
                current_file: dir_name.to_string(),
                current_file_index: current_index,
                total_files,
                current_file_progress: 100.0, // tar添加目录是原子操作
                total_progress: if total_bytes > 0 {
                    (*bytes_processed as f32 / total_bytes as f32) * 100.0
                } else {
                    0.0
                },
                bytes_processed: *bytes_processed,
                total_bytes,
            };
            callback(progress);
        }

        Ok(())
    }

    /// 并行解压ZIP文件
    async fn extract_zip_parallel(
        zip_path: &Path,
        output_dir: &Path,
        password: Option<&str>,
    ) -> Result<()> {
        // 创建缓冲区池
        let buffer_pool = IOBufferPool::default();

        // 创建并行解压器
        let extractor = ParallelExtractor::new(buffer_pool, 4);

        // 使用并行解压器解压文件
        extractor.extract_zip_parallel(zip_path, output_dir, password).await
    }

    /// 独立处理单个ZIP条目（线程安全）
    fn process_zip_entry_independent(
        zip_path: &Path,
        entry: &ZipEntryInfo,
        output_dir: &Path,
    ) -> Result<()> {
        // 每个线程独立打开ZIP文件
        let file = File::open(zip_path)
            .context(format!("打开ZIP文件失败: {:?}", zip_path))?;
        let mut archive = ZipArchive::new(file)
            .context("读取ZIP归档失败")?;

        // 获取文件条目
        let mut file = archive.by_index(entry.index)
            .context(format!("获取ZIP文件条目失败: {}", entry.name))?;

        // 创建父目录
        if let Some(parent) = entry.outpath.parent() {
            std::fs::create_dir_all(parent)
                .context(format!("创建父目录失败: {:?}", parent))?;
        }

        // 写入文件（使用优化缓冲区）
        const BUFFER_SIZE: usize = 64 * 1024; // 64KB缓冲区
        let mut outfile = File::create(&entry.outpath)
            .context(format!("创建输出文件失败: {:?}", entry.outpath))?;

        let mut buffer = vec![0u8; BUFFER_SIZE];
        loop {
            let bytes_read = file.read(&mut buffer)
                .context("读取ZIP文件内容失败")?;
            if bytes_read == 0 {
                break;
            }
            outfile.write_all(&buffer[..bytes_read])
                .context("写入文件内容失败")?;
        }

        Ok(())
    }
}

/// ZIP条目信息
struct ZipEntryInfo {
    index: usize,
    name: String,
    outpath: PathBuf,
    is_dir: bool,
    compressed_size: u64,
    uncompressed_size: u64,
}

impl CompressionService {
    /// 解压RAR文件
    async fn extract_rar(
        rar_path: &Path,
        output_dir: &Path,
        password: Option<&str>,
    ) -> Result<()> {
        log::info!("开始解压RAR文件: {:?}", rar_path);

        // 创建RAR支持服务
        let rar_service = RarSupportService::new();

        // 调用RAR服务解压
        match rar_service.extract_rar(rar_path, output_dir, password).await {
            Ok(_) => Ok(()),
            Err(RarError::PasswordError) => {
                Err(anyhow::anyhow!("RAR密码错误或缺失"))
            }
            Err(RarError::ToolNotInstalled) => {
                Err(anyhow::anyhow!("系统未安装RAR解压工具，请安装unrar或7z"))
            }
            Err(e) => {
                Err(anyhow::anyhow!("RAR解压失败: {}", e))
            }
        }
    }

    /// 解压7z文件
    async fn extract_7z(
        sevenz_path: &Path,
        output_dir: &Path,
        password: Option<&str>,
    ) -> Result<()> {
        log::info!("开始解压7z文件: {:?}", sevenz_path);

        // 确保输出目录存在
        if !output_dir.exists() {
            std::fs::create_dir_all(output_dir)
                .context("创建输出目录失败")?;
        }

        // 构建解压配置
        let mut config = sevenz_rust::decompress::DecompressConfig::default();

        // 设置密码（如果有）
        if let Some(pwd) = password {
            config.password = Some(pwd.to_string());
        }

        // 设置输出目录
        config.output_path = output_dir.to_path_buf();

        // 解压7z文件
        sevenz_rust::decompress_file_with_extract_fn(
            sevenz_path,
            config,
            |entry, reader| {
                let entry_path = entry.name();
                log::debug!("解压文件: {}", entry_path);

                // 这里可以添加进度回调或其他自定义逻辑
                Ok(true) // 继续解压
            },
        )
        .map_err(|e| {
            // 提供更详细的错误信息
            let error_msg = match e.to_string().to_lowercase().as_str() {
                s if s.contains("password") || s.contains("wrong password") => {
                    format!("7z文件解压失败：密码错误或缺失 - {}", sevenz_path.display())
                }
                s if s.contains("not found") || s.contains("no such file") => {
                    format!("7z文件解压失败：文件不存在或无法访问 - {}", sevenz_path.display())
                }
                s if s.contains("corrupt") || s.contains("damaged") => {
                    format!("7z文件解压失败：文件可能已损坏 - {}", sevenz_path.display())
                }
                s if s.contains("permission") || s.contains("access denied") => {
                    format!("7z文件解压失败：权限不足 - {}", sevenz_path.display())
                }
                s if s.contains("unsupported") || s.contains("not supported") => {
                    format!("7z文件解压失败：不支持的压缩格式或版本 - {}", sevenz_path.display())
                }
                _ => {
                    format!("7z文件解压失败：{} - {}", sevenz_path.display(), e)
                }
            };
            anyhow::anyhow!(error_msg)
        })?;

        log::info!("7z文件解压完成: {:?}", sevenz_path);
        Ok(())
    }

    /// 压缩为7z文件（基础实现）
    async fn compress_to_7z(
        files: &[String],
        output_path: &Path,
        options: CompressionOptions,
    ) -> Result<()> {
        log::info!("开始压缩为7z文件: {:?}", output_path);

        // 确保输出目录存在
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .context("创建输出目录失败")?;
            }
        }

        // 构建压缩配置
        let mut config = sevenz_rust::compress::CompressConfig::default();

        // 设置压缩级别（1-9，9为最高压缩）
        config.compression_level = options.compression_level.clamp(1, 9) as u32;

        // 设置压缩方法（LZMA2是7z的默认方法）
        config.compression_method = sevenz_rust::CompressionMethod::LZMA2;

        // 设置多线程（如果支持）
        config.multithread = options.use_multithreading;

        // 创建7z归档写入器
        let mut archive = sevenz_rust::SevenZWriter::create(output_path, config)
            .context("创建7z归档失败")?;

        // 添加文件到归档
        for file_path in files {
            let path = Path::new(file_path);
            if !path.exists() {
                return Err(anyhow::anyhow!("文件不存在: {}", file_path));
            }

            if path.is_file() {
                // 添加单个文件
                archive.push_archive_entry_by_path(
                    path,
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown"),
                    false,
                )
                .context(format!("添加文件到7z归档失败: {}", file_path))?;
            } else if path.is_dir() {
                // 添加目录及其内容
                for entry in walkdir::WalkDir::new(path) {
                    let entry = entry.context("遍历目录失败")?;
                    let entry_path = entry.path();

                    if entry_path.is_file() {
                        let relative_path = entry_path.strip_prefix(path)
                            .context("计算相对路径失败")?;

                        archive.push_archive_entry_by_path(
                            entry_path,
                            relative_path.to_string_lossy().as_ref(),
                            false,
                        )
                        .context(format!("添加文件到7z归档失败: {:?}", entry_path))?;
                    }
                }
            }
        }

        // 完成压缩
        archive.finish().context("完成7z压缩失败")?;

        log::info!("7z文件压缩完成: {:?}", output_path);
        Ok(())
    }
}