-- 增加使用次数统计字段
ALTER TABLE password_entries ADD COLUMN use_count INTEGER NOT NULL DEFAULT 0;

-- 增加基于使用次数的索引以优化排序
CREATE INDEX IF NOT EXISTS idx_password_entries_use_count ON password_entries(use_count DESC);
