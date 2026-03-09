use crate::services::file_service::{FileService, FileInfo, FileServiceError, HashAlgorithm};
use tauri::command;

#[command]
pub async fn list_files(path: String) -> Result<Vec<FileInfo>, String> {
    let service = FileService::new(crate::services::file_service::FileServiceConfig::default());
    service.list_files(&path, false)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_file_info(path: String) -> Result<FileInfo, String> {
    let service = FileService::new(crate::services::file_service::FileServiceConfig::default());
    service.get_file_info(&path)
        .await
        .map_err(|e| e.to_string())
}
