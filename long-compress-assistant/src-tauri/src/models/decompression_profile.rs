use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 解压配置组 - 可复用的解压设置模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompressionProfile {
    /// 配置组唯一标识
    pub id: String,

    /// 配置组名称（用户可见）
    pub name: String,

    /// 配置组图标（emoji）
    pub icon: String,

    /// 配置组描述
    pub description: String,

    /// 核心解压配置
    pub config: DecompressionConfig,

    /// 自动应用规则
    pub auto_apply: AutoApplyRule,

    /// 密码尝试策略
    pub password_attempt_strategy: PasswordAttemptStrategyConfig,

    /// 统计信息
    pub stats: ProfileStats,

    /// 创建时间（Unix 时间戳）
    pub created_at: i64,

    /// 最后使用时间
    pub last_used_at: Option<i64>,
}

/// 解压配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompressionConfig {
    /// 输出目录模式
    pub output_mode: OutputMode,

    /// 是否创建子目录
    pub create_subdirectory: bool,

    /// 子目录命名模板（支持 {name}, {date}, {time}）
    pub subdirectory_template: Option<String>,

    /// 是否保留路径结构
    pub preserve_paths: bool,

    /// 遇到同名文件时的处理
    pub overwrite_policy: OverwritePolicy,

    /// 是否保留时间戳
    pub preserve_timestamps: bool,

    /// 解压后是否删除源文件
    pub delete_after: bool,

    /// 是否跳过损坏的文件
    pub skip_corrupted: bool,

    /// 仅解压比现有文件新的文件
    pub extract_only_newer: bool,

    /// 文件名过滤器（正则表达式）
    pub file_filter: Option<String>,

    /// 额外参数
    pub extra_params: HashMap<String, String>,
}

/// 输出目录模式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    /// 解压到源文件所在目录
    SameAsSource,

    /// 解压到固定目录
    FixedDirectory(String),

    /// 解压到自动生成的目录
    AutoGenerate,

    /// 每次询问用户
    AskUser,
}

/// 覆盖策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverwritePolicy {
    /// 总是覆盖
    AlwaysOverwrite,

    /// 总是跳过
    AlwaysSkip,

    /// 根据时间戳决定（保留新的）
    KeepNewer,

    /// 重命名新文件
    RenameNew,

    /// 询问用户
    AskUser,
}

/// 自动应用规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoApplyRule {
    /// 是否启用自动应用
    pub enabled: bool,

    /// 应用模式
    pub mode: AutoApplyMode,

    /// 文件扩展名匹配（如 "zip", "7z", "rar"）
    pub extension_patterns: Vec<String>,

    /// 文件大小范围（min_mb, max_mb）
    pub size_range: Option<(u64, u64)>,

    /// 文件名模式（正则表达式）
    pub filename_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoApplyMode {
    /// 不自动应用
    None,

    /// 应用到所有文件
    All,

    /// 仅应用到匹配扩展名的文件
    Extension,

    /// 仅应用到指定大小范围的文件
    SizeRange,

    /// 仅应用到匹配文件名模式的文件
    Filename,
}

/// 密码尝试策略配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordAttemptStrategyConfig {
    /// 是否启用自动密码尝试
    pub enabled: bool,

    /// 是否启用并行尝试
    pub enable_parallel: bool,

    /// 并行度
    pub parallelism: usize,

    /// 策略列表（按顺序尝试）
    pub strategies: Vec<PasswordStrategyType>,

    /// 是否从密码本尝试
    pub try_known_passwords: bool,

    /// 密码本策略（All/Recent/Category等）
    pub password_vault_strategy: String,

    /// 是否从词表文件尝试
    pub try_wordlists: bool,

    /// 词表文件路径列表
    pub wordlist_paths: Vec<String>,

    /// 最大尝试次数
    pub max_attempts: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PasswordStrategyType {
    /// 尝试空密码
    Empty,

    /// 尝试基于文件名的常见密码
    FilenameGuess,

    /// 从密码保险箱获取
    PasswordVault,

    /// 从词表文件获取
    Wordlist,

    /// 手动输入
    Manual,
}

/// 配置组统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileStats {
    /// 使用次数
    pub use_count: u32,

    /// 成功次数
    pub success_count: u32,

    /// 失败次数
    pub failure_count: u32,

    /// 处理的文件总数
    pub total_files_processed: u64,

    /// 处理的字节总数
    pub total_bytes_processed: u64,

    /// 平均解压时间（秒）
    pub avg_extraction_time: Option<f64>,
}

impl Default for ProfileStats {
    fn default() -> Self {
        Self {
            use_count: 0,
            success_count: 0,
            failure_count: 0,
            total_files_processed: 0,
            total_bytes_processed: 0,
            avg_extraction_time: None,
        }
    }
}

/// 创建默认解压配置组
pub fn create_default_profiles() -> Vec<DecompressionProfile> {
    vec![
        // 1. 快速解压（默认）
        DecompressionProfile {
            id: "quick-extract".to_string(),
            name: "快速解压".to_string(),
            icon: "⚡".to_string(),
            description: "快速解压到源文件目录，自动创建同名文件夹".to_string(),
            config: DecompressionConfig {
                output_mode: OutputMode::SameAsSource,
                create_subdirectory: true,
                subdirectory_template: Some("{name}".to_string()),
                preserve_paths: true,
                overwrite_policy: OverwritePolicy::KeepNewer,
                preserve_timestamps: true,
                delete_after: false,
                skip_corrupted: true,
                extract_only_newer: false,
                file_filter: None,
                extra_params: HashMap::new(),
            },
            auto_apply: AutoApplyRule {
                enabled: false,
                mode: AutoApplyMode::None,
                extension_patterns: vec![],
                size_range: None,
                filename_patterns: vec![],
            },
            password_attempt_strategy: PasswordAttemptStrategyConfig {
                enabled: true,
                enable_parallel: true,
                parallelism: 4,
                strategies: vec![
                    PasswordStrategyType::Empty,
                    PasswordStrategyType::PasswordVault,
                    PasswordStrategyType::FilenameGuess,
                ],
                try_known_passwords: true,
                password_vault_strategy: "Recent(50)".to_string(),
                try_wordlists: false,
                wordlist_paths: vec![],
                max_attempts: Some(100),
            },
            stats: ProfileStats::default(),
            created_at: chrono::Utc::now().timestamp(),
            last_used_at: None,
        },

        // 2. 智能解密
        DecompressionProfile {
            id: "smart-decrypt".to_string(),
            name: "智能解密".to_string(),
            icon: "🔓".to_string(),
            description: "自动尝试密码本和词表，支持并行破解".to_string(),
            config: DecompressionConfig {
                output_mode: OutputMode::SameAsSource,
                create_subdirectory: true,
                subdirectory_template: Some("{name}".to_string()),
                preserve_paths: true,
                overwrite_policy: OverwritePolicy::KeepNewer,
                preserve_timestamps: true,
                delete_after: false,
                skip_corrupted: true,
                extract_only_newer: false,
                file_filter: None,
                extra_params: HashMap::new(),
            },
            auto_apply: AutoApplyRule {
                enabled: false,
                mode: AutoApplyMode::None,
                extension_patterns: vec![],
                size_range: None,
                filename_patterns: vec![],
            },
            password_attempt_strategy: PasswordAttemptStrategyConfig {
                enabled: true,
                enable_parallel: true,
                parallelism: 8,
                strategies: vec![
                    PasswordStrategyType::Empty,
                    PasswordStrategyType::PasswordVault,
                    PasswordStrategyType::FilenameGuess,
                    PasswordStrategyType::Wordlist,
                ],
                try_known_passwords: true,
                password_vault_strategy: "All".to_string(),
                try_wordlists: true,
                wordlist_paths: vec![],
                max_attempts: Some(10000),
            },
            stats: ProfileStats::default(),
            created_at: chrono::Utc::now().timestamp(),
            last_used_at: None,
        },

        // 3. 安全解压
        DecompressionProfile {
            id: "safe-extract".to_string(),
            name: "安全解压".to_string(),
            icon: "🛡️".to_string(),
            description: "保护现有文件，总是重命名新文件，跳过损坏文件".to_string(),
            config: DecompressionConfig {
                output_mode: OutputMode::SameAsSource,
                create_subdirectory: true,
                subdirectory_template: Some("{name}".to_string()),
                preserve_paths: true,
                overwrite_policy: OverwritePolicy::RenameNew,
                preserve_timestamps: true,
                delete_after: false,
                skip_corrupted: true,
                extract_only_newer: false,
                file_filter: None,
                extra_params: HashMap::new(),
            },
            auto_apply: AutoApplyRule {
                enabled: false,
                mode: AutoApplyMode::None,
                extension_patterns: vec![],
                size_range: None,
                filename_patterns: vec![],
            },
            password_attempt_strategy: PasswordAttemptStrategyConfig {
                enabled: true,
                enable_parallel: false,
                parallelism: 1,
                strategies: vec![
                    PasswordStrategyType::Empty,
                    PasswordStrategyType::Manual,
                ],
                try_known_passwords: false,
                password_vault_strategy: "None".to_string(),
                try_wordlists: false,
                wordlist_paths: vec![],
                max_attempts: Some(3),
            },
            stats: ProfileStats::default(),
            created_at: chrono::Utc::now().timestamp(),
            last_used_at: None,
        },

        // 4. 清理模式
        DecompressionProfile {
            id: "extract-and-delete".to_string(),
            name: "清理模式".to_string(),
            icon: "🗑️".to_string(),
            description: "解压后自动删除压缩包，节省空间".to_string(),
            config: DecompressionConfig {
                output_mode: OutputMode::SameAsSource,
                create_subdirectory: true,
                subdirectory_template: Some("{name}".to_string()),
                preserve_paths: true,
                overwrite_policy: OverwritePolicy::AlwaysOverwrite,
                preserve_timestamps: true,
                delete_after: true,
                skip_corrupted: true,
                extract_only_newer: false,
                file_filter: None,
                extra_params: HashMap::new(),
            },
            auto_apply: AutoApplyRule {
                enabled: false,
                mode: AutoApplyMode::None,
                extension_patterns: vec![],
                size_range: None,
                filename_patterns: vec![],
            },
            password_attempt_strategy: PasswordAttemptStrategyConfig {
                enabled: true,
                enable_parallel: true,
                parallelism: 4,
                strategies: vec![
                    PasswordStrategyType::Empty,
                    PasswordStrategyType::PasswordVault,
                ],
                try_known_passwords: true,
                password_vault_strategy: "Recent(20)".to_string(),
                try_wordlists: false,
                wordlist_paths: vec![],
                max_attempts: Some(50),
            },
            stats: ProfileStats::default(),
            created_at: chrono::Utc::now().timestamp(),
            last_used_at: None,
        },

        // 5. 更新模式
        DecompressionProfile {
            id: "update-only".to_string(),
            name: "更新模式".to_string(),
            icon: "🔄".to_string(),
            description: "只解压比现有文件新的文件".to_string(),
            config: DecompressionConfig {
                output_mode: OutputMode::SameAsSource,
                create_subdirectory: false,
                subdirectory_template: None,
                preserve_paths: true,
                overwrite_policy: OverwritePolicy::KeepNewer,
                preserve_timestamps: true,
                delete_after: false,
                skip_corrupted: true,
                extract_only_newer: true,
                file_filter: None,
                extra_params: HashMap::new(),
            },
            auto_apply: AutoApplyRule {
                enabled: false,
                mode: AutoApplyMode::None,
                extension_patterns: vec![],
                size_range: None,
                filename_patterns: vec![],
            },
            password_attempt_strategy: PasswordAttemptStrategyConfig {
                enabled: true,
                enable_parallel: true,
                parallelism: 4,
                strategies: vec![
                    PasswordStrategyType::Empty,
                    PasswordStrategyType::PasswordVault,
                ],
                try_known_passwords: true,
                password_vault_strategy: "Recent(30)".to_string(),
                try_wordlists: false,
                wordlist_paths: vec![],
                max_attempts: Some(30),
            },
            stats: ProfileStats::default(),
            created_at: chrono::Utc::now().timestamp(),
            last_used_at: None,
        },
    ]
}
