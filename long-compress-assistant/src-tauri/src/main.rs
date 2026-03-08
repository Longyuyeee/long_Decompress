#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod services;
mod models;
mod database;
mod crypto;
mod utils;
mod task_queue;
mod config;
mod system_integration;

use tauri::Manager;
use std::path::PathBuf;
use commands::encrypted_password::EncryptedPasswordServiceState;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // 初始化日志
            env_logger::init();
            log::info!("胧压缩·方便助手启动");

            // 初始化数据库
            let app_handle = app.handle();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = database::connection::init().await {
                    log::error!("数据库初始化失败: {}", e);
                }
            });

            // 初始化任务管理器
            let app_handle_for_tasks = app.handle();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = task_queue::TASK_MANAGER.initialize(app_handle_for_tasks).await {
                    log::error!("任务管理器初始化失败: {}", e);
                } else {
                    log::info!("任务管理器初始化成功");
                }
            });

            // 初始化通知管理器
            let app_handle_for_notifications = app.handle();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = system_integration::NOTIFIER.initialize(app_handle_for_notifications).await {
                    log::error!("通知管理器初始化失败: {}", e);
                } else {
                    log::info!("通知管理器初始化成功");
                }
            });

            Ok(())
        })
        .manage(EncryptedPasswordServiceState::new(
            app.path_resolver().app_data_dir().unwrap_or_else(|| PathBuf::from("./data"))
        ))
        .invoke_handler(tauri::generate_handler![
            // 压缩命令
            commands::compression::extract_file,
            commands::compression::extract_multiple,
            commands::compression::compress_file,

            // 密码命令（旧版，未加密）
            commands::password::add_password,
            commands::password::find_password,
            commands::password::search_passwords,

            // 加密密码命令
            commands::encrypted_password::init_encrypted_password_service,
            commands::encrypted_password::unlock_encrypted_password_service,
            commands::encrypted_password::lock_encrypted_password_service,
            commands::encrypted_password::is_encrypted_password_service_unlocked,
            commands::encrypted_password::add_encrypted_password,
            commands::encrypted_password::get_encrypted_password,
            commands::encrypted_password::update_encrypted_password,
            commands::encrypted_password::delete_encrypted_password,
            commands::encrypted_password::search_encrypted_passwords,
            commands::encrypted_password::list_encrypted_passwords,
            commands::encrypted_password::generate_strong_password,
            commands::encrypted_password::audit_encrypted_passwords,
            commands::encrypted_password::export_encrypted_passwords,
            commands::encrypted_password::import_encrypted_passwords,
            commands::encrypted_password::create_password_group,
            commands::encrypted_password::get_password_group,
            commands::encrypted_password::update_password_group,
            commands::encrypted_password::delete_password_group,
            commands::encrypted_password::list_password_groups,
            commands::encrypted_password::add_entry_to_password_group,
            commands::encrypted_password::remove_entry_from_password_group,
            commands::encrypted_password::get_group_entries,

            // 文件命令
            commands::file::list_files,
            commands::file::get_file_info,

            // 系统命令
            commands::system::get_system_info,

            // 配置命令
            config::commands::get_all_configs,
            config::commands::get_configs_by_category,
            config::commands::get_config,
            config::commands::get_config_value,
            config::commands::get_string_config,
            config::commands::get_integer_config,
            config::commands::get_float_config,
            config::commands::get_boolean_config,
            config::commands::set_config,
            config::commands::batch_set_configs,
            config::commands::reset_config,
            config::commands::batch_reset_configs,
            config::commands::validate_config,
            config::commands::search_configs,
            config::commands::export_configs,
            config::commands::import_configs,
            config::commands::get_config_statistics,
            config::commands::refresh_config_cache,
            config::commands::clear_config_cache,
            config::commands::get_config_categories,

            // 任务队列命令
            commands::task_queue::add_compression_task,
            commands::task_queue::add_extraction_task,
            commands::task_queue::get_task,
            commands::task_queue::list_tasks,
            commands::task_queue::get_task_status,
            commands::task_queue::get_task_progress,
            commands::task_queue::cancel_task,
            commands::task_queue::pause_task,
            commands::task_queue::resume_task,
            commands::task_queue::get_queue_statistics,
            commands::task_queue::get_scheduler_status,
            commands::task_queue::get_executor_status,
            commands::task_queue::cleanup_completed_tasks,
            commands::task_queue::stop_all_tasks,
            commands::task_queue::get_all_tasks,

            // 数据库命令
            database::commands::get_database_status,
            database::commands::backup_database,
            database::commands::restore_database,
            database::commands::optimize_database,
            database::commands::get_database_health_report,
            database::commands::perform_database_maintenance,
            database::commands::export_database,
            database::commands::get_database_config,
            database::commands::check_database_connection,
            database::commands::reinitialize_database,

            // 系统集成命令
            commands::system_integration::send_notification,
            commands::system_integration::send_task_completed_notification,
            commands::system_integration::send_task_failed_notification,
            commands::system_integration::send_task_progress_notification,
            commands::system_integration::send_system_alert_notification,
            commands::system_integration::get_notification_config,
            commands::system_integration::update_notification_config,
            commands::system_integration::get_notification_history,
            commands::system_integration::mark_notification_as_read,
            commands::system_integration::mark_all_notifications_as_read,
            commands::system_integration::clear_notification_history,
            commands::system_integration::get_unread_notification_count,
            commands::system_integration::get_notification_stats,
            commands::system_integration::test_notification_system,
        ])
        .run(tauri::generate_context!())
        .expect("运行Tauri应用时出错");
}