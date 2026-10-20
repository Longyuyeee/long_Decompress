//! Optional, unprivileged XPRESS/LZX/LZMS encoder. Legacy store requests keep their original route.
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{io::{Read, Write}, path::{Path, PathBuf}, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc}, time::Duration};
use super::compression_service::CompressionError;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WimMethod { #[default] Store, Xpress, Lzx, Lzms }

impl WimMethod {
    pub fn label(self) -> &'static str { match self { Self::Store => "仅存储", Self::Xpress => "XPRESS", Self::Lzx => "LZX", Self::Lzms => "LZMS" } }
}

pub fn command_path() -> Option<PathBuf> {
    let mut roots = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources")];
    if let Ok(exe) = std::env::current_exe() { if let Some(parent) = exe.parent() { roots.insert(0,parent.join("resources")); roots.insert(0,parent.into()); } }
    roots.into_iter().map(|p|p.join("wim-engine/wimlib-imagex.exe")).find(|p|p.is_file() && p.with_file_name("libwim-15.dll").is_file())
}

fn source_list(sources: &[String], preserve_paths: bool, cancelled: &AtomicBool) -> Result<(Vec<u8>,u64)> {
    let entries = super::compression_entries::collect(sources,preserve_paths,true)?;
    let mut list = String::from("\u{feff}");
    let mut input_bytes=0u64;
    for entry in entries {
        ensure!(!cancelled.load(Ordering::Relaxed), CompressionError::Cancelled);
        // File targets create their parent directories. Only list genuinely empty
        // directories; recursively listing populated ones would recapture children.
        if entry.is_dir && std::fs::read_dir(&entry.path)?.next().transpose()?.is_some() { continue; }
        if !entry.is_dir { input_bytes=input_bytes.checked_add(entry.path.metadata()?.len()).context("WIM 来源大小超过可统计范围")?; }
        let absolute = std::path::absolute(&entry.path)?;
        let source = absolute.to_str().context("WIM 来源路径无法表示为 Unicode")?;
        ensure!(!source.chars().chain(entry.archive_name.chars()).any(|c| matches!(c,'"'|'\r'|'\n'|'\0')), "WIM 来源路径含无法写入列表的字符");
        list.push_str(&format!("\"{source}\" \"/{}\"\n",entry.archive_name));
    }
    ensure!(list.len()>3,"没有可归档的普通文件或空目录");
    Ok((list.into_bytes(),input_bytes))
}

fn append_tail(tail: &mut Vec<u8>, bytes: &[u8]) {
    const LIMIT: usize = 16*1024;
    tail.extend_from_slice(bytes);
    if tail.len()>LIMIT { tail.drain(..tail.len()-LIMIT); }
}

fn progress(line: &[u8]) -> Option<f32> {
    let text = std::str::from_utf8(line).ok()?;
    if !text.contains("Archiving file data:") { return None; }
    let start = text.rfind('(')?+1;
    let percent: f32 = text[start..].split('%').next()?.parse().ok()?;
    (0.0..=100.0).contains(&percent).then_some(percent/100.0)
}

pub fn create(sources: &[String], output: &Path, preserve_paths: bool, method: WimMethod, cancelled: Arc<AtomicBool>, on_progress: impl Fn(f32)) -> Result<u64> {
    ensure!(!cancelled.load(Ordering::Relaxed),CompressionError::Cancelled);
    ensure!(!output.exists(),"WIM 临时输出已存在，未覆盖旧文件");
    let compression = match method { WimMethod::Xpress => "--compress=XPRESS", WimMethod::Lzx => "--compress=LZX", WimMethod::Lzms => "--compress=LZMS", WimMethod::Store => anyhow::bail!("仅存储 WIM 应使用原有创建通道") };
    let threads = if method == WimMethod::Lzms { "--threads=1" } else { "--threads=2" };
    let tool = command_path().context("缺少随附的 WIM 压缩编码器，请修复安装，或选择仅存储方式")?;
    let temporary = tempfile::tempdir()?;
    let list = temporary.path().join("sources.txt");
    let (list_bytes,input_bytes)=source_list(sources,preserve_paths,&cancelled)?;
    std::fs::File::create(&list)?.write_all(&list_bytes)?;
    let config = temporary.path().join("capture.ini");
    std::fs::write(&config,b"[ExclusionList]\n")?;
    if let Some(parent)=output.parent() { std::fs::create_dir_all(parent)?; }
    let mut command = crate::utils::process::command(tool);
    command.arg("capture").arg(&list).arg(output).arg("Long archive")
        .args(["--source-list",compression,"--chunk-size=32768","--no-acls","--check",threads])
        .arg(format!("--config={}",config.display()))
        .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    let mut child = command.spawn().context("无法启动 WIM 压缩编码器")?;
    let mut stdout = child.stdout.take().context("无法读取 WIM 编码进度")?;
    let mut stderr = child.stderr.take().context("无法读取 WIM 编码错误")?;
    let (tx,rx) = mpsc::sync_channel(8);
    let output_reader = std::thread::spawn(move || {
        let mut buffer=[0u8;4096];let mut tail=Vec::new();let mut line=Vec::new();
        loop { match stdout.read(&mut buffer) {
            Ok(0)|Err(_)=>break,
            Ok(n)=>{append_tail(&mut tail,&buffer[..n]);for &b in &buffer[..n] { if matches!(b,b'\r'|b'\n') { if let Some(p)=progress(&line){let _=tx.try_send(p);}line.clear(); } else if line.len()<4096 { line.push(b); } }}
        }}tail
    });
    let error_reader = std::thread::spawn(move || { let mut tail=Vec::new();let mut buffer=[0u8;4096];while let Ok(n)=stderr.read(&mut buffer){if n==0{break}append_tail(&mut tail,&buffer[..n]);}tail });
    let mut suspended=false;let mut last=0.0f32;
    let result=(|| -> Result<_> { loop {
        super::task_control::sync_child_pause(&cancelled,child.id(),&mut suspended).map_err(anyhow::Error::msg)?;
        ensure!(!cancelled.load(Ordering::Relaxed),CompressionError::Cancelled);
        for p in rx.try_iter() { if p>=last {last=p;on_progress(p.min(0.99));} }
        if let Some(status)=child.try_wait()? { break Ok(status); }
        std::thread::sleep(Duration::from_millis(50));
    } })();
    if result.is_err() { let _=child.kill();let _=child.wait(); }
    let stdout=output_reader.join().unwrap_or_default();let stderr=error_reader.join().unwrap_or_default();
    let result=result.and_then(|status| {
        ensure!(status.success(),"WIM {} 编码失败（{}）：{}",method.label(),status.code().unwrap_or(-1),String::from_utf8_lossy(if stderr.is_empty(){&stdout}else{&stderr}).trim());
        ensure!(!cancelled.load(Ordering::Relaxed),CompressionError::Cancelled);
        ensure!(output.is_file(),"WIM 编码器未生成输出文件");Ok(())
    });
    if result.is_err() { let _=std::fs::remove_file(output); }
    result.map(|_|input_bytes)
}

#[cfg(test)]
#[path = "wim_xpress_1531_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "wim_lzx_1532_tests.rs"]
mod lzx_tests;

#[cfg(test)]
#[path = "wim_lzms_1533_tests.rs"]
mod lzms_tests;
