use crate::services::password_dictionary_service::PasswordDictionaryService;
use crate::services::split_archive_detector::SplitArchiveDetector;
use serde::Serialize;
use std::path::Path;
use std::io::Read;

fn input_archive_format(path: &Path) -> Option<String> {
    let reader = crate::services::numeric_volume_reader::ZipInputReader::open_seven_zip(path).ok()?;
    let mut header = Vec::with_capacity(512);
    reader.take(512).read_to_end(&mut header).ok()?;
    let mut format = crate::services::archive_format::ArchiveFormat::from_magic(&header);
    if format == crate::services::archive_format::ArchiveFormat::Unknown {
        let logical = if crate::services::numeric_volume_reader::ZipInputReader::is_numeric_first(path) {
            Path::new(path.file_stem()?.to_str()?)
        } else { path };
        format = crate::services::archive_format::ArchiveFormat::from_extension(logical.extension()?.to_str()?);
    }
    if format == crate::services::archive_format::ArchiveFormat::Unknown { return None }
    Some(crate::services::compression_service::CompressionService::extraction_history_format(path, &format))
}

/// Pure path preview: shares execution's naming logic and never creates a directory.
#[tauri::command]
pub fn preview_extraction_output(file_path: String, output_path: String, create_subdirectory: bool) -> Result<String, String> {
    if file_path.trim().is_empty() || output_path.trim().is_empty() {
        return Err("请选择来源文件和输出目录".into());
    }
    Ok(crate::services::compression_service::CompressionService::extraction_output_path(
        Path::new(&file_path), Some(&output_path), create_subdirectory,
    ).to_string_lossy().into_owned())
}

fn destination_state(path: &Path) -> &'static str {
    match std::fs::metadata(path) {
        Ok(metadata) if !metadata.is_dir() => "file",
        Ok(_) => match std::fs::read_dir(path) {
            Ok(mut entries) => match entries.next() {
                None => "empty",
                Some(Ok(_)) => "occupied",
                Some(Err(_)) => "unavailable",
            },
            Err(_) => "unavailable",
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => "missing",
        Err(_) => "unavailable",
    }
}

#[tauri::command]
pub async fn inspect_extraction_destination(path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || destination_state(Path::new(&path)).to_string())
        .await.map_err(|error| error.to_string())
}

#[cfg(test)]
mod destination_tests {
    use super::*;
    #[test]
    fn distinguishes_new_empty_occupied_and_file_without_creating_output() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("new");
        assert_eq!(destination_state(&missing), "missing");
        assert!(!missing.exists());
        assert_eq!(destination_state(temp.path()), "empty");
        let file = temp.path().join("existing.txt");
        std::fs::write(&file, "original").unwrap();
        assert_eq!(destination_state(temp.path()), "occupied");
        assert_eq!(destination_state(&file), "file");
        assert_eq!(std::fs::read_to_string(file).unwrap(), "original");
    }
}

#[derive(Debug, Serialize)]
pub struct SplitArchiveResponse {
    pub is_split: bool,
    pub format: Option<String>,
    pub archive_format: Option<String>,
    pub base_name: Option<String>,
    pub parts: Vec<String>,
    pub first_part: Option<String>,
    pub total_parts: usize,
    pub total_size: u64,
    pub is_complete: bool,
    pub missing_parts: Vec<String>,
}

#[tauri::command]
pub async fn detect_split_archive(path: String) -> Result<SplitArchiveResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let info = SplitArchiveDetector::detect_split_archive(Path::new(&path))
            .map_err(|error| error.to_string())?;
        Ok(match info {
            Some(info) => SplitArchiveResponse {
                is_split: true,
                archive_format: input_archive_format(&info.first_part),
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
                is_complete: info.is_complete,
                missing_parts: info
                    .missing_parts
                    .iter()
                    .map(|part| part.to_string_lossy().into_owned())
                    .collect(),
            },
            None => SplitArchiveResponse {
                is_split: false,
                archive_format: input_archive_format(Path::new(&path)),
                format: None,
                base_name: None,
                parts: Vec::new(),
                first_part: None,
                total_parts: 0,
                total_size: 0,
                is_complete: true,
                missing_parts: Vec::new(),
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
