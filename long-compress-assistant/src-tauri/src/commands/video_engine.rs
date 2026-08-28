use crate::services::video_engine::{validate_video_engine, VideoEngineStatus};

#[tauri::command]
pub async fn preflight_video_engine(app: tauri::AppHandle) -> Result<VideoEngineStatus, String> {
    let resource_root = app
        .path_resolver()
        .resource_dir()
        .ok_or_else(|| "VIDEO_ENGINE_RESOURCE_DIRECTORY_UNAVAILABLE".to_string())?;
    tauri::async_runtime::spawn_blocking(move || validate_video_engine(&resource_root))
        .await
        .map_err(|error| format!("VIDEO_ENGINE_PREFLIGHT_JOIN_FAILED: {error}"))?
        .map_err(|error| error.to_string())
}
