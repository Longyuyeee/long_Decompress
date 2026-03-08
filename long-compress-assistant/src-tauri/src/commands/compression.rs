use crate::services::compression_service::CompressionService;
use crate::models::compression::CompressionOptions;
use tauri::command;

#[command]
pub async fn extract_file(file_path: String, output_dir: Option<String>, password: Option<String>) -> Result<String, String> {
    match CompressionService::extract(&file_path, output_dir.as_deref(), password.as_deref()).await {
        Ok(output) => Ok(output),
        Err(e) => Err(format!("解压失败: {}", e)),
    }
}

#[command]
pub async fn extract_multiple(files: Vec<String>, output_dir: Option<String>) -> Result<Vec<String>, String> {
    let mut results = Vec::new();

    for file in files {
        match CompressionService::extract(&file, output_dir.as_deref(), None).await {
            Ok(output) => results.push(format!("{}: 成功 -> {}", file, output)),
            Err(e) => results.push(format!("{}: 失败 - {}", file, e)),
        }
    }

    Ok(results)
}

#[command]
pub async fn compress_file(
    files: Vec<String>,
    output_path: String,
    options: Option<CompressionOptions>,
) -> Result<String, String> {
    let opts = options.unwrap_or_default();

    match CompressionService::compress(&files, &output_path, opts).await {
        Ok(_) => Ok(format!("压缩成功: {}", output_path)),
        Err(e) => Err(format!("压缩失败: {}", e)),
    }
}