use anyhow::Result;
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::database::connection::{get_connection, DatabaseConnection};
use crate::database::management::{DatabaseManager, DatabaseHealthReport, MaintenanceReport};
use crate::database::config::DatabaseConfig;

/// 数据库状态响应
#[derive(Debug, Serialize)]
pub struct DatabaseStatusResponse {
    pub initialized: bool,
    pub integrity_ok: bool,
    pub size_bytes: u64,
    pub table_count: u64,
    pub last_backup_time: Option<String>,
    pub health_status: String,
}

/// 备份请求
#[derive(Debug, Deserialize)]
pub struct BackupRequest {
    pub backup_path: String,
    pub description: Option<String>,
}

/// 备份响应
#[derive(Debug, Serialize)]
pub struct BackupResponse {
    pub success: bool,
    pub backup_path: String,
    pub size_bytes: u64,
    pub timestamp: String,
}

/// 恢复请求
#[derive(Debug, Deserialize)]
pub struct RestoreRequest {
    pub backup_path: String,
    pub password: Option<String>,
}

/// 恢复响应
#[derive(Debug, Serialize)]
pub struct RestoreResponse {
    pub success: bool,
    pub message: String,
    pub timestamp: String,
}

/// 优化请求
#[derive(Debug, Deserialize)]
pub struct OptimizeRequest {
    pub vacuum: bool,
    pub analyze: bool,
    pub rebuild_indexes: bool,
}

/// 优化响应
#[derive(Debug, Serialize)]
pub struct OptimizeResponse {
    pub success: bool,
    pub duration_seconds: f64,
    pub size_before: u64,
    pub size_after: u64,
    pub fragmentation_before: f64,
    pub fragmentation_after: f64,
}

/// 获取数据库状态
#[tauri::command]
pub async fn get_database_status() -> Result<DatabaseStatusResponse, String> {
    log::info!("获取数据库状态");

    let connection = get_connection().await
        .map_err(|e| format!("获取数据库连接失败: {}", e))?;

    let integrity_ok = connection.check_integrity().await
        .unwrap_or(false);

    let size_bytes = connection.get_size().await
        .unwrap_or(0);

    let stats = connection.get_statistics().await
        .unwrap_or_else(|_| crate::database::connection::DatabaseStatistics {
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
        });

    let metrics = connection.get_metrics().await;
    let last_backup_time = metrics.last_backup_time
        .map(|t| chrono::DateTime::from_timestamp(t.elapsed().as_secs() as i64, 0)
            .unwrap_or_else(|| chrono::Utc::now())
            .to_rfc3339());

    // 创建数据库管理器以获取健康状态
    let manager = DatabaseManager::new(connection.as_ref().clone());
    let health_report = manager.get_health_report().await
        .unwrap_or_else(|_| DatabaseHealthReport {
            timestamp: chrono::Utc::now(),
            integrity_check: false,
            size_bytes: 0,
            table_count: 0,
            index_count: 0,
            total_rows: 0,
            fragmentation_percentage: 0.0,
            last_backup_time: None,
            connection_metrics: crate::database::management::ConnectionMetricsReport {
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
            },
            recommendations: Vec::new(),
            overall_health: crate::database::management::HealthStatus::Unhealthy,
        });

    Ok(DatabaseStatusResponse {
        initialized: true,
        integrity_ok,
        size_bytes,
        table_count: stats.table_count,
        last_backup_time,
        health_status: health_report.overall_health.to_string().to_string(),
    })
}

/// 执行数据库备份
#[tauri::command]
pub async fn backup_database(request: BackupRequest) -> Result<BackupResponse, String> {
    log::info!("执行数据库备份: {:?}", request.backup_path);

    let connection = get_connection().await
        .map_err(|e| format!("获取数据库连接失败: {}", e))?;

    let backup_path = std::path::Path::new(&request.backup_path);

    // 执行备份
    connection.backup(backup_path).await
        .map_err(|e| format!("备份数据库失败: {}", e))?;

    // 获取备份文件信息
    let metadata = tokio::fs::metadata(backup_path).await
        .map_err(|e| format!("获取备份文件信息失败: {}", e))?;

    let timestamp = chrono::Utc::now().to_rfc3339();

    Ok(BackupResponse {
        success: true,
        backup_path: request.backup_path,
        size_bytes: metadata.len(),
        timestamp,
    })
}

/// 执行数据库恢复
#[tauri::command]
pub async fn restore_database(request: RestoreRequest) -> Result<RestoreResponse, String> {
    log::info!("执行数据库恢复: {:?}", request.backup_path);

    let mut connection = get_connection().await
        .map_err(|e| format!("获取数据库连接失败: {}", e))?
        .as_ref()
        .clone();

    let backup_path = std::path::Path::new(&request.backup_path);

    // 执行恢复
    connection.restore(backup_path, request.password.as_deref()).await
        .map_err(|e| format!("恢复数据库失败: {}", e))?;

    let timestamp = chrono::Utc::now().to_rfc3339();

    Ok(RestoreResponse {
        success: true,
        message: "数据库恢复成功".to_string(),
        timestamp,
    })
}

/// 执行数据库优化
#[tauri::command]
pub async fn optimize_database(request: OptimizeRequest) -> Result<OptimizeResponse, String> {
    log::info!("执行数据库优化");

    let connection = get_connection().await
        .map_err(|e| format!("获取数据库连接失败: {}", e))?;

    let start_time = std::time::Instant::now();

    // 获取优化前的状态
    let stats_before = connection.get_statistics().await
        .map_err(|e| format!("获取优化前统计信息失败: {}", e))?;

    let size_before = connection.get_total_size().await
        .unwrap_or(0);

    // 执行优化操作
    if request.vacuum {
        sqlx::query("VACUUM;")
            .execute(connection.pool())
            .await
            .map_err(|e| format!("执行VACUUM失败: {}", e))?;
        log::info!("已执行VACUUM优化");
    }

    if request.analyze {
        sqlx::query("ANALYZE;")
            .execute(connection.pool())
            .await
            .map_err(|e| format!("执行ANALYZE失败: {}", e))?;
        log::info!("已更新统计信息");
    }

    if request.rebuild_indexes {
        // 重建所有索引
        let indexes: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='index' AND name NOT LIKE 'sqlite_%'"
        )
            .fetch_all(connection.pool())
            .await
            .map_err(|e| format!("获取索引列表失败: {}", e))?;

        for (index_name,) in indexes {
            let rebuild_sql = format!("REINDEX {}", index_name);
            sqlx::query(&rebuild_sql)
                .execute(connection.pool())
                .await
                .map_err(|e| format!("重建索引 {} 失败: {}", index_name, e))?;
            log::debug!("已重建索引: {}", index_name);
        }
        log::info!("已重建所有索引");
    }

    // 执行标准优化
    connection.optimize().await
        .map_err(|e| format!("执行优化失败: {}", e))?;

    // 获取优化后的状态
    let stats_after = connection.get_statistics().await
        .map_err(|e| format!("获取优化后统计信息失败: {}", e))?;

    let size_after = connection.get_total_size().await
        .unwrap_or(0);

    let duration = start_time.elapsed().as_secs_f64();

    Ok(OptimizeResponse {
        success: true,
        duration_seconds: duration,
        size_before,
        size_after,
        fragmentation_before: stats_before.fragmentation,
        fragmentation_after: stats_after.fragmentation,
    })
}

/// 获取数据库健康报告
#[tauri::command]
pub async fn get_database_health_report() -> Result<DatabaseHealthReport, String> {
    log::info!("获取数据库健康报告");

    let connection = get_connection().await
        .map_err(|e| format!("获取数据库连接失败: {}", e))?;

    let manager = DatabaseManager::new(connection.as_ref().clone());
    let report = manager.get_health_report().await
        .map_err(|e| format!("生成健康报告失败: {}", e))?;

    Ok(report)
}

/// 执行数据库维护
#[tauri::command]
pub async fn perform_database_maintenance() -> Result<MaintenanceReport, String> {
    log::info!("执行数据库维护");

    let connection = get_connection().await
        .map_err(|e| format!("获取数据库连接失败: {}", e))?;

    let manager = DatabaseManager::new(connection.as_ref().clone());
    let report = manager.perform_maintenance().await
        .map_err(|e| format!("执行维护失败: {}", e))?;

    Ok(report)
}

/// 导出数据库
#[tauri::command]
pub async fn export_database(export_path: String) -> Result<BackupResponse, String> {
    log::info!("导出数据库: {}", export_path);

    let connection = get_connection().await
        .map_err(|e| format!("获取数据库连接失败: {}", e))?;

    let manager = DatabaseManager::new(connection.as_ref().clone());
    let export_path_obj = std::path::Path::new(&export_path);

    manager.export_database(export_path_obj).await
        .map_err(|e| format!("导出数据库失败: {}", e))?;

    // 获取导出文件信息
    let metadata = tokio::fs::metadata(export_path_obj).await
        .map_err(|e| format!("获取导出文件信息失败: {}", e))?;

    let timestamp = chrono::Utc::now().to_rfc3339();

    Ok(BackupResponse {
        success: true,
        backup_path: export_path,
        size_bytes: metadata.len(),
        timestamp,
    })
}

/// 获取数据库配置
#[tauri::command]
pub async fn get_database_config() -> Result<DatabaseConfig, String> {
    log::info!("获取数据库配置");

    let connection = get_connection().await
        .map_err(|e| format!("获取数据库连接失败: {}", e))?;

    let config = connection.config().clone();

    Ok(config)
}

/// 检查数据库连接
#[tauri::command]
pub async fn check_database_connection() -> Result<bool, String> {
    log::info!("检查数据库连接");

    match get_connection().await {
        Ok(connection) => {
            // 尝试执行简单查询
            let result: Result<(i64,), _> = sqlx::query_as("SELECT 1")
                .fetch_one(connection.pool())
                .await;

            match result {
                Ok((value,)) => {
                    let ok = value == 1;
                    if ok {
                        log::info!("数据库连接正常");
                    } else {
                        log::warn!("数据库连接检查返回异常值: {}", value);
                    }
                    Ok(ok)
                }
                Err(e) => {
                    log::error!("数据库连接检查失败: {}", e);
                    Err(format!("数据库连接检查失败: {}", e))
                }
            }
        }
        Err(e) => {
            log::error!("获取数据库连接失败: {}", e);
            Err(format!("获取数据库连接失败: {}", e))
        }
    }
}

/// 重新初始化数据库
#[tauri::command]
pub async fn reinitialize_database() -> Result<bool, String> {
    log::info!("重新初始化数据库");

    use crate::database::connection::reinitialize;

    reinitialize().await
        .map_err(|e| format!("重新初始化数据库失败: {}", e))?;

    log::info!("数据库重新初始化成功");
    Ok(true)
}