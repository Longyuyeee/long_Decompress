use crate::task_queue::{
    TASK_MANAGER,
    models::{TaskPriority, QueueTaskStatus, QueueTask},
    batch_task_processor::{BatchTaskRequest, BatchTaskResult}
};
use crate::models::compression::CompressionTask;
use tauri::command;

#[command]
pub async fn add_compression_task(task: CompressionTask, priority: TaskPriority) -> Result<String, String> {
    TASK_MANAGER.add_compression_task(task, priority).await.map_err(|e| e.to_string())
}

#[command]
pub async fn add_extraction_task(
    file_path: String,
    output_dir: Option<String>,
    password: Option<String>,
    priority: TaskPriority
) -> Result<String, String> {
    TASK_MANAGER.add_extraction_task(file_path, output_dir, password, priority).await.map_err(|e| e.to_string())
}

#[command]
pub async fn get_task_status(task_id: String) -> Result<QueueTaskStatus, String> {
    TASK_MANAGER.get_task_status(&task_id).await.map_err(|e| e.to_string())
}

#[command]
pub async fn cancel_task(task_id: String) -> Result<(), String> {
    TASK_MANAGER.cancel_task(&task_id).await.map_err(|e| e.to_string())
}

#[command]
pub async fn get_all_tasks() -> Result<Vec<QueueTask>, String> {
    TASK_MANAGER.get_all_tasks().await.map_err(|e| e.to_string())
}

#[command]
pub async fn add_batch_task(request: BatchTaskRequest) -> Result<BatchTaskResult, String> {
    let processor = crate::task_queue::batch_task_processor::BatchTaskProcessor::new();
    processor.process_batch_task(request).await.map_err(|e| e.to_string())
}
