use crate::task_queue::models::{QueueTask, QueueTaskStatus, SharedQueueTask};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{Result, anyhow};

pub struct TaskQueue {
    pub tasks: RwLock<HashMap<String, SharedQueueTask>>,
    pub max_size: usize,
}

impl TaskQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            max_size,
        }
    }

    pub async fn add_task(&self, task: QueueTask) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        if tasks.len() >= self.max_size {
            return Err(anyhow!("队列已满"));
        }
        let id = task.id.clone();
        tasks.insert(id, Arc::new(RwLock::new(task)));
        Ok(())
    }

    pub async fn get_task(&self, task_id: &str) -> Option<SharedQueueTask> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned()
    }

    pub async fn update_task_status(&self, task_id: &str, status: QueueTaskStatus) -> Result<()> {
        if let Some(task_ref) = self.get_task(task_id).await {
            let mut task = task_ref.write().await;
            task.status = status;
            Ok(())
        } else {
            Err(anyhow!("任务未找到"))
        }
    }

    pub async fn get_next_task(&self) -> Option<SharedQueueTask> {
        let tasks = self.tasks.read().await;
        for task_ref in tasks.values() {
            let task = task_ref.read().await;
            if task.status == QueueTaskStatus::Pending {
                return Some(task_ref.clone());
            }
        }
        None
    }

    pub async fn get_all_tasks(&self) -> HashMap<String, SharedQueueTask> {
        let tasks = self.tasks.read().await;
        tasks.clone()
    }
}
