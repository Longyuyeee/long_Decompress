use tauri::command;
use crate::services::file_service::FileService;
use crate::models::file::FileInfo;
use anyhow::Result;

#[command]
pub async fn list_files(path: String) -> Result<Vec<FileInfo>, String> {
    let service = FileService::new();
    service.list_directory(&std::path::Path::new(&path))
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_file_info(path: String) -> Result<FileInfo, String> {
    let service = FileService::new();
    service.get_file_info(&std::path::Path::new(&path))
        .await
        .map_err(|e| e.to_string())
}
