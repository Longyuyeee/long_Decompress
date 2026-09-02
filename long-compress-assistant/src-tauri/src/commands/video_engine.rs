use crate::services::video_engine::{
    bundled_resource_root, validate_video_engine, VideoEngineStatus,
};
use crate::services::video_probe::{probe_video_file, VideoProbeReport};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;

const VIDEO_PROBE_CACHE_LIMIT: usize = 64;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct VideoProbeCacheKey {
    path: String,
    bytes: u64,
    modified_millis: u128,
}

fn video_probe_cache() -> &'static Mutex<HashMap<VideoProbeCacheKey, VideoProbeReport>> {
    static CACHE: OnceLock<Mutex<HashMap<VideoProbeCacheKey, VideoProbeReport>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn video_probe_cache_key(path: &Path) -> Result<VideoProbeCacheKey, String> {
    let metadata = path
        .metadata()
        .map_err(|error| format!("VIDEO_SOURCE_METADATA_FAILED: {error}"))?;
    let modified_millis = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_millis())
        .unwrap_or_default();
    Ok(VideoProbeCacheKey {
        path: path.to_string_lossy().replace('/', "\\").to_lowercase(),
        bytes: metadata.len(),
        modified_millis,
    })
}

fn cached_video_probe(key: &VideoProbeCacheKey) -> Option<VideoProbeReport> {
    video_probe_cache().lock().ok()?.get(key).cloned()
}

fn cache_video_probe(key: VideoProbeCacheKey, probe: VideoProbeReport) {
    if let Ok(mut cache) = video_probe_cache().lock() {
        if cache.len() >= VIDEO_PROBE_CACHE_LIMIT && !cache.contains_key(&key) {
            cache.clear();
        }
        cache.insert(key, probe);
    }
}

#[tauri::command]
pub async fn preflight_video_engine(app: tauri::AppHandle) -> Result<VideoEngineStatus, String> {
    let app_resource_dir = app
        .path_resolver()
        .resource_dir()
        .ok_or_else(|| "VIDEO_ENGINE_RESOURCE_DIRECTORY_UNAVAILABLE".to_string())?;
    let resource_root = bundled_resource_root(&app_resource_dir);
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
    let app_resource_dir = app
        .path_resolver()
        .resource_dir()
        .ok_or_else(|| "VIDEO_ENGINE_RESOURCE_DIRECTORY_UNAVAILABLE".to_string())?;
    let resource_root = bundled_resource_root(&app_resource_dir);
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
    let source = Path::new(&request.path);
    let cache_key = video_probe_cache_key(source)?;
    if let Some(probe) = cached_video_probe(&cache_key) {
        return build_video_compression_plan(probe, &request).map_err(|error| error.to_string());
    }
    let app_resource_dir = app
        .path_resolver()
        .resource_dir()
        .ok_or_else(|| "VIDEO_ENGINE_RESOURCE_DIRECTORY_UNAVAILABLE".to_string())?;
    let resource_root = bundled_resource_root(&app_resource_dir);
    let validation_root = resource_root.clone();
    tauri::async_runtime::spawn_blocking(move || validate_video_engine(&validation_root))
        .await
        .map_err(|error| format!("VIDEO_ENGINE_PREFLIGHT_JOIN_FAILED: {error}"))?
        .map_err(|error| error.to_string())?;
    let ffprobe = resource_root.join("video-engine").join("ffprobe.exe");
    let probe = probe_video_file(&ffprobe, source)
        .await
        .map_err(|error| error.to_string())?;
    cache_video_probe(cache_key, probe.clone());
    build_video_compression_plan(probe, &request).map_err(|error| error.to_string())
}
use crate::services::video_compression_plan::{
    build_video_compression_plan, VideoCompressionPlan, VideoCompressionPlanRequest,
};
