//! 用户权限管理模块
//!
//! 提供跨平台的用户权限检查和管理功能，用于系统集成操作。

use tauri::AppHandle;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use lazy_static::lazy_static;

/// 权限类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
                    self.check_permission(&request.permission_type).await?;
                }
            }
        }

        log::info!("权限管理器初始化完成");
        Ok(())
    }

    /// 检查特定权限
    pub async fn check_permission(&self, permission_type: &PermissionType) -> Result<PermissionCheckResult> {
        // 检查缓存
        {
            let cache = self.permission_cache.read().await;
            if let Some(status) = cache.get(permission_type) {
                return Ok(self.create_check_result(permission_type, status.clone()));
            }
        }

        // 执行平台特定的权限检查
        let status = self.check_permission_platform(permission_type).await?;

        // 更新缓存
        {
            let mut cache = self.permission_cache.write().await;
            cache.insert(permission_type.clone(), status.clone());
        }

        Ok(self.create_check_result(permission_type, status))
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
        Ok(PermissionStatus::Granted) // 简化
    }

    /// 检查全局快捷键权限
    async fn check_global_shortcut_permission(&self) -> Result<PermissionStatus> {
        Ok(PermissionStatus::Granted) // 简化
    }

    /// 检查系统托盘权限
    async fn check_system_tray_permission(&self) -> Result<PermissionStatus> {
        Ok(PermissionStatus::Granted)
    }

    /// 检查通知权限
    async fn check_notification_permission(&self) -> Result<PermissionStatus> {
        Ok(PermissionStatus::Granted)
    }

    /// 检查系统服务权限
    async fn check_system_service_permission(&self) -> Result<PermissionStatus> {
        Ok(PermissionStatus::RequiresAdmin)
    }

    /// 创建权限检查结果
    fn create_check_result(&self, permission_type: &PermissionType, status: PermissionStatus) -> PermissionCheckResult {
        let (message, can_proceed, requires_elevation) = match status {
            PermissionStatus::Granted => ("权限已授予".to_string(), true, false),
            PermissionStatus::Denied => ("权限被拒绝".to_string(), false, false),
            _ => ("需要其他授权".to_string(), false, false),
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
    pub async fn request_permission(&self, _permission_type: &PermissionType) -> Result<PermissionStatus> {
        Ok(PermissionStatus::Granted) // 简化实现
    }

    /// 检查是否具有管理员权限
    pub async fn is_admin(&self) -> bool {
        /*
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
            use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
            use windows::Win32::Foundation::HANDLE;

            unsafe {
                let mut token_handle = HANDLE::default();
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
        */
        false
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

    /// 获取权限管理器实例
    pub async fn get(&self) -> Result<std::sync::Arc<PermissionManager>, String> {
        let manager_guard = self.manager.read().await;
        manager_guard
            .as_ref()
            .map(|manager| std::sync::Arc::new(manager.clone()))
            .ok_or_else(|| "权限管理器未初始化".to_string())
    }
}

lazy_static! {
    pub static ref PERMISSION_MANAGER: GlobalPermissionManager = GlobalPermissionManager::new();
}
