use crate::system_integration::{
    NOTIFIER, NotificationRequest, NotificationHistory,
    PermissionManager, PermissionType, PermissionStatus,
    IntegrationType, IntegrationStatus
};
use tauri::{command, AppHandle, Manager};

#[command]
pub async fn send_notification(request: NotificationRequest) -> Result<(), String> {
    NOTIFIER.send_notification(request).await.map_err(|e| e.to_string())
}

#[command]
pub async fn get_notification_history() -> Result<Vec<NotificationHistory>, String> {
    Ok(NOTIFIER.get_history().await)
}

#[command]
pub async fn check_permission(permission_type: PermissionType) -> Result<PermissionStatus, String> {
    let manager = PermissionManager::new();
    manager.check_permission(&permission_type)
        .await
        .map(|res| res.status)
        .map_err(|e| e.to_string())
}

#[command]
pub async fn request_permission(permission_type: PermissionType) -> Result<PermissionStatus, String> {
    let manager = PermissionManager::new();
    manager.request_permission(&permission_type).await.map_err(|e| e.to_string())
}

#[command]
pub async fn check_system_integration() -> Result<Vec<(IntegrationType, IntegrationStatus)>, String> {
    // 简化实现
    Ok(vec![
        (IntegrationType::Notification, IntegrationStatus::Running),
        (IntegrationType::Permission, IntegrationStatus::Initialized),
    ])
}

/// 在系统文件管理器中打开指定路径
#[command]
pub fn open_in_explorer(app: AppHandle, path: String) -> Result<(), String> {
    let path = if std::path::Path::new(&path).is_file() {
        std::path::Path::new(&path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or(path)
    } else {
        path
    };
    tauri::api::shell::open(&app.shell_scope(), &path, None)
        .map_err(|e| e.to_string())
}
