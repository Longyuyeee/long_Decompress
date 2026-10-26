use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use lazy_static::lazy_static;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAssociation {
    pub extension: String,
    pub description: String,
    pub icon_path: Option<String>,
}

#[derive(Clone)]
pub struct FileAssociationManager {
    associations: Arc<RwLock<Vec<FileAssociation>>>,
}

impl Default for FileAssociationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FileAssociationManager {
    pub fn new() -> Self {
        Self {
            associations: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register(&self, association: FileAssociation) -> Result<()> {
        let mut associations = self.associations.write().await;
        associations.push(association);
        Ok(())
    }
}

pub struct GlobalFileAssociationManager {
    manager: Arc<RwLock<Option<FileAssociationManager>>>,
}

impl Default for GlobalFileAssociationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalFileAssociationManager {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(RwLock::new(Some(FileAssociationManager::new()))),
        }
    }

    pub async fn get(&self) -> Result<Arc<FileAssociationManager>, String> {
        let manager_guard = self.manager.read().await;
        manager_guard
            .as_ref()
            .map(|m| Arc::new(m.clone()))
            .ok_or_else(|| "文件关联管理器未初始化".to_string())
    }
}

lazy_static! {
    pub static ref FILE_ASSOCIATION_MANAGER: GlobalFileAssociationManager = GlobalFileAssociationManager::new();
}

/// Intentionally excludes documents, executables and disk images, even when
/// their container can be inspected by the archive reader.
pub const ARCHIVE_ASSOCIATION_EXTENSIONS: &[&str] = &[
    ".zip", ".7z", ".rar", ".zipx", ".tar", ".gz", ".bz2", ".xz", ".zst",
    ".tgz", ".tbz2", ".txz", ".lz", ".lzo", ".lz4", ".br", ".cab", ".arj", ".alz", ".egg", ".zpaq",
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveAssociationStatus {
    pub extension: String,
    pub registered: bool,
    pub is_default: bool,
}

pub fn validate_archive_associations(extensions: &[String]) -> std::result::Result<(), String> {
    if extensions.is_empty() || extensions.iter().any(|extension| !ARCHIVE_ASSOCIATION_EXTENSIONS.contains(&extension.as_str())) {
        return Err("请选择支持的压缩文件类型".into());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn association_status() -> std::result::Result<Vec<ArchiveAssociationStatus>, String> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "shlwapi")]
    extern "system" {
        fn AssocQueryStringW(flags: u32, query: u32, association: *const u16, extra: *const u16, out: *mut u16, length: *mut u32) -> i32;
    }
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    Ok(ARCHIVE_ASSOCIATION_EXTENSIONS.iter().map(|extension| {
        let association: Vec<u16> = std::ffi::OsStr::new(extension).encode_wide().chain(Some(0)).collect();
        let mut out = vec![0u16; 32768];
        let mut length = out.len() as u32;
        // ASSOCSTR_EXECUTABLE queries the effective user-selected handler.
        let found = unsafe { AssocQueryStringW(0, 2, association.as_ptr(), std::ptr::null(), out.as_mut_ptr(), &mut length) } == 0;
        let registered = hkcu.open_subkey(format!(r"Software\LongDecompress\Capabilities\FileAssociations"))
            .ok().and_then(|key| key.get_value::<String, _>(extension).ok()).is_some();
        ArchiveAssociationStatus {
            extension: (*extension).into(), registered,
            is_default: found && String::from_utf16_lossy(&out[..out.iter().position(|value| *value == 0).unwrap_or(out.len())]).eq_ignore_ascii_case(&exe.to_string_lossy()),
        }
    }).collect())
}

#[cfg(target_os = "windows")]
pub fn register_archive_opening(exe: &std::path::Path, extensions: &[String]) -> std::result::Result<(), String> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};
    validate_archive_associations(extensions)?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let command = format!("\"{}\" --open-archive \"%1\"", exe.display());
    let write = |path: &str, name: &str, value: &str| -> std::result::Result<(), String> {
        hkcu.create_subkey(path).and_then(|(key, _)| key.set_value(name, &value)).map_err(|error| error.to_string())
    };
    let capabilities = r"Software\LongDecompress\Capabilities";
    write(capabilities, "ApplicationName", "Long解压")?;
    write(capabilities, "ApplicationDescription", "浏览、解压和管理压缩文件")?;
    write(capabilities, "ApplicationIcon", &format!("\"{}\",0", exe.display()))?;
    write(r"Software\RegisteredApplications", "LongDecompress", capabilities)?;
    let application = format!(r"Software\Classes\Applications\{}", exe.file_name().unwrap_or_default().to_string_lossy());
    write(&application, "FriendlyAppName", "Long解压")?;
    write(&format!(r"{application}\shell\open\command"), "", &command)?;
    for extension in extensions {
        let prog_id = format!("LongDecompress.Archive.{}", &extension[1..]);
        let class = format!(r"Software\Classes\{prog_id}");
        write(&class, "", &format!("{} 压缩文件", extension[1..].to_uppercase()))?;
        write(&format!(r"{class}\DefaultIcon"), "", &format!("\"{}\",0", exe.display()))?;
        write(&format!(r"{class}\shell\open\command"), "", &command)?;
        write(&format!(r"{capabilities}\FileAssociations"), extension, &prog_id)?;
        write(&format!(r"{application}\SupportedTypes"), extension, "")?;
        // REG_NONE OpenWithProgids advertises a candidate; UserChoice and the
        // existing default value are deliberately untouched.
        let (open_with, _) = hkcu.create_subkey(format!(r"Software\Classes\{extension}\OpenWithProgids")).map_err(|error| error.to_string())?;
        open_with.set_raw_value(&prog_id, &winreg::RegValue { vtype: winreg::enums::REG_NONE, bytes: vec![] }).map_err(|error| error.to_string())?;
    }
    // Update the offered formats exactly. Keep old ProgID commands valid for
    // any existing Windows UserChoice until the user changes that choice.
    for extension in ARCHIVE_ASSOCIATION_EXTENSIONS.iter().filter(|extension| !extensions.iter().any(|chosen| chosen == **extension)) {
        let prog_id=format!("LongDecompress.Archive.{}", &extension[1..]);
        for (path,name) in [
            (format!(r"{capabilities}\FileAssociations"),(*extension).to_string()),
            (format!(r"{application}\SupportedTypes"),(*extension).to_string()),
            (format!(r"Software\Classes\{extension}\OpenWithProgids"),prog_id),
        ] {
            if let Ok(key)=hkcu.open_subkey_with_flags(path,winreg::enums::KEY_SET_VALUE) {
                match key.delete_value(name) {Ok(())=>(),Err(error) if error.kind()==std::io::ErrorKind::NotFound=>(),Err(error)=>return Err(error.to_string())}
            }
        }
    }
    #[link(name = "shell32")]
    extern "system" { fn SHChangeNotify(event: i32, flags: u32, item1: *const std::ffi::c_void, item2: *const std::ffi::c_void); }
    unsafe { SHChangeNotify(0x08000000, 0, std::ptr::null(), std::ptr::null()) };
    Ok(())
}

/// Refresh only candidates the user previously configured, e.g. after moving
/// an installation. This never chooses a default or registers new formats.
#[cfg(target_os = "windows")]
pub fn refresh_archive_opening_if_present(exe: &std::path::Path) -> std::result::Result<(), String> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};
    let Ok(key) = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\LongDecompress\Capabilities\FileAssociations") else { return Ok(()) };
    let extensions: Vec<String> = ARCHIVE_ASSOCIATION_EXTENSIONS.iter()
        .filter(|extension| key.get_value::<String, _>(**extension).is_ok())
        .map(|extension| (*extension).to_string()).collect();
    if extensions.is_empty() { return Ok(()); }
    register_archive_opening(exe, &extensions)
}

#[cfg(not(target_os = "windows"))]
pub fn association_status() -> std::result::Result<Vec<ArchiveAssociationStatus>, String> { Ok(vec![]) }

#[cfg(not(target_os = "windows"))]
pub fn register_archive_opening(_: &std::path::Path, _: &[String]) -> std::result::Result<(), String> { Err("仅 Windows 支持文件关联设置".into()) }

#[cfg(test)]
mod archive_opening_tests {
    use super::*;
    #[test]
    fn only_archive_candidates_can_be_registered() {
        assert!(validate_archive_associations(&[".zip".into(),".7z".into(),".rar".into()]).is_ok());
        for extension in [".exe",".docx",".xlsx",".pdf",".txt",r".zip\shell"] {
            assert!(validate_archive_associations(&[extension.into()]).is_err());
        }
        assert!(validate_archive_associations(&[]).is_err());
    }
}
