use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

pub const MAX_OPEN_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_SESSION_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_SESSION_ENTRIES: usize = 64;
pub const MAX_NESTED_ARCHIVE_DEPTH: usize = 3;
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveEntryOpenResult {
    pub status: String,
    pub entry_path: String,
    pub cache_path: Option<String>,
    pub dangerous: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NestedArchiveMaterializeResult {
    pub entry_path: String,
    pub cache_path: String,
    pub parent_sha256: String,
    pub content_sha256: String,
    pub depth: usize,
}

#[derive(Default)]
struct CacheUsage {
    entries: usize,
    bytes: u64,
}

pub struct ArchiveEntryOpenCache {
    session: PathBuf,
    usage: Mutex<CacheUsage>,
    nested: Mutex<HashMap<PathBuf, NestedCacheIdentity>>,
}

#[derive(Clone)]
struct NestedCacheIdentity {
    depth: usize,
    ancestor_hashes: Vec<String>,
}

impl ArchiveEntryOpenCache {
    pub fn new(root: PathBuf) -> Self {
        if let Err(error) = cleanup_stale_sessions(&root, SystemTime::now()) {
            eprintln!("Failed to clean stale archive preview cache: {error}");
        }
        let session = root.join(Uuid::new_v4().to_string());
        Self {
            session,
            usage: Mutex::new(CacheUsage::default()),
            nested: Mutex::new(HashMap::new()),
        }
    }

    pub fn create_entry_dir(&self, expected_bytes: u64) -> Result<(PathBuf, CacheReservation<'_>)> {
        if expected_bytes > MAX_OPEN_FILE_BYTES {
            anyhow::bail!(
                "文件超过安全打开上限（{} MiB）",
                MAX_OPEN_FILE_BYTES / 1024 / 1024
            );
        }
        let mut usage = self
            .usage
            .lock()
            .map_err(|_| anyhow::anyhow!("预览缓存状态不可用"))?;
        if usage.entries >= MAX_SESSION_ENTRIES
            || usage.bytes.saturating_add(expected_bytes) > MAX_SESSION_BYTES
        {
            anyhow::bail!("本次会话的预览缓存已达到安全上限，请重启 Long解压后重试");
        }
        usage.entries += 1;
        usage.bytes += expected_bytes;
        drop(usage);
        let path = self.session.join(Uuid::new_v4().to_string());
        if let Err(error) = fs::create_dir_all(&path) {
            self.release(expected_bytes);
            return Err(error).context("无法创建隔离预览缓存");
        }
        Ok((
            path,
            CacheReservation {
                cache: self,
                bytes: expected_bytes,
                committed: false,
            },
        ))
    }

    fn release(&self, bytes: u64) {
        if let Ok(mut usage) = self.usage.lock() {
            usage.entries = usage.entries.saturating_sub(1);
            usage.bytes = usage.bytes.saturating_sub(bytes);
        }
    }

    pub fn cleanup_session(&self) {
        let _ = fs::remove_dir_all(&self.session);
    }

    pub fn register_nested_archive(
        &self,
        parent_path: &Path,
        nested_path: &Path,
        parent_sha256: &str,
        content_sha256: &str,
        claimed_depth: usize,
    ) -> Result<()> {
        let mut nested = self
            .nested
            .lock()
            .map_err(|_| anyhow::anyhow!("嵌套归档缓存状态不可用"))?;
        let parent_key = parent_path.canonicalize().unwrap_or_else(|_| parent_path.to_path_buf());
        let child_key = nested_path.canonicalize().unwrap_or_else(|_| nested_path.to_path_buf());
        let (expected_depth, mut ancestor_hashes) = match nested.get(&parent_key) {
            Some(parent) => (parent.depth + 1, parent.ancestor_hashes.clone()),
            None => (2, vec![parent_sha256.to_ascii_lowercase()]),
        };
        if claimed_depth != expected_depth || claimed_depth > MAX_NESTED_ARCHIVE_DEPTH {
            anyhow::bail!(
                "嵌套归档层级状态不一致：服务端期望第 {expected_depth} 层，收到第 {claimed_depth} 层"
            );
        }
        let normalized_hash = content_sha256.to_ascii_lowercase();
        if ancestor_hashes.iter().any(|value| value == &normalized_hash) {
            anyhow::bail!("检测到重复归档内容，已阻止循环进入");
        }
        ancestor_hashes.push(normalized_hash);
        nested.insert(
            child_key,
            NestedCacheIdentity {
                depth: claimed_depth,
                ancestor_hashes,
            },
        );
        Ok(())
    }
}

impl Drop for ArchiveEntryOpenCache {
    fn drop(&mut self) {
        self.cleanup_session();
    }
}

pub struct CacheReservation<'a> {
    cache: &'a ArchiveEntryOpenCache,
    bytes: u64,
    committed: bool,
}

impl CacheReservation<'_> {
    pub fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for CacheReservation<'_> {
    fn drop(&mut self) {
        if !self.committed {
            self.cache.release(self.bytes);
        }
    }
}

pub fn normalize_safe_entry_path(value: &str) -> Result<String> {
    let normalized = value.replace('\\', "/");
    if normalized.is_empty()
        || normalized.contains('\0')
        || normalized.starts_with('/')
        || normalized.contains(':')
    {
        anyhow::bail!("归档内路径不安全，已拒绝打开");
    }
    let path = Path::new(&normalized);
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => {
                let value = value.to_string_lossy();
                if is_reserved_windows_name(&value) {
                    anyhow::bail!("归档内路径包含 Windows 保留设备名");
                }
                parts.push(value.into_owned());
            }
            _ => anyhow::bail!("归档内路径不安全，已拒绝打开"),
        }
    }
    if parts.is_empty() {
        anyhow::bail!("归档内路径为空");
    }
    Ok(parts.join("/"))
}

fn is_reserved_windows_name(value: &str) -> bool {
    let stem = value
        .trim_end_matches([' ', '.'])
        .split('.')
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || stem
            .strip_prefix("COM")
            .and_then(|v| v.parse::<u8>().ok())
            .is_some_and(|v| (1..=9).contains(&v))
        || stem
            .strip_prefix("LPT")
            .and_then(|v| v.parse::<u8>().ok())
            .is_some_and(|v| (1..=9).contains(&v))
}

pub fn is_dangerous_entry(path: &str) -> bool {
    let extension = Path::new(path)
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        extension.as_str(),
        "exe"
            | "msi"
            | "msp"
            | "com"
            | "bat"
            | "cmd"
            | "ps1"
            | "psm1"
            | "js"
            | "jse"
            | "vbs"
            | "vbe"
            | "wsf"
            | "wsh"
            | "hta"
            | "lnk"
            | "scr"
            | "reg"
            | "cpl"
    )
}

#[cfg(windows)]
pub fn open_with_default_application(path: &Path) -> Result<()> {
    use std::ffi::c_void;

    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteW(
            window: *mut c_void,
            operation: *const u16,
            file: *const u16,
            parameters: *const u16,
            directory: *const u16,
            show_command: i32,
        ) -> isize;
    }

    let operation: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
    let file: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    };
    if result <= 32 {
        anyhow::bail!("Windows 默认应用启动失败（ShellExecuteW={result}）");
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn open_with_default_application(path: &Path) -> Result<()> {
    let command = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(command)
        .arg(path)
        .spawn()
        .with_context(|| format!("Unable to launch {command}"))?;
    Ok(())
}

pub fn validate_extracted_file(
    root: &Path,
    expected_path: &str,
    expected_bytes: u64,
) -> Result<PathBuf> {
    let expected = root.join(expected_path.replace('/', std::path::MAIN_SEPARATOR_STR));
    let metadata = fs::symlink_metadata(&expected).context("解压后的预览文件不存在")?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        anyhow::bail!("预览缓存只允许普通文件");
    }
    if metadata.len() != expected_bytes || metadata.len() > MAX_OPEN_FILE_BYTES {
        anyhow::bail!("预览文件大小与归档元数据不一致");
    }
    let mut files = 0usize;
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry.context("无法校验预览缓存")?;
        if entry.file_type().is_symlink() {
            anyhow::bail!("预览缓存中检测到链接");
        }
        if entry.file_type().is_file() {
            files += 1;
        }
    }
    if files != 1 {
        anyhow::bail!("预览缓存内容数量异常");
    }
    Ok(expected)
}

fn sha256_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};

    let mut reader = BufReader::new(fs::File::open(path)?);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn validate_nested_archive_identity(
    parent_archive: &Path,
    nested_archive: &Path,
    target_depth: usize,
    ancestor_hashes: &[String],
) -> Result<(String, String)> {
    use crate::services::archive_format::ArchiveFormat;

    if !(2..=MAX_NESTED_ARCHIVE_DEPTH).contains(&target_depth) {
        anyhow::bail!(
            "嵌套归档最多允许 {} 层，当前目标为第 {target_depth} 层",
            MAX_NESTED_ARCHIVE_DEPTH
        );
    }
    let mut file = fs::File::open(nested_archive)?;
    let mut header = [0u8; 560];
    let read = file.read(&mut header)?;
    let magic = ArchiveFormat::from_magic(&header[..read]);
    let extension = nested_archive
        .extension()
        .and_then(|value| value.to_str())
        .map(ArchiveFormat::from_extension)
        .unwrap_or(ArchiveFormat::Unknown);
    if magic == ArchiveFormat::Unknown && extension == ArchiveFormat::Unknown {
        anyhow::bail!("所选条目不是受支持的归档格式");
    }

    let parent_sha256 = sha256_file(parent_archive)?;
    let content_sha256 = sha256_file(nested_archive)?;
    if content_sha256 == parent_sha256
        || ancestor_hashes
            .iter()
            .any(|value| value.eq_ignore_ascii_case(&content_sha256))
    {
        anyhow::bail!("检测到重复归档内容，已阻止循环进入");
    }
    Ok((parent_sha256, content_sha256))
}

fn cleanup_stale_sessions(root: &Path, now: SystemTime) -> Result<()> {
    fs::create_dir_all(root)?;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let modified = entry
            .metadata()?
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH);
        if now.duration_since(modified).unwrap_or_default() >= CACHE_TTL {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rejects_traversal_absolute_device_and_ads_paths() {
        for path in [
            "../evil.exe",
            "/root.txt",
            "C:/evil.txt",
            "\\\\?\\C:\\evil.txt",
            "safe/file.txt:payload",
            "CON.txt",
        ] {
            assert!(normalize_safe_entry_path(path).is_err(), "accepted {path}");
        }
        assert_eq!(
            normalize_safe_entry_path("资料/报告 01.pdf").unwrap(),
            "资料/报告 01.pdf"
        );
    }

    #[tokio::test]
    async fn rejects_a_traversal_entry_read_from_a_real_zip_before_cache_creation() {
        let temp = tempfile::tempdir().unwrap();
        let archive_path = temp.path().join("malicious.zip");
        let file = fs::File::create(&archive_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        writer
            .start_file("../escape.txt", zip::write::FileOptions::default())
            .unwrap();
        writer.write_all(b"must not escape").unwrap();
        writer.finish().unwrap();

        let metadata = crate::services::archive_browser::browse_archive(&archive_path, None)
            .await
            .unwrap();
        assert_eq!(metadata.entries[0].path, "../escape.txt");
        assert!(normalize_safe_entry_path(&metadata.entries[0].path).is_err());
        assert!(!temp.path().join("preview-cache").exists());
    }

    #[test]
    fn classifies_active_content_without_blocking_documents() {
        assert!(is_dangerous_entry("setup.EXE"));
        assert!(is_dangerous_entry("scripts/run.ps1"));
        assert!(!is_dangerous_entry("manual.pdf"));
        assert!(!is_dangerous_entry("photo.png"));
    }

    #[test]
    fn validates_exactly_one_regular_file_and_size() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("中文 目录")).unwrap();
        fs::write(temp.path().join("中文 目录/readme.txt"), b"hello").unwrap();
        assert!(validate_extracted_file(temp.path(), "中文 目录/readme.txt", 5).is_ok());
        assert!(validate_extracted_file(temp.path(), "中文 目录/readme.txt", 4).is_err());
    }

    #[test]
    fn stale_sessions_are_removed_but_current_files_survive() {
        let temp = tempfile::tempdir().unwrap();
        let old = temp.path().join("old");
        fs::create_dir_all(&old).unwrap();
        filetime::set_file_mtime(
            &old,
            filetime::FileTime::from_system_time(
                SystemTime::now() - CACHE_TTL - Duration::from_secs(1),
            ),
        )
        .unwrap();
        let fresh = temp.path().join("fresh");
        fs::create_dir_all(&fresh).unwrap();
        cleanup_stale_sessions(temp.path(), SystemTime::now()).unwrap();
        assert!(!old.exists());
        assert!(fresh.exists());
    }

    #[test]
    fn dropping_cache_removes_the_current_session() {
        let temp = tempfile::tempdir().unwrap();
        let session;
        {
            let cache = ArchiveEntryOpenCache::new(temp.path().to_path_buf());
            session = cache.session.clone();
            fs::create_dir_all(&session).unwrap();
            fs::write(session.join("occupied.txt"), b"preview").unwrap();
        }
        assert!(!session.exists());
    }

    #[test]
    fn nested_identity_enforces_depth_and_repeated_content() {
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().join("parent.zip");
        let child = temp.path().join("child.zip");
        fs::write(&parent, b"PK\x03\x04parent").unwrap();
        fs::write(&child, b"PK\x03\x04child").unwrap();

        let (parent_hash, child_hash) =
            validate_nested_archive_identity(&parent, &child, 2, &[]).unwrap();
        assert_ne!(parent_hash, child_hash);
        assert!(validate_nested_archive_identity(&parent, &child, 4, &[])
            .unwrap_err()
            .to_string()
            .contains("最多允许"));
        assert!(validate_nested_archive_identity(&parent, &child, 3, &[child_hash])
            .unwrap_err()
            .to_string()
            .contains("重复归档"));

        fs::write(&child, fs::read(&parent).unwrap()).unwrap();
        assert!(validate_nested_archive_identity(&parent, &child, 2, &[])
            .unwrap_err()
            .to_string()
            .contains("循环进入"));
    }

    #[test]
    fn nested_cache_derives_depth_server_side_and_rejects_a_fourth_layer() {
        let temp = tempfile::tempdir().unwrap();
        let cache = ArchiveEntryOpenCache::new(temp.path().join("cache"));
        let root = temp.path().join("root.zip");
        let middle = temp.path().join("middle.7z");
        let inner = temp.path().join("inner.zip");
        let fourth = temp.path().join("fourth.zip");
        for path in [&root, &middle, &inner, &fourth] {
            fs::write(path, b"archive").unwrap();
        }
        cache
            .register_nested_archive(&root, &middle, "root", "middle", 2)
            .unwrap();
        cache
            .register_nested_archive(&middle, &inner, "middle", "inner", 3)
            .unwrap();
        assert!(cache
            .register_nested_archive(&inner, &fourth, "inner", "fourth", 4)
            .unwrap_err()
            .to_string()
            .contains("层级状态不一致"));
        assert!(cache
            .register_nested_archive(&middle, &fourth, "middle", "root", 3)
            .unwrap_err()
            .to_string()
            .contains("循环进入"));
    }
}
