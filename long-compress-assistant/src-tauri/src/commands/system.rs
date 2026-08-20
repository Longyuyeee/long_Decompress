use crate::services::system_service::{SystemService, SystemInfo};
use tauri::{command, AppHandle};
use serde::Serialize;
use sysinfo::System;

#[command]
pub async fn get_system_info() -> Result<SystemInfo, String> {
    let mut service = SystemService::new();
    Ok(service.get_system_info())
}

#[derive(Debug, Serialize)]
pub struct ResourceUsage {
    pub cpu_usage: f32,
    pub memory_usage: f32,
}

#[command]
pub async fn get_resource_usage() -> Result<ResourceUsage, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut system = System::new_all();
        system.refresh_cpu();
        system.refresh_memory();
        let total_memory = system.total_memory();
        let memory_usage = if total_memory == 0 {
            0.0
        } else {
            system.used_memory() as f32 / total_memory as f32 * 100.0
        };
        ResourceUsage {
            cpu_usage: system.global_cpu_info().cpu_usage(),
            memory_usage,
        }
    })
    .await
    .map_err(|error| error.to_string())
}

#[command]
pub async fn get_disk_space(path: String) -> Result<(u64, u64), String> {
    let target = crate::services::storage_preflight::probe_storage(std::path::Path::new(&path));
    target
        .total_bytes
        .zip(target.available_bytes)
        .ok_or_else(|| "Unable to determine disk space for the selected path".to_string())
}

#[command]
pub async fn preflight_operation_resources(
    operation: String,
    output_path: String,
    source_paths: Vec<String>,
    password: Option<String>,
    estimated_output_bytes: Option<u64>,
    estimate_reliable: Option<bool>,
) -> Result<crate::services::storage_preflight::ResourcePreflightReport, String> {
    if source_paths.len() > 1_000 {
        return Err("Resource preflight accepts at most 1000 explicit source paths".to_string());
    }
    crate::services::storage_preflight::preflight_operation_resources(
        &operation,
        &output_path,
        &source_paths,
        password.as_deref(),
        estimated_output_bytes,
        estimate_reliable.unwrap_or(false),
    )
    .await
    .map_err(|error| error.to_string())
}

#[command]
pub async fn get_app_version() -> Result<String, String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

/// 从数据目录加载持久化的应用设置 (JSON)
#[command]
pub async fn load_app_settings(app: AppHandle) -> Result<String, String> {
    let resolver = app.path_resolver();
    let data_dir = resolver.app_data_dir()
        .ok_or_else(|| "无法获取数据目录".to_string())?;
    let settings_path = data_dir.join("app_settings.json");
    if !settings_path.exists() {
        return Ok("{}".to_string());
    }
    std::fs::read_to_string(&settings_path)
        .map_err(|e| format!("读取设置文件失败: {}", e))
}

/// 将应用设置持久化到数据目录 (JSON)
#[command]
pub async fn save_app_settings(app: AppHandle, settings_json: String) -> Result<(), String> {
    let resolver = app.path_resolver();
    let data_dir = resolver.app_data_dir()
        .ok_or_else(|| "无法获取数据目录".to_string())?;
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("创建数据目录失败: {}", e))?;
    let settings_path = data_dir.join("app_settings.json");
    std::fs::write(&settings_path, &settings_json)
        .map_err(|e| format!("保存设置文件失败: {}", e))
}
