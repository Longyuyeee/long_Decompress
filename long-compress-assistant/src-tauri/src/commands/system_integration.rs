use crate::system_integration::{
    IntegrationStatus, IntegrationType, NotificationHistory, NotificationRequest,
    PermissionManager, PermissionStatus, PermissionType, NOTIFIER,
};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{command, AppHandle, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAction {
    pub action: String,
    pub files: Vec<String>,
}

pub struct DesktopBehaviorState {
    pub close_to_tray: AtomicBool,
    pub has_active_tasks: AtomicBool,
    pub pending_context_actions: Mutex<Vec<ContextAction>>,
}

impl Default for DesktopBehaviorState {
    fn default() -> Self {
        Self {
            close_to_tray: AtomicBool::new(true),
            has_active_tasks: AtomicBool::new(false),
            pending_context_actions: Mutex::new(Vec::new()),
        }
    }
}

#[command]
pub fn set_close_to_tray(state: State<'_, DesktopBehaviorState>, enabled: bool) {
    state.close_to_tray.store(enabled, Ordering::SeqCst);
}

#[command]
pub fn set_has_active_tasks(state: State<'_, DesktopBehaviorState>, active: bool) {
    state.has_active_tasks.store(active, Ordering::SeqCst);
}

#[command]
pub fn exit_app(app: AppHandle) {
    app.exit(0);
}

#[command]
pub fn take_pending_context_actions(
    state: State<'_, DesktopBehaviorState>,
) -> Result<Vec<ContextAction>, String> {
    let mut actions = state
        .pending_context_actions
        .lock()
        .map_err(|_| "右键操作队列不可用".to_string())?;
    Ok(actions.drain(..).collect())
}

#[command]
pub async fn send_notification(request: NotificationRequest) -> Result<(), String> {
    NOTIFIER
        .send_notification(request)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_notification_history() -> Result<Vec<NotificationHistory>, String> {
    Ok(NOTIFIER.get_history().await)
}

#[command]
pub async fn check_permission(permission_type: PermissionType) -> Result<PermissionStatus, String> {
    let manager = PermissionManager::new();
    manager
        .check_permission(&permission_type)
        .await
        .map(|res| res.status)
        .map_err(|e| e.to_string())
}

#[command]
pub async fn request_permission(
    permission_type: PermissionType,
) -> Result<PermissionStatus, String> {
    let manager = PermissionManager::new();
    manager
        .request_permission(&permission_type)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn check_system_integration() -> Result<Vec<(IntegrationType, IntegrationStatus)>, String>
{
    // 简化实现
    Ok(vec![
        (IntegrationType::Notification, IntegrationStatus::Running),
        (IntegrationType::Permission, IntegrationStatus::Initialized),
    ])
}

/// 在系统文件管理器中打开指定路径
#[command]
pub fn open_in_explorer(_app: AppHandle, path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let target = if std::path::Path::new(&path).is_file() {
            format!("/select,\"{}\"", path)
        } else {
            path
        };
        crate::utils::process::command("explorer")
            .arg(&target)
            .spawn()
            .map_err(|e| format!("Failed to open explorer: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        let target = if std::path::Path::new(&path).is_file() {
            std::path::Path::new(&path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or(path)
        } else {
            path
        };
        crate::utils::process::command("open")
            .arg(&target)
            .spawn()
            .map_err(|e| format!("Failed to open finder: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        let target = if std::path::Path::new(&path).is_file() {
            std::path::Path::new(&path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or(path)
        } else {
            path
        };
        crate::utils::process::command("xdg-open")
            .arg(&target)
            .spawn()
            .map_err(|e| format!("Failed to open file manager: {}", e))?;
    }
    Ok(())
}

/// 注册 Windows 右键上下文菜单
#[tauri::command]
pub async fn register_context_menu() -> Result<bool, String> {
    let exe_path = std::env::current_exe().map_err(|e| format!("无法获取应用路径: {}", e))?;
    let app_path = exe_path.to_string_lossy().to_string();
    crate::system_integration::context_menu::register_context_menu(&app_path)
        .map_err(|e| format!("注册右键菜单失败: {}", e))?;
    Ok(true)
}

/// 移除 Windows 右键上下文菜单
#[tauri::command]
pub async fn unregister_context_menu() -> Result<bool, String> {
    crate::system_integration::context_menu::unregister_context_menu()
        .map_err(|e| format!("移除右键菜单失败: {}", e))?;
    Ok(true)
}

/// 检查右键菜单是否已注册
#[tauri::command]
pub async fn is_context_menu_registered() -> Result<bool, String> {
    Ok(crate::system_integration::context_menu::is_context_menu_registered())
}
