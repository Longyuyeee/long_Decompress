//! Explicit prefix recovery; normal archive readers continue requiring a complete source.
use super::{LzopReader, ADLER_D, CRC_D};
use crate::services::{
    extraction_transaction,
    numeric_volume_reader::{NumericVolumeReader, ZipInputReader},
    split_archive_detector::{SplitArchiveDetector, SplitFormat},
    zip_password::ZipPasswordEncoding,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: usize,
    pub name: String,
    pub bytes: u64,
    pub sha256: String,
    pub is_directory: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stored_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_target: Option<String>,
    #[serde(skip)]
    pub(crate) link_source: Option<usize>,
    #[serde(skip)]
    pub(crate) sparse: Option<crate::services::tar_sparse_recovery::Layout>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_time: Option<crate::services::tar_recovery_time::ModificationTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_warning: Option<String>,
    #[serde(skip)]
    pub(super) offset: u64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub format: &'static str,
    pub revision: String,
    pub entries: Vec<Entry>,
    pub complete: bool,
    pub stop_reason: String,
    pub stopped_member: Option<String>,
    pub stopped_offset: u64,
    pub available_volumes: usize,
    pub ignored_volumes: usize,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub output_directory: String,
    pub recovered_files: usize,
    pub recovered_directories: usize,
    pub recovered_bytes: u64,
    pub entries: Vec<Entry>,
    pub stop_reason: String,
    pub partial: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_preservation: Option<crate::services::tar_recovery_time::TimePreservation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistorySelection {
    pub recovered_files: usize,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub recovered_directories: usize,
    pub stop_reason: String,
    pub revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_preservation: Option<crate::services::tar_recovery_time::TimePreservation>,
}
fn is_zero(n: &usize) -> bool {
    *n == 0
}
impl HistorySelection {
    pub fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            self.recovered_files <= 100_000
                && self.time_preservation.as_ref().map_or(true, |t| t.valid_for(self.recovered_files, self.recovered_directories))
                && self.recovered_directories <= 100_000
                && self.stop_reason.len() <= 32768
                && self.revision.len() == 64
                && self.revision.bytes().all(|b| b.is_ascii_hexdigit()),
            "无效的 LZOP 部分取回记录"
        );
        Ok(())
    }
}
pub(super) struct Source {
    pub(super) paths: Vec<PathBuf>,
    pub(super) length: u64,
    pub(super) ignored: usize,
    pub(super) boundary: Option<String>,
    pub(super) stamp: String,
}
impl Source {
    pub(super) fn open(&self) -> Result<ZipInputReader> {
        Ok(if self.paths.len() == 1 && !self.numeric() {
            ZipInputReader::File(File::open(&self.paths[0])?)
        } else {
            ZipInputReader::Volumes(NumericVolumeReader::from_paths(self.paths.clone())?)
        })
    }
    fn numeric(&self) -> bool {
        self.paths.first().is_some_and(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.len() >= 3 && s.bytes().all(|b| b.is_ascii_digit()))
        })
    }
}
pub(crate) fn is_candidate(path: &Path) -> bool {
    let logical = if path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.len() >= 3 && s.bytes().all(|b| b.is_ascii_digit()))
    {
        Path::new(path.file_stem().unwrap_or_default())
    } else {
        path
    };
    let name = logical
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    name.ends_with(".lzo")
}
pub(crate) fn is_tar(path: &Path) -> bool {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    name.ends_with(".tar.lzo")
        || name.rsplit_once('.').is_some_and(|(base, n)| {
            n.len() >= 3 && n.bytes().all(|b| b.is_ascii_digit()) && base.ends_with(".tar.lzo")
        })
}
fn source(path: &Path) -> Result<Source> {
    anyhow::ensure!(is_candidate(path), "本入口只支持 LZOP / TAR.LZO 部分取回");
    let mut ignored = 0;
    let mut boundary = None;
    let paths = if let Some(group) = SplitArchiveDetector::detect_split_archive(path)? {
        anyhow::ensure!(
            group.format == SplitFormat::GenericNumeric,
            "只支持 .lzo.001 数字切片"
        );
        let mut numbered = Vec::new();
        let mut numbers = std::collections::HashSet::new();
        for part in group.parts {
            let n = part
                .extension()
                .and_then(|s| s.to_str())
                .and_then(|s| s.parse::<usize>().ok())
                .context("无效的 LZOP 卷号")?;
            anyhow::ensure!(
                n > 0 && numbers.insert(n),
                "LZOP 卷号为 0 或重复，请核对原始分卷"
            );
            numbered.push((n, part));
        }
        numbered.sort_by_key(|p| p.0);
        let count = numbered.len();
        let mut prefix = Vec::new();
        for (n, part) in numbered {
            if n != prefix.len() + 1 || part.metadata()?.len() == 0 {
                boundary = Some(format!(
                    "从第 {} 卷起不连续或为空；仅检查此前连续卷，后续卷不拼接",
                    prefix.len() + 1
                ));
                break;
            }
            prefix.push(part)
        }
        ignored = count - prefix.len();
        anyhow::ensure!(
            !prefix.is_empty(),
            "缺少有效首卷 .001，没有可确认的连续前缀"
        );
        prefix
    } else {
        vec![path.to_path_buf()]
    };
    let mut length = 0u64;
    let mut h = Sha256::new();
    for part in &paths {
        let m = part.metadata()?;
        anyhow::ensure!(m.is_file(), "来源不是普通文件");
        length = length
            .checked_add(m.len())
            .context("LZOP 卷组长度超出范围")?;
        h.update(part.to_string_lossy().as_bytes());
        h.update(m.len().to_le_bytes());
        h.update(format!("{:?}", m.modified()?).as_bytes());
    }
    Ok(Source {
        paths,
        length,
        ignored,
        boundary,
        stamp: format!("{:x}", h.finalize()),
    })
}
pub(crate) fn check(flag: &AtomicBool) -> Result<()> {
    anyhow::ensure!(!flag.load(Ordering::SeqCst), "LZOP recovery cancelled");
    Ok(())
}
fn scan_source(
    src: &Source,
    encoding: Option<ZipPasswordEncoding>,
    cancelled: &AtomicBool,
    progress: &mut impl FnMut(u64, u64),
) -> Result<Report> {
    if src.paths.first().is_some_and(|p| is_tar(p)) {
        return super::tar_recovery::scan_source(src, encoding, cancelled, progress);
    }
    let mut input = src.open()?;
    let mut names = super::names::Names::default();
    let mut report = Report {
        format: "LZOP",
        revision: String::new(),
        entries: Vec::new(),
        complete: false,
        stop_reason: String::new(),
        stopped_member: None,
        stopped_offset: 0,
        available_volumes: src.paths.len(),
        ignored_volumes: src.ignored,
    };
    let mut total = 0u64;
    if src.length == 0 {
        report.stop_reason = "未找到完整 LZOP 文件头".into();
    }
    while input.stream_position()? < src.length {
        check(cancelled)?;
        let offset = input.stream_position()?;
        report.stopped_offset = offset;
        let mut reader = match LzopReader::with_mode(input, true) {
            Ok(r) => r,
            Err(e) => {
                if crate::services::archive_io::io_kind(&e).is_some() {
                    return Err(e.into());
                }
                report.stop_reason = e.to_string();
                break;
            }
        };
        let decoded = match super::names::decode(&reader.name, reader.flags, encoding) {
            Ok(n) => n,
            Err(e) => {
                report.stop_reason = format!("{e:#}");
                break;
            }
        };
        let name = names.allocate(decoded.as_deref(), reader.flags, report.entries.len() + 1);
        report.stopped_member = Some(name.clone());
        let mut bytes = 0u64;
        let mut h = Sha256::new();
        let mut buffer = [0u8; 65536];
        let decoded = (|| -> Result<()> {
            loop {
                check(cancelled)?;
                let n = reader.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                bytes = bytes.checked_add(n as u64).context("成员大小超出范围")?;
                anyhow::ensure!(
                    total.saturating_add(bytes) <= extraction_transaction::MAX_EXTRACTED_BYTES,
                    "LZOP 展开大小超过本次支持范围"
                );
                h.update(&buffer[..n]);
                progress(reader.input.stream_position()?, src.length);
            }
            Ok(())
        })();
        if let Err(e) = decoded {
            check(cancelled)?;
            if crate::services::archive_io::kind(&e).is_some() {
                return Err(e);
            }
            report.stop_reason = format!("{e:#}");
            break;
        }
        if bytes > 0 && reader.flags & (ADLER_D | CRC_D) == 0 {
            report.stop_reason = "此成员未提供内容校验，无法确认完好；停止在此处".into();
            break;
        }
        anyhow::ensure!(
            report.entries.len() < 100_000,
            "LZOP 成员超过 100000 个，请使用原始工具处理"
        );
        total += bytes;
        report.entries.push(Entry {
            id: report.entries.len(),
            name,
            bytes,
            sha256: format!("{:x}", h.finalize()),
            is_directory: false,
            stored_bytes: None,
            link_target: None,
            link_source: None,
            sparse: None,
            modified_time: None,
            time_warning: None,
            offset,
        });
        input = reader.input;
        report.stopped_member = None;
        report.stopped_offset = input.stream_position()?;
    }
    if report.stop_reason.is_empty() {
        if let Some(reason) = &src.boundary {
            report.stop_reason = reason.clone()
        } else {
            report.complete = true;
            report.stop_reason = "现有内容已完整结束；格式未声明总成员数或总卷数".into()
        }
    } else if let Some(reason) = &src.boundary {
        report.stop_reason = format!("{}；{}", report.stop_reason, reason)
    }
    check(cancelled)?;
    let mut h = Sha256::new();
    h.update(src.stamp.as_bytes());
    h.update(serde_json::to_vec(&report)?);
    for e in &report.entries {
        h.update(e.offset.to_le_bytes())
    }
    report.revision = format!("{:x}", h.finalize());
    progress(src.length, src.length);
    Ok(report)
}
pub(crate) fn scan(
    path: &Path,
    encoding: Option<ZipPasswordEncoding>,
    cancelled: &AtomicBool,
    mut progress: impl FnMut(u64, u64),
) -> Result<Report> {
    check(cancelled)?;
    let src = source(path)?;
    let report = scan_source(&src, encoding, cancelled, &mut progress)?;
    anyhow::ensure!(
        src.stamp == source(path)?.stamp,
        "来源在检查期间已变化，请重新检查"
    );
    Ok(report)
}
struct Staging(PathBuf);
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
pub(crate) fn recover(
    path: &Path,
    parent: &Path,
    revision: &str,
    selected: &[usize],
    encoding: Option<ZipPasswordEncoding>,
    cancelled: &AtomicBool,
    mut progress: impl FnMut(&str, u64, u64),
) -> Result<Outcome> {
    check(cancelled)?;
    anyhow::ensure!(parent.is_dir(), "请选择部分取回结果的保存目录");
    let src = source(path)?;
    let report = scan_source(&src, encoding, cancelled, &mut |b, t| {
        progress("checking", b, t)
    })?;
    anyhow::ensure!(
        report.revision == revision,
        "来源或检查结果已变化，请重新检查并选择文件"
    );
    anyhow::ensure!(!report.complete, "现有内容完整，请沿正常解压流程取回");
    let ids: std::collections::HashSet<_> = selected.iter().copied().collect();
    anyhow::ensure!(
        !ids.is_empty()
            && ids.len() == selected.len()
            && ids.len() <= report.entries.len()
            && ids.iter().all(|n| *n < report.entries.len()),
        "请选择已通过检查的完整成员"
    );
    let mut wanted: Vec<_> = report
        .entries
        .iter()
        .filter(|e| ids.contains(&e.id))
        .cloned()
        .collect();
    let total = wanted.iter().map(|e| e.bytes).sum::<u64>();
    let stage = Staging(parent.join(format!(".long-lzop-recovery-{}", uuid::Uuid::new_v4())));
    fs::create_dir(&stage.0)?;
    let mut done = 0u64;
    if is_tar(path) {
        super::tar_recovery::copy_selected(&src, &stage.0, &mut wanted, cancelled, &mut |bytes| {
            done = bytes;
            progress("recovering", done, total)
        })?;
    } else {
        for entry in &wanted {
            check(cancelled)?;
            let mut input = src.open()?;
            input.seek(SeekFrom::Start(entry.offset))?;
            let mut reader = LzopReader::with_mode(input, true)?;
            let relative = Path::new(&entry.name);
            anyhow::ensure!(
                relative
                    .components()
                    .all(|c| matches!(c, std::path::Component::Normal(_))),
                "成员路径无法取回"
            );
            let output = stage.0.join(relative);
            if let Some(p) = output.parent() {
                fs::create_dir_all(p)?
            }
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(output)?;
            let mut h = Sha256::new();
            let mut count = 0u64;
            let mut buf = [0u8; 65536];
            loop {
                check(cancelled)?;
                let n = reader.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                count += n as u64;
                anyhow::ensure!(count <= entry.bytes, "成员在取回期间已变化");
                file.write_all(&buf[..n])?;
                h.update(&buf[..n]);
                done += n as u64;
                progress("recovering", done, total);
            }
            anyhow::ensure!(
                count == entry.bytes && format!("{:x}", h.finalize()) == entry.sha256,
                "成员在取回期间已变化，请重新检查"
            );
            file.flush()?;
        }
    }
    check(cancelled)?;
    anyhow::ensure!(
        source(path)?.stamp == src.stamp,
        "来源在取回期间已变化，请重新检查"
    );
    let label = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .chars()
        .take(45)
        .collect::<String>();
    let destination = parent.join(format!(
        "{label}.部分取回-{}",
        &uuid::Uuid::new_v4().to_string()[..8]
    ));
    #[cfg(windows)]
    crate::services::output_publish_transaction::transfer_no_replace(&stage.0, &destination)?;
    #[cfg(not(windows))]
    {
        anyhow::ensure!(!destination.exists(), "结果位置已存在");
        fs::rename(&stage.0, &destination)?;
    }
    Ok(Outcome {
        time_preservation: is_tar(path).then(|| crate::services::tar_recovery_time::TimePreservation {
            preserved_files: wanted.iter().filter(|e| !e.is_directory && e.time_warning.is_none()).count(),
            unpreserved_files: wanted.iter().filter(|e| !e.is_directory && e.time_warning.is_some()).count(),
            preserved_directories: Some(wanted.iter().filter(|e| e.is_directory && e.time_warning.is_none()).count()),
            unpreserved_directories: Some(wanted.iter().filter(|e| e.is_directory && e.time_warning.is_some()).count()),
        }),
        output_directory: destination.to_string_lossy().into_owned(),
        recovered_files: wanted.iter().filter(|e| !e.is_directory).count(),
        recovered_directories: wanted.iter().filter(|e| e.is_directory).count(),
        recovered_bytes: done,
        entries: wanted,
        stop_reason: report.stop_reason,
        partial: true,
    })
}
