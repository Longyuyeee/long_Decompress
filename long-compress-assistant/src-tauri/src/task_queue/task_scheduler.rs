use crate::task_queue::models::{QueueTask, QueueTaskStatus, TaskPriority, ResourceUsage};
use crate::task_queue::task_queue::TaskQueue;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tokio::time::{Duration, interval};
use anyhow::{Result, anyhow};
use std::collections::VecDeque;

/// 调度器配置
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub max_concurrent_tasks: usize,
    pub cpu_threshold: f32,      // CPU使用率阈值（0-100）
    pub memory_threshold: f32,   // 内存使用率阈值（0-100）
    pub check_interval_ms: u64,  // 资源检查间隔（毫秒）
    pub enable_resource_aware: bool, // 是否启用资源感知调度
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 3,
            cpu_threshold: 80.0,
            memory_threshold: 85.0,
            check_interval_ms: 5000, // 5秒
            enable_resource_aware: true,
        }
    }
}

/// 任务调度器
pub struct TaskScheduler {
    queue: Arc<TaskQueue>,
    config: SchedulerConfig,
    semaphore: Arc<Semaphore>,
    running_tasks: Arc<RwLock<Vec<String>>>, // 正在运行的任务ID列表
    resource_usage: Arc<RwLock<ResourceUsage>>,
    task_buffer: Arc<RwLock<VecDeque<Arc<tokio::sync::RwLock<QueueTask>>>>>, // 任务缓冲区
}

impl TaskScheduler {
    /// 创建新的任务调度器
    pub fn new(queue: Arc<TaskQueue>, config: SchedulerConfig) -> Self {
        let max_concurrent = config.max_concurrent_tasks;

        Self {
            queue,
            config: config.clone(),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            running_tasks: Arc::new(RwLock::new(Vec::new())),
            resource_usage: Arc::new(RwLock::new(ResourceUsage::default())),
            task_buffer: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    /// 启动调度器
    pub async fn start(&self) -> Result<()> {
        log::info!("任务调度器启动，最大并发任务数: {}", self.config.max_concurrent_tasks);

        // 启动资源监控（如果启用）
        if self.config.enable_resource_aware {
            self.start_resource_monitoring().await?;
        }

        // 启动任务调度循环
        self.start_scheduling_loop().await?;

        Ok(())
    }

    /// 启动资源监控
    async fn start_resource_monitoring(&self) -> Result<()> {
        let config = self.config.clone();
        let resource_usage = self.resource_usage.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(config.check_interval_ms));

            loop {
                interval.tick().await;

                // 获取系统资源使用情况
                let usage = Self::get_system_resource_usage().await;

                let mut current_usage = resource_usage.write().await;
                *current_usage = usage;
            }
        });

        Ok(())
    }

    /// 获取系统资源使用情况
    async fn get_system_resource_usage() -> ResourceUsage {
        use sysinfo::System;

        let mut system = System::new_all();
        system.refresh_all();

        // 等待一小段时间获取准确的CPU使用率
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        system.refresh_cpu();

        // 计算CPU使用率
        let cpus = system.cpus();
        let cpu_usage = if cpus.is_empty() {
            0.0
        } else {
            let total_usage: f32 = cpus.iter().map(|cpu| cpu.cpu_usage()).sum();
            total_usage / cpus.len() as f32
        };

        // 获取内存使用情况
        let total_memory = system.total_memory();
        let used_memory = system.used_memory();
        let memory_usage_mb = used_memory as f64 / 1024.0 / 1024.0;

        // 获取磁盘IO信息（简化实现）
        let disks = system.disks();
        let mut disk_io_read_mb = 0.0;
        let mut disk_io_write_mb = 0.0;

        for disk in disks {
            // sysinfo目前不直接提供IO统计，这里使用简化估算
            // 在实际应用中，可以使用其他库或平台特定API
            disk_io_read_mb += disk.total_space() as f64 / 1024.0 / 1024.0 * 0.01; // 简化估算
            disk_io_write_mb += disk.total_space() as f64 / 1024.0 / 1024.0 * 0.005; // 简化估算
        }

        // 获取网络IO信息
        let networks = system.networks();
        let mut network_io_mb = 0.0;

        for (_, network_data) in networks {
            network_io_mb += (network_data.received() + network_data.transmitted()) as f64 / 1024.0 / 1024.0;
        }

        ResourceUsage {
            cpu_usage,
            memory_usage_mb,
            disk_io_read_mb,
            disk_io_write_mb,
            network_io_mb,
        }
    }

    /// 启动任务调度循环
    async fn start_scheduling_loop(&self) -> Result<()> {
        let queue = self.queue.clone();
        let semaphore = self.semaphore.clone();
        let running_tasks = self.running_tasks.clone();
        let resource_usage = self.resource_usage.clone();
        let config = self.config.clone();
        let task_buffer = self.task_buffer.clone();

        tokio::spawn(async move {
            loop {
                // 检查是否有可用的并发槽位
                let available_slots = semaphore.available_permits();

                if available_slots > 0 {
                    // 检查资源使用情况（如果启用资源感知调度）
                    let can_schedule = if config.enable_resource_aware {
                        let usage = resource_usage.read().await;
                        usage.cpu_usage < config.cpu_threshold &&
                        usage.memory_usage_mb < config.memory_threshold as f64
                    } else {
                        true
                    };

                    if can_schedule {
                        // 从队列中获取下一个任务
                        if let Some(task_ref) = queue.get_next_task().await {
                            let task_id = {
                                let task = task_ref.read().await;
                                task.id.clone()
                            };

                            // 检查任务是否仍然可执行
                            let should_schedule = {
                                let task = task_ref.read().await;
                                task.status == QueueTaskStatus::Queued
                            };

                            if should_schedule {
                                // 获取信号量许可
                                let permit = semaphore.acquire().await.unwrap();

                                // 更新任务状态为Scheduled
                                if let Err(e) = queue.update_task_status(&task_id, QueueTaskStatus::Scheduled).await {
                                    log::error!("更新任务状态失败: {}", e);
                                    drop(permit);
                                    continue;
                                }

                                // 添加到运行任务列表
                                {
                                    let mut running = running_tasks.write().await;
                                    running.push(task_id.clone());
                                }

                                // 将任务放入缓冲区等待执行
                                {
                                    let mut buffer = task_buffer.write().await;
                                    buffer.push_back(task_ref.clone());
                                }

                                // 注意：这里不drop permit，任务执行完成后再drop
                                // 信号量许可会随着任务执行完成而释放
                            }
                        }
                    }
                }

                // 短暂休眠避免CPU占用过高
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });

        Ok(())
    }

    /// 从缓冲区获取下一个待执行任务
    pub async fn get_next_executable_task(&self) -> Option<Arc<tokio::sync::RwLock<QueueTask>>> {
        let mut buffer = self.task_buffer.write().await;
        buffer.pop_front()
    }

    /// 任务执行完成回调
    pub async fn on_task_completed(&self, task_id: &str, success: bool, error_message: Option<String>) {
        // 从运行任务列表中移除
        {
            let mut running_tasks = self.running_tasks.write().await;
            running_tasks.retain(|id| id != task_id);
        }

        // 更新任务状态
        let status = if success {
            QueueTaskStatus::Completed
        } else {
            QueueTaskStatus::Failed
        };

        if let Err(e) = self.queue.update_task_status(task_id, status).await {
            log::error!("更新任务完成状态失败: {}", e);
        }

        // 释放信号量许可（通过drop permit）
        // 注意：实际信号量释放在任务执行器中完成
    }

    /// 获取当前运行任务数
    pub async fn get_running_task_count(&self) -> usize {
        let running_tasks = self.running_tasks.read().await;
        running_tasks.len()
    }

    /// 获取可用并发槽位数
    pub async fn get_available_slots(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// 获取调度器状态
    pub async fn get_scheduler_status(&self) -> SchedulerStatus {
        let running_count = self.get_running_task_count().await;
        let available_slots = self.get_available_slots().await;
        let buffer_size = {
            let buffer = self.task_buffer.read().await;
            buffer.len()
        };

        let resource_usage = self.resource_usage.read().await;

        SchedulerStatus {
            max_concurrent_tasks: self.config.max_concurrent_tasks,
            running_tasks: running_count,
            available_slots,
            buffered_tasks: buffer_size,
            cpu_usage: resource_usage.cpu_usage,
            memory_usage_mb: resource_usage.memory_usage_mb,
            resource_aware_enabled: self.config.enable_resource_aware,
            cpu_threshold: self.config.cpu_threshold,
            memory_threshold: self.config.memory_threshold,
        }
    }

    /// 更新调度器配置
    pub async fn update_config(&mut self, config: SchedulerConfig) -> Result<()> {
        // 更新最大并发任务数
        let old_max = self.config.max_concurrent_tasks;
        let new_max = config.max_concurrent_tasks;

        if new_max != old_max {
            // 调整信号量大小
            let difference = new_max as isize - old_max as isize;

            if difference > 0 {
                // 增加许可
                self.semaphore.add_permits(difference as usize);
            } else {
                // 减少许可（需要更复杂的逻辑，这里简化处理）
                log::warn!("减少最大并发任务数需要重启调度器");
                return Err(anyhow!("不支持动态减少最大并发任务数"));
            }
        }

        self.config = config;
        Ok(())
    }

    /// 暂停调度器
    pub async fn pause(&self) {
        // 实现暂停逻辑
        log::info!("调度器已暂停");
    }

    /// 恢复调度器
    pub async fn resume(&self) {
        // 实现恢复逻辑
        log::info!("调度器已恢复");
    }
}

/// 调度器状态
#[derive(Debug, Clone, serde::Serialize)]
pub struct SchedulerStatus {
    pub max_concurrent_tasks: usize,
    pub running_tasks: usize,
    pub available_slots: usize,
    pub buffered_tasks: usize,
    pub cpu_usage: f32,
    pub memory_usage_mb: f64,
    pub resource_aware_enabled: bool,
    pub cpu_threshold: f32,
    pub memory_threshold: f32,
}

/// 资源感知调度策略
pub struct ResourceAwarePolicy {
    pub cpu_weight: f32,
    pub memory_weight: f32,
    pub io_weight: f32,
    pub min_resource_score: f32, // 最小资源评分（0-1）
}

impl Default for ResourceAwarePolicy {
    fn default() -> Self {
        Self {
            cpu_weight: 0.4,
            memory_weight: 0.4,
            io_weight: 0.2,
            min_resource_score: 0.3,
        }
    }
}

impl ResourceAwarePolicy {
    /// 计算任务资源需求评分
    pub fn calculate_task_score(&self, task: &QueueTask, current_usage: &ResourceUsage) -> f32 {
        // 根据任务类型和大小估算资源需求
        let task_size = task.compression_task.total_size as f64;

        // 简化的资源需求估算
        let cpu_demand = if task_size > 100 * 1024 * 1024 { // 大于100MB
            0.8
        } else if task_size > 10 * 1024 * 1024 { // 大于10MB
            0.5
        } else {
            0.2
        };

        let memory_demand = if task_size > 50 * 1024 * 1024 { // 大于50MB
            0.7
        } else if task_size > 5 * 1024 * 1024 { // 大于5MB
            0.4
        } else {
            0.1
        };

        // 考虑当前系统负载
        let cpu_score = 1.0 - (current_usage.cpu_usage / 100.0).max(0.0).min(1.0);
        let memory_score = 1.0 - (current_usage.memory_usage_mb / 8192.0).max(0.0).min(1.0); // 假设8GB内存

        // 综合评分
        let score = (cpu_score * self.cpu_weight + memory_score * self.memory_weight)
            / (cpu_demand + memory_demand).max(0.1);

        score.max(0.0).min(1.0)
    }

    /// 检查是否应该调度任务
    pub fn should_schedule_task(&self, task: &QueueTask, current_usage: &ResourceUsage) -> bool {
        let score = self.calculate_task_score(task, current_usage);
        score >= self.min_resource_score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_queue::task_queue::TaskQueue;
    use crate::models::compression::{CompressionTask, CompressionFormat, CompressionOptions};

    fn create_test_queue() -> Arc<TaskQueue> {
        Arc::new(TaskQueue::new(100))
    }

    fn create_test_task() -> QueueTask {
        let compression_task = CompressionTask::new(
            vec!["test.txt".to_string()],
            "output.zip".to_string(),
            CompressionFormat::Zip,
            CompressionOptions::default(),
        );

        QueueTask::new(
            crate::task_queue::models::TaskType::Compress,
            TaskPriority::Medium,
            compression_task,
        )
    }

    #[tokio::test]
    async fn test_scheduler_initialization() {
        let queue = create_test_queue();
        let config = SchedulerConfig::default();

        let scheduler = TaskScheduler::new(queue, config);

        let status = scheduler.get_scheduler_status().await;
        assert_eq!(status.max_concurrent_tasks, 3);
        assert_eq!(status.running_tasks, 0);
        assert_eq!(status.available_slots, 3);
    }

    #[tokio::test]
    async fn test_resource_aware_policy() {
        let policy = ResourceAwarePolicy::default();
        let task = create_test_task();
        let current_usage = ResourceUsage::default();

        let score = policy.calculate_task_score(&task, &current_usage);
        assert!(score >= 0.0 && score <= 1.0);

        let should_schedule = policy.should_schedule_task(&task, &current_usage);
        assert!(should_schedule); // 默认情况下应该可以调度
    }

    #[tokio::test]
    async fn test_task_completion_callback() {
        let queue = create_test_queue();
        let config = SchedulerConfig::default();

        let scheduler = TaskScheduler::new(queue.clone(), config);

        // 添加一个任务
        let task = create_test_task();
        let task_id = task.id.clone();
        queue.add_task(task).await.unwrap();

        // 模拟任务完成
        scheduler.on_task_completed(&task_id, true, None).await;

        // 检查任务状态
        let task_ref = queue.get_task(&task_id).await.unwrap();
        let task = task_ref.read().await;

        assert_eq!(task.status, QueueTaskStatus::Completed);
    }
}