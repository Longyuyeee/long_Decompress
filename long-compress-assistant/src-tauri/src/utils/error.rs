use thiserror::Error;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("IO错误: {0}")]
    Io(Arc<std::io::Error>),

    #[error("数据库错误: {0}")]
    Database(Arc<sqlx::Error>),

    #[error("JSON错误: {0}")]
    Json(Arc<serde_json::Error>),

    #[error("加密错误: {0}")]
    Crypto(String),

    #[error("压缩错误: {0}")]
    Compression(String),

    #[error("文件错误: {0}")]
    File(String),

    #[error("系统错误: {0}")]
    System(String),

    #[error("验证错误: {0}")]
    Validation(String),

    #[error("配置错误: {0}")]
    Config(String),

    #[error("网络错误: {0}")]
    Network(String),

    #[error("权限错误: {0}")]
    Permission(String),

    #[error("资源未找到: {0}")]
    NotFound(String),

    #[error("资源已存在: {0}")]
    AlreadyExists(String),

    #[error("无效参数: {0}")]
    InvalidArgument(String),

    #[error("操作超时: {0}")]
    Timeout(String),

    #[error("操作被取消")]
    Cancelled,

    #[error("任务错误: {0}")]
    Task(String),

    #[error("未知错误: {0}")]
    Unknown(String),
}

impl Clone for AppError {
    fn clone(&self) -> Self {
        match self {
            Self::Io(e) => Self::Io(Arc::clone(e)),
            Self::Database(e) => Self::Database(Arc::clone(e)),
            Self::Json(e) => Self::Json(Arc::clone(e)),
            Self::Crypto(s) => Self::Crypto(s.clone()),
            Self::Compression(s) => Self::Compression(s.clone()),
            Self::File(s) => Self::File(s.clone()),
            Self::System(s) => Self::System(s.clone()),
            Self::Validation(s) => Self::Validation(s.clone()),
            Self::Config(s) => Self::Config(s.clone()),
            Self::Network(s) => Self::Network(s.clone()),
            Self::Permission(s) => Self::Permission(s.clone()),
            Self::NotFound(s) => Self::NotFound(s.clone()),
            Self::AlreadyExists(s) => Self::AlreadyExists(s.clone()),
            Self::InvalidArgument(s) => Self::InvalidArgument(s.clone()),
            Self::Timeout(s) => Self::Timeout(s.clone()),
            Self::Cancelled => Self::Cancelled,
            Self::Task(s) => Self::Task(s.clone()),
            Self::Unknown(s) => Self::Unknown(s.clone()),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        AppError::Io(Arc::new(error))
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        AppError::Database(Arc::new(error))
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        AppError::Json(Arc::new(error))
    }
}

impl AppError {
    pub fn crypto(msg: impl Into<String>) -> Self {
        AppError::Crypto(msg.into())
    }

    pub fn compression(msg: impl Into<String>) -> Self {
        AppError::Compression(msg.into())
    }

    pub fn file(msg: impl Into<String>) -> Self {
        AppError::File(msg.into())
    }

    pub fn system(msg: impl Into<String>) -> Self {
        AppError::System(msg.into())
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        AppError::Validation(msg.into())
    }

    pub fn config(msg: impl Into<String>) -> Self {
        AppError::Config(msg.into())
    }

    pub fn network(msg: impl Into<String>) -> Self {
        AppError::Network(msg.into())
    }

    pub fn permission(msg: impl Into<String>) -> Self {
        AppError::Permission(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        AppError::NotFound(msg.into())
    }

    pub fn already_exists(msg: impl Into<String>) -> Self {
        AppError::AlreadyExists(msg.into())
    }

    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        AppError::InvalidArgument(msg.into())
    }

    pub fn timeout(msg: impl Into<String>) -> Self {
        AppError::Timeout(msg.into())
    }

    pub fn task(msg: impl Into<String>) -> Self {
        AppError::Task(msg.into())
    }

    pub fn unknown(msg: impl Into<String>) -> Self {
        AppError::Unknown(msg.into())
    }

    pub fn to_api_error(&self) -> ApiError {
        let (code, message, severity) = match self {
            AppError::Io(e) => (ErrorCode::IoError, e.to_string(), ErrorSeverity::Error),
            AppError::Database(e) => (ErrorCode::DatabaseError, e.to_string(), ErrorSeverity::Critical),
            AppError::Json(e) => (ErrorCode::JsonError, e.to_string(), ErrorSeverity::Error),
            AppError::Crypto(msg) => (ErrorCode::CryptoError, msg.clone(), ErrorSeverity::Error),
            AppError::Compression(msg) => (ErrorCode::CompressionError, msg.clone(), ErrorSeverity::Error),
            AppError::File(msg) => (ErrorCode::FileError, msg.clone(), ErrorSeverity::Error),
            AppError::System(msg) => (ErrorCode::SystemError, msg.clone(), ErrorSeverity::Critical),
            AppError::Validation(msg) => (ErrorCode::ValidationError, msg.clone(), ErrorSeverity::Warning),
            AppError::Config(msg) => (ErrorCode::ConfigError, msg.clone(), ErrorSeverity::Error),
            AppError::Network(msg) => (ErrorCode::NetworkError, msg.clone(), ErrorSeverity::Warning),
            AppError::Permission(msg) => (ErrorCode::PermissionError, msg.clone(), ErrorSeverity::Warning),
            AppError::NotFound(msg) => (ErrorCode::NotFound, msg.clone(), ErrorSeverity::Warning),
            AppError::AlreadyExists(msg) => (ErrorCode::AlreadyExists, msg.clone(), ErrorSeverity::Warning),
            AppError::InvalidArgument(msg) => (ErrorCode::InvalidArgument, msg.clone(), ErrorSeverity::Warning),
            AppError::Timeout(msg) => (ErrorCode::Timeout, msg.clone(), ErrorSeverity::Warning),
            AppError::Cancelled => (ErrorCode::Cancelled, "操作被取消".to_string(), ErrorSeverity::Info),
            AppError::Task(msg) => (ErrorCode::SystemError, msg.clone(), ErrorSeverity::Error),
            AppError::Unknown(msg) => (ErrorCode::UnknownError, msg.clone(), ErrorSeverity::Error),
        };

        ApiError::new(code, message, severity)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        if let Some(io_error) = error.downcast_ref::<std::io::Error>() {
            // 注意：downcast_ref 得到的是引用，无法直接通过 From 转为 AppError::Io(Arc<...>)
            // 除非 std::io::Error 实现了 Clone，或者我们有办法通过引用创建 Arc。
            // 简单的做法是直接转为字符串，或者重新尝试转换逻辑。
            return AppError::Unknown(io_error.to_string());
        }
        
        AppError::Unknown(error.to_string())
    }
}

impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    pub severity: ErrorSeverity,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub details: Option<serde_json::Value>,
    pub request_id: Option<String>,
}

impl ApiError {
    pub fn new(code: ErrorCode, message: String, severity: ErrorSeverity) -> Self {
        Self {
            code,
            message,
            severity,
            timestamp: chrono::Utc::now(),
            details: None,
            request_id: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ErrorCode {
    UnknownError,
    ValidationError,
    ConfigError,
    Timeout,
    Cancelled,
    IoError,
    FileError,
    FileNotFound,
    FileAlreadyExists,
    FilePermissionDenied,
    DatabaseError,
    DatabaseConnectionError,
    DatabaseQueryError,
    DatabaseConstraintViolation,
    NetworkError,
    ConnectionError,
    RequestError,
    ResponseError,
    CryptoError,
    EncryptionError,
    DecryptionError,
    HashError,
    KeyError,
    CompressionError,
    DecompressionError,
    FormatNotSupported,
    PasswordRequired,
    InvalidPassword,
    SystemError,
    MemoryError,
    ProcessError,
    ResourceExhausted,
    NotFound,
    AlreadyExists,
    InvalidArgument,
    PermissionError,
    AuthenticationError,
    AuthorizationError,
    JsonError,
}

impl ErrorCode {
    pub fn name(&self) -> &'static str {
        match self {
            Self::UnknownError => "UNKNOWN_ERROR",
            Self::ValidationError => "VALIDATION_ERROR",
            Self::ConfigError => "CONFIG_ERROR",
            Self::Timeout => "TIMEOUT",
            Self::Cancelled => "CANCELLED",
            Self::IoError => "IO_ERROR",
            Self::FileError => "FILE_ERROR",
            Self::FileNotFound => "FILE_NOT_FOUND",
            Self::FileAlreadyExists => "FILE_ALREADY_EXISTS",
            Self::FilePermissionDenied => "FILE_PERMISSION_DENIED",
            Self::DatabaseError => "DATABASE_ERROR",
            Self::DatabaseConnectionError => "DATABASE_CONNECTION_ERROR",
            Self::DatabaseQueryError => "DATABASE_QUERY_ERROR",
            Self::DatabaseConstraintViolation => "DATABASE_CONSTRAINT_VIOLATION",
            Self::NetworkError => "NETWORK_ERROR",
            Self::ConnectionError => "CONNECTION_ERROR",
            Self::RequestError => "REQUEST_ERROR",
            Self::ResponseError => "RESPONSE_ERROR",
            Self::CryptoError => "CRYPTO_ERROR",
            Self::EncryptionError => "ENCRYPTION_ERROR",
            Self::DecryptionError => "DECRYPTION_ERROR",
            Self::HashError => "HASH_ERROR",
            Self::KeyError => "KEY_ERROR",
            Self::CompressionError => "COMPRESSION_ERROR",
            Self::DecompressionError => "DECOMPRESSION_ERROR",
            Self::FormatNotSupported => "FORMAT_NOT_SUPPORTED",
            Self::PasswordRequired => "PASSWORD_REQUIRED",
            Self::InvalidPassword => "INVALID_PASSWORD",
            Self::SystemError => "SYSTEM_ERROR",
            Self::MemoryError => "MEMORY_ERROR",
            Self::ProcessError => "PROCESS_ERROR",
            Self::ResourceExhausted => "RESOURCE_EXHAUSTED",
            Self::NotFound => "NOT_FOUND",
            Self::AlreadyExists => "ALREADY_EXISTS",
            Self::InvalidArgument => "INVALID_ARGUMENT",
            Self::PermissionError => "PERMISSION_ERROR",
            Self::AuthenticationError => "AUTHENTICATION_ERROR",
            Self::AuthorizationError => "AUTHORIZATION_ERROR",
            Self::JsonError => "JSON_ERROR",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl ErrorSeverity {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warning => "WARNING",
            Self::Error => "ERROR",
            Self::Critical => "CRITICAL",
        }
    }

    pub fn should_log(&self) -> bool {
        matches!(self, Self::Warning | Self::Error | Self::Critical)
    }

    pub fn should_alert(&self) -> bool {
        matches!(self, Self::Error | Self::Critical)
    }
}

pub struct ErrorHandler;

impl ErrorHandler {
    pub fn handle_error(error: &AppError, context: &str) {
        let api_error = error.to_api_error();

        if api_error.severity.should_log() {
            log::error!(
                "{} - 错误代码: {}, 消息: {}, 上下文: {}",
                api_error.severity.name(),
                api_error.code.name(),
                api_error.message,
                context
            );
        }
    }

    pub fn to_user_message(error: &AppError) -> String {
        match error {
            AppError::Io(_) => "文件操作失败，请检查文件权限和路径".to_string(),
            AppError::Database(_) => "数据库操作失败，请稍后重试".to_string(),
            AppError::Json(_) => "数据格式错误".to_string(),
            AppError::Crypto(msg) => format!("加密错误: {}", msg),
            AppError::Compression(msg) => format!("压缩错误: {}", msg),
            AppError::File(msg) => format!("文件错误: {}", msg),
            AppError::System(msg) => format!("系统错误: {}", msg),
            AppError::Validation(msg) => format!("验证错误: {}", msg),
            AppError::Config(msg) => format!("配置错误: {}", msg),
            AppError::Network(msg) => format!("网络错误: {}", msg),
            AppError::Permission(msg) => format!("权限错误: {}", msg),
            AppError::NotFound(msg) => format!("未找到: {}", msg),
            AppError::AlreadyExists(msg) => format!("已存在: {}", msg),
            AppError::InvalidArgument(msg) => format!("无效参数: {}", msg),
            AppError::Timeout(msg) => format!("操作超时: {}", msg),
            AppError::Cancelled => "操作被取消".to_string(),
            AppError::Task(msg) => format!("任务错误: {}", msg),
            AppError::Unknown(msg) => format!("未知错误: {}", msg),
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
