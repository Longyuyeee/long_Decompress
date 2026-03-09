//! 用户权限管理模块
//!
//! 提供跨平台的用户权限检查和管理功能，用于系统集成操作。

use tauri::AppHandle;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{Result, anyhow};
use lazy_static::lazy_static;

/// 权限类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionType {
    /// 文件关联权限（需要管理员权限）
    FileAssociation,
    /// 全局快捷键权限（需要管理员权限）
    GlobalShortcut,
    /// 系统托盘权限（通常不需要特殊权限）
    SystemTray,
    /// 通知权限（需要用户授权）
    Notification,
    /// 系统服务权限（需要管理员权限）
    SystemService,
}

/// 权限状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionStatus {
    /// 权限已授予
    Granted,
    /// 权限被拒绝
    Denied,
    /// 权限需要用户确认
    RequiresConfirmation,
    /// 权限需要管理员权限
    RequiresAdmin,
    /// 权限不可用（平台不支持）
    NotAvailable,
}

/// 权限请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub permission_type: PermissionType,
    pub description: String,
    pub required: bool,
    pub auto_request: bool,
}

/// 权限配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionConfig {
    pub auto_check_permissions: bool,
    pub show_permission_prompts: bool,
    pub remember_decisions: bool,
    pub fallback_to_user_mode: bool,
    pub permission_requests: Vec<PermissionRequest>,
}

impl Default for PermissionConfig {
    fn default() -> Self {
        Self {
            auto_check_permissions: true,
            show_permission_prompts: true,
            remember_decisions: true,
            fallback_to_user_mode: true,
            permission_requests: vec![
                PermissionRequest {
                    permission_type: PermissionType::FileAssociation,
                    description: "注册文件关联，支持双击文件使用应用打开".to_string(),
                    required: true,
                    auto_request: true,
                },
                PermissionRequest {
                    permission_type: PermissionType::GlobalShortcut,
                    description: "注册全局快捷键，支持快速操作".to_string(),
                    required: false,
                    auto_request: true,
                },
                PermissionRequest {
                    permission_type: PermissionType::Notification,
                    description: "发送系统通知，显示任务进度和结果".to_string(),
                    required: false,
                    auto_request: true,
                },
                PermissionRequest {
                    permission_type: PermissionType::SystemTray,
                    description: "显示系统托盘图标，提供快捷菜单".to_string(),
                    required: false,
                    auto_request: true,
                },
            ],
        }
    }
}

/// 权限检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionCheckResult {
    pub permission_type: PermissionType,
    pub status: PermissionStatus,
    pub message: String,
    pub can_proceed: bool,
    pub requires_elevation: bool,
}

/// 权限管理器
#[derive(Clone)]
pub struct PermissionManager {
    config: Arc<RwLock<PermissionConfig>>,
    app_handle: Option<AppHandle>,
    permission_cache: Arc<RwLock<std::collections::HashMap<PermissionType, PermissionStatus>>>,
}

impl PermissionManager {
    /// 创建新的权限管理器
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(PermissionConfig::default())),
            app_handle: None,
            permission_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// 设置应用句柄
    pub fn set_app_handle(&mut self, app_handle: AppHandle) {
        self.app_handle = Some(app_handle);
    }

    /// 初始化权限管理器
    pub async fn initialize(&self) -> Result<()> {
        log::info!("初始化权限管理器");

        // 检查所有权限
        let config = self.config.read().await;
        if config.auto_check_permissions {
            for request in &config.permission_requests {
                if request.auto_request {
                    self.check_permission(request.permission_type.clone()).await?;
                }
            }
        }

        log::info!("权限管理器初始化完成");
        Ok(())
    }

    /// 检查特定权限
    pub async fn check_permission(&self, permission_type: PermissionType) -> Result<PermissionCheckResult> {
        // 检查缓存
        {
            let cache = self.permission_cache.read().await;
            if let Some(status) = cache.get(&permission_type) {
                return Ok(self.create_check_result(&permission_type, status.clone()));
            }
        }

        // 执行平台特定的权限检查
        let status = self.check_permission_platform(&permission_type).await?;

        // 更新缓存
        {
            let mut cache = self.permission_cache.write().await;
            cache.insert(permission_type.clone(), status.clone());
        }

        Ok(self.create_check_result(&permission_type, status))
    }

    /// 平台特定的权限检查
    async fn check_permission_platform(&self, permission_type: &PermissionType) -> Result<PermissionStatus> {
        match permission_type {
            PermissionType::FileAssociation => self.check_file_association_permission().await,
            PermissionType::GlobalShortcut => self.check_global_shortcut_permission().await,
            PermissionType::SystemTray => self.check_system_tray_permission().await,
            PermissionType::Notification => self.check_notification_permission().await,
            PermissionType::SystemService => self.check_system_service_permission().await,
        }
    }

    /// 检查文件关联权限
    async fn check_file_association_permission(&self) -> Result<PermissionStatus> {
        #[cfg(target_os = "windows")]
        {
            use winreg::enums::*;
            use winreg::RegKey;

            // 尝试写入注册表测试权限
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            let test_key = hkcu.create_subkey("Software\\LongDecompress\\Test");

            match test_key {
                Ok((_key, _disp)) => {
                    // 可以写入HKCU，不需要管理员权限
                    Ok(PermissionStatus::Granted)
                }
                Err(_) => {
                    // 需要管理员权限
                    Ok(PermissionStatus::RequiresAdmin)
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            // macOS上文件关联通常需要用户确认
            Ok(PermissionStatus::RequiresConfirmation)
        }

        #[cfg(target_os = "linux")]
        {
            // Linux上通常需要写入.desktop文件
            let home_dir = std::env::var("HOME").unwrap_or_default();
            let desktop_dir = format!("{}/.local/share/applications", home_dir);

            if std::path::Path::new(&desktop_dir).exists() {
                // 可以写入用户目录
                Ok(PermissionStatus::Granted)
            } else {
                // 可能需要创建目录
                Ok(PermissionStatus::RequiresConfirmation)
            }
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            Ok(PermissionStatus::NotAvailable)
        }
    }

    /// 检查全局快捷键权限
    async fn check_global_shortcut_permission(&self) -> Result<PermissionStatus> {
        #[cfg(target_os = "windows")]
        {
            // Windows上全局快捷键通常需要管理员权限
            Ok(PermissionStatus::RequiresAdmin)
        }

        #[cfg(target_os = "macos")]
        {
            // macOS上需要辅助功能权限
            Ok(PermissionStatus::RequiresConfirmation)
        }

        #[cfg(target_os = "linux")]
        {
            // Linux上通常需要X11或Wayland权限
            Ok(PermissionStatus::RequiresConfirmation)
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            Ok(PermissionStatus::NotAvailable)
        }
    }

    /// 检查系统托盘权限
    async fn check_system_tray_permission(&self) -> Result<PermissionStatus> {
        // 系统托盘通常不需要特殊权限
        Ok(PermissionStatus::Granted)
    }

    /// 检查通知权限
    async fn check_notification_permission(&self) -> Result<PermissionStatus> {
        #[cfg(target_os = "windows")]
        {
            // Windows 10+需要通知权限
            Ok(PermissionStatus::RequiresConfirmation)
        }

        #[cfg(target_os = "macos")]
        {
            // macOS需要通知权限
            Ok(PermissionStatus::RequiresConfirmation)
        }

        #[cfg(target_os = "linux")]
        {
            // Linux上通知通常通过DBus，不需要特殊权限
            Ok(PermissionStatus::Granted)
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            Ok(PermissionStatus::NotAvailable)
        }
    }

    /// 检查系统服务权限
    async fn check_system_service_permission(&self) -> Result<PermissionStatus> {
        // 系统服务总是需要管理员权限
        Ok(PermissionStatus::RequiresAdmin)
    }

    /// 创建权限检查结果
    fn create_check_result(&self, permission_type: &PermissionType, status: PermissionStatus) -> PermissionCheckResult {
        let (message, can_proceed, requires_elevation) = match status {
            PermissionStatus::Granted => (
                "权限已授予".to_string(),
                true,
                false,
            ),
            PermissionStatus::Denied => (
                "权限被拒绝".to_string(),
                false,
                false,
            ),
            PermissionStatus::RequiresConfirmation => (
                "需要用户确认".to_string(),
                false,
                false,
            ),
            PermissionStatus::RequiresAdmin => (
                "需要管理员权限".to_string(),
                false,
                true,
            ),
            PermissionStatus::NotAvailable => (
                "当前平台不支持此功能".to_string(),
                false,
                false,
            ),
        };

        PermissionCheckResult {
            permission_type: permission_type.clone(),
            status,
            message,
            can_proceed,
            requires_elevation,
        }
    }

    /// 请求权限
    pub async fn request_permission(&self, permission_type: PermissionType) -> Result<PermissionCheckResult> {
        log::info!("请求权限: {:?}", permission_type);

        // 先检查当前权限状态
        let check_result = self.check_permission(permission_type.clone()).await?;

        if check_result.can_proceed {
            return Ok(check_result);
        }

        // 根据平台显示权限请求
        let new_status = self.request_permission_platform(&permission_type).await?;

        // 更新缓存
        {
            let mut cache = self.permission_cache.write().await;
            cache.insert(permission_type.clone(), new_status.clone());
        }

        Ok(self.create_check_result(&permission_type, new_status))
    }

    /// 平台特定的权限请求
    async fn request_permission_platform(&self, permission_type: &PermissionType) -> Result<PermissionStatus> {
        // 这里可以集成平台特定的权限请求对话框
        // 目前返回需要确认的状态
        Ok(PermissionStatus::RequiresConfirmation)
    }

    /// 检查是否具有管理员权限
    pub async fn is_admin(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
            use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

            unsafe {
                let mut token_handle = std::ptr::null_mut();
                let process_handle = GetCurrentProcess();

                if OpenProcessToken(process_handle, TOKEN_QUERY, &mut token_handle).is_ok() {
                    let mut elevation = TOKEN_ELEVATION::default();
                    let mut return_length = 0;

                    if GetTokenInformation(
                        token_handle,
                        TokenElevation,
                        Some(&mut elevation as *mut _ as *mut _),
                        std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                        &mut return_length,
                    ).is_ok()
                    {
                        return elevation.TokenIsElevated != 0;
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            // macOS上检查是否以root运行
            let output = std::process::Command::new("id")
                .arg("-u")
                .output()
                .unwrap_or_default();

            if let Ok(output_str) = String::from_utf8(output.stdout) {
                return output_str.trim() == "0";
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Linux上检查是否以root运行
            let output = std::process::Command::new("id")
                .arg("-u")
                .output()
                .unwrap_or_default();

            if let Ok(output_str) = String::from_utf8(output.stdout) {
                return output_str.trim() == "0";
            }
        }

        false
    }

    /// 获取配置
    pub async fn get_config(&self) -> PermissionConfig {
        self.config.read().await.clone()
    }

    /// 更新配置
    pub async fn update_config(&self, config: PermissionConfig) {
        let mut current_config = self.config.write().await;
        *current_config = config;
    }

    /// 获取所有权限状态
    pub async fn get_all_permission_status(&self) -> Vec<PermissionCheckResult> {
        let mut results = Vec::new();
        let config = self.config.read().await;

        for request in &config.permission_requests {
            match self.check_permission(request.permission_type.clone()).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    log::warn!("检查权限失败: {:?} - {}", request.permission_type, e);
                }
            }
        }

        results
    }

    /// 清理权限缓存
    pub async fn clear_cache(&self) {
        let mut cache = self.permission_cache.write().await;
        cache.clear();
    }
}

/// 全局权限管理器包装器
pub struct GlobalPermissionManager {
    manager: Arc<RwLock<Option<PermissionManager>>>,
}

impl GlobalPermissionManager {
    /// 创建新的全局权限管理器
    pub fn new() -> Self {
        Self {
            manager: Arc::new(RwLock::new(None)),
        }
    }

    /// 初始化权限管理器
    pub async fn initialize(&self, app_handle: AppHandle) -> Result<()> {
        log::info!("初始化全局权限管理器");

        let mut manager_guard = self.manager.write().await;
        let mut manager = PermissionManager::new();
        manager.set_app_handle(app_handle);
        *manager_guard = Some(manager);

        // 初始化管理器
        if let Some(manager) = manager_guard.as_ref() {
            manager.initialize().await?;
        }

        log::info!("全局权限管理器初始化完成");
        Ok(())
    }

    /// 获取权限管理器实例
    pub async fn get(&self) -> Result<std::sync::Arc<PermissionManager>, String> {
        let manager_guard = self.manager.read().await;
        manager_guard
            .as_ref()
            .map(|manager| std::sync::Arc::new(manager.clone()))
            .ok_or_else(|| "权限管理器未初始化".to_string())
    }
}

// 实现默认的全局实例
lazy_static! {
    pub static ref PERMISSION_MANAGER: GlobalPermissionManager = GlobalPermissionManager::new();
}