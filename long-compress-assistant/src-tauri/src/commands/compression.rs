use crate::services::compression_service::CompressionService;
use crate::models::compression::{CompressionOptions, DecompressOptions};
use tauri::command;

#[command]
pub async fn extract_file(file_path: String, output_path: Option<String>, password: Option<String>, options: Option<DecompressOptions>) -> Result<String, String> {
    let service = CompressionService::default();
    service.extract(&file_path, output_path.as_deref(), password.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn extract_multiple(files: Vec<String>, output_path: Option<String>, password: Option<String>, options: Option<DecompressOptions>) -> Result<Vec<String>, String> {
    let service = CompressionService::default();
    let mut results = Vec::new();
    for file in files {
        match service.extract(&file, output_path.as_deref(), password.as_deref()).await {
            Ok(path) => results.push(path),
            Err(e) => return Err(format!("解压文件 {} 失败: {}", file, e)),
        }
    }
    Ok(results)
}

#[command]
pub async fn compress_files(files: Vec<String>, output_path: String, options: Option<CompressionOptions>) -> Result<String, String> {
    let service = CompressionService::default();
    let opts = options.unwrap_or_default();

    match service.compress(&files, &output_path, opts).await {
        Ok(_) => Ok(format!("压缩成功: {}", output_path)),
        Err(e) => Err(format!("压缩失败: {}", e)),
    }
}
