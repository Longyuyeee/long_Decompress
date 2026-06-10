use crate::task_queue::models::{QueueTaskStatus, SharedQueueTask, TaskType};
use crate::task_queue::task_scheduler::TaskScheduler;
use crate::services::compression_service::CompressionService;
use crate::models::compression::DecompressOptions;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    pub max_workers: usize,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self { max_workers: 4 }
    }
}

pub struct TaskExecutor {
    scheduler: Arc<TaskScheduler>,
    config: ExecutorConfig,
    app_handle: AppHandle,
}

impl TaskExecutor {
    pub fn new(scheduler: Arc<TaskScheduler>, config: ExecutorConfig, app_handle: AppHandle) -> Self {
        Self {
            scheduler,
            config,
            app_handle,
        }
    }

    /// 执行单个队列任务，调用真实的 CompressionService 进行压缩或解压
    pub async fn execute_task(&self, task_ref: SharedQueueTask) {
        let task_id = {
            let task = task_ref.read().await;
            task.id.clone()
        };

        let (task_type, compression_task) = {
            let task = task_ref.read().await;
            (task.task_type.clone(), task.compression_task.clone())
        };

        let result = match task_type {
            TaskType::Compress => {
                let service = CompressionService::new_with_defaults().await;
                let window = self.app_handle.get_window("main")
                    .unwrap_or_else(|| panic!("main window not found for task executor"));
                let options = compression_task.options.clone();
                service.compress(
                    window,
                    task_id.clone(),
                    compression_task.source_files.clone(),
                    compression_task.output_path.clone(),
                    options,
                ).await
            }
            TaskType::Extract => {
                let service = CompressionService::new_with_defaults().await;
                let window = self.app_handle.get_window("main")
                    .unwrap_or_else(|| panic!("main window not found for task executor"));
                let file_path = compression_task.source_files
                    .first()
                    .cloned()
                    .unwrap_or_default();
                let output_dir = if compression_task.output_path.is_empty() {
                    None
                } else {
                    Some(compression_task.output_path.clone())
                };
                let password = compression_task.password.clone();
                let options = DecompressOptions::default();
                service.extract(
                    window,
                    task_id.clone(),
                    file_path,
                    output_dir,
                    password,
                    options,
                ).await.map(|_| ())
            }
        };

        // 更新任务状态
        {
            let mut task = task_ref.write().await;
            task.completed_at = Some(chrono::Utc::now());
            match &result {
                Ok(_) => {
                    task.status = QueueTaskStatus::Completed;
                }
                Err(e) => {
                    task.status = QueueTaskStatus::Failed;
                    task.error_message = Some(e.to_string());
                }
            }
        }

        // 通知调度器任务已完成
        self.scheduler.on_task_completed().await;
    }
}
