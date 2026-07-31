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
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::process::Command;

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
const QUICK_EXTRACT_COMMAND_CLSID: &str = "{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4B}";
#[cfg(target_os = "windows")]
const QUICK_EXTRACT_COMMAND_VERB: &str = "LongDecompressNativeQuickExtract";
#[cfg(target_os = "windows")]
const QUICK_PACK_COMMAND_CLSID: &str = "{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4C}";
#[cfg(target_os = "windows")]
const QUICK_PACK_COMMAND_VERB: &str = "LongDecompressNativeQuickPack";
#[cfg(target_os = "windows")]
const SHELL_EXTENSION_DLL_PREFIX: &str = "long_compress_shell_extension";
#[cfg(target_os = "windows")]
const CONTEXT_MENU_QUICK_EXTRACT_PACKAGE_NAME: &str =
    "long_compress_context_menu_extract.msix";
#[cfg(target_os = "windows")]
const CONTEXT_MENU_QUICK_PACK_PACKAGE_NAME: &str = "long_compress_context_menu_pack.msix";
#[cfg(target_os = "windows")]
const CONTEXT_MENU_REGISTRATION_SCRIPT: &str = "long_compress_context_menu_registration.ps1";
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(target_os = "windows")]
fn context_menu_package_version() -> String {
    let mut components = env!("CARGO_PKG_VERSION").split('.');
    let major = components.next().unwrap_or("0");
    let minor = components.next().unwrap_or("0");
    let patch = components
        .next()
        .unwrap_or("0")
        .split(|character: char| !character.is_ascii_digit())
        .next()
        .unwrap_or("0");
    format!("{major}.{minor}.{patch}.0")
}

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
const COMPRESS_VERBS: &[&str] = &[
    "LongDecompress.compressZip",
    "LongDecompress.compress7z",
    "LongDecompress.compressCustom",
];

#[cfg(target_os = "windows")]
const ARCHIVE_VERBS: &[&str] = &[
    "LongDecompress.open",
    "LongDecompress.quickExtract",
    "LongDecompress.extractHere",
    "LongDecompress.extractTo",
    "LongDecompress.testArchive",
    "LongDecompress.compressZip",
    "LongDecompress.compress7z",
    "LongDecompress.compressCustom",
];

#[cfg(target_os = "windows")]
const BACKGROUND_VERBS: &[&str] = &[
    "LongDecompress.compressZipHere",
    "LongDecompress.compress7zHere",
    "LongDecompress.compressCustomHere",
];

#[cfg(target_os = "windows")]
fn verbs() -> Vec<VerbDef> {
    vec![
        VerbDef {
            verb: "LongDecompress.open",
            label: "用 Long解压 打开",
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

    // Windows 11 uses a dynamic IExplorerCommand menu. Registering the legacy
    // CommandStore cascade at the same time makes Explorer merge two menu
    // providers and can produce duplicated or malformed "Show more options"
    // entries. Always clean both implementations first, then install exactly
    // one implementation for the current Windows version.
    unregister_legacy_menu(&hkcu)?;
    unregister_native_menu(&hkcu)?;

    if is_windows_11_or_newer() {
        let native_result = (|| -> Result<bool> {
            if !register_sparse_identity_package(app_path)? {
                return Ok(false);
            }
            register_native_menu(&hkcu, app_path)?;
            {
                let clsid_path = format!(r"Software\Classes\CLSID\{}", NATIVE_COMMAND_CLSID);
                let clsid_key = hkcu.open_subkey_with_flags(clsid_path, KEY_SET_VALUE)?;
                clsid_key.set_value("SparsePackageRegistered", &1u32)?;
            }
            Ok(true)
        })();
        match native_result {
            Ok(true) => {}
            Ok(false) => {
                unregister_native_menu(&hkcu)?;
                register_legacy_menu(&hkcu, app_path)?;
            }
            Err(error) => {
                // Public unsigned builds cannot deploy the Windows 11 sparse
                // identity package. Remove every partial native registration
                // before falling back to the classic "Show more options" menu.
                log::warn!("Windows 11 primary context-menu registration failed: {error}");
                if let Err(cleanup_error) = unregister_sparse_identity_package() {
                    log::warn!(
                        "Windows 11 identity-package cleanup after fallback failed: {cleanup_error}"
                    );
                }
                unregister_native_menu(&hkcu)?;
                register_legacy_menu(&hkcu, app_path)?;
            }
        }
    } else {
        register_legacy_menu(&hkcu, app_path)?;
    }

    notify_shell_associations_changed();
    Ok(())
}

#[cfg(target_os = "windows")]
fn register_legacy_menu(hkcu: &RegKey, app_path: &str) -> Result<()> {
    let all_verbs = verbs();

    // CommandStore custom verbs are documented under HKLM. Putting them under
    // HKCU leaves the cascade root visible while Explorer silently drops its
    // children on current Windows 11 builds. ExtendedSubCommandsKey supports
    // inline per-user verbs under HKCU\Software\Classes, so it works without
    // elevation and keeps every submenu self-contained.
    reg_shell_entry(
        hkcu,
        r"Software\Classes\*\shell\LongDecompress",
        &all_verbs,
        COMPRESS_VERBS,
        app_path,
    )?;
    for ext in ARCHIVE_EXTENSIONS {
        reg_shell_entry(
            hkcu,
            &format!(
                r"Software\Classes\SystemFileAssociations\{}\shell\LongDecompress",
                ext
            ),
            &all_verbs,
            ARCHIVE_VERBS,
            app_path,
        )?;
    }

    // 文件夹和文件夹空白处压缩菜单。
    reg_shell_entry(
        hkcu,
        r"Software\Classes\directory\shell\LongDecompress",
        &all_verbs,
        COMPRESS_VERBS,
        app_path,
    )?;
    reg_shell_entry(
        hkcu,
        r"Software\Classes\directory\Background\shell\LongDecompress",
        &all_verbs,
        BACKGROUND_VERBS,
        app_path,
    )?;

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

    for (clsid, label) in [
        (NATIVE_COMMAND_CLSID, "Long解压 Windows 11 原生菜单"),
        (QUICK_EXTRACT_COMMAND_CLSID, "Long解压一键解压"),
        (QUICK_PACK_COMMAND_CLSID, "Long解压一键打包"),
    ] {
        let clsid_path = format!(r"Software\Classes\CLSID\{}", clsid);
        let (clsid_key, _) = hkcu.create_subkey(&clsid_path)?;
        clsid_key.set_value("", &label)?;
        // Every class can be activated independently. Keeping the application
        // path on each registration also makes repair/migration self-contained.
        clsid_key.set_value("ApplicationPath", &app_path)?;
        let (server_key, _) = hkcu.create_subkey(format!(r"{}\InprocServer32", clsid_path))?;
        server_key.set_value("", &dll_path.to_string_lossy().as_ref())?;
        server_key.set_value("ThreadingModel", &"Apartment")?;
    }

    for class in ["*", "Directory"] {
        let verb_path = format!(
            r"Software\Classes\{}\shell\{}",
            class, NATIVE_COMMAND_VERB
        );
        let (verb_key, _) = hkcu.create_subkey(&verb_path)?;
        verb_key.set_value("MUIVerb", &"Long解压")?;
        verb_key.set_value("Icon", &format!(r#""{}""#, app_path))?;
        verb_key.set_value("ExplorerCommandHandler", &NATIVE_COMMAND_CLSID)?;
        verb_key.set_value("MultiSelectModel", &"Player")?;
    }

    register_native_verb(
        hkcu,
        "*",
        QUICK_EXTRACT_COMMAND_VERB,
        "一键解压到同名文件夹",
        QUICK_EXTRACT_COMMAND_CLSID,
        app_path,
    )?;
    for class in ["*", "Directory"] {
        register_native_verb(
            hkcu,
            class,
            QUICK_PACK_COMMAND_VERB,
            "一键打包为 ZIP",
            QUICK_PACK_COMMAND_CLSID,
            app_path,
        )?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn context_menu_resource_path(app_path: &str, file_name: &str) -> std::path::PathBuf {
    let executable = std::path::Path::new(app_path);
    let installed = executable
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("resources")
        .join(file_name);
    if installed.exists() {
        installed
    } else {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join(file_name)
    }
}

#[cfg(target_os = "windows")]
fn invoke_sparse_package_script(action: &str, app_path: Option<&str>) -> Result<bool> {
    let lookup_path = app_path
        .map(std::borrow::ToOwned::to_owned)
        .or_else(|| std::env::current_exe().ok().map(|path| path.to_string_lossy().into_owned()))
        .unwrap_or_default();
    let script = context_menu_resource_path(&lookup_path, CONTEXT_MENU_REGISTRATION_SCRIPT);
    if !script.is_file() {
        return Ok(false);
    }

    let mut command = Command::new("powershell.exe");
    command.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-WindowStyle",
        "Hidden",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
    ]);
    command.arg(&script).args(["-Action", action]);
    if action == "Install" {
        let Some(app_path) = app_path else {
            anyhow::bail!("Application path is required to register the identity package");
        };
        let quick_extract_package =
            context_menu_resource_path(app_path, CONTEXT_MENU_QUICK_EXTRACT_PACKAGE_NAME);
        let quick_pack_package =
            context_menu_resource_path(app_path, CONTEXT_MENU_QUICK_PACK_PACKAGE_NAME);
        if !quick_extract_package.is_file() || !quick_pack_package.is_file() {
            return Ok(false);
        }
        let external_location = std::path::Path::new(app_path)
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        command
            .arg("-QuickExtractPackagePath")
            .arg(quick_extract_package)
            .arg("-QuickPackPackagePath")
            .arg(quick_pack_package)
            .arg("-ExternalLocation")
            .arg(external_location)
            .arg("-PackageVersion")
            .arg(context_menu_package_version());
    }
    let status = command.creation_flags(CREATE_NO_WINDOW).status()?;
    if !status.success() {
        anyhow::bail!("identity package script exited with status {status}");
    }
    Ok(true)
}

#[cfg(target_os = "windows")]
fn register_sparse_identity_package(app_path: &str) -> Result<bool> {
    invoke_sparse_package_script("Install", Some(app_path))
}

#[cfg(target_os = "windows")]
fn unregister_sparse_identity_package() -> Result<bool> {
    invoke_sparse_package_script("Uninstall", None)
}

#[cfg(target_os = "windows")]
fn register_native_verb(
    hkcu: &RegKey,
    class: &str,
    verb: &str,
    label: &str,
    clsid: &str,
    app_path: &str,
) -> Result<()> {
    let verb_path = format!(r"Software\Classes\{}\shell\{}", class, verb);
    let (verb_key, _) = hkcu.create_subkey(verb_path)?;
    verb_key.set_value("MUIVerb", &label)?;
    verb_key.set_value("Icon", &format!(r#""{}""#, app_path))?;
    verb_key.set_value("ExplorerCommandHandler", &clsid)?;
    verb_key.set_value("MultiSelectModel", &"Player")?;
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
        delete_tree_if_present(
            hkcu,
            &format!(r"Software\Classes\{}\shell\{}", class, QUICK_PACK_COMMAND_VERB),
        )?;
    }
    delete_tree_if_present(
        hkcu,
        &format!(r"Software\Classes\*\shell\{}", QUICK_EXTRACT_COMMAND_VERB),
    )?;
    for clsid in [
        NATIVE_COMMAND_CLSID,
        QUICK_EXTRACT_COMMAND_CLSID,
        QUICK_PACK_COMMAND_CLSID,
    ] {
        delete_tree_if_present(
            hkcu,
            &format!(r"Software\Classes\CLSID\{}", clsid),
        )?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn unregister_legacy_menu(hkcu: &RegKey) -> Result<()> {
    let store_base = r"Software\Microsoft\Windows\CurrentVersion\Explorer\CommandStore\shell";
    for verb in verbs() {
        delete_tree_if_present(hkcu, &format!("{}\\{}", store_base, verb.verb))?;
    }
    for entry in [
        r"Software\Classes\*\shell\LongDecompress",
        r"Software\Classes\directory\shell\LongDecompress",
        r"Software\Classes\directory\Background\shell\LongDecompress",
    ] {
        delete_tree_if_present(hkcu, entry)?;
    }
    for extension in ARCHIVE_EXTENSIONS {
        delete_tree_if_present(
            hkcu,
            &format!(
                r"Software\Classes\SystemFileAssociations\{}\shell\LongDecompress",
                extension
            ),
        )?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn reg_shell_entry(
    hkcu: &RegKey,
    path: &str,
    all_verbs: &[VerbDef],
    selected_verbs: &[&str],
    app_path: &str,
) -> Result<()> {
    let (key, _) = hkcu.create_subkey(path)?;
    key.set_value("MUIVerb", &"Long解压")?;
    key.set_value("Icon", &format!(r#""{}""#, app_path))?;
    key.set_value("MultiSelectModel", &"Document")?;

    let submenu_path = format!(r"{}\ExtendedSubCommandsKey\shell", path);
    for (index, verb_name) in selected_verbs.iter().enumerate() {
        let verb = all_verbs
            .iter()
            .find(|candidate| candidate.verb == *verb_name)
            .ok_or_else(|| anyhow::anyhow!("unknown Explorer context-menu verb: {verb_name}"))?;
        // Prefixes make Explorer's registry enumeration deterministic while
        // the stable verb name remains available for diagnostics.
        let verb_path = format!(r"{}\{:02}.{}", submenu_path, index + 1, verb.verb);
        let (verb_key, _) = hkcu.create_subkey(&verb_path)?;
        verb_key.set_value("MUIVerb", &verb.label)?;
        verb_key.set_value("Icon", &format!(r#""{}""#, app_path))?;
        verb_key.set_value("MultiSelectModel", &"Document")?;

        let (command_key, _) = hkcu.create_subkey(format!(r"{}\command", verb_path))?;
        command_key.set_value("", &format!(r#""{}" {}"#, app_path, verb.cli))?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn refresh_context_menu_if_present(app_path: &str) -> Result<bool> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let legacy_present = hkcu
        .open_subkey(r"Software\Classes\*\shell\LongDecompress")
        .is_ok();
    let native_present = hkcu
        .open_subkey(format!(
            r"Software\Classes\*\shell\{}",
            NATIVE_COMMAND_VERB
        ))
        .is_ok();
    if !legacy_present && !native_present {
        return Ok(false);
    }
    register_context_menu(app_path)?;
    Ok(true)
}

/// 从 Windows 注册表中移除所有右键菜单
#[cfg(target_os = "windows")]
pub fn unregister_context_menu() -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if is_windows_11_or_newer() {
        unregister_sparse_identity_package()?;
    }
    unregister_legacy_menu(&hkcu)?;
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
fn native_package_registration_is_current(
    signed_packages_present: bool,
    sparse_package_registered: bool,
) -> bool {
    signed_packages_present && sparse_package_registered
}

#[cfg(target_os = "windows")]
fn legacy_shell_entry_is_current(
    hkcu: &RegKey,
    path: &str,
    all_verbs: &[VerbDef],
    selected_verbs: &[&str],
    app_path: &str,
) -> bool {
    let Ok(root) = hkcu.open_subkey(path) else {
        return false;
    };
    if root
        .get_value::<String, _>("MUIVerb")
        .map(|label| label != "Long解压")
        .unwrap_or(true)
        || root.get_value::<String, _>("SubCommands").is_ok()
    {
        return false;
    }

    selected_verbs.iter().enumerate().all(|(index, verb_name)| {
        let Some(verb) = all_verbs
            .iter()
            .find(|candidate| candidate.verb == *verb_name)
        else {
            return false;
        };
        let command_path = format!(
            r"{}\ExtendedSubCommandsKey\shell\{:02}.{}\command",
            path,
            index + 1,
            verb.verb
        );
        let expected_command = format!(r#""{}" {}"#, app_path, verb.cli);
        hkcu.open_subkey(command_path)
            .and_then(|key| key.get_value::<String, _>(""))
            .map(|command| command.eq_ignore_ascii_case(&expected_command))
            .unwrap_or(false)
    })
}

#[cfg(target_os = "windows")]
pub fn is_context_menu_registered() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(exe_path) = std::env::current_exe() else { return false };
    let exe_string = exe_path.to_string_lossy();

    if is_windows_11_or_newer() {
        let clsid_path = format!(r"Software\Classes\CLSID\{}", NATIVE_COMMAND_CLSID);
        let expected_dll = shell_extension_path(&exe_string).to_string_lossy().into_owned();
        let root_registered = hkcu
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
        let quick_extract_registered = hkcu
            .open_subkey(format!(
                r"Software\Classes\*\shell\{}",
                QUICK_EXTRACT_COMMAND_VERB
            ))
            .and_then(|key| key.get_value::<String, _>("ExplorerCommandHandler"))
            .map(|handler| handler == QUICK_EXTRACT_COMMAND_CLSID)
            .unwrap_or(false);
        let quick_pack_registered = ["*", "Directory"].iter().all(|class| {
            hkcu.open_subkey(format!(
                r"Software\Classes\{}\shell\{}",
                class, QUICK_PACK_COMMAND_VERB
            ))
            .and_then(|key| key.get_value::<String, _>("ExplorerCommandHandler"))
            .map(|handler| handler == QUICK_PACK_COMMAND_CLSID)
            .unwrap_or(false)
        });
        let signed_packages_present = [
            CONTEXT_MENU_QUICK_EXTRACT_PACKAGE_NAME,
            CONTEXT_MENU_QUICK_PACK_PACKAGE_NAME,
        ]
        .iter()
        .all(|name| context_menu_resource_path(&exe_string, name).is_file());
        let sparse_package_registered = hkcu
            .open_subkey(&clsid_path)
            .and_then(|key| key.get_value::<u32, _>("SparsePackageRegistered"))
            .map(|registered| registered == 1)
            .unwrap_or(false);
        let package_registration_is_current = native_package_registration_is_current(
            signed_packages_present,
            sparse_package_registered,
        );
        if root_registered
            && quick_extract_registered
            && quick_pack_registered
            && package_registration_is_current
        {
            return true;
        }
    }

    let all_verbs = verbs();
    let app_path = exe_path.to_string_lossy();
    legacy_shell_entry_is_current(
        &hkcu,
        r"Software\Classes\*\shell\LongDecompress",
        &all_verbs,
        COMPRESS_VERBS,
        &app_path,
    ) && legacy_shell_entry_is_current(
        &hkcu,
        r"Software\Classes\directory\shell\LongDecompress",
        &all_verbs,
        COMPRESS_VERBS,
        &app_path,
    ) && legacy_shell_entry_is_current(
        &hkcu,
        r"Software\Classes\directory\Background\shell\LongDecompress",
        &all_verbs,
        BACKGROUND_VERBS,
        &app_path,
    ) && ARCHIVE_EXTENSIONS.iter().all(|extension| {
        legacy_shell_entry_is_current(
            &hkcu,
            &format!(
                r"Software\Classes\SystemFileAssociations\{}\shell\LongDecompress",
                extension
            ),
            &all_verbs,
            ARCHIVE_VERBS,
            &app_path,
        )
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
        legacy_shell_entry_is_current, native_package_registration_is_current, reg_shell_entry,
        verbs, RegKey, ARCHIVE_EXTENSIONS, ARCHIVE_VERBS, BACKGROUND_VERBS, COMPRESS_VERBS,
        HKEY_CURRENT_USER, NATIVE_COMMAND_CLSID, NATIVE_COMMAND_VERB, QUICK_EXTRACT_COMMAND_CLSID,
        QUICK_EXTRACT_COMMAND_VERB, QUICK_PACK_COMMAND_CLSID, QUICK_PACK_COMMAND_VERB,
    };

    const INSTALLER_TEMPLATE: &str = include_str!("../../installer.nsi");
    const SHELL_EXTENSION_SOURCE: &str = include_str!("../../shell-extension/src/lib.rs");
    const IDENTITY_PACKAGE_MANIFEST: &str =
        include_str!("../../windows-context-menu/AppxManifest.xml.template");
    const IDENTITY_PACKAGE_BUILD_SCRIPT: &str =
        include_str!("../../../scripts/build-context-menu-package.ps1");
    const INSTALLED_RELEASE_TEST_SCRIPT: &str =
        include_str!("../../../scripts/test-installed-release.ps1");
    const PUBLIC_UPDATE_TEST_SCRIPT: &str = include_str!("../../../scripts/test-public-update.ps1");
    const CONTEXT_MENU_SOURCE: &str = include_str!("context_menu.rs");

    #[test]
    fn windows_11_uses_one_menu_provider_at_a_time() {
        assert!(CONTEXT_MENU_SOURCE.contains("unregister_legacy_menu(&hkcu)?;"));
        assert!(CONTEXT_MENU_SOURCE.contains("unregister_native_menu(&hkcu)?;"));
        assert!(CONTEXT_MENU_SOURCE.contains("register_native_menu(&hkcu, app_path)?;"));
        assert!(CONTEXT_MENU_SOURCE.contains("register_legacy_menu(&hkcu, app_path)?;"));
        assert!(CONTEXT_MENU_SOURCE.contains(
            "if !register_sparse_identity_package(app_path)?"
        ));
        assert!(CONTEXT_MENU_SOURCE.contains(
            "Windows 11 primary context-menu registration failed"
        ));
    }

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
    fn legacy_menu_uses_per_user_inline_subcommands() {
        let all_verbs = verbs();
        for selected in [COMPRESS_VERBS, ARCHIVE_VERBS, BACKGROUND_VERBS] {
            assert!(!selected.is_empty());
            assert!(selected.iter().all(|name| {
                all_verbs.iter().any(|candidate| candidate.verb == *name)
            }));
        }
        assert!(CONTEXT_MENU_SOURCE
            .matches("ExtendedSubCommandsKey\\shell")
            .count()
            >= 2);
        assert_eq!(
            CONTEXT_MENU_SOURCE
                .matches("let store_base = r\"Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\CommandStore")
                .count(),
            1
        );
        assert!(CONTEXT_MENU_SOURCE.contains(
            "root.get_value::<String, _>(\"SubCommands\").is_ok()"
        ));
    }

    #[test]
    fn legacy_menu_registry_roundtrip_contains_real_subcommands() {
        struct RegistryCleanup(String);
        impl Drop for RegistryCleanup {
            fn drop(&mut self) {
                let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(&self.0);
            }
        }

        let test_root = format!(
            r"Software\Classes\LongDecompress.ContextMenuTest.{}",
            std::process::id()
        );
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let _cleanup = RegistryCleanup(test_root.clone());
        let _ = hkcu.delete_subkey_all(&test_root);
        let menu_path = format!(r"{}\shell\LongDecompress", test_root);
        let app_path = r"C:\Program Files\LongDecompress\Long解压.exe";
        let all_verbs = verbs();

        reg_shell_entry(
            &hkcu,
            &menu_path,
            &all_verbs,
            ARCHIVE_VERBS,
            app_path,
        )
        .expect("register test cascade");

        assert!(legacy_shell_entry_is_current(
            &hkcu,
            &menu_path,
            &all_verbs,
            ARCHIVE_VERBS,
            app_path,
        ));
        let root = hkcu.open_subkey(&menu_path).expect("open cascade root");
        assert!(root.get_value::<String, _>("SubCommands").is_err());
        let submenu = hkcu
            .open_subkey(format!(r"{}\ExtendedSubCommandsKey\shell", menu_path))
            .expect("open inline submenu");
        assert_eq!(
            submenu.enum_keys().filter_map(Result::ok).count(),
            ARCHIVE_VERBS.len()
        );
    }

    #[test]
    fn installed_release_checks_validate_real_legacy_submenus() {
        for script in [INSTALLED_RELEASE_TEST_SCRIPT, PUBLIC_UPDATE_TEST_SCRIPT] {
            assert!(script.contains("Get-ClassicContextMenuCascadeStatus"));
            assert!(script.contains("ExtendedSubCommandsKey\\shell"));
            assert!(script.contains("obsolete SubCommands value"));
            assert!(script.contains("roots=4; commands=$commandCount"));
        }
        assert!(INSTALLED_RELEASE_TEST_SCRIPT.contains(
            "classic context-menu submenus are complete and target current executable"
        ));
        assert!(PUBLIC_UPDATE_TEST_SCRIPT
            .contains("updated release registers complete legacy submenus"));
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
        assert!(INSTALLER_TEMPLATE.contains(QUICK_EXTRACT_COMMAND_CLSID));
        assert!(INSTALLER_TEMPLATE.contains(QUICK_EXTRACT_COMMAND_VERB));
        assert!(INSTALLER_TEMPLATE.contains(QUICK_PACK_COMMAND_CLSID));
        assert!(INSTALLER_TEMPLATE.contains(QUICK_PACK_COMMAND_VERB));
        assert!(INSTALLER_TEMPLATE.contains("SHChangeNotify(i 0x08000000"));
        assert!(SHELL_EXTENSION_SOURCE.contains(NATIVE_COMMAND_CLSID));
    }

    #[test]
    fn nsis_installer_migrates_legacy_brand_identity() {
        assert!(INSTALLER_TEMPLATE.contains(
            r#"!define LEGACY_PRODUCTNAME "胧解压·方便助手""#
        ));
        assert!(INSTALLER_TEMPLATE.contains("LEGACY_MANUPRODUCTKEY"));
        assert!(INSTALLER_TEMPLATE.contains(
            r#"Delete "$INSTDIR\${LEGACY_PRODUCTNAME}.exe""#
        ));
        assert!(INSTALLER_TEMPLATE.contains(
            r#"DeleteRegKey SHCTX "${LEGACY_UNINSTKEY}""#
        ));
    }

    #[test]
    fn nsis_overlay_upgrade_removes_stale_context_menu_binaries() {
        assert!(INSTALLER_TEMPLATE.contains(
            r#"Delete "$INSTDIR\resources\long_compress_shell_extension_*.dll""#
        ));
        assert!(INSTALLER_TEMPLATE.contains(r#"-Action Uninstall' $0"#));
        for package in [
            "long_compress_context_menu_extract.msix",
            "long_compress_context_menu_pack.msix",
            "long_compress_context_menu.msix",
        ] {
            assert!(
                INSTALLER_TEMPLATE.contains(&format!(
                    r#"Delete "$INSTDIR\resources\{}""#,
                    package
                )),
                "overlay cleanup is missing {package}"
            );
        }
    }

    #[test]
    fn unsigned_native_menu_is_never_treated_as_current() {
        assert!(!native_package_registration_is_current(false, false));
        assert!(!native_package_registration_is_current(false, true));
        assert!(!native_package_registration_is_current(true, false));
        assert!(native_package_registration_is_current(true, true));
    }

    #[test]
    fn passive_updater_restarts_unless_validation_opts_out() {
        assert!(INSTALLER_TEMPLATE.contains(r#""/NR""#));
        assert!(INSTALLER_TEMPLATE.contains("restart_passive:"));
        assert!(INSTALLER_TEMPLATE.contains(
            r#"nsis_tauri_utils::RunAsUser "$INSTDIR\${MAINBINARYNAME}.exe" """#
        ));
    }

    #[test]
    fn native_menu_forwards_every_primary_shell_action() {
        for flag in [
            "--quick-extract",
            "--extract-here",
            "--extract-to",
            "--test-archive",
            "--quick-pack",
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

    #[test]
    fn identity_package_exposes_dedicated_primary_commands() {
        for clsid in [QUICK_EXTRACT_COMMAND_CLSID, QUICK_PACK_COMMAND_CLSID] {
            let without_braces = clsid.trim_matches(['{', '}']);
            assert!(IDENTITY_PACKAGE_BUILD_SCRIPT.contains(without_braces));
            assert!(SHELL_EXTENSION_SOURCE.contains(clsid));
        }
        assert!(IDENTITY_PACKAGE_BUILD_SCRIPT.contains("LongDecompressQuickExtract"));
        assert!(IDENTITY_PACKAGE_BUILD_SCRIPT.contains("LongDecompressQuickPack"));
        assert!(IDENTITY_PACKAGE_BUILD_SCRIPT.contains("LongCompressAssistant.ContextMenu.QuickExtract"));
        assert!(IDENTITY_PACKAGE_BUILD_SCRIPT.contains("LongCompressAssistant.ContextMenu.QuickPack"));
        assert_eq!(
            IDENTITY_PACKAGE_MANIFEST
                .lines()
                .filter(|line| line.trim() == "<Application")
                .count(),
            1
        );
        assert_eq!(IDENTITY_PACKAGE_MANIFEST.matches("__APP_EXECUTABLE__").count(), 1);
        assert!(IDENTITY_PACKAGE_MANIFEST.contains("__PACKAGE_NAME__"));
        assert!(IDENTITY_PACKAGE_MANIFEST.contains("__CONTEXT_MENU_ITEMS__"));
        assert!(IDENTITY_PACKAGE_MANIFEST.contains("windows.fileExplorerContextMenus"));
        assert!(IDENTITY_PACKAGE_MANIFEST.contains("ProcessorArchitecture=\"x64\""));
        assert!(IDENTITY_PACKAGE_BUILD_SCRIPT.contains("$stagedResourceDirectory"));
        assert!(IDENTITY_PACKAGE_BUILD_SCRIPT.contains("Copy-Item -LiteralPath $shellDll.FullName"));
        assert!(IDENTITY_PACKAGE_BUILD_SCRIPT.contains(".Replace('__APP_EXECUTABLE__'"));
        assert!(IDENTITY_PACKAGE_BUILD_SCRIPT.contains("long_compress_context_menu_extract.msix"));
        assert!(IDENTITY_PACKAGE_BUILD_SCRIPT.contains("long_compress_context_menu_pack.msix"));
        assert!(super::context_menu_package_version().ends_with(".0"));
    }
}
