use crate::models::compression::{CompressionTask, CompressionStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 任务类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskType {
    Compress,
    Extract,
}

/// 任务优先级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low = 0,
    Medium = 1,
    High = 2,
}

/// 队列任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueueTaskStatus {
    Queued,      // 已排队
    Scheduled,   // 已调度
    Running,     // 执行中
    Paused,      // 已暂停
    Completed,   // 已完成
    Failed,      // 已失败
    Cancelled,   // 已取消
}

impl QueueTaskStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Running | Self::Scheduled)
    }

    pub fn is_finished(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    pub fn can_cancel(&self) -> bool {
        matches!(self, Self::Queued | Self::Scheduled | Self::Running | Self::Paused)
    }

    pub fn can_pause(&self) -> bool {
        matches!(self, Self::Running)
    }

    pub fn can_resume(&self) -> bool {
        matches!(self, Self::Paused)
    }
}

/// 队列任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueTask {
    pub id: String,
    pub task_type: TaskType,
    pub priority: TaskPriority,
    pub status: QueueTaskStatus,
    pub compression_task: CompressionTask,
    pub created_at: DateTime<Utc>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub error_message: Option<String>,
}

impl QueueTask {
    pub fn new(
        task_type: TaskType,
        priority: TaskPriority,
        compression_task: CompressionTask,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            task_type,
            priority,
            status: QueueTaskStatus::Queued,
            compression_task,
            created_at: Utc::now(),
            scheduled_at: None,
            started_at: None,
            completed_at: None,
            retry_count: 0,
            max_retries: 3,
            error_message: None,
        }
    }

    pub fn schedule(&mut self) {
        self.status = QueueTaskStatus::Scheduled;
        self.scheduled_at = Some(Utc::now());
    }

    pub fn start(&mut self) {
        self.status = QueueTaskStatus::Running;
        self.started_at = Some(Utc::now());
        self.compression_task.start();
    }

    pub fn pause(&mut self) {
        if self.status == QueueTaskStatus::Running {
            self.status = QueueTaskStatus::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.status == QueueTaskStatus::Paused {
            self.status = QueueTaskStatus::Running;
        }
    }

    pub fn complete(&mut self, success: bool, error_message: Option<String>) {
        if success {
            self.status = QueueTaskStatus::Completed;
            self.compression_task.complete(true, None);
        } else {
            self.status = QueueTaskStatus::Failed;
            self.error_message = error_message.clone();
            self.compression_task.complete(false, error_message);
        }
        self.completed_at = Some(Utc::now());
    }

    pub fn cancel(&mut self) {
        self.status = QueueTaskStatus::Cancelled;
        self.compression_task.cancel();
        self.completed_at = Some(Utc::now());
    }

    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries && !self.status.is_finished()
    }

    pub fn update_progress(&mut self, processed: u64, total: u64) {
        self.compression_task.update_progress(processed, total);
    }
}

/// 队列统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatistics {
    pub total_tasks: u32,
    pub queued_tasks: u32,
    pub scheduled_tasks: u32,
    pub running_tasks: u32,
    pub paused_tasks: u32,
    pub completed_tasks: u32,
    pub failed_tasks: u32,
    pub cancelled_tasks: u32,
    pub average_wait_time_seconds: f64,
    pub average_execution_time_seconds: f64,
    pub success_rate: f32,
}

impl Default for QueueStatistics {
    fn default() -> Self {
        Self {
            total_tasks: 0,
            queued_tasks: 0,
            scheduled_tasks: 0,
            running_tasks: 0,
            paused_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
            cancelled_tasks: 0,
            average_wait_time_seconds: 0.0,
            average_execution_time_seconds: 0.0,
            success_rate: 0.0,
        }
    }
}

/// 队列配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    pub max_concurrent_tasks: usize,
    pub max_queue_size: usize,
    pub default_priority: TaskPriority,
    pub enable_persistence: bool,
    pub persistence_interval_ms: u64,
    pub cleanup_days: u32,
    pub auto_retry_enabled: bool,
    pub max_retries: u32,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 3,
            max_queue_size: 1000,
            default_priority: TaskPriority::Medium,
            enable_persistence: true,
            persistence_interval_ms: 5000, // 5秒
            cleanup_days: 30,
            auto_retry_enabled: true,
            max_retries: 3,
        }
    }
}

/// 资源使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_usage: f32,
    pub memory_usage_mb: f64,
    pub disk_io_read_mb: f64,
    pub disk_io_write_mb: f64,
    pub network_io_mb: f64,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage_mb: 0.0,
            disk_io_read_mb: 0.0,
            disk_io_write_mb: 0.0,
            network_io_mb: 0.0,
        }
    }
}

/// 任务过滤器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFilter {
    pub status: Option<QueueTaskStatus>,
    pub task_type: Option<TaskType>,
    pub priority: Option<TaskPriority>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub search_text: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl Default for TaskFilter {
    fn default() -> Self {
        Self {
            status: None,
            task_type: None,
            priority: None,
            created_after: None,
            created_before: None,
            search_text: None,
            limit: Some(100),
            offset: Some(0),
        }
    }
}

/// 任务事件
#[derive(Debug, Clone, Serialize)]
pub struct TaskEvent {
    pub event_type: TaskEventType,
    pub task_id: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub enum TaskEventType {
    TaskAdded,
    TaskScheduled,
    TaskStarted,
    TaskProgress,
    TaskPaused,
    TaskResumed,
    TaskCompleted,
    TaskFailed,
    TaskCancelled,
    TaskRetried,
}

/// 共享的任务引用
pub type SharedQueueTask = Arc<tokio::sync::RwLock<QueueTask>>;