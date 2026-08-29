use crate::services::compression_service::CompressionService;
use crate::services::compression_service::FileConflictResolution;
use crate::services::compression_service::RarCompressionSupport;
use crate::services::video_compression_plan::VideoCompressionPlanRequest;
use crate::services::video_encoding::{VideoEncodingEvent, VideoProgressSnapshot};
use crate::services::video_publish::PublishedVideoOutput;
use crate::models::compression::{
    CompressionOptions, DecompressOptions, TaskLog, TaskLogSeverity,
};
use serde::{Deserialize, Serialize};
use tauri::{command, AppHandle, Manager, State, Window};
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::path::{Component, Path, PathBuf};
use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use once_cell::sync::Lazy;

static CANCELLATION_FLAGS: Lazy<DashMap<String, Arc<AtomicBool>>> = Lazy::new(DashMap::new);
static ACTIVE_COMPRESSION_OUTPUTS: Lazy<DashMap<String, String>> = Lazy::new(DashMap::new);
static COMPRESSION_ANALYSIS_FLAGS: Lazy<DashMap<String, Arc<AtomicBool>>> = Lazy::new(DashMap::new);
static ARCHIVE_DIAGNOSTIC_FLAGS: Lazy<DashMap<String, Arc<AtomicBool>>> = Lazy::new(DashMap::new);
static ZIP_REPAIR_FLAGS: Lazy<DashMap<String, Arc<AtomicBool>>> = Lazy::new(DashMap::new);
static ARCHIVE_BROWSE_FLAGS: Lazy<DashMap<String, Arc<AtomicBool>>> = Lazy::new(DashMap::new);

fn register_task_cancellation(task_id: &str) -> Result<Arc<AtomicBool>, String> {
    let cancellation_flag = Arc::new(AtomicBool::new(false));
    match CANCELLATION_FLAGS.entry(task_id.to_string()) {
        Entry::Vacant(entry) => {
            entry.insert(cancellation_flag.clone());
            Ok(cancellation_flag)
        }
        Entry::Occupied(_) => Err(format!("Task is already running: {task_id}")),
    }
}

async fn service_for_task(task_id: &str) -> Result<CompressionService, String> {
    let cancellation_flag = register_task_cancellation(task_id)?;

    let mut service = CompressionService::new_with_defaults().await;
    service.cancellation_flag = cancellation_flag;
    Ok(service)
}

fn cleanup_task(task_id: &str) {
    CANCELLATION_FLAGS.remove(task_id);
}

struct TaskCancellationGuard {
    task_id: String,
}

impl TaskCancellationGuard {
    fn new(task_id: &str) -> Self {
        Self { task_id: task_id.to_string() }
    }
}

impl Drop for TaskCancellationGuard {
    fn drop(&mut self) {
        cleanup_task(&self.task_id);
    }
}

fn normalized_output_key(output_path: &str) -> Result<String, String> {
    let path = PathBuf::from(output_path);
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map_err(|error| format!("Unable to resolve compression output path: {error}"))?
            .join(path)
    };

    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }

    let key = normalized.to_string_lossy().replace('/', "\\");
    #[cfg(target_os = "windows")]
    let key = key.to_lowercase();
    Ok(key)
}

#[derive(Debug)]
struct CompressionOutputGuard {
    key: String,
}

impl CompressionOutputGuard {
    fn acquire(task_id: &str, output_path: &str) -> Result<Self, String> {
        let key = normalized_output_key(output_path)?;
        match ACTIVE_COMPRESSION_OUTPUTS.entry(key.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(task_id.to_string());
                Ok(Self { key })
            }
            Entry::Occupied(entry) => Err(format!(
                "Another compression task ({}) is already writing this output: {}",
                entry.get(),
                output_path
            )),
        }
    }
}

impl Drop for CompressionOutputGuard {
    fn drop(&mut self) {
        ACTIVE_COMPRESSION_OUTPUTS.remove(&self.key);
    }
}

#[command]
pub async fn extract_file(
    _app: AppHandle,
    window: Window,
    task_id: String,
    file_path: String, 
    output_path: Option<String>, 
    password: Option<String>, 
    options: Option<DecompressOptions>
) -> Result<String, String> {
    let service = service_for_task(&task_id).await?;
    let _task_guard = TaskCancellationGuard::new(&task_id);
    let opts = options.unwrap_or_default();
    
    let result = service.extract(window, task_id.clone(), file_path, output_path, password, opts)
        .await
        .map_err(|e| e.to_string());

    cleanup_task(&task_id);
    result
}

#[command]
pub async fn resolve_extraction_conflict(
    window: Window,
    task_id: String,
    resolutions: Vec<FileConflictResolution>,
    fallback_action: Option<String>,
) -> Result<String, String> {
    let service = service_for_task(&task_id).await?;
    let _task_guard = TaskCancellationGuard::new(&task_id);
    service
        .resolve_pending_extraction(&window, &task_id, resolutions, fallback_action)
        .await
        .map_err(|error| error.to_string())
}

#[command]
pub async fn verify_archive_password(
    task_id: String,
    file_path: String,
    password: String,
) -> Result<bool, String> {
    let service = service_for_task(&task_id).await?;
    let _task_guard = TaskCancellationGuard::new(&task_id);
    service
        .verify_archive_password_candidate(&file_path, &password)
        .await
        .map_err(|error| error.to_string())
}

#[command]
pub async fn extract_multiple(
    _app: AppHandle,
    window: Window,
    task_ids: Vec<String>,
    files: Vec<String>, 
    output_path: Option<String>, 
    password: Option<String>, 
    options: Option<DecompressOptions>
) -> Result<Vec<String>, String> {
    let mut results = Vec::new();
    
    for (i, file) in files.iter().enumerate() {
        let task_id = task_ids.get(i).cloned().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let opts = options.clone().unwrap_or_default();
        
        let service = service_for_task(&task_id).await?;
        let _task_guard = TaskCancellationGuard::new(&task_id);
        match service.extract(window.clone(), task_id.clone(), file.clone(), output_path.clone(), password.clone(), opts).await {
            Ok(path) => {
                cleanup_task(&task_id);
                results.push(path);
            },
            Err(e) => return Err(format!("解压文件 {} 失败: {}", file, e)),
        }
    }
    Ok(results)
}

#[command]
pub async fn compress_files(
    window: Window,
    task_id: String,
    files: Vec<String>, 
    output_path: String, 
    options: Option<CompressionOptions>
) -> Result<String, String> {
    let _output_guard = CompressionOutputGuard::acquire(&task_id, &output_path)?;
    let service = service_for_task(&task_id).await?;
    let _task_guard = TaskCancellationGuard::new(&task_id);
    let opts = options.unwrap_or_default();

    let result = match service.compress(window, task_id.clone(), files, output_path.clone(), opts).await {
        Ok(_) => Ok(format!("压缩成功: {}", output_path)),
        Err(e) => Err(format!("压缩失败: {}", e)),
    };

    cleanup_task(&task_id);
    result
}

#[command]
pub async fn compress_image_file(
    window: Window,
    task_id: String,
    request: crate::services::image_compression_service::ImageCompressionRequest,
) -> Result<crate::services::image_compression_service::ImageCompressionOutcome, String> {
    let log_task_id = task_id.clone();
    run_image_compression(task_id, request, move |stage| {
        let _ = window.emit("task-log", image_stage_log(&log_task_id, stage));
    })
    .await
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VideoCompressionExecutionRequest {
    pub plan: VideoCompressionPlanRequest,
    pub destination: PathBuf,
    #[serde(default)]
    pub confirmed_stream_changes: Vec<String>,
    pub preserve_mark_of_web: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VideoCompressionDestinationPlan {
    pub destination: PathBuf,
}

fn plan_video_destination(
    source: &Path,
    output_directory: Option<&Path>,
    reserved_destinations: &[PathBuf],
) -> Result<VideoCompressionDestinationPlan, String> {
    let metadata = std::fs::metadata(source)
        .map_err(|error| format!("VIDEO_DESTINATION_SOURCE_UNAVAILABLE: {error}"))?;
    if !metadata.is_file() {
        return Err("VIDEO_DESTINATION_SOURCE_NOT_FILE".to_string());
    }
    let directory = output_directory
        .map(Path::to_path_buf)
        .or_else(|| source.parent().map(Path::to_path_buf))
        .ok_or_else(|| "VIDEO_DESTINATION_DIRECTORY_UNAVAILABLE".to_string())?;
    if !directory.is_dir() {
        return Err(format!(
            "VIDEO_DESTINATION_DIRECTORY_NOT_FOUND: {}",
            directory.display()
        ));
    }
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "VIDEO_DESTINATION_SOURCE_NAME_INVALID".to_string())?;
    let reserved = reserved_destinations
        .iter()
        .map(|path| normalized_output_key(&path.to_string_lossy()))
        .collect::<Result<std::collections::HashSet<_>, _>>()?;

    for index in 0..10_000_u32 {
        let suffix = if index == 0 {
            String::new()
        } else {
            format!(" ({index})")
        };
        let destination = directory.join(format!("{stem}.compressed{suffix}.mp4"));
        let key = normalized_output_key(&destination.to_string_lossy())?;
        if !destination.exists() && !reserved.contains(&key) {
            return Ok(VideoCompressionDestinationPlan { destination });
        }
    }
    Err("VIDEO_DESTINATION_RENAME_LIMIT_REACHED".to_string())
}

#[command]
pub fn plan_video_compression_destination(
    source: String,
    output_directory: Option<String>,
    reserved_destinations: Vec<String>,
) -> Result<VideoCompressionDestinationPlan, String> {
    let reserved = reserved_destinations
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    plan_video_destination(
        Path::new(&source),
        output_directory.as_deref().map(Path::new),
        &reserved,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VideoCompressionStage {
    Probing,
    Encoding,
    Validating,
    Publishing,
}

impl VideoCompressionStage {
    fn event_name(self) -> &'static str {
        match self {
            Self::Probing => "Probing",
            Self::Encoding => "Encoding",
            Self::Validating => "Validating",
            Self::Publishing => "Publishing",
        }
    }

    fn log_message(self) -> &'static str {
        match self {
            Self::Probing => "正在重新探测视频并确认执行计划",
            Self::Encoding => "正在使用软件编码器生成隔离暂存输出",
            Self::Validating => "正在完整扫描并验证暂存视频",
            Self::Publishing => "验证通过，正在原子发布最终视频",
        }
    }
}

enum VideoCompressionCommandEvent {
    Stage(VideoCompressionStage),
    Encoding(VideoEncodingEvent),
}

#[derive(Debug, Clone, Serialize)]
struct VideoTaskProgress {
    task_id: String,
    stage: Option<String>,
    current_password: Option<String>,
    progress: f32,
    speed: Option<String>,
    eta_seconds: Option<u64>,
    current_file: Option<String>,
    processed_bytes: u64,
    total_bytes: u64,
    output_bytes: u64,
    output_bytes_estimated: bool,
    password_attempt_current: Option<usize>,
    password_attempt_total: Option<usize>,
    current_time_ms: Option<u64>,
    speed_multiple: Option<f64>,
    output_to_input_ratio: Option<f64>,
    heartbeat_seconds_since_progress: Option<u64>,
    heartbeat_at: Option<String>,
}

fn video_stage_log(task_id: &str, stage: VideoCompressionStage) -> TaskLog {
    TaskLog {
        task_id: task_id.to_string(),
        timestamp: chrono::Utc::now(),
        message: stage.log_message().to_string(),
        severity: TaskLogSeverity::Info,
    }
}

fn video_progress_payload(
    task_id: &str,
    source: &Path,
    stage: &str,
    snapshot: Option<&VideoProgressSnapshot>,
    heartbeat_seconds_since_progress: Option<u64>,
) -> VideoTaskProgress {
    let snapshot = snapshot.cloned().unwrap_or(VideoProgressSnapshot {
        current_time_ms: 0,
        progress_percent: 0.0,
        speed_multiple: None,
        output_bytes: None,
        output_bytes_provisional: true,
        eta_seconds: None,
        output_to_input_ratio: None,
        finished: false,
    });
    VideoTaskProgress {
        task_id: task_id.to_string(),
        stage: Some(stage.to_string()),
        current_password: None,
        // The shared task event contract carries a 0..1 fraction; the video
        // parser deliberately exposes a user-facing 0..100 percentage.
        progress: snapshot.progress_percent / 100.0,
        speed: snapshot.speed_multiple.map(|speed| format!("{speed:.2}x")),
        eta_seconds: snapshot.eta_seconds,
        current_file: Some(source.to_string_lossy().into_owned()),
        processed_bytes: 0,
        total_bytes: 0,
        output_bytes: snapshot.output_bytes.unwrap_or(0),
        output_bytes_estimated: snapshot.output_bytes_provisional,
        password_attempt_current: None,
        password_attempt_total: None,
        current_time_ms: Some(snapshot.current_time_ms),
        speed_multiple: snapshot.speed_multiple,
        output_to_input_ratio: snapshot.output_to_input_ratio,
        heartbeat_seconds_since_progress,
        heartbeat_at: heartbeat_seconds_since_progress.map(|_| chrono::Utc::now().to_rfc3339()),
    }
}

async fn await_video_step_or_cancellation<T, F>(
    future: F,
    cancelled: &AtomicBool,
) -> Result<T, String>
where
    F: Future<Output = Result<T, String>>,
{
    tokio::pin!(future);
    loop {
        tokio::select! {
            result = &mut future => return result,
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(25)) => {
                if cancelled.load(Ordering::SeqCst) {
                    return Err("VIDEO_COMPRESSION_CANCELLED".to_string());
                }
            }
        }
    }
}

async fn run_video_compression<F>(
    resource_root: PathBuf,
    task_id: String,
    request: VideoCompressionExecutionRequest,
    mut observe: F,
) -> Result<PublishedVideoOutput, String>
where
    F: FnMut(VideoCompressionCommandEvent),
{
    let output_path = request.destination.to_string_lossy().into_owned();
    let _output_guard = CompressionOutputGuard::acquire(&task_id, &output_path)?;
    let cancelled = register_task_cancellation(&task_id)?;
    let _task_guard = TaskCancellationGuard::new(&task_id);

    if cancelled.load(Ordering::SeqCst) {
        return Err("VIDEO_COMPRESSION_CANCELLED".to_string());
    }
    let validation_root = resource_root.clone();
    tauri::async_runtime::spawn_blocking(move || {
        crate::services::video_engine::validate_video_engine(&validation_root)
    })
    .await
    .map_err(|error| format!("VIDEO_ENGINE_PREFLIGHT_JOIN_FAILED: {error}"))?
    .map_err(|error| error.to_string())?;
    if cancelled.load(Ordering::SeqCst) {
        return Err("VIDEO_COMPRESSION_CANCELLED".to_string());
    }

    let runtime_root = resource_root.join("video-engine");
    let ffmpeg = runtime_root.join("ffmpeg.exe");
    let ffprobe = runtime_root.join("ffprobe.exe");
    let source = PathBuf::from(&request.plan.path);

    observe(VideoCompressionCommandEvent::Stage(VideoCompressionStage::Probing));
    let probe = await_video_step_or_cancellation(
        async {
            crate::services::video_probe::probe_video_file(&ffprobe, &source)
                .await
                .map_err(|error| error.to_string())
        },
        &cancelled,
    )
    .await?;
    let plan = crate::services::video_compression_plan::build_video_compression_plan(
        probe,
        &request.plan,
    )
    .map_err(|error| error.to_string())?;
    if !plan.can_encode {
        return Err(format!(
            "VIDEO_COMPRESSION_PLAN_BLOCKED: {}",
            plan.probe.blocking_reasons.join("; ")
        ));
    }
    if plan.requires_explicit_confirmation
        && request.confirmed_stream_changes != plan.stream_changes
    {
        return Err("VIDEO_COMPRESSION_STREAM_CHANGES_CONFIRMATION_REQUIRED".to_string());
    }

    observe(VideoCompressionCommandEvent::Stage(VideoCompressionStage::Encoding));
    let staged = crate::services::video_encoding::encode_video_to_staging(
        &ffmpeg,
        &plan,
        &request.destination,
        cancelled.clone(),
        |event| observe(VideoCompressionCommandEvent::Encoding(event)),
    )
    .await
    .map_err(|error| error.to_string())?;

    observe(VideoCompressionCommandEvent::Stage(VideoCompressionStage::Validating));
    let verified = await_video_step_or_cancellation(
        async {
            crate::services::video_output_validation::validate_staged_video_output(
                &ffprobe, &plan, &staged,
            )
            .await
            .map_err(|error| error.to_string())
        },
        &cancelled,
    )
    .await?;

    if cancelled.load(Ordering::SeqCst) {
        return Err("VIDEO_COMPRESSION_CANCELLED".to_string());
    }
    observe(VideoCompressionCommandEvent::Stage(VideoCompressionStage::Publishing));
    crate::services::video_publish::publish_validated_video_output(
        staged,
        verified,
        &source,
        &request.destination,
        request.preserve_mark_of_web,
        &cancelled,
    )
    .map_err(|error| error.to_string())
}

#[command]
pub async fn compress_video_file(
    app: AppHandle,
    window: Window,
    task_id: String,
    request: VideoCompressionExecutionRequest,
) -> Result<PublishedVideoOutput, String> {
    let app_resource_dir = app
        .path_resolver()
        .resource_dir()
        .ok_or_else(|| "VIDEO_ENGINE_RESOURCE_DIRECTORY_UNAVAILABLE".to_string())?;
    let resource_root =
        crate::services::video_engine::bundled_resource_root(&app_resource_dir);
    let event_task_id = task_id.clone();
    let source = PathBuf::from(&request.plan.path);
    let mut last_snapshot: Option<VideoProgressSnapshot> = None;
    run_video_compression(resource_root, task_id, request, move |event| match event {
        VideoCompressionCommandEvent::Stage(stage) => {
            let _ = window.emit("task-log", video_stage_log(&event_task_id, stage));
            let _ = window.emit(
                "task-progress",
                video_progress_payload(
                    &event_task_id,
                    &source,
                    stage.event_name(),
                    last_snapshot.as_ref(),
                    None,
                ),
            );
        }
        VideoCompressionCommandEvent::Encoding(VideoEncodingEvent::Progress { snapshot }) => {
            let payload = video_progress_payload(
                &event_task_id,
                &source,
                VideoCompressionStage::Encoding.event_name(),
                Some(&snapshot),
                None,
            );
            last_snapshot = Some(snapshot);
            let _ = window.emit("task-progress", payload);
        }
        VideoCompressionCommandEvent::Encoding(VideoEncodingEvent::Heartbeat {
            seconds_since_progress,
        }) => {
            let _ = window.emit(
                "task-progress",
                video_progress_payload(
                    &event_task_id,
                    &source,
                    "still-encoding",
                    last_snapshot.as_ref(),
                    Some(seconds_since_progress),
                ),
            );
        }
    })
    .await
}

#[command]
pub fn plan_image_compression_destination(
    source: String,
    output_directory: Option<String>,
    target_format: crate::services::image_compression_service::ImageFileFormat,
    conflict_policy: crate::services::image_compression_service::ImageConflictPolicy,
    reserved_destinations: Vec<String>,
) -> Result<crate::services::image_compression_service::ImageDestinationPlan, String> {
    let output_directory = output_directory.as_deref().map(std::path::Path::new);
    let reserved_destinations = reserved_destinations
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    crate::services::image_compression_service::plan_image_destination(
        std::path::Path::new(&source),
        output_directory,
        target_format,
        conflict_policy,
        &reserved_destinations,
    )
    .map_err(|error| error.to_string())
}

fn image_stage_log(
    task_id: &str,
    stage: crate::services::image_compression_service::ImageCompressionStage,
) -> TaskLog {
    TaskLog {
        task_id: task_id.to_string(),
        timestamp: chrono::Utc::now(),
        message: stage.log_message().to_string(),
        severity: TaskLogSeverity::Info,
    }
}

async fn run_image_compression<F>(
    task_id: String,
    request: crate::services::image_compression_service::ImageCompressionRequest,
    observe_stage: F,
) -> Result<crate::services::image_compression_service::ImageCompressionOutcome, String>
where
    F: FnMut(crate::services::image_compression_service::ImageCompressionStage) + Send + 'static,
{
    let output_path = request.destination.to_string_lossy().into_owned();
    let source_path = request.source.to_string_lossy().into_owned();
    let _output_guard = CompressionOutputGuard::acquire(&task_id, &output_path)?;
    let cancelled = register_task_cancellation(&task_id)?;
    let _task_guard = TaskCancellationGuard::new(&task_id);

    let input_bytes = std::fs::metadata(&request.source)
        .map_err(|error| format!("Unable to inspect image input: {error}"))?
        .len();
    let preflight = crate::services::storage_preflight::preflight_operation_resources(
        "compression",
        &output_path,
        &[source_path],
        None,
        Some(input_bytes),
        false,
    )
    .await
    .map_err(|error| error.to_string())?;
    if !preflight.can_start {
        return Err(preflight.summary);
    }

    tauri::async_runtime::spawn_blocking(move || {
        crate::services::image_compression_service::compress_single_image_with_observer(
            &request,
            &cancelled,
            observe_stage,
        )
        .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[command]
pub async fn cancel_compression(task_id: String) -> Result<(), String> {
    let Some(flag) = CANCELLATION_FLAGS.get(&task_id) else {
        if CompressionService::discard_pending_extraction(&task_id) {
            return Ok(());
        }
        return Err(format!("Task is not active: {task_id}"));
    };
    flag.store(true, Ordering::SeqCst);
    drop(flag);

    for _ in 0..200 {
        if !CANCELLATION_FLAGS.contains_key(&task_id) {
            return Ok(());
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    Err(format!("Timed out waiting for task cancellation: {task_id}"))
}

/// Runs a deterministic, cancellable file-writing task for the real desktop E2E suite.
///
/// The executable task body is compiled only for desktop E2E builds; production
/// builds reject the command. The test exercises the same cancellation registry
/// used by compression and decompression commands while remaining deterministic.
#[command]
pub async fn desktop_e2e_run_cancellable_task(
    task_id: String,
    output_path: String,
) -> Result<(), String> {
    #[cfg(not(feature = "desktop-e2e"))]
    {
        let _ = (task_id, output_path);
        Err("desktop E2E support is not enabled".to_string())
    }

    #[cfg(feature = "desktop-e2e")]
    {
    use std::io::Write;
    use std::path::PathBuf;

    let cancellation_flag = Arc::new(AtomicBool::new(false));
    CANCELLATION_FLAGS.insert(task_id.clone(), cancellation_flag.clone());
    let _task_guard = TaskCancellationGuard::new(&task_id);
    let output = PathBuf::from(output_path);
    let mut file = std::fs::File::create(&output).map_err(|error| error.to_string())?;
    let chunk = vec![0x5a; 256 * 1024];

    for _ in 0..6_000 {
        if cancellation_flag.load(Ordering::SeqCst) {
            drop(file);
            let _ = std::fs::remove_file(&output);
            return Err("desktop E2E task cancelled".to_string());
        }
        file.write_all(&chunk).map_err(|error| error.to_string())?;
        file.flush().map_err(|error| error.to_string())?;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

        Ok(())
    }
}

#[command]
pub async fn cancel_tasks_and_wait(task_ids: Vec<String>) -> Result<(), String> {
    for task_id in &task_ids {
        if let Some(flag) = CANCELLATION_FLAGS.get(task_id) {
            flag.store(true, Ordering::SeqCst);
        }
    }

    for _ in 0..200 {
        if task_ids.iter().all(|task_id| !CANCELLATION_FLAGS.contains_key(task_id)) {
            return Ok(());
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    Err("等待任务安全停止超时，应用未退出".to_string())
}

#[command]
pub async fn check_rar_compression_support() -> Result<RarCompressionSupport, String> {
    Ok(CompressionService::check_rar_compression_support())
}

#[command]
pub async fn get_archive_engine_capabilities() -> Result<crate::utils::archive_tools::ArchiveEngineCapabilities, String> {
    Ok(crate::utils::archive_tools::detect_archive_engine_capabilities())
}

#[command]
pub async fn install_winrar_with_winget() -> Result<RarCompressionSupport, String> {
    let output = crate::utils::process::async_command("winget")
        .args([
            "install", "--id", "RARLab.WinRAR", "--exact", "--source", "winget",
            "--accept-source-agreements", "--accept-package-agreements", "--silent",
        ])
        .output()
        .await
        .map_err(|err| format!("Unable to start winget: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if stderr.trim().is_empty() { stdout.trim() } else { stderr.trim() };
        return Err(format!("WinRAR installation failed: {detail}"));
    }
    let support = CompressionService::check_rar_compression_support();
    if support.available { Ok(support) } else { Err("WinRAR installation finished, but Rar.exe was not detected. Restart the application and retry.".to_string()) }
}

#[command]
pub async fn open_rar_download_page(app: AppHandle) -> Result<(), String> {
    tauri::api::shell::open(
        &app.shell_scope(),
        "https://www.rarlab.com/download.htm",
        None,
    )
    .map_err(|err| err.to_string())
}

/// 列出归档文件内容条目（通过 7z CLI）
#[command]
pub async fn list_archive_contents(file_path: String, password: Option<String>) -> Result<Vec<String>, String> {
    use crate::services::universal_engine::UniversalCliEngine;

    let path = std::path::Path::new(&file_path);
    UniversalCliEngine::list_contents(path, password.as_deref())
        .await
        .map_err(|e| e.to_string())
}

struct CompressionAnalysisGuard {
    analysis_id: String,
}

struct ArchiveDiagnosticGuard {
    diagnostic_id: String,
}

impl Drop for ArchiveDiagnosticGuard {
    fn drop(&mut self) {
        ARCHIVE_DIAGNOSTIC_FLAGS.remove(&self.diagnostic_id);
    }
}

struct ZipRepairGuard {
    repair_id: String,
}

impl Drop for ZipRepairGuard {
    fn drop(&mut self) {
        ZIP_REPAIR_FLAGS.remove(&self.repair_id);
    }
}

impl Drop for CompressionAnalysisGuard {
    fn drop(&mut self) {
        COMPRESSION_ANALYSIS_FLAGS.remove(&self.analysis_id);
    }
}

#[command]
pub async fn analyze_compression_sources(
    analysis_id: String,
    paths: Vec<String>,
    format: String,
    level: u32,
) -> Result<crate::services::compression_analysis::CompressionAnalysisResult, String> {
    if paths.is_empty() {
        return Err("Compression analysis requires at least one source".to_string());
    }
    let cancelled = Arc::new(AtomicBool::new(false));
    match COMPRESSION_ANALYSIS_FLAGS.entry(analysis_id.clone()) {
        Entry::Vacant(entry) => {
            entry.insert(cancelled.clone());
        }
        Entry::Occupied(_) => {
            return Err(format!(
                "Compression analysis is already running: {analysis_id}"
            ))
        }
    }
    let _analysis_guard = CompressionAnalysisGuard { analysis_id: analysis_id.clone() };
    let result = tauri::async_runtime::spawn_blocking(move || {
        crate::services::compression_analysis::analyze_compression(
            &paths, &format, level, &cancelled,
        )
    })
    .await
    .map_err(|error| error.to_string())
    .and_then(|result| result.map_err(|error| error.to_string()));
    result
}

#[command]
pub async fn cancel_compression_analysis(analysis_id: String) -> Result<(), String> {
    let flag = COMPRESSION_ANALYSIS_FLAGS
        .get(&analysis_id)
        .ok_or_else(|| format!("Compression analysis is not active: {analysis_id}"))?;
    flag.store(true, Ordering::SeqCst);
    Ok(())
}

#[command]
pub async fn diagnose_archive(
    diagnostic_id: String,
    file_path: String,
    password: Option<String>,
) -> Result<crate::services::archive_diagnostics::ArchiveDiagnosticReport, String> {
    let cancelled = Arc::new(AtomicBool::new(false));
    match ARCHIVE_DIAGNOSTIC_FLAGS.entry(diagnostic_id.clone()) {
        Entry::Vacant(entry) => {
            entry.insert(cancelled.clone());
        }
        Entry::Occupied(_) => return Err(format!("Archive diagnosis is already running: {diagnostic_id}")),
    }
    let _guard = ArchiveDiagnosticGuard { diagnostic_id };
    crate::services::archive_diagnostics::diagnose_archive(
        std::path::Path::new(&file_path),
        password.as_deref(),
        cancelled,
    )
    .await
    .map_err(|error| error.to_string())
}

#[command]
pub async fn cancel_archive_diagnosis(diagnostic_id: String) -> Result<(), String> {
    let flag = ARCHIVE_DIAGNOSTIC_FLAGS
        .get(&diagnostic_id)
        .ok_or_else(|| format!("Archive diagnosis is not active: {diagnostic_id}"))?;
    flag.store(true, Ordering::SeqCst);
    Ok(())
}

/// Returns structured archive metadata for the archive browser.
#[command]
pub async fn browse_archive(
    file_path: String,
    password: Option<String>,
    browse_id: Option<String>,
) -> Result<crate::models::compression::ArchiveBrowseResult, String> {
    let resolved_password = match password.filter(|value| !value.is_empty()) {
        Some(password) => Some(password),
        None => {
            let service = CompressionService::new_with_defaults().await;
            service
                .resolve_archive_password_silent(&file_path, &DecompressOptions::default())
                .await
        }
    };
    let browse_id = browse_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let cancelled = Arc::new(AtomicBool::new(false));
    match ARCHIVE_BROWSE_FLAGS.entry(browse_id.clone()) {
        Entry::Vacant(entry) => {
            entry.insert(cancelled.clone());
        }
        Entry::Occupied(entry) => {
            if entry.get().load(Ordering::Relaxed) {
                entry.remove();
                return Err("ARCHIVE_BROWSE_CANCELLED|已取消读取压缩包内容".to_string());
            }
            return Err("ARCHIVE_BROWSE_FAILED|同一个读取请求已在执行".to_string());
        }
    }
    let _guard = ArchiveBrowseGuard { browse_id };
    crate::services::archive_browser::browse_archive_cancellable(
        std::path::Path::new(&file_path),
        resolved_password.as_deref(),
        cancelled,
    )
    .await
    .map_err(|error| classify_archive_browse_error(&error.to_string()))
}

#[command]
pub async fn cancel_archive_browse(browse_id: String) -> Result<(), String> {
    let flag = match ARCHIVE_BROWSE_FLAGS.entry(browse_id.clone()) {
        Entry::Vacant(entry) => {
            entry.insert(Arc::new(AtomicBool::new(true))).clone()
        }
        Entry::Occupied(entry) => {
            entry.get().store(true, Ordering::Relaxed);
            entry.get().clone()
        }
    };
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        if let Some(current) = ARCHIVE_BROWSE_FLAGS.get(&browse_id) {
            if Arc::ptr_eq(current.value(), &flag) {
                drop(current);
                ARCHIVE_BROWSE_FLAGS.remove(&browse_id);
            }
        }
    });
    Ok(())
}

/// Reads one supported raster image from an archive under strict byte and pixel limits.
#[command]
pub async fn preview_archive_image(
    file_path: String,
    entry_path: String,
    password: Option<String>,
) -> Result<crate::services::archive_preview::ArchiveImagePreview, String> {
    let resolved_password = match password.filter(|value| !value.is_empty()) {
        Some(password) => Some(password),
        None => {
            let service = CompressionService::new_with_defaults().await;
            service
                .resolve_archive_password_silent(&file_path, &DecompressOptions::default())
                .await
        }
    };
    crate::services::archive_preview::preview_archive_image(
        std::path::Path::new(&file_path),
        &entry_path,
        resolved_password.as_deref(),
    )
    .await
    .map_err(|error| error.to_string())
}

struct ArchiveBrowseGuard {
    browse_id: String,
}

impl Drop for ArchiveBrowseGuard {
    fn drop(&mut self) {
        ARCHIVE_BROWSE_FLAGS.remove(&self.browse_id);
    }
}

fn classify_archive_browse_error(error: &str) -> String {
    let lower = error.to_ascii_lowercase();
    if error.contains("ARCHIVE_BROWSE_CANCELLED") {
        return "ARCHIVE_BROWSE_CANCELLED|已取消读取压缩包内容".to_string();
    }
    if error.contains("ARCHIVE_BROWSE_TIMEOUT") {
        return "ARCHIVE_BROWSE_TIMEOUT|读取压缩包内容超过 30 秒，已停止等待；归档可能损坏或所在设备响应过慢".to_string();
    }
    if lower.contains("password")
        || lower.contains("checksumverificationfailed")
        || lower.contains("encrypted")
    {
        return format!("ARCHIVE_BROWSE_PASSWORD|密码不正确，或这一层归档需要单独的密码|{error}");
    }
    if lower.contains("unsupported") || lower.contains("did not expose any browseable") {
        return format!("ARCHIVE_BROWSE_UNSUPPORTED|该文件不是受支持的可浏览归档|{error}");
    }
    if lower.contains("invalid")
        || lower.contains("corrupt")
        || lower.contains("unable to read")
        || lower.contains("unexpected")
    {
        return format!("ARCHIVE_BROWSE_DAMAGED|压缩包结构损坏或内容不完整|{error}");
    }
    format!("ARCHIVE_BROWSE_FAILED|无法读取压缩包内容|{error}")
}

/// Reads one text entry without extracting it to disk. The service caps the
/// decoded prefix, rejects binary payloads and reports the detected encoding.
#[command]
pub async fn preview_archive_text(
    file_path: String,
    entry_path: String,
    password: Option<String>,
) -> Result<crate::services::archive_preview::ArchiveTextPreview, String> {
    let resolved_password = match password.filter(|value| !value.is_empty()) {
        Some(password) => Some(password),
        None => {
            let service = CompressionService::new_with_defaults().await;
            service
                .resolve_archive_password_silent(&file_path, &DecompressOptions::default())
                .await
        }
    };
    crate::services::archive_preview::preview_archive_text(
        std::path::Path::new(&file_path),
        &entry_path,
        resolved_password.as_deref(),
    )
    .await
    .map_err(|error| error.to_string())
}

/// Safely materializes one nested archive into the shared session cache without
/// launching it. The caller can then browse the returned path as a child workspace.
#[command]
pub async fn materialize_nested_archive(
    window: Window,
    cache: State<'_, crate::services::archive_entry_open::ArchiveEntryOpenCache>,
    file_path: String,
    entry_path: String,
    password: Option<String>,
    target_depth: usize,
    ancestor_hashes: Vec<String>,
) -> Result<crate::services::archive_entry_open::NestedArchiveMaterializeResult, String> {
    use crate::services::archive_entry_open::{
        normalize_safe_entry_path, validate_extracted_file, validate_nested_archive_identity,
        NestedArchiveMaterializeResult,
    };

    let entry_path = normalize_safe_entry_path(&entry_path).map_err(|error| error.to_string())?;
    let archive = std::path::PathBuf::from(&file_path);
    let service = CompressionService::new_with_defaults().await;
    let resolved_password = match password.filter(|value| !value.is_empty()) {
        Some(password) => Some(password),
        None => service
            .resolve_archive_password_silent(&file_path, &DecompressOptions::default())
            .await,
    };
    let metadata = crate::services::archive_browser::browse_archive(
        &archive,
        resolved_password.as_deref(),
    )
    .await
    .map_err(|error| error.to_string())?;
    let selected = metadata
        .entries
        .iter()
        .find(|entry| !entry.is_dir && entry.path.replace('\\', "/") == entry_path)
        .ok_or_else(|| "所选嵌套归档不存在，或不是普通文件".to_string())?;
    let expected_bytes = selected.size;
    let (entry_dir, reservation) = cache
        .create_entry_dir(expected_bytes)
        .map_err(|error| error.to_string())?;
    let options = DecompressOptions {
        preserve_paths: true,
        overwrite_existing: false,
        delete_after: false,
        preserve_timestamps: true,
        skip_corrupted: false,
        extract_only_newer: false,
        create_subdirectory: false,
        preserve_mark_of_web: true,
        file_filter: None,
        selected_entries: vec![entry_path.clone()],
        conflict_policy: "rename".to_string(),
        enable_bruteforce: false,
        bruteforce_wordlists: Vec::new(),
    };
    let task_id = format!("archive-nested-{}", uuid::Uuid::new_v4());
    if let Err(error) = service
        .extract(
            window,
            task_id,
            file_path,
            Some(entry_dir.to_string_lossy().into_owned()),
            resolved_password,
            options,
        )
        .await
    {
        let _ = std::fs::remove_dir_all(&entry_dir);
        return Err(error.to_string());
    }
    let extracted = validate_extracted_file(&entry_dir, &entry_path, expected_bytes)
        .map_err(|error| {
            let _ = std::fs::remove_dir_all(&entry_dir);
            error.to_string()
        })?;
    let (parent_sha256, content_sha256) = validate_nested_archive_identity(
        &archive,
        &extracted,
        target_depth,
        &ancestor_hashes,
    )
    .map_err(|error| {
        let _ = std::fs::remove_dir_all(&entry_dir);
        error.to_string()
    })?;
    cache
        .register_nested_archive(
            &archive,
            &extracted,
            &parent_sha256,
            &content_sha256,
            target_depth,
        )
        .map_err(|error| {
            let _ = std::fs::remove_dir_all(&entry_dir);
            error.to_string()
        })?;
    reservation.commit();
    Ok(NestedArchiveMaterializeResult {
        entry_path,
        cache_path: extracted.to_string_lossy().into_owned(),
        parent_sha256,
        content_sha256,
        depth: target_depth,
    })
}

/// Extracts one validated archive entry into an isolated session cache and opens it
/// through the Windows default application. Active content requires an explicit
/// second call with `allow_dangerous` set to true.
#[command]
pub async fn open_archive_entry(
    window: Window,
    cache: State<'_, crate::services::archive_entry_open::ArchiveEntryOpenCache>,
    file_path: String,
    entry_path: String,
    password: Option<String>,
    allow_dangerous: bool,
) -> Result<crate::services::archive_entry_open::ArchiveEntryOpenResult, String> {
    use crate::services::archive_entry_open::{
        is_dangerous_entry, normalize_safe_entry_path, open_with_default_application,
        validate_extracted_file,
        ArchiveEntryOpenResult,
    };

    let entry_path = normalize_safe_entry_path(&entry_path).map_err(|error| error.to_string())?;
    let dangerous = is_dangerous_entry(&entry_path);
    if dangerous && !allow_dangerous {
        return Ok(ArchiveEntryOpenResult {
            status: "confirmationRequired".to_string(),
            entry_path,
            cache_path: None,
            dangerous: true,
        });
    }

    let archive = std::path::Path::new(&file_path);
    let service = CompressionService::new_with_defaults().await;
    let resolved_password = match password.filter(|value| !value.is_empty()) {
        Some(password) => Some(password),
        None => service
            .resolve_archive_password_silent(&file_path, &DecompressOptions::default())
            .await,
    };
    let metadata = crate::services::archive_browser::browse_archive(
        archive,
        resolved_password.as_deref(),
    )
    .await
    .map_err(|error| error.to_string())?;
    let selected = metadata
        .entries
        .iter()
        .find(|entry| !entry.is_dir && entry.path.replace('\\', "/") == entry_path)
        .ok_or_else(|| "所选文件不存在于压缩包中，或不是普通文件".to_string())?;
    let expected_bytes = selected.size;
    let (entry_dir, reservation) = cache
        .create_entry_dir(expected_bytes)
        .map_err(|error| error.to_string())?;

    let options = DecompressOptions {
        preserve_paths: true,
        overwrite_existing: false,
        delete_after: false,
        preserve_timestamps: true,
        skip_corrupted: false,
        extract_only_newer: false,
        create_subdirectory: false,
        preserve_mark_of_web: true,
        file_filter: None,
        selected_entries: vec![entry_path.clone()],
        conflict_policy: "rename".to_string(),
        enable_bruteforce: false,
        bruteforce_wordlists: Vec::new(),
    };
    let task_id = format!("archive-open-{}", uuid::Uuid::new_v4());
    if let Err(error) = service
        .extract(
            window,
            task_id,
            file_path,
            Some(entry_dir.to_string_lossy().into_owned()),
            resolved_password,
            options,
        )
        .await
    {
        let _ = std::fs::remove_dir_all(&entry_dir);
        return Err(error.to_string());
    }

    let extracted = validate_extracted_file(&entry_dir, &entry_path, expected_bytes)
        .map_err(|error| {
            let _ = std::fs::remove_dir_all(&entry_dir);
            error.to_string()
        })?;
    open_with_default_application(&extracted)
    .map_err(|error| {
        let _ = std::fs::remove_dir_all(&entry_dir);
        error.to_string()
    })?;
    reservation.commit();

    Ok(ArchiveEntryOpenResult {
        status: "opened".to_string(),
        entry_path,
        cache_path: Some(extracted.to_string_lossy().into_owned()),
        dangerous,
    })
}

/// 检测归档文件完整性（通过 7z CLI 的 t 命令）
#[command]
pub async fn test_archive_integrity(file_path: String, password: Option<String>) -> Result<String, String> {
    use crate::services::universal_engine::UniversalCliEngine;

    let path = std::path::Path::new(&file_path);
    UniversalCliEngine::test_integrity(path, password.as_deref())
        .await
        .map(|_| "Archive integrity verified".to_string())
        .map_err(|e| e.to_string())
}

/// 将 ZIP 中仍可完整读取的条目重建到一个新的、已校验的归档。
#[command]
pub async fn repair_zip(
    repair_id: String,
    file_path: String,
    output_path: String,
) -> Result<crate::services::archive_diagnostics::ZipRepairResult, String> {
    let cancelled = Arc::new(AtomicBool::new(false));
    match ZIP_REPAIR_FLAGS.entry(repair_id.clone()) {
        Entry::Vacant(entry) => { entry.insert(cancelled.clone()); }
        Entry::Occupied(_) => return Err(format!("ZIP repair is already running: {repair_id}")),
    }
    let _guard = ZipRepairGuard { repair_id };
    tauri::async_runtime::spawn_blocking(move || {
        crate::services::archive_diagnostics::repair_zip_to_new(
            std::path::Path::new(&file_path),
            std::path::Path::new(&output_path),
            &cancelled,
        )
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}

#[command]
pub async fn cancel_zip_repair(repair_id: String) -> Result<(), String> {
    let flag = ZIP_REPAIR_FLAGS
        .get(&repair_id)
        .ok_or_else(|| format!("ZIP repair is not active: {repair_id}"))?;
    flag.store(true, Ordering::SeqCst);
    Ok(())
}

#[cfg(test)]
mod cancellation_tests {
    use super::{
        cancel_archive_browse, cancel_compression, cancel_tasks_and_wait,
        classify_archive_browse_error, image_stage_log, normalized_output_key,
        plan_video_destination, run_image_compression, run_video_compression, video_progress_payload,
        ArchiveDiagnosticGuard, CompressionAnalysisGuard, CompressionOutputGuard,
        VideoCompressionCommandEvent, VideoCompressionExecutionRequest, VideoCompressionStage,
        ZipRepairGuard,
        ACTIVE_COMPRESSION_OUTPUTS, CANCELLATION_FLAGS, COMPRESSION_ANALYSIS_FLAGS,
        ARCHIVE_BROWSE_FLAGS, ARCHIVE_DIAGNOSTIC_FLAGS, ZIP_REPAIR_FLAGS,
    };
    use crate::services::image_compression_service::{
        ImageCompressionMode, ImageCompressionOutcome, ImageCompressionRequest,
        ImageCompressionStage, ImageFileFormat,
    };
    use crate::services::video_compression_plan::{
        build_video_compression_plan, VideoCompressionPlanRequest, VideoCompressionPreset,
    };
    use crate::services::video_encoding::{VideoEncodingEvent, VideoProgressSnapshot};
    use crate::services::video_probe::probe_video_file;
    use std::path::Path;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    #[tokio::test]
    async fn cancel_and_wait_signals_then_observes_task_cleanup() {
        let task_id = "cancel-and-wait-test".to_string();
        let flag = Arc::new(AtomicBool::new(false));
        CANCELLATION_FLAGS.insert(task_id.clone(), flag.clone());

        let cleanup_id = task_id.clone();
        let cleanup_flag = flag.clone();
        tokio::spawn(async move {
            while !cleanup_flag.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
            CANCELLATION_FLAGS.remove(&cleanup_id);
        });

        cancel_tasks_and_wait(vec![task_id]).await.unwrap();
        assert!(flag.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn single_task_cancellation_waits_for_backend_cleanup() {
        let task_id = format!("cancel-single-{}", uuid::Uuid::new_v4());
        let flag = Arc::new(AtomicBool::new(false));
        CANCELLATION_FLAGS.insert(task_id.clone(), flag.clone());

        let cleanup_id = task_id.clone();
        let cleanup_flag = flag.clone();
        tokio::spawn(async move {
            while !cleanup_flag.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(25)).await;
            CANCELLATION_FLAGS.remove(&cleanup_id);
        });

        cancel_compression(task_id.clone()).await.unwrap();
        assert!(flag.load(Ordering::SeqCst));
        assert!(!CANCELLATION_FLAGS.contains_key(&task_id));
    }

    #[tokio::test]
    async fn image_command_uses_preflight_blocking_worker_and_shared_task_registry() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("test-results")
            .join("media-fixture-audit")
            .join("fixtures")
            .join("images")
            .join("transparent.png");
        if !source.exists() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("command-output.png");
        let task_id = format!("image-command-{}", uuid::Uuid::new_v4());
        let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
        let observed_for_worker = observed.clone();
        let outcome = run_image_compression(
            task_id.clone(),
            ImageCompressionRequest {
                source,
                destination: destination.clone(),
                mode: ImageCompressionMode::Lossless,
                quality: 82,
                target_format: ImageFileFormat::Png,
                max_dimensions: None,
                preserve_metadata: true,
                only_if_smaller: false,
            },
            move |stage| observed_for_worker.lock().unwrap().push(stage),
        )
        .await
        .unwrap();
        assert!(matches!(outcome, ImageCompressionOutcome::Published { .. }));
        assert_eq!(
            *observed.lock().unwrap(),
            vec![
                ImageCompressionStage::Decoding,
                ImageCompressionStage::Encoding,
                ImageCompressionStage::Validating,
                ImageCompressionStage::Publishing,
            ]
        );
        assert!(destination.is_file());
        assert!(!CANCELLATION_FLAGS.contains_key(&task_id));
    }

    #[test]
    fn image_stage_log_matches_the_existing_task_log_contract() {
        let payload = serde_json::to_value(image_stage_log(
            "image-task",
            ImageCompressionStage::Validating,
        ))
        .unwrap();
        assert_eq!(payload["task_id"], "image-task");
        assert_eq!(payload["severity"], "Info");
        assert_eq!(payload["message"], "正在重新解码并验证候选输出");
        assert!(payload["timestamp"].as_str().is_some());
    }

    #[test]
    fn video_progress_extends_the_existing_event_without_faking_byte_progress() {
        let snapshot = VideoProgressSnapshot {
            current_time_ms: 1_250,
            progress_percent: 37.5,
            speed_multiple: Some(1.25),
            output_bytes: Some(8_192),
            output_bytes_provisional: true,
            eta_seconds: Some(12),
            output_to_input_ratio: Some(0.4),
            finished: false,
        };
        let payload = serde_json::to_value(video_progress_payload(
            "video-task",
            Path::new("C:/input/video.mp4"),
            "still-encoding",
            Some(&snapshot),
            Some(5),
        ))
        .unwrap();

        assert_eq!(payload["task_id"], "video-task");
        assert_eq!(payload["stage"], "still-encoding");
        assert_eq!(payload["progress"], 0.375);
        assert_eq!(payload["processed_bytes"], 0);
        assert_eq!(payload["total_bytes"], 0);
        assert_eq!(payload["output_bytes"], 8_192);
        assert_eq!(payload["output_bytes_estimated"], true);
        assert_eq!(payload["current_time_ms"], 1_250);
        assert_eq!(payload["heartbeat_seconds_since_progress"], 5);
        assert!(payload["heartbeat_at"].as_str().is_some());
    }

    #[test]
    fn video_destination_planner_never_overwrites_or_duplicates_batch_targets() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("演示.video.mov");
        std::fs::write(&source, b"source").unwrap();
        let first = temp.path().join("演示.video.compressed.mp4");
        std::fs::write(&first, b"existing").unwrap();
        let reserved = temp.path().join("演示.video.compressed (1).mp4");

        let plan = plan_video_destination(&source, None, &[reserved]).unwrap();

        assert_eq!(
            plan.destination,
            temp.path().join("演示.video.compressed (2).mp4")
        );
        assert_eq!(std::fs::read(first).unwrap(), b"existing");
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn video_command_replans_then_validates_and_publishes_through_shared_guards() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let resource_root = manifest.join("resources");
        let source = manifest
            .join("..")
            .join("tests/fixtures/media/videos/h264-vfr-audio-rotation-subtitles.mp4");
        if !source.exists() {
            return;
        }
        let plan_request = VideoCompressionPlanRequest {
            path: source.to_string_lossy().into_owned(),
            preset: VideoCompressionPreset::Balanced,
            max_width: None,
            max_height: None,
        };
        let probe = probe_video_file(
            &resource_root.join("video-engine/ffprobe.exe"),
            &source,
        )
        .await
        .unwrap();
        let plan = build_video_compression_plan(probe, &plan_request).unwrap();
        assert!(plan.requires_explicit_confirmation);

        let temp = tempfile::tempdir().unwrap();
        let refused_output = temp.path().join("refused.mp4");
        let refused_task = format!("video-refused-{}", uuid::Uuid::new_v4());
        let refusal = run_video_compression(
            resource_root.clone(),
            refused_task.clone(),
            VideoCompressionExecutionRequest {
                plan: plan_request.clone(),
                destination: refused_output.clone(),
                confirmed_stream_changes: Vec::new(),
                preserve_mark_of_web: true,
            },
            |_| {},
        )
        .await
        .unwrap_err();
        assert_eq!(
            refusal,
            "VIDEO_COMPRESSION_STREAM_CHANGES_CONFIRMATION_REQUIRED"
        );
        assert!(!refused_output.exists());
        assert!(!CANCELLATION_FLAGS.contains_key(&refused_task));

        let destination = temp.path().join("published & verified.mp4");
        let task_id = format!("video-command-{}", uuid::Uuid::new_v4());
        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let observed = events.clone();
        let outcome = run_video_compression(
            resource_root,
            task_id.clone(),
            VideoCompressionExecutionRequest {
                plan: plan_request,
                destination: destination.clone(),
                confirmed_stream_changes: plan.stream_changes,
                preserve_mark_of_web: true,
            },
            move |event| {
                let label = match event {
                    VideoCompressionCommandEvent::Stage(stage) => stage.event_name().to_string(),
                    VideoCompressionCommandEvent::Encoding(VideoEncodingEvent::Progress { .. }) => {
                        "progress".to_string()
                    }
                    VideoCompressionCommandEvent::Encoding(VideoEncodingEvent::Heartbeat { .. }) => {
                        "heartbeat".to_string()
                    }
                };
                observed.lock().unwrap().push(label);
            },
        )
        .await
        .unwrap();

        assert_eq!(outcome.path, destination);
        assert!(outcome.output_bytes > 0);
        assert_eq!(outcome.verified.container, "mp4");
        assert_eq!(outcome.verified.video_codec, "h264");
        assert!(source.exists());
        assert!(!CANCELLATION_FLAGS.contains_key(&task_id));
        let events = events.lock().unwrap();
        for stage in [
            VideoCompressionStage::Probing,
            VideoCompressionStage::Encoding,
            VideoCompressionStage::Validating,
            VideoCompressionStage::Publishing,
        ] {
            assert!(events.contains(&stage.event_name().to_string()));
        }
        assert!(events.iter().any(|event| event == "progress"));
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn c05_real_format_resolution_preset_matrix() {
        let Ok(manifest_path) = std::env::var("LONG_C05_VIDEO_MATRIX_MANIFEST") else {
            println!("C-05.2.1 matrix skipped: LONG_C05_VIDEO_MATRIX_MANIFEST is not set");
            return;
        };
        let output_root = std::env::var("LONG_C05_VIDEO_MATRIX_OUTPUT")
            .expect("LONG_C05_VIDEO_MATRIX_OUTPUT must accompany the matrix manifest");
        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&manifest_path).expect("read C-05.2.1 runtime manifest"),
        )
        .expect("parse C-05.2.1 runtime manifest");
        let cases = manifest["cases"]
            .as_array()
            .expect("runtime manifest cases");
        assert_eq!(cases.len(), 7);

        let resource_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources");
        let ffprobe = resource_root.join("video-engine/ffprobe.exe");
        let output_root = Path::new(&output_root);
        std::fs::create_dir_all(output_root).expect("create C-05.2.1 output root");
        let mut results = Vec::with_capacity(cases.len());

        for case in cases {
            let id = case["id"].as_str().expect("case id");
            let source = Path::new(case["sourcePath"].as_str().expect("source path"));
            let source_bytes = std::fs::metadata(source).expect("source metadata").len();
            let input = probe_video_file(&ffprobe, source)
                .await
                .expect("product probe must accept matrix input");
            assert!(input
                .container
                .as_deref()
                .unwrap_or_default()
                .contains(case["inputContainerNeedle"].as_str().expect("container")));
            assert_eq!(
                input.primary_video.codec.as_deref(),
                case["inputVideoCodec"].as_str()
            );
            assert_eq!(
                input.audio_streams.first().and_then(|audio| audio.codec.as_deref()),
                case["inputAudioCodec"].as_str()
            );

            let preset: VideoCompressionPreset = serde_json::from_value(case["preset"].clone())
                .expect("deserialize matrix preset");
            let plan_request = VideoCompressionPlanRequest {
                path: source.to_string_lossy().into_owned(),
                preset,
                max_width: None,
                max_height: None,
            };
            let plan = build_video_compression_plan(input, &plan_request)
                .expect("build product compression plan");
            let expected_width = case["outputWidth"].as_u64().expect("output width") as u32;
            let expected_height = case["outputHeight"].as_u64().expect("output height") as u32;
            assert_eq!((plan.output_width, plan.output_height), (expected_width, expected_height));

            let destination = output_root.join(format!("{id}.mp4"));
            let outcome = run_video_compression(
                resource_root.clone(),
                format!("c05-matrix-{id}-{}", uuid::Uuid::new_v4()),
                VideoCompressionExecutionRequest {
                    plan: plan_request,
                    destination: destination.clone(),
                    confirmed_stream_changes: plan.stream_changes,
                    preserve_mark_of_web: false,
                },
                |_| {},
            )
            .await
            .expect("execute product compression pipeline");

            assert_eq!(outcome.path, destination);
            assert_eq!(outcome.input_bytes, source_bytes);
            assert_eq!(std::fs::metadata(source).expect("source remains").len(), source_bytes);
            assert_eq!(outcome.verified.container, "mp4");
            assert_eq!(outcome.verified.video_codec, "h264");
            assert_eq!(outcome.verified.visible_width, expected_width);
            assert_eq!(outcome.verified.visible_height, expected_height);
            assert_eq!(
                outcome.verified.audio_codec.as_deref(),
                case["inputAudioCodec"].as_str().map(|_| "aac")
            );
            assert!(outcome.verified.decoded_video_frames > 0);
            assert!(outcome.output_bytes > 0);
            results.push(serde_json::to_value(&outcome).expect("serialize matrix outcome"));
        }

        std::fs::write(
            output_root.join("backend-result.json"),
            serde_json::to_vec_pretty(&results).expect("serialize matrix results"),
        )
        .expect("write matrix backend result");
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn c05_real_long_duration_and_large_input_matrix() {
        let Ok(manifest_path) = std::env::var("LONG_C05_VIDEO_LONG_LARGE_MANIFEST") else {
            println!(
                "C-05.2.2 matrix skipped: LONG_C05_VIDEO_LONG_LARGE_MANIFEST is not set"
            );
            return;
        };
        let output_root = std::env::var("LONG_C05_VIDEO_LONG_LARGE_OUTPUT")
            .expect("LONG_C05_VIDEO_LONG_LARGE_OUTPUT must accompany the matrix manifest");
        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&manifest_path).expect("read C-05.2.2 runtime manifest"),
        )
        .expect("parse C-05.2.2 runtime manifest");
        let cases = manifest["cases"]
            .as_array()
            .expect("runtime manifest cases");
        assert_eq!(cases.len(), 2);

        let resource_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources");
        let ffprobe = resource_root.join("video-engine/ffprobe.exe");
        let output_root = Path::new(&output_root);
        std::fs::create_dir_all(output_root).expect("create C-05.2.2 output root");
        let mut results = Vec::with_capacity(cases.len());

        for case in cases {
            let id = case["id"].as_str().expect("case id");
            let source = Path::new(case["sourcePath"].as_str().expect("source path"));
            let source_bytes = std::fs::metadata(source).expect("source metadata").len();
            let minimum_input_bytes = case["minimumInputBytes"]
                .as_u64()
                .expect("minimum input bytes");
            assert!(source_bytes >= minimum_input_bytes);

            let input = probe_video_file(&ffprobe, source)
                .await
                .expect("product probe must accept long/large input");
            let expected_duration_ms = case["durationSeconds"]
                .as_u64()
                .expect("duration seconds")
                * 1_000;
            assert!(input.duration_ms.abs_diff(expected_duration_ms) <= 100);
            assert_eq!(input.primary_video.codec.as_deref(), Some("mpeg4"));
            assert!(input.audio_streams.is_empty());

            let preset: VideoCompressionPreset = serde_json::from_value(case["preset"].clone())
                .expect("deserialize long/large preset");
            let plan_request = VideoCompressionPlanRequest {
                path: source.to_string_lossy().into_owned(),
                preset,
                max_width: None,
                max_height: None,
            };
            let plan = build_video_compression_plan(input, &plan_request)
                .expect("build product long/large plan");
            let expected_width = case["outputWidth"].as_u64().expect("output width") as u32;
            let expected_height = case["outputHeight"].as_u64().expect("output height") as u32;
            assert_eq!((plan.output_width, plan.output_height), (expected_width, expected_height));

            let progress = Arc::new(std::sync::Mutex::new(Vec::new()));
            let observed = progress.clone();
            let destination = output_root.join(format!("{id}.mp4"));
            let outcome = run_video_compression(
                resource_root.clone(),
                format!("c05-long-large-{id}-{}", uuid::Uuid::new_v4()),
                VideoCompressionExecutionRequest {
                    plan: plan_request,
                    destination: destination.clone(),
                    confirmed_stream_changes: plan.stream_changes,
                    preserve_mark_of_web: false,
                },
                move |event| {
                    if let VideoCompressionCommandEvent::Encoding(
                        VideoEncodingEvent::Progress { snapshot },
                    ) = event
                    {
                        observed.lock().unwrap().push(snapshot);
                    }
                },
            )
            .await
            .expect("execute product long/large compression pipeline");

            let progress = progress.lock().unwrap();
            let progress_events = progress.len();
            let maximum_progress_time_ms = progress
                .iter()
                .map(|snapshot| snapshot.current_time_ms)
                .max()
                .unwrap_or(0);
            assert!(progress_events > 0);
            assert!(progress.iter().any(|snapshot| snapshot.finished));
            drop(progress);

            let minimum_frames = case["minimumDecodedVideoFrames"]
                .as_u64()
                .expect("minimum decoded frames");
            assert_eq!(outcome.path, destination);
            assert_eq!(outcome.input_bytes, source_bytes);
            assert_eq!(std::fs::metadata(source).expect("source remains").len(), source_bytes);
            assert_eq!(outcome.verified.container, "mp4");
            assert_eq!(outcome.verified.video_codec, "h264");
            assert_eq!(outcome.verified.audio_codec, None);
            assert_eq!(outcome.verified.visible_width, expected_width);
            assert_eq!(outcome.verified.visible_height, expected_height);
            assert!(outcome.verified.decoded_video_frames >= minimum_frames);
            assert!(outcome.verified.duration_difference_ms <= outcome.verified.duration_tolerance_ms);
            assert!(outcome.output_bytes > 0);
            results.push(serde_json::json!({
                "id": id,
                "kind": case["kind"],
                "sourceBytes": source_bytes,
                "progressEvents": progress_events,
                "maximumProgressTimeMs": maximum_progress_time_ms,
                "outcome": outcome,
            }));
        }

        std::fs::write(
            output_root.join("backend-result.json"),
            serde_json::to_vec_pretty(&results).expect("serialize long/large results"),
        )
        .expect("write long/large backend result");
    }

    #[tokio::test]
    async fn archive_browse_cancellation_can_arrive_before_registration() {
        let browse_id = format!("browse-early-cancel-{}", uuid::Uuid::new_v4());
        cancel_archive_browse(browse_id.clone()).await.unwrap();
        assert!(ARCHIVE_BROWSE_FLAGS
            .get(&browse_id)
            .unwrap()
            .load(Ordering::Relaxed));
        ARCHIVE_BROWSE_FLAGS.remove(&browse_id);
    }

    #[test]
    fn archive_browse_errors_are_classified_for_the_ui() {
        assert!(classify_archive_browse_error("ARCHIVE_BROWSE_TIMEOUT")
            .starts_with("ARCHIVE_BROWSE_TIMEOUT|"));
        assert!(classify_archive_browse_error("ChecksumVerificationFailed")
            .starts_with("ARCHIVE_BROWSE_PASSWORD|"));
        assert!(classify_archive_browse_error("corrupt archive")
            .starts_with("ARCHIVE_BROWSE_DAMAGED|"));
    }

    #[test]
    fn equivalent_output_paths_share_one_active_reservation() {
        let task_id = format!("output-owner-{}", uuid::Uuid::new_v4());
        let output = std::env::temp_dir()
            .join(format!("long-compress-output-{}.7z", uuid::Uuid::new_v4()));
        let equivalent = output
            .parent()
            .unwrap()
            .join(".")
            .join(output.file_name().unwrap());

        let guard = CompressionOutputGuard::acquire(&task_id, &output.to_string_lossy()).unwrap();
        let duplicate = CompressionOutputGuard::acquire(
            "duplicate-output-owner",
            &equivalent.to_string_lossy(),
        );

        assert!(duplicate.unwrap_err().contains("already writing this output"));
        drop(guard);
        assert!(CompressionOutputGuard::acquire(
            "replacement-output-owner",
            &output.to_string_lossy(),
        )
        .is_ok());
    }

    #[test]
    fn output_reservation_is_removed_when_guard_drops() {
        let task_id = format!("output-cleanup-{}", uuid::Uuid::new_v4());
        let output = std::env::temp_dir()
            .join(format!("long-compress-cleanup-{}.zip", uuid::Uuid::new_v4()));
        let key = normalized_output_key(&output.to_string_lossy()).unwrap();

        {
            let _guard =
                CompressionOutputGuard::acquire(&task_id, &output.to_string_lossy()).unwrap();
            assert_eq!(
                ACTIVE_COMPRESSION_OUTPUTS
                    .get(&key)
                    .map(|owner| owner.value().clone()),
                Some(task_id)
            );
        }

        assert!(!ACTIVE_COMPRESSION_OUTPUTS.contains_key(&key));
    }

    #[test]
    fn analysis_registration_is_removed_when_command_future_drops() {
        let analysis_id = format!("analysis-cleanup-{}", uuid::Uuid::new_v4());
        COMPRESSION_ANALYSIS_FLAGS.insert(
            analysis_id.clone(),
            Arc::new(AtomicBool::new(false)),
        );
        {
            let _guard = CompressionAnalysisGuard {
                analysis_id: analysis_id.clone(),
            };
            assert!(COMPRESSION_ANALYSIS_FLAGS.contains_key(&analysis_id));
        }
        assert!(!COMPRESSION_ANALYSIS_FLAGS.contains_key(&analysis_id));
    }

    #[test]
    fn diagnostic_and_repair_registrations_are_removed_when_futures_drop() {
        let diagnostic_id = format!("diagnostic-cleanup-{}", uuid::Uuid::new_v4());
        ARCHIVE_DIAGNOSTIC_FLAGS.insert(diagnostic_id.clone(), Arc::new(AtomicBool::new(false)));
        {
            let _guard = ArchiveDiagnosticGuard { diagnostic_id: diagnostic_id.clone() };
            assert!(ARCHIVE_DIAGNOSTIC_FLAGS.contains_key(&diagnostic_id));
        }
        assert!(!ARCHIVE_DIAGNOSTIC_FLAGS.contains_key(&diagnostic_id));

        let repair_id = format!("repair-cleanup-{}", uuid::Uuid::new_v4());
        ZIP_REPAIR_FLAGS.insert(repair_id.clone(), Arc::new(AtomicBool::new(false)));
        {
            let _guard = ZipRepairGuard { repair_id: repair_id.clone() };
            assert!(ZIP_REPAIR_FLAGS.contains_key(&repair_id));
        }
        assert!(!ZIP_REPAIR_FLAGS.contains_key(&repair_id));
    }
}
