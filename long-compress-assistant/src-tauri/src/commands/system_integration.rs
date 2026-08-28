use crate::system_integration::{
    IntegrationStatus, IntegrationType, NotificationHistory, NotificationRequest,
    PermissionManager, PermissionStatus, PermissionType, NOTIFIER,
};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{command, AppHandle, State, Window};
#[cfg(feature = "desktop-e2e")]
use tauri::Manager;

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

pub fn should_confirm_exit(state: &DesktopBehaviorState) -> bool {
    !state.close_to_tray.load(Ordering::SeqCst)
        && state.has_active_tasks.load(Ordering::SeqCst)
}

#[derive(Debug, Clone, Serialize)]
pub struct DesktopE2EBehaviorState {
    pub close_to_tray: bool,
    pub has_active_tasks: bool,
}

#[command]
pub fn desktop_e2e_get_behavior_state(
    state: State<'_, DesktopBehaviorState>,
) -> Result<DesktopE2EBehaviorState, String> {
    #[cfg(not(feature = "desktop-e2e"))]
    {
        let _ = state;
        Err("desktop E2E support is not enabled".to_string())
    }

    #[cfg(feature = "desktop-e2e")]
    {
        Ok(DesktopE2EBehaviorState {
            close_to_tray: state.close_to_tray.load(Ordering::SeqCst),
            has_active_tasks: state.has_active_tasks.load(Ordering::SeqCst),
        })
    }
}

#[command]
pub fn desktop_e2e_request_exit_confirmation(
    app: AppHandle,
    state: State<'_, DesktopBehaviorState>,
) -> Result<bool, String> {
    #[cfg(not(feature = "desktop-e2e"))]
    {
        let _ = (app, state);
        Err("desktop E2E support is not enabled".to_string())
    }

    #[cfg(feature = "desktop-e2e")]
    {
        let should_confirm = should_confirm_exit(&state);
        if should_confirm {
            app.emit_all("exit-confirmation-requested", ())
                .map_err(|error| error.to_string())?;
        }
        Ok(should_confirm)
    }
}

#[command]
pub fn desktop_e2e_hide_window(
    window: Window,
    marker_path: String,
) -> Result<(), String> {
    #[cfg(not(feature = "desktop-e2e"))]
    {
        let _ = (window, marker_path);
        Err("desktop E2E support is not enabled".to_string())
    }

    #[cfg(feature = "desktop-e2e")]
    {
        window.hide().map_err(|error| error.to_string())?;
        let visibility = if window.is_visible().map_err(|error| error.to_string())? {
            "visible"
        } else {
            "hidden"
        };
        std::fs::write(marker_path, visibility).map_err(|error| error.to_string())
    }
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

fn validate_video_output_for_default_open(path: &str) -> Result<std::path::PathBuf, String> {
    let path = std::path::PathBuf::from(path);
    if !path.is_absolute() {
        return Err("VIDEO_DEFAULT_OPEN_PATH_MUST_BE_ABSOLUTE".to_string());
    }
    let metadata = std::fs::symlink_metadata(&path)
        .map_err(|error| format!("VIDEO_DEFAULT_OPEN_METADATA_FAILED: {error}"))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err("VIDEO_DEFAULT_OPEN_REQUIRES_REGULAR_FILE".to_string());
    }
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case("mp4"))
    {
        return Err("VIDEO_DEFAULT_OPEN_REQUIRES_MP4".to_string());
    }
    Ok(path)
}

#[command]
pub fn open_video_output_with_default_application(path: String) -> Result<(), String> {
    let path = validate_video_output_for_default_open(&path)?;
    crate::services::archive_entry_open::open_with_default_application(&path)
        .map_err(|error| format!("VIDEO_DEFAULT_OPEN_FAILED: {error}"))
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

#[cfg(test)]
mod desktop_behavior_tests {
    use super::{
        should_confirm_exit, validate_video_output_for_default_open, DesktopBehaviorState,
    };
    use std::sync::atomic::Ordering;

    #[test]
    fn only_active_tasks_without_close_to_tray_require_exit_confirmation() {
        let state = DesktopBehaviorState::default();
        assert!(!should_confirm_exit(&state));

        state.close_to_tray.store(false, Ordering::SeqCst);
        assert!(!should_confirm_exit(&state));

        state.has_active_tasks.store(true, Ordering::SeqCst);
        assert!(should_confirm_exit(&state));

        state.close_to_tray.store(true, Ordering::SeqCst);
        assert!(!should_confirm_exit(&state));
    }

    #[test]
    fn video_default_open_accepts_only_absolute_regular_mp4_files() {
        let temp = tempfile::tempdir().unwrap();
        let video = temp.path().join("published.MP4");
        let text = temp.path().join("not-video.txt");
        std::fs::write(&video, b"mp4-placeholder").unwrap();
        std::fs::write(&text, b"text").unwrap();

        assert_eq!(validate_video_output_for_default_open(&video.to_string_lossy()), Ok(video));
        assert_eq!(
            validate_video_output_for_default_open("relative.mp4").unwrap_err(),
            "VIDEO_DEFAULT_OPEN_PATH_MUST_BE_ABSOLUTE"
        );
        assert_eq!(
            validate_video_output_for_default_open(&text.to_string_lossy()).unwrap_err(),
            "VIDEO_DEFAULT_OPEN_REQUIRES_MP4"
        );
        assert!(validate_video_output_for_default_open(
            &temp.path().join("missing.mp4").to_string_lossy()
        )
        .unwrap_err()
        .starts_with("VIDEO_DEFAULT_OPEN_METADATA_FAILED:"));
    }
}
