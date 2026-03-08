# 数据库模块

胧压缩·方便助手的数据库模块，提供数据库连接、迁移、管理和维护功能。

## 模块结构

```
database/
├── mod.rs          # 模块导出
├── README.md       # 本文档
├── config.rs       # 数据库配置
├── connection.rs   # 数据库连接和连接池管理
├── models.rs       # 数据库模型定义
├── repositories.rs # 数据访问层（仓库模式）
├── migrations.rs   # 数据库迁移
├── management.rs   # 数据库管理和维护
├── commands.rs     # Tauri命令接口
└── tests.rs        # 单元测试
```

## 功能特性

### 1. 数据库连接管理
- 连接池配置和管理
- 自动重连机制
- 连接指标监控
- 事务支持

### 2. 配置管理
- 数据库配置（路径、密码、连接池参数等）
- 性能配置（WAL模式、缓存大小、页面大小等）
- 备份配置（自动备份、备份间隔、保留数量等）
- 环境变量支持

### 3. 数据库迁移
- 版本化迁移管理
- 表结构初始化
- 索引创建
- 默认数据插入

### 4. 数据访问层
- 仓库模式实现
- 类型安全的查询
- 错误处理
- 事务支持

### 5. 数据库维护
- 完整性检查和修复
- 性能优化
- 自动备份
- 旧数据清理
- 健康检查

### 6. 监控和诊断
- 连接指标监控
- 性能统计
- 健康报告
- 建议和推荐

## 使用示例

### 初始化数据库
```rust
use crate::database::connection;

// 初始化全局数据库连接
connection::init().await?;

// 获取数据库连接
let conn = connection::get_connection().await?;
```

### 执行查询
```rust
use crate::database::repositories::CompressionTaskRepository;

let repo = CompressionTaskRepository::new(conn.pool().clone());

// 创建任务
let task = CompressionTaskDb { ... };
repo.create(&task).await?;

// 查询任务
let tasks = repo.list(Some(10), None).await?;
```

### 使用事务
```rust
use crate::database::connection::DatabaseConnection;

let result = conn.execute_transaction(|tx| {
    Box::pin(async move {
        // 执行多个数据库操作
        sqlx::query("INSERT INTO table1 ...").execute(tx).await?;
        sqlx::query("INSERT INTO table2 ...").execute(tx).await?;
        Ok::<(), sqlx::Error>(())
    })
}).await?;
```

### 数据库维护
```rust
use crate::database::management::DatabaseManager;

let manager = DatabaseManager::new(conn);

// 执行维护
let report = manager.perform_maintenance().await?;

// 获取健康报告
let health_report = manager.get_health_report().await?;
```

## Tauri命令

数据库模块提供了以下Tauri命令：

### 状态和监控
- `get_database_status()` - 获取数据库状态
- `get_database_health_report()` - 获取健康报告
- `check_database_connection()` - 检查数据库连接

### 备份和恢复
- `backup_database(request)` - 备份数据库
- `restore_database(request)` - 恢复数据库
- `export_database(path)` - 导出数据库

### 维护和优化
- `optimize_database(request)` - 优化数据库
- `perform_database_maintenance()` - 执行数据库维护
- `reinitialize_database()` - 重新初始化数据库

### 配置管理
- `get_database_config()` - 获取数据库配置

## 配置说明

### 环境变量
- `DATABASE_PATH` - 数据库文件路径
- `DATABASE_PASSWORD` - 数据库密码（暂未实现加密）
- `DATABASE_MAX_CONNECTIONS` - 最大连接数（默认：10）
- `DATABASE_WAL_MODE` - 是否启用WAL模式（默认：true）
- `DATABASE_AUTO_BACKUP` - 是否启用自动备份（默认：true）

### 配置文件
数据库配置可以从JSON文件加载：
```json
{
  "path": "./data/database.sqlite",
  "password": null,
  "pool_config": {
    "max_connections": 10,
    "min_connections": 2,
    "connect_timeout": 30,
    "idle_timeout": 600,
    "max_lifetime": 1800,
    "acquire_timeout": 30
  },
  "performance_config": {
    "wal_mode": true,
    "synchronous": 1,
    "cache_size": -2000,
    "page_size": 4096,
    "foreign_keys": true,
    "auto_vacuum": true,
    "mmap_size": 134217728,
    "journal_mode": "WAL"
  },
  "backup_config": {
    "auto_backup": true,
    "backup_interval_hours": 24,
    "retain_backup_count": 7,
    "backup_dir": "./data/backups",
    "compress_backup": true
  }
}
```

## 表结构

数据库包含以下表：

### 核心表
1. `compression_tasks` - 压缩任务表
2. `compression_history` - 压缩历史表
3. `password_entries` - 密码条目表
4. `password_groups` - 密码组表

### 系统表
5. `file_operations` - 文件操作表
6. `system_metrics` - 系统指标表
7. `system_alerts` - 系统警报表
8. `application_settings` - 应用设置表

### 安全和审计表
9. `user_sessions` - 用户会话表
10. `audit_logs` - 审计日志表

### 内部表
11. `schema_migrations` - 迁移版本表（自动创建）

## 性能优化

### 连接池优化
- 合理设置最小和最大连接数
- 配置连接超时和生存时间
- 监控连接使用情况

### SQLite优化
- 启用WAL模式提高并发性能
- 配置适当的缓存大小
- 使用合适的页面大小
- 定期执行VACUUM和ANALYZE

### 索引优化
- 为常用查询字段创建索引
- 定期重建索引
- 监控索引使用情况

## 备份策略

### 自动备份
- 每天自动备份一次
- 保留最近7天的备份
- 备份文件包含时间戳

### 手动备份
- 支持手动备份到指定路径
- 备份前执行VACUUM确保数据一致性
- 验证备份文件完整性

### 恢复机制
- 恢复前自动创建回滚点
- 验证备份文件有效性
- 恢复后检查数据库完整性

## 错误处理

数据库模块使用自定义错误类型`DatabaseError`，包含以下错误变体：

- `ConnectionFailed` - 连接失败
- `InitializationFailed` - 初始化失败
- `MigrationFailed` - 迁移失败
- `BackupFailed` - 备份失败
- `RestoreFailed` - 恢复失败
- `IntegrityCheckFailed` - 完整性检查失败
- `TransactionFailed` - 事务操作失败
- `QueryExecutionFailed` - 查询执行失败
- `PoolExhausted` - 连接池耗尽
- `ConnectionTimeout` - 连接超时
- `FileNotFound` - 文件不存在
- `PermissionDenied` - 权限不足
- `ConfigurationError` - 配置错误

## 测试

运行数据库测试：
```bash
cargo test --package long-compress-assistant --lib database::tests
```

测试包括：
- 数据库创建和连接
- 迁移执行
- 事务处理
- 备份和恢复
- 完整性检查
- 配置验证

## 注意事项

1. **加密支持**：当前版本暂未实现数据库加密，密码参数被忽略
2. **并发访问**：使用连接池和WAL模式支持并发访问
3. **数据一致性**：使用事务确保数据一致性
4. **性能监控**：定期监控数据库性能和健康状况
5. **备份策略**：确保有有效的备份和恢复策略

## 未来改进

1. 数据库加密支持
2. 更完善的迁移版本管理
3. 数据库复制和同步
4. 更详细的性能监控
5. 自动化测试覆盖