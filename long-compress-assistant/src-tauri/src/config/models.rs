use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// 配置分类
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ConfigCategory {
    /// 系统配置
    System,
    /// 压缩配置
    Compression,
    /// 安全配置
    Security,
    /// 界面配置
    Ui,
    /// 网络配置
    Network,
    /// 存储配置
    Storage,
    /// 高级配置
    Advanced,
    /// 其他配置
    Other,
}

impl ConfigCategory {
    /// 获取分类的显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            ConfigCategory::System => "系统配置",
            ConfigCategory::Compression => "压缩配置",
            ConfigCategory::Security => "安全配置",
            ConfigCategory::Ui => "界面配置",
            ConfigCategory::Network => "网络配置",
            ConfigCategory::Storage => "存储配置",
            ConfigCategory::Advanced => "高级配置",
            ConfigCategory::Other => "其他配置",
        }
    }

    /// 获取分类的描述
    pub fn description(&self) -> &'static str {
        match self {
            ConfigCategory::System => "系统相关配置，如监控、更新等",
            ConfigCategory::Compression => "压缩解压相关配置",
            ConfigCategory::Security => "安全相关配置，如密码、加密等",
            ConfigCategory::Ui => "用户界面相关配置",
            ConfigCategory::Network => "网络相关配置",
            ConfigCategory::Storage => "存储相关配置",
            ConfigCategory::Advanced => "高级功能和实验性配置",
            ConfigCategory::Other => "其他未分类配置",
        }
    }

    /// 获取所有分类
    pub fn all_categories() -> Vec<Self> {
        vec![
            ConfigCategory::System,
            ConfigCategory::Compression,
            ConfigCategory::Security,
            ConfigCategory::Ui,
            ConfigCategory::Network,
            ConfigCategory::Storage,
            ConfigCategory::Advanced,
            ConfigCategory::Other,
        ]
    }
}

/// 配置数据类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConfigDataType {
    /// 字符串类型
    String,
    /// 整数类型
    Integer,
    /// 浮点数类型
    Float,
    /// 布尔类型
    Boolean,
    /// 数组类型
    Array,
    /// 对象类型
    Object,
    /// 枚举类型
    Enum,
}

impl ConfigDataType {
    /// 从字符串解析数据类型
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "string" => Some(ConfigDataType::String),
            "integer" => Some(ConfigDataType::Integer),
            "float" => Some(ConfigDataType::Float),
            "boolean" => Some(ConfigDataType::Boolean),
            "array" => Some(ConfigDataType::Array),
            "object" => Some(ConfigDataType::Object),
            "enum" => Some(ConfigDataType::Enum),
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            ConfigDataType::String => "string",
            ConfigDataType::Integer => "integer",
            ConfigDataType::Float => "float",
            ConfigDataType::Boolean => "boolean",
            ConfigDataType::Array => "array",
            ConfigDataType::Object => "object",
            ConfigDataType::Enum => "enum",
        }
    }

    /// 验证值是否符合数据类型
    pub fn validate_value(&self, value: &Value) -> bool {
        match self {
            ConfigDataType::String => value.is_string(),
            ConfigDataType::Integer => value.is_number() && value.as_i64().is_some(),
            ConfigDataType::Float => value.is_number(),
            ConfigDataType::Boolean => value.is_boolean(),
            ConfigDataType::Array => value.is_array(),
            ConfigDataType::Object => value.is_object(),
            ConfigDataType::Enum => value.is_string(),
        }
    }
}

/// 验证规则类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ValidationRule {
    /// 最小长度（字符串）
    MinLength { value: usize },
    /// 最大长度（字符串）
    MaxLength { value: usize },
    /// 最小值（数字）
    MinValue { value: f64 },
    /// 最大值（数字）
    MaxValue { value: f64 },
    /// 正则表达式
    Regex { pattern: String },
    /// 范围（最小值，最大值）
    Range { min: f64, max: f64 },
    /// 枚举值
    Enum { values: Vec<String> },
    /// 自定义验证
    Custom { function: String },
}

/// 配置项元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMetadata {
    /// 配置键
    pub key: String,
    /// 配置分类
    pub category: ConfigCategory,
    /// 显示名称
    pub display_name: String,
    /// 描述
    pub description: String,
    /// 数据类型
    pub data_type: ConfigDataType,
    /// 默认值（JSON格式）
    pub default_value: Value,
    /// 验证规则
    pub validation_rules: Vec<ValidationRule>,
    /// 是否必填
    pub is_required: bool,
    /// 是否敏感（需要加密存储）
    pub is_sensitive: bool,
    /// 是否只读
    pub is_readonly: bool,
    /// 版本号
    pub version: String,
    /// 排序顺序
    pub sort_order: i32,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 配置项（包含当前值）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigItem {
    /// 元数据
    pub metadata: ConfigMetadata,
    /// 当前值（JSON格式）
    pub current_value: Value,
    /// 最后修改时间
    pub last_modified: DateTime<Utc>,
    /// 最后修改者
    pub last_modified_by: String,
}

impl ConfigItem {
    /// 创建新的配置项
    pub fn new(metadata: ConfigMetadata) -> Self {
        Self {
            metadata: metadata.clone(),
            current_value: metadata.default_value.clone(),
            last_modified: Utc::now(),
            last_modified_by: "system".to_string(),
        }
    }

    /// 更新配置值
    pub fn update_value(&mut self, value: Value, modified_by: &str) {
        self.current_value = value;
        self.last_modified = Utc::now();
        self.last_modified_by = modified_by.to_string();
    }

    /// 重置为默认值
    pub fn reset_to_default(&mut self, modified_by: &str) {
        self.current_value = self.metadata.default_value.clone();
        self.last_modified = Utc::now();
        self.last_modified_by = modified_by.to_string();
    }

    /// 获取字符串值
    pub fn as_string(&self) -> Option<String> {
        self.current_value.as_str().map(|s| s.to_string())
    }

    /// 获取整数值
    pub fn as_integer(&self) -> Option<i64> {
        self.current_value.as_i64()
    }

    /// 获取浮点数值
    pub fn as_float(&self) -> Option<f64> {
        self.current_value.as_f64()
    }

    /// 获取布尔值
    pub fn as_boolean(&self) -> Option<bool> {
        self.current_value.as_bool()
    }

    /// 获取数组值
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        self.current_value.as_array()
    }

    /// 获取对象值
    pub fn as_object(&self) -> Option<HashMap<String, Value>> {
        self.current_value.as_object().map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
    }
}

/// 配置变更事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangeEvent {
    /// 事件ID
    pub id: String,
    /// 配置键
    pub key: String,
    /// 旧值
    pub old_value: Option<Value>,
    /// 新值
    pub new_value: Value,
    /// 变更时间
    pub timestamp: DateTime<Utc>,
    /// 变更者
    pub changed_by: String,
    /// 变更来源
    pub source: ChangeSource,
    /// 变更原因
    pub reason: Option<String>,
}

impl ConfigChangeEvent {
    /// 创建新的配置变更事件
    pub fn new(
        key: String,
        old_value: Option<Value>,
        new_value: Value,
        changed_by: String,
        source: ChangeSource,
        reason: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            key,
            old_value,
            new_value,
            timestamp: Utc::now(),
            changed_by,
            source,
            reason,
        }
    }
}

/// 变更来源
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
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

/// 验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// 是否有效
    pub is_valid: bool,
    /// 错误列表
    pub errors: Vec<ValidationError>,
    /// 警告列表
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationResult {
    /// 创建有效的验证结果
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// 创建包含错误的验证结果
    pub fn invalid(errors: Vec<ValidationError>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    /// 添加错误
    pub fn add_error(&mut self, error: ValidationError) {
        self.is_valid = false;
        self.errors.push(error);
    }

    /// 添加警告
    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }
}

/// 验证错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// 错误代码
    pub code: String,
    /// 错误消息
    pub message: String,
    /// 字段名
    pub field: String,
    /// 错误详情
    pub details: Option<Value>,
}

impl ValidationError {
    /// 创建新的验证错误
    pub fn new(code: &str, message: &str, field: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            field: field.to_string(),
            details: None,
        }
    }

    /// 创建带有详情的验证错误
    pub fn with_details(code: &str, message: &str, field: &str, details: Value) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            field: field.to_string(),
            details: Some(details),
        }
    }
}

/// 验证警告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// 警告代码
    pub code: String,
    /// 警告消息
    pub message: String,
    /// 字段名
    pub field: String,
    /// 警告详情
    pub details: Option<Value>,
}

impl ValidationWarning {
    /// 创建新的验证警告
    pub fn new(code: &str, message: &str, field: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            field: field.to_string(),
            details: None,
        }
    }
}

/// 导入导出格式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    /// JSON格式
    Json,
    /// YAML格式
    Yaml,
    /// TOML格式
    Toml,
}

/// 导入策略
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportStrategy {
    /// 合并现有配置
    Merge,
    /// 替换现有配置
    Replace,
    /// 只更新存在的配置
    UpdateOnly,
    /// 跳过已存在的配置
    SkipExisting,
}

/// 导入结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    /// 总项目数
    pub total_items: usize,
    /// 导入项目数
    pub imported_items: usize,
    /// 跳过项目数
    pub skipped_items: usize,
    /// 失败项目数
    pub failed_items: usize,
    /// 错误列表
    pub errors: Vec<ImportError>,
    /// 警告列表
    pub warnings: Vec<ImportWarning>,
}

/// 导入错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportError {
    /// 错误代码
    pub code: String,
    /// 错误消息
    pub message: String,
    /// 配置键
    pub key: Option<String>,
    /// 错误详情
    pub details: Option<Value>,
}

/// 导入警告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportWarning {
    /// 警告代码
    pub code: String,
    /// 警告消息
    pub message: String,
    /// 配置键
    pub key: Option<String>,
    /// 警告详情
    pub details: Option<Value>,
}

/// 默认配置生成器
pub struct DefaultConfigGenerator;

impl DefaultConfigGenerator {
    /// 生成所有默认配置的元数据
    pub fn generate_all_metadata() -> Vec<ConfigMetadata> {
        let mut metadata = Vec::new();

        // 系统配置
        metadata.extend(Self::generate_system_configs());
        // 压缩配置
        metadata.extend(Self::generate_compression_configs());
        // 安全配置
        metadata.extend(Self::generate_security_configs());
        // 界面配置
        metadata.extend(Self::generate_ui_configs());
        // 网络配置
        metadata.extend(Self::generate_network_configs());
        // 存储配置
        metadata.extend(Self::generate_storage_configs());
        // 高级配置
        metadata.extend(Self::generate_advanced_configs());

        metadata
    }

    /// 生成系统配置
    fn generate_system_configs() -> Vec<ConfigMetadata> {
        vec![
            ConfigMetadata {
                key: "system.monitoring_enabled".to_string(),
                category: ConfigCategory::System,
                display_name: "系统监控开关".to_string(),
                description: "启用或禁用系统监控功能".to_string(),
                data_type: ConfigDataType::Boolean,
                default_value: Value::Bool(true),
                validation_rules: vec![],
                is_required: false,
                is_sensitive: false,
                is_readonly: false,
                version: "1.0.0".to_string(),
                sort_order: 100,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            ConfigMetadata {
                key: "system.update_interval".to_string(),
                category: ConfigCategory::System,
                display_name: "监控更新间隔".to_string(),
                description: "系统监控数据更新的时间间隔（毫秒）".to_string(),
                data_type: ConfigDataType::Integer,
                default_value: Value::Number(5000.into()),
                validation_rules: vec![
                    ValidationRule::MinValue { value: 1000.0 },
                    ValidationRule::MaxValue { value: 60000.0 },
                ],
                is_required: false,
                is_sensitive: false,
                is_readonly: false,
                version: "1.0.0".to_string(),
                sort_order: 110,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            ConfigMetadata {
                key: "system.language".to_string(),
                category: ConfigCategory::System,
                display_name: "界面语言".to_string(),
                description: "应用程序界面显示语言".to_string(),
                data_type: ConfigDataType::String,
                default_value: Value::String("zh-CN".to_string()),
                validation_rules: vec![
                    ValidationRule::Enum {
                        values: vec![
                            "zh-CN".to_string(),
                            "en-US".to_string(),
                            "ja-JP".to_string(),
                        ],
                    },
                ],
                is_required: false,
                is_sensitive: false,
                is_readonly: false,
                version: "1.0.0".to_string(),
                sort_order: 120,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ]
    }

    /// 生成压缩配置
    fn generate_compression_configs() -> Vec<ConfigMetadata> {
        vec![
            ConfigMetadata {
                key: "compression.default_format".to_string(),
                category: ConfigCategory::Compression,
                display_name: "默认压缩格式".to_string(),
                description: "压缩文件时使用的默认格式".to_string(),
                data_type: ConfigDataType::String,
                default_value: Value::String("zip".to_string()),
                validation_rules: vec![
                    ValidationRule::Enum {
                        values: vec![
                            "zip".to_string(),
                            "tar".to_string(),
                            "gz".to_string(),
                            "bz2".to_string(),
                            "xz".to_string(),
                        ],
                    },
                ],
                is_required: false,
                is_sensitive: false,
                is_readonly: false,
                version: "1.0.0".to_string(),
                sort_order: 200,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            ConfigMetadata {
                key: "compression.compression_level".to_string(),
                category: ConfigCategory::Compression,
                display_name: "压缩级别".to_string(),
                description: "压缩级别（1-9，1最快，9最好）".to_string(),
                data_type: ConfigDataType::Integer,
                default_value: Value::Number(6.into()),
                validation_rules: vec![
                    ValidationRule::Range { min: 1.0, max: 9.0 },
                ],
                is_required: false,
                is_sensitive: false,
                is_readonly: false,
                version: "1.0.0".to_string(),
                sort_order: 210,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ]
    }

    /// 生成安全配置
    fn generate_security_configs() -> Vec<ConfigMetadata> {
        vec![
            ConfigMetadata {
                key: "security.master_password_enabled".to_string(),
                category: ConfigCategory::Security,
                display_name: "主密码开关".to_string(),
                description: "启用或禁用主密码保护".to_string(),
                data_type: ConfigDataType::Boolean,
                default_value: Value::Bool(false),
                validation_rules: vec![],
                is_required: false,
                is_sensitive: false,
                is_readonly: false,
                version: "1.0.0".to_string(),
                sort_order: 300,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            ConfigMetadata {
                key: "security.auto_lock_timeout".to_string(),
                category: ConfigCategory::Security,
                display_name: "自动锁定超时".to_string(),
                description: "应用程序自动锁定的时间（分钟，0表示禁用）".to_string(),
                data_type: ConfigDataType::Integer,
                default_value: Value::Number(10.into()),
                validation_rules: vec![
                    ValidationRule::MinValue { value: 0.0 },
                    ValidationRule::MaxValue { value: 120.0 },
                ],
                is_required: false,
                is_sensitive: false,
                is_readonly: false,
                version: "1.0.0".to_string(),
                sort_order: 310,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ]
    }

    /// 生成界面配置
    fn generate_ui_configs() -> Vec<ConfigMetadata> {
        vec![
            ConfigMetadata {
                key: "ui.theme".to_string(),
                category: ConfigCategory::Ui,
                display_name: "主题设置".to_string(),
                description: "应用程序界面主题".to_string(),
                data_type: ConfigDataType::String,
                default_value: Value::String("light".to_string()),
                validation_rules: vec![
                    ValidationRule::Enum {
                        values: vec![
                            "light".to_string(),
                            "dark".to_string(),
                            "auto".to_string(),
                        ],
                    },
                ],
                is_required: false,
                is_sensitive: false,
                is_readonly: false,
                version: "1.0.0".to_string(),
                sort_order: 400,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            ConfigMetadata {
                key: "ui.show_hidden_files".to_string(),
                category: ConfigCategory::Ui,
                display_name: "显示隐藏文件".to_string(),
                description: "在文件列表中显示隐藏文件".to_string(),
                data_type: ConfigDataType::Boolean,
                default_value: Value::Bool(false),
                validation_rules: vec![],
                is_required: false,
                is_sensitive: false,
                is_readonly: false,
                version: "1.0.0".to_string(),
                sort_order: 410,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ]
    }

    /// 生成网络配置
    fn generate_network_configs() -> Vec<ConfigMetadata> {
        vec![
            ConfigMetadata {
                key: "network.proxy_enabled".to_string(),
                category: ConfigCategory::Network,
                display_name: "代理开关".to_string(),
                description: "启用或禁用网络代理".to_string(),
                data_type: ConfigDataType::Boolean,
                default_value: Value::Bool(false),
                validation_rules: vec![],
                is_required: false,
                is_sensitive: false,
                is_readonly: false,
                version: "1.0.0".to_string(),
                sort_order: 500,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ]
    }

    /// 生成存储配置
    fn generate_storage_configs() -> Vec<ConfigMetadata> {
        vec![
            ConfigMetadata {
                key: "storage.default_save_path".to_string(),
                category: ConfigCategory::Storage,
                display_name: "默认保存路径".to_string(),
                description: "文件保存的默认路径".to_string(),
                data_type: ConfigDataType::String,
                default_value: Value::String("".to_string()),
                validation_rules: vec![],
                is_required: false,
                is_sensitive: false,
                is_readonly: false,
                version: "1.0.0".to_string(),
                sort_order: 600,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ]
    }

    /// 生成高级配置
    fn generate_advanced_configs() -> Vec<ConfigMetadata> {
        vec![
            ConfigMetadata {
                key: "advanced.debug_mode".to_string(),
                category: ConfigCategory::Advanced,
                display_name: "调试模式".to_string(),
                description: "启用或禁用调试模式".to_string(),
                data_type: ConfigDataType::Boolean,
                default_value: Value::Bool(false),
                validation_rules: vec![],
                is_required: false,
                is_sensitive: false,
                is_readonly: false,
                version: "1.0.0".to_string(),
                sort_order: 700,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            ConfigMetadata {
                key: "advanced.log_level".to_string(),
                category: ConfigCategory::Advanced,
                display_name: "日志级别".to_string(),
                description: "应用程序日志记录级别".to_string(),
                data_type: ConfigDataType::String,
                default_value: Value::String("info".to_string()),
                validation_rules: vec![
                    ValidationRule::Enum {
                        values: vec![
                            "trace".to_string(),
                            "debug".to_string(),
                            "info".to_string(),
                            "warn".to_string(),
                            "error".to_string(),
                        ],
                    },
                ],
                is_required: false,
                is_sensitive: false,
                is_readonly: false,
                version: "1.0.0".to_string(),
                sort_order: 710,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ]
    }
}