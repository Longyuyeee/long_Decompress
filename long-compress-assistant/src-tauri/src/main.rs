#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unexpected_cfgs)]

use long_compress_assistant::database;

use long_compress_assistant::commands::encrypted_password::EncryptedPasswordServiceState;

use tauri::Manager;
use window_shadows::set_shadow;

fn main() {
    // 在开发环境下使用项目根目录下的隐藏文件夹，在发布环境下使用 AppData
    let data_dir = if cfg!(debug_assertions) {
        let mut path = std::env::current_dir().unwrap_or_default();
        if path.ends_with("src-tauri") {
            path.pop();
        }
        path.join(".password_book_data")
    } else {
        std::path::PathBuf::from("data")
    };

    if !data_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&data_dir) {
            eprintln!("Failed to create data directory at {:?}: {}", data_dir, e);
        }
    }

    // 数据库路径指向 data_dir
    let db_path = data_dir.join("data.db");

    tauri::Builder::default()
        .manage(EncryptedPasswordServiceState::new(data_dir.clone()))
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

            // 处理右键菜单传递的文件路径 (--context-menu)
            let args: Vec<String> = std::env::args().collect();
            if let Some(pos) = args.iter().position(|a| a == "--context-menu") {
                if let Some(file_path) = args.get(pos + 1) {
                    let file_path = file_path.clone();
                    let handle = app.handle().clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        let _ = handle.emit_all("context-menu-open", file_path);
                    });
                }
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
            long_compress_assistant::commands::file::validate_wordlists,
            long_compress_assistant::commands::password::add_password,
            long_compress_assistant::commands::password::delete_password,
            long_compress_assistant::commands::password::update_password,
            long_compress_assistant::commands::password::get_all_passwords,
            long_compress_assistant::commands::password::search_passwords,
            long_compress_assistant::commands::password::get_password_suggestions,
            long_compress_assistant::commands::system::get_system_info,
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
            long_compress_assistant::commands::encrypted_password::import_passwords_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
