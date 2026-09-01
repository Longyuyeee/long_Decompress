use crate::services::file_manager::{
    self, FileManagerLocation, FileOperationReport, FileProperties,
};
use std::path::PathBuf;

#[tauri::command]
pub fn file_manager_locations() -> Vec<FileManagerLocation> {
    file_manager::locations()
}

#[tauri::command]
pub async fn file_manager_copy(
    sources: Vec<String>,
    destination: String,
) -> Result<FileOperationReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        file_manager::copy_to_directory(&paths(sources), &PathBuf::from(destination))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn file_manager_move(
    sources: Vec<String>,
    destination: String,
) -> Result<FileOperationReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        file_manager::move_to_directory(&paths(sources), &PathBuf::from(destination))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn file_manager_rename(source: String, new_name: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        file_manager::rename_item(&PathBuf::from(source), &new_name)
            .map(|path| path.to_string_lossy().to_string())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn file_manager_create_directory(parent: String, name: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        file_manager::create_directory(&PathBuf::from(parent), &name)
            .map(|path| path.to_string_lossy().to_string())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn file_manager_recycle(paths: Vec<String>) -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || {
        file_manager::recycle_items(&self::paths(paths)).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn file_manager_properties(path: String) -> Result<FileProperties, String> {
    tauri::async_runtime::spawn_blocking(move || {
        file_manager::properties(&PathBuf::from(path)).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

fn paths(values: Vec<String>) -> Vec<PathBuf> {
    values.into_iter().map(PathBuf::from).collect()
}
