use crate::services::compression_service::CompressionService;
use crate::models::compression::{CompressionOptions, DecompressOptions, TaskLogSeverity};
use crate::services::password_attempt_service::{PasswordAttemptService, PasswordAttemptStrategy};
use crate::services::password_query_service::PasswordQueryService;
use crate::commands::encrypted_password::EncryptedPasswordServiceState;
use tauri::{command, Window, AppHandle, Manager};
use std::sync::Arc;

async fn resolve_password(
    app: &AppHandle,
    service: &CompressionService,
    window: &Window,
    task_id: &str,
    file_path: &str,
    password: Option<String>
) -> Option<String> {
    if password.is_some() {
        return password;
    }

    if let Some(state) = app.try_state::<EncryptedPasswordServiceState>() {
        let service_lock = state.service.lock().await;
        if let Some(enc_service) = service_lock.as_ref() {
            if let Ok(conn) = crate::database::connection::get_connection().await {
                let query_service = Arc::new(PasswordQueryService::new(
                    conn.pool().clone(),
                    Arc::new(enc_service.clone())
                ));
                let attempt_service = PasswordAttemptService::new(query_service);
                
                let _ = service.emit_log(window, task_id, "正在密码本中智能寻找匹配密码...", TaskLogSeverity::Info);
                
                let strategy = PasswordAttemptStrategy::All;
                match attempt_service.attempt_extract_with_passwords(file_path, ".", strategy).await {
                    Ok(result) => {
                        if result.success && result.password.is_some() {
                            let entry_name = result.matched_entry.map(|e| e.name).unwrap_or_else(|| "未知条目".to_string());
                            let _ = service.emit_log(window, task_id, &format!("自动匹配成功! 使用密码本条目: {}", entry_name), TaskLogSeverity::Success);
                            return result.password;
                        } else {
                            let _ = service.emit_log(window, task_id, "未能自动匹配到密码，请手动输入", TaskLogSeverity::Warning);
                        }
                    },
                    Err(e) => {
                        let _ = service.emit_log(window, task_id, &format!("密码搜索出错: {}", e), TaskLogSeverity::Warning);
                    }
                }
            }
        }
    }
    None
}

#[command]
pub async fn extract_file(
    app: AppHandle,
    window: Window,
    task_id: String,
    file_path: String, 
    output_path: Option<String>, 
    password: Option<String>, 
    _options: Option<DecompressOptions>
) -> Result<String, String> {
    let service = CompressionService::new_with_defaults().await;
    service.reset_cancellation();
    
    let actual_password = resolve_password(&app, &service, &window, &task_id, &file_path, password).await;

    service.extract(window, task_id, file_path, output_path, actual_password)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn extract_multiple(
    app: AppHandle,
    window: Window,
    task_ids: Vec<String>,
    files: Vec<String>, 
    output_path: Option<String>, 
    password: Option<String>, 
    _options: Option<DecompressOptions>
) -> Result<Vec<String>, String> {
    let service = CompressionService::new_with_defaults().await;
    service.reset_cancellation();
    let mut results = Vec::new();
    
    for (i, file) in files.iter().enumerate() {
        let task_id = task_ids.get(i).cloned().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        
        let actual_password = resolve_password(&app, &service, &window, &task_id, file, password.clone()).await;

        match service.extract(window.clone(), task_id, file.clone(), output_path.clone(), actual_password).await {
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
    let service = CompressionService::new_with_defaults().await;
    service.reset_cancellation();
    let opts = options.unwrap_or_default();

    match service.compress(window, task_id, files, output_path.clone(), opts).await {
        Ok(_) => Ok(format!("压缩成功: {}", output_path)),
        Err(e) => Err(format!("压缩失败: {}", e)),
    }
}

#[command]
pub async fn cancel_compression() -> Result<(), String> {
    let service = CompressionService::new_with_defaults().await;
    service.cancel();
    Ok(())
}
