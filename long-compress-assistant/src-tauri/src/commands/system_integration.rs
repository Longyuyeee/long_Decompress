use crate::system_integration::{NOTIFIER, NotificationRequest, NotificationConfig, NotificationHistory};
use tauri::command;
use serde::{Deserialize, Serialize};

/// 发送通知请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendNotificationRequest {
    pub title: String,
    pub body: String,
    pub notification_type: String,
    pub priority: String,
    pub icon: Option<String>,
    pub sound: Option<String>,
    pub actions: Vec<NotificationActionRequest>,
    pub metadata: serde_json::Value,
}

/// 通知操作请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationActionRequest {
    pub id: String,
    pub title: String,
    pub icon: Option<String>,
}

/// 通知历史响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationHistoryResponse {
    pub id: String,
    pub title: String,
    pub body: String,
    pub notification_type: String,
    pub priority: String,
    pub timestamp: String,
    pub read: bool,
    pub action_taken: Option<String>,
    pub metadata: serde_json::Value,
}

/// 通知统计响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationStatsResponse {
    pub total_notifications: usize,
    pub unread_notifications: usize,
    pub notification_types: Vec<NotificationTypeStats>,
    pub recent_notifications: Vec<NotificationHistoryResponse>,
}

/// 通知类型统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationTypeStats {
    pub notification_type: String,
    pub count: usize,
    pub unread_count: usize,
}

#[command]
pub async fn send_notification(request: SendNotificationRequest) -> Result<(), String> {
    let notifier = NOTIFIER.get().await
        .map_err(|e| format!("获取通知管理器失败: {}", e))?;

    // 转换通知类型
    let notification_type = match request.notification_type.to_lowercase().as_str() {
        "task_completed" => crate::system_integration::NotificationType::TaskCompleted,
        "task_failed" => crate::system_integration::NotificationType::TaskFailed,
        "task_progress" => crate::system_integration::NotificationType::TaskProgress,
        "system_alert" => crate::system_integration::NotificationType::SystemAlert,
        "password_hint" => crate::system_integration::NotificationType::PasswordHint,
        "update_available" => crate::system_integration::NotificationType::UpdateAvailable,
        _ => return Err(format!("未知的通知类型: {}", request.notification_type)),
    };

    // 转换优先级
    let priority = match request.priority.to_lowercase().as_str() {
        "low" => crate::system_integration::NotificationPriority::Low,
        "normal" => crate::system_integration::NotificationPriority::Normal,
        "high" => crate::system_integration::NotificationPriority::High,
        "critical" => crate::system_integration::NotificationPriority::Critical,
        _ => return Err(format!("未知的优先级: {}", request.priority)),
    };

    // 转换操作
    let actions = request.actions.into_iter()
        .map(|action| crate::system_integration::NotificationAction {
            id: action.id,
            title: action.title,
            icon: action.icon,
        })
        .collect();

    let notification_request = NotificationRequest {
        title: request.title,
        body: request.body,
        notification_type,
        priority,
        icon: request.icon,
        sound: request.sound,
        actions,
        metadata: request.metadata,
    };

    notifier.send_notification(notification_request).await
}

#[command]
pub async fn send_task_completed_notification(
    task_id: String,
    task_name: String,
    output_path: String,
) -> Result<(), String> {
    let notifier = NOTIFIER.get().await
        .map_err(|e| format!("获取通知管理器失败: {}", e))?;

    notifier.send_task_completed_notification(&task_id, &task_name, &output_path).await
}

#[command]
pub async fn send_task_failed_notification(
    task_id: String,
    task_name: String,
    error_message: String,
) -> Result<(), String> {
    let notifier = NOTIFIER.get().await
        .map_err(|e| format!("获取通知管理器失败: {}", e))?;

    notifier.send_task_failed_notification(&task_id, &task_name, &error_message).await
}

#[command]
pub async fn send_task_progress_notification(
    task_id: String,
    task_name: String,
    progress: f32,
) -> Result<(), String> {
    let notifier = NOTIFIER.get().await
        .map_err(|e| format!("获取通知管理器失败: {}", e))?;

    notifier.send_task_progress_notification(&task_id, &task_name, progress).await
}

#[command]
pub async fn send_system_alert_notification(
    title: String,
    message: String,
    severity: String,
) -> Result<(), String> {
    let notifier = NOTIFIER.get().await
        .map_err(|e| format!("获取通知管理器失败: {}", e))?;

    notifier.send_system_alert_notification(&title, &message, &severity).await
}

#[command]
pub async fn get_notification_config() -> Result<NotificationConfig, String> {
    let notifier = NOTIFIER.get().await
        .map_err(|e| format!("获取通知管理器失败: {}", e))?;

    Ok(notifier.get_config().await)
}

#[command]
pub async fn update_notification_config(config: NotificationConfig) -> Result<(), String> {
    let notifier = NOTIFIER.get().await
        .map_err(|e| format!("获取通知管理器失败: {}", e))?;

    notifier.update_config(config).await;
    Ok(())
}

#[command]
pub async fn get_notification_history(
    limit: Option<usize>,
    unread_only: bool,
) -> Result<Vec<NotificationHistoryResponse>, String> {
    let notifier = NOTIFIER.get().await
        .map_err(|e| format!("获取通知管理器失败: {}", e))?;

    let history = notifier.get_notification_history(limit, unread_only).await;

    let response = history.into_iter()
        .map(|item| NotificationHistoryResponse {
            id: item.id,
            title: item.title,
            body: item.body,
            notification_type: format!("{:?}", item.notification_type),
            priority: format!("{:?}", item.priority),
            timestamp: item.timestamp.to_rfc3339(),
            read: item.read,
            action_taken: item.action_taken,
            metadata: item.metadata,
        })
        .collect();

    Ok(response)
}

#[command]
pub async fn mark_notification_as_read(notification_id: String) -> Result<bool, String> {
    let notifier = NOTIFIER.get().await
        .map_err(|e| format!("获取通知管理器失败: {}", e))?;

    Ok(notifier.mark_as_read(&notification_id).await)
}

#[command]
pub async fn mark_all_notifications_as_read() -> Result<usize, String> {
    let notifier = NOTIFIER.get().await
        .map_err(|e| format!("获取通知管理器失败: {}", e))?;

    Ok(notifier.mark_all_as_read().await)
}

#[command]
pub async fn clear_notification_history() -> Result<usize, String> {
    let notifier = NOTIFIER.get().await
        .map_err(|e| format!("获取通知管理器失败: {}", e))?;

    Ok(notifier.clear_history().await)
}

#[command]
pub async fn get_unread_notification_count() -> Result<usize, String> {
    let notifier = NOTIFIER.get().await
        .map_err(|e| format!("获取通知管理器失败: {}", e))?;

    Ok(notifier.get_unread_count().await)
}

#[command]
pub async fn get_notification_stats() -> Result<NotificationStatsResponse, String> {
    let notifier = NOTIFIER.get().await
        .map_err(|e| format!("获取通知管理器失败: {}", e))?;

    // 获取历史记录
    let history = notifier.get_notification_history(None, false).await;

    // 计算统计信息
    let total_notifications = history.len();
    let unread_notifications = history.iter().filter(|item| !item.read).count();

    // 按类型统计
    let mut type_stats = std::collections::HashMap::new();
    for item in &history {
        let type_str = format!("{:?}", item.notification_type);
        let entry = type_stats.entry(type_str.clone()).or_insert((0, 0));
        entry.0 += 1;
        if !item.read {
            entry.1 += 1;
        }
    }

    let notification_types = type_stats.into_iter()
        .map(|(notification_type, (count, unread_count))| NotificationTypeStats {
            notification_type,
            count,
            unread_count,
        })
        .collect();

    // 获取最近通知
    let recent_notifications = history.into_iter()
        .take(5)
        .map(|item| NotificationHistoryResponse {
            id: item.id,
            title: item.title,
            body: item.body,
            notification_type: format!("{:?}", item.notification_type),
            priority: format!("{:?}", item.priority),
            timestamp: item.timestamp.to_rfc3339(),
            read: item.read,
            action_taken: item.action_taken,
            metadata: item.metadata,
        })
        .collect();

    Ok(NotificationStatsResponse {
        total_notifications,
        unread_notifications,
        notification_types,
        recent_notifications,
    })
}

#[command]
pub async fn test_notification_system() -> Result<(), String> {
    let notifier = NOTIFIER.get().await
        .map_err(|e| format!("获取通知管理器失败: {}", e))?;

    // 发送测试通知
    let test_request = NotificationRequest {
        title: "测试通知".to_string(),
        body: "这是一个测试通知，用于验证通知系统功能是否正常。".to_string(),
        notification_type: crate::system_integration::NotificationType::SystemAlert,
        priority: crate::system_integration::NotificationPriority::Normal,
        icon: Some("bell".to_string()),
        sound: Some("default".to_string()),
        actions: vec![
            crate::system_integration::NotificationAction {
                id: "dismiss".to_string(),
                title: "知道了".to_string(),
                icon: Some("check".to_string()),
            },
            crate::system_integration::NotificationAction {
                id: "view_details".to_string(),
                title: "查看详情".to_string(),
                icon: Some("eye".to_string()),
            },
        ],
        metadata: serde_json::json!({
            "test": true,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    };

    notifier.send_notification(test_request).await?;

    log::info!("测试通知发送成功");
    Ok(())
}