use serde::Serialize;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileManagerLocation {
    pub name: String,
    pub path: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileOperationReport {
    pub operation: String,
    pub processed: usize,
    pub files: u64,
    pub directories: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileProperties {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub bytes: u64,
    pub files: u64,
    pub directories: u64,
    pub readonly: bool,
    pub modified_unix_ms: Option<u128>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
struct TreeStats {
    files: u64,
    directories: u64,
    bytes: u64,
}

pub fn locations() -> Vec<FileManagerLocation> {
    let mut result = Vec::new();
    if let Some(home) = dirs::home_dir() {
        result.push(location("主目录", home, "home"));
    }
    for (label, path) in [
        ("桌面", dirs::desktop_dir()),
        ("下载", dirs::download_dir()),
        ("文档", dirs::document_dir()),
    ] {
        if let Some(path) = path.filter(|path| path.is_dir()) {
            if !result
                .iter()
                .any(|item| item.path.eq_ignore_ascii_case(&path.to_string_lossy()))
            {
                result.push(location(label, path, "known"));
            }
        }
    }
    #[cfg(windows)]
    for letter in b'A'..=b'Z' {
        let path = PathBuf::from(format!("{}:\\", letter as char));
        if path.is_dir() {
            result.push(location(&format!("{} 盘", letter as char), path, "drive"));
        }
    }
    #[cfg(not(windows))]
    result.push(location("根目录", PathBuf::from("/"), "drive"));
    result
}

fn location(name: &str, path: PathBuf, kind: &str) -> FileManagerLocation {
    FileManagerLocation {
        name: name.to_string(),
        path: path.to_string_lossy().to_string(),
        kind: kind.to_string(),
    }
}

pub fn copy_to_directory(
    sources: &[PathBuf],
    destination: &Path,
) -> anyhow::Result<FileOperationReport> {
    validate_sources_and_destination(sources, destination)?;
    let mut total = TreeStats::default();
    for source in sources {
        let name = source
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("不能复制磁盘根目录"))?;
        let target = destination.join(name);
        if target.exists() {
            anyhow::bail!("目标已存在，为避免覆盖已停止：{}", target.display());
        }
        reject_descendant_target(source, &target)?;
        let staging = unique_staging_path(destination, name);
        let expected = tree_stats(source)?;
        if let Err(error) = copy_tree(source, &staging).and_then(|_| verify_tree(source, &staging))
        {
            let _ = remove_tree(&staging);
            return Err(error);
        }
        if let Err(error) = fs::rename(&staging, &target) {
            let _ = remove_tree(&staging);
            return Err(error).map_err(|error| anyhow::anyhow!("发布复制结果失败：{error}"));
        }
        total = add_stats(total, expected);
    }
    Ok(report("copy", sources.len(), total))
}

pub fn move_to_directory(
    sources: &[PathBuf],
    destination: &Path,
) -> anyhow::Result<FileOperationReport> {
    validate_sources_and_destination(sources, destination)?;
    let mut total = TreeStats::default();
    for source in sources {
        let name = source
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("不能移动磁盘根目录"))?;
        let target = destination.join(name);
        if target.exists() {
            anyhow::bail!("目标已存在，为避免覆盖已停止：{}", target.display());
        }
        reject_descendant_target(source, &target)?;
        let expected = tree_stats(source)?;
        match fs::rename(source, &target) {
            Ok(()) => {}
            Err(_) => {
                let staging = unique_staging_path(destination, name);
                if let Err(error) =
                    copy_tree(source, &staging).and_then(|_| verify_tree(source, &staging))
                {
                    let _ = remove_tree(&staging);
                    return Err(error);
                }
                fs::rename(&staging, &target)
                    .map_err(|error| anyhow::anyhow!("发布移动结果失败：{error}"))?;
                remove_tree(source).map_err(|error| {
                    anyhow::anyhow!("文件已安全复制到目标，但源删除失败，请手动处理：{error}")
                })?;
            }
        }
        total = add_stats(total, expected);
    }
    Ok(report("move", sources.len(), total))
}

pub fn recycle_items(paths: &[PathBuf]) -> anyhow::Result<usize> {
    if paths.is_empty() {
        anyhow::bail!("没有选择文件");
    }
    for path in paths {
        require_regular_entry(path)?;
        if path.parent().is_none() {
            anyhow::bail!("不能删除磁盘根目录");
        }
    }
    crate::services::source_recycle::move_paths_to_system_recycle_bin(paths)
}

pub fn rename_item(source: &Path, new_name: &str) -> anyhow::Result<PathBuf> {
    require_regular_entry(source)?;
    let trimmed = new_name.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." || trimmed.contains(['/', '\\']) {
        anyhow::bail!("新名称无效");
    }
    #[cfg(windows)]
    if trimmed.contains(['<', '>', ':', '"', '|', '?', '*']) || trimmed.ends_with(['.', ' ']) {
        anyhow::bail!("新名称包含 Windows 不允许的字符");
    }
    let target = source
        .parent()
        .ok_or_else(|| anyhow::anyhow!("不能重命名磁盘根目录"))?
        .join(trimmed);
    if target.exists() {
        anyhow::bail!("同名文件或文件夹已存在");
    }
    fs::rename(source, &target).map_err(|error| anyhow::anyhow!("重命名失败：{error}"))?;
    Ok(target)
}

pub fn create_directory(parent: &Path, name: &str) -> anyhow::Result<PathBuf> {
    require_directory(parent)?;
    let target = parent.join(name.trim());
    if name.trim().is_empty() || name.contains(['/', '\\']) || target.exists() {
        anyhow::bail!("文件夹名称无效或已经存在");
    }
    fs::create_dir(&target).map_err(|error| anyhow::anyhow!("新建文件夹失败：{error}"))?;
    Ok(target)
}

pub fn properties(path: &Path) -> anyhow::Result<FileProperties> {
    require_regular_entry(path)?;
    let metadata = fs::symlink_metadata(path)?;
    let stats = tree_stats(path)?;
    Ok(FileProperties {
        path: path.to_string_lossy().to_string(),
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        is_dir: metadata.is_dir(),
        bytes: stats.bytes,
        files: stats.files,
        directories: stats.directories,
        readonly: metadata.permissions().readonly(),
        modified_unix_ms: metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|value| value.as_millis()),
    })
}

fn validate_sources_and_destination(sources: &[PathBuf], destination: &Path) -> anyhow::Result<()> {
    if sources.is_empty() {
        anyhow::bail!("没有选择文件");
    }
    require_directory(destination)?;
    let mut targets = std::collections::HashSet::new();
    for source in sources {
        require_regular_entry(source)?;
        let name = source
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("不能操作磁盘根目录"))?;
        let target = destination.join(name);
        if target.exists() {
            anyhow::bail!("目标已存在，为避免覆盖已停止：{}", target.display());
        }
        if !targets.insert(target.to_string_lossy().to_lowercase()) {
            anyhow::bail!("选择项中存在同名目标，操作未开始");
        }
        reject_descendant_target(source, &target)?;
    }
    Ok(())
}

fn require_directory(path: &Path) -> anyhow::Result<()> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| anyhow::anyhow!("目录不可访问：{error}"))?;
    if !metadata.is_dir() || is_link_or_reparse(&metadata) {
        anyhow::bail!("目标必须是可访问的真实目录");
    }
    Ok(())
}

fn require_regular_entry(path: &Path) -> anyhow::Result<()> {
    if !path.is_absolute() {
        anyhow::bail!("路径必须是绝对路径");
    }
    let metadata =
        fs::symlink_metadata(path).map_err(|error| anyhow::anyhow!("路径不可访问：{error}"))?;
    if is_link_or_reparse(&metadata) {
        anyhow::bail!("为避免越界，暂不操作符号链接或重解析点");
    }
    Ok(())
}

fn reject_descendant_target(source: &Path, target: &Path) -> anyhow::Result<()> {
    if source.is_dir() {
        let canonical_source = source.canonicalize()?;
        let canonical_parent = target.parent().unwrap_or(target).canonicalize()?;
        if canonical_parent.starts_with(&canonical_source) {
            anyhow::bail!("不能把文件夹复制或移动到自身内部");
        }
    }
    Ok(())
}

fn unique_staging_path(parent: &Path, name: &std::ffi::OsStr) -> PathBuf {
    loop {
        let candidate = parent.join(format!(
            ".long-staging-{}-{}",
            name.to_string_lossy(),
            uuid::Uuid::new_v4()
        ));
        if !candidate.exists() {
            return candidate;
        }
    }
}

fn copy_tree(source: &Path, target: &Path) -> anyhow::Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    if is_link_or_reparse(&metadata) {
        anyhow::bail!("复制树包含符号链接或重解析点：{}", source.display());
    }
    if metadata.is_dir() {
        fs::create_dir(target)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_tree(&entry.path(), &target.join(entry.file_name()))?;
        }
    } else if metadata.is_file() {
        let mut input = fs::File::open(source)?;
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)?;
        std::io::copy(&mut input, &mut output)?;
        output.flush()?;
        output.sync_all()?;
    } else {
        anyhow::bail!("不支持的文件类型：{}", source.display());
    }
    Ok(())
}

fn verify_tree(source: &Path, target: &Path) -> anyhow::Result<()> {
    let source_meta = fs::symlink_metadata(source)?;
    let target_meta = fs::symlink_metadata(target)?;
    if source_meta.is_dir() != target_meta.is_dir() {
        anyhow::bail!("复制校验失败：类型不一致");
    }
    if source_meta.is_dir() {
        let mut source_names = fs::read_dir(source)?
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<Result<Vec<_>, _>>()?;
        let mut target_names = fs::read_dir(target)?
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<Result<Vec<_>, _>>()?;
        source_names.sort();
        target_names.sort();
        if source_names != target_names {
            anyhow::bail!("复制校验失败：目录内容不一致");
        }
        for name in source_names {
            verify_tree(&source.join(&name), &target.join(name))?;
        }
    } else if hash_file(source)? != hash_file(target)? {
        anyhow::bail!("复制校验失败：文件哈希不一致");
    }
    Ok(())
}

fn hash_file(path: &Path) -> anyhow::Result<blake3::Hash> {
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize())
}

fn tree_stats(path: &Path) -> anyhow::Result<TreeStats> {
    let metadata = fs::symlink_metadata(path)?;
    if is_link_or_reparse(&metadata) {
        anyhow::bail!("目录包含符号链接或重解析点：{}", path.display());
    }
    if metadata.is_file() {
        return Ok(TreeStats {
            files: 1,
            directories: 0,
            bytes: metadata.len(),
        });
    }
    let mut result = TreeStats {
        files: 0,
        directories: 1,
        bytes: 0,
    };
    for entry in fs::read_dir(path)? {
        result = add_stats(result, tree_stats(&entry?.path())?);
    }
    Ok(result)
}

fn add_stats(left: TreeStats, right: TreeStats) -> TreeStats {
    TreeStats {
        files: left.files + right.files,
        directories: left.directories + right.directories,
        bytes: left.bytes + right.bytes,
    }
}

fn report(operation: &str, processed: usize, stats: TreeStats) -> FileOperationReport {
    FileOperationReport {
        operation: operation.to_string(),
        processed,
        files: stats.files,
        directories: stats.directories,
        bytes: stats.bytes,
    }
}

fn remove_tree(path: &Path) -> std::io::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_copy_move_rename_and_properties_match_expected_bytes() {
        let root = tempfile::tempdir().unwrap();
        let left = root.path().join("left");
        let right = root.path().join("right");
        fs::create_dir_all(left.join("album/nested")).unwrap();
        fs::create_dir_all(&right).unwrap();
        fs::write(left.join("album/a.txt"), b"alpha").unwrap();
        fs::write(left.join("album/nested/b.bin"), [0_u8, 1, 2, 3]).unwrap();

        let copied = copy_to_directory(&[left.join("album")], &right).unwrap();
        assert_eq!(
            (
                copied.processed,
                copied.files,
                copied.directories,
                copied.bytes
            ),
            (1, 2, 2, 9)
        );
        assert_eq!(fs::read(right.join("album/a.txt")).unwrap(), b"alpha");
        assert!(copy_to_directory(&[left.join("album")], &right)
            .unwrap_err()
            .to_string()
            .contains("避免覆盖"));

        let renamed = rename_item(&right.join("album/a.txt"), "renamed.txt").unwrap();
        assert_eq!(fs::read(&renamed).unwrap(), b"alpha");
        let moved = move_to_directory(&[renamed], &left).unwrap();
        assert_eq!((moved.processed, moved.files, moved.bytes), (1, 1, 5));
        assert_eq!(properties(&left.join("renamed.txt")).unwrap().bytes, 5);
    }

    #[test]
    fn rejects_descendant_copy_and_path_like_rename() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        fs::create_dir_all(source.join("child")).unwrap();
        assert!(copy_to_directory(std::slice::from_ref(&source), &source.join("child"))
            .unwrap_err()
            .to_string()
            .contains("自身内部"));
        assert!(rename_item(&source, "bad/name")
            .unwrap_err()
            .to_string()
            .contains("无效"));
    }

    #[test]
    fn batch_preflight_prevents_partial_copy_when_any_target_conflicts() {
        let root = tempfile::tempdir().unwrap();
        let left = root.path().join("left");
        let right = root.path().join("right");
        fs::create_dir_all(&left).unwrap();
        fs::create_dir_all(&right).unwrap();
        fs::write(left.join("first.txt"), b"first").unwrap();
        fs::write(left.join("second.txt"), b"second").unwrap();
        fs::write(right.join("second.txt"), b"existing").unwrap();
        assert!(
            copy_to_directory(&[left.join("first.txt"), left.join("second.txt")], &right).is_err()
        );
        assert!(!right.join("first.txt").exists());
        assert_eq!(fs::read(right.join("second.txt")).unwrap(), b"existing");
    }
}
