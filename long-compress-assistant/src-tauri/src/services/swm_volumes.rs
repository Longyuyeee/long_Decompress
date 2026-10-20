//! Native WIM parts are separate resource containers, never concatenated byte slices.
use anyhow::{Context, Result};
use std::{collections::HashMap, fs::{self, File}, io::Read, path::Path};
use super::split_archive_detector::{SplitArchiveInfo, SplitFormat};

#[derive(PartialEq)]
struct Header {
    identity: [u8; 16], version: u32, method: u32, chunk: u32,
    number: usize, total: usize,
}

fn header(path: &Path) -> Result<Header> {
    let mut file = File::open(path).with_context(|| format!("无法读取 SWM 分卷：{}", path.display()))?;
    let size = file.metadata()?.len();
    let mut bytes = [0u8; 208];
    file.read_exact(&mut bytes).with_context(|| format!("[archive-inspection:damaged] SWM 卷头不完整：{}", path.display()))?;
    anyhow::ensure!(&bytes[..8] == b"MSWIM\0\0\0" && u32::from_le_bytes(bytes[8..12].try_into().unwrap()) == 208,
        "[archive-inspection:damaged] 不是有效的原生 SWM 分卷：{}", path.display());
    let version = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
    // Independently verified native SWM: non-solid Copy/XPRESS/LZX. Normal WIM
    // and its numeric volumes keep their existing LZMS reader unchanged.
    let method = u32::from_le_bytes(bytes[16..20].try_into().unwrap()) & 0x003f0002;
    anyhow::ensure!(version == 0x10d00 && [0, 0x20002, 0x40002].contains(&method),
        "[archive-inspection:unsupported] 此 SWM 压缩方式尚未支持；当前支持非固实 Copy、XPRESS、LZX 原生分卷");
    let number = u16::from_le_bytes(bytes[40..42].try_into().unwrap()) as usize;
    let total = u16::from_le_bytes(bytes[42..44].try_into().unwrap()) as usize;
    anyhow::ensure!(number > 0 && number <= total,
        "[archive-inspection:damaged] SWM 卷号或总卷数无效：{}", path.display());
    // Blob table, XML, boot metadata and integrity resources belong to THIS
    // physical part. Catch truncated files before a misleading successful list.
    for start in [48, 72, 96, 124] {
        let length = u64::from_le_bytes(bytes[start..start+8].try_into().unwrap()) & 0x00ff_ffff_ffff_ffff;
        let offset = u64::from_le_bytes(bytes[start+8..start+16].try_into().unwrap());
        anyhow::ensure!(length == 0 || (offset >= 208 && offset.checked_add(length).is_some_and(|end| end <= size)),
            "[archive-inspection:damaged] SWM 分卷资源不完整，文件可能被截断：{}", path.display());
    }
    Ok(Header { identity: bytes[24..40].try_into().unwrap(), version, method,
        chunk: u32::from_le_bytes(bytes[20..24].try_into().unwrap()), number, total })
}

pub(crate) fn inspect(path: &Path) -> Result<Option<SplitArchiveInfo>> {
    if !path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("swm")) || !path.is_file() { return Ok(None); }
    let selected = header(path)?;
    let stem = path.file_stem().and_then(|name| name.to_str()).ok_or_else(|| anyhow::anyhow!("无效的 SWM 文件名"))?;
    let base = if selected.number == 1 { stem } else {
        stem.strip_suffix(&selected.number.to_string()).ok_or_else(|| anyhow::anyhow!(
            "[archive-inspection:unsupported] SWM 分卷名称与卷号不符；请保留原始名称，如 install.swm、install2.swm，再打开任意一卷"))?
    };
    anyhow::ensure!(!base.is_empty(), "[archive-inspection:unsupported] SWM 缺少原始文件名前缀，请补回原名后重试");
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let extension = path.extension().unwrap().to_string_lossy();
    let mut names = HashMap::<String, Vec<_>>::new();
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            names.entry(entry.file_name().to_string_lossy().to_lowercase()).or_default().push(entry.path());
        }
    }
    let expected_name = |number: usize| if number == 1 { format!("{base}.{extension}") } else { format!("{base}{number}.{extension}") };
    let mut parts = Vec::new(); let mut missing_parts = Vec::new(); let mut total_size = 0;
    for number in 1..=selected.total {
        let expected = expected_name(number);
        if let Some(found) = names.get(&expected.to_lowercase()) {
            anyhow::ensure!(found.len() == 1, "[archive-inspection:damaged] SWM 分卷名称重复：{expected}");
            let part = &found[0]; let current = header(part)?;
            anyhow::ensure!(current.identity == selected.identity && current.total == selected.total
                && current.version == selected.version && current.method == selected.method && current.chunk == selected.chunk,
                "[archive-inspection:damaged] SWM 分卷来自不同镜像或卷组：{}；请放回同一组原始分卷", part.display());
            anyhow::ensure!(current.number == number,
                "[archive-inspection:damaged] SWM 文件名与内部卷号不符：{}；请恢复原始名称", part.display());
            total_size += fs::metadata(part)?.len(); parts.push(part.clone());
        } else { missing_parts.push(parent.join(expected)); }
    }
    let first_part = names.get(&expected_name(1).to_lowercase()).map(|paths| paths[0].clone())
        .unwrap_or_else(|| parent.join(expected_name(1)));
    Ok(Some(SplitArchiveInfo { format: SplitFormat::WimSplit, base_name: base.into(), parts, first_part,
        total_parts: selected.total, total_size, is_complete: missing_parts.is_empty(), missing_parts }))
}
