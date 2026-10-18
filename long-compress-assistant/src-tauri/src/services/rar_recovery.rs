//! RAR repairs run on private copies. Only a fully readable result is published.
use super::archive_diagnostics::{ArchiveDiagnosticReport, DiagnosticIssue};
use super::compression_service::CompressionService;
use super::split_archive_detector::SplitArchiveDetector;
use anyhow::{ensure, Context, Result};
use regex::Regex;
use serde::Serialize;
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

fn cancelled(flag: &AtomicBool) -> Result<()> {
    ensure!(!flag.load(Ordering::SeqCst), "RAR recovery cancelled");
    Ok(())
}

pub(crate) fn family(name: &str) -> Option<String> {
    static PATTERNS: std::sync::OnceLock<Vec<Regex>> = std::sync::OnceLock::new();
    let patterns = PATTERNS.get_or_init(|| {
        [
            r"(?i)^(.+)\.part\d+\.(?:rar|rev)$",
            r"(?i)^(.+)_\d+_\d+_\d+\.rev$",
            r"(?i)^(.+)\.(?:rar|rev|r\d{2,3})$",
        ]
        .iter()
        .map(|p| Regex::new(p).unwrap())
        .collect()
    });
    for pattern in patterns {
        if let Some(c) = pattern.captures(name) {
            return Some(c[1].to_lowercase());
        }
    }
    None
}
pub(crate) fn is_candidate(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .and_then(family)
        .is_some()
}

fn numbered_parts(name: &str) -> bool {
    static PATTERN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    PATTERN
        .get_or_init(|| Regex::new(r"(?i)\.part\d+\.(?:rar|rev)$").unwrap())
        .is_match(name)
}

#[derive(Debug)]
struct Group {
    files: Vec<PathBuf>,
    data: Vec<PathBuf>,
    recovery: Vec<PathBuf>,
    rar4: bool,
    recovery_data_count: Option<usize>,
    rejected_recovery: Vec<String>,
}
pub(crate) fn diagnosis_group_path(path: &Path) -> Result<PathBuf> {
    let group = collect(path)?;
    Ok(group.data.first().or(group.recovery.first()).cloned().unwrap_or_else(|| path.to_path_buf()))
}
fn collect(path: &Path) -> Result<Group> {
    let mut key = family(path.file_name().and_then(|n| n.to_str()).unwrap_or(""))
        .context("不是 RAR 或恢复卷")?;
    let part_style = numbered_parts(path.file_name().and_then(|n| n.to_str()).unwrap_or(""));
    let entries = fs::read_dir(path.parent().unwrap_or(Path::new(".")))?
        .map(|e| e.map(|e| e.path())).collect::<std::io::Result<Vec<_>>>()?;
    let mut rar4_families = entries.iter().filter(|p| rar4_header(p).1)
        .filter_map(|p| family(p.file_name()?.to_str()?)).collect::<Vec<_>>();
    rar4_families.sort(); rar4_families.dedup();
    // RAR4 -vn creates base1.rev, not base.rev or base.part1.rev.
    // Resolve against actual RAR4 volume families rather than stripping every
    // trailing digit (which would confuse "backup" and "backup2024").
    if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("rev")) && !part_style {
        let mut candidates = rar4_families.iter().cloned()
            .filter(|base| recovery_name_matches(path, base, false, true))
            .collect::<Vec<_>>();
        candidates.sort(); candidates.dedup();
        ensure!(candidates.len() <= 1, "恢复卷名称对应多个 RAR4 卷组，请将原卷组放到单独目录后重试");
        if let Some(base) = candidates.pop() { key = base; }
    }
    let rar4 = entries.iter().any(|p| family(p.file_name().and_then(|n| n.to_str()).unwrap_or("")).as_ref() == Some(&key)
        && numbered_parts(p.file_name().and_then(|n| n.to_str()).unwrap_or("")) == part_style && rar4_header(p).0);
    let mut group = Group {
        files: vec![],
        data: vec![],
        recovery: vec![],
        rar4,
        recovery_data_count: None,
        rejected_recovery: vec![],
    };
    for file in entries {
        if !file.is_file() {
            continue;
        }
        if file
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("rev"))
        {
            if !recovery_name_matches(&file, &key, part_style, rar4) { continue; }
            if rar4 && !part_style {
                ensure!(rar4_families.iter().filter(|base| recovery_name_matches(&file, base, false, true)).count() <= 1,
                    "恢复卷名称对应多个 RAR4 卷组，请将原卷组放到单独目录后重试");
            }
            group.recovery.push(file.clone());
        } else {
            if family(file.file_name().and_then(|n| n.to_str()).unwrap_or("")).as_ref() != Some(&key)
                || numbered_parts(file.file_name().and_then(|n| n.to_str()).unwrap_or("")) != part_style { continue; }
            group.data.push(file.clone());
        }
        group.files.push(file);
    }
    group.data.sort_by_key(|p| {
        let name = p.file_name().unwrap().to_string_lossy().to_lowercase();
        let matcher = Regex::new(r"\.(?:part(\d+)\.rar|r(\d{2,3}))$").unwrap();
        let index = matcher
            .captures(&name)
            .map(|c| {
                c.get(1)
                    .map(|v| v.as_str().parse::<u32>().unwrap_or(u32::MAX))
                    .unwrap_or_else(|| c[2].parse::<u32>().unwrap_or(u32::MAX).saturating_add(1))
            })
            .unwrap_or(0);
        (index, name)
    });
    group.recovery.sort();
    Ok(group)
}

fn rar4_header(path: &Path) -> (bool, bool) {
    let mut bytes = [0; 13];
    if fs::File::open(path).and_then(|mut f| f.read_exact(&mut bytes)).is_err() { return (false, false); }
    let rar4 = bytes[..7] == *b"Rar!\x1a\x07\x00";
    (rar4, rar4 && bytes[9] == 0x73 && bytes[10] & 1 != 0)
}

fn recovery_name_matches(path: &Path, key: &str, part_style: bool, rar4: bool) -> bool {
    let name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
    if numbered_parts(&name) != part_style { return false; }
    if family(&name).as_deref() == Some(key) { return true; }
    rar4 && !part_style && name.strip_prefix(key).and_then(|s| s.strip_suffix(".rev"))
        .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
}

// RAR 3.10–4.x REV footer: data count - 1, recovery count - 1,
// recovery index - 1, then CRC32 of all preceding bytes. Stream the checksum
// with cancellation; never treat a matching filename as verified recovery data.
fn check_rar4_recovery(group: &mut Group, flag: &AtomicBool) -> Result<()> {
    if !group.rar4 { return Ok(()); }
    let mut set = None;
    let mut indices = std::collections::HashSet::new();
    let mut valid = Vec::new();
    for path in &group.recovery {
        cancelled(flag)?;
        // RAR 3.0's name_#_#_#.rev has no footer checksum; keep its existing
        // external-tool path, but do not infer a reliable volume count from it.
        if Regex::new(r"(?i)_\d+_\d+_\d+\.rev$").unwrap().is_match(&path.to_string_lossy()) {
            valid.push(path.clone()); continue;
        }
        let mut file = fs::File::open(path)?;
        let len = file.metadata()?.len();
        let metadata = (|| -> Result<(usize, usize, usize)> {
            ensure!(len >= 7, "恢复卷过短");
            file.seek(SeekFrom::End(-7))?;
            let mut footer = [0; 7]; file.read_exact(&mut footer)?;
            let data = footer[0] as usize + 1;
            let recovery = footer[1] as usize + 1;
            let index = footer[2] as usize + 1;
            ensure!(data + recovery <= 255 && index <= recovery, "恢复卷计数无效");
            file.rewind()?;
            let mut remaining = len - 4;
            let mut hasher = crc32fast::Hasher::new();
            let mut buffer = [0; 256 * 1024];
            while remaining > 0 {
                cancelled(flag)?;
                let count = remaining.min(buffer.len() as u64) as usize;
                file.read_exact(&mut buffer[..count])?; hasher.update(&buffer[..count]); remaining -= count as u64;
            }
            ensure!(hasher.finalize() == u32::from_le_bytes(footer[3..].try_into().unwrap()), "恢复卷校验失败");
            Ok((data, recovery, index))
        })();
        cancelled(flag)?;
        match metadata {
            Ok((data, recovery, index)) => {
                let identity = (data, recovery, len);
                ensure!(set.is_none_or(|previous| previous == identity), "同名恢复卷的卷数或尺寸不一致，请使用同一原卷组的恢复卷");
                ensure!(indices.insert(index), "恢复卷序号重复，请去除重复副本后重试");
                set = Some(identity); valid.push(path.clone());
            }
            Err(error) => group.rejected_recovery.push(format!("{}：{error}", path.file_name().unwrap().to_string_lossy())),
        }
    }
    group.recovery = valid;
    group.files.retain(|p| !p.extension().is_some_and(|e| e.eq_ignore_ascii_case("rev")) || group.recovery.contains(p));
    group.recovery_data_count = set.map(|(data, _, _)| data);
    Ok(())
}

fn recovery_missing(group: &Group) -> Vec<String> {
    let (Some(total), Some(first)) = (group.recovery_data_count, group.data.first()) else { return vec![]; };
    let name = first.file_name().unwrap().to_string_lossy();
    let key = family(&name).unwrap();
    let modern = Regex::new(r"(?i)\.part(\d+)\.rar$").unwrap();
    let width = modern.captures(&name).map(|c| c[1].len());
    (0..total).map(|i| match width {
        Some(width) => format!("{key}.part{:0width$}.rar", i + 1),
        None if i == 0 => format!("{key}.rar"),
        None => format!("{key}.r{:02}", i - 1),
    }).filter(|expected| !group.data.iter().any(|p| p.file_name().unwrap().to_string_lossy().eq_ignore_ascii_case(expected))).collect()
}

#[derive(Debug, thiserror::Error)]
#[error("{detail}")]
struct VerificationFailure {
    detail: String,
    encrypted: bool,
    category: &'static str,
    missing_volume: Option<String>,
}

#[derive(Default)]
struct Contents {
    volume: bool,
    files: usize,
    directories: usize,
    size: u64,
    encrypted: bool,
}
fn verify(path: &Path, password: Option<&str>, flag: &AtomicBool) -> Result<Contents> {
    cancelled(flag)?;
    let archive = match password.filter(|p| !p.is_empty()) {
        Some(password) => unrar::Archive::with_password(path, password),
        None => unrar::Archive::new(path),
    };
    let observed_encrypted = std::cell::Cell::new(false);
    let password_proven = std::cell::Cell::new(false);
    let error = |e: unrar::error::UnrarError| {
        use unrar::error::Code;
        let encrypted =
            observed_encrypted.get() || matches!(e.code, Code::MissingPassword | Code::BadPassword);
        let native = super::rar_support::RarSupportService::native_failure_message(&e, password_proven.get());
        let requested = e.volume.as_ref().filter(|v| v.requested);
        let missing = requested.filter(|v| v.unavailable_kind == Some(std::io::ErrorKind::NotFound));
        let category = if missing.is_some() { "missing-volume" }
            else if requested.is_some() { "io" }
            else if native.is_some() { "rar-later-data" }
            else { match e.code { Code::MissingPassword => "password_required", Code::BadPassword => "wrong_password", _ => "damaged" } };
        let missing_volume = missing.map(|v| v.path.file_name().unwrap_or(v.path.as_os_str()).to_string_lossy().into_owned());
        let detail = native.unwrap_or_else(|| match e.code {
            Code::MissingPassword => "需要填写正确的 RAR 密码".into(),
            Code::BadPassword => "RAR 密码错误，请修改后重新检查".into(),
            _ => format!("RAR 内容未通过完整校验：{e:?}"),
        });
        anyhow::Error::new(VerificationFailure { detail, encrypted, category, missing_volume })
    };
    let mut archive = archive.open_for_processing().map_err(error)?;
    super::sfx_volumes::validate_native_volume(path, !matches!(archive.volume_info(), unrar::VolumeInfo::None))?;
    observed_encrypted.set(archive.has_encrypted_headers());
    let encrypted_headers = archive.has_encrypted_headers();
    let mut contents = Contents {
        volume: !matches!(archive.volume_info(), unrar::VolumeInfo::None),
        encrypted: archive.has_encrypted_headers(),
        ..Default::default()
    };
    loop {
        cancelled(flag)?;
        let Some(header) = archive.read_header().map_err(error)? else {
            break;
        };
        password_proven.set(password_proven.get() || encrypted_headers);
        contents.encrypted |= header.entry().is_encrypted();
        observed_encrypted.set(contents.encrypted);
        if header.entry().is_directory() {
            contents.directories += 1;
            archive = header.skip().map_err(error)?;
        } else {
            let proves_content = header.entry().is_encrypted() && header.entry().unpacked_size > 0;
            contents.files += 1;
            contents.size = contents.size.saturating_add(header.entry().unpacked_size);
            archive = header.test_with_cancel(flag).map_err(|failure| {
                if flag.load(Ordering::SeqCst) { anyhow::anyhow!("RAR recovery cancelled") }
                else { error(failure) }
            })?;
            password_proven.set(password_proven.get() || proves_content);
        }
    }
    cancelled(flag)?;
    Ok(contents)
}

pub(crate) async fn verify_readonly(path: &Path, password: Option<&str>, cancelled: Arc<AtomicBool>) -> Result<()> {
    let source = SplitArchiveDetector::complete_source(path)?;
    let password = password.map(str::to_owned);
    tokio::task::spawn_blocking(move || verify(&source, password.as_deref(), &cancelled).map(|_| ())).await?
}

pub async fn diagnose(
    path: &Path,
    password: Option<&str>,
    flag: Arc<AtomicBool>,
) -> Result<ArchiveDiagnosticReport> {
    let path = fs::canonicalize(path)?;
    let password = password.map(str::to_owned);
    tauri::async_runtime::spawn_blocking(move || {
        let sfx_group = SplitArchiveDetector::detect_split_archive(&path)?
            .filter(|g| g.format == super::split_archive_detector::SplitFormat::RarSplit
                && super::sfx_archive::is_executable_path(&g.first_part));
        let sfx = super::sfx_archive::is_executable_path(&path) || sfx_group.is_some();
        let mut group = if sfx {
            let source = sfx_group.as_ref().map(|g| g.first_part.as_path()).unwrap_or(&path);
            ensure!(super::sfx_archive::detect(source)?.is_some_and(|(format, _)|
                format == super::archive_format::ArchiveFormat::Rar), "不是可读取的 RAR 自解压归档");
            let data = sfx_group.as_ref().map(|g| g.parts.clone()).unwrap_or_else(|| vec![path.clone()]);
            Group { files: data.clone(), data, recovery: vec![], rar4: false, recovery_data_count: None, rejected_recovery: vec![] }
        } else { collect(&path)? };
        check_rar4_recovery(&mut group, &flag)?;
        let tool = CompressionService::find_rar_encoder().is_some();
        let first = group.data.first();
        let split = first
            .map(|p| SplitArchiveDetector::detect_split_archive(p))
            .transpose()?
            .flatten();
        let mut missing: Vec<String> = split
            .as_ref()
            .map(|g| {
                g.missing_parts
                    .iter()
                    .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default();
        for name in recovery_missing(&group) { if !missing.iter().any(|p| p.eq_ignore_ascii_case(&name)) { missing.push(name); } }
        let mut report = ArchiveDiagnosticReport {
            file_path: path.to_string_lossy().to_string(),
            file_size: group
                .data
                .iter()
                .filter_map(|p| p.metadata().ok())
                .map(|m| m.len())
                .sum(),
            actual_format: "RAR".into(),
            status: "damaged".into(),
            encrypted: false,
            split_archive: split.is_some() || !group.recovery.is_empty(),
            volumes_found: group.data.len(),
            missing_volumes: missing,
            total_files: 0,
            total_directories: 0,
            total_uncompressed_size: 0,
            total_uncompressed_size_known: false,
            integrity_tested: false,
            can_repair: false,
            recoverability: "diagnostic_only".into(),
            issues: vec![],
            evidence: vec![],
        };
        report.evidence.push(format!(
            "发现 {} 个数据卷、{} 个恢复卷；原件不会被修复命令直接修改",
            group.data.len(),
            group.recovery.len()
        ));
        if group.rar4 { report.evidence.push("RAR4 旧格式；恢复记录与恢复卷在副本中处理，结果须通过逐文件校验".into()); }
        if !group.rejected_recovery.is_empty() {
            report.issues.push(DiagnosticIssue { code: "invalid_recovery_volume".into(), severity: "warning".into(), title: "已忽略无效恢复卷".into(), detail: group.rejected_recovery.join("；") });
        }
        let readable = split
            .as_ref()
            .map(|g| &g.first_part)
            .filter(|p| p.is_file())
            .or(first);
        if !report.missing_volumes.is_empty() {
            report.status = "missing_volume".into();
            report.issues.push(DiagnosticIssue {
                code: "missing_volume".into(),
                severity: "warning".into(),
                title: "缺少分卷".into(),
                detail: format!("缺少：{}", report.missing_volumes.join("、")),
            });
        } else if let Some(source) = readable {
            match verify(source, password.as_deref(), &flag) {
                Ok(c) => {
                    if group.data.len() > 1 && !c.volume {
                        report.issues.push(DiagnosticIssue {
                            code: "ambiguous_volume_names".into(),
                            severity: "warning".into(),
                            title: "同名文件并非同一卷组".into(),
                            detail: "RAR 首文件是独立压缩包，请将分卷整理到单独目录后重新检查。"
                                .into(),
                        });
                        return Ok(report);
                    }
                    report.status = "healthy".into();
                    report.integrity_tested = true;
                    report.total_files = c.files;
                    report.total_directories = c.directories;
                    report.total_uncompressed_size = c.size;
                    report.total_uncompressed_size_known = true;
                    report.encrypted = c.encrypted;
                    report.recoverability = if sfx { "sfx_readonly" } else { "not_needed" }.into();
                    if sfx { report.evidence.push("自解压包仅做只读完整性检查，不提供自解压模块修复或恢复卷处理".into()); }
                    return Ok(report);
                }
                Err(e) => {
                    cancelled(&flag)?;
                    let failure = e.downcast_ref::<VerificationFailure>();
                    report.encrypted = failure.is_some_and(|e| e.encrypted);
                    let detail = e.to_string();
                    if let Some(failure) = failure {
                        report.status = match failure.category {
                            "missing-volume" => "missing_volume",
                            "password_required" => "password_required",
                            "wrong_password" => "wrong_password",
                            _ => "damaged",
                        }.into();
                        if let Some(name) = &failure.missing_volume { report.missing_volumes.push(name.clone()); }
                    }
                    report.issues.push(DiagnosticIssue {
                        code: if failure.is_some_and(|f| f.category == "rar-later-data") { "rar_later_data".into() }
                            else if failure.is_some_and(|f| f.category == "io") { "rar_volume_unavailable".into() } else { report.status.clone() },
                        severity: "warning".into(),
                        title: "归档尚未完整可读".into(),
                        detail,
                    });
                }
            }
        }
        if first.is_none() {
            report.status = "missing_volume".into();
            report.issues.push(DiagnosticIssue {
                code: "missing_data".into(),
                severity: "warning".into(),
                title: "只找到了恢复卷".into(),
                detail: "请把仍保留的同组 RAR 分卷与恢复卷放在同一目录，再重新检查。".into(),
            });
        }
        if sfx {
            report.evidence.push("自解压包仅做只读完整性检查，不提供自解压模块修复或恢复卷处理".into());
            return Ok(report);
        }
        let unresolved_input = report.status.contains("password")
            || (report.status == "missing_volume" && group.recovery.is_empty())
            || report.issues.iter().any(|issue| matches!(issue.code.as_str(), "rar_later_data" | "rar_volume_unavailable"));
        if !tool {
            if !unresolved_input { report.issues.push(DiagnosticIssue {
                code: "rar_tool_required".into(),
                severity: "warning".into(),
                title: "修复需要本机 RAR 工具".into(),
                detail: "安装 WinRAR 后重新检查即可；普通 RAR 解压不需要安装。".into(),
            }); }
        } else if group.recovery_data_count.is_some() && report.missing_volumes.len() > group.recovery.len() {
            report.issues.push(DiagnosticIssue { code: "insufficient_recovery_volumes".into(), severity: "warning".into(), title: "恢复卷不足".into(), detail: format!("缺少 {} 个数据卷，仅有 {} 个校验通过的恢复卷；请补齐原卷或更多同组恢复卷。", report.missing_volumes.len(), group.recovery.len()) });
        } else if first.is_some()
            && (!group.recovery.is_empty() || report.missing_volumes.is_empty())
            && !report.status.contains("password")
            && !report.issues.iter().any(|issue| matches!(issue.code.as_str(), "rar_later_data" | "rar_volume_unavailable"))
        {
            report.can_repair = true;
            report.recoverability = if group.recovery.is_empty() {
                "rar_repair"
            } else {
                "rar_recovery_volumes"
            }
            .into();
        }
        Ok(report)
    })
    .await?
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RarRepairResult {
    pub output_path: String,
    pub output_files: Vec<String>,
    pub format: String,
    pub recovered_files: usize,
    pub recovered_directories: usize,
    pub skipped_entries: Vec<String>,
    pub verified: bool,
    pub partial: bool,
    pub directory_rebuilt: bool,
    pub output_encrypted: bool,
}

#[cfg(windows)]
fn password_bytes_for_codepage(password: &str, codepage: u32) -> Result<Vec<u8>> {
    use windows_sys::Win32::Globalization::{WideCharToMultiByte, WC_NO_BEST_FIT_CHARS};
    if codepage == 65001 { return Ok(password.as_bytes().to_vec()); }
    let wide: Vec<u16> = password.encode_utf16().collect();
    let length = i32::try_from(wide.len())?;
    let mut substituted = 0;
    let size = unsafe { WideCharToMultiByte(codepage, WC_NO_BEST_FIT_CHARS, wide.as_ptr(), length, std::ptr::null_mut(), 0, std::ptr::null(), &mut substituted) };
    ensure!(size > 0 && substituted == 0, "当前 RAR 工具输入编码无法表示该密码，请在 WinRAR 修复后重新导入");
    let mut result = vec![0; size as usize];
    let written = unsafe { WideCharToMultiByte(codepage, WC_NO_BEST_FIT_CHARS, wide.as_ptr(), length, result.as_mut_ptr(), size, std::ptr::null(), &mut substituted) };
    ensure!(written == size && substituted == 0, "RAR 修复密码无法使用当前工具输入编码传递");
    Ok(result)
}

fn password_input(password: &str) -> Result<Vec<u8>> {
    ensure!(
        !password.contains(['\r', '\n', '\0']),
        "RAR 修复暂不支持包含换行或空字符的密码"
    );
    #[cfg(windows)]
    // RAR's redirected getwstr -> CharToWide uses CP_ACP, not the console OEM CP.
    let mut bytes = password_bytes_for_codepage(password, unsafe { windows_sys::Win32::Globalization::GetACP() })?;
    #[cfg(not(windows))]
    let mut bytes = password.as_bytes().to_vec();
    bytes.extend_from_slice(b"\r\n");
    Ok(bytes)
}

async fn run(
    tool: &str,
    operation: &str,
    source: &Path,
    work: &Path,
    password: Option<&str>,
    flag: &AtomicBool,
    password_prompts: usize,
) -> Result<()> {
    let input = match password
        .filter(|p| !p.is_empty())
        .map(password_input)
        .transpose()
    {
        Ok(input) => input,
        Err(_) if operation == "r" => None, // Recovery records can repair ciphertext without opening headers.
        Err(error) => return Err(error),
    };
    let mut command = crate::utils::process::async_command(tool);
    command
        .current_dir(work)
        .arg(operation)
        .arg("-y")
        .arg(if input.is_some() { "-p" } else { "-p-" })
        .arg("-idq")
        .arg("-cfg-")
        .arg(source.file_name().context("无效的来源文件名")?);
    if operation == "r" {
        command.arg(work);
    }
    let output = if password_prompts > 0 && input.is_some() {
        crate::utils::process::cancellable_output_with_rar_prompts(
            &mut command, flag, std::time::Duration::from_secs(3600), input, password_prompts,
        ).await?
    } else { crate::utils::process::cancellable_output_with_stdin(
        &mut command,
        flag,
        std::time::Duration::from_secs(3600),
        input,
    )
    .await? };
    cancelled(flag)?;
    if operation.starts_with("rv") {
        ensure!(output.status.success(), "恢复卷创建未完成，请确认密码、WinRAR 版本和可用空间（退出码 {:?}）", output.status.code());
        return Ok(());
    }
    if operation == "rc" && output.status.code() == Some(11) {
        anyhow::bail!("RAR 无法验证原包密码，请填写或修改密码后重新检查；未保存不完整卷组");
    }
    // RAR can finish ciphertext recovery, then return 11 when its own final test
    // cannot open encrypted headers. We verify the candidate natively with the
    // supplied password before publishing; no password enters process arguments.
    ensure!(matches!(output.status.code(), Some(0 | 1 | 11)), "RAR 修复未完成：恢复记录或恢复卷可能不足，或来源损坏超出恢复能力（退出码 {:?}）。原件保持不变。", output.status.code());
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryVolumesResult {
    pub output_files: Vec<String>,
    pub total_bytes: u64,
    pub data_volumes: usize,
}

pub async fn create_recovery_volumes(path: &Path, destination: &Path, count: u32, password: Option<&str>, flag: Arc<AtomicBool>) -> Result<RecoveryVolumesResult> {
    ensure!((1..=10).contains(&count), "恢复卷数量请选择 1 至 10 个");
    cancelled(&flag)?;
    let source = fs::canonicalize(path)?;
    let display_root = std::path::absolute(destination)?;
    let destination = fs::canonicalize(destination)?;
    ensure!(destination.is_dir(), "请选择恢复卷保存目录");
    let group = collect(&source)?;
    ensure!(group.data.len() >= 2, "恢复卷用于多个 RAR 分包，请选择完整分包中的任意一卷");
    let selected = group.data[0].clone();
    let split = SplitArchiveDetector::detect_split_archive(&selected)?.context("没有找到 RAR 分卷组")?;
    ensure!(split.is_complete, "请先补齐分卷：{:?}", split.missing_parts);
    ensure!(collect(&destination.join(selected.file_name().unwrap()))?.recovery.is_empty(), "保存目录已存在同组恢复卷，请选择其他位置，原文件不会被覆盖");
    let tool = CompressionService::find_rar_encoder().context("创建恢复卷需要本机安装 WinRAR/Rar.exe")?;
    let work = tempfile::Builder::new().prefix(".long-rar-recovery-").tempdir_in(&destination)?;
    let copy_paths = group.data.clone();
    let root = work.path().to_path_buf();
    let copy_flag = flag.clone();
    let verify_password = password.map(str::to_owned);
    let verify_name = selected.file_name().unwrap().to_owned();
    tauri::async_runtime::spawn_blocking(move || -> Result<()> {
        let mut buffer = vec![0u8; 256 * 1024];
        for file in copy_paths {
            cancelled(&copy_flag)?;
            let target = root.join(file.file_name().unwrap());
            // The rv command only reads data volumes. NTFS links avoid copying
            // a large complete set; other filesystems use cancellable copies.
            if fs::hard_link(&file, &target).is_err() {
                let mut input = fs::File::open(file)?;
                let mut output = fs::File::create(target)?;
                loop { cancelled(&copy_flag)?; let len = input.read(&mut buffer)?; if len == 0 { break; } output.write_all(&buffer[..len])?; }
            }
        }
        let contents = verify(&root.join(verify_name), verify_password.as_deref(), &copy_flag)?;
        ensure!(contents.volume && contents.files + contents.directories > 0, "原分包未通过完整读取，请先检查或修复后再创建恢复卷");
        Ok(())
    }).await??;
    run(&tool, &format!("rv{count}"), &selected, work.path(), password, &flag, if group.rar4 { group.data.len().saturating_add(2) } else { 0 }).await?;
    let generated = collect(&work.path().join(selected.file_name().unwrap()))?.recovery;
    ensure!(generated.len() == count as usize, "没有完整生成所选数量的恢复卷，请确认 WinRAR 版本");
    let mut published = Vec::new();
    let mut bytes = 0u64;
    let result = (|| -> Result<()> {
        for file in generated {
            cancelled(&flag)?;
            let size = file.metadata()?.len(); ensure!(size > 0, "生成的恢复卷为空");
            let target = destination.join(file.file_name().unwrap());
            super::output_publish_transaction::transfer_no_replace(&file, &target)?;
            bytes = bytes.saturating_add(size);
            published.push(target);
        }
        cancelled(&flag)?;
        Ok(())
    })();
    if let Err(error) = result { for file in published { let _ = fs::remove_file(file); } return Err(error); }
    Ok(RecoveryVolumesResult { output_files: published.iter().map(|p| display_root.join(p.file_name().unwrap()).to_string_lossy().to_string()).collect(), total_bytes: bytes, data_volumes: group.data.len() })
}

pub async fn repair(
    path: &Path,
    destination: &Path,
    password: Option<&str>,
    flag: Arc<AtomicBool>,
) -> Result<RarRepairResult> {
    cancelled(&flag)?;
    let source = fs::canonicalize(path)?;
    let requested_destination = std::path::absolute(destination)?;
    let destination = fs::canonicalize(destination)?;
    ensure!(destination.is_dir(), "请选择用于保存修复结果的文件夹");
    let collect_path = source.clone();
    let collect_flag = flag.clone();
    let group = tauri::async_runtime::spawn_blocking(move || -> Result<Group> {
        let mut group = collect(&collect_path)?;
        check_rar4_recovery(&mut group, &collect_flag)?;
        Ok(group)
    }).await??;
    ensure!(
        !group.data.is_empty(),
        "没有找到同组 RAR 数据卷，请先补齐仍保留的分卷"
    );
    let tool = CompressionService::find_rar_encoder().context("修复需要本机安装 WinRAR/Rar.exe")?;
    let stem = family(group.data[0].file_name().unwrap().to_str().unwrap_or("")).context("无效的 RAR 名称")?;
    let missing = recovery_missing(&group);
    ensure!(missing.is_empty() || missing.len() <= group.recovery.len(), "恢复卷不足：缺少 {} 个数据卷，仅有 {} 个校验通过的恢复卷。原件保持不变。", missing.len(), group.recovery.len());
    let final_dir = destination.join(format!("{stem}.repaired"));
    let display_dir = requested_destination.join(format!("{stem}.repaired"));
    ensure!(
        !final_dir.exists(),
        "修复结果目录已存在，请选择其他位置或先重命名：{}",
        final_dir.display()
    );
    let work = tempfile::Builder::new()
        .prefix(".long-rar-repair-")
        .tempdir_in(&destination)?;
    let copy_files = group.files.clone();
    let copy_root = work.path().to_path_buf();
    let copy_flag = flag.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<()> {
        let mut buffer = vec![0; 256 * 1024];
        for file in copy_files {
            let mut input = fs::File::open(&file)?;
            let mut output = fs::File::create(copy_root.join(file.file_name().unwrap()))?;
            loop {
                cancelled(&copy_flag)?;
                let len = input.read(&mut buffer)?;
                if len == 0 {
                    break;
                }
                output.write_all(&buffer[..len])?;
            }
        }
        Ok(())
    })
    .await??;
    let use_recovery = !group.recovery.is_empty();
    if use_recovery {
        run(
            &tool,
            "rc",
            &group.recovery[0],
            work.path(),
            password,
            &flag,
            if group.rar4 { group.recovery_data_count.unwrap_or(group.data.len()).saturating_add(2) } else { 0 },
        )
        .await?;
    } else {
        // RAR repairs one damaged volume at a time and leaves healthy volumes
        // untouched. Assemble replacement and original copies under common names.
        for file in &group.data {
            run(&tool, "r", file, work.path(), password, &flag, 0).await?;
        }
    }
    let output = tempfile::Builder::new()
        .prefix(".long-rar-result-")
        .tempdir_in(&destination)?;
    let mut partial = false;
    let mut replacements = 0;
    let sources = if use_recovery {
        collect(&work.path().join(source.file_name().unwrap()))?.data
    } else {
        group.data.clone()
    };
    for file in sources {
        cancelled(&flag)?;
        let name = file.file_name().unwrap();
        let original = work.path().join(name);
        let selected = if use_recovery {
            original
        } else {
            let fixed = work
                .path()
                .join(format!("fixed.{}", name.to_string_lossy()));
            let rebuilt = work
                .path()
                .join(format!("rebuilt.{}", name.to_string_lossy()));
            if fixed.is_file() {
                replacements += 1;
                fixed
            } else if rebuilt.is_file() {
                replacements += 1;
                partial = true;
                rebuilt
            } else {
                original
            }
        };
        fs::rename(selected, output.path().join(name))?;
    }
    ensure!(
        use_recovery || replacements > 0,
        "RAR 工具没有生成修复文件，原件保持不变"
    );
    let staged_group = collect(&output.path().join(group.data[0].file_name().unwrap()))?;
    let selected = staged_group
        .data
        .first()
        .context("没有找到修复后的 RAR 数据卷")?
        .clone();
    if let Some(split) = SplitArchiveDetector::detect_split_archive(&selected)? {
        ensure!(
            split.first_part.is_file() && split.is_complete,
            "修复后仍缺少 RAR 分卷，未发布不完整卷组"
        );
    }
    let verify_path = selected.clone();
    let verify_password = password.map(str::to_owned);
    let verify_flag = flag.clone();
    let contents = tauri::async_runtime::spawn_blocking(move || {
        verify(&verify_path, verify_password.as_deref(), &verify_flag)
    })
    .await??;
    cancelled(&flag)?;
    ensure!(
        staged_group.data.len() == 1 || contents.volume,
        "同名 RAR 是独立包，不能作为分卷首卷；未发布混合卷组"
    );
    ensure!(
        contents.files + contents.directories > 0,
        "修复结果为空，未保存恢复包"
    );
    let output_files = staged_group
        .data
        .iter()
        .map(|file| {
            display_dir
                .join(file.file_name().unwrap())
                .to_string_lossy()
                .to_string()
        })
        .collect();
    // Windows rename refuses a destination created by another operation.
    fs::rename(output.path(), &final_dir).context("结果目录已出现或无法保存，未覆盖原有内容")?;
    Ok(RarRepairResult {
        output_path: display_dir
            .join(selected.file_name().unwrap())
            .to_string_lossy()
            .to_string(),
        output_files,
        format: "RAR".into(),
        recovered_files: contents.files,
        recovered_directories: contents.directories,
        skipped_entries: vec![],
        verified: true,
        partial,
        directory_rebuilt: partial,
        output_encrypted: contents.encrypted,
    })
}

#[cfg(test)]
#[path = "rar4_recovery_tests.rs"]
mod rar4_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn redirected_password_uses_ansi_not_console_oem_encoding() {
        assert_eq!(password_bytes_for_codepage("café €", 1252).unwrap(), b"caf\xe9 \x80");
        assert_eq!(password_bytes_for_codepage("中文", 936).unwrap(), [0xd6, 0xd0, 0xce, 0xc4]);
        assert_eq!(password_bytes_for_codepage("中文 🔑", 65001).unwrap(), "中文 🔑".as_bytes());
        assert!(password_bytes_for_codepage("中文 🔑", 1252).is_err());
        assert!(password_input("secret\nnext").is_err());
    }
    fn command(tool: &str, args: &[&str]) {
        let output = crate::utils::process::command(tool)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    #[test]
    #[ignore = "Requires an external Rar.exe; creates an isolated 128 MiB single-entry archive"]
    fn native_large_entry_check_can_cancel_without_waiting_for_the_entry() {
        let tool = CompressionService::find_rar_encoder().expect("Rar.exe required");
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("large.bin");
        let mut file = fs::File::create(&source).unwrap();
        let block = vec![0x5a; 1024 * 1024];
        for _ in 0..128 { file.write_all(&block).unwrap(); }
        drop(file);
        let path = dir.path().join("large.rar");
        command(&tool, &["a", "-m0", "-idq", "-cfg-", path.to_str().unwrap(), source.to_str().unwrap()]);
        let original = fs::metadata(&path).unwrap().len();
        let entry = unrar::Archive::new(&path).open_for_processing().unwrap().read_header().unwrap().unwrap();
        let flag = Arc::new(AtomicBool::new(false));
        let signal = flag.clone();
        let started = std::time::Instant::now();
        let cancel = std::thread::spawn(move || { std::thread::sleep(std::time::Duration::from_millis(1)); signal.store(true, Ordering::SeqCst); });
        let result = entry.test_with_cancel(&flag);
        cancel.join().unwrap();
        assert!(result.is_err(), "cancel was ignored while processing the single entry");
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        assert_eq!(fs::metadata(&path).unwrap().len(), original);
        flag.store(false, Ordering::SeqCst);
        assert_eq!(verify(&path, None, &flag).unwrap().size, 128 * 1024 * 1024);
    }
    #[test]
    fn recovery_members_share_only_the_exact_family() {
        for name in [
            "资料.part01.rar",
            "资料.part2.rev",
            "资料.rar",
            "资料.r00",
            "资料_4_2_1.rev",
        ] {
            assert_eq!(family(name).as_deref(), Some("资料"));
        }
        assert_ne!(family("资料2.part1.rev"), family("资料.part1.rar"));
        assert_eq!(family("ordinary.txt"), None);
    }

    #[test]
    fn coexisting_single_archive_and_numbered_volumes_are_separate_groups() {
        let directory = tempfile::tempdir().unwrap();
        for name in [
            "same.rar",
            "same.part1.rar",
            "same.part2.rar",
            "same.part1.rev",
            "same2.part1.rar",
        ] {
            fs::write(directory.path().join(name), b"fixture").unwrap();
        }
        let parts = collect(&directory.path().join("same.part2.rar")).unwrap();
        assert_eq!(parts.data.len(), 2);
        assert_eq!(parts.recovery.len(), 1);
        assert!(!parts.files.iter().any(|p| p.ends_with("same.rar")));
        let single = collect(&directory.path().join("same.rar")).unwrap();
        assert_eq!(single.data.len(), 1);
        assert!(single.recovery.is_empty());
    }

    #[tokio::test]
    async fn renamed_zip_keeps_zip_diagnosis_instead_of_entering_rar_repair() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("actually-zip.rar");
        let mut writer = zip_aes::ZipWriter::new(fs::File::create(&path).unwrap());
        writer
            .start_file("good.txt", zip_aes::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"known content").unwrap();
        writer.finish().unwrap();
        let report = super::super::archive_diagnostics::diagnose_archive(
            &path,
            None,
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .unwrap();
        assert_eq!(report.actual_format, "ZIP");
        assert_eq!(report.status, "healthy");
    }

    #[tokio::test]
    #[ignore = "Requires an external Rar.exe; run explicitly with the RAR recovery development tool on PATH"]
    async fn real_repair_and_recovery_volumes_preserve_originals_and_passwords() {
        let tool = CompressionService::find_rar_encoder().expect("Rar.exe required");
        for encryption in ["plain", "content", "headers", "headers_split"] {
            let dir = tempfile::tempdir().unwrap();
            let source = dir.path().join("资料.bin");
            let bytes: Vec<u8> = (0..180_000)
                .map(|i| ((i * 73 + i / 257) % 256) as u8)
                .collect();
            fs::write(&source, &bytes).unwrap();
            let archive = dir.path().join("backup.rar");
            let password = if encryption == "plain" {
                None
            } else {
                Some("中文 recovery + !")
            };
            let password_arg = match encryption {
                "content" => "-p中文 recovery + !",
                "headers" | "headers_split" => "-hp中文 recovery + !",
                _ => "-c-",
            };
            command(
                &tool,
                &[
                    "a",
                    "-idq",
                    "-y",
                    "-cfg-",
                    "-m0",
                    "-rr5p",
                    password_arg,
                    if encryption == "headers_split" {
                        "-v60000b"
                    } else {
                        "-cfg-"
                    },
                    archive.to_str().unwrap(),
                    source.to_str().unwrap(),
                ],
            );
            let archive = if encryption == "headers_split" {
                dir.path().join("backup.part1.rar")
            } else {
                archive
            };
            assert_eq!(
                verify(&archive, password, &AtomicBool::new(false))
                    .unwrap()
                    .encrypted,
                password.is_some()
            );
            let damaged_path = if encryption == "headers_split" {
                dir.path().join("backup.part2.rar")
            } else {
                archive.clone()
            };
            let mut damaged = fs::read(&damaged_path).unwrap();
            damaged[3000] ^= 0x55;
            fs::write(&damaged_path, &damaged).unwrap();
            let out = dir.path().join("output");
            fs::create_dir(&out).unwrap();
            let flag = Arc::new(AtomicBool::new(false));
            let report = diagnose(&archive, password, flag.clone()).await.unwrap();
            assert!(report.can_repair, "{encryption}: {report:?}");
            let result = repair(&archive, &out, password, flag.clone())
                .await
                .unwrap_or_else(|e| panic!("{encryption}: {e:#}"));
            assert!(result.verified);
            assert_eq!(result.recovered_files, 1);
            assert_eq!(result.output_encrypted, password.is_some());
            assert_eq!(
                fs::read(&damaged_path).unwrap(),
                damaged,
                "source was modified"
            );
            let output = Path::new(&result.output_path);
            assert_eq!(verify(output, password, &flag).unwrap().files, 1);
            assert!(
                repair(&archive, &out, password, flag.clone())
                    .await
                    .is_err(),
                "existing result must not be overwritten"
            );
            if password.is_some() {
                let rejected = dir.path().join("wrong");
                fs::create_dir(&rejected).unwrap();
                assert!(repair(&archive, &rejected, Some("wrong"), flag.clone())
                    .await
                    .is_err());
                assert_eq!(
                    fs::read_dir(&rejected).unwrap().count(),
                    0,
                    "wrong password published an output"
                );
            }
        }
    }

    #[tokio::test]
    #[ignore = "Requires external Rar.exe for application recovery-volume creation"]
    async fn created_recovery_volumes_restore_a_missing_first_part_without_changing_retained_data() {
        let tool = CompressionService::find_rar_encoder().expect("Rar.exe required");
        for encrypted in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let input = dir.path().join("payload.bin"); fs::write(&input, vec![42u8; 180_000]).unwrap();
            let target = dir.path().join("parts.rar");
            let mut args = vec!["a", "-idq", "-y", "-cfg-", "-m0", "-v60000b"];
            if encrypted { args.push("-hp中文 recovery + !"); }
            args.extend([target.to_str().unwrap(), input.to_str().unwrap()]);
            command(&tool, &args);
            let first = dir.path().join("parts.part1.rar");
            let originals = collect(&first).unwrap().data.into_iter().map(|p| { let bytes = fs::read(&p).unwrap(); (p, bytes) }).collect::<Vec<_>>();
            let password = encrypted.then_some("中文 recovery + !");
            if encrypted { assert!(create_recovery_volumes(&first, dir.path(), 2, Some("wrong"), Arc::new(AtomicBool::new(false))).await.is_err()); }
            let result = create_recovery_volumes(&first, dir.path(), 2, password, Arc::new(AtomicBool::new(false))).await.unwrap();
            assert_eq!(result.output_files.len(), 2); assert_eq!(result.data_volumes, originals.len()); assert!(result.total_bytes > 0);
            assert!(create_recovery_volumes(&first, dir.path(), 1, password, Arc::new(AtomicBool::new(false))).await.is_err());
            for (file, bytes) in &originals { assert_eq!(fs::read(file).unwrap(), *bytes); }
            fs::remove_file(&first).unwrap();
            let out = dir.path().join("restored"); fs::create_dir(&out).unwrap();
            let restored = repair(Path::new(&result.output_files[0]), &out, password, Arc::new(AtomicBool::new(false))).await.unwrap();
            assert_eq!(fs::read(&restored.output_path).unwrap(), originals[0].1);
            assert!(!first.exists());
            for (file, bytes) in originals.iter().skip(1) { assert_eq!(fs::read(file).unwrap(), *bytes); }
        }
    }

    #[tokio::test]
    #[ignore = "Requires external Rar.exe and recovery-volume fixtures generated by it"]
    async fn real_rev_reconstruction_handles_first_middle_and_last_volume() {
        let tool = CompressionService::find_rar_encoder().expect("Rar.exe required");
        for missing in [vec![1], vec![2], vec![4], vec![1, 3], vec![1, 2, 3]] {
            let dir = tempfile::tempdir().unwrap();
            let source = dir.path().join("payload.bin");
            fs::write(&source, vec![42u8; 180_000]).unwrap();
            let archive = dir.path().join("parts.rar");
            command(
                &tool,
                &[
                    "a",
                    "-idq",
                    "-y",
                    "-cfg-",
                    "-m0",
                    "-v60000b",
                    "-hp中文 recovery + !",
                    archive.to_str().unwrap(),
                    source.to_str().unwrap(),
                ],
            );
            let first = dir.path().join("parts.part1.rar");
            command(
                &tool,
                &[
                    "rv2",
                    "-idq",
                    "-y",
                    "-cfg-",
                    "-p中文 recovery + !",
                    first.to_str().unwrap(),
                ],
            );
            let absent: Vec<_> = missing
                .iter()
                .map(|n| dir.path().join(format!("parts.part{n}.rar")))
                .collect();
            for file in &absent {
                assert!(file.is_file());
                fs::remove_file(file).unwrap();
            }
            let rev = dir.path().join("parts.part1.rev");
            let originals: Vec<_> = collect(&rev)
                .unwrap()
                .files
                .into_iter()
                .map(|p| {
                    let bytes = fs::read(&p).unwrap();
                    (p, bytes)
                })
                .collect();
            let out = dir.path().join("output");
            fs::create_dir(&out).unwrap();
            let flag = Arc::new(AtomicBool::new(false));
            let report = diagnose(&rev, Some("中文 recovery + !"), flag.clone())
                .await
                .unwrap();
            assert!(report.can_repair, "missing {missing:?}: {report:?}");
            if missing.len() > 2 {
                assert!(repair(&rev, &out, Some("中文 recovery + !"), flag.clone())
                    .await
                    .is_err());
                assert_eq!(fs::read_dir(&out).unwrap().count(), 0);
                continue;
            }
            let result = repair(&rev, &out, Some("中文 recovery + !"), flag.clone())
                .await
                .unwrap();
            assert_eq!(result.output_files.len(), 4);
            assert_eq!(result.recovered_files, 1);
            assert!(result.output_encrypted);
            assert!(result.output_path.ends_with("parts.part1.rar"));
            assert!(
                absent.iter().all(|p| !p.exists()),
                "missing source must not be reconstructed in place"
            );
            for (p, bytes) in originals {
                assert_eq!(fs::read(p).unwrap(), bytes);
            }
        }
    }
}
