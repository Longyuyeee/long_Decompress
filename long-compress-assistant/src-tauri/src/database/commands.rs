use anyhow::{Context, Result};
use crate::database::connection::{get_connection, DatabaseStatistics, DatabaseError};
use crate::database::management::DatabaseManagementService;
use serde::{Deserialize, Serialize};
use std::path::Path;
use log;

#[derive(Debug, Deserialize)]
pub struct BackupRequest {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct RestoreRequest {
    pub path: String,
    pub password: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DatabaseStatus {
    pub integrity_ok: bool,
    pub size_bytes: u64,
    pub statistics: DatabaseStatistics,
}

/// 获取数据库状态
#[tauri::command]
pub async fn get_database_status() -> Result<DatabaseStatus, String> {
    let connection = get_connection().await
        .map_err(|e| format!("获取数据库连接失败: {}", e))?;

    let integrity_ok = connection.check_integrity().await
        .unwrap_or(false);

    let size_bytes = connection.get_total_size().await
        .unwrap_or(0);

    let stats = connection.get_statistics().await
        .unwrap_or_else(|_| DatabaseStatistics {
            page_size: 4096,
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

    Ok(DatabaseStatus {
        integrity_ok,
        size_bytes,
        statistics: stats,
    })
}

/// 优化数据库
#[tauri::command]
pub async fn optimize_database() -> Result<(), String> {
    let connection = get_connection().await
        .map_err(|e| format!("获取数据库连接失败: {}", e))?;

    connection.optimize().await
        .map_err(|e| format!("优化数据库失败: {}", e))
}

/// 备份数据库
#[tauri::command]
pub async fn backup_database(request: BackupRequest) -> Result<String, String> {
    let connection = get_connection().await
        .map_err(|e| format!("获取数据库连接失败: {}", e))?;

    let backup_path = Path::new(&request.path);
    connection.backup(backup_path).await
        .map_err(|e| format!("备份数据库失败: {}", e))?;

    Ok(request.path)
}

/// 恢复数据库
#[tauri::command]
pub async fn restore_database(request: RestoreRequest) -> Result<(), String> {
    let connection = get_connection().await
        .map_err(|e| format!("获取数据库连接失败: {}", e))?;

    let backup_path = Path::new(&request.path);
    connection.restore(backup_path, request.password.as_deref()).await
        .map_err(|e| format!("恢复数据库失败: {}", e))
}

/// 运行数据库维护
#[tauri::command]
pub async fn run_database_maintenance() -> Result<bool, String> {
    let connection = get_connection().await
        .map_err(|e| format!("获取数据库连接失败: {}", e))?;

    let mut management_service = DatabaseManagementService::new(connection.clone());
    
    match management_service.run_maintenance_task().await {
        Ok(_) => Ok(true),
        Err(e) => Err(format!("运行维护任务失败: {}", e)),
    }
}

/// 获取维护报告
#[tauri::command]
pub async fn get_maintenance_report() -> Result<Option<String>, String> {
    let connection = get_connection().await
        .map_err(|e| format!("获取数据库连接失败: {}", e))?;

    let management_service = DatabaseManagementService::new(connection.clone());
    
    // 假设有获取最新报告的方法
    Ok(None)
}

/// 获取数据库性能指标
#[tauri::command]
pub async fn get_database_metrics() -> Result<crate::database::connection::DatabaseMetrics, String> {
    let connection = get_connection().await
        .map_err(|e| format!("获取数据库连接失败: {}", e))?;

    Ok(connection.get_metrics().await)
}

/// 检查并修复数据库
#[tauri::command]
pub async fn check_and_repair_database() -> Result<bool, String> {
    let connection = get_connection().await
        .map_err(|e| format!("获取数据库连接失败: {}", e))?;

    connection.check_and_repair().await
        .map_err(|e| format!("检查并修复数据库失败: {}", e))
}

/// 重新初始化数据库
#[tauri::command]
pub async fn reinitialize_database() -> Result<bool, String> {
    log::info!("重新初始化数据库");

    crate::database::connection::reinitialize().await
        .map_err(|e: anyhow::Error| format!("重新初始化数据库失败: {}", e))?;

    log::info!("数据库重新初始化成功");
    Ok(true)
}
