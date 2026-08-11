use crate::services::archive_browser;
use crate::services::archive_format::ArchiveFormat;
use crate::services::compression_format::{
    compression_route, infer_compression_format, CompressionRoute,
};
use crate::services::compression_verification::verify_native;
use crate::services::extraction_transaction;
use crate::services::native_extraction::seven_zip;
use crate::services::universal_engine::UniversalCliEngine;
use anyhow::{Context, Result};
use serde::Serialize;
use std::fs::{File, OpenOptions};
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
    pub recovered_directories: usize,
    pub skipped_entries: Vec<String>,
    pub verified: bool,
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

fn split_evidence(path: &Path) -> Result<(bool, usize, Vec<String>)> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut indices = Vec::new();
    let mut prefix = String::new();
    let mut suffix = String::new();
    let mut number_width = 3usize;

    if let Some(position) = name.rfind(".7z.") {
        prefix = name[..position + 4].to_string();
        suffix = String::new();
        number_width = name[position + 4..].len().max(1);
    } else if let Some(position) = name.rfind(".part") {
        if name.ends_with(".rar") {
            prefix = name[..position + 5].to_string();
            suffix = ".rar".to_string();
            number_width = name[position + 5..name.len() - 4].len().max(1);
        }
    } else if name.ends_with(".rar")
        || name.rsplit_once('.').is_some_and(|(_, ext)| {
            ext.starts_with('r') && ext[1..].chars().all(|c| c.is_ascii_digit())
        })
    {
        let stem = name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(&name);
        for entry in std::fs::read_dir(parent)? {
            let candidate = entry?.file_name().to_string_lossy().to_ascii_lowercase();
            if let Some(extension) = candidate.strip_prefix(&format!("{stem}.r")) {
                if let Ok(index) = extension.parse::<usize>() {
                    indices.push(index);
                }
            }
        }
        if indices.is_empty() {
            return Ok((false, 1, Vec::new()));
        }
        let terminal = parent.join(format!("{stem}.rar")).exists();
        let max = *indices.iter().max().unwrap_or(&0);
        let mut missing = (0..=max)
            .filter(|index| !indices.contains(index))
            .map(|index| format!("{stem}.r{index:02}"))
            .collect::<Vec<_>>();
        if !terminal {
            missing.push(format!("{stem}.rar"));
        }
        return Ok((true, indices.len() + usize::from(terminal), missing));
    } else if name.ends_with(".zip")
        || name.rsplit_once('.').is_some_and(|(_, ext)| {
            ext.starts_with('z') && ext[1..].chars().all(|c| c.is_ascii_digit())
        })
    {
        let stem = name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(&name);
        for entry in std::fs::read_dir(parent)? {
            let candidate = entry?.file_name().to_string_lossy().to_ascii_lowercase();
            if let Some(extension) = candidate.strip_prefix(&format!("{stem}.z")) {
                if let Ok(index) = extension.parse::<usize>() {
                    indices.push(index);
                }
            }
        }
        if indices.is_empty() {
            return Ok((false, 1, Vec::new()));
        }
        let terminal = parent.join(format!("{stem}.zip")).exists();
        let max = *indices.iter().max().unwrap_or(&0);
        let mut missing = (1..=max)
            .filter(|index| !indices.contains(index))
            .map(|index| format!("{stem}.z{index:02}"))
            .collect::<Vec<_>>();
        if !terminal {
            missing.push(format!("{stem}.zip"));
        }
        return Ok((true, indices.len() + usize::from(terminal), missing));
    }

    if prefix.is_empty() {
        return Ok((false, 1, Vec::new()));
    }
    for entry in std::fs::read_dir(parent)? {
        let candidate = entry?.file_name().to_string_lossy().to_ascii_lowercase();
        if candidate.starts_with(&prefix) && candidate.ends_with(&suffix) {
            let number = candidate[prefix.len()..candidate.len() - suffix.len()].parse::<usize>();
            if let Ok(number) = number {
                indices.push(number);
            }
        }
    }
    if indices.is_empty() {
        return Ok((true, 0, vec![name]));
    }
    let max = *indices.iter().max().unwrap();
    let missing = (1..=max)
        .filter(|index| !indices.contains(index))
        .map(|index| format!("{prefix}{index:0number_width$}{suffix}"))
        .collect();
    Ok((true, indices.len(), missing))
}

fn classify_failure(
    message: &str,
    encrypted: bool,
    password_supplied: bool,
) -> (&'static str, &'static str, &'static str) {
    let lower = message.to_ascii_lowercase();
    if lower.contains("password") || lower.contains("encrypted") {
        if encrypted && !password_supplied {
            (
                "password_required",
                "需要密码",
                "归档已加密，需要正确密码才能完成内容校验",
            )
        } else {
            ("wrong_password", "密码错误", "提供的密码无法解密归档内容")
        }
    } else if encrypted && password_supplied && lower.contains("corrupted input data") {
        (
            "wrong_password",
            "密码错误或密文损坏",
            "7Z 无法区分错误密码与加密数据损坏；请先确认密码，若密码正确则归档已损坏",
        )
    } else if lower.contains("crc") || lower.contains("checksum") || lower.contains("invalid data")
    {
        (
            "crc_error",
            "内容校验失败",
            "至少一个条目的 CRC 或解压数据校验失败",
        )
    } else if lower.contains("eof")
        || lower.contains("central directory")
        || lower.contains("eocd")
        || lower.contains("end of central")
        || lower.contains("truncated")
        || lower.contains("unexpected end")
    {
        ("truncated", "归档被截断", "归档尾部或中央目录不完整")
    } else {
        ("damaged", "归档损坏", "归档结构或内容无法通过完整性测试")
    }
}

pub async fn diagnose_archive(
    path: &Path,
    password: Option<&str>,
    cancelled: Arc<AtomicBool>,
) -> Result<ArchiveDiagnosticReport> {
    check_cancelled(&cancelled)?;
    if !path.is_file() {
        anyhow::bail!(
            "Archive does not exist or is not a file: {}",
            path.display()
        );
    }
    let file_size = path.metadata()?.len();
    let mut header = [0u8; 560];
    let read = File::open(path)?.read(&mut header)?;
    let actual_format = format_label(path, &header[..read]);
    let requested_format = match actual_format.as_str() {
        "ZIP" => "zip".to_string(),
        "7Z" => "7z".to_string(),
        "RAR" => "rar".to_string(),
        _ => infer_compression_format(path.to_string_lossy().as_ref(), None),
    };
    let (split_archive, volumes_found, missing_volumes) = split_evidence(path)?;
    let mut report = ArchiveDiagnosticReport {
        file_path: path.to_string_lossy().to_string(),
        file_size,
        actual_format: actual_format.clone(),
        status: "checking".to_string(),
        encrypted: false,
        split_archive,
        volumes_found,
        missing_volumes,
        total_files: 0,
        total_directories: 0,
        total_uncompressed_size: 0,
        integrity_tested: false,
        can_repair: false,
        recoverability: "diagnostic_only".to_string(),
        issues: Vec::new(),
        evidence: Vec::new(),
    };
    report
        .evidence
        .push(format!("Magic/extension format: {actual_format}"));
    report
        .evidence
        .push(format!("Archive size: {file_size} bytes"));
    if !report.missing_volumes.is_empty() {
        report.status = "missing_volume".to_string();
        report.recoverability = "unrecoverable".to_string();
        report.issues.push(DiagnosticIssue {
            code: "missing_volume".to_string(),
            severity: "error".to_string(),
            title: "缺少分卷".to_string(),
            detail: format!("缺少：{}", report.missing_volumes.join("、")),
        });
        return Ok(report);
    }

    let metadata = archive_browser::browse_archive(path, password).await;
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
        report.evidence.push(format!(
            "Entries: {} files, {} directories",
            metadata.total_files, metadata.total_directories
        ));
    }
    check_cancelled(&cancelled)?;

    if actual_format == "ZIP" {
        report.encrypted =
            UniversalCliEngine::zip_requires_password(path).unwrap_or(report.encrypted);
    } else if actual_format == "7Z" {
        report.encrypted = seven_zip::requires_password(path).unwrap_or(report.encrypted);
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

    if actual_format == "ZIP"
        && report.encrypted
        && !UniversalCliEngine::try_zip_password(path, password.unwrap_or_default())?
    {
        report.status = "wrong_password".to_string();
        report.recoverability = "password_required".to_string();
        report.issues.push(DiagnosticIssue {
            code: "wrong_password".to_string(),
            severity: "error".to_string(),
            title: "密码错误".to_string(),
            detail: "提供的密码无法读取加密 ZIP 内容".to_string(),
        });
        return Ok(report);
    }

    let route = compression_route(&requested_format);
    let verify_path = path.to_path_buf();
    let verify_password = password.map(str::to_string);
    let verify_cancelled = cancelled.clone();
    let verification = match route {
        Some(route)
            if !matches!(
                route,
                CompressionRoute::Rar | CompressionRoute::Wim | CompressionRoute::Lzma
            ) =>
        {
            report.integrity_tested = true;
            tauri::async_runtime::spawn_blocking(move || {
                verify_native(route, &verify_path, verify_password.as_deref(), || {
                    verify_cancelled.load(Ordering::SeqCst)
                })
            })
            .await
            .context("Archive verification worker failed")?
            .map(|_| ())
        }
        _ if password.is_none() => {
            report.integrity_tested = true;
            UniversalCliEngine::test_integrity(path, None).await
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
            let message = error.to_string();
            if message.to_ascii_lowercase().contains("cancel") {
                return Err(error);
            }
            let lower = message.to_ascii_lowercase();
            if lower.contains("7-zip") || lower.contains("archive engine") {
                report.status = "verification_unavailable".to_string();
                report.recoverability = "diagnostic_only".to_string();
                report.issues.push(DiagnosticIssue {
                    code: "verification_unavailable".to_string(),
                    severity: "warning".to_string(),
                    title: "无法完成内容校验".to_string(),
                    detail: "当前归档引擎不可用；已保留结构诊断结果，不能据此宣称归档完整"
                        .to_string(),
                });
                return Ok(report);
            }
            let (code, title, detail) =
                classify_failure(&message, report.encrypted, password.is_some());
            report.status = code.to_string();
            report.can_repair = actual_format == "ZIP" && !report.encrypted && code != "truncated";
            report.recoverability = if report.can_repair {
                "repairable"
            } else {
                "diagnostic_only"
            }
            .to_string();
            report.issues.push(DiagnosticIssue {
                code: code.to_string(),
                severity: "error".to_string(),
                title: title.to_string(),
                detail: detail.to_string(),
            });
            report
                .evidence
                .push(format!("Integrity failure class: {code}"));
        }
    }
    Ok(report)
}

pub fn repair_zip_to_new(
    path: &Path,
    output: &Path,
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
    if UniversalCliEngine::zip_requires_password(path)? {
        anyhow::bail!(
            "Encrypted ZIP repair is not supported; the original archive was not modified"
        );
    }

    let mut source = zip::ZipArchive::new(File::open(path)?)
        .context("ZIP central directory is unreadable; no repair output was created")?;
    let entry_count = source.len();
    let mut declared_expanded_bytes = 0u64;
    for index in 0..entry_count {
        let entry = source.by_index(index)?;
        if !entry.is_dir() {
            declared_expanded_bytes = declared_expanded_bytes.saturating_add(entry.size());
        }
    }
    extraction_transaction::validate_resource_limits(path, entry_count, declared_expanded_bytes)?;

    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    // During rebuilding, the temporary ZIP and one uncompressed spool can coexist.
    extraction_transaction::validate_disk_capacity(
        parent,
        declared_expanded_bytes.saturating_mul(2),
    )?;
    let temporary = parent.join(format!(".long-repair-{}.zip", uuid::Uuid::new_v4()));
    let mut guard = UnpublishedFile(Some(temporary.clone()));
    let temp_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let mut writer = zip::ZipWriter::new(temp_file);
    let mut recovered_files = 0usize;
    let mut recovered_directories = 0usize;
    let mut recovered_bytes = 0u64;
    let mut skipped_entries = Vec::new();

    for index in 0..source.len() {
        check_cancelled(cancelled)?;
        let mut entry = source.by_index(index)?;
        let name = entry.name().replace('\\', "/");
        let declared_entry_size = entry.size();
        if entry.enclosed_name().is_none() {
            skipped_entries.push(format!("{name}: unsafe path"));
            continue;
        }
        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        if entry.is_dir() {
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
                skipped_entries.push(format!("{name}: {error}"));
                continue;
            }
        };
        let next_recovered_bytes = recovered_bytes.saturating_add(copied);
        extraction_transaction::validate_resource_limits(
            path,
            recovered_files + recovered_directories + 1,
            next_recovered_bytes,
        )?;
        spool.rewind()?;
        writer.start_file(name, options)?;
        copy_cancellable(&mut spool, &mut writer, cancelled)?;
        recovered_bytes = next_recovered_bytes;
        recovered_files += 1;
        std::fs::remove_file(&spool_path)?;
        spool_guard.0.take();
    }
    if recovered_files == 0 {
        anyhow::bail!("No complete file entries could be recovered; no repair output was created");
    }
    writer.finish()?.sync_all()?;
    verify_native(CompressionRoute::Zip, &temporary, None, || {
        cancelled.load(Ordering::SeqCst)
    })
    .context("Rebuilt ZIP failed integrity verification")?;
    std::fs::rename(&temporary, output)?;
    guard.0.take();
    Ok(ZipRepairResult {
        output_path: output.to_string_lossy().to_string(),
        recovered_files,
        recovered_directories,
        skipped_entries,
        verified: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::compression::{CompressionOptions, TaskLogSeverity};
    use crate::services::native_compression::{self, CompressionRuntime};
    use sha2::{Digest, Sha256};
    use std::io::Write;
    use tauri::Window;

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
    async fn truncated_zip_is_diagnostic_only_and_repair_leaves_no_output() {
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
        assert_eq!(report.status, "truncated");
        assert!(!report.can_repair);
        assert!(repair_zip_to_new(&archive, &output, &AtomicBool::new(false)).is_err());
        assert!(!output.exists());
        assert_eq!(hash(&archive), before);
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
        assert_eq!(wrong.status, "wrong_password");
        assert!(!serde_json::to_string(&wrong).unwrap().contains("wrong-7z"));
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
