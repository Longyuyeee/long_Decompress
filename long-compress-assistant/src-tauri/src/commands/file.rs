use crate::services::file_service::{FileService, FileInfo};
use serde::Serialize;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
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

#[derive(Debug, Serialize)]
pub struct WordlistValidationResult {
    pub path: String,
    pub valid: bool,
    pub valid_password_count: usize,
    pub error: Option<String>,
}

#[command]
pub async fn validate_wordlists(paths: Vec<String>) -> Result<Vec<WordlistValidationResult>, String> {
    let mut results = Vec::with_capacity(paths.len());

    for path in paths {
        results.push(validate_wordlist_path(path));
    }

    Ok(results)
}

fn validate_wordlist_path(path: String) -> WordlistValidationResult {
    let source = Path::new(&path);

    if source.extension().and_then(|ext| ext.to_str()).map(|ext| !ext.eq_ignore_ascii_case("txt")).unwrap_or(true) {
        return invalid_wordlist(path, "Only .txt wordlists are supported.");
    }

    let metadata = match std::fs::metadata(source) {
        Ok(metadata) => metadata,
        Err(err) => return invalid_wordlist(path, &format!("File is not readable: {}", err)),
    };

    if !metadata.is_file() {
        return invalid_wordlist(path, "Selected path is not a file.");
    }

    if metadata.len() == 0 {
        return invalid_wordlist(path, "Wordlist is empty.");
    }

    let file = match File::open(source) {
        Ok(file) => file,
        Err(err) => return invalid_wordlist(path, &format!("File is not readable: {}", err)),
    };

    let mut valid_password_count = 0usize;
    for line in BufReader::new(file).lines() {
        match line {
            Ok(value) => {
                if !value.trim().trim_end_matches('\u{feff}').is_empty() {
                    valid_password_count += 1;
                }
            }
            Err(err) => return invalid_wordlist(path, &format!("Failed to read wordlist: {}", err)),
        }
    }

    if valid_password_count == 0 {
        return invalid_wordlist(path, "Wordlist has no usable password lines.");
    }

    WordlistValidationResult {
        path,
        valid: true,
        valid_password_count,
        error: None,
    }
}

fn invalid_wordlist(path: String, error: &str) -> WordlistValidationResult {
    WordlistValidationResult {
        path,
        valid: false,
        valid_password_count: 0,
        error: Some(error.to_string()),
    }
}
