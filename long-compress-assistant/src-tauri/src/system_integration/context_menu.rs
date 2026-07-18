use anyhow::Result;
/// Windows 级联右键菜单集成
/// 使用 Explorer CommandStore 注册级联子菜单
#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

#[cfg(target_os = "windows")]
use std::ffi::c_void;

#[cfg(target_os = "windows")]
const SHCNE_ASSOCCHANGED: i32 = 0x0800_0000;
#[cfg(target_os = "windows")]
const SHCNF_IDLIST: u32 = 0;

#[cfg(target_os = "windows")]
#[link(name = "shell32")]
extern "system" {
    fn SHChangeNotify(
        event_id: i32,
        flags: u32,
        item1: *const c_void,
        item2: *const c_void,
    );
}

#[cfg(target_os = "windows")]
const NATIVE_COMMAND_CLSID: &str = "{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4A}";
#[cfg(target_os = "windows")]
const NATIVE_COMMAND_VERB: &str = "LongDecompressNative";
#[cfg(target_os = "windows")]
const SHELL_EXTENSION_DLL_PREFIX: &str = "long_compress_shell_extension";

const ARCHIVE_EXTENSIONS: &[&str] = &[
    ".zip", ".zipx", ".7z", ".rar", ".tar", ".gz", ".gzip", ".bz2", ".bzip2", ".xz",
    ".zst", ".zstd", ".lzma", ".tgz", ".tpz", ".tbz", ".tbz2", ".txz", ".tzst",
    ".iso", ".img", ".dmg", ".wim", ".vhd", ".vhdx", ".cab", ".msi", ".deb", ".rpm",
    ".lzh", ".lha", ".arj", ".chm", ".xar", ".cpio", ".squashfs", ".sfs", ".udf",
    ".jar", ".xpi", ".odt", ".ods", ".docx", ".xlsx", ".pptx", ".epub", ".ipa", ".apk",
    ".appx", ".ova", ".aes",
];

#[cfg(target_os = "windows")]
struct VerbDef {
    verb: &'static str,
    label: &'static str,
    cli: &'static str,
}

#[cfg(target_os = "windows")]
fn verbs() -> Vec<VerbDef> {
    vec![
        VerbDef {
            verb: "LongDecompress.open",
            label: "用 胧解压 打开",
            cli: "--open \"%1\"",
        },
        VerbDef {
            verb: "LongDecompress.quickExtract",
            label: "一键解压（推荐）",
            cli: "--quick-extract \"%1\"",
        },
        VerbDef {
            verb: "LongDecompress.extractHere",
            label: "解压到此处",
            cli: "--extract-here \"%1\"",
        },
        VerbDef {
            verb: "LongDecompress.extractTo",
            label: "解压到同名文件夹",
            cli: "--extract-to \"%1\"",
        },
        VerbDef {
            verb: "LongDecompress.testArchive",
            label: "测试归档完整性",
            cli: "--test-archive \"%1\"",
        },
        VerbDef {
            verb: "LongDecompress.compressZip",
            label: "压缩为 ZIP",
            cli: "--compress-zip \"%1\"",
        },
        VerbDef {
            verb: "LongDecompress.compress7z",
            label: "压缩为 7Z",
            cli: "--compress-7z \"%1\"",
        },
        VerbDef {
            verb: "LongDecompress.compressCustom",
            label: "添加到压缩包...",
            cli: "--compress-custom \"%1\"",
        },
        VerbDef {
            verb: "LongDecompress.compressZipHere",
            label: "将当前文件夹压缩为 ZIP",
            cli: "--compress-zip \"%V\"",
        },
        VerbDef {
            verb: "LongDecompress.compress7zHere",
            label: "将当前文件夹压缩为 7Z",
            cli: "--compress-7z \"%V\"",
        },
        VerbDef {
            verb: "LongDecompress.compressCustomHere",
            label: "压缩当前文件夹...",
            cli: "--compress-custom \"%V\"",
        },
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
        // Explorer invokes legacy static verbs once per selected item. The app's
        // single-instance action queue combines those invocations safely.
        verb_key.set_value("MultiSelectModel", &"Document")?;
        let icon = format!(r#""{}""#, app_path);
        verb_key.set_value("Icon", &icon.as_str())?;

        let cmd_path = format!("{}\\command", verb_path);
        let (cmd_key, _) = hkcu.create_subkey(&cmd_path)?;
        let cmd_line = format!(r#""{}" {}"#, app_path, v.cli);
        cmd_key.set_value("", &cmd_line.as_str())?;
    }

    // 2. 普通文件只显示压缩操作；已支持的归档文件同时显示解压和压缩操作。
    let compress_verbs =
        "LongDecompress.compressZip;LongDecompress.compress7z;LongDecompress.compressCustom";
    let archive_verbs = "LongDecompress.open;LongDecompress.quickExtract;LongDecompress.extractHere;LongDecompress.extractTo;LongDecompress.testArchive;LongDecompress.compressZip;LongDecompress.compress7z;LongDecompress.compressCustom";
    reg_shell_entry(
        &hkcu,
        r"Software\Classes\*\shell\LongDecompress",
        compress_verbs,
    )?;
    for ext in ARCHIVE_EXTENSIONS {
        reg_shell_entry(
            &hkcu,
            &format!(
                r"Software\Classes\SystemFileAssociations\{}\shell\LongDecompress",
                ext
            ),
            archive_verbs,
        )?;
    }

    // 3. 文件夹和文件夹空白处压缩菜单
    reg_shell_entry(
        &hkcu,
        r"Software\Classes\directory\shell\LongDecompress",
        compress_verbs,
    )?;
    let background_verbs = "LongDecompress.compressZipHere;LongDecompress.compress7zHere;LongDecompress.compressCustomHere";
    reg_shell_entry(
        &hkcu,
        r"Software\Classes\directory\Background\shell\LongDecompress",
        background_verbs,
    )?;

    if is_windows_11_or_newer() {
        register_native_menu(&hkcu, app_path)?;
    } else {
        unregister_native_menu(&hkcu)?;
    }

    notify_shell_associations_changed();
    Ok(())
}

#[cfg(target_os = "windows")]
fn notify_shell_associations_changed() {
    // SAFETY: SHCNE_ASSOCCHANGED with SHCNF_IDLIST requires both item pointers
    // to be null. The call only invalidates Explorer's association cache.
    unsafe {
        SHChangeNotify(
            SHCNE_ASSOCCHANGED,
            SHCNF_IDLIST,
            std::ptr::null(),
            std::ptr::null(),
        );
    }
}

#[cfg(target_os = "windows")]
fn is_windows_11_or_newer() -> bool {
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
        .and_then(|key| key.get_value::<String, _>("CurrentBuildNumber"))
        .ok()
        .and_then(|build| build.parse::<u32>().ok())
        .is_some_and(|build| build >= 22_000)
}

#[cfg(target_os = "windows")]
fn shell_extension_path(app_path: &str) -> std::path::PathBuf {
    let version = env!("CARGO_PKG_VERSION")
        .replace(|character: char| !character.is_ascii_alphanumeric(), "_");
    let dll_name = format!("{}_{}.dll", SHELL_EXTENSION_DLL_PREFIX, version);
    let executable = std::path::Path::new(app_path);
    let installed = executable
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("resources")
        .join(&dll_name);
    if installed.exists() {
        installed
    } else {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join(dll_name)
    }
}

#[cfg(target_os = "windows")]
fn register_native_menu(hkcu: &RegKey, app_path: &str) -> Result<()> {
    let dll_path = shell_extension_path(app_path);
    if !dll_path.is_file() {
        anyhow::bail!(
            "Windows shell extension is missing: {}",
            dll_path.display()
        );
    }

    let clsid_path = format!(r"Software\Classes\CLSID\{}", NATIVE_COMMAND_CLSID);
    let (clsid_key, _) = hkcu.create_subkey(&clsid_path)?;
    clsid_key.set_value("", &"胧解压 Windows 11 原生菜单")?;
    clsid_key.set_value("ApplicationPath", &app_path)?;
    let (server_key, _) = hkcu.create_subkey(format!(r"{}\InprocServer32", clsid_path))?;
    server_key.set_value("", &dll_path.to_string_lossy().as_ref())?;
    server_key.set_value("ThreadingModel", &"Apartment")?;

    for class in ["*", "Directory"] {
        let verb_path = format!(
            r"Software\Classes\{}\shell\{}",
            class, NATIVE_COMMAND_VERB
        );
        let (verb_key, _) = hkcu.create_subkey(&verb_path)?;
        verb_key.set_value("MUIVerb", &"胧解压")?;
        verb_key.set_value("Icon", &format!(r#""{}""#, app_path))?;
        verb_key.set_value("ExplorerCommandHandler", &NATIVE_COMMAND_CLSID)?;
        verb_key.set_value("MultiSelectModel", &"Player")?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn unregister_native_menu(hkcu: &RegKey) -> Result<()> {
    // Remove the first native-menu prototype during migration.
    delete_tree_if_present(
        hkcu,
        r"Software\Classes\*\shell\LongDecompressQuickExtract",
    )?;
    for class in ["*", "Directory"] {
        delete_tree_if_present(
            hkcu,
            &format!(r"Software\Classes\{}\shell\{}", class, NATIVE_COMMAND_VERB),
        )?;
    }
    delete_tree_if_present(
        hkcu,
        &format!(r"Software\Classes\CLSID\{}", NATIVE_COMMAND_CLSID),
    )
}

#[cfg(target_os = "windows")]
fn reg_shell_entry(hkcu: &RegKey, path: &str, sub_commands: &str) -> Result<()> {
    let (key, _) = hkcu.create_subkey(path)?;
    key.set_value("MUIVerb", &"胧解压")?;
    key.set_value("SubCommands", &sub_commands)?;
    key.set_value("MultiSelectModel", &"Document")?;
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn refresh_context_menu_if_present(app_path: &str) -> Result<bool> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if hkcu
        .open_subkey(r"Software\Classes\*\shell\LongDecompress")
        .is_err()
    {
        return Ok(false);
    }
    register_context_menu(app_path)?;
    Ok(true)
}

/// 从 Windows 注册表中移除所有右键菜单
#[cfg(target_os = "windows")]
pub fn unregister_context_menu() -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let store_base = r"Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell";
    for v in verbs() {
        let p = format!("{}\\{}", store_base, v.verb);
        delete_tree_if_present(&hkcu, &p)?;
    }
    for entry in [
        r"Software\Classes\*\shell\LongDecompress",
        r"Software\Classes\directory\shell\LongDecompress",
        r"Software\Classes\directory\Background\shell\LongDecompress",
    ] {
        delete_tree_if_present(&hkcu, entry)?;
    }
    for ext in ARCHIVE_EXTENSIONS {
        delete_tree_if_present(
            &hkcu,
            &format!(
                r"Software\Classes\SystemFileAssociations\{}\shell\LongDecompress",
                ext
            ),
        )?;
    }
    unregister_native_menu(&hkcu)?;
    notify_shell_associations_changed();
    Ok(())
}

#[cfg(target_os = "windows")]
fn delete_tree_if_present(hkcu: &RegKey, path: &str) -> Result<()> {
    match hkcu.delete_subkey_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(target_os = "windows")]
pub fn is_context_menu_registered() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let required_keys = [
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell\LongDecompress.open",
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell\LongDecompress.quickExtract",
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell\LongDecompress.compressZip",
        r"Software\Classes\*\shell\LongDecompress",
        r"Software\Classes\directory\shell\LongDecompress",
        r"Software\Classes\directory\Background\shell\LongDecompress",
    ];
    if !required_keys.iter().all(|path| hkcu.open_subkey(path).is_ok()) {
        return false;
    }
    let Ok(exe_path) = std::env::current_exe() else { return false };
    let exe_string = exe_path.to_string_lossy();
    if is_windows_11_or_newer() {
        let clsid_path = format!(r"Software\Classes\CLSID\{}", NATIVE_COMMAND_CLSID);
        let expected_dll = shell_extension_path(&exe_string).to_string_lossy().into_owned();
        let native_paths_are_current = hkcu
            .open_subkey(&clsid_path)
            .and_then(|key| key.get_value::<String, _>("ApplicationPath"))
            .map(|path| path.eq_ignore_ascii_case(&exe_string))
            .unwrap_or(false)
            && hkcu
                .open_subkey(format!(r"{}\InprocServer32", clsid_path))
                .and_then(|key| key.get_value::<String, _>(""))
                .map(|path| path.eq_ignore_ascii_case(&expected_dll))
                .unwrap_or(false)
            && ["*", "Directory"].iter().all(|class| {
                hkcu.open_subkey(format!(
                    r"Software\Classes\{}\shell\{}",
                    class, NATIVE_COMMAND_VERB
                ))
                .and_then(|key| key.get_value::<String, _>("ExplorerCommandHandler"))
                .map(|handler| handler == NATIVE_COMMAND_CLSID)
                .unwrap_or(false)
            });
        if !native_paths_are_current {
            return false;
        }
    }

    let expected = format!(r#""{}""#, exe_path.to_string_lossy());
    ["LongDecompress.open", "LongDecompress.quickExtract", "LongDecompress.compressZip"]
        .iter()
        .all(|verb| {
            let path = format!(
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell\{}\command",
                verb
            );
            hkcu.open_subkey(path)
                .and_then(|key| key.get_value::<String, _>(""))
                .map(|command| command.starts_with(&expected))
                .unwrap_or(false)
        })
}

// 非 Windows 空实现
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
#[cfg(not(target_os = "windows"))]
pub fn refresh_context_menu_if_present(_app_path: &str) -> Result<bool> {
    Ok(false)
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::{
        verbs, ARCHIVE_EXTENSIONS, NATIVE_COMMAND_CLSID, NATIVE_COMMAND_VERB,
    };

    const INSTALLER_TEMPLATE: &str = include_str!("../../installer.nsi");
    const SHELL_EXTENSION_SOURCE: &str = include_str!("../../shell-extension/src/lib.rs");

    #[test]
    fn shell_commands_quote_paths_and_include_quick_extract() {
        let verbs = verbs();
        assert!(verbs
            .iter()
            .all(|verb| { !verb.cli.contains("%1") || verb.cli.contains("\"%1\"") }));
        assert!(verbs.iter().any(|verb| {
            verb.verb == "LongDecompress.quickExtract" && verb.cli == "--quick-extract \"%1\""
        }));
    }

    #[test]
    fn nsis_uninstaller_cleans_every_registered_context_menu_key() {
        assert!(INSTALLER_TEMPLATE.contains("!insertmacro RemoveLongDecompressContextMenu"));
        assert!(INSTALLER_TEMPLATE.contains(
            r#"Delete /REBOOTOK "$INSTDIR\\{{this.[1]}}""#
        ));
        for verb in verbs() {
            let key = format!(
                "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\CommandStore\\shell\\{}",
                verb.verb
            );
            assert!(
                INSTALLER_TEMPLATE.contains(&key),
                "NSIS cleanup is missing CommandStore verb {key}"
            );
        }
        for extension in ARCHIVE_EXTENSIONS {
            let key = format!(
                "Software\\Classes\\SystemFileAssociations\\{}\\shell\\LongDecompress",
                extension
            );
            assert!(
                INSTALLER_TEMPLATE.contains(&key),
                "NSIS cleanup is missing archive association {key}"
            );
        }
        assert!(INSTALLER_TEMPLATE.contains(NATIVE_COMMAND_CLSID));
        assert!(INSTALLER_TEMPLATE.contains(NATIVE_COMMAND_VERB));
        assert!(INSTALLER_TEMPLATE.contains("SHChangeNotify(i 0x08000000"));
        assert!(SHELL_EXTENSION_SOURCE.contains(NATIVE_COMMAND_CLSID));
    }

    #[test]
    fn native_menu_forwards_every_primary_shell_action() {
        for flag in [
            "--quick-extract",
            "--extract-here",
            "--extract-to",
            "--test-archive",
            "--compress-zip",
            "--compress-7z",
            "--compress-custom",
        ] {
            assert!(SHELL_EXTENSION_SOURCE.contains(flag), "native menu is missing {flag}");
        }
        for extension in ARCHIVE_EXTENSIONS {
            let extension_without_dot = extension.trim_start_matches('.');
            assert!(
                SHELL_EXTENSION_SOURCE.contains(&format!(r#""{}""#, extension_without_dot)),
                "native menu archive detection is missing {extension}"
            );
        }
    }
}
