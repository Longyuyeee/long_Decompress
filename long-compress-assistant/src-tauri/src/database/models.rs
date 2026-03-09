use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CompressionTaskDb {
    pub id: String,
    pub source_files: String, // JSON序列化的文件列表
    pub output_path: String,
    pub format: String,
    pub options: String, // JSON序列化的选项
    pub status: String,
    pub progress: f32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub total_size: i64,
    pub processed_size: i64,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CompressionHistoryDb {
    pub id: String,
    pub task_id: String,
    pub operation_type: String,
    pub source_paths: String, // JSON序列化的路径列表
    pub output_path: String,
    pub format: String,
    pub size_before: i64,
    pub size_after: i64,
    pub compression_ratio: f32,
    pub duration_seconds: f32,
    pub created_at: DateTime<Utc>,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasswordKeyDb {
    pub id: String,
    pub key_type: String,
    pub algorithm: String,
    pub key_data: String,
    pub key_hash: String,
    pub key_size: i32,
    pub key_version: i32,
    pub derived_from: Option<String>,
    pub max_usage_count: Option<i32>,
    pub usage_count: i32,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub rotated_at: Option<DateTime<Utc>>,
    pub rotated_to: Option<String>,
    pub active: bool,
    pub archived: bool,
    pub metadata: String, // JSON序列化的元数据
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasswordEntryDb {
    pub id: String,
    pub name: String,
    pub username: Option<String>,
    pub password: String, // 加密存储
    pub url: Option<String>,
    pub notes: Option<String>,
    pub tags: String, // JSON序列化的标签列表
    pub category: String,
    pub strength: String,
    pub key_id: String, // 外键到password_keys
    pub encryption_algorithm: String,
    pub encryption_version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub favorite: bool,
    pub archived: bool,
    pub deleted: bool,
    pub custom_fields: String, // JSON序列化的自定义字段
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasswordGroupDb {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub require_master_password: bool,
    pub auto_lock_minutes: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub favorite: bool,
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasswordGroupEntryDb {
    pub id: String,
    pub group_id: String,
    pub entry_id: String,
    pub added_at: DateTime<Utc>,
    pub added_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasswordAuditDb {
    pub id: String,
    pub entry_id: String,
    pub audit_type: String,
    pub severity: String,
    pub score: i32,
    pub issues: String, // JSON序列化的问题列表
    pub recommendations: String, // JSON序列化的建议列表
    pub auditor: Option<String>,
    pub audit_date: DateTime<Utc>,
    pub next_audit_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasswordUsageHistoryDb {
    pub id: String,
    pub entry_id: String,
    pub user_id: Option<String>,
    pub action: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub device_info: Option<String>,
    pub used_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasswordPolicyDb {
    pub id: String,
    pub policy_name: String,
    pub policy_type: String,
    pub description: Option<String>,
    pub min_length: Option<i32>,
    pub require_uppercase: Option<bool>,
    pub require_lowercase: Option<bool>,
    pub require_numbers: Option<bool>,
    pub require_symbols: Option<bool>,
    pub max_age_days: Option<i32>,
    pub warn_before_days: Option<i32>,
    pub prevent_reuse_count: Option<i32>,
    pub prevent_similarity: Option<bool>,
    pub apply_to_categories: String, // JSON序列化的分类列表
    pub apply_to_groups: String, // JSON序列化的组列表
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasswordImportExportDb {
    pub id: String,
    pub operation_type: String,
    pub format: String,
    pub file_name: Option<String>,
    pub file_size: Option<i64>,
    pub encrypted: bool,
    pub encryption_algorithm: Option<String>,
    pub entry_count: Option<i32>,
    pub success_count: Option<i32>,
    pub failed_count: Option<i32>,
    pub status: String,
    pub error_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FileOperationDb {
    pub id: String,
    pub operation_type: String,
    pub source_paths: String, // JSON序列化的路径列表
    pub destination_path: Option<String>,
    pub status: String,
    pub progress: f32,
    pub total_size: i64,
    pub processed_size: i64,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SystemMetricsDb {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub cpu_usage: f32,
    pub memory_usage: i64,
    pub disk_io_read: i64,
    pub disk_io_write: i64,
    pub network_io_received: i64,
    pub network_io_transmitted: i64,
    pub process_count: i32,
    pub thread_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SystemAlertDb {
    pub id: String,
    pub alert_type: String,
    pub severity: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub acknowledged: bool,
    pub component: Option<String>,
    pub value: Option<f32>,
    pub threshold: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApplicationSettingsDb {
    pub id: String,
    pub key: String,
    pub value: String, // JSON序列化的值
    pub category: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserSessionDb {
    pub id: String,
    pub user_id: Option<String>,
    pub session_token: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLogDb {
    pub id: String,
    pub user_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: String, // JSON序列化的详细信息
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

// 转换函数
impl From<CompressionTaskDb> for crate::models::compression::CompressionTask {
    fn from(db: CompressionTaskDb) -> Self {
        use crate::models::compression::*;

        let source_files: Vec<String> = serde_json::from_str(&db.source_files)
            .unwrap_or_else(|_| Vec::new());

        let options: CompressionOptions = serde_json::from_str(&db.options)
            .unwrap_or_default();

        let format = CompressionFormat::from_extension(&db.format);

        let status = match db.status.as_str() {
            "Preparing" => CompressionStatus::Preparing,
            "Compressing" => CompressionStatus::Compressing,
            "Extracting" => CompressionStatus::Extracting,
            "Finalizing" => CompressionStatus::Finalizing,
            "Completed" => CompressionStatus::Completed,
            "Failed" => CompressionStatus::Failed,
            "Cancelled" => CompressionStatus::Cancelled,
            _ => CompressionStatus::Pending,
        };

        crate::models::compression::CompressionTask {
            id: db.id,
            source_files,
            output_path: db.output_path,
            format,
            options,
            status,
            progress: db.progress as f32,
            created_at: db.created_at,
            started_at: db.started_at,
            completed_at: db.completed_at,
            error_message: db.error_message,
            total_size: db.total_size as u64,
            processed_size: db.processed_size as u64,
            password: db.password,
        }
    }
}

impl From<crate::models::compression::CompressionTask> for CompressionTaskDb {
    fn from(task: crate::models::compression::CompressionTask) -> Self {
        let source_files = serde_json::to_string(&task.source_files)
            .unwrap_or_else(|_| "[]".to_string());

        let options = serde_json::to_string(&task.options)
            .unwrap_or_else(|_| "{}".to_string());

        let status = match task.status {
            crate::models::compression::CompressionStatus::Pending => "Pending",
            crate::models::compression::CompressionStatus::Preparing => "Preparing",
            crate::models::compression::CompressionStatus::Compressing => "Compressing",
            crate::models::compression::CompressionStatus::Extracting => "Extracting",
            crate::models::compression::CompressionStatus::Finalizing => "Finalizing",
            crate::models::compression::CompressionStatus::Running => "Running",
            crate::models::compression::CompressionStatus::Completed => "Completed",
            crate::models::compression::CompressionStatus::Failed => "Failed",
            crate::models::compression::CompressionStatus::Cancelled => "Cancelled",
        }.to_string();

        Self {
            id: task.id,
            source_files,
            output_path: task.output_path,
            format: task.format.extension().to_string(),
            options,
            status,
            progress: task.progress,
            created_at: task.created_at,
            started_at: task.started_at,
            completed_at: task.completed_at,
            error_message: task.error_message,
            total_size: task.total_size as i64,
            processed_size: task.processed_size as i64,
            password: task.password,
        }
    }
}

impl From<PasswordGroupDb> for crate::models::password::PasswordGroup {
    fn from(db: PasswordGroupDb) -> Self {
        crate::models::password::PasswordGroup {
            id: db.id,
            name: db.name,
            description: db.description,
            entry_ids: Vec::new(),
            created_at: db.created_at,
            updated_at: db.updated_at,
        }
    }
}

impl From<crate::models::password::PasswordGroup> for PasswordGroupDb {
    fn from(group: crate::models::password::PasswordGroup) -> Self {
        Self {
            id: group.id,
            name: group.name,
            description: group.description,
            category: "Other".to_string(),
            icon: None,
            color: None,
            require_master_password: false,
            auto_lock_minutes: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            favorite: false,
            archived: false,
        }
    }
}

impl From<PasswordEntryDb> for crate::models::password::PasswordEntry {
    fn from(db: PasswordEntryDb) -> Self {
        use crate::models::password::*;

        let tags: Vec<String> = serde_json::from_str(&db.tags)
            .unwrap_or_else(|_| Vec::new());

        let custom_fields: Vec<CustomField> = serde_json::from_str(&db.custom_fields)
            .unwrap_or_else(|_| Vec::new());

        let category = match db.category.as_str() {
            "Work" => PasswordCategory::Work,
            "Finance" => PasswordCategory::Finance,
            "Social" => PasswordCategory::Social,
            "Shopping" => PasswordCategory::Shopping,
            "Entertainment" => PasswordCategory::Entertainment,
            "Education" => PasswordCategory::Education,
            "Travel" => PasswordCategory::Travel,
            "Health" => PasswordCategory::Health,
            "Other" => PasswordCategory::Other,
            _ => PasswordCategory::Personal,
        };

        let strength = match db.strength.as_str() {
            "VeryWeak" => PasswordStrength::VeryWeak,
            "Weak" => PasswordStrength::Weak,
            "Medium" => PasswordStrength::Medium,
            "Strong" => PasswordStrength::Strong,
            "VeryStrong" => PasswordStrength::VeryStrong,
            _ => PasswordStrength::Weak,
        };

        Self {
            id: db.id,
            name: db.name,
            username: db.username,
            password: db.password,
            url: db.url,
            notes: db.notes,
            tags,
            category,
            strength,
            created_at: db.created_at,
            updated_at: db.updated_at,
            last_used: db.last_used,
            expires_at: db.expires_at,
            favorite: db.favorite,
            custom_fields,
            // 注意：新添加的字段在PasswordEntry模型中可能不存在
            // 需要在models/password.rs中添加相应字段
        }
    }
}

impl From<crate::models::password::PasswordEntry> for PasswordEntryDb {
    fn from(entry: crate::models::password::PasswordEntry) -> Self {
        let tags = serde_json::to_string(&entry.tags)
            .unwrap_or_else(|_| "[]".to_string());

        let custom_fields = serde_json::to_string(&entry.custom_fields)
            .unwrap_or_else(|_| "[]".to_string());

        let category = match entry.category {
            crate::models::password::PasswordCategory::Personal => "Personal",
            crate::models::password::PasswordCategory::Work => "Work",
            crate::models::password::PasswordCategory::Finance => "Finance",
            crate::models::password::PasswordCategory::Social => "Social",
            crate::models::password::PasswordCategory::Shopping => "Shopping",
            crate::models::password::PasswordCategory::Entertainment => "Entertainment",
            crate::models::password::PasswordCategory::Education => "Education",
            crate::models::password::PasswordCategory::Travel => "Travel",
            crate::models::password::PasswordCategory::Health => "Health",
            crate::models::password::PasswordCategory::Other => "Other",
        }.to_string();

        let strength = match entry.strength {
            crate::models::password::PasswordStrength::VeryWeak => "VeryWeak",
            crate::models::password::PasswordStrength::Weak => "Weak",
            crate::models::password::PasswordStrength::Medium => "Medium",
            crate::models::password::PasswordStrength::Strong => "Strong",
            crate::models::password::PasswordStrength::VeryStrong => "VeryStrong",
        }.to_string();

        Self {
            id: entry.id,
            name: entry.name,
            username: entry.username,
            password: entry.password,
            url: entry.url,
            notes: entry.notes,
            tags,
            category,
            strength,
            // 新字段使用默认值，需要在PasswordEntry模型中添加相应字段
            key_id: "".to_string(), // 需要从加密服务获取
            encryption_algorithm: "AES256GCM".to_string(),
            encryption_version: 1,
            created_at: entry.created_at,
            updated_at: entry.updated_at,
            last_used: entry.last_used,
            expires_at: entry.expires_at,
            favorite: entry.favorite,
            archived: false,
            deleted: false,
            custom_fields,
        }
    }
}