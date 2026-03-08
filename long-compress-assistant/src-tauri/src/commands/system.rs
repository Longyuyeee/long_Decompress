use crate::services::system_service::{SystemService, SystemInfo};
use tauri::command;

#[command]
pub async fn get_system_info() -> Result<SystemInfo, String> {
    let mut service = SystemService::new();
    match service.get_system_info() {
        Ok(info) => Ok(info),
        Err(e) => Err(format!("获取系统信息失败: {}", e)),
    }
}

#[command]
pub async fn get_disk_space(path: String) -> Result<(u64, u64), String> {
    match std::fs::metadata(&path) {
        Ok(_) => {
            // 简化实现，实际应该使用更精确的磁盘空间检查
            let total = 1024 * 1024 * 1024 * 100; // 假设100GB
            let free = 1024 * 1024 * 1024 * 50;   // 假设50GB空闲
            Ok((total, free))
        }
        Err(e) => Err(format!("获取磁盘空间失败: {}", e)),
    }
}

#[command]
pub async fn get_app_version() -> Result<String, String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}