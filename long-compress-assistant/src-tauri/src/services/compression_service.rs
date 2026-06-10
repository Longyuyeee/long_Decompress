use crate::models::compression::{CompressionOptions, DecompressOptions, TaskLog, TaskLogSeverity};
use anyhow::Result;
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use zip::{ZipArchive, write::FileOptions, CompressionMethod};
use std::io::{BufRead, BufReader, Read, Write};
use std::fs::File;
use sevenz_rust;
use thiserror::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Window;
use chrono::Utc;
use serde::Serialize;

use crate::utils::io_utils::ProgressReader;
use crate::services::io_buffer_pool::IOBufferPool;
use crate::services::rar_support::RarSupportService;
use crate::services::universal_engine::UniversalCliEngine;
use crate::services::archive_engine::ArchiveEngine;
use crate::services::password_query_service::PasswordQueryService;

#[derive(Debug, Error)]
pub enum CompressionError {
    #[error("文件不存在: {0}")]
    FileNotFound(String),
    #[error("压缩失败: {0}")]
    CompressionFailed(String),
    #[error("解压失败: {0}")]
    ExtractionFailed(String),
    #[error("需要输入密码才能解压")]
    PasswordRequired,
    #[error("提供的密码不正确")]
    InvalidPassword,
    #[error("密码错误")]
    PasswordError,
    #[error("不支持的加密算法或压缩方法")]
    UnsupportedEncryption,
    #[error("目标磁盘空间不足")]
    DiskFull,
    #[error("批量解压部分完成，部分文件失败")]
    PartialSuccess(Vec<String>),
    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),
    #[error("任务已取消")]
    Cancelled,
}

#[derive(Debug, Clone, Default)]
pub struct CompressionServiceConfig {
    pub max_concurrent_files: usize,
    pub buffer_size: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArchiveFormat {
    Zip,
    SevenZip,
    Rar,
    Tar,
    Gzip,
    Bzip2,
    Xz,
    Iso,
    /// 7z CLI 支持的杂项格式（CAB, LZH, ARJ, DMG, WIM, VHD, IMG 等）
    Universal,
    Unknown,
}

impl ArchiveFormat {
    pub fn from_magic(header: &[u8]) -> Self {
        if header.len() >= 4 && &header[0..4] == b"PK\x03\x04" {
            return ArchiveFormat::Zip;
        }
        if header.len() >= 6 && &header[0..6] == b"7z\xBC\xAF\x27\x1C" {
            return ArchiveFormat::SevenZip;
        }
        if header.len() >= 7 && &header[0..7] == b"Rar!\x1a\x07\x00" {
            return ArchiveFormat::Rar;
        }
        if header.len() >= 8 && &header[0..8] == b"Rar!\x1a\x07\x01\x00" {
            return ArchiveFormat::Rar;
        }
        if header.len() >= 2 && &header[0..2] == b"\x1F\x8B" {
            return ArchiveFormat::Gzip;
        }
        if header.len() >= 3 && &header[0..3] == b"BZh" {
            return ArchiveFormat::Bzip2;
        }
        if header.len() >= 6 && &header[0..6] == b"\xFD7zXZ\x00" {
            return ArchiveFormat::Xz;
        }
        ArchiveFormat::Unknown
    }

    pub fn supports_password(&self) -> bool {
        match self {
            ArchiveFormat::Zip | ArchiveFormat::SevenZip | ArchiveFormat::Rar => true,
            _ => false,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct TaskProgress {
    pub task_id: String,
    pub stage: Option<String>, // "Pre-checking" | "Extracting" | "Finalizing"
    pub current_password: Option<String>,
    pub progress: f32,
    pub speed: Option<String>,
    pub current_file: Option<String>,
    pub processed_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Clone, Serialize)]
pub struct PasswordRequiredPayload {
    pub task_id: String,
    pub file_path: String,
    pub file_name: String,
    pub format: String,
}

#[derive(Clone, Serialize)]
pub struct RarCompressionSupport {
    pub available: bool,
    pub encoder_path: Option<String>,
    pub message: String,
}

use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct CompressionService {
    pub config: CompressionServiceConfig,
    pub cancellation_flag: Arc<AtomicBool>,
    pub buffer_pool: Arc<IOBufferPool>,
    pub rar_service: Arc<RarSupportService>,
    pub universal_engine: Arc<UniversalCliEngine>,
    pub password_query_service: Arc<PasswordQueryService>,
    pub semaphore: Arc<Semaphore>,
}

impl CompressionService {
    /// 自动凑齐所有默认依赖并创建实例 (推荐在 Command 层使用)
    pub async fn new_with_defaults() -> Self {
        let pool = match crate::database::connection::get_connection().await {
            Ok(conn) => conn.pool().clone(),
            Err(e) => {
                log::warn!("无法获取数据库连接，密码本功能将不启用: {}", e);
                sqlx::pool::Pool::<sqlx::Sqlite>::connect_lazy("sqlite::memory:").unwrap_or_else(|_| {
                    sqlx::pool::Pool::<sqlx::Sqlite>::connect_lazy("sqlite::memory:").unwrap()
                })
            }
        };

        // 动态计算数据目录，确保与 main.rs 逻辑一致
        let mut data_dir = std::env::current_dir().unwrap_or_default();
        if data_dir.ends_with("src-tauri") {
            data_dir.pop();
        }
        let data_dir = data_dir.join(".password_book_data");

        let enc_service = Arc::new(crate::services::encrypted_password_service::EncryptedPasswordService::new(&data_dir));
        let query_service = Arc::new(PasswordQueryService::new(pool, enc_service));

        Self::new(
            CompressionServiceConfig::default(),
            Arc::new(IOBufferPool::default()),
            Arc::new(RarSupportService::new()),
            Arc::new(UniversalCliEngine::new()),
            query_service,
        )
    }

    pub fn new(
        config: CompressionServiceConfig,
        buffer_pool: Arc<IOBufferPool>,
        rar_service: Arc<RarSupportService>,
        universal_engine: Arc<UniversalCliEngine>,
        password_query_service: Arc<PasswordQueryService>,
    ) -> Self {
        // 默认并发数为 CPU 核心数，最低为 2
        let max_concurrency = if config.max_concurrent_files > 0 {
            config.max_concurrent_files
        } else {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(2)
        };

        Self { 
            config, 
            cancellation_flag: Arc::new(AtomicBool::new(false)),
            buffer_pool,
            rar_service,
            universal_engine,
            password_query_service,
            semaphore: Arc::new(Semaphore::new(max_concurrency)),
        }
    }

    pub fn default() -> Self {
        // 此默认实现使用内存数据库作为密码本后端，仅用于兼容性和测试场景。
        // 在生产代码中请使用 new_with_defaults() 以接入真实的数据库连接。
        log::warn!("CompressionService::default() called - password book features will be unavailable. Use new_with_defaults() instead.");
        let pool = sqlx::pool::Pool::<sqlx::Sqlite>::connect_lazy("sqlite::memory:")
            .unwrap_or_else(|_| sqlx::pool::Pool::<sqlx::Sqlite>::connect_lazy("sqlite::memory:").unwrap());
        let data_dir = std::env::current_dir().unwrap_or_default().join(".password_book_data");
        let enc_service = Arc::new(crate::services::encrypted_password_service::EncryptedPasswordService::new(&data_dir));
        let query_service = Arc::new(PasswordQueryService::new(pool, enc_service));

        Self {
            config: CompressionServiceConfig::default(),
            cancellation_flag: Arc::new(AtomicBool::new(false)),
            buffer_pool: Arc::new(IOBufferPool::default()),
            rar_service: Arc::new(RarSupportService::new()),
            universal_engine: Arc::new(UniversalCliEngine::new()),
            password_query_service: query_service,
            semaphore: Arc::new(Semaphore::new(2)),
        }
    }

    pub fn cancel(&self) {
        self.cancellation_flag.store(true, Ordering::SeqCst);
    }

    pub fn reset_cancellation(&self) {
        self.cancellation_flag.store(false, Ordering::SeqCst);
    }

    fn check_cancellation(&self) -> Result<()> {
        if self.cancellation_flag.load(Ordering::SeqCst) {
            return Err(CompressionError::Cancelled.into());
        }
        Ok(())
    }

    pub fn emit_log(&self, window: &Window, task_id: &str, message: &str, severity: TaskLogSeverity) {
        let log = TaskLog {
            task_id: task_id.to_string(),
            timestamp: Utc::now(),
            message: message.to_string(),
            severity,
        };
        let _ = window.emit("task-log", log);
    }

    pub fn emit_progress(&self, window: &Window, task_id: &str, progress: f32, current_file: Option<String>, processed_bytes: u64, total_bytes: u64) {
        let payload = TaskProgress {
            task_id: task_id.to_string(),
            stage: None,
            current_password: None,
            progress,
            current_file,
            processed_bytes,
            total_bytes,
            speed: None,
        };
        let _ = window.emit("task-progress", payload);
    }

    pub fn validate_compression_request(source_files: &[String], output_path: &str, options: &CompressionOptions) -> Result<String> {
        let requested_format = options.format.as_deref().unwrap_or_else(|| {
            if output_path.to_lowercase().ends_with(".zip") {
                "zip"
            } else {
                "unknown"
            }
        }).to_lowercase();

        if options.split_size.is_some_and(|size| size > 0) {
            return Err(CompressionError::CompressionFailed(
                "Split archive output is not supported yet.".to_string()
            ).into());
        }

        if options.password.as_deref().is_some_and(|password| !password.is_empty())
            && !matches!(requested_format.as_str(), "7z" | "rar")
        {
            return Err(CompressionError::UnsupportedEncryption.into());
        }

        if matches!(requested_format.as_str(), "gz" | "bz2" | "xz") {
            let single_regular_file = source_files.len() == 1 && Path::new(&source_files[0]).is_file();
            if !single_regular_file {
                return Err(CompressionError::CompressionFailed(format!(
                    "{} compression only supports one regular file. Please use a TAR-based format for folders or multiple files.",
                    requested_format
                )).into());
            }
        }

        Ok(requested_format)
    }

    pub fn removable_compressed_sources(source_files: &[String], output_path: &str) -> Result<Vec<PathBuf>> {
        let output = Path::new(output_path);
        if !output.is_file() {
            return Ok(Vec::new());
        }

        let output_canonical = match output.canonicalize() {
            Ok(path) => path,
            Err(_) => return Ok(Vec::new()),
        };

        let mut removable = Vec::new();
        for source in source_files {
            let source_path = Path::new(source);
            if !source_path.is_file() {
                continue;
            }

            let source_canonical = match source_path.canonicalize() {
                Ok(path) => path,
                Err(_) => continue,
            };

            if source_canonical != output_canonical {
                removable.push(source_canonical);
            }
        }

        Ok(removable)
    }

    fn delete_sources_after_success(&self, window: &Window, task_id: &str, source_files: &[String], output_path: &str) {
        match Self::removable_compressed_sources(source_files, output_path) {
            Ok(paths) => {
                for path in paths {
                    if let Err(err) = std::fs::remove_file(&path) {
                        self.emit_log(window, task_id, &format!("Unable to delete source file {}: {}", path.display(), err), TaskLogSeverity::Warning);
                    }
                }
            },
            Err(err) => {
                self.emit_log(window, task_id, &format!("Unable to prepare source cleanup: {}", err), TaskLogSeverity::Warning);
            }
        }
    }

    pub async fn compress(&self, window: Window, task_id: String, source_files: Vec<String>, output_path: String, options: CompressionOptions) -> Result<()> {
        let service = self.clone();
        tokio::task::spawn_blocking(move || {
            let requested_format = Self::validate_compression_request(&source_files, &output_path, &options)?;
            let delete_after = options.delete_after;
            service.emit_log(&window, &task_id, &format!("开始压缩到: {}", output_path), TaskLogSeverity::Info);
            let res = match requested_format.as_str() {
                "zip" => service.do_compress_zip(&window, &task_id, &source_files, &output_path, options),
                "tar" => service.do_compress_tar(&window, &task_id, &source_files, &output_path, options),
                "tar.gz" | "tgz" => service.do_compress_tar_gz(&window, &task_id, &source_files, &output_path, options),
                "tar.bz2" | "tbz" | "tbz2" => service.do_compress_tar_bz2(&window, &task_id, &source_files, &output_path, options),
                "tar.xz" | "txz" => service.do_compress_tar_xz(&window, &task_id, &source_files, &output_path, options),
                "7z" => service.do_compress_7z(&window, &task_id, &source_files, &output_path, options),
                "rar" => service.do_compress_rar(&window, &task_id, &source_files, &output_path, options),
                "gz" => service.do_compress_gz(&window, &task_id, &source_files, &output_path, options),
                "bz2" => service.do_compress_bz2(&window, &task_id, &source_files, &output_path, options),
                "xz" => service.do_compress_xz(&window, &task_id, &source_files, &output_path, options),
                _ => Err(CompressionError::CompressionFailed(format!(
                    "Unsupported compression format '{}'.",
                    requested_format
                )).into()),
            };
            if res.is_ok() {
                if delete_after {
                    service.delete_sources_after_success(&window, &task_id, &source_files, &output_path);
                }
                service.emit_log(&window, &task_id, "压缩完成", TaskLogSeverity::Success);
                service.emit_progress(&window, &task_id, 1.0, None, 0, 0);
            } else {
                service.emit_log(&window, &task_id, &format!("压缩失败: {:?}", res.as_ref().err()), TaskLogSeverity::Error);
            }
            res
        }).await?
    }

    /// 智能尝试密码本中的密码
    async fn resolve_archive_password(&self, window: &Window, task_id: &str, file_path: &str, options: &DecompressOptions) -> Option<String> {
        if let Some(password) = self.attempt_passwords_smartly(window, task_id, file_path).await {
            return Some(password);
        }

        if options.enable_bruteforce && !options.bruteforce_wordlists.is_empty() {
            return self.attempt_bruteforce_wordlists(window, task_id, file_path, &options.bruteforce_wordlists).await;
        }

        None
    }

    async fn attempt_passwords_smartly(&self, window: &Window, task_id: &str, file_path: &str) -> Option<String> {
        use crate::services::password_query_service::{PasswordQueryRequest, SortField, SortOrder};
        
        self.emit_log(window, task_id, "正在检索高频密码本...", TaskLogSeverity::Info);

        let request = PasswordQueryRequest {
            sort_by: Some(SortField::UsageCount),
            sort_order: Some(SortOrder::Desc),
            page_size: Some(1000),
            include_decrypted: true,
            ..Default::default()
        };

        let passwords = match self.password_query_service.search_passwords(&request).await {
            Ok(res) => res.data,
            Err(e) => {
                log::error!("获取密码本失败: {}", e);
                return None;
            }
        };

        let total = passwords.len();
        if total == 0 {
            self.emit_log(window, task_id, "密码本为空，跳过尝试", TaskLogSeverity::Info);
            return None;
        }

        for (idx, entry) in passwords.iter().enumerate() {
            let pwd = &entry.password;
            self.emit_log(window, task_id, &format!("正在尝试已知密码 [{}/{}]: {}...", idx + 1, total, entry.name), TaskLogSeverity::Info);
            
            match self.test_archive_password(file_path, pwd).await {
                Ok(true) => {
                    self.emit_log(window, task_id, &format!("密码匹配成功 ({})", entry.name), TaskLogSeverity::Success);
                    let _ = self.password_query_service.increment_use_count(&entry.id).await;
                    return Some(pwd.clone());
                },
                _ => continue,
            }
        }
        
        self.emit_log(window, task_id, "所有已知密码均匹配失败", TaskLogSeverity::Warning);
        None
    }

    async fn attempt_bruteforce_wordlists(&self, window: &Window, task_id: &str, file_path: &str, wordlists: &[String]) -> Option<String> {
        self.emit_log(window, task_id, "Starting imported wordlist password attempts...", TaskLogSeverity::Info);

        let mut tested = HashSet::new();
        let mut attempted = 0usize;

        for wordlist in wordlists {
            if self.cancellation_flag.load(Ordering::SeqCst) {
                self.emit_log(window, task_id, "Wordlist password attempts cancelled.", TaskLogSeverity::Warning);
                return None;
            }

            let path = Path::new(wordlist);
            let file = match File::open(path) {
                Ok(file) => file,
                Err(err) => {
                    self.emit_log(window, task_id, &format!("Unable to read wordlist {}: {}", path.display(), err), TaskLogSeverity::Warning);
                    continue;
                }
            };

            self.emit_log(
                window,
                task_id,
                &format!("Trying imported wordlist: {}", path.file_name().and_then(|name| name.to_str()).unwrap_or("wordlist")),
                TaskLogSeverity::Info,
            );

            for line in BufReader::new(file).lines() {
                if self.cancellation_flag.load(Ordering::SeqCst) {
                    self.emit_log(window, task_id, "Wordlist password attempts cancelled.", TaskLogSeverity::Warning);
                    return None;
                }

                let password = match line {
                    Ok(value) => value.trim().trim_end_matches('\u{feff}').to_string(),
                    Err(err) => {
                        self.emit_log(window, task_id, &format!("Skipped unreadable wordlist line in {}: {}", path.display(), err), TaskLogSeverity::Warning);
                        continue;
                    }
                };

                if password.is_empty() || !tested.insert(password.clone()) {
                    continue;
                }

                attempted += 1;
                match self.test_archive_password(file_path, &password).await {
                    Ok(true) => {
                        self.emit_log(window, task_id, &format!("Imported wordlist matched after {} attempts.", attempted), TaskLogSeverity::Success);
                        return Some(password);
                    }
                    Ok(false) => {}
                    Err(err) => {
                        self.emit_log(window, task_id, &format!("Wordlist password test failed: {}", err), TaskLogSeverity::Warning);
                    }
                }
            }
        }

        self.emit_log(window, task_id, &format!("Imported wordlists exhausted after {} attempts.", attempted), TaskLogSeverity::Warning);
        None
    }

    pub async fn extract(&self, window: Window, task_id: String, file_path: String, output_dir: Option<String>, password: Option<String>, options: DecompressOptions) -> Result<String> {
        let service = self.clone();
        let path = Path::new(&file_path);
        let mut out_dir = output_dir.map(PathBuf::from).unwrap_or_else(|| {
            path.parent().unwrap_or(Path::new(".")).to_path_buf()
        });
        if options.create_subdirectory {
            out_dir = out_dir.join(Self::archive_output_dir_name(path));
        }

        if !out_dir.exists() {
            std::fs::create_dir_all(&out_dir)?;
        }

        let mut format = ArchiveFormat::Unknown;
        if let Ok(mut f) = File::open(&file_path) {
            let mut header = [0u8; 32];
            if let Ok(_) = f.read(&mut header) {
                format = ArchiveFormat::from_magic(&header);
            }
        }

        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("未知文件").to_string();

        // 托底识别：如果 magic 识别失败，尝试根据后缀识别
        if format == ArchiveFormat::Unknown {
            format = match ext.as_str() {
                "zip" => ArchiveFormat::Zip,
                "7z" => ArchiveFormat::SevenZip,
                "rar" => ArchiveFormat::Rar,
                "tar" => ArchiveFormat::Tar,
                "gz" | "tgz" => ArchiveFormat::Gzip,
                "bz2" => ArchiveFormat::Bzip2,
                "xz" => ArchiveFormat::Xz,
                "iso" | "img" => ArchiveFormat::Iso,
                // 7z CLI 原生支持的杂项格式
                "cab" | "lzh" | "lha" | "arj" | "dmg" | "wim" | "vhd" | "vhdx" | "chm"
                    => ArchiveFormat::Universal,
                _ => ArchiveFormat::Unknown,
            };
            if format != ArchiveFormat::Unknown {
                service.emit_log(&window, &task_id, &format!("Magic匹配失败，根据后缀识别为: {:?}", format), TaskLogSeverity::Warning);
            }
        }

        service.emit_log(&window, &task_id, &format!("确定解压格式: {:?} (后缀: {})", format, ext), TaskLogSeverity::Info);

        let mut final_password = password.clone();
        // ... (省略部分代码以便定位)
        if final_password.is_none() && format.supports_password() {
            // 首先探测是否真的加密
            let needs_pwd = match service.test_archive_password(&file_path, "").await {
                Ok(true) => false,
                _ => true,
            };

            if needs_pwd {
                service.emit_log(&window, &task_id, "检测到加密格式，正在尝试静默解锁...", TaskLogSeverity::Info);
                if let Some(smart_pwd) = service.resolve_archive_password(&window, &task_id, &file_path, &options).await {
                    service.emit_log(&window, &task_id, "密码本匹配成功", TaskLogSeverity::Success);
                    final_password = Some(smart_pwd);
                } else {
                    service.emit_log(&window, &task_id, "所有已知密码均无效，等待手动输入", TaskLogSeverity::Warning);
                    
                    // 主动发射事件触发前端 UI 弹窗
                    let _ = window.emit("password-required", PasswordRequiredPayload {
                        task_id: task_id.clone(),
                        file_path: file_path.clone(),
                        file_name: file_name,
                        format: format!("{:?}", format),
                    });
                    
                    return Err(CompressionError::PasswordRequired.into());
                }
            }
        }

        let win_progress = window.clone();
        let tid_progress = task_id.clone();
        let srv_progress = service.clone();
        let on_progress: Arc<dyn Fn(f32) + Send + Sync> = Arc::new(move |p| {
            srv_progress.emit_progress(&win_progress, &tid_progress, p, None, 0, 0);
        });

        let win_log = window.clone();
        let tid_log = task_id.clone();
        let srv_log = service.clone();
        let on_log: Arc<dyn Fn(String, TaskLogSeverity) + Send + Sync> = Arc::new(move |msg, severity| {
            srv_log.emit_log(&win_log, &tid_log, &msg, severity);
        });

        let result = match format {
            ArchiveFormat::Zip => {
                let srv = service.clone();
                let f_path = file_path.clone();
                let o_dir = out_dir.clone();
                let pwd = final_password.clone();
                let opts = options.clone();
                let t_id = task_id.clone();
                let w = window.clone();
                let o_dir_str = o_dir.to_string_lossy().to_string();
                tokio::task::spawn_blocking(move || {
                    srv.do_extract_zip(&w, &t_id, &f_path, &o_dir_str, pwd.as_deref(), &opts)
                }).await?
            },
            ArchiveFormat::Rar => {
                service.rar_service.extract_rar(
                    Path::new(&file_path),
                    &out_dir,
                    final_password.as_deref(),
                    &options,
                    service.cancellation_flag.clone()
                ).await.map_err(|e| anyhow::anyhow!("RAR 解压失败: {}", e))
            },
            ArchiveFormat::SevenZip => {
                let srv = service.clone();
                let f_path = file_path.clone();
                let o_dir = out_dir.clone();
                let pwd = final_password.clone();
                let opts = options.clone();
                let t_id = task_id.clone();
                let w = window.clone();
                let o_dir_str = o_dir.to_string_lossy().to_string();
                tokio::task::spawn_blocking(move || {
                    srv.do_extract_7z(&w, &t_id, &f_path, &o_dir_str, pwd.as_deref(), &opts)
                }).await?
            },
            ArchiveFormat::Tar => {
                let srv = service.clone();
                let f_path = file_path.clone();
                let o_dir = out_dir.clone();
                let opts = options.clone();
                let t_id = task_id.clone();
                let w = window.clone();
                tokio::task::spawn_blocking(move || srv.do_extract_tar(&w, &t_id, &f_path, &o_dir, None, &opts)).await?
            },
            ArchiveFormat::Gzip => {
                let srv = service.clone();
                let f_path = file_path.clone();
                let o_dir = out_dir.clone();
                let opts = options.clone();
                let t_id = task_id.clone();
                let w = window.clone();
                tokio::task::spawn_blocking(move || {
                    if Self::is_tar_wrapped_archive(Path::new(&f_path), &[".tar.gz", ".tgz"]) {
                        srv.do_extract_tar_gz(&w, &t_id, &f_path, &o_dir, &opts)
                    } else {
                        srv.do_extract_gz(&w, &t_id, &f_path, &o_dir, &opts)
                    }
                }).await?
            },
            ArchiveFormat::Bzip2 => {
                let srv = service.clone();
                let f_path = file_path.clone();
                let o_dir = out_dir.clone();
                let opts = options.clone();
                let t_id = task_id.clone();
                let w = window.clone();
                tokio::task::spawn_blocking(move || {
                    if Self::is_tar_wrapped_archive(Path::new(&f_path), &[".tar.bz2", ".tbz", ".tbz2"]) {
                        srv.do_extract_tar_bz2(&w, &t_id, &f_path, &o_dir, &opts)
                    } else {
                        srv.do_extract_bz2(&w, &t_id, &f_path, &o_dir, &opts)
                    }
                }).await?
            },
            ArchiveFormat::Xz => {
                let srv = service.clone();
                let f_path = file_path.clone();
                let o_dir = out_dir.clone();
                let opts = options.clone();
                let t_id = task_id.clone();
                let w = window.clone();
                tokio::task::spawn_blocking(move || {
                    if Self::is_tar_wrapped_archive(Path::new(&f_path), &[".tar.xz", ".txz"]) {
                        srv.do_extract_tar_xz(&w, &t_id, &f_path, &o_dir, &opts)
                    } else {
                        srv.do_extract_xz(&w, &t_id, &f_path, &o_dir, &opts)
                    }
                }).await?
            },
            ArchiveFormat::Iso | ArchiveFormat::Universal => {
                let fmt_name = format!("{:?}", format);
                service.universal_engine.extract_with_progress(
                    Path::new(&file_path),
                    &out_dir,
                    final_password.as_deref(),
                    options.overwrite_existing,
                    on_progress,
                    on_log,
                    service.cancellation_flag.clone()
                ).await.map_err(|e| anyhow::anyhow!("{}提取失败: {}", fmt_name, e))
            },
            ArchiveFormat::Unknown => {
                match ext.as_str() {
                    "tar" => {
                        let srv = service.clone();
                        let f_path = file_path.clone();
                        let o_dir = out_dir.clone();
                        let opts = options.clone();
                        let t_id = task_id.clone();
                        let w = window.clone();
                        tokio::task::spawn_blocking(move || srv.do_extract_tar(&w, &t_id, &f_path, &o_dir, None, &opts)).await?
                    },
                    _ => {
                        service.universal_engine.extract_with_progress(
                            Path::new(&file_path),
                            &out_dir,
                            final_password.as_deref(),
                            options.overwrite_existing,
                            on_progress,
                            on_log,
                            service.cancellation_flag.clone()
                        ).await.map_err(|e| anyhow::anyhow!("通用引擎解压失败: {}", e))
                    }
                }
            },
        };

        result?;
        if options.delete_after {
            std::fs::remove_file(&file_path)?;
        }
        service.emit_log(&window, &task_id, "全部解压任务已完成", TaskLogSeverity::Success);
        service.emit_progress(&window, &task_id, 1.0, None, 0, 0);
        Ok(out_dir.to_string_lossy().to_string())
    }

    fn archive_output_dir_name(path: &Path) -> String {
        let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("archive");
        for suffix in [".tar.gz", ".tar.bz2", ".tar.xz"] {
            if file_name.to_lowercase().ends_with(suffix) {
                return file_name[..file_name.len() - suffix.len()].to_string();
            }
        }
        path.file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("archive")
            .to_string()
    }

    fn is_tar_wrapped_archive(path: &Path, suffixes: &[&str]) -> bool {
        let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");
        let lower_name = file_name.to_lowercase();
        suffixes.iter().any(|suffix| lower_name.ends_with(suffix))
    }

    fn single_stream_output_name(path: &Path, suffixes: &[&str]) -> String {
        let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("output");
        let lower_name = file_name.to_lowercase();
        for suffix in suffixes {
            if lower_name.ends_with(suffix) && file_name.len() > suffix.len() {
                return file_name[..file_name.len() - suffix.len()].to_string();
            }
        }
        path.file_stem()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("output")
            .to_string()
    }

    fn normalized_archive_path(path: &Path, preserve_paths: bool) -> Option<PathBuf> {
        let source = if preserve_paths {
            path.to_path_buf()
        } else {
            path.file_name().map(PathBuf::from)?
        };

        let mut safe_path = PathBuf::new();
        for component in source.components() {
            if let Component::Normal(part) = component {
                safe_path.push(part);
            }
        }

        if safe_path.as_os_str().is_empty() {
            None
        } else {
            Some(safe_path)
        }
    }

    fn matches_file_filter(path: &Path, filter: Option<&str>) -> bool {
        let Some(filter) = filter.map(str::trim).filter(|value| !value.is_empty()) else {
            return true;
        };

        let normalized = path.to_string_lossy().replace('\\', "/");
        let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");

        filter
            .split([',', ';'])
            .map(str::trim)
            .filter(|pattern| !pattern.is_empty())
            .any(|pattern| {
                Self::wildcard_match(pattern, &normalized)
                    || Self::wildcard_match(pattern, file_name)
            })
    }

    fn wildcard_match(pattern: &str, value: &str) -> bool {
        let escaped = regex::escape(pattern)
            .replace("\\*", ".*")
            .replace("\\?", ".");
        let regex = format!("(?i)^{}$", escaped);
        regex::Regex::new(&regex)
            .map(|compiled| compiled.is_match(value))
            .unwrap_or(false)
    }

    fn resolve_extract_path(target: &Path, options: &DecompressOptions) -> Result<PathBuf> {
        if options.overwrite_existing || !target.exists() {
            return Ok(target.to_path_buf());
        }

        let parent = target.parent().unwrap_or_else(|| Path::new(""));
        let stem = target.file_stem().and_then(|name| name.to_str()).unwrap_or("file");
        let extension = target.extension().and_then(|name| name.to_str());

        for index in 1..10_000 {
            let file_name = match extension {
                Some(ext) if !ext.is_empty() => format!("{} ({}).{}", stem, index, ext),
                _ => format!("{} ({})", stem, index),
            };
            let candidate = parent.join(file_name);
            if !candidate.exists() {
                return Ok(candidate);
            }
        }

        Err(CompressionError::ExtractionFailed(format!(
            "Unable to find available output name for {}",
            target.display()
        )).into())
    }

    pub fn do_extract_zip(&self, window: &Window, task_id: &str, file: &str, output: &str, password: Option<&str>, options: &DecompressOptions) -> Result<()> {
        use crate::utils::io_utils::SmartFileReader;
        let f = SmartFileReader::open(file)?;
        let mut archive = ZipArchive::new(f)?;
        let total_files = archive.len();

        if total_files > 0 {
            if let Some(pwd) = password {
                for i in 0..total_files {
                    let is_file = {
                        let zip_file = archive.by_index(i)?;
                        zip_file.is_file()
                    };
                    if is_file {
                        match archive.by_index_decrypt(i, pwd.as_bytes()) {
                            Ok(Ok(mut reader)) => {
                                let mut probe = [0u8; 4];
                                let _ = reader.read(&mut probe); 
                                drop(reader);
                                break;
                            },
                            Ok(Err(_)) | Err(_) => return Err(CompressionError::InvalidPassword.into()),
                        }
                    }
                }
            } else {
                for i in 0..total_files {
                    let is_file = {
                        let zip_file = archive.by_index(i)?;
                        zip_file.is_file()
                    };
                    if is_file {
                        match archive.by_index_decrypt(i, b"") {
                            Ok(Err(_)) => return Err(CompressionError::PasswordRequired.into()),
                            _ => {}
                        }
                    }
                    if i > 5 { break; } 
                }
            }
        }

        for i in 0..total_files {
            self.check_cancellation()?;
            let (file_name, outpath, is_dir, source_size) = {
                let zip_file = archive.by_index(i)?;
                let file_name = zip_file.name().to_string();
                let is_dir = zip_file.is_dir();
                let relative = match Self::normalized_archive_path(&zip_file.mangled_name(), options.preserve_paths) {
                    Some(path) => path,
                    None => continue,
                };
                if !Self::matches_file_filter(&relative, options.file_filter.as_deref()) {
                    continue;
                }
                let target = Path::new(output).join(relative);
                let outpath = if is_dir {
                    target
                } else {
                    Self::resolve_extract_path(&target, options)?
                };
                (file_name, outpath, is_dir, zip_file.size())
            };

            let entry_result = (|| -> Result<()> {
                if is_dir {
                    std::fs::create_dir_all(&outpath)?;
                    return Ok(());
                }
                if let Some(p) = outpath.parent() {
                    std::fs::create_dir_all(p)?;
                }
                let reader = if let Some(pwd) = password {
                    archive.by_index_decrypt(i, pwd.as_bytes())??
                } else {
                    archive.by_index(i)?
                };
                let mut outfile = File::create(&outpath)?;
                let buf_size = self.buffer_pool.recommend_buffer_size(source_size);
                let mut handle = tauri::async_runtime::block_on(self.buffer_pool.acquire(Some(buf_size)));
                let buffer = handle.buffer_mut().as_mut_slice();
                let mut progress_reader = ProgressReader::new(reader, source_size, Arc::new(|_, _| {}));
                loop {
                    self.check_cancellation()?;
                    let n = progress_reader.read(buffer)?;
                    if n == 0 { break; }
                    outfile.write_all(&buffer[..n])?;
                    let entry_progress = if source_size == 0 {
                        1.0
                    } else {
                        progress_reader.current_pos() as f32 / source_size as f32
                    };
                    let file_progress = (i as f32 / total_files as f32) + (entry_progress / total_files as f32);
                    self.emit_progress(window, task_id, file_progress, Some(file_name.clone()), progress_reader.current_pos(), source_size);
                }
                Ok(())
            })();

            if let Err(err) = entry_result {
                if options.skip_corrupted {
                    self.emit_log(window, task_id, &format!("Skipped entry {}: {}", file_name, err), TaskLogSeverity::Warning);
                    continue;
                }
                return Err(err);
            }
            self.emit_progress(window, task_id, (i + 1) as f32 / total_files as f32, Some(file_name), source_size, source_size);
        }
        Ok(())
    }

    pub fn do_extract_7z(&self, window: &Window, task_id: &str, file: &str, output: &str, password: Option<&str>, options: &DecompressOptions) -> Result<()> {
        let output_root = PathBuf::from(output);
        let opts = options.clone();
        let mut processed = 0usize;
        let total_entries = {
            let archive_file = File::open(file);
            archive_file
                .and_then(|mut archive_file| {
                    let len = archive_file.metadata()?.len();
                    let archive = if let Some(pwd) = password {
                        let password = sevenz_rust::Password::from(pwd);
                        sevenz_rust::Archive::read(&mut archive_file, len, password.as_slice())
                            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err.to_string()))
                    } else {
                        sevenz_rust::Archive::read(&mut archive_file, len, &[])
                            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err.to_string()))
                    }?;
                    Ok(archive.files.iter()
                        .filter(|entry| !entry.is_directory())
                        .filter(|entry| {
                            Self::normalized_archive_path(Path::new(entry.name()), opts.preserve_paths)
                                .map(|path| Self::matches_file_filter(&path, opts.file_filter.as_deref()))
                                .unwrap_or(false)
                        })
                        .count()
                        .max(1))
                })
                .unwrap_or(1)
        };

        let mut extract_entry = |entry: &sevenz_rust::SevenZArchiveEntry, reader: &mut dyn Read, _default_dest: &PathBuf| -> Result<bool, sevenz_rust::Error> {
            self.check_cancellation()
                .map_err(|err| sevenz_rust::Error::other(err.to_string()))?;

            let relative = match Self::normalized_archive_path(Path::new(entry.name()), opts.preserve_paths) {
                Some(path) => path,
                None => {
                    std::io::copy(reader, &mut std::io::sink()).map_err(sevenz_rust::Error::io)?;
                    return Ok(true);
                }
            };

            if !Self::matches_file_filter(&relative, opts.file_filter.as_deref()) {
                std::io::copy(reader, &mut std::io::sink()).map_err(sevenz_rust::Error::io)?;
                return Ok(true);
            }

            let entry_result = (|| -> Result<()> {
                let target = output_root.join(&relative);

                if entry.is_directory() {
                    std::fs::create_dir_all(&target)?;
                    return Ok(());
                }

                let target = Self::resolve_extract_path(&target, &opts)?;
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let mut outfile = File::create(&target)?;
                std::io::copy(reader, &mut outfile)?;
                processed += 1;
                let progress = (processed as f32 / total_entries as f32).min(1.0);
                self.emit_progress(window, task_id, progress, Some(relative.to_string_lossy().to_string()), entry.size(), entry.size());
                Ok(())
            })();

            if let Err(err) = entry_result {
                std::io::copy(reader, &mut std::io::sink()).map_err(sevenz_rust::Error::io)?;
                if opts.skip_corrupted {
                    self.emit_log(window, task_id, &format!("Skipped 7z entry {}: {}", entry.name(), err), TaskLogSeverity::Warning);
                    return Ok(true);
                }
                return Err(sevenz_rust::Error::other(err.to_string()));
            }

            Ok(true)
        };

        let result = if let Some(pwd) = password {
            sevenz_rust::decompress_with_extract_fn_and_password(
                File::open(file)?,
                output,
                sevenz_rust::Password::from(pwd),
                &mut extract_entry,
            )
        } else {
            sevenz_rust::decompress_file_with_extract_fn(file, output, &mut extract_entry)
        };

        result.map_err(|err| {
            let err_msg = err.to_string();
            if err_msg.contains("password") || err_msg.contains("Password") || err_msg.contains("CRC") {
                if password.is_none() {
                    CompressionError::PasswordRequired
                } else {
                    CompressionError::InvalidPassword
                }
            } else {
                CompressionError::ExtractionFailed(err_msg)
            }
        })?;

        if processed == 0 {
            self.emit_log(window, task_id, "No 7z entries matched the current extraction options.", TaskLogSeverity::Warning);
        }

        Ok(())
    }

    fn do_extract_tar(&self, window: &Window, task_id: &str, file: &str, output: &Path, decoder: Option<Box<dyn Read + Send>>, options: &DecompressOptions) -> Result<()> {
        let f = File::open(file)?;
        let mut archive = if let Some(d) = decoder {
            tar::Archive::new(d)
        } else {
            tar::Archive::new(Box::new(f) as Box<dyn Read + Send>)
        };
        let entries = archive.entries()?;
        for entry in entries {
            self.check_cancellation()?;
            let entry_result = (|| -> Result<()> {
                let mut entry = entry?;
                let relative = match Self::normalized_archive_path(&entry.path()?, options.preserve_paths) {
                    Some(path) => path,
                    None => return Ok(()),
                };
                if !Self::matches_file_filter(&relative, options.file_filter.as_deref()) {
                    return Ok(());
                }

                if entry.header().entry_type().is_dir() {
                    let target = output.join(relative);
                    std::fs::create_dir_all(&target)?;
                    return Ok(());
                }
                let target = Self::resolve_extract_path(&output.join(relative), options)?;
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                entry.unpack(&target)?;
                Ok(())
            })();

            if let Err(err) = entry_result {
                if options.skip_corrupted {
                    self.emit_log(window, task_id, &format!("Skipped tar entry: {}", err), TaskLogSeverity::Warning);
                    continue;
                }
                return Err(err);
            }
        }
        Ok(())
    }

    fn do_extract_tar_gz(&self, w: &Window, tid: &str, file: &str, output: &Path, options: &DecompressOptions) -> Result<()> {
        let f = File::open(file)?;
        let gz = flate2::read::GzDecoder::new(f);
        self.do_extract_tar(w, tid, file, output, Some(Box::new(gz)), options)
    }

    fn do_extract_tar_bz2(&self, w: &Window, tid: &str, file: &str, output: &Path, options: &DecompressOptions) -> Result<()> {
        let f = File::open(file)?;
        let bz = bzip2::read::BzDecoder::new(f);
        self.do_extract_tar(w, tid, file, output, Some(Box::new(bz)), options)
    }

    fn do_extract_tar_xz(&self, w: &Window, tid: &str, file: &str, output: &Path, options: &DecompressOptions) -> Result<()> {
        let f = File::open(file)?;
        let xz = xz2::read::XzDecoder::new(f);
        self.do_extract_tar(w, tid, file, output, Some(Box::new(xz)), options)
    }

    fn do_extract_single_stream<R: Read>(&self, window: &Window, task_id: &str, mut reader: R, output: &Path, output_name: String, options: &DecompressOptions) -> Result<()> {
        let relative = PathBuf::from(output_name);
        if !Self::matches_file_filter(&relative, options.file_filter.as_deref()) {
            self.emit_log(window, task_id, "Single-file archive skipped by current file filter.", TaskLogSeverity::Warning);
            return Ok(());
        }

        let target = Self::resolve_extract_path(&output.join(relative), options)?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut outfile = File::create(&target)?;
        let mut buffer = vec![0u8; self.config.buffer_size.max(64 * 1024)];
        let mut processed = 0u64;
        loop {
            self.check_cancellation()?;
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            outfile.write_all(&buffer[..read])?;
            processed += read as u64;
            self.emit_progress(window, task_id, 0.5, None, processed, 0);
        }

        self.emit_log(window, task_id, &format!("Extracted single-file stream to {}", target.display()), TaskLogSeverity::Success);
        Ok(())
    }

    fn do_extract_gz(&self, w: &Window, tid: &str, file: &str, output: &Path, options: &DecompressOptions) -> Result<()> {
        let f = File::open(file)?;
        let gz = flate2::read::GzDecoder::new(f);
        let output_name = Self::single_stream_output_name(Path::new(file), &[".gz"]);
        self.do_extract_single_stream(w, tid, gz, output, output_name, options)
    }

    fn do_extract_bz2(&self, w: &Window, tid: &str, file: &str, output: &Path, options: &DecompressOptions) -> Result<()> {
        let f = File::open(file)?;
        let bz = bzip2::read::BzDecoder::new(f);
        let output_name = Self::single_stream_output_name(Path::new(file), &[".bz2"]);
        self.do_extract_single_stream(w, tid, bz, output, output_name, options)
    }

    fn do_extract_xz(&self, w: &Window, tid: &str, file: &str, output: &Path, options: &DecompressOptions) -> Result<()> {
        let f = File::open(file)?;
        let xz = xz2::read::XzDecoder::new(f);
        let output_name = Self::single_stream_output_name(Path::new(file), &[".xz"]);
        self.do_extract_single_stream(w, tid, xz, output, output_name, options)
    }

    fn unique_archive_name(used_archive_names: &mut HashSet<String>, raw_name: String) -> String {
        let normalized = raw_name.replace('\\', "/");
        if used_archive_names.insert(normalized.clone()) {
            return normalized;
        }

        let path = Path::new(&normalized);
        let parent = path.parent().filter(|value| !value.as_os_str().is_empty());
        let stem = path.file_stem().and_then(|name| name.to_str()).unwrap_or("file");
        let extension = path.extension().and_then(|name| name.to_str());

        for index in 1..10_000 {
            let file_name = match extension {
                Some(ext) if !ext.is_empty() => format!("{} ({}).{}", stem, index, ext),
                _ => format!("{} ({})", stem, index),
            };
            let candidate = parent
                .map(|dir| dir.join(&file_name))
                .unwrap_or_else(|| PathBuf::from(&file_name))
                .to_string_lossy()
                .replace('\\', "/");
            if used_archive_names.insert(candidate.clone()) {
                return candidate;
            }
        }

        normalized
    }

    fn collect_compression_entries(sources: &[String], preserve_paths: bool, include_dirs: bool) -> Result<Vec<(PathBuf, String, bool)>> {
        let mut used_archive_names = HashSet::new();
        let mut entries = Vec::new();

        for source in sources {
            let path = Path::new(source);
            if path.is_file() {
                let file_name = path.file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| CompressionError::CompressionFailed(format!("Invalid file name: {}", source)))?;
                entries.push((path.to_path_buf(), Self::unique_archive_name(&mut used_archive_names, file_name.to_string()), false));
            } else if path.is_dir() {
                let root_name = path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("folder")
                    .to_string();

                for entry in walkdir::WalkDir::new(path).into_iter().filter_map(|item| item.ok()) {
                    let entry_path = entry.path();
                    let is_dir = entry_path.is_dir();
                    if is_dir && !include_dirs {
                        continue;
                    }
                    if !is_dir && !entry_path.is_file() {
                        continue;
                    }

                    let relative = entry_path.strip_prefix(path)
                        .map_err(|e| CompressionError::CompressionFailed(e.to_string()))?;
                    if relative.as_os_str().is_empty() {
                        continue;
                    }

                    let archive_name = if preserve_paths {
                        Path::new(&root_name).join(relative)
                            .to_string_lossy()
                            .replace('\\', "/")
                    } else {
                        entry_path.file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or(if is_dir { "folder" } else { "file" })
                            .to_string()
                    };
                    entries.push((
                        entry_path.to_path_buf(),
                        Self::unique_archive_name(&mut used_archive_names, archive_name),
                        is_dir,
                    ));
                }
            }
        }

        Ok(entries)
    }

    fn do_compress_zip(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        if options.password.as_deref().is_some_and(|password| !password.is_empty()) {
            return Err(CompressionError::UnsupportedEncryption.into());
        }

        if !output.to_lowercase().ends_with(".zip") {
            return Err(CompressionError::CompressionFailed(
                "ZIP compression output path must end with .zip".to_string()
            ).into());
        }

        if let Some(parent) = Path::new(output).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = File::create(output)?;
        let mut zip = zip::ZipWriter::new(file);
        let level = options.level.clamp(1, 9) as i32;
        let zip_options = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(level));

        let preserve_paths = options.preserve_paths.unwrap_or(true);
        let entries = Self::collect_compression_entries(sources, preserve_paths, false)?;

        let total = entries.len().max(1);
        for (i, (path, archive_name, _is_dir)) in entries.iter().enumerate() {
            self.check_cancellation()?;
            zip.start_file(archive_name, zip_options)?;
            let mut f = File::open(path)?;
            std::io::copy(&mut f, &mut zip)?;
            self.emit_progress(window, task_id, (i + 1) as f32 / total as f32, Some(archive_name.clone()), 0, 0);
        }
        zip.finish()?;
        Ok(())
    }

    fn ensure_tar_compression_supported(output: &str, options: &CompressionOptions, extensions: &[&str]) -> Result<()> {
        if options.password.as_deref().is_some_and(|password| !password.is_empty()) {
            return Err(CompressionError::UnsupportedEncryption.into());
        }

        let output_lower = output.to_lowercase();
        if !extensions.iter().any(|extension| output_lower.ends_with(extension)) {
            return Err(CompressionError::CompressionFailed(format!(
                "Output path must end with one of: {}",
                extensions.join(", ")
            )).into());
        }

        if let Some(parent) = Path::new(output).parent() {
            std::fs::create_dir_all(parent)?;
        }

        Ok(())
    }

    fn write_tar_entries<W: Write>(&self, window: &Window, task_id: &str, sources: &[String], options: &CompressionOptions, builder: &mut tar::Builder<W>) -> Result<()> {
        let preserve_paths = options.preserve_paths.unwrap_or(true);
        let entries = Self::collect_compression_entries(sources, preserve_paths, true)?;
        let total = entries.len().max(1);

        for (i, (path, archive_name, is_dir)) in entries.iter().enumerate() {
            self.check_cancellation()?;
            if *is_dir {
                builder.append_dir(archive_name, path)?;
            } else {
                builder.append_path_with_name(path, archive_name)?;
            }
            self.emit_progress(window, task_id, (i + 1) as f32 / total as f32, Some(archive_name.clone()), 0, 0);
        }

        Ok(())
    }

    fn do_compress_tar(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        Self::ensure_tar_compression_supported(output, &options, &[".tar"])?;
        let file = File::create(output)?;
        let mut builder = tar::Builder::new(file);
        self.write_tar_entries(window, task_id, sources, &options, &mut builder)?;
        builder.finish()?;
        Ok(())
    }

    fn do_compress_tar_gz(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        Self::ensure_tar_compression_supported(output, &options, &[".tar.gz", ".tgz"])?;
        let file = File::create(output)?;
        let level = flate2::Compression::new(options.level.clamp(1, 9));
        let encoder = flate2::write::GzEncoder::new(file, level);
        let mut builder = tar::Builder::new(encoder);
        self.write_tar_entries(window, task_id, sources, &options, &mut builder)?;
        let encoder = builder.into_inner()?;
        encoder.finish()?;
        Ok(())
    }

    fn do_compress_tar_bz2(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        Self::ensure_tar_compression_supported(output, &options, &[".tar.bz2", ".tbz", ".tbz2"])?;
        let file = File::create(output)?;
        let level = bzip2::Compression::new(options.level.clamp(1, 9));
        let encoder = bzip2::write::BzEncoder::new(file, level);
        let mut builder = tar::Builder::new(encoder);
        self.write_tar_entries(window, task_id, sources, &options, &mut builder)?;
        let encoder = builder.into_inner()?;
        encoder.finish()?;
        Ok(())
    }

    fn do_compress_tar_xz(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        Self::ensure_tar_compression_supported(output, &options, &[".tar.xz", ".txz"])?;
        let file = File::create(output)?;
        let encoder = xz2::write::XzEncoder::new(file, options.level.clamp(1, 9));
        let mut builder = tar::Builder::new(encoder);
        self.write_tar_entries(window, task_id, sources, &options, &mut builder)?;
        let encoder = builder.into_inner()?;
        encoder.finish()?;
        Ok(())
    }

    fn ensure_single_file_stream_supported<'a>(sources: &'a [String], output: &str, options: &CompressionOptions, extension: &str) -> Result<&'a Path> {
        if options.password.as_deref().is_some_and(|password| !password.is_empty()) {
            return Err(CompressionError::UnsupportedEncryption.into());
        }

        if sources.len() != 1 {
            return Err(CompressionError::CompressionFailed(format!(
                "{} compression only supports one regular file.",
                extension
            )).into());
        }

        let source = Path::new(&sources[0]);
        if !source.is_file() {
            return Err(CompressionError::CompressionFailed(format!(
                "{} compression only supports one regular file.",
                extension
            )).into());
        }

        if !output.to_lowercase().ends_with(extension) {
            return Err(CompressionError::CompressionFailed(format!(
                "Output path must end with {}",
                extension
            )).into());
        }

        if let Some(parent) = Path::new(output).parent() {
            std::fs::create_dir_all(parent)?;
        }

        Ok(source)
    }

    fn do_compress_gz(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        let source = Self::ensure_single_file_stream_supported(sources, output, &options, ".gz")?;
        let mut input = File::open(source)?;
        let file = File::create(output)?;
        let level = flate2::Compression::new(options.level.clamp(1, 9));
        let mut encoder = flate2::write::GzEncoder::new(file, level);
        std::io::copy(&mut input, &mut encoder)?;
        encoder.finish()?;
        self.emit_progress(window, task_id, 1.0, source.file_name().and_then(|name| name.to_str()).map(|name| name.to_string()), 0, 0);
        Ok(())
    }

    fn do_compress_bz2(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        let source = Self::ensure_single_file_stream_supported(sources, output, &options, ".bz2")?;
        let mut input = File::open(source)?;
        let file = File::create(output)?;
        let level = bzip2::Compression::new(options.level.clamp(1, 9));
        let mut encoder = bzip2::write::BzEncoder::new(file, level);
        std::io::copy(&mut input, &mut encoder)?;
        encoder.finish()?;
        self.emit_progress(window, task_id, 1.0, source.file_name().and_then(|name| name.to_str()).map(|name| name.to_string()), 0, 0);
        Ok(())
    }

    fn do_compress_xz(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        let source = Self::ensure_single_file_stream_supported(sources, output, &options, ".xz")?;
        let mut input = File::open(source)?;
        let file = File::create(output)?;
        let mut encoder = xz2::write::XzEncoder::new(file, options.level.clamp(1, 9));
        std::io::copy(&mut input, &mut encoder)?;
        encoder.finish()?;
        self.emit_progress(window, task_id, 1.0, source.file_name().and_then(|name| name.to_str()).map(|name| name.to_string()), 0, 0);
        Ok(())
    }

    fn do_compress_7z(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        if !output.to_lowercase().ends_with(".7z") {
            return Err(CompressionError::CompressionFailed(
                "7z compression output path must end with .7z".to_string()
            ).into());
        }

        if let Some(parent) = Path::new(output).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let preserve_paths = options.preserve_paths.unwrap_or(true);
        let entries = Self::collect_compression_entries(sources, preserve_paths, true)?;
        let total = entries.len().max(1);
        let mut writer = sevenz_rust::SevenZWriter::create(output)
            .map_err(|err| CompressionError::CompressionFailed(err.to_string()))?;

        let level = options.level.clamp(1, 9);
        let lzma_options = sevenz_rust::lzma::LZMA2Options::with_preset(level);
        let mut methods = Vec::new();
        if let Some(password) = options.password.as_deref().filter(|password| !password.is_empty()) {
            methods.push(sevenz_rust::AesEncoderOptions::new(sevenz_rust::Password::from(password)).into());
        }
        methods.push(lzma_options.into());
        writer.set_content_methods(methods);

        for (i, (path, archive_name, is_dir)) in entries.iter().enumerate() {
            self.check_cancellation()?;
            let entry = sevenz_rust::SevenZArchiveEntry::from_path(path, archive_name.clone());
            if *is_dir {
                writer.push_archive_entry::<&[u8]>(entry, None)
                    .map_err(|err| CompressionError::CompressionFailed(err.to_string()))?;
            } else {
                let file = File::open(path)?;
                writer.push_archive_entry(entry, Some(file))
                    .map_err(|err| CompressionError::CompressionFailed(err.to_string()))?;
            }
            self.emit_progress(window, task_id, (i + 1) as f32 / total as f32, Some(archive_name.clone()), 0, 0);
        }

        writer.finish()
            .map_err(|err| CompressionError::CompressionFailed(err.to_string()))?;
        Ok(())
    }

    pub fn find_rar_encoder() -> Option<String> {
        for command in ["rar", "WinRAR"] {
            let exists = if cfg!(target_os = "windows") {
                std::process::Command::new("where")
                    .arg(command)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .map(|status| status.success())
                    .unwrap_or(false)
            } else {
                std::process::Command::new("which")
                    .arg(command)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .map(|status| status.success())
                    .unwrap_or(false)
            };
            if exists {
                return Some(command.to_string());
            }
        }

        #[cfg(target_os = "windows")]
        {
            for path in [
                "C:\\Program Files\\WinRAR\\Rar.exe",
                "C:\\Program Files\\WinRAR\\WinRAR.exe",
                "C:\\Program Files (x86)\\WinRAR\\Rar.exe",
                "C:\\Program Files (x86)\\WinRAR\\WinRAR.exe",
            ] {
                if Path::new(path).exists() {
                    return Some(path.to_string());
                }
            }
        }

        None
    }

    pub fn check_rar_compression_support() -> RarCompressionSupport {
        match Self::find_rar_encoder() {
            Some(encoder_path) => RarCompressionSupport {
                available: true,
                message: format!("RAR encoder detected: {}", encoder_path),
                encoder_path: Some(encoder_path),
            },
            None => RarCompressionSupport {
                available: false,
                encoder_path: None,
                message: "RAR compression requires WinRAR/RAR command line tools. Please install WinRAR or add Rar.exe to PATH.".to_string(),
            },
        }
    }

    fn do_compress_rar(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        if !output.to_lowercase().ends_with(".rar") {
            return Err(CompressionError::CompressionFailed(
                "RAR compression output path must end with .rar".to_string()
            ).into());
        }

        if let Some(parent) = Path::new(output).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let encoder = Self::find_rar_encoder().ok_or_else(|| {
            CompressionError::CompressionFailed(
                "RAR compression requires WinRAR/RAR command line tools. Please install WinRAR or add Rar.exe to PATH.".to_string()
            )
        })?;

        let mut command = std::process::Command::new(encoder);
        command.arg("a");
        command.arg("-idq");
        command.arg("-y");
        command.arg(format!("-m{}", options.level.clamp(1, 5)));

        if options.preserve_paths == Some(false) {
            command.arg("-ep");
        }

        if let Some(password) = options.password.as_deref().filter(|password| !password.is_empty()) {
            command.arg(format!("-p{}", password));
        }

        command.arg(output);
        for source in sources {
            self.check_cancellation()?;
            command.arg(source);
        }

        let output_result = command.output()
            .map_err(|err| CompressionError::CompressionFailed(format!("Failed to run RAR encoder: {}", err)))?;

        if !output_result.status.success() {
            let stderr = String::from_utf8_lossy(&output_result.stderr);
            let stdout = String::from_utf8_lossy(&output_result.stdout);
            let message = if stderr.trim().is_empty() { stdout.trim() } else { stderr.trim() };
            return Err(CompressionError::CompressionFailed(format!("RAR compression failed: {}", message)).into());
        }

        self.emit_progress(window, task_id, 1.0, Some(output.to_string()), 0, 0);
        Ok(())
    }

    pub async fn test_archive_password(&self, file_path: &str, password: &str) -> Result<bool> {
        let file = file_path.to_string();
        let pwd = password.to_string();
        let path = Path::new(&file);
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        tokio::task::spawn_blocking(move || {
            match ext.as_str() {
                "zip" => {
                    let f = File::open(&file)?;
                    let mut archive = ZipArchive::new(f)?;
                    if archive.len() > 0 {
                        // 1. 首先尝试普通读取（判断是否未加密）
                        // 借用 A 开始
                        let can_read_normally = if let Ok(mut zip_file) = archive.by_index(0) {
                            let mut probe = [0u8; 1];
                            zip_file.read(&mut probe).is_ok()
                        } else {
                            false
                        };
                        // 借用 A 结束（zip_file 已 drop）

                        if can_read_normally {
                            return Ok(true);
                        }

                        // 2. 如果普通读取失败，说明可能加密，尝试解密读取
                        // 借用 B 开始
                        if let Ok(Ok(mut reader)) = archive.by_index_decrypt(0, pwd.as_bytes()) {
                            let mut probe = [0u8; 4];
                            return Ok(reader.read(&mut probe).is_ok());
                        }
                        // 借用 B 结束

                        Ok(false)
                    } else { Ok(true) }
                },
                "7z" | "rar" => {
                    let pwd_bytes = sevenz_rust::Password::from(pwd.as_str());
                    let mut file = std::fs::File::open(&file)?;
                    let len = file.metadata()?.len();
                    match sevenz_rust::Archive::read(&mut file, len, pwd_bytes.as_slice()) {
                        Ok(_) => Ok(true),
                        _ => Ok(false)
                    }
                },
                _ => Ok(false)
            }
        }).await?
    }

    pub async fn compress_zip_enhanced(&self, sources: &[String], output: &str, _options: CompressionOptions) -> Result<()> {
        let sources = sources.to_vec();
        let output = output.to_string();
        tokio::task::spawn_blocking(move || {
            let file = File::create(&output)?;
            let mut zip = zip::ZipWriter::new(file);
            let zip_options = FileOptions::default().compression_method(CompressionMethod::Deflated);
            for source in sources {
                let path = Path::new(&source);
                if path.is_file() {
                    let entry_name = path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("unknown");
                    zip.start_file(entry_name, zip_options)?;
                    let mut f = File::open(path)?;
                    std::io::copy(&mut f, &mut zip)?;
                }
            }
            zip.finish()?;
            Ok(())
        }).await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_refined_error_variants() {
        let err1 = CompressionError::PasswordRequired;
        assert_eq!(err1.to_string(), "需要输入密码才能解压");
    }
}
