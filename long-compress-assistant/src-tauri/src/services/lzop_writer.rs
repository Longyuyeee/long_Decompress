//! Bounded LZO1X blocks in an interoperable LZOP container, using lzokay's fixed encoder.
use anyhow::{Context, Result};
use std::{io::{Read, Write}, path::Path};
use super::native_compression::CompressionRuntime;

pub(crate) const BLOCK_BYTES: usize = 256 * 1024;
#[derive(Debug, Default)]
pub(crate) struct Summary { pub input_bytes: u64, pub blocks: u64, pub stored_blocks: u64 }

pub(crate) fn write_stream(
    input: &mut impl Read, output: &mut impl Write, name: Option<&str>,
    mut progress: impl FnMut(u64) -> Result<()>,
) -> Result<Summary> {
    progress(0)?;
    // A root-relative member path retains the source basename when the user
    // chooses a different output archive name. No absolute source path is stored.
    // LZOP's name field is one byte long; very long UTF-8 names use the output
    // filename convention instead of truncating or replacing characters.
    let name = name.filter(|n| !n.is_empty()).map(|n| format!("./{n}"))
        .filter(|n| n.len() <= 255).unwrap_or_default();
    let flags: u32 = 0x03000000 | 0x00500000 | 0x1300 | if name.is_empty() { 0 } else { 0x2000 };
    let mut header = Vec::with_capacity(300);
    header.extend(0x1030u16.to_be_bytes()); // LZOP framing version
    header.extend(0u16.to_be_bytes()); // no liblzo dependency/version claim
    header.extend(0x1001u16.to_be_bytes()); // UTF-8/path flag support
    header.extend([3, 9]); // fixed LZO1X-999-compatible encoding, no adjustable levels
    header.extend(flags.to_be_bytes());
    header.extend(0o100644u32.to_be_bytes());
    header.extend(0u32.to_be_bytes()); header.extend(0u32.to_be_bytes());
    header.push(name.len() as u8); header.extend(name.as_bytes());
    output.write_all(super::lzop::MAGIC)?;
    output.write_all(&header)?; output.write_all(&crc32fast::hash(&header).to_be_bytes())?;
    let mut dictionary = lzokay::compress::Dict::new();
    let mut raw = vec![0; BLOCK_BYTES];
    let mut compressed = vec![0; lzokay::compress::compress_worst_size(BLOCK_BYTES)];
    let mut summary = Summary::default();
    loop {
        let mut length = 0;
        while length < raw.len() {
            progress(summary.input_bytes)?;
            match input.read(&mut raw[length..]) {
                Ok(0) => break,
                Ok(n) => length += n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.into()),
            }
        }
        if length == 0 { break; }
        progress(summary.input_bytes)?;
        let packed = lzokay::compress::compress_no_alloc(&raw[..length], &mut compressed, &mut dictionary)
            .map_err(|e| anyhow::anyhow!("LZOP 编码未完成：{e:?}"))?;
        progress(summary.input_bytes)?;
        let payload = if packed < length { &compressed[..packed] } else {
            summary.stored_blocks += 1; &raw[..length]
        };
        output.write_all(&(length as u32).to_be_bytes())?;
        output.write_all(&(payload.len() as u32).to_be_bytes())?;
        output.write_all(&crc32fast::hash(&raw[..length]).to_be_bytes())?;
        if packed < length { output.write_all(&crc32fast::hash(payload).to_be_bytes())?; }
        output.write_all(payload)?;
        summary.input_bytes = summary.input_bytes.checked_add(length as u64).context("LZOP 输入大小超出范围")?;
        summary.blocks += 1;
        progress(summary.input_bytes)?;
    }
    progress(summary.input_bytes)?;
    output.write_all(&0u32.to_be_bytes())?;
    output.flush()?;
    Ok(summary)
}

pub(crate) fn compress<R: CompressionRuntime>(runtime: &R, window: Option<&tauri::Window>, task_id: &str,
    sources: &[String], output: &str, _options: crate::models::compression::CompressionOptions) -> Result<()> {
    anyhow::ensure!(sources.len() == 1 && Path::new(&sources[0]).is_file(),
        "LZOP 只支持一个普通文件；目录或多个文件请选择 ZIP、7Z 或 TAR 组合，不能遗漏其他内容");
    runtime.check_cancellation()?;
    let source = Path::new(&sources[0]);
    let mut input = std::fs::File::open(source)?;
    let total = input.metadata()?.len();
    if source.file_name().and_then(|n|n.to_str()).is_some_and(|n|n.len() > 253) {
        if let Some(window)=window { runtime.emit_log(window,task_id,"名称超过 LZOP 头部长度；取回时沿用输出文件名，请保留原扩展名。",crate::models::compression::TaskLogSeverity::Info); }
    }
    if let Some(parent) = Path::new(output).parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let mut output = std::fs::File::create(output)?;
    let mut last = 0;
    write_stream(&mut input, &mut output, source.file_name().and_then(|n|n.to_str()), |processed| {
        runtime.check_cancellation()?;
        if processed.saturating_sub(last) >= 1024 * 1024 || processed == total && processed != last {
            last = processed;
            if let Some(window)=window { runtime.emit_progress(window,task_id,
                if total == 0 {0.99} else {(processed as f32 / total as f32).min(0.99)},
                Some(source.to_string_lossy().into_owned()),processed,total); }
        }
        Ok(())
    })?;
    runtime.check_cancellation()?;
    output.sync_all()?;
    Ok(())
}
