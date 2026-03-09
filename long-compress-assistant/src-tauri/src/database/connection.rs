use anyhow::{Context, Result};
use sqlx::{Sqlite, SqlitePool, sqlite::SqliteConnectOptions, Transaction};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::sync::{Mutex, RwLock};
use crate::database::config::{DatabaseConfig, PoolConfig, PerformanceConfig};

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("数据库连接失败: {0}")]
    ConnectionFailed(String),

    #[error("数据库初始化失败: {0}")]
    InitializationFailed(String),

    #[error("数据库迁移失败: {0}")]
    MigrationFailed(String),

    #[error("数据库备份失败: {0}")]
    BackupFailed(String),

    #[error("数据库恢复失败: {0}")]
    RestoreFailed(String),

    #[error("数据库完整性检查失败: {0}")]
    IntegrityCheckFailed(String),

    #[error("数据库优化失败: {0}")]
    OptimizationFailed(String),

    #[error("事务操作失败: {0}")]
    TransactionFailed(String),

    #[error("查询执行失败: {0}")]
    QueryExecutionFailed(String),

    #[error("连接池耗尽")]
    PoolExhausted,

    #[error("连接超时")]
    ConnectionTimeout,

    #[error("数据库文件不存在: {0}")]
    FileNotFound(String),

    #[error("权限不足: {0}")]
    PermissionDenied(String),

    #[error("配置错误: {0}")]
    ConfigurationError(String),
}

#[derive(Clone)]
pub struct DatabaseConnection {
    pool: SqlitePool,
    database_path: PathBuf,
    config: DatabaseConfig,
    metrics: Arc<RwLock<ConnectionMetrics>>,
    last_backup_time: Arc<Mutex<Option<Instant>>>,
}

#[derive(Debug, Clone)]
pub struct ConnectionMetrics {
    pub total_connections: u64,
    pub active_connections: u64,
    pub idle_connections: u64,
    pub connection_errors: u64,
    pub query_executions: u64,
    pub transaction_count: u64,
    pub backup_count: u64,
    pub last_connection_time: Option<Instant>,
    pub last_query_time: Option<Instant>,
    pub last_backup_time: Option<Instant>,
}

impl Default for ConnectionMetrics {
    fn default() -> Self {
        Self {
            total_connections: 0,
            active_connections: 0,
            idle_connections: 0,
            connection_errors: 0,
            query_executions: 0,
            transaction_count: 0,
            backup_count: 0,
            last_connection_time: None,
            last_query_time: None,
            last_backup_time: None,
        }
    }
}

impl DatabaseConnection {
    /// 使用默认配置初始化数据库连接
    pub async fn new(database_path: &Path, password: Option<&str>) -> Result<Self> {
        let config = DatabaseConfig::default();
        Self::with_config(database_path, password, config).await
    }

    /// 使用自定义配置初始化数据库连接
    pub async fn with_config(
        database_path: &Path,
        password: Option<&str>,
        config: DatabaseConfig,
    ) -> Result<Self> {
        // 验证配置
        config.validate()
            .map_err(|e| DatabaseError::ConfigurationError(e.to_string()))?;

        // 确保数据库目录存在
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| DatabaseError::ConnectionFailed(format!("创建数据库目录失败: {}", e)))?;
        }

        // 构建连接选项
        let mut connect_options = SqliteConnectOptions::from_str(
            &format!("sqlite:{}", database_path.display())
        )
        .map_err(|e| DatabaseError::ConnectionFailed(format!("解析连接字符串失败: {}", e)))?
        .create_if_missing(true);

        // 设置连接超时
        connect_options = connect_options.connect_timeout(config.get_connect_timeout());

        // 注意：SQLCipher支持已移除，密码参数暂时忽略
        // 在实际应用中，可以考虑其他加密方案

        // 配置连接池
        let pool_options = sqlx::pool::PoolOptions::<sqlx::Sqlite>::new()
            .max_connections(config.pool_config.max_connections)
            .min_connections(config.pool_config.min_connections)
            .acquire_timeout(config.get_acquire_timeout())
            .idle_timeout(Some(config.get_idle_timeout()))
            .max_lifetime(Some(config.get_max_lifetime()));

        // 建立连接池
        let pool = pool_options.connect_with(connect_options)
            .await
            .map_err(|e| DatabaseError::ConnectionFailed(format!("连接数据库失败: {}", e)))?;

        // 应用性能配置
        Self::apply_performance_config(&pool, &config.performance_config).await?;

        let metrics = Arc::new(RwLock::new(ConnectionMetrics::default()));
        let last_backup_time = Arc::new(Mutex::new(None));

        let connection = Self {
            pool,
            database_path: database_path.to_path_buf(),
            config,
            metrics,
            last_backup_time,
        };

        // 更新连接指标
        connection.update_connection_metrics().await;

        Ok(connection)
    }

    /// 应用性能配置
    async fn apply_performance_config(pool: &SqlitePool, config: &PerformanceConfig) -> Result<()> {
        use sqlx::Connection;

        let mut conn = pool.acquire()
            .await
            .map_err(|e| DatabaseError::ConnectionFailed(format!("获取连接失败: {}", e)))?;

        // 设置WAL模式
        if config.wal_mode {
            sqlx::query("PRAGMA journal_mode = WAL;")
                .execute(&mut *conn)
                .await
                .map_err(|e| DatabaseError::ConfigurationError(format!("设置WAL模式失败: {}", e)))?;
        }

        // 设置同步模式
        sqlx::query(&format!("PRAGMA synchronous = {};", config.synchronous))
            .execute(&mut *conn)
            .await
            .map_err(|e| DatabaseError::ConfigurationError(format!("设置同步模式失败: {}", e)))?;

        // 设置缓存大小
        sqlx::query(&format!("PRAGMA cache_size = {};", config.cache_size))
            .execute(&mut *conn)
            .await
            .map_err(|e| DatabaseError::ConfigurationError(format!("设置缓存大小失败: {}", e)))?;

        // 设置页面大小
        sqlx::query(&format!("PRAGMA page_size = {};", config.page_size))
            .execute(&mut *conn)
            .await
            .map_err(|e| DatabaseError::ConfigurationError(format!("设置页面大小失败: {}", e)))?;

        // 设置外键约束
        if config.foreign_keys {
            sqlx::query("PRAGMA foreign_keys = ON;")
                .execute(&mut *conn)
                .await
                .map_err(|e| DatabaseError::ConfigurationError(format!("启用外键约束失败: {}", e)))?;
        }

        // 设置自动清理
        if config.auto_vacuum {
            sqlx::query("PRAGMA auto_vacuum = INCREMENTAL;")
                .execute(&mut *conn)
                .await
                .map_err(|e| DatabaseError::ConfigurationError(format!("启用自动清理失败: {}", e)))?;
        }

        // 设置内存映射大小
        if config.mmap_size > 0 {
            sqlx::query(&format!("PRAGMA mmap_size = {};", config.mmap_size))
                .execute(&mut *conn)
                .await
                .map_err(|e| DatabaseError::ConfigurationError(format!("设置内存映射大小失败: {}", e)))?;
        }

        Ok(())
    }

    /// 更新连接指标
    async fn update_connection_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.total_connections += 1;
        metrics.last_connection_time = Some(Instant::now());

        // 获取连接池状态
        let pool_status = self.pool.status();
        metrics.active_connections = pool_status.size() as u64;
        metrics.idle_connections = pool_status.idle() as u64;
    }

    /// 获取数据库连接池
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// 获取数据库路径
    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    /// 获取数据库配置
    pub fn config(&self) -> &DatabaseConfig {
        &self.config
    }

    /// 获取连接指标
    pub async fn get_metrics(&self) -> ConnectionMetrics {
        self.metrics.read().await.clone()
    }

    /// 开始一个新的事务
    pub async fn begin_transaction(&self) -> Result<Transaction<'_, Sqlite>> {
        let mut metrics = self.metrics.write().await;
        metrics.transaction_count += 1;

        self.pool.begin()
            .await
            .map_err(|e| DatabaseError::TransactionFailed(format!("开始事务失败: {}", e)).into())
    }

    /// 执行事务
    pub async fn execute_transaction<F, T, E>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut Transaction<'_, Sqlite>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
        E: Into<anyhow::Error>,
    {
        let mut transaction = self.begin_transaction().await?;

        let result = f(&mut transaction).await;

        match result {
            Ok(value) => {
                transaction.commit()
                    .await
                    .map_err(|e| DatabaseError::TransactionFailed(format!("提交事务失败: {}", e)))?;
                Ok(value)
            }
            Err(e) => {
                transaction.rollback()
                    .await
                    .map_err(|e| DatabaseError::TransactionFailed(format!("回滚事务失败: {}", e)))?;
                Err(e.into())
            }
        }
    }

    /// 执行查询并更新指标
    pub async fn execute_query<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&SqlitePool) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
    {
        let result = f(&self.pool).await;

        // 更新查询指标
        let mut metrics = self.metrics.write().await;
        metrics.query_executions += 1;
        metrics.last_query_time = Some(Instant::now());

        result
    }

    /// 执行数据库迁移
    pub async fn run_migrations(&self) -> Result<()> {
        log::info!("开始执行数据库迁移");

        // 创建迁移表（如果不存在）
        self.create_migration_table().await?;

        // 获取当前数据库版本
        let current_version = self.get_current_version().await?;
        log::info!("当前数据库版本: {}", current_version);

        // 运行迁移
        crate::database::migrations::run_migrations(&self.pool).await
            .map_err(|e| DatabaseError::MigrationFailed(format!("执行迁移失败: {}", e)))?;

        // 更新数据库版本
        let new_version = self.get_latest_version().await;
        if new_version > current_version {
            self.update_version(new_version).await?;
            log::info!("数据库版本已更新到: {}", new_version);
        }

        log::info!("数据库迁移完成");
        Ok(())
    }

    /// 创建迁移表
    async fn create_migration_table(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at DATETIME NOT NULL,
                checksum TEXT NOT NULL
            )
            "#
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DatabaseError::MigrationFailed(format!("创建迁移表失败: {}", e)))?;

        Ok(())
    }

    /// 获取当前数据库版本
    async fn get_current_version(&self) -> Result<i64> {
        let result: Result<(i64,), _> = sqlx::query_as(
            "SELECT MAX(version) FROM schema_migrations"
        )
        .fetch_one(&self.pool)
        .await;

        match result {
            Ok((version,)) => Ok(version.unwrap_or(0)),
            Err(_) => Ok(0),
        }
    }

    /// 获取最新版本号
    async fn get_latest_version(&self) -> i64 {
        // 这里应该从迁移文件中获取最新版本号
        // 暂时返回一个固定值
        1
    }

    /// 更新数据库版本
    async fn update_version(&self, version: i64) -> Result<()> {
        use chrono::Utc;

        sqlx::query(
            r#"
            INSERT INTO schema_migrations (version, name, applied_at, checksum)
            VALUES (?, ?, ?, ?)
            "#
        )
        .bind(version)
        .bind(format!("migration_v{}", version))
        .bind(Utc::now())
        .bind("") // 暂时不计算校验和
        .execute(&self.pool)
        .await
        .map_err(|e| DatabaseError::MigrationFailed(format!("更新版本失败: {}", e)))?;

        Ok(())
    }

    /// 备份数据库
    pub async fn backup(&self, backup_path: &Path) -> Result<()> {
        log::info!("开始备份数据库到: {:?}", backup_path);

        // 确保备份目录存在
        if let Some(parent) = backup_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| DatabaseError::BackupFailed(format!("创建备份目录失败: {}", e)))?;
        }

        // 使用VACUUM INTO进行备份（这会创建一个干净的备份）
        let backup_sql = format!("VACUUM INTO '{}'", backup_path.display());
        sqlx::query(&backup_sql)
            .execute(&self.pool)
            .await
            .map_err(|e| DatabaseError::BackupFailed(format!("执行VACUUM备份失败: {}", e)))?;

        // 更新备份指标
        let mut metrics = self.metrics.write().await;
        metrics.backup_count += 1;
        metrics.last_backup_time = Some(Instant::now());

        // 更新最后备份时间
        *self.last_backup_time.lock().await = Some(Instant::now());

        log::info!("数据库备份完成: {:?}", backup_path);
        Ok(())
    }

    /// 自动备份数据库
    pub async fn auto_backup(&self) -> Result<()> {
        if !self.config.backup_config.auto_backup {
            log::debug!("自动备份已禁用");
            return Ok(());
        }

        // 检查是否需要备份
        let last_backup = self.last_backup_time.lock().await;
        if let Some(last_time) = *last_backup {
            let elapsed = last_time.elapsed();
            let backup_interval = Duration::from_secs(self.config.backup_config.backup_interval_hours as u64 * 3600);

            if elapsed < backup_interval {
                log::debug!("未到备份时间，跳过自动备份");
                return Ok(());
            }
        }

        drop(last_backup); // 释放锁

        // 创建备份文件名（包含时间戳）
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let backup_filename = format!("backup_{}.sqlite", timestamp);
        let backup_path = self.config.backup_config.backup_dir.join(&backup_filename);

        // 执行备份
        self.backup(&backup_path).await?;

        // 清理旧备份
        self.cleanup_old_backups().await?;

        Ok(())
    }

    /// 清理旧备份
    async fn cleanup_old_backups(&self) -> Result<()> {
        let backup_dir = &self.config.backup_config.backup_dir;
        let retain_count = self.config.backup_config.retain_backup_count;

        // 确保备份目录存在
        if !backup_dir.exists() {
            return Ok(());
        }

        // 获取所有备份文件
        let mut entries = tokio::fs::read_dir(backup_dir)
            .await
            .map_err(|e| DatabaseError::BackupFailed(format!("读取备份目录失败: {}", e)))?;

        let mut backup_files = Vec::new();
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| DatabaseError::BackupFailed(format!("读取备份文件失败: {}", e)))? {

            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "sqlite") {
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(modified) = metadata.modified() {
                        backup_files.push((path, modified));
                    }
                }
            }
        }

        // 按修改时间排序（最新的在前面）
        backup_files.sort_by(|a, b| b.1.cmp(&a.1));

        // 删除超出保留数量的旧备份
        for i in retain_count as usize..backup_files.len() {
            let (path, _) = &backup_files[i];
            tokio::fs::remove_file(path)
                .await
                .map_err(|e| DatabaseError::BackupFailed(format!("删除旧备份失败: {}", e)))?;
            log::info!("删除旧备份: {:?}", path);
        }

        Ok(())
    }

    /// 恢复数据库
    pub async fn restore(&mut self, backup_path: &Path, password: Option<&str>) -> Result<()> {
        log::info!("开始从备份恢复数据库: {:?}", backup_path);

        // 检查备份文件是否存在
        if !backup_path.exists() {
            return Err(DatabaseError::FileNotFound(format!("备份文件不存在: {:?}", backup_path)).into());
        }

        // 验证备份文件
        self.validate_backup_file(backup_path).await?;

        // 关闭当前连接
        self.pool.close().await;

        // 备份当前数据库（以防恢复失败）
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let rollback_filename = format!("rollback_{}.sqlite", timestamp);
        let rollback_path = self.config.backup_config.backup_dir.join(&rollback_filename);

        if self.database_path.exists() {
            tokio::fs::copy(&self.database_path, &rollback_path)
                .await
                .map_err(|e| DatabaseError::RestoreFailed(format!("创建回滚备份失败: {}", e)))?;
        }

        // 使用Result来处理错误，而不是try-catch
        let restore_result = async {
            // 复制备份文件到数据库位置
            tokio::fs::copy(backup_path, &self.database_path)
                .await
                .map_err(|e| DatabaseError::RestoreFailed(format!("复制备份文件失败: {}", e)))?;

            // 重新连接
            let connect_options = SqliteConnectOptions::from_str(
                &format!("sqlite:{}", self.database_path.display())
            )
            .map_err(|e| DatabaseError::ConnectionFailed(format!("解析连接字符串失败: {}", e)))?
            .create_if_missing(true);

            // 设置连接超时
            let connect_options = connect_options.connect_timeout(self.config.get_connect_timeout());

            // 重新建立连接池
            let pool_options = sqlx::pool::PoolOptions::<sqlx::Sqlite>::new()
                .max_connections(self.config.pool_config.max_connections)
                .min_connections(self.config.pool_config.min_connections)
                .acquire_timeout(self.config.get_acquire_timeout())
                .idle_timeout(Some(self.config.get_idle_timeout()))
                .max_lifetime(Some(self.config.get_max_lifetime()));

            self.pool = pool_options.connect_with(connect_options)
                .await
                .map_err(|e| DatabaseError::ConnectionFailed(format!("重新连接数据库失败: {}", e)))?;

            // 应用性能配置
            Self::apply_performance_config(&self.pool, &self.config.performance_config).await?;

            // 验证恢复后的数据库
            self.check_integrity().await?;

            log::info!("数据库恢复成功");
            Ok(())
        }.await;

        match restore_result {
            Ok(_) => Ok(()),
            Err(e) => {
                // 恢复失败，尝试回滚
                log::error!("数据库恢复失败，尝试回滚: {}", e);

                if rollback_path.exists() {
                    tokio::fs::copy(&rollback_path, &self.database_path)
                        .await
                        .map_err(|e| DatabaseError::RestoreFailed(format!("回滚失败: {}", e)))?;
                    log::info!("已回滚到之前的数据库状态");
                }

                Err(e)
            }
        }
    }

    /// 验证备份文件
    async fn validate_backup_file(&self, backup_path: &Path) -> Result<()> {
        // 检查文件大小
        let metadata = tokio::fs::metadata(backup_path)
            .await
            .map_err(|e| DatabaseError::BackupFailed(format!("获取备份文件元数据失败: {}", e)))?;

        if metadata.len() == 0 {
            return Err(DatabaseError::BackupFailed("备份文件为空".to_string()).into());
        }

        // 尝试连接备份文件以验证完整性
        let test_connect_options = SqliteConnectOptions::from_str(
            &format!("sqlite:{}", backup_path.display())
        )
        .map_err(|e| DatabaseError::BackupFailed(format!("解析备份文件连接字符串失败: {}", e)))?
        .read_only(true);

        let test_pool = SqlitePool::connect_with(test_connect_options)
            .await;

        if let Err(e) = test_pool {
            return Err(DatabaseError::BackupFailed(format!("备份文件无效或损坏: {}", e)).into());
        }

        let test_pool = test_pool.unwrap();

        // 检查完整性
        let result: Result<(String,), _> = sqlx::query_as("PRAGMA integrity_check;")
            .fetch_one(&test_pool)
            .await;

        test_pool.close().await;

        match result {
            Ok((check,)) => {
                if check == "ok" {
                    Ok(())
                } else {
                    Err(DatabaseError::BackupFailed(format!("备份文件完整性检查失败: {}", check)).into())
                }
            }
            Err(e) => Err(DatabaseError::BackupFailed(format!("执行完整性检查失败: {}", e)).into()),
        }
    }

    /// 检查数据库完整性
    pub async fn check_integrity(&self) -> Result<bool> {
        let result: (String,) = sqlx::query_as("PRAGMA integrity_check;")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::IntegrityCheckFailed(format!("检查数据库完整性失败: {}", e)))?;

        let is_ok = result.0 == "ok";

        if !is_ok {
            log::error!("数据库完整性检查失败: {}", result.0);
        }

        Ok(is_ok)
    }

    /// 执行完整性检查并修复
    pub async fn check_and_repair(&self) -> Result<bool> {
        log::info!("开始检查并修复数据库");

        // 检查完整性
        let is_ok = self.check_integrity().await?;

        if !is_ok {
            log::warn!("数据库完整性检查失败，尝试修复");

            // 尝试备份当前数据库
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let corrupt_filename = format!("corrupt_{}.sqlite", timestamp);
            let corrupt_path = self.config.backup_config.backup_dir.join(&corrupt_filename);

            if self.database_path.exists() {
                tokio::fs::copy(&self.database_path, &corrupt_path)
                    .await
                    .map_err(|e| DatabaseError::IntegrityCheckFailed(format!("备份损坏的数据库失败: {}", e)))?;
                log::info!("已备份损坏的数据库到: {:?}", corrupt_path);
            }

            // 尝试使用VACUUM修复
            sqlx::query("VACUUM;")
                .execute(&self.pool)
                .await
                .map_err(|e| DatabaseError::IntegrityCheckFailed(format!("执行VACUUM修复失败: {}", e)))?;

            log::info!("已执行VACUUM修复");

            // 再次检查完整性
            let is_ok_after_repair = self.check_integrity().await?;

            if is_ok_after_repair {
                log::info!("数据库修复成功");
            } else {
                log::error!("数据库修复失败");
            }

            Ok(is_ok_after_repair)
        } else {
            log::info!("数据库完整性检查通过");
            Ok(true)
        }
    }

    /// 优化数据库
    pub async fn optimize(&self) -> Result<()> {
        log::info!("开始优化数据库");

        // 执行优化
        sqlx::query("PRAGMA optimize;")
            .execute(&self.pool)
            .await
            .map_err(|e| DatabaseError::OptimizationFailed(format!("优化数据库失败: {}", e)))?;

        // 执行ANALYZE以更新统计信息
        sqlx::query("ANALYZE;")
            .execute(&self.pool)
            .await
            .map_err(|e| DatabaseError::OptimizationFailed(format!("更新统计信息失败: {}", e)))?;

        // 清理WAL文件
        self.cleanup_wal_files().await?;

        log::info!("数据库优化完成");
        Ok(())
    }

    /// 清理WAL文件
    async fn cleanup_wal_files(&self) -> Result<()> {
        if self.config.performance_config.wal_mode {
            // 检查WAL文件
            let wal_path = self.database_path.with_extension("sqlite-wal");
            let shm_path = self.database_path.with_extension("sqlite-shm");

            // 执行检查点以清理WAL文件
            sqlx::query("PRAGMA wal_checkpoint(TRUNCATE);")
                .execute(&self.pool)
                .await
                .map_err(|e| DatabaseError::OptimizationFailed(format!("执行检查点失败: {}", e)))?;

            // 删除WAL文件（如果存在且为空）
            if wal_path.exists() {
                let metadata = tokio::fs::metadata(&wal_path).await;
                if let Ok(metadata) = metadata {
                    if metadata.len() == 0 {
                        tokio::fs::remove_file(&wal_path)
                            .await
                            .map_err(|e| DatabaseError::OptimizationFailed(format!("删除WAL文件失败: {}", e)))?;
                    }
                }
            }

            if shm_path.exists() {
                let metadata = tokio::fs::metadata(&shm_path).await;
                if let Ok(metadata) = metadata {
                    if metadata.len() == 0 {
                        tokio::fs::remove_file(&shm_path)
                            .await
                            .map_err(|e| DatabaseError::OptimizationFailed(format!("删除SHM文件失败: {}", e)))?;
                    }
                }
            }
        }

        Ok(())
    }

    /// 获取数据库大小
    pub async fn get_size(&self) -> Result<u64> {
        let metadata = fs::metadata(&self.database_path)
            .await
            .map_err(|e| DatabaseError::ConfigurationError(format!("获取数据库文件元数据失败: {}", e)))?;

        Ok(metadata.len())
    }

    /// 获取数据库文件大小（包括WAL文件）
    pub async fn get_total_size(&self) -> Result<u64> {
        let mut total_size = self.get_size().await?;

        if self.config.performance_config.wal_mode {
            let wal_path = self.database_path.with_extension("sqlite-wal");
            let shm_path = self.database_path.with_extension("sqlite-shm");

            if wal_path.exists() {
                if let Ok(metadata) = fs::metadata(&wal_path).await {
                    total_size += metadata.len();
                }
            }

            if shm_path.exists() {
                if let Ok(metadata) = fs::metadata(&shm_path).await {
                    total_size += metadata.len();
                }
            }
        }

        Ok(total_size)
    }

    /// 获取数据库统计信息
    pub async fn get_statistics(&self) -> Result<DatabaseStatistics> {
        let page_size: (i64,) = sqlx::query_as("PRAGMA page_size;")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryExecutionFailed(format!("获取页面大小失败: {}", e)))?;

        let page_count: (i64,) = sqlx::query_as("PRAGMA page_count;")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryExecutionFailed(format!("获取页面数量失败: {}", e)))?;

        let freelist_count: (i64,) = sqlx::query_as("PRAGMA freelist_count;")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryExecutionFailed(format!("获取空闲列表数量失败: {}", e)))?;

        let cache_size: (i64,) = sqlx::query_as("PRAGMA cache_size;")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryExecutionFailed(format!("获取缓存大小失败: {}", e)))?;

        let table_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%';"
        )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryExecutionFailed(format!("获取表数量失败: {}", e)))?;

        let index_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name NOT LIKE 'sqlite_%';"
        )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryExecutionFailed(format!("获取索引数量失败: {}", e)))?;

        let total_rows: (i64,) = sqlx::query_as(
            r#"
            SELECT SUM("rows") FROM (
                SELECT COUNT(*) as "rows" FROM sqlite_master
                WHERE type='table' AND name NOT LIKE 'sqlite_%'
            )
            "#
        )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryExecutionFailed(format!("获取总行数失败: {}", e)))?;

        Ok(DatabaseStatistics {
            page_size: page_size.0 as u64,
            page_count: page_count.0 as u64,
            freelist_count: freelist_count.0 as u64,
            cache_size: cache_size.0 as u64,
            table_count: table_count.0 as u64,
            index_count: index_count.0 as u64,
            total_rows: total_rows.0.unwrap_or(0) as u64,
            total_size: page_size.0 as u64 * page_count.0 as u64,
            used_size: (page_count.0 - freelist_count.0) as u64 * page_size.0 as u64,
            fragmentation: if page_count.0 > 0 {
                (freelist_count.0 as f64 / page_count.0 as f64) * 100.0
            } else {
                0.0
            },
        })
    }

    /// 获取表空间使用情况
    pub async fn get_table_usage(&self) -> Result<Vec<TableUsage>> {
        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
        )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryExecutionFailed(format!("获取表列表失败: {}", e)))?;

        let mut table_usage = Vec::new();

        for (table_name,) in tables {
            let row_count: (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM {}", table_name))
                .fetch_one(&self.pool)
                .await
                .map_err(|e| DatabaseError::QueryExecutionFailed(format!("获取表行数失败: {}", e)))?;

            let size_info: (String,) = sqlx::query_as(&format!("PRAGMA table_info({})", table_name))
                .fetch_one(&self.pool)
                .await
                .map_err(|e| DatabaseError::QueryExecutionFailed(format!("获取表信息失败: {}", e)))?;

            // 这里可以添加更详细的大小计算逻辑
            table_usage.push(TableUsage {
                name: table_name,
                row_count: row_count.0,
                estimated_size: 0, // 需要更复杂的计算
            });
        }

        Ok(table_usage)
    }
}

#[derive(Debug)]
pub struct DatabaseStatistics {
    pub page_size: u64,
    pub page_count: u64,
    pub freelist_count: u64,
    pub cache_size: u64,
    pub table_count: u64,
    pub index_count: u64,
    pub total_rows: u64,
    pub total_size: u64,
    pub used_size: u64,
    pub fragmentation: f64,
}

impl DatabaseStatistics {
    pub fn format_size(&self) -> String {
        const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];

        if self.total_size == 0 {
            return "0 B".to_string();
        }

        let base = 1024_f64;
        let size_f64 = self.total_size as f64;
        let exponent = (size_f64.log10() / base.log10()).floor() as i32;
        let unit_index = exponent.min(5).max(0) as usize;

        let formatted = size_f64 / base.powi(exponent);

        format!("{:.2} {}", formatted, UNITS[unit_index])
    }

    pub fn format_used_size(&self) -> String {
        const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];

        if self.used_size == 0 {
            return "0 B".to_string();
        }

        let base = 1024_f64;
        let size_f64 = self.used_size as f64;
        let exponent = (size_f64.log10() / base.log10()).floor() as i32;
        let unit_index = exponent.min(5).max(0) as usize;

        let formatted = size_f64 / base.powi(exponent);

        format!("{:.2} {}", formatted, UNITS[unit_index])
    }

    pub fn usage_percentage(&self) -> f64 {
        if self.total_size == 0 {
            return 0.0;
        }

        (self.used_size as f64 / self.total_size as f64) * 100.0
    }

    pub fn fragmentation_percentage(&self) -> f64 {
        self.fragmentation
    }
}

#[derive(Debug)]
pub struct TableUsage {
    pub name: String,
    pub row_count: i64,
    pub estimated_size: u64,
}

impl TableUsage {
    pub fn format_size(&self) -> String {
        const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];

        if self.estimated_size == 0 {
            return "0 B".to_string();
        }

        let base = 1024_f64;
        let size_f64 = self.estimated_size as f64;
        let exponent = (size_f64.log10() / base.log10()).floor() as i32;
        let unit_index = exponent.min(5).max(0) as usize;

        let formatted = size_f64 / base.powi(exponent);

        format!("{:.2} {}", formatted, UNITS[unit_index])
    }
}


/// 全局数据库连接实例
pub struct GlobalDatabase {
    connection: Arc<RwLock<Option<DatabaseConnection>>>,
}

impl GlobalDatabase {
    /// 创建新的全局数据库实例
    pub fn new() -> Self {
        Self {
            connection: Arc::new(RwLock::new(None)),
        }
    }

    /// 初始化全局数据库连接
    pub async fn init(&self) -> Result<()> {
        log::info!("开始初始化全局数据库连接");

        let config = DatabaseConfig::from_env()
            .unwrap_or_else(|_| DatabaseConfig::default());

        // 确保数据目录存在
        if let Some(parent) = config.path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| DatabaseError::InitializationFailed(format!("创建数据目录失败: {}", e)))?;
        }

        let password = config.password.as_deref();
        let db = DatabaseConnection::with_config(&config.path, password, config).await?;

        // 运行迁移
        db.run_migrations().await?;

        // 检查并修复数据库（如果需要）
        if !db.check_integrity().await? {
            log::warn!("数据库完整性检查失败，尝试修复");
            db.check_and_repair().await?;
        }

        // 执行初始优化
        db.optimize().await?;

        // 设置连接
        let mut connection = self.connection.write().await;
        *connection = Some(db);

        log::info!("全局数据库连接初始化完成");
        Ok(())
    }

    /// 获取数据库连接
    pub async fn get_connection(&self) -> Result<Arc<DatabaseConnection>> {
        let connection = self.connection.read().await;

        connection
            .as_ref()
            .map(|conn| Arc::new(conn.clone()))
            .ok_or_else(|| DatabaseError::InitializationFailed("数据库未初始化".to_string()).into())
    }

    /// 检查数据库是否已初始化
    pub async fn is_initialized(&self) -> bool {
        let connection = self.connection.read().await;
        connection.is_some()
    }

    /// 关闭数据库连接
    pub async fn shutdown(&self) -> Result<()> {
        log::info!("开始关闭数据库连接");

        let mut connection = self.connection.write().await;

        if let Some(db) = connection.take() {
            // 执行最终优化
            if let Err(e) = db.optimize().await {
                log::warn!("关闭前优化数据库失败: {}", e);
            }

            // 执行自动备份
            if let Err(e) = db.auto_backup().await {
                log::warn!("关闭前自动备份失败: {}", e);
            }

            // 关闭连接池
            db.pool().close().await;
        }

        log::info!("数据库连接已关闭");
        Ok(())
    }

    /// 重新初始化数据库连接
    pub async fn reinitialize(&self) -> Result<()> {
        log::info!("开始重新初始化数据库连接");

        // 先关闭现有连接
        self.shutdown().await?;

        // 重新初始化
        self.init().await?;

        log::info!("数据库连接重新初始化完成");
        Ok(())
    }
}

impl Default for GlobalDatabase {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局数据库实例
pub static GLOBAL_DATABASE: GlobalDatabase = GlobalDatabase::new();

/// 初始化全局数据库连接
pub async fn init() -> Result<()> {
    GLOBAL_DATABASE.init().await
}

/// 获取全局数据库连接
pub async fn get_connection() -> Result<Arc<DatabaseConnection>> {
    GLOBAL_DATABASE.get_connection().await
}

/// 检查数据库是否已初始化
pub async fn is_initialized() -> bool {
    GLOBAL_DATABASE.is_initialized().await
}

/// 关闭数据库连接
pub async fn shutdown() -> Result<()> {
    GLOBAL_DATABASE.shutdown().await
}

/// 重新初始化数据库连接
pub async fn reinitialize() -> Result<()> {
    GLOBAL_DATABASE.reinitialize().await
}