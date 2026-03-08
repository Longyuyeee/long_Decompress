#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::path::PathBuf;

    async fn create_test_database() -> (DatabaseConnection, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let config = DatabaseConfig {
            path: db_path.clone(),
            password: None,
            pool_config: PoolConfig {
                max_connections: 5,
                min_connections: 1,
                connect_timeout: 5,
                idle_timeout: 30,
                max_lifetime: 300,
                acquire_timeout: 5,
            },
            performance_config: PerformanceConfig {
                wal_mode: false, // 测试中禁用WAL以简化
                synchronous: 1,
                cache_size: -1000,
                page_size: 4096,
                foreign_keys: true,
                auto_vacuum: false,
                mmap_size: 0,
                journal_mode: "DELETE".to_string(),
            },
            backup_config: BackupConfig {
                auto_backup: false,
                backup_interval_hours: 24,
                retain_backup_count: 7,
                backup_dir: temp_dir.path().join("backups"),
                compress_backup: false,
            },
        };

        let db = DatabaseConnection::with_config(&db_path, None, config)
            .await
            .expect("创建测试数据库失败");

        (db, temp_dir)
    }

    #[tokio::test]
    async fn test_database_creation() {
        let (db, _temp_dir) = create_test_database().await;

        // 验证数据库文件存在
        assert!(db.database_path().exists());

        // 验证连接池工作正常
        let pool = db.pool();
        assert_eq!(pool.status().size(), 1); // 最小连接数

        // 验证可以执行查询
        let result: Result<(i64,), _> = sqlx::query_as("SELECT 1")
            .fetch_one(pool)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, 1);
    }

    #[tokio::test]
    async fn test_migrations() {
        let (db, _temp_dir) = create_test_database().await;

        // 运行迁移
        db.run_migrations().await.expect("迁移失败");

        // 验证表已创建
        let pool = db.pool();

        // 检查压缩任务表
        let result: Result<(i64,), _> = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='compression_tasks'"
        )
        .fetch_one(pool)
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, 1);

        // 检查密码条目表
        let result: Result<(i64,), _> = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='password_entries'"
        )
        .fetch_one(pool)
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, 1);
    }

    #[tokio::test]
    async fn test_transactions() {
        let (db, _temp_dir) = create_test_database().await;
        db.run_migrations().await.expect("迁移失败");

        use chrono::Utc;
        use uuid::Uuid;

        let task_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // 测试事务成功
        let result = db.execute_transaction(|tx| {
            Box::pin(async move {
                sqlx::query(
                    r#"
                    INSERT INTO compression_tasks (
                        id, source_files, output_path, format, options, status, progress,
                        created_at, total_size, processed_size
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#
                )
                .bind(&task_id)
                .bind("[]")
                .bind("/test/output.zip")
                .bind("zip")
                .bind("{}")
                .bind("Pending")
                .bind(0.0)
                .bind(now)
                .bind(0)
                .bind(0)
                .execute(tx)
                .await?;

                Ok::<(), sqlx::Error>(())
            })
        }).await;

        assert!(result.is_ok());

        // 验证数据已插入
        let pool = db.pool();
        let result: Result<(i64,), _> = sqlx::query_as(
            "SELECT COUNT(*) FROM compression_tasks WHERE id = ?"
        )
        .bind(&task_id)
        .fetch_one(pool)
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, 1);

        // 测试事务回滚
        let result = db.execute_transaction(|tx| {
            Box::pin(async move {
                // 第一次插入应该成功
                sqlx::query(
                    "INSERT INTO compression_tasks (id, source_files, output_path, format, options, status, progress, created_at, total_size, processed_size) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(Uuid::new_v4().to_string())
                .bind("[]")
                .bind("/test/output2.zip")
                .bind("zip")
                .bind("{}")
                .bind("Pending")
                .bind(0.0)
                .bind(now)
                .bind(0)
                .bind(0)
                .execute(tx)
                .await?;

                // 第二次插入应该失败（重复ID）
                sqlx::query(
                    "INSERT INTO compression_tasks (id, source_files, output_path, format, options, status, progress, created_at, total_size, processed_size) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(&task_id) // 重复ID，应该失败
                .bind("[]")
                .bind("/test/output3.zip")
                .bind("zip")
                .bind("{}")
                .bind("Pending")
                .bind(0.0)
                .bind(now)
                .bind(0)
                .bind(0)
                .execute(tx)
                .await?;

                Ok::<(), sqlx::Error>(())
            })
        }).await;

        assert!(result.is_err());

        // 验证第一个插入已回滚
        let pool = db.pool();
        let result: Result<(i64,), _> = sqlx::query_as(
            "SELECT COUNT(*) FROM compression_tasks WHERE output_path = ?"
        )
        .bind("/test/output2.zip")
        .fetch_one(pool)
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, 0); // 应该为0，因为事务回滚了
    }

    #[tokio::test]
    async fn test_backup_and_restore() {
        let (db, temp_dir) = create_test_database().await;
        db.run_migrations().await.expect("迁移失败");

        // 插入一些测试数据
        use chrono::Utc;
        use uuid::Uuid;

        let pool = db.pool();
        let task_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO compression_tasks (
                id, source_files, output_path, format, options, status, progress,
                created_at, total_size, processed_size
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&task_id)
        .bind("[]")
        .bind("/test/output.zip")
        .bind("zip")
        .bind("{}")
        .bind("Completed")
        .bind(100.0)
        .bind(now)
        .bind(1024)
        .bind(1024)
        .execute(pool)
        .await
        .expect("插入测试数据失败");

        // 创建备份
        let backup_path = temp_dir.path().join("backup.db");
        db.backup(&backup_path).await.expect("备份失败");

        // 验证备份文件存在
        assert!(backup_path.exists());

        // 验证备份文件完整性
        let test_pool = SqlitePool::connect(&format!("sqlite:{}", backup_path.display()))
            .await
            .expect("连接备份文件失败");

        let result: Result<(i64,), _> = sqlx::query_as(
            "SELECT COUNT(*) FROM compression_tasks WHERE id = ?"
        )
        .bind(&task_id)
        .fetch_one(&test_pool)
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, 1);

        test_pool.close().await;
    }

    #[tokio::test]
    async fn test_integrity_check() {
        let (db, _temp_dir) = create_test_database().await;
        db.run_migrations().await.expect("迁移失败");

        // 检查完整性
        let is_ok = db.check_integrity().await.expect("完整性检查失败");
        assert!(is_ok);

        // 测试检查和修复
        let is_ok = db.check_and_repair().await.expect("检查和修复失败");
        assert!(is_ok);
    }

    #[tokio::test]
    async fn test_statistics() {
        let (db, _temp_dir) = create_test_database().await;
        db.run_migrations().await.expect("迁移失败");

        // 获取统计信息
        let stats = db.get_statistics().await.expect("获取统计信息失败");

        // 验证基本统计信息
        assert!(stats.page_size > 0);
        assert!(stats.table_count >= 0);
        assert!(stats.total_size >= 0);

        // 测试大小格式化
        let formatted = stats.format_size();
        assert!(!formatted.is_empty());

        // 测试使用百分比
        let percentage = stats.usage_percentage();
        assert!(percentage >= 0.0 && percentage <= 100.0);
    }

    #[tokio::test]
    async fn test_optimization() {
        let (db, _temp_dir) = create_test_database().await;
        db.run_migrations().await.expect("迁移失败");

        // 执行优化
        db.optimize().await.expect("优化失败");

        // 验证优化后数据库仍然可用
        let is_ok = db.check_integrity().await.expect("完整性检查失败");
        assert!(is_ok);
    }

    #[tokio::test]
    async fn test_connection_metrics() {
        let (db, _temp_dir) = create_test_database().await;

        // 获取指标
        let metrics = db.get_metrics().await;

        // 验证指标
        assert!(metrics.total_connections > 0);
        assert!(metrics.last_connection_time.is_some());

        // 执行一些查询以更新指标
        let pool = db.pool();
        let _: (i64,) = sqlx::query_as("SELECT 1")
            .fetch_one(pool)
            .await
            .expect("查询失败");

        // 再次获取指标
        let metrics_after = db.get_metrics().await;
        assert!(metrics_after.query_executions > metrics.query_executions);
    }

    #[tokio::test]
    async fn test_config_validation() {
        // 测试有效配置
        let valid_config = DatabaseConfig::default();
        assert!(valid_config.validate().is_ok());

        // 测试无效配置 - 最大连接数为0
        let mut invalid_config = DatabaseConfig::default();
        invalid_config.pool_config.max_connections = 0;
        assert!(invalid_config.validate().is_err());

        // 测试无效配置 - 最小连接数大于最大连接数
        let mut invalid_config = DatabaseConfig::default();
        invalid_config.pool_config.min_connections = 20;
        invalid_config.pool_config.max_connections = 10;
        assert!(invalid_config.validate().is_err());

        // 测试无效配置 - 无效的页面大小
        let mut invalid_config = DatabaseConfig::default();
        invalid_config.performance_config.page_size = 1000; // 不是有效的页面大小
        assert!(invalid_config.validate().is_err());
    }

    #[tokio::test]
    async fn test_global_database() {
        // 创建临时目录用于测试
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("global_test.db");

        // 设置环境变量
        std::env::set_var("DATABASE_PATH", db_path.to_str().unwrap());
        std::env::set_var("DATABASE_WAL_MODE", "false");
        std::env::set_var("DATABASE_AUTO_BACKUP", "false");

        // 初始化全局数据库
        let result = init().await;
        assert!(result.is_ok(), "全局数据库初始化失败: {:?}", result.err());

        // 验证数据库已初始化
        assert!(is_initialized().await);

        // 获取连接
        let connection = get_connection().await;
        assert!(connection.is_ok());

        // 验证连接可用
        let conn = connection.unwrap();
        let is_ok = conn.check_integrity().await;
        assert!(is_ok.is_ok());

        // 关闭数据库
        let result = shutdown().await;
        assert!(result.is_ok());

        // 清理环境变量
        std::env::remove_var("DATABASE_PATH");
        std::env::remove_var("DATABASE_WAL_MODE");
        std::env::remove_var("DATABASE_AUTO_BACKUP");
    }
}