/// Windows 级联右键菜单集成
/// 使用 Explorer CommandStore 注册级联子菜单
#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;
use anyhow::{Context, Result};

const ARCHIVE_EXTENSIONS: &[&str] = &[
    ".zip", ".7z", ".rar", ".tar", ".gz", ".bz2", ".xz", ".zst",
    ".tgz", ".tbz", ".txz", ".tzst", ".iso", ".lzh", ".lha", ".arj",
    ".cab", ".chm", ".deb", ".rpm", ".dmg", ".wim", ".xar", ".cpio",
];

#[cfg(target_os = "windows")]
struct VerbDef { verb: &'static str, label: &'static str, cli: &'static str }

#[cfg(target_os = "windows")]
fn verbs() -> Vec<VerbDef> {
    vec![
        VerbDef { verb: "LongDecompress.open",           label: "用 胧解压 打开",        cli: "--open" },
        VerbDef { verb: "LongDecompress.extractHere",    label: "解压到此处",             cli: "--extract-here %1 --cwd %V" },
        VerbDef { verb: "LongDecompress.extractTo",      label: "解压到同名文件夹",        cli: "--extract-to %1 --cwd %V" },
        VerbDef { verb: "LongDecompress.testArchive",    label: "测试归档完整性",          cli: "--test-archive %1" },
        VerbDef { verb: "LongDecompress.compressZip",    label: "压缩为 ZIP",              cli: "--compress-zip %1" },
        VerbDef { verb: "LongDecompress.compress7z",     label: "压缩为 7Z",               cli: "--compress-7z %1" },
        VerbDef { verb: "LongDecompress.compressCustom", label: "添加到压缩包...",          cli: "--compress-custom %1" },
    ]
}

/// 在 Windows 注册表中添加全部级联右键菜单
#[cfg(target_os = "windows")]
pub fn register_context_menu(app_path: &str) -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // 1. 注册所有操作动词到 CommandStore
    let store_base = r"Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell";
    for v in verbs() {
        let verb_path = format!("{}\\{}", store_base, v.verb);
        let (verb_key, _) = hkcu.create_subkey(&verb_path)?;
        verb_key.set_value("", &v.label)?;
        let icon = format!(r#""{}""#, app_path);
        verb_key.set_value("Icon", &icon.as_str())?;

        let cmd_path = format!("{}\\command", verb_path);
        let (cmd_key, _) = hkcu.create_subkey(&cmd_path)?;
        let cmd_line = format!(r#""{}" {}"#, app_path, v.cli);
        cmd_key.set_value("", &cmd_line.as_str())?;
    }

    // 2. 归档文件菜单（所有文件 + 各扩展名）
    let archive_verbs = "LongDecompress.open;LongDecompress.extractHere;LongDecompress.extractTo;LongDecompress.testArchive";
    reg_shell_entry(&hkcu, r"Software\Classes\*\shell\LongDecompress", archive_verbs)?;
    for ext in ARCHIVE_EXTENSIONS {
        reg_shell_entry(&hkcu, &format!(r"Software\Classes\SystemFileAssociations\{}\shell\LongDecompress", ext), archive_verbs)?;
    }

    // 3. 文件夹/普通文件的压缩菜单
    let compress_verbs = "LongDecompress.compressZip;LongDecompress.compress7z;LongDecompress.compressCustom";
    reg_shell_entry(&hkcu, r"Software\Classes\directory\shell\LongDecompress", compress_verbs)?;
    reg_shell_entry(&hkcu, r"Software\Classes\directory\Background\shell\LongDecompress", compress_verbs)?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn reg_shell_entry(hkcu: &RegKey, path: &str, sub_commands: &str) -> Result<()> {
    let (key, _) = hkcu.create_subkey(path)?;
    key.set_value("MUIVerb", &"胧解压")?;
    key.set_value("SubCommands", &sub_commands)?;
    Ok(())
}

/// 从 Windows 注册表中移除所有右键菜单
#[cfg(target_os = "windows")]
pub fn unregister_context_menu() -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let store_base = r"Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell";
    for v in verbs() {
        let p = format!("{}\\{}", store_base, v.verb);
        let _ = hkcu.delete_subkey_all(format!("{}\\command", p));
        let _ = hkcu.delete_subkey_all(&p);
    }
    for entry in [r"Software\Classes\*\shell\LongDecompress",
                  r"Software\Classes\directory\shell\LongDecompress",
                  r"Software\Classes\directory\Background\shell\LongDecompress"] {
        let _ = hkcu.delete_subkey_all(entry);
    }
    for ext in ARCHIVE_EXTENSIONS {
        let _ = hkcu.delete_subkey_all(format!(r"Software\Classes\SystemFileAssociations\{}\shell\LongDecompress", ext));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn is_context_menu_registered() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell\LongDecompress.open")
        .is_ok()
}

// 非 Windows 空实现
#[cfg(not(target_os = "windows"))]
pub fn register_context_menu(_app_path: &str) -> Result<()> { Ok(()) }
#[cfg(not(target_os = "windows"))]
pub fn unregister_context_menu() -> Result<()> { Ok(()) }
#[cfg(not(target_os = "windows"))]
pub fn is_context_menu_registered() -> bool { false }
