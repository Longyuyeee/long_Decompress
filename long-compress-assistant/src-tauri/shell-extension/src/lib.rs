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
    IShellItemArray, SHStrDupW, ECF_DEFAULT, ECF_HASSUBCOMMANDS, ECS_DISABLED, ECS_ENABLED,
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
}

const SUBCOMMANDS: &[CommandKind] = &[
    CommandKind::ExtractHere,
    CommandKind::ExtractTo,
    CommandKind::ExtractChoose,
    CommandKind::BrowseArchive,
    CommandKind::MoreArchive,
    CommandKind::CompressZip,
    CommandKind::Compress7z,
    CommandKind::CompressCustom,
];
const MORE_ARCHIVE_COMMANDS: &[CommandKind] = &[
    CommandKind::TestArchive,
    CommandKind::RepairArchive,
    CommandKind::ConvertArchive,
    CommandKind::ReconfigureArchive,
    CommandKind::RepackArchiveFiles,
];

impl CommandKind {
    fn title(self) -> &'static str {
        match self {
            Self::Root => "Long解压",
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
            Self::CompressZip => "压缩为 ZIP…",
            Self::Compress7z => "压缩为 7Z…",
            Self::CompressCustom => "更多压缩选项…",
        }
    }

    fn cli_flag(self) -> Option<&'static str> {
        match self {
            Self::Root | Self::MoreArchive => None,
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
        };
        GUID::from_u128(0xd4bba0b2_6a58_4d40_8b79_ba50c54e8d40 + suffix)
    }
}

#[implement(IExplorerCommand)]
struct ShellCommand {
    kind: CommandKind,
    _lifetime: ObjectLifetime,
}

impl ShellCommand {
    fn new(kind: CommandKind) -> Self {
        Self {
            kind,
            _lifetime: ObjectLifetime::new(),
        }
    }
}

impl IExplorerCommand_Impl for ShellCommand_Impl {
    fn GetTitle(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
        duplicate_string(self.kind.title())
    }

    fn GetIcon(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
        let executable = read_application_path()?;
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
        if matches!(self.kind, CommandKind::QuickPack | CommandKind::CompressZip | CommandKind::Compress7z | CommandKind::CompressCustom) {
            let paths=selected_paths(selection).unwrap_or_default();
            if !paths.is_empty() && paths.iter().all(|path|is_archive_path(path)){return Ok(ECS_HIDDEN.0 as u32)}
        }
        if matches!(self.kind, CommandKind::RepairArchive | CommandKind::MoreArchive) {
            let paths = selected_paths(selection).unwrap_or_default();
            if paths.is_empty() || !paths.iter().all(|path| is_repair_path(path) || self.kind == CommandKind::MoreArchive && is_archive_path(path)) { return Ok(ECS_HIDDEN.0 as u32); }
        } else if self.kind.requires_archive() {
            let paths = selected_paths(selection).unwrap_or_default();
            if paths.is_empty() || !paths.iter().all(|path| is_archive_path(path)) {
                return Ok(ECS_HIDDEN.0 as u32);
            }
        }
        Ok(ECS_ENABLED.0 as u32)
    }

    fn Invoke(&self, items: Ref<IShellItemArray>, _bind_context: Ref<IBindCtx>) -> Result<()> {
        let flag = self.kind.cli_flag().ok_or_else(|| Error::from(E_NOTIMPL))?;
        let paths = selected_paths(items.ok()?)?;
        if paths.is_empty() {
            return Err(Error::from(E_FAIL));
        }

        Command::new(read_application_path()?)
            .arg(flag)
            .args(paths)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|_| Error::from(E_FAIL))?;
        Ok(())
    }

    fn GetFlags(&self) -> Result<u32> {
        Ok(if matches!(self.kind, CommandKind::Root | CommandKind::MoreArchive) {
            ECF_HASSUBCOMMANDS.0 as u32
        } else {
            ECF_DEFAULT.0 as u32
        })
    }

    fn EnumSubCommands(&self) -> Result<IEnumExplorerCommand> {
        match self.kind {
            CommandKind::Root => Ok(CommandEnumerator::new(0).into()),
            CommandKind::MoreArchive => Ok(CommandEnumerator::with_commands(0, MORE_ARCHIVE_COMMANDS).into()),
            _ => Err(Error::from(E_NOTIMPL)),
        }
    }
}

#[implement(IEnumExplorerCommand)]
struct CommandEnumerator {
    position: Mutex<usize>,
    commands: &'static [CommandKind],
    _lifetime: ObjectLifetime,
}

impl CommandEnumerator {
    fn new(position: usize) -> Self {
        Self::with_commands(position, SUBCOMMANDS)
    }
    fn with_commands(position: usize, commands: &'static [CommandKind]) -> Self {
        Self {
            position: Mutex::new(position.min(commands.len())),
            commands,
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
            let command: IExplorerCommand = ShellCommand::new(self.commands[*position]).into();
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
        Ok(CommandEnumerator::with_commands(position, self.commands).into())
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
    if is_archive_path(path) { return true; }
    let extension = std::path::Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    extension == "rev" || extension.strip_prefix('r').is_some_and(|n| (2..=3).contains(&n.len()) && n.bytes().all(|b| b.is_ascii_digit()))
        || extension.strip_prefix('z').is_some_and(|n| n.len() >= 2 && n.bytes().all(|b| b.is_ascii_digit()))
        || (extension.len() >= 3 && extension.bytes().all(|b| b.is_ascii_digit()))
}

fn is_archive_path(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            ARCHIVE_EXTENSIONS
                .iter()
                .any(|known| extension.eq_ignore_ascii_case(known))
        })
}

fn read_application_path() -> Result<String> {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(REGISTRATION_KEY)
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
        assert_eq!(SUBCOMMANDS.len(), 8);
        assert_eq!(SUBCOMMANDS[0], CommandKind::ExtractHere);
        assert_eq!(SUBCOMMANDS[2], CommandKind::ExtractChoose);
        assert_eq!(SUBCOMMANDS[3], CommandKind::BrowseArchive);
        assert_eq!(SUBCOMMANDS[4], CommandKind::MoreArchive);
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
            {
                let more=commands[4].as_ref().unwrap();
                assert_eq!(more.GetFlags().unwrap(),ECF_HASSUBCOMMANDS.0 as u32);
                let nested=more.EnumSubCommands().unwrap();
                let mut children:[Option<IExplorerCommand>;MORE_ARCHIVE_COMMANDS.len()]=std::array::from_fn(|_|None);
                let mut count=0;
                assert_eq!(nested.Next(&mut children,Some(&mut count)),S_OK);
                assert_eq!(count as usize,MORE_ARCHIVE_COMMANDS.len());
                assert!(children.iter().all(Option::is_some));
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
}
