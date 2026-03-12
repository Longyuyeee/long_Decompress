use crate::config::models::{
    ConfigCategory, ConfigItem, ConfigMetadata, ValidationResult,
};
use crate::config::validation::ConfigValidator;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::{SqlitePool, query, query_as};
use std::collections::HashMap;
use uuid::Uuid;

/// 配置Repository
#[derive(Debug, Clone)]
pub struct ConfigRepository {
    pool: SqlitePool,
}

impl ConfigRepository {
    /// 创建新的配置Repository
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 初始化配置表（如果需要）
    pub async fn init_tables(&self) -> Result<()> {
        // 检查表是否存在
        let table_exists: (i64,) = query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='application_settings'"
        )
            .fetch_one(&self.pool)
            .await
            .context("检查表是否存在失败")?;

        if table_exists.0 == 0 {
            // 创建表（使用现有的表结构）
            query(
                r#"
                CREATE TABLE application_settings (
                    id TEXT PRIMARY KEY,
                    key TEXT UNIQUE NOT NULL,
                    value TEXT NOT NULL,
                    category TEXT NOT NULL,
                    description TEXT,
                    created_at DATETIME NOT NULL,
                    updated_at DATETIME NOT NULL
                )
                "#
            )
                .execute(&self.pool)
                .await
                .context("创建应用设置表失败")?;

            // 创建索引
            query("CREATE INDEX idx_application_settings_key ON application_settings(key)")
                .execute(&self.pool)
                .await
                .context("创建键索引失败")?;

            query("CREATE INDEX idx_application_settings_category ON application_settings(category)")
                .execute(&self.pool)
                .await
                .context("创建分类索引失败")?;
        }

        Ok(())
    }

    /// 保存配置项
    pub async fn save_config(&self, item: &ConfigItem) -> Result<()> {
        let validation_result = ConfigValidator::validate(&item.metadata, &item.current_value);
        if !validation_result.is_valid {
            return Err(anyhow::anyhow!(
                "配置验证失败: {:?}",
                validation_result.errors
            ));
        }

        let value_json = serde_json::to_string(&item.current_value)
            .context("序列化配置值失败")?;

        let category_str = Self::category_to_string(&item.metadata.category);

        // 检查是否已存在
        let exists: (i64,) = query_as(
            "SELECT COUNT(*) FROM application_settings WHERE key = ?"
        )
            .bind(&item.metadata.key)
            .fetch_one(&self.pool)
            .await
            .context("检查配置是否存在失败")?;

        let now = Utc::now();

        if exists.0 == 0 {
            // 插入新配置
            query(
                r#"
                INSERT INTO application_settings (id, key, value, category, description, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#
            )
                .bind(Uuid::new_v4().to_string())
                .bind(&item.metadata.key)
                .bind(&value_json)
                .bind(category_str)
                .bind(&item.metadata.description)
                .bind(now)
                .bind(now)
                .execute(&self.pool)
                .await
                .context("插入配置失败")?;
        } else {
            // 更新现有配置
            query(
                r#"
                UPDATE application_settings SET
                    value = ?, category = ?, description = ?, updated_at = ?
                WHERE key = ?
                "#
            )
                .bind(&value_json)
                .bind(category_str)
                .bind(&item.metadata.description)
                .bind(now)
                .bind(&item.metadata.key)
                .execute(&self.pool)
                .await
                .context("更新配置失败")?;
        }

        Ok(())
    }

    /// 批量保存配置项
    pub async fn save_configs(&self, items: &[ConfigItem]) -> Result<()> {
        for item in items {
            self.save_config(item).await?;
        }
        Ok(())
    }

    /// 获取配置项
    pub async fn get_config(&self, key: &str, metadata: &ConfigMetadata) -> Result<Option<ConfigItem>> {
        let row: Option<(String, String, String, DateTime<Utc>)> = query_as(
            "SELECT value, category, description, updated_at FROM application_settings WHERE key = ?"
        )
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .context("获取配置失败")?;

        if let Some((value_json, _category_str, _description, updated_at)) = row {
            let value: Value = serde_json::from_str(&value_json)
                .context("解析配置值失败")?;

            let mut item = ConfigItem::new(metadata.clone());
            item.current_value = value;
            item.last_modified = updated_at;
            item.last_modified_by = "database".to_string();

            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    /// 获取所有配置项
    pub async fn get_all_configs(&self, metadata_list: &[ConfigMetadata]) -> Result<Vec<ConfigItem>> {
        let mut items = Vec::new();

        for metadata in metadata_list {
            if let Some(item) = self.get_config(&metadata.key, metadata).await? {
                items.push(item);
            } else {
                // 使用默认值创建新配置项
                items.push(ConfigItem::new(metadata.clone()));
            }
        }

        Ok(items)
    }

    /// 根据分类获取配置项
    pub async fn get_configs_by_category(
        &self,
        category: ConfigCategory,
        metadata_list: &[ConfigMetadata],
    ) -> Result<Vec<ConfigItem>> {
        let _category_str = Self::category_to_string(&category);
        let filtered_metadata: Vec<&ConfigMetadata> = metadata_list
            .iter()
            .filter(|m| m.category == category)
            .collect();

        let mut items = Vec::new();
        for metadata in filtered_metadata {
            if let Some(item) = self.get_config(&metadata.key, metadata).await? {
                items.push(item);
            } else {
                items.push(ConfigItem::new(metadata.clone()));
            }
        }

        Ok(items)
    }

    /// 删除配置项
    pub async fn delete_config(&self, key: &str) -> Result<()> {
        query("DELETE FROM application_settings WHERE key = ?")
            .bind(key)
            .execute(&self.pool)
            .await
            .context("删除配置失败")?;

        Ok(())
    }

    /// 批量删除配置项
    pub async fn delete_configs(&self, keys: &[String]) -> Result<()> {
        for key in keys {
            self.delete_config(key).await?;
        }
        Ok(())
    }

    /// 重置配置为默认值
    pub async fn reset_to_default(&self, _key: &str, metadata: &ConfigMetadata) -> Result<ConfigItem> {
        let mut item = ConfigItem::new(metadata.clone());
        item.reset_to_default("system");

        self.save_config(&item).await?;

        Ok(item)
    }

    /// 批量重置配置为默认值
    pub async fn batch_reset_to_default(
        &self,
        keys: &[String],
        metadata_map: &HashMap<String, ConfigMetadata>,
    ) -> Result<Vec<ConfigItem>> {
        let mut items = Vec::new();

        for key in keys {
            if let Some(metadata) = metadata_map.get(key) {
                let item = self.reset_to_default(key, metadata).await?;
                items.push(item);
            }
        }

        Ok(items)
    }

    /// 验证配置值
    pub async fn validate_config(
        &self,
        _key: &str,
        value: &Value,
        metadata: &ConfigMetadata,
    ) -> Result<ValidationResult> {
        Ok(ConfigValidator::validate(metadata, value))
    }

    /// 批量验证配置值
    pub async fn validate_configs(
        &self,
        updates: &HashMap<String, (ConfigMetadata, Value)>,
    ) -> Result<HashMap<String, ValidationResult>> {
        Ok(ConfigValidator::validate_batch(updates))
    }

    /// 获取配置统计信息
    pub async fn get_statistics(&self) -> Result<ConfigStatistics> {
        let total_count: (i64,) = query_as(
            "SELECT COUNT(*) FROM application_settings"
        )
            .fetch_one(&self.pool)
            .await
            .context("获取配置总数失败")?;

        let category_counts: Vec<(String, i64)> = query_as(
            "SELECT category, COUNT(*) FROM application_settings GROUP BY category"
        )
            .fetch_all(&self.pool)
            .await
            .context("获取分类统计失败")?;

        let last_updated: (Option<DateTime<Utc>>,) = query_as(
            "SELECT MAX(updated_at) FROM application_settings"
        )
            .fetch_one(&self.pool)
            .await
            .context("获取最后更新时间失败")?;

        let mut categories = HashMap::new();
        for (category_str, count) in category_counts {
            let category = Self::string_to_category(&category_str);
            categories.insert(category, count as usize);
        }

        Ok(ConfigStatistics {
            total_count: total_count.0 as usize,
            category_counts: categories,
            last_updated: last_updated.0,
        })
    }

    /// 搜索配置项
    pub async fn search_configs(
        &self,
        query_str: &str,
        metadata_list: &[ConfigMetadata],
    ) -> Result<Vec<ConfigItem>> {
        let search_pattern = format!("%{}%", query_str);

        let rows: Vec<(String, String, String, DateTime<Utc>)> = query_as(
            r#"
            SELECT key, value, category, updated_at FROM application_settings
            WHERE key LIKE ? OR description LIKE ?
            ORDER BY key
            "#
        )
            .bind(&search_pattern)
            .bind(&search_pattern)
            .fetch_all(&self.pool)
            .await
            .context("搜索配置失败")?;

        let mut items = Vec::new();
        for (key, value_json, _category_str, updated_at) in rows {
            // 查找对应的元数据
            if let Some(metadata) = metadata_list.iter().find(|m| m.key == key) {
                let value: Value = serde_json::from_str(&value_json)
                    .context("解析配置值失败")?;

                let mut item = ConfigItem::new(metadata.clone());
                item.current_value = value;
                item.last_modified = updated_at;
                item.last_modified_by = "database".to_string();

                items.push(item);
            }
        }

        Ok(items)
    }

    /// 导出配置（JSON格式）
    pub async fn export_configs(
        &self,
        metadata_list: &[ConfigMetadata],
    ) -> Result<Value> {
        let items = self.get_all_configs(metadata_list).await?;

        let export_data: Vec<Value> = items
            .iter()
            .map(|item| {
                json!({
                    "key": item.metadata.key,
                    "category": Self::category_to_string(&item.metadata.category),
                    "display_name": item.metadata.display_name,
                    "description": item.metadata.description,
                    "data_type": item.metadata.data_type.as_str(),
                    "value": item.current_value,
                    "default_value": item.metadata.default_value,
                    "validation_rules": item.metadata.validation_rules,
                    "is_required": item.metadata.is_required,
                    "is_sensitive": item.metadata.is_sensitive,
                    "is_readonly": item.metadata.is_readonly,
                    "version": item.metadata.version,
                    "last_modified": item.last_modified,
                    "last_modified_by": item.last_modified_by,
                })
            })
            .collect();

        Ok(json!({
            "export_version": "1.0.0",
            "export_date": Utc::now(),
            "config_count": export_data.len(),
            "configs": export_data,
        }))
    }

    /// 导入配置
    pub async fn import_configs(
        &self,
        import_data: &Value,
        metadata_list: &[ConfigMetadata],
        _strategy: ImportStrategy,
    ) -> Result<ImportResult> {
        // 简化实现，实际应该更复杂
        let configs = import_data.get("configs")
            .and_then(|c| c.as_array())
            .context("导入数据格式无效")?;

        let mut result = ImportResult {
            total_items: configs.len(),
            imported_items: 0,
            skipped_items: 0,
            failed_items: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        for config in configs {
            let key = config.get("key")
                .and_then(|k| k.as_str())
                .context("配置项缺少key字段")?;

            // 查找对应的元数据
            if let Some(metadata) = metadata_list.iter().find(|m| m.key == key) {
                let value = config.get("value")
                    .cloned()
                    .unwrap_or_else(|| metadata.default_value.clone());

                // 验证配置值
                let validation_result = ConfigValidator::validate(metadata, &value);
                if !validation_result.is_valid {
                    result.failed_items += 1;
                    result.errors.push(ImportError {
                        code: "validation_failed".to_string(),
                        message: format!("配置验证失败: {}", key),
                        key: Some(key.to_string()),
                        details: Some(json!(validation_result.errors)),
                    });
                    continue;
                }

                // 创建配置项
                let mut item = ConfigItem::new(metadata.clone());
                item.update_value(value, "import");

                // 保存配置
                if let Err(e) = self.save_config(&item).await {
                    result.failed_items += 1;
                    result.errors.push(ImportError {
                        code: "save_failed".to_string(),
                        message: format!("保存配置失败: {}", e),
                        key: Some(key.to_string()),
                        details: None,
                    });
                } else {
                    result.imported_items += 1;
                }
            } else {
                result.skipped_items += 1;
                result.warnings.push(ImportWarning {
                    code: "metadata_not_found".to_string(),
                    message: format!("配置元数据未找到: {}", key),
                    key: Some(key.to_string()),
                    details: None,
                });
            }
        }

        Ok(result)
    }

    /// 将分类转换为字符串
    fn category_to_string(category: &ConfigCategory) -> String {
        match category {
            ConfigCategory::System => "system",
            ConfigCategory::Compression => "compression",
            ConfigCategory::Security => "security",
            ConfigCategory::Ui => "ui",
            ConfigCategory::Network => "network",
            ConfigCategory::Storage => "storage",
            ConfigCategory::Advanced => "advanced",
            ConfigCategory::Other => "other",
        }.to_string()
    }

    /// 将字符串转换为分类
    fn string_to_category(category_str: &str) -> ConfigCategory {
        match category_str.to_lowercase().as_str() {
            "system" => ConfigCategory::System,
            "compression" => ConfigCategory::Compression,
            "security" => ConfigCategory::Security,
            "ui" => ConfigCategory::Ui,
            "network" => ConfigCategory::Network,
            "storage" => ConfigCategory::Storage,
            "advanced" => ConfigCategory::Advanced,
            _ => ConfigCategory::Other,
        }
    }
}

/// 配置统计信息
#[derive(Debug)]
pub struct ConfigStatistics {
    pub total_count: usize,
    pub category_counts: HashMap<ConfigCategory, usize>,
    pub last_updated: Option<DateTime<Utc>>,
}

/// 导入策略
#[derive(Debug, Clone, Copy)]
pub enum ImportStrategy {
    Merge,
    Replace,
    UpdateOnly,
    SkipExisting,
}

/// 导入结果
#[derive(Debug)]
pub struct ImportResult {
    pub total_items: usize,
    pub imported_items: usize,
    pub skipped_items: usize,
    pub failed_items: usize,
    pub errors: Vec<ImportError>,
    pub warnings: Vec<ImportWarning>,
}

/// 导入错误
#[derive(Debug)]
pub struct ImportError {
    pub code: String,
    pub message: String,
    pub key: Option<String>,
    pub details: Option<Value>,
}

/// 导入警告
#[derive(Debug)]
pub struct ImportWarning {
    pub code: String,
    pub message: String,
    pub key: Option<String>,
    pub details: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::models::{ConfigDataType, DefaultConfigGenerator, ValidationRule};
    use sqlx::SqlitePool;
    use tempfile::tempdir;

    async fn create_test_pool() -> SqlitePool {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = SqlitePool::connect(&format!("sqlite:{}", db_path.display()))
            .await
            .unwrap();
        pool
    }

    #[tokio::test]
    async fn test_init_tables() {
        let pool = create_test_pool().await;
        let repo = ConfigRepository::new(pool);

        let result = repo.init_tables().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_save_and_get_config() {
        let pool = create_test_pool().await;
        let repo = ConfigRepository::new(pool);
        repo.init_tables().await.unwrap();

        let metadata = ConfigMetadata {
            key: "test.config".to_string(),
            category: ConfigCategory::System,
            display_name: "测试配置".to_string(),
            description: "测试配置项".to_string(),
            data_type: ConfigDataType::String,
            default_value: Value::String("default".to_string()),
            validation_rules: vec![ValidationRule::MinLength { value: 3 }],
            is_required: false,
            is_sensitive: false,
            is_readonly: false,
            version: "1.0.0".to_string(),
            sort_order: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut item = ConfigItem::new(metadata.clone());
        item.update_value(Value::String("test value".to_string()), "test");

        // 保存配置
        let result = repo.save_config(&item).await;
        assert!(result.is_ok());

        // 获取配置
        let retrieved = repo.get_config("test.config", &metadata).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved_item = retrieved.unwrap();
        assert_eq!(retrieved_item.metadata.key, "test.config");
        assert_eq!(retrieved_item.current_value, Value::String("test value".to_string()));
    }

    #[tokio::test]
    async fn test_delete_config() {
        let pool = create_test_pool().await;
        let repo = ConfigRepository::new(pool);
        repo.init_tables().await.unwrap();

        let metadata = ConfigMetadata {
            key: "test.delete".to_string(),
            category: ConfigCategory::System,
            display_name: "删除测试".to_string(),
            description: "删除测试配置".to_string(),
            data_type: ConfigDataType::String,
            default_value: Value::String("default".to_string()),
            validation_rules: vec![],
            is_required: false,
            is_sensitive: false,
            is_readonly: false,
            version: "1.0.0".to_string(),
            sort_order: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let item = ConfigItem::new(metadata.clone());
        repo.save_config(&item).await.unwrap();

        // 删除配置
        let result = repo.delete_config("test.delete").await;
        assert!(result.is_ok());

        // 验证配置已删除
        let retrieved = repo.get_config("test.delete", &metadata).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_reset_to_default() {
        let pool = create_test_pool().await;
        let repo = ConfigRepository::new(pool);
        repo.init_tables().await.unwrap();

        let metadata = ConfigMetadata {
            key: "test.reset".to_string(),
            category: ConfigCategory::System,
            display_name: "重置测试".to_string(),
            description: "重置测试配置".to_string(),
            data_type: ConfigDataType::String,
            default_value: Value::String("default value".to_string()),
            validation_rules: vec![],
            is_required: false,
            is_sensitive: false,
            is_readonly: false,
            version: "1.0.0".to_string(),
            sort_order: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut item = ConfigItem::new(metadata.clone());
        item.update_value(Value::String("custom value".to_string()), "test");
        repo.save_config(&item).await.unwrap();

        // 重置为默认值
        let reset_item = repo.reset_to_default("test.reset", &metadata).await.unwrap();
        assert_eq!(reset_item.current_value, Value::String("default value".to_string()));

        // 验证数据库中已更新
        let retrieved = repo.get_config("test.reset", &metadata).await.unwrap().unwrap();
        assert_eq!(retrieved.current_value, Value::String("default value".to_string()));
    }

    #[tokio::test]
    async fn test_get_statistics() {
        let pool = create_test_pool().await;
        let repo = ConfigRepository::new(pool);
        repo.init_tables().await.unwrap();

        // 添加一些测试配置
        let metadata1 = ConfigMetadata {
            key: "test.stat1".to_string(),
            category: ConfigCategory::System,
            display_name: "统计测试1".to_string(),
            description: "统计测试配置1".to_string(),
            data_type: ConfigDataType::String,
            default_value: Value::String("default".to_string()),
            validation_rules: vec![],
            is_required: false,
            is_sensitive: false,
            is_readonly: false,
            version: "1.0.0".to_string(),
            sort_order: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let metadata2 = ConfigMetadata {
            key: "test.stat2".to_string(),
            category: ConfigCategory::Compression,
            display_name: "统计测试2".to_string(),
            description: "统计测试配置2".to_string(),
            data_type: ConfigDataType::Integer,
            default_value: Value::Number(100.into()),
            validation_rules: vec![],
            is_required: false,
            is_sensitive: false,
            is_readonly: false,
            version: "1.0.0".to_string(),
            sort_order: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let item1 = ConfigItem::new(metadata1);
        let item2 = ConfigItem::new(metadata2);

        repo.save_config(&item1).await.unwrap();
        repo.save_config(&item2).await.unwrap();

        // 获取统计信息
        let stats = repo.get_statistics().await.unwrap();
        assert_eq!(stats.total_count, 2);
        assert_eq!(stats.category_counts.get(&ConfigCategory::System), Some(&1));
        assert_eq!(stats.category_counts.get(&ConfigCategory::Compression), Some(&1));
        assert!(stats.last_updated.is_some());
    }
}