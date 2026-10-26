//! Bounded LZO1X blocks in an interoperable LZOP container, using lzokay's fixed encoder.
use super::native_compression::CompressionRuntime;
use anyhow::{Context, Result};
use std::{
    collections::HashSet,
    io::{Read, Write},
    path::Path,
};

pub(crate) const BLOCK_BYTES: usize = 256 * 1024;
#[derive(Debug, Default)]
pub(crate) struct Summary {
    pub input_bytes: u64,
    pub blocks: u64,
    pub stored_blocks: u64,
}

pub(crate) fn write_stream(
    input: &mut impl Read,
    output: &mut impl Write,
    name: Option<&str>,
    progress: impl FnMut(u64) -> Result<()>,
) -> Result<Summary> {
    write_member(input, output, name, false, progress)
}

fn write_member(
    input: &mut impl Read,
    output: &mut impl Write,
    name: Option<&str>,
    multipart: bool,
    mut progress: impl FnMut(u64) -> Result<()>,
) -> Result<Summary> {
    progress(0)?;
    // Store a relative name without a './' component: official lzop rejects
    // dot components on read. Never truncate a multi-member name to fit u8.
    let name = name
        .filter(|n| !n.is_empty() && n.len() <= 255)
        .unwrap_or_default();
    anyhow::ensure!(
        !multipart || !name.is_empty(),
        "LZOP 多成员需要完整文件名；超长名称请选择 TAR.LZO"
    );
    let flags: u32 = 0x03000000
        | 0x00500000
        | 0x1300
        | if name.is_empty() { 0 } else { 0x2010 }
        | if multipart { 0x400 } else { 0 };
    let mut header = Vec::with_capacity(300);
    header.extend(0x1030u16.to_be_bytes()); // LZOP framing version
    header.extend(0u16.to_be_bytes()); // no liblzo dependency/version claim
    header.extend(0x1001u16.to_be_bytes()); // UTF-8/path flag support
    header.extend([3, 9]); // fixed LZO1X-999-compatible encoding, no adjustable levels
    header.extend(flags.to_be_bytes());
    header.extend(0o100644u32.to_be_bytes());
    header.extend(0u32.to_be_bytes());
    header.extend(0u32.to_be_bytes());
    header.push(name.len() as u8);
    header.extend(name.as_bytes());
    output.write_all(super::lzop::MAGIC)?;
    output.write_all(&header)?;
    output.write_all(&crc32fast::hash(&header).to_be_bytes())?;
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
        if length == 0 {
            break;
        }
        progress(summary.input_bytes)?;
        let packed =
            lzokay::compress::compress_no_alloc(&raw[..length], &mut compressed, &mut dictionary)
                .map_err(|e| anyhow::anyhow!("LZOP 编码未完成：{e:?}"))?;
        progress(summary.input_bytes)?;
        let payload = if packed < length {
            &compressed[..packed]
        } else {
            summary.stored_blocks += 1;
            &raw[..length]
        };
        output.write_all(&(length as u32).to_be_bytes())?;
        output.write_all(&(payload.len() as u32).to_be_bytes())?;
        output.write_all(&crc32fast::hash(&raw[..length]).to_be_bytes())?;
        if packed < length {
            output.write_all(&crc32fast::hash(payload).to_be_bytes())?;
        }
        output.write_all(payload)?;
        summary.input_bytes = summary
            .input_bytes
            .checked_add(length as u64)
            .context("LZOP 输入大小超出范围")?;
        summary.blocks += 1;
        progress(summary.input_bytes)?;
    }
    progress(summary.input_bytes)?;
    output.write_all(&0u32.to_be_bytes())?;
    output.flush()?;
    Ok(summary)
}

/// Plan every member before creating output. LZOP cannot express empty folders.
pub(crate) fn prepare_members(
    sources: &[String],
) -> Result<Vec<super::compression_entries::CompressionEntry>> {
    anyhow::ensure!(
        !sources.is_empty() && sources.len() <= 100_000,
        "请选择 1～100000 个普通文件创建 LZOP"
    );
    anyhow::ensure!(
        sources.iter().all(|p| Path::new(p).is_file()),
        "LZOP 只接收普通文件；含文件夹的内容请选择 TAR.LZO，以保留目录和空文件夹"
    );
    let mut used = HashSet::new();
    let mut entries = Vec::with_capacity(sources.len());
    for source in sources {
        let path = Path::new(source);
        let raw = path
            .file_name()
            .and_then(|n| n.to_str())
            .context("LZOP 文件名无法按 UTF-8 保存，请选择 TAR.LZO")?;
        let mut name = raw.to_owned();
        let mut index = 1;
        while !used.insert(name.to_lowercase()) {
            let stem = path.file_stem().and_then(|n| n.to_str()).unwrap_or(raw);
            name = match path
                .extension()
                .and_then(|n| n.to_str())
                .filter(|n| !n.is_empty())
            {
                Some(ext) => format!("{stem} ({index}).{ext}"),
                None => format!("{stem} ({index})"),
            };
            index += 1;
        }
        anyhow::ensure!(
            sources.len() == 1 || name.len() <= 255,
            "LZOP 多成员文件名“{name}”超过 255 字节，无法完整保存；请选择 TAR.LZO，所选文件未修改"
        );
        entries.push(super::compression_entries::CompressionEntry {
            path: path.to_owned(),
            archive_name: name,
            is_dir: false,
        });
    }
    Ok(entries)
}

pub(crate) fn compress<R: CompressionRuntime>(
    runtime: &R,
    window: Option<&tauri::Window>,
    task_id: &str,
    sources: &[String],
    output: &str,
    _options: crate::models::compression::CompressionOptions,
) -> Result<()> {
    runtime.check_cancellation()?;
    let entries = prepare_members(sources)?;
    let total = entries.iter().try_fold(0u64, |sum, e| {
        sum.checked_add(e.path.metadata()?.len())
            .context("LZOP 输入大小超出范围")
    })?;
    if entries.len() == 1 && entries[0].archive_name.len() > 255 {
        if let Some(window) = window {
            runtime.emit_log(
                window,
                task_id,
                "名称超过 LZOP 头部长度；取回时沿用输出文件名，请保留原扩展名。",
                crate::models::compression::TaskLogSeverity::Info,
            );
        }
    }
    if let Some(parent) = Path::new(output)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut output = std::fs::File::create(output)?;
    let mut last = 0;
    let mut completed = 0u64;
    for entry in &entries {
        runtime.check_cancellation()?;
        let mut input = std::fs::File::open(&entry.path)?;
        let stats = write_member(
            &mut input,
            &mut output,
            Some(&entry.archive_name),
            entries.len() > 1,
            |member_bytes| {
                let processed = completed.saturating_add(member_bytes);
                runtime.check_cancellation()?;
                if processed.saturating_sub(last) >= 1024 * 1024
                    || processed == total && processed != last
                {
                    last = processed;
                    if let Some(window) = window {
                        runtime.emit_progress(
                            window,
                            task_id,
                            if total == 0 {
                                0.99
                            } else {
                                (processed as f32 / total as f32).min(0.99)
                            },
                            Some(entry.archive_name.clone()),
                            processed,
                            total,
                        );
                    }
                }
                Ok(())
            },
        )?;
        completed = completed
            .checked_add(stats.input_bytes)
            .context("LZOP 输入大小超出范围")?;
    }
    runtime.check_cancellation()?;
    output.sync_all()?;
    Ok(())
}
