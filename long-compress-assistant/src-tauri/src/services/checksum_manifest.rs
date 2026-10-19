//! Per-file observations. A selective pass never labels unvisited files as matched.
use super::file_integrity_service::{ChecksumAlgorithm, FileIntegrityService};
use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::HashSet, path::{Component, Path}, time::{SystemTime, UNIX_EPOCH}};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChecksumEntryStatus { Matched, Missing, Mismatch, Unreadable, InvalidEntry, NotChecked }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecksumVerificationEntry {
    pub line: usize,
    pub file_name: String,
    pub path: String,
    pub algorithm: Option<String>,
    pub expected_checksum: Option<String>,
    pub actual_checksum: Option<String>,
    pub status: ChecksumEntryStatus,
    pub reason: String,
    pub checked_at: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecksumVerificationReport {
    pub valid: bool,
    pub message: String,
    pub checksum_path: String,
    pub manifest_id: String,
    pub entries: Vec<ChecksumVerificationEntry>,
}

pub fn verify(checksum_file: &Path, base_dir: &Path, only_lines: Option<&[usize]>) -> Result<ChecksumVerificationReport> {
    let bytes = std::fs::read(checksum_file).with_context(|| format!("无法读取校验清单 {}", checksum_file.display()))?;
    let manifest_id = format!("{:x}", Sha256::digest(&bytes));
    let text = std::str::from_utf8(&bytes).context("校验清单不是有效的 UTF-8 文本，请先转换编码")?;
    let sfv = checksum_file.extension().and_then(|s| s.to_str()).is_some_and(|s| s.eq_ignore_ascii_case("sfv"));
    let selected = only_lines.map(|lines| lines.iter().copied().collect::<HashSet<_>>());
    let mut entries = Vec::new();
    for (index, raw) in text.trim_start_matches('\u{feff}').lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') { continue; }
        let parsed = if sfv {
            line.rsplit_once(char::is_whitespace).map(|(name, hash)| (hash.trim(), name.trim()))
        } else {
            // GNU binary marker is one '*', not part of the filename.
            line.split_once(char::is_whitespace).map(|(hash, name)| {
                let name = name.trim_start(); (hash, name.strip_prefix('*').unwrap_or(name))
            })
        };
        let mut row = ChecksumVerificationEntry { line: index + 1, file_name: line.to_string(), path: String::new(),
            algorithm: None, expected_checksum: None, actual_checksum: None, status: ChecksumEntryStatus::InvalidEntry,
            reason: "清单行格式无法识别，请检查文件名与摘要".into(), checked_at: None };
        let Some((expected, name)) = parsed else { entries.push(row); continue; };
        row.file_name = name.to_string();
        row.expected_checksum = Some(expected.to_string());
        let algorithm = match expected.len() {
            8 => Some((ChecksumAlgorithm::CRC32, "crc32")),
            32 if !sfv => Some((ChecksumAlgorithm::MD5, "md5")),
            64 if !sfv => Some((ChecksumAlgorithm::SHA256, "sha256")),
            _ => None,
        };
        let Some((algorithm, label)) = algorithm.filter(|_| expected.bytes().all(|c| c.is_ascii_hexdigit())) else {
            row.reason = "摘要长度或十六进制内容有误，请修正清单后重查".into(); entries.push(row); continue;
        };
        row.algorithm = Some(label.into());
        let relative = Path::new(name);
        if name.is_empty() || relative.components().any(|part| matches!(part, Component::ParentDir | Component::RootDir | Component::Prefix(_))) {
            row.reason = "清单文件名必须是同目录内的相对路径，请修正此行".into(); entries.push(row); continue;
        }
        let path = base_dir.join(relative);
        row.path = path.to_string_lossy().into_owned();
        if selected.as_ref().is_some_and(|lines| !lines.contains(&row.line)) {
            row.status = ChecksumEntryStatus::NotChecked; row.reason = "本次未选择此项，尚未读取文件".into(); entries.push(row); continue;
        }
        row.checked_at = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64);
        match FileIntegrityService::calculate_checksum(&path, algorithm) {
            Ok(result) => {
                row.status = if result.checksum.eq_ignore_ascii_case(expected) { ChecksumEntryStatus::Matched } else { ChecksumEntryStatus::Mismatch };
                row.reason = if row.status == ChecksumEntryStatus::Matched { "文件摘要与清单一致" } else { "文件内容与清单不符，请核对来源或重新下载此文件" }.into();
                row.actual_checksum = Some(result.checksum);
            }
            Err(error) => {
                let io = error.chain().find_map(|cause| cause.downcast_ref::<std::io::Error>());
                row.status = if io.is_some_and(|cause| cause.kind() == std::io::ErrorKind::NotFound) { ChecksumEntryStatus::Missing } else { ChecksumEntryStatus::Unreadable };
                row.reason = if row.status == ChecksumEntryStatus::Missing { format!("文件不存在，请补回此文件。原始原因：{error:#}") }
                    else { format!("文件无法读取，请检查占用、权限或路径后复查。原始原因：{error:#}") };
            }
        }
        entries.push(row);
    }
    let checked = entries.iter().filter(|row| row.status != ChecksumEntryStatus::NotChecked).count();
    let matched = entries.iter().filter(|row| row.status == ChecksumEntryStatus::Matched).count();
    let valid = !entries.is_empty() && matched == entries.len();
    let message = if entries.is_empty() { "清单中没有可检查的文件，请核对清单内容".into() }
        else { format!("共 {} 项，本次检查 {} 项，匹配 {} 项，未检查 {} 项", entries.len(), checked, matched, entries.len() - checked) };
    Ok(ChecksumVerificationReport { valid, message, checksum_path: checksum_file.to_string_lossy().into_owned(), manifest_id, entries })
}
