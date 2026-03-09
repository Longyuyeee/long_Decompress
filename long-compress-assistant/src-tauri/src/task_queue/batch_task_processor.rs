//! 批量任务处理器
//!
//! 提供批量文件处理与任务队列系统的集成功能。

use crate::task_queue::models::{QueueTask, TaskType, TaskPriority, QueueTaskStatus};
use crate::task_queue::task_manager::TaskManager;
use crate::models::compression::{CompressionTask, CompressionFormat, CompressionOptions};
use crate::services::file_service::{FileService, BatchOperationResult};
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use std::sync::Arc;

/// 批量任务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTaskConfig {
    pub max_concurrent_batch_tasks: usize,
    pub batch_size_limit: usize,
    pub enable_progressive_processing: bool,
    pub auto_split_large_batches: bool,
    pub retry_failed_items: bool,
    pub max_retries_per_item: u32,
}

impl Default for BatchTaskConfig {
    fn default() -> Self {
        Self {
            max_concurrent_batch_tasks: 5,
            batch_size_limit: 1000,
            enable_progressive_processing: true,
            auto_split_large_batches: true,
            retry_failed_items: true,
            max_retries_per_item: 3,
        }
    }
}

/// 批量任务类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchTaskType {
    BatchCompress,      // 批量压缩
    BatchExtract,       // 批量解压
    BatchCopy,          // 批量复制
    BatchMove,          // 批量移动
    BatchDelete,        // 批量删除
    BatchRename,        // 批量重命名
}

/// 批量任务请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTaskRequest {
    pub task_type: BatchTaskType,
    pub items: Vec<BatchItem>,
    pub destination: Option<String>,
    pub options: BatchTaskOptions,
    pub priority: TaskPriority,
}

/// 批量任务项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItem {
    pub id: String,
    pub source_path: String,
    pub metadata: serde_json::Value,
}

/// 批量任务选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTaskOptions {
    pub compression_format: Option<CompressionFormat>,
    pub compression_options: Option<CompressionOptions>,
    pub overwrite_existing: bool,
    pub preserve_structure: bool,
    pub create_subdirectories: bool,
    pub skip_errors: bool,
}

impl Default for BatchTaskOptions {
    fn default() -> Self {
        Self {
            compression_format: Some(CompressionFormat::Zip),
            compression_options: Some(CompressionOptions::default()),
            overwrite_existing: false,
            preserve_structure: true,
            create_subdirectories: true,
            skip_errors: false,
        }
    }
}

/// 批量任务结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTaskResult {
    pub task_id: String,
    pub total_items: usize,
    pub successful_items: usize,
    pub failed_items: usize,
    pub skipped_items: usize,
    pub item_results: Vec<BatchItemResult>,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
    pub total_duration_seconds: f64,
}

/// 批量任务项结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItemResult {
    pub item_id: String,
    pub source_path: String,
    pub success: bool,
    pub error_message: Option<String>,
    pub output_path: Option<String>,
    pub duration_seconds: f64,
    pub retry_count: u32,
}

/// 批量任务处理器
pub struct BatchTaskProcessor {
    task_manager: Arc<TaskManager>,
    file_service: FileService,
    config: BatchTaskConfig,
}

impl BatchTaskProcessor {
    /// 创建新的批量任务处理器
    pub fn new(task_manager: Arc<TaskManager>, file_service: FileService) -> Self {
        Self {
            task_manager,
            file_service,
            config: BatchTaskConfig::default(),
        }
    }

    /// 提交批量任务
    pub async fn submit_batch_task(&self, request: BatchTaskRequest) -> Result<String> {
        log::info!("提交批量任务: {:?}, 项目数: {}", request.task_type, request.items.len());

        // 验证请求
        self.validate_batch_request(&request)?;

        // 根据任务类型创建不同的任务
        let task_id = match request.task_type {
            BatchTaskType::BatchCompress => {
                self.create_batch_compression_task(request).await?
            }
            BatchTaskType::BatchExtract => {
                self.create_batch_extraction_task(request).await?
            }
            BatchTaskType::BatchCopy => {
                self.create_batch_copy_task(request).await?
            }
            BatchTaskType::BatchMove => {
                self.create_batch_move_task(request).await?
            }
            BatchTaskType::BatchDelete => {
                self.create_batch_delete_task(request).await?
            }
            BatchTaskType::BatchRename => {
                self.create_batch_rename_task(request).await?
            }
        };

        log::info!("批量任务已提交: {}", task_id);
        Ok(task_id)
    }

    /// 验证批量请求
    fn validate_batch_request(&self, request: &BatchTaskRequest) -> Result<()> {
        // 检查项目数量
        if request.items.is_empty() {
            return Err(anyhow!("批量任务不能为空"));
        }

        if request.items.len() > self.config.batch_size_limit {
            return Err(anyhow!(
                "批量任务项目数超过限制: {} > {}",
                request.items.len(),
                self.config.batch_size_limit
            ));
        }

        // 检查目标路径（如果需要）
        if let BatchTaskType::BatchCompress | BatchTaskType::BatchExtract |
            BatchTaskType::BatchCopy | BatchTaskType::BatchMove = request.task_type {

            if request.destination.is_none() {
                return Err(anyhow!("此任务类型需要目标路径"));
            }
        }

        Ok(())
    }

    /// 创建批量压缩任务
    async fn create_batch_compression_task(&self, request: BatchTaskRequest) -> Result<String> {
        let destination = request.destination
            .ok_or_else(|| anyhow!("批量压缩需要目标路径"))?;

        let format = request.options.compression_format
            .ok_or_else(|| anyhow!("批量压缩需要压缩格式"))?;

        let options = request.options.compression_options
            .unwrap_or_else(CompressionOptions::default);

        // 创建主压缩任务
        let source_files: Vec<String> = request.items.iter()
            .map(|item| item.source_path.clone())
            .collect();

        let task_id = self.task_manager.add_compression_task(
            source_files,
            destination,
            format,
            options,
            request.priority,
        ).await?;

        Ok(task_id)
    }

    /// 创建批量解压任务
    async fn create_batch_extraction_task(&self, request: BatchTaskRequest) -> Result<String> {
        // 对于批量解压，我们为每个文件创建单独的任务
        let mut task_ids = Vec::new();

        for item in request.items {
            let destination = request.destination.clone()
                .ok_or_else(|| anyhow!("批量解压需要目标路径"))?;

            let options = request.options.compression_options
                .clone()
                .unwrap_or_else(CompressionOptions::default);

            let task_id = self.task_manager.add_extraction_task(
                item.source_path,
                Some(destination.clone()),
                options.password.clone(),
                request.priority.clone(),
            ).await?;

            task_ids.push(task_id);
        }

        // 返回第一个任务的ID作为批量任务的代表
        task_ids.first()
            .cloned()
            .ok_or_else(|| anyhow!("没有创建任何任务"))
    }

    /// 创建批量复制任务
    async fn create_batch_copy_task(&self, request: BatchTaskRequest) -> Result<String> {
        let destination = request.destination
            .ok_or_else(|| anyhow!("批量复制需要目标路径"))?;

        // 创建自定义的批量复制任务
        let source_files: Vec<String> = request.items.iter()
            .map(|item| item.source_path.clone())
            .collect();

        // 这里需要创建一个自定义的任务类型
        // 简化实现：使用文件服务的批量复制功能
        let batch_result = self.file_service.batch_copy_files(
            &source_files,
            &destination,
        ).await?;

        // 创建任务记录
        let task_id = self.create_custom_batch_task(
            BatchTaskType::BatchCopy,
            request,
            batch_result,
        ).await?;

        Ok(task_id)
    }

    /// 创建批量移动任务
    async fn create_batch_move_task(&self, request: BatchTaskRequest) -> Result<String> {
        let destination = request.destination
            .ok_or_else(|| anyhow!("批量移动需要目标路径"))?;

        // 简化实现：先复制后删除
        let source_files: Vec<String> = request.items.iter()
            .map(|item| item.source_path.clone())
            .collect();

        // 批量复制
        let copy_result = self.file_service.batch_copy_files(
            &source_files,
            &destination,
        ).await?;

        // 批量删除源文件（仅复制成功的）
        let mut delete_files = Vec::new();
        for (i, item) in copy_result.items.iter().enumerate() {
            if item.success {
                delete_files.push(source_files[i].clone());
            }
        }

        if !delete_files.is_empty() {
            let _ = self.file_service.batch_delete_files(&delete_files).await;
        }

        // 创建任务记录
        let task_id = self.create_custom_batch_task(
            BatchTaskType::BatchMove,
            request,
            copy_result,
        ).await?;

        Ok(task_id)
    }

    /// 创建批量删除任务
    async fn create_batch_delete_task(&self, request: BatchTaskRequest) -> Result<String> {
        let source_files: Vec<String> = request.items.iter()
            .map(|item| item.source_path.clone())
            .collect();

        let batch_result = self.file_service.batch_delete_files(&source_files).await?;

        // 创建任务记录
        let task_id = self.create_custom_batch_task(
            BatchTaskType::BatchDelete,
            request,
            batch_result,
        ).await?;

        Ok(task_id)
    }

    /// 创建批量重命名任务
    async fn create_batch_rename_task(&self, request: BatchTaskRequest) -> Result<String> {
        // 批量重命名需要特殊的处理逻辑
        // 这里简化实现，实际需要更复杂的逻辑
        let mut batch_result = BatchOperationResult::new();

        for item in request.items {
            // 简化：这里需要实际的批量重命名逻辑
            batch_result.add_item(
                item.source_path.clone(),
                false, // 简化：假设失败
                Some("批量重命名功能待实现".to_string()),
            );
        }

        // 创建任务记录
        let task_id = self.create_custom_batch_task(
            BatchTaskType::BatchRename,
            request,
            batch_result,
        ).await?;

        Ok(task_id)
    }

    /// 创建自定义批量任务记录
    async fn create_custom_batch_task(
        &self,
        task_type: BatchTaskType,
        request: BatchTaskRequest,
        batch_result: BatchOperationResult,
    ) -> Result<String> {
        // 创建自定义的队列任务
        let task_id = uuid::Uuid::new_v4().to_string();

        // 创建压缩任务（作为载体）
        let compression_task = CompressionTask::new(
            vec!["batch_task".to_string()], // 虚拟源文件
            "batch_result".to_string(),     // 虚拟输出路径
            CompressionFormat::Zip,         // 虚拟格式
            CompressionOptions::default(),
        );

        let queue_task = QueueTask::new(
            TaskType::Compress, // 使用压缩任务类型作为载体
            request.priority,
            compression_task,
        );

        // 这里需要将任务添加到队列中
        // 简化实现：记录日志
        log::info!("创建自定义批量任务: {} ({:?})", task_id, task_type);

        // 记录批量结果
        self.record_batch_result(&task_id, &request, &batch_result).await?;

        Ok(task_id)
    }

    /// 记录批量任务结果
    async fn record_batch_result(
        &self,
        task_id: &str,
        request: &BatchTaskRequest,
        batch_result: &BatchOperationResult,
    ) -> Result<()> {
        // 创建结果记录
        let task_result = BatchTaskResult {
            task_id: task_id.to_string(),
            total_items: request.items.len(),
            successful_items: batch_result.successful_count(),
            failed_items: batch_result.failed_count(),
            skipped_items: batch_result.skipped_count(),
            item_results: request.items.iter().enumerate()
                .map(|(i, item)| {
                    let result_item = if i < batch_result.items.len() {
                        &batch_result.items[i]
                    } else {
                        // 默认结果
                        &BatchOperationItem {
                            path: item.source_path.clone(),
                            success: false,
                            error: Some("未处理".to_string()),
                        }
                    };

                    BatchItemResult {
                        item_id: item.id.clone(),
                        source_path: item.source_path.clone(),
                        success: result_item.success,
                        error_message: result_item.error.clone(),
                        output_path: None, // 需要根据任务类型设置
                        duration_seconds: 0.0, // 简化
                        retry_count: 0,
                    }
                })
                .collect(),
            start_time: chrono::Utc::now(),
            end_time: chrono::Utc::now(),
            total_duration_seconds: 0.0,
        };

        // 这里应该将结果保存到数据库或文件
        log::debug!("批量任务结果: {:?}", task_result);

        Ok(())
    }

    /// 获取批量任务状态
    pub async fn get_batch_task_status(&self, task_id: &str) -> Result<BatchTaskStatus> {
        // 尝试从任务管理器获取任务状态
        match self.task_manager.get_task_status(task_id).await {
            Ok(status) => {
                Ok(BatchTaskStatus::from_queue_status(&status))
            }
            Err(_) => {
                // 可能是自定义任务，检查是否有记录
                Ok(BatchTaskStatus::Unknown)
            }
        }
    }

    /// 获取批量任务结果
    pub async fn get_batch_task_result(&self, task_id: &str) -> Result<Option<BatchTaskResult>> {
        // 这里应该从数据库或文件加载结果
        // 简化实现：返回None
        Ok(None)
    }

    /// 取消批量任务
    pub async fn cancel_batch_task(&self, task_id: &str) -> Result<bool> {
        self.task_manager.cancel_task(task_id).await
    }

    /// 获取配置
    pub fn get_config(&self) -> &BatchTaskConfig {
        &self.config
    }

    /// 更新配置
    pub fn update_config(&mut self, new_config: BatchTaskConfig) {
        self.config = new_config;
    }

    /// 拆分大型批量任务
    pub fn split_large_batch(&self, request: &BatchTaskRequest) -> Vec<BatchTaskRequest> {
        if !self.config.auto_split_large_batches {
            return vec![request.clone()];
        }

        let batch_size = self.config.batch_size_limit;
        let mut batches = Vec::new();

        for chunk in request.items.chunks(batch_size) {
            let mut batch_request = request.clone();
            batch_request.items = chunk.to_vec();
            batches.push(batch_request);
        }

        batches
    }
}

/// 批量任务状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchTaskStatus {
    Pending,      // 等待中
    Preparing,    // 准备中
    Processing,   // 处理中
    Completed,    // 已完成
    PartiallyCompleted, // 部分完成
    Failed,       // 失败
    Cancelled,    // 已取消
    Unknown,      // 未知
}

impl BatchTaskStatus {
    /// 从队列任务状态转换
    pub fn from_queue_status(status: &crate::task_queue::models::QueueTaskStatus) -> Self {
        match status {
            crate::task_queue::models::QueueTaskStatus::Queued => BatchTaskStatus::Pending,
            crate::task_queue::models::QueueTaskStatus::Scheduled => BatchTaskStatus::Preparing,
            crate::task_queue::models::QueueTaskStatus::Running => BatchTaskStatus::Processing,
            crate::task_queue::models::QueueTaskStatus::Completed => BatchTaskStatus::Completed,
            crate::task_queue::models::QueueTaskStatus::Failed => BatchTaskStatus::Failed,
            crate::task_queue::models::QueueTaskStatus::Cancelled => BatchTaskStatus::Cancelled,
            crate::task_queue::models::QueueTaskStatus::Paused => BatchTaskStatus::Processing,
        }
    }
}

/// 批量任务进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTaskProgress {
    pub task_id: String,
    pub status: BatchTaskStatus,
    pub total_items: usize,
    pub processed_items: usize,
    pub successful_items: usize,
    pub failed_items: usize,
    pub current_item: Option<String>,
    pub progress_percentage: f32,
    pub estimated_time_remaining: Option<f64>, // 秒
}

/// 批量任务统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTaskStatistics {
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub cancelled_tasks: usize,
    pub total_items_processed: usize,
    pub average_items_per_task: f32,
    pub success_rate: f32,
    pub average_duration_seconds: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_queue::task_manager::TaskManager;

    #[test]
    fn test_batch_task_config_default() {
        let config = BatchTaskConfig::default();

        assert_eq!(config.max_concurrent_batch_tasks, 5);
        assert_eq!(config.batch_size_limit, 1000);
        assert!(config.enable_progressive_processing);
        assert!(config.auto_split_large_batches);
        assert!(config.retry_failed_items);
        assert_eq!(config.max_retries_per_item, 3);
    }

    #[test]
    fn test_batch_task_request() {
        let request = BatchTaskRequest {
            task_type: BatchTaskType::BatchCompress,
            items: vec![
                BatchItem {
                    id: "item1".to_string(),
                    source_path: "/path/to/file1.txt".to_string(),
                    metadata: serde_json::json!({}),
                },
                BatchItem {
                    id: "item2".to_string(),
                    source_path: "/path/to/file2.txt".to_string(),
                    metadata: serde_json::json!({}),
                },
            ],
            destination: Some("/output/path".to_string()),
            options: BatchTaskOptions::default(),
            priority: TaskPriority::Medium,
        };

        assert_eq!(request.items.len(), 2);
        assert_eq!(request.task_type, BatchTaskType::BatchCompress);
        assert!(request.destination.is_some());
    }

    #[test]
    fn test_batch_task_status_conversion() {
        use crate::task_queue::models::QueueTaskStatus;

        assert_eq!(
            BatchTaskStatus::from_queue_status(&QueueTaskStatus::Queued),
            BatchTaskStatus::Pending
        );
        assert_eq!(
            BatchTaskStatus::from_queue_status(&QueueTaskStatus::Running),
            BatchTaskStatus::Processing
        );
        assert_eq!(
            BatchTaskStatus::from_queue_status(&QueueTaskStatus::Completed),
            BatchTaskStatus::Completed
        );
        assert_eq!(
            BatchTaskStatus::from_queue_status(&QueueTaskStatus::Failed),
            BatchTaskStatus::Failed
        );
    }

    #[test]
    fn test_split_large_batch() {
        let config = BatchTaskConfig {
            batch_size_limit: 3,
            auto_split_large_batches: true,
            ..Default::default()
        };

        let processor = BatchTaskProcessor {
            task_manager: Arc::new(TaskManager::new()),
            file_service: FileService::new(crate::services::file_service::FileServiceConfig::default()),
            config,
        };

        let request = BatchTaskRequest {
            task_type: BatchTaskType::BatchCompress,
            items: (0..10)
                .map(|i| BatchItem {
                    id: format!("item{}", i),
                    source_path: format!("/path/to/file{}.txt", i),
                    metadata: serde_json::json!({}),
                })
                .collect(),
            destination: Some("/output/path".to_string()),
            options: BatchTaskOptions::default(),
            priority: TaskPriority::Medium,
        };

        let batches = processor.split_large_batch(&request);

        // 10个项目，每批3个，应该分成4批（3+3+3+1）
        assert_eq!(batches.len(), 4);
        assert_eq!(batches[0].items.len(), 3);
        assert_eq!(batches[3].items.len(), 1);
    }
}