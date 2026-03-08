use crate::task_queue::models::{QueueTask, TaskPriority, QueueTaskStatus, TaskFilter};
use crossbeam::queue::SegQueue;
use dashmap::DashMap;
use std::sync::Arc;
use std::collections::BinaryHeap;
use std::cmp::Reverse;
use tokio::sync::RwLock;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};

/// 优化的任务队列项（用于优先级队列）
#[derive(Debug, Clone)]
struct OptimizedQueueItem {
    priority: TaskPriority,
    created_at: DateTime<Utc>,
    task_id: String,
}

impl PartialEq for OptimizedQueueItem {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.created_at == other.created_at
    }
}

impl Eq for OptimizedQueueItem {}

impl PartialOrd for OptimizedQueueItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OptimizedQueueItem {
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

/// 工作窃取队列
struct WorkStealingQueue {
    local_queue: SegQueue<String>, // 本地任务队列（无锁）
    global_queue: Arc<SegQueue<String>>, // 全局任务队列（用于工作窃取）
}

impl WorkStealingQueue {
    fn new(global_queue: Arc<SegQueue<String>>) -> Self {
        Self {
            local_queue: SegQueue::new(),
            global_queue,
        }
    }

    /// 推送任务到本地队列
    fn push_local(&self, task_id: String) {
        self.local_queue.push(task_id);
    }

    /// 从本地队列弹出任务
    fn pop_local(&self) -> Option<String> {
        self.local_queue.pop()
    }

    /// 尝试从其他队列窃取工作
    fn steal(&self) -> Option<String> {
        self.global_queue.pop()
    }

    /// 将任务发布到全局队列（供其他工作者窃取）
    fn publish_to_global(&self, task_id: String) {
        self.global_queue.push(task_id);
    }

    /// 获取队列大小
    fn len(&self) -> usize {
        self.local_queue.len()
    }

    /// 检查队列是否为空
    fn is_empty(&self) -> bool {
        self.local_queue.is_empty()
    }
}

/// 优化的任务队列
pub struct OptimizedTaskQueue {
    // 任务存储（使用DashMap实现并发安全的HashMap）
    tasks: Arc<DashMap<String, Arc<RwLock<QueueTask>>>>,

    // 优先级队列（使用多个队列减少锁竞争）
    high_priority_queue: Arc<SegQueue<String>>,
    medium_priority_queue: Arc<SegQueue<String>>,
    low_priority_queue: Arc<SegQueue<String>>,

    // 工作窃取队列集合
    work_stealing_queues: Arc<DashMap<usize, WorkStealingQueue>>,

    // 配置
    max_size: usize,
    worker_count: usize,
}

impl OptimizedTaskQueue {
    /// 创建新的优化任务队列
    pub fn new(max_size: usize, worker_count: usize) -> Self {
        // 创建全局队列
        let high_priority_queue = Arc::new(SegQueue::new());
        let medium_priority_queue = Arc::new(SegQueue::new());
        let low_priority_queue = Arc::new(SegQueue::new());

        // 创建工作窃取队列
        let work_stealing_queues = Arc::new(DashMap::new());
        let global_queue = Arc::new(SegQueue::new());

        for worker_id in 0..worker_count {
            work_stealing_queues.insert(worker_id, WorkStealingQueue::new(global_queue.clone()));
        }

        Self {
            tasks: Arc::new(DashMap::new()),
            high_priority_queue,
            medium_priority_queue,
            low_priority_queue,
            work_stealing_queues,
            max_size,
            worker_count,
        }
    }

    /// 根据优先级获取对应的队列
    fn get_priority_queue(&self, priority: &TaskPriority) -> &Arc<SegQueue<String>> {
        match priority {
            TaskPriority::High => &self.high_priority_queue,
            TaskPriority::Medium => &self.medium_priority_queue,
            TaskPriority::Low => &self.low_priority_queue,
        }
    }

    /// 添加任务到队列
    pub async fn add_task(&self, task: QueueTask) -> Result<String> {
        let task_id = task.id.clone();

        // 检查队列是否已满
        if self.tasks.len() >= self.max_size {
            return Err(anyhow!("任务队列已满，最大容量: {}", self.max_size));
        }

        // 创建任务引用
        let task_ref = Arc::new(RwLock::new(task));

        // 添加到任务存储
        self.tasks.insert(task_id.clone(), task_ref.clone());

        // 如果任务状态是Queued，添加到优先级队列
        let task_guard = task_ref.read().await;
        if task_guard.status == QueueTaskStatus::Queued {
            let priority_queue = self.get_priority_queue(&task_guard.priority);
            priority_queue.push(task_id.clone());

            // 同时添加到工作窃取队列
            if let Some(work_queue) = self.work_stealing_queues.get(&0) {
                work_queue.push_local(task_id.clone());
            }
        }

        Ok(task_id)
    }

    /// 获取下一个可执行任务（使用工作窃取）
    pub async fn get_next_task(&self, worker_id: usize) -> Option<Arc<RwLock<QueueTask>>> {
        // 1. 首先尝试从本地工作窃取队列获取
        if let Some(mut work_queue) = self.work_stealing_queues.get_mut(&worker_id) {
            if let Some(task_id) = work_queue.pop_local() {
                if let Some(task_ref) = self.tasks.get(&task_id) {
                    let task = task_ref.read().await;

                    // 检查任务是否仍然可执行
                    if task.status == QueueTaskStatus::Queued {
                        drop(task);
                        return Some(task_ref.clone());
                    }
                }
            }

            // 2. 尝试从全局优先级队列获取
            for priority_queue in [
                &self.high_priority_queue,
                &self.medium_priority_queue,
                &self.low_priority_queue,
            ] {
                while let Some(task_id) = priority_queue.pop() {
                    if let Some(task_ref) = self.tasks.get(&task_id) {
                        let task = task_ref.read().await;

                        if task.status == QueueTaskStatus::Queued {
                            drop(task);
                            return Some(task_ref.clone());
                        }
                    }
                }
            }

            // 3. 尝试工作窃取
            if let Some(task_id) = work_queue.steal() {
                if let Some(task_ref) = self.tasks.get(&task_id) {
                    let task = task_ref.read().await;

                    if task.status == QueueTaskStatus::Queued {
                        drop(task);
                        return Some(task_ref.clone());
                    }
                }
            }
        }

        None
    }

    /// 根据ID获取任务
    pub async fn get_task(&self, task_id: &str) -> Option<Arc<RwLock<QueueTask>>> {
        self.tasks.get(task_id).map(|entry| entry.clone())
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
            self.remove_from_priority_queues(task_id).await;
        }

        // 如果状态变为Queued，添加到优先级队列
        if old_status != QueueTaskStatus::Queued && status == QueueTaskStatus::Queued {
            let priority_queue = self.get_priority_queue(&task.priority);
            priority_queue.push(task_id.to_string());
        }

        Ok(())
    }

    /// 从所有优先级队列中移除任务
    async fn remove_from_priority_queues(&self, task_id: &str) {
        // 由于使用无锁队列，我们无法直接删除特定元素
        // 任务状态更新时会检查状态，所以不需要特别处理
        // 这里留空，因为无锁队列的pop操作会自然过滤掉无效任务
    }

    /// 获取所有任务
    pub async fn get_all_tasks(&self) -> Vec<Arc<RwLock<QueueTask>>> {
        self.tasks.iter()
            .map(|entry| entry.clone())
            .collect()
    }

    /// 根据过滤器获取任务
    pub async fn get_tasks_by_filter(&self, filter: &TaskFilter) -> Vec<Arc<RwLock<QueueTask>>> {
        let mut result = Vec::new();

        for entry in self.tasks.iter() {
            let task_ref = entry.clone();
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
        // 从任务存储中移除
        let removed = self.tasks.remove(task_id).is_some();

        Ok(removed)
    }

    /// 获取队列统计信息
    pub async fn get_statistics(&self) -> Result<crate::task_queue::models::QueueStatistics> {
        let mut stats = crate::task_queue::models::QueueStatistics::default();
        stats.total_tasks = self.tasks.len() as u32;

        let mut total_wait_time = 0.0;
        let mut total_execution_time = 0.0;
        let mut completed_count = 0;
        let mut successful_count = 0;

        for entry in self.tasks.iter() {
            let task_ref = entry.clone();
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

        // 计算队列大小
        stats.high_priority_queue_size = self.high_priority_queue.len() as u32;
        stats.medium_priority_queue_size = self.medium_priority_queue.len() as u32;
        stats.low_priority_queue_size = self.low_priority_queue.len() as u32;

        // 计算工作窃取队列统计
        let mut total_work_stealing_size = 0;
        for queue in self.work_stealing_queues.iter() {
            total_work_stealing_size += queue.len();
        }
        stats.work_stealing_queue_size = total_work_stealing_size as u32;

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
        self.tasks.len()
    }

    /// 检查队列是否为空
    pub async fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// 清空队列
    pub async fn clear(&self) {
        // 清空优先级队列
        while self.high_priority_queue.pop().is_some() {}
        while self.medium_priority_queue.pop().is_some() {}
        while self.low_priority_queue.pop().is_some() {}

        // 清空任务存储
        self.tasks.clear();

        // 清空工作窃取队列
        for mut queue in self.work_stealing_queues.iter_mut() {
            while queue.pop_local().is_some() {}
        }
    }

    /// 清理已完成的任务
    pub async fn cleanup_completed_tasks(&self, older_than_days: u32) -> Result<usize> {
        let cutoff_time = Utc::now() - chrono::Duration::days(older_than_days as i64);
        let mut removed_count = 0;

        let task_ids: Vec<String> = self.tasks.iter()
            .filter_map(|entry| {
                let task_ref = entry.clone();
                let task = task_ref.read().unwrap();
                if task.status.is_finished() && task.completed_at.map_or(false, |t| t < cutoff_time) {
                    Some(task.id.clone())
                } else {
                    None
                }
            })
            .collect();

        for task_id in task_ids {
            if self.remove_task(&task_id).await? {
                removed_count += 1;
            }
        }

        Ok(removed_count)
    }

    /// 获取工作窃取队列统计
    pub fn get_work_stealing_stats(&self) -> WorkStealingStats {
        let mut stats = WorkStealingStats::default();

        for (worker_id, queue) in self.work_stealing_queues.iter() {
            stats.worker_queues.insert(*worker_id, queue.len());
            stats.total_local_tasks += queue.len();
        }

        stats.total_global_tasks = self.high_priority_queue.len()
            + self.medium_priority_queue.len()
            + self.low_priority_queue.len();

        stats
    }
}

/// 工作窃取统计信息
#[derive(Debug, Clone, Default)]
pub struct WorkStealingStats {
    pub worker_queues: std::collections::HashMap<usize, usize>,
    pub total_local_tasks: usize,
    pub total_global_tasks: usize,
}

impl WorkStealingStats {
    /// 计算负载均衡度
    pub fn load_balance_score(&self) -> f64 {
        if self.worker_queues.is_empty() {
            return 1.0;
        }

        let avg_load = self.total_local_tasks as f64 / self.worker_queues.len() as f64;
        let mut variance = 0.0;

        for &load in self.worker_queues.values() {
            let diff = load as f64 - avg_load;
            variance += diff * diff;
        }

        variance = variance / self.worker_queues.len() as f64;

        // 方差越小，负载越均衡
        1.0 / (1.0 + variance.sqrt())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::compression::{CompressionTask, CompressionFormat, CompressionOptions};
    use crate::task_queue::models::{TaskType, TaskPriority, QueueTaskStatus};

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
    async fn test_optimized_queue_creation() {
        let queue = OptimizedTaskQueue::new(100, 4);

        assert_eq!(queue.max_size, 100);
        assert_eq!(queue.worker_count, 4);
        assert!(queue.is_empty().await);
    }

    #[tokio::test]
    async fn test_add_and_get_task() {
        let queue = OptimizedTaskQueue::new(100, 2);
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
    async fn test_get_next_task_with_work_stealing() {
        let queue = OptimizedTaskQueue::new(100, 2);

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
        let next_task_ref = queue.get_next_task(0).await;
        assert!(next_task_ref.is_some());

        let next_task = next_task_ref.unwrap().read().await;
        assert_eq!(next_task.priority, TaskPriority::High);
    }

    #[tokio::test]
    async fn test_update_task_status() {
        let queue = OptimizedTaskQueue::new(100, 2);
        let task = create_test_task();
        let task_id = task.id.clone();

        queue.add_task(task).await.unwrap();

        // 更新状态为Running
        queue.update_task_status(&task_id, QueueTaskStatus::Running).await.unwrap();

        let task_ref = queue.get_task(&task_id).await.unwrap();
        let task = task_ref.read().await;
        assert_eq!(task.status, QueueTaskStatus::Running);
    }

    #[tokio::test]
    async fn test_queue_size_limit() {
        let queue = OptimizedTaskQueue::new(2, 2); // 限制为2个任务

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
        let queue = OptimizedTaskQueue::new(100, 2);

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

        // 检查优先级队列统计
        assert!(stats.high_priority_queue_size > 0 || stats.medium_priority_queue_size > 0 || stats.low_priority_queue_size > 0);
    }

    #[tokio::test]
    async fn test_work_stealing_stats() {
        let queue = OptimizedTaskQueue::new(100, 4);

        // 添加一些任务
        for i in 0..10 {
            let task = create_test_task();
            queue.add_task(task).await.unwrap();
        }

        // 获取工作窃取统计
        let stats = queue.get_work_stealing_stats();

        assert_eq!(stats.worker_queues.len(), 4);
        assert!(stats.total_local_tasks > 0);
        assert!(stats.total_global_tasks > 0);

        // 检查负载均衡分数
        let balance_score = stats.load_balance_score();
        assert!(balance_score >= 0.0 && balance_score <= 1.0);
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let queue = Arc::new(OptimizedTaskQueue::new(1000, 4));
        let mut tasks = Vec::new();

        // 并发添加任务
        for i in 0..100 {
            let queue_clone = queue.clone();
            let task = tokio::spawn(async move {
                let test_task = create_test_task();
                queue_clone.add_task(test_task).await
            });
            tasks.push(task);
        }

        // 等待所有添加任务完成
        for task in tasks {
            assert!(task.await.unwrap().is_ok());
        }

        // 验证任务数量
        assert_eq!(queue.size().await, 100);

        // 并发获取任务
        let mut get_tasks = Vec::new();
        for worker_id in 0..4 {
            let queue_clone = queue.clone();
            let task = tokio::spawn(async move {
                let mut count = 0;
                while let Some(_) = queue_clone.get_next_task(worker_id).await {
                    count += 1;
                }
                count
            });
            get_tasks.push(task);
        }

        // 等待所有获取任务完成
        let mut total_retrieved = 0;
        for task in get_tasks {
            total_retrieved += task.await.unwrap();
        }

        // 验证所有任务都被获取
        assert_eq!(total_retrieved, 100);
    }
}