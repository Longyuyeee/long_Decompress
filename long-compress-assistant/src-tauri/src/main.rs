#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unexpected_cfgs)]

use long_compress_assistant::database;
use long_compress_assistant::utils::app_paths::app_data_dir;

use long_compress_assistant::commands::compression_profile::CompressionProfileServiceState;
use long_compress_assistant::commands::encrypted_password::EncryptedPasswordServiceState;
use long_compress_assistant::services::decompression_profile_service::DecompressionProfileService;

use interprocess::local_socket::{
    prelude::*, GenericFilePath, GenericNamespaced, ListenerOptions, Stream,
};
use std::io::{BufRead, BufReader, Read, Write};
use std::sync::atomic::Ordering;
use tauri::Manager;
use window_shadows::set_shadow;

use long_compress_assistant::commands::system_integration::{ContextAction, DesktopBehaviorState};

#[cfg(not(feature = "desktop-e2e"))]
const INSTANCE_NAME: &str = "com.longcompress.assistant.desktop";
#[cfg(feature = "desktop-e2e")]
const INSTANCE_NAME_PREFIX: &str = "com.longcompress.assistant.desktop.e2e";
#[cfg(not(feature = "desktop-e2e"))]
const INSTANCE_SOCKET_NAME: &str = "com.longcompress.assistant.desktop.sock";
#[cfg(feature = "desktop-e2e")]
const INSTANCE_SOCKET_NAME_PREFIX: &str = "com.longcompress.assistant.desktop.e2e";

#[cfg(not(feature = "desktop-e2e"))]
fn instance_name() -> String {
    INSTANCE_NAME.to_string()
}

#[cfg(feature = "desktop-e2e")]
fn desktop_e2e_instance_id() -> String {
    let id: String = std::env::var("LONG_DECOMPRESS_E2E_INSTANCE_ID")
        .unwrap_or_else(|_| "default".to_string())
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        .take(48)
        .collect();
    if id.is_empty() {
        "default".to_string()
    } else {
        id
    }
}

#[cfg(feature = "desktop-e2e")]
fn instance_name() -> String {
    format!("{INSTANCE_NAME_PREFIX}.{}", desktop_e2e_instance_id())
}

#[cfg(not(feature = "desktop-e2e"))]
fn instance_socket_name_value() -> String {
    INSTANCE_SOCKET_NAME.to_string()
}

#[cfg(feature = "desktop-e2e")]
fn instance_socket_name_value() -> String {
    format!(
        "{INSTANCE_SOCKET_NAME_PREFIX}.{}.sock",
        desktop_e2e_instance_id()
    )
}

fn parse_context_action(args: &[String]) -> Option<ContextAction> {
    let actions = [
        ("--extract-here", "context-extract-here"),
        ("--extract-to", "context-extract-to"),
        ("--quick-extract", "context-quick-extract"),
        ("--quick-pack", "context-quick-pack"),
        ("--test-archive", "context-test-archive"),
        ("--compress-zip", "context-compress-zip"),
        ("--compress-7z", "context-compress-7z"),
        ("--compress-custom", "context-compress-custom"),
        ("--open", "context-open"),
        ("--context-menu", "context-open"),
    ];

    for (flag, event) in actions {
        if let Some(pos) = args.iter().position(|arg| arg == flag) {
            let files = args
                .iter()
                .skip(pos + 1)
                .take_while(|arg| !arg.starts_with("--"))
                .filter(|arg| !arg.starts_with('%'))
                .cloned()
                .collect();
            return Some(ContextAction {
                action: event.to_string(),
                files,
            });
        }
    }
    None
}

fn instance_socket_name() -> std::io::Result<interprocess::local_socket::Name<'static>> {
    let socket_name = instance_socket_name_value();
    if GenericNamespaced::is_supported() {
        socket_name.to_ns_name::<GenericNamespaced>()
    } else {
        #[cfg(not(feature = "desktop-e2e"))]
        {
            "/tmp/long-compress-assistant.sock".to_fs_name::<GenericFilePath>()
        }
        #[cfg(feature = "desktop-e2e")]
        {
            format!("/tmp/{socket_name}").to_fs_name::<GenericFilePath>()
        }
    }
}

fn forward_to_running_instance(args: &[String]) -> bool {
    for _ in 0..20 {
        let stream = instance_socket_name().and_then(Stream::connect);
        if let Ok(mut stream) = stream {
            if let Ok(payload) = serde_json::to_vec(args) {
                if stream.write_all(&payload).is_ok()
                    && stream.write_all(b"\n").is_ok()
                    && stream.flush().is_ok()
                {
                    let mut acknowledgement = String::new();
                    if BufReader::new(stream)
                        .read_line(&mut acknowledgement)
                        .is_ok()
                    {
                        return acknowledgement.trim() == "ok";
                    }
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    false
}

#[cfg(feature = "desktop-e2e")]
fn write_restore_visibility_probe(args: &[String], window: &tauri::Window) {
    let Some(position) = args.iter().position(|arg| arg == "--desktop-e2e-restore") else {
        return;
    };
    let Some(marker_path) = args.get(position + 1) else {
        return;
    };
    let visibility = if window.is_visible().unwrap_or(false) {
        "visible"
    } else {
        "hidden"
    };
    let _ = std::fs::write(marker_path, visibility);
}

fn main() {
    let instance_name = instance_name();
    let instance = single_instance::SingleInstance::new(&instance_name)
        .expect("failed to create application instance guard");
    let args: Vec<String> = std::env::args().collect();
    if !instance.is_single() {
        if !forward_to_running_instance(&args) {
            rfd::MessageDialog::new()
                .set_title("Long解压")
                .set_description(
                    "软件已经在运行，但无法将本次操作发送到现有窗口。请从托盘打开软件后重试。",
                )
                .set_level(rfd::MessageLevel::Error)
                .show();
        }
        return;
    }
    let mut ipc_listener = instance_socket_name()
        .and_then(|name| {
            ListenerOptions::new()
                .name(name)
                .try_overwrite(true)
                .create_sync()
        })
        .map_err(|error| {
            rfd::MessageDialog::new()
                .set_title("Long解压")
                .set_description(format!(
                    "单实例通信初始化失败：{}\n右键操作可能无法唤醒当前窗口。",
                    error
                ))
                .set_level(rfd::MessageLevel::Warning)
                .show();
            error
        })
        .ok();

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
        .manage(DesktopBehaviorState::default())
        .system_tray(long_compress_assistant::system_integration::setup_tray())
        .on_system_tray_event(long_compress_assistant::system_integration::handle_tray_event)
        .on_window_event(|event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event.event() {
                let state = event.window().state::<DesktopBehaviorState>();
                if state.close_to_tray.load(Ordering::SeqCst) {
                    api.prevent_close();
                    let _ = event.window().hide();
                } else if long_compress_assistant::commands::system_integration::should_confirm_exit(&state) {
                    api.prevent_close();
                    let _ = event.window().show();
                    let _ = event.window().unminimize();
                    let _ = event.window().set_focus();
                    let _ = event.window().emit("exit-confirmation-requested", ());
                } else {
                    event.window().app_handle().exit(0);
                }
            }
        })
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

            if let Ok(exe_path) = std::env::current_exe() {
                if let Err(error) = long_compress_assistant::system_integration::context_menu::refresh_context_menu_if_present(
                    &exe_path.to_string_lossy(),
                ) {
                    eprintln!("Failed to refresh Explorer context menu: {}", error);
                }
            }

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

            if let Some(action) = parse_context_action(&args) {
                if let Ok(mut pending) = app
                    .state::<DesktopBehaviorState>()
                    .pending_context_actions
                    .lock()
                {
                    pending.push(action);
                }
            }

            if let Some(listener) = ipc_listener.take() {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    for incoming in listener.incoming() {
                        let Ok(stream) = incoming else { continue };
                        let connection_handle = handle.clone();
                        std::thread::spawn(move || {
                            let mut reader = BufReader::new(stream);
                            let mut payload = String::new();
                            if reader
                                .by_ref()
                                .take(256 * 1024)
                                .read_line(&mut payload)
                                .is_err()
                            {
                                return;
                            }
                            let Ok(args) = serde_json::from_str::<Vec<String>>(payload.trim()) else { return };
                            if let Some(action) = parse_context_action(&args) {
                                let state = connection_handle.state::<DesktopBehaviorState>();
                                if let Ok(mut pending) = state.pending_context_actions.lock() {
                                    pending.push(action);
                                }
                                let _ = connection_handle.emit_all("context-actions-available", ());
                            }
                            if let Some(window) = connection_handle.get_window("main") {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                                #[cfg(feature = "desktop-e2e")]
                                write_restore_visibility_probe(&args, &window);
                            }
                            let _ = reader.get_mut().write_all(b"ok\n");
                            let _ = reader.get_mut().flush();
                        });
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            long_compress_assistant::commands::compression::extract_file,
            long_compress_assistant::commands::compression::verify_archive_password,
            long_compress_assistant::commands::compression::extract_multiple,
            long_compress_assistant::commands::compression::compress_files,
            long_compress_assistant::commands::compression::analyze_compression_sources,
            long_compress_assistant::commands::compression::cancel_compression_analysis,
            long_compress_assistant::commands::compression::diagnose_archive,
            long_compress_assistant::commands::compression::cancel_archive_diagnosis,
            long_compress_assistant::commands::compression::cancel_zip_repair,
            long_compress_assistant::commands::compression::cancel_compression,
            long_compress_assistant::commands::compression::cancel_tasks_and_wait,
            long_compress_assistant::commands::compression::desktop_e2e_run_cancellable_task,
            long_compress_assistant::commands::compression::check_rar_compression_support,
            long_compress_assistant::commands::compression::get_archive_engine_capabilities,
            long_compress_assistant::commands::compression::install_winrar_with_winget,
            long_compress_assistant::commands::compression::open_rar_download_page,
            long_compress_assistant::commands::compression::list_archive_contents,
            long_compress_assistant::commands::compression::browse_archive,
            long_compress_assistant::commands::compression::preview_archive_image,
            long_compress_assistant::commands::compression::test_archive_integrity,
            long_compress_assistant::commands::compression::repair_zip,
            long_compress_assistant::commands::file::list_files,
            long_compress_assistant::commands::file::get_file_info,
            long_compress_assistant::commands::file::path_exists,
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
            long_compress_assistant::commands::system_integration::set_close_to_tray,
            long_compress_assistant::commands::system_integration::set_has_active_tasks,
            long_compress_assistant::commands::system_integration::desktop_e2e_get_behavior_state,
            long_compress_assistant::commands::system_integration::desktop_e2e_request_exit_confirmation,
            long_compress_assistant::commands::system_integration::desktop_e2e_hide_window,
            long_compress_assistant::commands::system_integration::exit_app,
            long_compress_assistant::commands::system_integration::take_pending_context_actions,
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
            long_compress_assistant::commands::compression_profile::export_task_template,
            long_compress_assistant::commands::compression_profile::preview_task_template,
            long_compress_assistant::commands::compression_profile::import_task_template,
            long_compress_assistant::commands::compression_profile::plan_task_template_draft,
            long_compress_assistant::commands::compression_profile::preview_task_template_watch_folder,
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
