//! ZPAQ 7.15 read adapter. Native key material travels only through binary stdin.
use anyhow::{Context, Result};
use serde::Deserialize;
use std::{io::Read, path::{Path, PathBuf}, sync::{Arc, atomic::AtomicBool}, time::Duration};
use crate::models::compression::{ArchiveBrowseResult, ArchiveEntryInfo};

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
    let mut header = [0u8; 32];
    let n = std::fs::File::open(path)?.read(&mut header)?;
    anyhow::ensure!(n >= 4, "[archive-inspection:damaged] ZPAQ 文件头不完整");
    if header.starts_with(b"7kSt") || header.starts_with(b"zPQ") { return Ok(false); }
    anyhow::ensure!(n == 32, "[archive-inspection:damaged] ZPAQ 文件过短，文件头或密码盐不完整");
    Ok(true)
}

fn input(password: Option<&str>, selected: &[String]) -> Result<Vec<u8>> {
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

async fn run(path: &Path, password: Option<&str>, output: Option<&Path>, test: bool, selected: &[String], cancelled: Arc<AtomicBool>) -> Result<Vec<u8>> {
    let encrypted = requires_password(path)?;
    anyhow::ensure!(!encrypted || password.is_some(), "[archive-inspection:password-required] ZPAQ 需要原生密码；也请确认文件未损坏");
    let tool = command_path().context("[archive-inspection:engine-unavailable] 缺少随附的 ZPAQ 读取引擎，请修复安装")?;
    let mut command = crate::utils::process::async_command(tool);
    command.arg(if test || output.is_some() { "x" } else { "l" }).arg(path)
        .args(["-long-stdin", "-threads", "2", "-noattributes"]);
    if test { command.arg("-test"); }
    else if let Some(output) = output { command.arg("-to").arg(output); }
    else { command.arg("-long-json"); }
    let result = crate::utils::process::cancellable_output_with_stdin(
        &mut command, &cancelled, Duration::from_secs(24 * 60 * 60), Some(input(if encrypted { password } else { None }, selected)?))
        .await.map_err(|e| anyhow::anyhow!("ZPAQ 读取取消或引擎运行失败：{e}"))?;
    if !result.status.success() {
        let detail = format!("{}\n{}", String::from_utf8_lossy(&result.stderr), String::from_utf8_lossy(&result.stdout));
        if detail.contains("password incorrect") {
            anyhow::bail!("[archive-inspection:ambiguous-data] ZPAQ 密码不匹配或文件头已损坏，无法仅凭解密头确定原因");
        }
        let detail: String = detail.chars().take(4000).collect();
        anyhow::bail!("[archive-inspection:damaged] ZPAQ 结构或内容校验失败（可能损坏或事务未完成）：{detail}");
    }
    Ok(result.stdout)
}

#[derive(Deserialize)]
struct Directory { kind: String, versions: u64, entries: Vec<Entry> }
#[derive(Deserialize)]
struct Entry { path: String, size: i64, date: i64, directory: bool }

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
            compressed_size: None, modified: Some(item.date.to_string()), crc: None, encrypted, is_dir,
        });
    }
    let total_files = entries.iter().filter(|e| !e.is_dir).count();
    let total_uncompressed_size = entries.iter().filter(|e| !e.is_dir).try_fold(0u64, |sum, e| sum.checked_add(e.size).context("ZPAQ 大小溢出"))?;
    Ok(ArchiveBrowseResult { format: "ZPAQ".into(), total_files, total_directories: entries.len() - total_files,
        total_uncompressed_size, total_compressed_size: 0, encrypted, entries })
}

pub async fn browse(path: &Path, password: Option<&str>, cancelled: Arc<AtomicBool>) -> Result<ArchiveBrowseResult> {
    let bytes = run(path, password, None, false, &[], cancelled).await?;
    let directory = parse_directory(&bytes, requires_password(path)?)?;
    super::extraction_transaction::validate_resource_limits(path, directory.entries.len(), directory.total_uncompressed_size)?;
    Ok(directory)
}

pub async fn verify(path: &Path, password: Option<&str>, cancelled: Arc<AtomicBool>) -> Result<()> {
    browse(path, password, cancelled.clone()).await?;
    run(path, password, None, true, &[], cancelled).await?;
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
    let directory = browse(path, password, cancelled.clone()).await?;
    let mut exact = Vec::new();
    for name in selected {
        let normalized = super::archive_entry_open::normalize_safe_entry_path(name.trim_end_matches('/'))?;
        let entry = directory.entries.iter().find(|e| e.path == normalized).context("所选 ZPAQ 条目不存在于当前视图")?;
        exact.push(if entry.is_dir { format!("{normalized}/") } else { normalized });
    }
    run(path, password, Some(output), false, &exact, cancelled).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(name: &str) -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/zpaq").join(name) }
    fn flag() -> Arc<AtomicBool> { Arc::new(AtomicBool::new(false)) }
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
    }
    #[test]
    fn metadata_rejects_unsafe_paths_and_missing_protocol() {
        assert!(parse_directory(b"no directory", false).is_err());
        assert!(parse_directory(b"\nLONG_ZPAQ_JSON\n{\"kind\":\"journaling\",\"versions\":1,\"entries\":[{\"path\":\"../file\",\"size\":1,\"date\":20261018000000,\"directory\":false}]}", false).is_err());
    }
}
