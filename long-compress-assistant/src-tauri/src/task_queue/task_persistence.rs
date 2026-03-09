//! 任务持久化模块
//!
//! 提供任务队列的持久化存储功能，支持任务状态的保存和恢复。

use crate::task_queue::models::{QueueTask, QueueTaskStatus, TaskType, TaskPriority};
use crate::models::compression::{CompressionTask, CompressionFormat, CompressionOptions};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};

/// 持久化任务数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentTask {
    pub id: String,
    pub task_type: TaskType,
    pub priority: TaskPriority,
    pub status: QueueTaskStatus,
    pub compression_task: PersistentCompressionTask,
    pub created_at: DateTime<Utc>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub error_message: Option<String>,
}

/// 持久化压缩任务数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentCompressionTask {
    pub source_files: Vec<String>,
    pub output_path: String,
    pub format: CompressionFormat,
    pub options: CompressionOptions,
    pub status: String,
    pub progress: f32,
    pub total_size: u64,
    pub processed_size: u64,
    pub error_message: Option<String>,
}

impl From<&QueueTask> for PersistentTask {
    fn from(task: &QueueTask) -> Self {
        Self {
            id: task.id.clone(),
            task_type: task.task_type.clone(),
            priority: task.priority.clone(),
            status: task.status.clone(),
            compression_task: PersistentCompressionTask::from(&task.compression_task),
            created_at: task.created_at,
            scheduled_at: task.scheduled_at,
            started_at: task.started_at,
            completed_at: task.completed_at,
            retry_count: task.retry_count,
            max_retries: task.max_retries,
            error_message: task.error_message.clone(),
        }
    }
}

impl From<&CompressionTask> for PersistentCompressionTask {
    fn from(task: &CompressionTask) -> Self {
        Self {
            source_files: task.source_files.clone(),
            output_path: task.output_path.clone(),
            format: task.format.clone(),
            options: task.options.clone(),
            status: format!("{:?}", task.status),
            progress: task.progress,
            total_size: task.total_size,
            processed_size: task.processed_size,
            error_message: task.error_message.clone(),
        }
    }
}

impl TryFrom<PersistentTask> for QueueTask {
    type Error = anyhow::Error;

    fn try_from(persistent: PersistentTask) -> Result<Self> {
        let compression_task = CompressionTask::try_from(persistent.compression_task)?;

        Ok(QueueTask {
            id: persistent.id,
            task_type: persistent.task_type,
            priority: persistent.priority,
            status: persistent.status,
            compression_task,
            created_at: persistent.created_at,
            scheduled_at: persistent.scheduled_at,
            started_at: persistent.started_at,
            completed_at: persistent.completed_at,
            retry_count: persistent.retry_count,
            max_retries: persistent.max_retries,
            error_message: persistent.error_message,
        })
    }
}

impl TryFrom<PersistentCompressionTask> for CompressionTask {
    type Error = anyhow::Error;

    fn try_from(persistent: PersistentCompressionTask) -> Result<Self> {
        let mut task = CompressionTask::new(
            persistent.source_files,
            persistent.output_path,
            persistent.format,
            persistent.options,
        );

        // 恢复状态
        task.status = match persistent.status.as_str() {
            "Pending" => crate::models::compression::CompressionStatus::Pending,
            "Running" => crate::models::compression::CompressionStatus::Running,
            "Completed" => crate::models::compression::CompressionStatus::Completed,
            "Failed" => crate::models::compression::CompressionStatus::Failed,
            "Cancelled" => crate::models::compression::CompressionStatus::Cancelled,
            _ => crate::models::compression::CompressionStatus::Pending,
        };

        task.progress = persistent.progress;
        task.total_size = persistent.total_size;
        task.processed_size = persistent.processed_size;
        task.error_message = persistent.error_message;

        Ok(task)
    }
}

/// 任务持久化管理器
pub struct TaskPersistenceManager {
    storage_dir: PathBuf,
}

impl TaskPersistenceManager {
    /// 创建新的持久化管理器
    pub fn new(storage_dir: PathBuf) -> Result<Self> {
        // 确保存储目录存在
        if !storage_dir.exists() {
            fs::create_dir_all(&storage_dir)?;
        }

        Ok(Self { storage_dir })
    }

    /// 获取默认存储目录
    pub fn default_storage_dir() -> Result<PathBuf> {
        let mut dir = dirs::data_dir()
            .ok_or_else(|| anyhow!("无法获取数据目录"))?;

        dir.push("long-compress-assistant");
        dir.push("tasks");

        Ok(dir)
    }

    /// 保存任务
    pub fn save_task(&self, task: &QueueTask) -> Result<()> {
        let persistent_task = PersistentTask::from(task);
        let task_file = self.get_task_file_path(&task.id);

        // 序列化为JSON
        let json = serde_json::to_string_pretty(&persistent_task)?;

        // 写入文件
        fs::write(task_file, json)?;

        log::debug!("任务已保存: {}", task.id);
        Ok(())
    }

    /// 加载任务
    pub fn load_task(&self, task_id: &str) -> Result<QueueTask> {
        let task_file = self.get_task_file_path(task_id);

        if !task_file.exists() {
            return Err(anyhow!("任务文件不存在: {}", task_id));
        }

        // 读取文件
        let json = fs::read_to_string(task_file)?;

        // 反序列化
        let persistent_task: PersistentTask = serde_json::from_str(&json)?;

        // 转换为QueueTask
        let task = QueueTask::try_from(persistent_task)?;

        log::debug!("任务已加载: {}", task_id);
        Ok(task)
    }

    /// 删除任务
    pub fn delete_task(&self, task_id: &str) -> Result<()> {
        let task_file = self.get_task_file_path(task_id);

        if task_file.exists() {
            fs::remove_file(task_file)?;
            log::debug!("任务已删除: {}", task_id);
        }

        Ok(())
    }

    /// 获取所有已保存的任务ID
    pub fn list_saved_tasks(&self) -> Result<Vec<String>> {
        let mut task_ids = Vec::new();

        if self.storage_dir.exists() {
            for entry in fs::read_dir(&self.storage_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                    if let Some(file_name) = path.file_stem() {
                        if let Some(task_id) = file_name.to_str() {
                            task_ids.push(task_id.to_string());
                        }
                    }
                }
            }
        }

        Ok(task_ids)
    }

    /// 批量保存任务
    pub fn save_tasks(&self, tasks: &[&QueueTask]) -> Result<()> {
        for task in tasks {
            self.save_task(task)?;
        }
        log::info!("已保存 {} 个任务", tasks.len());
        Ok(())
    }

    /// 批量加载任务
    pub fn load_all_tasks(&self) -> Result<Vec<QueueTask>> {
        let task_ids = self.list_saved_tasks()?;
        let mut tasks = Vec::new();

        for task_id in task_ids {
            match self.load_task(&task_id) {
                Ok(task) => tasks.push(task),
                Err(e) => log::warn!("加载任务失败 {}: {}", task_id, e),
            }
        }

        log::info!("已加载 {} 个任务", tasks.len());
        Ok(tasks)
    }

    /// 清理已完成的任务
    pub fn cleanup_completed_tasks(&self, older_than_days: u32) -> Result<usize> {
        let cutoff_time = Utc::now() - chrono::Duration::days(older_than_days as i64);
        let task_ids = self.list_saved_tasks()?;
        let mut removed_count = 0;

        for task_id in task_ids {
            match self.load_task(&task_id) {
                Ok(task) => {
                    // 检查任务是否已完成且超过指定时间
                    if task.status.is_finished() {
                        if let Some(completed_at) = task.completed_at {
                            if completed_at < cutoff_time {
                                self.delete_task(&task_id)?;
                                removed_count += 1;
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("检查任务失败 {}: {}", task_id, e);
                    // 如果无法加载，也尝试删除损坏的文件
                    self.delete_task(&task_id)?;
                    removed_count += 1;
                }
            }
        }

        log::info!("清理了 {} 个已完成的任务", removed_count);
        Ok(removed_count)
    }

    /// 获取任务文件路径
    fn get_task_file_path(&self, task_id: &str) -> PathBuf {
        let mut path = self.storage_dir.clone();
        path.push(format!("{}.json", task_id));
        path
    }
}

/// 任务持久化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    pub enabled: bool,
    pub auto_save: bool,
    pub save_interval_seconds: u64,
    pub max_stored_tasks: usize,
    pub cleanup_days: u32,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_save: true,
            save_interval_seconds: 30,
            max_stored_tasks: 1000,
            cleanup_days: 30,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::compression::{CompressionFormat, CompressionOptions};
    use tempfile::tempdir;

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

    #[test]
    fn test_task_conversion() {
        let original_task = create_test_task();
        let persistent_task = PersistentTask::from(&original_task);

        // 测试序列化/反序列化
        let json = serde_json::to_string(&persistent_task).unwrap();
        let deserialized: PersistentTask = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, original_task.id);
        assert_eq!(deserialized.task_type, original_task.task_type);

        // 测试转换回QueueTask
        let restored_task = QueueTask::try_from(deserialized).unwrap();
        assert_eq!(restored_task.id, original_task.id);
        assert_eq!(restored_task.task_type, original_task.task_type);
    }

    #[test]
    fn test_persistence_manager() {
        let temp_dir = tempdir().unwrap();
        let manager = TaskPersistenceManager::new(temp_dir.path().to_path_buf()).unwrap();

        // 创建测试任务
        let task = create_test_task();
        let task_id = task.id.clone();

        // 测试保存
        manager.save_task(&task).unwrap();

        // 测试加载
        let loaded_task = manager.load_task(&task_id).unwrap();
        assert_eq!(loaded_task.id, task_id);

        // 测试列出任务
        let task_ids = manager.list_saved_tasks().unwrap();
        assert!(task_ids.contains(&task_id));

        // 测试删除
        manager.delete_task(&task_id).unwrap();
        let task_ids_after = manager.list_saved_tasks().unwrap();
        assert!(!task_ids_after.contains(&task_id));
    }

    #[test]
    fn test_batch_operations() {
        let temp_dir = tempdir().unwrap();
        let manager = TaskPersistenceManager::new(temp_dir.path().to_path_buf()).unwrap();

        // 创建多个测试任务
        let tasks: Vec<QueueTask> = (0..3)
            .map(|i| {
                let mut task = create_test_task();
                task.id = format!("test-task-{}", i);
                task
            })
            .collect();

        // 批量保存
        let task_refs: Vec<&QueueTask> = tasks.iter().collect();
        manager.save_tasks(&task_refs).unwrap();

        // 批量加载
        let loaded_tasks = manager.load_all_tasks().unwrap();
        assert_eq!(loaded_tasks.len(), 3);

        // 验证加载的任务
        for loaded_task in loaded_tasks {
            assert!(loaded_task.id.starts_with("test-task-"));
        }
    }
}