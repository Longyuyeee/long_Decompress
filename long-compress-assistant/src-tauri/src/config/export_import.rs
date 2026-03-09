use crate::config::models::{ConfigItem, ConfigMetadata, ExportFormat, ImportResult, ImportStrategy, ValidationError, ValidationResult};
use crate::config::validation::ConfigValidator;
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::HashMap;

pub struct ConfigExportImport;

impl ConfigExportImport {
    /// 解析导入数据
    pub fn parse_import_data(data: &[u8], format: ExportFormat) -> Result<Value> {
        match format {
            ExportFormat::Json => {
                serde_json::from_slice(data).context("解析JSON失败")
            }
            ExportFormat::Yaml => {
                serde_yaml::from_slice(data).context("解析YAML失败")
            }
            ExportFormat::Toml => {
                let s = String::from_utf8_lossy(data);
                let toml_value: toml::Value = toml::from_str(&s).map_err(|e| anyhow::anyhow!("解析TOML失败: {}", e))?;
                // 简单将 toml::Value 转为 json::Value
                Ok(json!(toml_value))
            }
        }
    }
}
