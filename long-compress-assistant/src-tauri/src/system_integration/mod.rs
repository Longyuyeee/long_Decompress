//! 系统集成模块
//!
//! 提供系统通知、托盘集成、文件关联等系统级功能。

pub mod notification;

// 重新导出主要类型
pub use notification::{
    SystemNotifier, GlobalNotifier, NOTIFIER,
    NotificationType, NotificationPriority, NotificationConfig,
    NotificationRequest, NotificationAction, NotificationHistory,
};