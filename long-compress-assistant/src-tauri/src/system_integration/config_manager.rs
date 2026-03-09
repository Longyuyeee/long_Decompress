//! 系统集成配置管理模块
//!
//! 提供统一的系统集成配置管理功能。

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{Result, anyhow};

/// 系统集成配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemIntegrationConfig {
    /// 文件关联配置
    pub file_association: crate::system_integration::FileAssociationConfig,
    /// 全局快捷键配置
    pub global_shortcut: crate::system_integration::ShortcutConfig,
    /// 系统托盘配置
    pub system_tray: crate::system_integration::TrayConfig,
    /// 通知配置
    pub notification: crate::system_integration::NotificationConfig,
    /// 权限管理配置
    pub permission: crate::system_integration::PermissionConfig,
    /// 平台兼容性配置
    pub platform_compatibility: PlatformCompatibilityConfig,
    /// 通用配置
    pub general: GeneralIntegrationConfig,
}

/// 平台兼容性配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformCompatibilityConfig {
    /// 启用平台检测
    pub enable_platform_detection: bool,
    /// 自动适配平台特性
    pub auto_adapt_platform_features: bool,
    /// 显示平台兼容性警告
    pub show_compatibility_warnings: bool,
    /// 平台特定配置
    pub platform_specific: PlatformSpecificConfig,
}

/// 平台特定配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformSpecificConfig {
    /// Windows特定配置
    pub windows: WindowsIntegrationConfig,
    /// macOS特定配置
    pub macos: MacOSIntegrationConfig,
    /// Linux特定配置
    pub linux: LinuxIntegrationConfig,
}

/// Windows集成配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsIntegrationConfig {
    /// 使用管理员权限
    pub use_admin_privileges: bool,
    /// 注册表操作超时(秒)
    pub registry_operation_timeout: u32,
    /// 启用Windows通知
    pub enable_windows_notifications: bool,
    /// 启用任务栏集成
    pub enable_taskbar_integration: bool,
}

/// macOS集成配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacOSIntegrationConfig {
    /// 启用辅助功能权限请求
    pub enable_accessibility_permission_request: bool,
    /// 启用通知权限请求
    pub enable_notification_permission_request: bool,
    /// 启用Dock集成
    pub enable_dock_integration: bool,
    /// 启用菜单栏集成
    pub enable_menu_bar_integration: bool,
}

/// Linux集成配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxIntegrationConfig {
    /// 启用DBus集成
    pub enable_dbus_integration: bool,
    /// 启用系统托盘协议检测
    pub enable_tray_protocol_detection: bool,
    /// 启用桌面环境检测
    pub enable_desktop_environment_detection: bool,
    /// 启用自动启动集成
    pub enable_autostart_integration: bool,
}

/// 通用集成配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralIntegrationConfig {
    /// 启用所有系统集成功能
    pub enable_all_integrations: bool,
    /// 自动初始化系统集成
    pub auto_initialize_integrations: bool,
    /// 显示集成状态通知
    pub show_integration_status_notifications: bool,
    /// 记录集成操作日志
    pub log_integration_operations: bool,
    /// 集成操作超时(秒)
    pub integration_operation_timeout: u32,
    /// 启用错误恢复
    pub enable_error_recovery: bool,
    /// 最大重试次数
    pub max_retry_count: u32,
}

impl Default for SystemIntegrationConfig {
    fn default() -> Self {
        Self {
            file_association: crate::system_integration::FileAssociationConfig::default(),
            global_shortcut: crate::system_integration::ShortcutConfig::default(),
            system_tray: crate::system_integration::TrayConfig::default(),
            notification: crate::system_integration::NotificationConfig::default(),
            permission: crate::system_integration::PermissionConfig::default(),
            platform_compatibility: PlatformCompatibilityConfig::default(),
            general: GeneralIntegrationConfig::default(),
        }
    }
}

impl Default for PlatformCompatibilityConfig {
    fn default() -> Self {
        Self {
            enable_platform_detection: true,
            auto_adapt_platform_features: true,
            show_compatibility_warnings: true,
            platform_specific: PlatformSpecificConfig::default(),
        }
    }
}

impl Default for PlatformSpecificConfig {
    fn default() -> Self {
        Self {
            windows: WindowsIntegrationConfig::default(),
            macos: MacOSIntegrationConfig::default(),
            linux: LinuxIntegrationConfig::default(),
        }
    }
}

impl Default for WindowsIntegrationConfig {
    fn default() -> Self {
        Self {
            use_admin_privileges: false,
            registry_operation_timeout: 10,
            enable_windows_notifications: true,
            enable_taskbar_integration: true,
        }
    }
}

impl Default for MacOSIntegrationConfig {
    fn default() -> Self {
        Self {
            enable_accessibility_permission_request: true,
            enable_notification_permission_request: true,
            enable_dock_integration: true,
            enable_menu_bar_integration: true,
        }
    }
}

impl Default for LinuxIntegrationConfig {
    fn default() -> Self {
        Self {
            enable_dbus_integration: true,
            enable_tray_protocol_detection: true,
            enable_desktop_environment_detection: true,
            enable_autostart_integration: true,
        }
    }
}

impl Default for GeneralIntegrationConfig {
    fn default() -> Self {
        Self {
            enable_all_integrations: true,
            auto_initialize_integrations: true,
            show_integration_status_notifications: true,
            log_integration_operations: true,
            integration_operation_timeout: 30,
            enable_error_recovery: true,
            max_retry_count: 3,
        }
    }
}

/// 系统集成状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemIntegrationStatus {
    /// 文件关联状态
    pub file_association: IntegrationStatus,
    /// 全局快捷键状态
    pub global_shortcut: IntegrationStatus,
    /// 系统托盘状态
    pub system_tray: IntegrationStatus,
    /// 通知系统状态
    pub notification: IntegrationStatus,
    /// 权限管理系统状态
    pub permission: IntegrationStatus,
    /// 平台兼容性状态
    pub platform_compatibility: IntegrationStatus,
    /// 总体状态
    pub overall_status: IntegrationStatus,
    /// 平台信息
    pub platform_info: PlatformInfo,
    /// 最后更新时间
    pub last_updated: String,
}

/// 集成状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IntegrationStatus {
    /// 未初始化
    NotInitialized,
    /// 初始化中
    Initializing,
    /// 已初始化
    Initialized,
    /// 运行中
    Running,
    /// 已停止
    Stopped,
    /// 错误
    Error(String),
    /// 不支持
    NotSupported,
}

/// 平台信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    /// 平台类型
    pub platform_type: crate::system_integration::PlatformType,
    /// 操作系统版本
    pub os_version: Option<String>,
    /// 桌面环境（Linux）
    pub desktop_environment: Option<String>,
    /// 架构
    pub architecture: String,
    /// 是否具有管理员权限
    pub has_admin_privileges: bool,
}

/// 系统集成配置管理器
pub struct SystemIntegrationConfigManager {
    config: Arc<RwLock<SystemIntegrationConfig>>,
    status: Arc<RwLock<SystemIntegrationStatus>>,
}

impl SystemIntegrationConfigManager {
    /// 创建新的配置管理器
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(SystemIntegrationConfig::default())),
            status: Arc::new(RwLock::new(SystemIntegrationStatus::default())),
        }
    }

    /// 获取配置
    pub async fn get_config(&self) -> SystemIntegrationConfig {
        self.config.read().await.clone()
    }

    /// 更新配置
    pub async fn update_config(&self, config: SystemIntegrationConfig) {
        let mut current_config = self.config.write().await;
        *current_config = config;
    }

    /// 获取状态
    pub async fn get_status(&self) -> SystemIntegrationStatus {
        self.status.read().await.clone()
    }

    /// 更新状态
    pub async fn update_status(&self, status: SystemIntegrationStatus) {
        let mut current_status = self.status.write().await;
        *current_status = status;
    }

    /// 更新特定集成状态
    pub async fn update_integration_status(
        &self,
        integration_type: IntegrationType,
        status: IntegrationStatus,
    ) {
        let mut current_status = self.status.write().await;

        match integration_type {
            IntegrationType::FileAssociation => current_status.file_association = status,
            IntegrationType::GlobalShortcut => current_status.global_shortcut = status,
            IntegrationType::SystemTray => current_status.system_tray = status,
            IntegrationType::Notification => current_status.notification = status,
            IntegrationType::Permission => current_status.permission = status,
            IntegrationType::PlatformCompatibility => current_status.platform_compatibility = status,
        }

        // 更新总体状态
        current_status.overall_status = self.calculate_overall_status(&current_status);
        current_status.last_updated = chrono::Utc::now().to_rfc3339();
    }

    /// 计算总体状态
    fn calculate_overall_status(&self, status: &SystemIntegrationStatus) -> IntegrationStatus {
        let statuses = vec![
            &status.file_association,
            &status.global_shortcut,
            &status.system_tray,
            &status.notification,
            &status.permission,
            &status.platform_compatibility,
        ];

        // 检查是否有错误状态
        for status in &statuses {
            if let IntegrationStatus::Error(_) = status {
                return IntegrationStatus::Error("至少一个集成组件出错".to_string());
            }
        }

        // 检查是否所有组件都已初始化
        let all_initialized = statuses.iter().all(|s| {
            matches!(s, IntegrationStatus::Initialized | IntegrationStatus::Running | IntegrationStatus::NotSupported)
        });

        if all_initialized {
            IntegrationStatus::Running
        } else {
            IntegrationStatus::Initializing
        }
    }

    /// 获取平台特定的配置
    pub async fn get_platform_specific_config(&self) -> Result<PlatformSpecificConfig> {
        let config = self.config.read().await;
        Ok(config.platform_compatibility.platform_specific.clone())
    }

    /// 根据当前平台获取适配的配置
    pub async fn get_adapted_config(&self) -> SystemIntegrationConfig {
        let mut config = self.get_config().await;
        let platform = crate::system_integration::PlatformCompatibilityChecker::get_current_platform();

        // 根据平台调整配置
        match platform {
            crate::system_integration::PlatformType::Windows => {
                // Windows特定调整
                config.file_association.enabled = true;
                config.global_shortcut.enabled = true;
            }
            crate::system_integration::PlatformType::MacOS => {
                // macOS特定调整
                config.system_tray.enabled = true;
                config.notification.enabled = true;
            }
            crate::system_integration::PlatformType::Linux => {
                // Linux特定调整
                config.file_association.enabled = true;
                config.system_tray.enabled = true;
            }
            _ => {
                // 未知平台，禁用部分功能
                config.file_association.enabled = false;
                config.global_shortcut.enabled = false;
            }
        }

        config
    }

    /// 导出配置到JSON
    pub async fn export_config(&self) -> Result<String> {
        let config = self.get_config().await;
        serde_json::to_string_pretty(&config)
            .map_err(|e| anyhow!("序列化配置失败: {}", e))
    }

    /// 从JSON导入配置
    pub async fn import_config(&self, config_json: &str) -> Result<()> {
        let config: SystemIntegrationConfig = serde_json::from_str(config_json)
            .map_err(|e| anyhow!("解析配置失败: {}", e))?;

        self.update_config(config).await;
        Ok(())
    }

    /// 重置为默认配置
    pub async fn reset_to_default(&self) {
        self.update_config(SystemIntegrationConfig::default()).await;
    }

    /// 验证配置
    pub async fn validate_config(&self) -> Result<Vec<String>> {
        let config = self.get_config().await;
        let mut warnings = Vec::new();

        // 验证文件关联配置
        if config.file_association.enabled && config.file_association.associations.is_empty() {
            warnings.push("文件关联已启用但未配置任何关联类型".to_string());
        }

        // 验证全局快捷键配置
        if config.global_shortcut.enabled && config.global_shortcut.shortcuts.is_empty() {
            warnings.push("全局快捷键已启用但未配置任何快捷键".to_string());
        }

        // 验证权限配置
        if config.permission.auto_check_permissions && config.permission.permission_requests.is_empty() {
            warnings.push("自动权限检查已启用但未配置任何权限请求".to_string());
        }

        Ok(warnings)
    }
}

impl Default for SystemIntegrationStatus {
    fn default() -> Self {
        Self {
            file_association: IntegrationStatus::NotInitialized,
            global_shortcut: IntegrationStatus::NotInitialized,
            system_tray: IntegrationStatus::NotInitialized,
            notification: IntegrationStatus::NotInitialized,
            permission: IntegrationStatus::NotInitialized,
            platform_compatibility: IntegrationStatus::NotInitialized,
            overall_status: IntegrationStatus::NotInitialized,
            platform_info: PlatformInfo::default(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        }
    }
}

impl Default for PlatformInfo {
    fn default() -> Self {
        Self {
            platform_type: crate::system_integration::PlatformCompatibilityChecker::get_current_platform(),
            os_version: None,
            desktop_environment: None,
            architecture: std::env::consts::ARCH.to_string(),
            has_admin_privileges: false,
        }
    }
}

/// 集成类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum IntegrationType {
    FileAssociation,
    GlobalShortcut,
    SystemTray,
    Notification,
    Permission,
    PlatformCompatibility,
}

/// 全局配置管理器实例
pub static SYSTEM_INTEGRATION_CONFIG_MANAGER: once_cell::sync::Lazy<SystemIntegrationConfigManager> = once_cell::sync::Lazy::new(|| {
    SystemIntegrationConfigManager::new()
});