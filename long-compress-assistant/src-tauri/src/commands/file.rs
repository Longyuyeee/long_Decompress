use crate::services::file_service::{FileService, FileInfo};
use tauri::command;

#[command]
pub async fn list_files(dir_path: String, recursive: Option<bool>) -> Result<Vec<FileInfo>, String> {
    let recursive = recursive.unwrap_or(false);
    match FileService::list_files(&dir_path, recursive).await {
        Ok(files) => Ok(files),
        Err(e) => Err(format!("列出文件失败: {}", e)),
    }
}

#[command]
pub async fn get_file_info(file_path: String) -> Result<FileInfo, String> {
    match FileService::get_file_info(&file_path).await {
        Ok(info) => Ok(info),
        Err(e) => Err(format!("获取文件信息失败: {}", e)),
    }
}

#[command]
pub async fn check_file_exists(file_path: String) -> Result<bool, String> {
    use std::path::Path;
    Ok(Path::new(&file_path).exists())
}

#[command]
pub async fn detect_file_type(file_path: String) -> Result<String, String> {
    use std::path::Path;
    let path = Path::new(&file_path);

    if !path.exists() {
        return Err("文件不存在".to_string());
    }

    if path.is_dir() {
        Ok("directory".to_string())
    } else if path.is_file() {
        // 根据扩展名判断文件类型
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "txt" | "md" | "json" | "yaml" | "yml" => Ok("text".to_string()),
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" => Ok("image".to_string()),
            "mp3" | "wav" | "flac" | "ogg" => Ok("audio".to_string()),
            "mp4" | "avi" | "mkv" | "mov" => Ok("video".to_string()),
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => Ok("archive".to_string()),
            "pdf" => Ok("pdf".to_string()),
            "doc" | "docx" => Ok("document".to_string()),
            "xls" | "xlsx" => Ok("spreadsheet".to_string()),
            "ppt" | "pptx" => Ok("presentation".to_string()),
            "exe" | "msi" => Ok("executable".to_string()),
            _ => Ok("unknown".to_string()),
        }
    } else {
        Ok("unknown".to_string())
    }
}