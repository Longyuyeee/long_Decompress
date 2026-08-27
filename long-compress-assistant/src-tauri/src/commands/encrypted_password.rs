#![allow(dead_code, unused_imports)]
use crate::models::password::{PasswordCategory, PasswordEntry, PasswordGroup};
use crate::services::encrypted_password_service::{EncryptedPasswordService, PasswordGroupService};
use crate::services::password_strength_service::{
    ImportExportFormat, PasswordAuditResult, PasswordGeneratorOptions, PasswordImportExportOptions,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{command, AppHandle, Manager, State};
use tokio::sync::Mutex;

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

/// 在 Rust 后端确保密码保险箱就绪。安装密钥不会返回给 WebView。
#[tauri::command]
pub async fn ensure_encrypted_password_service(app: AppHandle) -> Result<bool, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    {
        let service_lock = state.service.lock().await;
        if let Some(service) = service_lock.as_ref() {
            if service.is_unlocked().await {
                return Ok(true);
            }
        }
    }

    let master_key = EncryptedPasswordService::get_or_create_master_key(&state.data_dir)
        .await
        .map_err(|error| format!("读取本机密码保护密钥失败: {}", error))?;
    let mut service = EncryptedPasswordService::new(&state.data_dir);
    let unlocked = service
        .unlock(&master_key)
        .await
        .map_err(|error| format!("解锁密码保险箱失败: {}", error))?;

    if !unlocked {
        let hash_path = state.data_dir.join("master_password.hash");
        if hash_path.exists() {
            return Err(
                "本机保护密钥与现有密码保险箱不匹配；已保留原数据，未重新初始化".to_string(),
            );
        }
        service
            .initialize(&master_key)
            .await
            .map_err(|error| format!("初始化密码保险箱失败: {}", error))?;
    }

    let mut service_lock = state.service.lock().await;
    *service_lock = Some(service.clone());
    let mut group_service_lock = state.group_service.lock().await;
    *group_service_lock = Some(PasswordGroupService::new(Arc::new(service)));
    Ok(true)
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
    entry: ArchivePasswordEntryRequest,
) -> Result<ArchivePasswordEntry, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let service_lock = state.service.lock().await;
    let service = service_lock.as_ref().ok_or("服务未初始化")?;

    let entry_model: PasswordEntry = entry.into_new_entry();
    match service.add_password(entry_model).await {
        Ok(entry) => Ok(entry.into()),
        Err(e) => Err(format!("添加密码失败: {}", e)),
    }
}

/// 获取密码条目
#[tauri::command]
pub async fn get_encrypted_password(
    app: AppHandle,
    id: String,
) -> Result<Option<ArchivePasswordEntry>, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let service_lock = state.service.lock().await;
    let service = service_lock.as_ref().ok_or("服务未初始化")?;

    match service.get_password(&id).await {
        Ok(entry) => Ok(entry.map(Into::into)),
        Err(e) => Err(format!("获取密码失败: {}", e)),
    }
}

/// 更新密码条目
#[tauri::command]
pub async fn update_encrypted_password(
    app: AppHandle,
    id: String,
    entry: ArchivePasswordEntryRequest,
) -> Result<ArchivePasswordEntry, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let service_lock = state.service.lock().await;
    let service = service_lock.as_ref().ok_or("服务未初始化")?;

    let existing = service
        .get_password(&id)
        .await
        .map_err(|error| format!("读取待更新密码失败: {}", error))?
        .ok_or_else(|| format!("密码条目不存在: {}", id))?;
    let updated = entry.apply_to_existing(existing);

    match service.update_password(&id, updated).await {
        Ok(entry) => Ok(entry.into()),
        Err(e) => Err(format!("更新密码失败: {}", e)),
    }
}

/// 删除密码条目
#[tauri::command]
pub async fn delete_encrypted_password(app: AppHandle, id: String) -> Result<(), String> {
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
pub async fn clear_encrypted_passwords(app: AppHandle) -> Result<(), String> {
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
) -> Result<Vec<ArchivePasswordEntry>, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let service_lock = state.service.lock().await;
    let service = service_lock.as_ref().ok_or("服务未初始化")?;

    match service.list_passwords().await {
        Ok(entries) => {
            let query = query.to_lowercase();
            Ok(entries
                .into_iter()
                .filter(|entry| {
                    query.is_empty()
                        || entry.name.to_lowercase().contains(&query)
                        || entry
                            .notes
                            .as_ref()
                            .is_some_and(|notes| notes.to_lowercase().contains(&query))
                        || entry
                            .tags
                            .iter()
                            .any(|tag| tag.to_lowercase().contains(&query))
                })
                .map(Into::into)
                .collect())
        }
        Err(e) => Err(format!("搜索密码失败: {}", e)),
    }
}

/// 列出所有密码条目
#[tauri::command]
pub async fn list_encrypted_passwords(app: AppHandle) -> Result<Vec<ArchivePasswordEntry>, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();

    let service_lock = state.service.lock().await;
    let service = service_lock.as_ref().ok_or("服务未初始化")?;

    match service.list_passwords().await {
        Ok(entries) => Ok(entries.into_iter().map(Into::into).collect()),
        Err(e) => Err(format!("列出密码失败: {}", e)),
    }
}

/// 原子记录一次密码使用，并返回包含最新统计的条目。
#[tauri::command]
pub async fn increment_encrypted_password_use_count(
    app: AppHandle,
    id: String,
) -> Result<ArchivePasswordEntry, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();
    let service_lock = state.service.lock().await;
    let service = service_lock.as_ref().ok_or("服务未初始化")?;

    service
        .increment_use_count(&id)
        .await
        .map(Into::into)
        .map_err(|error| format!("记录密码使用失败: {}", error))
}

/// 生成强密码
#[tauri::command]
pub fn generate_strong_password(options: PasswordGeneratorOptions) -> Result<String, String> {
    Ok(EncryptedPasswordService::generate_password(&options))
}

/// 审计密码安全性
#[tauri::command]
pub async fn audit_encrypted_passwords(app: AppHandle) -> Result<Vec<PasswordAuditResult>, String> {
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

    match service
        .import_passwords(&import_data, &options, import_password.as_deref())
        .await
    {
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
pub async fn delete_password_group(app: AppHandle, id: String) -> Result<(), String> {
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
pub async fn list_password_groups(app: AppHandle) -> Result<Vec<PasswordGroup>, String> {
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

    match group_service
        .remove_entry_from_group(&group_id, &entry_id)
        .await
    {
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
    export_password: Option<String>,
    encrypt: Option<bool>,
    include_passwords: Option<bool>,
    include_metadata: Option<bool>,
    format: Option<String>,
) -> Result<bool, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();
    let service_lock = state.service.lock().await;
    let service = service_lock.as_ref().ok_or("服务未初始化")?;

    // 解析格式
    let format_enum = match format.as_deref().unwrap_or("Json") {
        "Json" => ImportExportFormat::Json,
        "Csv" => ImportExportFormat::Csv,
        "KeePass" => ImportExportFormat::KeePass,
        _ => ImportExportFormat::Json,
    };

    let options = PasswordImportExportOptions {
        format: format_enum,
        include_passwords: include_passwords.unwrap_or(true),
        include_metadata: include_metadata.unwrap_or(true),
        encrypt: encrypt.unwrap_or(false),
    };

    let password = export_password.as_deref().unwrap_or("");

    match service.export_passwords(&options, password).await {
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
    import_password: Option<String>,
    encrypt: Option<bool>,
    format: Option<String>,
) -> Result<usize, String> {
    let state: State<'_, EncryptedPasswordServiceState> = app.state();
    let service_lock = state.service.lock().await;
    let service = service_lock.as_ref().ok_or("服务未初始化")?;

    let data = std::fs::read(file_path).map_err(|e| format!("读取文件失败: {}", e))?;

    // 解析格式
    let format_enum = match format.as_deref().unwrap_or("Json") {
        "Json" => ImportExportFormat::Json,
        "Csv" => ImportExportFormat::Csv,
        "KeePass" => ImportExportFormat::KeePass,
        _ => ImportExportFormat::Json,
    };

    let options = PasswordImportExportOptions {
        format: format_enum,
        include_passwords: true,
        include_metadata: true,
        encrypt: encrypt.unwrap_or(false),
    };

    match service
        .import_passwords(&data, &options, import_password.as_deref())
        .await
    {
        Ok(count) => Ok(count),
        Err(e) => Err(format!("导入失败: {}", e)),
    }
}

/// 活动界面使用的归档密码分类。旧网站密码分类只保留在磁盘兼容模型中。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArchivePasswordCategory {
    General,
    Work,
    Media,
    Documents,
    Other,
}

impl From<PasswordCategory> for ArchivePasswordCategory {
    fn from(category: PasswordCategory) -> Self {
        match category {
            PasswordCategory::Personal => Self::General,
            PasswordCategory::Work => Self::Work,
            PasswordCategory::Entertainment => Self::Media,
            PasswordCategory::Education => Self::Documents,
            _ => Self::Other,
        }
    }
}

impl From<ArchivePasswordCategory> for PasswordCategory {
    fn from(category: ArchivePasswordCategory) -> Self {
        match category {
            ArchivePasswordCategory::General => Self::Personal,
            ArchivePasswordCategory::Work => Self::Work,
            ArchivePasswordCategory::Media => Self::Entertainment,
            ArchivePasswordCategory::Documents => Self::Education,
            ArchivePasswordCategory::Other => Self::Other,
        }
    }
}

/// WebView 可见的最小归档密码模型，不暴露网站账号、URL、到期、强度或自定义登录字段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivePasswordEntry {
    pub id: String,
    pub name: String,
    pub password: String,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub category: ArchivePasswordCategory,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
    pub favorite: bool,
    pub use_count: u32,
    pub usage_history: std::collections::HashMap<String, u32>,
}

impl From<PasswordEntry> for ArchivePasswordEntry {
    fn from(entry: PasswordEntry) -> Self {
        Self {
            id: entry.id,
            name: entry.name,
            password: entry.password,
            notes: entry.notes,
            tags: entry.tags,
            category: entry.category.into(),
            created_at: entry.created_at,
            updated_at: entry.updated_at,
            last_used: entry.last_used,
            favorite: entry.favorite,
            use_count: entry.use_count,
            usage_history: entry.usage_history,
        }
    }
}

/// 新增与编辑只接受归档密码字段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivePasswordEntryRequest {
    pub name: String,
    pub password: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub category: Option<ArchivePasswordCategory>,
    #[serde(default)]
    pub favorite: Option<bool>,
}

impl ArchivePasswordEntryRequest {
    fn into_new_entry(self) -> PasswordEntry {
        let category = self
            .category
            .unwrap_or(ArchivePasswordCategory::Other)
            .into();
        let mut entry = PasswordEntry::new(self.name, self.password, category);
        entry.notes = self.notes;
        entry.tags = self.tags;
        entry.favorite = self.favorite.unwrap_or(false);
        entry
    }

    fn apply_to_existing(self, mut entry: PasswordEntry) -> PasswordEntry {
        entry.name = self.name;
        entry.password = self.password;
        entry.notes = self.notes;
        entry.tags = self.tags;
        if let Some(category) = self.category {
            entry.category = category.into();
        }
        if let Some(favorite) = self.favorite {
            entry.favorite = favorite;
        }
        entry
    }
}

/// 密码组请求
#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordGroupRequest {
    pub name: String,
    pub description: Option<String>,
    pub category: PasswordCategory,
}

/// 从请求创建密码组
impl From<PasswordGroupRequest> for PasswordGroup {
    fn from(req: PasswordGroupRequest) -> Self {
        PasswordGroup::new(req.name, req.description)
    }
}

#[cfg(test)]
mod password_entry_request_tests {
    use super::*;

    #[test]
    fn archive_request_does_not_replace_legacy_metadata_on_edit() {
        let mut legacy = PasswordEntry::new(
            "Legacy".to_string(),
            "old".to_string(),
            PasswordCategory::Finance,
        );
        legacy.username = Some("legacy-user".to_string());
        legacy.url = Some("https://legacy.example".to_string());
        legacy.use_count = 9;

        let request = ArchivePasswordEntryRequest {
            name: "Archive password".to_string(),
            password: "new".to_string(),
            notes: Some("RAR source".to_string()),
            tags: vec!["rar".to_string()],
            category: None,
            favorite: Some(true),
        };
        let updated = request.apply_to_existing(legacy);

        assert_eq!(updated.username.as_deref(), Some("legacy-user"));
        assert_eq!(updated.url.as_deref(), Some("https://legacy.example"));
        assert_eq!(updated.category, PasswordCategory::Finance);
        assert_eq!(updated.use_count, 9);
        assert!(updated.favorite);
    }

    #[test]
    fn archive_response_hides_traditional_password_manager_fields() {
        let mut legacy = PasswordEntry::new(
            "Legacy".to_string(),
            "secret".to_string(),
            PasswordCategory::Entertainment,
        );
        legacy.username = Some("owner".to_string());
        legacy.url = Some("https://example.com".to_string());
        let response: ArchivePasswordEntry = legacy.into();
        let json = serde_json::to_value(response).unwrap();

        assert_eq!(json["category"], "Media");
        assert!(json.get("username").is_none());
        assert!(json.get("url").is_none());
        assert!(json.get("strength").is_none());
        assert!(json.get("expires_at").is_none());
        assert!(json.get("custom_fields").is_none());
    }
}
