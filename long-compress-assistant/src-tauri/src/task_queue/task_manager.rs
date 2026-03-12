use crate::task_queue::models::{QueueTask, QueueTaskStatus, TaskPriority, TaskType};
use crate::task_queue::task_queue::TaskQueue;
use crate::task_queue::task_scheduler::{TaskScheduler, SchedulerConfig};
use crate::task_queue::task_executor::{TaskExecutor, ExecutorConfig};
use crate::task_queue::batch_task_processor::BatchTaskProcessor;
use crate::task_queue::task_persistence::TaskPersistenceManager;
use crate::models::compression::{CompressionFormat, CompressionTask};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{Result, Context};

pub struct QueueConfig {
    pub max_queue_size: usize,
    pub max_concurrent_tasks: usize,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 1000,
            max_concurrent_tasks: 4,
        }
    }
}

pub struct TaskManager {
    queue: Arc<TaskQueue>,
    scheduler: Arc<TaskScheduler>,
    executor: Arc<TaskExecutor>,
    batch_processor: RwLock<Option<Arc<BatchTaskProcessor>>>,
    persistence_manager: Arc<RwLock<Option<TaskPersistenceManager>>>,
    config: Arc<RwLock<QueueConfig>>,
    app_handle: Option<tauri::AppHandle>,
}

impl TaskManager {
    pub fn new() -> Self {
        let config = QueueConfig::default();
        let queue = Arc::new(TaskQueue::new(config.max_queue_size));
        let scheduler = Arc::new(TaskScheduler::new(queue.clone(), SchedulerConfig::default()));
        let executor = Arc::new(TaskExecutor::new(scheduler.clone(), ExecutorConfig::default()));

        Self {
            queue,
            scheduler,
            executor,
            batch_processor: RwLock::new(None),
            persistence_manager: Arc::new(RwLock::new(None)),
            config: Arc::new(RwLock::new(config)),
            app_handle: None,
        }
    }

    pub fn set_app_handle(&mut self, app_handle: tauri::AppHandle) {
        self.app_handle = Some(app_handle);
    }

    pub async fn initialize(&self) -> Result<()> {
        Ok(())
    }

    pub async fn add_compression_task(&self, task: CompressionTask, priority: TaskPriority) -> Result<String> {
        let queue_task = QueueTask::new(TaskType::Compression, priority, task);
        let task_id = queue_task.id.clone();
        self.queue.add_task(queue_task).await?;
        Ok(task_id)
    }

    pub async fn add_extraction_task(&self, file_path: String, output_dir: Option<String>, password: Option<String>, priority: TaskPriority) -> Result<String> {
        let mut task = CompressionTask::default();
        task.source_files = vec![file_path];
        task.output_path = output_dir.unwrap_or_default();
        task.password = password;
        task.format = CompressionFormat::Zip;
        
        self.add_compression_task(task, priority).await
    }

    pub async fn cancel_task(&self, task_id: &str) -> Result<()> {
        self.queue.update_task_status(task_id, QueueTaskStatus::Cancelled).await
    }

    pub async fn get_task_status(&self, task_id: &str) -> Result<QueueTaskStatus> {
        let task_ref = self.queue.get_task(task_id).await.context("任务不存在")?;
        let task = task_ref.read().await;
        Ok(task.status.clone())
    }

    pub async fn get_all_tasks(&self) -> Result<Vec<QueueTask>> {
        let tasks_map = self.queue.get_all_tasks().await;
        let mut tasks = Vec::new();
        for task_ref in tasks_map.values() {
            tasks.push(task_ref.read().await.clone());
        }
        Ok(tasks)
    }
}

lazy_static::lazy_static! {
    pub static ref TASK_MANAGER: Arc<TaskManager> = Arc::new(TaskManager::new());
}
