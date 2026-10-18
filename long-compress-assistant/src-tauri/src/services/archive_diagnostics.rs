use crate::services::archive_browser;
use crate::services::archive_format::ArchiveFormat;
use crate::services::compression_format::{
    compression_route, infer_compression_format, CompressionRoute,
};
use crate::services::compression_verification::verify_native_with_zip_encoding;
use crate::services::zip_password::ZipPasswordEncoding;
use crate::services::zip_recovery_outcome::{RecoveryEntryResult, RecoveryStatus};
use crate::services::extraction_transaction;
use crate::services::native_extraction::seven_zip;
use crate::services::numeric_volume_reader::ZipInputReader;
use crate::services::split_archive_detector::{SplitArchiveDetector, SplitFormat};
use crate::services::universal_engine::UniversalCliEngine;
use anyhow::{Context, Result};
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticIssue {
    pub code: String,
    pub severity: String,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveDiagnosticReport {
    pub file_path: String,
    pub file_size: u64,
    pub actual_format: String,
    pub status: String,
    pub encrypted: bool,
    pub split_archive: bool,
    pub volumes_found: usize,
    pub missing_volumes: Vec<String>,
    pub total_files: usize,
    pub total_directories: usize,
    pub total_uncompressed_size: u64,
    pub total_uncompressed_size_known: bool,
    pub integrity_tested: bool,
    pub can_repair: bool,
    pub recoverability: String,
    pub issues: Vec<DiagnosticIssue>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZipRepairResult {
    pub output_path: String,
    pub recovered_files: usize,
    pub recovered_entries: Vec<String>,
    pub entry_results: Vec<RecoveryEntryResult>,
    pub recovered_directories: usize,
    pub skipped_entries: Vec<String>,
    pub verified: bool,
    pub partial: bool,
    pub directory_rebuilt: bool,
    pub output_encrypted: bool,
}

struct UnpublishedFile(Option<PathBuf>);

impl Drop for UnpublishedFile {
    fn drop(&mut self) {
        if let Some(path) = self.0.as_ref() {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn check_cancelled(cancelled: &AtomicBool) -> Result<()> {
    if cancelled.load(Ordering::SeqCst) {
        anyhow::bail!("Archive diagnosis cancelled");
    }
    Ok(())
}

fn copy_cancellable(
    reader: &mut impl Read,
    writer: &mut impl Write,
    cancelled: &AtomicBool,
) -> Result<u64> {
    let mut buffer = [0u8; 256 * 1024];
    let mut written = 0u64;
    loop {
        check_cancelled(cancelled)?;
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            return Ok(written);
        }
        writer.write_all(&buffer[..read])?;
        written = written.saturating_add(read as u64);
    }
}

fn copy_cancellable_bounded(
    reader: &mut impl Read,
    writer: &mut impl Write,
    cancelled: &AtomicBool,
    max_bytes: u64,
) -> Result<u64> {
    let mut buffer = [0u8; 256 * 1024];
    let mut written = 0u64;
    loop {
        check_cancelled(cancelled)?;
        let remaining = max_bytes.saturating_sub(written);
        if remaining == 0 {
            let mut overflow = [0u8; 1];
            if reader.read(&mut overflow)? == 0 {
                return Ok(written);
            }
            anyhow::bail!("ZIP repair expanded data exceeds the safety limit");
        }
        let capacity = usize::try_from(remaining.min(buffer.len() as u64)).unwrap_or(buffer.len());
        let read = reader.read(&mut buffer[..capacity])?;
        if read == 0 {
            return Ok(written);
        }
        writer.write_all(&buffer[..read])?;
        written += read as u64;
    }
}

fn format_label(path: &Path, header: &[u8]) -> String {
    let magic = ArchiveFormat::from_magic(header);
    if magic == ArchiveFormat::Unknown && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("zpaq")) { return "ZPAQ".into(); }
    if magic == ArchiveFormat::Unknown && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("alz")) { return "ALZ".into(); }
    if magic == ArchiveFormat::Unknown && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("egg")) { return "EGG".into(); }
    match magic {
        ArchiveFormat::Zip => "ZIP".to_string(),
        ArchiveFormat::SevenZip => "7Z".to_string(),
        ArchiveFormat::Rar => "RAR".to_string(),
        ArchiveFormat::AesEncrypted => "LONG AES".to_string(),
        ArchiveFormat::Unknown => {
            infer_compression_format(path.to_string_lossy().as_ref(), None).to_ascii_uppercase()
        }
        other => format!("{other:?}").to_ascii_uppercase(),
    }
}

#[cfg(test)]
fn split_evidence(path: &Path) -> Result<(bool, usize, Vec<String>)> {
    let Some(group) = SplitArchiveDetector::detect_split_archive(path)? else {
        return Ok((false, 1, Vec::new()));
    };
    Ok((true, group.parts.len(), group.missing_parts.iter().map(|part|
        part.file_name().unwrap_or_default().to_string_lossy().to_string()
    ).collect()))
}

fn classify_failure(
    message: &str,
    encrypted: bool,
    password_supplied: bool,
) -> (&'static str, &'static str, &'static str) {
    let lower = message.to_ascii_lowercase();
    // Structural/IO evidence and explicit ambiguous markers outrank incidental
    // words such as "encrypted" or "wrong password?" in an engine's message.
    if lower.contains("[archive-inspection:incomplete]") || lower.contains("missing disk") {
        ("missing_volume", "缺少分卷", "卷组不完整；请补齐分卷后重新检查，不必反复尝试密码")
    } else if lower.contains("[archive-inspection:damaged]") {
        // A path can contain incidental words such as "eof". Explicit format
        // evidence must outrank those free-text heuristics.
        ("damaged", "归档损坏", "归档结构已确认损坏或不完整；请重新获取完整文件，反复尝试密码不能修复损坏")
    } else if lower.contains("[egg-io]") {
        ("io_error", "EGG 文件读写失败", "请检查源文件是否可读、输出目录权限和磁盘可用空间。读写失败不能据此判定密码错误。")
    } else if lower.contains("[egg-password-encoding]") {
        ("password_encoding", "EGG 密码编码不兼容", "请明确选择原工具密码代码页；不会替换字符或据此判定文件损坏")
    } else if lower.contains("[egg-password]") {
        ("ambiguous_data", "EGG 密码或加密数据未通过校验", "请核对密码，并检查加密头或文件是否损坏；短头不匹配不能单独证明密码错误。")
    } else if lower.contains("[archive-inspection:unsupported]") {
        ("unsupported_feature", "暂不支持此归档特性", "当前引擎无法读取此算法或归档特性；这不等于文件损坏或密码错误")
    } else if lower.contains("[alz-io]") {
        ("io_error", "ALZ 文件读写失败", "请按原始原因检查空间、设备连接或访问权限，不应继续尝试密码")
    } else if lower.contains("[alz-encoding]") || lower.contains("[alz-password-encoding]") {
        ("password_encoding", "ALZ 编码不兼容", "请明确选择原工具使用的 ALZ 编码；不会用替换字符继续尝试密码")
    } else if lower.contains("[alz-password]") {
        ("ambiguous_data", "ALZ 密码或加密头异常", "密码短头不匹配，也可能加密头损坏；请核对密码和文件来源")
    } else if lower.contains("[zpaq-io]") {
        ("io_error", "ZPAQ 文件读写失败", "文件系统操作失败；请按原始错误检查空间、设备连接和访问权限，不应继续尝试密码")
    } else if lower.contains("permission denied") || lower.contains("[archive-inspection:permission]") {
        ("permission_error", "无法读取归档", "请确认文件访问权限或占用情况，处理后重新检查")
    } else if lower.contains("[archive-inspection:engine-unavailable]")
        || lower.contains(&crate::utils::archive_tools::missing_7z_message().to_ascii_lowercase())
        || lower.contains("7z command failed")
    {
        ("verification_unavailable", "无法完成内容校验", "归档引擎不可用，不能据此判定文件损坏或密码错误")
    } else if lower.contains("[archive-inspection:ambiguous-data]") || lower.contains("invalid authentication code") {
        ("ambiguous_data", "密码或加密内容异常", "当前无法区分密码错误与加密内容损坏；若已确认密码正确，请重新获取完整文件，反复尝试密码不能修复损坏")
    } else if lower.contains("eof") || lower.contains("central directory")
        || lower.contains("eocd") || lower.contains("end of central")
        || lower.contains("truncated") || lower.contains("unexpected end")
        || lower.contains("数据提前结束")
    {
        ("truncated", "归档被截断", "归档结构或数据不完整；请确认下载、复制已完成，必要时重新获取文件")
    } else if lower.contains("unsupported") || lower.contains("not supported") {
        ("unsupported_feature", "暂不支持此归档特性", "当前引擎无法读取此算法或归档特性；这不等于文件损坏或密码错误")
    } else if lower.contains("passwordrequired") || lower.contains("password required")
        || lower.contains("invalidpassword") || lower.contains("invalid password")
        || lower.contains("wrong password") || lower.contains("password is incorrect")
    {
        if encrypted && !password_supplied {
            (
                "password_required",
                "需要密码",
                "归档已加密，需要正确密码才能完成内容校验",
            )
        } else {
            ("wrong_password", "密码错误", "提供的密码无法解密归档内容")
        }
    } else if encrypted && password_supplied {
        (
            "ambiguous_data",
            "密码或加密内容异常",
            "当前无法区分密码错误与加密内容损坏；若已确认密码正确，请重新获取完整文件，反复尝试密码不能修复损坏",
        )
    } else if lower.contains("crc") || lower.contains("checksum") || lower.contains("invalid data")
    {
        (
            "crc_error",
            "内容校验失败",
            "至少一个条目的 CRC 或解压数据校验失败",
        )
    } else if lower.contains("data error") || lower.contains("corrupt")
        || lower.contains("[archive-inspection:damaged]") || lower.contains("invalid archive") {
        ("damaged", "归档损坏", "归档结构或内容无法通过完整性测试")
    } else {
        ("unknown_failure", "暂不能确定失败原因", "当前检查未能确认完整性；请保留原文件和诊断报告，不要仅据此反复尝试密码")
    }
}

fn record_failure(report: &mut ArchiveDiagnosticReport, error: &anyhow::Error, password_supplied: bool, readable_directory: bool, source: &Path) {
    if let Some(cause) = super::zip_password::encoding_error(error) {
        report.status = "password_encoding".into();
        report.can_repair = false;
        report.recoverability = "diagnostic_only".into();
        report.issues.push(DiagnosticIssue { code: "password_encoding".into(), severity: "error".into(),
            title: if cause.is_unrepresentable() { "密码无法按所选编码表示" } else { "密码编码转换失败" }.into(),
            detail: format!("ZIP 编码 {}：{cause}。请调整编码后重新检查，不能据此判定密码错误或文件损坏。", cause.encoding.as_str()) });
        return;
    }
    let io_kind = error.chain().find_map(|cause| {
        cause.downcast_ref::<std::io::Error>().map(|cause| cause.kind())
            .or_else(|| match cause.downcast_ref::<zip_aes::result::ZipError>() {
                Some(zip_aes::result::ZipError::Io(cause)) => Some(cause.kind()), _ => None,
            })
    });
    let (code, title, detail) = match io_kind {
        Some(std::io::ErrorKind::PermissionDenied) =>
            ("permission_error", "无法读取归档", "请确认文件访问权限或占用情况，处理后重新检查"),
        Some(std::io::ErrorKind::NotFound) =>
            ("source_unavailable", "来源文件不可用", "检查期间来源文件或分卷已不可用；请确认路径和存储设备后重新检查"),
        Some(std::io::ErrorKind::Other | std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) =>
            ("io_error", "读取归档失败", "无法完成文件读取；请确认文件占用情况和存储设备后重新检查"),
        _ => classify_failure(&format!("{error:#}"), report.encrypted, password_supplied),
    };
    report.status = code.to_string();
    report.can_repair = report.actual_format == "ZIP"
        && if report.encrypted {
            password_supplied && readable_directory
                && matches!(code, "crc_error" | "damaged" | "truncated" | "ambiguous_data")
        } else {
            matches!(code, "crc_error" | "damaged" | "truncated")
                && (readable_directory || super::zip_recovery_reader::can_scan_local(source))
        };
    report.recoverability = if report.can_repair && !readable_directory { "local_entries" }
        else if report.can_repair { "repairable" }
        else if matches!(code, "password_required" | "wrong_password") { "password_required" }
        else { "diagnostic_only" }.to_string();
    let message=format!("{error:#}");
    let detail=if message.contains("[archive-inspection:damaged] TAR") || message.contains("[archive-inspection:unsupported] TAR") {
        format!("{detail}。原始原因：{message}")
    } else {detail.to_string()};
    report.issues.push(DiagnosticIssue { code: code.to_string(), severity: "error".to_string(),
        title: title.to_string(), detail });
    report.evidence.push(format!("Integrity failure class: {code}"));
}

pub async fn diagnose_archive(
    path: &Path,
    password: Option<&str>,
    cancelled: Arc<AtomicBool>,
) -> Result<ArchiveDiagnosticReport> {
    diagnose_archive_with_encoding(path, password, None, cancelled).await
}

pub async fn diagnose_archive_with_encoding(
    path: &Path,
    password: Option<&str>,
    zip_password_encoding: Option<ZipPasswordEncoding>,
    cancelled: Arc<AtomicBool>,
) -> Result<ArchiveDiagnosticReport> {
    check_cancelled(&cancelled)?;
    if !path.is_file() {
        anyhow::bail!(
            "Archive does not exist or is not a file: {}",
            path.display()
        );
    }
    let password = password.filter(|value| !value.is_empty());
    if super::sfx_archive::is_executable_path(path)
        && archive_browser::detect_format(path)? == ArchiveFormat::Rar {
        return super::rar_recovery::diagnose(path, password, cancelled).await;
    }
    if super::rar_recovery::is_candidate(path) {
        let mut header = [0u8; 16];
        let len = std::fs::File::open(path)?.read(&mut header)?;
        if matches!(ArchiveFormat::from_magic(&header[..len]), ArchiveFormat::Rar | ArchiveFormat::Unknown) {
            return super::rar_recovery::diagnose(path, password, cancelled).await;
        }
    }
    let (group, alz_structure_error) = match SplitArchiveDetector::detect_split_archive(path) {
        Ok(group) => (group, None),
        Err(error) if super::alz_volumes::is_candidate(path) || super::egg_volumes::is_candidate(path) => (None, Some(error)),
        Err(error) => return Err(error),
    };
    let source = group.as_ref().map(|group| group.first_part.as_path()).unwrap_or(path);
    let logical = if group.as_ref().is_some_and(|group|
        matches!(group.format, SplitFormat::SevenZipSplit | SplitFormat::GenericNumeric)) {
        Path::new(source.file_stem().unwrap_or_default())
    } else { source };
    let missing_volumes = group.as_ref().map(|group| group.missing_parts.iter()
        .map(|part| part.file_name().unwrap_or_default().to_string_lossy().to_string())
        .collect()).unwrap_or_default();
    let mut report = ArchiveDiagnosticReport {
        file_path: path.to_string_lossy().to_string(),
        file_size: group.as_ref().map(|group| group.total_size).unwrap_or(path.metadata()?.len()),
        actual_format: format_label(logical, &[]),
        status: "checking".to_string(),
        encrypted: false,
        split_archive: group.is_some(),
        volumes_found: group.as_ref().map(|group| group.parts.len()).unwrap_or(1),
        missing_volumes,
        total_files: 0,
        total_directories: 0,
        total_uncompressed_size: 0,
        total_uncompressed_size_known: false,
        integrity_tested: false,
        can_repair: false,
        recoverability: "diagnostic_only".to_string(),
        issues: Vec::new(),
        evidence: Vec::new(),
    };
    report
        .evidence
        .push(format!("Archive size: {} bytes", report.file_size));
    if let Some(error) = alz_structure_error {
        report.actual_format = if super::egg_volumes::is_candidate(path) { "EGG" } else { "ALZ" }.into();
        record_failure(&mut report, &error, password.is_some(), false, path);
        return Ok(report);
    }
    if !report.missing_volumes.is_empty() {
        report.status = "missing_volume".to_string();
        report.recoverability = "diagnostic_only".to_string();
        report.issues.push(DiagnosticIssue {
            code: "missing_volume".to_string(),
            severity: "error".to_string(),
            title: "缺少分卷".to_string(),
            detail: format!("缺少：{}。请补齐到同一目录后重新检查，不必反复尝试密码。", report.missing_volumes.join("、")),
        });
        if group.as_ref().is_some_and(|group| group.format == SplitFormat::ZipSplit) {
            match super::zip_recovery_reader::RecoveryReader::open(path, &cancelled) {
                Ok(mut local) => {
                    report.issues[0].severity = "warning".into();
                    report.actual_format = "ZIP".into();
                    report.encrypted = local.encrypted;
                    report.total_directories = local.names.iter().filter(|name| name.ends_with(['/', '\\'])).count();
                    report.total_files = local.names.len() - report.total_directories;
                    report.total_uncompressed_size = local.expanded_bytes;
                    report.total_uncompressed_size_known = true;
                    report.recoverability = "missing_volume_entries".into();
                    report.status = "partial_recovery".into();
                    report.can_repair = report.total_files > 0 && (!local.encrypted || password.is_some());
                    report.issues[0].detail = format!("缺少：{}。目录仍可读取，已找到 {} 个可尝试救回的文件；涉及缺失卷或不完整的条目会跳过。补齐分卷后也可重新检查。", report.missing_volumes.join("、"), report.total_files);
                    if local.encrypted {
                        if let Some(password) = password {
                            if let Err(error) = local.check_password_headers_with_encoding(password, zip_password_encoding) {
                                check_cancelled(&cancelled)?;
                                let error: anyhow::Error = error.into();
                                if super::zip_password::encoding_error(&error).is_some() {
                                    record_failure(&mut report, &error, true, false, source);
                                    return Ok(report);
                                }
                                report.can_repair = false;
                                report.status = if matches!(error.downcast_ref::<zip_aes::result::ZipError>(), Some(zip_aes::result::ZipError::InvalidPassword)) { "wrong_password" } else { "ambiguous_data" }.into();
                                report.issues.push(DiagnosticIssue { code: report.status.clone(), severity: "warning".into(),
                                    title: "暂不能打开现有卷中的加密文件".into(),
                                    detail: "请确认密码；若可读取的加密条目也不完整，请补齐分卷后重试。未创建新文件。".into() });
                            }
                        } else {
                            report.status = "password_required".into();
                            report.issues.push(DiagnosticIssue { code: "password_required".into(), severity: "warning".into(),
                                title: "需要原压缩包密码".into(), detail: "填写密码后重新诊断，可尝试救回现有卷中的完整文件。".into() });
                        }
                    }
                    report.evidence.push(format!("Missing-disk recovery: {} candidate files; content will be checked during recovery", report.total_files));
                }
                Err(error) => {
                    check_cancelled(&cancelled)?;
                    report.issues.push(DiagnosticIssue { code: "partial_recovery_unavailable".into(), severity: "warning".into(),
                        title: "本次暂不能部分救援".into(), detail: format!("{error:#}") });
                }
            }
        }
        return Ok(report);
    }

    // Read across tiny numeric volumes and use the same standard ZIP view as
    // extraction. Keep the user's selected member in the exported report.
    let inspected = (|| -> Result<String> {
        if super::sfx_archive::is_executable_path(source) {
            return Ok(match archive_browser::detect_format(source)? {
                ArchiveFormat::Zip => "ZIP", ArchiveFormat::SevenZip => "7Z", ArchiveFormat::Rar => "RAR",
                _ => anyhow::bail!("Unsupported SFX payload"),
            }.to_string());
        }
        if group.as_ref().is_some_and(|group| group.format == SplitFormat::ZipSplit) {
            ZipInputReader::open_recovery(source)?;
            return Ok("ZIP".to_string());
        }
        let reader = ZipInputReader::open(source)?;
        if matches!(&reader, ZipInputReader::StandardZip(_)) { return Ok("ZIP".to_string()); }
        let mut header = Vec::new();
        reader.take(560).read_to_end(&mut header)?;
        Ok(format_label(logical, &header))
    })();
    let actual_format = match inspected {
        Ok(format) => format,
        Err(error) => { record_failure(&mut report, &error, password.is_some(), false, source); return Ok(report); }
    };
    report.actual_format = actual_format.clone();
    report.evidence.push(format!("Magic/extension format: {actual_format}"));
    let requested_format = match actual_format.as_str() {
        "ZIP" => "zip".to_string(), "7Z" => "7z".to_string(), "RAR" => "rar".to_string(),
        "LZIP" => if super::compression_format::is_tar_wrapped_archive(logical, "tar.lz") { "tar.lz" } else { "lz" }.to_string(),
        _ => infer_compression_format(logical.to_string_lossy().as_ref(), None),
    };

    let metadata = archive_browser::browse_archive_with_encoding(source, password, zip_password_encoding).await;
    if let Ok(metadata) = &metadata {
        if metadata.format != report.actual_format {
            report
                .evidence
                .push(format!("Container detail: {}", metadata.format));
            report.actual_format = metadata.format.clone();
        }
        report.encrypted = metadata.encrypted;
        report.total_files = metadata.total_files;
        report.total_directories = metadata.total_directories;
        report.total_uncompressed_size = metadata.total_uncompressed_size;
        report.total_uncompressed_size_known = metadata.entries.iter().all(|entry| entry.is_dir || entry.size_known != Some(false));
        report.evidence.push(format!(
            "Entries: {} files, {} directories",
            metadata.total_files, metadata.total_directories
        ));
    }
    check_cancelled(&cancelled)?;

    if actual_format == "ZIP" {
        if metadata.is_err() {
            if let Ok(mut local) = super::zip_recovery_reader::RecoveryReader::open(source, &cancelled) {
                if local.directory_rebuilt {
                    report.encrypted = local.encrypted;
                    report.total_directories = local.names.iter().filter(|name| name.ends_with(['/', '\\'])).count();
                    report.total_files = local.names.len() - report.total_directories;
                    report.total_uncompressed_size = local.expanded_bytes;
                    report.total_uncompressed_size_known = false;
                    report.status = "directory_lost".to_string();
                    report.recoverability = "local_entries".to_string();
                    report.can_repair = !local.encrypted || password.is_some();
                    report.issues.push(DiagnosticIssue { code: "directory_lost".to_string(), severity: "warning".to_string(),
                        title: "目录信息已丢失，可尝试救回完整文件".to_string(),
                        detail: format!("已找到 {} 个具有明确边界的文件条目，原包总条目数无法确认。只恢复完整读取并通过校验的文件，原包保留。", report.total_files) });
                    if local.encrypted {
                        if let Some(password) = password {
                            if let Err(error) = local.check_password_headers_with_encoding(password, zip_password_encoding) {
                                let error: anyhow::Error = error.into();
                                if super::zip_password::encoding_error(&error).is_some() {
                                    record_failure(&mut report, &error, true, false, source);
                                    return Ok(report);
                                }
                                report.can_repair = false;
                                let wrong = matches!(error.downcast_ref::<zip_aes::result::ZipError>(), Some(zip_aes::result::ZipError::InvalidPassword));
                                report.status = if wrong { "wrong_password" } else { "ambiguous_data" }.to_string();
                                report.issues.push(DiagnosticIssue { code: report.status.clone(), severity: "error".to_string(),
                                    title: if wrong { "密码无法打开已找到的加密条目" } else { "暂不能读取已找到的加密条目" }.to_string(),
                                    detail: "请确认密码与来源完整性，填写后可重新检查；未创建新文件".to_string() });
                            }
                        } else {
                            report.status = "password_required".to_string();
                            report.issues.push(DiagnosticIssue { code: "password_required".to_string(), severity: "warning".to_string(),
                                title: "需要原压缩包密码".to_string(), detail: "找到的本地条目包含加密文件，填写密码后可尝试救援".to_string() });
                        }
                    }
                    report.evidence.push("Local ZIP metadata rebuilt; file content will be verified during recovery".to_string());
                    report.issues.extend(local.notes.into_iter().enumerate().map(|(index, detail)| DiagnosticIssue {
                        code: format!("local_boundary_{index}"), severity: "warning".to_string(), title: "部分内容无法确认".to_string(), detail,
                    }));
                    check_cancelled(&cancelled)?;
                    return Ok(report);
                }
            }
        }
        check_cancelled(&cancelled)?;
        match UniversalCliEngine::zip_requires_password(source) {
            Ok(encrypted) => report.encrypted = encrypted,
            Err(error) => { record_failure(&mut report, &error, password.is_some(), false, source); return Ok(report); }
        }
    } else if actual_format == "7Z" {
        report.encrypted = seven_zip::requires_password(source).unwrap_or(report.encrypted);
    } else if actual_format == "EGG" {
        match super::egg_engine::requires_password(source, &Default::default(), cancelled.clone()).await {
            Ok(encrypted) => report.encrypted = encrypted,
            Err(error) => { record_failure(&mut report, &error, password.is_some(), false, source); return Ok(report); }
        }
    } else if actual_format == "ALZ" {
        // Reject unsupported methods/structure before asking for a password.
        let options = crate::models::compression::DecompressOptions { zip_password_encoding, ..Default::default() };
        match super::alz_engine::requires_password(source, &options, cancelled.clone()).await {
            Ok(encrypted) => report.encrypted = encrypted,
            Err(error) => { record_failure(&mut report, &error, password.is_some(), false, source); return Ok(report); }
        }
    } else if actual_format == "ZPAQ" {
        match super::zpaq_engine::requires_password(source) {
            Ok(encrypted) => report.encrypted = encrypted,
            Err(error) => { record_failure(&mut report, &error, password.is_some(), false, source); return Ok(report); }
        }
    }
    if report.encrypted && password.filter(|value| !value.is_empty()).is_none() {
        report.status = "password_required".to_string();
        report.recoverability = "password_required".to_string();
        report.issues.push(DiagnosticIssue {
            code: "password_required".to_string(),
            severity: "warning".to_string(),
            title: "需要密码".to_string(),
            detail: "归档已加密；报告不会记录密码内容".to_string(),
        });
        return Ok(report);
    }

    let route = compression_route(&requested_format);
    let verify_path = source.to_path_buf();
    let verify_password = password.map(str::to_string);
    let verify_cancelled = cancelled.clone();
    let verification = match route {
        _ if actual_format == "EGG" => {
            report.integrity_tested = true;
            super::egg_engine::verify(source, password, &crate::models::compression::DecompressOptions { zip_password_encoding, ..Default::default() }, cancelled.clone()).await
        }
        _ if actual_format == "ALZ" => {
            report.integrity_tested = true;
            super::alz_engine::verify(source, password, &crate::models::compression::DecompressOptions { zip_password_encoding, ..Default::default() }, cancelled.clone()).await
        }
        _ if actual_format == "ZPAQ" => {
            report.integrity_tested = true;
            report.evidence.push("ZPAQ current-file view; format checksums do not authenticate all header attributes".into());
            super::zpaq_engine::verify(source, password, cancelled.clone()).await
        }
        Some(route)
            if !matches!(
                route,
                CompressionRoute::Rar | CompressionRoute::Wim | CompressionRoute::Lzma
            ) =>
        {
            report.integrity_tested = true;
            tauri::async_runtime::spawn_blocking(move || {
                verify_native_with_zip_encoding(route, &verify_path, verify_password.as_deref(), zip_password_encoding, || {
                    verify_cancelled.load(Ordering::SeqCst)
                })
            })
            .await
            .context("Archive verification worker failed")?
            .map(|_| ())
        }
        _ if password.is_none() => {
            report.integrity_tested = true;
            UniversalCliEngine::test_integrity(source, None).await
        }
        _ => {
            report.issues.push(DiagnosticIssue {
                code: "verification_unavailable".to_string(),
                severity: "warning".to_string(),
                title: "仅完成结构诊断".to_string(),
                detail: "该格式无法在不暴露密码的前提下调用外部工具做完整内容测试".to_string(),
            });
            report.status = "structure_only".to_string();
            report.recoverability = "diagnostic_only".to_string();
            return Ok(report);
        }
    };
    check_cancelled(&cancelled)?;

    match verification {
        Ok(()) => {
            report.status = "healthy".to_string();
            report.recoverability = "not_needed".to_string();
            report
                .evidence
                .push("Integrity test completed without errors".to_string());
        }
        Err(error) => {
            let message = format!("{error:#}");
            if message.to_ascii_lowercase().contains("cancel") {
                return Err(error);
            }
            record_failure(&mut report, &error, password.is_some(), metadata.is_ok(), source);
        }
    }
    Ok(report)
}

pub fn repair_zip_to_new(
    path: &Path,
    output: &Path,
    cancelled: &AtomicBool,
) -> Result<ZipRepairResult> {
    repair_zip_with_password_to_new(path, output, None, cancelled)
}

pub fn repair_zip_with_password_to_new(
    path: &Path,
    output: &Path,
    password: Option<&str>,
    cancelled: &AtomicBool,
) -> Result<ZipRepairResult> {
    repair_zip_with_encoding_to_new(path, output, password, None, cancelled)
}

/// Explicit input encodings are retained in the repaired encrypted copy.
/// Automatic legacy input keeps the existing UTF-8 output convention.
pub fn repair_zip_with_encoding_to_new(
    path: &Path,
    output: &Path,
    password: Option<&str>,
    zip_password_encoding: Option<ZipPasswordEncoding>,
    cancelled: &AtomicBool,
) -> Result<ZipRepairResult> {
    check_cancelled(cancelled)?;
    if !path.is_file() {
        anyhow::bail!("ZIP source does not exist: {}", path.display());
    }
    if output.exists() {
        anyhow::bail!("Repair output already exists: {}", output.display());
    }
    let source_canonical = path.canonicalize()?;
    let output_absolute = if output.is_absolute() {
        output.to_path_buf()
    } else {
        std::env::current_dir()?.join(output)
    };
    if source_canonical == output_absolute {
        anyhow::bail!("ZIP repair must write to a new file");
    }
    let group = SplitArchiveDetector::detect_split_archive(path)?;
    let input = group.as_ref().filter(|group| group.is_complete)
        .map(|group| group.first_part.as_path()).unwrap_or(path);
    let mut source = super::zip_recovery_reader::RecoveryReader::open(input, cancelled)?;
    let output_encrypted = source.encrypted;
    let password = password.filter(|value| !value.is_empty());
    anyhow::ensure!(!output_encrypted || password.is_some(), "需要原压缩包密码才能恢复文件；未创建新文件");
    let output_password = if output_encrypted { password } else { None };
    let output_password_bytes = output_password.map(|password|
        super::zip_password::encode(password, zip_password_encoding.unwrap_or_default())).transpose()?;
    let entry_count = source.names.len();
    let declared_expanded_bytes = source.expanded_bytes;
    let directory_rebuilt = source.directory_rebuilt;
    let validate_limits = |count, bytes| {
        if let Some(group) = group.as_ref().filter(|group| !group.is_complete) {
            extraction_transaction::validate_resource_limits_for_size(group.total_size, count, bytes)
        } else {
            extraction_transaction::validate_resource_limits(input, count, bytes)
        }
    };
    validate_limits(entry_count, declared_expanded_bytes)?;

    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    // During rebuilding, the temporary ZIP and one uncompressed spool can coexist.
    extraction_transaction::validate_disk_capacity(
        parent,
        declared_expanded_bytes.saturating_mul(2),
    )?;
    let temporary = super::output_publish_transaction::staged_output_path(output, "long-repair")?;
    let mut guard = UnpublishedFile(Some(temporary.clone()));
    let temp_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let mut writer = zip_aes::ZipWriter::new(temp_file);
    let mut recovered_files = 0usize;
    let mut recovered_entries = Vec::new();
    let mut recovered_directories = 0usize;
    let mut recovered_bytes = 0u64;
    let mut recovered_encrypted_files = 0usize;
    let mut skipped_entries = source.notes.clone();
    let mut entry_results = source.entry_results.clone();
    let mut written_names = std::collections::HashSet::new();

    for index in 0..entry_count {
        check_cancelled(cancelled)?;
        let label = source.names[index].clone();
        let mut entry = match source.entry_with_encoding(index, password, zip_password_encoding) {
            Ok(entry) => entry,
            Err(error) => {
                if matches!(&error, zip_aes::result::ZipError::Io(cause)
                    if cause.get_ref().is_some_and(|cause| cause.is::<super::zip_password::ZipPasswordEncodingError>())) {
                    return Err(error.into());
                }
                let reason = match error {
                    zip_aes::result::ZipError::InvalidPassword => "密码无法打开此条目，未恢复".to_string(),
                    zip_aes::result::ZipError::UnsupportedArchive(_) => "当前不支持此条目的压缩或加密方法，未恢复".to_string(),
                    other => format!("无法读取此条目，未恢复：{other}"),
                };
                entry_results.push(RecoveryEntryResult::new(Some(label.clone()), RecoveryStatus::Unknown, &reason));
                skipped_entries.push(format!("{label}: {reason}")); continue;
            }
        };
        let name = entry.name().replace('\\', "/");
        let declared_entry_size = entry.size();
        let entry_encrypted = entry.encrypted();
        if entry.enclosed_name().is_none() {
            entry_results.push(RecoveryEntryResult::new(Some(name.clone()), RecoveryStatus::Unknown, "条目路径不能写入输出目录"));
            skipped_entries.push(format!("{name}: unsafe path"));
            continue;
        }
        let options =
            zip_aes::write::SimpleFileOptions::default().compression_method(zip_aes::CompressionMethod::Deflated)
                .large_file(declared_entry_size >= u32::MAX as u64);
        let options = match output_password_bytes.as_deref() {
            Some(bytes) => options.with_aes_encryption_bytes(zip_aes::AesMode::Aes256, bytes),
            None => options,
        };
        if entry.is_dir() {
            if !written_names.insert(name.clone()) { continue; }
            writer.add_directory(name, options)?;
            recovered_directories += 1;
            continue;
        }
        let spool_path = parent.join(format!(".long-repair-entry-{}", uuid::Uuid::new_v4()));
        let mut spool_guard = UnpublishedFile(Some(spool_path.clone()));
        let mut spool = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&spool_path)?;
        let remaining = extraction_transaction::MAX_EXTRACTED_BYTES.saturating_sub(recovered_bytes);
        let entry_limit = remaining.min(declared_entry_size);
        let copied = match copy_cancellable_bounded(&mut entry, &mut spool, cancelled, entry_limit)
        {
            Ok(copied) => copied,
            Err(error)
                if error
                    .to_string()
                    .contains("expanded data exceeds the safety limit") =>
            {
                return Err(error);
            }
            Err(error) => {
                if error.to_string().to_ascii_lowercase().contains("cancelled") {
                    return Err(error);
                }
                let detail = error.to_string();
                let failed_check = ["checksum", "crc", "authentication"].iter().any(|marker| detail.to_ascii_lowercase().contains(marker));
                let reason = if detail.to_ascii_lowercase().contains("checksum") || detail.to_ascii_lowercase().contains("crc") {
                    "内容校验失败".to_string()
                } else if detail.to_ascii_lowercase().contains("authentication") {
                    "加密内容校验失败".to_string()
                } else { detail };
                skipped_entries.push(format!("{name}: {reason}，未恢复此条目"));
                entry_results.push(RecoveryEntryResult::new(Some(name), if failed_check { RecoveryStatus::ChecksumFailed } else { RecoveryStatus::Unknown }, reason));
                continue;
            }
        };
        if copied != declared_entry_size {
            entry_results.push(RecoveryEntryResult::new(Some(name.clone()), RecoveryStatus::ChecksumFailed, "条目声明大小与实际读取大小不一致"));
            skipped_entries.push(format!("{name}: 条目声明大小与实际读取大小不一致，未恢复"));
            continue;
        }
        if !written_names.insert(name.clone()) {
            entry_results.push(RecoveryEntryResult::new(Some(name.clone()), RecoveryStatus::Unknown, "包内名称重复，未恢复此条目"));
            skipped_entries.push(format!("{name}: 包内名称重复，未恢复此条目"));
            continue;
        }
        let next_recovered_bytes = recovered_bytes.saturating_add(copied);
        validate_limits(recovered_files + recovered_directories + 1, next_recovered_bytes)?;
        spool.rewind()?;
        writer.start_file(&name, options)?;
        copy_cancellable(&mut spool, &mut writer, cancelled)?;
        recovered_bytes = next_recovered_bytes;
        recovered_files += 1;
        entry_results.push(RecoveryEntryResult::new(Some(name.clone()), RecoveryStatus::Recovered, "完整读取并通过内容校验"));
        recovered_entries.push(name);
        if entry_encrypted { recovered_encrypted_files += 1; }
        std::fs::remove_file(&spool_path)?;
        spool_guard.0.take();
    }
    // A plain file in a mixed ZIP cannot prove that the supplied password works.
    // Publish only after a complete encrypted file passes its content check.
    anyhow::ensure!(!output_encrypted || recovered_encrypted_files > 0,
        "未能用此密码完整读取任何加密文件；请确认密码和文件完整性，未创建新文件");
    if recovered_files == 0 {
        anyhow::bail!("No complete file entries could be recovered; no repair output was created");
    }
    writer.finish()?.sync_all()?;
    verify_native_with_zip_encoding(CompressionRoute::Zip, &temporary, output_password, Some(zip_password_encoding.unwrap_or_default()), || {
        cancelled.load(Ordering::SeqCst)
    })
    .context("Rebuilt ZIP failed integrity verification")?;
    check_cancelled(cancelled)?;
    super::output_publish_transaction::transfer_no_replace(&temporary, output)?;
    #[cfg(not(windows))] std::fs::remove_file(&temporary)?;
    guard.0.take();
    let partial = directory_rebuilt || !skipped_entries.is_empty();
    Ok(ZipRepairResult {
        output_path: output.to_string_lossy().to_string(),
        recovered_files,
        recovered_entries,
        entry_results,
        recovered_directories,
        skipped_entries,
        verified: true,
        partial,
        directory_rebuilt,
        output_encrypted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use crate::models::compression::{CompressionOptions, TaskLogSeverity};
    use crate::services::native_compression::{self, CompressionRuntime};
    use sha2::{Digest, Sha256};
    use std::io::Write;
    use tauri::Window;

    #[cfg(windows)]
    #[tokio::test]
    async fn fixed_encoding_real_zip_diagnosis_integrity_and_lost_directory_recovery() {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/legacy-zip-password-1416");
        let expected = std::fs::read(fixtures.join("payload.txt")).unwrap();
        for (encoding, password) in [
            (ZipPasswordEncoding::Cp932, "日本語パスワード+!1416"),
            (ZipPasswordEncoding::Cp936, "简体密码测试+!1416"),
            (ZipPasswordEncoding::Cp950, "繁體密碼測試+!1416"),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let original = fixtures.join(format!("{}.zip", encoding.as_str()));
            let before = std::fs::read(&original).unwrap();
            let split = super::super::split_compression::SplitCompressionService::split_archive_file(
                &original, &temp.path().join("numeric.zip"), 37, || Ok(())).unwrap();
            for input in [&original, &split.part_files[0]] {
                let report = diagnose_archive_with_encoding(input, Some(password), Some(encoding), Arc::new(AtomicBool::new(false))).await.unwrap();
                assert_eq!(report.status, "healthy");
                assert!(report.integrity_tested && report.encrypted && !report.can_repair);
                UniversalCliEngine::test_integrity_with_zip_encoding(input, Some(password), Some(encoding)).await.unwrap();
                assert!(UniversalCliEngine::test_integrity_with_zip_encoding(input, Some(password), Some(ZipPasswordEncoding::Utf8)).await.is_err());
                let report = diagnose_archive_with_encoding(input, Some("🔑"), Some(encoding), Arc::new(AtomicBool::new(false))).await.unwrap();
                assert_eq!(report.status, "password_encoding", "{report:?}");
                assert!(!report.can_repair);
                assert!(report.issues.iter().any(|issue| issue.code == "password_encoding" && issue.detail.contains(encoding.as_str())));
            }
            // Remove only the central directory from a private copy. The
            // original producer's local header, descriptor and ciphertext stay intact.
            let end = before.windows(4).rposition(|bytes| bytes == b"PK\x05\x06").unwrap();
            let central = u32::from_le_bytes(before[end+16..end+20].try_into().unwrap()) as usize;
            let lost = temp.path().join("directory-lost.zip");
            std::fs::write(&lost, &before[..central]).unwrap();
            let report = diagnose_archive_with_encoding(&lost, Some(password), Some(encoding), Arc::new(AtomicBool::new(false))).await.unwrap();
            assert_eq!(report.status, "directory_lost", "{report:?}");
            assert!(report.can_repair && report.encrypted);
            let invalid = diagnose_archive_with_encoding(&lost, Some("🔑"), Some(encoding), Arc::new(AtomicBool::new(false))).await.unwrap();
            assert_eq!(invalid.status, "password_encoding");
            assert!(!invalid.can_repair);
            let absent = temp.path().join("invalid.zip");
            assert!(repair_zip_with_encoding_to_new(&lost, &absent, Some("🔑"), Some(encoding), &AtomicBool::new(false)).is_err());
            assert!(!absent.exists());
            let output = temp.path().join("repaired.zip");
            let result = repair_zip_with_encoding_to_new(&lost, &output, Some(password), Some(encoding), &AtomicBool::new(false)).unwrap();
            assert!(result.verified && result.output_encrypted && result.partial);
            assert_eq!(result.recovered_files, 1);
            let mut archive = zip_aes::ZipArchive::new(File::open(&output).unwrap()).unwrap();
            let mut content = Vec::new();
            super::super::zip_password::open_named_entry_with_encoding(&mut archive, "payload.txt", password, Some(encoding)).unwrap().read_to_end(&mut content).unwrap();
            assert_eq!(content, expected);
            assert!(verify_native_with_zip_encoding(CompressionRoute::Zip, &output, Some(password), Some(ZipPasswordEncoding::Utf8), || false).is_err());
            if let Some(tools) = std::env::var_os("LONG_ZIP_1416_INTEROP_TOOLS") {
                let destination = temp.path().join("independent");
                let result = crate::utils::process::command(PathBuf::from(tools).join(encoding.as_str()).join("7z.exe"))
                    .arg("x").arg("-y").arg(format!("-p{password}")).arg(format!("-o{}", destination.display())).arg(&output).output().unwrap();
                assert!(result.status.success(), "independent repair extraction failed: {}", String::from_utf8_lossy(&result.stderr));
                assert_eq!(std::fs::read(destination.join("payload.txt")).unwrap(), expected);
            }
            assert_eq!(std::fs::read(&original).unwrap(), before);
            assert_eq!(std::fs::read(&lost).unwrap(), before[..central]);
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn fixed_encoding_missing_standard_disk_recovery_preserves_password_choice() {
        for (encoding, password) in [
            (ZipPasswordEncoding::Cp932, "日本語パスワード+!1416"),
            (ZipPasswordEncoding::Cp936, "简体密码测试+!1416"),
            (ZipPasswordEncoding::Cp950, "繁體密碼測試+!1416"),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let source = temp.path().join("original.zip");
            let bytes = super::super::zip_password::encode(password, encoding).unwrap();
            let mut writer = zip_aes::ZipWriter::new(File::create(&source).unwrap());
            let options = zip_aes::write::SimpleFileOptions::default()
                .compression_method(zip_aes::CompressionMethod::Stored)
                .with_aes_encryption_bytes(zip_aes::AesMode::Aes256, &bytes);
            writer.start_file("跨卷.bin", options).unwrap(); writer.write_all(&vec![42; 140_000]).unwrap();
            writer.start_file("完整.txt", options).unwrap(); writer.write_all(b"intact after missing disk").unwrap();
            writer.finish().unwrap();
            let parts = super::super::standard_zip_writer::split(&source, &temp.path().join("standard.zip"),
                65_536, || Ok(()), |_,_| {}).unwrap().part_files;
            assert!(parts.len() > 2);
            let before: Vec<_> = parts.iter().map(|path| std::fs::read(path).unwrap()).collect();
            let withheld = temp.path().join("withheld-volume");
            std::fs::rename(&parts[1], &withheld).unwrap();
            let input = parts.last().unwrap();
            let report = diagnose_archive_with_encoding(input, Some(password), Some(encoding), Arc::new(AtomicBool::new(false))).await.unwrap();
            assert!(report.can_repair && report.encrypted, "{report:?}");
            assert_eq!(report.status, "partial_recovery");
            let invalid = diagnose_archive_with_encoding(input, Some("🔑"), Some(encoding), Arc::new(AtomicBool::new(false))).await.unwrap();
            assert_eq!(invalid.status, "password_encoding");
            assert!(!invalid.can_repair);
            let output = temp.path().join("recovered.zip");
            let result = repair_zip_with_encoding_to_new(input, &output, Some(password), Some(encoding), &AtomicBool::new(false)).unwrap();
            assert!(result.partial && result.verified && result.output_encrypted);
            assert_eq!(result.recovered_entries, vec!["完整.txt"]);
            assert!(!result.skipped_entries.is_empty());
            let mut archive = zip_aes::ZipArchive::new(File::open(&output).unwrap()).unwrap();
            let mut restored = Vec::new();
            super::super::zip_password::open_named_entry_with_encoding(&mut archive, "完整.txt", password, Some(encoding)).unwrap().read_to_end(&mut restored).unwrap();
            assert_eq!(restored, b"intact after missing disk");
            assert!(archive.index_for_name("跨卷.bin").is_none());
            if let Some(tools) = std::env::var_os("LONG_ZIP_1416_INTEROP_TOOLS") {
                let destination = temp.path().join("independent");
                let result = crate::utils::process::command(PathBuf::from(tools).join(encoding.as_str()).join("7z.exe"))
                    .arg("x").arg("-y").arg(format!("-p{password}")).arg(format!("-o{}", destination.display())).arg(&output).output().unwrap();
                assert!(result.status.success());
                assert_eq!(std::fs::read(destination.join("完整.txt")).unwrap(), b"intact after missing disk");
                assert!(!destination.join("跨卷.bin").exists());
            }
            std::fs::rename(&withheld, &parts[1]).unwrap();
            assert_eq!(parts.iter().map(|path| std::fs::read(path).unwrap()).collect::<Vec<_>>(), before);
        }
    }

    #[test]
    fn bounded_repair_copy_stops_before_writing_overflow_data() {
        let mut source = std::io::Cursor::new(vec![7u8; 9]);
        let mut output = Vec::new();

        let error = copy_cancellable_bounded(&mut source, &mut output, &AtomicBool::new(false), 8)
            .unwrap_err();

        assert!(error
            .to_string()
            .contains("expanded data exceeds the safety limit"));
        assert_eq!(output.len(), 8);
    }

    #[derive(Default)]
    struct TestRuntime {
        cancelled: Arc<AtomicBool>,
    }

    impl CompressionRuntime for TestRuntime {
        fn check_cancellation(&self) -> Result<()> {
            check_cancelled(&self.cancelled)
        }
        fn cancellation_flag(&self) -> Arc<AtomicBool> {
            self.cancelled.clone()
        }
        fn copy_buffer_size(&self) -> usize {
            256 * 1024
        }
        fn emit_log(&self, _: &Window, _: &str, _: &str, _: TaskLogSeverity) {}
        fn emit_progress(&self, _: &Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {}
    }

    #[tokio::test]
    async fn lost_standard_zip_directory_recovers_cross_disk_plain_and_password_files() {
        for encrypted in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let input = temp.path().join("original.zip");
            let password = "中文 secret + !";
            let plain = zip_aes::write::SimpleFileOptions::default().compression_method(zip_aes::CompressionMethod::Stored);
            let options = if encrypted { plain.with_aes_encryption(zip_aes::AesMode::Aes256, password) } else { plain };
            let content = vec![23u8; 180_000];
            let mut writer = zip_aes::ZipWriter::new(File::create(&input).unwrap());
            writer.start_file("跨卷.bin", options).unwrap(); writer.write_all(&content).unwrap();
            writer.start_file("完整.txt", options).unwrap(); writer.write_all(b"complete").unwrap(); writer.finish().unwrap();
            let parts = super::super::standard_zip_writer::split(&input, &temp.path().join("分卷.zip"), 65_536, || Ok(()), |_, _| {}).unwrap().part_files;
            assert!(parts.len() >= 3);
            let originals: Vec<Vec<u8>> = parts.iter().map(|p| std::fs::read(p).unwrap()).collect();
            let last = originals.last().unwrap();
            let end = last.len() - 22;
            let cd_disk = u16::from_le_bytes(last[end+6..end+8].try_into().unwrap()) as usize;
            let cd_relative = u32::from_le_bytes(last[end+16..end+20].try_into().unwrap()) as usize;
            let cd_start = originals.iter().take(cd_disk).map(Vec::len).sum::<usize>() + cd_relative;
            let mut bytes = originals.concat(); bytes[cd_start..].fill(0);
            let mut at = 0;
            for (part, original) in parts.iter().zip(&originals) { std::fs::write(part, &bytes[at..at+original.len()]).unwrap(); at += original.len(); }
            let report = diagnose_archive(&parts[1], encrypted.then_some(password), Arc::new(AtomicBool::new(false))).await.unwrap();
            assert_eq!(report.status, "directory_lost", "{report:?}"); assert!(report.can_repair && report.split_archive);
            let output = temp.path().join("rescued.zip");
            if encrypted { assert!(repair_zip_with_password_to_new(&parts[1], &output, Some("wrong"), &AtomicBool::new(false)).is_err()); assert!(!output.exists()); }
            let result = repair_zip_with_password_to_new(&parts[1], &output, encrypted.then_some(password), &AtomicBool::new(false)).unwrap();
            assert!(result.partial && result.verified && result.directory_rebuilt); assert_eq!(result.recovered_files, 2);
            assert_eq!(result.output_encrypted, encrypted);
            let mut archive = zip_aes::ZipArchive::new(File::open(output).unwrap()).unwrap();
            let mut restored = Vec::new();
            if encrypted { super::super::zip_password::open_named_entry(&mut archive, "跨卷.bin", password).unwrap().read_to_end(&mut restored).unwrap(); }
            else { archive.by_name("跨卷.bin").unwrap().read_to_end(&mut restored).unwrap(); }
            assert_eq!(restored, content);
            assert_eq!(parts.iter().flat_map(|p| std::fs::read(p).unwrap()).collect::<Vec<_>>(), bytes);
            std::fs::remove_file(&parts[1]).unwrap();
            let missing = diagnose_archive(parts.last().unwrap(), Some(password), Arc::new(AtomicBool::new(false))).await.unwrap();
            assert_eq!(missing.status, "missing_volume"); assert!(!missing.can_repair);
        }
    }

    #[tokio::test]
    async fn lost_directory_password_zip_recovers_intact_encrypted_files_and_rejects_wrong_password() {
        for aes in [true, false] {
            let temp = tempfile::tempdir().unwrap();
            let input = temp.path().join("密码目录丢失.zip");
            let password = "救援密码 + !";
            let plain = zip_aes::write::SimpleFileOptions::default().compression_method(zip_aes::CompressionMethod::Stored).large_file(aes);
            let secret = if aes { plain.with_aes_encryption(zip_aes::AesMode::Aes256, password) }
                else { plain };
            let mut writer = zip_aes::ZipWriter::new(File::create(&input).unwrap());
            writer.start_file("公开说明.txt", plain).unwrap(); writer.write_all(b"public").unwrap();
            writer.add_directory("资料/", plain.large_file(false)).unwrap();
            writer.start_file("坏文件.bin", secret).unwrap(); writer.write_all(&vec![31; 4096]).unwrap();
            writer.start_file("资料/完整.txt", secret).unwrap(); writer.write_all(b"complete secret").unwrap();
            writer.finish().unwrap();
            let mut archive = zip_aes::ZipArchive::new(File::open(&input).unwrap()).unwrap();
            let index = archive.index_for_name("坏文件.bin").unwrap();
            let data = archive.by_index_raw(index).unwrap().data_start().unwrap() as usize;
            drop(archive);
            let mut bytes = std::fs::read(&input).unwrap();
            let central = u32::from_le_bytes(bytes[bytes.len()-6..bytes.len()-2].try_into().unwrap()) as usize;
            bytes.truncate(central);
            if aes { bytes[data + 50] ^= 1; }
            else { bytes = zipcrypto_local_fixture(&bytes, password); }
            std::fs::write(&input, &bytes).unwrap();
            let report = diagnose_archive(&input, None, Arc::new(AtomicBool::new(false))).await.unwrap();
            assert_eq!(report.status, "password_required", "AES={aes}: {report:?}"); assert!(report.encrypted && !report.can_repair);
            let wrong = diagnose_archive(&input, Some("wrong"), Arc::new(AtomicBool::new(false))).await.unwrap();
            assert_eq!(wrong.status, "wrong_password", "AES={aes}: {:?}", super::super::zip_recovery_reader::RecoveryReader::open(&input, &AtomicBool::new(false)).unwrap().check_password_headers("wrong")); assert!(!wrong.can_repair);
            let parts = super::super::split_compression::SplitCompressionService::split_archive_file(
                &input, &temp.path().join("数字.zip"), 97, || Ok(())).unwrap().part_files;
            for (route, path) in [("single", &input), ("numeric", &parts[1])] {
                let report = diagnose_archive(path, Some(password), Arc::new(AtomicBool::new(false))).await.unwrap();
                assert_eq!(report.status, "directory_lost"); assert!(report.encrypted && report.can_repair && !report.integrity_tested);
                assert_eq!(report.total_files, 3); assert_eq!(report.total_directories, 1);
                let output = temp.path().join(format!("{route}-new.zip"));
                assert!(repair_zip_with_password_to_new(path, &output, Some("wrong"), &AtomicBool::new(false)).is_err());
                assert!(!output.exists());
                let result = repair_zip_with_password_to_new(path, &output, Some(password), &AtomicBool::new(false)).unwrap();
                assert!(result.directory_rebuilt && result.output_encrypted && result.partial && result.verified);
                assert_eq!(result.recovered_files, 2); assert_eq!(result.recovered_directories, 1);
                assert!(result.skipped_entries.iter().any(|note| note.contains("坏文件.bin")));
                let mut archive = zip_aes::ZipArchive::new(File::open(&output).unwrap()).unwrap();
                let mut restored = Vec::new();
                super::super::zip_password::open_named_entry(&mut archive, "资料/完整.txt", password).unwrap().read_to_end(&mut restored).unwrap();
                assert_eq!(restored, b"complete secret"); assert!(archive.index_for_name("坏文件.bin").is_none());
                assert_eq!(std::fs::read(&input).unwrap(), bytes);
            }
        }
    }

    // A later uncertain entry must not change the trust of the intact prefix.
    #[tokio::test]
    async fn lost_directory_keeps_only_complete_encrypted_prefix_and_never_uses_plain_files_as_password_proof() {
        for first_encrypted in [true, false] {
            for descriptor in [true, false] {
                let temp = tempfile::tempdir().unwrap();
                let input = temp.path().join("不完整的密码包.zip");
                let password = "correct password";
                let plain = zip_aes::write::SimpleFileOptions::default().compression_method(zip_aes::CompressionMethod::Stored);
                let encrypted = plain.with_aes_encryption(zip_aes::AesMode::Aes256, password);
                let mut writer = zip_aes::ZipWriter::new(File::create(&input).unwrap());
                writer.start_file("first.txt", if first_encrypted { encrypted } else { plain }).unwrap();
                writer.write_all(b"intact prefix contents").unwrap();
                writer.start_file("second.txt", encrypted).unwrap(); writer.write_all(&vec![17; 4096]).unwrap();
                writer.finish().unwrap();
                let mut archive = zip_aes::ZipArchive::new(File::open(&input).unwrap()).unwrap();
                let index = archive.index_for_name("second.txt").unwrap();
                let second = archive.by_index_raw(index).unwrap();
                let start = second.header_start() as usize;
                let data = second.data_start().unwrap() as usize;
                drop(second); drop(archive);
                let mut bytes = std::fs::read(&input).unwrap();
                if descriptor {
                    let central = u32::from_le_bytes(bytes[bytes.len()-6..bytes.len()-2].try_into().unwrap()) as usize;
                    bytes.truncate(central);
                    let flags = u16::from_le_bytes(bytes[start+6..start+8].try_into().unwrap()) | 8;
                    bytes[start+6..start+8].copy_from_slice(&flags.to_le_bytes());
                    bytes[start+14..start+26].fill(0);
                } else { bytes.truncate(data + 100); }
                std::fs::write(&input, &bytes).unwrap();
                let report = diagnose_archive(&input, Some(password), Arc::new(AtomicBool::new(false))).await.unwrap();
                assert!(report.encrypted);
                assert_eq!(report.can_repair, first_encrypted, "{report:?}");
                let output = temp.path().join("new.zip");
                let result = repair_zip_with_password_to_new(&input, &output, Some(password), &AtomicBool::new(false));
                if first_encrypted {
                    let result = result.unwrap(); assert!(result.output_encrypted && result.partial);
                    assert_eq!(result.recovered_files, 1); assert!(!result.skipped_entries.is_empty());
                } else { assert!(result.is_err()); assert!(!output.exists()); }
                assert_eq!(std::fs::read(&input).unwrap(), bytes);
            }
        }
    }

    // The zip 8.6 writer always uses unknown-size descriptors for ZipCrypto.
    // Construct declared-size records here; external 7-Zip covers this path in
    // the desktop check so the fixture implementation is not the only oracle.
    fn zipcrypto_local_fixture(input: &[u8], password: &str) -> Vec<u8> {
        fn crc(mut value: u32, byte: u8) -> u32 {
            value ^= byte as u32;
            for _ in 0..8 { value = if value & 1 != 0 { (value >> 1) ^ 0xedb88320 } else { value >> 1 }; }
            value
        }
        fn update(keys: &mut [u32; 3], byte: u8) {
            keys[0] = crc(keys[0], byte);
            keys[1] = keys[1].wrapping_add(keys[0] & 255).wrapping_mul(134775813).wrapping_add(1);
            keys[2] = crc(keys[2], (keys[1] >> 24) as u8);
        }
        let mut result = Vec::new(); let mut at = 0;
        while at < input.len() {
            let size = u32::from_le_bytes(input[at+18..at+22].try_into().unwrap()) as usize;
            let name_len = u16::from_le_bytes(input[at+26..at+28].try_into().unwrap()) as usize;
            let header_len = 30 + name_len + u16::from_le_bytes(input[at+28..at+30].try_into().unwrap()) as usize;
            let name = String::from_utf8_lossy(&input[at+30..at+30+name_len]);
            let mut header = input[at..at+header_len].to_vec();
            if name.contains("坏文件") || name.contains("完整") {
                let flags = u16::from_le_bytes(header[6..8].try_into().unwrap()) | 1;
                header[6..8].copy_from_slice(&flags.to_le_bytes());
                header[18..22].copy_from_slice(&((size+12) as u32).to_le_bytes());
                let mut keys = [0x12345678, 0x23456789, 0x34567890];
                for byte in password.bytes() { update(&mut keys, byte); }
                let mut clear = vec![19; 12]; clear[11] = header[17];
                clear.extend_from_slice(&input[at+header_len..at+header_len+size]);
                let mut encrypted: Vec<u8> = clear.into_iter().map(|byte| {
                    let temp = (keys[2] as u16 | 2) as u32;
                    let cipher = byte ^ (temp.wrapping_mul(temp ^ 1) >> 8) as u8;
                    update(&mut keys, byte); cipher
                }).collect();
                if name.contains("坏文件") { encrypted[50] ^= 1; }
                result.extend_from_slice(&header); result.extend_from_slice(&encrypted);
            } else { result.extend_from_slice(&input[at..at+header_len+size]); }
            at += header_len + size;
        }
        result
    }

    #[tokio::test]
    async fn encrypted_zip_rescue_preserves_password_and_only_intact_files_across_volume_types() {
        use crate::services::zip_password::{encode, ZipPasswordEncoding};
        use zip_aes::unstable::write::FileOptionsExt;
        for aes in [true, false] {
            for encoding in [ZipPasswordEncoding::Utf8, ZipPasswordEncoding::WindowsLocal] {
                let password = "救援密码 + 测试!";
                let Ok(encoded) = encode(password, encoding) else { continue };
                let temp = tempfile::tempdir().unwrap();
                let archive = temp.path().join("收到的密码.zipx");
                let plain = zip_aes::write::SimpleFileOptions::default()
                    .compression_method(zip_aes::CompressionMethod::Stored);
                let options = if aes {
                    plain.with_aes_encryption_bytes(zip_aes::AesMode::Aes256, &encoded)
                } else { plain.with_deprecated_encryption(&encoded).unwrap() };
                let mut writer = zip_aes::ZipWriter::new(File::create(&archive).unwrap());
                writer.start_file("公开说明.txt", plain).unwrap();
                writer.write_all(b"a plain file must not prove the password").unwrap();
                writer.add_directory("资料/空目录/", plain).unwrap();
                writer.start_file("坏文件.bin", options).unwrap();
                writer.write_all(&vec![17; 140_000]).unwrap();
                writer.start_file("资料/完整.txt", options).unwrap();
                writer.write_all(b"intact secret contents").unwrap();
                writer.finish().unwrap();
                let mut reader = zip_aes::ZipArchive::new(File::open(&archive).unwrap()).unwrap();
                let bad_index = reader.index_for_name("坏文件.bin").unwrap();
                let at = reader.by_index_raw(bad_index).unwrap().data_start().unwrap() as usize;
                drop(reader);
                let mut bytes = std::fs::read(&archive).unwrap(); bytes[at + 50] ^= 1;
                std::fs::write(&archive, bytes).unwrap();
                let numeric = super::super::split_compression::SplitCompressionService::split_archive_file(
                    &archive, &temp.path().join("数字.zip"), 65536, || Ok(())).unwrap().part_files;
                let standard = super::super::standard_zip_writer::split(
                    &archive, &temp.path().join("标准.zip"), 65536, || Ok(()), |_,_| {}).unwrap().part_files;
                for (route, input) in [("single", &archive), ("numeric", &numeric[1]), ("standard", &standard[1])] {
                    let before: Vec<_> = std::iter::once(&archive).chain(numeric.iter()).chain(standard.iter()).map(|path| hash(path)).collect();
                    let report = diagnose_archive(input, Some(password), Arc::new(AtomicBool::new(false))).await.unwrap();
                    assert!(report.can_repair && report.encrypted, "{aes}/{encoding:?}/{route}: {report:?}");
                    let absent = temp.path().join(format!("{route}-missing.zip"));
                    assert!(repair_zip_with_password_to_new(input, &absent, None, &AtomicBool::new(false)).unwrap_err().to_string().contains("密码"));
                    assert!(!absent.exists());
                    let wrong = temp.path().join(format!("{route}-wrong.zip"));
                    assert!(repair_zip_with_password_to_new(input, &wrong, Some("wrong"), &AtomicBool::new(false)).is_err());
                    assert!(!wrong.exists(), "plain entry must not allow a wrong password to publish");
                    let output = temp.path().join(format!("{route}-recovered.zip"));
                    let result = repair_zip_with_password_to_new(input, &output, Some(password), &AtomicBool::new(false)).unwrap();
                    assert!(result.output_encrypted && result.partial && result.verified && !result.directory_rebuilt);
                    assert_eq!(result.recovered_files, 2);
                    assert!(result.skipped_entries.iter().any(|note| note.contains("坏文件.bin")));
                    let mut recovered = zip_aes::ZipArchive::new(File::open(&output).unwrap()).unwrap();
                    assert!(recovered.by_name("资料/完整.txt").is_err());
                    assert!(recovered.index_for_name("坏文件.bin").is_none());
                    let public_index = recovered.index_for_name("公开说明.txt").unwrap();
                    assert!(recovered.by_index_raw(public_index).unwrap().encrypted());
                    let mut content = String::new();
                    super::super::zip_password::open_named_entry(&mut recovered, "资料/完整.txt", password).unwrap().read_to_string(&mut content).unwrap();
                    assert_eq!(content, "intact secret contents");
                    assert_eq!(std::iter::once(&archive).chain(numeric.iter()).chain(standard.iter()).map(|path| hash(path)).collect::<Vec<_>>(), before);
                    assert!(!std::fs::read_dir(temp.path()).unwrap().any(|entry| entry.unwrap().file_name().to_string_lossy().starts_with(".long-repair")));
                }
            }
        }
    }

    #[test]
    fn duplicate_local_names_do_not_abort_the_whole_recovery() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("duplicates.zip");
        let mut writer = zip::ZipWriter::new(File::create(&archive).unwrap());
        for contents in ["first intact file", "second intact file"] {
            writer.start_file("same.txt", zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Stored)).unwrap();
            writer.write_all(contents.as_bytes()).unwrap();
        }
        writer.finish().unwrap();
        let mut bytes = std::fs::read(&archive).unwrap();
        let end = bytes.len() - 22;
        let central = u32::from_le_bytes(bytes[end + 16..end + 20].try_into().unwrap()) as usize;
        bytes.truncate(central); std::fs::write(&archive, bytes).unwrap();
        let result = repair_zip_to_new(&archive, &temp.path().join("new.zip"), &AtomicBool::new(false)).unwrap();
        assert!(result.verified && result.partial);
        assert_eq!(result.recovered_files, 1);
        assert!(result.skipped_entries.iter().any(|note| note.contains("名称重复")));
    }

    fn create_stored_zip(path: &Path) {
        let file = File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        writer.start_file("good.txt", options).unwrap();
        writer.write_all(b"known-good-entry").unwrap();
        writer.start_file("damaged.txt", options).unwrap();
        writer
            .write_all(b"unique-damaged-payload-0123456789")
            .unwrap();
        writer.finish().unwrap();
    }

    fn hash(path: &Path) -> Vec<u8> {
        Sha256::digest(std::fs::read(path).unwrap()).to_vec()
    }

    #[tokio::test]
    async fn healthy_zip_has_structured_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("healthy.zip");
        create_stored_zip(&archive);
        let report = diagnose_archive(&archive, None, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap();
        assert_eq!(report.status, "healthy");
        assert_eq!(report.actual_format, "ZIP");
        assert_eq!(report.total_files, 2);
        assert!(report.integrity_tested);
        assert!(!report.can_repair);
    }

    #[tokio::test]
    async fn diagnosis_accepts_any_tiny_numeric_zip_volume_and_checks_the_whole_group() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("收到的资料.zip");
        create_stored_zip(&archive);
        let volumes = crate::services::split_compression::SplitCompressionService::split_archive_file(
            &archive, &temp.path().join("分卷资料.zip"), 29, || Ok(()),
        ).unwrap();
        let selected = &volumes.part_files[2];
        let report = diagnose_archive(selected, None, Arc::new(AtomicBool::new(false))).await.unwrap();
        assert_eq!(report.file_path, selected.to_string_lossy());
        assert_eq!(report.status, "healthy", "{report:?}");
        assert_eq!(report.actual_format, "ZIP");
        assert!(report.split_archive && report.integrity_tested);
        assert_eq!(report.volumes_found, volumes.part_files.len());
        assert_eq!(report.file_size, std::fs::metadata(&archive).unwrap().len());
        assert_eq!(report.total_files, 2);
        assert!(!report.can_repair);

        std::fs::remove_file(&volumes.part_files[1]).unwrap();
        let missing = diagnose_archive(selected, None, Arc::new(AtomicBool::new(false))).await.unwrap();
        assert_eq!(missing.status, "missing_volume");
        assert_eq!(missing.missing_volumes, vec!["分卷资料.zip.002"]);
        assert!(!missing.integrity_tested);
    }

    #[tokio::test]
    async fn diagnosis_reports_zip_declared_missing_disks_even_with_only_the_terminal_file() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("只收到终卷.zip");
        let mut end = vec![0u8; 22];
        end[..4].copy_from_slice(b"PK\x05\x06");
        end[4..6].copy_from_slice(&2u16.to_le_bytes());
        std::fs::write(&archive, end).unwrap();
        let report = diagnose_archive(&archive, Some("不用试密码"), Arc::new(AtomicBool::new(false))).await.unwrap();
        assert_eq!(report.status, "missing_volume");
        assert_eq!(report.missing_volumes, vec!["只收到终卷.z01", "只收到终卷.z02"]);
        assert!(!report.integrity_tested && !report.can_repair);
        assert!(!serde_json::to_string(&report).unwrap().contains("不用试密码"));
    }

    #[tokio::test]
    async fn corrupted_encrypted_zip_returns_a_report_instead_of_requesting_another_password() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("密码正确但文件坏了.zip");
        let mut writer = zip_aes::ZipWriter::new(File::create(&archive).unwrap());
        writer.start_file("受损文件.txt", zip_aes::write::SimpleFileOptions::default()
            .compression_method(zip_aes::CompressionMethod::Stored)
            .with_aes_encryption(zip_aes::AesMode::Aes256, "真实密码")).unwrap();
        writer.write_all(&vec![42u8; 128]).unwrap();
        writer.finish().unwrap();
        let data_start = zip_aes::ZipArchive::new(File::open(&archive).unwrap()).unwrap()
            .by_index_raw(0).unwrap().data_start().unwrap() as usize;
        let mut bytes = std::fs::read(&archive).unwrap();
        bytes[data_start + 25] ^= 1;
        std::fs::write(&archive, bytes).unwrap();
        let before = hash(&archive);
        let report = diagnose_archive(&archive, Some("真实密码"), Arc::new(AtomicBool::new(false))).await.unwrap();
        assert_eq!(report.status, "ambiguous_data", "{report:?}");
        assert!(report.encrypted && report.integrity_tested);
        assert!(report.can_repair);
        assert!(report.issues.iter().any(|issue| issue.detail.contains("反复尝试密码")));
        assert!(!serde_json::to_string(&report).unwrap().contains("真实密码"));
        assert_eq!(hash(&archive), before);
    }

    #[tokio::test]
    async fn plain_zip_with_zstd_entries_uses_the_same_reader_as_extraction() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("新算法.zipx");
        let mut writer = zip_aes::ZipWriter::new(File::create(&archive).unwrap());
        writer.start_file("说明.txt", zip_aes::write::SimpleFileOptions::default()
            .compression_method(zip_aes::CompressionMethod::Zstd)).unwrap();
        writer.write_all(b"actual zstd entry payload").unwrap();
        writer.finish().unwrap();
        let report = diagnose_archive(&archive, None, Arc::new(AtomicBool::new(false))).await.unwrap();
        assert_eq!(report.status, "healthy", "{report:?}");
        assert_eq!(report.total_files, 1);
        assert!(report.integrity_tested);
    }

    #[tokio::test]
    async fn unsupported_zip_method_is_not_advertised_as_damage_or_repairable() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("未知算法.zip");
        create_stored_zip(&archive);
        let mut bytes = std::fs::read(&archive).unwrap();
        let local = bytes.windows(4).position(|value| value == b"PK\x03\x04").unwrap();
        let central = bytes.windows(4).position(|value| value == b"PK\x01\x02").unwrap();
        // PPMd (98) is now supported; use a genuinely unknown method while
        // retaining the strict not-damaged/not-repairable assertions.
        bytes[local + 8..local + 10].copy_from_slice(&60000u16.to_le_bytes());
        bytes[central + 10..central + 12].copy_from_slice(&60000u16.to_le_bytes());
        std::fs::write(&archive, bytes).unwrap();
        let report = diagnose_archive(&archive, None, Arc::new(AtomicBool::new(false))).await.unwrap();
        assert_eq!(report.status, "unsupported_feature", "{report:?}");
        assert!(!report.can_repair);
    }

    #[tokio::test]
    async fn diagnosis_reads_tiny_tar_gzip_numeric_volumes_as_one_archive() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("目录.tar.gz");
        let encoder = flate2::write::GzEncoder::new(File::create(&archive).unwrap(), flate2::Compression::default());
        let mut tar = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(18); header.set_mode(0o644); header.set_cksum();
        tar.append_data(&mut header, "说明.txt", &b"tar volume payload"[..]).unwrap();
        tar.into_inner().unwrap().finish().unwrap();
        let volumes = crate::services::split_compression::SplitCompressionService::split_archive_file(
            &archive, &temp.path().join("目录分卷.tar.gz"), 11, || Ok(()),
        ).unwrap();
        let report = diagnose_archive(&volumes.part_files[2], None, Arc::new(AtomicBool::new(false))).await.unwrap();
        assert_eq!(report.status, "healthy", "{report:?}");
        assert_eq!(report.total_files, 1);
        assert!(report.split_archive && report.integrity_tested);
    }

    #[tokio::test]
    async fn corrupted_entry_is_rebuilt_to_a_new_verified_zip_without_mutating_source() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("damaged.zip");
        let repaired = temp.path().join("damaged.repaired.zip");
        create_stored_zip(&archive);
        let mut bytes = std::fs::read(&archive).unwrap();
        let needle = b"unique-damaged-payload-0123456789";
        let offset = bytes
            .windows(needle.len())
            .position(|window| window == needle)
            .unwrap();
        bytes[offset + 4] ^= 0xff;
        std::fs::write(&archive, bytes).unwrap();
        let before = hash(&archive);

        let report = diagnose_archive(&archive, None, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap();
        assert_eq!(report.status, "crc_error");
        assert!(report.can_repair);

        let result = repair_zip_to_new(&archive, &repaired, &AtomicBool::new(false)).unwrap();
        assert_eq!(result.recovered_files, 1);
        assert_eq!(result.skipped_entries.len(), 1);
        assert!(result.verified);
        assert_eq!(hash(&archive), before);
        let mut zip = zip::ZipArchive::new(File::open(repaired).unwrap()).unwrap();
        let mut content = String::new();
        zip.by_name("good.txt")
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert_eq!(content, "known-good-entry");
    }

    #[tokio::test]
    async fn truncated_directory_is_reported_and_complete_entries_are_recovered() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("truncated.zip");
        let output = temp.path().join("should-not-exist.zip");
        create_stored_zip(&archive);
        let mut bytes = std::fs::read(&archive).unwrap();
        bytes.truncate(bytes.len() - 24);
        std::fs::write(&archive, bytes).unwrap();
        let before = hash(&archive);

        let report = diagnose_archive(&archive, None, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap();
        assert_eq!(report.status, "directory_lost");
        assert!(report.can_repair);
        let result = repair_zip_to_new(&archive, &output, &AtomicBool::new(false)).unwrap();
        assert_eq!(result.recovered_files, 2);
        assert!(result.verified && result.directory_rebuilt && result.partial);
        assert_eq!(hash(&archive), before);
    }

    #[test]
    fn lost_directory_recovery_preserves_complete_entries_and_excludes_bad_payloads() {
        let temp = tempfile::tempdir().unwrap();
        for corrupt in [false, true] {
            let archive = temp.path().join(format!("lost-{corrupt}.zip"));
            create_stored_zip(&archive);
            let mut bytes = std::fs::read(&archive).unwrap();
            assert_eq!(&bytes[bytes.len()-22..bytes.len()-18], b"PK\x05\x06");
            let central = u32::from_le_bytes(bytes[bytes.len()-6..bytes.len()-2].try_into().unwrap());
            bytes.truncate(central as usize);
            if corrupt {
                let needle = b"unique-damaged-payload-0123456789";
                let at = bytes.windows(needle.len()).position(|part| part == needle).unwrap();
                bytes[at] ^= 1;
            }
            std::fs::write(&archive, bytes).unwrap();
            let before = hash(&archive); let output = archive.with_extension("recovered.zip");
            let result = repair_zip_to_new(&archive, &output, &AtomicBool::new(false)).unwrap();
            assert!(result.partial && result.directory_rebuilt && result.verified);
            assert_eq!(result.recovered_files, if corrupt { 1 } else { 2 });
            assert_eq!(hash(&archive), before);
            let mut rebuilt = zip::ZipArchive::new(File::open(output).unwrap()).unwrap();
            let mut good = String::new(); rebuilt.by_name("good.txt").unwrap().read_to_string(&mut good).unwrap();
            assert_eq!(good, "known-good-entry");
            if corrupt { assert!(rebuilt.by_name("bad.txt").is_err()); }
        }
    }

    #[test]
    fn recovery_does_not_guess_descriptor_boundaries_or_publish_truncated_first_payload() {
        let temp = tempfile::tempdir().unwrap();
        for scenario in ["descriptor", "truncated", "bad-zip64", "cancel", "existing"] {
            let archive = temp.path().join(format!("{scenario}.zip"));
            create_stored_zip(&archive); let mut bytes = std::fs::read(&archive).unwrap();
            let central = u32::from_le_bytes(bytes[bytes.len()-6..bytes.len()-2].try_into().unwrap()) as usize;
            let (first_data, second_header) = {
                let mut zip = zip_aes::ZipArchive::new(File::open(&archive).unwrap()).unwrap();
                let first = zip.by_index_raw(0).unwrap();
                (first.data_start().unwrap() as usize, (first.data_start().unwrap()+first.compressed_size()) as usize)
            };
            bytes.truncate(central);
            if scenario == "descriptor" { bytes[second_header+6..second_header+8].copy_from_slice(&8u16.to_le_bytes()); }
            if scenario == "bad-zip64" { bytes[second_header+18..second_header+26].fill(255); }
            if scenario == "truncated" { bytes.truncate(first_data+2); }
            std::fs::write(&archive, &bytes).unwrap(); let before = hash(&archive);
            let output = archive.with_extension("recovered.zip");
            if scenario == "existing" { std::fs::write(&output, b"keep this file").unwrap(); }
            let result = repair_zip_to_new(&archive, &output, &AtomicBool::new(scenario == "cancel"));
            if matches!(scenario, "descriptor" | "bad-zip64") {
                let result = result.unwrap(); assert_eq!(result.recovered_files, 1);
                assert!(result.partial && result.directory_rebuilt && !result.skipped_entries.is_empty());
                assert!(result.entry_results.iter().any(|entry| entry.name.as_deref() == Some("damaged.txt") && entry.status == RecoveryStatus::Unknown));
                assert!(result.entry_results.iter().any(|entry| entry.name.is_none() && entry.status == RecoveryStatus::Unknown));
            } else {
                assert!(result.is_err());
                if scenario == "existing" { assert_eq!(std::fs::read(&output).unwrap(), b"keep this file"); }
                else { assert!(!output.exists()); }
            }
            assert_eq!(hash(&archive), before);
            assert!(!std::fs::read_dir(temp.path()).unwrap().any(|entry| entry.unwrap().file_name().to_string_lossy().starts_with(".long-repair")));
        }
    }

    #[tokio::test]
    async fn complete_plain_standard_disks_can_recover_good_entries_after_crc_damage() {
        let temp = tempfile::tempdir().unwrap(); let original = temp.path().join("original.zip");
        let mut writer = zip_aes::ZipWriter::new(File::create(&original).unwrap());
        let options = zip_aes::write::SimpleFileOptions::default().compression_method(zip_aes::CompressionMethod::Stored);
        writer.start_file("good.txt", options).unwrap(); writer.write_all(b"known-good-entry").unwrap();
        writer.start_file("bad.txt", options).unwrap(); writer.write_all(&vec![65;90000]).unwrap(); writer.finish().unwrap();
        let output = temp.path().join("parts.zip");
        let parts = super::super::standard_zip_writer::split(&original, &output, 65536, || Ok(()), |_,_| {}).unwrap().part_files;
        let mut first = std::fs::read(&parts[0]).unwrap();
        let at = first.windows(32).position(|part| part == [65;32]).unwrap(); first[at] ^= 1; std::fs::write(&parts[0], first).unwrap();
        let before: Vec<_> = parts.iter().map(|path| hash(path)).collect();
        let report = diagnose_archive(&parts[0], None, Arc::new(AtomicBool::new(false))).await.unwrap();
        assert!(report.can_repair && report.split_archive); assert_eq!(report.status, "crc_error");
        let recovered = temp.path().join("recovered.zip");
        let result = repair_zip_to_new(&parts[0], &recovered, &AtomicBool::new(false)).unwrap();
        assert_eq!(result.recovered_files, 1); assert!(result.partial && !result.directory_rebuilt && result.verified);
        assert_eq!(parts.iter().map(|path| hash(path)).collect::<Vec<_>>(), before);
    }

    #[tokio::test]
    async fn lost_directory_with_plain_folder_then_encrypted_file_never_offers_plaintext_repair() {
        let temp = tempfile::tempdir().unwrap(); let archive = temp.path().join("mixed.zip");
        let mut writer = zip_aes::ZipWriter::new(File::create(&archive).unwrap());
        writer.add_directory("plain-folder/", zip_aes::write::SimpleFileOptions::default()).unwrap();
        writer.start_file("secret.txt", zip_aes::write::SimpleFileOptions::default().with_aes_encryption(zip_aes::AesMode::Aes256,"secret")).unwrap();
        writer.write_all(b"must stay encrypted").unwrap(); writer.finish().unwrap();
        let mut bytes = std::fs::read(&archive).unwrap();
        let central = u32::from_le_bytes(bytes[bytes.len()-6..bytes.len()-2].try_into().unwrap()) as usize;
        bytes.truncate(central); std::fs::write(&archive, bytes).unwrap(); let before = hash(&archive);
        let report = diagnose_archive(&archive, None, Arc::new(AtomicBool::new(false))).await.unwrap();
        assert!(!report.can_repair);
        let output = temp.path().join("must-not-exist.zip");
        assert!(repair_zip_to_new(&archive,&output,&AtomicBool::new(false)).is_err());
        assert!(!output.exists()); assert_eq!(hash(&archive), before);
    }

    #[tokio::test]
    async fn lost_directory_recovery_reads_tiny_complete_numeric_volumes() {
        let temp = tempfile::tempdir().unwrap(); let source = temp.path().join("lost.zip"); create_stored_zip(&source);
        let mut bytes = std::fs::read(&source).unwrap();
        let central = u32::from_le_bytes(bytes[bytes.len()-6..bytes.len()-2].try_into().unwrap()) as usize;
        bytes.truncate(central); std::fs::write(&source,bytes).unwrap();
        let volumes = super::super::split_compression::SplitCompressionService::split_archive_file(&source,&temp.path().join("numeric.zip"),11,|| Ok(())).unwrap().part_files;
        let before: Vec<_> = volumes.iter().map(|file| hash(file)).collect();
        let report = diagnose_archive(&volumes[2], None, Arc::new(AtomicBool::new(false))).await.unwrap();
        assert!(report.can_repair && report.split_archive);
        let result = repair_zip_to_new(&volumes[2],&temp.path().join("recovered.zip"),&AtomicBool::new(false)).unwrap();
        assert_eq!(result.recovered_files,2); assert!(result.partial && result.directory_rebuilt && result.verified);
        assert_eq!(volumes.iter().map(|file| hash(file)).collect::<Vec<_>>(),before);
    }

    #[tokio::test]
    async fn missing_split_volume_is_reported_before_archive_testing() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("parts.zip");
        std::fs::write(&archive, b"terminal").unwrap();
        std::fs::write(temp.path().join("parts.z01"), b"one").unwrap();
        std::fs::write(temp.path().join("parts.z03"), b"three").unwrap();
        let report = diagnose_archive(&archive, None, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap();
        assert_eq!(report.status, "missing_volume");
        assert_eq!(report.missing_volumes, vec!["parts.z02"]);
        assert!(!report.integrity_tested);
    }

    #[tokio::test]
    async fn encrypted_zip_distinguishes_missing_and_wrong_password_without_logging_it() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("secret.zip");
        let file = File::create(&archive).unwrap();
        let mut writer = zip_aes::ZipWriter::new(file);
        let options = zip_aes::write::SimpleFileOptions::default()
            .compression_method(zip_aes::CompressionMethod::Deflated)
            .with_aes_encryption(zip_aes::AesMode::Aes256, "correct-secret");
        writer.start_file("secret.txt", options).unwrap();
        writer.write_all(b"classified").unwrap();
        writer.finish().unwrap();

        let missing = diagnose_archive(&archive, None, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap();
        assert_eq!(missing.status, "password_required");
        let wrong = diagnose_archive(
            &archive,
            Some("wrong-secret"),
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .unwrap();
        assert_eq!(wrong.status, "wrong_password", "report: {wrong:?}");
        let report_json = serde_json::to_string(&wrong).unwrap();
        assert!(!report_json.contains("wrong-secret"));
        assert!(!report_json.contains("correct-secret"));
    }

    #[tokio::test]
    async fn real_seven_zip_reports_health_and_wrong_password() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("payload.txt");
        std::fs::write(&source, b"seven zip diagnostic payload").unwrap();
        let archive = temp.path().join("payload.7z");
        native_compression::seven_zip::compress(
            &TestRuntime::default(),
            None,
            "diagnostic-7z",
            &[source.to_string_lossy().to_string()],
            archive.to_string_lossy().as_ref(),
            CompressionOptions::default(),
        )
        .unwrap();
        let healthy = diagnose_archive(&archive, None, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap();
        assert_eq!(healthy.actual_format, "7Z");
        assert_eq!(healthy.status, "healthy");

        let encrypted = temp.path().join("encrypted.7z");
        native_compression::seven_zip::compress(
            &TestRuntime::default(),
            None,
            "diagnostic-encrypted-7z",
            &[source.to_string_lossy().to_string()],
            encrypted.to_string_lossy().as_ref(),
            CompressionOptions {
                password: Some("correct-7z".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        let missing = diagnose_archive(&encrypted, None, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap();
        assert_eq!(missing.status, "password_required");
        let wrong = diagnose_archive(
            &encrypted,
            Some("wrong-7z"),
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .unwrap();
        assert_eq!(wrong.status, "ambiguous_data", "7Z decoder failure cannot prove a wrong password: {wrong:?}");
        assert!(!serde_json::to_string(&wrong).unwrap().contains("wrong-7z"));
    }

    #[test]
    fn zpaq_io_fault_never_becomes_an_encrypted_data_diagnosis() {
        for encrypted in [false, true] {
            for supplied in [false, true] {
                let (code, _, detail) = classify_failure("[zpaq-io] ZPAQ 文件读写失败：Windows error 30", encrypted, supplied);
                assert_eq!(code, "io_error");
                assert!(detail.contains("不应继续尝试密码"));
            }
        }
    }

    #[test]
    fn encrypted_seven_zip_decoder_failures_keep_the_ambiguous_password_class() {
        for message in [
            "Data Error",
            "Decompression failed",
            "ChecksumVerificationFailed",
            "corrupted input data",
            "Decoder returned an implementation-specific failure",
        ] {
            let (code, title, detail) = classify_failure(message, true, true);
            assert_eq!(code, "ambiguous_data");
            assert!(title.contains("密码"));
            assert!(detail.contains("无法区分密码错误与加密内容损坏"));
        }
        assert_eq!(classify_failure("Data Error", false, false).0, "damaged");
    }

    #[test]
    fn structural_and_capability_failures_do_not_turn_into_password_requests() {
        for (message, expected) in [
            ("[archive-inspection:incomplete] missing ZIP disks; wrong password?", "missing_volume"),
            ("Unexpected end of archive; wrong password?", "truncated"),
            ("[archive-inspection:damaged] broken ZIP structure; wrong password?", "damaged"),
            ("[archive-inspection:damaged] ALZ 卷号错误 C:/tmp/eof-sample/卷.a00", "damaged"),
            ("Unsupported compression method for encrypted entry", "unsupported_feature"),
            ("Permission denied reading encrypted archive", "permission_error"),
            ("[archive-inspection:ambiguous-data] invalid password or damaged content", "ambiguous_data"),
            ("7-Zip integrity test failed: CRC error", "ambiguous_data"),
        ] {
            assert_eq!(classify_failure(message, true, true).0, expected, "{message}");
        }
        assert_eq!(classify_failure("Implementation-specific verification failure", false, false).0, "unknown_failure");
        assert_eq!(classify_failure("InvalidPassword", true, true).0, "wrong_password");
    }

    #[test]
    fn split_gap_detection_preserves_real_volume_naming() {
        let temp = tempfile::tempdir().unwrap();
        let seven = temp.path().join("bundle.7z.001");
        std::fs::write(&seven, b"one").unwrap();
        std::fs::write(temp.path().join("bundle.7z.003"), b"three").unwrap();
        let (_, found, missing) = split_evidence(&seven).unwrap();
        assert_eq!(found, 2);
        assert_eq!(missing, vec!["bundle.7z.002"]);

        let rar = temp.path().join("legacy.rar");
        std::fs::write(&rar, b"first").unwrap();
        std::fs::write(temp.path().join("legacy.r00"), b"two").unwrap();
        std::fs::write(temp.path().join("legacy.r02"), b"four").unwrap();
        let (_, found, missing) = split_evidence(&rar).unwrap();
        assert_eq!(found, 3);
        assert_eq!(missing, vec!["legacy.r01"]);
    }

    #[tokio::test]
    async fn pre_cancelled_diagnosis_stops_without_reading_source() {
        let result = diagnose_archive(
            Path::new("missing.zip"),
            None,
            Arc::new(AtomicBool::new(true)),
        )
        .await;
        assert!(result.unwrap_err().to_string().contains("cancelled"));
    }

    #[test]
    fn pre_cancelled_repair_creates_no_output() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("source.zip");
        let output = temp.path().join("cancelled.zip");
        create_stored_zip(&archive);
        let result = repair_zip_to_new(&archive, &output, &AtomicBool::new(true));
        assert!(result.unwrap_err().to_string().contains("cancelled"));
        assert!(!output.exists());
    }

    #[tokio::test]
    async fn unsupported_password_route_never_claims_full_integrity() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("opaque.container");
        std::fs::write(&archive, b"opaque payload").unwrap();
        let report = diagnose_archive(
            &archive,
            Some("not-recorded"),
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .unwrap();
        assert_eq!(report.status, "structure_only");
        assert!(!report.integrity_tested);
        assert!(!serde_json::to_string(&report)
            .unwrap()
            .contains("not-recorded"));
    }
}
