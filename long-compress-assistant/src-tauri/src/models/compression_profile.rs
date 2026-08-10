use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 压缩配置组 - 可复用的压缩设置模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionProfile {
    /// 配置组唯一标识
    pub id: String,

    /// 配置组名称（用户可见）
    pub name: String,

    /// 配置组图标（emoji）
    pub icon: String,

    /// 配置组描述
    pub description: String,

    /// 核心压缩配置
    pub config: CompressionConfig,

    /// 批量应用规则
    pub auto_apply: AutoApplyRule,

    /// 密码策略
    pub password_strategy: PasswordStrategy,

    /// 统计信息
    pub stats: ProfileStats,

    /// 创建时间（Unix 时间戳）
    pub created_at: i64,

    /// 最后使用时间
    pub last_used_at: Option<i64>,
}

/// 压缩配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// 压缩格式（zip/7z/rar/tar.gz 等）
    pub format: String,

    /// 压缩级别（0-9）
    pub level: u8,

    /// 固定密码（如果使用固定密码策略）
    pub password: Option<String>,

    /// 是否分卷压缩
    pub split_archive: bool,

    /// 分卷大小（MB）
    pub split_size: Option<u32>,

    /// 是否保留目录结构
    pub keep_structure: bool,

    /// 压缩后是否删除源文件
    pub delete_after: bool,

    /// 压缩完成后、发布最终文件前是否执行完整性校验
    #[serde(default = "default_verify_after")]
    pub verify_after: bool,

    /// 是否创建固实归档（7z 专属）
    pub create_solid_archive: bool,

    /// 输出文件名模板（支持变量：{name}, {date}, {time}）
    pub filename_template: Option<String>,

    /// 额外参数（格式特定的高级选项）
    pub extra_params: HashMap<String, String>,
}

fn default_verify_after() -> bool {
    true
}

/// 自动应用规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoApplyRule {
    /// 是否启用自动应用
    pub enabled: bool,

    /// 应用模式
    pub mode: AutoApplyMode,

    /// 文件模式匹配（如 "*.jpg", "*.mp4"）
    pub file_patterns: Vec<String>,

    /// 文件大小范围（min_mb, max_mb）
    pub size_range: Option<(u64, u64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoApplyMode {
    /// 不自动应用
    None,

    /// 应用到所有文件
    All,

    /// 仅应用到匹配模式的文件
    Pattern,

    /// 仅应用到指定大小范围的文件
    SizeRange,
}

/// 密码策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PasswordStrategy {
    /// 不使用密码
    None,

    /// 使用固定密码（存储在 config.password）
    Fixed,

    /// 从密码本指定分类中选择
    FromVault { category_id: Option<String> },

    /// 自动生成密码（并保存到密码本）
    AutoGenerate { length: u8, save_to_vault: bool },
}

/// 配置组统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct ProfileStats {
    /// 使用次数
    pub use_count: u64,

    /// 成功次数
    pub success_count: u64,

    /// 失败次数
    pub failure_count: u64,

    /// 总处理文件数
    pub total_files_processed: u64,

    /// 总处理大小（字节）
    pub total_bytes_processed: u64,
}


impl CompressionProfile {
    /// 创建新的配置组
    pub fn new(name: String, icon: String, description: String, config: CompressionConfig) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            icon,
            description,
            config,
            auto_apply: AutoApplyRule {
                enabled: false,
                mode: AutoApplyMode::None,
                file_patterns: vec![],
                size_range: None,
            },
            password_strategy: PasswordStrategy::None,
            stats: ProfileStats::default(),
            created_at: chrono::Utc::now().timestamp(),
            last_used_at: None,
        }
    }

    /// 更新使用统计
    pub fn record_usage(&mut self, success: bool, files_count: u64, bytes_processed: u64) {
        self.stats.use_count += 1;
        if success {
            self.stats.success_count += 1;
        } else {
            self.stats.failure_count += 1;
        }
        self.stats.total_files_processed += files_count;
        self.stats.total_bytes_processed += bytes_processed;
        self.last_used_at = Some(chrono::Utc::now().timestamp());
    }

    /// 检查文件是否匹配自动应用规则
    pub fn matches_auto_apply(&self, file_path: &str, file_size: u64) -> bool {
        if !self.auto_apply.enabled {
            return false;
        }

        match &self.auto_apply.mode {
            AutoApplyMode::None => false,
            AutoApplyMode::All => true,
            AutoApplyMode::Pattern => {
                self.auto_apply.file_patterns.iter().any(|pattern| {
                    glob::Pattern::new(pattern)
                        .map(|p| p.matches(file_path))
                        .unwrap_or(false)
                })
            }
            AutoApplyMode::SizeRange => {
                if let Some((min, max)) = self.auto_apply.size_range {
                    let size_mb = file_size / (1024 * 1024);
                    size_mb >= min && size_mb <= max
                } else {
                    false
                }
            }
        }
    }
}

/// 内置默认配置组
pub fn create_default_profiles() -> Vec<CompressionProfile> {
    vec![
        CompressionProfile::new(
            "🔥 极限压缩".to_string(),
            "🔥".to_string(),
            "最高压缩率，适合长期存档（7Z-L9 固实）".to_string(),
            CompressionConfig {
                format: "7z".to_string(),
                level: 9,
                password: None,
                split_archive: false,
                split_size: None,
                keep_structure: true,
                delete_after: false,
                verify_after: true,
                create_solid_archive: true,
                filename_template: Some("{name}_极限压缩_{date}".to_string()),
                extra_params: HashMap::new(),
            },
        ),
        CompressionProfile::new(
            "⚡ 快速压缩".to_string(),
            "⚡".to_string(),
            "极速处理，适合临时打包（ZIP-L1）".to_string(),
            CompressionConfig {
                format: "zip".to_string(),
                level: 1,
                password: None,
                split_archive: false,
                split_size: None,
                keep_structure: true,
                delete_after: false,
                verify_after: true,
                create_solid_archive: false,
                filename_template: Some("{name}_快速_{date}".to_string()),
                extra_params: HashMap::new(),
            },
        ),
        CompressionProfile::new(
            "🔐 加密归档".to_string(),
            "🔐".to_string(),
            "平衡压缩 + 密码保护（7Z-L6）".to_string(),
            CompressionConfig {
                format: "7z".to_string(),
                level: 6,
                password: None,
                split_archive: false,
                split_size: None,
                keep_structure: true,
                delete_after: false,
                verify_after: true,
                create_solid_archive: false,
                filename_template: Some("{name}_加密_{date}".to_string()),
                extra_params: HashMap::new(),
            },
        ),
        CompressionProfile::new(
            "📦 分卷备份".to_string(),
            "📦".to_string(),
            "适合大文件分卷传输（ZIP-L6，4GB 分卷）".to_string(),
            CompressionConfig {
                format: "zip".to_string(),
                level: 6,
                password: None,
                split_archive: true,
                split_size: Some(4096),
                keep_structure: true,
                delete_after: false,
                verify_after: true,
                create_solid_archive: false,
                filename_template: Some("{name}_分卷_{date}".to_string()),
                extra_params: HashMap::new(),
            },
        ),
        CompressionProfile::new(
            "📄 文档归档".to_string(),
            "📄".to_string(),
            "Linux 通用格式（TAR.GZ-L6）".to_string(),
            CompressionConfig {
                format: "tar.gz".to_string(),
                level: 6,
                password: None,
                split_archive: false,
                split_size: None,
                keep_structure: true,
                delete_after: false,
                verify_after: true,
                create_solid_archive: false,
                filename_template: Some("{name}_归档_{date}".to_string()),
                extra_params: HashMap::new(),
            },
        ),
    ]
}
