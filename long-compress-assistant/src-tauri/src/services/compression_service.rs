use crate::models::compression::{CompressionOptions, DecompressOptions, TaskLog, TaskLogSeverity};
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use zip::{write::FileOptions, CompressionMethod};
use std::io::{BufRead, BufReader, Read, Write};
use std::fs::File;
use sevenz_rust;
use thiserror::Error;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tauri::Window;
use chrono::Utc;
use serde::Serialize;

use crate::services::io_buffer_pool::IOBufferPool;
use crate::services::rar_support::RarSupportService;
use crate::services::universal_engine::UniversalCliEngine;
use crate::services::archive_engine::ArchiveEngine;
use crate::services::password_query_service::PasswordQueryService;
use crate::services::tar_aes_engine::TarAesEngine;
use crate::services::aes_wrapper::AesWrapper;
use crate::utils::archive_tools::{find_7z_command, missing_7z_message};
use crate::services::compression_format::{self, CompressionRoute};
use crate::services::compression_verification;
use crate::services::extraction_transaction::{self, ExtractionStaging};
use crate::services::mark_of_web::{self, PropagationStatus};
use crate::services::native_compression::{self, CompressionRuntime};
use crate::services::native_extraction::{self, ExtractionRuntime};

pub use crate::services::archive_format::ArchiveFormat;
pub use crate::services::compression_format::{
    CompressionFormatCapability, COMPRESSION_FORMAT_CAPABILITIES,
};

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

#[derive(Clone, Serialize)]
pub struct TaskProgress {
    pub task_id: String,
    pub stage: Option<String>,
    pub current_password: Option<String>,
    pub progress: f32,
    pub speed: Option<String>,
    pub eta_seconds: Option<u64>,
    pub current_file: Option<String>,
    pub processed_bytes: u64,
    pub total_bytes: u64,
    // 密码尝试进度
    pub password_attempt_current: Option<usize>,
    pub password_attempt_total: Option<usize>,
}

#[derive(Clone, Serialize)]
pub struct PasswordRequiredPayload {
    pub task_id: String,
    pub file_path: String,
    pub file_name: String,
    pub format: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileConflictPayload {
    pub task_id: String,
    pub file_name: String,
    pub source_path: String,
    pub dest_path: String,
    pub source_size: u64,
    pub dest_size: u64,
    pub source_modified: u64,
    pub dest_modified: u64,
}

#[derive(Clone, Serialize)]
pub struct RarCompressionSupport {
    pub available: bool,
    pub encoder_path: Option<String>,
    pub message: String,
}

use tokio::sync::Semaphore;

struct ProgressMetric {
    started_at: Instant,
    last_bytes: u64,
}

#[derive(Clone)]
pub struct CompressionService {
    pub config: CompressionServiceConfig,
    pub cancellation_flag: Arc<AtomicBool>,
    pub buffer_pool: Arc<IOBufferPool>,
    pub rar_service: Arc<RarSupportService>,
    pub universal_engine: Arc<UniversalCliEngine>,
    pub password_query_service: Arc<PasswordQueryService>,
    pub semaphore: Arc<Semaphore>,
    progress_metrics: Arc<Mutex<HashMap<String, ProgressMetric>>>,
}

impl ExtractionRuntime for CompressionService {
    fn check_cancellation(&self) -> Result<()> {
        CompressionService::check_cancellation(self)
    }

    fn buffer_pool(&self) -> &IOBufferPool {
        &self.buffer_pool
    }

    fn copy_buffer_size(&self) -> usize {
        self.config.buffer_size.max(Self::COPY_BUFFER_SIZE)
    }

    fn normalized_archive_path(&self, path: &Path, preserve_paths: bool) -> Option<PathBuf> {
        CompressionService::normalized_archive_path(path, preserve_paths)
    }

    fn emit_log(&self, window: &Window, task_id: &str, message: &str, severity: TaskLogSeverity) {
        CompressionService::emit_log(self, window, task_id, message, severity);
    }

    fn emit_progress(&self, window: &Window, task_id: &str, progress: f32, current_file: Option<String>, processed_bytes: u64, total_bytes: u64) {
        CompressionService::emit_progress(self, window, task_id, progress, current_file, processed_bytes, total_bytes);
    }
}

impl CompressionRuntime for CompressionService {
    fn check_cancellation(&self) -> Result<()> {
        CompressionService::check_cancellation(self)
    }

    fn cancellation_flag(&self) -> Arc<AtomicBool> {
        self.cancellation_flag.clone()
    }

    fn copy_buffer_size(&self) -> usize {
        self.config.buffer_size.max(Self::COPY_BUFFER_SIZE)
    }

    fn emit_log(&self, window: &Window, task_id: &str, message: &str, severity: TaskLogSeverity) {
        CompressionService::emit_log(self, window, task_id, message, severity);
    }

    fn emit_progress(&self, window: &Window, task_id: &str, progress: f32, current_file: Option<String>, processed_bytes: u64, total_bytes: u64) {
        CompressionService::emit_progress(self, window, task_id, progress, current_file, processed_bytes, total_bytes);
    }
}

impl CompressionService {
    #[cfg(test)]
    const MAX_EXTRACTED_FILES: usize = extraction_transaction::MAX_EXTRACTED_ENTRIES;
    #[cfg(test)]
    const MAX_EXTRACTED_BYTES: u64 = extraction_transaction::MAX_EXTRACTED_BYTES;
    const COPY_BUFFER_SIZE: usize = 256 * 1024;

    pub async fn new_with_defaults() -> Self {
        let pool = match crate::database::connection::get_connection().await {
            Ok(conn) => conn.pool().clone(),
            Err(e) => {
                log::warn!("Database connection unavailable; password book features will be limited: {}", e);
                sqlx::pool::Pool::<sqlx::Sqlite>::connect_lazy("sqlite::memory:").unwrap_or_else(|_| {
                    sqlx::pool::Pool::<sqlx::Sqlite>::connect_lazy("sqlite::memory:").unwrap()
                })
            }
        };

        let data_dir = crate::utils::app_paths::app_data_dir();
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
            progress_metrics: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn for_testing() -> Self {
        // 此实现使用内存数据库作为密码本后端，仅用于兼容性和测试场景。
        // 在生产代码中请使用 new_with_defaults() 以接入真实的数据库连接。
        log::warn!("CompressionService::for_testing() called - password book features will be unavailable. Use new_with_defaults() instead.");
        let pool = sqlx::pool::Pool::<sqlx::Sqlite>::connect_lazy("sqlite::memory:")
            .unwrap_or_else(|_| sqlx::pool::Pool::<sqlx::Sqlite>::connect_lazy("sqlite::memory:").unwrap());
        let data_dir = crate::utils::app_paths::app_data_dir();
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
            progress_metrics: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn cancel(&self) {
        self.cancellation_flag.store(true, Ordering::SeqCst);
    }

    pub fn reset_cancellation(&self) {
        self.cancellation_flag.store(false, Ordering::SeqCst);
    }

    fn check_cancellation(&self) -> Result<()> {
        if self.cancellation_flag.load(Ordering::Relaxed) {
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

    fn progress_telemetry(
        &self,
        task_id: &str,
        progress: f32,
        processed_bytes: u64,
        total_bytes: u64,
    ) -> (Option<String>, Option<u64>) {
        if processed_bytes == 0 || total_bytes == 0 {
            return (None, None);
        }

        let Ok(mut metrics) = self.progress_metrics.lock() else {
            return (None, None);
        };
        let metric = metrics.entry(task_id.to_string()).or_insert_with(|| ProgressMetric {
            started_at: Instant::now(),
            last_bytes: 0,
        });
        if processed_bytes < metric.last_bytes {
            metric.started_at = Instant::now();
        }
        metric.last_bytes = processed_bytes;

        let elapsed = metric.started_at.elapsed().as_secs_f64();
        let bytes_per_second = if elapsed > f64::EPSILON {
            processed_bytes as f64 / elapsed
        } else {
            0.0
        };
        let result = if bytes_per_second > 0.0 {
            let speed = if bytes_per_second >= 1024.0 * 1024.0 {
                format!("{:.1} MB/s", bytes_per_second / (1024.0 * 1024.0))
            } else if bytes_per_second >= 1024.0 {
                format!("{:.1} KB/s", bytes_per_second / 1024.0)
            } else {
                format!("{:.0} B/s", bytes_per_second)
            };
            let remaining = total_bytes.saturating_sub(processed_bytes) as f64;
            (Some(speed), Some((remaining / bytes_per_second).ceil() as u64))
        } else {
            (None, None)
        };

        if progress >= 1.0 || processed_bytes >= total_bytes {
            metrics.remove(task_id);
        }
        result
    }

    fn begin_progress_telemetry(&self, task_id: &str) {
        if let Ok(mut metrics) = self.progress_metrics.lock() {
            metrics.insert(task_id.to_string(), ProgressMetric {
                started_at: Instant::now(),
                last_bytes: 0,
            });
        }
    }

    pub fn emit_progress(&self, window: &Window, task_id: &str, progress: f32, current_file: Option<String>, processed_bytes: u64, total_bytes: u64) {
        let (speed, eta_seconds) = self.progress_telemetry(task_id, progress, processed_bytes, total_bytes);
        let payload = TaskProgress {
            task_id: task_id.to_string(),
            stage: None,
            current_password: None,
            progress,
            current_file,
            processed_bytes,
            total_bytes,
            speed,
            eta_seconds,
            password_attempt_current: None,
            password_attempt_total: None,
        };
        let _ = window.emit("task-progress", payload);
    }

    fn emit_compression_stage(
        &self,
        window: &Window,
        task_id: &str,
        stage: &str,
        current_file: Option<String>,
    ) {
        let payload = TaskProgress {
            task_id: task_id.to_string(),
            stage: Some(stage.to_string()),
            current_password: None,
            progress: 1.0,
            current_file,
            processed_bytes: 0,
            total_bytes: 0,
            speed: None,
            eta_seconds: None,
            password_attempt_current: None,
            password_attempt_total: None,
        };
        let _ = window.emit("task-progress", payload);
    }

    pub fn infer_compression_format(output_path: &str, explicit_format: Option<&str>) -> String {
        compression_format::infer_compression_format(output_path, explicit_format)
    }

    pub fn compression_format_capabilities() -> &'static [CompressionFormatCapability] {
        compression_format::compression_format_capabilities()
    }

    pub fn find_compression_format_capability(format: &str) -> Option<&'static CompressionFormatCapability> {
        compression_format::find_compression_format_capability(format)
    }

    pub fn validate_compression_request(source_files: &[String], output_path: &str, options: &CompressionOptions) -> Result<String> {
        compression_format::validate_compression_request(source_files, output_path, options)
    }

    fn run_command_cancellable(
        &self,
        mut command: std::process::Command,
    ) -> Result<std::process::Output> {
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());
        let mut child = command.spawn()?;
        let mut stdout = child.stdout.take().ok_or_else(|| {
            CompressionError::CompressionFailed("Unable to capture encoder output".to_string())
        })?;
        let mut stderr = child.stderr.take().ok_or_else(|| {
            CompressionError::CompressionFailed("Unable to capture encoder errors".to_string())
        })?;
        let stdout_reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stdout.read_to_end(&mut bytes);
            bytes
        });
        let stderr_reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stderr.read_to_end(&mut bytes);
            bytes
        });

        let status = loop {
            if self.cancellation_flag.load(Ordering::SeqCst) {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(CompressionError::Cancelled.into());
            }
            if let Some(status) = child.try_wait()? {
                break status;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        };
        Ok(std::process::Output {
            status,
            stdout: stdout_reader.join().unwrap_or_default(),
            stderr: stderr_reader.join().unwrap_or_default(),
        })
    }

    fn canonicalize_with_missing_tail(path: &Path) -> Result<PathBuf> {
        let absolute = std::path::absolute(path)?;
        let mut existing = absolute.as_path();
        let mut tail = Vec::new();
        while !existing.exists() {
            let name = existing.file_name().ok_or_else(|| {
                CompressionError::CompressionFailed(format!("Invalid path: {}", path.display()))
            })?;
            tail.push(name.to_os_string());
            existing = existing.parent().ok_or_else(|| {
                CompressionError::CompressionFailed(format!("Invalid path: {}", path.display()))
            })?;
        }
        let mut resolved = existing.canonicalize()?;
        for component in tail.into_iter().rev() {
            resolved.push(component);
        }
        Ok(resolved)
    }

    fn validate_compression_io_paths(source_files: &[String], output_path: &str, options: &CompressionOptions) -> Result<()> {
        if source_files.is_empty() {
            return Err(CompressionError::CompressionFailed("At least one source file or folder is required".to_string()).into());
        }
        if output_path.trim().is_empty() {
            return Err(CompressionError::CompressionFailed("Output path is required".to_string()).into());
        }

        let output = Path::new(output_path);
        if output.exists() {
            return Err(CompressionError::CompressionFailed(format!(
                "Output already exists; choose a new path to avoid overwriting data: {}",
                output.display()
            )).into());
        }
        let resolved_output = Self::canonicalize_with_missing_tail(output)?;
        let split_requested = options.split_size.is_some_and(|size| size > 0);
        let has_password = options.password.as_deref().is_some_and(|password| !password.is_empty());
        if split_requested && has_password {
            return Err(CompressionError::CompressionFailed(
                "Encrypted split archives are not supported safely; disable splitting or remove the password".to_string()
            ).into());
        }

        for source in source_files {
            let source_path = Path::new(source);
            if !source_path.exists() {
                return Err(CompressionError::FileNotFound(source.clone()).into());
            }
            if !source_path.is_file() && !source_path.is_dir() {
                return Err(CompressionError::CompressionFailed(format!("Unsupported source type: {}", source)).into());
            }
            if std::fs::symlink_metadata(source_path)?.file_type().is_symlink() {
                return Err(CompressionError::CompressionFailed(format!(
                    "Symbolic links or reparse points are not accepted as archive sources: {}",
                    source_path.display()
                )).into());
            }
            if source_path.is_dir() {
                for entry in walkdir::WalkDir::new(source_path).follow_links(false) {
                    let entry = entry.map_err(|error| {
                        CompressionError::CompressionFailed(format!(
                            "Unable to inspect source tree safely: {}",
                            error
                        ))
                    })?;
                    if entry.file_type().is_symlink() {
                        return Err(CompressionError::CompressionFailed(format!(
                            "Source tree contains a symbolic link or reparse point: {}",
                            entry.path().display()
                        )).into());
                    }
                }
            }
            if split_requested && source_path.is_dir() {
                return Err(CompressionError::CompressionFailed(
                    "Split ZIP creation currently supports regular files only".to_string()
                ).into());
            }

            let resolved_source = source_path.canonicalize()?;
            if resolved_source == resolved_output
                || (source_path.is_dir() && resolved_output.starts_with(&resolved_source))
            {
                return Err(CompressionError::CompressionFailed(format!(
                    "Output must not replace a source or be created inside a source folder: {}",
                    output.display()
                )).into());
            }
        }
        Ok(())
    }

    fn temporary_compression_output(output: &Path) -> Result<PathBuf> {
        let parent = output.parent().unwrap_or_else(|| Path::new("."));
        let file_name = output.file_name().and_then(|name| name.to_str()).ok_or_else(|| {
            CompressionError::CompressionFailed(format!("Invalid output file name: {}", output.display()))
        })?;
        Ok(parent.join(format!(".long-compress-{}.{}", uuid::Uuid::new_v4(), file_name)))
    }

    fn cleanup_failed_compression_outputs(path: &Path, split_requested: bool) {
        // Standard split creation owns and cleans only the volumes it created.
        // Never glob-clean a user-visible prefix here: pre-existing .001 files
        // may belong to another archive and must survive a validation failure.
        if split_requested {
            return;
        }
        let _ = std::fs::remove_file(path);
        let Some(parent) = path.parent() else { return; };
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else { return; };
        let unique_temp_prefix = if file_name.starts_with(".long-compress-") {
            file_name.split('.').nth(1).map(|id| format!(".{}.", id))
        } else {
            None
        };
        if let Ok(entries) = std::fs::read_dir(parent) {
            for entry in entries.flatten() {
                let candidate = entry.path();
                let name = candidate.file_name().and_then(|value| value.to_str()).unwrap_or("");
                let is_temp_sidecar = name.starts_with(file_name)
                    || unique_temp_prefix.as_ref().is_some_and(|prefix| name.starts_with(prefix));
                if is_temp_sidecar {
                    let _ = std::fs::remove_file(candidate);
                }
            }
        }
    }

    fn normalize_storage_full_error(error: anyhow::Error) -> anyhow::Error {
        let storage_full = error.chain().any(|cause| {
            cause
                .downcast_ref::<std::io::Error>()
                .is_some_and(|io_error| io_error.kind() == std::io::ErrorKind::StorageFull)
        });
        if storage_full {
            CompressionError::DiskFull.into()
        } else {
            error
        }
    }

    fn finalize_compression_output(
        route_result: Result<()>,
        working_output: &Path,
        final_output: &Path,
        split_requested: bool,
    ) -> Result<()> {
        let result = route_result
            .map_err(Self::normalize_storage_full_error)
            .and_then(|_| {
                if !split_requested {
                    if final_output.exists() {
                        return Err(CompressionError::CompressionFailed(format!(
                            "Output appeared while compression was running; it was not overwritten: {}",
                            final_output.display()
                        ))
                        .into());
                    }
                    std::fs::rename(working_output, final_output)?;
                }
                Ok(())
            })
            .map_err(Self::normalize_storage_full_error);

        if result.is_err() {
            Self::cleanup_failed_compression_outputs(working_output, split_requested);
        }
        result
    }

    fn cleanup_unverified_compression_output(
        working_output: &Path,
        final_output: &Path,
        split_requested: bool,
    ) {
        if split_requested {
            crate::services::split_compression::SplitCompressionService::cleanup_parts(
                final_output,
            );
        } else {
            Self::cleanup_failed_compression_outputs(working_output, false);
        }
    }

    fn verify_compression_output(
        &self,
        route: CompressionRoute,
        output: &Path,
        password: Option<&str>,
        split_requested: bool,
    ) -> Result<()> {
        if !split_requested
            && compression_verification::verify_native(route, output, password, || {
                self.cancellation_flag.load(Ordering::Relaxed)
            })?
        {
            return Ok(());
        }

        self.check_cancellation()?;
        let output_result = if route == CompressionRoute::Rar {
            let encoder = Self::find_rar_encoder().ok_or_else(|| {
                CompressionError::CompressionFailed(
                    "RAR verification requires the RAR command line encoder".to_string(),
                )
            })?;
            let mut command = crate::utils::process::command(encoder);
            command.arg("t").arg("-idq").arg("-y");
            if let Some(password) = password.filter(|value| !value.is_empty()) {
                command.arg(format!("-p{password}"));
            } else {
                command.arg("-p-");
            }
            command.arg(output);
            self.run_command_cancellable(command)?
        } else {
            let engine = find_7z_command().ok_or_else(|| {
                CompressionError::CompressionFailed(missing_7z_message())
            })?;
            let mut command = crate::utils::process::command(engine);
            command.arg("t").arg("-y").arg("-p-").arg(output);
            self.run_command_cancellable(command)?
        };

        if !output_result.status.success() {
            let stderr = String::from_utf8_lossy(&output_result.stderr);
            let stdout = String::from_utf8_lossy(&output_result.stdout);
            let detail = if stderr.trim().is_empty() {
                stdout.trim()
            } else {
                stderr.trim()
            };
            return Err(CompressionError::CompressionFailed(format!(
                "Newly created archive failed integrity verification: {detail}"
            ))
            .into());
        }
        Ok(())
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
            Self::validate_compression_io_paths(&source_files, &output_path, &options)?;
            service.begin_progress_telemetry(&task_id);
            let delete_after = options.delete_after;
            let requested_verify_after = options.verify_after;
            let verify_after = requested_verify_after || delete_after;
            let verification_password = options.password.clone();
            let split_requested = options.split_size.is_some_and(|size| size > 0);
            let final_output = PathBuf::from(&output_path);
            let working_output = if split_requested {
                final_output.clone()
            } else {
                Self::temporary_compression_output(&final_output)?
            };
            let working_output_string = working_output.to_string_lossy().to_string();
            service.emit_log(&window, &task_id, &format!("开始压缩到: {}", output_path), TaskLogSeverity::Info);
            let route = compression_format::compression_route(&requested_format).ok_or_else(|| {
                CompressionError::CompressionFailed(format!(
                    "Unsupported compression format '{}'.",
                    requested_format
                ))
            })?;
            let res = match route {
                CompressionRoute::TarAes => service.do_compress_tar_aes(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::TarGzipAes => service.do_compress_tar_gz_aes(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::TarBzip2Aes => service.do_compress_tar_bz2_aes(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::TarXzAes => service.do_compress_tar_xz_aes(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::TarZstdAes => service.do_compress_tar_zst_aes(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::GzipAes => service.do_compress_gz_aes(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::Bzip2Aes => service.do_compress_bz2_aes(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::XzAes => service.do_compress_xz_aes(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::ZstdAes => service.do_compress_zst_aes(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::Zip => service.do_compress_zip(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::Tar => service.do_compress_tar(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::TarGzip => service.do_compress_tar_gz(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::TarBzip2 => service.do_compress_tar_bz2(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::TarXz => service.do_compress_tar_xz(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::SevenZip => service.do_compress_7z(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::Rar => service.do_compress_rar(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::Wim => service.do_compress_wim(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::Gzip => service.do_compress_gz(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::Bzip2 => service.do_compress_bz2(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::Xz => service.do_compress_xz(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::Zstd => service.do_compress_zstd(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::TarZstd => service.do_compress_tar_zstd(&window, &task_id, &source_files, &working_output_string, options),
                CompressionRoute::Lzma => service.do_compress_lzma(&window, &task_id, &source_files, &working_output_string, options),
            };
            let verification_output = if split_requested {
                PathBuf::from(format!("{}.001", output_path))
            } else {
                working_output.clone()
            };
            let res = res.and_then(|_| {
                if verify_after {
                    service.emit_compression_stage(
                        &window,
                        &task_id,
                        "Verifying",
                        Some(output_path.clone()),
                    );
                    service.emit_log(
                        &window,
                        &task_id,
                        if delete_after && !requested_verify_after {
                            "已启用删除源文件，正在执行强制完整性校验"
                        } else {
                            "正在校验新压缩包的完整性"
                        },
                        TaskLogSeverity::Info,
                    );
                    service.verify_compression_output(
                        route,
                        &verification_output,
                        verification_password.as_deref(),
                        split_requested,
                    )?;
                    service.emit_log(
                        &window,
                        &task_id,
                        "压缩包完整性校验通过",
                        TaskLogSeverity::Success,
                    );
                }
                Ok(())
            });
            if res.is_err() {
                Self::cleanup_unverified_compression_output(
                    &working_output,
                    &final_output,
                    split_requested,
                );
            }
            if res.is_ok() {
                service.emit_compression_stage(
                    &window,
                    &task_id,
                    "Finalizing",
                    Some(output_path.clone()),
                );
            }
            let res = Self::finalize_compression_output(
                res,
                &working_output,
                &final_output,
                split_requested,
            );
            if res.is_ok() {
                if delete_after {
                    let verified_output = if split_requested {
                        format!("{}.001", output_path)
                    } else {
                        output_path.clone()
                    };
                    service.delete_sources_after_success(&window, &task_id, &source_files, &verified_output);
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

        if options.enable_bruteforce {
            if !options.bruteforce_wordlists.is_empty() {
                if let Some(password) = self.attempt_bruteforce_wordlists(window, task_id, file_path, &options.bruteforce_wordlists).await {
                    return Some(password);
                }
            }
            if let Some(password) = self.attempt_recommended_dictionary(window, task_id, file_path).await {
                return Some(password);
            }
        }

        None
    }

    pub async fn resolve_archive_password_silent(&self, file_path: &str, options: &DecompressOptions) -> Option<String> {
        if let Some(password) = self.attempt_password_book_candidates(file_path).await {
            return Some(password);
        }

        if options.enable_bruteforce {
            if !options.bruteforce_wordlists.is_empty() {
                if let Some(password) = self.attempt_bruteforce_wordlists_silent(file_path, &options.bruteforce_wordlists).await {
                    return Some(password);
                }
            }
            if let Some(password) = self.attempt_recommended_dictionary_silent(file_path).await {
                return Some(password);
            }
        }

        None
    }

    fn recommended_dictionary(file_path: &str) -> Vec<String> {
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(file_path);
        crate::services::password_dictionary_service::PasswordDictionaryService::new()
            .get_recommended_strategy(Some(file_name))
    }

    async fn attempt_recommended_dictionary_silent(&self, file_path: &str) -> Option<String> {
        for password in Self::recommended_dictionary(file_path) {
            if self.cancellation_flag.load(Ordering::SeqCst) {
                return None;
            }
            if self.test_archive_password(file_path, &password).await.is_ok_and(|matched| matched) {
                return Some(password);
            }
        }
        None
    }

    async fn attempt_recommended_dictionary(&self, window: &Window, task_id: &str, file_path: &str) -> Option<String> {
        let passwords = Self::recommended_dictionary(file_path);
        let total = passwords.len();
        self.emit_log(
            window,
            task_id,
            &format!("已授权密码字典尝试，共 {} 个候选", total),
            TaskLogSeverity::Info,
        );

        for (index, password) in passwords.into_iter().enumerate() {
            if self.cancellation_flag.load(Ordering::SeqCst) {
                return None;
            }
            let current = index + 1;
            if current == 1 || current % 10 == 0 || current == total {
                let _ = window.emit("task-progress", TaskProgress {
                    task_id: task_id.to_string(),
                    stage: Some("password-attempt".to_string()),
                    current_password: None,
                    progress: current as f32 / total.max(1) as f32,
                    speed: None,
                    eta_seconds: None,
                    current_file: None,
                    processed_bytes: 0,
                    total_bytes: 0,
                    password_attempt_current: Some(current),
                    password_attempt_total: Some(total),
                });
            }
            if self.test_archive_password(file_path, &password).await.is_ok_and(|matched| matched) {
                self.emit_log(
                    window,
                    task_id,
                    &format!("密码字典在第 {} 次尝试时匹配成功", current),
                    TaskLogSeverity::Success,
                );
                return Some(password);
            }
        }

        self.emit_log(
            window,
            task_id,
            &format!("密码字典已完成 {} 次尝试，未找到匹配项", total),
            TaskLogSeverity::Warning,
        );
        None
    }

    async fn password_book_candidates(&self) -> Result<Vec<(String, String, String)>> {
        use crate::services::password_query_service::{PasswordQueryRequest, SortField, SortOrder};

        let request = PasswordQueryRequest {
            sort_by: Some(SortField::UsageCount),
            sort_order: Some(SortOrder::Desc),
            page_size: Some(1000),
            include_decrypted: true,
            ..Default::default()
        };

        let response = self.password_query_service.search_passwords(&request).await?;
        Ok(response
            .data
            .into_iter()
            .map(|entry| (entry.id, entry.name, entry.password))
            .collect())
    }

    async fn attempt_password_book_candidates(&self, file_path: &str) -> Option<String> {
        let passwords = match self.password_book_candidates().await {
            Ok(passwords) => passwords,
            Err(err) => {
                log::error!("Failed to read password book candidates: {}", err);
                return None;
            }
        };

        for (entry_id, _entry_name, password) in passwords {
            match self.test_archive_password(file_path, &password).await {
                Ok(true) => {
                    let _ = self.password_query_service.increment_use_count(&entry_id).await;
                    return Some(password);
                }
                _ => continue,
            }
        }

        None
    }

    async fn attempt_passwords_smartly(&self, window: &Window, task_id: &str, file_path: &str) -> Option<String> {
        self.emit_log(window, task_id, "正在检索高频密码本...", TaskLogSeverity::Info);

        let passwords = match self.password_book_candidates().await {
            Ok(res) => res,
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

        for (idx, (entry_id, entry_name, pwd)) in passwords.iter().enumerate() {
            let current = idx + 1;

            // 发送密码尝试进度事件
            let _ = window.emit("task-progress", TaskProgress {
                task_id: task_id.to_string(),
                stage: Some("password-attempt".to_string()),
                current_password: Some(entry_name.clone()),
                progress: current as f32 / total as f32,
                speed: None,
                eta_seconds: None,
                current_file: None,
                processed_bytes: 0,
                total_bytes: 0,
                password_attempt_current: Some(current),
                password_attempt_total: Some(total),
            });

            self.emit_log(window, task_id, &format!("正在尝试已知密码 [{}/{}]: {}...", current, total, entry_name), TaskLogSeverity::Info);

            match self.test_archive_password(file_path, pwd).await {
                Ok(true) => {
                    self.emit_log(window, task_id, &format!("密码匹配成功 ({})", entry_name), TaskLogSeverity::Success);
                    let _ = self.password_query_service.increment_use_count(entry_id).await;
                    return Some(pwd.clone());
                },
                _ => continue,
            }
        }
        
        self.emit_log(window, task_id, "所有已知密码均匹配失败", TaskLogSeverity::Warning);
        None
    }

    async fn attempt_bruteforce_wordlists_silent(&self, file_path: &str, wordlists: &[String]) -> Option<String> {
        let mut tested = HashSet::new();

        for wordlist in wordlists {
            if self.cancellation_flag.load(Ordering::SeqCst) {
                return None;
            }

            let file = match File::open(Path::new(wordlist)) {
                Ok(file) => file,
                Err(_) => continue,
            };

            for line in BufReader::new(file).lines() {
                if self.cancellation_flag.load(Ordering::SeqCst) {
                    return None;
                }

                let password = match line {
                    Ok(value) => value.trim().trim_end_matches('\u{feff}').to_string(),
                    Err(_) => continue,
                };

                if password.is_empty() || !tested.insert(password.clone()) {
                    continue;
                }

                if matches!(self.test_archive_password(file_path, &password).await, Ok(true)) {
                    return Some(password);
                }
            }
        }

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

    async fn archive_requires_password(&self, file_path: &str, format: ArchiveFormat) -> Result<bool> {
        match format {
            // 通过 7z CLI 处理的格式
            ArchiveFormat::Universal | ArchiveFormat::Iso | ArchiveFormat::Cab |
            ArchiveFormat::Lzh | ArchiveFormat::Arj | ArchiveFormat::Dmg |
            ArchiveFormat::Wim | ArchiveFormat::Vhd | ArchiveFormat::Chm |
            ArchiveFormat::Deb | ArchiveFormat::Rpm | ArchiveFormat::SquashFs |
            ArchiveFormat::Nsis | ArchiveFormat::Msi | ArchiveFormat::Xar |
            ArchiveFormat::Cpio | ArchiveFormat::Udf | ArchiveFormat::Fat |
            ArchiveFormat::Ntfs | ArchiveFormat::Hfs | ArchiveFormat::Alz |
            ArchiveFormat::Arc | ArchiveFormat::Apfs | ArchiveFormat::Ext => {
                self.universal_engine.requires_password(Path::new(file_path)).await
            }
            ArchiveFormat::Zip => {
                UniversalCliEngine::zip_requires_password(Path::new(file_path))
            }
            ArchiveFormat::SevenZip => {
                native_extraction::seven_zip::requires_password(Path::new(file_path))
            }
            // RAR 仍交由完整引擎做无密码测试；7Z 可以直接读取 coder 元数据。
            ArchiveFormat::Rar => {
                self.universal_engine.requires_password(Path::new(file_path)).await
            }
            ArchiveFormat::AesEncrypted => {
                let path = Path::new(file_path);
                if TarAesEngine::is_tar_aes(path).unwrap_or(false)
                    || AesWrapper::is_aes_encrypted(path).unwrap_or(false)
                {
                    Ok(true)
                } else {
                    Err(anyhow::anyhow!("File has an .aes extension but no supported encrypted-container header"))
                }
            },
            _ => Ok(false),
        }
    }

    fn detect_password_format(file_path: &str) -> Result<ArchiveFormat> {
        let path = Path::new(file_path);
        let mut file = File::open(path)?;
        let mut header = [0u8; 512];
        let bytes_read = file.read(&mut header)?;
        let detected = ArchiveFormat::from_magic(&header[..bytes_read]);
        if detected != ArchiveFormat::Unknown {
            return Ok(detected);
        }

        let ext = path.extension().and_then(|value| value.to_str()).unwrap_or("").to_ascii_lowercase();
        Ok(ArchiveFormat::from_password_extension(&ext))
    }

    pub async fn verify_archive_password_candidate(&self, file_path: &str, password: &str) -> Result<bool> {
        self.check_cancellation()?;
        let format = Self::detect_password_format(file_path)?;

        if !self.archive_requires_password(file_path, format).await? {
            return Ok(false);
        }

        self.test_archive_password(file_path, password).await
    }

    pub async fn extract(&self, window: Window, task_id: String, file_path: String, output_dir: Option<String>, password: Option<String>, options: DecompressOptions) -> Result<String> {
        let service = self.clone();
        let path = Path::new(&file_path);
        if !path.is_file() {
            return Err(CompressionError::FileNotFound(file_path).into());
        }
        if !matches!(options.conflict_policy.as_str(), "ask" | "overwrite" | "skip" | "rename") {
            return Err(CompressionError::ExtractionFailed(format!(
                "Unsupported conflict policy: {}",
                options.conflict_policy
            )).into());
        }
        if options.delete_after
            && (options.skip_corrupted
                || options.file_filter.as_deref().is_some_and(|filter| !filter.trim().is_empty()))
        {
            return Err(CompressionError::ExtractionFailed(
                "The source archive cannot be deleted after a partial or corruption-tolerant extraction".to_string()
            ).into());
        }
        let mut final_out_dir = output_dir.map(PathBuf::from).unwrap_or_else(|| {
            path.parent().unwrap_or(Path::new(".")).to_path_buf()
        });
        if options.create_subdirectory {
            final_out_dir = final_out_dir.join(Self::archive_output_dir_name(path));
        }

        if final_out_dir.exists() && !final_out_dir.is_dir() {
            return Err(CompressionError::ExtractionFailed(format!(
                "Extraction output is not a directory: {}",
                final_out_dir.display()
            )).into());
        }
        if std::fs::symlink_metadata(&final_out_dir)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
        {
            return Err(CompressionError::ExtractionFailed(
                "Extraction output cannot be a symbolic link or reparse point".to_string()
            ).into());
        }
        let mut format = ArchiveFormat::Unknown;
        if let Ok(mut f) = File::open(&file_path) {
            let mut header = [0u8; 32];
            if let Ok(bytes_read) = f.read(&mut header) {
                if bytes_read > 0 {
                    format = ArchiveFormat::from_magic(&header);
                }
            }
        }

        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("未知文件").to_string();
        let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

        // 分卷压缩包检测：扩展名为数字编号(.001, .002...)或 .z01/.z02 后缀
        // 这些文件即使 magic bytes 匹配了 ZIP/7Z，也必须走 7z CLI 才能正确合并解压
        let is_split_archive = {
            let ext_is_numeric = ext.len() == 3 && ext.chars().all(|c| c.is_ascii_digit());
            let ext_is_zsplit = ext.len() == 3 && ext.starts_with('z') && ext[1..].chars().all(|c| c.is_ascii_digit());
            // 也检查 stem 是否以 .zip / .7z / .rar 结尾（如 test_split.zip.001）
            let stem_has_archive_ext = file_stem.ends_with(".zip") || file_stem.ends_with(".7z") || file_stem.ends_with(".rar");
            ext_is_numeric || ext_is_zsplit || stem_has_archive_ext
        };

        // 分卷文件强制走 7z CLI 通用引擎
        if is_split_archive && format != ArchiveFormat::Unknown && format != ArchiveFormat::Universal {
            service.emit_log(&window, &task_id, &format!("检测到分卷压缩包 (后缀: .{})，使用 7z CLI 合并解压", ext), TaskLogSeverity::Info);
            format = ArchiveFormat::Universal;
        }

        // 托底识别：如果 magic 识别失败，尝试根据后缀识别
        if format == ArchiveFormat::Unknown {
            format = ArchiveFormat::from_extension(&ext);
            if format != ArchiveFormat::Unknown {
                service.emit_log(&window, &task_id, &format!("Magic匹配失败，根据后缀识别为: {:?}", format), TaskLogSeverity::Warning);
            }
        }

        service.emit_log(&window, &task_id, &format!("确定解压格式: {:?} (后缀: {})", format, ext), TaskLogSeverity::Info);

        let mut final_password = password.clone();
        let password_required = if format.supports_password() {
            service.emit_log(
                &window,
                &task_id,
                "正在检测归档加密状态...",
                TaskLogSeverity::Info,
            );
            Some(service.archive_requires_password(&file_path, format.clone()).await.map_err(|err| {
                CompressionError::ExtractionFailed(format!(
                    "Unable to determine archive encryption state safely: {}",
                    err
                ))
            })?)
        } else {
            None
        };

        if password_required == Some(false) && final_password.is_some() {
            service.emit_log(&window, &task_id, "归档未加密，已忽略多余的密码参数", TaskLogSeverity::Info);
            final_password = None;
        }

        if final_password.is_none() && password_required == Some(true) {
            let needs_pwd = true;

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
                        file_name,
                        format: format!("{:?}", format),
                    });
                    
                    return Err(CompressionError::PasswordRequired.into());
                }
            }
        }

        service
            .preflight_extraction(path, &format, final_password.as_deref(), &final_out_dir)
            .await?;
        let mark_of_web = if options.preserve_mark_of_web {
            mark_of_web::read_from(path).map_err(|error| {
                CompressionError::ExtractionFailed(format!(
                    "Unable to inspect the archive's Windows security zone: {}",
                    error
                ))
            })?
        } else {
            None
        };
        if let Some(mark) = mark_of_web.as_ref() {
            service.emit_log(
                &window,
                &task_id,
                &format!(
                    "检测到互联网来源安全标记 (ZoneId={})，将在事务提交时传播到输出文件",
                    mark.zone_id()
                ),
                TaskLogSeverity::Info,
            );
        }
        self.check_cancellation()?;
        Self::ensure_no_link_ancestors(&final_out_dir)?;
        let mut staging = ExtractionStaging::create_for(&final_out_dir)?;
        let out_dir = staging.path().to_path_buf();

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

        let effective_format = if format == ArchiveFormat::Zip && final_password.is_some() {
            service.emit_log(&window, &task_id, "检测到加密 ZIP，使用内置加密 ZIP 兼容引擎解压", TaskLogSeverity::Info);
            ArchiveFormat::Universal
        } else {
            format
        };

        let result = match effective_format {
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
            ArchiveFormat::AesEncrypted => {
                let srv = service.clone();
                let f_path = file_path.clone();
                let o_dir = out_dir.clone();
                let pwd = final_password.clone();
                let opts = options.clone();
                let t_id = task_id.clone();
                let w = window.clone();
                tokio::task::spawn_blocking(move || {
                    srv.do_extract_aes(
                        &w,
                        &t_id,
                        &f_path,
                        &o_dir,
                        pwd.as_deref(),
                        &opts,
                    )
                })
                .await?
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
                    if compression_format::is_tar_wrapped_archive(Path::new(&f_path), "tar.gz") {
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
                    if compression_format::is_tar_wrapped_archive(Path::new(&f_path), "tar.bz2") {
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
                    if compression_format::is_tar_wrapped_archive(Path::new(&f_path), "tar.xz") {
                        srv.do_extract_tar_xz(&w, &t_id, &f_path, &o_dir, &opts)
                    } else {
                        srv.do_extract_xz(&w, &t_id, &f_path, &o_dir, &opts)
                    }
                }).await?
            },
            ArchiveFormat::Zstd => {
                let srv = service.clone();
                let f_path = file_path.clone();
                let o_dir = out_dir.clone();
                let opts = options.clone();
                let t_id = task_id.clone();
                let w = window.clone();
                tokio::task::spawn_blocking(move || {
                    if compression_format::is_tar_wrapped_archive(Path::new(&f_path), "tar.zst") {
                        srv.do_extract_tar_zstd(&w, &t_id, &f_path, &o_dir, &opts)
                    } else {
                        srv.do_extract_zstd(&w, &t_id, &f_path, &o_dir, &opts)
                    }
                }).await?
            },
            // 所有通过 7z CLI 处理的格式
            ArchiveFormat::Lzma | ArchiveFormat::Iso | ArchiveFormat::Cab |
            ArchiveFormat::Lzh | ArchiveFormat::Arj | ArchiveFormat::Dmg |
            ArchiveFormat::Wim | ArchiveFormat::Vhd | ArchiveFormat::Chm |
            ArchiveFormat::Deb | ArchiveFormat::Rpm | ArchiveFormat::SquashFs |
            ArchiveFormat::Nsis | ArchiveFormat::Msi | ArchiveFormat::Xar |
            ArchiveFormat::Cpio | ArchiveFormat::Udf | ArchiveFormat::Fat |
            ArchiveFormat::Ntfs | ArchiveFormat::Hfs | ArchiveFormat::Alz |
            ArchiveFormat::Arc | ArchiveFormat::Apfs | ArchiveFormat::Ext |
            ArchiveFormat::Universal => {
                let fmt_name = format!("{:?}", effective_format);
                let extraction_result = service.universal_engine.extract_with_progress(
                    Path::new(&file_path),
                    &out_dir,
                    final_password.as_deref(),
                    options.overwrite_existing,
                    on_progress.clone(),
                    on_log.clone(),
                    service.cancellation_flag.clone()
                ).await.map_err(|e| anyhow::anyhow!("{}提取失败: {}", fmt_name, e));

                if extraction_result.is_ok() && ext == "msm" {
                    let merge_module_cabinet = out_dir.join("MergeModule.CABinet");
                    if merge_module_cabinet.is_file() {
                        service.emit_log(
                            &window,
                            &task_id,
                            "检测到 MSM 内嵌 CAB，正在继续提取真实载荷",
                            TaskLogSeverity::Info,
                        );
                        service.universal_engine.extract_with_progress(
                            &merge_module_cabinet,
                            &out_dir,
                            None,
                            true,
                            on_progress,
                            on_log,
                            service.cancellation_flag.clone(),
                        ).await.map_err(|e| anyhow::anyhow!(
                            "MSM 内嵌 CAB 提取失败: {}",
                            e
                        ))?;
                    }
                }

                extraction_result
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

        if let Err(error) = result.map_err(Self::normalize_storage_full_error) {
            if let Err(cleanup_error) = staging.cleanup() {
                service.emit_log(
                    &window,
                    &task_id,
                    &format!("Unable to clean incomplete extraction output: {}", cleanup_error),
                    TaskLogSeverity::Warning,
                );
            }
            return Err(error);
        }
        if let Err(error) = service.prepare_staging_layout(path, &out_dir, &options) {
            let _ = staging.cleanup();
            return Err(error);
        }
        service.check_cancellation()?;
        if let Some(mark) = mark_of_web.as_ref() {
            match mark_of_web::propagate_to_tree(&out_dir, mark, || {
                service.cancellation_flag.load(Ordering::Relaxed)
            }) {
                Ok(PropagationStatus::Applied(count)) => {
                    service.emit_log(
                        &window,
                        &task_id,
                        &format!("已为 {} 个待提交文件保留互联网来源安全标记", count),
                        TaskLogSeverity::Success,
                    );
                }
                Ok(PropagationStatus::Unsupported) => {
                    service.emit_log(
                        &window,
                        &task_id,
                        "目标文件系统不支持 Windows 来源安全标记，将继续完成解压",
                        TaskLogSeverity::Warning,
                    );
                }
                Err(error) => {
                    let _ = staging.cleanup();
                    if error.kind() == std::io::ErrorKind::Interrupted {
                        service.check_cancellation()?;
                    }
                    return Err(CompressionError::ExtractionFailed(format!(
                        "Unable to preserve the archive's Windows security zone safely: {}",
                        error
                    ))
                    .into());
                }
            }
        }
        service.check_cancellation()?;
        if let Err(error) = service.commit_staged_extraction(
            Some(&window),
            &task_id,
            &file_path,
            &out_dir,
            &final_out_dir,
            &options,
        ) {
            let _ = staging.cleanup();
            return Err(error);
        }
        if let Err(error) = staging.cleanup() {
            service.emit_log(
                &window,
                &task_id,
                &format!(
                    "Extraction completed, but the temporary staging directory could not be removed: {}",
                    error
                ),
                TaskLogSeverity::Warning,
            );
        }
        if options.delete_after {
            if let Err(error) = std::fs::remove_file(&file_path) {
                service.emit_log(
                    &window,
                    &task_id,
                    &format!("Extraction succeeded, but the source archive could not be deleted: {}", error),
                    TaskLogSeverity::Warning,
                );
            }
        }
        service.emit_log(&window, &task_id, "全部解压任务已完成", TaskLogSeverity::Success);
        service.emit_progress(&window, &task_id, 1.0, None, 0, 0);
        Ok(final_out_dir.to_string_lossy().to_string())
    }

    fn validate_resource_limits(
        archive_path: &Path,
        entry_count: usize,
        expanded_bytes: u64,
    ) -> Result<()> {
        extraction_transaction::validate_resource_limits(
            archive_path,
            entry_count,
            expanded_bytes,
        )
    }

    fn ensure_no_link_ancestors(path: &Path) -> Result<()> {
        extraction_transaction::ensure_no_link_ancestors(path)
    }

    async fn preflight_extraction(
        &self,
        archive_path: &Path,
        format: &ArchiveFormat,
        password: Option<&str>,
        output: &Path,
    ) -> Result<()> {
        let stats = match format {
            ArchiveFormat::Zip => {
                let file = File::open(archive_path)?;
                let mut archive = zip_aes::ZipArchive::new(file)?;
                let mut expanded = 0u64;
                for index in 0..archive.len() {
                    expanded = expanded
                        .checked_add(archive.by_index_raw(index)?.size())
                        .ok_or_else(|| CompressionError::ExtractionFailed(
                            "Archive expanded size overflowed the supported range".to_string(),
                        ))?;
                }
                Some((archive.len(), expanded))
            }
            ArchiveFormat::SevenZip => {
                let reader = sevenz_rust::SevenZReader::open(
                    archive_path,
                    sevenz_rust::Password::from(password.unwrap_or("")),
                )?;
                let mut expanded = 0u64;
                for entry in &reader.archive().files {
                    expanded = expanded.checked_add(entry.size).ok_or_else(|| {
                        CompressionError::ExtractionFailed(
                            "Archive expanded size overflowed the supported range".to_string(),
                        )
                    })?;
                }
                Some((reader.archive().files.len(), expanded))
            }
            ArchiveFormat::Rar => {
                let path = archive_path.to_str().ok_or_else(|| {
                    CompressionError::ExtractionFailed("RAR path is not valid Unicode".to_string())
                })?;
                let archive = if let Some(password) = password {
                    unrar::Archive::with_password(path, password)
                } else {
                    unrar::Archive::new(path)
                };
                let mut archive = archive.open_for_processing().map_err(|error| {
                    CompressionError::ExtractionFailed(format!(
                        "Unable to inspect RAR metadata safely: {:?}",
                        error
                    ))
                })?;
                let mut entries = 0usize;
                let mut expanded = 0u64;
                while let Some(header) = archive.read_header().map_err(|error| {
                    CompressionError::ExtractionFailed(format!(
                        "Unable to inspect RAR entry metadata: {:?}",
                        error
                    ))
                })? {
                    entries = entries.saturating_add(1);
                    expanded = expanded.checked_add(header.entry().unpacked_size).ok_or_else(|| {
                        CompressionError::ExtractionFailed(
                            "RAR expanded size overflowed the supported range".to_string(),
                        )
                    })?;
                    archive = header.skip().map_err(|error| {
                        CompressionError::ExtractionFailed(format!(
                            "Unable to continue RAR metadata inspection: {:?}",
                            error
                        ))
                    })?;
                }
                Some((entries, expanded))
            }
            _ if password.is_none() => {
                UniversalCliEngine::archive_uncompressed_stats(archive_path).await.ok()
            }
            _ => None,
        };

        if let Some((entry_count, expanded_bytes)) = stats {
            Self::validate_resource_limits(archive_path, entry_count, expanded_bytes)?;
            let disk_probe = output.parent().unwrap_or(output);
            extraction_transaction::validate_disk_capacity(disk_probe, expanded_bytes)?;
        }
        Ok(())
    }

    fn prepare_staging_layout(
        &self,
        archive_path: &Path,
        staging: &Path,
        options: &DecompressOptions,
    ) -> Result<()> {
        extraction_transaction::prepare_staging_layout(archive_path, staging, options)
    }

    fn archive_output_dir_name(path: &Path) -> String {
        let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("archive");
        for suffix in [".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst"] {
            if file_name.to_lowercase().ends_with(suffix) {
                return file_name[..file_name.len() - suffix.len()].to_string();
            }
        }
        path.file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("archive")
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

    pub fn do_extract_zip(&self, window: &Window, task_id: &str, file: &str, output: &str, password: Option<&str>, options: &DecompressOptions) -> Result<()> {
        native_extraction::zip::extract(self, window, task_id, file, output, password, options)
    }

    pub fn do_extract_7z(&self, window: &Window, task_id: &str, file: &str, output: &str, password: Option<&str>, options: &DecompressOptions) -> Result<()> {
        native_extraction::seven_zip::extract(
            self,
            Some(window),
            task_id,
            file,
            output,
            password,
            options,
        )
    }

    fn do_extract_tar(&self, window: &Window, task_id: &str, file: &str, output: &Path, decoder: Option<Box<dyn Read + Send>>, options: &DecompressOptions) -> Result<()> {
        native_extraction::tar::extract(self, window, task_id, file, output, decoder, options)
    }

    fn do_extract_tar_gz(&self, w: &Window, tid: &str, file: &str, output: &Path, options: &DecompressOptions) -> Result<()> {
        native_extraction::tar::extract_gzip(self, w, tid, file, output, options)
    }

    fn do_extract_tar_bz2(&self, w: &Window, tid: &str, file: &str, output: &Path, options: &DecompressOptions) -> Result<()> {
        native_extraction::tar::extract_bzip2(self, w, tid, file, output, options)
    }

    fn do_extract_tar_xz(&self, w: &Window, tid: &str, file: &str, output: &Path, options: &DecompressOptions) -> Result<()> {
        native_extraction::tar::extract_xz(self, w, tid, file, output, options)
    }

    fn do_extract_gz(&self, w: &Window, tid: &str, file: &str, output: &Path, options: &DecompressOptions) -> Result<()> {
        native_extraction::single_stream::extract_gzip(self, w, tid, file, output, options)
    }

    fn do_extract_bz2(&self, w: &Window, tid: &str, file: &str, output: &Path, options: &DecompressOptions) -> Result<()> {
        native_extraction::single_stream::extract_bzip2(self, w, tid, file, output, options)
    }

    fn do_extract_xz(&self, w: &Window, tid: &str, file: &str, output: &Path, options: &DecompressOptions) -> Result<()> {
        native_extraction::single_stream::extract_xz(self, w, tid, file, output, options)
    }

    fn do_extract_zstd(&self, w: &Window, tid: &str, file: &str, output: &Path, options: &DecompressOptions) -> Result<()> {
        native_extraction::single_stream::extract_zstandard(self, w, tid, file, output, options)
    }

    fn do_extract_tar_zstd(&self, w: &Window, tid: &str, file: &str, output: &Path, options: &DecompressOptions) -> Result<()> {
        native_extraction::tar::extract_zstandard(self, w, tid, file, output, options)
    }

    fn map_aes_extraction_error(error: anyhow::Error) -> anyhow::Error {
        if matches!(
            error.downcast_ref::<CompressionError>(),
            Some(CompressionError::Cancelled)
        ) {
            return CompressionError::Cancelled.into();
        }
        let message = error.to_string();
        if message.contains("密码错误") || message.contains("解密失败") {
            CompressionError::InvalidPassword.into()
        } else {
            CompressionError::ExtractionFailed(message).into()
        }
    }

    fn do_extract_aes(
        &self,
        window: &Window,
        task_id: &str,
        file: &str,
        output: &Path,
        password: Option<&str>,
        options: &DecompressOptions,
    ) -> Result<()> {
        let password = password.ok_or(CompressionError::PasswordRequired)?;
        let source = Path::new(file);

        if TarAesEngine::is_tar_aes(source).unwrap_or(false) {
            self.emit_log(window, task_id, "正在解密并解压 TAR.AES", TaskLogSeverity::Info);
            return TarAesEngine::decompress_tar_aes_cancellable(
                source,
                output,
                password,
                || self.check_cancellation(),
            )
            .map_err(Self::map_aes_extraction_error);
        }

        if !AesWrapper::is_aes_encrypted(source).unwrap_or(false) {
            return Err(CompressionError::ExtractionFailed(
                "无法识别的 AES 加密归档".to_string(),
            )
            .into());
        }

        let source_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("archive.aes");
        let inner_name = source_name
            .strip_suffix(".aes")
            .or_else(|| source_name.strip_suffix(".AES"))
            .filter(|name| !name.is_empty())
            .ok_or_else(|| {
                CompressionError::ExtractionFailed(
                    "AES 归档文件名缺少内层格式，例如 .tar.gz.aes".to_string(),
                )
            })?;
        let temporary_root = std::env::temp_dir().join(format!(
            "long-compress-aes-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&temporary_root)?;
        let decrypted = temporary_root.join(inner_name);

        let result = (|| -> Result<()> {
            AesWrapper::decrypt_file_cancellable(
                source,
                &decrypted,
                password,
                || self.check_cancellation(),
            )
            .map_err(Self::map_aes_extraction_error)?;

            let lower_name = inner_name.to_lowercase();
            let decrypted_path = decrypted.to_string_lossy();
            if lower_name.ends_with(".tar.gz") || lower_name.ends_with(".tgz") {
                self.do_extract_tar_gz(window, task_id, &decrypted_path, output, options)
            } else if lower_name.ends_with(".tar.bz2")
                || lower_name.ends_with(".tbz")
                || lower_name.ends_with(".tbz2")
            {
                self.do_extract_tar_bz2(window, task_id, &decrypted_path, output, options)
            } else if lower_name.ends_with(".tar.xz") || lower_name.ends_with(".txz") {
                self.do_extract_tar_xz(window, task_id, &decrypted_path, output, options)
            } else if lower_name.ends_with(".tar.zst") || lower_name.ends_with(".tzst") {
                self.do_extract_tar_zstd(window, task_id, &decrypted_path, output, options)
            } else if lower_name.ends_with(".gz") || lower_name.ends_with(".gzip") {
                self.do_extract_gz(window, task_id, &decrypted_path, output, options)
            } else if lower_name.ends_with(".bz2") || lower_name.ends_with(".bzip2") {
                self.do_extract_bz2(window, task_id, &decrypted_path, output, options)
            } else if lower_name.ends_with(".xz") {
                self.do_extract_xz(window, task_id, &decrypted_path, output, options)
            } else if lower_name.ends_with(".zst") || lower_name.ends_with(".zstd") {
                self.do_extract_zstd(window, task_id, &decrypted_path, output, options)
            } else {
                Err(CompressionError::ExtractionFailed(format!(
                    "不支持的 AES 内层格式: {}",
                    inner_name
                ))
                .into())
            }
        })();

        let _ = std::fs::remove_dir_all(&temporary_root);
        result
    }

    fn do_compress_zip(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        native_compression::zip::compress(self, Some(window), task_id, sources, output, options)
    }

    /// 使用原生 zstd 进行 Zstd 压缩。
    /// 当需要密码时，使用 7z 作为加密容器（AES-256）。
    /// 对于不支持原生加密的格式（TAR, GZ, BZ2, XZ, Zstd, LZMA 等），
    /// 自动路由到 do_compress_7z 并将输出扩展名改为 .7z。
    fn maybe_delegate_to_7z_for_password(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: &CompressionOptions, format_label: &str) -> Result<bool> {
        if options.password.as_deref().is_some_and(|p| !p.is_empty()) {
            let base = output.rsplit_once('.').map(|(b, _)| b).unwrap_or(output);
            let output_7z = format!("{}.7z", base);
            self.emit_log(window, task_id,
                &format!("{} 不支持原生加密; 自动切换为 7z 格式 (AES-256)", format_label),
                TaskLogSeverity::Info);
            self.do_compress_7z(window, task_id, sources, &output_7z, options.clone())?;
            return Ok(true);
        }
        Ok(false)
    }

    fn do_compress_zstd(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        if self.maybe_delegate_to_7z_for_password(window, task_id, sources, output, &options, "Zstd")? {
            return Ok(());
        }
        native_compression::single_stream::compress_zstd(
            self, Some(window), task_id, sources, output, options,
        )
    }

    /// 使用原生 tar + zstd 进行 tar.zst 压缩
    fn do_compress_tar_zstd(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        if self.maybe_delegate_to_7z_for_password(window, task_id, sources, output, &options, "TAR.Zst")? {
            return Ok(());
        }
        native_compression::tar::compress_zstd(
            self, Some(window), task_id, sources, output, options,
        )
    }

    /// 使用 7z CLI 进行 LZMA 压缩
    fn do_compress_lzma(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        if self.maybe_delegate_to_7z_for_password(window, task_id, sources, output, &options, "LZMA")? {
            return Ok(());
        }
        native_compression::single_stream::compress_lzma(
            self, Some(window), task_id, sources, output, options,
        )
    }

    fn do_compress_tar(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        if self.maybe_delegate_to_7z_for_password(window, task_id, sources, output, &options, "TAR")? {
            return Ok(());
        }
        native_compression::tar::compress_tar(
            self, Some(window), task_id, sources, output, options,
        )
    }

    fn do_compress_tar_gz(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        if self.maybe_delegate_to_7z_for_password(window, task_id, sources, output, &options, "TAR.GZ")? {
            return Ok(());
        }
        native_compression::tar::compress_gzip(
            self, Some(window), task_id, sources, output, options,
        )
    }

    fn do_compress_tar_bz2(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        if self.maybe_delegate_to_7z_for_password(window, task_id, sources, output, &options, "TAR.BZ2")? {
            return Ok(());
        }
        native_compression::tar::compress_bzip2(
            self, Some(window), task_id, sources, output, options,
        )
    }

    fn do_compress_tar_xz(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        if self.maybe_delegate_to_7z_for_password(window, task_id, sources, output, &options, "TAR.XZ")? {
            return Ok(());
        }
        native_compression::tar::compress_xz(
            self, Some(window), task_id, sources, output, options,
        )
    }

    fn do_compress_tar_aes(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        native_compression::aes::compress(
            self, Some(window), task_id, sources, output, options,
            native_compression::aes::AesCompressionKind::Tar,
        )
    }

    fn do_compress_tar_gz_aes(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        native_compression::aes::compress(
            self, Some(window), task_id, sources, output, options,
            native_compression::aes::AesCompressionKind::TarGzip,
        )
    }

    fn do_compress_tar_bz2_aes(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        native_compression::aes::compress(
            self, Some(window), task_id, sources, output, options,
            native_compression::aes::AesCompressionKind::TarBzip2,
        )
    }

    fn do_compress_tar_xz_aes(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        native_compression::aes::compress(
            self, Some(window), task_id, sources, output, options,
            native_compression::aes::AesCompressionKind::TarXz,
        )
    }

    fn do_compress_tar_zst_aes(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        native_compression::aes::compress(
            self, Some(window), task_id, sources, output, options,
            native_compression::aes::AesCompressionKind::TarZstd,
        )
    }

    fn do_compress_gz_aes(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        native_compression::aes::compress(
            self, Some(window), task_id, sources, output, options,
            native_compression::aes::AesCompressionKind::Gzip,
        )
    }

    fn do_compress_bz2_aes(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        native_compression::aes::compress(
            self, Some(window), task_id, sources, output, options,
            native_compression::aes::AesCompressionKind::Bzip2,
        )
    }

    fn do_compress_xz_aes(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        native_compression::aes::compress(
            self, Some(window), task_id, sources, output, options,
            native_compression::aes::AesCompressionKind::Xz,
        )
    }

    fn do_compress_zst_aes(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        native_compression::aes::compress(
            self, Some(window), task_id, sources, output, options,
            native_compression::aes::AesCompressionKind::Zstd,
        )
    }

    fn do_compress_gz(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        if self.maybe_delegate_to_7z_for_password(window, task_id, sources, output, &options, "GZ")? {
            return Ok(());
        }
        native_compression::single_stream::compress_gzip(
            self, Some(window), task_id, sources, output, options,
        )
    }

    fn do_compress_bz2(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        if self.maybe_delegate_to_7z_for_password(window, task_id, sources, output, &options, "BZ2")? {
            return Ok(());
        }
        native_compression::single_stream::compress_bzip2(
            self, Some(window), task_id, sources, output, options,
        )
    }

    fn do_compress_xz(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        if self.maybe_delegate_to_7z_for_password(window, task_id, sources, output, &options, "XZ")? {
            return Ok(());
        }
        native_compression::single_stream::compress_xz(
            self, Some(window), task_id, sources, output, options,
        )
    }

    fn do_compress_7z(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        native_compression::seven_zip::compress(
            self,
            Some(window),
            task_id,
            sources,
            output,
            options,
        )
    }

    pub fn find_rar_encoder() -> Option<String> {
        // Only use the console encoder. WinRAR.exe is a GUI application and may
        // display windows or modal errors even when the child console is hidden.
        let command = "rar";
        let exists = if cfg!(target_os = "windows") {
            crate::utils::process::command("where")
                .arg(command)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|status| status.success())
                .unwrap_or(false)
        } else {
            crate::utils::process::command("which")
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

        #[cfg(target_os = "windows")]
        {
            for path in [
                "C:\\Program Files\\WinRAR\\Rar.exe",
                "C:\\Program Files (x86)\\WinRAR\\Rar.exe",
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

        let mut command = crate::utils::process::command(encoder);
        command.arg("a");
        command.arg("-idq");
        command.arg("-y");
        command.arg(format!("-m{}", options.level.clamp(1, 5)));

        if options.preserve_paths == Some(false) {
            command.arg("-ep");
        }

        if let Some(password) = options.password.as_deref().filter(|password| !password.is_empty()) {
            if !options.allow_insecure_password_cli {
                return Err(CompressionError::CompressionFailed(
                    "Password-protected RAR creation requires explicit approval because RAR.exe exposes the password briefly in local process arguments. Use encrypted ZIP or 7Z to avoid this risk.".to_string()
                ).into());
            }
            command.arg(format!("-hp{}", password));
        }

        command.arg(output);
        for source in sources {
            self.check_cancellation()?;
            command.arg(source);
        }

        let output_result = self.run_command_cancellable(command)?;

        if !output_result.status.success() {
            let stderr = String::from_utf8_lossy(&output_result.stderr);
            let stdout = String::from_utf8_lossy(&output_result.stdout);
            let message = if stderr.trim().is_empty() { stdout.trim() } else { stderr.trim() };
            return Err(CompressionError::CompressionFailed(format!("RAR compression failed: {}", message)).into());
        }

        self.emit_progress(window, task_id, 1.0, Some(output.to_string()), 0, 0);
        Ok(())
    }

    fn do_compress_wim(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        if !output.to_ascii_lowercase().ends_with(".wim") {
            return Err(CompressionError::CompressionFailed("WIM compression output path must end with .wim".to_string()).into());
        }
        if options.password.as_deref().is_some_and(|password| !password.is_empty()) {
            return Err(CompressionError::UnsupportedEncryption.into());
        }
        if options.split_size.is_some_and(|size| size > 0) {
            return Err(CompressionError::CompressionFailed("WIM split creation is not supported yet.".to_string()).into());
        }
        if !crate::utils::archive_tools::archive_engine_can_create("wim") {
            return Err(CompressionError::CompressionFailed("The active archive engine cannot create WIM files.".to_string()).into());
        }
        if let Some(parent) = Path::new(output).parent() { std::fs::create_dir_all(parent)?; }
        let engine = find_7z_command().ok_or_else(|| CompressionError::CompressionFailed(missing_7z_message()))?;
        let mut command = crate::utils::process::command(engine);
        command.arg("a").arg("-twim").arg("-y").arg(format!("-mx{}", options.level.clamp(1, 9))).arg(output);
        for source in sources { self.check_cancellation()?; command.arg(source); }
        let result = self.run_command_cancellable(command)?;
        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            let stdout = String::from_utf8_lossy(&result.stdout);
            let detail = if stderr.trim().is_empty() { stdout.trim() } else { stderr.trim() };
            return Err(CompressionError::CompressionFailed(format!("WIM compression failed: {detail}")).into());
        }
        self.emit_progress(window, task_id, 1.0, Some(output.to_string()), 0, 0);
        Ok(())
    }

    pub async fn test_archive_password(&self, file_path: &str, password: &str) -> Result<bool> {
        self.check_cancellation()?;
        let file = file_path.to_string();
        let pwd = password.to_string();
        let path = Path::new(&file);
        let format = Self::detect_password_format(file_path)?;

        // 该接口的 true 只能表示“文件确实加密且密码正确”。
        // 未加密归档即使携带任意密码也能读取，仍必须返回 false。
        if !self.archive_requires_password(file_path, format.clone()).await? {
            return Ok(false);
        }

        if TarAesEngine::is_tar_aes(path).unwrap_or(false) {
            return TarAesEngine::verify_password_cancellable(path, password, || {
                self.check_cancellation()
            });
        }
        if AesWrapper::is_aes_encrypted(path).unwrap_or(false) {
            return AesWrapper::verify_password_cancellable(path, password, || {
                self.check_cancellation()
            });
        }

        if format == ArchiveFormat::Zip {
            return UniversalCliEngine::try_zip_password(path, password);
        }
        if format == ArchiveFormat::Rar {
            return Ok(self.rar_service.test_rar_password(path, password).await);
        }

        let cancellation_flag = self.cancellation_flag.clone();
        tokio::task::spawn_blocking(move || {
            match format {
                ArchiveFormat::SevenZip => {
                    let scratch = std::env::temp_dir().join(format!(
                        "long-compress-password-check-{}",
                        uuid::Uuid::new_v4()
                    ));
                    std::fs::create_dir_all(&scratch)?;
                    let mut tested_file = false;
                    let mut verify_entry = |entry: &sevenz_rust::SevenZArchiveEntry,
                                            reader: &mut dyn Read,
                                            _destination: &PathBuf|
                     -> Result<bool, sevenz_rust::Error> {
                        if !entry.is_directory() {
                            if cancellation_flag.load(Ordering::SeqCst) {
                                return Err(sevenz_rust::Error::other("Password verification cancelled"));
                            }
                            std::io::copy(reader, &mut std::io::sink())
                                .map_err(sevenz_rust::Error::io)?;
                            tested_file = true;
                        }
                        Ok(true)
                    };
                    let result = sevenz_rust::decompress_with_extract_fn_and_password(
                        File::open(&file)?,
                        &scratch,
                        sevenz_rust::Password::from(pwd.as_str()),
                        &mut verify_entry,
                    );
                    let _ = std::fs::remove_dir_all(&scratch);
                    Ok(result.is_ok() && tested_file)
                },
                _ => Ok(false)
            }
        }).await?
    }

    pub async fn compress_zip_enhanced(&self, sources: &[String], output: &str, _options: CompressionOptions) -> Result<()> {
        let sources = sources.to_vec();
        let output = output.to_string();
        let cancellation_flag = self.cancellation_flag.clone();
        tokio::task::spawn_blocking(move || {
            if sources.is_empty() {
                return Err(CompressionError::CompressionFailed(
                    "At least one source file is required".to_string(),
                )
                .into());
            }
            for source in &sources {
                let path = Path::new(source);
                if !path.exists() {
                    return Err(CompressionError::FileNotFound(source.clone()).into());
                }
                if !path.is_file() {
                    return Err(CompressionError::CompressionFailed(format!(
                        "Enhanced ZIP compression requires regular files: {}",
                        source
                    ))
                    .into());
                }
            }

            let file = File::create(&output)?;
            let mut zip = zip::ZipWriter::new(file);
            let zip_options = FileOptions::default().compression_method(CompressionMethod::Deflated);
            let mut buffer = vec![0u8; Self::COPY_BUFFER_SIZE];
            for source in sources {
                let path = Path::new(&source);
                if path.is_file() {
                    let entry_name = path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("unknown");
                    zip.start_file(entry_name, zip_options)?;
                    let mut f = File::open(path)?;
                    loop {
                        if cancellation_flag.load(Ordering::Relaxed) {
                            return Err(CompressionError::Cancelled.into());
                        }
                        let read = f.read(&mut buffer)?;
                        if read == 0 {
                            break;
                        }
                        zip.write_all(&buffer[..read])?;
                    }
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
    fn recognizes_application_native_aes_headers() {
        assert_eq!(
            ArchiveFormat::from_magic(b"TARAES01payload"),
            ArchiveFormat::AesEncrypted
        );
        assert_eq!(
            ArchiveFormat::from_magic(b"AESENC01payload"),
            ArchiveFormat::AesEncrypted
        );
        assert_eq!(
            ArchiveFormat::from_magic(b"TARAES02payload"),
            ArchiveFormat::AesEncrypted
        );
        assert_eq!(
            ArchiveFormat::from_magic(b"AESENC02payload"),
            ArchiveFormat::AesEncrypted
        );
        assert!(ArchiveFormat::AesEncrypted.supports_password());
    }

    #[tokio::test]
    async fn validates_application_native_aes_passwords() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("payload.gz");
        let encrypted = temp.path().join("payload.gz.aes");
        std::fs::write(&input, b"compressed payload").expect("write fixture");
        AesWrapper::encrypt_file(&input, &encrypted, "correct-password")
            .expect("encrypt fixture");
        let service = CompressionService::new_with_defaults().await;

        assert!(service
            .test_archive_password(&encrypted.to_string_lossy(), "correct-password")
            .await
            .expect("correct password check"));
        assert!(!service
            .test_archive_password(&encrypted.to_string_lossy(), "wrong-password")
            .await
            .expect("wrong password check"));
    }

    #[test]
    fn injected_storage_full_cleans_unpublished_compression_outputs() {
        let temp = tempfile::tempdir().expect("temp dir");
        let final_output = temp.path().join("archive.zip");
        let working_output =
            CompressionService::temporary_compression_output(&final_output).expect("working path");
        let sidecar = working_output.with_file_name(format!(
            "{}.sidecar",
            working_output
                .file_name()
                .and_then(|name| name.to_str())
                .expect("working file name")
        ));
        std::fs::write(&working_output, b"partial archive").expect("partial output");
        std::fs::write(&sidecar, b"partial sidecar").expect("partial sidecar");

        let error = CompressionService::finalize_compression_output(
            Err(std::io::Error::new(
                std::io::ErrorKind::StorageFull,
                "simulated compression storage full",
            )
            .into()),
            &working_output,
            &final_output,
            false,
        )
        .expect_err("storage-full compression must fail");

        assert!(matches!(
            error.downcast_ref::<CompressionError>(),
            Some(CompressionError::DiskFull)
        ));
        assert!(!working_output.exists());
        assert!(!sidecar.exists());
        assert!(!final_output.exists());
    }

}

impl CompressionService {
    #[cfg(test)]
    fn staged_file_is_not_newer(source: &Path, destination: &Path) -> bool {
        extraction_transaction::staged_file_is_not_newer(source, destination)
    }

    fn commit_staged_extraction(
        &self,
        window: Option<&Window>,
        task_id: &str,
        source_archive: &str,
        staging: &Path,
        output: &Path,
        options: &DecompressOptions,
    ) -> Result<()> {
        extraction_transaction::commit_staged_extraction(
            source_archive,
            staging,
            output,
            options,
            |conflict| {
                if let Some(window) = window {
                    let _ = window.emit(
                        "file-conflict",
                        FileConflictPayload {
                            task_id: task_id.to_string(),
                            file_name: conflict.file_name,
                            source_path: conflict.source_path,
                            dest_path: conflict.dest_path,
                            source_size: conflict.source_size,
                            dest_size: conflict.dest_size,
                            source_modified: conflict.source_modified,
                            dest_modified: conflict.dest_modified,
                        },
                    );
                }
            },
        )
    }
}

#[cfg(test)]
mod tests_continued {
    use super::*;

    #[tokio::test]
    async fn progress_telemetry_reports_real_speed_and_eta() {
        let service = CompressionService::for_testing();
        service.progress_metrics.lock().expect("progress metrics").insert(
            "telemetry-task".to_string(),
            ProgressMetric {
                started_at: Instant::now() - std::time::Duration::from_secs(2),
                last_bytes: 0,
            },
        );

        let (speed, eta) = service.progress_telemetry(
            "telemetry-task",
            0.5,
            5 * 1024 * 1024,
            10 * 1024 * 1024,
        );

        assert!(speed.as_deref().is_some_and(|value| value.ends_with(" MB/s")));
        assert!(matches!(eta, Some(2..=3)));
    }

    #[tokio::test]
    async fn progress_telemetry_reports_short_task_throughput() {
        let service = CompressionService::for_testing();
        service.begin_progress_telemetry("short-telemetry-task");
        std::thread::sleep(std::time::Duration::from_millis(1));

        let (speed, eta) = service.progress_telemetry(
            "short-telemetry-task",
            1.0,
            4 * 1024 * 1024,
            4 * 1024 * 1024,
        );

        assert!(speed.is_some());
        assert_eq!(eta, Some(0));
    }

    #[test]
    fn extraction_resource_limits_reject_oversized_archives() {
        let temp = tempfile::tempdir().expect("temp dir");
        let archive = temp.path().join("archive.bin");
        std::fs::write(&archive, [0u8; 1]).expect("archive fixture");

        assert!(CompressionService::validate_resource_limits(
            &archive,
            CompressionService::MAX_EXTRACTED_FILES + 1,
            1,
        ).is_err());
        assert!(CompressionService::validate_resource_limits(
            &archive,
            1,
            CompressionService::MAX_EXTRACTED_BYTES + 1,
        ).is_err());
        assert!(CompressionService::validate_resource_limits(
            &archive,
            1,
            2 * 1024 * 1024 * 1024,
        ).is_err());
    }

    #[test]
    fn extract_only_newer_compares_staged_and_destination_timestamps() {
        let temp = tempfile::tempdir().expect("temp dir");
        let staged = temp.path().join("staged.txt");
        let destination = temp.path().join("destination.txt");
        std::fs::write(&staged, b"staged").expect("staged fixture");
        std::fs::write(&destination, b"destination").expect("destination fixture");
        filetime::set_file_mtime(&staged, filetime::FileTime::from_unix_time(1_000, 0))
            .expect("staged timestamp");
        filetime::set_file_mtime(&destination, filetime::FileTime::from_unix_time(2_000, 0))
            .expect("destination timestamp");
        assert!(CompressionService::staged_file_is_not_newer(&staged, &destination));

        filetime::set_file_mtime(&staged, filetime::FileTime::from_unix_time(3_000, 0))
            .expect("new staged timestamp");
        assert!(!CompressionService::staged_file_is_not_newer(&staged, &destination));
    }

    #[tokio::test]
    async fn transactional_commit_rolls_back_overwrites_after_a_later_failure() {
        let temp = tempfile::tempdir().expect("temp dir");
        let staging = temp.path().join("staging");
        let output = temp.path().join("output");
        std::fs::create_dir_all(staging.join("z")).expect("staging tree");
        std::fs::create_dir_all(&output).expect("output tree");
        std::fs::write(staging.join("a.txt"), b"new").expect("new staged file");
        std::fs::write(staging.join("z/child.txt"), b"child").expect("later staged file");
        std::fs::write(output.join("a.txt"), b"old").expect("old destination");
        std::fs::write(output.join("z"), b"blocks directory creation").expect("blocking file");
        let service = CompressionService::new_with_defaults().await;
        let options = DecompressOptions {
            overwrite_existing: true,
            conflict_policy: "overwrite".to_string(),
            ..Default::default()
        };

        let result = service.commit_staged_extraction(
            None,
            "task",
            "archive.zip",
            &staging,
            &output,
            &options,
        );
        assert!(result.is_err());
        assert_eq!(std::fs::read(output.join("a.txt")).unwrap(), b"old");
        assert!(!output.join("z/child.txt").exists());
    }

    #[tokio::test]
    async fn split_archive_verification_failure_keeps_sources_and_cleans_every_volume() {
        if find_7z_command().is_none() {
            return;
        }
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("source.bin");
        let output = temp.path().join("archive.zip");
        let mut state = 0x1234_5678u32;
        let payload: Vec<u8> = (0..256 * 1024)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as u8
            })
            .collect();
        std::fs::write(&source, payload).expect("source fixture");
        let service = CompressionService::new_with_defaults().await;
        let split_service = crate::services::split_compression::SplitCompressionService::new();
        let result = split_service
            .compress_to_split_zips(
                &[source.to_string_lossy().to_string()],
                &output,
                CompressionOptions {
                    format: Some("zip".to_string()),
                    split_size: Some(4096),
                    ..Default::default()
                },
            )
            .await
            .expect("create split fixture");
        assert!(result.part_files.len() > 1);
        let first_volume = output.with_extension("zip.001");
        let mut bytes = std::fs::read(&first_volume).expect("first volume");
        bytes[0] ^= 0xff;
        std::fs::write(&first_volume, bytes).expect("corrupt first volume");

        assert!(service
            .verify_compression_output(
                CompressionRoute::Zip,
                &first_volume,
                None,
                true,
            )
            .is_err());
        CompressionService::cleanup_unverified_compression_output(&output, &output, true);

        assert!(source.exists(), "verification failure must preserve the source");
        assert!(result.part_files.iter().all(|part| !part.exists()));
    }

    #[tokio::test]
    async fn verification_failure_never_publishes_over_an_existing_target() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("source.txt");
        let working = temp.path().join(".long-compress-test.archive.zip");
        let final_output = temp.path().join("archive.zip");
        std::fs::write(&source, b"source must survive").expect("source fixture");
        std::fs::write(&working, b"not a zip archive").expect("corrupt working output");
        std::fs::write(&final_output, b"existing archive").expect("existing target");
        let service = CompressionService::new_with_defaults().await;

        assert!(service
            .verify_compression_output(CompressionRoute::Zip, &working, None, false)
            .is_err());
        CompressionService::cleanup_unverified_compression_output(
            &working,
            &final_output,
            false,
        );

        assert_eq!(std::fs::read(&final_output).unwrap(), b"existing archive");
        assert_eq!(std::fs::read(&source).unwrap(), b"source must survive");
        assert!(!working.exists());
    }

    #[tokio::test]
    async fn staging_normalization_flattens_duplicates_and_applies_filters() {
        let temp = tempfile::tempdir().expect("temp dir");
        let staging = temp.path().join("staging");
        let archive_path = temp.path().join("fixture.zip");
        std::fs::write(&archive_path, b"fixture").expect("archive fixture");
        std::fs::create_dir_all(staging.join("one")).expect("first folder");
        std::fs::create_dir_all(staging.join("two")).expect("second folder");
        std::fs::write(staging.join("one/same.txt"), b"one").expect("first file");
        std::fs::write(staging.join("two/same.txt"), b"two").expect("second file");
        std::fs::write(staging.join("two/drop.bin"), b"drop").expect("filtered file");
        let service = CompressionService::new_with_defaults().await;
        let options = DecompressOptions {
            preserve_paths: false,
            file_filter: Some("*.txt".to_string()),
            ..Default::default()
        };

        service
            .prepare_staging_layout(&archive_path, &staging, &options)
            .expect("normalize staging");
        assert_eq!(std::fs::read(staging.join("same.txt")).unwrap(), b"one");
        assert_eq!(std::fs::read(staging.join("same (1).txt")).unwrap(), b"two");
        assert!(!staging.join("drop.bin").exists());
        assert!(!staging.join("one").exists());
        assert!(!staging.join("two").exists());
    }

    #[tokio::test]
    async fn rejects_fake_aes_extensions_instead_of_requesting_a_password() {
        let temp = tempfile::tempdir().expect("temp dir");
        let fake = temp.path().join("plain.txt.aes");
        std::fs::write(&fake, b"not an encrypted container").expect("fake fixture");
        let service = CompressionService::new_with_defaults().await;

        assert!(service
            .verify_archive_password_candidate(&fake.to_string_lossy(), "anything")
            .await
            .is_err());
    }

    #[test]
    fn test_refined_error_variants() {
        let err1 = CompressionError::PasswordRequired;
        assert_eq!(err1.to_string(), "需要输入密码才能解压");
    }

    #[tokio::test]
    async fn encrypted_zip_uses_the_requested_password() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("secret.txt");
        let archive = temp.path().join("secret.zip");
        std::fs::write(&source, b"top secret").expect("source fixture");

        let service = CompressionService::for_testing();
        native_compression::zip::create_encrypted_zip(
            &service,
            "open-sesame",
            6,
            &[source.to_string_lossy().to_string()],
            &archive.to_string_lossy(),
            true,
            None,
        ).expect("encrypted ZIP creation");

        let engine = UniversalCliEngine::new();
        assert!(engine.try_password(&archive, "open-sesame").await.expect("correct password"));
        assert!(!engine.try_password(&archive, "wrong-password").await.expect("wrong password"));

        assert!(service
            .archive_requires_password(&archive.to_string_lossy(), ArchiveFormat::Zip)
            .await
            .expect("encrypted state"));
        assert!(service
            .verify_archive_password_candidate(&archive.to_string_lossy(), "open-sesame")
            .await
            .expect("correct candidate"));
        assert!(!service
            .verify_archive_password_candidate(&archive.to_string_lossy(), "wrong-password")
            .await
            .expect("wrong candidate"));

        let renamed_archive = temp.path().join("secret-without-extension.bin");
        std::fs::copy(&archive, &renamed_archive).expect("renamed encrypted ZIP fixture");
        assert!(service
            .verify_archive_password_candidate(&renamed_archive.to_string_lossy(), "open-sesame")
            .await
            .expect("magic-based password validation"));

        let output_dir = temp.path().join("extracted");
        engine
            .extract_with_progress(
                &archive,
                &output_dir,
                Some("open-sesame"),
                true,
                Arc::new(|_| {}),
                Arc::new(|_, _| {}),
                Arc::new(AtomicBool::new(false)),
            )
            .await
            .expect("encrypted ZIP extraction");
        assert_eq!(
            std::fs::read(output_dir.join("secret.txt")).expect("extracted file"),
            b"top secret"
        );
    }

    #[test]
    fn rejects_dangerous_compression_output_paths() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("source.txt");
        std::fs::write(&source, b"must survive").expect("source fixture");

        assert!(CompressionService::validate_compression_io_paths(
            &[source.to_string_lossy().to_string()],
            &source.to_string_lossy(),
            &CompressionOptions::default(),
        ).is_err());

        let folder = temp.path().join("folder");
        std::fs::create_dir_all(&folder).expect("source folder");
        let nested_output = folder.join("archive.zip");
        assert!(CompressionService::validate_compression_io_paths(
            &[folder.to_string_lossy().to_string()],
            &nested_output.to_string_lossy(),
            &CompressionOptions::default(),
        ).is_err());
    }

    #[test]
    fn rejects_encrypted_or_directory_split_creation() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("source.txt");
        std::fs::write(&source, b"split fixture").expect("source fixture");
        let output = temp.path().join("archive.zip");

        let encrypted_split = CompressionOptions {
            password: Some("secret".to_string()),
            split_size: Some(1024),
            ..Default::default()
        };
        assert!(CompressionService::validate_compression_io_paths(
            &[source.to_string_lossy().to_string()],
            &output.to_string_lossy(),
            &encrypted_split,
        ).is_err());

        let folder = temp.path().join("folder");
        std::fs::create_dir_all(&folder).expect("source folder");
        let directory_split = CompressionOptions {
            split_size: Some(1024),
            ..Default::default()
        };
        assert!(CompressionService::validate_compression_io_paths(
            &[folder.to_string_lossy().to_string()],
            &output.to_string_lossy(),
            &directory_split,
        ).is_err());
    }
}
