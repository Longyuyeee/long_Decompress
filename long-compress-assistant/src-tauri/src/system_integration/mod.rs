//! 系统集成模块
//!
//! 提供系统通知、托盘集成、文件关联等系统级功能。

pub mod notification;
pub mod tray;
pub mod file_association;
pub mod global_shortcut;
pub mod permission_manager;
pub mod platform_compatibility;
pub mod config_manager;

// 重新导出主要类型
pub use notification::{
    SystemNotifier, GlobalNotifier, NOTIFIER,
    NotificationType, NotificationPriority, NotificationConfig,
    NotificationRequest, NotificationAction, NotificationHistory,
};

pub use tray::{
    SystemTrayManager, GlobalTrayManager, TRAY_MANAGER,
    TrayConfig, TrayIconType, TrayMenuItemConfig,
};

pub use file_association::{
    FileAssociationManager, GlobalFileAssociationManager, FILE_ASSOCIATION_MANAGER,
    FileAssociationConfig, FileTypeAssociation, ContextMenuItem,
};

pub use global_shortcut::{
    AppShortcutManager, AppShortcutManagerWrapper, GLOBAL_SHORTCUT_MANAGER,
    ShortcutConfig, ShortcutDefinition, ShortcutAction, ShortcutConflict,
};

pub use permission_manager::{
    PermissionManager, GlobalPermissionManager, PERMISSION_MANAGER,
    PermissionType, PermissionStatus, PermissionRequest, PermissionConfig, PermissionCheckResult,
};

pub use platform_compatibility::{
    PlatformCompatibilityChecker,
    PlatformType, FeatureSupport, PlatformFeatureCheck, SystemIntegrationFeature,
};

pub use config_manager::{
    SystemIntegrationConfigManager, SYSTEM_INTEGRATION_CONFIG_MANAGER,
    SystemIntegrationConfig, PlatformCompatibilityConfig, PlatformSpecificConfig,
    WindowsIntegrationConfig, MacOSIntegrationConfig, LinuxIntegrationConfig,
    GeneralIntegrationConfig, SystemIntegrationStatus, IntegrationStatus,
    PlatformInfo, IntegrationType,
};
