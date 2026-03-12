use crate::config::file_loader::ConfigFileLoader;
use crate::config::models::{
    ConfigCategory, ConfigItem, ConfigMetadata, DefaultConfigGenerator,
};
use crate::config::repository::{ConfigRepository};
use anyhow::Result;
use serde_json::Value;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub trait ConfigChangeListener: Send + Sync {
    fn on_config_changed(&self, key: &str, old_value: Option<Value>, new_value: Value, modified_by: &str);
    fn get_id(&self) -> &str;
}

#[derive(Clone)]
pub struct ConfigService {
    repository: Arc<ConfigRepository>,
    metadata_cache: Arc<RwLock<HashMap<String, ConfigMetadata>>>,
    config_cache: Arc<RwLock<HashMap<String, ConfigItem>>>,
    listeners: Arc<RwLock<Vec<Box<dyn ConfigChangeListener>>>>,
    file_loader: Option<Arc<ConfigFileLoader>>,
}

impl ConfigService {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            repository: Arc::new(ConfigRepository::new(pool)),
            metadata_cache: Arc::new(RwLock::new(HashMap::new())),
            config_cache: Arc::new(RwLock::new(HashMap::new())),
            listeners: Arc::new(RwLock::new(Vec::new())),
            file_loader: None,
        }
    }

    pub fn with_file_loader(pool: SqlitePool, file_loader: ConfigFileLoader) -> Self {
        Self {
            repository: Arc::new(ConfigRepository::new(pool)),
            metadata_cache: Arc::new(RwLock::new(HashMap::new())),
            config_cache: Arc::new(RwLock::new(HashMap::new())),
            listeners: Arc::new(RwLock::new(Vec::new())),
            file_loader: Some(Arc::new(file_loader)),
        }
    }

    pub async fn init(&self) -> Result<()> {
        // 加载默认元数据
        let mut metadata = self.metadata_cache.write().await;
        for m in DefaultConfigGenerator::generate_all_metadata() {
            metadata.insert(m.key.clone(), m);
        }
        Ok(())
    }

    pub async fn get_all_configs(&self) -> Result<Vec<ConfigItem>> {
        let cache = self.config_cache.read().await;
        let mut items: Vec<ConfigItem> = cache.values().cloned().collect();
        items.sort_by(|a, b| a.metadata.key.cmp(&b.metadata.key));
        Ok(items)
    }

    pub async fn get_configs_by_category(&self, category: ConfigCategory) -> Result<Vec<ConfigItem>> {
        let cache = self.config_cache.read().await;
        Ok(cache.values().filter(|i| i.metadata.category == category).cloned().collect())
    }

    pub async fn get_config(&self, key: &str) -> Result<Option<ConfigItem>> {
        let cache = self.config_cache.read().await;
        Ok(cache.get(key).cloned())
    }

    pub async fn get_value(&self, key: &str) -> Result<Option<Value>> {
        Ok(self.get_config(key).await?.map(|i| i.current_value))
    }

    pub async fn set_config(&self, key: &str, value: Value, modified_by: &str) -> Result<()> {
        let mut cache = self.config_cache.write().await;
        if let Some(item) = cache.get_mut(key) {
            item.update_value(value, modified_by);
        }
        Ok(())
    }

    pub async fn refresh_cache(&self) -> Result<()> { Ok(()) }
    pub async fn reload_from_files(&self) -> Result<()> { Ok(()) }
}
