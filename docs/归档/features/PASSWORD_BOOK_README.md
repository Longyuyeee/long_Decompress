# 密码本功能文档

## 概述
密码本功能是胧解压项目的密码管理系统，提供安全的密码存储、加密、查询和管理功能。

## 功能特性

### 1. 密码数据库设计 (PW-001)
- 完整的密码本数据库表结构
- 支持密码加密存储
- 多表关联设计（密码条目、密钥、分类、标签、审计等）
- 索引优化查询性能

### 2. 密码添加功能 (PW-002)
- 密码强度验证和评分
- AES-256-GCM加密存储
- 支持自定义字段和标签
- 分类管理（个人、工作、金融、社交等）
- 过期时间设置

### 3. 密码查询功能 (PW-003)
- 全文搜索（名称、用户名、URL、备注、标签）
- 高级过滤（分类、强度、标签、收藏状态等）
- 多种排序选项
- 分页支持
- 统计信息获取

## 数据库表结构

### 主要表
1. `password_entries` - 密码条目表
2. `password_keys` - 加密密钥表
3. `password_categories` - 密码分类表
4. `password_tags` - 密码标签表
5. `password_audits` - 密码审计表
6. `password_usage_history` - 使用历史表
7. `password_policies` - 密码策略表

## API接口

### 密码管理
- `add_password` - 添加密码
- `update_password` - 更新密码
- `get_password` - 获取密码（可选包含解密密码）
- `delete_password` - 删除密码（软删除）
- `archive_password` - 归档密码

### 密码查询
- `search_passwords` - 搜索密码（支持高级过滤）
- `get_all_passwords` - 获取所有密码
- `get_favorite_passwords` - 获取收藏的密码
- `get_recently_used_passwords` - 获取最近使用的密码
- `get_expiring_passwords` - 获取即将过期的密码

### 工具函数
- `validate_password` - 验证密码强度
- `get_password_statistics` - 获取密码统计信息

## 安全特性

### 加密
- 使用AES-256-GCM加密算法
- 每个密码独立加密
- 支持主密钥和会话密钥
- 密钥派生使用Argon2id

### 验证
- 密码强度评分（0-10分）
- 常见密码检测
- 序列和重复字符检测
- 字符类型要求检查

### 审计
- 使用历史记录
- 密码审计日志
- 完整性检查

## 使用示例

### 添加密码
```rust
let request = AddPasswordRequest {
    name: "GitHub".to_string(),
    username: Some("user@example.com".to_string()),
    password: "StrongP@ssw0rd123!".to_string(),
    url: Some("https://github.com".to_string()),
    notes: Some("工作账户".to_string()),
    tags: vec!["work".to_string(), "code".to_string()],
    category: PasswordCategory::Work,
    expires_at: Some(Utc::now() + Duration::days(90)),
    custom_fields: vec![
        CustomField {
            name: "安全提示".to_string(),
            value: "我的宠物名字".to_string(),
            field_type: CustomFieldType::Text,
            sensitive: false,
        }
    ],
};

let entry = service.add_password(request).await?;
```

### 搜索密码
```rust
let filters = SearchFilters {
    categories: vec![PasswordCategory::Work, PasswordCategory::Finance],
    strengths: vec![PasswordStrength::Strong, PasswordStrength::VeryStrong],
    tags: vec!["work".to_string()],
    favorite: Some(true),
    archived: Some(false),
    sort_by: SortOption::UpdatedAtDesc,
    limit: Some(20),
    offset: Some(0),
};

let results = service.search_passwords("github", Some(filters)).await?;
```

### 验证密码强度
```rust
let validation = service.validate_password("MyP@ssw0rd").await?;
if validation.is_valid {
    println!("密码强度: {:?}, 分数: {}", validation.strength, validation.score);
} else {
    println!("密码问题: {:?}", validation.issues);
}
```

## 配置说明

### 数据库迁移
密码本功能需要运行数据库迁移：
```sql
-- 初始迁移已包含在0001_initial.sql中
-- 密码本扩展迁移在0002_password_book.sql中
```

### 加密配置
- 默认使用AES-256-GCM加密
- 密钥大小：256位
- Nonce大小：96位
- 密钥派生：Argon2id

## 性能考虑

### 索引优化
- 为常用查询字段创建索引
- 分类、强度、收藏状态等字段已索引
- 更新时间字段索引用于排序

### 查询优化
- 使用参数化查询防止SQL注入
- 分页支持避免大量数据加载
- 懒加载加密密码

## 测试

运行测试：
```bash
cargo test --package long-compress-assistant --lib services::password_book_test
```

## 注意事项

1. **安全存储**：密码始终以加密形式存储，仅在需要时解密
2. **密钥管理**：妥善保管主密钥，定期轮换会话密钥
3. **审计日志**：重要操作记录审计日志
4. **密码策略**：建议配置密码策略要求
5. **定期备份**：定期备份密码数据库

## 后续开发计划

1. PW-004: 密码尝试逻辑
2. PW-005: 密码分类管理
3. PW-006: 密码生成功能
4. PW-007: 密码强度评估增强
5. PW-008: 加密模块优化（已完成）
6. PW-009: 模型模块定义（已完成）