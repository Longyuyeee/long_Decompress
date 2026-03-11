#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
    list_encrypted_passwords,
    add_encrypted_password,
    delete_encrypted_password,
    update_encrypted_password,
    search_encrypted_passwords,
    is_encrypted_password_service_unlocked,
    unlock_encrypted_password_service,
    lock_encrypted_password_service,
    list_password_groups
};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
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
            list_encrypted_passwords,
            add_encrypted_password,
            delete_encrypted_password,
            update_encrypted_password,
            search_encrypted_passwords,
            is_encrypted_password_service_unlocked,
            unlock_encrypted_password_service,
            lock_encrypted_password_service,
            list_password_groups
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
