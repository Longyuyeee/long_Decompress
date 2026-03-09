//! 配置文件加载器
//!
//! 提供从文件系统加载配置的功能，支持多种格式（JSON、YAML、TOML）。
//! 支持配置文件热重载和环境变量覆盖。

use crate::config::models::{ConfigCategory, ConfigDataType, ConfigItem, ConfigMetadata, ValidationRule};
use anyhow::{Context, Result};
use chrono::Utc;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

/// 配置文件格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigFileFormat {
    /// JSON格式
    Json,
    /// YAML格式
    Yaml,
    /// TOML格式
    Toml,
    /// 自动检测
    Auto,
}

impl ConfigFileFormat {
    /// 从文件扩展名检测格式
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.to_lowercase().as_str() {
            "json" => Some(ConfigFileFormat::Json),
            "yaml" | "yml" => Some(ConfigFileFormat::Yaml),
            "toml" => Some(ConfigFileFormat::Toml),
            _ => None,
        }
    }

    /// 从文件路径检测格式
    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(Self::from_extension)
    }
}

/// 配置文件加载器
pub struct ConfigFileLoader {
    pub config_dir: PathBuf,
    format: ConfigFileFormat,
    watcher: Option<RecommendedWatcher>,
    file_watches: Arc<RwLock<HashMap<PathBuf, FileWatchInfo>>>,
}

/// 文件监视信息
struct FileWatchInfo {
    last_modified: std::time::SystemTime,
    callback: Box<dyn Fn(&Path) + Send + Sync>,
}

impl ConfigFileLoader {
    /// 创建新的配置文件加载器
    pub fn new(config_dir: impl Into<PathBuf>, format: ConfigFileFormat) -> Self {
        Self {
            config_dir: config_dir.into(),
            format,
            watcher: None,
            file_watches: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 从环境变量创建配置文件加载器
    pub fn from_env() -> Result<Self> {
        let config_dir = std::env::var("CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let mut dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("./config"));
                dir.push("long-compress-assistant");
                dir
            });

        let format = std::env::var("CONFIG_FORMAT")
            .ok()
            .and_then(|f| match f.to_lowercase().as_str() {
                "json" => Some(ConfigFileFormat::Json),
                "yaml" => Some(ConfigFileFormat::Yaml),
                "toml" => Some(ConfigFileFormat::Toml),
                "auto" => Some(ConfigFileFormat::Auto),
                _ => None,
            })
            .unwrap_or(ConfigFileFormat::Auto);

        Ok(Self::new(config_dir, format))
    }

    /// 加载配置文件
    pub async fn load_config_file(&self, filename: &str) -> Result<Vec<ConfigItem>> {
        let path = self.config_dir.join(filename);
        self.load_config_file_from_path(&path).await
    }

    /// 从指定路径加载配置文件
    pub async fn load_config_file_from_path(&self, path: &Path) -> Result<Vec<ConfigItem>> {
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(path)
            .await
            .context(format!("读取配置文件失败: {:?}", path))?;

        let format = if self.format == ConfigFileFormat::Auto {
            Self::detect_format(&content, path)
        } else {
            self.format
        };

        let value = self.parse_content(&content, format)
            .context(format!("解析配置文件失败: {:?}", path))?;

        self.convert_to_config_items(&value)
    }

    /// 加载所有配置文件
    pub async fn load_all_config_files(&self) -> Result<Vec<ConfigItem>> {
        let mut all_items = Vec::new();

        if !self.config_dir.exists() {
            return Ok(all_items);
        }

        let mut entries = fs::read_dir(&self.config_dir).await
            .context(format!("读取配置目录失败: {:?}", self.config_dir))?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                if let Some(format) = ConfigFileFormat::from_path(&path) {
                    if format != ConfigFileFormat::Auto {
                        match self.load_config_file_from_path(&path).await {
                            Ok(items) => all_items.extend(items),
                            Err(e) => log::warn!("加载配置文件失败 {:?}: {}", path, e),
                        }
                    }
                }
            }
        }

        Ok(all_items)
    }

    /// 保存配置到文件
    pub async fn save_config_file(
        &self,
        filename: &str,
        items: &[ConfigItem],
        format: ConfigFileFormat,
    ) -> Result<()> {
        let path = self.config_dir.join(filename);
        self.save_config_file_to_path(&path, items, format).await
    }

    /// 保存配置到指定路径
    pub async fn save_config_file_to_path(
        &self,
        path: &Path,
        items: &[ConfigItem],
        format: ConfigFileFormat,
    ) -> Result<()> {
        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await
                .context(format!("创建目录失败: {:?}", parent))?;
        }

        let export_data = self.prepare_export_data(items)?;
        let content = self.serialize_content(&export_data, format)
            .context("序列化配置数据失败")?;

        fs::write(path, content).await
            .context(format!("写入配置文件失败: {:?}", path))?;

        Ok(())
    }

    /// 监视配置文件变化
    pub async fn watch_config_file(
        &mut self,
        filename: &str,
        callback: impl Fn(&Path) + Send + Sync + 'static,
    ) -> Result<()> {
        let path = self.config_dir.join(filename);
        self.watch_config_file_path(&path, callback).await
    }

    /// 监视指定路径的配置文件变化
    pub async fn watch_config_file_path(
        &mut self,
        path: &Path,
        callback: impl Fn(&Path) + Send + Sync + 'static,
    ) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }

        // 初始化文件监视器（如果需要）
        if self.watcher.is_none() {
            self.init_watcher().await?;
        }

        let metadata = fs::metadata(path).await?;
        let last_modified = metadata.modified()?;

        let mut file_watches = self.file_watches.write().await;
        file_watches.insert(
            path.to_path_buf(),
            FileWatchInfo {
                last_modified,
                callback: Box::new(callback),
            },
        );

        // 添加监视
        if let Some(watcher) = &mut self.watcher {
            watcher.watch(path, RecursiveMode::NonRecursive)?;
        }

        Ok(())
    }

    /// 停止监视配置文件
    pub async fn unwatch_config_file(&mut self, filename: &str) -> Result<()> {
        let path = self.config_dir.join(filename);
        self.unwatch_config_file_path(&path).await
    }

    /// 停止监视指定路径的配置文件
    pub async fn unwatch_config_file_path(&mut self, path: &Path) -> Result<()> {
        let mut file_watches = self.file_watches.write().await;
        file_watches.remove(path);

        if let Some(watcher) = &mut self.watcher {
            watcher.unwatch(path)?;
        }

        Ok(())
    }

    /// 加载环境变量配置
    pub fn load_env_vars(&self, prefix: &str) -> Result<Vec<ConfigItem>> {
        let mut items = Vec::new();

        for (key, value) in std::env::vars() {
            if key.starts_with(prefix) {
                let config_key = key.trim_start_matches(prefix).trim_start_matches('_');
                let config_key = config_key.to_lowercase().replace('_', ".");

                // 尝试解析值
                let config_value = if let Ok(int_val) = value.parse::<i64>() {
                    Value::Number(int_val.into())
                } else if let Ok(float_val) = value.parse::<f64>() {
                    Value::from(float_val)
                } else if let Ok(bool_val) = value.parse::<bool>() {
                    Value::Bool(bool_val)
                } else {
                    Value::String(value)
                };

                // 创建配置元数据（简化版本）
                let metadata = ConfigMetadata {
                    key: config_key.clone(),
                    category: ConfigCategory::System,
                    display_name: config_key.clone(),
                    description: format!("从环境变量加载: {}", key),
                    data_type: ConfigDataType::String,
                    default_value: config_value.clone(),
                    validation_rules: Vec::new(),
                    is_required: false,
                    is_sensitive: key.to_lowercase().contains("password") || key.to_lowercase().contains("secret"),
                    is_readonly: true, // 环境变量配置通常是只读的
                    version: "1.0.0".to_string(),
                    sort_order: 0,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };

                let mut item = ConfigItem::new(metadata);
                item.update_value(config_value, "environment");
                items.push(item);
            }
        }

        Ok(items)
    }

    /// 初始化文件监视器
    async fn init_watcher(&mut self) -> Result<()> {
        let file_watches = self.file_watches.clone();

        let watcher = RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    if event.kind.is_modify() || event.kind.is_create() {
                        for path in event.paths {
                            let file_watches = file_watches.clone();
                            tokio::spawn(async move {
                                let watches = file_watches.read().await;
                                if let Some(info) = watches.get(&path) {
                                    // 检查文件是否真的被修改
                                    if let Ok(metadata) = std::fs::metadata(&path) {
                                        if let Ok(modified) = metadata.modified() {
                                            if modified > info.last_modified {
                                                (info.callback)(&path);
                                            }
                                        }
                                    }
                                }
                            });
                        }
                    }
                }
            },
            Config::default(),
        )?;

        self.watcher = Some(watcher);
        Ok(())
    }

    /// 检测内容格式
    fn detect_format(content: &str, path: &Path) -> ConfigFileFormat {
        // 首先尝试从扩展名检测
        if let Some(format) = ConfigFileFormat::from_path(path) {
            return format;
        }

        // 尝试根据内容检测
        if content.trim_start().starts_with('{') {
            ConfigFileFormat::Json
        } else if content.trim_start().starts_with("---") || content.contains(": ") {
            ConfigFileFormat::Yaml
        } else if content.contains('=') {
            ConfigFileFormat::Toml
        } else {
            ConfigFileFormat::Json // 默认
        }
    }

    /// 解析内容
    fn parse_content(&self, content: &str, format: ConfigFileFormat) -> Result<Value> {
        match format {
            ConfigFileFormat::Json => {
                serde_json::from_str(content).context("解析JSON失败")
            }
            ConfigFileFormat::Yaml => {
                serde_yaml::from_str(content).context("解析YAML失败")
            }
            ConfigFileFormat::Toml => {
                toml::from_str(content).context("解析TOML失败")
            }
            ConfigFileFormat::Auto => {
                // 尝试所有格式
                if let Ok(value) = serde_json::from_str(content) {
                    return Ok(value);
                }
                if let Ok(value) = serde_yaml::from_str(content) {
                    return Ok(value);
                }
                if let Ok(value) = toml::from_str(content) {
                    return Ok(value);
                }
                Err(anyhow::anyhow!("无法自动检测配置文件格式"))
            }
        }
    }

    /// 序列化内容
    fn serialize_content(&self, value: &Value, format: ConfigFileFormat) -> Result<String> {
        match format {
            ConfigFileFormat::Json => {
                serde_json::to_string_pretty(value).context("序列化JSON失败")
            }
            ConfigFileFormat::Yaml => {
                serde_yaml::to_string(value).context("序列化YAML失败")
            }
            ConfigFileFormat::Toml => {
                toml::to_string_pretty(value).context("序列化TOML失败")
            }
            ConfigFileFormat::Auto => {
                // 默认使用JSON
                serde_json::to_string_pretty(value).context("序列化JSON失败")
            }
        }
    }

    /// 准备导出数据
    fn prepare_export_data(&self, items: &[ConfigItem]) -> Result<Value> {
        let configs: Vec<Value> = items
            .iter()
            .map(|item| {
                let category_str = match item.metadata.category {
                    ConfigCategory::System => "system",
                    ConfigCategory::Compression => "compression",
                    ConfigCategory::Security => "security",
                    ConfigCategory::Ui => "ui",
                    ConfigCategory::Network => "network",
                    ConfigCategory::Storage => "storage",
                    ConfigCategory::Advanced => "advanced",
                    ConfigCategory::Other => "other",
                };

                json!({
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

        Ok(json!({
            "export_version": "1.0.0",
            "export_date": Utc::now().to_rfc3339(),
            "config_count": configs.len(),
            "configs": configs,
        }))
    }

    /// 将解析的值转换为配置项
    fn convert_to_config_items(&self, value: &Value) -> Result<Vec<ConfigItem>> {
        let configs = value.get("configs")
            .and_then(|c| c.as_array())
            .context("配置文件格式无效，缺少configs数组")?;

        let mut items = Vec::new();

        for config in configs {
            let key = config.get("key")
                .and_then(|k| k.as_str())
                .context("配置项缺少key字段")?;

            let category_str = config.get("category")
                .and_then(|c| c.as_str())
                .unwrap_or("system");
            let category = match category_str {
                "system" => ConfigCategory::System,
                "compression" => ConfigCategory::Compression,
                "security" => ConfigCategory::Security,
                "ui" => ConfigCategory::Ui,
                "network" => ConfigCategory::Network,
                "storage" => ConfigCategory::Storage,
                "advanced" => ConfigCategory::Advanced,
                "other" => ConfigCategory::Other,
                _ => ConfigCategory::System,
            };

            let display_name = config.get("display_name")
                .and_then(|n| n.as_str())
                .unwrap_or(key)
                .to_string();

            let description = config.get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();

            let data_type_str = config.get("data_type")
                .and_then(|t| t.as_str())
                .unwrap_or("string");
            let data_type = ConfigDataType::from_str(data_type_str)
                .unwrap_or(ConfigDataType::String);

            let default_value = config.get("default_value")
                .cloned()
                .unwrap_or(Value::Null);

            let value = config.get("value")
                .cloned()
                .unwrap_or(default_value.clone());

            let validation_rules = config.get("validation_rules")
                .and_then(|r| serde_json::from_value(r.clone()).ok())
                .unwrap_or_else(Vec::new);

            let is_required = config.get("is_required")
                .and_then(|r| r.as_bool())
                .unwrap_or(false);

            let is_sensitive = config.get("is_sensitive")
                .and_then(|s| s.as_bool())
                .unwrap_or(false);

            let is_readonly = config.get("is_readonly")
                .and_then(|r| r.as_bool())
                .unwrap_or(false);

            let version = config.get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("1.0.0")
                .to_string();

            let metadata = ConfigMetadata {
                key: key.to_string(),
                category,
                display_name,
                description,
                data_type,
                default_value,
                validation_rules,
                is_required,
                is_sensitive,
                is_readonly,
                version,
                sort_order: 0,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            let mut item = ConfigItem::new(metadata);
            item.update_value(value, "file_loader");
            items.push(item);
        }

        Ok(items)
    }
}

/// 默认配置生成器（扩展）
pub struct DefaultConfigFileGenerator;

impl DefaultConfigFileGenerator {
    /// 生成默认配置文件内容
    pub fn generate_default_config(format: ConfigFileFormat) -> Result<String> {
        let metadata = Self::generate_default_metadata();
        let items: Vec<ConfigItem> = metadata.into_iter().map(ConfigItem::new).collect();

        let loader = ConfigFileLoader::new(".", format);
        let export_data = loader.prepare_export_data(&items)?;
        loader.serialize_content(&export_data, format)
    }

    /// 生成默认配置元数据
    fn generate_default_metadata() -> Vec<ConfigMetadata> {
        use crate::config::models::DefaultConfigGenerator;
        DefaultConfigGenerator::generate_all_metadata()
    }

    /// 保存默认配置文件
    pub async fn save_default_config_file(
        path: &Path,
        format: ConfigFileFormat,
    ) -> Result<()> {
        let content = Self::generate_default_config(format)?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(path, content).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_load_config_file_json() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");

        let config_content = r#"{
            "export_version": "1.0.0",
            "export_date": "2024-01-01T00:00:00Z",
            "config_count": 1,
            "configs": [
                {
                    "key": "test.config",
                    "category": "system",
                    "display_name": "测试配置",
                    "description": "测试配置项",
                    "data_type": "string",
                    "value": "test value",
                    "default_value": "default",
                    "validation_rules": [],
                    "is_required": false,
                    "is_sensitive": false,
                    "is_readonly": false,
                    "version": "1.0.0",
                    "last_modified": "2024-01-01T00:00:00Z",
                    "last_modified_by": "test"
                }
            ]
        }"#;

        std::fs::write(&config_path, config_content).unwrap();

        let loader = ConfigFileLoader::new(dir.path(), ConfigFileFormat::Json);
        let items = loader.load_config_file_from_path(&config_path).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].metadata.key, "test.config");
        assert_eq!(items[0].current_value, Value::String("test value".to_string()));
    }

    #[tokio::test]
    async fn test_save_config_file_json() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");

        let metadata = ConfigMetadata {
            key: "test.save".to_string(),
            category: ConfigCategory::System,
            display_name: "保存测试".to_string(),
            description: "保存测试配置".to_string(),
            data_type: ConfigDataType::String,
            default_value: Value::String("default".to_string()),
            validation_rules: Vec::new(),
            is_required: false,
            is_sensitive: false,
            is_readonly: false,
            version: "1.0.0".to_string(),
            sort_order: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut item = ConfigItem::new(metadata);
        item.update_value(Value::String("saved value".to_string()), "test");

        let loader = ConfigFileLoader::new(dir.path(), ConfigFileFormat::Json);
        loader.save_config_file_to_path(&config_path, &[item], ConfigFileFormat::Json)
            .await
            .unwrap();

        assert!(config_path.exists());

        // 验证文件内容
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("test.save"));
        assert!(content.contains("saved value"));
    }

    #[test]
    fn test_detect_format() {
        assert_eq!(
            ConfigFileFormat::from_extension("json"),
            Some(ConfigFileFormat::Json)
        );
        assert_eq!(
            ConfigFileFormat::from_extension("yaml"),
            Some(ConfigFileFormat::Yaml)
        );
        assert_eq!(
            ConfigFileFormat::from_extension("toml"),
            Some(ConfigFileFormat::Toml)
        );
        assert_eq!(ConfigFileFormat::from_extension("txt"), None);
    }

    #[tokio::test]
    async fn test_load_env_vars() {
        // 设置测试环境变量
        std::env::set_var("APP_TEST_CONFIG", "test_value");
        std::env::set_var("APP_DATABASE_PORT", "5432");
        std::env::set_var("APP_DEBUG_MODE", "true");

        let loader = ConfigFileLoader::new(".", ConfigFileFormat::Json);
        let items = loader.load_env_vars("APP_").unwrap();

        // 清理环境变量
        std::env::remove_var("APP_TEST_CONFIG");
        std::env::remove_var("APP_DATABASE_PORT");
        std::env::remove_var("APP_DEBUG_MODE");

        assert!(!items.is_empty());

        let has_test_config = items.iter().any(|item| item.metadata.key == "test.config");
        let has_database_port = items.iter().any(|item| item.metadata.key == "database.port");
        let has_debug_mode = items.iter().any(|item| item.metadata.key == "debug.mode");

        assert!(has_test_config || has_database_port || has_debug_mode);
    }

    #[tokio::test]
    async fn test_generate_default_config() {
        let result = DefaultConfigFileGenerator::generate_default_config(ConfigFileFormat::Json);
        assert!(result.is_ok());

        let content = result.unwrap();
        assert!(content.contains("export_version"));
        assert!(content.contains("configs"));

        // 验证JSON可以解析
        let parsed: Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.get("configs").is_some());
    }
}