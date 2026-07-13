use crate::models::compression_profile::CompressionProfile;
use crate::services::compression_profile_service::CompressionProfileService;
use tauri::{command, State};
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::Result;

/// 应用状态中的配置组服务
pub struct CompressionProfileServiceState {
    pub service: Arc<Mutex<Option<CompressionProfileService>>>,
}

impl Default for CompressionProfileServiceState {
    fn default() -> Self {
        Self::new()
    }
}

impl CompressionProfileServiceState {
    pub fn new() -> Self {
        Self {
            service: Arc::new(Mutex::new(None)),
        }
    }
}

/// 获取所有配置组
#[command]
pub async fn get_compression_profiles(
    state: State<'_, CompressionProfileServiceState>,
) -> Result<Vec<CompressionProfile>, String> {
    let service_lock = state.service.lock().await;
    let service = service_lock
        .as_ref()
        .ok_or_else(|| "配置组服务未初始化".to_string())?;

    service
        .get_all_profiles()
        .await
        .map_err(|e| format!("获取配置组列表失败: {}", e))
}

/// 根据 ID 获取配置组
#[command]
pub async fn get_compression_profile(
    state: State<'_, CompressionProfileServiceState>,
    id: String,
) -> Result<Option<CompressionProfile>, String> {
    let service_lock = state.service.lock().await;
    let service = service_lock
        .as_ref()
        .ok_or_else(|| "配置组服务未初始化".to_string())?;

    service
        .get_profile_by_id(&id)
        .await
        .map_err(|e| format!("获取配置组失败: {}", e))
}

/// 创建新配置组
#[command]
pub async fn create_compression_profile(
    state: State<'_, CompressionProfileServiceState>,
    profile: CompressionProfile,
) -> Result<String, String> {
    let service_lock = state.service.lock().await;
    let service = service_lock
        .as_ref()
        .ok_or_else(|| "配置组服务未初始化".to_string())?;

    service
        .create_profile(profile)
        .await
        .map_err(|e| format!("创建配置组失败: {}", e))
}

/// 更新配置组
#[command]
pub async fn update_compression_profile(
    state: State<'_, CompressionProfileServiceState>,
    id: String,
    profile: CompressionProfile,
) -> Result<(), String> {
    let service_lock = state.service.lock().await;
    let service = service_lock
        .as_ref()
        .ok_or_else(|| "配置组服务未初始化".to_string())?;

    service
        .update_profile(&id, profile)
        .await
        .map_err(|e| format!("更新配置组失败: {}", e))
}

/// 删除配置组
#[command]
pub async fn delete_compression_profile(
    state: State<'_, CompressionProfileServiceState>,
    id: String,
) -> Result<(), String> {
    let service_lock = state.service.lock().await;
    let service = service_lock
        .as_ref()
        .ok_or_else(|| "配置组服务未初始化".to_string())?;

    service
        .delete_profile(&id)
        .await
        .map_err(|e| format!("删除配置组失败: {}", e))
}

/// 重新排序配置组
#[command]
pub async fn reorder_compression_profiles(
    state: State<'_, CompressionProfileServiceState>,
    ids: Vec<String>,
) -> Result<(), String> {
    let service_lock = state.service.lock().await;
    let service = service_lock
        .as_ref()
        .ok_or_else(|| "配置组服务未初始化".to_string())?;

    service
        .reorder_profiles(ids)
        .await
        .map_err(|e| format!("重新排序失败: {}", e))
}

/// 应用配置组到任务（更新统计信息）
#[command]
pub async fn apply_compression_profile(
    state: State<'_, CompressionProfileServiceState>,
    profile_id: String,
    success: bool,
    files_count: u64,
    bytes_processed: u64,
) -> Result<(), String> {
    let service_lock = state.service.lock().await;
    let service = service_lock
        .as_ref()
        .ok_or_else(|| "配置组服务未初始化".to_string())?;

    service
        .update_profile_stats(&profile_id, success, files_count, bytes_processed)
        .await
        .map_err(|e| format!("更新配置组统计失败: {}", e))
}

/// 推荐配置组（根据文件路径和大小）
#[command]
pub async fn suggest_compression_profile(
    state: State<'_, CompressionProfileServiceState>,
    file_path: String,
    file_size: u64,
) -> Result<Option<CompressionProfile>, String> {
    let service_lock = state.service.lock().await;
    let service = service_lock
        .as_ref()
        .ok_or_else(|| "配置组服务未初始化".to_string())?;

    service
        .suggest_profile_for_file(&file_path, file_size)
        .await
        .map_err(|e| format!("推荐配置组失败: {}", e))
}
