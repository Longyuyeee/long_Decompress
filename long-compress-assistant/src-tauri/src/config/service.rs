use crate::config::file_loader::ConfigFileLoader;
use crate::config::models::{
    ConfigCategory, ConfigItem, ConfigMetadata, DefaultConfigGenerator, ExportFormat,
    ImportError, ImportResult, ImportStrategy, ValidationResult,
};
use crate::config::validation::ConfigValidator;
use crate::config::repository::{ConfigRepository};
use anyhow::Result;
use serde_json::Value;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy)]
pub struct ConfigServiceStatistics {
    pub total_configs: usize,
    pub cached_configs: usize,
}

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
        self.repository.init_tables().await?;
        let metadata_list = DefaultConfigGenerator::generate_all_metadata();
        let items = self.repository.get_all_configs(&metadata_list).await?;

        {
            let mut metadata = self.metadata_cache.write().await;
            metadata.clear();
            for item in &metadata_list {
                metadata.insert(item.key.clone(), item.clone());
            }
        }
        {
            let mut cache = self.config_cache.write().await;
            cache.clear();
            for item in items {
                cache.insert(item.metadata.key.clone(), item);
            }
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

    pub async fn get_string(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .get_value(key)
            .await?
            .and_then(|value| value.as_str().map(ToOwned::to_owned)))
    }

    pub async fn set_config(&self, key: &str, value: Value, modified_by: &str) -> Result<()> {
        let (old_value, updated_item) = {
            let mut cache = self.config_cache.write().await;
            let item = cache
                .get_mut(key)
                .ok_or_else(|| anyhow::anyhow!("Unknown configuration key: {}", key))?;
            if item.metadata.is_readonly {
                return Err(anyhow::anyhow!("Configuration is read-only: {}", key));
            }
            let validation = ConfigValidator::validate(&item.metadata, &value);
            if !validation.is_valid {
                return Err(anyhow::anyhow!(
                    "Configuration validation failed: {:?}",
                    validation.errors
                ));
            }
            let old_value = item.current_value.clone();
            item.update_value(value.clone(), modified_by);
            (old_value, item.clone())
        };

        self.repository.save_config(&updated_item).await?;
        let listeners = self.listeners.read().await;
        for listener in listeners.iter() {
            listener.on_config_changed(key, Some(old_value.clone()), value.clone(), modified_by);
        }
        Ok(())
    }

    pub async fn reset_to_default(&self, key: &str, modified_by: &str) -> Result<()> {
        let metadata = self
            .metadata_cache
            .read()
            .await
            .get(key)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Unknown configuration key: {}", key))?;
        let mut item = self.repository.reset_to_default(key, &metadata).await?;
        item.last_modified_by = modified_by.to_string();
        self.config_cache.write().await.insert(key.to_string(), item);
        Ok(())
    }

    pub async fn search_configs(&self, query: &str) -> Result<Vec<ConfigItem>> {
        let query = query.to_lowercase();
        let cache = self.config_cache.read().await;
        let mut items: Vec<_> = cache
            .values()
            .filter(|item| {
                item.metadata.key.to_lowercase().contains(&query)
                    || item.metadata.display_name.to_lowercase().contains(&query)
                    || item.metadata.description.to_lowercase().contains(&query)
            })
            .cloned()
            .collect();
        items.sort_by(|a, b| a.metadata.key.cmp(&b.metadata.key));
        Ok(items)
    }

    pub async fn validate_config(&self, key: &str, value: &Value) -> Result<ValidationResult> {
        let metadata = self
            .metadata_cache
            .read()
            .await
            .get(key)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Unknown configuration key: {}", key))?;
        Ok(ConfigValidator::validate(&metadata, value))
    }

    pub async fn get_metadata(&self, key: &str) -> Result<Option<ConfigMetadata>> {
        Ok(self.metadata_cache.read().await.get(key).cloned())
    }

    pub async fn batch_set_configs(
        &self,
        updates: HashMap<String, Value>,
        modified_by: &str,
    ) -> Result<()> {
        for (key, value) in updates {
            self.set_config(&key, value, modified_by).await?;
        }
        Ok(())
    }

    pub async fn batch_reset_to_default(
        &self,
        keys: Vec<String>,
        modified_by: &str,
    ) -> Result<()> {
        for key in keys {
            self.reset_to_default(&key, modified_by).await?;
        }
        Ok(())
    }

    pub async fn get_statistics(&self) -> Result<ConfigServiceStatistics> {
        Ok(ConfigServiceStatistics {
            total_configs: self.metadata_cache.read().await.len(),
            cached_configs: self.config_cache.read().await.len(),
        })
    }

    pub async fn export_configs(&self, format: ExportFormat) -> Result<Vec<u8>> {
        let items = self.get_all_configs().await?;
        let payload = serde_json::json!({
            "export_version": "1.0.0",
            "configs": items.iter().map(|item| serde_json::json!({
                "key": item.metadata.key,
                "value": item.current_value,
            })).collect::<Vec<_>>(),
        });

        match format {
            ExportFormat::Json => Ok(serde_json::to_vec_pretty(&payload)?),
            ExportFormat::Yaml => Ok(serde_yaml::to_string(&payload)?.into_bytes()),
            ExportFormat::Toml => {
                let value: toml::Value = serde_json::from_value(payload)?;
                Ok(toml::to_string_pretty(&value)?.into_bytes())
            }
        }
    }

    pub async fn import_configs(
        &self,
        data: &[u8],
        format: ExportFormat,
        strategy: ImportStrategy,
        modified_by: &str,
    ) -> Result<ImportResult> {
        let payload: Value = match format {
            ExportFormat::Json => serde_json::from_slice(data)?,
            ExportFormat::Yaml => {
                let value: serde_yaml::Value = serde_yaml::from_slice(data)?;
                serde_json::to_value(value)?
            }
            ExportFormat::Toml => {
                let value: toml::Value = std::str::from_utf8(data)?.parse()?;
                serde_json::to_value(value)?
            }
        };
        let configs = payload
            .get("configs")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("Import payload is missing configs"))?;
        let mut result = ImportResult {
            total_items: configs.len(),
            imported_items: 0,
            skipped_items: 0,
            failed_items: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        for config in configs {
            let Some(key) = config.get("key").and_then(Value::as_str) else {
                result.failed_items += 1;
                result.errors.push(ImportError {
                    code: "missing_key".to_string(),
                    message: "Configuration entry is missing a key".to_string(),
                    key: None,
                    details: None,
                });
                continue;
            };
            if self.get_metadata(key).await?.is_none() {
                result.skipped_items += 1;
                continue;
            }
            if matches!(strategy, ImportStrategy::SkipExisting)
                && self.get_config(key).await?.is_some()
            {
                result.skipped_items += 1;
                continue;
            }
            let value = config.get("value").cloned().unwrap_or(Value::Null);
            match self.set_config(key, value, modified_by).await {
                Ok(()) => result.imported_items += 1,
                Err(error) => {
                    result.failed_items += 1;
                    result.errors.push(ImportError {
                        code: "import_failed".to_string(),
                        message: error.to_string(),
                        key: Some(key.to_string()),
                        details: None,
                    });
                }
            }
        }
        Ok(result)
    }

    pub async fn clear_cache(&self) -> Result<()> {
        self.config_cache.write().await.clear();
        Ok(())
    }

    pub async fn refresh_cache(&self) -> Result<()> {
        let metadata: Vec<_> = self.metadata_cache.read().await.values().cloned().collect();
        let items = self.repository.get_all_configs(&metadata).await?;
        let mut cache = self.config_cache.write().await;
        cache.clear();
        for item in items {
            cache.insert(item.metadata.key.clone(), item);
        }
        Ok(())
    }
    pub async fn reload_from_files(&self) -> Result<()> { Ok(()) }
}
