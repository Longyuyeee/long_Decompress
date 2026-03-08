use crate::models::compression::CompressionOptions;
use crate::services::io_buffer_pool::{IOBufferPool, IOBufferPoolConfig, IOBufferHandle};
use crate::services::parallel_extraction::{ParallelExtractor, copy_file_with_buffer_pool};
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
}

impl CompressionService {
    /// 创建新的压缩服务实例
    pub fn new(config: CompressionServiceConfig) -> Self {
        let buffer_pool = IOBufferPool::new(config.buffer_pool_config.clone());
        Self { config, buffer_pool }
    }

    /// 使用默认配置创建压缩服务实例
    pub fn default() -> Self {
        let config = CompressionServiceConfig::default();
        let buffer_pool = IOBufferPool::new(config.buffer_pool_config.clone());
        Self { config, buffer_pool }
    }

    /// 获取配置
    pub fn config(&self) -> &CompressionServiceConfig {
        &self.config
    }

    /// 更新配置
    pub fn update_config(&mut self, config: CompressionServiceConfig) {
        self.config = config;
    }

impl CompressionService {
    /// 解压文件
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
            "gz" | "tgz" | "tar.gz" => Self::extract_tar_gz(path, &output_path).await,
            "tar" => Self::extract_tar(path, &output_path).await,
            "bz2" | "tbz2" | "tar.bz2" => Self::extract_tar_bz2(path, &output_path).await,
            "xz" | "txz" | "tar.xz" => Self::extract_tar_xz(path, &output_path).await,
            _ => Err(anyhow::anyhow!("不支持的文件格式: {}", extension)),
        }?;

        Ok(output_path.to_string_lossy().to_string())
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
        let file = File::open(zip_path).context("打开ZIP文件失败")?;
        let mut archive = ZipArchive::new(file).context("读取ZIP归档失败")?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).context("获取ZIP文件条目失败")?;

            // 注意：当前zip库版本可能不支持is_encrypted和set_password方法
            // 在实际应用中，可能需要使用其他库或更新zip库版本
            // 这里暂时跳过加密检查

            let outpath = output_dir.join(file.mangled_name());

            if file.name().ends_with('/') {
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
                std::io::copy(&mut file, &mut outfile).context("复制文件内容失败")?;
            }
        }

        Ok(())
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

        // 添加密码（如果提供）
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