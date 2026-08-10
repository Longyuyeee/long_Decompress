use crate::services::watch_folder_service::{
    WatchFolderDraftBatch, WatchFolderRegistration, WatchFolderService, WatchFolderStatus,
};
use std::sync::Arc;
use tauri::{command, State};
use tokio::sync::Mutex;

pub struct WatchFolderServiceState {
    pub service: Arc<Mutex<Option<WatchFolderService>>>,
}

impl WatchFolderServiceState {
    pub fn new() -> Self {
        Self {
            service: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for WatchFolderServiceState {
    fn default() -> Self {
        Self::new()
    }
}

#[command]
pub async fn list_task_template_watch_folders(
    state: State<'_, WatchFolderServiceState>,
) -> Result<Vec<WatchFolderRegistration>, String> {
    let service = initialized_service(&state).await?;
    service
        .list_watch_folders()
        .await
        .map_err(|error| format!("读取监控目录失败: {error}"))
}

#[command]
pub async fn create_task_template_watch_folder(
    state: State<'_, WatchFolderServiceState>,
    profile_id: String,
    folder_path: String,
) -> Result<WatchFolderRegistration, String> {
    let service = initialized_service(&state).await?;
    service
        .create_watch_folder(&profile_id, &folder_path)
        .await
        .map_err(|error| format!("保存并启动监控目录失败: {error}"))
}

#[command]
pub async fn set_task_template_watch_folder_status(
    state: State<'_, WatchFolderServiceState>,
    id: String,
    status: WatchFolderStatus,
) -> Result<WatchFolderRegistration, String> {
    let service = initialized_service(&state).await?;
    service
        .set_status(&id, status)
        .await
        .map_err(|error| format!("更新监控目录状态失败: {error}"))
}

#[command]
pub async fn delete_task_template_watch_folder(
    state: State<'_, WatchFolderServiceState>,
    id: String,
) -> Result<(), String> {
    let service = initialized_service(&state).await?;
    service
        .delete_watch_folder(&id)
        .await
        .map_err(|error| format!("删除监控目录授权失败: {error}"))
}

#[command]
pub async fn list_pending_task_template_watch_batches(
    state: State<'_, WatchFolderServiceState>,
) -> Result<Vec<WatchFolderDraftBatch>, String> {
    let service = initialized_service(&state).await?;
    service
        .list_pending_batches()
        .await
        .map_err(|error| format!("读取监控草稿批次失败: {error}"))
}

#[command]
pub async fn acknowledge_task_template_watch_batch(
    state: State<'_, WatchFolderServiceState>,
    id: String,
) -> Result<(), String> {
    let service = initialized_service(&state).await?;
    service
        .acknowledge_batch(&id)
        .await
        .map_err(|error| format!("确认监控草稿批次失败: {error}"))
}

async fn initialized_service(
    state: &State<'_, WatchFolderServiceState>,
) -> Result<WatchFolderService, String> {
    state
        .service
        .lock()
        .await
        .as_ref()
        .cloned()
        .ok_or_else(|| "监控目录服务未初始化".to_string())
}
