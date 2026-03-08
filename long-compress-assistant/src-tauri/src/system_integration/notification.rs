use tauri::{AppHandle, Manager};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 通知类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    TaskCompleted,
    TaskFailed,
    TaskProgress,
    SystemAlert,
    PasswordHint,
    UpdateAvailable,
}

/// 通知优先级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// 通知配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub enabled: bool,
    pub sound_enabled: bool,
    pub duration_ms: u32,
    pub max_notifications: usize,
    pub show_progress: bool,
    pub show_in_tray: bool,
    pub priority_filter: Vec<NotificationPriority>,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sound_enabled: true,
            duration_ms: 5000,
            max_notifications: 10,
            show_progress: true,
            show_in_tray: true,
            priority_filter: vec![
                NotificationPriority::Normal,
                NotificationPriority::High,
                NotificationPriority::Critical,
            ],
        }
    }
}

/// 通知请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRequest {
    pub title: String,
    pub body: String,
    pub notification_type: NotificationType,
    pub priority: NotificationPriority,
    pub icon: Option<String>,
    pub sound: Option<String>,
    pub actions: Vec<NotificationAction>,
    pub metadata: serde_json::Value,
}

/// 通知操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    pub id: String,
    pub title: String,
    pub icon: Option<String>,
}

/// 通知历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationHistory {
    pub id: String,
    pub title: String,
    pub body: String,
    pub notification_type: NotificationType,
    pub priority: NotificationPriority,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub read: bool,
    pub action_taken: Option<String>,
    pub metadata: serde_json::Value,
}

/// 系统通知管理器
pub struct SystemNotifier {
    app_handle: AppHandle,
    config: Arc<RwLock<NotificationConfig>>,
    history: Arc<RwLock<Vec<NotificationHistory>>>,
}

impl SystemNotifier {
    /// 创建新的系统通知管理器
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            config: Arc::new(RwLock::new(NotificationConfig::default())),
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 发送通知
    pub async fn send_notification(&self, request: NotificationRequest) -> Result<(), String> {
        let config = self.config.read().await;

        // 检查通知是否启用
        if !config.enabled {
            return Ok(()); // 通知被禁用，静默成功
        }

        // 检查优先级过滤
        if !config.priority_filter.contains(&request.priority) {
            return Ok(()); // 优先级被过滤，静默成功
        }

        // 构建通知选项
        let mut notification = tauri::api::notification::Notification::new(&self.app_handle.config().tauri.bundle.identifier)
            .title(&request.title)
            .body(&request.body);

        // 设置图标（如果提供）
        if let Some(icon) = &request.icon {
            notification = notification.icon(icon);
        }

        // 发送通知
        if let Err(e) = notification.show() {
            return Err(format!("发送通知失败: {}", e));
        }

        // 记录到历史
        self.add_to_history(&request).await;

        // 发送前端事件
        self.emit_notification_event(&request).await;

        Ok(())
    }

    /// 发送任务完成通知
    pub async fn send_task_completed_notification(
        &self,
        task_id: &str,
        task_name: &str,
        output_path: &str,
    ) -> Result<(), String> {
        let request = NotificationRequest {
            title: format!("任务完成: {}", task_name),
            body: format!("任务 {} 已完成\n输出路径: {}", task_id, output_path),
            notification_type: NotificationType::TaskCompleted,
            priority: NotificationPriority::Normal,
            icon: Some("check-circle".to_string()),
            sound: Some("default".to_string()),
            actions: vec![
                NotificationAction {
                    id: "view_details".to_string(),
                    title: "查看详情".to_string(),
                    icon: Some("eye".to_string()),
                },
                NotificationAction {
                    id: "open_folder".to_string(),
                    title: "打开文件夹".to_string(),
                    icon: Some("folder-open".to_string()),
                },
            ],
            metadata: serde_json::json!({
                "task_id": task_id,
                "task_name": task_name,
                "output_path": output_path,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        };

        self.send_notification(request).await
    }

    /// 发送任务失败通知
    pub async fn send_task_failed_notification(
        &self,
        task_id: &str,
        task_name: &str,
        error_message: &str,
    ) -> Result<(), String> {
        let request = NotificationRequest {
            title: format!("任务失败: {}", task_name),
            body: format!("任务 {} 失败\n错误: {}", task_id, error_message),
            notification_type: NotificationType::TaskFailed,
            priority: NotificationPriority::High,
            icon: Some("x-circle".to_string()),
            sound: Some("alert".to_string()),
            actions: vec![
                NotificationAction {
                    id: "view_details".to_string(),
                    title: "查看详情".to_string(),
                    icon: Some("eye".to_string()),
                },
                NotificationAction {
                    id: "retry".to_string(),
                    title: "重试".to_string(),
                    icon: Some("refresh-cw".to_string()),
                },
            ],
            metadata: serde_json::json!({
                "task_id": task_id,
                "task_name": task_name,
                "error_message": error_message,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        };

        self.send_notification(request).await
    }

    /// 发送任务进度通知
    pub async fn send_task_progress_notification(
        &self,
        task_id: &str,
        task_name: &str,
        progress: f32,
    ) -> Result<(), String> {
        let config = self.config.read().await;
        if !config.show_progress {
            return Ok(());
        }

        let request = NotificationRequest {
            title: format!("任务进度: {}", task_name),
            body: format!("任务 {}: {:.1}% 完成", task_id, progress),
            notification_type: NotificationType::TaskProgress,
            priority: NotificationPriority::Low,
            icon: Some("activity".to_string()),
            sound: None,
            actions: vec![NotificationAction {
                id: "view_progress".to_string(),
                title: "查看进度".to_string(),
                icon: Some("bar-chart".to_string()),
            }],
            metadata: serde_json::json!({
                "task_id": task_id,
                "task_name": task_name,
                "progress": progress,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        };

        self.send_notification(request).await
    }

    /// 发送系统警告通知
    pub async fn send_system_alert_notification(
        &self,
        title: &str,
        message: &str,
        severity: &str,
    ) -> Result<(), String> {
        let priority = match severity {
            "warning" => NotificationPriority::High,
            "error" => NotificationPriority::Critical,
            _ => NotificationPriority::Normal,
        };

        let request = NotificationRequest {
            title: title.to_string(),
            body: message.to_string(),
            notification_type: NotificationType::SystemAlert,
            priority,
            icon: Some(match severity {
                "warning" => "alert-triangle".to_string(),
                "error" => "alert-circle".to_string(),
                _ => "info".to_string(),
            }),
            sound: Some(match severity {
                "warning" => "alert".to_string(),
                "error" => "alarm".to_string(),
                _ => "default".to_string(),
            }),
            actions: vec![NotificationAction {
                id: "dismiss".to_string(),
                title: "知道了".to_string(),
                icon: Some("x".to_string()),
            }],
            metadata: serde_json::json!({
                "severity": severity,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        };

        self.send_notification(request).await
    }

    /// 添加通知到历史记录
    async fn add_to_history(&self, request: &NotificationRequest) {
        let mut history = self.history.write().await;

        let history_item = NotificationHistory {
            id: uuid::Uuid::new_v4().to_string(),
            title: request.title.clone(),
            body: request.body.clone(),
            notification_type: request.notification_type.clone(),
            priority: request.priority.clone(),
            timestamp: chrono::Utc::now(),
            read: false,
            action_taken: None,
            metadata: request.metadata.clone(),
        };

        history.push(history_item);

        // 限制历史记录数量
        let config = self.config.read().await;
        if history.len() > config.max_notifications {
            history.remove(0); // 移除最旧的通知
        }
    }

    /// 发送通知事件到前端
    async fn emit_notification_event(&self, request: &NotificationRequest) {
        let event_data = serde_json::json!({
            "title": request.title,
            "body": request.body,
            "type": format!("{:?}", request.notification_type),
            "priority": format!("{:?}", request.priority),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "metadata": request.metadata,
        });

        if let Err(e) = self.app_handle.emit_all("notification_received", event_data) {
            log::error!("发送通知事件失败: {}", e);
        }
    }

    /// 获取通知历史
    pub async fn get_notification_history(
        &self,
        limit: Option<usize>,
        unread_only: bool,
    ) -> Vec<NotificationHistory> {
        let history = self.history.read().await;

        let mut filtered_history: Vec<NotificationHistory> = if unread_only {
            history.iter()
                .filter(|item| !item.read)
                .cloned()
                .collect()
        } else {
            history.clone()
        };

        // 按时间倒序排序（最新的在前面）
        filtered_history.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // 应用限制
        if let Some(limit) = limit {
            filtered_history.truncate(limit);
        }

        filtered_history
    }

    /// 标记通知为已读
    pub async fn mark_as_read(&self, notification_id: &str) -> bool {
        let mut history = self.history.write().await;

        for item in history.iter_mut() {
            if item.id == notification_id {
                item.read = true;
                return true;
            }
        }

        false
    }

    /// 标记所有通知为已读
    pub async fn mark_all_as_read(&self) -> usize {
        let mut history = self.history.write().await;
        let mut count = 0;

        for item in history.iter_mut() {
            if !item.read {
                item.read = true;
                count += 1;
            }
        }

        count
    }

    /// 清除通知历史
    pub async fn clear_history(&self) -> usize {
        let mut history = self.history.write().await;
        let count = history.len();
        history.clear();
        count
    }

    /// 获取配置
    pub async fn get_config(&self) -> NotificationConfig {
        let config = self.config.read().await;
        config.clone()
    }

    /// 更新配置
    pub async fn update_config(&self, new_config: NotificationConfig) {
        let mut config = self.config.write().await;
        *config = new_config;
    }

    /// 获取未读通知数量
    pub async fn get_unread_count(&self) -> usize {
        let history = self.history.read().await;
        history.iter().filter(|item| !item.read).count()
    }
}

/// 全局通知管理器
pub struct GlobalNotifier {
    notifier: Arc<RwLock<Option<SystemNotifier>>>,
}

impl GlobalNotifier {
    /// 创建新的全局通知管理器
    pub fn new() -> Self {
        Self {
            notifier: Arc::new(RwLock::new(None)),
        }
    }

    /// 初始化全局通知管理器
    pub async fn initialize(&self, app_handle: AppHandle) -> Result<(), String> {
        let mut notifier_guard = self.notifier.write().await;

        if notifier_guard.is_none() {
            let notifier = SystemNotifier::new(app_handle);
            *notifier_guard = Some(notifier);
            log::info!("全局通知管理器初始化完成");
        }

        Ok(())
    }

    /// 获取通知管理器实例
    pub async fn get(&self) -> Result<Arc<SystemNotifier>, String> {
        let notifier_guard = self.notifier.read().await;
        notifier_guard
            .as_ref()
            .cloned()
            .ok_or_else(|| "通知管理器未初始化".to_string())
    }
}

// 实现默认的全局实例
lazy_static::lazy_static! {
    pub static ref NOTIFIER: GlobalNotifier = GlobalNotifier::new();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::test::MockRuntime;

    #[tokio::test]
    async fn test_notification_request_serialization() {
        let request = NotificationRequest {
            title: "测试通知".to_string(),
            body: "这是一个测试通知".to_string(),
            notification_type: NotificationType::TaskCompleted,
            priority: NotificationPriority::Normal,
            icon: Some("test-icon".to_string()),
            sound: Some("default".to_string()),
            actions: vec![NotificationAction {
                id: "test_action".to_string(),
                title: "测试操作".to_string(),
                icon: Some("test-icon".to_string()),
            }],
            metadata: serde_json::json!({"test": "data"}),
        };

        // 测试序列化
        let serialized = serde_json::to_string(&request).unwrap();
        assert!(serialized.contains("测试通知"));

        // 测试反序列化
        let deserialized: NotificationRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.title, "测试通知");
    }

    #[test]
    fn test_notification_config_default() {
        let config = NotificationConfig::default();

        assert!(config.enabled);
        assert!(config.sound_enabled);
        assert_eq!(config.duration_ms, 5000);
        assert_eq!(config.max_notifications, 10);
        assert!(config.show_progress);
        assert!(config.show_in_tray);
        assert!(config.priority_filter.contains(&NotificationPriority::Normal));
        assert!(config.priority_filter.contains(&NotificationPriority::High));
        assert!(config.priority_filter.contains(&NotificationPriority::Critical));
    }

    #[tokio::test]
    async fn test_notification_priority_filter() {
        let config = NotificationConfig {
            priority_filter: vec![NotificationPriority::High, NotificationPriority::Critical],
            ..Default::default()
        };

        // 低优先级应该被过滤
        assert!(!config.priority_filter.contains(&NotificationPriority::Low));

        // 普通优先级应该被过滤
        assert!(!config.priority_filter.contains(&NotificationPriority::Normal));

        // 高优先级应该通过
        assert!(config.priority_filter.contains(&NotificationPriority::High));

        // 关键优先级应该通过
        assert!(config.priority_filter.contains(&NotificationPriority::Critical));
    }
}