use crate::task_queue::models::{QueueTask, TaskPriority, QueueTaskStatus, TaskFilter};
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{Result, anyhow};

/// 任务队列项（用于优先级队列）
#[derive(Debug)]
struct QueueItem {
    priority: TaskPriority,
    created_at: chrono::DateTime<chrono::Utc>,
    task_id: String,
}

impl PartialEq for QueueItem {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.created_at == other.created_at
    }
}

impl Eq for QueueItem {}

impl PartialOrd for QueueItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueueItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // 先按优先级排序（高优先级在前）
        match self.priority.cmp(&other.priority).reverse() {
            std::cmp::Ordering::Equal => {
                // 同优先级按创建时间排序（先创建的在前面）
                self.created_at.cmp(&other.created_at)
            }
            other => other,
        }
    }
}

/// 任务队列
pub struct TaskQueue {
    // 优先级队列（用于调度）
    priority_queue: Arc<RwLock<BinaryHeap<QueueItem>>>,
    // 任务存储
    tasks: Arc<RwLock<HashMap<String, Arc<RwLock<QueueTask>>>>>,
    // 配置
    max_size: usize,
}

impl TaskQueue {
    /// 创建新的任务队列
    pub fn new(max_size: usize) -> Self {
        Self {
            priority_queue: Arc::new(RwLock::new(BinaryHeap::new())),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            max_size,
        }
    }

    /// 添加任务到队列
    pub async fn add_task(&self, task: QueueTask) -> Result<String> {
        let task_id = task.id.clone();

        // 检查队列是否已满
        let tasks = self.tasks.read().await;
        if tasks.len() >= self.max_size {
            return Err(anyhow!("任务队列已满，最大容量: {}", self.max_size));
        }
        drop(tasks);

        // 创建任务引用
        let task_ref = Arc::new(RwLock::new(task));

        // 添加到任务存储
        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(task_id.clone(), task_ref.clone());
        }

        // 如果任务状态是Queued，添加到优先级队列
        let task_guard = task_ref.read().await;
        if task_guard.status == QueueTaskStatus::Queued {
            let queue_item = QueueItem {
                priority: task_guard.priority.clone(),
                created_at: task_guard.created_at,
                task_id: task_id.clone(),
            };

            let mut priority_queue = self.priority_queue.write().await;
            priority_queue.push(queue_item);
        }

        Ok(task_id)
    }

    /// 获取下一个可执行任务
    pub async fn get_next_task(&self) -> Option<Arc<RwLock<QueueTask>>> {
        let mut priority_queue = self.priority_queue.write().await;

        while let Some(queue_item) = priority_queue.pop() {
            let tasks = self.tasks.read().await;
            if let Some(task_ref) = tasks.get(&queue_item.task_id) {
                let task = task_ref.read().await;

                // 检查任务是否仍然可执行
                if task.status == QueueTaskStatus::Queued {
                    drop(task);
                    drop(tasks);

                    // 返回任务引用
                    return Some(task_ref.clone());
                }
                // 如果任务状态不是Queued，继续检查下一个
            }
        }

        None
    }

    /// 根据ID获取任务
    pub async fn get_task(&self, task_id: &str) -> Option<Arc<RwLock<QueueTask>>> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned()
    }

    /// 更新任务状态
    pub async fn update_task_status(
        &self,
        task_id: &str,
        status: QueueTaskStatus,
    ) -> Result<()> {
        let task_ref = self.get_task(task_id).await
            .ok_or_else(|| anyhow!("任务不存在: {}", task_id))?;

        let mut task = task_ref.write().await;
        let old_status = task.status.clone();
        task.status = status.clone();

        // 如果状态从Queued变为其他状态，从优先级队列中移除
        if old_status == QueueTaskStatus::Queued && status != QueueTaskStatus::Queued {
            self.remove_from_priority_queue(task_id).await;
        }

        // 如果状态变为Queued，添加到优先级队列
        if old_status != QueueTaskStatus::Queued && status == QueueTaskStatus::Queued {
            let queue_item = QueueItem {
                priority: task.priority.clone(),
                created_at: task.created_at,
                task_id: task_id.to_string(),
            };

            let mut priority_queue = self.priority_queue.write().await;
            priority_queue.push(queue_item);
        }

        Ok(())
    }

    /// 从优先级队列中移除任务
    async fn remove_from_priority_queue(&self, task_id: &str) {
        let mut priority_queue = self.priority_queue.write().await;

        // 由于BinaryHeap不支持直接删除，需要重建队列
        let mut new_queue = BinaryHeap::new();

        while let Some(item) = priority_queue.pop() {
            if item.task_id != task_id {
                new_queue.push(item);
            }
        }

        *priority_queue = new_queue;
    }

    /// 获取所有任务
    pub async fn get_all_tasks(&self) -> Vec<Arc<RwLock<QueueTask>>> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }

    /// 根据过滤器获取任务
    pub async fn get_tasks_by_filter(&self, filter: &TaskFilter) -> Vec<Arc<RwLock<QueueTask>>> {
        let tasks = self.tasks.read().await;
        let mut result = Vec::new();

        for task_ref in tasks.values() {
            let task = task_ref.read().await;

            // 应用状态过滤
            if let Some(status_filter) = &filter.status {
                if &task.status != status_filter {
                    continue;
                }
            }

            // 应用任务类型过滤
            if let Some(task_type_filter) = &filter.task_type {
                if &task.task_type != task_type_filter {
                    continue;
                }
            }

            // 应用优先级过滤
            if let Some(priority_filter) = &filter.priority {
                if &task.priority != priority_filter {
                    continue;
                }
            }

            // 应用创建时间过滤
            if let Some(created_after) = &filter.created_after {
                if &task.created_at < created_after {
                    continue;
                }
            }

            if let Some(created_before) = &filter.created_before {
                if &task.created_at > created_before {
                    continue;
                }
            }

            // 应用文本搜索过滤
            if let Some(search_text) = &filter.search_text {
                let search_text_lower = search_text.to_lowercase();
                let mut found = false;

                // 搜索任务ID
                if task.id.to_lowercase().contains(&search_text_lower) {
                    found = true;
                }

                // 搜索输出路径
                if task.compression_task.output_path.to_lowercase().contains(&search_text_lower) {
                    found = true;
                }

                // 搜索源文件
                for source_file in &task.compression_task.source_files {
                    if source_file.to_lowercase().contains(&search_text_lower) {
                        found = true;
                        break;
                    }
                }

                // 搜索错误信息
                if let Some(error_msg) = &task.error_message {
                    if error_msg.to_lowercase().contains(&search_text_lower) {
                        found = true;
                    }
                }

                if !found {
                    continue;
                }
            }

            result.push(task_ref.clone());
        }

        // 应用分页
        let offset = filter.offset.unwrap_or(0) as usize;
        let limit = filter.limit.unwrap_or(100) as usize;

        // 按创建时间排序（最新的在前面）
        result.sort_by(|a, b| {
            let a_task = a.read().unwrap();
            let b_task = b.read().unwrap();
            b_task.created_at.cmp(&a_task.created_at)
        });

        result.into_iter()
            .skip(offset)
            .take(limit)
            .collect()
    }

    /// 删除任务
    pub async fn remove_task(&self, task_id: &str) -> Result<bool> {
        // 先从优先级队列中移除
        self.remove_from_priority_queue(task_id).await;

        // 从任务存储中移除
        let mut tasks = self.tasks.write().await;
        let removed = tasks.remove(task_id).is_some();

        Ok(removed)
    }

    /// 获取队列统计信息
    pub async fn get_statistics(&self) -> Result<crate::task_queue::models::QueueStatistics> {
        let tasks = self.tasks.read().await;

        let mut stats = crate::task_queue::models::QueueStatistics::default();
        stats.total_tasks = tasks.len() as u32;

        let mut total_wait_time = 0.0;
        let mut total_execution_time = 0.0;
        let mut completed_count = 0;
        let mut successful_count = 0;

        for task_ref in tasks.values() {
            let task = task_ref.read().await;

            // 统计各状态任务数量
            match task.status {
                QueueTaskStatus::Queued => stats.queued_tasks += 1,
                QueueTaskStatus::Scheduled => stats.scheduled_tasks += 1,
                QueueTaskStatus::Running => stats.running_tasks += 1,
                QueueTaskStatus::Paused => stats.paused_tasks += 1,
                QueueTaskStatus::Completed => {
                    stats.completed_tasks += 1;
                    completed_count += 1;
                    successful_count += 1;
                }
                QueueTaskStatus::Failed => {
                    stats.failed_tasks += 1;
                    completed_count += 1;
                }
                QueueTaskStatus::Cancelled => stats.cancelled_tasks += 1,
            }

            // 计算等待时间和执行时间
            if let (Some(started_at), Some(completed_at)) = (task.started_at, task.completed_at) {
                let execution_duration = completed_at.signed_duration_since(started_at);
                total_execution_time += execution_duration.num_milliseconds() as f64 / 1000.0;

                let wait_duration = started_at.signed_duration_since(task.created_at);
                total_wait_time += wait_duration.num_milliseconds() as f64 / 1000.0;
            } else if let Some(started_at) = task.started_at {
                let wait_duration = started_at.signed_duration_since(task.created_at);
                total_wait_time += wait_duration.num_milliseconds() as f64 / 1000.0;
            }
        }

        // 计算平均值
        if completed_count > 0 {
            stats.average_wait_time_seconds = total_wait_time / completed_count as f64;
            stats.average_execution_time_seconds = total_execution_time / completed_count as f64;
            stats.success_rate = (successful_count as f32 / completed_count as f32) * 100.0;
        }

        Ok(stats)
    }

    /// 获取队列大小
    pub async fn size(&self) -> usize {
        let tasks = self.tasks.read().await;
        tasks.len()
    }

    /// 检查队列是否为空
    pub async fn is_empty(&self) -> bool {
        let tasks = self.tasks.read().await;
        tasks.is_empty()
    }

    /// 清空队列
    pub async fn clear(&self) {
        let mut priority_queue = self.priority_queue.write().await;
        priority_queue.clear();

        let mut tasks = self.tasks.write().await;
        tasks.clear();
    }

    /// 清理已完成的任务
    pub async fn cleanup_completed_tasks(&self, older_than_days: u32) -> Result<usize> {
        let cutoff_time = chrono::Utc::now() - chrono::Duration::days(older_than_days as i64);
        let mut removed_count = 0;

        let task_ids: Vec<String> = {
            let tasks = self.tasks.read().await;
            tasks.values()
                .filter_map(|task_ref| {
                    let task = task_ref.read().unwrap();
                    if task.status.is_finished() && task.completed_at.map_or(false, |t| t < cutoff_time) {
                        Some(task.id.clone())
                    } else {
                        None
                    }
                })
                .collect()
        };

        for task_id in task_ids {
            if self.remove_task(&task_id).await? {
                removed_count += 1;
            }
        }

        Ok(removed_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::compression::{CompressionTask, CompressionFormat, CompressionOptions};

    fn create_test_task() -> QueueTask {
        let compression_task = CompressionTask::new(
            vec!["test.txt".to_string()],
            "output.zip".to_string(),
            CompressionFormat::Zip,
            CompressionOptions::default(),
        );

        QueueTask::new(
            TaskType::Compress,
            TaskPriority::Medium,
            compression_task,
        )
    }

    #[tokio::test]
    async fn test_add_and_get_task() {
        let queue = TaskQueue::new(100);
        let task = create_test_task();
        let task_id = task.id.clone();

        // 添加任务
        let result = queue.add_task(task).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), task_id);

        // 获取任务
        let task_ref = queue.get_task(&task_id).await;
        assert!(task_ref.is_some());

        let task = task_ref.unwrap().read().await;
        assert_eq!(task.id, task_id);
        assert_eq!(task.status, QueueTaskStatus::Queued);
    }

    #[tokio::test]
    async fn test_get_next_task() {
        let queue = TaskQueue::new(100);

        // 添加高优先级任务
        let mut high_priority_task = create_test_task();
        high_priority_task.priority = TaskPriority::High;
        let high_task_id = high_priority_task.id.clone();
        queue.add_task(high_priority_task).await.unwrap();

        // 添加低优先级任务
        let mut low_priority_task = create_test_task();
        low_priority_task.priority = TaskPriority::Low;
        queue.add_task(low_priority_task).await.unwrap();

        // 获取下一个任务应该是高优先级任务
        let next_task_ref = queue.get_next_task().await;
        assert!(next_task_ref.is_some());

        let next_task = next_task_ref.unwrap().read().await;
        assert_eq!(next_task.id, high_task_id);
        assert_eq!(next_task.priority, TaskPriority::High);
    }

    #[tokio::test]
    async fn test_update_task_status() {
        let queue = TaskQueue::new(100);
        let task = create_test_task();
        let task_id = task.id.clone();

        queue.add_task(task).await.unwrap();

        // 更新状态为Running
        queue.update_task_status(&task_id, QueueTaskStatus::Running).await.unwrap();

        let task_ref = queue.get_task(&task_id).await.unwrap();
        let task = task_ref.read().await;
        assert_eq!(task.status, QueueTaskStatus::Running);

        // 任务状态已更新，不应该再出现在优先级队列中
        let next_task = queue.get_next_task().await;
        assert!(next_task.is_none());
    }

    #[tokio::test]
    async fn test_remove_task() {
        let queue = TaskQueue::new(100);
        let task = create_test_task();
        let task_id = task.id.clone();

        queue.add_task(task).await.unwrap();

        // 移除任务
        let removed = queue.remove_task(&task_id).await.unwrap();
        assert!(removed);

        // 任务应该不存在了
        let task_ref = queue.get_task(&task_id).await;
        assert!(task_ref.is_none());
    }

    #[tokio::test]
    async fn test_queue_size_limit() {
        let queue = TaskQueue::new(2); // 限制为2个任务

        // 添加第一个任务
        let task1 = create_test_task();
        assert!(queue.add_task(task1).await.is_ok());

        // 添加第二个任务
        let task2 = create_test_task();
        assert!(queue.add_task(task2).await.is_ok());

        // 添加第三个任务应该失败
        let task3 = create_test_task();
        let result = queue.add_task(task3).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("任务队列已满"));
    }

    #[tokio::test]
    async fn test_get_statistics() {
        let queue = TaskQueue::new(100);

        // 添加一些任务
        for i in 0..5 {
            let mut task = create_test_task();
            if i % 2 == 0 {
                task.priority = TaskPriority::High;
            }
            queue.add_task(task).await.unwrap();
        }

        // 获取统计信息
        let stats = queue.get_statistics().await.unwrap();

        assert_eq!(stats.total_tasks, 5);
        assert_eq!(stats.queued_tasks, 5);
        assert_eq!(stats.running_tasks, 0);
        assert_eq!(stats.completed_tasks, 0);
    }
}