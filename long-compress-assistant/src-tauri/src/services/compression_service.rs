use crate::models::compression::{CompressionOptions, DecompressOptions, TaskLog, TaskLogSeverity};
use anyhow::{Context,Result};
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use zip::{write::FileOptions, CompressionMethod};
use std::io::{BufRead, BufReader, Read, Write, Seek, SeekFrom};
use std::fs::File;
use sevenz_rust;
use thiserror::Error;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, SystemTime};
use tauri::Window;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use dashmap::DashMap;
use once_cell::sync::Lazy;

use crate::services::io_buffer_pool::IOBufferPool;
use crate::services::rar_support::RarSupportService;
use crate::services::universal_engine::UniversalCliEngine;
use crate::services::archive_engine::ArchiveEngine;
use crate::services::encrypted_password_service::EncryptedPasswordService;
use crate::services::tar_aes_engine::TarAesEngine;
use crate::services::aes_wrapper::AesWrapper;
use crate::utils::archive_tools::{find_7z_command, missing_7z_message};
use crate::services::compression_format::{self, CompressionRoute};
use crate::services::compression_verification;
use crate::services::zip_password::ZipPasswordEncoding;
use crate::services::extraction_transaction::{self, ExtractionStaging};
use crate::services::mark_of_web::{self, PropagationStatus};
use crate::services::native_compression::{self, CompressionRuntime};
use crate::services::native_extraction::{self, ExtractionRuntime};
use crate::services::output_publish_transaction::{self, PublishError};
use crate::services::split_archive_detector::SplitArchiveDetector;

#[derive(Debug, PartialEq, Eq)]
enum PasswordCandidateOutcome {
    Matched,
    Mismatch,
    Unrepresentable(ZipPasswordEncoding),
    AlzUnrepresentable,
    EggUnrepresentable,
}

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
    #[error("[archive-source:Pre-checking:source-changed] 源压缩包在密码检测或预检期间发生变化，请等待下载或复制完成后重试")]
    SourceChangedDuringPrecheck,
    #[error("[archive-source:Extracting:source-changed] 源压缩包在解压期间发生变化，当前结果已丢弃；请等待下载或复制完成后重试")]
    SourceChangedDuringExtraction,
    #[error("[archive-source:{stage}:{code}] {detail}")]
    SourceAccess { stage: &'static str, code: &'static str, detail: String },
    #[error("[archive-output:Verifying:verification-failed] 压缩产物校验未通过，未完成后续发布及源文件清理：{0}")]
    VerificationFailed(#[source] anyhow::Error),
    #[error("[archive-output:Publishing:{code}] 输出提交未完成，请检查目标目录和原始详情：{cause}")]
    PublicationFailed { code: &'static str, #[source] cause: anyhow::Error },
    #[error("[archive-output:Publishing:rollback-incomplete] 解压提交失败且回滚不完整，已停止自动重试。请保留源文件、输出和恢复目录，核对后再处理：{0}")]
    RollbackIncomplete(String),
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
    #[error("{detail}: {cause}")]
    IoFailure { detail: String, #[source] cause: std::io::Error },
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
    pub output_bytes: u64,
    pub output_bytes_estimated: bool,
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
use tokio::sync::OnceCell;

static DEFAULT_PASSWORD_VAULT: OnceCell<Arc<EncryptedPasswordService>> = OnceCell::const_new();
static PENDING_EXTRACTIONS: Lazy<DashMap<String, PendingExtraction>> = Lazy::new(DashMap::new);

struct PendingExtraction {
    source_times: HashMap<PathBuf, u64>,
    staging: ExtractionStaging,
    source_archive: PathBuf,
    final_output: PathBuf,
    options: DecompressOptions,
    expected_expanded_bytes: Option<u64>,
    observed_conflicts: HashMap<PathBuf, ConflictTargetSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConflictTargetSnapshot {
    len: u64,
    modified: Option<SystemTime>,
    created: Option<SystemTime>,
    is_dir: bool,
}

impl ConflictTargetSnapshot {
    fn read(path: &Path) -> Option<Self> {
        std::fs::symlink_metadata(path).ok().map(|metadata| Self {
            len: metadata.len(), modified: metadata.modified().ok(),
            created: metadata.created().ok(), is_dir: metadata.is_dir(),
        })
    }
}

fn conflict_targets_changed(observed: &HashMap<PathBuf, ConflictTargetSnapshot>) -> bool {
    observed.iter().any(|(path, previous)| {
        // A removed target needs no overwrite permission. A changed/replaced
        // existing target must be shown again, including file/directory changes.
        ConflictTargetSnapshot::read(path).is_some_and(|current| current != *previous)
    })
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveFormatDetectedPayload {
    pub task_id: String,
    pub format: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceFileSnapshot {
    len: u64,
    modified: Option<SystemTime>,
    members: Vec<(PathBuf, u64, Option<SystemTime>, Option<SystemTime>)>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileConflictResolution {
    pub dest_path: String,
    pub action: String,
}

struct ProgressMetric {
    started_at: Instant,
    last_bytes: u64,
    paused_at_start: std::time::Duration,
}

struct ProgressBytes {
    processed: u64,
    total: u64,
    output_estimated: bool,
}

#[derive(Clone)]
struct CompressionProgressOutput {
    path: PathBuf,
    split: bool,
}

#[derive(Clone)]
pub struct CompressionService {
    pub config: CompressionServiceConfig,
    pub cancellation_flag: Arc<AtomicBool>,
    pub buffer_pool: Arc<IOBufferPool>,
    pub rar_service: Arc<RarSupportService>,
    pub universal_engine: Arc<UniversalCliEngine>,
    pub encrypted_password_service: Arc<EncryptedPasswordService>,
    pub semaphore: Arc<Semaphore>,
    progress_metrics: Arc<Mutex<HashMap<String, ProgressMetric>>>,
    compression_progress_outputs: Arc<Mutex<HashMap<String, CompressionProgressOutput>>>,
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
    fn emit_split_progress(&self, window: &Window, task_id: &str, processed: u64, total: u64) {
        if processed == 0 { self.begin_progress_telemetry(task_id); }
        let progress = if total == 0 { 0.0 } else { processed as f32 / total as f32 };
        let (speed, eta_seconds) = self.progress_telemetry(task_id, progress, processed, total);
        let _ = window.emit("task-progress", TaskProgress {
            task_id: task_id.to_string(), stage: Some("Splitting".to_string()),
            current_password: None, progress, current_file: None,
            processed_bytes: processed, total_bytes: total,
            output_bytes: processed, output_bytes_estimated: false,
            speed, eta_seconds, password_attempt_current: None, password_attempt_total: None,
        });
    }
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
    pub(crate) fn extraction_history_format(path: &Path, format: &ArchiveFormat) -> String {
        let mut name = path.file_name().and_then(|value| value.to_str())
            .unwrap_or_default().to_ascii_lowercase();
        if let Some((base, number)) = name.rsplit_once('.') {
            if number.len() >= 3 && number.bytes().all(|byte| byte.is_ascii_digit()) { name = base.to_string(); }
        }
        let wrapped = |suffixes: &[&str], canonical: &str| {
            suffixes.iter().any(|suffix| name.ends_with(suffix)).then(|| canonical.to_string())
        };
        match format {
            ArchiveFormat::Zip => if name.ends_with(".zipx") || regex::Regex::new(r"\.zx\d{2,5}$").unwrap().is_match(&name) { "zipx" } else { "zip" }.to_string(),
            ArchiveFormat::SevenZip => "7z".to_string(),
            ArchiveFormat::Rar => "rar".to_string(),
            ArchiveFormat::Zpaq => "zpaq".to_string(),
            ArchiveFormat::AesEncrypted => {
                let requested = compression_format::infer_compression_format(&name, None);
                if requested.ends_with(".aes") && compression_format::has_native_password_container(&requested) { requested }
                else { "aes".to_string() }
            },
            ArchiveFormat::Tar => "tar".to_string(),
            ArchiveFormat::Gzip => wrapped(&[".tar.gz", ".tgz", ".tpz"], "tar.gz")
                .unwrap_or_else(|| "gz".to_string()),
            ArchiveFormat::Bzip2 => wrapped(&[".tar.bz2", ".tbz", ".tbz2"], "tar.bz2")
                .unwrap_or_else(|| "bz2".to_string()),
            ArchiveFormat::Xz => wrapped(&[".tar.xz", ".txz"], "tar.xz")
                .unwrap_or_else(|| "xz".to_string()),
            ArchiveFormat::Zstd => wrapped(&[".tar.zst", ".tzst"], "tar.zst")
                .unwrap_or_else(|| "zst".to_string()),
            ArchiveFormat::Lzma => wrapped(&[".tar.lzma"], "tar.lzma").unwrap_or_else(|| "lzma".to_string()),
            ArchiveFormat::Lz4 => wrapped(&[".tar.lz4"], "tar.lz4").unwrap_or_else(|| "lz4".to_string()),
            ArchiveFormat::Brotli => wrapped(&[".tar.br"], "tar.br").unwrap_or_else(|| "br".to_string()),
            ArchiveFormat::Lzip => wrapped(&[".tar.lz", ".tlz"], "tar.lz").unwrap_or_else(|| "lz".to_string()),
            ArchiveFormat::Iso => "iso".to_string(),
            ArchiveFormat::Cab => "cab".to_string(),
            ArchiveFormat::Lzh => "lzh".to_string(),
            ArchiveFormat::Arj => "arj".to_string(),
            ArchiveFormat::Dmg => "dmg".to_string(),
            ArchiveFormat::Wim => "wim".to_string(),
            ArchiveFormat::Vhd => "vhd".to_string(),
            ArchiveFormat::Chm => "chm".to_string(),
            ArchiveFormat::Deb => "deb".to_string(),
            ArchiveFormat::Rpm => "rpm".to_string(),
            ArchiveFormat::SquashFs => "squashfs".to_string(),
            ArchiveFormat::Nsis => "nsis".to_string(),
            ArchiveFormat::Msi => "msi".to_string(),
            ArchiveFormat::Xar => "xar".to_string(),
            ArchiveFormat::Cpio => "cpio".to_string(),
            ArchiveFormat::Udf => "udf".to_string(),
            ArchiveFormat::Fat => "fat".to_string(),
            ArchiveFormat::Ntfs => "ntfs".to_string(),
            ArchiveFormat::Hfs => "hfs".to_string(),
            ArchiveFormat::Alz => "alz".to_string(),
            ArchiveFormat::Egg => "egg".to_string(),
            ArchiveFormat::Arc => "arc".to_string(),
            ArchiveFormat::Apfs => "apfs".to_string(),
            ArchiveFormat::Ext => "ext".to_string(),
            ArchiveFormat::Universal | ArchiveFormat::Unknown => {
                let mut extension = path.extension().and_then(|value| value.to_str())
                    .unwrap_or("unknown").to_ascii_lowercase();
                if extension.chars().all(|value| value.is_ascii_digit()) {
                    extension = path.file_stem().map(Path::new)
                        .and_then(Path::extension).and_then(|value| value.to_str())
                        .unwrap_or("split").to_ascii_lowercase();
                } else if extension.starts_with('z')
                    && extension[1..].chars().all(|value| value.is_ascii_digit())
                {
                    extension = "zip".to_string();
                } else if extension.starts_with('r')
                    && extension[1..].chars().all(|value| value.is_ascii_digit())
                {
                    extension = "rar".to_string();
                }
                extension
            }
        }
    }
    fn is_conflict_resolution_required(error: &anyhow::Error) -> bool {
        matches!(
            error.downcast_ref::<CompressionError>(),
            Some(CompressionError::ExtractionFailed(message))
                if message == "File conflict requires resolution"
        )
    }

    async fn finish_committed_extraction(
        &self,
        window: &Window,
        task_id: &str,
        source_archive: &Path,
        final_output: &Path,
        options: &DecompressOptions,
        expected_expanded_bytes: Option<u64>,
        published_bytes: u64,
    ) -> Result<String> {
        if options.delete_after {
            let source_path = source_archive.to_path_buf();
            let recycle_result = tokio::task::spawn_blocking(move || {
                Self::move_source_archives_to_system_recycle_bin(&source_path)
            }).await;
            match recycle_result
                .map_err(|error| anyhow::anyhow!("System recycle-bin task failed: {}", error))
                .and_then(|result| result)
            {
                Err(error) => self.emit_log(
                    window, task_id,
                    &format!("解压已完成，但源压缩包未能移入系统回收站：{}", error),
                    TaskLogSeverity::Warning,
                ),
                Ok(source_count) => self.emit_log(
                    window, task_id,
                    &format!("已将 {} 个源压缩文件移入系统回收站，可从 Windows 回收站恢复", source_count),
                    TaskLogSeverity::Success,
                ),
            }
        }
        // Only this transaction's published files, not existing target contents
        // (including the source volumes in the user's "same directory" flow).
        let final_expanded_bytes = published_bytes;
        let telemetry_total = expected_expanded_bytes
            .unwrap_or(final_expanded_bytes)
            .max(final_expanded_bytes);
        self.emit_progress_with_output_estimate(
            window, task_id, 1.0, None,
            ProgressBytes {
                processed: final_expanded_bytes,
                total: telemetry_total,
                output_estimated: false,
            },
        );
        self.emit_log(
            window, task_id,
            &format!("解压数据统计完成：实际产出 {} 字节", final_expanded_bytes),
            TaskLogSeverity::Success,
        );
        self.emit_log(window, task_id, "全部解压任务已完成", TaskLogSeverity::Success);
        Ok(final_output.to_string_lossy().to_string())
    }

    pub fn discard_pending_extraction(task_id: &str) -> bool {
        PENDING_EXTRACTIONS.remove(task_id).is_some()
    }

    fn retain_pending_after_commit_error(task_id: &str, mut pending: PendingExtraction, error: &anyhow::Error) -> Option<String> {
        if pending.staging.retain_after_incomplete_rollback(error) { return None; }
        // The UI treats confirmed filesystem failures as terminal and offers a
        // new task. Keep staging only for an actual conflict reconfirmation;
        // retaining an unreachable pending commit leaks output space.
        if crate::services::archive_io::kind(error).is_some() {
            let path = pending.staging.path().to_path_buf();
            return pending.staging.cleanup().err().map(|cleanup| format!(
                "未发布暂存的首次清理未完成，请核对路径 {}；原文件保持不变。清理详情：{}",
                path.display(), cleanup));
        }
        PENDING_EXTRACTIONS.insert(task_id.to_string(), pending);
        None
    }

    pub async fn resolve_pending_extraction(
        &self,
        window: &Window,
        task_id: &str,
        resolutions: Vec<FileConflictResolution>,
        fallback_action: Option<String>,
    ) -> Result<String> {
        let (_, mut pending) = PENDING_EXTRACTIONS
            .remove(task_id)
            .ok_or_else(|| anyhow::anyhow!("No pending extraction conflict for task: {task_id}"))?;

        let valid_action = |action: &str| matches!(action, "overwrite" | "skip" | "rename");
        if fallback_action.as_deref().is_some_and(|action| !valid_action(action))
            || resolutions.iter().any(|item| !valid_action(&item.action))
        {
            PENDING_EXTRACTIONS.insert(task_id.to_string(), pending);
            anyhow::bail!("Unsupported extraction conflict action");
        }

        let targets_changed = conflict_targets_changed(&pending.observed_conflicts);
        let resolution_map = resolutions.into_iter()
            .map(|item| (item.dest_path, item.action))
            .collect::<HashMap<_, _>>();
        let empty_resolutions = HashMap::new();
        let mut refreshed_conflicts = HashMap::new();
        let commit_result = extraction_transaction::commit_staged_extraction_with_resolutions(
            pending.source_archive.to_string_lossy().as_ref(),
            pending.staging.path(),
            &pending.final_output,
            &pending.options,
            if targets_changed { &empty_resolutions } else { &resolution_map },
            if targets_changed { None } else { fallback_action.as_deref() },
            |conflict| {
                let path = PathBuf::from(&conflict.dest_path);
                if let Some(snapshot) = ConflictTargetSnapshot::read(&path) {
                    refreshed_conflicts.insert(path.clone(), snapshot);
                }
                let _ = window.emit("file-conflict", FileConflictPayload {
                    task_id: task_id.to_string(),
                    file_name: conflict.file_name,
                    source_path: conflict.source_path,
                    dest_path: conflict.dest_path,
                    source_size: conflict.source_size,
                    dest_size: conflict.dest_size,
                    source_modified: pending.source_times.get(&path).copied().unwrap_or(conflict.source_modified),
                    dest_modified: conflict.dest_modified,
                });
            },
        );
        if let Err(error) = commit_result {
            let needs_confirmation = Self::is_conflict_resolution_required(&error);
            if needs_confirmation {
                pending.observed_conflicts = refreshed_conflicts;
            }
            if let Some(warning) = Self::retain_pending_after_commit_error(task_id, pending, &error) {
                self.emit_log(window, task_id, &warning, TaskLogSeverity::Warning);
                return Err(error.context(warning));
            }
            if targets_changed && needs_confirmation {
                anyhow::bail!("File conflict requires resolution：目标文件在确认期间发生变化，请重新比较并选择；尚未覆盖任何文件。");
            }
            return Err(error);
        }
        let published_bytes = commit_result?;
        if let Some(warning) = pending.staging.cleanup_after_commit() {
            self.emit_log(window, task_id, &warning, TaskLogSeverity::Warning);
        }
        self.emit_log(
            window, task_id,
            "冲突策略已应用，直接提交既有解压暂存结果",
            TaskLogSeverity::Success,
        );
        self.finish_committed_extraction(
            window, task_id, &pending.source_archive, &pending.final_output,
            &pending.options, pending.expected_expanded_bytes,
            published_bytes,
        ).await
    }

    #[cfg(test)]
    const MAX_EXTRACTED_FILES: usize = extraction_transaction::MAX_EXTRACTED_ENTRIES;
    #[cfg(test)]
    const MAX_EXTRACTED_BYTES: u64 = extraction_transaction::MAX_EXTRACTED_BYTES;
    const COPY_BUFFER_SIZE: usize = 256 * 1024;

    pub async fn new_with_defaults() -> Self {
        let encrypted_password_service = DEFAULT_PASSWORD_VAULT
            .get_or_init(|| async {
                let data_dir = crate::utils::app_paths::app_data_dir();
                let mut service = EncryptedPasswordService::new(&data_dir);
                match EncryptedPasswordService::get_or_create_master_key(&data_dir).await {
                    Ok(master_key) => match service.unlock(&master_key).await {
                        Ok(true) => {}
                        Ok(false) if !data_dir.join("master_password.hash").exists() => {
                            if let Err(error) = service.initialize(&master_key).await {
                                log::error!("Failed to initialize password vault for archive tasks: {}", error);
                            }
                        }
                        Ok(false) => {
                            log::error!("Password vault installation key did not unlock the existing vault");
                        }
                        Err(error) => {
                            log::error!("Failed to unlock password vault for archive tasks: {}", error);
                        }
                    },
                    Err(error) => {
                        log::error!("Failed to load password vault installation key: {}", error);
                    }
                }
                Arc::new(service)
            })
            .await
            .clone();

        Self::new(
            CompressionServiceConfig::default(),
            Arc::new(IOBufferPool::default()),
            Arc::new(RarSupportService::new()),
            Arc::new(UniversalCliEngine::new()),
            encrypted_password_service,
        )
    }
    pub fn new(
        config: CompressionServiceConfig,
        buffer_pool: Arc<IOBufferPool>,
        rar_service: Arc<RarSupportService>,
        universal_engine: Arc<UniversalCliEngine>,
        encrypted_password_service: Arc<EncryptedPasswordService>,
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
            encrypted_password_service,
            semaphore: Arc::new(Semaphore::new(max_concurrency)),
            progress_metrics: Arc::new(Mutex::new(HashMap::new())),
            compression_progress_outputs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn for_testing() -> Self {
        // 测试默认使用未解锁的文件型密码保险箱；需要密码候选的测试应显式注入临时保险箱。
        log::warn!("CompressionService::for_testing() called - inject an unlocked temporary vault when testing password lookup.");
        let data_dir = crate::utils::app_paths::app_data_dir();
        let encrypted_password_service = Arc::new(EncryptedPasswordService::new(&data_dir));

        Self {
            config: CompressionServiceConfig::default(),
            cancellation_flag: Arc::new(AtomicBool::new(false)),
            buffer_pool: Arc::new(IOBufferPool::default()),
            rar_service: Arc::new(RarSupportService::new()),
            universal_engine: Arc::new(UniversalCliEngine::new()),
            encrypted_password_service,
            semaphore: Arc::new(Semaphore::new(2)),
            progress_metrics: Arc::new(Mutex::new(HashMap::new())),
            compression_progress_outputs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn cancel(&self) {
        self.cancellation_flag.store(true, Ordering::SeqCst);
    }

    pub fn reset_cancellation(&self) {
        self.cancellation_flag.store(false, Ordering::SeqCst);
    }

    fn check_cancellation(&self) -> Result<()> {
        crate::services::task_control::wait_if_paused(&self.cancellation_flag);
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
            paused_at_start: super::task_control::paused_duration(task_id),
        });
        if processed_bytes < metric.last_bytes {
            metric.started_at = Instant::now();
            metric.paused_at_start = super::task_control::paused_duration(task_id);
        }
        metric.last_bytes = processed_bytes;

        let paused = super::task_control::paused_duration(task_id).saturating_sub(metric.paused_at_start);
        let elapsed = metric.started_at.elapsed().saturating_sub(paused).as_secs_f64();
        // The first/final callback can arrive within a few milliseconds for
        // cached or tiny work. Reporting a quotient for that interval creates
        // physically impossible multi-billion MB/s values. Wait for a real
        // observation window and never publish a terminal one-sample rate.
        let has_stable_window = elapsed >= 0.25 && progress < 1.0 && processed_bytes < total_bytes;
        let bytes_per_second = if has_stable_window {
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
                paused_at_start: super::task_control::paused_duration(task_id),
            });
        }
    }

    fn register_compression_progress_output(&self, task_id: &str, path: PathBuf, split: bool) {
        if let Ok(mut outputs) = self.compression_progress_outputs.lock() {
            outputs.insert(task_id.to_string(), CompressionProgressOutput { path, split });
        }
    }

    fn clear_compression_progress_output(&self, task_id: &str) {
        if let Ok(mut outputs) = self.compression_progress_outputs.lock() {
            outputs.remove(task_id);
        }
    }

    fn compression_output_bytes(&self, task_id: &str) -> Option<u64> {
        let output = self
            .compression_progress_outputs
            .lock()
            .ok()?
            .get(task_id)
            .cloned()?;
        if !output.split {
            return std::fs::metadata(output.path).ok().map(|metadata| metadata.len());
        }

        let mut total = 0u64;
        for (_, path) in crate::services::archive_volumes::volume_paths(&output.path).ok()? {
            if let Ok(metadata) = path.metadata() { total = total.saturating_add(metadata.len()); }
        }
        for path in crate::services::archive_volumes::recovery_volume_paths(&output.path).ok()? {
            if let Ok(metadata) = path.metadata() { total = total.saturating_add(metadata.len()); }
        }
        Some(total)
    }

    fn has_compression_progress_output(&self, task_id: &str) -> bool {
        self.compression_progress_outputs
            .lock()
            .map(|outputs| outputs.contains_key(task_id))
            .unwrap_or(false)
    }

    #[cfg(test)]
    fn directory_payload_size(path: &Path) -> u64 {
        let Ok(entries) = std::fs::read_dir(path) else {
            return 0;
        };
        entries.flatten().fold(0u64, |total, entry| {
            let entry_path = entry.path();
            let size = entry
                .metadata()
                .ok()
                .map(|metadata| {
                    if metadata.is_dir() {
                        Self::directory_payload_size(&entry_path)
                    } else if metadata.is_file() {
                        metadata.len()
                    } else {
                        0
                    }
                })
                .unwrap_or(0);
            total.saturating_add(size)
        })
    }

    pub fn emit_progress(&self, window: &Window, task_id: &str, progress: f32, current_file: Option<String>, processed_bytes: u64, total_bytes: u64) {
        self.emit_progress_with_output_estimate(
            window,
            task_id,
            progress,
            current_file,
            ProgressBytes {
                processed: processed_bytes,
                total: total_bytes,
                output_estimated: false,
            },
        );
    }

    fn emit_progress_with_output_estimate(
        &self,
        window: &Window,
        task_id: &str,
        progress: f32,
        current_file: Option<String>,
        bytes: ProgressBytes,
    ) {
        let ProgressBytes {
            processed: processed_bytes,
            total: total_bytes,
            output_estimated: output_bytes_estimated,
        } = bytes;
        let (speed, eta_seconds) = self.progress_telemetry(task_id, progress, processed_bytes, total_bytes);
        let output_bytes = if self.has_compression_progress_output(task_id) {
            self.compression_output_bytes(task_id).unwrap_or(0)
        } else {
            processed_bytes
        };
        let payload = TaskProgress {
            task_id: task_id.to_string(),
            stage: None,
            current_password: None,
            progress,
            current_file,
            processed_bytes,
            total_bytes,
            output_bytes,
            output_bytes_estimated,
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
            output_bytes: self.compression_output_bytes(task_id).unwrap_or(0),
            output_bytes_estimated: false,
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

        let mut suspended = false;
        let status = loop {
            let process_id = child.id();
            if let Err(error) = crate::services::task_control::sync_child_pause(
                &self.cancellation_flag,
                process_id,
                &mut suspended,
            ) {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(CompressionError::CompressionFailed(error).into());
            }
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
        for parent in output.ancestors().skip(1) {
            if parent.exists() && !parent.is_dir() {
                return Err(CompressionError::CompressionFailed(format!(
                    "目标保存路径中的“{}”是文件，不是文件夹。请选择其他输出目录。",
                    parent.display()
                )).into());
            }
        }
        if output.exists() {
            return Err(CompressionError::CompressionFailed(format!(
                "Output already exists; choose a new path to avoid overwriting data: {}",
                output.display()
            )).into());
        }
        let resolved_output = Self::canonicalize_with_missing_tail(output)?;
        let split_requested = options.split_size.is_some_and(|size| size > 0);
        let has_password = options.password.as_deref().is_some_and(|password| !password.is_empty());
        let format = Self::infer_compression_format(output_path, options.format.as_deref());
        if split_requested && has_password && !matches!(format.as_str(), "zip" | "zipx" | "7z" | "rar" | "zpaq") && !(format.ends_with(".aes") && compression_format::has_native_password_container(&format)) {
            return Err(CompressionError::CompressionFailed(
                "当前格式不支持密码分卷，请选择 ZIP、ZIPX、7Z、RAR、ZPAQ 或 Long AES 格式，或关闭分卷".to_string()
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

    pub(crate) fn temporary_compression_output(output: &Path) -> Result<PathBuf> {
        output_publish_transaction::staged_output_path(output, "long-compress")
            .map_err(|error| CompressionError::CompressionFailed(error.to_string()).into())
    }

    fn cleanup_failed_compression_outputs(path: &Path, split_requested: bool) {
        // Standard split creation owns and cleans only the volumes it created.
        // Never glob-clean a user-visible prefix here: pre-existing .001 files
        // may belong to another archive and must survive a validation failure.
        if split_requested {
            return;
        }
        output_publish_transaction::cleanup_staged_output_family(path);
    }

    fn normalize_storage_full_error(error: anyhow::Error) -> anyhow::Error {
        // Typed terminal errors already carry a confirmed stage; preserve their I/O detail.
        if matches!(error.downcast_ref::<CompressionError>(), Some(CompressionError::VerificationFailed(_) | CompressionError::PublicationFailed { .. })) {
            return error;
        }
        let storage_full = error.chain().any(|cause| {
            cause
                .downcast_ref::<std::io::Error>()
                .is_some_and(|io_error| io_error.kind() == std::io::ErrorKind::StorageFull)
        });
        if storage_full {
            error.context("[archive-io:disk-full] 磁盘空间不足，当前处理未完成；请释放空间或更换输出目录后创建新任务")
        } else {
            error
        }
    }

    pub(crate) fn finalize_compression_output(
        route_result: Result<()>,
        working_output: &Path,
        final_output: &Path,
        split_requested: bool,
        cancelled: bool,
    ) -> Result<()> {
        let result = route_result
            .map_err(Self::normalize_storage_full_error)
            .and_then(|_| {
                if split_requested {
                    output_publish_transaction::publish_verified_volumes(
                        working_output, final_output, || cancelled,
                    ).map_err(Self::publication_error)?;
                } else {
                    output_publish_transaction::publish_verified_file(
                        working_output,
                        final_output,
                        || cancelled,
                    )
                    .map_err(Self::publication_error)?;
                }
                Ok(())
            })
            .map_err(Self::normalize_storage_full_error);

        if result.is_err() {
            output_publish_transaction::cleanup_staged_output_family(working_output);
            Self::cleanup_failed_compression_outputs(working_output, split_requested);
        }
        result
    }

    fn publication_error(error: PublishError) -> anyhow::Error {
        match error {
            PublishError::Cancelled => CompressionError::Cancelled.into(),
            PublishError::Io(error) => CompressionError::PublicationFailed {
                code: "publication-failed", cause: error.into(),
            }.into(),
            PublishError::TargetAppeared(path) => CompressionError::PublicationFailed {
                code: "output-conflict",
                cause: anyhow::anyhow!("目标文件在处理期间出现，未覆盖已有文件：{}", path.display()),
            }.into(),
            other => CompressionError::PublicationFailed {
                code: "publication-failed", cause: other.into(),
            }.into(),
        }
    }

    fn cleanup_unverified_compression_output(
        working_output: &Path,
        _final_output: &Path,
        split_requested: bool,
        creation_succeeded: bool,
    ) {
        // Creation owns its failure cleanup. Validation may have rejected an
        // existing destination, so its files must never be treated as our output.
        if !creation_succeeded { return; }
        if split_requested {
            output_publish_transaction::cleanup_staged_output_family(working_output);
        } else {
            Self::cleanup_failed_compression_outputs(working_output, false);
        }
    }

    fn verify_compression_output(
        &self,
        route: CompressionRoute,
        output: &Path,
        password: Option<&str>,
        zip_password_encoding: Option<ZipPasswordEncoding>,
        split_requested: bool,
    ) -> Result<()> {
        self.verify_compression_output_inner(route, output, password, zip_password_encoding, split_requested)
            .map_err(|error| {
                if matches!(error.downcast_ref::<CompressionError>(), Some(CompressionError::Cancelled)) {
                    error
                } else {
                    CompressionError::VerificationFailed(error).into()
                }
            })
    }

    fn verify_compression_output_inner(
        &self,
        route: CompressionRoute,
        output: &Path,
        password: Option<&str>,
        zip_password_encoding: Option<ZipPasswordEncoding>,
        split_requested: bool,
    ) -> Result<()> {
        if route == CompressionRoute::Zpaq {
            self.check_cancellation()?;
            let result = tauri::async_runtime::block_on(crate::services::zpaq_engine::verify(output, password, self.cancellation_flag.clone()));
            self.check_cancellation()?;
            return result;
        }
        if split_requested && route == CompressionRoute::Zip {
            return compression_verification::verify_numeric_zip_with_encoding(
                output, password, zip_password_encoding, || self.check_cancellation().is_err());
        }
        if (!split_requested || route != CompressionRoute::Rar)
            && compression_verification::verify_native_with_zip_encoding(route, output, password, zip_password_encoding, || {
                self.check_cancellation().is_err()
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
            command.arg("t").arg("-y");
            if let Some(password) = password.filter(|value| !value.is_empty()) {
                command.arg(format!("-p{password}"));
            } else {
                command.arg("-p-");
            }
            command.arg(output);
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
            let verification_encoding = options.zip_password_encoding;
            let split_requested = options.split_size.is_some_and(|size| size > 0);
            let final_output = PathBuf::from(&output_path);
            let working_output = Self::temporary_compression_output(&final_output)?;
            let working_output_string = working_output.to_string_lossy().to_string();
            service.emit_log(&window, &task_id, &format!("开始压缩到: {}", output_path), TaskLogSeverity::Info);
            let route = compression_format::compression_route(&requested_format).ok_or_else(|| {
                CompressionError::CompressionFailed(format!(
                    "Unsupported compression format '{}'.",
                    requested_format
                ))
            })?;
            service.register_compression_progress_output(
                &task_id,
                working_output.clone(),
                split_requested,
            );
            let generic_split = requested_format != "swm" && split_requested && !matches!(route, CompressionRoute::Zip | CompressionRoute::SevenZip | CompressionRoute::Rar);
            let split_size = options.split_size.unwrap_or(0);
            let mut options = options;
            if generic_split { options.split_size = None; }
            let res = match route {
                CompressionRoute::LzmaAes => native_compression::aes::compress(&service, Some(&window), &task_id, &source_files, &working_output_string, options, native_compression::aes::AesCompressionKind::Lzma),
                CompressionRoute::TarLzmaAes => native_compression::aes::compress(&service, Some(&window), &task_id, &source_files, &working_output_string, options, native_compression::aes::AesCompressionKind::TarLzma),
                CompressionRoute::Zpaq => {
                    service.check_cancellation()?;
                    let result = tauri::async_runtime::block_on(crate::services::zpaq_engine::create(&source_files, &working_output, &options, service.cancellation_flag.clone()));
                    service.check_cancellation().and(result)
                },
                CompressionRoute::Lz4Aes => native_compression::aes::compress(&service, Some(&window), &task_id, &source_files, &working_output_string, options, native_compression::aes::AesCompressionKind::Lz4),
                CompressionRoute::BrotliAes => native_compression::aes::compress(&service, Some(&window), &task_id, &source_files, &working_output_string, options, native_compression::aes::AesCompressionKind::Brotli),
                CompressionRoute::LzipAes => native_compression::aes::compress(&service, Some(&window), &task_id, &source_files, &working_output_string, options, native_compression::aes::AesCompressionKind::Lzip),
                CompressionRoute::TarLz4Aes => native_compression::aes::compress(&service, Some(&window), &task_id, &source_files, &working_output_string, options, native_compression::aes::AesCompressionKind::TarLz4),
                CompressionRoute::TarBrotliAes => native_compression::aes::compress(&service, Some(&window), &task_id, &source_files, &working_output_string, options, native_compression::aes::AesCompressionKind::TarBrotli),
                CompressionRoute::TarLzipAes => native_compression::aes::compress(&service, Some(&window), &task_id, &source_files, &working_output_string, options, native_compression::aes::AesCompressionKind::TarLzip),
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
                CompressionRoute::Lz4 | CompressionRoute::Brotli | CompressionRoute::TarLz4 | CompressionRoute::TarBrotli | CompressionRoute::Lzip | CompressionRoute::TarLzip | CompressionRoute::TarLzma => {
                    crate::services::codec_stream::compress(&service, Some(&window), &task_id, &source_files, &working_output_string, options, &requested_format)
                },
            };
            let res = res.and_then(|_| {
                if generic_split {
                    crate::services::split_compression::SplitCompressionService::split_archive_file_with_progress(
                        &working_output, &working_output, split_size, || service.check_cancellation(), |processed, total| {
                            service.emit_split_progress(&window, &task_id, processed, total);
                        })?;
                    std::fs::remove_file(&working_output)?;
                }
                Ok(())
            });
            let verification_output = if split_requested {
                crate::services::archive_volumes::first_volume_path(&working_output)
            } else {
                working_output.clone()
            };
            let creation_succeeded = res.is_ok();
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
                        Some(verification_encoding),
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
                    creation_succeeded,
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
            let res = res.and_then(|_| service.check_cancellation());
            let res = if split_requested && res.is_ok() {
                output_publish_transaction::publish_verified_volumes(
                    &working_output, &final_output,
                    || service.cancellation_flag.load(Ordering::Relaxed),
                ).map_err(Self::publication_error)
            } else { Self::finalize_compression_output(
                res,
                &working_output,
                &final_output,
                split_requested,
                service.cancellation_flag.load(Ordering::Relaxed),
            ) };
            if res.is_ok() {
                if delete_after {
                    let verified_output = if split_requested {
                        crate::services::archive_volumes::first_volume_path(&final_output).to_string_lossy().into_owned()
                    } else {
                        output_path.clone()
                    };
                    service.delete_sources_after_success(&window, &task_id, &source_files, &verified_output);
                }
                service.emit_log(&window, &task_id, "压缩完成", TaskLogSeverity::Success);
                service.register_compression_progress_output(
                    &task_id,
                    final_output.clone(),
                    split_requested,
                );
                service.emit_progress(&window, &task_id, 1.0, None, 0, 0);
            } else if let Err(error) = &res {
                if matches!(error.downcast_ref::<CompressionError>(), Some(CompressionError::Cancelled)) {
                    service.emit_log(&window, &task_id, "压缩已取消", TaskLogSeverity::Info);
                } else {
                    service.emit_log(&window, &task_id, &format!("压缩失败: {error}"), TaskLogSeverity::Error);
                }
            }
            service.clear_compression_progress_output(&task_id);
            res
        }).await?
    }

    /// 智能尝试密码本中的密码
    async fn resolve_archive_password(&self, window: &Window, task_id: &str, file_path: &str, options: &DecompressOptions) -> Result<Option<String>> {
        self.check_cancellation()?;
        let mut attempts = 0usize;
        self.emit_log(
            window,
            task_id,
            if options.enable_bruteforce {
                "自动解锁顺序：密码保险箱 → 导入词表 → 内置字典"
            } else {
                "自动解锁顺序：仅尝试密码保险箱；密码字典未启用"
            },
            TaskLogSeverity::Info,
        );
        if let Some(password) = self.attempt_passwords_smartly(window, task_id, file_path, options, &mut attempts).await? {
            return Ok(Some(password));
        }

        if options.enable_bruteforce {
            if !options.bruteforce_wordlists.is_empty() {
                if let Some(password) = self.attempt_bruteforce_wordlists(window, task_id, file_path, &options.bruteforce_wordlists, options, &mut attempts).await? {
                    return Ok(Some(password));
                }
            }
            if let Some(password) = self.attempt_recommended_dictionary(window, task_id, file_path, options, &mut attempts).await? {
                return Ok(Some(password));
            }
        }

        if attempts > 0 && Self::detect_password_format(file_path)? == ArchiveFormat::SevenZip {
            return Err(anyhow::anyhow!("已尝试 {attempts} 个候选但未能完整验证归档。{}", native_extraction::seven_zip::ambiguous_password_error()));
        }
        Ok(None)
    }

    pub async fn resolve_archive_password_silent(&self, file_path: &str, options: &DecompressOptions) -> Result<Option<String>> {
        self.check_cancellation()?;
        if Self::detect_password_format(file_path)? == ArchiveFormat::Egg
            && !crate::services::egg_engine::requires_password(Path::new(file_path), options, self.cancellation_flag.clone()).await? { return Ok(None); }
        if Self::detect_password_format(file_path)? == ArchiveFormat::Alz
            && !crate::services::alz_engine::requires_password(Path::new(file_path), options, self.cancellation_flag.clone()).await? { return Ok(None); }
        if Self::detect_password_format(file_path)? == ArchiveFormat::Zip
            && !UniversalCliEngine::zip_requires_password_for_selection(Path::new(file_path), options)?
        {
            return Ok(None);
        }
        if let Some(password) = self.attempt_password_book_candidates(file_path, options).await? {
            return Ok(Some(password));
        }

        if options.enable_bruteforce {
            if !options.bruteforce_wordlists.is_empty() {
                if let Some(password) = self.attempt_bruteforce_wordlists_silent(file_path, &options.bruteforce_wordlists, options).await? {
                    return Ok(Some(password));
                }
            }
            if let Some(password) = self.attempt_recommended_dictionary_silent(file_path, options).await? {
                return Ok(Some(password));
            }
        }

        Ok(None)
    }

    fn recommended_dictionary(file_path: &str) -> Vec<String> {
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(file_path);
        crate::services::password_dictionary_service::PasswordDictionaryService::new()
            .get_recommended_strategy(Some(file_name))
    }

    fn elapsed_label(started_at: Instant) -> String {
        let elapsed = started_at.elapsed();
        if elapsed.as_millis() < 1_000 {
            format!("{} ms", elapsed.as_millis().max(1))
        } else {
            format!("{:.2} s", elapsed.as_secs_f64())
        }
    }

    fn emit_password_attempt_progress(
        &self,
        window: &Window,
        task_id: &str,
        current: usize,
        total: Option<usize>,
        candidate: String,
    ) {
        let _ = window.emit(
            "task-progress",
            Self::password_attempt_progress_payload(task_id, current, total, candidate),
        );
    }

    fn extraction_progress_from_output(processed_bytes: u64, total_bytes: u64) -> Option<f32> {
        if processed_bytes == 0 || total_bytes == 0 {
            return None;
        }
        Some(((processed_bytes as f64 / total_bytes as f64) as f32).clamp(0.0, 0.99))
    }

    fn password_attempt_progress_payload(
        task_id: &str,
        current: usize,
        total: Option<usize>,
        candidate: String,
    ) -> TaskProgress {
        TaskProgress {
            task_id: task_id.to_string(),
            stage: Some("password-attempt".to_string()),
            current_password: Some(candidate),
            // Password candidate position is diagnostic metadata, not extraction
            // progress. Extraction has not started yet, so the task must remain
            // at exactly zero regardless of how many candidates were attempted.
            progress: 0.0,
            speed: None,
            eta_seconds: None,
            current_file: None,
            processed_bytes: 0,
            total_bytes: 0,
            output_bytes: 0,
            output_bytes_estimated: false,
            password_attempt_current: Some(current),
            password_attempt_total: total,
        }
    }

    async fn attempt_recommended_dictionary_silent(&self, file_path: &str, options: &DecompressOptions) -> Result<Option<String>> {
        for password in Self::recommended_dictionary(file_path) {
            self.check_cancellation()?;
            if self.test_password_candidate(file_path, &password, options).await? == PasswordCandidateOutcome::Matched {
                return Ok(Some(password));
            }
        }
        Ok(None)
    }

    async fn attempt_recommended_dictionary(&self, window: &Window, task_id: &str, file_path: &str, options: &DecompressOptions, attempts: &mut usize) -> Result<Option<String>> {
        let passwords = Self::recommended_dictionary(file_path);
        let total = passwords.len();
        let started_at = Instant::now();
        self.emit_log(
            window,
            task_id,
            &format!(
                "内置密码字典准备完成：共 {} 个候选；实时显示正在验证的密码",
                total
            ),
            TaskLogSeverity::Info,
        );

        for (index, password) in passwords.into_iter().enumerate() {
            self.check_cancellation()?;
            let current = index + 1;
            self.emit_password_attempt_progress(
                window,
                task_id,
                current,
                Some(total),
                format!("内置字典 #{} · 密码「{}」", current, password),
            );
            let attempt_started_at = Instant::now();
            *attempts += 1;
            match self.test_password_candidate(file_path, &password, options).await? {
                PasswordCandidateOutcome::Matched => {
                    self.emit_log(
                        window,
                        task_id,
                        &format!(
                            "内置字典候选 [{}/{}] · 密码「{}」→ 匹配成功（{}）",
                            current,
                            total,
                            password,
                            Self::elapsed_label(attempt_started_at)
                        ),
                        TaskLogSeverity::Success,
                    );
                    return Ok(Some(password));
                }
                PasswordCandidateOutcome::Mismatch => {
                    // The live progress payload already exposes every candidate.
                    // Keep the permanent task log readable by sampling routine
                    // misses instead of appending hundreds of near-identical rows.
                    if Self::should_log_password_attempt(current, total) {
                        self.emit_log(
                            window,
                            task_id,
                            &format!(
                                "内置字典进度 [{}/{}] · 密码「{}」未匹配（{}）",
                                current,
                                total,
                                password,
                                Self::elapsed_label(attempt_started_at)
                            ),
                            TaskLogSeverity::Info,
                        );
                    }
                },
                PasswordCandidateOutcome::AlzUnrepresentable => self.emit_log(window, task_id, "候选密码不能按所选 ALZ 编码无损表示，已跳过；未据此判定密码错误或文件损坏", TaskLogSeverity::Info),
                PasswordCandidateOutcome::EggUnrepresentable => self.emit_log(window, task_id, "候选密码不能按所选 EGG 密码编码无损表示，已跳过；未据此判定密码错误或文件损坏", TaskLogSeverity::Info),
                PasswordCandidateOutcome::Unrepresentable(encoding) => self.emit_unrepresentable_candidate(window, task_id, encoding),
            }
        }

        self.emit_log(
            window,
            task_id,
            &format!(
                "内置密码字典已检查 {} 个候选，未找到匹配项；总耗时 {}",
                total,
                Self::elapsed_label(started_at)
            ),
            TaskLogSeverity::Warning,
        );
        Ok(None)
    }

    fn should_log_password_attempt(current: usize, total: usize) -> bool {
        current <= 3 || current == total || current.is_multiple_of(25)
    }

    async fn password_book_candidates(&self) -> Result<Vec<(String, String, String)>> {
        let mut entries = self.encrypted_password_service.list_passwords().await?;
        entries.retain(|entry| !entry.password.is_empty());
        entries.sort_by(|left, right| {
            right
                .favorite
                .cmp(&left.favorite)
                .then_with(|| right.use_count.cmp(&left.use_count))
                .then_with(|| right.last_used.cmp(&left.last_used))
                .then_with(|| left.name.cmp(&right.name))
        });

        Ok(entries
            .into_iter()
            .map(|entry| (entry.id, entry.name, entry.password))
            .collect())
    }

    async fn mark_matching_vault_password_used(&self, password: &str) {
        let Ok(passwords) = self.password_book_candidates().await else {
            return;
        };
        if let Some((entry_id, _, _)) = passwords
            .into_iter()
            .find(|(_, _, candidate)| candidate == password)
        {
            let _ = self.encrypted_password_service.increment_use_count(&entry_id).await;
        }
    }

    async fn attempt_password_book_candidates(&self, file_path: &str, options: &DecompressOptions) -> Result<Option<String>> {
        let passwords = match self.password_book_candidates().await {
            Ok(passwords) => passwords,
            Err(err) => {
                log::error!("Failed to read password book candidates: {}", err);
                return Ok(None);
            }
        };

        for (_entry_id, _entry_name, password) in passwords {
            self.check_cancellation()?;
            match self.test_password_candidate(file_path, &password, options).await? {
                PasswordCandidateOutcome::Matched => return Ok(Some(password)),
                PasswordCandidateOutcome::Mismatch | PasswordCandidateOutcome::Unrepresentable(_) | PasswordCandidateOutcome::AlzUnrepresentable | PasswordCandidateOutcome::EggUnrepresentable => continue,
            }
        }

        Ok(None)
    }

    async fn attempt_passwords_smartly(&self, window: &Window, task_id: &str, file_path: &str, options: &DecompressOptions, attempts: &mut usize) -> Result<Option<String>> {
        self.emit_log(window, task_id, "正在检索密码保险箱...", TaskLogSeverity::Info);
        let started_at = Instant::now();

        let passwords = match self.password_book_candidates().await {
            Ok(res) => res,
            Err(e) => {
                log::error!("获取密码本失败: {}", e);
                self.emit_log(
                    window,
                    task_id,
                    &format!("密码保险箱读取失败，已跳过保险箱尝试：{}", e),
                    TaskLogSeverity::Warning,
                );
                return Ok(None);
            }
        };

        let total = passwords.len();
        if total == 0 {
            self.emit_log(window, task_id, "密码保险箱为空，跳过尝试", TaskLogSeverity::Info);
            return Ok(None);
        }

        self.emit_log(
            window,
            task_id,
            &format!(
                "密码保险箱已载入 {} 个候选，按收藏、使用频率和最近使用排序；实时显示正在验证的密码",
                total
            ),
            TaskLogSeverity::Info,
        );

        for (idx, (_entry_id, entry_name, pwd)) in passwords.iter().enumerate() {
            self.check_cancellation()?;
            let current = idx + 1;
            self.emit_password_attempt_progress(
                window,
                task_id,
                current,
                Some(total),
                format!("保险箱「{}」· 密码「{}」", entry_name, pwd),
            );
            let attempt_started_at = Instant::now();

            *attempts += 1;
            match self.test_password_candidate(file_path, pwd, options).await? {
                PasswordCandidateOutcome::Matched => {
                    self.emit_log(
                        window,
                        task_id,
                        &format!(
                            "保险箱候选 [{}/{}]「{}」· 密码「{}」→ 匹配成功（{}）",
                            current,
                            total,
                            entry_name,
                            pwd,
                            Self::elapsed_label(attempt_started_at)
                        ),
                        TaskLogSeverity::Success,
                    );
                    return Ok(Some(pwd.clone()));
                },
                PasswordCandidateOutcome::Mismatch => self.emit_log(
                    window,
                    task_id,
                    &format!(
                        "保险箱候选 [{}/{}]「{}」· 密码「{}」→ 未匹配（{}）",
                        current,
                        total,
                        entry_name,
                        pwd,
                        Self::elapsed_label(attempt_started_at)
                    ),
                    TaskLogSeverity::Info,
                ),
                PasswordCandidateOutcome::AlzUnrepresentable => self.emit_log(window, task_id, "候选密码不能按所选 ALZ 编码无损表示，已跳过；未据此判定密码错误或文件损坏", TaskLogSeverity::Info),
                PasswordCandidateOutcome::EggUnrepresentable => self.emit_log(window, task_id, "候选密码不能按所选 EGG 密码编码无损表示，已跳过；未据此判定密码错误或文件损坏", TaskLogSeverity::Info),
                PasswordCandidateOutcome::Unrepresentable(encoding) => self.emit_unrepresentable_candidate(window, task_id, encoding),
            }
        }
        
        self.emit_log(
            window,
            task_id,
            &format!(
                "密码保险箱的 {} 个候选均未匹配；总耗时 {}",
                total,
                Self::elapsed_label(started_at)
            ),
            TaskLogSeverity::Warning,
        );
        Ok(None)
    }

    async fn attempt_bruteforce_wordlists_silent(&self, file_path: &str, wordlists: &[String], options: &DecompressOptions) -> Result<Option<String>> {
        let mut tested = HashSet::new();

        for wordlist in wordlists {
            self.check_cancellation()?;

            let file = match File::open(Path::new(wordlist)) {
                Ok(file) => file,
                Err(_) => continue,
            };

            for line in BufReader::new(file).lines() {
                self.check_cancellation()?;

                let password = match line {
                    Ok(value) => value.trim().trim_end_matches('\u{feff}').to_string(),
                    Err(_) => continue,
                };

                if password.is_empty() || !tested.insert(password.clone()) {
                    continue;
                }

                if self.test_password_candidate(file_path, &password, options).await? == PasswordCandidateOutcome::Matched {
                    return Ok(Some(password));
                }
            }
        }

        Ok(None)
    }

    async fn attempt_bruteforce_wordlists(&self, window: &Window, task_id: &str, file_path: &str, wordlists: &[String], options: &DecompressOptions, attempts: &mut usize) -> Result<Option<String>> {
        self.emit_log(
            window,
            task_id,
            &format!(
                "开始尝试 {} 个导入密码词表；实时显示正在验证的密码",
                wordlists.len()
            ),
            TaskLogSeverity::Info,
        );

        let mut tested = HashSet::new();
        let mut attempted = 0usize;
        let started_at = Instant::now();

        for wordlist in wordlists {
            self.check_cancellation()?;

            let path = Path::new(wordlist);
            let file = match File::open(path) {
                Ok(file) => file,
                Err(err) => {
                    self.emit_log(window, task_id, &format!("无法读取密码词表 {}：{}", path.display(), err), TaskLogSeverity::Warning);
                    continue;
                }
            };

            self.emit_log(
                window,
                task_id,
                &format!("正在读取密码词表：{}", path.file_name().and_then(|name| name.to_str()).unwrap_or("未命名词表")),
                TaskLogSeverity::Info,
            );

            for line in BufReader::new(file).lines() {
                self.check_cancellation()?;

                let password = match line {
                    Ok(value) => value.trim().trim_end_matches('\u{feff}').to_string(),
                    Err(err) => {
                        self.emit_log(window, task_id, &format!("词表 {} 存在无法读取的行，已跳过：{}", path.display(), err), TaskLogSeverity::Warning);
                        continue;
                    }
                };

                if password.is_empty() || !tested.insert(password.clone()) {
                    continue;
                }

                attempted += 1;
                self.emit_password_attempt_progress(
                    window,
                    task_id,
                    attempted,
                    None,
                    format!("导入词表 #{} · 密码「{}」", attempted, password),
                );
                let attempt_started_at = Instant::now();
                *attempts += 1;
                match self.test_password_candidate(file_path, &password, options).await? {
                    PasswordCandidateOutcome::Matched => {
                        self.emit_log(
                            window,
                            task_id,
                            &format!(
                                "导入词表候选 #{} · 密码「{}」→ 匹配成功（{}）",
                                attempted,
                                password,
                                Self::elapsed_label(attempt_started_at)
                            ),
                            TaskLogSeverity::Success,
                        );
                        return Ok(Some(password));
                    }
                    PasswordCandidateOutcome::Mismatch => {
                        if attempted == 1 || attempted.is_multiple_of(25) {
                            self.emit_log(
                                window,
                                task_id,
                                &format!(
                                    "导入词表已验证 {} 个候选；密码「{}」未匹配（{}）",
                                    attempted,
                                    password,
                                    Self::elapsed_label(attempt_started_at)
                                ),
                                TaskLogSeverity::Info,
                            );
                        }
                    }
                    PasswordCandidateOutcome::AlzUnrepresentable => self.emit_log(window, task_id, "候选密码不能按所选 ALZ 编码无损表示，已跳过；未据此判定密码错误或文件损坏", TaskLogSeverity::Info),
                    PasswordCandidateOutcome::EggUnrepresentable => self.emit_log(window, task_id, "候选密码不能按所选 EGG 密码编码无损表示，已跳过；未据此判定密码错误或文件损坏", TaskLogSeverity::Info),
                    PasswordCandidateOutcome::Unrepresentable(encoding) => self.emit_unrepresentable_candidate(window, task_id, encoding),
                }
            }
        }

        self.emit_log(
            window,
            task_id,
            &format!(
                "导入密码词表已检查 {} 个候选，未找到匹配项；总耗时 {}",
                attempted,
                Self::elapsed_label(started_at)
            ),
            TaskLogSeverity::Warning,
        );
        Ok(None)
    }

    async fn archive_requires_password(&self, file_path: &str, format: ArchiveFormat) -> Result<bool> {
        match format {
            ArchiveFormat::Alz => crate::services::alz_engine::requires_password(Path::new(file_path), &Default::default(), self.cancellation_flag.clone()).await,
            ArchiveFormat::Egg => crate::services::egg_engine::requires_password(Path::new(file_path), &Default::default(), self.cancellation_flag.clone()).await,
            ArchiveFormat::Zpaq => crate::services::zpaq_engine::requires_password(Path::new(file_path)),
            // 通过 7z CLI 处理的格式
            ArchiveFormat::Universal | ArchiveFormat::Iso | ArchiveFormat::Cab |
            ArchiveFormat::Lzh | ArchiveFormat::Arj | ArchiveFormat::Dmg |
            ArchiveFormat::Wim | ArchiveFormat::Vhd | ArchiveFormat::Chm |
            ArchiveFormat::Deb | ArchiveFormat::Rpm | ArchiveFormat::SquashFs |
            ArchiveFormat::Nsis | ArchiveFormat::Msi | ArchiveFormat::Xar |
            ArchiveFormat::Cpio | ArchiveFormat::Udf | ArchiveFormat::Fat |
            ArchiveFormat::Ntfs | ArchiveFormat::Hfs |
            ArchiveFormat::Arc | ArchiveFormat::Apfs | ArchiveFormat::Ext => {
                self.universal_engine.requires_password(Path::new(file_path)).await
            }
            ArchiveFormat::Zip => {
                UniversalCliEngine::zip_requires_password(Path::new(file_path))
            }
            ArchiveFormat::SevenZip => {
                native_extraction::seven_zip::requires_password(Path::new(file_path))
            }
            ArchiveFormat::Rar => {
                let path = PathBuf::from(file_path);
                let flag = self.cancellation_flag.clone();
                tokio::task::spawn_blocking(move || RarSupportService::requires_password(&path, &flag)).await?
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
        let source = SplitArchiveDetector::complete_source(Path::new(file_path))?;
        let path = source.as_path();
        if crate::services::sfx_archive::is_executable_path(path) {
            return crate::services::archive_browser::detect_format(path);
        }
        let file = crate::services::numeric_volume_reader::ZipInputReader::open_seven_zip(path)?;
        if matches!(&file, crate::services::numeric_volume_reader::ZipInputReader::StandardZip(_)) {
            return Ok(ArchiveFormat::Zip);
        }
        let mut header = Vec::with_capacity(512);
        file.take(512).read_to_end(&mut header)?;
        let detected = ArchiveFormat::from_magic(&header);
        if detected != ArchiveFormat::Unknown {
            return Ok(detected);
        }

        let logical = if crate::services::numeric_volume_reader::ZipInputReader::is_numeric_first(path) { Path::new(path.file_stem().unwrap()) } else { path };
        let ext = logical.extension().and_then(|value| value.to_str()).unwrap_or("").to_ascii_lowercase();
        Ok(ArchiveFormat::from_password_extension(&ext))
    }

    pub async fn verify_archive_password_candidate(&self, file_path: &str, password: &str) -> Result<bool> {
        self.verify_archive_password_candidate_with_encoding(file_path, password, None).await
    }

    pub async fn verify_archive_password_candidate_with_encoding(&self, file_path: &str, password: &str, encoding: Option<ZipPasswordEncoding>) -> Result<bool> {
        self.check_cancellation()?;
        let source = SplitArchiveDetector::complete_source(Path::new(file_path))?;
        let file_path = source.to_str().ok_or_else(|| anyhow::anyhow!("无效归档路径"))?;
        if Self::detect_password_format(file_path)? == ArchiveFormat::Egg {
            return crate::services::egg_engine::try_password(Path::new(file_path), password, &DecompressOptions { zip_password_encoding: encoding, ..Default::default() }, self.cancellation_flag.clone()).await;
        }
        if Self::detect_password_format(file_path)? == ArchiveFormat::Alz {
            return crate::services::alz_engine::try_password(Path::new(file_path), password,
                &DecompressOptions { zip_password_encoding: encoding, ..Default::default() }, self.cancellation_flag.clone()).await;
        }
        if Self::detect_password_format(file_path)? == ArchiveFormat::Zip {
            return self.test_extraction_password(file_path, password, &DecompressOptions {
                zip_password_encoding: encoding, ..Default::default()
            }).await;
        }
        let format = Self::detect_password_format(file_path)?;

        if !self.archive_requires_password(file_path, format).await? {
            return Ok(false);
        }

        self.test_archive_password(file_path, password).await
    }

    fn source_archive_paths_for_recycle_bin(file_path: &Path) -> Result<Vec<PathBuf>> {
        match SplitArchiveDetector::detect_split_archive(file_path)? {
            Some(info) if !info.parts.is_empty() => Ok(info.parts),
            _ => Ok(vec![file_path.to_path_buf()]),
        }
    }

    fn move_source_archives_to_system_recycle_bin(file_path: &Path) -> Result<usize> {
        let source_paths = Self::source_archive_paths_for_recycle_bin(file_path)?;
        crate::services::source_recycle::move_paths_to_system_recycle_bin(&source_paths)
    }

    fn validate_recycle_bin_options(options: &DecompressOptions) -> Result<()> {
        let filtered = options
            .file_filter
            .as_deref()
            .is_some_and(|filter| !filter.trim().is_empty());
        if options.delete_after
            && (options.skip_corrupted
                || options.extract_only_newer
                || filtered
                || !options.selected_entries.is_empty()
                || options.conflict_policy == "skip")
        {
            return Err(CompressionError::ExtractionFailed(
                "部分解压、容错解压或跳过冲突时不能将源压缩包移入系统回收站".to_string(),
            )
            .into());
        }
        Ok(())
    }

    fn resolve_extraction_source(selected: &Path) -> Result<String> {
        Self::source_file_snapshot(selected, "Pre-checking")?;
        if crate::services::egg_volumes::is_candidate(selected) {
            return Ok(crate::services::egg_volumes::complete(selected)?.first.to_string_lossy().into_owned());
        }
        if crate::services::alz_volumes::is_candidate(selected) {
            return Ok(crate::services::alz_volumes::complete(selected)?.first.to_string_lossy().into_owned());
        }
        let path = if let Some(group) = SplitArchiveDetector::detect_split_archive(selected)? {
            anyhow::ensure!(group.is_complete, "[archive-inspection:incomplete] 缺少分卷：{}", group.missing_part_names());
            group.first_part
        } else { selected.to_path_buf() };
        Ok(path.to_string_lossy().into_owned())
    }

    pub async fn extract(&self, window: Window, task_id: String, file_path: String, output_dir: Option<String>, password: Option<String>, options: DecompressOptions) -> Result<String> {
        let file_path = Self::resolve_extraction_source(Path::new(&file_path))?;
        let snapshot = Self::source_file_snapshot(Path::new(&file_path), "Pre-checking")?;
        let executing = AtomicBool::new(false);
        let result = self.extract_with_source_snapshot(window, task_id, file_path.clone(), output_dir, password, options, snapshot.clone(), &executing).await;
        if let Err(error) = result {
            let message = format!("{error:#}");
            if !self.cancellation_flag.load(Ordering::Relaxed) && !message.contains("[archive-source:")
                && !message.contains("[archive-output:") {
                let stage = if executing.load(Ordering::Relaxed) { "Extracting" } else { "Pre-checking" };
                match Self::source_file_changed(Path::new(&file_path), &snapshot, stage) {
                    Ok(true) => return Err(anyhow::Error::new(if stage == "Extracting" {
                        CompressionError::SourceChangedDuringExtraction
                    } else { CompressionError::SourceChangedDuringPrecheck })
                        .context(format!("原处理未完成：{message}"))),
                    Err(source_error) => return Err(source_error.context(format!("原处理未完成：{message}"))),
                    Ok(false) => {}
                }
            }
            return Err(error);
        }
        result
    }

    async fn extract_with_source_snapshot(&self, window: Window, task_id: String, file_path: String,
        output_dir: Option<String>, password: Option<String>, options: DecompressOptions,
        source_snapshot: SourceFileSnapshot, executing: &AtomicBool) -> Result<String> {
        let service = self.clone();
        let path = Path::new(&file_path);
        if source_snapshot.len == 0 {
            return Err(CompressionError::ExtractionFailed(
                "压缩包为空，可能仍在下载、复制或尚未写入完成".to_string(),
            ).into());
        }
        if !matches!(options.conflict_policy.as_str(), "ask" | "overwrite" | "skip" | "rename") {
            return Err(CompressionError::ExtractionFailed(format!(
                "Unsupported conflict policy: {}",
                options.conflict_policy
            )).into());
        }
        Self::validate_recycle_bin_options(&options)?;
        let final_out_dir = Self::extraction_output_path(path, output_dir.as_deref(), options.create_subdirectory);

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
        let reader = crate::services::numeric_volume_reader::ZipInputReader::open_seven_zip(path)?;
        let mut header = Vec::with_capacity(512);
        reader.take(512).read_to_end(&mut header)?;
        let mut format = ArchiveFormat::from_magic(&header);
        if crate::services::sfx_archive::is_executable_path(path) {
            format = crate::services::archive_browser::detect_format(path)?;
        }

        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        if path.to_string_lossy().to_ascii_lowercase().ends_with(".7z.001") {
            // Read across even very small volumes and propagate missing-volume
            // errors instead of hiding them as unknown format/password errors.
            format = Self::detect_password_format(&file_path)?;
        }
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("未知文件").to_string();
        let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

        // The .zip member is the final directory volume in standard ZIP sets;
        // it must not be treated as a standalone ZIP after UI canonicalization.
        let standard_zip_group = SplitArchiveDetector::detect_split_archive(path)?
            .filter(|group| group.format == crate::services::split_archive_detector::SplitFormat::ZipSplit);
        if let Some(group) = standard_zip_group {
            anyhow::ensure!(group.is_complete, "[archive-inspection:incomplete] 缺少 ZIP 分卷：{}", group.missing_part_names());
            format = ArchiveFormat::Zip;
        }

        // 分卷压缩包检测：扩展名为数字编号(.001, .002...)或 .z01/.z02 后缀
        // 7Z 数字首卷使用可 Seek 的原生卷组读取；其他分卷保留通用路由。
        let is_split_archive = {
            let ext_is_numeric = ext.len() >= 3 && ext.chars().all(|c| c.is_ascii_digit());
            let ext_is_zsplit = ext.len() == 3 && ext.starts_with('z') && ext[1..].chars().all(|c| c.is_ascii_digit());
            // 也检查 stem 是否以 .zip / .7z / .rar 结尾（如 test_split.zip.001）
            let stem_has_archive_ext = file_stem.ends_with(".zip") || file_stem.ends_with(".7z") || file_stem.ends_with(".rar");
            ext_is_numeric || ext_is_zsplit || stem_has_archive_ext
        };

        if format == ArchiveFormat::Unknown && crate::services::numeric_volume_reader::ZipInputReader::is_numeric_first(path) {
            format = ArchiveFormat::from_extension(Path::new(file_stem).extension().and_then(|value| value.to_str()).unwrap_or(""));
        }
        anyhow::ensure!(options.zpaq_catalog_id.is_none() || (format==ArchiveFormat::Zpaq&&options.zpaq_snapshot.is_some()), "变化提取必须使用原ZPAQ快照，请重新对比");
        anyhow::ensure!(options.zpaq_snapshot.is_none() || format == ArchiveFormat::Zpaq,
            "所选快照只适用于 ZPAQ 日志归档，请重新选择原归档");
        let native_seven_zip = Self::uses_native_seven_zip_volumes(path, &format);
        let native_stream = matches!(format, ArchiveFormat::Wim | ArchiveFormat::Zpaq | ArchiveFormat::AesEncrypted | ArchiveFormat::Tar | ArchiveFormat::Gzip | ArchiveFormat::Bzip2 | ArchiveFormat::Xz | ArchiveFormat::Zstd | ArchiveFormat::Lz4 | ArchiveFormat::Brotli | ArchiveFormat::Lzip | ArchiveFormat::Lzma);
        if is_split_archive && !native_seven_zip && !native_stream && format != ArchiveFormat::Zip && format != ArchiveFormat::Unknown && format != ArchiveFormat::Universal {
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

        let _ = window.emit(
            "archive-format-detected",
            ArchiveFormatDetectedPayload {
                task_id: task_id.clone(),
                format: Self::extraction_history_format(path, &format),
            },
        );
        service.emit_log(&window, &task_id, &format!("确定解压格式: {:?} (后缀: {})", format, ext), TaskLogSeverity::Info);

        let mut final_password = password.clone();
        let mut password_verified = false;
        let scoped_zip = format == ArchiveFormat::Zip && crate::services::zip_selection::has_selection(&options);
        let password_required = if format == ArchiveFormat::Egg {
            Some(crate::services::egg_engine::requires_password(path, &options, service.cancellation_flag.clone()).await?)
        } else if format == ArchiveFormat::Alz {
            Some(crate::services::alz_engine::requires_password(path, &options, service.cancellation_flag.clone()).await?)
        } else if scoped_zip {
            Some(UniversalCliEngine::zip_requires_password_for_selection(path, &options)?)
        } else if format.supports_password() {
            service.emit_log(
                &window,
                &task_id,
                "正在检测归档加密状态...",
                TaskLogSeverity::Info,
            );
            Some(service.archive_requires_password(&file_path, format.clone()).await.map_err(|err| {
                let message = format!("归档检测失败：{}", err);
                CompressionError::ExtractionFailed(message)
            })?)
        } else {
            None
        };

        if password_required == Some(false) {
            if final_password.is_some() {
                service.emit_log(&window, &task_id, "本次解压内容无需密码，已忽略多余的密码参数", TaskLogSeverity::Info);
                final_password = None;
            }
            service.emit_log(
                &window,
                &task_id,
                "加密状态检测完成：本次解压内容无需密码，正在准备解压...",
                TaskLogSeverity::Success,
            );
        }

        if final_password.is_none() && password_required == Some(true) {
            let needs_pwd = true;

            if needs_pwd {
                service.emit_log(&window, &task_id, "检测到加密格式，正在启动自动解锁流程...", TaskLogSeverity::Info);
                if let Some(smart_pwd) = service.resolve_archive_password(&window, &task_id, &file_path, &options).await? {
                    service.emit_log(
                        &window,
                        &task_id,
                        &format!("已取得有效解压凭据：密码「{}」，准备执行安全预检", smart_pwd),
                        TaskLogSeverity::Success,
                    );
                    final_password = Some(smart_pwd);
                    password_verified = true;
                } else {
                    let unresolved_message = if format == ArchiveFormat::SevenZip {
                        "未取得能完整验证此 7Z 归档的密码；请手动核对密码，并检查文件或分卷是否损坏"
                    } else if format == ArchiveFormat::Alz {
                        "未取得能完整验证此 ALZ 归档的密码；请核对密码、原包编码和文件完整性"
                    } else if format == ArchiveFormat::Egg {
                        "未取得能完整验证此 EGG 归档的密码；请核对密码、原包密码编码和文件完整性"
                    } else if format == ArchiveFormat::Rar {
                        "未取得能完整验证此 RAR 归档的密码；请核对密码，或检查归档及分卷是否损坏"
                    } else {
                        "所有已知密码均无效，等待手动输入"
                    };
                    service.emit_log(&window, &task_id, unresolved_message, TaskLogSeverity::Warning);
                    
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

        if password_required == Some(true) && password.is_some() {
            service.emit_log(
                &window,
                &task_id,
                &format!(
                    "已收到手动输入的解压密码「{}」，正在安全预检",
                    final_password.as_deref().unwrap_or_default()
                ),
                TaskLogSeverity::Info,
            );
        }

        if password_required == Some(true) && final_password.is_some() && !password_verified {
            let candidate = final_password.as_deref().unwrap_or_default();
            if !service.test_extraction_password(&file_path, candidate, &options).await? {
                // 7Z has no independent password verifier: failed decoding can
                // also mean damaged ciphertext. Candidate iteration may continue,
                // but the final user-facing result must not assert a wrong password.
                if format == ArchiveFormat::SevenZip {
                    return Err(native_extraction::seven_zip::ambiguous_password_error().into());
                }
                if format == ArchiveFormat::Rar {
                    return Err(CompressionError::ExtractionFailed("[archive-inspection:ambiguous-data] RAR 密码预检未能通过内容校验：可能是密码不匹配，也可能是归档损坏或分卷混用。请核对密码，或检查并尝试修复。".into()).into());
                }
                if format == ArchiveFormat::Alz {
                    return Err(CompressionError::ExtractionFailed("[archive-inspection:ambiguous-data] ALZ 密码未能通过完整内容校验，请核对密码，并检查加密头或内容是否损坏。".into()).into());
                }
                if format == ArchiveFormat::Egg {
                    return Err(CompressionError::ExtractionFailed("[archive-inspection:ambiguous-data] EGG 密码未能通过完整内容校验，请核对密码及原包密码编码（原中文 ALZip 可使用 CP936），并检查加密头或内容是否损坏。".into()).into());
                }
                if format == ArchiveFormat::Zpaq {
                    return Err(CompressionError::ExtractionFailed("[archive-inspection:ambiguous-data] ZPAQ 密码不匹配或文件已损坏，请核对密码和源文件。".into()).into());
                }
                return Err(CompressionError::InvalidPassword.into());
            }
            password_verified = true;
        }

        let tar_plan = service.preflight_tar_extraction(path, &format, &final_out_dir, &options).await?;
        let preflight_stats = if let Some(plan) = &tar_plan {
            Some((plan.entries, plan.logical_bytes))
        } else {
            service.preflight_extraction(path, &format, final_password.as_deref(), &final_out_dir, &options).await?
        };
        let expected_expanded_bytes = preflight_stats.map(|(_, expanded_bytes)| expanded_bytes);
        if password_required == Some(true) && password_verified {
            service.emit_log(
                &window,
                &task_id,
                &format!(
                    "密码验证通过：本次解压使用密码「{}」",
                    final_password.as_deref().unwrap_or_default()
                ),
                TaskLogSeverity::Success,
            );
        }
        if Self::source_file_changed(path, &source_snapshot, "Pre-checking")? {
            return Err(CompressionError::SourceChangedDuringPrecheck.into());
        }
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
        let extraction_progress = Arc::new(Mutex::new(0.0f32));
        let progress_for_callback = extraction_progress.clone();
        let expanded_for_progress = expected_expanded_bytes;
        let on_progress: Arc<dyn Fn(f32) + Send + Sync> = Arc::new(move |p| {
            if let Ok(mut latest) = progress_for_callback.lock() {
                *latest = p;
            }
            let total = expanded_for_progress.unwrap_or(0);
            let processed = ((total as f64) * (p.clamp(0.0, 1.0) as f64)).round() as u64;
            srv_progress.emit_progress_with_output_estimate(
                &win_progress,
                &tid_progress,
                p,
                None,
                ProgressBytes {
                    processed,
                    total,
                    output_estimated: total > 0,
                },
            );
        });

        let win_log = window.clone();
        let tid_log = task_id.clone();
        let srv_log = service.clone();
        let progress_for_log = extraction_progress.clone();
        let expanded_for_log = expected_expanded_bytes;
        let last_file_log = Arc::new(Mutex::new(None::<Instant>));
        let last_file_log_for_callback = last_file_log.clone();
        let on_log: Arc<dyn Fn(String, TaskLogSeverity) + Send + Sync> = Arc::new(move |msg, severity| {
            if let Some(current_file) = msg.strip_prefix("正在解压：") {
                let progress = progress_for_log.lock().map(|value| *value).unwrap_or(0.0);
                let total = expanded_for_log.unwrap_or(0);
                let processed = ((total as f64) * (progress.clamp(0.0, 1.0) as f64)).round() as u64;
                srv_log.emit_progress_with_output_estimate(
                    &win_log,
                    &tid_log,
                    progress,
                    Some(current_file.to_string()),
                    ProgressBytes {
                        processed,
                        total,
                        output_estimated: total > 0,
                    },
                );
                let should_log = last_file_log_for_callback
                    .lock()
                    .map(|mut last| {
                        let should_log = last
                            .as_ref()
                            .is_none_or(|instant| instant.elapsed() >= std::time::Duration::from_secs(1));
                        if should_log {
                            *last = Some(Instant::now());
                        }
                        should_log
                    })
                    .unwrap_or(true);
                if !should_log {
                    return;
                }
            }
            srv_log.emit_log(&win_log, &tid_log, &msg, severity);
        });

        service.emit_log(
            &window,
            &task_id,
            &format!("开始解压：{}", file_name),
            TaskLogSeverity::Info,
        );
        // Explicitly leave preflight/password verification before the first
        // engine-specific callback. This is the exact point extraction begins.
        service.emit_progress(&window, &task_id, 0.0, None, 0, expected_expanded_bytes.unwrap_or(0));
        service.begin_progress_telemetry(&task_id);

        // ZIP, encrypted ZIP and both volume layouts share one selected-entry
        // reader. Routing encrypted ZIP through the option-less CLI interface
        // would read unselected entries before the staging filter can run.
        let effective_format = format;

        executing.store(true, Ordering::Relaxed);
        let result = match effective_format {
            ArchiveFormat::Egg => crate::services::egg_engine::extract(
                path, &out_dir, final_password.as_deref(), &options, service.cancellation_flag.clone()).await,
            ArchiveFormat::Alz => crate::services::alz_engine::extract(
                path, &out_dir, final_password.as_deref(), &options, service.cancellation_flag.clone()).await,
            ArchiveFormat::Zpaq => crate::services::zpaq_engine::extract_checked_at(
                path, &out_dir, final_password.as_deref(), &options.selected_entries, options.zpaq_snapshot.as_ref(), options.zpaq_catalog_id.as_deref(), service.cancellation_flag.clone()).await,
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
                let srv = service.clone();
                let w = window.clone();
                let id = task_id.clone();
                let total = expected_expanded_bytes.unwrap_or(0);
                let last = Mutex::new(None::<(Instant, String, u64)>);
                let progress_control = service.cancellation_flag.clone();
                let on_bytes = Arc::new(move |processed: u64, current_file: String| {
                    // Limit UI events, not decoder callbacks or cancellation.
                    let mut last = last.lock().unwrap_or_else(|p| p.into_inner());
                    if !super::task_control::is_paused(&progress_control) && last.as_ref().is_some_and(|(at, file, bytes)| file == &current_file && *bytes > 0 && at.elapsed().as_millis() < 100 && (total == 0 || processed < total)) { return; }
                    *last = Some((Instant::now(), current_file.clone(), processed));
                    let progress = Self::extraction_progress_from_output(processed, total).unwrap_or(0.0);
                    // Decoded bytes are not a promise of CRC/transaction success.
                    srv.emit_progress(&w, &id, progress, Some(current_file), processed, total);
                });
                let rar = service.rar_service.clone();
                let flag = service.cancellation_flag.clone();
                let runtime = tokio::runtime::Handle::current();
                let f_path = file_path.clone();
                let o_dir = out_dir.clone();
                let pwd = final_password.clone();
                let opts = options.clone();
                tokio::task::spawn_blocking(move || runtime.block_on(rar.extract_rar_with_progress(
                    Path::new(&f_path), &o_dir, pwd.as_deref(), &opts, flag, on_bytes,
                ))).await?.map_err(|e| anyhow::anyhow!("RAR 解压失败: {}", e))
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
            ref codec_format @ (ArchiveFormat::Lz4 | ArchiveFormat::Brotli | ArchiveFormat::Lzip | ArchiveFormat::Lzma) => {
                let srv = service.clone();
                let f_path = file_path.clone();
                let o_dir = out_dir.clone();
                let opts = options.clone();
                let t_id = task_id.clone();
                let w = window.clone();
                let codec = match codec_format { ArchiveFormat::Lz4 => "lz4", ArchiveFormat::Lzip => "lz", ArchiveFormat::Lzma => "lzma", _ => "br" };
                tokio::task::spawn_blocking(move || {
                    let decoder = crate::services::codec_stream::decoder(Path::new(&f_path), codec)?;
                    if compression_format::is_tar_wrapped_archive(Path::new(&f_path), &format!("tar.{codec}")) {
                        srv.do_extract_tar(&w, &t_id, &f_path, &o_dir, Some(decoder), &opts)
                    } else {
                        let original = Path::new(&f_path);
                        let name = native_extraction::single_stream::output_name(original, &[&format!(".{codec}")]);
                        native_extraction::single_stream::extract(&srv, &w, &t_id, decoder, &o_dir, name, &opts)
                    }
                }).await?
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
            ArchiveFormat::Wim => service.universal_engine.extract_wim_with_progress(
                Path::new(&file_path), &out_dir, options.overwrite_existing,
                on_progress.clone(), on_log.clone(), service.cancellation_flag.clone(), &options.selected_entries,
            ).await,
            // 所有通过 7z CLI 处理的格式
            ArchiveFormat::Iso | ArchiveFormat::Cab |
            ArchiveFormat::Lzh | ArchiveFormat::Arj | ArchiveFormat::Dmg |
            ArchiveFormat::Vhd | ArchiveFormat::Chm |
            ArchiveFormat::Deb | ArchiveFormat::Rpm | ArchiveFormat::SquashFs |
            ArchiveFormat::Nsis | ArchiveFormat::Msi | ArchiveFormat::Xar |
            ArchiveFormat::Cpio | ArchiveFormat::Udf | ArchiveFormat::Fat |
            ArchiveFormat::Ntfs | ArchiveFormat::Hfs |
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

        let extraction_result = result.map_err(Self::normalize_storage_full_error);
        let source_changed = Self::source_file_changed(path, &source_snapshot, "Extracting")?;
        if let Err(error) = extraction_result {
            if let Err(cleanup_error) = staging.cleanup() {
                service.emit_log(
                    &window,
                    &task_id,
                    &format!("Unable to clean incomplete extraction output: {}", cleanup_error),
                    TaskLogSeverity::Warning,
                );
            }
            if source_changed {
                return Err(CompressionError::SourceChangedDuringExtraction.into());
            }
            return Err(error);
        }
        if source_changed {
            let _ = staging.cleanup();
            return Err(CompressionError::SourceChangedDuringExtraction.into());
        }
        if let Some(password) = final_password.as_deref() {
            service.mark_matching_vault_password_used(password).await;
        }
        if let Err(error) = extraction_transaction::prepare_staging_layout_with_tar_plan(path, &out_dir, &options, tar_plan.as_ref()) {
            let _ = staging.cleanup();
            return Err(error);
        }
        service.check_cancellation()?;
        if let Some(mark) = mark_of_web.as_ref() {
            match mark_of_web::propagate_to_tree(&out_dir, mark, || {
                service.check_cancellation().is_err()
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
        let mut observed_conflicts = HashMap::new();
        let source_times = if effective_format == ArchiveFormat::SevenZip && options.conflict_policy == "ask" {
            Self::seven_zip_source_times(path, final_password.as_deref(), &final_out_dir, options.preserve_paths).unwrap_or_default()
        } else { HashMap::new() };
        let commit_result = service.commit_staged_extraction_recording_with_times(
            Some(&window),
            &task_id,
            &file_path,
            &out_dir,
            &final_out_dir,
            &options,
            &mut observed_conflicts,
            &source_times,
        );
        if let Err(error) = commit_result {
            if Self::is_conflict_resolution_required(&error) {
                service.emit_log(
                    &window,
                    &task_id,
                    "解压数据已安全保留，等待冲突策略；确认后只提交暂存文件，不会重新解压",
                    TaskLogSeverity::Warning,
                );
                PENDING_EXTRACTIONS.insert(
                    task_id.clone(),
                    PendingExtraction {
                        source_times,
                        staging,
                        source_archive: path.to_path_buf(),
                        final_output: final_out_dir,
                        options,
                        expected_expanded_bytes,
                        observed_conflicts,
                    },
                );
                return Err(error);
            }
            if !staging.retain_after_incomplete_rollback(&error) {
                let _ = staging.cleanup();
            }
            return Err(error);
        }
        let published_bytes = commit_result?;
        if let Some(warning) = staging.cleanup_after_commit() {
            service.emit_log(
                &window,
                &task_id,
                &warning,
                TaskLogSeverity::Warning,
            );
        }
        service.finish_committed_extraction(
            &window, &task_id, path, &final_out_dir, &options, expected_expanded_bytes, published_bytes,
        ).await
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

    fn source_access_error(error: std::io::Error, stage: &'static str) -> CompressionError {
        let (code, summary) = if error.kind() == std::io::ErrorKind::NotFound {
            ("source-missing", "源压缩包不存在或已被移动，请重新选择源文件")
        } else {
            ("source-unavailable", "无法检查源压缩包，请确认路径、访问权限和存储设备状态")
        };
        CompressionError::SourceAccess { stage, code, detail: format!("{summary}：{error}") }
    }

    fn source_file_snapshot(path: &Path, stage: &'static str) -> Result<SourceFileSnapshot> {
        let metadata = std::fs::metadata(path).map_err(|error| Self::source_access_error(error, stage))?;
        if !metadata.is_file() {
            return Err(CompressionError::SourceAccess {
                stage, code: "source-invalid", detail: "源路径不是普通文件，请选择压缩包文件".into(),
            }.into());
        }
        let paths = if crate::services::egg_volumes::is_candidate(path) {
            crate::services::egg_volumes::complete(path)?.parts
        } else if crate::services::alz_volumes::is_candidate(path) {
            crate::services::alz_volumes::complete(path)?.parts
        } else if let Some(group) = SplitArchiveDetector::detect_split_archive(path)? {
            anyhow::ensure!(group.is_complete, "[archive-inspection:incomplete] 缺少分卷：{}", group.missing_part_names());
            group.parts
        } else { vec![path.to_path_buf()] };
        let members = paths.into_iter().map(|part| {
            let m = std::fs::metadata(&part).map_err(|error| Self::source_access_error(error, stage))?;
            anyhow::ensure!(m.is_file(), "[archive-source:{stage}:source-invalid] 分卷不是普通文件：{}", part.display());
            Ok((part, m.len(), m.modified().ok(), m.created().ok()))
        }).collect::<Result<Vec<_>>>()?;
        Ok(SourceFileSnapshot { len: metadata.len(), modified: metadata.modified().ok(), members })
    }

    fn source_file_changed(path: &Path, original: &SourceFileSnapshot, stage: &'static str) -> Result<bool> {
        // Recheck every previously observed member before rediscovering the group:
        // a missing later disk must retain its actual path, not become wrong-password.
        for (part, len, modified, created) in &original.members {
            let m = std::fs::metadata(part).map_err(|error| {
                let mut access = Self::source_access_error(error, stage);
                if let CompressionError::SourceAccess { detail, .. } = &mut access {
                    detail.push_str(&format!("；原卷：{}", part.display()));
                }
                access
            })?;
            if !m.is_file() || m.len() != *len || m.modified().ok() != *modified || m.created().ok() != *created {
                return Ok(true);
            }
        }
        let current = Self::source_file_snapshot(path, stage).map_err(|error| CompressionError::SourceAccess {
            stage, code: "source-changed",
            detail: format!("处理中卷组无法按原状态读取，当前结果未提交，请确认同组原卷后创建新任务：{error:#}"),
        })?;
        Ok(current != *original)
    }

    fn ensure_no_link_ancestors(path: &Path) -> Result<()> {
        extraction_transaction::ensure_no_link_ancestors(path)
    }

    fn uses_native_seven_zip_volumes(path: &Path, format: &ArchiveFormat) -> bool {
        *format == ArchiveFormat::SevenZip && (crate::services::numeric_volume_reader::ZipInputReader::is_numeric_first(path)
            || SplitArchiveDetector::is_haozip_seven_zip(path))
    }

    async fn preflight_tar_extraction(
        &self,
        archive_path: &Path,
        format: &ArchiveFormat,
        output: &Path,
        options: &DecompressOptions,
    ) -> Result<Option<crate::services::tar_compatibility::Plan>> {
        let Some(route) = crate::services::tar_compatibility::integrity_route(archive_path, format) else { return Ok(None); };
        let service = self.clone();
        let path = archive_path.to_path_buf();
        let options = options.clone();
        let plan = tokio::task::spawn_blocking(move || crate::services::tar_compatibility::plan(&path, route, &options, || service.check_cancellation())).await??;
        extraction_transaction::validate_sparse_resource_limits(archive_path, plan.entries, plan.logical_bytes, plan.payload_bytes)?;
        extraction_transaction::validate_disk_capacity(output.parent().unwrap_or(output), plan.logical_bytes)?;
        Ok(Some(plan))
    }

    async fn preflight_extraction(
        &self,
        archive_path: &Path,
        format: &ArchiveFormat,
        password: Option<&str>,
        output: &Path,
        options: &DecompressOptions,
    ) -> Result<Option<(usize, u64)>> {
        if let Some(plan) = self.preflight_tar_extraction(archive_path, format, output, options).await? {
            return Ok(Some((plan.entries,plan.logical_bytes)));
        }
        let stats = match format {
            ArchiveFormat::Egg => {
                let directory = crate::services::egg_engine::plan(archive_path, options, self.cancellation_flag.clone()).await?;
                Some((directory.entries.len(), directory.total_uncompressed_size))
            }
            ArchiveFormat::Alz => {
                let directory = crate::services::alz_engine::plan(archive_path, options, self.cancellation_flag.clone()).await?;
                Some((directory.entries.len(), directory.total_uncompressed_size))
            }
            ArchiveFormat::Zpaq => {
                if let Some(expected)=options.zpaq_catalog_id.as_deref(){crate::services::zpaq_engine::check_catalog(archive_path,password,options.zpaq_snapshot.as_ref().context("变化提取缺少来源快照")?,expected,self.cancellation_flag.clone()).await?;}
                let directory = crate::services::zpaq_engine::browse_at(archive_path, password, options.zpaq_snapshot.as_ref(), self.cancellation_flag.clone()).await?;
                if directory.entries.iter().all(|e| e.size_known != Some(false)) {
                    Some((directory.entries.len(), directory.total_uncompressed_size))
                } else { None }
            }
            ArchiveFormat::Zip => {
                let file = crate::services::numeric_volume_reader::ZipInputReader::open(archive_path)?;
                let mut archive = zip_aes::ZipArchive::new(file)?;
                let selection = crate::services::zip_selection::ZipSelection::read(&mut archive, options, || self.check_cancellation())?;
                Some((selection.indices.len(), selection.expanded_bytes))
            }
            ArchiveFormat::SevenZip => {
                let mut input = crate::services::numeric_volume_reader::ZipInputReader::open_seven_zip(archive_path)?;
                let length = input.seek(SeekFrom::End(0))?;
                input.seek(SeekFrom::Start(0))?;
                let reader = sevenz_rust::SevenZReader::new(
                    input, length,
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
                let service = self.clone();
                let path = archive_path.to_path_buf();
                let password = password.map(str::to_owned);
                let options = options.clone();
                // Header walking and pause gates must not occupy an async
                // runtime worker. Keep full stats and the resource checks below.
                Some(tokio::task::spawn_blocking(move || {
                    crate::services::rar_preflight::stats_with_options(&path, password.as_deref(), &options, || service.check_cancellation())
                }).await.map_err(|error| CompressionError::ExtractionFailed(
                    format!("RAR 预检查线程异常退出: {error}")
                ))??)
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
        Ok(stats)
    }

    fn prepare_staging_layout(
        &self,
        archive_path: &Path,
        staging: &Path,
        options: &DecompressOptions,
    ) -> Result<()> {
        extraction_transaction::prepare_staging_layout(archive_path, staging, options)
    }

    pub fn extraction_output_path(path: &Path, output_dir: Option<&str>, create_subdirectory: bool) -> PathBuf {
        let mut output = output_dir.map(PathBuf::from).unwrap_or_else(|| {
            path.parent().unwrap_or(Path::new(".")).to_path_buf()
        });
        if create_subdirectory { output = output.join(Self::archive_output_dir_name(path)); }
        output
    }

    fn archive_output_dir_name(path: &Path) -> String {
        let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("archive");
        if crate::services::egg_volumes::is_candidate(path) {
            return regex::Regex::new(r"(?i)\.vol\d+\.egg$|\.egg$").unwrap().replace(file_name, "").into_owned();
        }
        if crate::services::alz_volumes::is_candidate(path) {
            return path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        }
        let mut candidate = file_name.to_string();
        if let Some(captures) = regex::Regex::new(r"(?i)^(.*\.)?haozip\d{2,5}\.(zip|7z)$").unwrap().captures(file_name) {
            return captures.get(1).map(|prefix| prefix.as_str().trim_end_matches('.'))
                .filter(|base| !base.is_empty()).unwrap_or("archive").to_string();
        }
        if let Some(captures) = regex::Regex::new(r"(?i)^(.*)\.zx?\d{2,5}$").unwrap().captures(file_name) {
            return captures[1].to_string();
        }
        let lower = candidate.to_lowercase();
        if let Some((_, extension)) = lower.rsplit_once('.') {
            if extension.len() >= 3 && extension.chars().all(|character| character.is_ascii_digit()) {
                candidate.truncate(candidate.len() - extension.len() - 1);
            }
        }
        let lower = candidate.to_lowercase();
        if lower.ends_with(".rar") {
            if let Some(part_position) = lower.rfind(".part") {
                let volume = &lower[part_position + 5..lower.len() - 4];
                if !volume.is_empty() && volume.chars().all(|character| character.is_ascii_digit()) {
                    candidate.truncate(part_position);
                }
            }
        } else if let Some(part_position) = lower.rfind(".part") {
            let volume = &lower[part_position + 5..];
            if !volume.is_empty() && volume.chars().all(|character| character.is_ascii_digit()) {
                candidate.truncate(part_position);
            }
        }

        for suffix in [
            ".tar.lzma", ".lzma", ".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst", ".tar.lz4", ".tar.br", ".tar.lz", ".tlz", ".lz4", ".br", ".lz", ".zip", ".zipx", ".7z", ".rar",
            ".tar", ".wim", ".esd",
        ] {
            if candidate.to_lowercase().ends_with(suffix) {
                candidate.truncate(candidate.len() - suffix.len());
                break;
            }
        }
        if candidate.is_empty() { "archive".to_string() } else { candidate }
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
        if error.downcast_ref::<CompressionError>().is_some()
            || crate::services::archive_io::kind(&error).is_some()
            || format!("{error:#}").contains("[archive-inspection:") {
            return error;
        }
        let message = format!("{error:#}");
        if message.contains("密码错误") || message.contains("解密失败") {
            CompressionError::InvalidPassword.into()
        } else {
            CompressionError::ExtractionFailed(message).into()
        }
    }

    fn do_extract_aes(
        &self, window: &Window, task_id: &str, file: &str, output: &Path,
        password: Option<&str>, options: &DecompressOptions,
    ) -> Result<()> {
        self.emit_log(window, task_id, "正在认证 AES 完整容器及卷组，完成后读取内层内容", TaskLogSeverity::Info);
        let inner = crate::services::aes_container::AuthenticatedInner::open(
            Path::new(file), password, || self.check_cancellation()).map_err(Self::map_aes_extraction_error)?;
        let source = inner.path.to_string_lossy();
        let name = inner.path.file_name().unwrap_or_default().to_string_lossy().to_ascii_lowercase();
        let result = if name.ends_with(".tar") {
            self.do_extract_tar(window, task_id, &source, output, None, options)
        } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") || name.ends_with(".tpz") {
            self.do_extract_tar_gz(window, task_id, &source, output, options)
        } else if name.ends_with(".tar.bz2") || name.ends_with(".tbz") || name.ends_with(".tbz2") {
            self.do_extract_tar_bz2(window, task_id, &source, output, options)
        } else if name.ends_with(".tar.xz") || name.ends_with(".txz") {
            self.do_extract_tar_xz(window, task_id, &source, output, options)
        } else if name.ends_with(".tar.zst") || name.ends_with(".tzst") {
            self.do_extract_tar_zstd(window, task_id, &source, output, options)
        } else if [".lzma", ".lz4", ".br", ".lz", ".tlz"].iter().any(|suffix| name.ends_with(suffix)) {
            let codec = if name.ends_with(".lzma") { "lzma" } else if name.ends_with(".lz4") { "lz4" } else if name.ends_with(".br") { "br" } else { "lz" };
            let reader = crate::services::codec_stream::decoder(&inner.path, codec)?;
            if compression_format::is_tar_wrapped_archive(&inner.path, &format!("tar.{codec}")) {
                self.do_extract_tar(window, task_id, &source, output, Some(reader), options)
            } else {
                let name = native_extraction::single_stream::output_name(&inner.path, &[&format!(".{codec}")]);
                native_extraction::single_stream::extract(self, window, task_id, reader, output, name, options)
            }
        } else if name.ends_with(".gz") || name.ends_with(".gzip") {
            self.do_extract_gz(window, task_id, &source, output, options)
        } else if name.ends_with(".bz2") || name.ends_with(".bzip2") {
            self.do_extract_bz2(window, task_id, &source, output, options)
        } else if name.ends_with(".xz") {
            self.do_extract_xz(window, task_id, &source, output, options)
        } else if name.ends_with(".zst") || name.ends_with(".zstd") {
            self.do_extract_zstd(window, task_id, &source, output, options)
        } else {
            anyhow::bail!("[archive-inspection:unsupported] 不支持的 AES 内层格式：{name}")
        };
        result.map_err(Self::map_aes_extraction_error)
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

    /// 使用原生 LZMA-alone 编码器压缩
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
        self.do_compress_rar_inner(Some(window), task_id, sources, output, options)
    }

    fn do_compress_rar_inner(&self, window: Option<&Window>, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
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

        // -ep1 gives RAR the same source-relative layout as other encoders.
        // For colliding root names, use the existing entry-name allocator so
        // RAR cannot silently replace one source with another inside the archive.
        // RAR does not stream source-byte progress. Snapshot the actual input
        // entries and report their total only after the encoder has succeeded.
        // Walk incrementally so large trees do not need a second in-memory list.
        let mut input_bytes = 0u64;
        for source in sources {
            for entry in walkdir::WalkDir::new(source) {
                self.check_cancellation()?;
                let entry = entry?;
                if entry.file_type().is_file() {
                    input_bytes = input_bytes.checked_add(entry.metadata()?.len()).ok_or_else(|| anyhow::anyhow!("RAR 源文件体积溢出"))?;
                }
            }
        }
        let mut names = HashSet::new();
        let collisions = sources.iter().any(|source| {
            !names.insert(Path::new(source).file_name().unwrap_or_default().to_string_lossy().to_lowercase())
        });
        let prepared = if collisions {
            let directory = tempfile::Builder::new().prefix(".long-rar-input-").tempdir_in(Path::new(output).parent().unwrap_or(Path::new(".")))?;
            let layout = crate::services::compression_entries::collect(sources, options.preserve_paths.unwrap_or(true), true)?;
            let naming = DecompressOptions { conflict_policy: "rename".into(), ..Default::default() };
            for entry in layout {
                self.check_cancellation()?;
                let destination = directory.path().join(entry.archive_name);
                if entry.is_dir { std::fs::create_dir_all(&destination)?; continue; }
                if let Some(parent) = destination.parent() { std::fs::create_dir_all(parent)?; }
                let destination = extraction_transaction::resolve_extract_path(&destination, &naming)?;
                if std::fs::hard_link(&entry.path, &destination).is_err() { std::fs::copy(&entry.path, &destination)?; }
            }
            Some(directory)
        } else { None };
        let mut command = crate::utils::process::command(encoder);
        command.arg("a");
        command.arg("-cfg-");
        command.arg("-idq");
        command.arg("-y");
        command.arg(format!("-m{}", options.level.clamp(1, 5)));
        if options.create_solid_archive { command.arg("-s"); }
        if options.rar_recovery_percent > 0 {
            anyhow::ensure!(matches!(options.rar_recovery_percent, 3 | 5 | 10), "RAR 恢复保护请选择 3%、5% 或 10%");
            command.arg(format!("-rr{}p", options.rar_recovery_percent));
        }
        if options.rar_recovery_volumes > 0 {
            anyhow::ensure!(options.rar_recovery_volumes <= 10 && options.split_size.is_some_and(|size| size > 0), "恢复卷需要启用 RAR 分包，数量请选择 1 至 10 个");
            command.arg(format!("-rv{}", options.rar_recovery_volumes));
        }

        if options.preserve_paths == Some(false) {
            command.arg("-ep");
        } else {
            // Keep source-relative paths, never the temporary conversion root.
            command.arg("-ep1");
        }

        if let Some(password) = options.password.as_deref().filter(|password| !password.is_empty()) {
            if !options.allow_insecure_password_cli {
                return Err(CompressionError::CompressionFailed(
                    "Password-protected RAR creation requires explicit approval because RAR.exe exposes the password briefly in local process arguments. Use encrypted ZIP or 7Z to avoid this risk.".to_string()
                ).into());
            }
            command.arg(format!("-{}{}", if options.encrypt_filenames { "hp" } else { "p" }, password));
        }

        if let Some(size) = options.split_size.filter(|size| *size > 0) {
            command.arg(format!("-v{size}b"));
        }

        command.arg(output);
        if let Some(directory) = &prepared {
            command.current_dir(directory.path());
            for source in std::fs::read_dir(directory.path())? { command.arg(source?.file_name()); }
        } else {
            for source in sources {
                self.check_cancellation()?;
                command.arg(source);
            }
        }

        let output_result = self.run_command_cancellable(command)?;

        if !output_result.status.success() {
            let stderr = String::from_utf8_lossy(&output_result.stderr);
            let stdout = String::from_utf8_lossy(&output_result.stdout);
            let message = if stderr.trim().is_empty() { stdout.trim() } else { stderr.trim() };
            return Err(CompressionError::CompressionFailed(format!("RAR compression failed: {}", message)).into());
        }

        // RAR can emit a plain .rar when a requested volume size holds the
        // whole payload. Keep one actual part in the same naming contract as
        // larger sets so verification, publication and results find that file.
        if options.split_size.is_some_and(|size| size > 0)
            && Path::new(output).is_file()
            && crate::services::archive_volumes::volume_paths(Path::new(output))?.is_empty()
        {
            std::fs::rename(output, crate::services::archive_volumes::first_volume_path(Path::new(output)))?;
        }
        if options.rar_recovery_volumes > 0 {
            let recovery = crate::services::archive_volumes::recovery_volume_paths(Path::new(output))?;
            let data_count = crate::services::archive_volumes::volume_paths(Path::new(output))?.len();
            if data_count > 1 { anyhow::ensure!(recovery.len() == options.rar_recovery_volumes as usize, "恢复卷没有完整生成，请检查 WinRAR 版本与剩余空间"); }
            if let Some(window) = window { self.emit_log(window, task_id, &if recovery.is_empty() { "本次只生成一个数据包，没有附带恢复卷；恢复卷仅用于多个 RAR 分包".into() } else { format!("已生成 {} 个 .rev 恢复卷，请与 RAR 分包整组保留", recovery.len()) }, TaskLogSeverity::Info); }
        }
        if let Some(window) = window { self.emit_progress(window, task_id, 1.0, Some(output.to_string()), input_bytes, input_bytes); }
        Ok(())
    }

    fn do_compress_wim(&self, window: &Window, task_id: &str, sources: &[String], output: &str, options: CompressionOptions) -> Result<()> {
        if options.format.as_deref() == Some("swm") || output.to_ascii_lowercase().ends_with(".swm") {
            self.emit_log(window, task_id, &format!("创建原生 SWM · {}：先编码内容，再生成资源分包；单个资源过大时成员可能超过目标大小。", options.wim_method.label()), TaskLogSeverity::Info);
            let splitting = std::cell::Cell::new(false);
            let bytes = crate::services::wim_writer::create_swm(sources, Path::new(output), options.preserve_paths.unwrap_or(true), options.wim_method,
                options.split_size.context("原生 SWM 需要目标卷大小")?, self.cancellation_flag.clone(), |progress| {
                    if progress >= 0.75 && !splitting.replace(true) { self.emit_compression_stage(window, task_id, "Splitting", None); self.emit_log(window, task_id, "内容编码完成，正在生成原生 SWM 资源分包", TaskLogSeverity::Info); }
                    self.emit_progress(window, task_id, progress, None, 0, 0);
                })?;
            self.emit_progress(window, task_id, 0.99, None, bytes, bytes);
            return Ok(());
        }
        if ![".wim", ".esd"].iter().any(|suffix| output.to_ascii_lowercase().ends_with(suffix)) {
            return Err(CompressionError::CompressionFailed("WIM/ESD 输出路径需要以 .wim 或 .esd 结尾".to_string()).into());
        }
        if options.password.as_deref().is_some_and(|password| !password.is_empty()) {
            return Err(CompressionError::UnsupportedEncryption.into());
        }
        let method = if options.format.as_deref() == Some("esd") || output.to_ascii_lowercase().ends_with(".esd") { crate::services::wim_writer::WimMethod::Solid } else { options.wim_method };
        if method != crate::services::wim_writer::WimMethod::Store {
            self.emit_log(window, task_id, &format!("使用 {} 压缩创建 WIM", method.label()), TaskLogSeverity::Info);
            let input_bytes = crate::services::wim_writer::create(sources, Path::new(output), options.preserve_paths.unwrap_or(true), method, self.cancellation_flag.clone(), |progress| {
                self.emit_progress(window, task_id, progress, None, 0, 0);
            })?;
            self.emit_progress(window, task_id, 0.99, None, input_bytes, input_bytes);
            return Ok(());
        }
        if !crate::utils::archive_tools::archive_engine_can_create("wim") {
            return Err(CompressionError::CompressionFailed("The active archive engine cannot create WIM files.".to_string()).into());
        }
        if let Some(parent) = Path::new(output).parent() { std::fs::create_dir_all(parent)?; }
        let engine = find_7z_command().ok_or_else(|| CompressionError::CompressionFailed(missing_7z_message()))?;
        let mut command = crate::utils::process::command(engine);
        command.arg("a").arg("-twim").arg("-y").arg("-mx0").arg(output);
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

    async fn test_extraction_password(&self, file_path: &str, password: &str, options: &DecompressOptions) -> Result<bool> {
        if Self::detect_password_format(file_path)? == ArchiveFormat::Egg {
            return crate::services::egg_engine::try_password(Path::new(file_path), password, options, self.cancellation_flag.clone()).await;
        }
        if Self::detect_password_format(file_path)? == ArchiveFormat::Alz {
            return crate::services::alz_engine::try_password(Path::new(file_path), password, options, self.cancellation_flag.clone()).await;
        }
        self.check_cancellation()?;
        if Self::detect_password_format(file_path)? == ArchiveFormat::Zip {
            return UniversalCliEngine::try_zip_password_for_selection_cancellable(
                Path::new(file_path), password, options, || self.check_cancellation());
        }
        self.test_archive_password(file_path, password).await
    }

    async fn test_password_candidate(&self, file_path: &str, password: &str, options: &DecompressOptions) -> Result<PasswordCandidateOutcome> {
        match self.test_extraction_password(file_path, password, options).await {
            Ok(true) => Ok(PasswordCandidateOutcome::Matched),
            Ok(false) => Ok(PasswordCandidateOutcome::Mismatch),
            Err(error) if error.to_string().contains("[alz-password-encoding]") => Ok(PasswordCandidateOutcome::AlzUnrepresentable),
            Err(error) if error.to_string().contains("[egg-password-encoding]") => Ok(PasswordCandidateOutcome::EggUnrepresentable),
            Err(error) => match crate::services::zip_password::encoding_error(&error) {
                Some(cause) if cause.is_unrepresentable() => Ok(PasswordCandidateOutcome::Unrepresentable(cause.encoding)),
                _ => Err(error),
            },
        }
    }

    fn emit_unrepresentable_candidate(&self, window: &Window, task_id: &str, encoding: ZipPasswordEncoding) {
        self.emit_log(window, task_id, &format!(
            "候选密码无法按所选 ZIP 编码 {} 无损表示，已跳过；不会据此判定密码错误或文件损坏", encoding.as_str()),
            TaskLogSeverity::Info);
    }

    pub async fn test_archive_password(&self, file_path: &str, password: &str) -> Result<bool> {
        let source = SplitArchiveDetector::complete_source(Path::new(file_path))?;
        let file_path = source.to_str().ok_or_else(|| anyhow::anyhow!("无效归档路径"))?;
        self.check_cancellation()?;
        if Self::detect_password_format(file_path)? == ArchiveFormat::Egg {
            return crate::services::egg_engine::try_password(Path::new(file_path), password, &Default::default(), self.cancellation_flag.clone()).await;
        }
        if Self::detect_password_format(file_path)? == ArchiveFormat::Alz {
            return crate::services::alz_engine::try_password(Path::new(file_path), password, &Default::default(), self.cancellation_flag.clone()).await;
        }
        if Self::detect_password_format(file_path)? == ArchiveFormat::Zpaq {
            return crate::services::zpaq_engine::try_password(Path::new(file_path), password, self.cancellation_flag.clone()).await;
        }
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
            return UniversalCliEngine::try_zip_password_cancellable(path, password, || {
                self.check_cancellation()
            });
        }
        if format == ArchiveFormat::Rar {
            return self
                .rar_service
                .verify_rar_password(path, password)
                .await
                .map_err(anyhow::Error::from);
        }

        let cancellation_flag = self.cancellation_flag.clone();
        tokio::task::spawn_blocking(move || {
            match format {
                ArchiveFormat::SevenZip => {
                    let archive_reader = crate::services::numeric_volume_reader::ZipInputReader::open_seven_zip(Path::new(&file))?;
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
                            crate::services::task_control::wait_if_paused(&cancellation_flag);
                            if cancellation_flag.load(Ordering::SeqCst) {
                                return Err(sevenz_rust::Error::other("Password verification cancelled"));
                            }
                            let mut buffer = vec![0u8; Self::COPY_BUFFER_SIZE];
                            loop {
                                crate::services::task_control::wait_if_paused(&cancellation_flag);
                                if cancellation_flag.load(Ordering::SeqCst) {
                                    return Err(sevenz_rust::Error::other("Password verification cancelled"));
                                }
                                let read = reader.read(&mut buffer).map_err(sevenz_rust::Error::io)?;
                                if read == 0 {
                                    break;
                                }
                            }
                            tested_file = true;
                        }
                        Ok(true)
                    };
                    let result = sevenz_rust::decompress_with_extract_fn_and_password(
                        archive_reader,
                        &scratch,
                        sevenz_rust::Password::from(pwd.as_str()),
                        &mut verify_entry,
                    );
                    let _ = std::fs::remove_dir_all(&scratch);
                    if cancellation_flag.load(Ordering::SeqCst) {
                        return Err(CompressionError::Cancelled.into());
                    }
                    native_extraction::seven_zip::password_candidate_result(result.map(|_| ()), tested_file)
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
                        crate::services::task_control::wait_if_paused(&cancellation_flag);
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

    #[tokio::test]
    async fn standard_zip_main_volume_reads_complete_set_with_real_engine() {
        use std::io::Write;
        use crate::services::archive_engine::ArchiveEngine;
        let temp = tempfile::tempdir().unwrap();
        let original = temp.path().join("original.zip");
        let expected = (0..32_771).map(|index| ((index * 31 + index / 17) % 251) as u8).collect::<Vec<_>>();
        let mut writer = zip::ZipWriter::new(File::create(&original).unwrap());
        writer.start_file("中文目录/载荷.bin", zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored)).unwrap();
        writer.write_all(&expected).unwrap();
        writer.add_directory("中文目录/空目录/", zip::write::FileOptions::default()).unwrap();
        writer.finish().unwrap();
        let bytes = std::fs::read(&original).unwrap();
        let eocd = bytes.len() - 22;
        assert_eq!(&bytes[eocd..eocd + 4], b"PK\x05\x06");
        let central = u32::from_le_bytes(bytes[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
        let cut = 8192;
        let main = temp.path().join("标准分卷.zip");
        let first = temp.path().join("标准分卷.z01");
        let mut last = bytes[cut..].to_vec();
        let mut cursor = central - cut;
        while &last[cursor..cursor + 4] == b"PK\x01\x02" {
            let offset = u32::from_le_bytes(last[cursor + 42..cursor + 46].try_into().unwrap()) as usize;
            let disk = if offset < cut { 0u16 } else { 1u16 };
            let relative = if disk == 0 { offset + 4 } else { offset - cut };
            last[cursor + 34..cursor + 36].copy_from_slice(&disk.to_le_bytes());
            last[cursor + 42..cursor + 46].copy_from_slice(&(relative as u32).to_le_bytes());
            let variable = [28, 30, 32].iter().map(|index| u16::from_le_bytes(last[cursor + index..cursor + index + 2].try_into().unwrap()) as usize).sum::<usize>();
            cursor += 46 + variable;
        }
        let tail = eocd - cut;
        last[tail + 4..tail + 6].copy_from_slice(&1u16.to_le_bytes());
        last[tail + 6..tail + 8].copy_from_slice(&1u16.to_le_bytes());
        last[tail + 16..tail + 20].copy_from_slice(&((central - cut) as u32).to_le_bytes());
        let mut start = b"PK\x07\x08".to_vec(); start.extend_from_slice(&bytes[..cut]);
        std::fs::write(&first, start).unwrap(); std::fs::write(&main, last).unwrap();
        let group = SplitArchiveDetector::detect_split_archive(&main).unwrap().unwrap();
        assert_eq!(group.format, crate::services::split_archive_detector::SplitFormat::ZipSplit);
        assert!(group.is_complete); assert_eq!(group.first_part, main);
        let output = temp.path().join("还原");
        UniversalCliEngine::new().extract_with_progress(&main, &output, None, false, Arc::new(|_| {}), Arc::new(|_, _| {}), Arc::new(AtomicBool::new(false))).await.unwrap();
        assert_eq!(std::fs::read(output.join("中文目录/载荷.bin")).unwrap(), expected);
        assert!(output.join("中文目录/空目录").is_dir());
    }

    #[tokio::test]
    #[ignore = "Requires an external RAR encoder on PATH; run with the archive upgrade tools"]
    async fn rar_native_volumes_roundtrip_with_content_and_header_passwords() {
        use rand::{RngCore, SeedableRng};
        let encoder = CompressionService::find_rar_encoder().expect("RAR encoder required for this explicit real test");
        let temp = tempfile::tempdir().unwrap();
        let folder = temp.path().join("资料");
        std::fs::create_dir_all(folder.join("空目录")).unwrap();
        let source = folder.join("中文资料.bin");
        let companion = temp.path().join("说明.txt");
        std::fs::write(&companion, b"keep directory layout").unwrap();
        let other = temp.path().join("另一位置"); std::fs::create_dir(&other).unwrap();
        let same = other.join("说明.txt"); std::fs::write(&same, b"keep the other file too").unwrap();
        let sources = vec![folder.to_string_lossy().into_owned(), companion.to_string_lossy().into_owned(), same.to_string_lossy().into_owned()];
        let mut expected = vec![0u8; 128 * 1024 + 39];
        rand::rngs::StdRng::seed_from_u64(47).fill_bytes(&mut expected);
        std::fs::write(&source, &expected).unwrap();
        let service = CompressionService::new(CompressionServiceConfig::default(), Arc::new(IOBufferPool::default()),
            Arc::new(RarSupportService::new()), Arc::new(UniversalCliEngine::new()),
            Arc::new(EncryptedPasswordService::new(&temp.path().join("private-vault"))));
        for mode in 0..3 {
            eprintln!("RAR mode {mode}: create and verify");
            let password = if mode == 0 { None } else { Some("中文 密码+!🔐".to_string()) };
            let target = temp.path().join(format!("归档-{mode}.rar"));
            let staged = CompressionService::temporary_compression_output(&target).unwrap();
            let options = CompressionOptions { format: Some("rar".into()), password: password.clone(),
                split_size: Some(16 * 1024), preserve_paths: Some(true), encrypt_filenames: mode == 2, allow_insecure_password_cli: true,
                rar_recovery_volumes: 2,
                ..Default::default() };
            CompressionService::validate_compression_request(&sources, &target.to_string_lossy(), &options).unwrap();
            service.do_compress_rar_inner(None, "rar-real", &sources, &staged.to_string_lossy(), options).unwrap();
            let first = crate::services::archive_volumes::first_volume_path(&staged);
            service.verify_compression_output(CompressionRoute::Rar, &first, password.as_deref(), None, true).unwrap();
            CompressionService::finalize_compression_output(Ok(()), &staged, &target, true, false).unwrap();
            let parts = crate::services::archive_volumes::complete_volume_paths(&target).unwrap();
            assert!(parts.len() > 1);
            assert_eq!(crate::services::archive_volumes::recovery_volume_paths(&target).unwrap().len(), 2);
            let decoded = temp.path().join(format!("还原-{mode}"));
            let mut command = crate::utils::process::command(&encoder);
            command.arg("x").arg("-idq").arg("-y").arg(format!("-p{}", password.as_deref().unwrap_or("-")))
                .arg(&parts[0]).arg(format!("{}\\", decoded.display()));
            assert!(command.status().unwrap().success());
            let payload = walkdir::WalkDir::new(&decoded).into_iter().filter_map(Result::ok)
                .find(|entry| entry.file_type().is_file() && entry.file_name() == source.file_name().unwrap()).unwrap();
            assert_eq!(std::fs::read(payload.path()).unwrap(), expected);
            eprintln!("RAR mode {mode}: native extraction");
            let app_output = temp.path().join(format!("应用还原-{mode}"));
            RarSupportService::new().extract_rar(&parts[0], &app_output, password.as_deref(), &DecompressOptions { preserve_paths: true, ..Default::default() }, Arc::new(AtomicBool::new(false))).await.unwrap();
            let app_payload = walkdir::WalkDir::new(&app_output).into_iter().filter_map(Result::ok)
                .find(|entry| entry.file_type().is_file() && entry.file_name() == source.file_name().unwrap()).unwrap();
            assert_eq!(std::fs::read(app_payload.path()).unwrap(), expected);
            eprintln!("RAR mode {mode}: native metadata");
            let browse = crate::services::archive_browser::browse_archive(&parts[0], password.as_deref()).await.unwrap();
            assert_eq!(browse.total_files, 3);
            assert_eq!(std::fs::read(app_output.join("资料/中文资料.bin")).unwrap(), expected);
            assert!(app_output.join("资料/空目录").is_dir());
            assert_eq!(std::fs::read(app_output.join("说明.txt")).unwrap(), b"keep directory layout");
            assert_eq!(std::fs::read(app_output.join("说明 (1).txt")).unwrap(), b"keep the other file too");
            if mode > 0 {
                eprintln!("RAR mode {mode}: correct/wrong password checks");
                assert!(service.verify_compression_output(CompressionRoute::Rar, &parts[0], Some("wrong-password"), None, true).is_err());
                let rar = RarSupportService::new();
                assert!(rar.verify_rar_password(&parts[0], password.as_deref().unwrap()).await.unwrap());
                assert!(!rar.verify_rar_password(&parts[0], "wrong-password").await.unwrap());
                assert!(rar.extract_rar(&parts[0], &temp.path().join(format!("错误密码-{mode}")), Some("wrong-password"), &DecompressOptions::default(), Arc::new(AtomicBool::new(false))).await.is_err());
            }
        }
    }

    #[test]
    fn zpaq_1526_public_io_paths_accept_native_password_volumes() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source.txt");
        std::fs::write(&source, b"data").unwrap();
        let options = CompressionOptions { format: Some("zpaq".into()), password: Some("native password".into()), split_size: Some(37), ..Default::default() };
        CompressionService::validate_compression_io_paths(&[source.to_string_lossy().into_owned()], &root.path().join("output.zpaq").to_string_lossy(), &options).unwrap();
    }

    #[test]
    fn pending_conflict_owns_staging_until_cancelled() {
        let temp = tempfile::tempdir().expect("temp dir");
        let output = temp.path().join("output");
        let staging = ExtractionStaging::create_for(&output).expect("staging");
        let staging_path = staging.path().to_path_buf();
        std::fs::write(staging_path.join("payload.txt"), b"payload").expect("payload");
        let task_id = format!("pending-{}", uuid::Uuid::new_v4());
        PENDING_EXTRACTIONS.insert(task_id.clone(), PendingExtraction {
            source_times: HashMap::new(),
            staging,
            source_archive: temp.path().join("archive.zip"),
            final_output: output,
            options: DecompressOptions::default(),
            expected_expanded_bytes: Some(7),
            observed_conflicts: HashMap::new(),
        });

        assert!(staging_path.exists());
        assert!(CompressionService::discard_pending_extraction(&task_id));
        assert!(!staging_path.exists());
    }

    #[test]
    fn extraction_history_format_uses_detected_route_and_wrapped_suffixes() {
        assert_eq!(
            CompressionService::extraction_history_format(
                Path::new("misleading.zip"),
                &ArchiveFormat::SevenZip,
            ),
            "7z"
        );
        assert_eq!(
            CompressionService::extraction_history_format(
                Path::new("backup.tar.gz"),
                &ArchiveFormat::Gzip,
            ),
            "tar.gz"
        );
        assert_eq!(
            CompressionService::extraction_history_format(
                Path::new("bundle.rar.001"),
                &ArchiveFormat::Universal,
            ),
            "rar"
        );
    }

    #[test]
    fn password_candidate_position_never_changes_extraction_progress() {
        let payload = CompressionService::password_attempt_progress_payload(
            "task-1",
            2,
            Some(2),
            "保险箱「常用密码」· 密码「open-sesame」".to_string(),
        );

        assert_eq!(payload.stage.as_deref(), Some("password-attempt"));
        assert_eq!(payload.progress, 0.0);
        assert_eq!(payload.processed_bytes, 0);
        assert_eq!(payload.total_bytes, 0);
        assert_eq!(payload.password_attempt_current, Some(2));
        assert_eq!(payload.password_attempt_total, Some(2));
        assert_eq!(
            payload.current_password.as_deref(),
            Some("保险箱「常用密码」· 密码「open-sesame」")
        );
    }

    #[test]
    fn rar_decoded_progress_does_not_finish_before_transaction_commit() {
        assert_eq!(CompressionService::extraction_progress_from_output(0, 100), None);
        assert_eq!(CompressionService::extraction_progress_from_output(50, 100), Some(0.5));
        assert_eq!(CompressionService::extraction_progress_from_output(100, 100), Some(0.99));
        assert_eq!(CompressionService::extraction_progress_from_output(150, 100), Some(0.99));
    }

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

    #[tokio::test]
    async fn zip_signature_preflight_is_not_stolen_by_a_tar_suffix() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("实际ZIP.tar.gz");
        let mut writer = zip_aes::ZipWriter::new(std::fs::File::create(&file).unwrap());
        writer.start_file("实际资料.txt", zip_aes::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(b"real ZIP").unwrap();
        writer.finish().unwrap();
        let format = crate::services::archive_browser::detect_format(&file).unwrap();
        assert_eq!(format, ArchiveFormat::Zip);
        assert_eq!(CompressionService::for_testing().preflight_extraction(&file, &format, None, temp.path(), &DecompressOptions::default()).await.unwrap(), Some((1, 8)));
    }

    #[tokio::test]
    async fn tar_preflight_keeps_logical_size_payload_accounting_and_cancellation() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/tar-1513/control/control.tar");
        let temp = tempfile::tempdir().unwrap();
        let service = CompressionService::for_testing();
        service.cancel();
        let error = service.preflight_extraction(&source, &ArchiveFormat::Tar, None, temp.path(), &DecompressOptions::default()).await.unwrap_err();
        assert!(matches!(error.downcast_ref::<CompressionError>(), Some(CompressionError::Cancelled)));
        service.reset_cancellation();
        assert_eq!(service.preflight_extraction(&source, &ArchiveFormat::Tar, None, temp.path(), &DecompressOptions::default()).await.unwrap(), Some((1, 2147483665)));
        let selected = DecompressOptions { selected_entries: vec!["not-present.txt".into()], ..Default::default() };
        assert_eq!(service.preflight_extraction(&source, &ArchiveFormat::Tar, None, temp.path(), &selected).await.unwrap(), Some((0, 0)));
        assert!(!temp.path().join("control.bin").exists(), "preflight must not materialize holes");
    }

    #[tokio::test]
    async fn rar_preflight_service_cancellation_and_reset_preserve_real_stats() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("实际资料.rar");
        std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/archives/rar-preflight-1415/stored.rar"), &source).unwrap();
        let service = CompressionService::for_testing();
        service.cancel();
        let error = service.preflight_extraction(&source, &ArchiveFormat::Rar, None, temp.path(), &DecompressOptions::default()).await.unwrap_err();
        assert!(matches!(error.downcast_ref::<CompressionError>(), Some(CompressionError::Cancelled)));
        service.reset_cancellation();
        assert_eq!(service.preflight_extraction(&source, &ArchiveFormat::Rar, None, temp.path(), &DecompressOptions::default()).await.unwrap(), Some((1, 29)));
        assert!(!temp.path().join("helloworld.txt").exists(), "preflight must not extract any content");
    }

    #[test]
    #[ignore = "requires scripts/build-zpaq-engine.ps1 -TestIO"]
    fn zpaq_actual_write_fault_cleans_staging_and_preserves_existing_destination() {
        let tool = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/zpaq-io-test/long-zpaq.exe");
        assert!(tool.is_file(), "build the isolated fault executable first");
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        std::fs::write(&source, b"original source payload").unwrap();
        for fault in ["write", "shortwrite", "flush"] {
            let destination = temp.path().join(format!("{fault}.zpaq"));
            std::fs::write(&destination, b"pre-existing destination").unwrap();
            let staged = CompressionService::temporary_compression_output(&destination).unwrap();
            let output = crate::utils::process::command(&tool).args(["a"])
                .arg(&staged).arg(&source).args(["-method", "1", "-threads", "2"])
                .env("LONG_ZPAQ_TEST_IO", fault).output().unwrap();
            assert!(!output.status.success());
            let detail = String::from_utf8_lossy(&output.stderr);
            assert!(detail.contains("LONG_ZPAQ_IO:"), "{detail}");
            let failure = Err(anyhow::anyhow!("ZPAQ 文件读写失败：{detail}"));
            let result = CompressionService::finalize_compression_output(failure, &staged, &destination, false, false);
            assert!(result.unwrap_err().to_string().contains("文件读写失败"));
            assert!(!staged.exists());
            assert_eq!(std::fs::read(&destination).unwrap(), b"pre-existing destination");
            assert_eq!(std::fs::read(&source).unwrap(), b"original source payload");
        }
    }

    // The single-thread async test can still run its timer while the real RAR
    // preflight is paused. A synchronous pause gate here would block that timer.
    #[tokio::test]
    async fn rar_preflight_pause_does_not_block_async_runtime_and_can_cancel() {
        use crate::services::task_control;
        use std::time::Duration;
        struct ControlGuard(String);
        impl Drop for ControlGuard {
            fn drop(&mut self) { task_control::cleanup(&self.0); }
        }
        let task_id = format!("rar-preflight-{}", uuid::Uuid::new_v4());
        let mut service = CompressionService::for_testing();
        service.cancellation_flag = task_control::register(&task_id).unwrap();
        let _guard = ControlGuard(task_id.clone());
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/archives/rar-preflight-1415/stored.rar");
        let temp = tempfile::tempdir().unwrap();
        for cancel in [false, true] {
            assert!(task_control::pause(&task_id));
            let service = service.clone();
            let source = source.clone();
            let output = temp.path().to_path_buf();
            let mut worker = tokio::spawn(async move {
                service.preflight_extraction(&source, &ArchiveFormat::Rar, None, &output, &DecompressOptions::default()).await
            });
            assert!(tokio::time::timeout(Duration::from_millis(40), &mut worker).await.is_err(), "paused preflight must not complete");
            if cancel { assert!(task_control::cancel(&task_id)); }
            else { assert!(task_control::resume(&task_id)); }
            let result = tokio::time::timeout(Duration::from_secs(2), worker).await.unwrap().unwrap();
            if cancel {
                assert!(matches!(result.unwrap_err().downcast_ref::<CompressionError>(), Some(CompressionError::Cancelled)));
            } else {
                assert_eq!(result.unwrap(), Some((1, 29)));
            }
        }
    }

    #[tokio::test]
    async fn selected_zip_service_preflight_and_wordlist_use_the_same_range() {
        use zip::unstable::write::FileOptionsExt;
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("混合.zip");
        let mut writer = zip::ZipWriter::new(File::create(&source).unwrap());
        for (name, password, bytes) in [("说明.txt", None, b"public".as_slice()), ("甲方.bin", Some("alpha"), b"alpha".as_slice()), ("乙方.bin", Some("beta"), b"beta".as_slice())] {
            let options = zip::write::FileOptions::default();
            let options = password.map_or(options, |password| options.with_deprecated_encryption(password.as_bytes()));
            writer.start_file(name, options).unwrap(); writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
        let service = CompressionService::for_testing();
        let words = temp.path().join("words.txt");
        std::fs::write(&words, "beta\nalpha\n").unwrap();
        let options = DecompressOptions {
            selected_entries: vec!["甲方.bin".into()], enable_bruteforce: true,
            bruteforce_wordlists: vec![words.to_string_lossy().into_owned()], ..Default::default()
        };
        let stats = service.preflight_extraction(&source, &ArchiveFormat::Zip, Some("alpha"), temp.path(), &options).await.unwrap().unwrap();
        assert_eq!(stats, (1, 5));
        assert_eq!(service.resolve_archive_password_silent(source.to_str().unwrap(), &options).await.unwrap(), Some("alpha".into()));
        assert!(!service.test_archive_password(source.to_str().unwrap(), "alpha").await.unwrap(), "不能把所选文件的密码说成整个包的密码");
        let public = DecompressOptions { file_filter: Some("*.txt".into()), ..options.clone() };
        let public = DecompressOptions { selected_entries: vec![], ..public };
        assert_eq!(service.resolve_archive_password_silent(source.to_str().unwrap(), &public).await.unwrap(), None);
        assert_eq!(service.preflight_extraction(&source, &ArchiveFormat::Zip, None, temp.path(), &public).await.unwrap(), Some((1, 6)));
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn legacy_zip_cross_encoding_rebuild_keeps_source_and_target_independent() {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/legacy-zip-password-1416");
        let expected = std::fs::read(fixtures.join("payload.txt")).unwrap();
        let cases = [(ZipPasswordEncoding::Cp932, "日本語パスワード+!1416"),
            (ZipPasswordEncoding::Cp936, "简体密码测试+!1416"),
            (ZipPasswordEncoding::Cp950, "繁體密碼測試+!1416")];
        for (index, &(source_encoding, source_password)) in cases.iter().enumerate() {
            let (target_encoding, target_password) = cases[(index + 1) % cases.len()];
            let temp = tempfile::tempdir().unwrap();
            let source = fixtures.join(format!("{}.zip", source_encoding.as_str()));
            let before = std::fs::read(&source).unwrap();
            let service = CompressionService::for_testing();
            assert!(service.verify_archive_password_candidate_with_encoding(source.to_str().unwrap(), source_password, Some(source_encoding)).await.unwrap());
            assert!(!service.verify_archive_password_candidate_with_encoding(source.to_str().unwrap(), source_password, Some(ZipPasswordEncoding::Utf8)).await.unwrap());
            let (runtime, input, root) = (service.clone(), source.clone(), temp.path().to_path_buf());
            let output = tokio::task::spawn_blocking(move || {
                let extracted = root.join("source-contents");
                native_extraction::zip::extract_with_optional_window(&runtime, None, "1416-convert-read",
                    input.to_str().unwrap(), extracted.to_str().unwrap(), Some(source_password),
                    &DecompressOptions { zip_password_encoding: Some(source_encoding), preserve_paths: true, ..Default::default() }).unwrap();
                let output = root.join("new.zip");
                let staged = CompressionService::temporary_compression_output(&output).unwrap();
                native_compression::zip::compress(&runtime, None, "1416-convert-write",
                    &[extracted.join("payload.txt").to_string_lossy().into_owned()], staged.to_str().unwrap(),
                    CompressionOptions { level: 6, password: Some(target_password.into()),
                        zip_password_encoding: target_encoding, ..Default::default() }).unwrap();
                runtime.verify_compression_output(CompressionRoute::Zip, &staged, Some(target_password), Some(target_encoding), false).unwrap();
                CompressionService::finalize_compression_output(Ok(()), &staged, &output, false, false).unwrap();
                output
            }).await.unwrap();
            assert!(service.verify_archive_password_candidate_with_encoding(output.to_str().unwrap(), target_password, Some(target_encoding)).await.unwrap());
            assert!(!service.verify_archive_password_candidate_with_encoding(output.to_str().unwrap(), target_password, Some(ZipPasswordEncoding::Utf8)).await.unwrap());
            let mut archive = zip_aes::ZipArchive::new(File::open(&output).unwrap()).unwrap();
            let mut bytes = Vec::new();
            crate::services::zip_password::open_named_entry_with_encoding(&mut archive, "payload.txt", target_password, Some(target_encoding)).unwrap().read_to_end(&mut bytes).unwrap();
            assert_eq!(bytes, expected);
            if let Some(tools) = std::env::var_os("LONG_ZIP_1416_INTEROP_TOOLS") {
                let destination = temp.path().join("independent");
                let result = crate::utils::process::command(PathBuf::from(tools).join(target_encoding.as_str()).join("7z.exe"))
                    .arg("x").arg("-y").arg(format!("-p{target_password}")).arg(format!("-o{}", destination.display())).arg(&output).output().unwrap();
                assert!(result.status.success(), "independent cross-encoding extraction failed: {}", String::from_utf8_lossy(&result.stderr));
                assert_eq!(std::fs::read(destination.join("payload.txt")).unwrap(), expected);
            }
            assert_eq!(std::fs::read(&source).unwrap(), before);
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn fixed_zip_input_options_read_real_originals_and_numeric_volumes_without_fallback() {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/legacy-zip-password-1416");
        let expected = std::fs::read(fixtures.join("payload.txt")).unwrap();
        for (encoding, password) in [
            (ZipPasswordEncoding::Cp932, "日本語パスワード+!1416"),
            (ZipPasswordEncoding::Cp936, "简体密码测试+!1416"),
            (ZipPasswordEncoding::Cp950, "繁體密碼測試+!1416"),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let original = fixtures.join(format!("{}.zip", encoding.as_str()));
            let before = std::fs::read(&original).unwrap();
            let split = crate::services::split_compression::SplitCompressionService::split_archive_file(
                &original, &temp.path().join("numeric.zip"), 37, || Ok(())).unwrap();
            let service = CompressionService::for_testing();
            for (layout, input) in [("original", &original), ("numeric", &split.part_files[0])] {
                for selected in [false, true] {
                    let options = DecompressOptions { zip_password_encoding: Some(encoding),
                        selected_entries: if selected { vec!["payload.txt".into()] } else { vec![] },
                        parallel_zip: true, skip_corrupted: true, ..Default::default() };
                    let source = input.to_str().unwrap();
                    assert!(service.test_extraction_password(source, password, &options).await.unwrap());
                    let wrong_encoding = DecompressOptions { zip_password_encoding: Some(ZipPasswordEncoding::Utf8), ..options.clone() };
                    assert!(!service.test_extraction_password(source, password, &wrong_encoding).await.unwrap());
                    assert!(!service.test_extraction_password(source, "wrong", &options).await.unwrap());
                    let error = service.test_extraction_password(source, "🔑", &options).await.unwrap_err();
                    assert!(error.to_string().contains("[archive-inspection:password-encoding]"));
                    assert_eq!(crate::services::zip_password::encoding_error(&error).unwrap().encoding, encoding);
                    let rejected = temp.path().join(format!("{layout}-{selected}-rejected"));
                    let error = native_extraction::zip::extract_with_optional_window(&service, None, "1416-invalid-encoding",
                        source, rejected.to_str().unwrap(), Some("🔑"), &options).unwrap_err();
                    assert!(crate::services::zip_password::encoding_error(&error).is_some());
                    assert!(!rejected.exists(), "encoding errors cannot be skipped as corrupt entries");
                    let restored = temp.path().join(format!("{layout}-{selected}"));
                    let (runtime, input_path, target, settings) = (service.clone(), input.to_path_buf(), restored.clone(), options.clone());
                    tokio::task::spawn_blocking(move || native_extraction::zip::extract_with_optional_window(
                        &runtime, None, "1416-original", input_path.to_str().unwrap(), target.to_str().unwrap(), Some(password), &settings))
                        .await.unwrap().unwrap();
                    assert_eq!(std::fs::read(restored.join("payload.txt")).unwrap(), expected);
                    assert_eq!(std::fs::read_dir(&restored).unwrap().count(), 1);
                }
            }
            assert_eq!(std::fs::read(&original).unwrap(), before);
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn fixed_zip_candidate_encoding_skips_only_unrepresentable_candidates() {
        use crate::models::password::{PasswordCategory, PasswordEntry};
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/legacy-zip-password-1416");
        for (encoding, password) in [
            (ZipPasswordEncoding::Cp932, "日本語パスワード+!1416"),
            (ZipPasswordEncoding::Cp936, "简体密码测试+!1416"),
            (ZipPasswordEncoding::Cp950, "繁體密碼測試+!1416"),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let source = fixtures.join(format!("{}.zip", encoding.as_str()));
            let source = source.to_str().unwrap();
            let mut vault = EncryptedPasswordService::new(&temp.path().join("vault"));
            vault.initialize("test-key").await.unwrap();
            let mut first = PasswordEntry::new("unrepresentable".into(), "🔑".into(), PasswordCategory::Other);
            first.favorite = true;
            vault.add_password(first).await.unwrap();
            vault.add_password(PasswordEntry::new("correct".into(), password.into(), PasswordCategory::Other)).await.unwrap();
            let service = CompressionService::new(CompressionServiceConfig::default(), Arc::new(IOBufferPool::default()),
                Arc::new(RarSupportService::new()), Arc::new(UniversalCliEngine::new()), Arc::new(vault));
            let options = DecompressOptions { zip_password_encoding: Some(encoding), ..Default::default() };
            assert_eq!(service.test_password_candidate(source, "🔑", &options).await.unwrap(), PasswordCandidateOutcome::Unrepresentable(encoding));
            assert_eq!(service.test_password_candidate(source, "wrong", &options).await.unwrap(), PasswordCandidateOutcome::Mismatch);
            assert_eq!(service.test_password_candidate(source, password, &options).await.unwrap(), PasswordCandidateOutcome::Matched);
            assert_eq!(service.resolve_archive_password_silent(source, &options).await.unwrap().as_deref(), Some(password));
            let words = temp.path().join("words.txt");
            std::fs::write(&words, format!("🔑\nwrong\n{password}\nunused-last\n")).unwrap();
            let wordlists = vec![words.to_string_lossy().into_owned()];
            assert_eq!(service.attempt_bruteforce_wordlists_silent(source, &wordlists, &options).await.unwrap().as_deref(), Some(password));
            // A candidate-local encoding failure may be skipped; a source read,
            // content-integrity failure or cancellation must stop the search.
            let damaged = temp.path().join("damaged.zip");
            let mut bytes = std::fs::read(source).unwrap();
            let mut archive = zip_aes::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
            let entry = archive.by_index_raw(0).unwrap();
            let tail = (entry.data_start().unwrap() + entry.compressed_size() - 1) as usize;
            drop(entry); drop(archive);
            bytes[tail] ^= 1;
            std::fs::write(&damaged, bytes).unwrap();
            let error = service.attempt_bruteforce_wordlists_silent(damaged.to_str().unwrap(), &wordlists, &options).await.unwrap_err();
            assert!(error.to_string().contains("[archive-inspection:ambiguous-data]"));
            assert!(crate::services::zip_password::encoding_error(&error).is_none());
            let missing = temp.path().join("missing.zip");
            assert!(service.test_password_candidate(missing.to_str().unwrap(), "🔑", &options).await.is_err());
            service.cancellation_flag.store(true, Ordering::SeqCst);
            let error = service.test_password_candidate(source, "🔑", &options).await.unwrap_err();
            assert!(matches!(error.downcast_ref::<CompressionError>(), Some(CompressionError::Cancelled)));
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn fixed_zip_input_encoding_and_selection_are_task_local() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("mixed.zip");
        let mut writer = zip_aes::ZipWriter::new(File::create(&source).unwrap());
        let cases = [("日文/内容.txt", ZipPasswordEncoding::Cp932, "日本語パスワード+!1416"),
            ("繁体/内容.txt", ZipPasswordEncoding::Cp950, "繁體密碼測試+!1416")];
        for (name, encoding, password) in cases {
            let bytes = crate::services::zip_password::encode(password, encoding).unwrap();
            writer.start_file(name, zip_aes::write::SimpleFileOptions::default()
                .with_aes_encryption_bytes(zip_aes::AesMode::Aes256, &bytes)).unwrap();
            writer.write_all(name.as_bytes()).unwrap();
        }
        writer.start_file("public.txt", zip_aes::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(b"public").unwrap();
        writer.finish().unwrap();
        let service = CompressionService::for_testing();
        for (name, encoding, password) in cases {
            let options = DecompressOptions { zip_password_encoding: Some(encoding), selected_entries: vec![name.into()], ..Default::default() };
            assert!(service.test_extraction_password(source.to_str().unwrap(), password, &options).await.unwrap());
            let all = DecompressOptions { selected_entries: vec![], ..options.clone() };
            assert!(!service.test_extraction_password(source.to_str().unwrap(), password, &all).await.unwrap());
            let restored = temp.path().join(encoding.as_str());
            let (runtime, input, target) = (service.clone(), source.clone(), restored.clone());
            tokio::task::spawn_blocking(move || native_extraction::zip::extract_with_optional_window(
                &runtime, None, "1416-selected", input.to_str().unwrap(), target.to_str().unwrap(), Some(password), &options))
                .await.unwrap().unwrap();
            assert_eq!(std::fs::read(restored.join(name)).unwrap(), name.as_bytes());
            assert!(!restored.join("public.txt").exists());
            assert_eq!(std::fs::read_dir(&restored).unwrap().count(), 1);
        }
        let public = DecompressOptions { zip_password_encoding: Some(ZipPasswordEncoding::Cp932),
            selected_entries: vec!["public.txt".into()], ..Default::default() };
        assert!(!service.test_extraction_password(source.to_str().unwrap(), "🔑", &public).await.unwrap());
        let restored = temp.path().join("public-only");
        let target = restored.clone();
        tokio::task::spawn_blocking(move || native_extraction::zip::extract_with_optional_window(&service, None, "1416-public",
            source.to_str().unwrap(), target.to_str().unwrap(), Some("🔑"), &public)).await.unwrap().unwrap();
        assert_eq!(std::fs::read(restored.join("public.txt")).unwrap(), b"public");
        assert_eq!(std::fs::read_dir(restored).unwrap().count(), 1);
    }

    #[test]
    fn zip_input_encoding_serialization_keeps_automatic_old_requests() {
        let old = serde_json::to_value(DecompressOptions::default()).unwrap();
        assert!(old.get("zip_password_encoding").is_none());
        assert_eq!(serde_json::from_value::<DecompressOptions>(old.clone()).unwrap().zip_password_encoding, None);
        for mode in ["utf8", "windows_local", "cp932", "cp936", "cp950"] {
            let mut request = old.clone();
            request["zip_password_encoding"] = mode.into();
            let options: DecompressOptions = serde_json::from_value(request).unwrap();
            assert_eq!(options.zip_password_encoding.unwrap().as_str(), mode);
            assert_eq!(serde_json::to_value(options).unwrap()["zip_password_encoding"], mode);
        }
        let mut invalid = old;
        invalid["zip_password_encoding"] = "unknown".into();
        assert!(serde_json::from_value::<DecompressOptions>(invalid).is_err());
    }

    #[tokio::test]
    async fn automatic_password_candidates_preserve_corruption_and_cancellation() {
        use crate::models::password::{PasswordCategory, PasswordEntry};
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("archive.zip");
        let password = CompressionService::recommended_dictionary(&source.to_string_lossy())[0].clone();
        let mut writer = zip_aes::ZipWriter::new(File::create(&source).unwrap());
        let options = zip_aes::write::SimpleFileOptions::default()
            .compression_method(zip_aes::CompressionMethod::Stored)
            .with_aes_encryption(zip_aes::AesMode::Aes256, &password);
        writer.start_file("payload.txt", options).unwrap();
        writer.write_all(b"encrypted content").unwrap();
        writer.finish().unwrap();
        let mut vault = EncryptedPasswordService::new(&temp.path().join("vault"));
        vault.initialize("test-key").await.unwrap();
        vault.add_password(PasswordEntry::new("fixture".into(), password.clone(), PasswordCategory::Other)).await.unwrap();
        let service = CompressionService::new(CompressionServiceConfig::default(),
            Arc::new(IOBufferPool::default()), Arc::new(RarSupportService::new()),
            Arc::new(UniversalCliEngine::new()), Arc::new(vault));
        let words = temp.path().join("words.txt");
        std::fs::write(&words, format!("wrong-first\n{password}\nunused-last\n")).unwrap();
        let wordlists = vec![words.to_string_lossy().into_owned()];
        assert_eq!(service.attempt_bruteforce_wordlists_silent(&source.to_string_lossy(), &wordlists, &DecompressOptions::default()).await.unwrap(), Some(password.clone()));
        assert_eq!(service.resolve_archive_password_silent(&source.to_string_lossy(), &DecompressOptions::default()).await.unwrap(), Some(password));
        let tail = {
            let mut zip = zip_aes::ZipArchive::new(File::open(&source).unwrap()).unwrap();
            let entry = zip.by_index_raw(0).unwrap();
            (entry.data_start().unwrap() + entry.compressed_size() - 1) as usize
        };
        let mut bytes = std::fs::read(&source).unwrap();
        bytes[tail] ^= 1;
        std::fs::write(&source, bytes).unwrap();
        let split = crate::services::split_compression::SplitCompressionService::split_archive_file(
            &source, &temp.path().join("split.zip"), 37, || Ok(())).unwrap();
        for path in [&source, &split.part_files[0]] {
            let path = path.to_string_lossy();
            let errors = [
                service.resolve_archive_password_silent(&path, &DecompressOptions::default()).await.unwrap_err(),
                service.attempt_bruteforce_wordlists_silent(&path, &wordlists, &DecompressOptions::default()).await.unwrap_err(),
            ];
            for error in errors {
                assert!(error.to_string().contains("[archive-inspection:ambiguous-data]"));
                assert!(error.to_string().contains("payload.txt"));
            }
        }
        assert!(service.attempt_recommended_dictionary_silent(&source.to_string_lossy(), &DecompressOptions::default()).await.unwrap_err()
            .to_string().contains("[archive-inspection:ambiguous-data]"));
        service.cancellation_flag.store(true, Ordering::SeqCst);
        let error = service.resolve_archive_password_silent(&source.to_string_lossy(), &DecompressOptions::default()).await.unwrap_err();
        assert!(matches!(error.downcast_ref::<CompressionError>(), Some(CompressionError::Cancelled)));
    }

    #[tokio::test]
    async fn password_vault_usage_is_recorded_only_after_explicit_success_marking() {
        use crate::models::password::{PasswordCategory, PasswordEntry};

        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("payload.gz");
        let encrypted = temp.path().join("payload.gz.aes");
        std::fs::write(&input, b"real encrypted payload").expect("write fixture");
        AesWrapper::encrypt_file(&input, &encrypted, "vault-password").expect("encrypt fixture");

        let vault_dir = temp.path().join("vault");
        let mut vault = EncryptedPasswordService::new(&vault_dir);
        vault.initialize("installation-key").await.expect("initialize vault");
        let saved = vault
            .add_password(PasswordEntry::new(
                "archive password".to_string(),
                "vault-password".to_string(),
                PasswordCategory::Other,
            ))
            .await
            .expect("save password");

        let service = CompressionService::new(
            CompressionServiceConfig::default(),
            Arc::new(IOBufferPool::default()),
            Arc::new(RarSupportService::new()),
            Arc::new(UniversalCliEngine::new()),
            Arc::new(vault),
        );
        let options = DecompressOptions {
            enable_bruteforce: false,
            ..Default::default()
        };

        let resolved = service
            .resolve_archive_password_silent(&encrypted.to_string_lossy(), &options)
            .await.expect("resolve vault password");

        assert_eq!(resolved.as_deref(), Some("vault-password"));
        let matched_only = service
            .encrypted_password_service
            .get_password(&saved.id)
            .await
            .expect("read password")
            .expect("password remains in vault");
        assert_eq!(matched_only.use_count, 0);
        assert!(matched_only.last_used.is_none());

        service.mark_matching_vault_password_used("vault-password").await;
        let updated = service
            .encrypted_password_service
            .get_password(&saved.id)
            .await
            .expect("read updated password")
            .expect("password remains in vault");
        assert_eq!(updated.use_count, 1);
        assert!(updated.last_used.is_some());
        let local_today = chrono::Local::now().format("%Y-%m-%d").to_string();
        assert_eq!(updated.usage_history.get(&local_today), Some(&1));
    }

    #[test]
    fn io1516_missing_later_volume_retains_its_path_and_confirmed_stage() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("任务.zip.001");
        let second = temp.path().join("任务.zip.002");
        std::fs::write(&first, b"first volume").unwrap();
        std::fs::write(&second, b"second volume").unwrap();
        let before = CompressionService::source_file_snapshot(&first, "Pre-checking").unwrap();
        std::fs::remove_file(&second).unwrap();
        let error = CompressionService::source_file_changed(&first, &before, "Extracting").unwrap_err();
        assert!(error.to_string().contains("[archive-source:Extracting:source-missing]"), "{error:#}");
        assert!(error.to_string().contains("任务.zip.002"), "{error:#}");
    }

    #[test]
    fn io1516_other_volume_changes_are_not_hidden_by_an_unchanged_first_volume() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("任务.zip.001");
        let second = temp.path().join("任务.zip.002");
        std::fs::write(&first, b"first volume").unwrap();
        std::fs::write(&second, b"second volume").unwrap();
        let before = CompressionService::source_file_snapshot(&first, "Pre-checking").unwrap();
        std::fs::write(&second, b"replaced second volume with different bytes").unwrap();
        assert!(CompressionService::source_file_changed(&first, &before, "Extracting").unwrap(),
            "A changed second volume must invalidate the extraction before publication");
    }

    #[test]
    fn source_snapshot_detects_archives_that_change_during_extraction() {
        let temp = tempfile::tempdir().expect("temp dir");
        let archive = temp.path().join("growing.rar");
        std::fs::write(&archive, b"initial bytes").expect("write fixture");
        let snapshot = CompressionService::source_file_snapshot(&archive, "Pre-checking").expect("snapshot");

        assert!(!CompressionService::source_file_changed(&archive, &snapshot, "Extracting").expect("unchanged"));
        std::fs::OpenOptions::new()
            .append(true)
            .open(&archive)
            .expect("open fixture")
            .write_all(b"more bytes")
            .expect("grow fixture");
        assert!(CompressionService::source_file_changed(&archive, &snapshot, "Extracting").expect("changed"));
    }

    #[test]
    fn source_snapshot_preserves_missing_invalid_and_access_failures() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("removed.rar");
        std::fs::write(&archive, b"fixture").unwrap();
        let snapshot = CompressionService::source_file_snapshot(&archive, "Pre-checking").unwrap();
        std::fs::remove_file(&archive).unwrap();
        let missing = CompressionService::source_file_changed(&archive, &snapshot, "Extracting").unwrap_err();
        assert!(missing.to_string().contains("[archive-source:Extracting:source-missing]"));
        let invalid = CompressionService::source_file_snapshot(temp.path(), "Pre-checking").err().unwrap();
        assert!(invalid.to_string().contains("[archive-source:Pre-checking:source-invalid]"));
        for kind in [std::io::ErrorKind::PermissionDenied, std::io::ErrorKind::Other] {
            let error = CompressionService::source_access_error(std::io::Error::new(kind, "injected detail"), "Pre-checking");
            assert!(error.to_string().contains("[archive-source:Pre-checking:source-unavailable]"));
            assert!(error.to_string().contains("injected detail"));
        }
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
            false,
        )
        .expect_err("storage-full compression must fail");

        assert_eq!(crate::services::archive_io::kind(&error).unwrap().0, "disk-full");
        assert!(format!("{error:#}").contains("simulated compression storage full"));
        assert!(!working_output.exists());
        assert!(!sidecar.exists());
        assert!(!final_output.exists());
    }

    #[test]
    fn cancellation_before_publication_keeps_verified_output_private() {
        let temp = tempfile::tempdir().expect("temp dir");
        let final_output = temp.path().join("archive.zip");
        let working_output =
            CompressionService::temporary_compression_output(&final_output).expect("working path");
        std::fs::write(&working_output, b"verified archive bytes").expect("working output");

        let error = CompressionService::finalize_compression_output(
            Ok(()),
            &working_output,
            &final_output,
            false,
            true,
        )
        .expect_err("cancelled publication must fail");

        assert!(matches!(
            error.downcast_ref::<CompressionError>(),
            Some(CompressionError::Cancelled)
        ));
        assert!(!working_output.exists());
        assert!(!final_output.exists());
    }

    #[test]
    fn publication_failure_preserves_target_stage_and_io_cause() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("output.zip");
        let staged = CompressionService::temporary_compression_output(&destination).unwrap();
        std::fs::write(&staged, b"new output").unwrap();
        std::fs::write(&destination, b"existing output").unwrap();
        let conflict = CompressionService::finalize_compression_output(Ok(()), &staged, &destination, false, false).unwrap_err();
        assert!(conflict.to_string().contains("[archive-output:Publishing:output-conflict]"));
        assert_eq!(std::fs::read(&destination).unwrap(), b"existing output");
        assert!(!staged.exists());
        let missing = CompressionService::finalize_compression_output(Ok(()), &staged, &destination, false, false).unwrap_err();
        assert!(missing.to_string().contains("[archive-output:Publishing:publication-failed]"));
        for kind in [std::io::ErrorKind::StorageFull, std::io::ErrorKind::PermissionDenied] {
            let error = CompressionService::publication_error(PublishError::Io(std::io::Error::new(kind, "publish detail")));
            let error = CompressionService::normalize_storage_full_error(error);
            assert!(error.to_string().contains("[archive-output:Publishing:publication-failed]"));
            assert!(error.chain().any(|cause| cause.downcast_ref::<std::io::Error>().is_some_and(|io| io.kind() == kind)));
        }
    }

    #[test]
    fn io1516_terminal_io_releases_unpublished_staging_and_pending_commit() {
        for kind in [std::io::ErrorKind::StorageFull, std::io::ErrorKind::PermissionDenied] {
            let temp = tempfile::tempdir().unwrap();
            let output = temp.path().join("output");
            std::fs::create_dir(&output).unwrap();
            std::fs::write(output.join("old.txt"), b"original output").unwrap();
            let staging = ExtractionStaging::create_for(&output).unwrap();
            let stage = staging.path().to_path_buf();
            std::fs::write(stage.join("not-published.bin"), b"decoded new output").unwrap();
            let id = uuid::Uuid::new_v4().to_string();
            let pending = PendingExtraction { source_times: HashMap::new(), staging, source_archive: temp.path().join("source.zip"),
                final_output: output.clone(), options: DecompressOptions::default(), expected_expanded_bytes: None, observed_conflicts: HashMap::new() };
            let error = anyhow::Error::new(std::io::Error::new(kind, "actual commit route"));
            assert!(CompressionService::retain_pending_after_commit_error(&id, pending, &error).is_none());
            assert!(!PENDING_EXTRACTIONS.contains_key(&id));
            assert!(!stage.exists());
            assert_eq!(std::fs::read(output.join("old.txt")).unwrap(), b"original output");
        }
    }

    #[test]
    fn incomplete_rollback_keeps_files_but_removes_pending_retry() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("output");
        let staging = ExtractionStaging::create_for(&output).unwrap();
        let path = staging.path().to_path_buf();
        std::fs::write(path.join("backup"), b"original data").unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let pending = PendingExtraction { source_times: HashMap::new(), staging, source_archive: temp.path().join("source.zip"), final_output: output,
            options: DecompressOptions::default(), expected_expanded_bytes: None, observed_conflicts: HashMap::new() };
        let error = CompressionError::RollbackIncomplete(format!("恢复目录：{}", path.display())).into();
        let _ = CompressionService::retain_pending_after_commit_error(&id, pending, &error);
        assert!(!PENDING_EXTRACTIONS.contains_key(&id));
        assert!(!CompressionService::discard_pending_extraction(&id));
        assert_eq!(std::fs::read(path.join("backup")).unwrap(), b"original data");
        let ordinary_staging = ExtractionStaging::create_for(&temp.path().join("other-output")).unwrap();
        let ordinary_path = ordinary_staging.path().to_path_buf();
        let pending = PendingExtraction { source_times: HashMap::new(), staging: ordinary_staging, source_archive: temp.path().join("source.zip"),
            final_output: temp.path().join("other-output"), options: DecompressOptions::default(), expected_expanded_bytes: None, observed_conflicts: HashMap::new() };
        let _ = CompressionService::retain_pending_after_commit_error(&id, pending, &anyhow::anyhow!("ordinary commit failure"));
        assert!(PENDING_EXTRACTIONS.contains_key(&id));
        assert!(CompressionService::discard_pending_extraction(&id));
        assert!(!ordinary_path.exists());
    }

}

impl CompressionService {
    #[cfg(test)]
    fn staged_file_is_not_newer(source: &Path, destination: &Path) -> bool {
        extraction_transaction::staged_file_is_not_newer(source, destination)
    }

    #[cfg(test)]
    fn commit_staged_extraction(
        &self,
        window: Option<&Window>,
        task_id: &str,
        source_archive: &str,
        staging: &Path,
        output: &Path,
        options: &DecompressOptions,
    ) -> Result<u64> {
        self.commit_staged_extraction_recording(window, task_id, source_archive, staging, output, options, &mut HashMap::new())
    }
}

#[cfg(test)]
mod conflict_confirmation_tests {
    use super::*;

    #[test]
    fn conflict_confirmation_detects_modified_and_replaced_targets() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("same.txt");
        std::fs::write(&target, b"old").unwrap();
        let observed = HashMap::from([(target.clone(), ConflictTargetSnapshot::read(&target).unwrap())]);
        assert!(!conflict_targets_changed(&observed));
        std::fs::write(&target, b"a different file edited while the dialog is open").unwrap();
        assert!(conflict_targets_changed(&observed));
        std::fs::remove_file(&target).unwrap();
        assert!(!conflict_targets_changed(&observed), "deleted targets need no overwrite");
        std::fs::create_dir(&target).unwrap();
        assert!(conflict_targets_changed(&observed));
    }

    #[test]
    fn changed_conflict_can_be_reprompted_without_reextracting_or_publishing() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("output");
        let staging = temp.path().join("staging");
        std::fs::create_dir_all(&output).unwrap();
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::write(staging.join("same.txt"), b"archive contents").unwrap();
        std::fs::write(output.join("same.txt"), b"old").unwrap();
        let service = CompressionService::for_testing();
        let options = DecompressOptions { conflict_policy: "ask".into(), ..Default::default() };
        let mut observed = HashMap::new();
        assert!(service.commit_staged_extraction_recording(None, "test", "source.zip", &staging, &output, &options, &mut observed).is_err());
        std::fs::write(output.join("same.txt"), b"edited after prompt").unwrap();
        assert!(conflict_targets_changed(&observed));
        observed.clear();
        assert!(service.commit_staged_extraction_recording(None, "test", "source.zip", &staging, &output, &options, &mut observed).is_err());
        assert!(!conflict_targets_changed(&observed));
        assert_eq!(std::fs::read(output.join("same.txt")).unwrap(), b"edited after prompt");
        assert_eq!(std::fs::read(staging.join("same.txt")).unwrap(), b"archive contents");
        extraction_transaction::commit_staged_extraction_with_resolutions("source.zip", &staging, &output, &options,
            &HashMap::new(), Some("rename"), |_| panic!("confirmed")).unwrap();
        assert_eq!(std::fs::read(output.join("same (1).txt")).unwrap(), b"archive contents");
        assert_eq!(std::fs::read(output.join("same.txt")).unwrap(), b"edited after prompt");
    }

}

impl CompressionService {
    fn seven_zip_source_times(archive: &Path, password: Option<&str>, output: &Path, preserve_paths: bool) -> Result<HashMap<PathBuf, u64>> {
        let mut input = crate::services::numeric_volume_reader::ZipInputReader::open_seven_zip(archive)?;
        let length = input.seek(SeekFrom::End(0))?;
        input.seek(SeekFrom::Start(0))?;
        let password = sevenz_rust::Password::from(password.unwrap_or(""));
        let metadata = sevenz_rust::Archive::read(&mut input, length, password.as_slice())?;
        let mut times = HashMap::new();
        let mut seen = HashSet::new();
        for entry in metadata.files {
            if entry.is_directory() { continue; }
            let Some(relative) = Self::normalized_archive_path(Path::new(entry.name()), preserve_paths) else { continue };
            let target = output.join(relative);
            // Flattened duplicate names are ambiguous; never show another entry's date.
            if !seen.insert(target.clone()) { times.remove(&target); continue; }
            if entry.has_last_modified_date {
                let time: SystemTime = entry.last_modified_date().into();
                if let Ok(duration) = time.duration_since(std::time::UNIX_EPOCH) {
                    times.insert(target, duration.as_millis() as u64);
                }
            }
        }
        Ok(times)
    }

    #[cfg(test)]
    fn commit_staged_extraction_recording(
        &self, window: Option<&Window>, task_id: &str, source_archive: &str,
        staging: &Path, output: &Path, options: &DecompressOptions,
        observed: &mut HashMap<PathBuf, ConflictTargetSnapshot>,
    ) -> Result<u64> {
        self.commit_staged_extraction_recording_with_times(window, task_id, source_archive, staging, output, options, observed, &HashMap::new())
    }

    fn commit_staged_extraction_recording_with_times(
        &self, window: Option<&Window>, task_id: &str, source_archive: &str,
        staging: &Path, output: &Path, options: &DecompressOptions,
        observed: &mut HashMap<PathBuf, ConflictTargetSnapshot>, source_times: &HashMap<PathBuf, u64>,
    ) -> Result<u64> {
        extraction_transaction::commit_staged_extraction(
            source_archive,
            staging,
            output,
            options,
            |conflict| {
                let path = PathBuf::from(&conflict.dest_path);
                if let Some(snapshot) = ConflictTargetSnapshot::read(&path) {
                    observed.insert(path.clone(), snapshot);
                }
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
                            source_modified: source_times.get(&path).copied().unwrap_or(conflict.source_modified),
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

    #[test]
    fn failed_zip_creation_does_not_clean_preexisting_volume_set() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("existing.zip");
        let first = temp.path().join("existing.zip.001");
        std::fs::write(&first, b"user volume").unwrap();
        CompressionService::cleanup_unverified_compression_output(&output, &output, true, false);
        assert_eq!(std::fs::read(first).unwrap(), b"user volume");
    }

    #[test]
    fn recycle_bin_source_resolution_includes_every_split_volume() {
        let temp = tempfile::tempdir().expect("temp dir");
        let first = temp.path().join("bundle.zip.001");
        let second = temp.path().join("bundle.zip.002");
        std::fs::write(&first, b"part one").expect("first split fixture");
        std::fs::write(&second, b"part two").expect("second split fixture");

        let paths = CompressionService::source_archive_paths_for_recycle_bin(&first)
            .expect("resolve split source paths");

        assert_eq!(paths, vec![first, second]);
    }

    #[test]
    fn recycle_bin_rejects_every_partial_extraction_mode() {
        let partial_options = [
            DecompressOptions {
                delete_after: true,
                skip_corrupted: true,
                ..Default::default()
            },
            DecompressOptions {
                delete_after: true,
                extract_only_newer: true,
                ..Default::default()
            },
            DecompressOptions {
                delete_after: true,
                file_filter: Some("*.txt".to_string()),
                ..Default::default()
            },
            DecompressOptions {
                delete_after: true,
                selected_entries: vec!["one.txt".to_string()],
                ..Default::default()
            },
            DecompressOptions {
                delete_after: true,
                conflict_policy: "skip".to_string(),
                ..Default::default()
            },
        ];

        for options in partial_options {
            assert!(CompressionService::validate_recycle_bin_options(&options).is_err());
        }
        assert!(CompressionService::validate_recycle_bin_options(&DecompressOptions {
            delete_after: true,
            ..Default::default()
        })
        .is_ok());
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "mutates the Windows system Recycle Bin; run explicitly for release validation"]
    fn real_windows_recycle_bin_moves_every_split_volume() {
        let temp = tempfile::tempdir().expect("temp dir");
        let first = temp.path().join(format!("long-recycle-{}.zip.001", uuid::Uuid::new_v4()));
        let second = first.with_extension("002");
        std::fs::write(&first, b"part one").expect("first split fixture");
        std::fs::write(&second, b"part two").expect("second split fixture");

        let moved = CompressionService::move_source_archives_to_system_recycle_bin(&first)
            .expect("move split sources to the Windows system Recycle Bin");

        assert_eq!(moved, 2);
        assert!(!first.exists(), "first volume must leave its original path");
        assert!(!second.exists(), "second volume must leave its original path");
    }

    #[tokio::test]
    async fn progress_telemetry_reports_real_speed_and_eta() {
        let service = CompressionService::for_testing();
        service.progress_metrics.lock().expect("progress metrics").insert(
            "telemetry-task".to_string(),
            ProgressMetric {
                started_at: Instant::now() - std::time::Duration::from_secs(2),
                last_bytes: 0,
                paused_at_start: std::time::Duration::ZERO,
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
    async fn progress_telemetry_suppresses_unstable_short_task_throughput() {
        let service = CompressionService::for_testing();
        service.begin_progress_telemetry("short-telemetry-task");
        std::thread::sleep(std::time::Duration::from_millis(1));

        let (speed, eta) = service.progress_telemetry(
            "short-telemetry-task",
            1.0,
            4 * 1024 * 1024,
            4 * 1024 * 1024,
        );

        assert_eq!(speed, None);
        assert_eq!(eta, None);
    }

    #[tokio::test]
    async fn progress_telemetry_excludes_pause_time_and_preflight_baseline() {
        let id = format!("rate-paused-{}", uuid::Uuid::new_v4());
        let _flag = super::super::task_control::register(&id).unwrap();
        let service = CompressionService::for_testing();
        service.begin_progress_telemetry(&id);
        std::thread::sleep(std::time::Duration::from_millis(300));
        let before = service.progress_telemetry(&id, 0.5, 1024 * 1024, 2 * 1024 * 1024);
        assert!(super::super::task_control::pause(&id));
        std::thread::sleep(std::time::Duration::from_millis(350));
        assert!(super::super::task_control::resume(&id));
        let after = service.progress_telemetry(&id, 0.5, 1024 * 1024, 2 * 1024 * 1024);
        let rate = |s: Option<String>| s.unwrap().split(' ').next().unwrap().parse::<f64>().unwrap();
        assert!((rate(before.0) - rate(after.0)).abs() <= 0.1);
        // Earlier paused preflight must not subtract from a fresh engine clock.
        service.begin_progress_telemetry(&id);
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(service.progress_telemetry(&id, 0.5, 1024, 2048).0.is_some());
        super::super::task_control::cleanup(&id);
    }

    #[test]
    fn password_attempt_logs_are_sampled_without_hiding_boundaries() {
        let retained: Vec<_> = (1..=254)
            .filter(|current| CompressionService::should_log_password_attempt(*current, 254))
            .collect();
        assert_eq!(retained, vec![1, 2, 3, 25, 50, 75, 100, 125, 150, 175, 200, 225, 250, 254]);
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
        let final_output = temp.path().join("archive.zip");
        let output = CompressionService::temporary_compression_output(&final_output).unwrap();
        let existing = temp.path().join("archive.zip.001");
        std::fs::write(&existing, b"existing user archive").unwrap();
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
                None,
                true,
            )
            .is_err());
        CompressionService::cleanup_unverified_compression_output(&output, &final_output, true, true);

        assert!(source.exists(), "verification failure must preserve the source");
        assert_eq!(std::fs::read(existing).unwrap(), b"existing user archive");
        assert!(result.part_files.iter().all(|part| !part.exists()));
    }

    #[tokio::test]
    async fn verification_failure_never_publishes_over_an_existing_target() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("source.txt");
        let final_output = temp.path().join("archive.zip");
        let working = CompressionService::temporary_compression_output(&final_output)
            .expect("transaction working output");
        std::fs::write(&source, b"source must survive").expect("source fixture");
        std::fs::write(&working, b"not a zip archive").expect("corrupt working output");
        std::fs::write(&final_output, b"existing archive").expect("existing target");
        let service = CompressionService::new_with_defaults().await;

        let error = service.verify_compression_output(CompressionRoute::Zip, &working, None, None, false).unwrap_err();
        assert!(matches!(error.downcast_ref::<CompressionError>(), Some(CompressionError::VerificationFailed(_))));
        assert!(error.to_string().contains("[archive-output:Verifying:verification-failed]"));
        CompressionService::cleanup_unverified_compression_output(
            &working,
            &final_output,
            false,
            true,
        );

        assert_eq!(std::fs::read(&final_output).unwrap(), b"existing archive");
        assert_eq!(std::fs::read(&source).unwrap(), b"source must survive");
        assert!(!working.exists());
    }

    #[tokio::test]
    async fn verification_failure_preserves_io_detail_and_cancellation() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing.zip");
        let service = CompressionService::new_with_defaults().await;
        let error = service.verify_compression_output(CompressionRoute::Zip, &missing, None, None, false).unwrap_err();
        assert!(matches!(error.downcast_ref::<CompressionError>(), Some(CompressionError::VerificationFailed(_))));
        assert!(error.chain().any(|cause| cause.downcast_ref::<std::io::Error>().is_some()));
        let full = CompressionError::VerificationFailed(std::io::Error::from(std::io::ErrorKind::StorageFull).into());
        let normalized = CompressionService::finalize_compression_output(
            Err(full.into()), &missing, &temp.path().join("final.zip"), false, false,
        ).unwrap_err();
        assert!(matches!(normalized.downcast_ref::<CompressionError>(), Some(CompressionError::VerificationFailed(_))));
        assert!(normalized.chain().any(|cause| cause.downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == std::io::ErrorKind::StorageFull)));
        service.cancellation_flag.store(true, Ordering::SeqCst);
        let cancelled = service.verify_compression_output(CompressionRoute::Zip, &missing, None, None, false).unwrap_err();
        assert!(matches!(cancelled.downcast_ref::<CompressionError>(), Some(CompressionError::Cancelled)));
        assert!(!cancelled.to_string().contains("verification-failed"));
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

    #[test]
    fn progress_artifact_size_tracks_regular_split_and_extracted_outputs() {
        let temp = tempfile::tempdir().expect("temp dir");
        let service = CompressionService::for_testing();
        let archive = temp.path().join("sample.7z");
        std::fs::write(&archive, vec![0u8; 1_024]).expect("archive fixture");
        service.register_compression_progress_output("regular", archive.clone(), false);
        assert_eq!(service.compression_output_bytes("regular"), Some(1_024));
        service.register_compression_progress_output(
            "pending",
            temp.path().join("not-created-yet.zip"),
            false,
        );
        assert!(service.has_compression_progress_output("pending"));
        assert_eq!(service.compression_output_bytes("pending"), None);

        let split_base = temp.path().join("sample.zip");
        std::fs::write(temp.path().join("sample.zip.001"), vec![0u8; 700]).expect("split one");
        std::fs::write(temp.path().join("sample.zip.002"), vec![0u8; 300]).expect("split two");
        service.register_compression_progress_output("split", split_base, true);
        assert_eq!(service.compression_output_bytes("split"), Some(1_000));

        let extracted = temp.path().join("output");
        std::fs::create_dir_all(extracted.join("nested")).expect("output tree");
        std::fs::write(extracted.join("one.bin"), vec![0u8; 80]).expect("output one");
        std::fs::write(extracted.join("nested/two.bin"), vec![0u8; 20]).expect("output two");
        assert_eq!(CompressionService::directory_payload_size(&extracted), 100);
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
            Default::default(),
            Default::default(),
            6,
            &[source.to_string_lossy().to_string()],
            &archive.to_string_lossy(),
            true,
            false,
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
        for suffix in ["zipx", "jar", "bin"] {
            let renamed = temp.path().join(format!("renamed.{suffix}"));
            std::fs::copy(&archive, &renamed).unwrap();
            assert!(engine.try_password(&renamed, "open-sesame").await.unwrap());
            let destination = temp.path().join(format!("extract-{suffix}"));
            engine.extract_with_progress(&renamed, &destination, Some("open-sesame"), true,
                Arc::new(|_| {}), Arc::new(|_, _| {}), Arc::new(AtomicBool::new(false))).await.unwrap();
            assert_eq!(std::fs::read(destination.join("secret.txt")).unwrap(), b"top secret");
        }
    }

    #[test]
    fn extraction_resolves_middle_volumes_and_reports_missing_first_before_passwords() {
        let temp = tempfile::tempdir().unwrap();
        for (first, second) in [("new.part1.rar", "new.part2.rar"), ("old.rar", "old.r00"), ("numeric.zip.001", "numeric.zip.002")] {
            let first = temp.path().join(first); let second = temp.path().join(second);
            std::fs::write(&first, b"Rar!\x1a\x07\x00first").unwrap();
            std::fs::write(&second, b"Rar!\x1a\x07\x00second").unwrap();
            assert_eq!(CompressionService::resolve_extraction_source(&second).unwrap(), first.to_string_lossy());
            std::fs::remove_file(&first).unwrap();
            let error = CompressionService::resolve_extraction_source(&second).unwrap_err().to_string();
            assert!(error.contains("缺少分卷"), "{error}");
            assert_eq!(error, format!("[archive-inspection:incomplete] 缺少分卷：{}", first.file_name().unwrap().to_string_lossy()));
            assert_eq!(std::fs::read(&second).unwrap(), b"Rar!\x1a\x07\x00second");
        }
    }

    #[test]
    fn rejects_file_as_output_directory_with_actionable_reason() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        let blocker = temp.path().join("existing.txt");
        std::fs::write(&source, b"source").unwrap();
        std::fs::write(&blocker, b"existing").unwrap();
        for output in [blocker.join("archive.7z"), blocker.join("missing").join("archive.7z")] {
            let error = CompressionService::validate_compression_io_paths(
                &[source.to_string_lossy().into_owned()], &output.to_string_lossy(),
                &CompressionOptions::default(),
            ).unwrap_err().to_string();
            assert!(error.contains("是文件，不是文件夹"), "{error}");
            assert!(error.contains("请选择其他输出目录"), "{error}");
        }
        CompressionService::validate_compression_io_paths(
            &[source.to_string_lossy().into_owned()],
            &temp.path().join("new-directory").join("archive.7z").to_string_lossy(),
            &CompressionOptions::default(),
        ).unwrap();
        assert_eq!(std::fs::read(&source).unwrap(), b"source");
        assert_eq!(std::fs::read(&blocker).unwrap(), b"existing");
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
    fn forum_seven_zip_uses_a_clean_output_directory_name() {
        // Name/path contract only; this does NOT validate HaoZip forum bytes.
        for name in ["旅行.haozip01.7z", "旅行.HAOZIP02.7Z", "旅行.haozip003.7z"] {
            let source = Path::new("downloads").join(name);
            assert_eq!(CompressionService::archive_output_dir_name(&source), "旅行");
            assert_eq!(CompressionService::extraction_output_path(&source, Some("chosen"), true), Path::new("chosen").join("旅行"));
            assert_eq!(CompressionService::extraction_output_path(&source, None, true), Path::new("downloads").join("旅行"));
            assert_eq!(CompressionService::extraction_output_path(&source, Some("chosen"), false), PathBuf::from("chosen"));
            assert_eq!(CompressionService::extraction_output_path(&source, None, false), PathBuf::from("downloads"));
        }
        assert_eq!(CompressionService::archive_output_dir_name(Path::new("haozip01.7z")), "archive");
        assert_eq!(CompressionService::archive_output_dir_name(Path::new("旅行-not-haozip02.7z")), "旅行-not-haozip02");
        assert_eq!(CompressionService::archive_output_dir_name(Path::new("旅行.haozip1.7z")), "旅行.haozip1");
        assert_eq!(CompressionService::archive_output_dir_name(Path::new("旅行.haozip02.zip")), "旅行");
    }

    #[test]
    fn split_entries_use_a_clean_output_directory_name() {
        assert_eq!(CompressionService::archive_output_dir_name(Path::new("project.zip.001")), "project");
        assert_eq!(CompressionService::archive_output_dir_name(Path::new("project.7z.001")), "project");
        assert_eq!(CompressionService::archive_output_dir_name(Path::new("project.part01.rar")), "project");
        assert_eq!(CompressionService::archive_output_dir_name(Path::new("project.part1")), "project");
        assert_eq!(CompressionService::archive_output_dir_name(Path::new("project.tar.gz")), "project");
        for source in ["旅行.zip", "旅行.tar.gz", "旅行.7z.001", "旅行.part02.rar"] {
            assert_eq!(CompressionService::extraction_output_path(Path::new(source), Some("output"), true), Path::new("output").join("旅行"));
            assert_eq!(CompressionService::extraction_output_path(Path::new(source), Some("output"), false), PathBuf::from("output"));
        }
    }

    #[cfg(windows)]
    #[test]
    fn fixed_zip_encodings_verify_before_publication_for_existing_methods_and_volumes() {
        use crate::services::{zip_method::ZipMethod, zip_password::ZipEncryption, standard_zip_writer::ZipSplitMode};
        // Optional independent 7-Zip test copies use process-local codepage
        // manifests. Never change the system locale or the shipped executable.
        let independent = std::env::var_os("LONG_ZIP_1416_INTEROP_TOOLS").map(PathBuf::from);
        let mut seed = 1416u32;
        let payload: Vec<u8> = (0..140_000).map(|_| {
            seed ^= seed << 13; seed ^= seed >> 17; seed ^= seed << 5; seed as u8
        }).collect();
        for (encoding, password) in [
            (ZipPasswordEncoding::Cp932, "日本語パスワード+!1416"),
            (ZipPasswordEncoding::Cp936, "简体密码测试+!1416"),
            (ZipPasswordEncoding::Cp950, "繁體密碼測試+!1416"),
        ] {
            for encryption in [ZipEncryption::Aes256, ZipEncryption::Zipcrypto] {
                for (format, method, level) in [
                    ("zip", ZipMethod::Deflate, 0), ("zip", ZipMethod::Deflate, 6),
                    ("zip", ZipMethod::Zstd, 6), ("zipx", ZipMethod::Deflate, 6),
                ] {
                    for split in [None, Some(ZipSplitMode::Numeric), Some(ZipSplitMode::Standard)] {
                        let temp = tempfile::tempdir().unwrap();
                        let folder = temp.path().join("资料");
                        std::fs::create_dir_all(folder.join("空目录")).unwrap();
                        std::fs::write(folder.join("内容.bin"), &payload).unwrap();
                        std::fs::write(folder.join("空文件.txt"), []).unwrap();
                        let output = temp.path().join(format!("分享.{format}"));
                        let sources = vec![folder.to_string_lossy().into_owned()];
                        let options = CompressionOptions {
                            format: Some(format.into()), level, zip_method: method,
                            password: Some(password.into()), zip_password_encoding: encoding,
                            zip_encryption: encryption, zip_split_mode: split.unwrap_or_default(),
                            split_size: split.map(|_| 65_536), preserve_paths: Some(true),
                            ..Default::default()
                        };
                        assert!(options.verify_after);
                        let service = CompressionService::for_testing();
                        let staged = CompressionService::temporary_compression_output(&output).unwrap();
                        CompressionService::validate_compression_request(&sources, &output.to_string_lossy(), &options).unwrap();
                        native_compression::zip::compress(&service, None, "1416", &sources,
                            &staged.to_string_lossy(), options).unwrap();
                        let input = if split.is_some() { crate::services::archive_volumes::first_volume_path(&staged) }
                            else { staged.clone() };
                        assert!(!output.exists(), "not published before verification");
                        for (candidate, mode) in [(password, ZipPasswordEncoding::Utf8), ("wrong", encoding)] {
                            let error = service.verify_compression_output(CompressionRoute::Zip, &input,
                                Some(candidate), Some(mode), split.is_some()).unwrap_err();
                            assert!(matches!(error.downcast_ref::<CompressionError>(), Some(CompressionError::VerificationFailed(_))));
                        }
                        service.verify_compression_output(CompressionRoute::Zip, &input,
                            Some(password), Some(encoding), split.is_some()).unwrap();
                        if split.is_some() {
                            output_publish_transaction::publish_verified_volumes(&staged, &output, || false).unwrap();
                        } else {
                            CompressionService::finalize_compression_output(Ok(()), &staged, &output, false, false).unwrap();
                        }
                        let published = if split.is_some() { crate::services::archive_volumes::first_volume_path(&output) }
                            else { output.clone() };
                        service.verify_compression_output(CompressionRoute::Zip, &published,
                            Some(password), Some(encoding), split.is_some()).unwrap();
                        let reader = crate::services::numeric_volume_reader::ZipInputReader::open(&published).unwrap();
                        let mut archive = zip_aes::ZipArchive::new(reader).unwrap();
                        let mut restored = Vec::new();
                        let mut entry = crate::services::zip_password::open_named_entry_with_encoding(
                            &mut archive, "资料/内容.bin", password, Some(encoding)).unwrap();
                        entry.read_to_end(&mut restored).unwrap();
                        assert_eq!(restored, payload);
                        assert_eq!(entry.compression(), if format == "zipx" { zip_aes::CompressionMethod::Xz }
                            else if method == ZipMethod::Zstd { zip_aes::CompressionMethod::Zstd }
                            else if level == 0 { zip_aes::CompressionMethod::Stored }
                            else { zip_aes::CompressionMethod::Deflated });
                        drop(entry);
                        assert!(archive.by_name("资料/空目录/").unwrap().is_dir());
                        let empty_index = archive.index_for_name("资料/空文件.txt").unwrap();
                        assert_eq!(archive.by_index_raw(empty_index).unwrap().size(), 0);
                        let extracted = temp.path().join("native-restored");
                        native_extraction::zip::extract_with_optional_window(&service, None, "1416-read",
                            published.to_str().unwrap(), extracted.to_str().unwrap(), Some(password),
                            &DecompressOptions { zip_password_encoding: Some(encoding), preserve_paths: true,
                                parallel_zip: true, ..Default::default() }).unwrap();
                        assert_eq!(std::fs::read(extracted.join("资料/内容.bin")).unwrap(), payload);
                        assert_eq!(std::fs::read(extracted.join("资料/空文件.txt")).unwrap(), b"");
                        assert!(extracted.join("资料/空目录").is_dir());
                        if let Some(tools) = &independent {
                            let engine = tools.join(encoding.as_str()).join("7z.exe");
                            assert!(engine.is_file(), "missing independent verifier: {}", engine.display());
                            let destination = temp.path().join("independent");
                            let result = crate::utils::process::command(engine).arg("x").arg("-y")
                                .arg(format!("-p{password}")).arg(format!("-o{}", destination.display()))
                                .arg(&published).output().unwrap();
                            assert!(result.status.success(), "{encoding:?}/{encryption:?}/{format}/{method:?}/{level}/{split:?}: {} {}",
                                String::from_utf8_lossy(&result.stdout), String::from_utf8_lossy(&result.stderr));
                            assert_eq!(std::fs::read(destination.join("资料/内容.bin")).unwrap(), payload);
                            assert_eq!(std::fs::read(destination.join("资料/空文件.txt")).unwrap(), b"");
                            assert!(destination.join("资料/空目录").is_dir());
                        }
                        assert_eq!(std::fs::read(folder.join("内容.bin")).unwrap(), payload);
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn fixed_zip_encoding_verification_failures_preserve_sources_targets_and_typed_causes() {
        use crate::services::zip_password::ZipEncryption;
        for (encoding, password) in [
            (ZipPasswordEncoding::Cp932, "日本語パスワード+!1416"),
            (ZipPasswordEncoding::Cp936, "简体密码测试+!1416"),
            (ZipPasswordEncoding::Cp950, "繁體密碼測試+!1416"),
        ] {
            for split in [false, true] {
                let temp = tempfile::tempdir().unwrap();
                let source = temp.path().join("source.bin");
                let payload = vec![42u8; 8192];
                std::fs::write(&source, &payload).unwrap();
                let output = temp.path().join("existing.zip");
                let existing = if split { output.with_extension("zip.001") } else { output.clone() };
                std::fs::write(&existing, b"existing user archive").unwrap();
                let service = CompressionService::for_testing();
                let staged = CompressionService::temporary_compression_output(&output).unwrap();
                native_compression::zip::compress(&service, None, "1416-errors", &[source.to_string_lossy().into()],
                    &staged.to_string_lossy(), CompressionOptions {
                        password: Some(password.into()), zip_password_encoding: encoding,
                        zip_encryption: ZipEncryption::Aes256, level: 0,
                        split_size: split.then_some(1024), ..Default::default()
                    }).unwrap();
                let input = if split { crate::services::archive_volumes::first_volume_path(&staged) } else { staged.clone() };
                let encoding_error = service.verify_compression_output(CompressionRoute::Zip, &input,
                    Some("🔑"), Some(encoding), split).unwrap_err();
                assert!(matches!(encoding_error.downcast_ref::<CompressionError>(), Some(CompressionError::VerificationFailed(_))));
                assert!(encoding_error.chain().filter_map(|error| error.downcast_ref::<std::io::Error>())
                    .any(|error| error.get_ref().is_some_and(|error| error.is::<crate::services::zip_password::ZipPasswordEncodingError>())));
                service.cancellation_flag.store(true, Ordering::SeqCst);
                let cancelled = service.verify_compression_output(CompressionRoute::Zip, &input,
                    Some(password), Some(encoding), split).unwrap_err();
                assert!(matches!(cancelled.downcast_ref::<CompressionError>(), Some(CompressionError::Cancelled)));
                service.cancellation_flag.store(false, Ordering::SeqCst);
                // The ciphertext must still be authenticated after the short
                // password check; target encoding is not permission to skip CRC.
                let mut bytes = std::fs::read(&input).unwrap();
                bytes[512] ^= 1;
                std::fs::write(&input, bytes).unwrap();
                let result = service.verify_compression_output(CompressionRoute::Zip, &input,
                    Some(password), Some(encoding), split);
                assert!(matches!(result.as_ref().unwrap_err().downcast_ref::<CompressionError>(), Some(CompressionError::VerificationFailed(_))));
                CompressionService::cleanup_unverified_compression_output(&staged, &output, split, true);
                assert_eq!(std::fs::read(&existing).unwrap(), b"existing user archive");
                assert_eq!(std::fs::read(&source).unwrap(), payload);
                assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 2, "only original source and target remain");
            }
        }
    }

    #[tokio::test]
    async fn encrypted_zip_split_creation_publication_and_extraction_roundtrip() {
        use crate::services::zip_password::{encode, ZipPasswordEncoding};
        for encoding in [ZipPasswordEncoding::Utf8, ZipPasswordEncoding::WindowsLocal] {
            let password = "中文密码 + !测试";
            if let Err(error) = encode(password, encoding) {
                assert!(matches!(encoding, ZipPasswordEncoding::WindowsLocal), "{error}");
                eprintln!("Windows-local encoding unavailable on this host: {error}");
                continue;
            }
            let temp = tempfile::tempdir().unwrap();
            let folder = temp.path().join("资料");
            std::fs::create_dir_all(folder.join("空目录")).unwrap();
            std::fs::write(folder.join("内容.txt"), b"folder payload").unwrap();
            let second = temp.path().join("第二份.txt");
            std::fs::write(&second, b"second payload").unwrap();
            let sources = vec![folder.to_string_lossy().into_owned(), second.to_string_lossy().into_owned()];
            let output = temp.path().join("分享.zip");
            let options = CompressionOptions {
                format: Some("zip".into()), password: Some(password.into()),
                zip_password_encoding: encoding, split_size: Some(37),
                preserve_paths: Some(true), ..Default::default()
            };
            assert_eq!(CompressionService::validate_compression_request(&sources, &output.to_string_lossy(), &options).unwrap(), "zip");
            CompressionService::validate_compression_io_paths(&sources, &output.to_string_lossy(), &options).unwrap();
            let service = CompressionService::for_testing();
            let staged = CompressionService::temporary_compression_output(&output).unwrap();
            native_compression::zip::compress(&service, None, "encrypted-split", &sources, &staged.to_string_lossy(), options).unwrap();
            let staged_first = PathBuf::from(format!("{}.001", staged.display()));
            service.verify_compression_output_inner(CompressionRoute::Zip, &staged_first, Some(password), Some(encoding), true).unwrap();
            CompressionService::finalize_compression_output(Ok(()), &staged, &output, true, false).unwrap();
            assert!(!staged_first.exists());
            let first = PathBuf::from(format!("{}.001", output.display()));
            let volume_snapshot: Vec<_> = std::fs::read_dir(temp.path()).unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| path.file_name().unwrap().to_string_lossy().starts_with("分享.zip."))
                .map(|path| { let bytes = std::fs::read(&path).unwrap(); assert!(bytes.len() <= 37); (path, bytes) })
                .collect();
            assert!(volume_snapshot.len() > 1);
            let extracted = temp.path().join("解压结果");
            std::fs::create_dir(&extracted).unwrap();
            UniversalCliEngine::new().extract_with_progress(&first, &extracted, Some(password), false,
                Arc::new(|_| {}), Arc::new(|_, _| {}), Arc::new(AtomicBool::new(false))).await.unwrap();
            assert_eq!(std::fs::read(extracted.join("资料/内容.txt")).unwrap(), b"folder payload");
            assert_eq!(std::fs::read(extracted.join("第二份.txt")).unwrap(), b"second payload");
            assert!(extracted.join("资料/空目录").is_dir());
            assert_eq!(std::fs::read(folder.join("内容.txt")).unwrap(), b"folder payload");
            assert_eq!(std::fs::read(&second).unwrap(), b"second payload");
            assert!(first.exists());
            let wrong_output = temp.path().join("错误密码");
            std::fs::create_dir(&wrong_output).unwrap();
            assert!(UniversalCliEngine::new().extract_with_progress(&first, &wrong_output, Some("wrong"), false,
                Arc::new(|_| {}), Arc::new(|_, _| {}), Arc::new(AtomicBool::new(false))).await.is_err());
            assert_eq!(std::fs::read_dir(wrong_output).unwrap().count(), 0);
            for (path, bytes) in volume_snapshot { assert_eq!(std::fs::read(path).unwrap(), bytes); }
        }
    }

    #[test]
    fn numeric_seven_zip_routing_reads_magic_across_tiny_volumes() {
        let temp = tempfile::tempdir().unwrap();
        for (index, bytes) in [0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c].chunks(2).enumerate() {
            std::fs::write(temp.path().join(format!("tiny.7z.{:03}", index + 1)), bytes).unwrap();
        }
        let first = temp.path().join("tiny.7z.001");
        let format = CompressionService::detect_password_format(&first.to_string_lossy()).unwrap();
        assert!(CompressionService::uses_native_seven_zip_volumes(&first, &format));
        assert!(CompressionService::uses_native_seven_zip_volumes(Path::new("other.zip.001"), &format));
        assert!(!CompressionService::uses_native_seven_zip_volumes(Path::new("tiny.7z.002"), &format));
        assert!(!CompressionService::uses_native_seven_zip_volumes(&first, &ArchiveFormat::Zip));
    }

    #[tokio::test]
    async fn seven_zip_split_native_creation_verification_publication_and_extraction() {
        // Keep legacy contents-only combinations alongside explicit filename encryption.
        for (password, encrypt_filenames) in [(None, false), (Some("中文密码 + !测试"), false), (Some("中文密码 + !测试"), true)] {
            for solid in [false, true] {
                let temp = tempfile::tempdir().unwrap();
                let folder = temp.path().join("资料");
                std::fs::create_dir_all(folder.join("空目录")).unwrap();
                std::fs::write(folder.join("内容.txt"), b"folder payload").unwrap();
                std::fs::write(folder.join("空文件.txt"), b"").unwrap();
                filetime::set_file_mtime(folder.join("内容.txt"), filetime::FileTime::from_unix_time(1_600_000_000, 0)).unwrap();
                let second = temp.path().join("第二份.txt");
                std::fs::write(&second, b"second payload").unwrap();
                let sources = vec![folder.to_string_lossy().into_owned(), second.to_string_lossy().into_owned()];
                let output = temp.path().join("分享.7z");
                let service = CompressionService::for_testing();
                let staged = CompressionService::temporary_compression_output(&output).unwrap();
                native_compression::seven_zip::compress(&service, None, "seven-split", &sources,
                    &staged.to_string_lossy(), CompressionOptions {
                        format: Some("7z".into()), password: password.map(str::to_owned),
                        split_size: Some(37), create_solid_archive: solid, encrypt_filenames, preserve_paths: Some(true), ..Default::default()
                    }).unwrap();
                let staged_first = PathBuf::from(format!("{}.001", staged.display()));
                service.verify_compression_output(CompressionRoute::SevenZip, &staged_first, password, None, true).unwrap();
                if password.is_some() {
                    assert!(service.verify_compression_output(CompressionRoute::SevenZip, &staged_first, Some("wrong"), None, true).is_err());
                }
                CompressionService::finalize_compression_output(Ok(()), &staged, &output, true, false).unwrap();
                assert!(!staged_first.exists());
                assert!(!output.exists());
                let volumes: Vec<_> = std::fs::read_dir(temp.path()).unwrap().map(|entry| entry.unwrap().path())
                    .filter(|path| path.file_name().unwrap().to_string_lossy().starts_with("分享.7z."))
                    .map(|path| { let data = std::fs::read(&path).unwrap(); assert!(data.len() <= 37); (path, data) }).collect();
                assert!(volumes.len() > 1);
                let first = PathBuf::from(format!("{}.001", output.display()));
                let mut header_reader = crate::services::numeric_volume_reader::ZipInputReader::open_seven_zip(&first).unwrap();
                let header_length = std::io::Seek::seek(&mut header_reader, std::io::SeekFrom::End(0)).unwrap();
                std::io::Seek::seek(&mut header_reader, std::io::SeekFrom::Start(0)).unwrap();
                assert_eq!(sevenz_rust::Archive::read(&mut header_reader, header_length, &[]).is_ok(), !encrypt_filenames,
                    "Only explicitly encrypted filenames should require a header password");
                let format = CompressionService::detect_password_format(&first.to_string_lossy()).unwrap();
                assert!(CompressionService::uses_native_seven_zip_volumes(&first, &format));
                let stats = service.preflight_extraction(&first, &format, password, temp.path(), &DecompressOptions::default()).await.unwrap().unwrap();
                assert_eq!(stats, (4, 28));
                assert_eq!(native_extraction::seven_zip::requires_password(&first).unwrap(), password.is_some());
                if let Some(password) = password {
                    assert!(service.verify_archive_password_candidate(&first.to_string_lossy(), password).await.unwrap());
                    assert!(!service.verify_archive_password_candidate(&first.to_string_lossy(), "wrong").await.unwrap());
                }
                let native_output = temp.path().join("原生解压");
                let times = CompressionService::seven_zip_source_times(&first, password, &native_output, true).unwrap();
                assert_eq!(times.get(&native_output.join("资料/内容.txt")), Some(&1_600_000_000_000));
                let flat_times = CompressionService::seven_zip_source_times(&first, password, &native_output, false).unwrap();
                assert_eq!(flat_times.get(&native_output.join("内容.txt")), Some(&1_600_000_000_000));
                native_extraction::seven_zip::extract(&service, None, "native-split-read", &first.to_string_lossy(),
                    &native_output.to_string_lossy(), password, &DecompressOptions { preserve_paths: true, ..Default::default() }).unwrap();
                assert_eq!(std::fs::read(native_output.join("资料/内容.txt")).unwrap(), b"folder payload");
                assert_eq!(std::fs::read(native_output.join("资料/空文件.txt")).unwrap(), b"");
                assert_eq!(std::fs::read(native_output.join("第二份.txt")).unwrap(), b"second payload");
                assert!(native_output.join("资料/空目录").is_dir());
                let extracted = temp.path().join("解压结果");
                std::fs::create_dir(&extracted).unwrap();
                // Independent bundled 7-Zip. This proves format interoperability,
                // not the currently gated application's encrypted 7Z split reader.
                let mut command = crate::utils::process::command(find_7z_command().unwrap());
                command.args(["x", "-y"]).arg(format!("-p{}", password.unwrap_or("-")))
                    .arg(format!("-o{}", extracted.display())).arg(&first);
                let result = service.run_command_cancellable(command).unwrap();
                assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
                assert_eq!(std::fs::read(extracted.join("资料/内容.txt")).unwrap(), b"folder payload");
                assert_eq!(std::fs::read(extracted.join("资料/空文件.txt")).unwrap(), b"");
                assert_eq!(std::fs::read(extracted.join("第二份.txt")).unwrap(), b"second payload");
                assert!(extracted.join("资料/空目录").is_dir(), "password={password:?}, solid={solid}, entries={:?}", walkdir::WalkDir::new(&extracted).into_iter().map(|entry| entry.unwrap().path().to_owned()).collect::<Vec<_>>());
                assert_eq!(std::fs::read(folder.join("内容.txt")).unwrap(), b"folder payload");
                assert_eq!(std::fs::read(&second).unwrap(), b"second payload");
                for (path, data) in volumes { assert_eq!(std::fs::read(path).unwrap(), data); }
                std::fs::remove_file(temp.path().join("分享.7z.002")).unwrap();
                let missing = service.verify_archive_password_candidate(&first.to_string_lossy(), password.unwrap_or("unused"))
                    .await.unwrap_err().to_string();
                assert!(missing.contains("[archive-inspection:incomplete]"), "{missing}");
                let failed_output = temp.path().join("缺卷输出");
                assert!(native_extraction::seven_zip::extract(&service, None, "missing-volume", &first.to_string_lossy(),
                    &failed_output.to_string_lossy(), password, &DecompressOptions::default()).is_err());
                assert!(!failed_output.exists());
                assert!(std::fs::read_dir(temp.path()).unwrap().all(|entry| !entry.unwrap().file_name().to_string_lossy().starts_with(".split-source-")));
            }
        }
    }

    #[test]
    fn allows_supported_splits_but_rejects_unsupported_formats_and_nested_output() {
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
        ).is_ok());
        let seven_zip = CompressionOptions { format: Some("7z".into()), ..encrypted_split.clone() };
        assert!(CompressionService::validate_compression_io_paths(
            &[source.to_string_lossy().to_string()], &temp.path().join("archive.7z").to_string_lossy(), &seven_zip,
        ).is_ok());
        // RAR and TAR gained splitting in v1.4.3. Keep this rejection test on
        // formats that still genuinely lack the requested password/split mode.
        for format in ["wim"] {
            let unsupported = CompressionOptions { format: Some(format.into()), ..encrypted_split.clone() };
            assert!(CompressionService::validate_compression_io_paths(
                &[source.to_string_lossy().to_string()], &output.to_string_lossy(), &unsupported,
            ).is_err());
        }

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
        ).is_ok());
        assert!(CompressionService::validate_compression_io_paths(
            &[folder.to_string_lossy().to_string(), source.to_string_lossy().to_string()],
            &output.to_string_lossy(),
            &directory_split,
        ).is_ok());
        assert!(CompressionService::validate_compression_io_paths(
            &[folder.to_string_lossy().to_string()],
            &folder.join("nested.zip").to_string_lossy(),
            &directory_split,
        ).is_err());
    }
}
