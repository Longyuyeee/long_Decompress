#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unexpected_cfgs)]

use long_compress_assistant::database;
use long_compress_assistant::utils::app_paths::app_data_dir;

use long_compress_assistant::commands::encrypted_password::EncryptedPasswordServiceState;
use long_compress_assistant::commands::compression_profile::CompressionProfileServiceState;
use long_compress_assistant::services::decompression_profile_service::DecompressionProfileService;

use tauri::Manager;
use window_shadows::set_shadow;

fn main() {
    let data_dir = app_data_dir();

    if !data_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&data_dir) {
            eprintln!("Failed to create data directory at {:?}: {}", data_dir, e);
        }
    }

    // 数据库路径指向 data_dir
    let db_path = data_dir.join("data.db");

    tauri::Builder::default()
        .manage(EncryptedPasswordServiceState::new(data_dir.clone()))
        .manage(CompressionProfileServiceState::new())
        .setup(move |app| {
            let window = match app.get_window("main") {
                Some(w) => w,
                None => {
                    eprintln!("Main window not available during setup");
                    return Ok(());
                }
            };
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            let _ = set_shadow(&window, true);

            // 初始化数据库
            tauri::async_runtime::block_on(async {
                match database::connection::DatabaseConnection::new(&db_path, None).await {
                    Ok(conn) => {
                        if let Err(e) = database::connection::set_global_connection(conn).await {
                            eprintln!("Failed to set global database connection: {}", e);
                        } else {
                            // 初始化配置组服务
                            if let Ok(pool) = database::connection::get_pool().await {
                                use long_compress_assistant::services::compression_profile_service::CompressionProfileService;
                                let profile_service = CompressionProfileService::new(pool.clone());

                                // 初始化默认配置组
                                if let Err(e) = profile_service.init_default_profiles().await {
                                    eprintln!("Failed to initialize default profiles: {}", e);
                                }

                                // 设置到应用状态
                                let state: tauri::State<CompressionProfileServiceState> = app.state();
                                let mut service_lock = state.service.lock().await;
                                *service_lock = Some(profile_service);

                                // 初始化解压配置组服务
                                let decompression_service = DecompressionProfileService::new(pool.clone());

                                // 创建表并初始化默认配置组
                                if let Err(e) = decompression_service.init_table().await {
                                    eprintln!("Failed to initialize decompression profiles table: {}", e);
                                }
                                if let Err(e) = decompression_service.init_default_profiles().await {
                                    eprintln!("Failed to initialize default decompression profiles: {}", e);
                                }

                                // 注册到应用状态
                                app.manage(decompression_service);
                            }
                        }
                    }
                    Err(e) => eprintln!("Failed to initialize database at {:?}: {}", db_path, e),
                }
            });

            // 初始化任务队列管理器（后台调度和执行）
            tauri::async_runtime::block_on(async {
                if let Err(e) = long_compress_assistant::task_queue::init_task_manager(app.handle()).await {
                    eprintln!("Failed to initialize task queue manager: {}", e);
                }
            });

            // 处理右键菜单 / CLI 传入的文件和动作
            let args: Vec<String> = std::env::args().collect();
            let handle = app.handle().clone();

            // 提取动作和文件路径
            let actions = [
                ("--extract-here",   "context-extract-here"),
                ("--extract-to",     "context-extract-to"),
                ("--test-archive",   "context-test-archive"),
                ("--compress-zip",   "context-compress-zip"),
                ("--compress-7z",    "context-compress-7z"),
                ("--compress-custom","context-compress-custom"),
                ("--open",           "context-open"),
                ("--context-menu",   "context-open"), // 向后兼容旧版
            ];

            let mut launch_action: Option<String> = None;
            let mut launch_files: Vec<String> = Vec::new();

            for (flag, event) in &actions {
                if let Some(pos) = args.iter().position(|a| a == flag) {
                    launch_action = Some(event.to_string());
                    // 收集该 flag 后面的文件路径参数（直到下一个 flag 或结束）
                    let mut i = pos + 1;
                    while i < args.len() && !args[i].starts_with("--") && !args[i].starts_with("%") {
                        // 排除 %V (当前目录占位符)
                        if args[i] != "%V" {
                            launch_files.push(args[i].clone());
                        }
                        i += 1;
                    }
                    break; // 只处理第一个匹配的动作
                }
            }

            if let Some(action) = launch_action {
                let files = launch_files;
                // 右键菜单触发：聚焦窗口并发送事件
                let window_handle = window.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    // 恢复并聚焦窗口（如果最小化）
                    let _ = window_handle.unminimize();
                    let _ = window_handle.set_focus();
                    let _ = handle.emit_all(&action, files);
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            long_compress_assistant::commands::compression::extract_file,
            long_compress_assistant::commands::compression::extract_multiple,
            long_compress_assistant::commands::compression::compress_files,
            long_compress_assistant::commands::compression::cancel_compression,
            long_compress_assistant::commands::compression::check_rar_compression_support,
            long_compress_assistant::commands::compression::open_rar_download_page,
            long_compress_assistant::commands::compression::list_archive_contents,
            long_compress_assistant::commands::compression::test_archive_integrity,
            long_compress_assistant::commands::compression::repair_zip,
            long_compress_assistant::commands::file::list_files,
            long_compress_assistant::commands::file::get_file_info,
            long_compress_assistant::commands::file::read_text_file,
            long_compress_assistant::commands::file::write_text_file,
            long_compress_assistant::commands::file::validate_wordlists,
            long_compress_assistant::commands::password::add_password,
            long_compress_assistant::commands::password::delete_password,
            long_compress_assistant::commands::password::update_password,
            long_compress_assistant::commands::password::get_all_passwords,
            long_compress_assistant::commands::password::search_passwords,
            long_compress_assistant::commands::password::get_password_suggestions,
            long_compress_assistant::commands::system::get_system_info,
            long_compress_assistant::commands::system::get_resource_usage,
            long_compress_assistant::commands::system::get_disk_space,
            long_compress_assistant::commands::system::get_app_version,
            long_compress_assistant::commands::system::set_auto_start,
            long_compress_assistant::commands::system::check_auto_start,
            long_compress_assistant::commands::system::load_app_settings,
            long_compress_assistant::commands::system::save_app_settings,
            long_compress_assistant::commands::system_integration::open_in_explorer,
            long_compress_assistant::commands::system_integration::register_context_menu,
            long_compress_assistant::commands::system_integration::unregister_context_menu,
            long_compress_assistant::commands::system_integration::is_context_menu_registered,
            long_compress_assistant::commands::encrypted_password::init_encrypted_password_service,
            long_compress_assistant::commands::encrypted_password::list_encrypted_passwords,
            long_compress_assistant::commands::encrypted_password::add_encrypted_password,
            long_compress_assistant::commands::encrypted_password::delete_encrypted_password,
            long_compress_assistant::commands::encrypted_password::update_encrypted_password,
            long_compress_assistant::commands::encrypted_password::search_encrypted_passwords,
            long_compress_assistant::commands::encrypted_password::is_encrypted_password_service_unlocked,
            long_compress_assistant::commands::encrypted_password::unlock_encrypted_password_service,
            long_compress_assistant::commands::encrypted_password::get_or_create_master_key,
            long_compress_assistant::commands::encrypted_password::lock_encrypted_password_service,
            long_compress_assistant::commands::encrypted_password::clear_encrypted_passwords,
            long_compress_assistant::commands::encrypted_password::list_password_groups,
            long_compress_assistant::commands::encrypted_password::export_passwords_command,
            long_compress_assistant::commands::encrypted_password::import_passwords_command,
            long_compress_assistant::commands::compression_profile::get_compression_profiles,
            long_compress_assistant::commands::compression_profile::get_compression_profile,
            long_compress_assistant::commands::compression_profile::create_compression_profile,
            long_compress_assistant::commands::compression_profile::update_compression_profile,
            long_compress_assistant::commands::compression_profile::delete_compression_profile,
            long_compress_assistant::commands::compression_profile::reorder_compression_profiles,
            long_compress_assistant::commands::compression_profile::apply_compression_profile,
            long_compress_assistant::commands::compression_profile::suggest_compression_profile,
            long_compress_assistant::commands::decompression_profile::get_all_decompression_profiles,
            long_compress_assistant::commands::decompression_profile::get_decompression_profile_by_id,
            long_compress_assistant::commands::decompression_profile::create_decompression_profile,
            long_compress_assistant::commands::decompression_profile::update_decompression_profile,
            long_compress_assistant::commands::decompression_profile::delete_decompression_profile,
            long_compress_assistant::commands::decompression_profile::update_decompression_profile_stats,
            long_compress_assistant::commands::file_integrity::calculate_checksum,
            long_compress_assistant::commands::file_integrity::export_checksum_file,
            long_compress_assistant::commands::file_integrity::verify_checksum_file,
            long_compress_assistant::commands::password_generator::generate_password,
            long_compress_assistant::commands::password_generator::generate_memorable_password,
            long_compress_assistant::commands::password_generator::generate_pin,
            long_compress_assistant::commands::archive_helpers::detect_split_archive,
            long_compress_assistant::commands::archive_helpers::get_dictionary_passwords
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
