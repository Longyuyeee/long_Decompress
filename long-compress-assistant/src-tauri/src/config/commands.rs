use crate::config::file_loader::{ConfigFileFormat, ConfigFileLoader};
use crate::config::models::{ConfigCategory, ConfigItem, ExportFormat, ImportStrategy};
use crate::config::service::ConfigService;
use crate::database::connection::get_connection;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tauri::command;
use tokio::sync::OnceCell;

/// 配置服务单例
static CONFIG_SERVICE_INSTANCE: OnceCell<ConfigService> = OnceCell::const_new();

/// 初始化全局配置服务
pub async fn set_global_config_service(service: ConfigService) -> Result<()> {
    CONFIG_SERVICE_INSTANCE.set(service).map_err(|_| anyhow::anyhow!("配置服务已存在"))
}

/// 获取配置服务
async fn get_config_service() -> Result<&'static ConfigService> {
    CONFIG_SERVICE_INSTANCE.get().ok_or_else(|| anyhow::anyhow!("配置服务未初始化"))
}

/// 获取所有配置项
#[command]
pub async fn get_all_configs() -> Result<Vec<ConfigItemResponse>, String> {
    let service = get_config_service()
        .await
        .map_err(|e| format!("获取配置服务失败: {}", e))?;

    let items = service
        .get_all_configs()
        .await
        .map_err(|e| format!("获取所有配置失败: {}", e))?;

    Ok(items.into_iter().map(ConfigItemResponse::from).collect())
}

/// 配置项响应
#[derive(Debug, Serialize)]
pub struct ConfigItemResponse {
    pub key: String,
    pub category: String,
    pub display_name: String,
    pub description: String,
    pub data_type: String,
    pub value: Value,
    pub default_value: Value,
    pub is_required: bool,
    pub is_sensitive: bool,
    pub is_readonly: bool,
    pub version: String,
}

impl From<ConfigItem> for ConfigItemResponse {
    fn from(item: ConfigItem) -> Self {
        Self {
            key: item.metadata.key,
            category: format!("{:?}", item.metadata.category),
            display_name: item.metadata.display_name,
            description: item.metadata.description,
            data_type: format!("{:?}", item.metadata.data_type),
            value: item.current_value,
            default_value: item.metadata.default_value,
            is_required: item.metadata.is_required,
            is_sensitive: item.metadata.is_sensitive,
            is_readonly: item.metadata.is_readonly,
            version: item.metadata.version,
        }
    }
}

// ... 暂时只保留核心命令以通过编译，保持最小侵入
