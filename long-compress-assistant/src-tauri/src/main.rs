#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(unused_assignments)]
#![allow(unreachable_patterns)]
#![allow(unexpected_cfgs)]

use long_compress_assistant::services;
use long_compress_assistant::models;
use long_compress_assistant::database;
use long_compress_assistant::crypto;
use long_compress_assistant::utils;
use long_compress_assistant::config;
use long_compress_assistant::task_queue;
use long_compress_assistant::system_integration;

use long_compress_assistant::commands::encrypted_password::EncryptedPasswordServiceState;

use tauri::Manager;
use window_shadows::set_shadow;

fn main() {
    // 在开发环境下使用项目根目录下的隐藏文件夹，在发布环境下使用 AppData
    let data_dir = if cfg!(debug_assertions) {
        let mut path = std::env::current_dir().unwrap();
        if path.ends_with("src-tauri") {
            path.pop();
        }
        path.join(".password_book_data")
    } else {
        std::path::PathBuf::from("data")
    };

    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir).unwrap();
    }

    // 数据库路径指向 data_dir
    let db_path = data_dir.join("data.db");

    tauri::Builder::default()
        .manage(EncryptedPasswordServiceState::new(data_dir.clone()))
        .setup(move |app| {
            let window = app.get_window("main").unwrap();
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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            long_compress_assistant::commands::compression::extract_file,
            long_compress_assistant::commands::compression::extract_multiple,
            long_compress_assistant::commands::compression::compress_files,
            long_compress_assistant::commands::compression::cancel_compression,
            long_compress_assistant::commands::file::list_files,
            long_compress_assistant::commands::file::get_file_info,
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
            long_compress_assistant::commands::encrypted_password::init_encrypted_password_service,
            long_compress_assistant::commands::encrypted_password::list_encrypted_passwords,
            long_compress_assistant::commands::encrypted_password::add_encrypted_password,
            long_compress_assistant::commands::encrypted_password::delete_encrypted_password,
            long_compress_assistant::commands::encrypted_password::update_encrypted_password,
            long_compress_assistant::commands::encrypted_password::search_encrypted_passwords,
            long_compress_assistant::commands::encrypted_password::is_encrypted_password_service_unlocked,
            long_compress_assistant::commands::encrypted_password::unlock_encrypted_password_service,
            long_compress_assistant::commands::encrypted_password::lock_encrypted_password_service,
            long_compress_assistant::commands::encrypted_password::clear_encrypted_passwords,
            long_compress_assistant::commands::encrypted_password::list_password_groups,
            long_compress_assistant::commands::encrypted_password::export_passwords_command,
            long_compress_assistant::commands::encrypted_password::import_passwords_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
