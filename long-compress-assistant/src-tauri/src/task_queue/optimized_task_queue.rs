use crate::task_queue::models::{QueueTask, QueueTaskStatus, SharedQueueTask};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

pub struct OptimizedTaskQueue {
    tasks: Arc<RwLock<HashMap<String, SharedQueueTask>>>,
}

impl OptimizedTaskQueue {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_statistics(&self) -> HashMap<String, usize> {
        let tasks = self.tasks.read().await;
        let mut stats = HashMap::new();
        stats.insert("total".to_string(), tasks.len());
        stats
    }
}
