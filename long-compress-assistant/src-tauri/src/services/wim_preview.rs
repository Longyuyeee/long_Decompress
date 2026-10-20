//! WIM previews reuse the bundled reader's random access, including numeric slices.
use anyhow::{Context, Result};
use std::{path::Path, process::Stdio, sync::{Arc, atomic::{AtomicBool, Ordering}}, time::Duration};
use tokio::io::{AsyncRead, AsyncReadExt};

pub(crate) fn type_argument(path: &Path) -> &'static str {
    if super::numeric_volume_reader::ZipInputReader::is_numeric_first(path) { "-twim.split" } else { "-twim" }
}

async fn read_limited(reader: impl AsyncRead + Unpin, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(limit + 1).read_to_end(&mut bytes).await?;
    anyhow::ensure!(bytes.len() as u64 <= limit, "WIM 内容超过内部预览范围，请先解压后打开");
    Ok(bytes)
}

async fn bounded_command(path: &Path, entry: Option<&str>, limit: u64, cancelled: &AtomicBool) -> Result<Vec<u8>> {
    anyhow::ensure!(!cancelled.load(Ordering::Relaxed), "WIM 预览已取消");
    let tool = crate::utils::archive_tools::find_7z_command().context("缺少随附 WIM 引擎，请修复安装")?;
    let mut command = crate::utils::process::async_command(tool);
    command.arg(if entry.is_some() { "x" } else { "l" }).arg(type_argument(path));
    command.args(["-p-", "-sccUTF-8", "-mmt=1"]);
    if let Some(entry) = entry { command.args(["-so", "-spd", "-bso0", "-bse2", "-bsp0"]).arg(format!("-i!{entry}")); }
    else { command.args(["-slt", "-ba"]); }
    command.arg("--").arg(path).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    #[cfg(windows)]
    let job = {
        let job = super::video_encoding::WindowsJobObject::create()?;
        job.limit_memory(256 * 1024 * 1024)?;
        job
    };
    let mut child = command.spawn()?;
    #[cfg(windows)]
    if let Err(error) = job.assign_raw_handle(child.raw_handle().context("无法取得 WIM 预览进程")?) { let _ = child.kill().await; return Err(error.into()); }
    let stdout = child.stdout.take().context("无法读取 WIM 预览")?;
    let stderr = child.stderr.take().context("无法读取 WIM 提示")?;
    let result = tokio::select! {
        result = tokio::time::timeout(Duration::from_secs(30), async {
            let (bytes, error, status) = tokio::try_join!(read_limited(stdout, limit), read_limited(stderr, 65536), async { child.wait().await.map_err(anyhow::Error::from) })?;
            anyhow::ensure!(status.success(), "WIM 预览未完成，可能超出内存预算或内容不完整，请先解压后打开：{}", String::from_utf8_lossy(&error));
            Ok::<_, anyhow::Error>(bytes)
        }) => result.context("WIM 预览超过等待时间，请先解压后打开")?,
        _ = async { while !cancelled.load(Ordering::Relaxed) { tokio::time::sleep(Duration::from_millis(25)).await; } } => Err(anyhow::anyhow!("WIM 预览已取消")),
    };
    if result.is_err() { let _ = child.kill().await; }
    result
}

pub(crate) async fn preview_entry(path: &Path, entry: &str, password: Option<&str>, max_output: u64, keep: u64, cancelled: Arc<AtomicBool>) -> Result<(Vec<u8>, u64)> {
    let source = super::split_archive_detector::SplitArchiveDetector::complete_source(path)?;
    anyhow::ensure!(!password.is_some_and(|value| !value.is_empty()), "WIM 不支持原生密码；需要加密时请转换为 ZIP 或其他密码格式");
    let entry = super::archive_entry_open::normalize_safe_entry_path(entry)?;
    let listing = bounded_command(&source, None, 16 * 1024 * 1024, &cancelled).await?;
    let metadata = super::universal_engine::UniversalCliEngine::parse_metadata_listing(std::str::from_utf8(&listing).context("WIM 目录编码无效")?, "WIM".into());
    let selected = metadata.entries.iter().find(|value| value.path == entry && !value.is_dir).context("没有找到所选 WIM 文件")?;
    anyhow::ensure!(selected.size_known != Some(false) && selected.size <= max_output, "所选 WIM 文件大小未知或超过内部预览范围，请先解压后打开");
    let bytes = bounded_command(&source, Some(&entry), max_output, &cancelled).await?;
    anyhow::ensure!(bytes.len() as u64 == selected.size, "WIM 文件内容不完整，未显示部分数据作为完整预览");
    Ok((bytes.into_iter().take(keep as usize).collect(), selected.size))
}
