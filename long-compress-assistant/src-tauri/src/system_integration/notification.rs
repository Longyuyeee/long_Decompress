use tauri::AppHandle;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{Result, anyhow};
use lazy_static::lazy_static;
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    Info,
    Warning,
    Error,
    Success,
    TaskStarted,
    TaskProgress,
    TaskCompleted,
    TaskFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRequest {
    pub title: String,
    pub body: String,
    pub notification_type: NotificationType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationHistory {
    pub title: String,
    pub body: String,
    pub notification_type: NotificationType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub enabled: bool,
}

#[derive(Clone)]
pub struct SystemNotifier {
    history: Arc<RwLock<VecDeque<NotificationHistory>>>,
}

impl SystemNotifier {
    pub fn new() -> Self {
        Self {
            history: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
        }
    }

    pub async fn send_notification(&self, request: NotificationRequest) -> Result<()> {
        let mut history = self.history.write().await;
        history.push_back(NotificationHistory {
            title: request.title,
            body: request.body,
            notification_type: request.notification_type,
            timestamp: chrono::Utc::now(),
        });
        Ok(())
    }

    pub async fn get_history(&self) -> Vec<NotificationHistory> {
        let history = self.history.read().await;
        history.iter().cloned().collect()
    }
}

pub struct GlobalNotifier {
    notifier: Arc<RwLock<Option<SystemNotifier>>>,
}

impl GlobalNotifier {
    pub fn new() -> Self {
        Self {
            notifier: Arc::new(RwLock::new(Some(SystemNotifier::new()))),
        }
    }

    pub async fn send_notification(&self, request: NotificationRequest) -> Result<()> {
        let notifier_guard = self.notifier.read().await;
        if let Some(notifier) = notifier_guard.as_ref() {
            notifier.send_notification(request).await
        } else {
            Err(anyhow!("通知管理器未初始化"))
        }
    }

    pub async fn get_history(&self) -> Vec<NotificationHistory> {
        let notifier_guard = self.notifier.read().await;
        if let Some(notifier) = notifier_guard.as_ref() {
            notifier.get_history().await
        } else {
            Vec::new()
        }
    }

    pub async fn get(&self) -> Result<Arc<SystemNotifier>, String> {
        let notifier_guard = self.notifier.read().await;
        notifier_guard
            .as_ref()
            .map(|n| Arc::new(n.clone()))
            .ok_or_else(|| "通知管理器未初始化".to_string())
    }
}

lazy_static! {
    pub static ref NOTIFIER: GlobalNotifier = GlobalNotifier::new();
}

pub struct NotificationManager;
impl NotificationManager {
    pub fn new() -> Self { Self }
}
