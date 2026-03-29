use crate::models::compression::{CompressionOptions, TaskLog, TaskLogSeverity};
use anyhow::Result;
use std::path::{Path, PathBuf};
use zip::{ZipArchive, write::FileOptions, CompressionMethod};
use std::io::{Read, Write};
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
            Err(_) => {
                panic!("Failed to get global database connection for CompressionService");
            }
        };

        // 动态计算数据目录，确保与 main.rs 逻辑一致
        let mut data_dir = std::env::current_dir().unwrap();
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
        // 警告：此默认实现仅用于兼容性，PasswordQueryService 必须在实际运行时被正确注入
        // 在命令层，我们应优先使用注入的 Service 实例
        panic!("CompressionService::default() 仅用于满足编译占位，不应在实际逻辑中调用。请使用依赖注入。");
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

    pub async fn compress(&self, window: Window, task_id: String, source_files: Vec<String>, output_path: String, options: CompressionOptions) -> Result<()> {
        let service = self.clone();
        tokio::task::spawn_blocking(move || {
            service.emit_log(&window, &task_id, &format!("开始压缩到: {}", output_path), TaskLogSeverity::Info);
            let res = service.do_compress_zip(&window, &task_id, &source_files, &output_path, options);
            if res.is_ok() {
                service.emit_log(&window, &task_id, "压缩完成", TaskLogSeverity::Success);
                service.emit_progress(&window, &task_id, 1.0, None, 0, 0);
            } else {
                service.emit_log(&window, &task_id, &format!("压缩失败: {:?}", res.as_ref().err()), TaskLogSeverity::Error);
            }
            res
        }).await?
    }

    /// 智能尝试密码本中的密码
    async fn attempt_passwords_smartly(&self, window: &Window, task_id: &str, file_path: &str) -> Option<String> {
        use crate::services::password_query_service::{PasswordQueryRequest, SortField, SortOrder};
        
        self.emit_log(window, task_id, "正在检索高频密码本...", TaskLogSeverity::Info);

        let request = PasswordQueryRequest {
            sort_by: Some(SortField::UsageCount),
            sort_order: Some(SortOrder::Desc),
            page_size: Some(10),
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

    pub async fn extract(&self, window: Window, task_id: String, file_path: String, output_dir: Option<String>, password: Option<String>) -> Result<String> {
        let service = self.clone();
        let path = Path::new(&file_path);
        let out_dir = output_dir.map(PathBuf::from).unwrap_or_else(|| {
            path.parent().unwrap_or(Path::new(".")).to_path_buf()
        });

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
                if let Some(smart_pwd) = service.attempt_passwords_smartly(&window, &task_id, &file_path).await {
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
                let t_id = task_id.clone();
                let w = window.clone();
                tokio::task::spawn_blocking(move || {
                    srv.do_extract_zip(&w, &t_id, &f_path, o_dir.to_str().unwrap(), pwd.as_deref())
                }).await?
            },
            ArchiveFormat::Rar => {
                service.rar_service.extract_rar(
                    Path::new(&file_path),
                    &out_dir,
                    final_password.as_deref()
                ).await.map_err(|e| anyhow::anyhow!("RAR 解压失败: {}", e))
            },
            ArchiveFormat::SevenZip => {
                let srv = service.clone();
                let f_path = file_path.clone();
                let o_dir = out_dir.clone();
                let pwd = final_password.clone();
                let t_id = task_id.clone();
                let w = window.clone();
                tokio::task::spawn_blocking(move || {
                    srv.do_extract_7z(&w, &t_id, &f_path, o_dir.to_str().unwrap(), pwd.as_deref())
                }).await?
            },
            ArchiveFormat::Tar => {
                let srv = service.clone();
                let f_path = file_path.clone();
                let o_dir = out_dir.clone();
                let t_id = task_id.clone();
                let w = window.clone();
                tokio::task::spawn_blocking(move || srv.do_extract_tar(&w, &t_id, &f_path, &o_dir, None)).await?
            },
            ArchiveFormat::Gzip => {
                let srv = service.clone();
                let f_path = file_path.clone();
                let o_dir = out_dir.clone();
                let t_id = task_id.clone();
                let w = window.clone();
                tokio::task::spawn_blocking(move || srv.do_extract_tar_gz(&w, &t_id, &f_path, &o_dir)).await?
            },
            ArchiveFormat::Bzip2 => {
                let srv = service.clone();
                let f_path = file_path.clone();
                let o_dir = out_dir.clone();
                let t_id = task_id.clone();
                let w = window.clone();
                tokio::task::spawn_blocking(move || srv.do_extract_tar_bz2(&w, &t_id, &f_path, &o_dir)).await?
            },
            ArchiveFormat::Xz => {
                let srv = service.clone();
                let f_path = file_path.clone();
                let o_dir = out_dir.clone();
                let t_id = task_id.clone();
                let w = window.clone();
                tokio::task::spawn_blocking(move || srv.do_extract_tar_xz(&w, &t_id, &f_path, &o_dir)).await?
            },
            ArchiveFormat::Unknown => {
                match ext.as_str() {
                    "tar" => {
                        let srv = service.clone();
                        let f_path = file_path.clone();
                        let o_dir = out_dir.clone();
                        let t_id = task_id.clone();
                        let w = window.clone();
                        tokio::task::spawn_blocking(move || srv.do_extract_tar(&w, &t_id, &f_path, &o_dir, None)).await?
                    },
                    _ => {
                        service.universal_engine.extract_with_progress(
                            Path::new(&file_path),
                            &out_dir,
                            final_password.as_deref(),
                            on_progress,
                            on_log,
                            service.cancellation_flag.clone()
                        ).await.map_err(|e| anyhow::anyhow!("通用引擎解压失败: {}", e))
                    }
                }
            },
        };

        result?;
        service.emit_log(&window, &task_id, "全部解压任务已完成", TaskLogSeverity::Success);
        service.emit_progress(&window, &task_id, 1.0, None, 0, 0);
        Ok(out_dir.to_str().unwrap().to_string())
    }

    pub fn do_extract_zip(&self, window: &Window, task_id: &str, file: &str, output: &str, password: Option<&str>) -> Result<()> {
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
                let outpath = Path::new(output).join(zip_file.mangled_name());
                (file_name, outpath, zip_file.is_dir(), zip_file.size())
            };

            if is_dir {
                std::fs::create_dir_all(&outpath)?;
            } else {
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
                    let file_progress = (i as f32 / total_files as f32) + (progress_reader.current_pos() as f32 / source_size as f32 / total_files as f32);
                    self.emit_progress(window, task_id, file_progress, Some(file_name.clone()), progress_reader.current_pos(), source_size);
                }
            }
            self.emit_progress(window, task_id, (i + 1) as f32 / total_files as f32, Some(file_name), source_size, source_size);
        }
        Ok(())
    }

    pub fn do_extract_7z(&self, _window: &Window, _task_id: &str, file: &str, output: &str, password: Option<&str>) -> Result<()> {
        let path = Path::new(file);
        let pwd_bytes = password.map(|p| sevenz_rust::Password::from(p));
        let mut f = File::open(path)?;
        let len = f.metadata()?.len();
        let archive_res = if let Some(ref p) = pwd_bytes {
            sevenz_rust::Archive::read(&mut f, len, p.as_slice())
        } else {
            sevenz_rust::Archive::read(&mut f, len, &[])
        };
        match archive_res {
            Ok(_archive) => {
                if let Some(p) = pwd_bytes {
                    sevenz_rust::decompress_file_with_password(file, output, p)
                        .map_err(|e| {
                            let err_msg = e.to_string();
                            if err_msg.contains("Password") || err_msg.contains("CRC") {
                                CompressionError::InvalidPassword
                            } else {
                                CompressionError::ExtractionFailed(err_msg)
                            }
                        })?;
                } else {
                    sevenz_rust::decompress_file(file, output)
                        .map_err(|e| {
                            let err_msg = e.to_string();
                            if err_msg.contains("password") || err_msg.contains("Password") {
                                CompressionError::PasswordRequired
                            } else {
                                CompressionError::ExtractionFailed(err_msg)
                            }
                        })?;
                }
            },
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("Password") || err_msg.contains("CRC") {
                    if password.is_none() {
                        return Err(CompressionError::PasswordRequired.into());
                    } else {
                        return Err(CompressionError::InvalidPassword.into());
                    }
                }
                return Err(CompressionError::ExtractionFailed(err_msg).into());
            }
        }
        Ok(())
    }

    fn do_extract_tar(&self, _window: &Window, _task_id: &str, file: &str, output: &Path, decoder: Option<Box<dyn Read + Send>>) -> Result<()> {
        let f = File::open(file)?;
        let mut archive = if let Some(d) = decoder {
            tar::Archive::new(d)
        } else {
            tar::Archive::new(Box::new(f) as Box<dyn Read + Send>)
        };
        let entries = archive.entries()?;
        for entry in entries {
            self.check_cancellation()?;
            let mut entry = entry?;
            let _path = entry.path()?.to_path_buf();
            entry.unpack_in(output)?;
        }
        Ok(())
    }

    fn do_extract_tar_gz(&self, _w: &Window, _tid: &str, file: &str, output: &Path) -> Result<()> {
        let f = File::open(file)?;
        let gz = flate2::read::GzDecoder::new(f);
        let mut archive = tar::Archive::new(gz);
        archive.unpack(output)?;
        Ok(())
    }

    fn do_extract_tar_bz2(&self, _w: &Window, _tid: &str, file: &str, output: &Path) -> Result<()> {
        let f = File::open(file)?;
        let bz = bzip2::read::BzDecoder::new(f);
        let mut archive = tar::Archive::new(bz);
        archive.unpack(output)?;
        Ok(())
    }

    fn do_extract_tar_xz(&self, _w: &Window, _tid: &str, file: &str, output: &Path) -> Result<()> {
        let f = File::open(file)?;
        let xz = xz2::read::XzDecoder::new(f);
        let mut archive = tar::Archive::new(xz);
        archive.unpack(output)?;
        Ok(())
    }

    fn do_compress_zip(&self, window: &Window, task_id: &str, sources: &[String], output: &str, _options: CompressionOptions) -> Result<()> {
        let file = File::create(output)?;
        let mut zip = zip::ZipWriter::new(file);
        let zip_options = FileOptions::default().compression_method(CompressionMethod::Deflated);
        let total = sources.len();
        for (i, source) in sources.iter().enumerate() {
            self.check_cancellation()?;
            let path = Path::new(source);
            if path.is_file() {
                let file_name = path.file_name().unwrap().to_str().unwrap();
                zip.start_file(file_name, zip_options)?;
                let mut f = File::open(path)?;
                std::io::copy(&mut f, &mut zip)?;
                self.emit_progress(window, task_id, (i + 1) as f32 / total as f32, Some(file_name.to_string()), 0, 0);
            }
        }
        zip.finish()?;
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
                    zip.start_file(path.file_name().unwrap().to_str().unwrap(), zip_options)?;
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
