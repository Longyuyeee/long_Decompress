//! Read only the checksum-confirmed LZOP prefix. Never search for a later TAR header.
use super::{
    recovery::{check, Entry, Report, Source},
    LzopReader, ADLER_D, CRC_D,
};
use crate::services::{
    extraction_transaction::MAX_EXTRACTED_BYTES, numeric_volume_reader::ZipInputReader,
    zip_password::ZipPasswordEncoding,
};
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Seek, Write},
    path::Path,
    sync::atomic::AtomicBool,
};

struct Verified<'a, F> {
    reader: LzopReader<ZipInputReader>,
    cancelled: &'a AtomicBool,
    progress: F,
    total: u64,
    position: u64,
}
impl<F: FnMut(u64, u64)> Read for Verified<'_, F> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        check(self.cancelled).map_err(io::Error::other)?;
        if self.reader.flags & (ADLER_D | CRC_D) == 0 {
            return Err(io::Error::other(
                "外层 LZOP 未提供内容校验，无法确认内部文件",
            ));
        }
        // LzopReader publishes a block only after decoding and checking its entire data.
        let n = self.reader.read(buf)?;
        self.position = self
            .position
            .checked_add(n as u64)
            .ok_or_else(|| io::Error::other("TAR 大小超出范围"))?;
        if self.position > MAX_EXTRACTED_BYTES {
            return Err(io::Error::other("TAR 展开大小超过本次支持范围"));
        }
        (self.progress)(self.reader.input.stream_position()?, self.total);
        Ok(n)
    }
}
fn padding(reader: &mut impl Read, size: u64) -> Result<()> {
    let count = ((512 - size % 512) % 512) as usize;
    let mut bytes = [0; 512];
    reader
        .read_exact(&mut bytes[..count])
        .context("TAR 条目填充未完整到达")?;
    anyhow::ensure!(bytes[..count].iter().all(|b| *b == 0), "TAR 条目填充无效");
    Ok(())
}
fn parse<F: FnMut(u64, u64)>(
    reader: &mut Verified<'_, F>,
    report: &mut Report,
    encoding: Option<ZipPasswordEncoding>,
) -> Result<()> {
    let mut names = super::names::Names::default();
    let mut pending: Option<crate::services::tar_pax::Fields> = None;
    let mut global = crate::services::tar_pax::State::default();
    let mut long_name: Option<String> = None;
    let mut records = 0usize;
    loop {
        report.stopped_offset = reader.position;
        report.stopped_member = None;
        let mut block = [0u8; 512];
        reader
            .read_exact(&mut block)
            .context("TAR 文件头或结束标记未完整到达")?;
        if block.iter().all(|b| *b == 0) {
            anyhow::ensure!(pending.is_none() && long_name.is_none(), "TAR 在名称元数据之后缺少对应条目");
            reader
                .read_exact(&mut block)
                .context("TAR 第二个结束标记未完整到达")?;
            anyhow::ensure!(block.iter().all(|b| *b == 0), "TAR 结束标记不完整");
            let mut buf = [0; 65536];
            loop {
                let n = reader.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                anyhow::ensure!(
                    buf[..n].iter().all(|b| *b == 0),
                    "TAR 结束后还有非零内容，后续清单未确认"
                );
            }
            anyhow::ensure!(
                reader.reader.input.stream_position()? == reader.total,
                "外层 LZOP 成员结束后仍有数据，后续内容未确认"
            );
            report.complete = true;
            report.stopped_offset = reader.position;
            return Ok(());
        }
        records += 1;
        anyhow::ensure!(
            records <= 200_000 && report.entries.len() < 100_000,
            "TAR 条目超过本次支持范围"
        );
        let header = tar::Header::from_byte_slice(&block);
        let sum = block
            .iter()
            .enumerate()
            .map(|(i, b)| {
                if (148..156).contains(&i) {
                    32
                } else {
                    *b as u32
                }
            })
            .sum::<u32>();
        anyhow::ensure!(header.cksum()? == sum, "TAR 文件头校验失败");
        let kind = header.entry_type();
        anyhow::ensure!(kind.is_pax_global_extensions() || kind.is_pax_local_extensions() || kind.is_gnu_longname() || kind.is_file() || kind.is_dir(), "[archive-inspection:unsupported] TAR.LZO 部分取回暂不支持此 TAR 条目类型（{}），已停止在此处", kind.as_byte());
        let size = header.size()?;
        anyhow::ensure!(size <= MAX_EXTRACTED_BYTES, "TAR 条目大小超过本次支持范围");
        if kind.is_pax_global_extensions() || kind.is_pax_local_extensions() || kind.is_gnu_longname() {
            anyhow::ensure!(
                kind.is_pax_global_extensions()
                    || (kind.is_gnu_longname() && long_name.is_none())
                    || (kind.is_pax_local_extensions() && pending.is_none()),
                "[archive-inspection:unsupported] TAR 名称元数据重复，已停止在此处"
            );
            anyhow::ensure!(size <= 1024 * 1024, "TAR 名称元数据超过本次支持范围");
            let mut bytes = vec![0; size as usize];
            reader
                .read_exact(&mut bytes)
                .context("TAR 名称元数据未完整到达")?;
            padding(reader, size)?;
            // Extension records use their own physical size. Apply their
            // contents only after the checked payload AND padding arrive.
            if kind.is_gnu_longname() {
                let end = bytes.iter().position(|byte| *byte == 0)
                    .context("GNU 长名称缺少结束标记")?;
                anyhow::ensure!(end > 0 && bytes[end..].iter().all(|byte| *byte == 0), "GNU 长名称为空或结束后仍有其他内容");
                long_name = Some(super::names::decode(&bytes[..end], 0, encoding)?
                    .context("[lzop-encoding] GNU 长名称不是 UTF-8，请选择名称编码后重新检查")?);
            } else {
                let fields = crate::services::tar_pax::parse(&bytes)?;
                crate::services::tar_pax::common(&fields)?;
                if kind.is_pax_global_extensions() { global.update(fields)?; }
                else { pending = Some(fields); }
            }
            continue;
        }
        anyhow::ensure!(kind.is_file() || kind.is_dir(),"[archive-inspection:unsupported] TAR.LZO 部分取回暂不支持此 TAR 条目类型（{}），已停止在此处",kind.as_byte());
        // GNU sparse records have a different layout even when their path looks ordinary.
        if let Some(gnu) = header.as_gnu() {
            anyhow::ensure!(
                gnu.isextended[0] == 0 && gnu.sparse.iter().all(|s| s.is_empty()),
                "[archive-inspection:unsupported] TAR 稀疏条目暂不支持部分取回"
            );
        }
        let local = pending.take().unwrap_or_default();
        if let Some(name) = &long_name { crate::services::tar_pax::check_mixed_path(name.as_bytes(), &local)?; }
        let meta = global.effective(&local);
        let size = crate::services::tar_pax::size(&meta)?.unwrap_or(size);
        anyhow::ensure!(
            size <= MAX_EXTRACTED_BYTES && (!kind.is_dir() || size == 0),
            "TAR 条目大小无效"
        );
        let decoded = if let Some(path) = long_name.take().or_else(|| crate::services::tar_pax::path(&meta).map(str::to_owned)) {
            path
        } else {
            super::names::decode(&header.path_bytes(), 0, encoding)?
                .context("[lzop-encoding] TAR 路径不是 UTF-8，请选择名称编码后重新检查")?
        };
        anyhow::ensure!(
            !decoded.starts_with(['/', '\\'])
                && !decoded.contains([':', '\0'])
                && !decoded.split(['/', '\\']).any(|p| p == ".."),
            "TAR 路径无法确认，停止在此处"
        );
        if decoded.split(['/', '\\']).all(|s| s.is_empty() || s == ".") {
            anyhow::ensure!(kind.is_dir(), "TAR 文件名为空");
            continue;
        }
        let name = if kind.is_dir() {
            names.allocate_directory(&decoded, report.entries.len() + 1)
        } else {
            names.allocate(Some(&decoded), 0x2000, report.entries.len() + 1)
        };
        report.stopped_member = Some(name.clone());
        let offset = reader.position;
        let mut remaining = size;
        let mut hash = Sha256::new();
        let mut buf = [0; 65536];
        while remaining > 0 {
            let count = remaining.min(buf.len() as u64) as usize;
            reader
                .read_exact(&mut buf[..count])
                .context("TAR 文件内容未完整到达")?;
            hash.update(&buf[..count]);
            remaining -= count as u64;
        }
        padding(reader, size)?;
        report.entries.push(Entry {
            modified_time: if kind.is_dir() { None } else {
                use crate::services::tar_recovery_time::ModificationTime;
                if let Some((_, value)) = meta.iter().find(|(k, _)| k == "mtime") {
                    ModificationTime::pax(value)
                } else { ModificationTime::header(header) }
            },
            time_warning: None,
            id: report.entries.len(),
            name,
            bytes: size,
            sha256: format!("{:x}", hash.finalize()),
            is_directory: kind.is_dir(),
            offset,
        });
    }
}
pub(super) fn scan_source(
    src: &Source,
    encoding: Option<ZipPasswordEncoding>,
    cancelled: &AtomicBool,
    progress: &mut impl FnMut(u64, u64),
) -> Result<Report> {
    let mut report = Report {
        format: "TAR.LZO",
        revision: String::new(),
        entries: Vec::new(),
        complete: false,
        stop_reason: String::new(),
        stopped_member: None,
        stopped_offset: 0,
        available_volumes: src.paths.len(),
        ignored_volumes: src.ignored,
    };
    let checked = (|| -> Result<()> {
        let reader = LzopReader::with_mode(src.open()?, true)?;
        let mut reader = Verified {
            reader,
            cancelled,
            progress: &mut *progress,
            total: src.length,
            position: 0,
        };
        parse(&mut reader, &mut report, encoding)
    })();
    if let Err(e) = checked {
        check(cancelled)?;
        if crate::services::archive_io::kind(&e).is_some() {
            return Err(e);
        }
        report.stop_reason = format!("{e:#}");
    }
    if let Some(boundary) = &src.boundary {
        report.complete = false;
        report.stop_reason = format!("{}；{boundary}", report.stop_reason);
    }
    if report.stop_reason.is_empty() {
        report.stop_reason = "TAR 条目及外层 LZOP 已完整结束".into();
    }
    check(cancelled)?;
    let mut hash = Sha256::new();
    hash.update(src.stamp.as_bytes());
    hash.update(serde_json::to_vec(&report)?);
    for e in &report.entries {
        hash.update(e.offset.to_le_bytes());
    }
    report.revision = format!("{:x}", hash.finalize());
    progress(src.length, src.length);
    Ok(report)
}
pub(super) fn copy_selected(
    src: &Source,
    stage: &Path,
    entries: &mut [Entry],
    cancelled: &AtomicBool,
    progress: &mut impl FnMut(u64),
) -> Result<()> {
    let reader = LzopReader::with_mode(src.open()?, true)?;
    let mut reader = Verified {
        reader,
        cancelled,
        progress: |_, _| {},
        total: src.length,
        position: 0,
    };
    let mut done = 0;
    let mut buf = [0; 65536];
    for entry in entries {
        check(cancelled)?;
        anyhow::ensure!(entry.offset >= reader.position, "TAR 取回位置无效");
        while reader.position < entry.offset {
            let n = (entry.offset - reader.position).min(buf.len() as u64) as usize;
            reader.read_exact(&mut buf[..n])?;
        }
        let relative = Path::new(&entry.name);
        anyhow::ensure!(
            relative
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_))),
            "TAR 条目路径无法取回"
        );
        let target = stage.join(relative);
        if entry.is_directory {
            fs::create_dir_all(target)?;
            continue;
        }
        if let Some(p) = target.parent() {
            fs::create_dir_all(p)?;
        }
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)?;
        let mut remaining = entry.bytes;
        let mut hash = Sha256::new();
        while remaining > 0 {
            let n = remaining.min(buf.len() as u64) as usize;
            reader.read_exact(&mut buf[..n])?;
            output.write_all(&buf[..n])?;
            hash.update(&buf[..n]);
            remaining -= n as u64;
            done += n as u64;
            progress(done);
        }
        anyhow::ensure!(
            format!("{:x}", hash.finalize()) == entry.sha256,
            "TAR 文件在取回期间已变化，请重新检查"
        );
        output.flush()?;
        check(cancelled)?;
        entry.time_warning = match &entry.modified_time {
            Some(time) => time.apply(&output).err().map(str::to_owned),
            None => Some("原修改时间缺失，或其范围、精度暂不支持；文件内容已取回".into()),
        };
    }
    Ok(())
}
