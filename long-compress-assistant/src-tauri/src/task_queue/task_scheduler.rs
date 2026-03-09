use crate::task_queue::task_queue::TaskQueue;
use crate::task_queue::models::{QueueTask, QueueTaskStatus, SharedQueueTask};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use serde::{Deserialize, Serialize};
use sysinfo::System;

/// 调度器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub max_concurrent_tasks: usize,
    pub check_interval_ms: u64,
    pub resource_check_enabled: bool,
    pub cpu_usage_threshold: f32,
    pub memory_usage_threshold: f32,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 4,
            check_interval_ms: 500,
            resource_check_enabled: true,
            cpu_usage_threshold: 85.0,
            memory_usage_threshold: 90.0,
        }
    }
}

/// 调度器状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStatus {
    pub is_running: bool,
    pub concurrent_tasks: usize,
    pub total_scheduled: usize,
    pub last_check_time: chrono::DateTime<chrono::Utc>,
}

/// 任务调度器
pub struct TaskScheduler {
    queue: Arc<TaskQueue>,
    config: SchedulerConfig,
    status: Arc<tokio::sync::RwLock<SchedulerStatus>>,
}

impl TaskScheduler {
    /// 创建新的任务调度器
    pub fn new(queue: Arc<TaskQueue>, config: SchedulerConfig) -> Self {
        Self {
            queue,
            config,
            status: Arc::new(tokio::sync::RwLock::new(SchedulerStatus {
                is_running: false,
                concurrent_tasks: 0,
                total_scheduled: 0,
                last_check_time: chrono::Utc::now(),
            })),
        }
    }

    /// 启动调度器
    pub async fn start(&self) -> anyhow::Result<()> {
        let mut status = self.status.write().await;
        if status.is_running {
            return Ok(());
        }

        let queue = self.queue.clone();
        let config = self.config.clone();
        let status_clone = self.status.clone();

        tokio::spawn(async move {
            let mut system = System::new_all();
            
            loop {
                sleep(Duration::from_millis(config.check_interval_ms)).await;
                
                // 更新系统信息
                system.refresh_all();
                
                // 检查资源
                let cpu_usage = system.global_cpu_info().cpu_usage();
                let total_mem = system.total_memory() as f32;
                let used_mem = system.used_memory() as f32;
                let memory_usage_pct = if total_mem > 0.0 { (used_mem / total_mem) * 100.0 } else { 0.0 };

                if config.resource_check_enabled && 
                    (cpu_usage > config.cpu_usage_threshold || memory_usage_pct > config.memory_usage_threshold) {
                    log::debug!("系统资源紧张，跳过本次调度: CPU={:.1}%, MEM={:.1}%", cpu_usage, memory_usage_pct);
                    continue;
                }

                // 尝试调度任务
                let mut status = status_clone.write().await;
                if status.concurrent_tasks < config.max_concurrent_tasks {
                    if let Some(task_ref) = queue.get_next_task().await {
                        let task_id = {
                            let task = task_ref.read().await;
                            task.id.clone()
                        };
                        
                        log::info!("调度任务: {}", task_id);
                        
                        // 更新任务状态
                        if let Err(e) = queue.update_task_status(&task_id, QueueTaskStatus::Running).await {
                            log::error!("更新任务状态失败: {}", e);
                            continue;
                        }

                        status.concurrent_tasks += 1;
                        status.total_scheduled += 1;
                        status.last_check_time = chrono::Utc::now();
                    }
                }
            }
        });

        status.is_running = true;
        Ok(())
    }

    /// 获取调度器状态
    pub async fn get_scheduler_status(&self) -> SchedulerStatus {
        let status = self.status.read().await;
        status.clone()
    }

    /// 任务完成回调
    pub async fn on_task_completed(&self) {
        let mut status = self.status.write().await;
        if status.concurrent_tasks > 0 {
            status.concurrent_tasks -= 1;
        }
    }
}

/// 资源使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_usage: f32,
    pub memory_usage_mb: f32,
    pub disk_io_mbps: f32,
}

/// 资源感知调度策略
pub struct ResourceAwarePolicy {
    pub cpu_weight: f32,
    pub memory_weight: f32,
    pub io_weight: f32,
    pub min_resource_score: f32,
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
        let task_size = task.compression_task.total_size as f32;

        // 简化的资源需求估算
        let cpu_demand = if task_size > 100.0 * 1024.0 * 1024.0 { // 大于100MB
            0.8f32
        } else if task_size > 10.0 * 1024.0 * 1024.0 { // 大于10MB
            0.5f32
        } else {
            0.2f32
        };

        let memory_demand = if task_size > 50.0 * 1024.0 * 1024.0 { // 大于50MB
            0.7f32
        } else if task_size > 5.0 * 1024.0 * 1024.0 { // 大于5MB
            0.4f32
        } else {
            0.1f32
        };

        // 考虑当前系统负载
        let cpu_score = 1.0f32 - (current_usage.cpu_usage / 100.0).max(0.0).min(1.0);
        let memory_score = 1.0f32 - (current_usage.memory_usage_mb / 8192.0).max(0.0).min(1.0); // 假设8GB内存

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
