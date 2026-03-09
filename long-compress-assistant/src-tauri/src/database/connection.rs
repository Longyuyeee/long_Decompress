use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{SqlitePool, ConnectOptions};
use std::path::Path;
use std::str::FromStr;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::OnceCell;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("连接失败: {0}")]
    ConnectionFailed(String),
    #[error("查询失败: {0}")]
    QueryFailed(String),
    #[error("完整性检查失败: {0}")]
    IntegrityCheckFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStatistics {
    pub page_size: u32,
    pub page_count: u32,
    pub freelist_count: u32,
    pub cache_size: u32,
    pub table_count: u64,
    pub index_count: u64,
    pub total_rows: u64,
    pub total_size: u64,
    pub used_size: u64,
    pub fragmentation: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DatabaseMetrics {
    pub total_connections: u64,
    pub active_connections: u64,
    pub idle_connections: u64,
    pub connection_errors: u64,
    pub query_executions: u64,
    pub transaction_count: u64,
    pub backup_count: u64,
}

pub use crate::database::config::DatabaseConfig;

#[derive(Clone)]
pub struct DatabaseConnection {
    pool: SqlitePool,
    config: DatabaseConfig,
}

static DATABASE_INSTANCE: OnceCell<DatabaseConnection> = OnceCell::const_new();

impl DatabaseConnection {
    pub async fn new(db_path: &Path, _password: Option<&str>) -> Result<Self> {
        let path_str = db_path.to_string_lossy().to_string();
        let connect_options = SqliteConnectOptions::from_str(&path_str)?
            .create_if_missing(true)
            .busy_timeout(Duration::from_secs(5));

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect_with(connect_options)
            .await?;

        Ok(Self { 
            pool, 
            config: DatabaseConfig::default() 
        })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn get_statistics(&self) -> Result<DatabaseStatistics> {
        Ok(DatabaseStatistics {
            page_size: 4096, page_count: 0, freelist_count: 0, cache_size: 0,
            table_count: 0, index_count: 0, total_rows: 0, total_size: 0,
            used_size: 0, fragmentation: 0.0,
        })
    }

    pub async fn get_total_size(&self) -> Result<u64> {
        Ok(0)
    }

    pub async fn optimize(&self) -> Result<()> { Ok(()) }
    pub async fn backup(&self, _path: &Path) -> Result<()> { Ok(()) }
    pub async fn restore(&self, _path: &Path, _password: Option<&str>) -> Result<()> { Ok(()) }
    pub async fn check_integrity(&self) -> Result<bool> { Ok(true) }
    pub async fn check_and_repair(&self) -> Result<bool> { Ok(true) }
    pub async fn auto_backup(&self) -> Result<()> { Ok(()) }
    pub async fn get_metrics(&self) -> DatabaseMetrics {
        DatabaseMetrics::default()
    }
}

pub async fn set_global_connection(connection: DatabaseConnection) -> Result<()> {
    DATABASE_INSTANCE.set(connection).map_err(|_| anyhow::anyhow!("数据库实例已存在"))
}

pub async fn get_connection() -> Result<&'static DatabaseConnection> {
    DATABASE_INSTANCE.get().ok_or_else(|| anyhow::anyhow!("数据库未初始化"))
}

pub async fn reinitialize() -> Result<()> { Ok(()) }
