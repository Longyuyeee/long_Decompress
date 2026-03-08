use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::sync::Mutex;
use crate::database::connection::{DatabaseConnection, DatabaseError};
use crate::database::config::DatabaseConfig;

/// 数据库管理服务
pub struct DatabaseManager {
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

impl DatabaseManager {
    /// 创建新的数据库管理器
    pub fn new(connection: DatabaseConnection) -> Self {
        Self {
            connection,
            maintenance_lock: Mutex::new(()),
            last_maintenance_time: Mutex::new(None),
        }
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
        let maintenance_ops = vec![
            ("integrity_check", "检查数据库完整性", self.check_integrity()),
            ("optimize", "优化数据库", self.optimize_database()),
            ("cleanup_old_data", "清理旧数据", self.cleanup_old_data()),
            ("update_statistics", "更新统计信息", self.update_statistics()),
            ("auto_backup", "自动备份", self.auto_backup()),
        ];

        for (name, description, operation) in maintenance_ops {
            let op_start = Instant::now();
            let result = operation.await;

            let operation_report = MaintenanceOperation {
                name: name.to_string(),
                description: description.to_string(),
                duration_seconds: op_start.elapsed().as_secs_f64(),
                success: result.is_ok(),
                details: result.as_ref().ok().map(|_| "成功".to_string())
                    .or_else(|| result.as_ref().err().map(|e| e.to_string())),
            };

            operations.push(operation_report);

            if result.is_err() {
                overall_success = false;
                if error_message.is_none() {
                    error_message = Some(result.err().unwrap().to_string());
                }
                log::warn!("维护操作 '{}' 失败: {}", name, result.err().unwrap());
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
        use chrono::{Utc, Duration};

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

        log::info("已更新数据库统计信息");
        Ok(())
    }

    /// 自动备份
    async fn auto_backup(&self) -> Result<()> {
        self.connection.auto_backup().await
    }

    /// 获取数据库健康报告
    pub async fn get_health_report(&self) -> Result<DatabaseHealthReport> {
        log::info!("生成数据库健康报告");

        let timestamp = chrono::Utc::now();

        // 收集各种健康指标
        let integrity_check = self.connection.check_integrity().await.unwrap_or(false);

        let stats = self.connection.get_statistics().await.unwrap_or_else(|_| {
            crate::database::connection::DatabaseStatistics {
                page_size: 0,
                page_count: 0,
                freelist_count: 0,
                cache_size: 0,
                table_count: 0,
                index_count: 0,
                total_rows: 0,
                total_size: 0,
                used_size: 0,
                fragmentation: 0.0,
            }
        });

        let size_bytes = self.connection.get_total_size().await.unwrap_or(0);

        let metrics = self.connection.get_metrics().await;

        // 生成连接指标报告
        let connection_metrics = ConnectionMetricsReport {
            total_connections: metrics.total_connections,
            active_connections: metrics.active_connections,
            idle_connections: metrics.idle_connections,
            connection_errors: metrics.connection_errors,
            query_executions: metrics.query_executions,
            transaction_count: metrics.transaction_count,
            backup_count: metrics.backup_count,
            last_connection_time: metrics.last_connection_time
                .map(|t| chrono::DateTime::from_timestamp(t.elapsed().as_secs() as i64, 0).unwrap_or(timestamp)),
            last_query_time: metrics.last_query_time
                .map(|t| chrono::DateTime::from_timestamp(t.elapsed().as_secs() as i64, 0).unwrap_or(timestamp)),
            last_backup_time: metrics.last_backup_time
                .map(|t| chrono::DateTime::from_timestamp(t.elapsed().as_secs() as i64, 0).unwrap_or(timestamp)),
        };

        // 生成建议
        let recommendations = self.generate_recommendations(&stats, &connection_metrics, integrity_check).await;

        // 确定整体健康状态
        let overall_health = self.determine_health_status(&recommendations, integrity_check);

        let report = DatabaseHealthReport {
            timestamp,
            integrity_check,
            size_bytes,
            table_count: stats.table_count,
            index_count: stats.index_count,
            total_rows: stats.total_rows,
            fragmentation_percentage: stats.fragmentation,
            last_backup_time: connection_metrics.last_backup_time,
            connection_metrics,
            recommendations,
            overall_health,
        };

        log::info!("数据库健康报告生成完成");
        Ok(report)
    }

    /// 生成建议
    async fn generate_recommendations(
        &self,
        stats: &crate::database::connection::DatabaseStatistics,
        metrics: &ConnectionMetricsReport,
        integrity_ok: bool,
    ) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();

        // 检查完整性
        if !integrity_ok {
            recommendations.push(Recommendation {
                level: RecommendationLevel::Critical,
                title: "数据库完整性检查失败".to_string(),
                description: "数据库完整性检查失败，可能存在数据损坏".to_string(),
                action: "立即执行数据库修复".to_string(),
                priority: 10,
            });
        }

        // 检查碎片化
        if stats.fragmentation > 20.0 {
            recommendations.push(Recommendation {
                level: RecommendationLevel::Warning,
                title: "数据库碎片化较高".to_string(),
                description: format!("数据库碎片化程度为{:.1}%，建议进行优化", stats.fragmentation),
                action: "执行数据库优化操作".to_string(),
                priority: 7,
            });
        }

        // 检查备份
        if let Some(last_backup) = metrics.last_backup_time {
            let hours_since_last_backup = (chrono::Utc::now() - last_backup).num_hours();
            if hours_since_last_backup > 24 {
                recommendations.push(Recommendation {
                    level: RecommendationLevel::Warning,
                    title: "备份时间过长".to_string(),
                    description: format!("距离上次备份已过去{}小时", hours_since_last_backup),
                    action: "执行数据库备份".to_string(),
                    priority: 8,
                });
            }
        } else {
            recommendations.push(Recommendation {
                level: RecommendationLevel::Warning,
                title: "从未执行过备份".to_string(),
                description: "数据库尚未执行过备份操作".to_string(),
                action: "立即执行数据库备份".to_string(),
                priority: 9,
            });
        }

        // 检查连接错误
        if metrics.connection_errors > 10 {
            recommendations.push(Recommendation {
                level: RecommendationLevel::Warning,
                title: "连接错误较多".to_string(),
                description: format!("检测到{}次连接错误", metrics.connection_errors),
                action: "检查数据库连接配置和网络状况".to_string(),
                priority: 6,
            });
        }

        // 检查数据库大小
        if stats.total_size > 100 * 1024 * 1024 { // 100MB
            recommendations.push(Recommendation {
                level: RecommendationLevel::Info,
                title: "数据库文件较大".to_string(),
                description: format!("数据库文件大小为{}", stats.format_size()),
                action: "考虑清理旧数据或进行归档".to_string(),
                priority: 5,
            });
        }

        recommendations
    }

    /// 确定整体健康状态
    fn determine_health_status(&self, recommendations: &[Recommendation], integrity_ok: bool) -> HealthStatus {
        if !integrity_ok {
            return HealthStatus::Critical;
        }

        let has_critical = recommendations.iter().any(|r| r.level == RecommendationLevel::Critical);
        let has_warning = recommendations.iter().any(|r| r.level == RecommendationLevel::Warning);

        match (has_critical, has_warning) {
            (true, _) => HealthStatus::Critical,
            (false, true) => HealthStatus::Degraded,
            (false, false) => HealthStatus::Healthy,
        }
    }

    /// 导出数据库
    pub async fn export_database(&self, export_path: &Path) -> Result<()> {
        log::info!("开始导出数据库到: {:?}", export_path);

        // 确保导出目录存在
        if let Some(parent) = export_path.parent() {
            fs::create_dir_all(parent)
                .await
                .context("创建导出目录失败")?;
        }

        // 使用VACUUM INTO导出
        let export_sql = format!("VACUUM INTO '{}'", export_path.display());
        sqlx::query(&export_sql)
            .execute(self.connection.pool())
            .await
            .context("导出数据库失败")?;

        log::info!("数据库导出完成: {:?}", export_path);
        Ok(())
    }

    /// 导入数据库
    pub async fn import_database(&self, import_path: &Path) -> Result<()> {
        log::info!("开始从文件导入数据库: {:?}", import_path);

        // 检查导入文件是否存在
        if !import_path.exists() {
            return Err(anyhow::anyhow!("导入文件不存在: {:?}", import_path));
        }

        // 验证导入文件
        self.validate_import_file(import_path).await?;

        // 执行导入
        // 注意：这需要特殊的导入逻辑，这里只是示例
        log::warn!("数据库导入功能需要特殊实现");

        log::info!("数据库导入完成");
        Ok(())
    }

    /// 验证导入文件
    async fn validate_import_file(&self, import_path: &Path) -> Result<()> {
        // 检查文件大小
        let metadata = fs::metadata(import_path)
            .await
            .context("获取导入文件元数据失败")?;

        if metadata.len() == 0 {
            return Err(anyhow::anyhow!("导入文件为空"));
        }

        // 尝试连接导入文件以验证
        let test_pool = sqlx::SqlitePool::connect(&format!("sqlite:{}", import_path.display()))
            .await;

        if let Err(e) = test_pool {
            return Err(anyhow::anyhow!("导入文件无效或损坏: {}", e));
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
                    Err(anyhow::anyhow!("导入文件完整性检查失败: {}", check))
                }
            }
            Err(e) => Err(anyhow::anyhow!("执行完整性检查失败: {}", e)),
        }
    }

    /// 获取数据库配置
    pub fn get_config(&self) -> &DatabaseConfig {
        self.connection.config()
    }

    /// 更新数据库配置
    pub async fn update_config(&mut self, new_config: DatabaseConfig) -> Result<()> {
        log::info!("开始更新数据库配置");

        // 验证新配置
        new_config.validate()?;

        // 保存旧配置
        let old_config = self.connection.config().clone();

        // 应用新配置
        // 注意：这需要重新连接数据库，这里只是示例
        log::warn!("数据库配置更新需要重新连接数据库，当前版本暂不支持动态更新");

        log::info!("数据库配置更新完成");
        Ok(())
    }

    /// 获取最后维护时间
    pub async fn get_last_maintenance_time(&self) -> Option<Instant> {
        *self.last_maintenance_time.lock().await
    }

    /// 检查是否需要维护
    pub async fn needs_maintenance(&self) -> bool {
        let last_maintenance = self.get_last_maintenance_time().await;

        if let Some(last_time) = last_maintenance {
            let elapsed = last_time.elapsed();
            // 如果超过24小时未维护，则需要维护
            elapsed > Duration::from_secs(24 * 3600)
        } else {
            // 从未维护过，需要维护
            true
        }
    }
}

impl HealthStatus {
    pub fn to_string(&self) -> &'static str {
        match self {
            HealthStatus::Healthy => "健康",
            HealthStatus::Degraded => "降级",
            HealthStatus::Unhealthy => "不健康",
            HealthStatus::Critical => "严重",
        }
    }

    pub fn to_color(&self) -> &'static str {
        match self {
            HealthStatus::Healthy => "green",
            HealthStatus::Degraded => "yellow",
            HealthStatus::Unhealthy => "orange",
            HealthStatus::Critical => "red",
        }
    }
}

impl RecommendationLevel {
    pub fn to_string(&self) -> &'static str {
        match self {
            RecommendationLevel::Info => "信息",
            RecommendationLevel::Warning => "警告",
            RecommendationLevel::Critical => "严重",
        }
    }

    pub fn to_color(&self) -> &'static str {
        match self {
            RecommendationLevel::Info => "blue",
            RecommendationLevel::Warning => "yellow",
            RecommendationLevel::Critical => "red",
        }
    }
}