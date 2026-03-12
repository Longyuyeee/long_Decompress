use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tokio::sync::Mutex;
use crate::database::connection::{DatabaseConnection, DatabaseError};

/// 数据库管理服务
pub struct DatabaseManagementService {
    connection: DatabaseConnection,
    maintenance_lock: Mutex<()>,
    last_maintenance_time: Mutex<Option<Instant>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceReport {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub operations: Vec<MaintenanceOperation>,
    pub duration_seconds: f64,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceOperation {
    pub name: String,
    pub description: String,
    pub duration_seconds: f64,
    pub success: bool,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseHealthReport {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub integrity_check: bool,
    pub size_bytes: u64,
    pub table_count: u64,
    pub index_count: u64,
    pub total_rows: u64,
    pub fragmentation_percentage: f64,
    pub last_backup_time: Option<chrono::DateTime<chrono::Utc>>,
    pub connection_metrics: ConnectionMetricsReport,
    pub recommendations: Vec<Recommendation>,
    pub overall_health: HealthStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetricsReport {
    pub total_connections: u64,
    pub active_connections: u64,
    pub idle_connections: u64,
    pub connection_errors: u64,
    pub query_executions: u64,
    pub transaction_count: u64,
    pub backup_count: u64,
    pub last_connection_time: Option<chrono::DateTime<chrono::Utc>>,
    pub last_query_time: Option<chrono::DateTime<chrono::Utc>>,
    pub last_backup_time: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub level: RecommendationLevel,
    pub title: String,
    pub description: String,
    pub action: String,
    pub priority: u8, // 1-10, 10为最高优先级
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecommendationLevel {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Critical,
}

impl DatabaseManagementService {
    /// 创建新的数据库管理服务
    pub fn new(connection: DatabaseConnection) -> Self {
        Self {
            connection,
            maintenance_lock: Mutex::new(()),
            last_maintenance_time: Mutex::new(None),
        }
    }

    /// 运行维护任务
    pub async fn run_maintenance_task(&self) -> Result<()> {
        self.perform_maintenance().await.map(|_| ())
    }

    /// 执行定期维护
    pub async fn perform_maintenance(&self) -> Result<MaintenanceReport> {
        log::info!("开始执行数据库维护");

        // 获取维护锁，防止并发维护
        let _lock = self.maintenance_lock.lock().await;

        let start_time = Instant::now();
        let timestamp = chrono::Utc::now();
        let mut operations = Vec::new();
        let mut overall_success = true;
        let mut error_message = None;

        // 执行维护操作
        let ops = [
            ("integrity_check", "检查数据库完整性"),
            ("optimize", "优化数据库"),
            ("cleanup_old_data", "清理旧数据"),
            ("update_statistics", "更新统计信息"),
            ("auto_backup", "自动备份"),
        ];

        for (name, description) in ops {
            let op_start = Instant::now();
            let result = match name {
                "integrity_check" => self.check_integrity().await,
                "optimize" => self.optimize_database().await,
                "cleanup_old_data" => self.cleanup_old_data().await,
                "update_statistics" => self.update_statistics().await,
                "auto_backup" => self.auto_backup().await,
                _ => unreachable!(),
            };

            let operation_report = MaintenanceOperation {
                name: name.to_string(),
                description: description.to_string(),
                duration_seconds: op_start.elapsed().as_secs_f64(),
                success: result.is_ok(),
                details: result.as_ref().ok().map(|_| "成功".to_string())
                    .or_else(|| result.as_ref().err().map(|e| e.to_string())),
            };

            operations.push(operation_report);

            if let Err(e) = result {
                overall_success = false;
                if error_message.is_none() {
                    error_message = Some(e.to_string());
                }
                log::warn!("维护操作 '{}' 失败: {}", name, e);
            }
        }

        // 更新最后维护时间
        *self.last_maintenance_time.lock().await = Some(Instant::now());

        let duration = start_time.elapsed().as_secs_f64();

        let report = MaintenanceReport {
            timestamp,
            operations,
            duration_seconds: duration,
            success: overall_success,
            error_message,
        };

        if overall_success {
            log::info!("数据库维护完成，耗时: {:.2}秒", duration);
        } else {
            log::warn!("数据库维护部分失败，耗时: {:.2}秒", duration);
        }

        Ok(report)
    }

    /// 检查数据库完整性
    async fn check_integrity(&self) -> Result<()> {
        let is_ok = self.connection.check_integrity().await?;

        if !is_ok {
            log::warn!("数据库完整性检查失败，尝试修复");
            let repaired = self.connection.check_and_repair().await?;

            if !repaired {
                return Err(DatabaseError::IntegrityCheckFailed("数据库修复失败".to_string()).into());
            }
        }

        Ok(())
    }

    /// 优化数据库
    async fn optimize_database(&self) -> Result<()> {
        self.connection.optimize().await
    }

    /// 清理旧数据
    async fn cleanup_old_data(&self) -> Result<()> {
        use crate::database::migrations::cleanup_old_data;

        // 默认保留30天数据
        let retention_days = 30;
        let pool = self.connection.pool();

        cleanup_old_data(pool, retention_days).await?;

        log::info!("已清理超过{}天的旧数据", retention_days);
        Ok(())
    }

    /// 更新统计信息
    async fn update_statistics(&self) -> Result<()> {
        let pool = self.connection.pool();

        // 执行ANALYZE以更新统计信息
        sqlx::query("ANALYZE;")
            .execute(pool)
            .await
            .context("更新统计信息失败")?;

        log::info!("已更新数据库统计信息");
        Ok(())
    }

    /// 自动备份
    async fn auto_backup(&self) -> Result<()> {
        self.connection.auto_backup().await
    }
}
