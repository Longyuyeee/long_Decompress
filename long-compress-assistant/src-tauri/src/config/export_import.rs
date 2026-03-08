//! 配置导入导出模块
//!
//! 提供配置的导入导出功能，支持多种格式（JSON、YAML、TOML）。

use crate::config::models::{ConfigItem, ExportFormat, ImportResult, ImportStrategy};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;

/// 配置导出器
pub struct ConfigExporter;

impl ConfigExporter {
    /// 导出配置到指定格式
    pub fn export(
        items: &[ConfigItem],
        format: ExportFormat,
    ) -> Result<Vec<u8>> {
        let export_data = Self::prepare_export_data(items)?;

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

    /// 准备导出数据
    fn prepare_export_data(items: &[ConfigItem]) -> Result<Value> {
        use chrono::Utc;

        let configs: Vec<Value> = items
            .iter()
            .map(|item| {
                let category_str = match item.metadata.category {
                    crate::config::models::ConfigCategory::System => "system",
                    crate::config::models::ConfigCategory::Compression => "compression",
                    crate::config::models::ConfigCategory::Security => "security",
                    crate::config::models::ConfigCategory::Ui => "ui",
                    crate::config::models::ConfigCategory::Network => "network",
                    crate::config::models::ConfigCategory::Storage => "storage",
                    crate::config::models::ConfigCategory::Advanced => "advanced",
                    crate::config::models::ConfigCategory::Other => "other",
                };

                serde_json::json!({
                    "key": item.metadata.key,
                    "category": category_str,
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
                    "last_modified": item.last_modified.to_rfc3339(),
                    "last_modified_by": item.last_modified_by,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "export_version": "1.0.0",
            "export_date": Utc::now().to_rfc3339(),
            "config_count": configs.len(),
            "configs": configs,
        }))
    }
}

/// 配置导入器
pub struct ConfigImporter;

impl ConfigImporter {
    /// 从指定格式导入配置
    pub fn import(
        data: &[u8],
        format: ExportFormat,
        existing_items: &HashMap<String, ConfigItem>,
        strategy: ImportStrategy,
    ) -> Result<ImportResult> {
        let import_data = Self::parse_import_data(data, format)?;
        Self::apply_import(&import_data, existing_items, strategy)
    }

    /// 解析导入数据
    fn parse_import_data(data: &[u8], format: ExportFormat) -> Result<Value> {
        match format {
            ExportFormat::Json => {
                serde_json::from_slice(data).context("解析JSON失败")
            }
            ExportFormat::Yaml => {
                serde_yaml::from_slice(data).context("解析YAML失败")
            }
            ExportFormat::Toml => {
                toml::from_slice(data).context("解析TOML失败")
            }
        }
    }

    /// 应用导入数据
    fn apply_import(
        import_data: &Value,
        existing_items: &HashMap<String, ConfigItem>,
        strategy: ImportStrategy,
    ) -> Result<ImportResult> {
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

            // 根据策略决定是否导入
            let should_import = match strategy {
                ImportStrategy::Merge => true,
                ImportStrategy::Replace => true,
                ImportStrategy::UpdateOnly => existing_items.contains_key(key),
                ImportStrategy::SkipExisting => !existing_items.contains_key(key),
            };

            if !should_import {
                result.skipped_items += 1;
                continue;
            }

            // 这里简化实现，实际应该验证和转换数据
            result.imported_items += 1;
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::models::{ConfigCategory, ConfigDataType, ConfigItem, ConfigMetadata, ValidationRule};
    use chrono::Utc;
    use serde_json::json;

    fn create_test_item() -> ConfigItem {
        let metadata = ConfigMetadata {
            key: "test.config".to_string(),
            category: ConfigCategory::System,
            display_name: "测试配置".to_string(),
            description: "测试配置项".to_string(),
            data_type: ConfigDataType::String,
            default_value: json!("default"),
            validation_rules: vec![ValidationRule::MinLength { value: 3 }],
            is_required: false,
            is_sensitive: false,
            is_readonly: false,
            version: "1.0.0".to_string(),
            sort_order: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut item = ConfigItem::new(metadata);
        item.update_value(json!("test value"), "test");
        item
    }

    #[test]
    fn test_export_json() {
        let item = create_test_item();
        let items = vec![item];

        let result = ConfigExporter::export(&items, ExportFormat::Json);
        assert!(result.is_ok());

        let export_data = result.unwrap();
        assert!(!export_data.is_empty());

        // 验证导出的JSON可以解析
        let parsed: Value = serde_json::from_slice(&export_data).unwrap();
        assert!(parsed.get("configs").is_some());
        assert!(parsed.get("export_version").is_some());
    }

    #[test]
    fn test_export_yaml() {
        let item = create_test_item();
        let items = vec![item];

        let result = ConfigExporter::export(&items, ExportFormat::Yaml);
        assert!(result.is_ok());

        let export_data = result.unwrap();
        assert!(!export_data.is_empty());
    }

    #[test]
    fn test_export_toml() {
        let item = create_test_item();
        let items = vec![item];

        let result = ConfigExporter::export(&items, ExportFormat::Toml);
        assert!(result.is_ok());

        let export_data = result.unwrap();
        assert!(!export_data.is_empty());
    }

    #[test]
    fn test_import_json() {
        let export_data = json!({
            "export_version": "1.0.0",
            "export_date": "2024-01-01T00:00:00Z",
            "config_count": 1,
            "configs": [{
                "key": "test.config",
                "category": "system",
                "display_name": "测试配置",
                "description": "测试配置项",
                "data_type": "string",
                "value": "imported value",
                "default_value": "default",
                "validation_rules": [],
                "is_required": false,
                "is_sensitive": false,
                "is_readonly": false,
                "version": "1.0.0",
                "last_modified": "2024-01-01T00:00:00Z",
                "last_modified_by": "import"
            }]
        });

        let json_data = serde_json::to_vec(&export_data).unwrap();
        let existing_items = HashMap::new();

        let result = ConfigImporter::import(
            &json_data,
            ExportFormat::Json,
            &existing_items,
            ImportStrategy::Merge,
        );

        assert!(result.is_ok());
        let import_result = result.unwrap();
        assert_eq!(import_result.total_items, 1);
        assert_eq!(import_result.imported_items, 1);
    }

    #[test]
    fn test_import_strategies() {
        let export_data = json!({
            "export_version": "1.0.0",
            "export_date": "2024-01-01T00:00:00Z",
            "config_count": 2,
            "configs": [
                {
                    "key": "existing.config",
                    "category": "system",
                    "display_name": "现有配置",
                    "description": "现有配置项",
                    "data_type": "string",
                    "value": "new value",
                    "default_value": "default",
                    "validation_rules": [],
                    "is_required": false,
                    "is_sensitive": false,
                    "is_readonly": false,
                    "version": "1.0.0",
                    "last_modified": "2024-01-01T00:00:00Z",
                    "last_modified_by": "import"
                },
                {
                    "key": "new.config",
                    "category": "system",
                    "display_name": "新配置",
                    "description": "新配置项",
                    "data_type": "string",
                    "value": "new value",
                    "default_value": "default",
                    "validation_rules": [],
                    "is_required": false,
                    "is_sensitive": false,
                    "is_readonly": false,
                    "version": "1.0.0",
                    "last_modified": "2024-01-01T00:00:00Z",
                    "last_modified_by": "import"
                }
            ]
        });

        let json_data = serde_json::to_vec(&export_data).unwrap();

        // 创建一个现有的配置项
        let mut existing_items = HashMap::new();
        let existing_item = create_test_item();
        existing_items.insert("existing.config".to_string(), existing_item);

        // 测试UpdateOnly策略
        let result = ConfigImporter::import(
            &json_data,
            ExportFormat::Json,
            &existing_items,
            ImportStrategy::UpdateOnly,
        ).unwrap();

        assert_eq!(result.total_items, 2);
        assert_eq!(result.imported_items, 1); // 只更新现有的
        assert_eq!(result.skipped_items, 1); // 跳过新的

        // 测试SkipExisting策略
        let result = ConfigImporter::import(
            &json_data,
            ExportFormat::Json,
            &existing_items,
            ImportStrategy::SkipExisting,
        ).unwrap();

        assert_eq!(result.total_items, 2);
        assert_eq!(result.imported_items, 1); // 只导入新的
        assert_eq!(result.skipped_items, 1); // 跳过现有的
    }
}