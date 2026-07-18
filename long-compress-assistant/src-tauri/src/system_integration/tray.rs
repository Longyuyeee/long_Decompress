use tauri::{
    AppHandle, CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem,
};
use std::sync::atomic::Ordering;
use crate::commands::system_integration::DesktopBehaviorState;

pub fn setup_tray() -> SystemTray {
    let quit = CustomMenuItem::new("quit".to_string(), "退出");
    let hide = CustomMenuItem::new("toggle".to_string(), "显示/隐藏");
    let tray_menu = SystemTrayMenu::new()
        .add_item(hide)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    SystemTray::new().with_menu(tray_menu)
}

pub fn handle_tray_event(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "quit" => {
                let state = app.state::<DesktopBehaviorState>();
                if state.has_active_tasks.load(Ordering::SeqCst) {
                    if let Some(window) = app.get_window("main") {
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                    }
                    let _ = app.emit_all("exit-confirmation-requested", ());
                } else {
                    app.exit(0);
                }
            }
            "toggle" => {
                if let Some(window) = app.get_window("main") {
                    if window.is_minimized().unwrap_or(false) {
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                    } else if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                    }
                }
            }
            _ => {}
        },
        SystemTrayEvent::LeftClick { .. } => {
            if let Some(window) = app.get_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }
        _ => {}
    }
}
