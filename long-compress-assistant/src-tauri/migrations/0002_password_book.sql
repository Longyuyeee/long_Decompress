-- 密码本数据库扩展迁移
-- 添加密码本相关的完整表结构

-- 启用外键约束
PRAGMA foreign_keys = ON;

-- 1. 密码密钥表（用于加密存储密码）
CREATE TABLE IF NOT EXISTS password_keys (
    id TEXT PRIMARY KEY,
    key_type TEXT NOT NULL, -- 'master', 'session', 'backup', 'recovery'
    algorithm TEXT NOT NULL, -- 'AES256GCM', 'ChaCha20Poly1305'
    key_data TEXT NOT NULL, -- 加密存储的密钥数据
    key_hash TEXT NOT NULL, -- 密钥哈希用于验证
    key_size INTEGER NOT NULL, -- 密钥大小（位）
    key_version INTEGER NOT NULL DEFAULT 1,
    derived_from TEXT, -- 父密钥ID
    max_usage_count INTEGER, -- 最大使用次数
    usage_count INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL,
    last_used_at DATETIME,
    expires_at DATETIME,
    rotated_at DATETIME, -- 密钥轮换时间
    rotated_to TEXT, -- 轮换到的新密钥ID
    active BOOLEAN NOT NULL DEFAULT TRUE,
    archived BOOLEAN NOT NULL DEFAULT FALSE,
    metadata TEXT NOT NULL DEFAULT '{}' -- JSON元数据
);

-- 2. 更新密码条目表，添加加密相关字段
ALTER TABLE password_entries ADD COLUMN key_id TEXT REFERENCES password_keys(id);
ALTER TABLE password_entries ADD COLUMN encryption_algorithm TEXT NOT NULL DEFAULT 'AES256GCM';
ALTER TABLE password_entries ADD COLUMN encryption_version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE password_entries ADD COLUMN archived BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE password_entries ADD COLUMN deleted BOOLEAN NOT NULL DEFAULT FALSE;

-- 3. 密码组条目关联表（多对多关系）
CREATE TABLE IF NOT EXISTS password_group_entries (
    id TEXT PRIMARY KEY,
    group_id TEXT NOT NULL REFERENCES password_groups(id) ON DELETE CASCADE,
    entry_id TEXT NOT NULL REFERENCES password_entries(id) ON DELETE CASCADE,
    added_at DATETIME NOT NULL,
    added_by TEXT, -- 添加者用户ID
    UNIQUE(group_id, entry_id) -- 防止重复添加
);

-- 4. 密码审计表
CREATE TABLE IF NOT EXISTS password_audits (
    id TEXT PRIMARY KEY,
    entry_id TEXT NOT NULL REFERENCES password_entries(id) ON DELETE CASCADE,
    audit_type TEXT NOT NULL, -- 'strength', 'reuse', 'breach', 'expiry'
    severity TEXT NOT NULL, -- 'low', 'medium', 'high', 'critical'
    score INTEGER NOT NULL, -- 0-100分
    issues TEXT NOT NULL DEFAULT '[]', -- JSON问题列表
    recommendations TEXT NOT NULL DEFAULT '[]', -- JSON建议列表
    auditor TEXT, -- 审计者
    audit_date DATETIME NOT NULL,
    next_audit_date DATETIME -- 下次审计时间
);

-- 5. 密码使用历史表
CREATE TABLE IF NOT EXISTS password_usage_history (
    id TEXT PRIMARY KEY,
    entry_id TEXT NOT NULL REFERENCES password_entries(id) ON DELETE CASCADE,
    user_id TEXT, -- 用户ID
    action TEXT NOT NULL, -- 'view', 'copy', 'auto-fill', 'export'
    ip_address TEXT, -- IP地址
    user_agent TEXT, -- 用户代理
    device_info TEXT, -- 设备信息
    used_at DATETIME NOT NULL
);

-- 6. 密码策略表
CREATE TABLE IF NOT EXISTS password_policies (
    id TEXT PRIMARY KEY,
    policy_name TEXT NOT NULL,
    policy_type TEXT NOT NULL, -- 'global', 'category', 'group', 'custom'
    description TEXT,
    min_length INTEGER, -- 最小长度
    require_uppercase BOOLEAN, -- 需要大写字母
    require_lowercase BOOLEAN, -- 需要小写字母
    require_numbers BOOLEAN, -- 需要数字
    require_symbols BOOLEAN, -- 需要符号
    max_age_days INTEGER, -- 最大使用天数
    warn_before_days INTEGER, -- 提前警告天数
    prevent_reuse_count INTEGER, -- 防止重复使用次数
    prevent_similarity BOOLEAN, -- 防止相似密码
    apply_to_categories TEXT NOT NULL DEFAULT '[]', -- JSON分类列表
    apply_to_groups TEXT NOT NULL DEFAULT '[]', -- JSON组列表
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL
);

-- 7. 密码导入导出记录表
CREATE TABLE IF NOT EXISTS password_import_exports (
    id TEXT PRIMARY KEY,
    operation_type TEXT NOT NULL, -- 'import', 'export'
    format TEXT NOT NULL, -- 'json', 'csv', 'xml', 'keepass', 'lastpass', 'bitwarden'
    file_name TEXT,
    file_size INTEGER,
    encrypted BOOLEAN NOT NULL DEFAULT FALSE,
    encryption_algorithm TEXT,
    entry_count INTEGER, -- 条目数量
    success_count INTEGER, -- 成功数量
    failed_count INTEGER, -- 失败数量
    status TEXT NOT NULL, -- 'pending', 'processing', 'completed', 'failed'
    error_message TEXT,
    started_at DATETIME,
    completed_at DATETIME,
    created_at DATETIME NOT NULL
);

-- 8. 密码分类表（扩展分类信息）
CREATE TABLE IF NOT EXISTS password_categories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    description TEXT,
    icon TEXT,
    color TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL
);

-- 9. 密码标签表
CREATE TABLE IF NOT EXISTS password_tags (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    color TEXT,
    description TEXT,
    usage_count INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL
);

-- 10. 密码条目标签关联表
CREATE TABLE IF NOT EXISTS password_entry_tags (
    id TEXT PRIMARY KEY,
    entry_id TEXT NOT NULL REFERENCES password_entries(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES password_tags(id) ON DELETE CASCADE,
    added_at DATETIME NOT NULL,
    UNIQUE(entry_id, tag_id)
);

-- 创建索引以提高查询性能

-- 密码密钥索引
CREATE INDEX IF NOT EXISTS idx_password_keys_type ON password_keys(key_type);
CREATE INDEX IF NOT EXISTS idx_password_keys_active ON password_keys(active);
CREATE INDEX IF NOT EXISTS idx_password_keys_expires ON password_keys(expires_at);

-- 密码条目索引
CREATE INDEX IF NOT EXISTS idx_password_entries_key_id ON password_entries(key_id);
CREATE INDEX IF NOT EXISTS idx_password_entries_encryption ON password_entries(encryption_algorithm);
CREATE INDEX IF NOT EXISTS idx_password_entries_archived ON password_entries(archived);
CREATE INDEX IF NOT EXISTS idx_password_entries_deleted ON password_entries(deleted);

-- 密码组条目索引
CREATE INDEX IF NOT EXISTS idx_password_group_entries_group ON password_group_entries(group_id);
CREATE INDEX IF NOT EXISTS idx_password_group_entries_entry ON password_group_entries(entry_id);

-- 密码审计索引
CREATE INDEX IF NOT EXISTS idx_password_audits_entry ON password_audits(entry_id);
CREATE INDEX IF NOT EXISTS idx_password_audits_type ON password_audits(audit_type);
CREATE INDEX IF NOT EXISTS idx_password_audits_severity ON password_audits(severity);
CREATE INDEX IF NOT EXISTS idx_password_audits_date ON password_audits(audit_date);

-- 密码使用历史索引
CREATE INDEX IF NOT EXISTS idx_password_usage_history_entry ON password_usage_history(entry_id);
CREATE INDEX IF NOT EXISTS idx_password_usage_history_action ON password_usage_history(action);
CREATE INDEX IF NOT EXISTS idx_password_usage_history_used ON password_usage_history(used_at);

-- 密码策略索引
CREATE INDEX IF NOT EXISTS idx_password_policies_type ON password_policies(policy_type);
CREATE INDEX IF NOT EXISTS idx_password_policies_enabled ON password_policies(enabled);

-- 密码导入导出索引
CREATE INDEX IF NOT EXISTS idx_password_import_exports_type ON password_import_exports(operation_type);
CREATE INDEX IF NOT EXISTS idx_password_import_exports_status ON password_import_exports(status);
CREATE INDEX IF NOT EXISTS idx_password_import_exports_created ON password_import_exports(created_at);

-- 密码分类索引
CREATE INDEX IF NOT EXISTS idx_password_categories_name ON password_categories(name);
CREATE INDEX IF NOT EXISTS idx_password_categories_order ON password_categories(sort_order);

-- 密码标签索引
CREATE INDEX IF NOT EXISTS idx_password_tags_name ON password_tags(name);
CREATE INDEX IF NOT EXISTS idx_password_tags_usage ON password_tags(usage_count);

-- 密码条目标签索引
CREATE INDEX IF NOT EXISTS idx_password_entry_tags_entry ON password_entry_tags(entry_id);
CREATE INDEX IF NOT EXISTS idx_password_entry_tags_tag ON password_entry_tags(tag_id);

-- 插入默认密码分类
INSERT OR IGNORE INTO password_categories (id, name, display_name, description, icon, color, sort_order, created_at, updated_at) VALUES
('personal', 'Personal', '个人', '个人使用的密码', '👤', '#4CAF50', 1, datetime('now'), datetime('now')),
('work', 'Work', '工作', '工作相关的密码', '💼', '#2196F3', 2, datetime('now'), datetime('now')),
('finance', 'Finance', '金融', '银行和金融账户', '💰', '#FF9800', 3, datetime('now'), datetime('now')),
('social', 'Social', '社交', '社交媒体账户', '👥', '#E91E63', 4, datetime('now'), datetime('now')),
('shopping', 'Shopping', '购物', '购物网站账户', '🛒', '#9C27B0', 5, datetime('now'), datetime('now')),
('entertainment', 'Entertainment', '娱乐', '娱乐和流媒体', '🎮', '#00BCD4', 6, datetime('now'), datetime('now')),
('education', 'Education', '教育', '教育平台账户', '📚', '#3F51B5', 7, datetime('now'), datetime('now')),
('travel', 'Travel', '旅行', '旅行和交通账户', '✈️', '#009688', 8, datetime('now'), datetime('now')),
('health', 'Health', '健康', '医疗和健康账户', '🏥', '#F44336', 9, datetime('now'), datetime('now')),
('other', 'Other', '其他', '其他类别', '📁', '#9E9E9E', 10, datetime('now'), datetime('now'));

-- 插入默认密码策略
INSERT OR IGNORE INTO password_policies (id, policy_name, policy_type, description, min_length, require_uppercase, require_lowercase, require_numbers, require_symbols, max_age_days, warn_before_days, prevent_reuse_count, prevent_similarity, apply_to_categories, apply_to_groups, enabled, created_at, updated_at) VALUES
('global_default', '全局默认策略', 'global', '适用于所有密码的默认策略', 12, TRUE, TRUE, TRUE, TRUE, 90, 7, 5, TRUE, '["all"]', '["all"]', TRUE, datetime('now'), datetime('now')),
('financial_strict', '金融账户严格策略', 'category', '金融账户的严格密码策略', 16, TRUE, TRUE, TRUE, TRUE, 60, 14, 10, TRUE, '["finance"]', '[]', TRUE, datetime('now'), datetime('now')),
('work_standard', '工作账户标准策略', 'category', '工作账户的标准密码策略', 14, TRUE, TRUE, TRUE, TRUE, 120, 7, 8, TRUE, '["work"]', '[]', TRUE, datetime('now'), datetime('now'));