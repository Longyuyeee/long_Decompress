use crate::services::compression_service::CompressionService;
use crate::models::compression::{CompressionOptions, DecompressOptions};
use tauri::{command, Window};

#[command]
pub async fn extract_file(
    window: Window,
    task_id: String,
    file_path: String, 
    output_path: Option<String>, 
    password: Option<String>, 
    _options: Option<DecompressOptions>
) -> Result<String, String> {
    let service = CompressionService::default();
    service.reset_cancellation();
    service.extract(window, task_id, file_path, output_path, password)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn extract_multiple(
    window: Window,
    task_ids: Vec<String>,
    files: Vec<String>, 
    output_path: Option<String>, 
    password: Option<String>, 
    _options: Option<DecompressOptions>
) -> Result<Vec<String>, String> {
    let service = CompressionService::default();
    service.reset_cancellation();
    let mut results = Vec::new();
    
    for (i, file) in files.iter().enumerate() {
        let task_id = task_ids.get(i).cloned().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        // 关键修复：传递 window.clone() 和 task_id
        match service.extract(window.clone(), task_id, file.clone(), output_path.clone(), password.clone()).await {
            Ok(path) => results.push(path),
            Err(e) => return Err(format!("解压文件 {} 失败: {}", file, e)),
        }
    }
    Ok(results)
}

#[command]
pub async fn compress_files(
    window: Window,
    task_id: String,
    files: Vec<String>, 
    output_path: String, 
    options: Option<CompressionOptions>
) -> Result<String, String> {
    let service = CompressionService::default();
    service.reset_cancellation();
    let opts = options.unwrap_or_default();

    match service.compress(window, task_id, files, output_path.clone(), opts).await {
        Ok(_) => Ok(format!("压缩成功: {}", output_path)),
        Err(e) => Err(format!("压缩失败: {}", e)),
    }
}

#[command]
pub async fn cancel_compression() -> Result<(), String> {
    let service = CompressionService::default();
    service.cancel();
    Ok(())
}
