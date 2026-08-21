use crate::services::system_service::{SystemInfo, SystemService};
use serde::Serialize;
use sysinfo::System;
use tauri::{command, AppHandle};

#[cfg(target_os = "windows")]
use winreg::{
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
    RegKey,
};

#[cfg(target_os = "windows")]
const AUTO_START_RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
#[cfg(target_os = "windows")]
const AUTO_START_VALUE_NAME: &str = "Long解压";
#[cfg(target_os = "windows")]
const LEGACY_AUTO_START_VALUE_NAMES: [&str; 2] = ["LongDecompress", "胧解压·方便助手"];
const AUTO_START_ARGUMENT: &str = "--autostart";

fn auto_start_command(executable: &std::path::Path) -> Result<String, String> {
    if !executable.is_absolute() {
        return Err("开机启动程序路径必须是绝对路径".to_string());
    }
    let executable = executable
        .to_str()
        .ok_or_else(|| "开机启动程序路径包含无法识别的字符".to_string())?;
    if executable.contains('"') {
        return Err("开机启动程序路径包含不安全的引号".to_string());
    }
    Ok(format!("\"{executable}\" {AUTO_START_ARGUMENT}"))
}

#[cfg(target_os = "windows")]
fn expected_auto_start_command() -> Result<String, String> {
    let executable =
        std::env::current_exe().map_err(|error| format!("无法获取当前程序路径: {error}"))?;
    auto_start_command(&executable)
}

#[cfg(target_os = "windows")]
fn read_auto_start_value() -> Result<Option<String>, String> {
    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let key = match current_user.open_subkey_with_flags(AUTO_START_RUN_KEY, KEY_READ) {
        Ok(key) => key,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("无法读取 Windows 启动项: {error}")),
    };
    match key.get_value::<String, _>(AUTO_START_VALUE_NAME) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("无法读取开机启动状态: {error}")),
    }
}

/// 仅响应设置页中的显式用户操作。应用启动、设置加载和更新流程都不会调用此命令。
#[command]
pub async fn set_auto_start(enable: bool) -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        let expected = expected_auto_start_command()?;
        let current_user = RegKey::predef(HKEY_CURRENT_USER);

        if enable {
            let (key, _) = current_user
                .create_subkey(AUTO_START_RUN_KEY)
                .map_err(|error| format!("无法打开 Windows 启动项: {error}"))?;
            let already_registered = key
                .get_value::<String, _>(AUTO_START_VALUE_NAME)
                .map(|value| value.eq_ignore_ascii_case(&expected))
                .unwrap_or(false);
            if !already_registered {
                key.set_value(AUTO_START_VALUE_NAME, &expected)
                    .map_err(|error| format!("无法注册开机启动: {error}"))?;
            }
            for legacy_name in LEGACY_AUTO_START_VALUE_NAMES {
                let _ = key.delete_value(legacy_name);
            }
        } else {
            match current_user.open_subkey_with_flags(AUTO_START_RUN_KEY, KEY_WRITE) {
                Ok(key) => {
                    let _ = key.delete_value(AUTO_START_VALUE_NAME);
                    for legacy_name in LEGACY_AUTO_START_VALUE_NAMES {
                        let _ = key.delete_value(legacy_name);
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("无法打开 Windows 启动项: {error}")),
            }
        }

        check_auto_start().await
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = enable;
        Err("开机自动启动目前仅支持 Windows".to_string())
    }
}

/// 只读检查，不修复、不迁移，也不写入任何持久化位置。
#[command]
pub async fn check_auto_start() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        let expected = expected_auto_start_command()?;
        Ok(read_auto_start_value()?
            .map(|value| value.eq_ignore_ascii_case(&expected))
            .unwrap_or(false))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(false)
    }
}

#[command]
pub async fn get_system_info() -> Result<SystemInfo, String> {
    let mut service = SystemService::new();
    Ok(service.get_system_info())
}

#[derive(Debug, Serialize)]
pub struct ResourceUsage {
    pub cpu_usage: f32,
    pub memory_usage: f32,
}

#[command]
pub async fn get_resource_usage() -> Result<ResourceUsage, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut system = System::new_all();
        system.refresh_cpu();
        system.refresh_memory();
        let total_memory = system.total_memory();
        let memory_usage = if total_memory == 0 {
            0.0
        } else {
            system.used_memory() as f32 / total_memory as f32 * 100.0
        };
        ResourceUsage {
            cpu_usage: system.global_cpu_info().cpu_usage(),
            memory_usage,
        }
    })
    .await
    .map_err(|error| error.to_string())
}

#[command]
pub async fn get_disk_space(path: String) -> Result<(u64, u64), String> {
    let target = crate::services::storage_preflight::probe_storage(std::path::Path::new(&path));
    target
        .total_bytes
        .zip(target.available_bytes)
        .ok_or_else(|| "Unable to determine disk space for the selected path".to_string())
}

#[command]
pub async fn preflight_operation_resources(
    operation: String,
    output_path: String,
    source_paths: Vec<String>,
    password: Option<String>,
    estimated_output_bytes: Option<u64>,
    estimate_reliable: Option<bool>,
) -> Result<crate::services::storage_preflight::ResourcePreflightReport, String> {
    if source_paths.len() > 1_000 {
        return Err("Resource preflight accepts at most 1000 explicit source paths".to_string());
    }
    crate::services::storage_preflight::preflight_operation_resources(
        &operation,
        &output_path,
        &source_paths,
        password.as_deref(),
        estimated_output_bytes,
        estimate_reliable.unwrap_or(false),
    )
    .await
    .map_err(|error| error.to_string())
}

#[command]
pub async fn get_app_version() -> Result<String, String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

/// 从数据目录加载持久化的应用设置 (JSON)
#[command]
pub async fn load_app_settings(app: AppHandle) -> Result<String, String> {
    let resolver = app.path_resolver();
    let data_dir = resolver
        .app_data_dir()
        .ok_or_else(|| "无法获取数据目录".to_string())?;
    let settings_path = data_dir.join("app_settings.json");
    if !settings_path.exists() {
        return Ok("{}".to_string());
    }
    std::fs::read_to_string(&settings_path).map_err(|e| format!("读取设置文件失败: {}", e))
}

/// 将应用设置持久化到数据目录 (JSON)
#[command]
pub async fn save_app_settings(app: AppHandle, settings_json: String) -> Result<(), String> {
    let resolver = app.path_resolver();
    let data_dir = resolver
        .app_data_dir()
        .ok_or_else(|| "无法获取数据目录".to_string())?;
    std::fs::create_dir_all(&data_dir).map_err(|e| format!("创建数据目录失败: {}", e))?;
    let settings_path = data_dir.join("app_settings.json");
    std::fs::write(&settings_path, &settings_json).map_err(|e| format!("保存设置文件失败: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_start_command_quotes_the_executable_and_marks_the_activation() {
        let command = auto_start_command(std::path::Path::new(
            r"C:\Program Files\Long解压\Long解压.exe",
        ))
        .expect("absolute Windows path should be accepted");

        assert_eq!(
            command,
            r#""C:\Program Files\Long解压\Long解压.exe" --autostart"#
        );
    }

    #[test]
    fn auto_start_command_rejects_relative_or_quoted_paths() {
        assert!(auto_start_command(std::path::Path::new("Long解压.exe")).is_err());
        assert!(auto_start_command(std::path::Path::new("C:\\Apps\\bad\"name.exe")).is_err());
    }
}
