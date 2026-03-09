//! 全局快捷键模块
//!
//! 提供全局快捷键注册和管理功能。

use tauri::{AppHandle, GlobalShortcutManager, Manager};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 快捷键配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutConfig {
    pub enabled: bool,
    pub shortcuts: Vec<ShortcutDefinition>,
    pub allow_customization: bool,
    pub enable_conflict_detection: bool,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            shortcuts: vec![
                ShortcutDefinition {
                    id: "show_hide_window".to_string(),
                    accelerator: "CmdOrCtrl+Shift+L".to_string(),
                    description: "显示/隐藏窗口".to_string(),
                    enabled: true,
                    global: true,
                    action: ShortcutAction::ToggleWindow,
                },
                ShortcutDefinition {
                    id: "new_compression".to_string(),
                    accelerator: "CmdOrCtrl+N".to_string(),
                    description: "新建压缩".to_string(),
                    enabled: true,
                    global: false,
                    action: ShortcutAction::NewCompression,
                },
                ShortcutDefinition {
                    id: "new_extraction".to_string(),
                    accelerator: "CmdOrCtrl+Shift+E".to_string(),
                    description: "新建解压".to_string(),
                    enabled: true,
                    global: false,
                    action: ShortcutAction::NewExtraction,
                },
                ShortcutDefinition {
                    id: "quick_compress".to_string(),
                    accelerator: "CmdOrCtrl+Shift+C".to_string(),
                    description: "快速压缩".to_string(),
                    enabled: true,
                    global: true,
                    action: ShortcutAction::QuickCompress,
                },
                ShortcutDefinition {
                    id: "quick_extract".to_string(),
                    accelerator: "CmdOrCtrl+Shift+X".to_string(),
                    description: "快速解压".to_string(),
                    enabled: true,
                    global: true,
                    action: ShortcutAction::QuickExtract,
                },
                ShortcutDefinition {
                    id: "open_task_manager".to_string(),
                    accelerator: "CmdOrCtrl+T".to_string(),
                    description: "打开任务管理器".to_string(),
                    enabled: true,
                    global: false,
                    action: ShortcutAction::OpenTaskManager,
                },
                ShortcutDefinition {
                    id: "open_password_manager".to_string(),
                    accelerator: "CmdOrCtrl+P".to_string(),
                    description: "打开密码本管理".to_string(),
                    enabled: true,
                    global: false,
                    action: ShortcutAction::OpenPasswordManager,
                },
                ShortcutDefinition {
                    id: "pause_resume_task".to_string(),
                    accelerator: "CmdOrCtrl+Space".to_string(),
                    description: "暂停/恢复当前任务".to_string(),
                    enabled: true,
                    global: false,
                    action: ShortcutAction::PauseResumeTask,
                },
                ShortcutDefinition {
                    id: "cancel_task".to_string(),
                    accelerator: "CmdOrCtrl+Shift+C".to_string(),
                    description: "取消当前任务".to_string(),
                    enabled: true,
                    global: false,
                    action: ShortcutAction::CancelTask,
                },
            ],
            allow_customization: true,
            enable_conflict_detection: true,
        }
    }
}

/// 快捷键定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutDefinition {
    pub id: String,
    pub accelerator: String,
    pub description: String,
    pub enabled: bool,
    pub global: bool,
    pub action: ShortcutAction,
}

/// 快捷键动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShortcutAction {
    ToggleWindow,
    NewCompression,
    NewExtraction,
    QuickCompress,
    QuickExtract,
    OpenTaskManager,
    OpenPasswordManager,
    PauseResumeTask,
    CancelTask,
    Custom(String),
}

/// 快捷键冲突
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutConflict {
    pub shortcut_id: String,
    pub conflicting_ids: Vec<String>,
    pub accelerator: String,
}

/// 全局快捷键管理器 (已重命名以避免与 Tauri trait 冲突)
pub struct AppShortcutManager {
    app_handle: AppHandle,
    config: ShortcutConfig,
    registered_shortcuts: HashMap<String, bool>,
}

impl AppShortcutManager {
    /// 创建新的全局快捷键管理器
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            config: ShortcutConfig::default(),
            registered_shortcuts: HashMap::new(),
        }
    }

    /// 初始化快捷键管理器
    pub fn initialize(&mut self) -> Result<()> {
        if !self.config.enabled {
            log::info!("全局快捷键功能已禁用");
            return Ok(());
        }

        log::info!("开始注册全局快捷键");

        // 收集需要注册的快捷键，避免循环中借用冲突
        let shortcuts_to_register: Vec<_> = self.config.shortcuts.iter()
            .filter(|s| s.enabled && s.global)
            .cloned()
            .collect();

        // 注册所有启用的全局快捷键
        for shortcut in &shortcuts_to_register {
            if let Err(e) = self.register_shortcut(shortcut) {
                log::warn!("注册快捷键失败 {}: {}", shortcut.id, e);
            }
        }

        log::info!("全局快捷键初始化完成，注册了 {} 个快捷键", self.registered_shortcuts.len());
        Ok(())
    }

    /// 注册单个快捷键
    fn register_shortcut(&mut self, shortcut: &ShortcutDefinition) -> Result<()> {
        let accelerator = &shortcut.accelerator;
        let shortcut_id = shortcut.id.clone();

        log::debug!("注册快捷键: {} ({})", shortcut_id, accelerator);

        // 获取全局快捷键管理器
        let mut shortcut_manager = self.app_handle.global_shortcut_manager();

        // 检查是否已注册
        if shortcut_manager.is_registered(accelerator)? {
            log::warn!("快捷键已注册: {}", accelerator);
            return Ok(());
        }

        // 注册快捷键
        let app_handle = self.app_handle.clone();
        let action = shortcut.action.clone();

        shortcut_manager.register(accelerator, move || {
            Self::handle_shortcut_action(&app_handle, &action);
        })?;

        self.registered_shortcuts.insert(shortcut_id, true);

        Ok(())
    }

    /// 处理快捷键动作
    fn handle_shortcut_action(app_handle: &AppHandle, action: &ShortcutAction) {
        log::debug!("快捷键触发: {:?}", action);

        match action {
            ShortcutAction::ToggleWindow => {
                Self::toggle_window(app_handle);
            }
            ShortcutAction::NewCompression => {
                Self::new_compression(app_handle);
            }
            ShortcutAction::NewExtraction => {
                Self::new_extraction(app_handle);
            }
            ShortcutAction::QuickCompress => {
                Self::quick_compress(app_handle);
            }
            ShortcutAction::QuickExtract => {
                Self::quick_extract(app_handle);
            }
            ShortcutAction::OpenTaskManager => {
                Self::open_task_manager(app_handle);
            }
            ShortcutAction::OpenPasswordManager => {
                Self::open_password_manager(app_handle);
            }
            ShortcutAction::PauseResumeTask => {
                Self::pause_resume_task(app_handle);
            }
            ShortcutAction::CancelTask => {
                Self::cancel_task(app_handle);
            }
            ShortcutAction::Custom(custom_action) => {
                Self::handle_custom_action(app_handle, custom_action);
            }
        }
    }

    /// 显示/隐藏窗口
    fn toggle_window(app_handle: &AppHandle) {
        if let Some(window) = app_handle.get_window("main") {
            if window.is_visible().unwrap_or(false) {
                window.hide().unwrap_or_else(|e| {
                    log::error!("隐藏窗口失败: {}", e);
                });
            } else {
                window.show().unwrap_or_else(|e| {
                    log::error!("显示窗口失败: {}", e);
                });
                window.set_focus().unwrap_or_else(|e| {
                    log::error!("设置窗口焦点失败: {}", e);
                });
            }
        }
    }

    /// 新建压缩
    fn new_compression(app_handle: &AppHandle) {
        if let Err(e) = app_handle.emit_all("shortcut_new_compression", ()) {
            log::error!("发送新建压缩事件失败: {}", e);
        }
    }

    /// 新建解压
    fn new_extraction(app_handle: &AppHandle) {
        if let Err(e) = app_handle.emit_all("shortcut_new_extraction", ()) {
            log::error!("发送新建解压事件失败: {}", e);
        }
    }

    /// 快速压缩
    fn quick_compress(app_handle: &AppHandle) {
        if let Err(e) = app_handle.emit_all("shortcut_quick_compress", ()) {
            log::error!("发送快速压缩事件失败: {}", e);
        }
    }

    /// 快速解压
    fn quick_extract(app_handle: &AppHandle) {
        if let Err(e) = app_handle.emit_all("shortcut_quick_extract", ()) {
            log::error!("发送快速解压事件失败: {}", e);
        }
    }

    /// 打开任务管理器
    fn open_task_manager(app_handle: &AppHandle) {
        if let Err(e) = app_handle.emit_all("shortcut_open_task_manager", ()) {
            log::error!("发送打开任务管理器事件失败: {}", e);
        }
    }

    /// 打开密码本管理
    fn open_password_manager(app_handle: &AppHandle) {
        if let Err(e) = app_handle.emit_all("shortcut_open_password_manager", ()) {
            log::error!("发送打开密码本管理事件失败: {}", e);
        }
    }

    /// 暂停/恢复当前任务
    fn pause_resume_task(app_handle: &AppHandle) {
        if let Err(e) = app_handle.emit_all("shortcut_pause_resume_task", ()) {
            log::error!("发送暂停/恢复任务事件失败: {}", e);
        }
    }

    /// 取消当前任务
    fn cancel_task(app_handle: &AppHandle) {
        if let Err(e) = app_handle.emit_all("shortcut_cancel_task", ()) {
            log::error!("发送取消任务事件失败: {}", e);
        }
    }

    /// 处理自定义动作
    fn handle_custom_action(app_handle: &AppHandle, custom_action: &str) {
        let event_data = serde_json::json!({
            "action": custom_action,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        if let Err(e) = app_handle.emit_all("shortcut_custom_action", event_data) {
            log::error!("发送自定义动作事件失败: {}", e);
        }
    }

    /// 注销所有快捷键
    pub fn unregister_all(&mut self) -> Result<()> {
        log::info!("开始注销所有快捷键");

        let mut shortcut_manager = self.app_handle.global_shortcut_manager();

        for shortcut in &self.config.shortcuts {
            if shortcut.enabled && shortcut.global {
                if let Err(e) = shortcut_manager.unregister(&shortcut.accelerator) {
                    log::warn!("注销快捷键失败 {}: {}", shortcut.id, e);
                }
            }
        }

        self.registered_shortcuts.clear();
        log::info!("所有快捷键已注销");

        Ok(())
    }

    /// 更新快捷键
    pub fn update_shortcut(&mut self, shortcut_id: &str, new_accelerator: &str) -> Result<()> {
        // 查找快捷键
        let mut shortcut_to_update = None;
        if let Some(shortcut) = self.config.shortcuts.iter_mut().find(|s| s.id == shortcut_id) {
            let old_accelerator = shortcut.accelerator.clone();
            shortcut.accelerator = new_accelerator.to_string();
            
            if shortcut.enabled && shortcut.global {
                shortcut_to_update = Some((old_accelerator, shortcut.clone()));
            }
        }

        if let Some((old_accelerator, shortcut)) = shortcut_to_update {
            let mut shortcut_manager = self.app_handle.global_shortcut_manager();

            // 注销旧的
            if shortcut_manager.is_registered(&old_accelerator)? {
                shortcut_manager.unregister(&old_accelerator)?;
            }

            // 注册新的
            self.register_shortcut(&shortcut)?;
        }

        Ok(())
    }

    /// 启用/禁用快捷键
    pub fn set_shortcut_enabled(&mut self, shortcut_id: &str, enabled: bool) -> Result<()> {
        let mut action_needed = None;
        
        if let Some(shortcut) = self.config.shortcuts.iter_mut().find(|s| s.id == shortcut_id) {
            let was_enabled = shortcut.enabled;
            shortcut.enabled = enabled;

            if was_enabled && !enabled && shortcut.global {
                action_needed = Some((false, shortcut.clone()));
            } else if !was_enabled && enabled && shortcut.global {
                action_needed = Some((true, shortcut.clone()));
            }
        }

        match action_needed {
            Some((true, shortcut)) => {
                // 启用：注册快捷键
                self.register_shortcut(&shortcut)?;
            }
            Some((false, shortcut)) => {
                // 禁用：注销快捷键
                let mut shortcut_manager = self.app_handle.global_shortcut_manager();
                if shortcut_manager.is_registered(&shortcut.accelerator)? {
                    shortcut_manager.unregister(&shortcut.accelerator)?;
                    self.registered_shortcuts.remove(shortcut_id);
                }
            }
            None => {}
        }

        log::info!("快捷键 {} 已{}", shortcut_id, if enabled { "启用" } else { "禁用" });
        Ok(())
    }

    /// 检查快捷键冲突
    pub fn check_conflicts(&self) -> Vec<ShortcutConflict> {
        if !self.config.enable_conflict_detection {
            return Vec::new();
        }

        let mut conflicts = Vec::new();
        let mut accelerator_map: HashMap<String, Vec<String>> = HashMap::new();

        // 收集所有启用的快捷键
        for shortcut in &self.config.shortcuts {
            if shortcut.enabled {
                accelerator_map
                    .entry(shortcut.accelerator.clone())
                    .or_insert_with(Vec::new)
                    .push(shortcut.id.clone());
            }
        }

        // 检查冲突
        for (accelerator, shortcut_ids) in accelerator_map {
            if shortcut_ids.len() > 1 {
                conflicts.push(ShortcutConflict {
                    shortcut_id: shortcut_ids[0].clone(),
                    conflicting_ids: shortcut_ids[1..].to_vec(),
                    accelerator,
                });
            }
        }

        conflicts
    }

    /// 验证快捷键格式
    pub fn validate_accelerator(&self, accelerator: &str) -> Result<()> {
        // 简单的验证逻辑
        if accelerator.trim().is_empty() {
            return Err(anyhow!("快捷键不能为空"));
        }

        // 检查是否包含必要的修饰键
        let parts: Vec<&str> = accelerator.split('+').collect();
        if parts.len() < 2 {
            return Err(anyhow!("快捷键必须包含修饰键（如Ctrl、Alt、Shift等）"));
        }

        // 检查最后一个部分是否是有效的键
        let last_part = parts.last().unwrap().trim();
        if last_part.is_empty() {
            return Err(anyhow!("快捷键必须包含一个有效的键"));
        }

        Ok(())
    }

    /// 获取配置
    pub fn get_config(&self) -> &ShortcutConfig {
        &self.config
    }

    /// 更新配置
    pub fn update_config(&mut self, new_config: ShortcutConfig) -> Result<()> {
        // 先注销所有快捷键
        self.unregister_all()?;

        // 更新配置
        self.config = new_config;

        // 重新注册快捷键
        self.initialize()?;

        Ok(())
    }

    /// 获取已注册的快捷键数量
    pub fn get_registered_count(&self) -> usize {
        self.registered_shortcuts.len()
    }

    /// 导出快捷键配置
    pub fn export_config(&self) -> Result<String> {
        serde_json::to_string_pretty(&self.config)
            .map_err(|e| anyhow!("导出配置失败: {}", e))
    }

    /// 导入快捷键配置
    pub fn import_config(&mut self, config_json: &str) -> Result<()> {
        let new_config: ShortcutConfig = serde_json::from_str(config_json)
            .map_err(|e| anyhow!("导入配置失败: {}", e))?;

        self.update_config(new_config)
    }
}

/// 全局快捷键管理器包装
pub struct AppShortcutManagerWrapper {
    manager: Arc<RwLock<Option<AppShortcutManager>>>,
}

impl AppShortcutManagerWrapper {
    /// 创建新的全局快捷键管理器包装
    pub fn new() -> Self {
        Self {
            manager: Arc::new(RwLock::new(None)),
        }
    }

    /// 初始化全局快捷键管理器
    pub async fn initialize(&self, app_handle: AppHandle) -> Result<()> {
        let mut manager_guard = self.manager.write().await;

        if manager_guard.is_none() {
            let mut manager = AppShortcutManager::new(app_handle);
            manager.initialize()?;
            *manager_guard = Some(manager);
            log::info!("全局快捷键管理器初始化完成");
        }

        Ok(())
    }
}

// 实现默认的全局实例
lazy_static::lazy_static! {
    pub static ref GLOBAL_SHORTCUT_MANAGER: AppShortcutManagerWrapper = AppShortcutManagerWrapper::new();
}
