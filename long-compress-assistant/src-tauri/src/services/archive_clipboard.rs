//! Materialize selected archive roots before publishing a Windows file clipboard.
//! Successful copies survive application exit; a failed copy never replaces the clipboard.
use crate::models::compression::ArchiveEntryInfo;
use anyhow::{Context, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

pub(crate) const CACHE_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);
pub(crate) struct PreparedCopy {
    directory: Option<tempfile::TempDir>,
    pub roots: Vec<String>,
    expected: BTreeMap<String, (bool, u64)>,
}

fn normalize(path: &str) -> Result<String> {
    super::archive_entry_open::normalize_safe_entry_path(path.trim_end_matches(['/', '\\']))
}
fn under(path: &str, parent: &str) -> bool {
    path == parent || path.starts_with(&format!("{parent}/"))
}

impl PreparedCopy {
    pub fn new(cache: &Path, entries: &[ArchiveEntryInfo], selected: &[String]) -> Result<Self> {
        anyhow::ensure!(!selected.is_empty(), "请先选择要复制的文件或文件夹");
        let mut roots = selected
            .iter()
            .map(|p| normalize(p))
            .collect::<Result<BTreeSet<_>>>()?
            .into_iter()
            .collect::<Vec<_>>();
        roots = roots
            .iter()
            .filter(|p| !roots.iter().any(|parent| parent != *p && under(p, parent)))
            .cloned()
            .collect();
        // Explorer copies each supplied path by its basename. Reject collisions rather than silently losing an item.
        let mut names = BTreeSet::new();
        for root in &roots {
            anyhow::ensure!(
                names.insert(root.rsplit('/').next().unwrap().to_lowercase()),
                "所选项目有同名文件，请分别复制，或选择它们的上级文件夹"
            );
        }
        let mut expected = BTreeMap::new();
        let mut total = 0u64;
        for entry in entries {
            let raw = entry
                .path
                .replace('\\', "/")
                .trim_end_matches('/')
                .to_owned();
            if !roots.iter().any(|root| under(&raw, root)) {
                continue;
            }
            let path = normalize(&raw)?;
            anyhow::ensure!(
                entry.size_known != Some(false),
                "所选内容大小未知，请先解压到指定文件夹"
            );
            anyhow::ensure!(
                entry.link_target.is_none(),
                "所选内容包含链接，请先解压到指定文件夹再复制"
            );
            anyhow::ensure!(
                expected.insert(path, (entry.is_dir, entry.size)).is_none(),
                "所选内容包含重复路径，请先整理后复制"
            );
            if !entry.is_dir {
                total = total.checked_add(entry.size).context("所选内容过大")?;
            }
        }
        for root in &roots {
            anyhow::ensure!(
                expected.keys().any(|p| under(p, root)),
                "压缩包中找不到所选项目：{root}"
            );
            if !expected.contains_key(root) {
                expected.insert(root.clone(), (true, 0));
            }
        }
        anyhow::ensure!(
            expected.len() <= super::extraction_transaction::MAX_EXTRACTED_ENTRIES
                && total <= super::extraction_transaction::MAX_EXTRACTED_BYTES,
            "所选内容超过单次解压范围，请分批复制"
        );
        fs::create_dir_all(cache).context("无法创建文件复制缓存")?;
        let directory = tempfile::Builder::new().prefix("copy-").tempdir_in(cache)?;
        Ok(Self {
            directory: Some(directory),
            roots,
            expected,
        })
    }
    pub fn path(&self) -> &Path {
        self.directory.as_ref().unwrap().path()
    }
    pub fn validate(&self) -> Result<Vec<PathBuf>> {
        // Verify the exact requested tree, including empty directories, before exposing any file to Explorer.
        let mut allowed = self.expected.clone();
        for path in self.expected.keys() {
            let mut parent = Path::new(path).parent();
            while let Some(p) = parent.filter(|p| !p.as_os_str().is_empty()) {
                allowed
                    .entry(p.to_string_lossy().replace('\\', "/"))
                    .or_insert((true, 0));
                parent = p.parent();
            }
        }
        for entry in walkdir::WalkDir::new(self.path())
            .min_depth(1)
            .follow_links(false)
        {
            let entry = entry?;
            let name = entry
                .path()
                .strip_prefix(self.path())?
                .to_string_lossy()
                .replace('\\', "/");
            let (directory, size) = allowed
                .remove(&name)
                .with_context(|| format!("复制缓存包含未选择的项目：{name}"))?;
            let meta = fs::symlink_metadata(entry.path())?;
            anyhow::ensure!(
                !meta.file_type().is_symlink()
                    && meta.is_dir() == directory
                    && (directory || meta.is_file() && meta.len() == size),
                "所选内容未完整提取：{name}"
            );
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                anyhow::ensure!(
                    meta.file_attributes() & 0x400 == 0,
                    "复制缓存包含链接：{name}"
                );
            }
        }
        anyhow::ensure!(
            allowed.is_empty(),
            "部分所选内容未提取完成，请刷新目录后重试"
        );
        Ok(self
            .roots
            .iter()
            .map(|root| self.path().join(root))
            .collect())
    }
    pub fn persist(mut self) {
        let _ = self.directory.take().unwrap().keep();
    }
}

pub(crate) fn cleanup(cache: &Path, protected: &[PathBuf], now: SystemTime) -> Result<()> {
    if !cache.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(cache)? {
        let entry = entry?;
        // Delete only our direct, non-link cache children; never follow arbitrary files.
        if !entry.file_name().to_string_lossy().starts_with("copy-") || !entry.file_type()?.is_dir()
        {
            continue;
        }
        let path = entry.path();
        if protected.iter().any(|p| p.starts_with(&path)) {
            continue;
        }
        if now
            .duration_since(entry.metadata()?.modified()?)
            .unwrap_or_default()
            >= CACHE_TTL
        {
            let _ = fs::remove_dir_all(path);
        }
    }
    Ok(())
}

#[cfg(windows)]
pub(crate) mod windows {
    use super::*;
    use std::{
        os::windows::ffi::{OsStrExt, OsStringExt},
        ptr,
    };
    use windows_sys::Win32::{
        Foundation::{HANDLE, HWND},
        System::{DataExchange::*, Memory::*},
        UI::{
            Shell::{DragQueryFileW, DROPFILES},
            WindowsAndMessaging::*,
        },
    };
    struct Window(HWND);
    impl Drop for Window {
        fn drop(&mut self) {
            unsafe {
                DestroyWindow(self.0);
            }
        }
    }
    struct Clipboard;
    impl Drop for Clipboard {
        fn drop(&mut self) {
            unsafe {
                CloseClipboard();
            }
        }
    }
    #[link(name = "ole32")]
    extern "system" {
        fn OleInitialize(reserved: *mut std::ffi::c_void) -> i32;
        fn OleUninitialize();
        fn OleGetClipboard(object: *mut *mut std::ffi::c_void) -> i32;
        fn OleSetClipboard(object: *mut std::ffi::c_void) -> i32;
        fn OleFlushClipboard() -> i32;
    }
    struct Ole;
    impl Drop for Ole {
        fn drop(&mut self) {
            unsafe {
                OleUninitialize();
            }
        }
    }
    struct Rollback {
        object: *mut std::ffi::c_void,
        committed: bool,
    }
    impl Drop for Rollback {
        fn drop(&mut self) {
            unsafe {
                if !self.committed {
                    if OleSetClipboard(self.object) >= 0 {
                        OleFlushClipboard();
                    }
                }
                if !self.object.is_null() {
                    // IDataObject inherits IUnknown; release the reference returned by OleGetClipboard.
                    let table = *(self.object as *const *const *const std::ffi::c_void);
                    let release: unsafe extern "system" fn(*mut std::ffi::c_void) -> u32 =
                        std::mem::transmute(*table.add(2));
                    release(self.object);
                }
            }
        }
    }
    struct Memory(HANDLE);
    impl Drop for Memory {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    windows_sys::Win32::Foundation::GlobalFree(self.0);
                }
            }
        }
    }
    fn allocation(bytes: &[u8]) -> Result<Memory> {
        unsafe {
            let memory = Memory(GlobalAlloc(GMEM_MOVEABLE, bytes.len()));
            anyhow::ensure!(!memory.0.is_null(), "无法分配剪贴板数据");
            let pointer = GlobalLock(memory.0);
            anyhow::ensure!(!pointer.is_null(), "无法写入剪贴板数据");
            ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.cast(), bytes.len());
            GlobalUnlock(memory.0);
            Ok(memory)
        }
    }
    fn open(owner: HWND) -> Result<Clipboard> {
        for _ in 0..10 {
            if unsafe { OpenClipboard(owner) } != 0 {
                return Ok(Clipboard);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        anyhow::bail!("剪贴板正被其他程序使用，请稍后重试")
    }
    pub fn read_files() -> Result<Vec<PathBuf>> {
        let _clipboard = open(ptr::null_mut())?;
        let handle = unsafe { GetClipboardData(15) };
        if handle.is_null() {
            return Ok(vec![]);
        }
        let mut paths = Vec::new();
        unsafe {
            for index in 0..DragQueryFileW(handle, u32::MAX, ptr::null_mut(), 0) {
                let mut path =
                    vec![0u16; DragQueryFileW(handle, index, ptr::null_mut(), 0) as usize + 1];
                let length = DragQueryFileW(handle, index, path.as_mut_ptr(), path.len() as u32);
                paths.push(PathBuf::from(std::ffi::OsString::from_wide(
                    &path[..length as usize],
                )));
            }
        }
        Ok(paths)
    }
    pub fn publish(paths: &[PathBuf]) -> Result<()> {
        let paths = paths.to_vec();
        // A short-lived STA owns only the clipboard operation; it does not show or control a desktop window.
        std::thread::spawn(move || publish_sta(&paths))
            .join()
            .map_err(|_| anyhow::anyhow!("文件剪贴板操作未完成"))?
    }
    fn publish_sta(paths: &[PathBuf]) -> Result<()> {
        anyhow::ensure!(
            !paths.is_empty() && paths.iter().all(|p| p.is_absolute() && p.exists()),
            "复制文件尚未准备完成"
        );
        let mut wide = Vec::<u16>::new();
        for path in paths {
            wide.extend(path.as_os_str().encode_wide());
            wide.push(0);
        }
        wide.push(0);
        let header = DROPFILES {
            pFiles: std::mem::size_of::<DROPFILES>() as u32,
            pt: windows_sys::Win32::Foundation::POINT { x: 0, y: 0 },
            fNC: 0,
            fWide: 1,
        };
        let mut bytes = unsafe {
            std::slice::from_raw_parts(
                (&header as *const DROPFILES).cast::<u8>(),
                std::mem::size_of::<DROPFILES>(),
            )
        }
        .to_vec();
        for unit in wide {
            bytes.extend(unit.to_le_bytes());
        }
        let mut files = allocation(&bytes)?;
        let mut effect = allocation(&1u32.to_le_bytes())?; // DROPEFFECT_COPY, never move the cache.
        let effect_name = "Preferred DropEffect\0".encode_utf16().collect::<Vec<_>>();
        unsafe {
            anyhow::ensure!(OleInitialize(ptr::null_mut()) >= 0, "无法初始化文件剪贴板");
            let _ole = Ole;
            let mut rollback = Rollback {
                object: ptr::null_mut(),
                committed: true,
            };
            anyhow::ensure!(
                OleGetClipboard(&mut rollback.object) >= 0,
                "无法保留原剪贴板，请稍后重试"
            );
            let format = RegisterClipboardFormatW(effect_name.as_ptr());
            anyhow::ensure!(format != 0, "无法注册文件复制格式");
            let class = "STATIC\0".encode_utf16().collect::<Vec<_>>();
            let owner = Window(CreateWindowExW(
                0,
                class.as_ptr(),
                ptr::null(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null(),
            ));
            anyhow::ensure!(!owner.0.is_null(), "无法创建文件剪贴板");
            let _clipboard = open(owner.0)?;
            anyhow::ensure!(EmptyClipboard() != 0, "无法更新剪贴板");
            rollback.committed = false;
            anyhow::ensure!(
                !SetClipboardData(15, files.0).is_null(),
                "写入文件剪贴板失败"
            );
            files.0 = ptr::null_mut(); // The system owns the HGLOBAL from this point.
            if !SetClipboardData(format, effect.0).is_null() {
                effect.0 = ptr::null_mut();
            }
            rollback.committed = true;
        }
        Ok(())
    }
}
