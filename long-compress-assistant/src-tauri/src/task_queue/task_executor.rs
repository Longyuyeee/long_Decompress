use crate::task_queue::models::{QueueTask, TaskType, QueueTaskStatus};
use crate::task_queue::task_scheduler::TaskScheduler;
use crate::services::compression_service::CompressionService;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{Result, anyhow};
use tokio::task::JoinHandle;
use std::time::Duration;

/// 任务执行器配置
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub timeout_seconds: u64,
    pub enable_progress_tracking: bool,
    pub progress_update_interval_ms: u64,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_ms: 1000,
            timeout_seconds: 3600, // 1小时
            enable_progress_tracking: true,
            progress_update_interval_ms: 500, // 500毫秒
        }
    }
}

/// 任务执行器
pub struct TaskExecutor {
    scheduler: Arc<TaskScheduler>,
    compression_service: CompressionService,
    config: ExecutorConfig,
    task_handles: Arc<RwLock<Vec<JoinHandle<()>>>>,
}

impl TaskExecutor {
    /// 创建新的任务执行器
    pub fn new(scheduler: Arc<TaskScheduler>, config: ExecutorConfig) -> Self {
        Self {
            scheduler,
            compression_service: CompressionService,
            config,
            task_handles: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 启动任务执行器
    pub async fn start(&self) -> Result<()> {
        log::info!("任务执行器启动");

        // 启动任务执行循环
        self.start_execution_loop().await?;

        Ok(())
    }

    /// 启动任务执行循环
    async fn start_execution_loop(&self) -> Result<()> {
        let scheduler = self.scheduler.clone();
        let compression_service = self.compression_service.clone();
        let config = self.config.clone();
        let task_handles = self.task_handles.clone();

        tokio::spawn(async move {
            loop {
                // 从调度器获取下一个可执行任务
                if let Some(task_ref) = scheduler.get_next_executable_task().await {
                    let task_id = {
                        let task = task_ref.read().await;
                        task.id.clone()
                    };

                    log::debug!("开始执行任务: {}", task_id);

                    // 创建任务执行句柄
                    let handle = tokio::spawn({
                        let task_ref = task_ref.clone();
                        let scheduler = scheduler.clone();
                        let compression_service = compression_service.clone();
                        let config = config.clone();

                        async move {
                            // 执行任务
                            if let Err(e) = Self::execute_task(task_ref, scheduler, compression_service, config).await {
                                log::error!("任务执行失败: {}", e);
                            }
                        }
                    });

                    // 保存任务句柄
                    {
                        let mut handles = task_handles.write().await;
                        handles.push(handle);
                    }
                }

                // 清理已完成的任务句柄
                Self::cleanup_completed_handles(&task_handles).await;

                // 短暂休眠避免CPU占用过高
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });

        Ok(())
    }

    /// 执行单个任务
    async fn execute_task(
        task_ref: Arc<RwLock<QueueTask>>,
        scheduler: Arc<TaskScheduler>,
        compression_service: CompressionService,
        config: ExecutorConfig,
    ) -> Result<()> {
        let task_id = {
            let task = task_ref.read().await;
            task.id.clone()
        };

        // 更新任务状态为Running
        {
            let mut task = task_ref.write().await;
            task.start();
        }

        let mut retry_count = 0;
        let mut last_error = None;

        while retry_count <= config.max_retries {
            // 如果是重试，等待一段时间
            if retry_count > 0 {
                let delay = Duration::from_millis(config.retry_delay_ms * 2u64.pow(retry_count - 1));
                log::info!("任务 {} 第 {} 次重试，等待 {:?}", task_id, retry_count, delay);
                tokio::time::sleep(delay).await;
            }

            // 执行任务
            let result = Self::execute_single_attempt(
                task_ref.clone(),
                scheduler.clone(),
                compression_service.clone(),
                config.clone(),
            ).await;

            match result {
                Ok(_) => {
                    // 任务成功完成
                    scheduler.on_task_completed(&task_id, true, None).await;
                    log::info!("任务 {} 执行成功", task_id);
                    return Ok(());
                }
                Err(e) => {
                    last_error = Some(e);
                    retry_count += 1;

                    // 更新重试计数
                    {
                        let mut task = task_ref.write().await;
                        task.increment_retry();
                    }

                    // 检查是否还可以重试
                    {
                        let task = task_ref.read().await;
                        if !task.can_retry() {
                            break;
                        }
                    }
                }
            }
        }

        // 任务失败
        let error_message = last_error.map(|e| e.to_string()).unwrap_or_else(|| "未知错误".to_string());
        scheduler.on_task_completed(&task_id, false, Some(error_message.clone())).await;
        log::error!("任务 {} 执行失败: {}", task_id, error_message);

        Err(anyhow!("任务执行失败: {}", error_message))
    }

    /// 执行单次任务尝试
    async fn execute_single_attempt(
        task_ref: Arc<RwLock<QueueTask>>,
        scheduler: Arc<TaskScheduler>,
        compression_service: CompressionService,
        config: ExecutorConfig,
    ) -> Result<()> {
        let task_id = {
            let task = task_ref.read().await;
            task.id.clone()
        };

        log::debug!("开始执行任务尝试: {}", task_id);

        // 获取任务数据
        let (task_type, source_files, output_path, password, format) = {
            let task = task_ref.read().await;
            let compression_task = &task.compression_task;

            (
                task.task_type.clone(),
                compression_task.source_files.clone(),
                compression_task.output_path.clone(),
                compression_task.options.password.clone(),
                compression_task.format.clone(),
            )
        };

        // 根据任务类型执行不同的操作
        match task_type {
            TaskType::Compress => {
                // 压缩任务
                let options = {
                    let task = task_ref.read().await;
                    task.compression_task.options.clone()
                };

                // 启动进度跟踪（如果启用）
                let progress_handle = if config.enable_progress_tracking {
                    Some(Self::start_progress_tracking(task_ref.clone(), config.progress_update_interval_ms))
                } else {
                    None
                };

                // 执行压缩
                let result = compression_service.compress(
                    &source_files,
                    &output_path,
                    options,
                ).await;

                // 停止进度跟踪
                if let Some(handle) = progress_handle {
                    handle.abort();
                }

                result.map_err(|e| anyhow!("压缩失败: {}", e))
            }
            TaskType::Extract => {
                // 解压任务
                // 启动进度跟踪（如果启用）
                let progress_handle = if config.enable_progress_tracking {
                    Some(Self::start_progress_tracking(task_ref.clone(), config.progress_update_interval_ms))
                } else {
                    None
                };

                // 执行解压
                let result = compression_service.extract(
                    &source_files[0], // 解压通常只有一个源文件
                    None, // 使用默认输出目录
                    password.as_deref(),
                ).await;

                // 停止进度跟踪
                if let Some(handle) = progress_handle {
                    handle.abort();
                }

                result.map(|_| ()).map_err(|e| anyhow!("解压失败: {}", e))
            }
        }
    }

    /// 启动进度跟踪
    fn start_progress_tracking(
        task_ref: Arc<RwLock<QueueTask>>,
        update_interval_ms: u64,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(update_interval_ms));

            loop {
                interval.tick().await;

                // 模拟进度更新（实际实现中需要从压缩服务获取真实进度）
                {
                    let mut task = task_ref.write().await;
                    if task.status == QueueTaskStatus::Running {
                        // 这里模拟进度更新，实际应该从压缩服务获取
                        let current_progress = task.compression_task.progress;
                        let new_progress = (current_progress + 1.0).min(99.0); // 模拟进度增加
                        task.update_progress(
                            (task.compression_task.total_size as f32 * new_progress / 100.0) as u64,
                            task.compression_task.total_size,
                        );
                    } else {
                        // 任务不再运行，停止进度跟踪
                        break;
                    }
                }
            }
        })
    }

    /// 清理已完成的任务句柄
    async fn cleanup_completed_handles(task_handles: &Arc<RwLock<Vec<JoinHandle<()>>>>) {
        let mut handles = task_handles.write().await;
        handles.retain(|handle| !handle.is_finished());
    }

    /// 获取当前执行中的任务数
    pub async fn get_running_task_count(&self) -> usize {
        let handles = self.task_handles.read().await;
        handles.len()
    }

    /// 停止所有任务
    pub async fn stop_all_tasks(&self) -> Result<()> {
        let mut handles = self.task_handles.write().await;

        for handle in handles.iter() {
            handle.abort();
        }

        handles.clear();
        log::info!("已停止所有任务");

        Ok(())
    }

    /// 获取执行器状态
    pub async fn get_executor_status(&self) -> ExecutorStatus {
        let running_count = self.get_running_task_count().await;

        ExecutorStatus {
            max_retries: self.config.max_retries,
            retry_delay_ms: self.config.retry_delay_ms,
            timeout_seconds: self.config.timeout_seconds,
            enable_progress_tracking: self.config.enable_progress_tracking,
            progress_update_interval_ms: self.config.progress_update_interval_ms,
            running_tasks: running_count,
        }
    }
}

/// 执行器状态
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecutorStatus {
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub timeout_seconds: u64,
    pub enable_progress_tracking: bool,
    pub progress_update_interval_ms: u64,
    pub running_tasks: usize,
}

/// 任务执行上下文
pub struct TaskExecutionContext {
    pub task_id: String,
    pub start_time: std::time::Instant,
    pub timeout: Duration,
    pub cancelled: bool,
}

impl TaskExecutionContext {
    pub fn new(task_id: String, timeout_seconds: u64) -> Self {
        Self {
            task_id,
            start_time: std::time::Instant::now(),
            timeout: Duration::from_secs(timeout_seconds),
            cancelled: false,
        }
    }

    pub fn check_timeout(&self) -> Result<()> {
        if self.start_time.elapsed() > self.timeout {
            Err(anyhow!("任务执行超时"))
        } else {
            Ok(())
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_queue::task_queue::TaskQueue;
    use crate::task_queue::task_scheduler::{TaskScheduler, SchedulerConfig};
    use crate::models::compression::{CompressionTask, CompressionFormat, CompressionOptions};

    fn create_test_scheduler() -> Arc<TaskScheduler> {
        let queue = Arc::new(TaskQueue::new(100));
        let config = SchedulerConfig::default();
        Arc::new(TaskScheduler::new(queue, config))
    }

    fn create_test_task() -> QueueTask {
        let compression_task = CompressionTask::new(
            vec!["test.txt".to_string()],
            "output.zip".to_string(),
            CompressionFormat::Zip,
            CompressionOptions::default(),
        );

        QueueTask::new(
            TaskType::Compress,
            crate::task_queue::models::TaskPriority::Medium,
            compression_task,
        )
    }

    #[tokio::test]
    async fn test_executor_initialization() {
        let scheduler = create_test_scheduler();
        let config = ExecutorConfig::default();

        let executor = TaskExecutor::new(scheduler, config);

        let status = executor.get_executor_status().await;
        assert_eq!(status.max_retries, 3);
        assert_eq!(status.running_tasks, 0);
    }

    #[test]
    fn test_task_execution_context() {
        let context = TaskExecutionContext::new("test-task".to_string(), 10);

        assert_eq!(context.task_id, "test-task");
        assert!(!context.is_cancelled());
        assert!(context.check_timeout().is_ok());

        // 测试取消
        let mut context2 = TaskExecutionContext::new("test-task2".to_string(), 10);
        assert!(!context2.is_cancelled());
        context2.cancel();
        assert!(context2.is_cancelled());
    }

    #[tokio::test]
    async fn test_executor_stop_all_tasks() {
        let scheduler = create_test_scheduler();
        let config = ExecutorConfig::default();

        let executor = TaskExecutor::new(scheduler, config);

        // 初始状态
        let initial_count = executor.get_running_task_count().await;
        assert_eq!(initial_count, 0);

        // 停止所有任务（应该成功，即使没有任务）
        let result = executor.stop_all_tasks().await;
        assert!(result.is_ok());
    }
}