# 配置管理系统设计文档 (BE-002)

## 概述
配置管理系统负责管理胧压缩·方便助手的所有配置项，包括系统设置、用户偏好、压缩参数、安全设置等。

## 设计目标
1. 集中管理所有配置项
2. 支持配置持久化存储
3. 提供配置验证机制
4. 支持配置导入导出
5. 提供配置变更通知
6. 支持多环境配置

## 架构设计

### 1. 配置模型层
基于现有的`SystemConfig`结构体扩展，定义完整的配置模型：

```rust
// 配置分类
pub enum ConfigCategory {
    System,      // 系统配置
    Compression, // 压缩配置
    Security,    // 安全配置
    UI,          // 界面配置
    Network,     // 网络配置
    Storage,     // 存储配置
    Advanced,    // 高级配置
}

// 配置项元数据
pub struct ConfigMetadata {
    pub key: String,
    pub category: ConfigCategory,
    pub display_name: String,
    pub description: String,
    pub data_type: ConfigDataType,
    pub default_value: serde_json::Value,
    pub validation_rules: Vec<ValidationRule>,
    pub is_required: bool,
    pub is_sensitive: bool,
    pub version: String,
    pub last_modified: chrono::DateTime<chrono::Utc>,
}

// 配置数据类型
pub enum ConfigDataType {
    String,
    Integer,
    Float,
    Boolean,
    Array,
    Object,
    Enum(Vec<String>),
}

// 验证规则
pub enum ValidationRule {
    MinLength(usize),
    MaxLength(usize),
    MinValue(f64),
    MaxValue(f64),
    Regex(String),
    Range(f64, f64),
    Custom(String), // 自定义验证函数名
}
```

### 2. 配置存储层
使用SQLite数据库存储配置，设计表结构：

```sql
-- 配置分类表
CREATE TABLE config_categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    description TEXT,
    icon TEXT,
    sort_order INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 配置项表
CREATE TABLE config_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key TEXT NOT NULL UNIQUE,
    category_id INTEGER NOT NULL,
    display_name TEXT NOT NULL,
    description TEXT,
    data_type TEXT NOT NULL, -- 'string', 'integer', 'float', 'boolean', 'array', 'object', 'enum'
    default_value TEXT,
    current_value TEXT,
    validation_rules TEXT, -- JSON数组
    is_required BOOLEAN DEFAULT 0,
    is_sensitive BOOLEAN DEFAULT 0,
    is_readonly BOOLEAN DEFAULT 0,
    version TEXT DEFAULT '1.0.0',
    sort_order INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (category_id) REFERENCES config_categories(id)
);

-- 配置历史表（审计日志）
CREATE TABLE config_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    config_key TEXT NOT NULL,
    old_value TEXT,
    new_value TEXT,
    changed_by TEXT, -- 用户或系统
    change_reason TEXT,
    ip_address TEXT,
    user_agent TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 配置导入导出记录
CREATE TABLE config_import_export (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_type TEXT NOT NULL, -- 'import', 'export'
    file_name TEXT,
    file_size INTEGER,
    config_count INTEGER,
    status TEXT NOT NULL, -- 'pending', 'success', 'failed'
    error_message TEXT,
    created_by TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP
);
```

### 3. 配置服务层
提供配置的CRUD操作和业务逻辑：

```rust
pub struct ConfigService {
    db_pool: SqlitePool,
    cache: ConfigCache,
    validators: HashMap<String, Box<dyn ConfigValidator>>,
    listeners: Vec<Box<dyn ConfigChangeListener>>,
}

impl ConfigService {
    // 核心方法
    pub async fn get_config(&self, key: &str) -> Result<ConfigItem>;
    pub async fn set_config(&self, key: &str, value: serde_json::Value) -> Result<()>;
    pub async fn get_all_configs(&self) -> Result<Vec<ConfigItem>>;
    pub async fn get_configs_by_category(&self, category: ConfigCategory) -> Result<Vec<ConfigItem>>;

    // 批量操作
    pub async fn batch_update(&self, updates: HashMap<String, serde_json::Value>) -> Result<()>;
    pub async fn reset_to_default(&self, keys: Vec<String>) -> Result<()>;

    // 验证
    pub async fn validate_config(&self, key: &str, value: &serde_json::Value) -> Result<ValidationResult>;

    // 导入导出
    pub async fn export_configs(&self, format: ExportFormat, category: Option<ConfigCategory>) -> Result<Vec<u8>>;
    pub async fn import_configs(&self, data: &[u8], format: ImportFormat, strategy: ImportStrategy) -> Result<ImportResult>;

    // 监听器
    pub fn add_listener(&mut self, listener: Box<dyn ConfigChangeListener>);
    pub fn remove_listener(&mut self, listener_id: &str);

    // 缓存管理
    pub async fn refresh_cache(&self);
    pub async fn clear_cache(&self);
}
```

### 4. 配置验证层
提供灵活的验证机制：

```rust
pub trait ConfigValidator: Send + Sync {
    fn validate(&self, key: &str, value: &serde_json::Value, metadata: &ConfigMetadata) -> ValidationResult;
}

pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

pub struct ValidationError {
    pub code: String,
    pub message: String,
    pub field: String,
}

// 内置验证器
pub struct StringLengthValidator { min: usize, max: usize }
pub struct NumberRangeValidator { min: f64, max: f64 }
pub struct RegexValidator { pattern: String }
pub struct EnumValidator { allowed_values: Vec<String> }
pub struct CustomValidator { function_name: String }
```

### 5. 配置监听层
支持配置变更通知：

```rust
pub trait ConfigChangeListener: Send + Sync {
    fn on_config_changed(&self, event: ConfigChangeEvent);
    fn get_id(&self) -> &str;
}

pub struct ConfigChangeEvent {
    pub key: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub changed_by: String,
    pub source: ChangeSource,
}

pub enum ChangeSource {
    UserInterface,
    CommandLine,
    Import,
    System,
    Api,
}
```

### 6. 配置导入导出层
支持多种格式：

```rust
pub enum ExportFormat {
    Json,
    Yaml,
    Toml,
    Ini,
    Xml,
}

pub enum ImportFormat {
    Json,
    Yaml,
    Toml,
    Ini,
    Xml,
}

pub enum ImportStrategy {
    Merge,      // 合并现有配置
    Replace,    // 替换现有配置
    UpdateOnly, // 只更新存在的配置
    SkipExisting, // 跳过已存在的配置
}

pub struct ImportResult {
    pub total_items: usize,
    pub imported_items: usize,
    pub skipped_items: usize,
    pub failed_items: usize,
    pub errors: Vec<ImportError>,
    pub warnings: Vec<ImportWarning>,
}
```

## 核心配置项设计

### 系统配置 (System)
- `system.monitoring_enabled`: 系统监控开关
- `system.update_interval`: 监控更新间隔
- `system.retention_days`: 数据保留天数
- `system.language`: 界面语言
- `system.theme`: 主题设置

### 压缩配置 (Compression)
- `compression.default_format`: 默认压缩格式
- `compression.compression_level`: 压缩级别
- `compression.split_size`: 分卷大小
- `compression.encryption_enabled`: 加密开关
- `compression.password_protection`: 密码保护

### 安全配置 (Security)
- `security.master_password_enabled`: 主密码开关
- `security.auto_lock_timeout`: 自动锁定超时
- `security.password_strength_requirement`: 密码强度要求
- `security.encryption_algorithm`: 加密算法
- `security.key_derivation_iterations`: 密钥派生迭代次数

### 界面配置 (UI)
- `ui.default_view`: 默认视图
- `ui.show_hidden_files`: 显示隐藏文件
- `ui.confirm_before_delete`: 删除前确认
- `ui.animation_enabled`: 动画效果
- `ui.tooltip_enabled`: 工具提示

### 网络配置 (Network)
- `network.proxy_enabled`: 代理开关
- `network.proxy_url`: 代理地址
- `network.timeout`: 网络超时
- `network.retry_count`: 重试次数
- `network.concurrent_downloads`: 并发下载数

### 存储配置 (Storage)
- `storage.default_save_path`: 默认保存路径
- `storage.temp_directory`: 临时目录
- `storage.max_cache_size`: 最大缓存大小
- `storage.auto_cleanup`: 自动清理
- `storage.backup_enabled`: 备份开关

### 高级配置 (Advanced)
- `advanced.debug_mode`: 调试模式
- `advanced.log_level`: 日志级别
- `advanced.performance_mode`: 性能模式
- `advanced.experimental_features`: 实验功能
- `advanced.custom_scripts`: 自定义脚本

## 数据库迁移设计

需要创建以下迁移脚本：

1. 初始迁移：创建配置相关表
2. 默认配置插入：插入所有预定义的配置项
3. 索引优化：创建必要的索引
4. 数据迁移：版本升级时的数据迁移

## 安全性考虑

1. 敏感配置加密存储
2. 配置访问权限控制
3. 配置变更审计日志
4. 输入验证和消毒
5. SQL注入防护

## 性能优化

1. 配置缓存机制
2. 批量操作支持
3. 懒加载配置
4. 增量更新
5. 索引优化

## 测试策略

1. 单元测试：配置验证、服务方法
2. 集成测试：数据库操作、导入导出
3. 性能测试：大量配置项的读写性能
4. 安全测试：敏感配置保护、输入验证

## 依赖关系

- 依赖BE-001数据库模块
- 使用现有的SystemConfig结构体
- 与前端配置界面交互
- 与其他服务模块共享配置

## 实施计划

### 阶段1：基础框架 (2天)
- 创建配置模型定义
- 设计数据库表结构
- 实现基础Repository

### 阶段2：核心服务 (3天)
- 实现ConfigService
- 添加配置验证机制
- 实现配置缓存

### 阶段3：高级功能 (2天)
- 实现导入导出
- 添加配置监听
- 实现配置审计

### 阶段4：集成测试 (1天)
- 单元测试
- 集成测试
- 性能测试

## 风险评估

1. 数据库设计变更风险
2. 配置兼容性问题
3. 性能瓶颈风险
4. 安全漏洞风险

## 成功标准

1. 所有配置项可持久化存储
2. 配置变更可审计追踪
3. 支持配置导入导出
4. 配置验证机制完善
5. 性能满足要求
6. 安全性达标