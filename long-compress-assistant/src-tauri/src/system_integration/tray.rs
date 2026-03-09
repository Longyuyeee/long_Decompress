//! 系统托盘模块
//!
//! 提供系统托盘图标、菜单和事件处理功能。

use tauri::{AppHandle, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem, CustomMenuItem};
use serde::{Deserialize, Serialize};
use crate::system_integration::IntegrationType;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{Result, anyhow};

/// 托盘图标类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrayIconType {
    Default,
    WithBadge,
    Animated,
    Custom(String),
}

/// 托盘配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrayConfig {
    pub enabled: bool,
    pub show_notifications: bool,
    pub menu_items: Vec<TrayMenuItemConfig>,
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_notifications: true,
            menu_items: vec![
                TrayMenuItemConfig {
                    id: "show_window".to_string(),
                    label: "显示主窗口".to_string(),
                    enabled: true,
                },
                TrayMenuItemConfig {
                    id: "separator".to_string(),
                    label: "---".to_string(),
                    enabled: true,
                },
                TrayMenuItemConfig {
                    id: "quit".to_string(),
                    label: "退出".to_string(),
                    enabled: true,
                },
            ],
        }
    }
}

/// 托盘菜单项配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrayMenuItemConfig {
    pub id: String,
    pub label: String,
    pub enabled: bool,
}

/// 系统托盘管理器
pub struct AppTrayManager {
    app_handle: Option<AppHandle>,
    config: Arc<RwLock<TrayConfig>>,
}

/// 别名，兼容旧代码
pub type SystemTrayManager = AppTrayManager;

impl AppTrayManager {
    /// 创建新的系统托盘管理器
    pub fn new() -> Self {
        Self {
            app_handle: None,
            config: Arc::new(RwLock::new(TrayConfig::default())),
        }
    }

    /// 设置应用句柄
    pub fn set_app_handle(&mut self, app_handle: AppHandle) {
        self.app_handle = Some(app_handle);
    }

    /// 获取系统托盘实例（用于主循环初始化）
    pub fn create_tray() -> SystemTray {
        let config = TrayConfig::default();
        let menu = Self::create_menu_internal(&config);
        
        // 尝试加载图标
        let icon_bytes = include_bytes!("../../../icons/32x32.png");
        // Tauri v1 Icon::Raw
        let icon = tauri::Icon::Raw(icon_bytes.to_vec());

        SystemTray::new()
            .with_icon(icon)
            .with_menu(menu)
    }

    /// 创建托盘菜单（内部使用）
    fn create_menu_internal(config: &TrayConfig) -> SystemTrayMenu {
        let mut menu = SystemTrayMenu::new();

        for item_config in &config.menu_items {
            if item_config.id == "separator" {
                menu = menu.add_native_item(SystemTrayMenuItem::Separator);
            } else {
                let menu_item = CustomMenuItem::new(item_config.id.clone(), item_config.label.clone());
                menu = menu.add_item(menu_item);
            }
        }

        menu
    }

    /// 处理托盘事件
    pub fn handle_tray_event(app: &AppHandle, event: SystemTrayEvent) {
        match event {
            SystemTrayEvent::LeftClick { .. } => {
                if let Some(window) = app.get_window("main") {
                    window.show().unwrap();
                    window.set_focus().unwrap();
                }
            }
            SystemTrayEvent::MenuItemClick { id, .. } => {
                match id.as_str() {
                    "show_window" => {
                        if let Some(window) = app.get_window("main") {
                            window.show().unwrap();
                            window.set_focus().unwrap();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {
                        // 发送自定义事件到前端
                        app.emit_all("tray_menu_click", id).unwrap();
                    }
                }
            }
            _ => {}
        }
    }
}

/// 全局托盘管理器
pub struct GlobalTrayManager {
    manager: Arc<RwLock<Option<AppTrayManager>>>,
}

impl GlobalTrayManager {
    /// 创建新的全局托盘管理器
    pub fn new() -> Self {
        Self {
            manager: Arc::new(RwLock::new(None)),
        }
    }

    /// 初始化全局托盘管理器
    pub async fn initialize(&self, app_handle: AppHandle) -> Result<()> {
        let mut manager_guard = self.manager.write().await;

        if manager_guard.is_none() {
            let mut manager = AppTrayManager::new();
            manager.set_app_handle(app_handle);
            *manager_guard = Some(manager);
            log::info!("全局托盘管理器初始化完成");
        }

        Ok(())
    }

    /// 获取托盘管理器实例
    pub async fn get(&self) -> Result<AppTrayManager> {
        let manager_guard = self.manager.read().await;
        manager_guard
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow!("托盘管理器未初始化"))
    }
}

impl Clone for AppTrayManager {
    fn clone(&self) -> Self {
        Self {
            app_handle: self.app_handle.clone(),
            config: self.config.clone(),
        }
    }
}

// 实现默认的全局实例
lazy_static::lazy_static! {
    pub static ref TRAY_MANAGER: GlobalTrayManager = GlobalTrayManager::new();
}
