use crate::task_queue::models::{QueueTask, TaskPriority, SharedQueueTask, TaskType};
use crate::models::compression::CompressionTask;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct QueueBenchmark;

impl QueueBenchmark {
    pub fn create_test_task(priority: TaskPriority) -> QueueTask {
        QueueTask::new(TaskType::Compression, priority, CompressionTask::default())
    }

    pub async fn run_benchmark() {
        let priority = TaskPriority::Medium;
        let _task = Self::create_test_task(priority.clone());
    }
}
