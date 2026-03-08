use crate::task_queue::models::{
    QueueTask, TaskType, TaskPriority, QueueTaskStatus, TaskFilter,
    QueueStatistics, QueueConfig, SharedQueueTask,
};
use crate::task_queue::task_queue::TaskQueue;
use crate::task_queue::task_scheduler::{TaskScheduler, SchedulerConfig};
use crate::task_queue::task_executor::{TaskExecutor, ExecutorConfig};
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
    config: Arc<RwLock<QueueConfig>>,
    app_handle: Option<tauri::AppHandle>,
}

impl TaskManager {
    /// 创建新的任务管理器
    pub fn new() -> Self {
        let config = QueueConfig::default();

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

        Self {
            queue,
            scheduler,
            executor,
            config: Arc::new(RwLock::new(config)),
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

        // 启动任务调度器
        self.scheduler.start().await?;

        // 启动任务执行器
        self.executor.start().await?;

        log::info!("任务管理器初始化完成");

        Ok(())
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
            source_files,
            output_path,
            format,
            options,
        );

        // 创建队列任务
        let queue_task = QueueTask::new(
            TaskType::Compress,
            priority,
            compression_task,
        );

        // 添加到队列
        let task_id = self.queue.add_task(queue_task).await?;

        // 发送任务添加事件
        self.emit_task_event("task_added", &task_id).await;

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

        log::info("已停止所有任务");

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