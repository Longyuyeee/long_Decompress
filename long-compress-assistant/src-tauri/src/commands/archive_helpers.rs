use crate::services::password_dictionary_service::PasswordDictionaryService;
use crate::services::split_archive_detector::SplitArchiveDetector;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct SplitArchiveResponse {
    pub is_split: bool,
    pub format: Option<String>,
    pub base_name: Option<String>,
    pub parts: Vec<String>,
    pub first_part: Option<String>,
    pub total_parts: usize,
    pub total_size: u64,
}

#[tauri::command]
pub async fn detect_split_archive(path: String) -> Result<SplitArchiveResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let info = SplitArchiveDetector::detect_split_archive(Path::new(&path))
            .map_err(|error| error.to_string())?;
        Ok(match info {
            Some(info) => SplitArchiveResponse {
                is_split: true,
                format: Some(format!("{:?}", info.format)),
                base_name: Some(info.base_name),
                parts: info
                    .parts
                    .iter()
                    .map(|part| part.to_string_lossy().into_owned())
                    .collect(),
                first_part: Some(info.first_part.to_string_lossy().into_owned()),
                total_parts: info.total_parts,
                total_size: info.total_size,
            },
            None => SplitArchiveResponse {
                is_split: false,
                format: None,
                base_name: None,
                parts: Vec::new(),
                first_part: None,
                total_parts: 0,
                total_size: 0,
            },
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn get_dictionary_passwords(
    file_name: String,
    strategy: String,
) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let service = PasswordDictionaryService::new();
        match strategy.as_str() {
            "common" => Ok(service
                .get_dictionary("common")
                .cloned()
                .unwrap_or_default()),
            "recommended" => Ok(service.get_recommended_strategy(Some(&file_name))),
            _ => Err(format!("Unsupported dictionary strategy: {strategy}")),
        }
    })
    .await
    .map_err(|error| error.to_string())?
}
