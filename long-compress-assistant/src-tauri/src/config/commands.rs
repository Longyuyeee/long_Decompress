use crate::config::models::{ConfigCategory, ConfigItem, ExportFormat, ImportStrategy};
use crate::config::service::ConfigService;
use crate::database::connection::get_connection;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tauri::command;

/// 配置服务单例
static mut CONFIG_SERVICE: Option<ConfigService> = None;

/// 初始化配置服务
async fn get_config_service() -> Result<&'static ConfigService> {
    unsafe {
        if CONFIG_SERVICE.is_none() {
            let pool = get_connection()?.pool().clone();
            let service = ConfigService::new(pool);
            service.init().await.context("初始化配置服务失败")?;
            CONFIG_SERVICE = Some(service);
        }
        Ok(CONFIG_SERVICE.as_ref().unwrap())
    }
}

/// 获取配置项请求
#[derive(Debug, Deserialize)]
pub struct GetConfigRequest {
    pub key: String,
}

/// 设置配置项请求
#[derive(Debug, Deserialize)]
pub struct SetConfigRequest {
    pub key: String,
    pub value: Value,
}

/// 批量设置配置项请求
#[derive(Debug, Deserialize)]
pub struct BatchSetConfigRequest {
    pub updates: HashMap<String, Value>,
}

/// 重置配置请求
#[derive(Debug, Deserialize)]
pub struct ResetConfigRequest {
    pub key: String,
}

/// 批量重置配置请求
#[derive(Debug, Deserialize)]
pub struct BatchResetConfigRequest {
    pub keys: Vec<String>,
}

/// 搜索配置请求
#[derive(Debug, Deserialize)]
pub struct SearchConfigsRequest {
    pub query: String,
}

/// 导出配置请求
#[derive(Debug, Deserialize)]
pub struct ExportConfigsRequest {
    pub format: ExportFormat,
}

/// 导入配置请求
#[derive(Debug, Deserialize)]
pub struct ImportConfigsRequest {
    pub data: Vec<u8>,
    pub format: ExportFormat,
    pub strategy: ImportStrategy,
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
    pub last_modified: String,
    pub last_modified_by: String,
}

impl From<ConfigItem> for ConfigItemResponse {
    fn from(item: ConfigItem) -> Self {
        Self {
            key: item.metadata.key,
            category: match item.metadata.category {
                ConfigCategory::System => "system".to_string(),
                ConfigCategory::Compression => "compression".to_string(),
                ConfigCategory::Security => "security".to_string(),
                ConfigCategory::Ui => "ui".to_string(),
                ConfigCategory::Network => "network".to_string(),
                ConfigCategory::Storage => "storage".to_string(),
                ConfigCategory::Advanced => "advanced".to_string(),
                ConfigCategory::Other => "other".to_string(),
            },
            display_name: item.metadata.display_name,
            description: item.metadata.description,
            data_type: item.metadata.data_type.as_str().to_string(),
            value: item.current_value,
            default_value: item.metadata.default_value,
            is_required: item.metadata.is_required,
            is_sensitive: item.metadata.is_sensitive,
            is_readonly: item.metadata.is_readonly,
            version: item.metadata.version,
            last_modified: item.last_modified.to_rfc3339(),
            last_modified_by: item.last_modified_by,
        }
    }
}

/// 验证结果响应
#[derive(Debug, Serialize)]
pub struct ValidationResultResponse {
    pub is_valid: bool,
    pub errors: Vec<ValidationErrorResponse>,
    pub warnings: Vec<ValidationWarningResponse>,
}

/// 验证错误响应
#[derive(Debug, Serialize)]
pub struct ValidationErrorResponse {
    pub code: String,
    pub message: String,
    pub field: String,
    pub details: Option<Value>,
}

/// 验证警告响应
#[derive(Debug, Serialize)]
pub struct ValidationWarningResponse {
    pub code: String,
    pub message: String,
    pub field: String,
    pub details: Option<Value>,
}

/// 统计信息响应
#[derive(Debug, Serialize)]
pub struct StatisticsResponse {
    pub total_configs: usize,
    pub cached_configs: usize,
    pub category_counts: HashMap<String, usize>,
    pub last_updated: Option<String>,
    pub cache_hit_rate: f64,
    pub cache_miss_rate: f64,
}

/// 导入结果响应
#[derive(Debug, Serialize)]
pub struct ImportResultResponse {
    pub total_items: usize,
    pub imported_items: usize,
    pub skipped_items: usize,
    pub failed_items: usize,
    pub errors: Vec<ImportErrorResponse>,
    pub warnings: Vec<ImportWarningResponse>,
}

/// 导入错误响应
#[derive(Debug, Serialize)]
pub struct ImportErrorResponse {
    pub code: String,
    pub message: String,
    pub key: Option<String>,
    pub details: Option<Value>,
}

/// 导入警告响应
#[derive(Debug, Serialize)]
pub struct ImportWarningResponse {
    pub code: String,
    pub message: String,
    pub key: Option<String>,
    pub details: Option<Value>,
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

/// 根据分类获取配置项
#[command]
pub async fn get_configs_by_category(category: String) -> Result<Vec<ConfigItemResponse>, String> {
    let service = get_config_service()
        .await
        .map_err(|e| format!("获取配置服务失败: {}", e))?;

    let config_category = match category.as_str() {
        "system" => ConfigCategory::System,
        "compression" => ConfigCategory::Compression,
        "security" => ConfigCategory::Security,
        "ui" => ConfigCategory::Ui,
        "network" => ConfigCategory::Network,
        "storage" => ConfigCategory::Storage,
        "advanced" => ConfigCategory::Advanced,
        "other" => ConfigCategory::Other,
        _ => return Err(format!("未知的分类: {}", category)),
    };

    let items = service
        .get_configs_by_category(config_category)
        .await
        .map_err(|e| format!("根据分类获取配置失败: {}", e))?;

    Ok(items.into_iter().map(ConfigItemResponse::from).collect())
}

/// 获取配置项
#[command]
pub async fn get_config(request: GetConfigRequest) -> Result<Option<ConfigItemResponse>, String> {
    let service = get_config_service()
        .await
        .map_err(|e| format!("获取配置服务失败: {}", e))?;

    let item = service
        .get_config(&request.key)
        .await
        .map_err(|e| format!("获取配置失败: {}", e))?;

    Ok(item.map(ConfigItemResponse::from))
}

/// 获取配置值
#[command]
pub async fn get_config_value(key: String) -> Result<Option<Value>, String> {
    let service = get_config_service()
        .await
        .map_err(|e| format!("获取配置服务失败: {}", e))?;

    service
        .get_value(&key)
        .await
        .map_err(|e| format!("获取配置值失败: {}", e))
}

/// 获取字符串配置值
#[command]
pub async fn get_string_config(key: String) -> Result<Option<String>, String> {
    let service = get_config_service()
        .await
        .map_err(|e| format!("获取配置服务失败: {}", e))?;

    service
        .get_string(&key)
        .await
        .map_err(|e| format!("获取字符串配置失败: {}", e))
}

/// 获取整数配置值
#[command]
pub async fn get_integer_config(key: String) -> Result<Option<i64>, String> {
    let service = get_config_service()
        .await
        .map_err(|e| format!("获取配置服务失败: {}", e))?;

    service
        .get_integer(&key)
        .await
        .map_err(|e| format!("获取整数配置失败: {}", e))
}

/// 获取浮点数配置值
#[command]
pub async fn get_float_config(key: String) -> Result<Option<f64>, String> {
    let service = get_config_service()
        .await
        .map_err(|e| format!("获取配置服务失败: {}", e))?;

    service
        .get_float(&key)
        .await
        .map_err(|e| format!("获取浮点数配置失败: {}", e))
}

/// 获取布尔配置值
#[command]
pub async fn get_boolean_config(key: String) -> Result<Option<bool>, String> {
    let service = get_config_service()
        .await
        .map_err(|e| format!("获取配置服务失败: {}", e))?;

    service
        .get_boolean(&key)
        .await
        .map_err(|e| format!("获取布尔配置失败: {}", e))
}

/// 设置配置项
#[command]
pub async fn set_config(request: SetConfigRequest) -> Result<(), String> {
    let service = get_config_service()
        .await
        .map_err(|e| format!("获取配置服务失败: {}", e))?;

    service
        .set_config(&request.key, request.value, "user")
        .await
        .map_err(|e| format!("设置配置失败: {}", e))
}

/// 批量设置配置项
#[command]
pub async fn batch_set_configs(request: BatchSetConfigRequest) -> Result<(), String> {
    let service = get_config_service()
        .await
        .map_err(|e| format!("获取配置服务失败: {}", e))?;

    service
        .batch_set_configs(request.updates, "user")
        .await
        .map_err(|e| format!("批量设置配置失败: {}", e))
}

/// 重置配置为默认值
#[command]
pub async fn reset_config(request: ResetConfigRequest) -> Result<(), String> {
    let service = get_config_service()
        .await
        .map_err(|e| format!("获取配置服务失败: {}", e))?;

    service
        .reset_to_default(&request.key, "user")
        .await
        .map_err(|e| format!("重置配置失败: {}", e))
}

/// 批量重置配置为默认值
#[command]
pub async fn batch_reset_configs(request: BatchResetConfigRequest) -> Result<(), String> {
    let service = get_config_service()
        .await
        .map_err(|e| format!("获取配置服务失败: {}", e))?;

    service
        .batch_reset_to_default(request.keys, "user")
        .await
        .map_err(|e| format!("批量重置配置失败: {}", e))
}

/// 验证配置值
#[command]
pub async fn validate_config(key: String, value: Value) -> Result<ValidationResultResponse, String> {
    let service = get_config_service()
        .await
        .map_err(|e| format!("获取配置服务失败: {}", e))?;

    let result = service
        .validate_config(&key, &value)
        .await
        .map_err(|e| format!("验证配置失败: {}", e))?;

    Ok(ValidationResultResponse {
        is_valid: result.is_valid,
        errors: result.errors.into_iter().map(|e| ValidationErrorResponse {
            code: e.code,
            message: e.message,
            field: e.field,
            details: e.details,
        }).collect(),
        warnings: result.warnings.into_iter().map(|w| ValidationWarningResponse {
            code: w.code,
            message: w.message,
            field: w.field,
            details: w.details,
        }).collect(),
    })
}

/// 搜索配置项
#[command]
pub async fn search_configs(request: SearchConfigsRequest) -> Result<Vec<ConfigItemResponse>, String> {
    let service = get_config_service()
        .await
        .map_err(|e| format!("获取配置服务失败: {}", e))?;

    let items = service
        .search_configs(&request.query)
        .await
        .map_err(|e| format!("搜索配置失败: {}", e))?;

    Ok(items.into_iter().map(ConfigItemResponse::from).collect())
}

/// 导出配置
#[command]
pub async fn export_configs(request: ExportConfigsRequest) -> Result<Vec<u8>, String> {
    let service = get_config_service()
        .await
        .map_err(|e| format!("获取配置服务失败: {}", e))?;

    service
        .export_configs(request.format)
        .await
        .map_err(|e| format!("导出配置失败: {}", e))
}

/// 导入配置
#[command]
pub async fn import_configs(request: ImportConfigsRequest) -> Result<ImportResultResponse, String> {
    let service = get_config_service()
        .await
        .map_err(|e| format!("获取配置服务失败: {}", e))?;

    let result = service
        .import_configs(&request.data, request.format, request.strategy, "user")
        .await
        .map_err(|e| format!("导入配置失败: {}", e))?;

    Ok(ImportResultResponse {
        total_items: result.total_items,
        imported_items: result.imported_items,
        skipped_items: result.skipped_items,
        failed_items: result.failed_items,
        errors: result.errors.into_iter().map(|e| ImportErrorResponse {
            code: e.code,
            message: e.message,
            key: e.key,
            details: e.details,
        }).collect(),
        warnings: result.warnings.into_iter().map(|w| ImportWarningResponse {
            code: w.code,
            message: w.message,
            key: w.key,
            details: w.details,
        }).collect(),
    })
}

/// 获取配置统计信息
#[command]
pub async fn get_config_statistics() -> Result<StatisticsResponse, String> {
    let service = get_config_service()
        .await
        .map_err(|e| format!("获取配置服务失败: {}", e))?;

    let stats = service
        .get_statistics()
        .await
        .map_err(|e| format!("获取统计信息失败: {}", e))?;

    let category_counts: HashMap<String, usize> = stats.category_counts
        .into_iter()
        .map(|(category, count)| {
            let category_str = match category {
                ConfigCategory::System => "system",
                ConfigCategory::Compression => "compression",
                ConfigCategory::Security => "security",
                ConfigCategory::Ui => "ui",
                ConfigCategory::Network => "network",
                ConfigCategory::Storage => "storage",
                ConfigCategory::Advanced => "advanced",
                ConfigCategory::Other => "other",
            };
            (category_str.to_string(), count)
        })
        .collect();

    Ok(StatisticsResponse {
        total_configs: stats.total_configs,
        cached_configs: stats.cached_configs,
        category_counts,
        last_updated: stats.last_updated.map(|dt| dt.to_rfc3339()),
        cache_hit_rate: stats.cache_hit_rate,
        cache_miss_rate: stats.cache_miss_rate,
    })
}

/// 刷新配置缓存
#[command]
pub async fn refresh_config_cache() -> Result<(), String> {
    let service = get_config_service()
        .await
        .map_err(|e| format!("获取配置服务失败: {}", e))?;

    service
        .refresh_cache()
        .await
        .map_err(|e| format!("刷新缓存失败: {}", e))
}

/// 清空配置缓存
#[command]
pub async fn clear_config_cache() -> Result<(), String> {
    let service = get_config_service()
        .await
        .map_err(|e| format!("获取配置服务失败: {}", e))?;

    service
        .clear_cache()
        .await
        .map_err(|e| format!("清空缓存失败: {}", e))
}

/// 获取所有配置分类
#[command]
pub async fn get_config_categories() -> Result<Vec<CategoryInfo>, String> {
    let categories = vec![
        CategoryInfo {
            id: "system".to_string(),
            name: "系统配置".to_string(),
            description: "系统相关配置，如监控、更新等".to_string(),
            icon: "settings".to_string(),
            count: 0, // 实际应该从数据库获取
        },
        CategoryInfo {
            id: "compression".to_string(),
            name: "压缩配置".to_string(),
            description: "压缩解压相关配置".to_string(),
            icon: "archive".to_string(),
            count: 0,
        },
        CategoryInfo {
            id: "security".to_string(),
            name: "安全配置".to_string(),
            description: "安全相关配置，如密码、加密等".to_string(),
            icon: "lock".to_string(),
            count: 0,
        },
        CategoryInfo {
            id: "ui".to_string(),
            name: "界面配置".to_string(),
            description: "用户界面相关配置".to_string(),
            icon: "palette".to_string(),
            count: 0,
        },
        CategoryInfo {
            id: "network".to_string(),
            name: "网络配置".to_string(),
            description: "网络相关配置".to_string(),
            icon: "wifi".to_string(),
            count: 0,
        },
        CategoryInfo {
            id: "storage".to_string(),
            name: "存储配置".to_string(),
            description: "存储相关配置".to_string(),
            icon: "database".to_string(),
            count: 0,
        },
        CategoryInfo {
            id: "advanced".to_string(),
            name: "高级配置".to_string(),
            description: "高级功能和实验性配置".to_string(),
            icon: "flask".to_string(),
            count: 0,
        },
    ];

    Ok(categories)
}

/// 分类信息
#[derive(Debug, Serialize)]
pub struct CategoryInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub count: usize,
}