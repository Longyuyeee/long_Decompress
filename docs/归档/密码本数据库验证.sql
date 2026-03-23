-- 密码本数据库设计验证脚本
-- 此脚本用于验证数据库表结构设计

-- 1. 创建加密密钥表
CREATE TABLE IF NOT EXISTS password_keys (
    id TEXT PRIMARY KEY,
    key_type TEXT NOT NULL,
    algorithm TEXT NOT NULL,
    key_data TEXT NOT NULL,
    key_hash TEXT NOT NULL,
    key_size INTEGER NOT NULL,
    key_version INTEGER NOT NULL DEFAULT 1,
    derived_from TEXT,
    max_usage_count INTEGER,
    usage_count INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL,
    last_used_at DATETIME,
    expires_at DATETIME,
    rotated_at DATETIME,
    rotated_to TEXT,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    archived BOOLEAN NOT NULL DEFAULT FALSE,
    metadata TEXT NOT NULL DEFAULT '{}'
);

-- 2. 创建密码条目表（加密版本）
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
    key_id TEXT NOT NULL,
    encryption_algorithm TEXT NOT NULL DEFAULT 'AES256GCM',
    encryption_version INTEGER NOT NULL DEFAULT 1,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    last_used DATETIME,
    expires_at DATETIME,
    favorite BOOLEAN NOT NULL DEFAULT FALSE,
    archived BOOLEAN NOT NULL DEFAULT FALSE,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    custom_fields TEXT NOT NULL DEFAULT '[]',
    FOREIGN KEY (key_id) REFERENCES password_keys(id)
);

-- 3. 创建密码组表
CREATE TABLE IF NOT EXISTS password_groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    category TEXT NOT NULL,
    icon TEXT,
    color TEXT,
    require_master_password BOOLEAN NOT NULL DEFAULT TRUE,
    auto_lock_minutes INTEGER DEFAULT 30,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    favorite BOOLEAN NOT NULL DEFAULT FALSE,
    archived BOOLEAN NOT NULL DEFAULT FALSE
);

-- 4. 创建密码组关系表
CREATE TABLE IF NOT EXISTS password_group_entries (
    id TEXT PRIMARY KEY,
    group_id TEXT NOT NULL,
    entry_id TEXT NOT NULL,
    added_at DATETIME NOT NULL,
    added_by TEXT,
    UNIQUE (group_id, entry_id),
    FOREIGN KEY (group_id) REFERENCES password_groups(id) ON DELETE CASCADE,
    FOREIGN KEY (entry_id) REFERENCES password_entries(id) ON DELETE CASCADE
);

-- 5. 创建密码审计表
CREATE TABLE IF NOT EXISTS password_audits (
    id TEXT PRIMARY KEY,
    entry_id TEXT NOT NULL,
    audit_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    score INTEGER NOT NULL,
    issues TEXT NOT NULL DEFAULT '[]',
    recommendations TEXT NOT NULL DEFAULT '[]',
    auditor TEXT,
    audit_date DATETIME NOT NULL,
    next_audit_date DATETIME,
    FOREIGN KEY (entry_id) REFERENCES password_entries(id)
);

-- 6. 创建密码使用历史表
CREATE TABLE IF NOT EXISTS password_usage_history (
    id TEXT PRIMARY KEY,
    entry_id TEXT NOT NULL,
    user_id TEXT,
    action TEXT NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    device_info TEXT,
    used_at DATETIME NOT NULL,
    FOREIGN KEY (entry_id) REFERENCES password_entries(id)
);

-- 7. 创建密码策略表
CREATE TABLE IF NOT EXISTS password_policies (
    id TEXT PRIMARY KEY,
    policy_name TEXT NOT NULL UNIQUE,
    policy_type TEXT NOT NULL,
    description TEXT,
    min_length INTEGER DEFAULT 8,
    require_uppercase BOOLEAN DEFAULT TRUE,
    require_lowercase BOOLEAN DEFAULT TRUE,
    require_numbers BOOLEAN DEFAULT TRUE,
    require_symbols BOOLEAN DEFAULT TRUE,
    max_age_days INTEGER DEFAULT 90,
    warn_before_days INTEGER DEFAULT 7,
    prevent_reuse_count INTEGER DEFAULT 5,
    prevent_similarity BOOLEAN DEFAULT TRUE,
    apply_to_categories TEXT DEFAULT '[]',
    apply_to_groups TEXT DEFAULT '[]',
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL
);

-- 8. 创建导入导出记录表
CREATE TABLE IF NOT EXISTS password_import_export (
    id TEXT PRIMARY KEY,
    operation_type TEXT NOT NULL,
    format TEXT NOT NULL,
    file_name TEXT,
    file_size INTEGER,
    encrypted BOOLEAN NOT NULL DEFAULT TRUE,
    encryption_algorithm TEXT,
    entry_count INTEGER,
    success_count INTEGER,
    failed_count INTEGER,
    status TEXT NOT NULL,
    error_message TEXT,
    started_at DATETIME,
    completed_at DATETIME,
    created_at DATETIME NOT NULL
);

-- 创建索引
-- 密码条目索引
CREATE INDEX IF NOT EXISTS idx_password_entries_category ON password_entries(category);
CREATE INDEX IF NOT EXISTS idx_password_entries_updated ON password_entries(updated_at);
CREATE INDEX IF NOT EXISTS idx_password_entries_favorite ON password_entries(favorite);
CREATE INDEX IF NOT EXISTS idx_password_entries_deleted ON password_entries(deleted);
CREATE INDEX IF NOT EXISTS idx_password_entries_search ON password_entries(name, category, deleted);
CREATE INDEX IF NOT EXISTS idx_password_entries_expiring ON password_entries(expires_at) WHERE expires_at IS NOT NULL;

-- 加密密钥索引
CREATE INDEX IF NOT EXISTS idx_password_keys_type ON password_keys(key_type);
CREATE INDEX IF NOT EXISTS idx_password_keys_active ON password_keys(active);
CREATE INDEX IF NOT EXISTS idx_password_keys_expires ON password_keys(expires_at);

-- 密码组索引
CREATE INDEX IF NOT EXISTS idx_password_groups_category ON password_groups(category);
CREATE INDEX IF NOT EXISTS idx_password_groups_updated ON password_groups(updated_at);

-- 密码组关系索引
CREATE INDEX IF NOT EXISTS idx_password_group_entries_group ON password_group_entries(group_id);
CREATE INDEX IF NOT EXISTS idx_password_group_entries_entry ON password_group_entries(entry_id);
CREATE INDEX IF NOT EXISTS idx_password_group_entries_composite ON password_group_entries(group_id, entry_id, added_at);

-- 密码审计索引
CREATE INDEX IF NOT EXISTS idx_password_audits_entry ON password_audits(entry_id);
CREATE INDEX IF NOT EXISTS idx_password_audits_date ON password_audits(audit_date);
CREATE INDEX IF NOT EXISTS idx_password_audits_severity ON password_audits(severity);

-- 密码使用历史索引
CREATE INDEX IF NOT EXISTS idx_password_usage_entry ON password_usage_history(entry_id);
CREATE INDEX IF NOT EXISTS idx_password_usage_date ON password_usage_history(used_at);
CREATE INDEX IF NOT EXISTS idx_password_usage_action ON password_usage_history(action);

-- 密码策略索引
CREATE INDEX IF NOT EXISTS idx_password_policies_enabled ON password_policies(enabled);
CREATE INDEX IF NOT EXISTS idx_password_policies_type ON password_policies(policy_type);

-- 导入导出记录索引
CREATE INDEX IF NOT EXISTS idx_password_import_export_type ON password_import_export(operation_type);
CREATE INDEX IF NOT EXISTS idx_password_import_export_status ON password_import_export(status);
CREATE INDEX IF NOT EXISTS idx_password_import_export_date ON password_import_export(created_at);

-- 验证表结构
SELECT 'password_keys' as table_name, COUNT(*) as column_count FROM pragma_table_info('password_keys')
UNION ALL
SELECT 'password_entries', COUNT(*) FROM pragma_table_info('password_entries')
UNION ALL
SELECT 'password_groups', COUNT(*) FROM pragma_table_info('password_groups')
UNION ALL
SELECT 'password_group_entries', COUNT(*) FROM pragma_table_info('password_group_entries')
UNION ALL
SELECT 'password_audits', COUNT(*) FROM pragma_table_info('password_audits')
UNION ALL
SELECT 'password_usage_history', COUNT(*) FROM pragma_table_info('password_usage_history')
UNION ALL
SELECT 'password_policies', COUNT(*) FROM pragma_table_info('password_policies')
UNION ALL
SELECT 'password_import_export', COUNT(*) FROM pragma_table_info('password_import_export');

-- 验证外键约束
SELECT 'Foreign Key Check' as check_type,
       CASE WHEN COUNT(*) = 0 THEN 'PASS' ELSE 'FAIL' END as result
FROM pragma_foreign_key_list('password_entries')
WHERE "table" = 'password_keys' AND "from" = 'key_id' AND "to" = 'id'
HAVING COUNT(*) = 1;

-- 验证索引
SELECT name as index_name, tbl_name as table_name
FROM sqlite_master
WHERE type = 'index' AND name LIKE 'idx_password_%'
ORDER BY tbl_name, name;

-- 示例数据插入（用于测试）
INSERT OR IGNORE INTO password_keys (
    id, key_type, algorithm, key_data, key_hash, key_size, key_version,
    created_at, active
) VALUES (
    'master_key_001', 'Master', 'AES256GCM',
    'encrypted_master_key_data', 'key_hash_value', 256, 1,
    datetime('now'), TRUE
);

INSERT OR IGNORE INTO password_entries (
    id, name, username, password, category, strength, key_id,
    encryption_algorithm, encryption_version, created_at, updated_at,
    favorite, archived, deleted, tags, custom_fields
) VALUES (
    'entry_001', '示例网站', 'user@example.com',
    'encrypted_password_data', 'Personal', 'Strong', 'master_key_001',
    'AES256GCM', 1, datetime('now'), datetime('now'),
    FALSE, FALSE, FALSE, '["示例","网站"]', '[]'
);

INSERT OR IGNORE INTO password_groups (
    id, name, description, category, created_at, updated_at,
    favorite, archived, require_master_password, auto_lock_minutes
) VALUES (
    'group_001', '个人密码', '个人使用的密码', 'Personal',
    datetime('now'), datetime('now'), FALSE, FALSE, TRUE, 30
);

INSERT OR IGNORE INTO password_group_entries (
    id, group_id, entry_id, added_at
) VALUES (
    'group_entry_001', 'group_001', 'entry_001', datetime('now')
);

-- 查询验证
SELECT 'Table Records' as query_type, table_name, COUNT(*) as record_count
FROM (
    SELECT 'password_keys' as table_name FROM password_keys
    UNION ALL
    SELECT 'password_entries' FROM password_entries
    UNION ALL
    SELECT 'password_groups' FROM password_groups
    UNION ALL
    SELECT 'password_group_entries' FROM password_group_entries
)
GROUP BY table_name
ORDER BY table_name;

-- 关系验证
SELECT
    'Group-Entry Relationship' as relationship,
    g.name as group_name,
    e.name as entry_name,
    ge.added_at
FROM password_groups g
JOIN password_group_entries ge ON g.id = ge.group_id
JOIN password_entries e ON ge.entry_id = e.id;

-- 加密信息验证
SELECT
    'Encryption Info' as info_type,
    e.name,
    e.encryption_algorithm,
    e.encryption_version,
    k.key_type,
    k.algorithm as key_algorithm
FROM password_entries e
JOIN password_keys k ON e.key_id = k.id;

-- 清理测试数据（可选）
-- DELETE FROM password_group_entries;
-- DELETE FROM password_entries;
-- DELETE FROM password_groups;
-- DELETE FROM password_keys;

PRAGMA foreign_keys = ON;
SELECT 'Database design validation completed successfully' as status;