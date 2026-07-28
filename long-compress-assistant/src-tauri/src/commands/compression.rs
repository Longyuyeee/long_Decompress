use crate::services::compression_service::CompressionService;
use crate::services::compression_service::RarCompressionSupport;
use crate::models::compression::{CompressionOptions, DecompressOptions};
use tauri::{command, AppHandle, Manager, Window};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::path::{Component, PathBuf};
use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use once_cell::sync::Lazy;

static CANCELLATION_FLAGS: Lazy<DashMap<String, Arc<AtomicBool>>> = Lazy::new(DashMap::new);
static ACTIVE_COMPRESSION_OUTPUTS: Lazy<DashMap<String, String>> = Lazy::new(DashMap::new);

async fn service_for_task(task_id: &str) -> Result<CompressionService, String> {
    let cancellation_flag = Arc::new(AtomicBool::new(false));
    match CANCELLATION_FLAGS.entry(task_id.to_string()) {
        Entry::Vacant(entry) => {
            entry.insert(cancellation_flag.clone());
        }
        Entry::Occupied(_) => {
            return Err(format!("Task is already running: {task_id}"));
        }
    }

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
pub async fn cancel_compression(task_id: String) -> Result<(), String> {
    let Some(flag) = CANCELLATION_FLAGS.get(&task_id) else {
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

/// 尝试修复损坏的 ZIP 文件
#[command]
pub async fn repair_zip(file_path: String) -> Result<String, String> {
    use crate::services::universal_engine::UniversalCliEngine;

    let path = std::path::Path::new(&file_path);
    UniversalCliEngine::repair_zip(path)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod cancellation_tests {
    use super::{
        cancel_compression, cancel_tasks_and_wait, normalized_output_key, CompressionOutputGuard,
        ACTIVE_COMPRESSION_OUTPUTS, CANCELLATION_FLAGS,
    };
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
}
