//! RAR repairs run on private copies. Only a fully readable result is published.
use super::archive_diagnostics::{ArchiveDiagnosticReport, DiagnosticIssue};
use super::compression_service::CompressionService;
use super::split_archive_detector::SplitArchiveDetector;
use anyhow::{ensure, Context, Result};
use regex::Regex;
use serde::Serialize;
use std::{
    fs,
    io::{Read, Write},
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

struct Group {
    files: Vec<PathBuf>,
    data: Vec<PathBuf>,
    recovery: Vec<PathBuf>,
}
fn collect(path: &Path) -> Result<Group> {
    let key = family(path.file_name().and_then(|n| n.to_str()).unwrap_or(""))
        .context("不是 RAR 或恢复卷")?;
    let part_style = numbered_parts(path.file_name().and_then(|n| n.to_str()).unwrap_or(""));
    let mut group = Group {
        files: vec![],
        data: vec![],
        recovery: vec![],
    };
    for entry in fs::read_dir(path.parent().unwrap_or(Path::new(".")))? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        if family(entry.file_name().to_str().unwrap_or("")).as_ref() != Some(&key) {
            continue;
        }
        if numbered_parts(entry.file_name().to_str().unwrap_or("")) != part_style {
            continue;
        }
        let file = entry.path();
        if file
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("rev"))
        {
            group.recovery.push(file.clone());
        } else {
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
    Ok(group)
}

#[derive(Debug, thiserror::Error)]
#[error("{detail}")]
struct VerificationFailure {
    detail: String,
    encrypted: bool,
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
    let error = |e: unrar::error::UnrarError| {
        use unrar::error::Code;
        let encrypted =
            observed_encrypted.get() || matches!(e.code, Code::MissingPassword | Code::BadPassword);
        let detail = match e.code {
            Code::MissingPassword => "需要填写正确的 RAR 密码".into(),
            Code::BadPassword => "RAR 密码错误，请修改后重新检查".into(),
            _ => format!("RAR 内容未通过完整校验：{e:?}"),
        };
        anyhow::Error::new(VerificationFailure { detail, encrypted })
    };
    let mut archive = archive.open_for_processing().map_err(error)?;
    observed_encrypted.set(archive.has_encrypted_headers());
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
        contents.encrypted |= header.entry().is_encrypted();
        observed_encrypted.set(contents.encrypted);
        if header.entry().is_directory() {
            contents.directories += 1;
            archive = header.skip().map_err(error)?;
        } else {
            contents.files += 1;
            contents.size = contents.size.saturating_add(header.entry().unpacked_size);
            archive = header.test().map_err(error)?;
        }
    }
    cancelled(flag)?;
    Ok(contents)
}

pub async fn diagnose(
    path: &Path,
    password: Option<&str>,
    flag: Arc<AtomicBool>,
) -> Result<ArchiveDiagnosticReport> {
    let path = fs::canonicalize(path)?;
    let password = password.map(str::to_owned);
    tauri::async_runtime::spawn_blocking(move || {
        let group = collect(&path)?;
        let tool = CompressionService::find_rar_encoder().is_some();
        let first = group.data.first();
        let split = first
            .map(|p| SplitArchiveDetector::detect_split_archive(p))
            .transpose()?
            .flatten();
        let missing: Vec<String> = split
            .as_ref()
            .map(|g| {
                g.missing_parts
                    .iter()
                    .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default();
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
                    report.recoverability = "not_needed".into();
                    return Ok(report);
                }
                Err(e) => {
                    cancelled(&flag)?;
                    report.encrypted = e
                        .downcast_ref::<VerificationFailure>()
                        .is_some_and(|e| e.encrypted);
                    let detail = e.to_string();
                    if detail.contains("密码") {
                        report.encrypted = true;
                        report.status = if password.as_deref().is_some_and(|p| !p.is_empty()) {
                            "wrong_password"
                        } else {
                            "password_required"
                        }
                        .into();
                    }
                    report.issues.push(DiagnosticIssue {
                        code: report.status.clone(),
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
        if !tool {
            report.issues.push(DiagnosticIssue {
                code: "rar_tool_required".into(),
                severity: "warning".into(),
                title: "修复需要本机 RAR 工具".into(),
                detail: "安装 WinRAR 后重新检查即可；普通 RAR 解压不需要安装。".into(),
            });
        } else if first.is_some()
            && (!group.recovery.is_empty() || report.missing_volumes.is_empty())
            && !report.status.contains("password")
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

#[derive(Serialize)]
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

fn password_input(password: &str) -> Result<Vec<u8>> {
    ensure!(
        !password.contains(['\r', '\n', '\0']),
        "RAR 修复暂不支持包含换行或空字符的密码"
    );
    #[cfg(windows)]
    let mut bytes = {
        use windows_sys::Win32::Globalization::{
            GetOEMCP, WideCharToMultiByte, WC_NO_BEST_FIT_CHARS,
        };
        let codepage = unsafe { GetOEMCP() };
        if codepage == 65001 {
            password.as_bytes().to_vec()
        } else {
            let wide: Vec<u16> = password.encode_utf16().collect();
            let length = i32::try_from(wide.len())?;
            let mut substituted = 0;
            let size = unsafe {
                WideCharToMultiByte(
                    codepage,
                    WC_NO_BEST_FIT_CHARS,
                    wide.as_ptr(),
                    length,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null(),
                    &mut substituted,
                )
            };
            ensure!(
                size > 0 && substituted == 0,
                "当前 RAR 控制台编码无法表示该密码，请在 WinRAR 修复后重新导入"
            );
            let mut result = vec![0u8; size as usize];
            let written = unsafe {
                WideCharToMultiByte(
                    codepage,
                    WC_NO_BEST_FIT_CHARS,
                    wide.as_ptr(),
                    length,
                    result.as_mut_ptr(),
                    size,
                    std::ptr::null(),
                    &mut substituted,
                )
            };
            ensure!(
                written == size && substituted == 0,
                "RAR 修复密码无法使用当前控制台编码传递"
            );
            result
        }
    };
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
    let output = crate::utils::process::cancellable_output_with_stdin(
        &mut command,
        flag,
        std::time::Duration::from_secs(3600),
        input,
    )
    .await?;
    cancelled(flag)?;
    if operation == "rc" && output.status.code() == Some(11) {
        anyhow::bail!("RAR 无法验证原包密码，请填写或修改密码后重新检查；未保存不完整卷组");
    }
    // RAR can finish ciphertext recovery, then return 11 when its own final test
    // cannot open encrypted headers. We verify the candidate natively with the
    // supplied password before publishing; no password enters process arguments.
    ensure!(matches!(output.status.code(), Some(0 | 1 | 11)), "RAR 修复未完成：恢复记录或恢复卷可能不足，或来源损坏超出恢复能力（退出码 {:?}）。原件保持不变。", output.status.code());
    Ok(())
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
    let group = collect(&source)?;
    ensure!(
        !group.data.is_empty(),
        "没有找到同组 RAR 数据卷，请先补齐仍保留的分卷"
    );
    let tool = CompressionService::find_rar_encoder().context("修复需要本机安装 WinRAR/Rar.exe")?;
    let stem =
        family(source.file_name().unwrap().to_str().unwrap_or("")).context("无效的 RAR 名称")?;
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
        )
        .await?;
    } else {
        // RAR repairs one damaged volume at a time and leaves healthy volumes
        // untouched. Assemble replacement and original copies under common names.
        for file in &group.data {
            run(&tool, "r", file, work.path(), password, &flag).await?;
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
mod tests {
    use super::*;
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
