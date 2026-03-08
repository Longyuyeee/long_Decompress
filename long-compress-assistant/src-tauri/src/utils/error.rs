use thiserror::Error;
use serde::{Deserialize, Serialize};

#[derive(Debug, Error)]
pub enum AppError {
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),

    #[error("JSON错误: {0}")]
    Json(#[from] serde_json::Error),

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

    #[error("未知错误: {0}")]
    Unknown(String),
}

impl AppError {
    pub fn crypto(msg: impl Into<String>) -> Self {
        Self::Crypto(msg.into())
    }

    pub fn compression(msg: impl Into<String>) -> Self {
        Self::Compression(msg.into())
    }

    pub fn file(msg: impl Into<String>) -> Self {
        Self::File(msg.into())
    }

    pub fn system(msg: impl Into<String>) -> Self {
        Self::System(msg.into())
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    pub fn network(msg: impl Into<String>) -> Self {
        Self::Network(msg.into())
    }

    pub fn permission(msg: impl Into<String>) -> Self {
        Self::Permission(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn already_exists(msg: impl Into<String>) -> Self {
        Self::AlreadyExists(msg.into())
    }

    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        Self::InvalidArgument(msg.into())
    }

    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::Timeout(msg.into())
    }

    pub fn unknown(msg: impl Into<String>) -> Self {
        Self::Unknown(msg.into())
    }

    pub fn to_api_error(&self) -> ApiError {
        match self {
            AppError::Io(e) => ApiError::new(
                ErrorCode::IoError,
                format!("IO错误: {}", e),
                ErrorSeverity::Error,
            ),
            AppError::Database(e) => ApiError::new(
                ErrorCode::DatabaseError,
                format!("数据库错误: {}", e),
                ErrorSeverity::Error,
            ),
            AppError::Json(e) => ApiError::new(
                ErrorCode::JsonError,
                format!("JSON错误: {}", e),
                ErrorSeverity::Error,
            ),
            AppError::Crypto(msg) => ApiError::new(
                ErrorCode::CryptoError,
                msg.clone(),
                ErrorSeverity::Error,
            ),
            AppError::Compression(msg) => ApiError::new(
                ErrorCode::CompressionError,
                msg.clone(),
                ErrorSeverity::Error,
            ),
            AppError::File(msg) => ApiError::new(
                ErrorCode::FileError,
                msg.clone(),
                ErrorSeverity::Error,
            ),
            AppError::System(msg) => ApiError::new(
                ErrorCode::SystemError,
                msg.clone(),
                ErrorSeverity::Error,
            ),
            AppError::Validation(msg) => ApiError::new(
                ErrorCode::ValidationError,
                msg.clone(),
                ErrorSeverity::Warning,
            ),
            AppError::Config(msg) => ApiError::new(
                ErrorCode::ConfigError,
                msg.clone(),
                ErrorSeverity::Error,
            ),
            AppError::Network(msg) => ApiError::new(
                ErrorCode::NetworkError,
                msg.clone(),
                ErrorSeverity::Error,
            ),
            AppError::Permission(msg) => ApiError::new(
                ErrorCode::PermissionError,
                msg.clone(),
                ErrorSeverity::Error,
            ),
            AppError::NotFound(msg) => ApiError::new(
                ErrorCode::NotFound,
                msg.clone(),
                ErrorSeverity::Warning,
            ),
            AppError::AlreadyExists(msg) => ApiError::new(
                ErrorCode::AlreadyExists,
                msg.clone(),
                ErrorSeverity::Warning,
            ),
            AppError::InvalidArgument(msg) => ApiError::new(
                ErrorCode::InvalidArgument,
                msg.clone(),
                ErrorSeverity::Warning,
            ),
            AppError::Timeout(msg) => ApiError::new(
                ErrorCode::Timeout,
                msg.clone(),
                ErrorSeverity::Error,
            ),
            AppError::Cancelled => ApiError::new(
                ErrorCode::Cancelled,
                "操作被取消".to_string(),
                ErrorSeverity::Info,
            ),
            AppError::Unknown(msg) => ApiError::new(
                ErrorCode::UnknownError,
                msg.clone(),
                ErrorSeverity::Error,
            ),
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        if let Some(io_error) = error.downcast_ref::<std::io::Error>() {
            return AppError::Io(io_error.clone());
        }
        if let Some(db_error) = error.downcast_ref::<sqlx::Error>() {
            return AppError::Database(db_error.clone());
        }
        if let Some(json_error) = error.downcast_ref::<serde_json::Error>() {
            return AppError::Json(json_error.clone());
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

    pub fn is_client_error(&self) -> bool {
        matches!(
            self.code,
            ErrorCode::ValidationError
                | ErrorCode::NotFound
                | ErrorCode::AlreadyExists
                | ErrorCode::InvalidArgument
                | ErrorCode::PermissionError
        )
    }

    pub fn is_server_error(&self) -> bool {
        matches!(
            self.code,
            ErrorCode::IoError
                | ErrorCode::DatabaseError
                | ErrorCode::SystemError
                | ErrorCode::UnknownError
        )
    }

    pub fn should_retry(&self) -> bool {
        matches!(
            self.code,
            ErrorCode::Timeout | ErrorCode::NetworkError | ErrorCode::DatabaseError
        ) && self.severity != ErrorSeverity::Critical
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ErrorCode {
    // 通用错误
    UnknownError,
    ValidationError,
    ConfigError,
    Timeout,
    Cancelled,

    // IO和文件错误
    IoError,
    FileError,
    FileNotFound,
    FileAlreadyExists,
    FilePermissionDenied,

    // 数据库错误
    DatabaseError,
    DatabaseConnectionError,
    DatabaseQueryError,
    DatabaseConstraintViolation,

    // 网络错误
    NetworkError,
    ConnectionError,
    RequestError,
    ResponseError,

    // 加密错误
    CryptoError,
    EncryptionError,
    DecryptionError,
    HashError,
    KeyError,

    // 压缩错误
    CompressionError,
    DecompressionError,
    FormatNotSupported,
    PasswordRequired,
    InvalidPassword,

    // 系统错误
    SystemError,
    MemoryError,
    ProcessError,
    ResourceExhausted,

    // 业务错误
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

    pub fn http_status(&self) -> u16 {
        match self {
            // 400 Bad Request
            Self::ValidationError
            | Self::InvalidArgument
            | Self::JsonError => 400,

            // 401 Unauthorized
            Self::AuthenticationError => 401,

            // 403 Forbidden
            Self::PermissionError
            | Self::AuthorizationError
            | Self::FilePermissionDenied => 403,

            // 404 Not Found
            Self::NotFound
            | Self::FileNotFound => 404,

            // 409 Conflict
            Self::AlreadyExists
            | Self::FileAlreadyExists
            | Self::DatabaseConstraintViolation => 409,

            // 408 Request Timeout
            Self::Timeout => 408,

            // 429 Too Many Requests
            Self::ResourceExhausted => 429,

            // 500 Internal Server Error
            Self::UnknownError
            | Self::SystemError
            | Self::DatabaseError
            | Self::DatabaseConnectionError
            | Self::DatabaseQueryError
            | Self::CryptoError
            | Self::EncryptionError
            | Self::DecryptionError
            | Self::HashError
            | Self::KeyError
            | Self::CompressionError
            | Self::DecompressionError
            | Self::MemoryError
            | Self::ProcessError => 500,

            // 503 Service Unavailable
            Self::NetworkError
            | Self::ConnectionError
            | Self::RequestError
            | Self::ResponseError => 503,

            // 默认400
            _ => 400,
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

/// 错误处理工具
pub struct ErrorHandler;

impl ErrorHandler {
    /// 处理错误并记录日志
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

            if let Some(details) = &api_error.details {
                log::debug!("错误详情: {}", details);
            }
        }

        // 如果是严重错误，可以发送警报
        if api_error.severity.should_alert() {
            // 这里可以集成警报系统
            log::warn!("需要发送警报: {}", api_error.message);
        }
    }

    /// 将错误转换为用户友好的消息
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
            AppError::Unknown(msg) => format!("未知错误: {}", msg),
        }
    }

    /// 创建验证错误
    pub fn validation_error(field: &str, message: &str) -> AppError {
        AppError::validation(format!("字段 '{}': {}", field, message))
    }

    /// 创建文件未找到错误
    pub fn file_not_found(path: &str) -> AppError {
        AppError::not_found(format!("文件未找到: {}", path))
    }

    /// 创建文件已存在错误
    pub fn file_already_exists(path: &str) -> AppError {
        AppError::already_exists(format!("文件已存在: {}", path))
    }

    /// 创建权限错误
    pub fn permission_denied(resource: &str) -> AppError {
        AppError::permission(format("访问资源被拒绝: {}", resource))
    }
}

/// 结果类型别名
pub type AppResult<T> = Result<T, AppError>;

/// API结果类型
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> ApiResult<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn error(error: ApiError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn from_result(result: AppResult<T>) -> Self {
        match result {
            Ok(data) => Self::success(data),
            Err(err) => Self::error(err.to_api_error()),
        }
    }
}

/// 错误上下文包装器
pub struct ErrorContext<'a> {
    context: &'a str,
}

impl<'a> ErrorContext<'a> {
    pub fn new(context: &'a str) -> Self {
        Self { context }
    }

    pub fn wrap<T>(&self, result: AppResult<T>) -> AppResult<T> {
        result.map_err(|err| {
            ErrorHandler::handle_error(&err, self.context);
            err
        })
    }
}

/// 宏用于简化错误处理
#[macro_export]
macro_rules! try_with_context {
    ($result:expr, $context:expr) => {
        match $result {
            Ok(val) => val,
            Err(err) => {
                let app_err: crate::utils::error::AppError = err.into();
                crate::utils::error::ErrorHandler::handle_error(&app_err, $context);
                return Err(app_err);
            }
        }
    };
}

#[macro_export]
macro_rules! ensure {
    ($condition:expr, $error:expr) => {
        if !$condition {
            return Err($error.into());
        }
    };
    ($condition:expr, $($arg:tt)*) => {
        if !$condition {
            return Err(format!($($arg)*).into());
        }
    };
}