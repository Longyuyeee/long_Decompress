use crate::services::password_service::{PasswordService, PasswordEntry};
use tauri::command;

#[command]
pub async fn add_password(entry: PasswordEntry) -> Result<String, String> {
    let mut service = PasswordService::new();
    match service.add_password(entry) {
        Ok(_) => Ok("密码添加成功".to_string()),
        Err(e) => Err(format!("添加密码失败: {}", e)),
    }
}

#[command]
pub async fn find_password(id: String) -> Result<Option<PasswordEntry>, String> {
    let service = PasswordService::new();
    match service.find_password(&id) {
        Some(entry) => Ok(Some(entry.clone())),
        None => Ok(None),
    }
}

#[command]
pub async fn search_passwords(query: String) -> Result<Vec<PasswordEntry>, String> {
    let service = PasswordService::new();
    let results = service.search_passwords(&query);
    Ok(results.into_iter().map(|entry| entry.clone()).collect())
}

// 注意：导入导出功能需要更复杂的实现，这里暂时简化
#[command]
pub async fn import_passwords(file_path: String, format: String) -> Result<String, String> {
    Err("导入功能暂未实现".to_string())
}

#[command]
pub async fn export_passwords(file_path: String, format: String) -> Result<String, String> {
    Err("导出功能暂未实现".to_string())
}