use crate::models::decompression_profile::DecompressionProfile;
use crate::services::decompression_profile_service::DecompressionProfileService;
use tauri::State;

#[tauri::command]
pub async fn get_all_decompression_profiles(
    service: State<'_, DecompressionProfileService>,
) -> Result<Vec<DecompressionProfile>, String> {
    service
        .get_all_profiles()
        .await
        .map_err(|e| format!("获取解压配置组失败: {}", e))
}

#[tauri::command]
pub async fn get_decompression_profile_by_id(
    service: State<'_, DecompressionProfileService>,
    id: String,
) -> Result<Option<DecompressionProfile>, String> {
    service
        .get_profile_by_id(&id)
        .await
        .map_err(|e| format!("获取解压配置组失败: {}", e))
}

#[tauri::command]
pub async fn create_decompression_profile(
    service: State<'_, DecompressionProfileService>,
    profile: DecompressionProfile,
) -> Result<(), String> {
    service
        .create_profile(profile)
        .await
        .map_err(|e| format!("创建解压配置组失败: {}", e))
}

#[tauri::command]
pub async fn update_decompression_profile(
    service: State<'_, DecompressionProfileService>,
    profile: DecompressionProfile,
) -> Result<(), String> {
    service
        .update_profile(profile)
        .await
        .map_err(|e| format!("更新解压配置组失败: {}", e))
}

#[tauri::command]
pub async fn delete_decompression_profile(
    service: State<'_, DecompressionProfileService>,
    id: String,
) -> Result<(), String> {
    service
        .delete_profile(&id)
        .await
        .map_err(|e| format!("删除解压配置组失败: {}", e))
}

#[tauri::command]
pub async fn update_decompression_profile_stats(
    service: State<'_, DecompressionProfileService>,
    id: String,
    success: bool,
    files_processed: u64,
    bytes_processed: u64,
    extraction_time: f64,
) -> Result<(), String> {
    service
        .update_stats(&id, success, files_processed, bytes_processed, extraction_time)
        .await
        .map_err(|e| format!("更新解压配置组统计失败: {}", e))
}
