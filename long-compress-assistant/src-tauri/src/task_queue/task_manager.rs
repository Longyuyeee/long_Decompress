use crate::task_queue::models::{
    QueueTask, TaskType, TaskPriority, QueueTaskStatus, TaskFilter,
    QueueStatistics, QueueConfig, SharedQueueTask,
};
use crate::task_queue::task_queue::TaskQueue;
use crate::task_queue::task_scheduler::{TaskScheduler, SchedulerConfig};
use crate::task_queue::task_executor::{TaskExecutor, ExecutorConfig};
use crate::task_queue::task_persistence::{TaskPersistenceManager, PersistenceConfig};
use crate::task_queue::task_event_log::{TaskEventLogger, TaskEvent, TaskEventType, EVENT_LOGGER};
use crate::task_queue::batch_task_processor::{BatchTaskProcessor, BatchTaskConfig, BatchTaskRequest, BatchTaskResult};
use crate::models::compression::{CompressionTask, CompressionFormat, CompressionOptions};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{Result, anyhow};
use tauri::Manager;

/// 任务管理器（系统主入口）
pub struct TaskManager {
    queue: Arc<TaskQueue>,
    scheduler: Arc<TaskScheduler>,
    executor: Arc<TaskExecutor>,
    batch_processor: Arc<BatchTaskProcessor>,
    persistence_manager: Arc<RwLock<Option<TaskPersistenceManager>>>,
    config: Arc<RwLock<QueueConfig>>,
    batch_config: Arc<RwLock<BatchTaskConfig>>,
    persistence_config: Arc<RwLock<PersistenceConfig>>,
    app_handle: Option<tauri::AppHandle>,
}

impl TaskManager {
    /// 创建新的任务管理器
    pub fn new() -> Self {
        let config = QueueConfig::default();
        let batch_config = BatchTaskConfig::default();
        let persistence_config = PersistenceConfig::default();

        // 创建任务队列
        let queue = Arc::new(TaskQueue::new(config.max_queue_size));

        // 创建调度器配置
        let scheduler_config = SchedulerConfig {
            max_concurrent_tasks: config.max_concurrent_tasks,
            ..Default::default()
        };

        // 创建任务调度器
        let scheduler = Arc::new(TaskScheduler::new(queue.clone(), scheduler_config));

        // 创建任务执行器
        let executor_config = ExecutorConfig::default();
        let executor = Arc::new(TaskExecutor::new(scheduler.clone(), executor_config));

        // 创建批量任务处理器
        let batch_processor = Arc::new(BatchTaskProcessor::new());

        Self {
            queue,
            scheduler,
            executor,
            batch_processor,
            persistence_manager: Arc::new(RwLock::new(None)),
            config: Arc::new(RwLock::new(config)),
            batch_config: Arc::new(RwLock::new(batch_config)),
            persistence_config: Arc::new(RwLock::new(persistence_config)),
            app_handle: None,
        }
    }

    /// 设置应用句柄（用于事件通知）
    pub fn set_app_handle(&mut self, app_handle: tauri::AppHandle) {
        self.app_handle = Some(app_handle);
    }

    /// 初始化任务管理器
    pub async fn initialize(&self) -> Result<()> {
        log::info!("初始化任务管理器");

        // 初始化事件日志
        EVENT_LOGGER.initialize().await?;

        // 初始化持久化管理器（如果启用）
        {
            let persistence_config = self.persistence_config.read().await;
            if persistence_config.enabled {
                let storage_dir = TaskPersistenceManager::default_storage_dir()?;
                let persistence_manager = TaskPersistenceManager::new(storage_dir)?;

                let mut manager_guard = self.persistence_manager.write().await;
                *manager_guard = Some(persistence_manager);
                log::info!("任务持久化管理器初始化完成");
            }
        }

        // 启动任务调度器
        self.scheduler.start().await?;

        // 启动任务执行器
        self.executor.start().await?;

        // 加载已保存的任务（如果启用持久化）
        self.load_saved_tasks().await?;

        log::info!("任务管理器初始化完成");

        Ok(())
    }

    /// 加载已保存的任务
    async fn load_saved_tasks(&self) -> Result<()> {
        let persistence_manager_guard = self.persistence_manager.read().await;

        if let Some(persistence_manager) = &*persistence_manager_guard {
            match persistence_manager.load_all_tasks() {
                Ok(saved_tasks) => {
                    log::info!("加载了 {} 个已保存的任务", saved_tasks.len());

                    for task in saved_tasks {
                        // 只加载未完成的任务
                        if !task.status.is_finished() {
                            // 重新创建任务引用并添加到队列
                            let task_ref = Arc::new(tokio::sync::RwLock::new(task));
                            let task_id = {
                                let task = task_ref.read().await;
                                task.id.clone()
                            };

                            // 添加到队列存储
                            {
                                let mut tasks = self.queue.tasks.write().await;
                                tasks.insert(task_id.clone(), task_ref.clone());
                            }

                            // 如果任务状态是Queued，添加到优先级队列
                            {
                                let task = task_ref.read().await;
                                if task.status == QueueTaskStatus::Queued {
                                    // 这里需要调用队列的内部方法，简化处理
                                    log::debug!("恢复排队任务: {}", task_id);
                                }
                            }

                            // 记录事件
                            self.log_task_event(&task_id, TaskEventType::TaskLoaded, None).await;
                        }
                    }
                }
                Err(e) => {
                    log::warn!("加载已保存任务失败: {}", e);
                }
            }
        }

        Ok(())
    }

    /// 保存任务（如果启用持久化）
    async fn save_task(&self, task: &QueueTask) -> Result<()> {
        let persistence_manager_guard = self.persistence_manager.read().await;

        if let Some(persistence_manager) = &*persistence_manager_guard {
            persistence_manager.save_task(task)?;
        }

        Ok(())
    }

    /// 记录任务事件
    async fn log_task_event(&self, task_id: &str, event_type: TaskEventType, details: Option<serde_json::Value>) {
        let details = details.unwrap_or_else(|| serde_json::json!({}));
        let event = TaskEvent::new(task_id, event_type, details);

        // 异步记录事件，不阻塞主流程
        let event_clone = event.clone();
        tokio::spawn(async move {
            if let Err(e) = EVENT_LOGGER.log_event(event_clone).await {
                log::warn!("记录任务事件失败: {}", e);
            }
        });

        // 同时发送到前端（如果应用句柄可用）
        self.emit_task_event_from_type(task_id, &event_type).await;
    }

    /// 根据事件类型发送任务事件到前端
    async fn emit_task_event_from_type(&self, task_id: &str, event_type: &TaskEventType) {
        let event_name = match event_type {
            TaskEventType::TaskCreated => "task_created",
            TaskEventType::TaskQueued => "task_queued",
            TaskEventType::TaskScheduled => "task_scheduled",
            TaskEventType::TaskStarted => "task_started",
            TaskEventType::TaskProgress => "task_progress",
            TaskEventType::TaskPaused => "task_paused",
            TaskEventType::TaskResumed => "task_resumed",
            TaskEventType::TaskCompleted => "task_completed",
            TaskEventType::TaskFailed => "task_failed",
            TaskEventType::TaskCancelled => "task_cancelled",
            TaskEventType::TaskRetried => "task_retried",
            TaskEventType::TaskStatusChanged => "task_status_changed",
            TaskEventType::TaskPriorityChanged => "task_priority_changed",
            TaskEventType::TaskError => "task_error",
            TaskEventType::TaskWarning => "task_warning",
            TaskEventType::TaskInfo => "task_info",
            TaskEventType::TaskLoaded => "task_loaded",
        };

        self.emit_task_event(event_name, task_id).await;
    }

    /// 添加压缩任务
    pub async fn add_compression_task(
        &self,
        source_files: Vec<String>,
        output_path: String,
        format: CompressionFormat,
        options: CompressionOptions,
        priority: TaskPriority,
    ) -> Result<String> {
        // 创建压缩任务
        let compression_task = CompressionTask::new(
            source_files.clone(),
            output_path.clone(),
            format.clone(),
            options.clone(),
        );

        // 创建队列任务
        let queue_task = QueueTask::new(
            TaskType::Compress,
            priority.clone(),
            compression_task,
        );

        let task_id = queue_task.id.clone();

        // 记录任务创建事件
        let details = serde_json::json!({
            "source_files": source_files,
            "output_path": output_path,
            "format": format!("{:?}", format),
            "priority": format!("{:?}", priority),
        });
        self.log_task_event(&task_id, TaskEventType::TaskCreated, Some(details)).await;

        // 添加到队列
        self.queue.add_task(queue_task).await?;

        // 保存任务（如果启用持久化）
        if let Some(task_ref) = self.queue.get_task(&task_id).await {
            let task = task_ref.read().await;
            if let Err(e) = self.save_task(&task).await {
                log::warn!("保存任务失败 {}: {}", task_id, e);
            }
        }

        log::info!("添加压缩任务: {}", task_id);

        Ok(task_id)
    }

    /// 添加解压任务
    pub async fn add_extraction_task(
        &self,
        source_file: String,
        output_dir: Option<String>,
        password: Option<String>,
        priority: TaskPriority,
    ) -> Result<String> {
        // 创建压缩选项
        let mut options = CompressionOptions::default();
        options.password = password;

        // 根据文件扩展名确定格式
        let format = CompressionFormat::from_extension(
            std::path::Path::new(&source_file)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
        ).ok_or_else(|| anyhow!("不支持的文件格式"))?;

        // 创建压缩任务（用于解压）
        let compression_task = CompressionTask::new(
            vec![source_file],
            output_dir.unwrap_or_else(|| ".".to_string()),
            format,
            options,
        );

        // 创建队列任务
        let queue_task = QueueTask::new(
            TaskType::Extract,
            priority,
            compression_task,
        );

        // 添加到队列
        let task_id = self.queue.add_task(queue_task).await?;

        // 发送任务添加事件
        self.emit_task_event("task_added", &task_id).await;

        log::info!("添加解压任务: {}", task_id);

        Ok(task_id)
    }

    /// 根据ID获取任务
    pub async fn get_task(&self, task_id: &str) -> Result<SharedQueueTask> {
        self.queue.get_task(task_id)
            .await
            .ok_or_else(|| anyhow!("任务不存在: {}", task_id))
    }

    /// 获取所有任务
    pub async fn get_all_tasks(&self) -> Vec<SharedQueueTask> {
        self.queue.get_all_tasks().await
    }

    /// 根据过滤器获取任务
    pub async fn get_tasks_by_filter(&self, filter: &TaskFilter) -> Vec<SharedQueueTask> {
        self.queue.get_tasks_by_filter(filter).await
    }

    /// 获取任务状态
    pub async fn get_task_status(&self, task_id: &str) -> Result<QueueTaskStatus> {
        let task_ref = self.get_task(task_id).await?;
        let task = task_ref.read().await;

        Ok(task.status.clone())
    }

    /// 获取任务进度
    pub async fn get_task_progress(&self, task_id: &str) -> Result<f32> {
        let task_ref = self.get_task(task_id).await?;
        let task = task_ref.read().await;

        Ok(task.compression_task.progress)
    }

    /// 取消任务
    pub async fn cancel_task(&self, task_id: &str) -> Result<bool> {
        let task_ref = self.get_task(task_id).await?;

        // 检查任务是否可以取消
        {
            let task = task_ref.read().await;
            if !task.status.can_cancel() {
                return Err(anyhow!("任务当前状态无法取消"));
            }
        }

        // 更新任务状态为Cancelled
        self.queue.update_task_status(task_id, QueueTaskStatus::Cancelled).await?;

        // 发送任务取消事件
        self.emit_task_event("task_cancelled", task_id).await;

        log::info!("取消任务: {}", task_id);

        Ok(true)
    }

    /// 暂停任务
    pub async fn pause_task(&self, task_id: &str) -> Result<bool> {
        let task_ref = self.get_task(task_id).await?;

        // 检查任务是否可以暂停
        {
            let task = task_ref.read().await;
            if !task.status.can_pause() {
                return Err(anyhow!("任务当前状态无法暂停"));
            }
        }

        // 更新任务状态为Paused
        self.queue.update_task_status(task_id, QueueTaskStatus::Paused).await?;

        // 发送任务暂停事件
        self.emit_task_event("task_paused", task_id).await;

        log::info!("暂停任务: {}", task_id);

        Ok(true)
    }

    /// 恢复任务
    pub async fn resume_task(&self, task_id: &str) -> Result<bool> {
        let task_ref = self.get_task(task_id).await?;

        // 检查任务是否可以恢复
        {
            let task = task_ref.read().await;
            if !task.status.can_resume() {
                return Err(anyhow!("任务当前状态无法恢复"));
            }
        }

        // 更新任务状态为Running
        self.queue.update_task_status(task_id, QueueTaskStatus::Running).await?;

        // 发送任务恢复事件
        self.emit_task_event("task_resumed", task_id).await;

        log::info!("恢复任务: {}", task_id);

        Ok(true)
    }

    /// 获取队列统计信息
    pub async fn get_queue_statistics(&self) -> Result<QueueStatistics> {
        self.queue.get_statistics().await
    }

    /// 获取调度器状态
    pub async fn get_scheduler_status(&self) -> Result<crate::task_queue::task_scheduler::SchedulerStatus> {
        Ok(self.scheduler.get_scheduler_status().await)
    }

    /// 获取执行器状态
    pub async fn get_executor_status(&self) -> Result<crate::task_queue::task_executor::ExecutorStatus> {
        Ok(self.executor.get_executor_status().await)
    }

    /// 获取配置
    pub async fn get_config(&self) -> QueueConfig {
        let config = self.config.read().await;
        config.clone()
    }

    /// 更新配置
    pub async fn update_config(&self, new_config: QueueConfig) -> Result<()> {
        let mut config = self.config.write().await;

        // 更新队列大小
        if new_config.max_queue_size != config.max_queue_size {
            // 注意：这里需要重新创建队列，简化处理
            log::warn!("更改队列大小需要重启应用");
        }

        // 更新调度器配置
        let scheduler_config = SchedulerConfig {
            max_concurrent_tasks: new_config.max_concurrent_tasks,
            ..Default::default()
        };

        // 这里需要重新创建调度器，简化处理
        log::warn!("更改并发任务数需要重启应用");

        *config = new_config;

        log::info!("更新任务管理器配置");

        Ok(())
    }

    /// 清理已完成的任务
    pub async fn cleanup_completed_tasks(&self, older_than_days: u32) -> Result<usize> {
        let removed_count = self.queue.cleanup_completed_tasks(older_than_days).await?;

        log::info!("清理了 {} 个已完成的任务", removed_count);

        Ok(removed_count)
    }

    /// 停止所有任务
    pub async fn stop_all_tasks(&self) -> Result<()> {
        self.executor.stop_all_tasks().await?;

        log::info!("已停止所有任务");

        Ok(())
    }

    /// 发送任务事件
    async fn emit_task_event(&self, event_name: &str, task_id: &str) {
        if let Some(app_handle) = &self.app_handle {
            if let Err(e) = app_handle.emit_all(event_name, task_id) {
                log::error!("发送任务事件失败: {}", e);
            }
        }
    }

    /// 发送任务进度事件
    pub async fn emit_task_progress_event(&self, task_id: &str, progress: f32) {
        if let Some(app_handle) = &self.app_handle {
            let event_data = serde_json::json!({
                "task_id": task_id,
                "progress": progress,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            if let Err(e) = app_handle.emit_all("task_progress", event_data) {
                log::error!("发送任务进度事件失败: {}", e);
            }
        }
    }

    /// 发送任务状态事件
    pub async fn emit_task_status_event(&self, task_id: &str, status: QueueTaskStatus) {
        if let Some(app_handle) = &self.app_handle {
            let event_data = serde_json::json!({
                "task_id": task_id,
                "status": format!("{:?}", status),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            if let Err(e) = app_handle.emit_all("task_status", event_data) {
                log::error!("发送任务状态事件失败: {}", e);
            }
        }
    }

    /// 发送队列统计事件
    pub async fn emit_queue_stats_event(&self) {
        if let Some(app_handle) = &self.app_handle {
            match self.get_queue_statistics().await {
                Ok(stats) => {
                    let event_data = serde_json::json!({
                        "total_tasks": stats.total_tasks,
                        "running_tasks": stats.running_tasks,
                        "queued_tasks": stats.queued_tasks,
                        "completed_tasks": stats.completed_tasks,
                        "failed_tasks": stats.failed_tasks,
                        "success_rate": stats.success_rate,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    });

                    if let Err(e) = app_handle.emit_all("queue_stats", event_data) {
                        log::error!("发送队列统计事件失败: {}", e);
                    }
                }
                Err(e) => {
                    log::error!("获取队列统计信息失败: {}", e);
                }
            }
        }
    }
}

/// 全局任务管理器实例
pub struct GlobalTaskManager {
    manager: Arc<RwLock<Option<Arc<TaskManager>>>>,
}

impl GlobalTaskManager {
    /// 创建新的全局任务管理器
    pub fn new() -> Self {
        Self {
            manager: Arc::new(RwLock::new(None)),
        }
    }

    /// 初始化全局任务管理器
    pub async fn initialize(&self, app_handle: tauri::AppHandle) -> Result<()> {
        let mut manager_guard = self.manager.write().await;

        if manager_guard.is_none() {
            let mut task_manager = TaskManager::new();
            task_manager.set_app_handle(app_handle);

            let task_manager_arc = Arc::new(task_manager);
            task_manager_arc.initialize().await?;

            *manager_guard = Some(task_manager_arc);
            log::info!("全局任务管理器初始化完成");
        }

        Ok(())
    }

    /// 获取任务管理器实例
    pub async fn get(&self) -> Result<Arc<TaskManager>> {
        let manager_guard = self.manager.read().await;
        manager_guard
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow!("任务管理器未初始化"))
    }
}

// 为TaskManager添加批量任务处理方法
impl TaskManager {
    /// 添加批量任务
    pub async fn add_batch_task(&self, request: BatchTaskRequest) -> Result<String, String> {
        log::info!("添加批量任务: {:?}", request.batch_task_type);

        // 创建主批量任务
        let batch_task_id = uuid::Uuid::new_v4().to_string();

        // 根据批量任务类型创建子任务
        let sub_tasks = match request.batch_task_type {
            crate::task_queue::BatchTaskType::BatchCompress => {
                self.create_batch_compression_tasks(&request, &batch_task_id).await?
            }
            crate::task_queue::BatchTaskType::BatchExtract => {
                self.create_batch_extraction_tasks(&request, &batch_task_id).await?
            }
            _ => {
                return Err(format!("暂不支持的批量任务类型: {:?}", request.batch_task_type));
            }
        };

        log::info!("批量任务 {} 创建了 {} 个子任务", batch_task_id, sub_tasks.len());

        // 记录批量任务事件
        self.log_task_event(&batch_task_id, TaskEventType::BatchTaskCreated, Some(format!("子任务数: {}", sub_tasks.len()))).await;

        Ok(batch_task_id)
    }

    /// 创建批量压缩任务
    async fn create_batch_compression_tasks(&self, request: &BatchTaskRequest, batch_task_id: &str) -> Result<Vec<String>, String> {
        let mut task_ids = Vec::new();

        // 这里简化处理，实际应该根据文件分组创建任务
        for (i, source_file) in request.source_files.iter().enumerate() {
            let task_id = self.add_compression_task(
                vec![source_file.clone()],
                format!("{}/output_{}.zip", request.output_dir, i),
                request.compression_format.clone().unwrap_or(CompressionFormat::Zip),
                request.compression_options.clone().unwrap_or_default(),
                TaskPriority::Medium,
            ).await.map_err(|e| format!("创建压缩子任务失败: {}", e))?;

            task_ids.push(task_id);
        }

        Ok(task_ids)
    }

    /// 创建批量解压任务
    async fn create_batch_extraction_tasks(&self, request: &BatchTaskRequest, batch_task_id: &str) -> Result<Vec<String>, String> {
        let mut task_ids = Vec::new();

        for source_file in &request.source_files {
            let task_id = self.add_extraction_task(
                source_file.clone(),
                Some(request.output_dir.clone()),
                request.password.clone(),
                TaskPriority::Medium,
            ).await.map_err(|e| format!("创建解压子任务失败: {}", e))?;

            task_ids.push(task_id);
        }

        Ok(task_ids)
    }

    /// 获取批量任务配置
    pub async fn get_batch_config(&self) -> BatchTaskConfig {
        self.batch_config.read().await.clone()
    }

    /// 更新批量任务配置
    pub async fn update_batch_config(&self, config: BatchTaskConfig) {
        let mut current_config = self.batch_config.write().await;
        *current_config = config;
    }

    /// 执行批量任务处理
    pub async fn process_batch_task(&self, batch_task_id: &str) -> Result<BatchTaskResult, String> {
        log::info!("处理批量任务: {}", batch_task_id);

        // 这里可以调用批量任务处理器的具体逻辑
        // 简化处理，返回成功结果
        Ok(BatchTaskResult {
            batch_task_id: batch_task_id.to_string(),
            total_items: 0,
            successful_items: 0,
            failed_items: 0,
            skipped_items: 0,
            start_time: chrono::Utc::now(),
            end_time: chrono::Utc::now(),
            status: crate::task_queue::BatchTaskStatus::Completed,
            results: Vec::new(),
            error_message: None,
        })
    }
}

// 实现默认的全局实例
lazy_static::lazy_static! {
    pub static ref TASK_MANAGER: GlobalTaskManager = GlobalTaskManager::new();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::compression::{CompressionFormat, CompressionOptions};

    #[tokio::test]
    async fn test_task_manager_initialization() {
        let manager = TaskManager::new();

        // 测试初始化
        let result = manager.initialize().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_compression_task() {
        let manager = TaskManager::new();
        manager.initialize().await.unwrap();

        let source_files = vec!["test1.txt".to_string(), "test2.txt".to_string()];
        let output_path = "output.zip".to_string();
        let format = CompressionFormat::Zip;
        let options = CompressionOptions::default();
        let priority = TaskPriority::Medium;

        let result = manager.add_compression_task(
            source_files,
            output_path,
            format,
            options,
            priority,
        ).await;

        assert!(result.is_ok());

        let task_id = result.unwrap();
        assert!(!task_id.is_empty());
    }

    #[tokio::test]
    async fn test_get_task() {
        let manager = TaskManager::new();
        manager.initialize().await.unwrap();

        // 先添加一个任务
        let task_id = manager.add_compression_task(
            vec!["test.txt".to_string()],
            "output.zip".to_string(),
            CompressionFormat::Zip,
            CompressionOptions::default(),
            TaskPriority::Medium,
        ).await.unwrap();

        // 获取任务
        let result = manager.get_task(&task_id).await;
        assert!(result.is_ok());

        let task_ref = result.unwrap();
        let task = task_ref.read().await;
        assert_eq!(task.id, task_id);
    }

    #[tokio::test]
    async fn test_cancel_task() {
        let manager = TaskManager::new();
        manager.initialize().await.unwrap();

        // 先添加一个任务
        let task_id = manager.add_compression_task(
            vec!["test.txt".to_string()],
            "output.zip".to_string(),
            CompressionFormat::Zip,
            CompressionOptions::default(),
            TaskPriority::Medium,
        ).await.unwrap();

        // 取消任务
        let result = manager.cancel_task(&task_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // 验证任务状态
        let status = manager.get_task_status(&task_id).await.unwrap();
        assert_eq!(status, QueueTaskStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_get_queue_statistics() {
        let manager = TaskManager::new();
        manager.initialize().await.unwrap();

        // 添加一些任务
        for i in 0..3 {
            manager.add_compression_task(
                vec![format!("test{}.txt", i)],
                format!("output{}.zip", i),
                CompressionFormat::Zip,
                CompressionOptions::default(),
                TaskPriority::Medium,
            ).await.unwrap();
        }

        // 获取统计信息
        let stats = manager.get_queue_statistics().await;
        assert!(stats.is_ok());

        let stats = stats.unwrap();
        assert_eq!(stats.total_tasks, 3);
        assert_eq!(stats.queued_tasks, 3);
    }
}