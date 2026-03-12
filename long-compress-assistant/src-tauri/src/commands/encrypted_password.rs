#![allow(dead_code, unused_imports)]
use crate::services::encrypted_password_service::{EncryptedPasswordService, PasswordGroupService};
use crate::models::password::{PasswordEntry, PasswordCategory, CustomField, CustomFieldType, PasswordGroup};
use crate::services::password_strength_service::{PasswordAuditResult, PasswordGeneratorOptions, PasswordImportExportOptions, ImportExportFormat};
use tauri::{AppHandle, Manager, State};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::PathBuf;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 应用状态中的加密密码服务
pub struct EncryptedPasswordServiceState {
    pub service: Arc<Mutex<Option<EncryptedPasswordService>>>,
    pub group_service: Arc<Mutex<Option<PasswordGroupService>>>,
    pub data_dir: PathBuf,
}

impl EncryptedPasswordServiceState {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            service: Arc::new(Mutex::new(None)),
            group_service: Arc::new(Mutex::new(None)),
            data_dir,
        }
    }

    fn get_service(&self) -> Result<Arc<Mutex<Option<EncryptedPasswordService>>>> {
        Ok(self.service.clone())
    }

    fn get_group_service(&self) -> Result<Arc<Mutex<Option<PasswordGroupService>>>> {
        Ok(self.group_service.clone())
    }
}

/// 初始化加密密码服务
#[tauri::command]
pub async fn init_encrypted_password_service(
    app: AppHandle,
    master_password: String,
) -> Result<(), String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let data_dir = state.data_dir.clone();
    let mut service = EncryptedPasswordService::new(&data_dir);

    match service.initialize(&master_password).await {
        Ok(_) => {
            let mut service_lock = state.service.lock().await;
            *service_lock = Some(service.clone());

            // 创建组服务
            let group_service = PasswordGroupService::new(Arc::new(service));
            let mut group_service_lock = state.group_service.lock().await;
            *group_service_lock = Some(group_service);

            Ok(())
        }
        Err(e) => Err(format!("初始化失败: {}", e)),
    }
}

/// 解锁加密密码服务
#[tauri::command]
pub async fn unlock_encrypted_password_service(
    app: AppHandle,
    master_password: String,
) -> Result<bool, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let data_dir = state.data_dir.clone();
    let mut service = EncryptedPasswordService::new(&data_dir);

    match service.unlock(&master_password).await {
        Ok(unlocked) => {
            if unlocked {
                let mut service_lock = state.service.lock().await;
                *service_lock = Some(service.clone());

                // 创建组服务
                let group_service = PasswordGroupService::new(Arc::new(service));
                let mut group_service_lock = state.group_service.lock().await;
                *group_service_lock = Some(group_service);
            }
            Ok(unlocked)
        }
        Err(e) => Err(format!("解锁失败: {}", e)),
    }
}

/// 锁定加密密码服务
#[tauri::command]
pub async fn lock_encrypted_password_service(app: AppHandle) -> Result<(), String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let mut service_lock = state.service.lock().await;
    if let Some(service) = service_lock.as_mut() {
        service.lock().await;
    }

    let mut group_service_lock = state.group_service.lock().await;
    *group_service_lock = None;

    Ok(())
}

/// 检查服务是否已解锁
#[tauri::command]
pub async fn is_encrypted_password_service_unlocked(app: AppHandle) -> Result<bool, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let service_lock = state.service.lock().await;
    if let Some(service) = service_lock.as_ref() {
        Ok(service.is_unlocked().await)
    } else {
        Ok(false)
    }
}

/// 添加密码条目
#[tauri::command]
pub async fn add_encrypted_password(
    app: AppHandle,
    entry: PasswordEntryRequest, // 使用 Request 结构
) -> Result<PasswordEntry, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let service_lock = state.service.lock().await;
    let service = service_lock.as_ref().ok_or("服务未初始化")?;

    let entry_model: PasswordEntry = entry.into(); // 转换为模型，自动生成 ID
    match service.add_password(entry_model).await {
        Ok(entry) => Ok(entry),
        Err(e) => Err(format!("添加密码失败: {}", e)),
    }
}

/// 获取密码条目
#[tauri::command]
pub async fn get_encrypted_password(
    app: AppHandle,
    id: String,
) -> Result<Option<PasswordEntry>, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let service_lock = state.service.lock().await;
    let service = service_lock.as_ref().ok_or("服务未初始化")?;

    match service.get_password(&id).await {
        Ok(entry) => Ok(entry),
        Err(e) => Err(format!("获取密码失败: {}", e)),
    }
}

/// 更新密码条目
#[tauri::command]
pub async fn update_encrypted_password(
    app: AppHandle,
    id: String,
    entry: PasswordEntry,
) -> Result<PasswordEntry, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let service_lock = state.service.lock().await;
    let service = service_lock.as_ref().ok_or("服务未初始化")?;

    match service.update_password(&id, entry).await {
        Ok(entry) => Ok(entry),
        Err(e) => Err(format!("更新密码失败: {}", e)),
    }
}

/// 删除密码条目
#[tauri::command]
pub async fn delete_encrypted_password(
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let service_lock = state.service.lock().await;
    let service = service_lock.as_ref().ok_or("服务未初始化")?;

    match service.delete_password(&id).await {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("删除密码失败: {}", e)),
    }
}

/// 清空所有密码条目
#[tauri::command]
pub async fn clear_encrypted_passwords(
    app: AppHandle,
) -> Result<(), String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let service_lock = state.service.lock().await;
    let service = service_lock.as_ref().ok_or("服务未初始化")?;

    match service.clear_all_passwords().await {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("清空密码本失败: {}", e)),
    }
}

/// 搜索密码条目
#[tauri::command]
pub async fn search_encrypted_passwords(
    app: AppHandle,
    query: String,
) -> Result<Vec<PasswordEntry>, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let service_lock = state.service.lock().await;
    let service = service_lock.as_ref().ok_or("服务未初始化")?;

    match service.search_passwords(&query).await {
        Ok(entries) => Ok(entries),
        Err(e) => Err(format!("搜索密码失败: {}", e)),
    }
}

/// 列出所有密码条目
#[tauri::command]
pub async fn list_encrypted_passwords(
    app: AppHandle,
) -> Result<Vec<PasswordEntry>, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let service_lock = state.service.lock().await;
    let service = service_lock.as_ref().ok_or("服务未初始化")?;

    match service.list_passwords().await {
        Ok(entries) => Ok(entries),
        Err(e) => Err(format!("列出密码失败: {}", e)),
    }
}

/// 生成强密码
#[tauri::command]
pub fn generate_strong_password(
    options: PasswordGeneratorOptions,
) -> Result<String, String> {
    Ok(EncryptedPasswordService::generate_password(&options))
}

/// 审计密码安全性
#[tauri::command]
pub async fn audit_encrypted_passwords(
    app: AppHandle,
) -> Result<Vec<PasswordAuditResult>, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let service_lock = state.service.lock().await;
    let service = service_lock.as_ref().ok_or("服务未初始化")?;

    match service.audit_passwords().await {
        Ok(results) => Ok(results),
        Err(e) => Err(format!("审计密码失败: {}", e)),
    }
}

/// 导出密码本
#[tauri::command]
pub async fn export_encrypted_passwords(
    app: AppHandle,
    options: PasswordImportExportOptions,
    export_password: String,
) -> Result<Vec<u8>, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let service_lock = state.service.lock().await;
    let service = service_lock.as_ref().ok_or("服务未初始化")?;

    match service.export_passwords(&options, &export_password).await {
        Ok(data) => Ok(data),
        Err(e) => Err(format!("导出密码失败: {}", e)),
    }
}

/// 导入密码本
#[tauri::command]
pub async fn import_encrypted_passwords(
    app: AppHandle,
    import_data: Vec<u8>,
    options: PasswordImportExportOptions,
    import_password: Option<String>,
) -> Result<usize, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let service_lock = state.service.lock().await;
    let service = service_lock.as_ref().ok_or("服务未初始化")?;

    match service.import_passwords(&import_data, &options, import_password.as_deref()).await {
        Ok(count) => Ok(count),
        Err(e) => Err(format!("导入密码失败: {}", e)),
    }
}

/// 创建密码组
#[tauri::command]
pub async fn create_password_group(
    app: AppHandle,
    group: PasswordGroup,
) -> Result<PasswordGroup, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let group_service_lock = state.group_service.lock().await;
    let group_service = group_service_lock.as_ref().ok_or("服务未初始化")?;

    match group_service.create_group(group).await {
        Ok(group) => Ok(group),
        Err(e) => Err(format!("创建密码组失败: {}", e)),
    }
}

/// 获取密码组
#[tauri::command]
pub async fn get_password_group(
    app: AppHandle,
    id: String,
) -> Result<Option<PasswordGroup>, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let group_service_lock = state.group_service.lock().await;
    let group_service = group_service_lock.as_ref().ok_or("服务未初始化")?;

    match group_service.get_group(&id).await {
        Ok(group) => Ok(group),
        Err(e) => Err(format!("获取密码组失败: {}", e)),
    }
}

/// 更新密码组
#[tauri::command]
pub async fn update_password_group(
    app: AppHandle,
    id: String,
    group: PasswordGroup,
) -> Result<PasswordGroup, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let group_service_lock = state.group_service.lock().await;
    let group_service = group_service_lock.as_ref().ok_or("服务未初始化")?;

    match group_service.update_group(&id, group).await {
        Ok(group) => Ok(group),
        Err(e) => Err(format!("更新密码组失败: {}", e)),
    }
}

/// 删除密码组
#[tauri::command]
pub async fn delete_password_group(
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let group_service_lock = state.group_service.lock().await;
    let group_service = group_service_lock.as_ref().ok_or("服务未初始化")?;

    match group_service.delete_group(&id).await {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("删除密码组失败: {}", e)),
    }
}

/// 列出所有密码组
#[tauri::command]
pub async fn list_password_groups(
    app: AppHandle,
) -> Result<Vec<PasswordGroup>, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let group_service_lock = state.group_service.lock().await;
    let group_service = group_service_lock.as_ref().ok_or("服务未初始化")?;

    match group_service.list_groups().await {
        Ok(groups) => Ok(groups),
        Err(e) => Err(format!("列出密码组失败: {}", e)),
    }
}

/// 向组中添加密码条目
#[tauri::command]
pub async fn add_entry_to_password_group(
    app: AppHandle,
    group_id: String,
    entry_id: String,
) -> Result<bool, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let group_service_lock = state.group_service.lock().await;
    let group_service = group_service_lock.as_ref().ok_or("服务未初始化")?;

    match group_service.add_entry_to_group(&group_id, &entry_id).await {
        Ok(_) => Ok(true),
        Err(e) => Err(format!("向组中添加条目失败: {}", e)),
    }
}

/// 从组中移除密码条目
#[tauri::command]
pub async fn remove_entry_from_password_group(
    app: AppHandle,
    group_id: String,
    entry_id: String,
) -> Result<bool, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let group_service_lock = state.group_service.lock().await;
    let group_service = group_service_lock.as_ref().ok_or("服务未初始化")?;

    match group_service.remove_entry_from_group(&group_id, &entry_id).await {
        Ok(_) => Ok(true),
        Err(e) => Err(format!("从组中移除条目失败: {}", e)),
    }
}

/// 获取组中的所有密码条目
#[tauri::command]
pub async fn get_group_entries(
    app: AppHandle,
    group_id: String,
) -> Result<Vec<PasswordEntry>, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let group_service_lock = state.group_service.lock().await;
    let group_service = group_service_lock.as_ref().ok_or("服务未初始化")?;

    match group_service.get_group_entries(&group_id).await {
        Ok(entries) => Ok(entries),
        Err(e) => Err(format!("获取组条目失败: {}", e)),
    }
}

/// 导出密码本
#[tauri::command]
pub async fn export_passwords_command(
    app: AppHandle,
    file_path: String,
) -> Result<bool, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();
    let service_lock = state.service.lock().await;
    let service = service_lock.as_ref().ok_or("服务未初始化")?;

    let options = PasswordImportExportOptions {
        format: ImportExportFormat::Json,
        include_passwords: true,
        include_metadata: true,
        encrypt: false, // 简化处理，导出未加密JSON供手动编辑或备份
    };

    match service.export_passwords(&options, "").await {
        Ok(data) => {
            std::fs::write(file_path, data).map_err(|e| format!("写入文件失败: {}", e))?;
            Ok(true)
        }
        Err(e) => Err(format!("导出失败: {}", e)),
    }
}

/// 导入密码本
#[tauri::command]
pub async fn import_passwords_command(
    app: AppHandle,
    file_path: String,
) -> Result<usize, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();
    let service_lock = state.service.lock().await;
    let service = service_lock.as_ref().ok_or("服务未初始化")?;

    let data = std::fs::read(file_path).map_err(|e| format!("读取文件失败: {}", e))?;
    let options = PasswordImportExportOptions {
        format: ImportExportFormat::Json,
        include_passwords: true,
        include_metadata: true,
        encrypt: false,
    };

    match service.import_passwords(&data, &options, None).await {
        Ok(count) => Ok(count),
        Err(e) => Err(format!("导入失败: {}", e)),
    }
}

/// 密码条目请求
#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordEntryRequest {
    pub name: String,
    pub username: Option<String>,
    pub password: String,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub category: PasswordCategory,
    pub tags: Vec<String>,
    pub custom_fields: Vec<CustomFieldRequest>,
}

/// 自定义字段请求
#[derive(Debug, Serialize, Deserialize)]
pub struct CustomFieldRequest {
    pub name: String,
    pub value: String,
    pub field_type: CustomFieldType,
    pub sensitive: bool,
}

/// 密码组请求
#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordGroupRequest {
    pub name: String,
    pub description: Option<String>,
    pub category: PasswordCategory,
}

/// 从请求创建密码条目
impl From<PasswordEntryRequest> for PasswordEntry {
    fn from(req: PasswordEntryRequest) -> Self {
        let mut entry = PasswordEntry::new(
            req.name,
            req.password,
            req.category,
        );

        entry.username = req.username;
        entry.url = req.url;
        entry.notes = req.notes;
        entry.tags = req.tags;
        entry.custom_fields = req.custom_fields.into_iter()
            .map(|cf| CustomField {
                name: cf.name,
                value: cf.value,
                field_type: cf.field_type,
                sensitive: cf.sensitive,
            })
            .collect();

        entry
    }
}

/// 从请求创建密码组
impl From<PasswordGroupRequest> for PasswordGroup {
    fn from(req: PasswordGroupRequest) -> Self {
        PasswordGroup::new(
            req.name,
            req.description,
        )
    }
}