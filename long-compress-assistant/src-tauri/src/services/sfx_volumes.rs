//! Original external-volume SFX layouts. Input executables are never launched.
use super::{
    archive_format::ArchiveFormat,
    split_archive_detector::{SplitArchiveDetector, SplitArchiveInfo, SplitFormat},
};
use anyhow::{ensure, Result};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

/// Official 7-Zip 24.09 separate GUI loader. Do not trust arbitrary EXEs merely
/// because there is a similarly named archive. New modules require originals.
pub(crate) fn seven_zip_loader_data(path: &Path) -> Result<Option<PathBuf>> {
    if !super::sfx_archive::is_executable_path(path) || path.metadata()?.len() != 213504 {
        return Ok(None);
    }
    use sha2::{Digest, Sha256};
    if format!("{:x}", Sha256::digest(fs::read(path)?))
        != "97b304ccf831e83ca58b5c99a736f6525f81378125143e92b76baea0988ec5e2"
    {
        return Ok(None);
    }
    Ok(Some(path.with_file_name(format!(
        "{}.7z.001",
        path.file_stem().unwrap().to_string_lossy()
    ))))
}
fn damaged(detail: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "[archive-inspection:damaged] RAR 卷头损坏：{detail}；请重新下载原卷，更换密码不能修复卷头"
    )
}
fn incomplete(detail: &str) -> anyhow::Error {
    anyhow::anyhow!("[archive-inspection:incomplete] RAR 卷不完整：{detail}；请补齐原卷或重新下载")
}
pub(crate) fn validate_native_volume(path: &Path, volume: bool) -> Result<()> {
    if !volume && super::sfx_archive::is_executable_path(path) {
        ensure!(
            !inspect(path)?.is_some_and(|g| g.format == SplitFormat::RarSplit && g.parts.len() > 1),
            damaged("首文件为独立压缩包，不能与同名外置卷混用")
        );
    }
    Ok(())
}
fn vint(bytes: &[u8], position: &mut usize) -> Result<u64> {
    let mut value = 0u64;
    for shift in (0..70).step_by(7) {
        let b = *bytes
            .get(*position)
            .ok_or_else(|| damaged("变长字段被截断"))?;
        *position += 1;
        ensure!(shift != 63 || b <= 1, damaged("变长字段溢出"));
        value |= u64::from(b & 127) << shift;
        if b & 128 == 0 {
            return Ok(value);
        }
    }
    Err(damaged("变长字段过长"))
}
/// Only CRC-checked visible headers. Encrypted headers cannot reveal a reliable
/// final volume count without a password: None is unknown, never 'complete'.
fn next_volume(path: &Path, expected_index: usize) -> Result<Option<bool>> {
    let mut file = File::open(path)?;
    if super::sfx_archive::is_executable_path(path) {
        let Some((ArchiveFormat::Rar, offset)) = super::sfx_archive::detect(path)? else {
            return Err(damaged("EXE 中未找到有效 RAR 头"));
        };
        file.seek(SeekFrom::Start(offset))?;
    }
    let mut magic = [0u8; 7];
    file.read_exact(&mut magic)
        .map_err(|_| incomplete("签名被截断"))?;
    let rar4 = &magic == b"Rar!\x1a\x07\0";
    if !rar4 {
        ensure!(&magic == b"Rar!\x1a\x07\x01", damaged("不是 RAR 卷"));
        let mut zero = [0];
        file.read_exact(&mut zero)?;
        ensure!(zero == [0], damaged("签名无效"));
    }
    let length = file.metadata()?.len();
    let mut split_after = None;
    for _ in 0..1_000_000 {
        if file.stream_position()? == length {
            return Ok(split_after);
        }
        let data = if rar4 {
            let mut prefix = [0u8; 7];
            file.read_exact(&mut prefix)
                .map_err(|_| incomplete("头部被截断"))?;
            let size = u16::from_le_bytes([prefix[5], prefix[6]]) as usize;
            ensure!(size >= 7, damaged("头部长度无效"));
            let mut bytes = vec![0; size];
            bytes[..7].copy_from_slice(&prefix);
            file.read_exact(&mut bytes[7..])
                .map_err(|_| incomplete("头部长度超出文件"))?;
            ensure!(
                crc32fast::hash(&bytes[2..]) as u16 == u16::from_le_bytes([bytes[0], bytes[1]]),
                damaged("CRC 不匹配")
            );
            let flags = u16::from_le_bytes([bytes[3], bytes[4]]);
            if bytes[2] == 0x73 && expected_index > 0 {
                ensure!(flags & 1 != 0, damaged("同名文件不是原生分卷"));
                ensure!(
                    (flags & 0x100 != 0) == (expected_index == 1),
                    damaged("首卷标记与卷序号不一致，可能混用卷组")
                );
            }
            if bytes[2] == 0x73 && flags & 0x80 != 0 {
                return Ok(None);
            }
            if bytes[2] == 0x7b {
                return Ok(Some(flags & 1 != 0));
            }
            if bytes[2] == 0x74 {
                split_after = Some(flags & 2 != 0);
            }
            let mut data = if flags & 0x8000 != 0 {
                ensure!(size >= 11, damaged("数据长度缺失"));
                u32::from_le_bytes(bytes[7..11].try_into()?) as u64
            } else {
                0
            };
            if bytes[2] == 0x74 && flags & 0x100 != 0 {
                ensure!(size >= 40, damaged("大文件长度缺失"));
                data |= (u32::from_le_bytes(bytes[32..36].try_into()?) as u64) << 32;
            }
            data
        } else {
            let mut crc = [0; 4];
            file.read_exact(&mut crc)
                .map_err(|_| incomplete("CRC 被截断"))?;
            let mut encoded = Vec::new();
            loop {
                let mut b = [0];
                file.read_exact(&mut b)
                    .map_err(|_| incomplete("长度被截断"))?;
                encoded.push(b[0]);
                ensure!(encoded.len() <= 3, damaged("头部超过 2 MiB"));
                if b[0] & 128 == 0 {
                    break;
                }
            }
            let size = vint(&encoded, &mut 0)? as usize;
            ensure!(size > 0 && size <= 2 * 1024 * 1024, damaged("头部长度无效"));
            let mut bytes = vec![0; encoded.len() + size];
            bytes[..encoded.len()].copy_from_slice(&encoded);
            file.read_exact(&mut bytes[encoded.len()..])
                .map_err(|_| incomplete("头部长度超出文件"))?;
            ensure!(
                crc32fast::hash(&bytes) == u32::from_le_bytes(crc),
                damaged("CRC 不匹配")
            );
            let mut pos = encoded.len();
            let kind = vint(&bytes, &mut pos)?;
            let flags = vint(&bytes, &mut pos)?;
            if flags & 1 != 0 {
                vint(&bytes, &mut pos)?;
            }
            let data = if flags & 2 != 0 {
                vint(&bytes, &mut pos)?
            } else {
                0
            };
            if kind == 1 && expected_index > 0 {
                let archive_flags = vint(&bytes, &mut pos)?;
                ensure!(archive_flags & 1 != 0, damaged("同名文件不是原生分卷"));
                let index = if archive_flags & 2 != 0 {
                    vint(&bytes, &mut pos)? + 1
                } else {
                    1
                };
                ensure!(
                    index == expected_index as u64,
                    damaged("卷内序号与文件名不一致，可能混用卷组")
                );
            }
            if kind == 4 {
                return Ok(None);
            }
            if kind == 5 {
                return Ok(Some(vint(&bytes, &mut pos)? & 1 != 0));
            }
            if kind == 2 {
                split_after = Some(flags & 16 != 0);
            }
            data
        };
        let end = file
            .stream_position()?
            .checked_add(data)
            .ok_or_else(|| damaged("数据长度溢出"))?;
        ensure!(end <= length, incomplete("压缩数据被截断"));
        file.seek(SeekFrom::Start(end))?;
    }
    Err(damaged("头部数量超过读取上限"))
}
pub(crate) fn inspect(path: &Path) -> Result<Option<SplitArchiveInfo>> {
    if let Some(data) = seven_zip_loader_data(path)? {
        return Ok(Some(
            SplitArchiveDetector::detect_split_archive(&data)?.unwrap_or_else(|| {
                SplitArchiveInfo {
                    format: SplitFormat::SevenZipSplit,
                    base_name: data.file_stem().unwrap().to_string_lossy().into_owned(),
                    parts: vec![],
                    first_part: data.clone(),
                    total_parts: 1,
                    total_size: 0,
                    is_complete: false,
                    missing_parts: vec![data],
                }
            }),
        ));
    }
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return Ok(None);
    };
    let modern = regex::Regex::new(r"(?i)^(.+)\.part(\d+)\.(rar|exe)$")?;
    let legacy = regex::Regex::new(r"(?i)^(.+)\.(r\d{2,3}|exe)$")?;
    let (base, is_modern, width) = if let Some(c) = modern.captures(name) {
        (c[1].to_string(), true, c[2].len())
    } else if let Some(c) = legacy.captures(name) {
        (c[1].to_string(), false, 2)
    } else {
        return Ok(None);
    };
    let parent = path.parent().unwrap_or(Path::new("."));
    let first_name = if is_modern {
        format!(r"(?i)^{}\.part0*1\.exe$", regex::escape(&base))
    } else {
        format!(r"(?i)^{}\.exe$", regex::escape(&base))
    };
    let first_match = regex::Regex::new(&first_name)?;
    let files = fs::read_dir(parent)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    let firsts: Vec<_> = files
        .iter()
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| first_match.is_match(n))
        })
        .collect();
    ensure!(
        firsts.len() <= 1,
        "[archive-inspection:damaged] 自解压首卷序号重复，请保留同一组原始分卷"
    );
    let Some(first) = firsts.first().copied() else {
        return Ok(None);
    };
    if !super::sfx_archive::detect(first)?.is_some_and(|(f, _)| f == ArchiveFormat::Rar) {
        return if path == first {
            Ok(None)
        } else {
            Err(damaged("同名 EXE 不是有效 RAR 首卷"))
        };
    }
    let ordinary = if is_modern {
        first.with_extension("rar")
    } else {
        parent.join(format!("{base}.rar"))
    };
    ensure!(!ordinary.exists(), "[archive-inspection:damaged] 同时存在 RAR 和 EXE 首卷，无法确认同一卷组；请分别放入独立目录");
    let pattern = regex::Regex::new(&if is_modern {
        format!(r"(?i)^{}\.part(\d+)\.rar$", regex::escape(&base))
    } else {
        format!(r"(?i)^{}\.r(\d{{2,3}})$", regex::escape(&base))
    })?;
    let mut indexed = Vec::new();
    let mut width = width;
    for p in &files {
        if let Some(c) = p
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| pattern.captures(n))
        {
            let raw: usize = c[1].parse()?;
            let index = if is_modern { raw } else { raw + 2 };
            ensure!(
                (2..=1000).contains(&index),
                "[archive-inspection:damaged] 外置卷序号无效或超过 1000 卷"
            );
            width = c[1].len();
            indexed.push((index, p.clone()));
        }
    }
    indexed.sort_by_key(|(i, _)| *i);
    ensure!(
        indexed.windows(2).all(|p| p[0].0 != p[1].0),
        "[archive-inspection:damaged] 外置卷序号重复，请使用同一组原始分卷"
    );
    let first_next = next_volume(first, if indexed.is_empty() { 0 } else { 1 })?;
    if indexed.is_empty() && first_next != Some(true) {
        return Ok(None);
    }
    let mut total = indexed.last().map(|(i, _)| *i).unwrap_or(2);
    let mut last_next = None;
    for (index, member) in &indexed {
        last_next = next_volume(member, *index)?;
    }
    if last_next == Some(true) {
        total += 1;
    }
    let mut parts = vec![first.clone()];
    parts.extend(indexed.iter().map(|(_, p)| p.clone()));
    let missing_parts: Vec<_> = (2..=total)
        .filter(|i| !indexed.iter().any(|(found, _)| found == i))
        .map(|i| {
            parent.join(if is_modern {
                format!("{base}.part{i:0width$}.rar")
            } else {
                format!("{base}.r{:0width$}", i - 2)
            })
        })
        .collect();
    let total_size = parts.iter().try_fold(0u64, |sum, p| -> Result<u64> {
        let m = p.metadata()?;
        ensure!(m.is_file() && m.len() > 0, incomplete("卷不是非空普通文件"));
        sum.checked_add(m.len())
            .ok_or_else(|| damaged("卷大小溢出"))
    })?;
    Ok(Some(SplitArchiveInfo {
        format: SplitFormat::RarSplit,
        base_name: base,
        first_part: first.clone(),
        total_parts: total,
        total_size,
        is_complete: missing_parts.is_empty(),
        missing_parts,
        parts,
    }))
}
