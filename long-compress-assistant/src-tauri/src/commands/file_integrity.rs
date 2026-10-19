use crate::services::file_integrity_service::{ChecksumAlgorithm, FileIntegrityService};
use serde::Deserialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use crate::services::checksum_manifest::ChecksumVerificationReport;

#[derive(Debug, Deserialize)]
pub struct ChecksumExportEntry {
    pub file_name: String,
    pub checksum: String,
}

fn parse_algorithm(value: &str) -> Result<ChecksumAlgorithm, String> {
    match value.to_ascii_lowercase().as_str() {
        "crc32" => Ok(ChecksumAlgorithm::CRC32),
        "md5" => Ok(ChecksumAlgorithm::MD5),
        "sha256" => Ok(ChecksumAlgorithm::SHA256),
        _ => Err(format!("Unsupported checksum algorithm: {value}")),
    }
}

#[tauri::command]
pub async fn calculate_checksum(path: String, algorithm: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let algorithm = parse_algorithm(&algorithm)?;
        FileIntegrityService::calculate_checksum(Path::new(&path), algorithm)
            .map(|result| result.checksum)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn export_checksum_file(
    path: String,
    results: Vec<ChecksumExportEntry>,
    algorithm: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let algorithm = parse_algorithm(&algorithm)?;
        let mut output = std::fs::File::create(&path).map_err(|error| error.to_string())?;

        for entry in results {
            let line = match algorithm {
                ChecksumAlgorithm::CRC32 => format!("{} {}\n", entry.file_name, entry.checksum),
                _ => format!("{}  {}\n", entry.checksum, entry.file_name),
            };
            output
                .write_all(line.as_bytes())
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn verify_checksum_file(
    checksum_path: String,
    entry_lines: Option<Vec<usize>>,
) -> Result<ChecksumVerificationReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let checksum_file = PathBuf::from(&checksum_path);
        let base_dir = checksum_file.parent().unwrap_or_else(|| Path::new("."));
        FileIntegrityService::verify_checksum_entries(&checksum_file, base_dir, entry_lines.as_deref())
            .map_err(|error| format!("{error:#}"))
    })
    .await
    .map_err(|error| error.to_string())?
}
