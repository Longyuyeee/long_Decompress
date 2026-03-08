//! 配置变更监听器模块
//!
//! 提供配置变更的监听和通知机制。

use crate::config::service::ConfigChangeListener;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 配置变更事件
#[derive(Debug, Clone)]
pub struct ConfigChangeEvent {
    /// 配置键
    pub key: String,
    /// 旧值
    pub old_value: Option<Value>,
    /// 新值
    pub new_value: Value,
    /// 变更时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 变更者
    pub changed_by: String,
    /// 变更来源
    pub source: ChangeSource,
}

/// 变更来源
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeSource {
    /// 用户界面
    UserInterface,
    /// 命令行
    CommandLine,
    /// 导入
    Import,
    /// 系统
    System,
    /// API
    Api,
    /// 其他
    Other,
}

/// 配置变更监听器管理器
pub struct ConfigChangeListenerManager {
    listeners: Arc<RwLock<Vec<Box<dyn ConfigChangeListener + Send + Sync>>>>,
}

impl ConfigChangeListenerManager {
    /// 创建新的监听器管理器
    pub fn new() -> Self {
        Self {
            listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 添加监听器
    pub async fn add_listener(&self, listener: Box<dyn ConfigChangeListener + Send + Sync>) {
        let mut listeners = self.listeners.write().await;
        listeners.push(listener);
    }

    /// 移除监听器
    pub async fn remove_listener(&self, listener_id: &str) {
        let mut listeners = self.listeners.write().await;
        listeners.retain(|l| l.get_id() != listener_id);
    }

    /// 通知所有监听器
    pub async fn notify_listeners(
        &self,
        key: &str,
        old_value: Option<Value>,
        new_value: Value,
        changed_by: &str,
        source: ChangeSource,
    ) {
        let listeners = self.listeners.read().await;
        for listener in listeners.iter() {
            listener.on_config_changed(key, old_value.clone(), new_value.clone(), changed_by);
        }

        // 记录事件（简化实现）
        let event = ConfigChangeEvent {
            key: key.to_string(),
            old_value,
            new_value,
            timestamp: chrono::Utc::now(),
            changed_by: changed_by.to_string(),
            source,
        };

        log::debug!("配置变更事件: {:?}", event);
    }

    /// 获取监听器数量
    pub async fn listener_count(&self) -> usize {
        let listeners = self.listeners.read().await;
        listeners.len()
    }

    /// 清空所有监听器
    pub async fn clear_listeners(&self) {
        let mut listeners = self.listeners.write().await;
        listeners.clear();
    }
}

/// 日志记录监听器
pub struct LoggingConfigListener {
    id: String,
    min_log_level: LogLevel,
}

impl LoggingConfigListener {
    /// 创建新的日志记录监听器
    pub fn new(min_log_level: LogLevel) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            min_log_level,
        }
    }
}

impl ConfigChangeListener for LoggingConfigListener {
    fn on_config_changed(&self, key: &str, old_value: Option<Value>, new_value: Value, changed_by: &str) {
        let message = format!(
            "配置变更: key={}, old_value={:?}, new_value={:?}, changed_by={}",
            key, old_value, new_value, changed_by
        );

        match self.min_log_level {
            LogLevel::Debug => log::debug!("{}", message),
            LogLevel::Info => log::info!("{}", message),
            LogLevel::Warn => log::warn!("{}", message),
            LogLevel::Error => log::error!("{}", message),
        }
    }

    fn get_id(&self) -> &str {
        &self.id
    }
}

/// 审计日志监听器
pub struct AuditLogConfigListener {
    id: String,
    audit_logger: Arc<RwLock<dyn AuditLogger + Send + Sync>>,
}

impl AuditLogConfigListener {
    /// 创建新的审计日志监听器
    pub fn new(audit_logger: Arc<RwLock<dyn AuditLogger + Send + Sync>>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            audit_logger,
        }
    }
}

impl ConfigChangeListener for AuditLogConfigListener {
    fn on_config_changed(&self, key: &str, old_value: Option<Value>, new_value: Value, changed_by: &str) {
        // 异步记录审计日志
        let audit_logger = self.audit_logger.clone();
        let key = key.to_string();
        let old_value_str = old_value.map(|v| v.to_string());
        let new_value_str = new_value.to_string();
        let changed_by = changed_by.to_string();

        tokio::spawn(async move {
            let logger = audit_logger.read().await;
            logger.log_config_change(&key, old_value_str.as_deref(), &new_value_str, &changed_by).await;
        });
    }

    fn get_id(&self) -> &str {
        &self.id
    }
}

/// 配置缓存监听器
pub struct ConfigCacheListener {
    id: String,
    cache: Arc<RwLock<dyn ConfigCache + Send + Sync>>,
}

impl ConfigCacheListener {
    /// 创建新的配置缓存监听器
    pub fn new(cache: Arc<RwLock<dyn ConfigCache + Send + Sync>>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            cache,
        }
    }
}

impl ConfigChangeListener for ConfigCacheListener {
    fn on_config_changed(&self, key: &str, _old_value: Option<Value>, new_value: Value, _changed_by: &str) {
        // 异步更新缓存
        let cache = self.cache.clone();
        let key = key.to_string();

        tokio::spawn(async move {
            let mut cache = cache.write().await;
            cache.update_config(&key, new_value).await;
        });
    }

    fn get_id(&self) -> &str {
        &self.id
    }
}

/// 审计日志记录器 trait
#[async_trait::async_trait]
pub trait AuditLogger: Send + Sync {
    /// 记录配置变更
    async fn log_config_change(&self, key: &str, old_value: Option<&str>, new_value: &str, changed_by: &str);
}

/// 配置缓存 trait
#[async_trait::async_trait]
pub trait ConfigCache: Send + Sync {
    /// 更新配置缓存
    async fn update_config(&mut self, key: &str, value: Value);
}

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// 文件审计日志记录器
pub struct FileAuditLogger {
    file_path: std::path::PathBuf,
}

impl FileAuditLogger {
    /// 创建新的文件审计日志记录器
    pub fn new(file_path: std::path::PathBuf) -> Self {
        Self { file_path }
    }
}

#[async_trait::async_trait]
impl AuditLogger for FileAuditLogger {
    async fn log_config_change(&self, key: &str, old_value: Option<&str>, new_value: &str, changed_by: &str) {
        use chrono::Utc;
        use tokio::fs::OpenOptions;
        use tokio::io::AsyncWriteExt;

        let timestamp = Utc::now().to_rfc3339();
        let old_value_str = old_value.unwrap_or("null");
        let log_entry = format!(
            "[{}] CONFIG_CHANGE key={} old_value={} new_value={} changed_by={}\n",
            timestamp, key, old_value_str, new_value, changed_by
        );

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .await
        {
            let _ = file.write_all(log_entry.as_bytes()).await;
        }
    }
}

/// 内存配置缓存
pub struct MemoryConfigCache {
    cache: std::collections::HashMap<String, Value>,
}

impl MemoryConfigCache {
    /// 创建新的内存配置缓存
    pub fn new() -> Self {
        Self {
            cache: std::collections::HashMap::new(),
        }
    }

    /// 获取缓存中的配置值
    pub fn get(&self, key: &str) -> Option<Value> {
        self.cache.get(key).cloned()
    }

    /// 获取所有缓存键
    pub fn keys(&self) -> Vec<String> {
        self.cache.keys().cloned().collect()
    }

    /// 清空缓存
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

#[async_trait::async_trait]
impl ConfigCache for MemoryConfigCache {
    async fn update_config(&mut self, key: &str, value: Value) {
        self.cache.insert(key.to_string(), value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_logging_listener() {
        let listener = LoggingConfigListener::new(LogLevel::Info);
        let listener_id = listener.get_id().to_string();

        // 测试监听器ID
        assert!(!listener_id.is_empty());

        // 测试配置变更回调（应该记录日志）
        listener.on_config_changed(
            "test.config",
            Some(json!("old")),
            json!("new"),
            "test",
        );
    }

    #[tokio::test]
    async fn test_listener_manager() {
        let manager = ConfigChangeListenerManager::new();

        // 初始监听器数量应为0
        assert_eq!(manager.listener_count().await, 0);

        // 添加监听器
        let listener = LoggingConfigListener::new(LogLevel::Info);
        let listener_id = listener.get_id().to_string();
        manager.add_listener(Box::new(listener)).await;

        // 监听器数量应为1
        assert_eq!(manager.listener_count().await, 1);

        // 移除监听器
        manager.remove_listener(&listener_id).await;
        assert_eq!(manager.listener_count().await, 0);

        // 清空监听器
        manager.add_listener(Box::new(LoggingConfigListener::new(LogLevel::Info))).await;
        manager.clear_listeners().await;
        assert_eq!(manager.listener_count().await, 0);
    }

    #[tokio::test]
    async fn test_memory_config_cache() {
        let mut cache = MemoryConfigCache::new();

        // 初始缓存应为空
        assert!(cache.get("test.config").is_none());

        // 更新缓存
        cache.update_config("test.config", json!("value")).await;

        // 验证缓存已更新
        assert_eq!(cache.get("test.config"), Some(json!("value")));

        // 获取所有键
        let keys = cache.keys();
        assert_eq!(keys, vec!["test.config".to_string()]);

        // 清空缓存
        cache.clear();
        assert!(cache.get("test.config").is_none());
    }

    #[test]
    fn test_config_change_event() {
        let event = ConfigChangeEvent {
            key: "test.config".to_string(),
            old_value: Some(json!("old")),
            new_value: json!("new"),
            timestamp: chrono::Utc::now(),
            changed_by: "test".to_string(),
            source: ChangeSource::UserInterface,
        };

        assert_eq!(event.key, "test.config");
        assert_eq!(event.old_value, Some(json!("old")));
        assert_eq!(event.new_value, json!("new"));
        assert_eq!(event.changed_by, "test");
        assert_eq!(event.source, ChangeSource::UserInterface);
    }
}