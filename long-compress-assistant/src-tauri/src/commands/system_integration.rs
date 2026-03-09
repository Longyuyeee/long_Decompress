use crate::system_integration::{
    NOTIFIER, NotificationRequest, NotificationConfig, NotificationHistory,
    PermissionManager, PermissionType, PermissionStatus,
    PlatformCompatibilityChecker, SystemIntegrationFeature,
    IntegrationType, IntegrationStatus, NotificationType
};
use tauri::command;
use serde::{Deserialize, Serialize};

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
