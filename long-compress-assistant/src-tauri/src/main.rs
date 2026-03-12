#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(unused_assignments)]
#![allow(unreachable_patterns)]
#![allow(unexpected_cfgs)]

mod commands;
mod services;
mod models;
mod database;
mod crypto;
mod utils;
mod config;
mod task_queue;
mod system_integration;

use commands::compression::{extract_file, extract_multiple, compress_files, cancel_compression};
use commands::file::{list_files, get_file_info};
use commands::password::{
    add_password, delete_password, update_password,
    get_all_passwords, search_passwords
};
use commands::encrypted_password::{
    init_encrypted_password_service,
    list_encrypted_passwords,
    add_encrypted_password,
    delete_encrypted_password,
    update_encrypted_password,
    search_encrypted_passwords,
    is_encrypted_password_service_unlocked,
    unlock_encrypted_password_service,
    lock_encrypted_password_service,
    clear_encrypted_passwords,
    list_password_groups,
    export_passwords_command,
    import_passwords_command,
    EncryptedPasswordServiceState
};

fn main() {
    // 关键修复：将数据目录移出 src-tauri 监控范围
    // 在开发环境下使用项目根目录下的隐藏文件夹，在发布环境下使用 AppData
    let data_dir = if cfg!(debug_assertions) {
        // 获取当前工作目录（通常是项目根目录或 src-tauri）
        let mut path = std::env::current_dir().unwrap();
        // 确保不在 src-tauri 内部
        if path.ends_with("src-tauri") {
            path.pop();
        }
        path.join(".password_book_data")
    } else {
        std::path::PathBuf::from("data") // 发布版逻辑保持原样
    };

    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir).unwrap();
    }

    tauri::Builder::default()
        .manage(EncryptedPasswordServiceState::new(data_dir))
        .setup(|_app| {
            // 初始化数据库
            tauri::async_runtime::block_on(async {
                let db_path = std::path::PathBuf::from("data.db");
                match database::connection::DatabaseConnection::new(&db_path, None).await {
                    Ok(conn) => {
                        if let Err(e) = database::connection::set_global_connection(conn).await {
                            eprintln!("Failed to set global database connection: {}", e);
                        }
                    }
                    Err(e) => eprintln!("Failed to initialize database: {}", e),
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Compression Commands
            extract_file,
            extract_multiple,
            compress_files,
            cancel_compression,
            
            // File Commands
            list_files,
            get_file_info,
            
            // Password Commands
            add_password,
            delete_password,
            update_password,
            get_all_passwords,
            search_passwords,

            // Encrypted Password Commands
            init_encrypted_password_service,
            list_encrypted_passwords,
            add_encrypted_password,
            delete_encrypted_password,
            update_encrypted_password,
            search_encrypted_passwords,
            is_encrypted_password_service_unlocked,
            unlock_encrypted_password_service,
            lock_encrypted_password_service,
            clear_encrypted_passwords,
            list_password_groups,
            export_passwords_command,
            import_passwords_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
