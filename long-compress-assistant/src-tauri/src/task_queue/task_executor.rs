use crate::task_queue::models::SharedQueueTask;
use crate::task_queue::task_scheduler::TaskScheduler;
use crate::services::compression_service::CompressionService;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

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
    compression_service: CompressionService,
}

impl TaskExecutor {
    pub fn new(scheduler: Arc<TaskScheduler>, config: ExecutorConfig) -> Self {
        Self {
            scheduler,
            config,
            compression_service: CompressionService::default(),
        }
    }

    pub async fn execute_task(&self, task_ref: SharedQueueTask) {
        let _task_id = {
            let task = task_ref.read().await;
            task.id.clone()
        };

        // 模拟执行
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        self.scheduler.on_task_completed().await;
    }
}
