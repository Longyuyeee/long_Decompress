use crate::task_queue::{
    TASK_MANAGER,
    models::{TaskPriority, QueueTaskStatus, QueueTask},
    batch_task_processor::{BatchTaskRequest, BatchTaskResult}
};
use crate::models::compression::CompressionTask;
use std::sync::Arc;
use crate::task_queue::task_manager::TaskManager;
use tauri::command;

/// 获取全局任务管理器实例，如果尚未初始化则返回错误
async fn get_manager() -> Result<Arc<TaskManager>, String> {
    let guard = TASK_MANAGER.read().await;
    guard.clone().ok_or_else(|| "任务管理器尚未初始化，请先初始化应用。".to_string())
}

#[command]
pub async fn add_compression_task(task: CompressionTask, priority: TaskPriority) -> Result<String, String> {
    get_manager().await?.add_compression_task(task, priority).await.map_err(|e| e.to_string())
}

#[command]
pub async fn add_extraction_task(
    file_path: String,
    output_dir: Option<String>,
    password: Option<String>,
    priority: TaskPriority
) -> Result<String, String> {
    get_manager().await?.add_extraction_task(file_path, output_dir, password, priority).await.map_err(|e| e.to_string())
}

#[command]
pub async fn get_task_status(task_id: String) -> Result<QueueTaskStatus, String> {
    get_manager().await?.get_task_status(&task_id).await.map_err(|e| e.to_string())
}

#[command]
pub async fn cancel_task(task_id: String) -> Result<(), String> {
    get_manager().await?.cancel_task(&task_id).await.map_err(|e| e.to_string())
}

#[command]
pub async fn get_all_tasks() -> Result<Vec<QueueTask>, String> {
    get_manager().await?.get_all_tasks().await.map_err(|e| e.to_string())
}

#[command]
pub async fn add_batch_task(request: BatchTaskRequest) -> Result<BatchTaskResult, String> {
    let processor = crate::task_queue::batch_task_processor::BatchTaskProcessor::new();
    processor.process_batch_task(request).await.map_err(|e| e.to_string())
}
