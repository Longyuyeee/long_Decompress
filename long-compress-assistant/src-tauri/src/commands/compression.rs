use crate::services::compression_service::CompressionService;
use crate::services::compression_service::RarCompressionSupport;
use crate::models::compression::{CompressionOptions, DecompressOptions};
use tauri::{command, AppHandle, Manager, Window};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use dashmap::DashMap;
use once_cell::sync::Lazy;

static CANCELLATION_FLAGS: Lazy<DashMap<String, Arc<AtomicBool>>> = Lazy::new(DashMap::new);

async fn service_for_task(task_id: &str) -> CompressionService {
    let cancellation_flag = Arc::new(AtomicBool::new(false));
    CANCELLATION_FLAGS.insert(task_id.to_string(), cancellation_flag.clone());

    let mut service = CompressionService::new_with_defaults().await;
    service.cancellation_flag = cancellation_flag;
    service
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
    let service = service_for_task(&task_id).await;
    let _task_guard = TaskCancellationGuard::new(&task_id);
    let opts = options.unwrap_or_default();
    
    let result = service.extract(window, task_id.clone(), file_path, output_path, password, opts)
        .await
        .map_err(|e| e.to_string());

    cleanup_task(&task_id);
    result
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
        
        let service = service_for_task(&task_id).await;
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
    let service = service_for_task(&task_id).await;
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
    if let Some(flag) = CANCELLATION_FLAGS.get(&task_id) {
        flag.store(true, Ordering::SeqCst);
    }
    Ok(())
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
    use super::{cancel_tasks_and_wait, CANCELLATION_FLAGS};
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
}
