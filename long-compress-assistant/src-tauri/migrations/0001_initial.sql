-- 启用外键约束
PRAGMA foreign_keys = ON;

-- 创建压缩任务表
CREATE TABLE IF NOT EXISTS compression_tasks (
    id TEXT PRIMARY KEY,
    source_files TEXT NOT NULL,
    output_path TEXT NOT NULL,
    format TEXT NOT NULL,
    options TEXT NOT NULL,
    status TEXT NOT NULL,
    progress REAL NOT NULL DEFAULT 0.0,
    created_at DATETIME NOT NULL,
    started_at DATETIME,
    completed_at DATETIME,
    error_message TEXT,
    total_size INTEGER NOT NULL DEFAULT 0,
    processed_size INTEGER NOT NULL DEFAULT 0
);

-- 创建压缩历史表
CREATE TABLE IF NOT EXISTS compression_history (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    operation_type TEXT NOT NULL,
    source_paths TEXT NOT NULL,
    output_path TEXT NOT NULL,
    format TEXT NOT NULL,
    size_before INTEGER NOT NULL,
    size_after INTEGER NOT NULL,
    compression_ratio REAL NOT NULL,
    duration_seconds REAL NOT NULL,
    created_at DATETIME NOT NULL,
    success BOOLEAN NOT NULL,
    error_message TEXT,
    FOREIGN KEY (task_id) REFERENCES compression_tasks(id)
);

-- 创建密码条目表
CREATE TABLE IF NOT EXISTS password_entries (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    username TEXT,
    password TEXT NOT NULL,
    url TEXT,
    notes TEXT,
    tags TEXT NOT NULL DEFAULT '[]',
    category TEXT NOT NULL,
    strength TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    last_used DATETIME,
    expires_at DATETIME,
    favorite BOOLEAN NOT NULL DEFAULT FALSE,
    custom_fields TEXT NOT NULL DEFAULT '[]'
);

-- 创建密码组表
CREATE TABLE IF NOT EXISTS password_groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    category TEXT NOT NULL,
    entry_ids TEXT NOT NULL DEFAULT '[]',
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL
);

-- 创建文件操作表
CREATE TABLE IF NOT EXISTS file_operations (
    id TEXT PRIMARY KEY,
    operation_type TEXT NOT NULL,
    source_paths TEXT NOT NULL,
    destination_path TEXT,
    status TEXT NOT NULL,
    progress REAL NOT NULL DEFAULT 0.0,
    total_size INTEGER NOT NULL DEFAULT 0,
    processed_size INTEGER NOT NULL DEFAULT 0,
    started_at DATETIME,
    completed_at DATETIME,
    error_message TEXT
);

-- 创建系统指标表
CREATE TABLE IF NOT EXISTS system_metrics (
    id TEXT PRIMARY KEY,
    timestamp DATETIME NOT NULL,
    cpu_usage REAL NOT NULL,
    memory_usage INTEGER NOT NULL,
    disk_io_read INTEGER NOT NULL,
    disk_io_write INTEGER NOT NULL,
    network_io_received INTEGER NOT NULL,
    network_io_transmitted INTEGER NOT NULL,
    process_count INTEGER NOT NULL,
    thread_count INTEGER NOT NULL
);

-- 创建系统警报表
CREATE TABLE IF NOT EXISTS system_alerts (
    id TEXT PRIMARY KEY,
    alert_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    message TEXT NOT NULL,
    timestamp DATETIME NOT NULL,
    acknowledged BOOLEAN NOT NULL DEFAULT FALSE,
    component TEXT,
    value REAL,
    threshold REAL
);

-- 创建应用设置表
CREATE TABLE IF NOT EXISTS application_settings (
    id TEXT PRIMARY KEY,
    key TEXT UNIQUE NOT NULL,
    value TEXT NOT NULL,
    category TEXT NOT NULL,
    description TEXT,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL
);

-- 创建用户会话表
CREATE TABLE IF NOT EXISTS user_sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT,
    session_token TEXT NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    created_at DATETIME NOT NULL,
    expires_at DATETIME NOT NULL,
    last_activity DATETIME NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

-- 创建审计日志表
CREATE TABLE IF NOT EXISTS audit_logs (
    id TEXT PRIMARY KEY,
    user_id TEXT,
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT,
    details TEXT NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    created_at DATETIME NOT NULL
);

-- 创建索引以提高查询性能

-- 压缩任务索引
CREATE INDEX IF NOT EXISTS idx_compression_tasks_status ON compression_tasks(status);
CREATE INDEX IF NOT EXISTS idx_compression_tasks_created ON compression_tasks(created_at);

-- 压缩历史索引
CREATE INDEX IF NOT EXISTS idx_compression_history_created ON compression_history(created_at);
CREATE INDEX IF NOT EXISTS idx_compression_history_operation ON compression_history(operation_type);

-- 密码条目索引
CREATE INDEX IF NOT EXISTS idx_password_entries_category ON password_entries(category);
CREATE INDEX IF NOT EXISTS idx_password_entries_updated ON password_entries(updated_at);
CREATE INDEX IF NOT EXISTS idx_password_entries_favorite ON password_entries(favorite);

-- 文件操作索引
CREATE INDEX IF NOT EXISTS idx_file_operations_status ON file_operations(status);

-- 系统指标索引
CREATE INDEX IF NOT EXISTS idx_system_metrics_timestamp ON system_metrics(timestamp);

-- 系统警报索引
CREATE INDEX IF NOT EXISTS idx_system_alerts_timestamp ON system_alerts(timestamp);
CREATE INDEX IF NOT EXISTS idx_system_alerts_acknowledged ON system_alerts(acknowledged);

-- 应用设置索引
CREATE INDEX IF NOT EXISTS idx_application_settings_key ON application_settings(key);
CREATE INDEX IF NOT EXISTS idx_application_settings_category ON application_settings(category);

-- 用户会话索引
CREATE INDEX IF NOT EXISTS idx_user_sessions_token ON user_sessions(session_token);
CREATE INDEX IF NOT EXISTS idx_user_sessions_expires ON user_sessions(expires_at);

-- 审计日志索引
CREATE INDEX IF NOT EXISTS idx_audit_logs_user ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created ON audit_logs(created_at);