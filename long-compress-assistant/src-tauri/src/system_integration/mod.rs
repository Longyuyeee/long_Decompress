pub mod file_association;
pub mod global_shortcut;
pub mod notification;
pub mod permission_manager;
pub mod platform_compatibility;

#[cfg(all(desktop, feature = "system-tray"))]
pub mod tray;

pub use file_association::FileAssociationManager;
pub use tauri::GlobalShortcutManager;
pub use notification::{NotificationManager, NOTIFIER, NotificationRequest, NotificationConfig, NotificationHistory, NotificationType};
pub use permission_manager::{PermissionManager, PermissionType, PermissionStatus};
pub use platform_compatibility::{PlatformCompatibilityChecker, PlatformType, FeatureSupport, PlatformFeatureCheck, SystemIntegrationFeature};

#[cfg(all(desktop, feature = "system-tray"))]
pub use tray::{setup_tray, handle_tray_event};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IntegrationType {
    FileAssociation,
    GlobalShortcut,
    SystemTray,
    Notification,
    Permission,
    PlatformCompatibility,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IntegrationStatus {
    NotInitialized,
    Initializing,
    Initialized,
    Running,
    Stopped,
    NotSupported,
    Error(String),
}
