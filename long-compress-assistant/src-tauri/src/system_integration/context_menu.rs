/// Windows 右键菜单集成 —— 通过注册表注册 shell 上下文菜单
/// HKCU\Software\Classes\*\shell\LongDecompress          → 所有文件
/// HKCU\Software\Classes\SystemFileAssociations\<ext>\shell\LongDecompress → 归档特定
/// HKCU\Software\Classes\directory\shell\LongDecompress  → 文件夹
#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;
use anyhow::{Context, Result};

/// 支持的归档扩展名列表
const ARCHIVE_EXTENSIONS: &[&str] = &[
    ".zip", ".7z", ".rar", ".tar", ".gz", ".bz2", ".xz", ".zst",
    ".tgz", ".tbz", ".txz", ".tzst", ".iso", ".lzh", ".lha", ".arj",
    ".cab", ".chm", ".deb", ".rpm", ".dmg", ".wim", ".xar", ".cpio",
];

/// 在 Windows 注册表中添加右键上下文菜单
#[cfg(target_os = "windows")]
pub fn register_context_menu(app_path: &str) -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // 1. 对所有文件的通用菜单
    let (all_files_key, _) = hkcu.create_subkey(r"Software\Classes\*\shell\LongDecompress")?;
    all_files_key.set_value("", &"用 胧解压 打开")?;
    all_files_key.set_value("Icon", &format!(r#""{}""#, app_path))?;
    let (all_files_cmd, _) = hkcu.create_subkey(r"Software\Classes\*\shell\LongDecompress\command")?;
    all_files_cmd.set_value("", &format!(r#""{}" --context-menu "%1""#, app_path))?;

    // 2. 对文件夹的菜单
    let (dir_key, _) = hkcu.create_subkey(r"Software\Classes\directory\shell\LongDecompress")?;
    dir_key.set_value("", &"用 胧解压 解压到此处")?;
    dir_key.set_value("Icon", &format!(r#""{}""#, app_path))?;
    let (dir_cmd, _) = hkcu.create_subkey(r"Software\Classes\directory\shell\LongDecompress\command")?;
    dir_cmd.set_value("", &format!(r#""{}" --context-menu "%1""#, app_path))?;

    // 3. 针对每个归档扩展名的专用菜单（显示在 ZIP/RAR 等文件上）
    for ext in ARCHIVE_EXTENSIONS {
        let subkey_path = format!(
            r"Software\Classes\SystemFileAssociations\{}\shell\LongDecompress",
            ext
        );
        let (ext_key, _) = hkcu.create_subkey(&subkey_path)?;
        ext_key.set_value("", &format!("用 胧解压 解压 {}", ext))?;
        ext_key.set_value("Icon", &format!(r#""{}""#, app_path))?;
        let (ext_cmd, _) = hkcu.create_subkey(&format!("{}\\command", subkey_path))?;
        ext_cmd.set_value("", &format!(r#""{}" --context-menu "%1""#, app_path))?;
    }

    Ok(())
}

/// 从 Windows 注册表中移除右键上下文菜单
#[cfg(target_os = "windows")]
pub fn unregister_context_menu() -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // 移除通用文件和文件夹的菜单
    let _ = hkcu.delete_subkey_all(r"Software\Classes\*\shell\LongDecompress\command");
    let _ = hkcu.delete_subkey_all(r"Software\Classes\*\shell\LongDecompress");
    let _ = hkcu.delete_subkey_all(r"Software\Classes\directory\shell\LongDecompress\command");
    let _ = hkcu.delete_subkey_all(r"Software\Classes\directory\shell\LongDecompress");

    // 移除每个扩展名的菜单
    for ext in ARCHIVE_EXTENSIONS {
        let subkey_path = format!(
            r"Software\Classes\SystemFileAssociations\{}\shell\LongDecompress",
            ext
        );
        let _ = hkcu.delete_subkey_all(&format!("{}\\command", subkey_path));
        let _ = hkcu.delete_subkey_all(&subkey_path);
    }

    Ok(())
}

/// 检查右键菜单是否已注册
#[cfg(target_os = "windows")]
pub fn is_context_menu_registered() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.open_subkey(r"Software\Classes\*\shell\LongDecompress").is_ok()
}

/// 非 Windows 平台的空实现
#[cfg(not(target_os = "windows"))]
pub fn register_context_menu(_app_path: &str) -> Result<()> {
    Ok(())
}
#[cfg(not(target_os = "windows"))]
pub fn unregister_context_menu() -> Result<()> {
    Ok(())
}
#[cfg(not(target_os = "windows"))]
pub fn is_context_menu_registered() -> bool {
    false
}
