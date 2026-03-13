use crate::services::system_service::{SystemService, SystemInfo};
use tauri::{command, AppHandle};

#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

#[command]
pub async fn get_system_info() -> Result<SystemInfo, String> {
    let mut service = SystemService::new();
    Ok(service.get_system_info())
}

#[command]
pub async fn get_disk_space(path: String) -> Result<(u64, u64), String> {
    match std::fs::metadata(&path) {
        Ok(_) => {
            // 简化实现，实际应该使用更精确的磁盘空间检查
            let total = 1024 * 1024 * 1024 * 100; // 假设100GB
            let free = 1024 * 1024 * 1024 * 50;   // 假设50GB空闲
            Ok((total, free))
        }
        Err(e) => Err(format!("获取磁盘空间失败: {}", e)),
    }
}

#[command]
pub async fn get_app_version() -> Result<String, String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

#[command]
pub async fn set_auto_start(enable: bool, app_handle: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
        let key = hkcu.open_subkey_with_flags(path, KEY_WRITE)
            .map_err(|e| format!("无法打开注册表项: {}", e))?;
        
        let app_name = app_handle.config().package.product_name.clone().unwrap_or_else(|| "LongDecompress".to_string());
        
        if enable {
            let exe_path = std::env::current_exe()
                .map_err(|e| format!("无法获取可执行文件路径: {}", e))?;
            key.set_value(&app_name, &exe_path.to_str().unwrap_or_default())
                .map_err(|e| format!("无法设置注册表值: {}", e))?;
        } else {
            let _ = key.delete_value(&app_name);
        }
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("目前仅支持 Windows 系统的自启动设置".to_string())
    }
}

#[command]
pub async fn check_auto_start(app_handle: AppHandle) -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
        let key = hkcu.open_subkey_with_flags(path, KEY_READ)
            .map_err(|e| format!("无法打开注册表项: {}", e))?;
        
        let app_name = app_handle.config().package.product_name.clone().unwrap_or_else(|| "LongDecompress".to_string());
        
        match key.get_value::<String, _>(&app_name) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(false)
    }
}
