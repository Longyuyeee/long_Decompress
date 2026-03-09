//! 任务事件日志模块
//!
//! 提供任务事件的记录、查询和分析功能。

use crate::task_queue::models::{QueueTaskStatus, TaskType, TaskPriority};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// 任务事件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskEventType {
    TaskCreated,      // 任务创建
    TaskQueued,       // 任务入队
    TaskScheduled,    // 任务调度
    TaskStarted,      // 任务开始执行
    TaskProgress,     // 任务进度更新
    TaskPaused,       // 任务暂停
    TaskResumed,      // 任务恢复
    TaskCompleted,    // 任务完成
    TaskFailed,       // 任务失败
    TaskCancelled,    // 任务取消
    TaskRetried,      // 任务重试
    TaskStatusChanged, // 任务状态变更
    TaskPriorityChanged, // 任务优先级变更
    TaskError,        // 任务错误
    TaskWarning,      // 任务警告
    TaskInfo,         // 任务信息
}

/// 任务事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvent {
    pub id: String,
    pub task_id: String,
    pub event_type: TaskEventType,
    pub timestamp: DateTime<Utc>,
    pub details: serde_json::Value,
    pub metadata: HashMap<String, String>,
}

impl TaskEvent {
    /// 创建新的事件
    pub fn new(task_id: &str, event_type: TaskEventType, details: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: task_id.to_string(),
            event_type,
            timestamp: Utc::now(),
            details,
            metadata: HashMap::new(),
        }
    }

    /// 添加元数据
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    /// 创建任务创建事件
    pub fn task_created(task_id: &str, task_type: TaskType, priority: TaskPriority) -> Self {
        let details = serde_json::json!({
            "task_type": format!("{:?}", task_type),
            "priority": format!("{:?}", priority),
        });

        Self::new(task_id, TaskEventType::TaskCreated, details)
    }

    /// 创建任务状态变更事件
    pub fn status_changed(task_id: &str, old_status: QueueTaskStatus, new_status: QueueTaskStatus) -> Self {
        let details = serde_json::json!({
            "old_status": format!("{:?}", old_status),
            "new_status": format!("{:?}", new_status),
        });

        Self::new(task_id, TaskEventType::TaskStatusChanged, details)
    }

    /// 创建任务进度事件
    pub fn progress_updated(task_id: &str, progress: f32, processed: u64, total: u64) -> Self {
        let details = serde_json::json!({
            "progress": progress,
            "processed_bytes": processed,
            "total_bytes": total,
            "percentage": format!("{:.1}%", progress),
        });

        Self::new(task_id, TaskEventType::TaskProgress, details)
    }

    /// 创建任务错误事件
    pub fn error(task_id: &str, error_message: &str, error_code: Option<&str>) -> Self {
        let mut details = serde_json::json!({
            "error_message": error_message,
        });

        if let Some(code) = error_code {
            details["error_code"] = serde_json::Value::String(code.to_string());
        }

        Self::new(task_id, TaskEventType::TaskError, details)
    }

    /// 创建任务完成事件
    pub fn completed(task_id: &str, success: bool, duration_seconds: f64) -> Self {
        let details = serde_json::json!({
            "success": success,
            "duration_seconds": duration_seconds,
            "completion_time": Utc::now().to_rfc3339(),
        });

        let event_type = if success {
            TaskEventType::TaskCompleted
        } else {
            TaskEventType::TaskFailed
        };

        Self::new(task_id, event_type, details)
    }
}

/// 任务事件过滤器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEventFilter {
    pub task_id: Option<String>,
    pub event_type: Option<TaskEventType>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub contains_text: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl Default for TaskEventFilter {
    fn default() -> Self {
        Self {
            task_id: None,
            event_type: None,
            start_time: None,
            end_time: None,
            contains_text: None,
            limit: Some(100),
            offset: Some(0),
        }
    }
}

/// 任务事件统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEventStats {
    pub total_events: usize,
    pub events_by_type: HashMap<String, usize>,
    pub events_by_task: HashMap<String, usize>,
    pub earliest_event: Option<DateTime<Utc>>,
    pub latest_event: Option<DateTime<Utc>>,
    pub error_count: usize,
    pub warning_count: usize,
}

/// 任务事件日志管理器
pub struct TaskEventLogger {
    log_dir: PathBuf,
    max_events_per_file: usize,
    current_file_index: usize,
    current_event_count: usize,
}

impl TaskEventLogger {
    /// 创建新的事件日志管理器
    pub fn new(log_dir: PathBuf) -> Result<Self> {
        // 确保日志目录存在
        if !log_dir.exists() {
            fs::create_dir_all(&log_dir)?;
        }

        // 查找最新的日志文件
        let (current_file_index, current_event_count) = Self::find_latest_log_file(&log_dir)?;

        Ok(Self {
            log_dir,
            max_events_per_file: 1000,
            current_file_index,
            current_event_count,
        })
    }

    /// 获取默认日志目录
    pub fn default_log_dir() -> Result<PathBuf> {
        let mut dir = dirs::data_dir()
            .ok_or_else(|| anyhow!("无法获取数据目录"))?;

        dir.push("long-compress-assistant");
        dir.push("task-events");

        Ok(dir)
    }

    /// 记录事件
    pub fn log_event(&mut self, event: &TaskEvent) -> Result<()> {
        // 检查是否需要创建新文件
        if self.current_event_count >= self.max_events_per_file {
            self.current_file_index += 1;
            self.current_event_count = 0;
        }

        // 获取当前日志文件路径
        let log_file = self.get_log_file_path(self.current_file_index);

        // 序列化事件
        let event_json = serde_json::to_string(event)?;

        // 追加到日志文件
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)?;

        use std::io::Write;
        writeln!(file, "{}", event_json)?;

        self.current_event_count += 1;

        log::debug!("事件已记录: {} ({})", event.event_type, event.task_id);
        Ok(())
    }

    /// 批量记录事件
    pub fn log_events(&mut self, events: &[TaskEvent]) -> Result<()> {
        for event in events {
            self.log_event(event)?;
        }
        Ok(())
    }

    /// 查询事件
    pub fn query_events(&self, filter: &TaskEventFilter) -> Result<Vec<TaskEvent>> {
        let mut all_events = Vec::new();

        // 获取所有日志文件
        let log_files = self.get_log_files()?;

        // 按时间倒序读取文件（最新的在前面）
        for log_file in log_files.iter().rev() {
            let events = self.read_log_file(log_file)?;
            all_events.extend(events);

            // 如果已经收集了足够的事件，提前停止
            if let Some(limit) = filter.limit {
                if all_events.len() >= limit + filter.offset.unwrap_or(0) {
                    break;
                }
            }
        }

        // 应用过滤器
        let filtered_events = self.apply_filter(&all_events, filter);

        // 应用分页
        let offset = filter.offset.unwrap_or(0);
        let limit = filter.limit.unwrap_or(100);

        let paginated_events = filtered_events
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();

        Ok(paginated_events)
    }

    /// 获取事件统计信息
    pub fn get_event_stats(&self) -> Result<TaskEventStats> {
        let all_events = self.query_events(&TaskEventFilter::default())?;

        let mut stats = TaskEventStats {
            total_events: all_events.len(),
            events_by_type: HashMap::new(),
            events_by_task: HashMap::new(),
            earliest_event: None,
            latest_event: None,
            error_count: 0,
            warning_count: 0,
        };

        for event in &all_events {
            // 按类型统计
            let type_key = format!("{:?}", event.event_type);
            *stats.events_by_type.entry(type_key).or_insert(0) += 1;

            // 按任务统计
            *stats.events_by_task.entry(event.task_id.clone()).or_insert(0) += 1;

            // 统计错误和警告
            match event.event_type {
                TaskEventType::TaskError => stats.error_count += 1,
                TaskEventType::TaskWarning => stats.warning_count += 1,
                _ => {}
            }

            // 更新最早和最晚时间
            if stats.earliest_event.is_none() || event.timestamp < stats.earliest_event.unwrap() {
                stats.earliest_event = Some(event.timestamp);
            }
            if stats.latest_event.is_none() || event.timestamp > stats.latest_event.unwrap() {
                stats.latest_event = Some(event.timestamp);
            }
        }

        Ok(stats)
    }

    /// 获取任务的事件历史
    pub fn get_task_event_history(&self, task_id: &str) -> Result<Vec<TaskEvent>> {
        let filter = TaskEventFilter {
            task_id: Some(task_id.to_string()),
            ..Default::default()
        };

        self.query_events(&filter)
    }

    /// 清理旧的事件日志
    pub fn cleanup_old_logs(&self, older_than_days: u32) -> Result<usize> {
        let cutoff_time = Utc::now() - chrono::Duration::days(older_than_days as i64);
        let log_files = self.get_log_files()?;
        let mut removed_count = 0;

        for log_file in log_files {
            // 检查文件修改时间
            if let Ok(metadata) = fs::metadata(&log_file) {
                if let Ok(modified) = metadata.modified() {
                    let modified_time: DateTime<Utc> = modified.into();

                    if modified_time < cutoff_time {
                        // 检查文件中是否有新于截止时间的事件
                        let events = self.read_log_file(&log_file)?;
                        let has_recent_events = events.iter()
                            .any(|event| event.timestamp >= cutoff_time);

                        if !has_recent_events {
                            fs::remove_file(&log_file)?;
                            removed_count += 1;
                            log::debug!("删除旧日志文件: {:?}", log_file);
                        }
                    }
                }
            }
        }

        log::info!("清理了 {} 个旧日志文件", removed_count);
        Ok(removed_count)
    }

    /// 查找最新的日志文件
    fn find_latest_log_file(log_dir: &PathBuf) -> Result<(usize, usize)> {
        let mut max_index = 0;
        let mut event_count = 0;

        if log_dir.exists() {
            for entry in fs::read_dir(log_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() {
                    if let Some(file_name) = path.file_stem() {
                        if let Some(name_str) = file_name.to_str() {
                            if let Some(index_str) = name_str.strip_prefix("events_") {
                                if let Ok(index) = index_str.parse::<usize>() {
                                    if index > max_index {
                                        max_index = index;

                                        // 计算当前文件中的事件数量
                                        event_count = Self::count_events_in_file(&path)?;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((max_index, event_count))
    }

    /// 获取日志文件路径
    fn get_log_file_path(&self, index: usize) -> PathBuf {
        let mut path = self.log_dir.clone();
        path.push(format!("events_{:04}.log", index));
        path
    }

    /// 获取所有日志文件
    fn get_log_files(&self) -> Result<Vec<PathBuf>> {
        let mut log_files = Vec::new();

        if self.log_dir.exists() {
            for entry in fs::read_dir(&self.log_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() {
                    if let Some(file_name) = path.file_stem() {
                        if let Some(name_str) = file_name.to_str() {
                            if name_str.starts_with("events_") {
                                log_files.push(path);
                            }
                        }
                    }
                }
            }
        }

        // 按索引排序
        log_files.sort();

        Ok(log_files)
    }

    /// 读取日志文件
    fn read_log_file(&self, log_file: &PathBuf) -> Result<Vec<TaskEvent>> {
        let mut events = Vec::new();

        if !log_file.exists() {
            return Ok(events);
        }

        let content = fs::read_to_string(log_file)?;

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<TaskEvent>(line) {
                Ok(event) => events.push(event),
                Err(e) => {
                    log::warn!("解析事件日志行失败: {} - {}", e, line);
                }
            }
        }

        Ok(events)
    }

    /// 计算文件中的事件数量
    fn count_events_in_file(log_file: &PathBuf) -> Result<usize> {
        if !log_file.exists() {
            return Ok(0);
        }

        let content = fs::read_to_string(log_file)?;
        let count = content.lines().filter(|line| !line.trim().is_empty()).count();

        Ok(count)
    }

    /// 应用事件过滤器
    fn apply_filter(&self, events: &[TaskEvent], filter: &TaskEventFilter) -> Vec<TaskEvent> {
        events.iter()
            .filter(|event| {
                // 任务ID过滤
                if let Some(task_id_filter) = &filter.task_id {
                    if &event.task_id != task_id_filter {
                        return false;
                    }
                }

                // 事件类型过滤
                if let Some(event_type_filter) = &filter.event_type {
                    if &event.event_type != event_type_filter {
                        return false;
                    }
                }

                // 时间范围过滤
                if let Some(start_time) = &filter.start_time {
                    if event.timestamp < *start_time {
                        return false;
                    }
                }

                if let Some(end_time) = &filter.end_time {
                    if event.timestamp > *end_time {
                        return false;
                    }
                }

                // 文本搜索过滤
                if let Some(search_text) = &filter.contains_text {
                    let search_text_lower = search_text.to_lowercase();

                    // 搜索事件类型
                    let event_type_str = format!("{:?}", event.event_type).to_lowercase();
                    if !event_type_str.contains(&search_text_lower) {
                        // 搜索任务ID
                        if !event.task_id.to_lowercase().contains(&search_text_lower) {
                            // 搜索详细信息
                            let details_str = event.details.to_string().to_lowercase();
                            if !details_str.contains(&search_text_lower) {
                                return false;
                            }
                        }
                    }
                }

                true
            })
            .cloned()
            .collect()
    }
}

/// 全局事件日志管理器
pub struct GlobalEventLogger {
    logger: std::sync::Arc<tokio::sync::RwLock<Option<TaskEventLogger>>>,
}

impl GlobalEventLogger {
    /// 创建新的全局事件日志管理器
    pub fn new() -> Self {
        Self {
            logger: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// 初始化全局事件日志管理器
    pub async fn initialize(&self) -> Result<()> {
        let mut logger_guard = self.logger.write().await;

        if logger_guard.is_none() {
            let log_dir = TaskEventLogger::default_log_dir()?;
            let logger = TaskEventLogger::new(log_dir)?;
            *logger_guard = Some(logger);
            log::info!("全局事件日志管理器初始化完成");
        }

        Ok(())
    }

    /// 记录事件
    pub async fn log_event(&self, event: TaskEvent) -> Result<()> {
        let logger_guard = self.logger.read().await;

        if let Some(logger) = &*logger_guard {
            let mut logger = logger.clone();
            tokio::task::spawn_blocking(move || logger.log_event(&event))
                .await?
        } else {
            Err(anyhow!("事件日志管理器未初始化"))
        }
    }

    /// 查询事件
    pub async fn query_events(&self, filter: TaskEventFilter) -> Result<Vec<TaskEvent>> {
        let logger_guard = self.logger.read().await;

        if let Some(logger) = &*logger_guard {
            let logger = logger.clone();
            tokio::task::spawn_blocking(move || logger.query_events(&filter))
                .await?
        } else {
            Err(anyhow!("事件日志管理器未初始化"))
        }
    }

    /// 获取事件统计信息
    pub async fn get_event_stats(&self) -> Result<TaskEventStats> {
        let logger_guard = self.logger.read().await;

        if let Some(logger) = &*logger_guard {
            let logger = logger.clone();
            tokio::task::spawn_blocking(move || logger.get_event_stats())
                .await?
        } else {
            Err(anyhow!("事件日志管理器未初始化"))
        }
    }
}

// 实现默认的全局实例
lazy_static::lazy_static! {
    pub static ref EVENT_LOGGER: GlobalEventLogger = GlobalEventLogger::new();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_task_event_creation() {
        let event = TaskEvent::task_created(
            "test-task-123",
            TaskType::Compress,
            TaskPriority::High,
        );

        assert_eq!(event.task_id, "test-task-123");
        assert_eq!(event.event_type, TaskEventType::TaskCreated);

        let details: serde_json::Value = serde_json::from_str(&event.details.to_string()).unwrap();
        assert_eq!(details["task_type"], "Compress");
        assert_eq!(details["priority"], "High");
    }

    #[test]
    fn test_event_logger_basic() {
        let temp_dir = tempdir().unwrap();
        let mut logger = TaskEventLogger::new(temp_dir.path().to_path_buf()).unwrap();

        // 创建测试事件
        let event = TaskEvent::task_created(
            "test-task-123",
            TaskType::Compress,
            TaskPriority::Medium,
        );

        // 记录事件
        logger.log_event(&event).unwrap();

        // 查询事件
        let filter = TaskEventFilter::default();
        let events = logger.query_events(&filter).unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].task_id, "test-task-123");
        assert_eq!(events[0].event_type, TaskEventType::TaskCreated);
    }

    #[test]
    fn test_event_filtering() {
        let temp_dir = tempdir().unwrap();
        let mut logger = TaskEventLogger::new(temp_dir.path().to_path_buf()).unwrap();

        // 创建多个测试事件
        let events = vec![
            TaskEvent::task_created("task-1", TaskType::Compress, TaskPriority::High),
            TaskEvent::task_created("task-2", TaskType::Extract, TaskPriority::Medium),
            TaskEvent::completed("task-1", true, 10.5),
        ];

        for event in &events {
            logger.log_event(event).unwrap();
        }

        // 按任务ID过滤
        let filter = TaskEventFilter {
            task_id: Some("task-1".to_string()),
            ..Default::default()
        };
        let filtered_events = logger.query_events(&filter).unwrap();
        assert_eq!(filtered_events.len(), 2);

        // 按事件类型过滤
        let filter = TaskEventFilter {
            event_type: Some(TaskEventType::TaskCreated),
            ..Default::default()
        };
        let filtered_events = logger.query_events(&filter).unwrap();
        assert_eq!(filtered_events.len(), 2);
    }

    #[test]
    fn test_event_stats() {
        let temp_dir = tempdir().unwrap();
        let mut logger = TaskEventLogger::new(temp_dir.path().to_path_buf()).unwrap();

        // 创建测试事件
        let events = vec![
            TaskEvent::task_created("task-1", TaskType::Compress, TaskPriority::High),
            TaskEvent::error("task-1", "测试错误", None),
            TaskEvent::completed("task-1", true, 15.0),
        ];

        for event in &events {
            logger.log_event(event).unwrap();
        }

        // 获取统计信息
        let stats = logger.get_event_stats().unwrap();

        assert_eq!(stats.total_events, 3);
        assert_eq!(stats.error_count, 1);
        assert!(stats.earliest_event.is_some());
        assert!(stats.latest_event.is_some());
    }
}