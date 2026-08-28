use crate::services::video_engine::{validate_video_engine, VideoEngineStatus};
use crate::services::video_probe::{probe_video_file, VideoProbeReport};
use std::path::Path;

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

#[tauri::command]
pub async fn probe_video_input(
    app: tauri::AppHandle,
    path: String,
) -> Result<VideoProbeReport, String> {
    let resource_root = app
        .path_resolver()
        .resource_dir()
        .ok_or_else(|| "VIDEO_ENGINE_RESOURCE_DIRECTORY_UNAVAILABLE".to_string())?;
    let validation_root = resource_root.clone();
    tauri::async_runtime::spawn_blocking(move || validate_video_engine(&validation_root))
        .await
        .map_err(|error| format!("VIDEO_ENGINE_PREFLIGHT_JOIN_FAILED: {error}"))?
        .map_err(|error| error.to_string())?;
    let ffprobe = resource_root.join("video-engine").join("ffprobe.exe");
    probe_video_file(&ffprobe, Path::new(&path))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn plan_video_compression(
    app: tauri::AppHandle,
    request: VideoCompressionPlanRequest,
) -> Result<VideoCompressionPlan, String> {
    let resource_root = app
        .path_resolver()
        .resource_dir()
        .ok_or_else(|| "VIDEO_ENGINE_RESOURCE_DIRECTORY_UNAVAILABLE".to_string())?;
    let validation_root = resource_root.clone();
    tauri::async_runtime::spawn_blocking(move || validate_video_engine(&validation_root))
        .await
        .map_err(|error| format!("VIDEO_ENGINE_PREFLIGHT_JOIN_FAILED: {error}"))?
        .map_err(|error| error.to_string())?;
    let ffprobe = resource_root.join("video-engine").join("ffprobe.exe");
    let probe = probe_video_file(&ffprobe, Path::new(&request.path))
        .await
        .map_err(|error| error.to_string())?;
    build_video_compression_plan(probe, &request).map_err(|error| error.to_string())
}
use crate::services::video_compression_plan::{
    build_video_compression_plan, VideoCompressionPlan, VideoCompressionPlanRequest,
};
