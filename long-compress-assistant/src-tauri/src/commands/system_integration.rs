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

// 托盘管理命令

#[command]
pub async fn get_tray_config() -> Result<crate::system_integration::TrayConfig, String> {
    let tray_manager = crate::system_integration::TRAY_MANAGER.get().await
        .map_err(|e| format!("获取托盘管理器失败: {}", e))?;

    Ok(tray_manager.get_config().await)
}

#[command]
pub async fn update_tray_config(config: crate::system_integration::TrayConfig) -> Result<(), String> {
    let tray_manager = crate::system_integration::TRAY_MANAGER.get().await
        .map_err(|e| format!("获取托盘管理器失败: {}", e))?;

    tray_manager.update_config(config).await
        .map_err(|e| format!("更新托盘配置失败: {}", e))
}

#[command]
pub async fn show_tray_notification(title: String, message: String) -> Result<(), String> {
    let tray_manager = crate::system_integration::TRAY_MANAGER.get().await
        .map_err(|e| format!("获取托盘管理器失败: {}", e))?;

    tray_manager.show_tray_notification(&title, &message).await
        .map_err(|e| format!("显示托盘通知失败: {}", e))
}

#[command]
pub async fn update_tray_task_count(count: usize) -> Result<(), String> {
    let tray_manager = crate::system_integration::TRAY_MANAGER.get().await
        .map_err(|e| format!("获取托盘管理器失败: {}", e))?;

    tray_manager.update_task_count(count).await
        .map_err(|e| format!("更新托盘任务计数失败: {}", e))
}

// 文件关联命令

#[command]
pub async fn get_file_association_config() -> Result<crate::system_integration::FileAssociationConfig, String> {
    let file_assoc_manager = crate::system_integration::FILE_ASSOCIATION_MANAGER.get().await
        .map_err(|e| format!("获取文件关联管理器失败: {}", e))?;

    Ok(file_assoc_manager.get_config().clone())
}

#[command]
pub async fn register_file_associations() -> Result<(), String> {
    let file_assoc_manager = crate::system_integration::FILE_ASSOCIATION_MANAGER.get().await
        .map_err(|e| format!("获取文件关联管理器失败: {}", e))?;

    file_assoc_manager.register_associations()
        .map_err(|e| format!("注册文件关联失败: {}", e))
}

#[command]
pub async fn unregister_file_associations() -> Result<(), String> {
    let file_assoc_manager = crate::system_integration::FILE_ASSOCIATION_MANAGER.get().await
        .map_err(|e| format!("获取文件关联管理器失败: {}", e))?;

    file_assoc_manager.unregister_associations()
        .map_err(|e| format!("取消注册文件关联失败: {}", e))
}

#[command]
pub async fn check_file_association_status(extension: String) -> Result<bool, String> {
    let file_assoc_manager = crate::system_integration::FILE_ASSOCIATION_MANAGER.get().await
        .map_err(|e| format!("获取文件关联管理器失败: {}", e))?;

    file_assoc_manager.check_association_status(&extension)
        .map_err(|e| format!("检查文件关联状态失败: {}", e))
}

// 全局快捷键命令

#[command]
pub async fn get_shortcut_config() -> Result<crate::system_integration::ShortcutConfig, String> {
    let shortcut_manager = crate::system_integration::GLOBAL_SHORTCUT_MANAGER.get().await
        .map_err(|e| format!("获取快捷键管理器失败: {}", e))?;

    Ok(shortcut_manager.get_config().clone())
}

#[command]
pub async fn update_shortcut_config(config: crate::system_integration::ShortcutConfig) -> Result<(), String> {
    let shortcut_manager = crate::system_integration::GLOBAL_SHORTCUT_MANAGER.get().await
        .map_err(|e| format!("获取快捷键管理器失败: {}", e))?;

    shortcut_manager.update_config(config)
        .map_err(|e| format!("更新快捷键配置失败: {}", e))
}

#[command]
pub async fn update_shortcut(shortcut_id: String, new_accelerator: String) -> Result<(), String> {
    let shortcut_manager = crate::system_integration::GLOBAL_SHORTCUT_MANAGER.get().await
        .map_err(|e| format!("获取快捷键管理器失败: {}", e))?;

    shortcut_manager.update_shortcut(&shortcut_id, &new_accelerator)
        .map_err(|e| format!("更新快捷键失败: {}", e))
}

#[command]
pub async fn set_shortcut_enabled(shortcut_id: String, enabled: bool) -> Result<(), String> {
    let shortcut_manager = crate::system_integration::GLOBAL_SHORTCUT_MANAGER.get().await
        .map_err(|e| format!("获取快捷键管理器失败: {}", e))?;

    shortcut_manager.set_shortcut_enabled(&shortcut_id, enabled)
        .map_err(|e| format!("设置快捷键启用状态失败: {}", e))
}

#[command]
pub async fn check_shortcut_conflicts() -> Result<Vec<crate::system_integration::ShortcutConflict>, String> {
    let shortcut_manager = crate::system_integration::GLOBAL_SHORTCUT_MANAGER.get().await
        .map_err(|e| format!("获取快捷键管理器失败: {}", e))?;

    Ok(shortcut_manager.check_conflicts())
}

#[command]
pub async fn validate_shortcut_accelerator(accelerator: String) -> Result<(), String> {
    let shortcut_manager = crate::system_integration::GLOBAL_SHORTCUT_MANAGER.get().await
        .map_err(|e| format!("获取快捷键管理器失败: {}", e))?;

    shortcut_manager.validate_accelerator(&accelerator)
        .map_err(|e| format!("验证快捷键格式失败: {}", e))
}

#[command]
pub async fn export_shortcut_config() -> Result<String, String> {
    let shortcut_manager = crate::system_integration::GLOBAL_SHORTCUT_MANAGER.get().await
        .map_err(|e| format!("获取快捷键管理器失败: {}", e))?;

    shortcut_manager.export_config()
        .map_err(|e| format!("导出快捷键配置失败: {}", e))
}

#[command]
pub async fn import_shortcut_config(config_json: String) -> Result<(), String> {
    let shortcut_manager = crate::system_integration::GLOBAL_SHORTCUT_MANAGER.get().await
        .map_err(|e| format!("获取快捷键管理器失败: {}", e))?;

    shortcut_manager.import_config(&config_json)
        .map_err(|e| format!("导入快捷键配置失败: {}", e))
}

// 系统集成测试命令

#[command]
pub async fn test_system_integration() -> Result<String, String> {
    log::info!("开始系统集成测试");

    // 测试通知系统
    match test_notification_system().await {
        Ok(_) => log::info!("通知系统测试通过"),
        Err(e) => log::warn!("通知系统测试失败: {}", e),
    }

    // 测试托盘系统
    let tray_manager = crate::system_integration::TRAY_MANAGER.get().await;
    if let Ok(tray_manager) = tray_manager {
        match tray_manager.show_tray_notification("系统集成测试", "托盘通知测试").await {
            Ok(_) => log::info!("托盘系统测试通过"),
            Err(e) => log::warn!("托盘系统测试失败: {}", e),
        }
    } else {
        log::warn!("托盘管理器未初始化，跳过测试");
    }

    // 测试快捷键系统
    let shortcut_manager = crate::system_integration::GLOBAL_SHORTCUT_MANAGER.get().await;
    if let Ok(shortcut_manager) = shortcut_manager {
        let count = shortcut_manager.get_registered_count();
        log::info!("快捷键系统测试通过，已注册 {} 个快捷键", count);
    } else {
        log::warn!("快捷键管理器未初始化，跳过测试");
    }

    // 测试文件关联系统
    let file_assoc_manager = crate::system_integration::FILE_ASSOCIATION_MANAGER.get().await;
    if let Ok(file_assoc_manager) = file_assoc_manager {
        match file_assoc_manager.check_association_status("zip") {
            Ok(status) => log::info!("文件关联系统测试通过，ZIP关联状态: {}", status),
            Err(e) => log::warn!("文件关联系统测试失败: {}", e),
        }
    } else {
        log::warn!("文件关联管理器未初始化，跳过测试");
    }

    // 测试权限管理系统
    let permission_manager = crate::system_integration::PERMISSION_MANAGER.get().await;
    if let Ok(permission_manager) = permission_manager {
        match permission_manager.check_permission(crate::system_integration::PermissionType::FileAssociation).await {
            Ok(result) => log::info!("权限管理系统测试通过，文件关联权限状态: {:?}", result.status),
            Err(e) => log::warn!("权限管理系统测试失败: {}", e),
        }
    } else {
        log::warn!("权限管理器未初始化，跳过测试");
    }

    // 测试平台兼容性检查
    log::info!("测试平台兼容性检查系统");
    let platform = crate::system_integration::PlatformCompatibilityChecker::get_current_platform();
    log::info!("当前平台: {:?}", platform);

    let features = crate::system_integration::PlatformCompatibilityChecker::check_all_features();
    log::info!("检查了 {} 个系统集成功能", features.len());

    for feature in &features {
        log::info!("功能 {}: {:?}", feature.feature_name, feature.support_status);
    }

    // 测试系统集成配置管理器
    log::info!("测试系统集成配置管理器");
    let config_manager = &crate::system_integration::SYSTEM_INTEGRATION_CONFIG_MANAGER;

    // 获取配置
    let config = config_manager.get_config().await;
    log::info!("获取系统集成配置成功");

    // 获取状态
    let status = config_manager.get_status().await;
    log::info!("获取系统集成状态成功，总体状态: {:?}", status.overall_status);

    // 验证配置
    match config_manager.validate_config().await {
        Ok(warnings) => {
            if warnings.is_empty() {
                log::info!("系统集成配置验证通过，无警告");
            } else {
                log::warn!("系统集成配置验证通过，但有 {} 个警告", warnings.len());
                for warning in warnings {
                    log::warn!("警告: {}", warning);
                }
            }
        }
        Err(e) => log::warn!("系统集成配置验证失败: {}", e),
    }

    log::info!("系统集成测试完成");
    Ok("系统集成测试完成".to_string())
}

// 权限管理命令

#[command]
pub async fn check_system_permission(permission_type: String) -> Result<crate::system_integration::PermissionCheckResult, String> {
    let permission_manager = crate::system_integration::PERMISSION_MANAGER.get().await
        .map_err(|e| format!("获取权限管理器失败: {}", e))?;

    let permission_type_enum = match permission_type.to_lowercase().as_str() {
        "file_association" => crate::system_integration::PermissionType::FileAssociation,
        "global_shortcut" => crate::system_integration::PermissionType::GlobalShortcut,
        "system_tray" => crate::system_integration::PermissionType::SystemTray,
        "notification" => crate::system_integration::PermissionType::Notification,
        "system_service" => crate::system_integration::PermissionType::SystemService,
        _ => return Err(format!("未知的权限类型: {}", permission_type)),
    };

    permission_manager.check_permission(permission_type_enum).await
        .map_err(|e| format!("检查权限失败: {}", e))
}

#[command]
pub async fn request_permission(permission_type: String) -> Result<crate::system_integration::PermissionCheckResult, String> {
    let permission_manager = crate::system_integration::PERMISSION_MANAGER.get().await
        .map_err(|e| format!("获取权限管理器失败: {}", e))?;

    let permission_type_enum = match permission_type.to_lowercase().as_str() {
        "file_association" => crate::system_integration::PermissionType::FileAssociation,
        "global_shortcut" => crate::system_integration::PermissionType::GlobalShortcut,
        "system_tray" => crate::system_integration::PermissionType::SystemTray,
        "notification" => crate::system_integration::PermissionType::Notification,
        "system_service" => crate::system_integration::PermissionType::SystemService,
        _ => return Err(format!("未知的权限类型: {}", permission_type)),
    };

    permission_manager.request_permission(permission_type_enum).await
        .map_err(|e| format!("请求权限失败: {}", e))
}

#[command]
pub async fn get_permission_config() -> Result<crate::system_integration::PermissionConfig, String> {
    let permission_manager = crate::system_integration::PERMISSION_MANAGER.get().await
        .map_err(|e| format!("获取权限管理器失败: {}", e))?;

    Ok(permission_manager.get_config().await)
}

#[command]
pub async fn update_permission_config(config: crate::system_integration::PermissionConfig) -> Result<(), String> {
    let permission_manager = crate::system_integration::PERMISSION_MANAGER.get().await
        .map_err(|e| format!("获取权限管理器失败: {}", e))?;

    permission_manager.update_config(config).await;
    Ok(())
}

#[command]
pub async fn get_all_permission_status() -> Result<Vec<crate::system_integration::PermissionCheckResult>, String> {
    let permission_manager = crate::system_integration::PERMISSION_MANAGER.get().await
        .map_err(|e| format!("获取权限管理器失败: {}", e))?;

    Ok(permission_manager.get_all_permission_status().await)
}

#[command]
pub async fn check_admin_permission() -> Result<bool, String> {
    let permission_manager = crate::system_integration::PERMISSION_MANAGER.get().await
        .map_err(|e| format!("获取权限管理器失败: {}", e))?;

    Ok(permission_manager.is_admin().await)
}

#[command]
pub async fn clear_permission_cache() -> Result<(), String> {
    let permission_manager = crate::system_integration::PERMISSION_MANAGER.get().await
        .map_err(|e| format!("获取权限管理器失败: {}", e))?;

    permission_manager.clear_cache().await;
    Ok(())
}

// 平台兼容性检查命令

#[command]
pub async fn get_current_platform() -> Result<String, String> {
    let platform = crate::system_integration::PlatformCompatibilityChecker::get_current_platform();
    Ok(format!("{:?}", platform))
}

#[command]
pub async fn check_feature_support(feature_name: String) -> Result<crate::system_integration::PlatformFeatureCheck, String> {
    let feature = match feature_name.to_lowercase().as_str() {
        "file_association" => crate::system_integration::SystemIntegrationFeature::FileAssociation,
        "global_shortcut" => crate::system_integration::SystemIntegrationFeature::GlobalShortcut,
        "system_tray" => crate::system_integration::SystemIntegrationFeature::SystemTray,
        "context_menu" => crate::system_integration::SystemIntegrationFeature::ContextMenu,
        "system_notification" => crate::system_integration::SystemIntegrationFeature::SystemNotification,
        "auto_start" => crate::system_integration::SystemIntegrationFeature::AutoStart,
        "file_drag_drop" => crate::system_integration::SystemIntegrationFeature::FileDragDrop,
        "clipboard_integration" => crate::system_integration::SystemIntegrationFeature::ClipboardIntegration,
        _ => return Err(format!("未知的系统集成功能: {}", feature_name)),
    };

    Ok(crate::system_integration::PlatformCompatibilityChecker::check_feature_support(feature))
}

#[command]
pub async fn check_all_features_support() -> Result<Vec<crate::system_integration::PlatformFeatureCheck>, String> {
    Ok(crate::system_integration::PlatformCompatibilityChecker::check_all_features())
}

#[command]
pub async fn get_platform_compatibility_report() -> Result<String, String> {
    let platform = crate::system_integration::PlatformCompatibilityChecker::get_current_platform();
    let features = crate::system_integration::PlatformCompatibilityChecker::check_all_features();

    let mut report = format!("平台兼容性报告\n");
    report.push_str(&format!("当前平台: {:?}\n", platform));
    report.push_str(&format!("生成时间: {}\n", chrono::Utc::now().to_rfc3339()));
    report.push_str("\n");

    for feature in features {
        report.push_str(&format!("功能: {}\n", feature.feature_name));
        report.push_str(&format!("  支持状态: {:?}\n", feature.support_status));
        report.push_str(&format!("  描述: {}\n", feature.description));

        if !feature.notes.is_empty() {
            report.push_str(&format!("  注意事项:\n"));
            for note in &feature.notes {
                report.push_str(&format!("    - {}\n", note));
            }
        }

        if !feature.workarounds.is_empty() {
            report.push_str(&format!("  解决方案:\n"));
            for workaround in &feature.workarounds {
                report.push_str(&format!("    - {}\n", workaround));
            }
        }

        report.push_str("\n");
    }

    Ok(report)
}

// 系统集成配置管理命令

#[command]
pub async fn get_system_integration_config() -> Result<crate::system_integration::SystemIntegrationConfig, String> {
    let config_manager = &crate::system_integration::SYSTEM_INTEGRATION_CONFIG_MANAGER;
    Ok(config_manager.get_config().await)
}

#[command]
pub async fn update_system_integration_config(config: crate::system_integration::SystemIntegrationConfig) -> Result<(), String> {
    let config_manager = &crate::system_integration::SYSTEM_INTEGRATION_CONFIG_MANAGER;
    config_manager.update_config(config).await;
    Ok(())
}

#[command]
pub async fn get_system_integration_status() -> Result<crate::system_integration::SystemIntegrationStatus, String> {
    let config_manager = &crate::system_integration::SYSTEM_INTEGRATION_CONFIG_MANAGER;
    Ok(config_manager.get_status().await)
}

#[command]
pub async fn update_integration_status(
    integration_type: String,
    status: String,
) -> Result<(), String> {
    let config_manager = &crate::system_integration::SYSTEM_INTEGRATION_CONFIG_MANAGER;

    let integration_type_enum = match integration_type.to_lowercase().as_str() {
        "file_association" => crate::system_integration::IntegrationType::FileAssociation,
        "global_shortcut" => crate::system_integration::IntegrationType::GlobalShortcut,
        "system_tray" => crate::system_integration::IntegrationType::SystemTray,
        "notification" => crate::system_integration::IntegrationType::Notification,
        "permission" => crate::system_integration::IntegrationType::Permission,
        "platform_compatibility" => crate::system_integration::IntegrationType::PlatformCompatibility,
        _ => return Err(format!("未知的集成类型: {}", integration_type)),
    };

    let status_enum = match status.to_lowercase().as_str() {
        "not_initialized" => crate::system_integration::IntegrationStatus::NotInitialized,
        "initializing" => crate::system_integration::IntegrationStatus::Initializing,
        "initialized" => crate::system_integration::IntegrationStatus::Initialized,
        "running" => crate::system_integration::IntegrationStatus::Running,
        "stopped" => crate::system_integration::IntegrationStatus::Stopped,
        "not_supported" => crate::system_integration::IntegrationStatus::NotSupported,
        _ => {
            if status.starts_with("error:") {
                let error_msg = status.trim_start_matches("error:").trim().to_string();
                crate::system_integration::IntegrationStatus::Error(error_msg)
            } else {
                return Err(format!("未知的状态: {}", status));
            }
        }
    };

    config_manager.update_integration_status(integration_type_enum, status_enum).await;
    Ok(())
}

#[command]
pub async fn get_adapted_config() -> Result<crate::system_integration::SystemIntegrationConfig, String> {
    let config_manager = &crate::system_integration::SYSTEM_INTEGRATION_CONFIG_MANAGER;
    Ok(config_manager.get_adapted_config().await)
}

#[command]
pub async fn export_system_integration_config() -> Result<String, String> {
    let config_manager = &crate::system_integration::SYSTEM_INTEGRATION_CONFIG_MANAGER;
    config_manager.export_config().await
        .map_err(|e| format!("导出配置失败: {}", e))
}

#[command]
pub async fn import_system_integration_config(config_json: String) -> Result<(), String> {
    let config_manager = &crate::system_integration::SYSTEM_INTEGRATION_CONFIG_MANAGER;
    config_manager.import_config(&config_json).await
        .map_err(|e| format!("导入配置失败: {}", e))
}

#[command]
pub async fn reset_system_integration_config() -> Result<(), String> {
    let config_manager = &crate::system_integration::SYSTEM_INTEGRATION_CONFIG_MANAGER;
    config_manager.reset_to_default().await;
    Ok(())
}

#[command]
pub async fn validate_system_integration_config() -> Result<Vec<String>, String> {
    let config_manager = &crate::system_integration::SYSTEM_INTEGRATION_CONFIG_MANAGER;
    config_manager.validate_config().await
        .map_err(|e| format!("验证配置失败: {}", e))
}

#[command]
pub async fn get_platform_specific_config() -> Result<crate::system_integration::PlatformSpecificConfig, String> {
    let config_manager = &crate::system_integration::SYSTEM_INTEGRATION_CONFIG_MANAGER;
    config_manager.get_platform_specific_config().await
        .map_err(|e| format!("获取平台特定配置失败: {}", e))
}