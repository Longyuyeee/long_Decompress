use crate::models::compression::CompressionTask;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low = 0,
    Medium = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueueTaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl QueueTaskStatus {
    pub fn is_finished(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    Compression,
    Extraction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueTask {
    pub id: String,
    pub task_type: TaskType,
    pub compression_task: CompressionTask,
    pub status: QueueTaskStatus,
    pub priority: TaskPriority,
    pub created_at: DateTime<Utc>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub error_message: Option<String>,
}

impl QueueTask {
    pub fn new(task_type: TaskType, priority: TaskPriority, compression_task: CompressionTask) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            task_type,
            compression_task,
            status: QueueTaskStatus::Pending,
            priority,
            created_at: Utc::now(),
            scheduled_at: None,
            started_at: None,
            completed_at: None,
            retry_count: 0,
            max_retries: 3,
            error_message: None,
        }
    }
}

pub type SharedQueueTask = Arc<RwLock<QueueTask>>;

pub struct TaskFilter {
    pub status: Option<QueueTaskStatus>,
}
