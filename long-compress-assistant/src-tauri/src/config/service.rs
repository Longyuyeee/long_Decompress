use crate::config::file_loader::{ConfigFileLoader, ConfigFileFormat};
use crate::config::models::{
    ConfigCategory, ConfigItem, ConfigMetadata, DefaultConfigGenerator, ExportFormat, ImportResult,
    ImportStrategy, ValidationResult,
};
use crate::config::repository::{ConfigRepository, ImportError, ImportWarning};
use crate::config::validation::ConfigValidator;
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// 配置服务
pub struct ConfigService {
    repository: ConfigRepository,
    metadata_cache: Arc<RwLock<HashMap<String, ConfigMetadata>>>,
    config_cache: Arc<RwLock<HashMap<String, ConfigItem>>>,
    listeners: Arc<RwLock<Vec<Box<dyn ConfigChangeListener + Send + Sync>>>>,
    file_loader: Option<ConfigFileLoader>,
    config_files: Arc<RwLock<Vec<PathBuf>>>,
}

impl Clone for ConfigService {
    fn clone(&self) -> Self {
        Self {
            repository: self.repository.clone(),
            metadata_cache: Arc::clone(&self.metadata_cache),
            config_cache: Arc::clone(&self.config_cache),
            listeners: Arc::clone(&self.listeners),
            file_loader: self.file_loader.clone(),
            config_files: Arc::clone(&self.config_files),
        }
    }
}

impl ConfigService {
    /// 创建新的配置服务
    pub fn new(pool: SqlitePool) -> Self {
        let repository = ConfigRepository::new(pool);
        let metadata_cache = Arc::new(RwLock::new(HashMap::new()));
        let config_cache = Arc::new(RwLock::new(HashMap::new()));
        let listeners = Arc::new(RwLock::new(Vec::new()));

        Self {
            repository,
            metadata_cache,
            config_cache,
            listeners,
            file_loader: None,
            config_files: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 创建带有文件加载器的配置服务
    pub fn with_file_loader(pool: SqlitePool, file_loader: ConfigFileLoader) -> Self {
        let mut service = Self::new(pool);
        service.file_loader = Some(file_loader);
        service
    }

    /// 初始化配置服务
    pub async fn init(&self) -> Result<()> {
        // 初始化数据库表
        self.repository.init_tables().await?;

        // 加载元数据缓存
        self.load_metadata_cache().await?;

        // 从文件加载配置（如果配置了文件加载器）
        self.load_configs_from_files().await?;

        // 从环境变量加载配置
        self.load_configs_from_env().await?;

        // 加载配置缓存
        self.load_config_cache().await?;

        Ok(())
    }

    /// 从文件加载配置
    async fn load_configs_from_files(&self) -> Result<()> {
        if let Some(loader) = &self.file_loader {
            let file_items = loader.load_all_config_files().await?;

            for item in file_items {
                // 验证配置值
                let validation_result = ConfigValidator::validate(&item.metadata, &item.current_value);
                if validation_result.is_valid {
                    // 保存到数据库
                    if let Err(e) = self.repository.save_config(&item).await {
                        log::warn!("保存文件配置失败 {}: {}", item.metadata.key, e);
                    } else {
                        log::info!("从文件加载配置: {}", item.metadata.key);
                    }
                } else {
                    log::warn!("文件配置验证失败 {}: {:?}", item.metadata.key, validation_result.errors);
                }
            }

            // 记录加载的文件
            let mut config_files = self.config_files.write().await;
            // 这里可以记录实际加载的文件路径
        }

        Ok(())
    }

    /// 从环境变量加载配置
    async fn load_configs_from_env(&self) -> Result<()> {
        if let Some(loader) = &self.file_loader {
            let env_items = loader.load_env_vars("APP_")?;

            for item in env_items {
                // 环境变量配置通常是只读的，直接保存到数据库
                if let Err(e) = self.repository.save_config(&item).await {
                    log::warn!("保存环境变量配置失败 {}: {}", item.metadata.key, e);
                } else {
                    log::info!("从环境变量加载配置: {}", item.metadata.key);
                }
            }
        }

        Ok(())
    }

    /// 加载元数据缓存
    async fn load_metadata_cache(&self) -> Result<()> {
        let mut cache = self.metadata_cache.write().await;
        let metadata_list = DefaultConfigGenerator::generate_all_metadata();

        for metadata in metadata_list {
            cache.insert(metadata.key.clone(), metadata);
        }

        Ok(())
    }

    /// 加载配置缓存
    async fn load_config_cache(&self) -> Result<()> {
        let metadata_list = self.get_all_metadata().await?;
        let config_items = self.repository.get_all_configs(&metadata_list).await?;

        let mut cache = self.config_cache.write().await;
        for item in config_items {
            cache.insert(item.metadata.key.clone(), item);
        }

        Ok(())
    }

    /// 获取所有元数据
    pub async fn get_all_metadata(&self) -> Result<Vec<ConfigMetadata>> {
        let cache = self.metadata_cache.read().await;
        Ok(cache.values().cloned().collect())
    }

    /// 获取元数据
    pub async fn get_metadata(&self, key: &str) -> Result<Option<ConfigMetadata>> {
        let cache = self.metadata_cache.read().await;
        Ok(cache.get(key).cloned())
    }

    /// 获取配置项
    pub async fn get_config(&self, key: &str) -> Result<Option<ConfigItem>> {
        // 先检查缓存
        {
            let cache = self.config_cache.read().await;
            if let Some(item) = cache.get(key) {
                return Ok(Some(item.clone()));
            }
        }

        // 缓存未命中，从数据库加载
        if let Some(metadata) = self.get_metadata(key).await? {
            if let Some(item) = self.repository.get_config(key, &metadata).await? {
                // 更新缓存
                let mut cache = self.config_cache.write().await;
                cache.insert(key.to_string(), item.clone());
                return Ok(Some(item));
            }
        }

        Ok(None)
    }

    /// 获取配置值
    pub async fn get_value(&self, key: &str) -> Result<Option<Value>> {
        if let Some(item) = self.get_config(key).await? {
            Ok(Some(item.current_value))
        } else {
            Ok(None)
        }
    }

    /// 获取字符串配置值
    pub async fn get_string(&self, key: &str) -> Result<Option<String>> {
        if let Some(value) = self.get_value(key).await? {
            if let Some(s) = value.as_str() {
                Ok(Some(s.to_string()))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// 获取整数配置值
    pub async fn get_integer(&self, key: &str) -> Result<Option<i64>> {
        if let Some(value) = self.get_value(key).await? {
            Ok(value.as_i64())
        } else {
            Ok(None)
        }
    }

    /// 获取浮点数配置值
    pub async fn get_float(&self, key: &str) -> Result<Option<f64>> {
        if let Some(value) = self.get_value(key).await? {
            Ok(value.as_f64())
        } else {
            Ok(None)
        }
    }

    /// 获取布尔配置值
    pub async fn get_boolean(&self, key: &str) -> Result<Option<bool>> {
        if let Some(value) = self.get_value(key).await? {
            Ok(value.as_bool())
        } else {
            Ok(None)
        }
    }

    /// 设置配置值
    pub async fn set_config(&self, key: &str, value: Value, modified_by: &str) -> Result<()> {
        let metadata = self.get_metadata(key).await?
            .context(format!("配置元数据未找到: {}", key))?;

        // 验证配置值
        let validation_result = ConfigValidator::validate(&metadata, &value);
        if !validation_result.is_valid {
            return Err(anyhow::anyhow!(
                "配置验证失败: {:?}",
                validation_result.errors
            ));
        }

        // 获取旧值（用于事件通知）
        let old_item = self.get_config(key).await?;
        let old_value = old_item.as_ref().map(|item| item.current_value.clone());

        // 创建新配置项
        let mut new_item = ConfigItem::new(metadata);
        new_item.update_value(value, modified_by);

        // 保存到数据库
        self.repository.save_config(&new_item).await?;

        // 更新缓存
        {
            let mut cache = self.config_cache.write().await;
            cache.insert(key.to_string(), new_item.clone());
        }

        // 通知监听器
        self.notify_listeners(key, old_value, new_item.current_value, modified_by).await;

        Ok(())
    }

    /// 批量设置配置值
    pub async fn batch_set_configs(
        &self,
        updates: HashMap<String, Value>,
        modified_by: &str,
    ) -> Result<()> {
        let mut errors = Vec::new();

        for (key, value) in updates {
            if let Err(e) = self.set_config(&key, value, modified_by).await {
                errors.push(format!("{}: {}", key, e));
            }
        }

        if !errors.is_empty() {
            return Err(anyhow::anyhow!("批量设置配置失败: {:?}", errors));
        }

        Ok(())
    }

    /// 获取所有配置项
    pub async fn get_all_configs(&self) -> Result<Vec<ConfigItem>> {
        let metadata_list = self.get_all_metadata().await?;
        let mut items = Vec::new();

        for metadata in metadata_list {
            if let Some(item) = self.get_config(&metadata.key).await? {
                items.push(item);
            } else {
                items.push(ConfigItem::new(metadata));
            }
        }

        // 按分类和排序顺序排序
        items.sort_by(|a, b| {
            let category_cmp = a.metadata.category.cmp(&b.metadata.category);
            if category_cmp == std::cmp::Ordering::Equal {
                a.metadata.sort_order.cmp(&b.metadata.sort_order)
            } else {
                category_cmp
            }
        });

        Ok(items)
    }

    /// 根据分类获取配置项
    pub async fn get_configs_by_category(&self, category: ConfigCategory) -> Result<Vec<ConfigItem>> {
        let all_items = self.get_all_configs().await?;
        let filtered_items: Vec<ConfigItem> = all_items
            .into_iter()
            .filter(|item| item.metadata.category == category)
            .collect();

        Ok(filtered_items)
    }

    /// 重置配置为默认值
    pub async fn reset_to_default(&self, key: &str, modified_by: &str) -> Result<()> {
        let metadata = self.get_metadata(key).await?
            .context(format!("配置元数据未找到: {}", key))?;

        // 获取旧值（用于事件通知）
        let old_item = self.get_config(key).await?;
        let old_value = old_item.as_ref().map(|item| item.current_value.clone());

        // 重置为默认值
        let new_item = self.repository.reset_to_default(key, &metadata).await?;

        // 更新缓存
        {
            let mut cache = self.config_cache.write().await;
            cache.insert(key.to_string(), new_item.clone());
        }

        // 通知监听器
        self.notify_listeners(key, old_value, new_item.current_value, modified_by).await;

        Ok(())
    }

    /// 批量重置配置为默认值
    pub async fn batch_reset_to_default(&self, keys: Vec<String>, modified_by: &str) -> Result<()> {
        let metadata_list = self.get_all_metadata().await?;
        let metadata_map: HashMap<String, ConfigMetadata> = metadata_list
            .into_iter()
            .map(|m| (m.key.clone(), m))
            .collect();

        let items = self.repository.batch_reset_to_default(&keys, &metadata_map).await?;

        // 更新缓存并通知监听器
        for item in items {
            let key = item.metadata.key.clone();
            let old_item = self.get_config(&key).await?;
            let old_value = old_item.as_ref().map(|i| i.current_value.clone());

            {
                let mut cache = self.config_cache.write().await;
                cache.insert(key.clone(), item.clone());
            }

            self.notify_listeners(&key, old_value, item.current_value, modified_by).await;
        }

        Ok(())
    }

    /// 验证配置值
    pub async fn validate_config(&self, key: &str, value: &Value) -> Result<ValidationResult> {
        let metadata = self.get_metadata(key).await?
            .context(format!("配置元数据未找到: {}", key))?;

        Ok(ConfigValidator::validate(&metadata, value))
    }

    /// 搜索配置项
    pub async fn search_configs(&self, query: &str) -> Result<Vec<ConfigItem>> {
        let metadata_list = self.get_all_metadata().await?;
        self.repository.search_configs(query, &metadata_list).await
    }

    /// 导出配置
    pub async fn export_configs(&self, format: ExportFormat) -> Result<Vec<u8>> {
        let metadata_list = self.get_all_metadata().await?;
        let export_data = self.repository.export_configs(&metadata_list).await?;

        match format {
            ExportFormat::Json => {
                let json = serde_json::to_string_pretty(&export_data)
                    .context("序列化JSON失败")?;
                Ok(json.into_bytes())
            }
            ExportFormat::Yaml => {
                let yaml = serde_yaml::to_string(&export_data)
                    .context("序列化YAML失败")?;
                Ok(yaml.into_bytes())
            }
            ExportFormat::Toml => {
                let toml = toml::to_string_pretty(&export_data)
                    .context("序列化TOML失败")?;
                Ok(toml.into_bytes())
            }
        }
    }

    /// 导入配置
    pub async fn import_configs(
        &self,
        data: &[u8],
        format: ExportFormat,
        strategy: ImportStrategy,
        imported_by: &str,
    ) -> Result<ImportResult> {
        let import_data: Value = match format {
            ExportFormat::Json => {
                serde_json::from_slice(data).context("解析JSON失败")?
            }
            ExportFormat::Yaml => {
                serde_yaml::from_slice(data).context("解析YAML失败")?
            }
            ExportFormat::Toml => {
                toml::from_slice(data).context("解析TOML失败")?
            }
        };

        let metadata_list = self.get_all_metadata().await?;
        let result = self.repository.import_configs(&import_data, &metadata_list, strategy).await?;

        // 重新加载缓存
        self.load_config_cache().await?;

        Ok(result)
    }

    /// 获取配置统计信息
    pub async fn get_statistics(&self) -> Result<ConfigStatistics> {
        let db_stats = self.repository.get_statistics().await?;
        let cache_stats = self.get_cache_statistics().await;

        Ok(ConfigStatistics {
            total_configs: db_stats.total_count,
            cached_configs: cache_stats.cached_count,
            category_counts: db_stats.category_counts,
            last_updated: db_stats.last_updated,
            cache_hit_rate: cache_stats.hit_rate,
            cache_miss_rate: cache_stats.miss_rate,
        })
    }

    /// 获取缓存统计信息
    async fn get_cache_statistics(&self) -> CacheStatistics {
        let cache = self.config_cache.read().await;
        let total_metadata = self.metadata_cache.read().await.len();

        CacheStatistics {
            cached_count: cache.len(),
            total_metadata,
            hit_rate: 0.0, // 简化实现
            miss_rate: 0.0, // 简化实现
        }
    }

    /// 刷新缓存
    pub async fn refresh_cache(&self) -> Result<()> {
        self.load_config_cache().await
    }

    /// 清空缓存
    pub async fn clear_cache(&self) -> Result<()> {
        {
            let mut cache = self.config_cache.write().await;
            cache.clear();
        }
        Ok(())
    }

    /// 从文件重新加载配置
    pub async fn reload_from_files(&self) -> Result<()> {
        self.load_configs_from_files().await?;
        self.refresh_cache().await?;
        Ok(())
    }

    /// 导出配置到文件
    pub async fn export_to_file(&self, path: &std::path::Path, format: ConfigFileFormat) -> Result<()> {
        let items = self.get_all_configs().await?;

        if let Some(loader) = &self.file_loader {
            loader.save_config_file_to_path(path, &items, format).await?;
            log::info!("配置已导出到文件: {:?}", path);
        } else {
            return Err(anyhow::anyhow!("文件加载器未配置"));
        }

        Ok(())
    }

    /// 从文件导入配置
    pub async fn import_from_file(&self, path: &std::path::Path, strategy: ImportStrategy) -> Result<ImportResult> {
        if let Some(loader) = &self.file_loader {
            let items = loader.load_config_file_from_path(path).await?;

            let mut result = ImportResult {
                total_items: items.len(),
                imported_items: 0,
                skipped_items: 0,
                failed_items: 0,
                errors: Vec::new(),
                warnings: Vec::new(),
            };

            for item in items {
                // 检查是否应该导入
                let should_import = match strategy {
                    ImportStrategy::Merge => true,
                    ImportStrategy::Replace => true,
                    ImportStrategy::UpdateOnly => {
                        self.get_config(&item.metadata.key).await?.is_some()
                    }
                    ImportStrategy::SkipExisting => {
                        self.get_config(&item.metadata.key).await?.is_none()
                    }
                };

                if !should_import {
                    result.skipped_items += 1;
                    continue;
                }

                // 验证并保存配置
                let validation_result = ConfigValidator::validate(&item.metadata, &item.current_value);
                if !validation_result.is_valid {
                    result.failed_items += 1;
                    result.errors.push(ImportError {
                        code: "validation_failed".to_string(),
                        message: format!("配置验证失败: {}", item.metadata.key),
                        key: Some(item.metadata.key.clone()),
                        details: Some(json!(validation_result.errors)),
                    });
                    continue;
                }

                if let Err(e) = self.repository.save_config(&item).await {
                    result.failed_items += 1;
                    result.errors.push(ImportError {
                        code: "save_failed".to_string(),
                        message: format!("保存配置失败: {}", e),
                        key: Some(item.metadata.key.clone()),
                        details: None,
                    });
                } else {
                    result.imported_items += 1;
                }
            }

            // 刷新缓存
            self.refresh_cache().await?;

            Ok(result)
        } else {
            Err(anyhow::anyhow!("文件加载器未配置"))
        }
    }

    /// 监视配置文件变化
    pub async fn watch_config_file(&mut self, path: &std::path::Path) -> Result<()> {
        if let Some(loader) = &mut self.file_loader {
            let service_clone = Arc::new(self.clone());
            let path_clone = path.to_path_buf();

            loader.watch_config_file_path(path, move |changed_path| {
                let service = service_clone.clone();
                let path = changed_path.to_path_buf();

                tokio::spawn(async move {
                    log::info!("配置文件发生变化: {:?}", path);
                    if let Err(e) = service.reload_from_files().await {
                        log::error!("重新加载配置文件失败: {}", e);
                    }
                });
            }).await?;

            // 记录监视的文件
            let mut config_files = self.config_files.write().await;
            config_files.push(path.to_path_buf());

            log::info!("开始监视配置文件: {:?}", path);
        }

        Ok(())
    }

    /// 停止监视配置文件
    pub async fn unwatch_config_file(&mut self, path: &std::path::Path) -> Result<()> {
        if let Some(loader) = &mut self.file_loader {
            loader.unwatch_config_file_path(path).await?;

            // 移除记录
            let mut config_files = self.config_files.write().await;
            config_files.retain(|p| p != path);

            log::info!("停止监视配置文件: {:?}", path);
        }

        Ok(())
    }

    /// 获取监视中的配置文件列表
    pub async fn get_watched_files(&self) -> Vec<PathBuf> {
        self.config_files.read().await.clone()
    }

    /// 添加配置变更监听器
    pub async fn add_listener(&self, listener: Box<dyn ConfigChangeListener + Send + Sync>) {
        let mut listeners = self.listeners.write().await;
        listeners.push(listener);
    }

    /// 移除配置变更监听器
    pub async fn remove_listener(&self, listener_id: &str) {
        let mut listeners = self.listeners.write().await;
        listeners.retain(|l| l.get_id() != listener_id);
    }

    /// 通知所有监听器
    async fn notify_listeners(
        &self,
        key: &str,
        old_value: Option<Value>,
        new_value: Value,
        modified_by: &str,
    ) {
        let listeners = self.listeners.read().await;
        for listener in listeners.iter() {
            listener.on_config_changed(key, old_value.clone(), new_value.clone(), modified_by);
        }
    }
}

/// 配置变更监听器 trait
pub trait ConfigChangeListener {
    /// 配置变更时的回调
    fn on_config_changed(&self, key: &str, old_value: Option<Value>, new_value: Value, modified_by: &str);

    /// 获取监听器ID
    fn get_id(&self) -> &str;
}

/// 配置统计信息
#[derive(Debug)]
pub struct ConfigStatistics {
    pub total_configs: usize,
    pub cached_configs: usize,
    pub category_counts: HashMap<ConfigCategory, usize>,
    pub last_updated: Option<chrono::DateTime<Utc>>,
    pub cache_hit_rate: f64,
    pub cache_miss_rate: f64,
}

/// 缓存统计信息
#[derive(Debug)]
struct CacheStatistics {
    cached_count: usize,
    total_metadata: usize,
    hit_rate: f64,
    miss_rate: f64,
}

/// 默认配置变更监听器（日志记录）
pub struct LoggingConfigListener {
    id: String,
}

impl LoggingConfigListener {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
        }
    }
}

impl ConfigChangeListener for LoggingConfigListener {
    fn on_config_changed(&self, key: &str, old_value: Option<Value>, new_value: Value, modified_by: &str) {
        log::info!(
            "配置变更: key={}, old_value={:?}, new_value={:?}, modified_by={}",
            key,
            old_value,
            new_value,
            modified_by
        );
    }

    fn get_id(&self) -> &str {
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use tempfile::tempdir;

    async fn create_test_service() -> ConfigService {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = SqlitePool::connect(&format!("sqlite:{}", db_path.display()))
            .await
            .unwrap();

        let service = ConfigService::new(pool);
        service.init().await.unwrap();
        service
    }

    #[tokio::test]
    async fn test_init_service() {
        let service = create_test_service().await;

        // 验证服务已初始化
        let metadata = service.get_all_metadata().await.unwrap();
        assert!(!metadata.is_empty());

        let configs = service.get_all_configs().await.unwrap();
        assert!(!configs.is_empty());
    }

    #[tokio::test]
    async fn test_get_and_set_config() {
        let service = create_test_service().await;

        // 获取配置
        let value = service.get_string("system.language").await.unwrap();
        assert_eq!(value, Some("zh-CN".to_string()));

        // 设置配置
        service.set_config(
            "system.language",
            Value::String("en-US".to_string()),
            "test"
        ).await.unwrap();

        // 验证配置已更新
        let updated_value = service.get_string("system.language").await.unwrap();
        assert_eq!(updated_value, Some("en-US".to_string()));
    }

    #[tokio::test]
    async fn test_validation() {
        let service = create_test_service().await;

        // 测试有效值
        let result = service.validate_config(
            "system.update_interval",
            &Value::Number(1000.into())
        ).await.unwrap();
        assert!(result.is_valid);

        // 测试无效值（小于最小值）
        let result = service.validate_config(
            "system.update_interval",
            &Value::Number(500.into())
        ).await.unwrap();
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_reset_to_default() {
        let service = create_test_service().await;

        // 修改配置
        service.set_config(
            "system.language",
            Value::String("en-US".to_string()),
            "test"
        ).await.unwrap();

        // 重置为默认值
        service.reset_to_default("system.language", "test").await.unwrap();

        // 验证已重置
        let value = service.get_string("system.language").await.unwrap();
        assert_eq!(value, Some("zh-CN".to_string()));
    }

    #[tokio::test]
    async fn test_search_configs() {
        let service = create_test_service().await;

        // 搜索包含"language"的配置
        let results = service.search_configs("language").await.unwrap();
        assert!(!results.is_empty());

        // 验证结果包含语言配置
        let has_language = results.iter().any(|item| item.metadata.key.contains("language"));
        assert!(has_language);
    }

    #[tokio::test]
    async fn test_export_import() {
        let service = create_test_service().await;

        // 导出配置
        let export_data = service.export_configs(ExportFormat::Json).await.unwrap();
        assert!(!export_data.is_empty());

        // 修改一个配置
        service.set_config(
            "system.language",
            Value::String("ja-JP".to_string()),
            "test"
        ).await.unwrap();

        // 导入配置（替换策略）
        let import_result = service.import_configs(
            &export_data,
            ExportFormat::Json,
            ImportStrategy::Replace,
            "test"
        ).await.unwrap();

        assert!(import_result.imported_items > 0);

        // 验证配置已恢复
        let value = service.get_string("system.language").await.unwrap();
        assert_eq!(value, Some("zh-CN".to_string()));
    }

    #[tokio::test]
    async fn test_get_statistics() {
        let service = create_test_service().await;

        let stats = service.get_statistics().await.unwrap();
        assert!(stats.total_configs > 0);
        assert!(stats.cached_configs > 0);
        assert!(!stats.category_counts.is_empty());
    }

    #[tokio::test]
    async fn test_listener() {
        let service = create_test_service().await;

        // 创建监听器
        let listener = LoggingConfigListener::new();
        let listener_id = listener.get_id().to_string();

        // 添加监听器
        service.add_listener(Box::new(listener)).await;

        // 修改配置（应该触发监听器）
        service.set_config(
            "system.language",
            Value::String("en-US".to_string()),
            "test"
        ).await.unwrap();

        // 移除监听器
        service.remove_listener(&listener_id).await;
    }
}