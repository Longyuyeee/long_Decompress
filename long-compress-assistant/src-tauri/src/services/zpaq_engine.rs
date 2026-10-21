//! ZPAQ 7.15 adapter. Native key material travels only through binary stdin.
use anyhow::{Context, Result};
use serde::Deserialize;
use std::{io::Read, path::{Path, PathBuf}, sync::{Arc, atomic::AtomicBool}, time::Duration};
use crate::models::compression::{ArchiveBrowseResult, ArchiveEntryInfo, ZpaqHistory, ZpaqSnapshot};

pub fn command_path() -> Option<PathBuf> {
    let mut roots = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources")];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            roots.insert(0, parent.join("resources"));
            roots.insert(0, parent.to_path_buf());
        }
    }
    roots.into_iter().map(|p| p.join("zpaq-engine/long-zpaq.exe")).find(|p| p.is_file())
}

pub fn requires_password(path: &Path) -> Result<bool> {
    anyhow::ensure!(super::zpaq_multipart::suggest(path)?.is_none(),"[archive-inspection:multipart-confirmation] 发现同名 ZPAQ 部分，请从浏览中心或重新添加到解压中心，确认整组或仅打开此文件；没有默认只读第一部分");
    let source = super::split_archive_detector::SplitArchiveDetector::complete_source(path)?;
    let mut header = Vec::with_capacity(32);
    super::numeric_volume_reader::ZipInputReader::open(&source)?.take(32).read_to_end(&mut header)?;
    let n = header.len();
    anyhow::ensure!(n >= 4, "[archive-inspection:damaged] ZPAQ 文件头不完整");
    if header.starts_with(b"7kSt") || header.starts_with(b"zPQ") { return Ok(false); }
    anyhow::ensure!(n == 32, "[archive-inspection:damaged] ZPAQ 文件过短，文件头或密码盐不完整");
    Ok(true)
}

pub(crate) fn input(password: Option<&str>, selected: &[String]) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    if let Some(password) = password {
        anyhow::ensure!(password.len() <= 65536, "ZPAQ 密码过长");
        data.extend_from_slice(&(password.len() as u32).to_le_bytes());
        data.extend_from_slice(password.as_bytes());
    } else { data.extend_from_slice(&u32::MAX.to_le_bytes()); }
    anyhow::ensure!(selected.len() <= 100000, "所选条目过多");
    data.extend_from_slice(&(selected.len() as u32).to_le_bytes());
    for name in selected {
        anyhow::ensure!(name.len() <= 65536, "归档内路径过长");
        data.extend_from_slice(&(name.len() as u32).to_le_bytes());
        data.extend_from_slice(name.as_bytes());
    }
    Ok(data)
}

async fn run(path: &Path, password: Option<&str>, output: Option<&Path>, test: bool, selected: &[String], cancelled: Arc<AtomicBool>, snapshot: Option<&ZpaqSnapshot>) -> Result<Vec<u8>> {
    run_with_preview(path, password, output, test, selected, cancelled, None, snapshot).await
}

async fn run_with_preview(path: &Path, password: Option<&str>, output: Option<&Path>, test: bool, selected: &[String], cancelled: Arc<AtomicBool>, preview_limit: Option<u64>, snapshot: Option<&ZpaqSnapshot>) -> Result<Vec<u8>> {
    run_catalog_mode(path,password,output,test,selected,cancelled,preview_limit,snapshot,false).await
}

async fn run_catalog_mode(path: &Path,password: Option<&str>,output: Option<&Path>,test: bool,selected: &[String],cancelled: Arc<AtomicBool>,preview_limit: Option<u64>,snapshot: Option<&ZpaqSnapshot>, refs: bool) -> Result<Vec<u8>> {
    anyhow::ensure!(!cancelled.load(std::sync::atomic::Ordering::Relaxed), "ZPAQ 读取已取消");
    let source = super::split_archive_detector::SplitArchiveDetector::complete_source(path)?;
    let path = source.as_path();
    let encrypted = requires_password(path)?;
    anyhow::ensure!(!encrypted || password.is_some(), "[archive-inspection:password-required] ZPAQ 需要原生密码；也请确认文件未损坏");
    let tool = command_path().context("[archive-inspection:engine-unavailable] 缺少随附的 ZPAQ 读取引擎，请修复安装")?;
    let mut command = crate::utils::process::async_command(tool);
    command.arg(if test || output.is_some() { "x" } else { "l" }).arg(path)
        .args(["-long-stdin", "-threads", "2", "-noattributes"]);
    if let Some(snapshot) = snapshot {
        anyhow::ensure!(snapshot.version > 0 && snapshot.version <= 9_999_999 && (19000101000000..=29991231235959).contains(&snapshot.date),
            "[archive-inspection:snapshot-unavailable] ZPAQ 快照标识无效，请重新打开归档选择快照");
        command.args(["-long-snapshot", &snapshot.version.to_string(), &snapshot.date.to_string()]);
    }
    let mut data = input(if encrypted { password } else { None }, selected)?;
    if super::numeric_volume_reader::ZipInputReader::is_numeric_first(path) || super::zpaq_multipart::details(path)?.is_some() {
        let group = super::split_archive_detector::SplitArchiveDetector::detect_split_archive(path)?.context("没有找到 ZPAQ 数字卷组")?;
        anyhow::ensure!(group.is_complete, "[archive-inspection:incomplete] 缺少 ZPAQ 分卷：{}", group.missing_part_names());
        anyhow::ensure!(group.parts.len() <= 100000, "ZPAQ 分卷数量过多");
        command.arg("-long-volumes");
        data.extend_from_slice(&(group.parts.len() as u32).to_le_bytes());
        for part in group.parts {
            let name = part.to_str().context("ZPAQ 分卷路径不能表示为 UTF-8")?.replace('\\', "/");
            anyhow::ensure!(name.len() <= 65536 && !name.contains('\0'), "ZPAQ 分卷路径过长或无效");
            data.extend_from_slice(&(name.len() as u32).to_le_bytes()); data.extend_from_slice(name.as_bytes());
        }
    }
    if let Some(limit) = preview_limit { command.args(["-long-preview", &limit.to_string()]); }
    if refs { command.arg("-long-refs"); }
    if test { command.arg("-test"); }
    else if let Some(output) = output { command.arg("-to").arg(output); }
    else { command.arg("-long-json"); }
    let result = crate::utils::process::cancellable_archive_output_with_stdin(
        &mut command, &cancelled, Duration::from_secs(if preview_limit.is_some() { 30 } else { 24 * 60 * 60 }), Some(data))
        .await.map_err(|e| anyhow::anyhow!("ZPAQ 读取取消或引擎运行失败：{e}"))?;
    if !result.status.success() {
        let detail = format!("{}\n{}", String::from_utf8_lossy(&result.stderr), String::from_utf8_lossy(&result.stdout));
        if let Some(error) = io_failure(&detail) { return Err(error); }
        if detail.contains("LONG_ZPAQ_SNAPSHOT:") {
            anyhow::bail!("[archive-inspection:snapshot-unavailable] 所选 ZPAQ 快照不存在或来源已变化，请重新打开归档并选择快照；没有改为读取最新版本。");
        }
        if detail.contains("LONG_ZPAQ_PREVIEW:") {
            anyhow::bail!("此 ZPAQ 条目的大小、共享块或读取量超过内部预览范围，请先解压后打开；未输出部分预览作为完整文件。");
        }
        if detail.contains("password incorrect") {
            anyhow::bail!("[archive-inspection:ambiguous-data] ZPAQ 密码不匹配或文件头已损坏，无法仅凭解密头确定原因");
        }
        let detail: String = detail.chars().take(4000).collect();
        anyhow::bail!("[archive-inspection:damaged] ZPAQ 结构或内容校验失败（可能损坏或事务未完成）：{detail}");
    }
    Ok(result.stdout)
}

pub(crate) fn io_failure(detail: &str) -> Option<anyhow::Error> {
    let line = detail.lines().find_map(|line| line.strip_prefix("LONG_ZPAQ_IO:"))?;
    let (operation, code) = line.split_once(':')?;
    let code = code.trim().parse::<i32>().ok()?;
    let cause = std::io::Error::from_raw_os_error(code);
    let message = format!("[zpaq-io] ZPAQ 文件读写失败（{operation}）：{cause}；请检查目标空间、磁盘连接和文件访问权限，不要反复试密码。");
    Some(anyhow::Error::new(cause).context(message))
}

pub async fn create(sources: &[String], output: &Path, options: &crate::models::compression::CompressionOptions, cancelled: Arc<AtomicBool>) -> Result<()> {
    anyhow::ensure!(!cancelled.load(std::sync::atomic::Ordering::Acquire), "ZPAQ 创建已取消");
    let tool = command_path().context("缺少随附的 ZPAQ 引擎，请修复安装")?;
    anyhow::ensure!(!output.exists(), "ZPAQ 暂存目标已存在，不能追加或覆盖");
    let preserve = options.preserve_paths.unwrap_or(true);
    let mut entries = super::compression_entries::collect(sources, preserve, preserve)?;
    // The shared collector omits a selected root itself. Preserve an empty
    // selected folder too, rather than producing no archive or losing it.
    if preserve {
        for source in sources {
            let path = Path::new(source);
            if path.is_dir() && std::fs::read_dir(path)?.next().is_none() {
                entries.push(super::compression_entries::CompressionEntry {
                    path: path.to_path_buf(), archive_name: path.file_name().context("目录名称无效")?.to_string_lossy().into_owned(), is_dir: true,
                });
            }
        }
    }
    anyhow::ensure!(!entries.is_empty() && entries.len() <= 100000, "ZPAQ 没有可压缩文件，或条目数超过 100000");
    let mut data = input(options.password.as_deref().filter(|p| !p.is_empty()), &[])?;
    data.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    let mut names = std::collections::HashSet::new();
    for entry in entries {
        let mut name = super::archive_entry_open::normalize_safe_entry_path(&entry.archive_name)?;
        anyhow::ensure!(names.insert(name.to_lowercase()), "ZPAQ 内部路径存在 Windows 同名冲突：{name}");
        let mut source = entry.path.to_string_lossy().replace('\\', "/");
        if entry.is_dir { source.push('/'); name.push('/'); }
        for text in [source, name] {
            anyhow::ensure!(text.len() <= 65536 && !text.contains('\0'), "ZPAQ 路径过长或含无效字符");
            data.extend_from_slice(&(text.len() as u32).to_le_bytes());
            data.extend_from_slice(text.as_bytes());
        }
    }
    // Bounded 16 MiB blocks and two workers; the nine UI steps map to the
    // upstream's six actual methods, never invented native volume options.
    let method = match options.level { 0 => "04", 1..=2 => "14", 3..=4 => "24", 5..=6 => "34", 7..=8 => "44", _ => "54" };
    let mut command = crate::utils::process::async_command(tool);
    command.arg("a").arg(output).args(["-long-stdin", "-long-create", "-method", method, "-threads", "2", "-noattributes"]);
    let result = crate::utils::process::cancellable_archive_output_with_stdin(&mut command, &cancelled, Duration::from_secs(24*60*60), Some(data)).await
        .map_err(|e| anyhow::anyhow!("ZPAQ 创建取消或引擎运行失败：{e}"))?;
    if !result.status.success() {
        let detail = format!("{}\n{}", String::from_utf8_lossy(&result.stderr), String::from_utf8_lossy(&result.stdout));
        if let Some(error) = io_failure(&detail) { return Err(error); }
        anyhow::bail!("ZPAQ 创建失败：{}", detail.chars().take(4000).collect::<String>());
    }
    anyhow::ensure!(output.is_file(), "ZPAQ 引擎没有生成压缩包");
    Ok(())
}

#[derive(serde::Serialize, Deserialize)]
struct Directory { kind: String, versions: u64, entries: Vec<Entry>, #[serde(default)] history: Option<ZpaqHistory> }
#[derive(Clone, serde::Serialize, Deserialize)]
pub(crate) struct Entry { pub path: String, pub size: i64, pub date: i64, pub directory: bool, #[serde(default,rename="contentRef",skip_serializing_if="Option::is_none")] pub content_ref: Option<String> }

fn modified_date(date: i64) -> Option<String> {
    let s = date.to_string();
    (s.len() == 14 && s.bytes().all(|c| c.is_ascii_digit())).then(||
        format!("{}-{}-{} {}:{}:{} UTC", &s[..4], &s[4..6], &s[6..8], &s[8..10], &s[10..12], &s[12..]))
}

fn parse_directory(bytes: &[u8], encrypted: bool) -> Result<ArchiveBrowseResult> {
    let text = std::str::from_utf8(bytes).context("ZPAQ 目录不是有效 UTF-8")?.replace("\r\n", "\n");
    let json = text.split_once("\nLONG_ZPAQ_JSON\n").context("ZPAQ 引擎缺少目录响应")?.1;
    let directory: Directory = serde_json::from_str(json).context("ZPAQ 目录响应无效")?;
    anyhow::ensure!(directory.versions > 0 && matches!(directory.kind.as_str(), "journaling" | "streaming" | "mixed"), "ZPAQ 没有有效事务或流式内容");
    let mut entries = Vec::new();
    for item in directory.entries {
        let path = super::archive_entry_open::normalize_safe_entry_path(item.path.trim_end_matches('/'))?;
        let is_dir = item.directory;
        entries.push(ArchiveEntryInfo {
            name: path.rsplit('/').next().unwrap_or(&path).to_string(), path,
            size: item.size.max(0) as u64, size_known: Some(is_dir || item.size >= 0),
            compressed_size: None, modified: modified_date(item.date), crc: None, encrypted, is_dir,
        });
    }
    let total_files = entries.iter().filter(|e| !e.is_dir).count();
    let total_uncompressed_size = entries.iter().filter(|e| !e.is_dir).try_fold(0u64, |sum, e| sum.checked_add(e.size).context("ZPAQ 大小溢出"))?;
    Ok(ArchiveBrowseResult { zpaq_history: directory.history, format: "ZPAQ".into(), total_files, total_directories: entries.len() - total_files,
        total_uncompressed_size, total_compressed_size: 0, encrypted, entries })
}

pub async fn browse(path: &Path, password: Option<&str>, cancelled: Arc<AtomicBool>) -> Result<ArchiveBrowseResult> {
    browse_at(path, password, None, cancelled).await
}

pub async fn browse_at(path: &Path, password: Option<&str>, snapshot: Option<&ZpaqSnapshot>, cancelled: Arc<AtomicBool>) -> Result<ArchiveBrowseResult> {
    let bytes = run(path, password, None, false, &[], cancelled, snapshot).await?;
    let mut directory = parse_directory(&bytes, requires_password(path)?)?;
    directory.total_compressed_size = super::split_archive_detector::SplitArchiveDetector::detect_split_archive(path)?
        .map(|group| group.total_size).unwrap_or(std::fs::metadata(path)?.len());
    super::extraction_transaction::validate_resource_limits(path, directory.entries.len(), directory.total_uncompressed_size)?;
    Ok(directory)
}

/// A bounded temporary copy of the selected entry, never of the whole archive.
/// Unknown streaming sizes are deliberately not treated as known zero.
pub async fn preview_entry(path: &Path, entry: &str, password: Option<&str>, max_output: u64, keep: u64) -> Result<(Vec<u8>, u64)> {
    preview_entry_at(path, entry, password, max_output, keep, None).await
}

pub async fn preview_entry_at(path: &Path, entry: &str, password: Option<&str>, max_output: u64, keep: u64, snapshot: Option<&ZpaqSnapshot>) -> Result<(Vec<u8>, u64)> {
    preview_entry_controlled_at(path,entry,password,max_output,keep,snapshot,Arc::new(AtomicBool::new(false))).await
}
pub async fn preview_entry_controlled_at(path:&Path,entry:&str,password:Option<&str>,max_output:u64,keep:u64,snapshot:Option<&ZpaqSnapshot>,flag:Arc<AtomicBool>)->Result<(Vec<u8>,u64)>{
    let entry=super::archive_entry_open::normalize_safe_entry_path(entry)?;
    let listing = run_with_preview(path, password, None, false, &[], flag.clone(), Some(max_output), snapshot).await?;
    let directory = parse_directory(&listing, requires_password(path)?)?;
    let item = directory.entries.iter().find(|item| item.path == entry && !item.is_dir).context("找不到所选 ZPAQ 文件")?;
    anyhow::ensure!(item.size_known != Some(false) && item.size <= max_output,
        "此 ZPAQ 文件大小未知或超过内部预览范围，请先解压后打开。");
    let temporary = tempfile::Builder::new().prefix("long-zpaq-preview-").tempdir()?;
    run_with_preview(path, password, Some(temporary.path()), false, &[entry.clone()], flag, Some(max_output), snapshot).await?;
    let file = std::fs::File::open(temporary.path().join(&entry))?;
    let actual = file.metadata()?.len();
    anyhow::ensure!(actual == item.size && actual <= max_output, "ZPAQ 预览条目大小不一致，请先解压并检查原包。");
    let mut bytes = Vec::with_capacity(actual.min(keep) as usize);
    file.take(keep).read_to_end(&mut bytes)?;
    Ok((bytes, actual))
}

pub async fn verify(path: &Path, password: Option<&str>, cancelled: Arc<AtomicBool>) -> Result<()> {
    verify_at(path, password, None, cancelled).await
}

pub async fn verify_at(path: &Path, password: Option<&str>, snapshot: Option<&ZpaqSnapshot>, cancelled: Arc<AtomicBool>) -> Result<()> {
    browse_at(path, password, snapshot, cancelled.clone()).await?;
    run(path, password, None, true, &[], cancelled, snapshot).await?;
    Ok(())
}

pub async fn try_password(path: &Path, password: &str, cancelled: Arc<AtomicBool>) -> Result<bool> {
    if !requires_password(path)? { return Ok(false); }
    match verify(path, Some(password), cancelled).await {
        Ok(()) => Ok(true),
        Err(e) if e.to_string().contains("[archive-inspection:ambiguous-data]") => Ok(false),
        Err(e) => Err(e),
    }
}

pub async fn extract(path: &Path, output: &Path, password: Option<&str>, selected: &[String], cancelled: Arc<AtomicBool>) -> Result<()> {
    extract_at(path, output, password, selected, None, cancelled).await
}

pub async fn extract_at(path: &Path, output: &Path, password: Option<&str>, selected: &[String], snapshot: Option<&ZpaqSnapshot>, cancelled: Arc<AtomicBool>) -> Result<()> {
    let directory = browse_at(path, password, snapshot, cancelled.clone()).await?;
    let mut exact = Vec::new();
    for name in selected {
        let normalized = super::archive_entry_open::normalize_safe_entry_path(name.trim_end_matches('/'))?;
        let entry = directory.entries.iter().find(|e| e.path == normalized).context("所选 ZPAQ 条目不存在于当前视图")?;
        exact.push(if entry.is_dir { format!("{normalized}/") } else { normalized });
    }
    run(path, password, Some(output), false, &exact, cancelled, snapshot).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(name: &str) -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/zpaq").join(name) }
    fn flag() -> Arc<AtomicBool> { Arc::new(AtomicBool::new(false)) }
    #[tokio::test]
    async fn create_preserves_names_contents_empty_folders_native_key_and_flattening() {
        use crate::models::compression::CompressionOptions;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("项目 空格");
        std::fs::create_dir_all(root.join("子目录/空目录")).unwrap();
        std::fs::write(root.join("说明.txt"), "中文载荷\r\n").unwrap();
        std::fs::write(root.join("子目录/说明.txt"), "同名不同内容").unwrap();
        std::fs::write(root.join("空文件"), []).unwrap();
        for (level, password) in [(0,None), (6,Some("中文 密码!-钥匙")), (9,Some(" "))] {
            let archive = temp.path().join(format!("created-{level}.zpaq"));
            let options = CompressionOptions { level, password: password.map(str::to_owned), preserve_paths: Some(true), ..Default::default() };
            create(&[root.to_string_lossy().into_owned()], &archive, &options, flag()).await.unwrap();
            verify(&archive, password, flag()).await.unwrap();
            let directory = browse(&archive, password, flag()).await.unwrap();
            assert_eq!(directory.total_files, 3);
            assert!(directory.entries.iter().any(|e| e.is_dir && e.path == "项目 空格/子目录/空目录"));
            let output = tempfile::tempdir().unwrap();
            extract(&archive, output.path(), password, &[], flag()).await.unwrap();
            assert_eq!(std::fs::read_to_string(output.path().join("项目 空格/说明.txt")).unwrap(), "中文载荷\r\n");
            assert_eq!(std::fs::read_to_string(output.path().join("项目 空格/子目录/说明.txt")).unwrap(), "同名不同内容");
            assert!(create(&[root.to_string_lossy().into_owned()], &archive, &options, flag()).await.unwrap_err().to_string().contains("不能追加或覆盖"));
            if password.is_some() { assert!(!try_password(&archive, "wrong", flag()).await.unwrap()); }
        }
        let flat = temp.path().join("flat.zpaq");
        create(&[root.to_string_lossy().into_owned()], &flat, &CompressionOptions { preserve_paths: Some(false), ..Default::default() }, flag()).await.unwrap();
        let directory = browse(&flat, None, flag()).await.unwrap();
        assert_eq!(directory.total_files, 3);
        assert!(directory.entries.iter().all(|e| !e.path.contains('/')));
        assert!(directory.entries.iter().any(|e| e.path == "说明 (1).txt"));
        let empty = temp.path().join("独立空目录"); std::fs::create_dir(&empty).unwrap();
        let archive = temp.path().join("empty.zpaq");
        create(&[empty.to_string_lossy().into_owned()], &archive, &CompressionOptions::default(), flag()).await.unwrap();
        let directory = browse(&archive, None, flag()).await.unwrap();
        assert_eq!(directory.total_directories, 1);
        assert_eq!(directory.total_files, 0);
    }
    #[tokio::test]
    async fn create_cancel_and_source_or_destination_failure_are_not_success() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("not-created.zpaq");
        let cancelled = flag(); cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(create(&[], &output, &Default::default(), cancelled).await.is_err());
        assert!(!output.exists());
        assert!(create(&[temp.path().join("missing").to_string_lossy().into_owned()], &output, &Default::default(), flag()).await.is_err());
        assert!(!output.exists());
        let source = temp.path().join("source"); std::fs::write(&source, b"payload").unwrap();
        let missing_parent = temp.path().join("missing-parent/output.zpaq");
        assert!(create(&[source.to_string_lossy().into_owned()], &missing_parent, &Default::default(), flag()).await.is_err());
        assert_eq!(std::fs::read(source).unwrap(), b"payload");
        let error = io_failure("LONG_ZPAQ_IO:write:112\n").unwrap().to_string();
        assert!(error.contains("文件读写失败") && error.contains("不要反复试密码"));
        assert!(!error.contains("[archive-inspection:damaged]"));
    }
    #[tokio::test]
    async fn official_streaming_and_journaling_current_view_extract_and_test() {
        for name in ["journal.zpaq", "journal-key.zpaq", "stream.zpaq", "stream-key.zpaq"] {
            let path = fixture(name);
            let password = name.contains("-key").then_some("中文 密码!-钥匙");
            let result = browse(&path, password, flag()).await.unwrap();
            assert_eq!(result.total_files, 3, "{name}: {result:?}");
            assert!(!result.entries.iter().any(|e| e.path.contains("已删除") || e.path.starts_with("1/")));
            verify(&path, password, flag()).await.unwrap();
            let output = tempfile::tempdir().unwrap();
            extract(&path, output.path(), password, &[], flag()).await.unwrap();
            assert_eq!(std::fs::read_to_string(output.path().join("项目 文档/说明.txt")).unwrap(), "第二版：当前文件视图，不混入旧版本。\r\n");
            assert_eq!(std::fs::read(output.path().join("项目 文档/数据.bin")).unwrap(), (0..32768).map(|i| (i*37) as u8).collect::<Vec<_>>());
            let selected = tempfile::tempdir().unwrap();
            extract(&path, selected.path(), password, &["项目 文档/说明.txt".into()], flag()).await.unwrap();
            assert!(selected.path().join("项目 文档/说明.txt").is_file());
            assert!(!selected.path().join("项目 文档/数据.bin").exists());
            assert!(!selected.path().join("项目 文档/已删除.txt").exists());
            if password.is_some() {
                assert!(!try_password(&path, "wrong", flag()).await.unwrap());
                assert!(try_password(&path, password.unwrap(), flag()).await.unwrap());
            }
        }
    }
    #[tokio::test]
    async fn damage_truncation_and_cancel_are_not_success() {
        for name in ["journal.zpaq", "stream.zpaq"] {
            let original = std::fs::read(fixture(name)).unwrap();
            for mode in ["damaged", "truncated"] {
                let dir = tempfile::tempdir().unwrap(); let source = dir.path().join("input.zpaq");
                let mut data = original.clone();
                if mode == "truncated" { data.truncate(data.len()-40); } else {
                    // Streaming headers are not authenticated. Damage a known
                    // stored payload byte, not an arbitrary attribute/comment.
                    let pos = if name.starts_with("stream") {
                        let prefix: Vec<u8> = (0..64).map(|i| (i*37) as u8).collect();
                        data.windows(prefix.len()).position(|p| p == prefix).expect("official stored payload") + 100
                    } else { data.len()-90 };
                    data[pos] ^= 0x55;
                }
                std::fs::write(&source, data).unwrap();
                assert!(verify(&source, None, flag()).await.is_err(), "{name} {mode}");
            }
        }
        let cancelled = flag(); cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(browse(&fixture("journal.zpaq"), None, cancelled).await.is_err());
    }
    #[test]
    fn transport_preserves_unicode_and_newlines_without_argv() {
        let p = "中文 密码\n-钥匙";
        let bytes = input(Some(p), &["-文件.txt".into()]).unwrap();
        assert_eq!(&bytes[4..4+p.len()], p.as_bytes());
        assert_eq!(u32::from_le_bytes(input(None, &[]).unwrap()[..4].try_into().unwrap()), u32::MAX);
    }
    #[test]
    fn streaming_unknown_size_is_not_reported_as_known_zero() {
        let result = parse_directory(b"\nLONG_ZPAQ_JSON\n{\"kind\":\"streaming\",\"versions\":1,\"entries\":[{\"path\":\"file\",\"size\":-1,\"date\":20261018000000,\"directory\":false}]}", false).unwrap();
        assert_eq!(result.entries[0].size_known, Some(false));
        assert_eq!(result.entries[0].modified.as_deref(), Some("2026-10-18 00:00:00 UTC"));
    }
    #[test]
    fn metadata_rejects_unsafe_paths_and_missing_protocol() {
        assert!(parse_directory(b"no directory", false).is_err());
        assert!(parse_directory(b"\nLONG_ZPAQ_JSON\n{\"kind\":\"journaling\",\"versions\":1,\"entries\":[{\"path\":\"../file\",\"size\":1,\"date\":20261018000000,\"directory\":false}]}", false).is_err());
    }
}

/// Metadata reference identity, not a hash of decoded file contents.
pub(crate) struct Catalog { pub id: String, pub entries: Vec<Entry>, pub history: ZpaqHistory }
fn catalog_source_stamp(path:&Path)->Result<Vec<u8>> {
    let first=super::split_archive_detector::SplitArchiveDetector::complete_source(path)?;
    let members=super::split_archive_detector::SplitArchiveDetector::detect_split_archive(&first)?.map(|g|g.parts).unwrap_or_else(||vec![first]);
    let mut stamp=Vec::new();for member in members {let metadata=std::fs::metadata(member)?;stamp.extend_from_slice(&metadata.len().to_le_bytes());stamp.extend_from_slice(format!("{:?};",metadata.modified()?).as_bytes());}Ok(stamp)
}
pub(crate) async fn catalog(path: &Path,password: Option<&str>,snapshot: &ZpaqSnapshot,flag: Arc<AtomicBool>) -> Result<Catalog> {
    use sha2::{Digest,Sha256};
    let stamp=catalog_source_stamp(path)?;
    let bytes=run_catalog_mode(path,password,None,false,&[],flag,None,Some(snapshot),true).await?;
    let text=String::from_utf8(bytes)?.replace("\r\n","\n");
    let directory:Directory=serde_json::from_str(text.split_once("\nLONG_ZPAQ_JSON\n").context("缺少ZPAQ目录响应")?.1)?;
    anyhow::ensure!(directory.kind=="journaling","仅支持完整日志式ZPAQ快照对比");
    let history=directory.history.clone().context("此归档没有历史快照")?;
    anyhow::ensure!(history.selected==*snapshot,"所选快照已变化，请重新对比");
    for e in &directory.entries { super::archive_entry_open::normalize_safe_entry_path(e.path.trim_end_matches('/'))?;anyhow::ensure!(e.size>=0&&e.content_ref.as_ref().is_some_and(|id|id.len()==64),"ZPAQ目录缺少完整内容引用，请修复引擎或重新获取归档"); }
    anyhow::ensure!(stamp==catalog_source_stamp(path)?,"读取期间来源已变化，请重新对比");
    let mut digest=Sha256::new();digest.update(stamp);digest.update(serde_json::to_vec(&directory)?);let id=format!("{:x}",digest.finalize());
    Ok(Catalog{id,entries:directory.entries,history})
}
pub async fn check_catalog(path:&Path,password:Option<&str>,snapshot:&ZpaqSnapshot,expected:&str,flag:Arc<AtomicBool>)->Result<()> {
    anyhow::ensure!(expected.len()==64&&expected.bytes().all(|b|b.is_ascii_hexdigit()),"快照目录标识无效，请重新对比");
    anyhow::ensure!(catalog(path,password,snapshot,flag).await?.id==expected,"[archive-inspection:snapshot-unavailable] 归档目录或历史已变化，请重新对比并选择；未改为提取其他版本");Ok(())
}
pub async fn extract_checked_at(path:&Path,output:&Path,password:Option<&str>,selected:&[String],snapshot:Option<&ZpaqSnapshot>,expected:Option<&str>,flag:Arc<AtomicBool>)->Result<()> {
    if let Some(expected)=expected { check_catalog(path,password,snapshot.context("变化提取必须保留来源快照")?,expected,flag.clone()).await?; }
    extract_at(path,output,password,selected,snapshot,flag).await
}
