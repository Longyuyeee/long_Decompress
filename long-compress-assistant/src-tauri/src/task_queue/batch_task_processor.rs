use crate::task_queue::models::TaskPriority;
use crate::models::compression::CompressionOptions;
use crate::services::file_service::{BatchOperationItem, BatchItemResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchTaskType {
    BatchCompress,
    BatchExtract,
    BatchCopy,
    BatchMove,
    BatchDelete,
    BatchHash,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTaskRequest {
    pub task_type: BatchTaskType,
    pub items: Vec<BatchOperationItem>,
    pub options: Option<CompressionOptions>,
    pub priority: TaskPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTaskResult {
    pub task_id: String,
    pub success: bool,
    pub message: String,
    pub items: Vec<BatchItemResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTaskProgress {
    pub task_id: String,
    pub total_items: usize,
    pub processed_items: usize,
    pub current_item: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchTaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTaskConfig {
    pub max_concurrent_batch_tasks: usize,
    pub batch_size_limit: usize,
}

impl Default for BatchTaskConfig {
    fn default() -> Self {
        Self {
            max_concurrent_batch_tasks: 2,
            batch_size_limit: 100,
        }
    }
}

pub struct BatchTaskProcessor {
    config: Arc<RwLock<BatchTaskConfig>>,
}

impl BatchTaskProcessor {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(BatchTaskConfig::default())),
        }
    }

    pub async fn process_batch_task(&self, request: BatchTaskRequest) -> Result<BatchTaskResult> {
        let task_id = Uuid::new_v4().to_string();

        let mut item_results = Vec::new();
        for item in &request.items {
            let result = self.process_item(&request.task_type, item).await;
            item_results.push(result);
        }

        let all_success = item_results.iter().all(|r| r.success);
        let success_count = item_results.iter().filter(|r| r.success).count();
        Ok(BatchTaskResult {
            task_id,
            success: all_success,
            message: if all_success {
                format!("批量处理完成，共 {} 项", item_results.len())
            } else {
                format!("批量处理完成，{}/{} 项成功", success_count, item_results.len())
            },
            items: item_results,
        })
    }

    async fn process_item(&self, task_type: &BatchTaskType, item: &BatchOperationItem) -> BatchItemResult {
        match task_type {
            BatchTaskType::BatchCompress | BatchTaskType::BatchExtract => {
                // 压缩/解压类操作应通过 add_compression_task / add_extraction_task 提交到任务队列
                // 批量处理器仅处理直接的文件操作
                BatchItemResult {
                    source: item.source.clone(),
                    success: false,
                    error: Some(format!(
                        "{:?} 操作请通过任务队列提交 (add_{}_task)",
                        task_type,
                        if matches!(task_type, BatchTaskType::BatchCompress) { "compression" } else { "extraction" }
                    )),
                }
            }
            BatchTaskType::BatchCopy => {
                let dest = format!("{}_copy", item.source);
                match std::fs::copy(&item.source, &dest) {
                    Ok(_bytes) => BatchItemResult {
                        source: item.source.clone(),
                        success: true,
                        error: None,
                    },
                    Err(e) => BatchItemResult {
                        source: item.source.clone(),
                        success: false,
                        error: Some(e.to_string()),
                    },
                }
            }
            BatchTaskType::BatchDelete => {
                match std::fs::remove_file(&item.source) {
                    Ok(_) => BatchItemResult {
                        source: item.source.clone(),
                        success: true,
                        error: None,
                    },
                    Err(e) => BatchItemResult {
                        source: item.source.clone(),
                        success: false,
                        error: Some(e.to_string()),
                    },
                }
            }
            _ => BatchItemResult {
                source: item.source.clone(),
                success: false,
                error: Some(format!("不支持的批量操作类型: {:?}", task_type)),
            },
        }
    }
}
