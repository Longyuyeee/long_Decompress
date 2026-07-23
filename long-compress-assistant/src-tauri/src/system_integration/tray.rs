use tauri::{
    AppHandle, CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem,
};
use std::sync::atomic::Ordering;
use crate::commands::system_integration::DesktopBehaviorState;

pub fn setup_tray() -> SystemTray {
    let open = CustomMenuItem::new("open".to_string(), "打开 Long解压");
    let decompress = CustomMenuItem::new("decompress".to_string(), "解压中心");
    let compress = CustomMenuItem::new("compress".to_string(), "压缩中心");
    let toggle = CustomMenuItem::new("toggle".to_string(), "显示/隐藏主窗口");
    let quit = CustomMenuItem::new("quit".to_string(), "退出 Long解压");
    let tray_menu = SystemTrayMenu::new()
        .add_item(open)
        .add_item(decompress)
        .add_item(compress)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(toggle)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    SystemTray::new()
        .with_menu(tray_menu)
        .with_tooltip("Long解压")
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn navigate_to(app: &AppHandle, route: &str) {
    show_main_window(app);
    let _ = app.emit_all("tray-navigate", route);
}

pub fn handle_tray_event(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "quit" => {
                let state = app.state::<DesktopBehaviorState>();
                if state.has_active_tasks.load(Ordering::SeqCst) {
                    show_main_window(app);
                    let _ = app.emit_all("exit-confirmation-requested", ());
                } else {
                    app.exit(0);
                }
            }
            "open" => show_main_window(app),
            "decompress" => navigate_to(app, "/decompress"),
            "compress" => navigate_to(app, "/compress"),
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
        SystemTrayEvent::LeftClick { .. } => show_main_window(app),
        _ => {}
    }
}
