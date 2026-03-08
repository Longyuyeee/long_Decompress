use crate::task_queue::{TaskManager, TASK_MANAGER, models::{TaskPriority, TaskFilter, QueueTaskStatus}};
use crate::models::compression::{CompressionFormat, CompressionOptions};
use tauri::command;
use serde::{Deserialize, Serialize};

/// 添加压缩任务请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddCompressionTaskRequest {
    pub source_files: Vec<String>,
    pub output_path: String,
    pub format: CompressionFormat,
    pub options: CompressionOptions,
    pub priority: TaskPriority,
}

/// 添加解压任务请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddExtractionTaskRequest {
    pub source_file: String,
    pub output_dir: Option<String>,
    pub password: Option<String>,
    pub priority: TaskPriority,
}

/// 任务列表响应项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskListItem {
    pub id: String,
    pub task_type: String,
    pub priority: String,
    pub status: String,
    pub progress: f32,
    pub source_files: Vec<String>,
    pub output_path: String,
    pub format: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
}

/// 任务详情响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDetailResponse {
    pub id: String,
    pub task_type: String,
    pub priority: String,
    pub status: String,
    pub progress: f32,
    pub source_files: Vec<String>,
    pub output_path: String,
    pub format: String,
    pub options: CompressionOptions,
    pub created_at: String,
    pub scheduled_at: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub error_message: Option<String>,
    pub total_size: u64,
    pub processed_size: u64,
}

/// 队列统计响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatsResponse {
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

/// 调度器状态响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStatusResponse {
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

/// 执行器状态响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorStatusResponse {
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub timeout_seconds: u64,
    pub enable_progress_tracking: bool,
    pub progress_update_interval_ms: u64,
    pub running_tasks: usize,
}

#[command]
pub async fn add_compression_task(request: AddCompressionTaskRequest) -> Result<String, String> {
    let manager = TASK_MANAGER.get().await
        .map_err(|e| format!("获取任务管理器失败: {}", e))?;

    manager.add_compression_task(
        request.source_files,
        request.output_path,
        request.format,
        request.options,
        request.priority,
    ).await
    .map_err(|e| format!("添加压缩任务失败: {}", e))
}

#[command]
pub async fn add_extraction_task(request: AddExtractionTaskRequest) -> Result<String, String> {
    let manager = TASK_MANAGER.get().await
        .map_err(|e| format!("获取任务管理器失败: {}", e))?;

    manager.add_extraction_task(
        request.source_file,
        request.output_dir,
        request.password,
        request.priority,
    ).await
    .map_err(|e| format!("添加解压任务失败: {}", e))
}

#[command]
pub async fn get_task(task_id: String) -> Result<TaskDetailResponse, String> {
    let manager = TASK_MANAGER.get().await
        .map_err(|e| format!("获取任务管理器失败: {}", e))?;

    let task_ref = manager.get_task(&task_id).await
        .map_err(|e| format!("获取任务失败: {}", e))?;

    let task = task_ref.read().await;

    Ok(TaskDetailResponse {
        id: task.id.clone(),
        task_type: format!("{:?}", task.task_type),
        priority: format!("{:?}", task.priority),
        status: format!("{:?}", task.status),
        progress: task.compression_task.progress,
        source_files: task.compression_task.source_files.clone(),
        output_path: task.compression_task.output_path.clone(),
        format: format!("{:?}", task.compression_task.format),
        options: task.compression_task.options.clone(),
        created_at: task.created_at.to_rfc3339(),
        scheduled_at: task.scheduled_at.map(|t| t.to_rfc3339()),
        started_at: task.started_at.map(|t| t.to_rfc3339()),
        completed_at: task.completed_at.map(|t| t.to_rfc3339()),
        retry_count: task.retry_count,
        max_retries: task.max_retries,
        error_message: task.error_message.clone(),
        total_size: task.compression_task.total_size,
        processed_size: task.compression_task.processed_size,
    })
}

#[command]
pub async fn list_tasks(
    status: Option<String>,
    task_type: Option<String>,
    priority: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<TaskListItem>, String> {
    let manager = TASK_MANAGER.get().await
        .map_err(|e| format!("获取任务管理器失败: {}", e))?;

    // 构建过滤器
    let mut filter = TaskFilter::default();

    if let Some(status_str) = status {
        filter.status = match status_str.to_lowercase().as_str() {
            "queued" => Some(QueueTaskStatus::Queued),
            "scheduled" => Some(QueueTaskStatus::Scheduled),
            "running" => Some(QueueTaskStatus::Running),
            "paused" => Some(QueueTaskStatus::Paused),
            "completed" => Some(QueueTaskStatus::Completed),
            "failed" => Some(QueueTaskStatus::Failed),
            "cancelled" => Some(QueueTaskStatus::Cancelled),
            _ => None,
        };
    }

    if let Some(task_type_str) = task_type {
        filter.task_type = match task_type_str.to_lowercase().as_str() {
            "compress" => Some(crate::task_queue::models::TaskType::Compress),
            "extract" => Some(crate::task_queue::models::TaskType::Extract),
            _ => None,
        };
    }

    if let Some(priority_str) = priority {
        filter.priority = match priority_str.to_lowercase().as_str() {
            "high" => Some(TaskPriority::High),
            "medium" => Some(TaskPriority::Medium),
            "low" => Some(TaskPriority::Low),
            _ => None,
        };
    }

    filter.limit = limit;
    filter.offset = offset;

    let task_refs = manager.get_tasks_by_filter(&filter).await;

    let mut tasks = Vec::new();
    for task_ref in task_refs {
        let task = task_ref.read().await;

        tasks.push(TaskListItem {
            id: task.id.clone(),
            task_type: format!("{:?}", task.task_type),
            priority: format!("{:?}", task.priority),
            status: format!("{:?}", task.status),
            progress: task.compression_task.progress,
            source_files: task.compression_task.source_files.clone(),
            output_path: task.compression_task.output_path.clone(),
            format: format!("{:?}", task.compression_task.format),
            created_at: task.created_at.to_rfc3339(),
            started_at: task.started_at.map(|t| t.to_rfc3339()),
            completed_at: task.completed_at.map(|t| t.to_rfc3339()),
            error_message: task.error_message.clone(),
        });
    }

    Ok(tasks)
}

#[command]
pub async fn get_task_status(task_id: String) -> Result<String, String> {
    let manager = TASK_MANAGER.get().await
        .map_err(|e| format!("获取任务管理器失败: {}", e))?;

    let status = manager.get_task_status(&task_id).await
        .map_err(|e| format!("获取任务状态失败: {}", e))?;

    Ok(format!("{:?}", status))
}

#[command]
pub async fn get_task_progress(task_id: String) -> Result<f32, String> {
    let manager = TASK_MANAGER.get().await
        .map_err(|e| format!("获取任务管理器失败: {}", e))?;

    manager.get_task_progress(&task_id).await
        .map_err(|e| format!("获取任务进度失败: {}", e))
}

#[command]
pub async fn cancel_task(task_id: String) -> Result<bool, String> {
    let manager = TASK_MANAGER.get().await
        .map_err(|e| format!("获取任务管理器失败: {}", e))?;

    manager.cancel_task(&task_id).await
        .map_err(|e| format!("取消任务失败: {}", e))
}

#[command]
pub async fn pause_task(task_id: String) -> Result<bool, String> {
    let manager = TASK_MANAGER.get().await
        .map_err(|e| format!("获取任务管理器失败: {}", e))?;

    manager.pause_task(&task_id).await
        .map_err(|e| format!("暂停任务失败: {}", e))
}

#[command]
pub async fn resume_task(task_id: String) -> Result<bool, String> {
    let manager = TASK_MANAGER.get().await
        .map_err(|e| format!("获取任务管理器失败: {}", e))?;

    manager.resume_task(&task_id).await
        .map_err(|e| format!("恢复任务失败: {}", e))
}

#[command]
pub async fn get_queue_statistics() -> Result<QueueStatsResponse, String> {
    let manager = TASK_MANAGER.get().await
        .map_err(|e| format!("获取任务管理器失败: {}", e))?;

    let stats = manager.get_queue_statistics().await
        .map_err(|e| format!("获取队列统计失败: {}", e))?;

    Ok(QueueStatsResponse {
        total_tasks: stats.total_tasks,
        queued_tasks: stats.queued_tasks,
        scheduled_tasks: stats.scheduled_tasks,
        running_tasks: stats.running_tasks,
        paused_tasks: stats.paused_tasks,
        completed_tasks: stats.completed_tasks,
        failed_tasks: stats.failed_tasks,
        cancelled_tasks: stats.cancelled_tasks,
        average_wait_time_seconds: stats.average_wait_time_seconds,
        average_execution_time_seconds: stats.average_execution_time_seconds,
        success_rate: stats.success_rate,
    })
}

#[command]
pub async fn get_scheduler_status() -> Result<SchedulerStatusResponse, String> {
    let manager = TASK_MANAGER.get().await
        .map_err(|e| format!("获取任务管理器失败: {}", e))?;

    let status = manager.get_scheduler_status().await
        .map_err(|e| format!("获取调度器状态失败: {}", e))?;

    Ok(SchedulerStatusResponse {
        max_concurrent_tasks: status.max_concurrent_tasks,
        running_tasks: status.running_tasks,
        available_slots: status.available_slots,
        buffered_tasks: status.buffered_tasks,
        cpu_usage: status.cpu_usage,
        memory_usage_mb: status.memory_usage_mb,
        resource_aware_enabled: status.resource_aware_enabled,
        cpu_threshold: status.cpu_threshold,
        memory_threshold: status.memory_threshold,
    })
}

#[command]
pub async fn get_executor_status() -> Result<ExecutorStatusResponse, String> {
    let manager = TASK_MANAGER.get().await
        .map_err(|e| format!("获取任务管理器失败: {}", e))?;

    let status = manager.get_executor_status().await
        .map_err(|e| format!("获取执行器状态失败: {}", e))?;

    Ok(ExecutorStatusResponse {
        max_retries: status.max_retries,
        retry_delay_ms: status.retry_delay_ms,
        timeout_seconds: status.timeout_seconds,
        enable_progress_tracking: status.enable_progress_tracking,
        progress_update_interval_ms: status.progress_update_interval_ms,
        running_tasks: status.running_tasks,
    })
}

#[command]
pub async fn cleanup_completed_tasks(older_than_days: u32) -> Result<usize, String> {
    let manager = TASK_MANAGER.get().await
        .map_err(|e| format!("获取任务管理器失败: {}", e))?;

    manager.cleanup_completed_tasks(older_than_days).await
        .map_err(|e| format!("清理已完成任务失败: {}", e))
}

#[command]
pub async fn stop_all_tasks() -> Result<(), String> {
    let manager = TASK_MANAGER.get().await
        .map_err(|e| format!("获取任务管理器失败: {}", e))?;

    manager.stop_all_tasks().await
        .map_err(|e| format!("停止所有任务失败: {}", e))
}

#[command]
pub async fn get_all_tasks() -> Result<Vec<TaskListItem>, String> {
    let manager = TASK_MANAGER.get().await
        .map_err(|e| format!("获取任务管理器失败: {}", e))?;

    let task_refs = manager.get_all_tasks().await;

    let mut tasks = Vec::new();
    for task_ref in task_refs {
        let task = task_ref.read().await;

        tasks.push(TaskListItem {
            id: task.id.clone(),
            task_type: format!("{:?}", task.task_type),
            priority: format!("{:?}", task.priority),
            status: format!("{:?}", task.status),
            progress: task.compression_task.progress,
            source_files: task.compression_task.source_files.clone(),
            output_path: task.compression_task.output_path.clone(),
            format: format!("{:?}", task.compression_task.format),
            created_at: task.created_at.to_rfc3339(),
            started_at: task.started_at.map(|t| t.to_rfc3339()),
            completed_at: task.completed_at.map(|t| t.to_rfc3339()),
            error_message: task.error_message.clone(),
        });
    }

    Ok(tasks)
}