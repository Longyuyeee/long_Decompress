#![cfg_attr(not(test), windows_subsystem = "windows")]
#![allow(non_snake_case)]

use std::ffi::c_void;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use windows::core::{implement, Error, Interface, Ref, Result, GUID, HRESULT, PWSTR};
use windows::Win32::Foundation::{
    CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, E_FAIL, E_NOTIMPL, E_POINTER, S_FALSE, S_OK,
};
use windows::Win32::System::Com::{CoTaskMemFree, IBindCtx, IClassFactory, IClassFactory_Impl};
use windows::Win32::UI::Shell::{
    IEnumExplorerCommand, IEnumExplorerCommand_Impl, IExplorerCommand, IExplorerCommand_Impl,
    IShellItemArray, SHStrDupW, ECF_DEFAULT, ECF_HASSUBCOMMANDS, ECF_SEPARATORBEFORE, ECS_DISABLED, ECS_ENABLED,
    ECS_HIDDEN, SIGDN_FILESYSPATH,
};
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

// Explorer loads this small COM DLL, never the Tauri application or its runtime.
pub const EXPLORER_COMMAND_CLSID: &str = "{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4A}";
const EXPLORER_COMMAND_GUID: GUID = GUID::from_u128(0xd4bba0b2_6a58_4d40_8b79_ba50c54e8d4a);
pub const QUICK_EXTRACT_COMMAND_CLSID: &str = "{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4B}";
const QUICK_EXTRACT_COMMAND_GUID: GUID = GUID::from_u128(0xd4bba0b2_6a58_4d40_8b79_ba50c54e8d4b);
pub const QUICK_PACK_COMMAND_CLSID: &str = "{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4C}";
const QUICK_PACK_COMMAND_GUID: GUID = GUID::from_u128(0xd4bba0b2_6a58_4d40_8b79_ba50c54e8d4c);
const CLASSIC_COMMAND_GUID: GUID = GUID::from_u128(0xd4bba0b2_6a58_4d40_8b79_ba50c54e8d4d);
const CLASSIC_COMMAND_CLSID: &str = "{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4D}";
const REGISTRATION_KEY: &str = r"Software\Classes\CLSID\{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4A}";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const ARCHIVE_EXTENSIONS: &[&str] = &[
    "alz", "egg", "zpaq",
    "zip", "zipx", "7z", "rar", "wim", "swm", "tar", "ova", "gz", "gzip", "tgz", "tpz", "bz2", "bzip2",
    "tbz", "tbz2", "xz", "txz", "zst", "zstd", "tzst", "aes", "lzma", "lzo", "lz4", "br", "lz", "tlz", "jar", "xpi", "ipa", "apk",
    "appx", "iso", "img", "dmg", "vhd", "vhdx", "qcow", "qcow2", "qcow2c", "vdi", "vmdk", "cab",
    "deb", "udeb", "rpm", "msi", "msp", "msm", "nsis", "apfs", "ext", "ext2", "ext3", "ext4",
    "gpt", "mbr", "uefif", "cramfs", "fat", "ntfs", "hfs", "hfsx", "ar", "a", "lzh", "lha",
    "squashfs", "sfs", "xar", "cpio", "ihex", "arj", "z", "udf",
];

static OBJECT_COUNT: AtomicU32 = AtomicU32::new(0);
#[cfg(test)]
static TEST_APPLICATION_PATH: Mutex<Option<String>> = Mutex::new(None);

static SERVER_LOCK_COUNT: AtomicU32 = AtomicU32::new(0);

struct ObjectLifetime;

impl ObjectLifetime {
    fn new() -> Self {
        OBJECT_COUNT.fetch_add(1, Ordering::SeqCst);
        Self
    }
}

impl Drop for ObjectLifetime {
    fn drop(&mut self) {
        OBJECT_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CommandKind {
    Root,
    ClassicRoot,
    CompressMenu,
    BrowseArchive,
    QuickExtract,
    ExtractHere,
    ExtractTo,
    ExtractChoose,
    MoreArchive,
    RepackArchiveFiles,
    TestArchive,
    RepairArchive,
    ConvertArchive,
    ReconfigureArchive,
    QuickPack,
    CompressZip,
    Compress7z,
    CompressCustom,
    CompressEncrypted,
    CompressSplit,
    ImageCompress,
    ImageWebp,
    ImageResize,
    ChecksumCreate,
    ChecksumVerify,
}

const SUBCOMMANDS: &[CommandKind] = &[
    CommandKind::BrowseArchive,
    CommandKind::ExtractHere,
    CommandKind::ExtractTo,
    CommandKind::ExtractChoose,
    CommandKind::ImageCompress,
    CommandKind::ImageWebp,
    CommandKind::ImageResize,
    CommandKind::CompressZip,
    CommandKind::Compress7z,
    CommandKind::CompressCustom,
    CommandKind::CompressEncrypted,
    CommandKind::CompressSplit,
    // The Windows 11 flyout is deliberately one level deep. Its host can omit
    // nested cascades; classic Explorer uses the categorized registry submenus.
    CommandKind::TestArchive,
    CommandKind::RepairArchive,
    CommandKind::ConvertArchive,
    CommandKind::ReconfigureArchive,
    CommandKind::ChecksumVerify,
    CommandKind::ChecksumCreate,
];
const MORE_ARCHIVE_COMMANDS: &[CommandKind] = &[
    CommandKind::TestArchive,
    CommandKind::RepairArchive,
    CommandKind::ConvertArchive,
    CommandKind::ReconfigureArchive,
];
const PACK_COMMANDS: &[CommandKind] = &[
    CommandKind::CompressZip, CommandKind::Compress7z, CommandKind::CompressCustom,
    CommandKind::CompressEncrypted, CommandKind::CompressSplit,
];
const CLASSIC_GENERAL: &[CommandKind] = &[
    CommandKind::CompressZip, CommandKind::Compress7z, CommandKind::CompressCustom,
    CommandKind::CompressEncrypted, CommandKind::CompressSplit, CommandKind::ChecksumCreate,
];
const CLASSIC_ARCHIVE: &[CommandKind] = &[
    CommandKind::BrowseArchive, CommandKind::ExtractHere, CommandKind::ExtractTo, CommandKind::ExtractChoose,
    CommandKind::CompressMenu, CommandKind::MoreArchive, CommandKind::ChecksumCreate,
];
const CLASSIC_IMAGE: &[CommandKind] = &[
    CommandKind::ImageCompress, CommandKind::ImageWebp, CommandKind::ImageResize,
    CommandKind::CompressMenu, CommandKind::ChecksumCreate,
];
const CLASSIC_MANIFEST: &[CommandKind] = &[
    CommandKind::ChecksumVerify, CommandKind::CompressMenu, CommandKind::ChecksumCreate,
];
const CLASSIC_REPAIR: &[CommandKind] = &[
    CommandKind::MoreArchive, CommandKind::CompressMenu, CommandKind::ChecksumCreate,
];

impl CommandKind {
    fn accepts(self, paths: &[String]) -> bool {
        if paths.is_empty() { return false; }
        match self {
            Self::QuickExtract => false,
            Self::BrowseArchive => paths.len() == 1 && is_archive_path(&paths[0]),
            Self::ImageCompress | Self::ImageWebp | Self::ImageResize => paths.iter().all(|p| is_image_path(p)),
            Self::ChecksumCreate => paths.iter().all(|p| !std::path::Path::new(p).is_dir()),
            Self::ChecksumVerify => paths.iter().all(|p| is_checksum_manifest(p)),
            Self::RepairArchive | Self::MoreArchive => paths.iter().all(|p| is_repair_path(p)),
            _ if self.requires_archive() => paths.iter().all(|p| is_archive_path(p)),
            _ => true,
        }
    }
    fn title(self) -> &'static str {
        match self {
            Self::Root | Self::ClassicRoot => "Long解压",
            Self::CompressMenu => "压缩打包",
            Self::BrowseArchive => "浏览压缩包内容",
            Self::QuickExtract => "一键解压到同名文件夹",
            Self::ExtractHere => "解压到此处",
            Self::ExtractTo => "解压到同名文件夹",
            Self::ExtractChoose => "选择解压文件夹…",
            Self::MoreArchive => "更多操作",
            Self::RepackArchiveFiles => "将压缩包作为文件重新打包…",
            Self::TestArchive => "测试压缩包完整性",
            Self::RepairArchive => "检查与尝试修复…",
            Self::ConvertArchive => "转换压缩包格式…",
            Self::ReconfigureArchive => "修改密码 / 调整分卷…",
            Self::QuickPack => "一键打包为 ZIP",
            Self::CompressZip => "压缩为 ZIP",
            Self::Compress7z => "压缩为 7Z",
            Self::CompressCustom => "添加到压缩包…",
            Self::CompressEncrypted => "加密压缩…",
            Self::CompressSplit => "分卷压缩…",
            Self::ImageCompress => "压缩图片…",
            Self::ImageWebp => "转换为 WebP…",
            Self::ImageResize => "缩小图片尺寸…",
            Self::ChecksumCreate => "生成校验清单…",
            Self::ChecksumVerify => "核对校验清单…",
        }
    }

    fn cli_flag(self) -> Option<&'static str> {
        match self {
            Self::Root | Self::ClassicRoot | Self::CompressMenu | Self::MoreArchive => None,
            Self::BrowseArchive => Some("--browse-archive"),
            Self::QuickExtract => Some("--quick-extract"),
            Self::ExtractHere => Some("--extract-here"),
            Self::ExtractTo => Some("--extract-to"),
            Self::ExtractChoose => Some("--extract-choose"),
            Self::RepackArchiveFiles => Some("--compress-custom"),
            Self::TestArchive => Some("--test-archive"),
            Self::RepairArchive => Some("--repair-archive"),
            Self::ConvertArchive => Some("--convert-archive"),
            Self::ReconfigureArchive => Some("--reconfigure-archive"),
            Self::QuickPack => Some("--quick-pack"),
            Self::CompressZip => Some("--compress-zip"),
            Self::Compress7z => Some("--compress-7z"),
            Self::CompressCustom => Some("--compress-custom"),
            Self::CompressEncrypted => Some("--compress-encrypted"),
            Self::CompressSplit => Some("--compress-split"),
            Self::ImageCompress => Some("--image-compress"),
            Self::ImageWebp => Some("--image-webp"),
            Self::ImageResize => Some("--image-resize"),
            Self::ChecksumCreate => Some("--checksum-create"),
            Self::ChecksumVerify => Some("--checksum-verify"),
        }
    }

    fn requires_archive(self) -> bool {
        matches!(
            self,
            Self::BrowseArchive
                | Self::QuickExtract
                | Self::ExtractHere
                | Self::ExtractTo
                | Self::ExtractChoose
                | Self::RepackArchiveFiles
                | Self::TestArchive
                | Self::ConvertArchive
                | Self::ReconfigureArchive
        )
    }

    fn canonical_guid(self) -> GUID {
        let suffix = match self {
            Self::Root => 0,
            Self::ClassicRoot => 33,
            Self::CompressMenu => 34,
            Self::BrowseArchive => 9,
            Self::QuickExtract => 1,
            Self::ExtractHere => 2,
            Self::ExtractTo => 3,
            Self::ExtractChoose => 23,
            Self::MoreArchive => 24,
            Self::RepackArchiveFiles => 25,
            Self::TestArchive => 4,
            Self::RepairArchive => 22,
            Self::ConvertArchive => 20,
            Self::ReconfigureArchive => 21,
            Self::QuickPack => 5,
            Self::CompressZip => 6,
            Self::Compress7z => 7,
            Self::CompressCustom => 8,
            Self::CompressEncrypted => 26,
            Self::CompressSplit => 27,
            Self::ImageCompress => 28,
            Self::ImageWebp => 29,
            Self::ImageResize => 30,
            Self::ChecksumCreate => 31,
            Self::ChecksumVerify => 32,
        };
        GUID::from_u128(0xd4bba0b2_6a58_4d40_8b79_ba50c54e8d40 + suffix)
    }
}

#[implement(IExplorerCommand)]
struct ShellCommand {
    kind: CommandKind,
    classic: bool,
    selection: Mutex<Vec<String>>,
    _lifetime: ObjectLifetime,
}

impl ShellCommand {
    fn new(kind: CommandKind) -> Self {
        Self::for_provider(kind, kind == CommandKind::ClassicRoot)
    }
    fn for_provider(kind: CommandKind, classic: bool) -> Self {
        Self {
            kind,
            classic,
            selection: Mutex::new(Vec::new()),
            _lifetime: ObjectLifetime::new(),
        }
    }
}

impl IExplorerCommand_Impl for ShellCommand_Impl {
    fn GetTitle(&self, items: Ref<IShellItemArray>) -> Result<PWSTR> {
        if self.kind == CommandKind::ClassicRoot {
            if let Some(items) = items.as_ref() {
                *self.selection.lock().map_err(|_| Error::from(E_FAIL))? = selected_paths(items).unwrap_or_default();
            }
        }
        duplicate_string(self.kind.title())
    }

    fn GetIcon(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
        let executable = read_application_path(self.classic)?;
        duplicate_string(&format!("{executable},0"))
    }

    fn GetToolTip(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
        duplicate_string(self.kind.title())
    }

    fn GetCanonicalName(&self) -> Result<GUID> {
        Ok(self.kind.canonical_guid())
    }

    fn GetState(
        &self,
        items: Ref<IShellItemArray>,
        _ok_to_be_slow: windows::core::BOOL,
    ) -> Result<u32> {
        // Retain the old COM class for package compatibility, without exposing
        // a second copy of the same extraction command in Explorer.
        if self.kind == CommandKind::QuickExtract { return Ok(ECS_HIDDEN.0 as u32); }
        let Some(selection) = items.as_ref() else {
            return Ok(ECS_DISABLED.0 as u32);
        };
        let count = unsafe { selection.GetCount().unwrap_or(0) };
        if count == 0 {
            return Ok(ECS_DISABLED.0 as u32);
        }
        let paths = selected_paths(selection).unwrap_or_default();
        if self.kind == CommandKind::ClassicRoot {
            *self.selection.lock().map_err(|_| Error::from(E_FAIL))? = paths.clone();
        }
        if !self.kind.accepts(&paths) { return Ok(ECS_HIDDEN.0 as u32); }
        Ok(ECS_ENABLED.0 as u32)
    }

    fn Invoke(&self, items: Ref<IShellItemArray>, _bind_context: Ref<IBindCtx>) -> Result<()> {
        let flag = self.kind.cli_flag().ok_or_else(|| Error::from(E_NOTIMPL))?;
        let paths = selected_paths(items.ok()?)?;
        if !self.kind.accepts(&paths) {
            return Err(Error::from(E_FAIL));
        }

        Command::new(read_application_path(self.classic)?)
            .arg(flag)
            .args(paths)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|_| Error::from(E_FAIL))?;
        Ok(())
    }

    fn GetFlags(&self) -> Result<u32> {
        Ok(if matches!(self.kind, CommandKind::Root | CommandKind::ClassicRoot | CommandKind::CompressMenu | CommandKind::MoreArchive) {
            ECF_HASSUBCOMMANDS.0 as u32
        } else if !self.classic && matches!(self.kind, CommandKind::CompressZip | CommandKind::TestArchive | CommandKind::ChecksumVerify | CommandKind::ChecksumCreate) {
            ECF_SEPARATORBEFORE.0 as u32
        } else {
            ECF_DEFAULT.0 as u32
        })
    }

    fn EnumSubCommands(&self) -> Result<IEnumExplorerCommand> {
        match self.kind {
            CommandKind::Root => Ok(CommandEnumerator::new(0).into()),
            CommandKind::ClassicRoot => {
                let paths = self.selection.lock().map_err(|_| Error::from(E_FAIL))?;
                let commands = if paths.is_empty() { SUBCOMMANDS }
                    else if paths.iter().all(|p| is_archive_path(p)) { CLASSIC_ARCHIVE }
                    else if paths.iter().all(|p| is_image_path(p)) { CLASSIC_IMAGE }
                    else if paths.iter().all(|p| is_checksum_manifest(p)) { CLASSIC_MANIFEST }
                    else if paths.iter().all(|p| is_repair_path(p)) { CLASSIC_REPAIR }
                    else { CLASSIC_GENERAL };
                Ok(CommandEnumerator::for_provider(0, commands, true).into())
            }
            CommandKind::CompressMenu => Ok(CommandEnumerator::for_provider(0, PACK_COMMANDS, self.classic).into()),
            CommandKind::MoreArchive => Ok(CommandEnumerator::for_provider(0, MORE_ARCHIVE_COMMANDS, self.classic).into()),
            _ => Err(Error::from(E_NOTIMPL)),
        }
    }
}

#[implement(IEnumExplorerCommand)]
struct CommandEnumerator {
    position: Mutex<usize>,
    commands: &'static [CommandKind],
    classic: bool,
    _lifetime: ObjectLifetime,
}

impl CommandEnumerator {
    fn new(position: usize) -> Self {
        Self::with_commands(position, SUBCOMMANDS)
    }
    fn with_commands(position: usize, commands: &'static [CommandKind]) -> Self {
        Self::for_provider(position, commands, false)
    }
    fn for_provider(position: usize, commands: &'static [CommandKind], classic: bool) -> Self {
        Self {
            position: Mutex::new(position.min(commands.len())),
            commands,
            classic,
            _lifetime: ObjectLifetime::new(),
        }
    }
}

impl IEnumExplorerCommand_Impl for CommandEnumerator_Impl {
    fn Next(
        &self,
        count: u32,
        commands: *mut Option<IExplorerCommand>,
        fetched: *mut u32,
    ) -> HRESULT {
        if (count > 0 && commands.is_null()) || (count != 1 && fetched.is_null()) {
            return E_POINTER;
        }
        let Ok(mut position) = self.position.lock() else {
            return E_FAIL;
        };

        let mut produced = 0u32;
        while produced < count && *position < self.commands.len() {
            let command: IExplorerCommand = ShellCommand::for_provider(self.commands[*position], self.classic).into();
            unsafe { commands.add(produced as usize).write(Some(command)) };
            *position += 1;
            produced += 1;
        }
        if !fetched.is_null() {
            unsafe { fetched.write(produced) };
        }
        if produced == count {
            S_OK
        } else {
            S_FALSE
        }
    }

    fn Skip(&self, count: u32) -> Result<()> {
        let mut position = self.position.lock().map_err(|_| Error::from(E_FAIL))?;
        *position = position
            .saturating_add(count as usize)
            .min(self.commands.len());
        Ok(())
    }

    fn Reset(&self) -> Result<()> {
        *self.position.lock().map_err(|_| Error::from(E_FAIL))? = 0;
        Ok(())
    }

    fn Clone(&self) -> Result<IEnumExplorerCommand> {
        let position = *self.position.lock().map_err(|_| Error::from(E_FAIL))?;
        Ok(CommandEnumerator::for_provider(position, self.commands, self.classic).into())
    }
}

fn duplicate_string(value: &str) -> Result<PWSTR> {
    let value = windows::core::HSTRING::from(value);
    unsafe { SHStrDupW(&value) }
}

fn selected_paths(items: &IShellItemArray) -> Result<Vec<String>> {
    let count = unsafe { items.GetCount()? };
    let mut paths = Vec::with_capacity(count as usize);
    for index in 0..count {
        let item = unsafe { items.GetItemAt(index)? };
        let path = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH)? };
        let converted = unsafe { path.to_string() };
        unsafe { CoTaskMemFree(Some(path.0.cast())) };
        paths.push(converted?);
    }
    Ok(paths)
}

fn is_repair_path(path: &str) -> bool {
    if std::path::Path::new(path).is_dir() { return false; }
    if is_archive_path(path) { return true; }
    let extension = std::path::Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    extension == "rev" || extension.strip_prefix('r').is_some_and(|n| (2..=3).contains(&n.len()) && n.bytes().all(|b| b.is_ascii_digit()))
        || extension.strip_prefix('z').is_some_and(|n| n.len() >= 2 && n.bytes().all(|b| b.is_ascii_digit()))
        || (extension.len() >= 3 && extension.bytes().all(|b| b.is_ascii_digit()))
}

fn is_image_path(path: &str) -> bool {
    !std::path::Path::new(path).is_dir() && std::path::Path::new(path).extension().and_then(|e| e.to_str())
        .is_some_and(|e| ["jpg", "jpeg", "png", "webp"].iter().any(|known| e.eq_ignore_ascii_case(known)))
}

fn is_checksum_manifest(path: &str) -> bool {
    !std::path::Path::new(path).is_dir() && std::path::Path::new(path).extension().and_then(|e| e.to_str())
        .is_some_and(|e| ["sfv", "md5", "sha256"].iter().any(|known| e.eq_ignore_ascii_case(known)))
}
fn is_archive_path(path: &str) -> bool {
    if std::path::Path::new(path).is_dir() { return false; }
    std::path::Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            let ext = extension.to_ascii_lowercase();
            let numbered = ext.len() >= 3 && ext.bytes().all(|b| b.is_ascii_digit());
            let legacy = ext.strip_prefix('z').or_else(||ext.strip_prefix('r')).is_some_and(|n| n.len() >= 2 && n.bytes().all(|b|b.is_ascii_digit()));
            numbered || legacy || ARCHIVE_EXTENSIONS
                .iter()
                .any(|known| extension.eq_ignore_ascii_case(known))
        })
}

fn read_application_path(classic: bool) -> Result<String> {
    #[cfg(test)] if let Some(path) = TEST_APPLICATION_PATH.lock().unwrap().as_ref() { return Ok(path.clone()); }
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(if classic {format!(r"Software\Classes\CLSID\{}", CLASSIC_COMMAND_CLSID)} else {REGISTRATION_KEY.to_owned()})
        .and_then(|key| key.get_value("ApplicationPath"))
        .map_err(|_| Error::from(E_FAIL))
}

#[implement(IClassFactory)]
struct ClassFactory {
    kind: CommandKind,
    _lifetime: ObjectLifetime,
}

impl ClassFactory {
    fn new(kind: CommandKind) -> Self {
        Self {
            kind,
            _lifetime: ObjectLifetime::new(),
        }
    }
}

impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<windows::core::IUnknown>,
        iid: *const GUID,
        object: *mut *mut c_void,
    ) -> Result<()> {
        if !outer.is_null() {
            return Err(Error::from(CLASS_E_NOAGGREGATION));
        }
        if iid.is_null() || object.is_null() {
            return Err(Error::from(E_POINTER));
        }

        let command: IExplorerCommand = ShellCommand::new(self.kind).into();
        unsafe { command.query(iid, object) }.ok()
    }

    fn LockServer(&self, lock: windows::core::BOOL) -> Result<()> {
        if lock.as_bool() {
            SERVER_LOCK_COUNT.fetch_add(1, Ordering::SeqCst);
        } else {
            let _ = SERVER_LOCK_COUNT.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                count.checked_sub(1)
            });
        }
        Ok(())
    }
}

#[no_mangle]
/// Returns the COM class factory used by Windows Explorer.
///
/// # Safety
///
/// `class_id` and `iid` must point to valid GUIDs and `object` must point to a
/// writable COM interface pointer, as required by the `DllGetClassObject` ABI.
pub unsafe extern "system" fn DllGetClassObject(
    class_id: *const GUID,
    iid: *const GUID,
    object: *mut *mut c_void,
) -> HRESULT {
    if class_id.is_null() || iid.is_null() || object.is_null() {
        return E_POINTER;
    }
    let kind = match unsafe { *class_id } {
        EXPLORER_COMMAND_GUID => CommandKind::Root,
        CLASSIC_COMMAND_GUID => CommandKind::ClassicRoot,
        QUICK_EXTRACT_COMMAND_GUID => CommandKind::QuickExtract,
        QUICK_PACK_COMMAND_GUID => CommandKind::QuickPack,
        _ => return CLASS_E_CLASSNOTAVAILABLE,
    };

    let factory: IClassFactory = ClassFactory::new(kind).into();
    unsafe { factory.query(iid, object) }
}

#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    if OBJECT_COUNT.load(Ordering::SeqCst) == 0 && SERVER_LOCK_COUNT.load(Ordering::SeqCst) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

#[cfg(test)]
mod tests {
    use super::MORE_ARCHIVE_COMMANDS;
    use std::ffi::c_void;
    use std::sync::Mutex;

    use windows::core::{IUnknown, Interface, GUID};
    use windows::Win32::Foundation::{CLASS_E_CLASSNOTAVAILABLE, S_FALSE, S_OK};
    use windows::Win32::System::Com::IClassFactory;
    use windows::Win32::UI::Shell::{IExplorerCommand, ECF_DEFAULT, ECF_HASSUBCOMMANDS};

    use super::{
        is_archive_path, is_repair_path, CommandKind, DllCanUnloadNow, DllGetClassObject, EXPLORER_COMMAND_CLSID,
        EXPLORER_COMMAND_GUID, QUICK_EXTRACT_COMMAND_CLSID, QUICK_EXTRACT_COMMAND_GUID,
        QUICK_PACK_COMMAND_CLSID, QUICK_PACK_COMMAND_GUID, SUBCOMMANDS,
    };

    static COM_LIFETIME_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn explorer_command_clsid_is_stable() {
        assert_eq!(
            EXPLORER_COMMAND_CLSID,
            "{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4A}"
        );
        assert_eq!(
            QUICK_EXTRACT_COMMAND_CLSID,
            "{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4B}"
        );
        assert_eq!(
            QUICK_PACK_COMMAND_CLSID,
            "{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4C}"
        );
    }

    #[test]
    fn native_menu_contains_expected_commands() {
        assert_eq!(SUBCOMMANDS.len(), 18);
        assert_eq!(SUBCOMMANDS[0], CommandKind::BrowseArchive);
        assert!(SUBCOMMANDS.contains(&CommandKind::ExtractChoose));
        assert!(SUBCOMMANDS.contains(&CommandKind::ChecksumVerify));
        assert!(SUBCOMMANDS.contains(&CommandKind::ChecksumCreate));
        assert!(!SUBCOMMANDS.contains(&CommandKind::MoreArchive));
        assert!(!SUBCOMMANDS.contains(&CommandKind::QuickExtract));
        assert!(MORE_ARCHIVE_COMMANDS.contains(&CommandKind::ConvertArchive));
        assert!(MORE_ARCHIVE_COMMANDS.contains(&CommandKind::ReconfigureArchive));
        assert!(is_archive_path(r"C:\archives\sample.ZIP"));
        assert!(!is_archive_path(r"C:\documents\sample.txt"));
        assert!(MORE_ARCHIVE_COMMANDS.contains(&CommandKind::RepairArchive));
        assert!(is_repair_path(r"C:\archives\parts.part1.rev"));
        assert!(is_repair_path(r"C:\archives\parts.r00"));
        assert!(!is_repair_path(r"C:\documents\sample.txt"));
    }

    #[test]
    fn com_factory_creates_root_and_enumerates_subcommands() {
        let _lifetime_guard = COM_LIFETIME_TEST_LOCK
            .lock()
            .expect("COM lifetime test lock should not be poisoned");
        unsafe {
            assert_eq!(DllCanUnloadNow(), S_OK);
            let mut factory_pointer = std::ptr::null_mut::<c_void>();
            let result = DllGetClassObject(
                &EXPLORER_COMMAND_GUID,
                &IClassFactory::IID,
                &mut factory_pointer,
            );
            assert_eq!(result, S_OK);

            let factory = IClassFactory::from_raw(factory_pointer);
            let root: IExplorerCommand = factory
                .CreateInstance(None::<&IUnknown>)
                .expect("class factory should create the root command");
            assert_eq!(root.GetFlags().unwrap(), ECF_HASSUBCOMMANDS.0 as u32);

            let enumerator = root.EnumSubCommands().unwrap();
            let mut commands: [Option<IExplorerCommand>; SUBCOMMANDS.len()] = std::array::from_fn(|_| None);
            let mut fetched = 0;
            assert_eq!(enumerator.Next(&mut commands, Some(&mut fetched)), S_OK);
            assert_eq!(fetched as usize, SUBCOMMANDS.len());
            assert!(commands.iter().all(Option::is_some));
            // Every offered native child is a leaf; no maintenance action can
            // disappear in a nested flyout that Windows 11 does not materialize.
            for child in commands.iter().flatten() {
                assert_eq!(child.GetFlags().unwrap() & ECF_HASSUBCOMMANDS.0 as u32, 0);
            }
            assert_eq!(DllCanUnloadNow(), S_FALSE);

            drop(commands);
            drop(enumerator);
            drop(root);
            drop(factory);
            assert_eq!(DllCanUnloadNow(), S_OK);
        }
    }

    #[test]
    fn dedicated_factories_create_top_level_commands() {
        let _lifetime_guard = COM_LIFETIME_TEST_LOCK
            .lock()
            .expect("COM lifetime test lock should not be poisoned");
        unsafe {
            for (class_id, expected_flags) in [
                (QUICK_EXTRACT_COMMAND_GUID, ECF_DEFAULT.0 as u32),
                (QUICK_PACK_COMMAND_GUID, ECF_DEFAULT.0 as u32),
            ] {
                let mut factory_pointer = std::ptr::null_mut::<c_void>();
                assert_eq!(
                    DllGetClassObject(&class_id, &IClassFactory::IID, &mut factory_pointer),
                    S_OK
                );
                let factory = IClassFactory::from_raw(factory_pointer);
                let command: IExplorerCommand = factory
                    .CreateInstance(None::<&IUnknown>)
                    .expect("dedicated class factory should create a command");
                assert_eq!(command.GetFlags().unwrap(), expected_flags);
            }
        }
    }

    #[test]
    fn com_factory_rejects_unknown_class() {
        unsafe {
            let mut factory_pointer = std::ptr::null_mut::<c_void>();
            let result =
                DllGetClassObject(&GUID::zeroed(), &IClassFactory::IID, &mut factory_pointer);
            assert_eq!(result, CLASS_E_CLASSNOTAVAILABLE);
            assert!(factory_pointer.is_null());
        }
    }
    #[test]
    fn explorer1740_real_shell_selections_and_process_arguments() {
        use windows::Win32::UI::Shell::{SHParseDisplayName, SHCreateShellItemArrayFromIDLists, ECS_ENABLED, ECS_HIDDEN};
        use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, CoTaskMemFree, COINIT_APARTMENTTHREADED};
        use windows::core::HSTRING;
        use std::{fs, path::PathBuf, time::{SystemTime,UNIX_EPOCH}};
        let _guard=COM_LIFETIME_TEST_LOCK.lock().unwrap();
        let root=std::env::temp_dir().join(format!("long-menu1740-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir(&root).unwrap();
        struct Cleanup(PathBuf);impl Drop for Cleanup {fn drop(&mut self){*super::TEST_APPLICATION_PATH.lock().unwrap()=None;let _=std::fs::remove_dir_all(&self.0);unsafe{CoUninitialize()}}}
        unsafe{CoInitializeEx(None,COINIT_APARTMENTTHREADED).ok().unwrap()};let _cleanup=Cleanup(root.clone());
        for name in ["中文 a.txt","图 片.PNG","另一张.webp","photo.jpg","photo.jpeg","压缩 包.ZIP","sample.7z","sample.rar","next.zip.003","parts.z02","修复.rev","check.sha256","list.md5","list.sfv"]{fs::write(root.join(name),b"selection").unwrap();}fs::create_dir(root.join("folder.zip")).unwrap();
        let selection=|names:&[&str]| unsafe{
            let pidls=names.iter().map(|name|{let mut pidl=std::ptr::null_mut();SHParseDisplayName(&HSTRING::from(root.join(name).to_string_lossy().as_ref()),None,&mut pidl,0,None).unwrap();pidl}).collect::<Vec<_>>();
            let array=SHCreateShellItemArrayFromIDLists(&pidls.iter().map(|p|*p as *const _).collect::<Vec<_>>()).unwrap();
            for pidl in pidls{CoTaskMemFree(Some(pidl.cast()))}array
        };
        for (names,archive,image,repair) in [
            (vec!["中文 a.txt"],false,false,false),(vec!["图 片.PNG","另一张.webp"],false,true,false),
            (vec!["压缩 包.ZIP"],true,false,true),(vec!["next.zip.003","parts.z02"],true,false,true),
            (vec!["修复.rev"],false,false,true),(vec!["folder.zip"],false,false,false),
            (vec!["图 片.PNG","中文 a.txt"],false,false,false),(vec!["压缩 包.ZIP","图 片.PNG"],false,false,false),
            (vec!["photo.jpg"],false,true,false),(vec!["photo.jpeg"],false,true,false),
            (vec!["sample.7z"],true,false,true),(vec!["sample.rar"],true,false,true),
        ] {
            let array=selection(&names);
            for kind in [CommandKind::CompressZip,CommandKind::Compress7z,CommandKind::CompressCustom,CommandKind::CompressEncrypted,CommandKind::CompressSplit]{let command:IExplorerCommand=super::ShellCommand::new(kind).into();assert_eq!(unsafe{command.GetState(Some(&array),true).unwrap()},ECS_ENABLED.0 as u32,"{names:?} {kind:?}");}
            for (kind,enabled) in [(CommandKind::ExtractHere,archive),(CommandKind::ImageCompress,image),(CommandKind::ImageWebp,image),(CommandKind::ImageResize,image),(CommandKind::RepairArchive,repair)]{let command:IExplorerCommand=super::ShellCommand::new(kind).into();assert_eq!(unsafe{command.GetState(Some(&array),true).unwrap()},if enabled{ECS_ENABLED}else{ECS_HIDDEN}.0 as u32,"{names:?} {kind:?}");}
        }
        // Ask the COM object for every offered command, as Explorer does. Each
        // native child must be a directly invokable leaf, never a lost cascade.
        let mut catalog = Vec::new();
        for (names,expected) in [
            (vec!["中文 a.txt"],6), (vec!["图 片.PNG"],9), (vec!["压缩 包.ZIP"],14),
            (vec!["check.sha256"],7), (vec!["list.md5"],7), (vec!["list.sfv"],7),
            (vec!["folder.zip"],5), (vec!["图 片.PNG","中文 a.txt"],6),
        ] {
            let array=selection(&names);
            let root_command:IExplorerCommand=super::ShellCommand::new(CommandKind::Root).into();
            let enumerator=unsafe{root_command.EnumSubCommands().unwrap()};
            let mut children:[Option<IExplorerCommand>;SUBCOMMANDS.len()]=std::array::from_fn(|_|None);
            let mut fetched=0;assert_eq!(unsafe{enumerator.Next(&mut children,Some(&mut fetched))},S_OK);
            let mut labels=Vec::new();
            for child in children.iter().flatten(){
                if unsafe{child.GetState(Some(&array),true).unwrap()}!=ECS_ENABLED.0 as u32{continue}
                assert_eq!(unsafe{child.GetFlags().unwrap()}&ECF_HASSUBCOMMANDS.0 as u32,0);
                let title=unsafe{child.GetTitle(Some(&array)).unwrap()};labels.push(unsafe{title.to_string().unwrap()});unsafe{CoTaskMemFree(Some(title.0.cast()))};
            }
            assert_eq!(labels.len(),expected,"{names:?}: {labels:?}");
            catalog.push(format!("{:?}: {:?}",names,labels));
        }
        // A tiny native process receives exactly the argv Explorer launches (no shell quoting/parsing surrogate).
        let probe=root.join("参数接收器.rs");fs::write(&probe,r#"fn main(){std::fs::write(std::env::current_exe().unwrap().with_extension("args"),std::env::args().skip(1).collect::<Vec<_>>().join("\n")).unwrap();}"#).unwrap();
        let exe=root.join("receiver.exe");assert!(std::process::Command::new("rustc").args(["--crate-name","argument_probe"]).arg(&probe).arg("-o").arg(&exe).status().unwrap().success());*super::TEST_APPLICATION_PATH.lock().unwrap()=Some(exe.to_string_lossy().into_owned());
        let mut invoked=Vec::new();
        for kind in SUBCOMMANDS.iter().copied().chain([CommandKind::QuickPack]) {
            let names=match kind {
                CommandKind::ImageCompress|CommandKind::ImageWebp|CommandKind::ImageResize=>vec!["图 片.PNG","另一张.webp"],
                CommandKind::BrowseArchive=>vec!["压缩 包.ZIP"],
                CommandKind::RepairArchive=>vec!["修复.rev"],
                CommandKind::ChecksumVerify=>vec!["check.sha256","list.md5","list.sfv"],
                _ if kind.requires_archive()=>vec!["next.zip.003","parts.z02"],
                _=>vec!["中文 a.txt","图 片.PNG"],
            };
            let receipt=exe.with_extension("args");let _=fs::remove_file(&receipt);let array=selection(&names);let command:IExplorerCommand=super::ShellCommand::new(kind).into();unsafe{command.Invoke(Some(&array),None).unwrap()};
            for _ in 0..100{if receipt.exists(){break}std::thread::sleep(std::time::Duration::from_millis(20));}
            let args=fs::read_to_string(&receipt).unwrap();assert_eq!(args.lines().next(),kind.cli_flag());assert_eq!(args.lines().skip(1).collect::<Vec<_>>(),names.iter().map(|name|root.join(name).to_string_lossy().into_owned()).collect::<Vec<_>>());
            invoked.push(kind.cli_flag().unwrap());
        }
        let output=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-results/explorer-menu-audit");fs::create_dir_all(&output).unwrap();
        fs::write(output.join("native-com-catalog.txt"),catalog.join("\n")).unwrap();
        fs::write(output.join("native-command-arguments.txt"),invoked.join("\n")).unwrap();
        let mixed=selection(&["图 片.PNG","中文 a.txt"]);let command:IExplorerCommand=super::ShellCommand::new(CommandKind::ImageCompress).into();assert!(unsafe{command.Invoke(Some(&mixed),None)}.is_err());
    }

}
