use anyhow::Result;
use crate::models::compression::{ArchiveBrowseResult, ArchiveEntryInfo};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use crate::models::compression::TaskLogSeverity;
use super::archive_engine::ArchiveEngine;
use crate::services::compression_service::CompressionError;
use crate::utils::archive_tools::{find_7z_command, missing_7z_message};

pub struct UniversalCliEngine;

impl Default for UniversalCliEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl UniversalCliEngine {
    const COPY_BUFFER_SIZE: usize = 256 * 1024;
    const RESOURCE_SCAN_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);
    const ENCRYPTION_INSPECTION_TIMEOUT: std::time::Duration =
        std::time::Duration::from_secs(15);

    pub fn new() -> Self {
        Self
    }

    pub fn overwrite_mode_arg(overwrite_existing: bool) -> &'static str {
        if overwrite_existing { "-aoa" } else { "-aou" }
    }

    /// 共享辅助：运行 7z 命令并返回输出
    async fn run_7z_command(args: &[String]) -> Result<std::process::Output> {
        let cmd = Self::get_7z_command()
            .ok_or_else(|| anyhow::anyhow!(missing_7z_message()))?;
        crate::utils::process::async_command(&cmd)
            .args(args)
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("7z command failed: {}", e))
    }

    /// 共享辅助：仅运行不含密码的 7z 命令。
    async fn run_7z_command_with_password(args: &[String], password: Option<&str>) -> Result<std::process::Output> {
        if password.is_some() {
            return Err(anyhow::anyhow!(
                "Password-protected CLI operations are disabled because 7z has no reliable non-argv password channel"
            ));
        }
        let cmd = Self::get_7z_command()
            .ok_or_else(|| anyhow::anyhow!(missing_7z_message()))?;
        crate::utils::process::async_command(&cmd)
            .args(args)
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("7z command failed: {}", e))
    }

    pub(crate) fn zip_requires_password(file_path: &Path) -> Result<bool> {
        let file = std::fs::File::open(file_path)?;
        let mut archive = zip_aes::ZipArchive::new(file)?;

        for index in 0..archive.len() {
            if archive.by_index_raw(index)?.encrypted() {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub(crate) fn try_zip_password(file_path: &Path, password: &str) -> Result<bool> {
        let file = std::fs::File::open(file_path)?;
        let mut archive = zip_aes::ZipArchive::new(file)?;
        let mut buffer = vec![0u8; Self::COPY_BUFFER_SIZE];
        let mut found_encrypted_entry = false;

        for index in 0..archive.len() {
            let encrypted = archive.by_index_raw(index)?.encrypted();
            if !encrypted {
                continue;
            }
            found_encrypted_entry = true;
            match archive.by_index_decrypt(index, password.as_bytes()) {
                Ok(mut entry) => {
                    if entry.is_dir() {
                        continue;
                    }
                    let mut sink = std::io::sink();
                    loop {
                        let read = match entry.read(&mut buffer) {
                            Ok(read) => read,
                            Err(_) => return Ok(false),
                        };
                        if read == 0 {
                            break;
                        }
                        sink.write_all(&buffer[..read])?;
                    }
                }
                Err(zip_aes::result::ZipError::InvalidPassword) => return Ok(false),
                Err(error) => return Err(error.into()),
            }
        }

        Ok(found_encrypted_entry)
    }

    fn available_output_path(target: &Path, overwrite_existing: bool) -> Result<std::path::PathBuf> {
        if overwrite_existing || !target.exists() {
            return Ok(target.to_path_buf());
        }

        let parent = target.parent().unwrap_or_else(|| Path::new(""));
        let stem = target.file_stem().and_then(|name| name.to_str()).unwrap_or("file");
        let extension = target.extension().and_then(|name| name.to_str());
        for index in 1..10_000 {
            let name = match extension {
                Some(extension) if !extension.is_empty() => {
                    format!("{} ({}).{}", stem, index, extension)
                }
                _ => format!("{} ({})", stem, index),
            };
            let candidate = parent.join(name);
            if !candidate.exists() {
                return Ok(candidate);
            }
        }

        Err(anyhow::anyhow!("Unable to find an available output name for {}", target.display()))
    }

    fn extract_zip_with_password(
        file_path: &Path,
        output_dir: &Path,
        password: &str,
        overwrite_existing: bool,
        on_progress: &Arc<dyn Fn(f32) + Send + Sync>,
        on_log: &Arc<dyn Fn(String, TaskLogSeverity) + Send + Sync>,
        is_cancelled: &Arc<AtomicBool>,
    ) -> Result<()> {
        let file = std::fs::File::open(file_path)?;
        let mut archive = zip_aes::ZipArchive::new(file)?;
        let total = archive.len().max(1);
        let mut buffer = vec![0u8; Self::COPY_BUFFER_SIZE];
        std::fs::create_dir_all(output_dir)?;

        for index in 0..archive.len() {
            if is_cancelled.load(Ordering::Relaxed) {
                return Err(CompressionError::Cancelled.into());
            }

            let mut entry = match archive.by_index_decrypt(index, password.as_bytes()) {
                Ok(entry) => entry,
                Err(zip_aes::result::ZipError::InvalidPassword) => {
                    return Err(CompressionError::InvalidPassword.into());
                }
                Err(error) => return Err(error.into()),
            };
            let Some(target) = crate::utils::file_utils::verify_extract_path(
                Path::new(entry.name()),
                output_dir,
                true,
            ) else {
                on_log(
                    format!("Skipped unsafe ZIP entry: {}", entry.name()),
                    TaskLogSeverity::Warning,
                );
                continue;
            };

            if entry.is_dir() {
                std::fs::create_dir_all(&target)?;
            } else {
                let modified = entry.last_modified().and_then(|value| {
                    crate::services::native_extraction::zip::system_time(
                        value.year(),
                        value.month(),
                        value.day(),
                        value.hour(),
                        value.minute(),
                        value.second(),
                    )
                });
                let target = Self::available_output_path(&target, overwrite_existing)?;
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut output = std::fs::File::create(&target)?;
                loop {
                    if is_cancelled.load(Ordering::Relaxed) {
                        return Err(CompressionError::Cancelled.into());
                    }
                    let read = entry.read(&mut buffer)?;
                    if read == 0 {
                        break;
                    }
                    output.write_all(&buffer[..read])?;
                }
                output.flush()?;
                if let Some(modified) = modified {
                    filetime::set_file_mtime(
                        &target,
                        filetime::FileTime::from_system_time(modified),
                    )?;
                }
                on_log(target.to_string_lossy().to_string(), TaskLogSeverity::Info);
            }
            on_progress((index + 1) as f32 / total as f32);
        }

        Ok(())
    }

    /// 检查系统中是否安装了 7z 或 7za
    fn get_7z_command() -> Option<String> {
        find_7z_command()
    }

    /// 解析 7z -bsp1 的进度行
    fn parse_progress(text: &str) -> Option<f32> {
        if let Some(idx) = text.find('%') {
            let mut start_idx = idx;
            while start_idx > 0
                && (text.as_bytes()[start_idx - 1].is_ascii_digit()
                    || text.as_bytes()[start_idx - 1] == b'.')
            {
                start_idx -= 1;
            }
            if start_idx < idx {
                if let Ok(percent) = text[start_idx..idx].parse::<f32>() {
                    return Some((percent / 100.0).clamp(0.0, 1.0));
                }
            }
        }
        None
    }

    fn current_file_from_progress(text: &str) -> Option<String> {
        let trimmed = text.trim();
        let candidate = text
            .find('%')
            .and_then(|percent| text.get(percent + 1..))
            .map(str::trim)
            .and_then(|tail| tail.strip_prefix("- "))
            .or_else(|| trimmed.strip_prefix("- "))
            .or_else(|| trimmed.strip_prefix("Extracting  "))
            .or_else(|| trimmed.strip_prefix("Extracting "))
            .map(str::trim)
            .filter(|path| !path.is_empty())?;
        Some(candidate.to_string())
    }

    fn publish_progress_record(
        text: &str,
        on_progress: &Arc<dyn Fn(f32) + Send + Sync>,
        on_log: &Arc<dyn Fn(String, TaskLogSeverity) + Send + Sync>,
        last_file: &mut Option<String>,
    ) {
        if let Some(progress) = Self::parse_progress(text) {
            on_progress(progress);
        }
        if let Some(current_file) = Self::current_file_from_progress(text) {
            if last_file.as_deref() != Some(current_file.as_str()) {
                on_log(
                    format!("正在解压：{}", current_file),
                    TaskLogSeverity::Info,
                );
                *last_file = Some(current_file);
            }
        }
    }

    fn encryption_state_from_listing(
        output: &std::process::Output,
    ) -> Result<bool> {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Self::encryption_state_from_listing_text(output.status.success(), &stdout, &stderr)
    }

    fn encryption_state_from_listing_text(
        succeeded: bool,
        stdout: &str,
        stderr: &str,
    ) -> Result<bool> {
        let combined = format!("{}\n{}", stdout, stderr);
        let stderr_lower = stderr.to_ascii_lowercase();

        if stdout.lines().any(|line| {
            line.split_once(" = ")
                .is_some_and(|(key, value)| key.trim() == "Encrypted" && value.trim() == "+")
        }) {
            return Ok(true);
        }

        if [
            "cannot open encrypted archive",
            "can not open encrypted archive",
            "enter password",
            "data error in encrypted file",
            "wrong password",
        ]
        .iter()
        .any(|marker| stderr_lower.contains(marker))
        {
            return Ok(true);
        }

        if !succeeded {
            return Err(anyhow::anyhow!(
                "Unable to inspect archive encryption metadata: {}",
                combined.trim()
            ));
        }

        Ok(false)
    }

    /// 列出归档文件中的内容条目（通过 7z CLI 的 l 命令）
    pub async fn list_contents(file_path: &Path, password: Option<&str>) -> Result<Vec<String>> {
        let mut args = vec!["l".to_string(), "-slt".to_string(), "-ba".to_string()];
        if password.is_some() {
            args.push("-p".to_string()); // 使用环境变量传递密码
        }
        args.push(file_path.to_string_lossy().to_string());

        let output = Self::run_7z_command_with_password(&args, password).await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let entries: Vec<String> = stdout.lines()
            .filter_map(|line| {
                line.strip_prefix("Path = ")
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty())
            })
            .collect();

        if !entries.is_empty() {
            return Ok(entries);
        }

        // 回退：解析普通列表模式的最后列
        Ok(stdout.lines()
            .filter(|line| {
                let t = line.trim();
                !t.starts_with("---") && !t.starts_with("Date") && !t.is_empty() && !t.contains("files, ")
            })
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                (parts.len() >= 6).then(|| {
                    let name = parts.last().unwrap();
                    name.to_string()
                })
            })
            .filter(|name| !name.starts_with("---") && name != "Name")
            .collect())
    }

    fn parse_metadata_listing(stdout: &str, format: String) -> ArchiveBrowseResult {
        let mut entries = Vec::new();
        let mut current = std::collections::HashMap::<String, String>::new();
        let flush = |current: &mut std::collections::HashMap<String, String>, entries: &mut Vec<ArchiveEntryInfo>| {
            let Some(path) = current.remove("Path") else {
                current.clear();
                return;
            };
            let path = path.replace('\\', "/");
            let attributes = current.get("Attributes").map(String::as_str).unwrap_or("");
            let is_dir = attributes.contains('D') || path.ends_with('/');
            let name = path.trim_end_matches('/').rsplit('/').next().unwrap_or(&path).to_string();
            entries.push(ArchiveEntryInfo {
                name,
                path,
                size: current.get("Size").and_then(|value| value.parse().ok()).unwrap_or(0),
                compressed_size: current.get("Packed Size").and_then(|value| value.parse().ok()),
                modified: current.get("Modified").cloned(),
                crc: current.get("CRC").cloned().filter(|value| !value.is_empty()),
                encrypted: current.get("Encrypted").is_some_and(|value| value == "+"),
                is_dir,
            });
            current.clear();
        };

        for line in stdout.lines() {
            let line = line.trim_end();
            if line.is_empty() {
                flush(&mut current, &mut entries);
            } else if let Some((key, value)) = line.split_once(" = ") {
                current.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
        flush(&mut current, &mut entries);

        ArchiveBrowseResult {
            format,
            total_files: entries.iter().filter(|entry| !entry.is_dir).count(),
            total_directories: entries.iter().filter(|entry| entry.is_dir).count(),
            total_uncompressed_size: entries.iter().filter(|entry| !entry.is_dir).map(|entry| entry.size).sum(),
            total_compressed_size: entries.iter().filter_map(|entry| entry.compressed_size).sum(),
            encrypted: entries.iter().any(|entry| entry.encrypted),
            entries,
        }
    }

    /// Reads structured metadata without placing passwords in process arguments.
    pub async fn list_metadata(file_path: &Path, format: String) -> Result<ArchiveBrowseResult> {
        let args = vec![
            "l".to_string(), "-slt".to_string(), "-ba".to_string(), "-p-".to_string(),
            file_path.to_string_lossy().to_string(),
        ];
        let output = Self::run_7z_command(&args).await?;
        if !output.status.success() {
            anyhow::bail!("Unable to read archive metadata safely");
        }
        let result = Self::parse_metadata_listing(&String::from_utf8_lossy(&output.stdout), format);
        if result.entries.is_empty() {
            anyhow::bail!("The archive did not expose any browseable entries");
        }
        Ok(result)
    }

    /// Read declared entry count and expanded size without extracting data.
    /// Passwords are deliberately not accepted here because 7z can only receive
    /// them through process arguments.
    pub async fn archive_uncompressed_stats(file_path: &Path) -> Result<(usize, u64)> {
        let args = vec![
            "l".to_string(),
            "-slt".to_string(),
            "-ba".to_string(),
            "-p-".to_string(),
            file_path.to_string_lossy().to_string(),
        ];
        let output = Self::run_7z_command(&args).await?;
        if !output.status.success() {
            return Err(anyhow::anyhow!("Unable to read archive metadata safely"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut entries = 0usize;
        let mut total_size = 0u64;
        for line in stdout.lines() {
            if line.starts_with("Path = ") {
                entries = entries.saturating_add(1);
            } else if let Some(size) = line.strip_prefix("Size = ") {
                let value = size.trim().parse::<u64>().map_err(|_| {
                    anyhow::anyhow!("Archive contains an invalid expanded-size field")
                })?;
                total_size = total_size.checked_add(value).ok_or_else(|| {
                    anyhow::anyhow!("Archive expanded size overflowed the supported range")
                })?;
            }
        }
        Ok((entries, total_size))
    }

    /// 检测归档文件的完整性（通过 7z CLI 的 t 命令）
    pub async fn test_integrity(file_path: &Path, password: Option<&str>) -> Result<()> {
        let mut args = vec!["t".to_string()];
        if password.is_some() {
            args.push("-p".to_string()); // 使用环境变量传递密码
        }
        args.push(file_path.to_string_lossy().to_string());

        let output = Self::run_7z_command_with_password(&args, password).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Archive integrity test failed: {}", stderr.trim()));
        }
        Ok(())
    }

}

#[async_trait::async_trait]
impl ArchiveEngine for UniversalCliEngine {
    fn name(&self) -> &'static str {
        "Universal_7z_CLI"
    }

    async fn can_handle(&self, _header: &[u8], _ext: &str) -> bool {
        // 作为托底引擎，只要系统安装了 7z，就声称可以尝试处理一切未知格式
        Self::get_7z_command().is_some()
    }

    fn supports_password(&self) -> bool {
        true
    }

    async fn try_password(&self, file_path: &Path, password: &str) -> Result<bool> {
        if file_path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
        {
            return Self::try_zip_password(file_path, password);
        }

        Err(anyhow::anyhow!(
            "Password validation for this format requires a native archive engine"
        ))
    }

    async fn requires_password(&self, file_path: &Path) -> Result<bool> {
        let cmd = match Self::get_7z_command() {
            Some(c) => c,
            None => return Ok(false),
        };

        // Encryption detection only needs the archive directory metadata. Using
        // `7z t` here performed a full read of every split volume before any
        // extraction progress was visible, which made large archives appear to
        // hang and duplicated all I/O immediately before extraction.
        let mut command = crate::utils::process::async_command(cmd);
        command
            .arg("l")
            .arg("-slt")
            .arg("-ba")
            .arg("-bd")
            .arg("-p-")
            .arg("--")
            .arg(file_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let child = command.spawn()?;
        let output = match tokio::time::timeout(
            Self::ENCRYPTION_INSPECTION_TIMEOUT,
            child.wait_with_output(),
        )
        .await
        {
            Ok(result) => result?,
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "Archive encryption metadata inspection timed out after {} seconds",
                    Self::ENCRYPTION_INSPECTION_TIMEOUT.as_secs()
                ));
            }
        };

        Self::encryption_state_from_listing(&output)
    }

    async fn extract_with_progress(
        &self,
        file_path: &Path,
        output_dir: &Path,
        password: Option<&str>,
        overwrite_existing: bool,
        on_progress: Arc<dyn Fn(f32) + Send + Sync>,
        on_log: Arc<dyn Fn(String, TaskLogSeverity) + Send + Sync>,
        is_cancelled: Arc<AtomicBool>,
    ) -> Result<()> {
        if let Some(password) = password {
            if file_path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
            {
                return Self::extract_zip_with_password(
                    file_path,
                    output_dir,
                    password,
                    overwrite_existing,
                    &on_progress,
                    &on_log,
                    &is_cancelled,
                );
            }
            return Err(CompressionError::UnsupportedEncryption.into());
        }
        let cmd = Self::get_7z_command().ok_or_else(|| {
            anyhow::anyhow!(missing_7z_message())
        })?;

        let mut command = crate::utils::process::async_command(cmd);
        command.arg("x"); // extract with full paths
        command.arg("-y"); // yes to all
        command.arg(Self::overwrite_mode_arg(overwrite_existing));
        command.arg("-bb1"); // report each processed file
        command.arg("-bsp1"); // progress stream -> stdout
        command.arg("-bso1"); // normal output -> stdout
        command.arg("-bse2"); // errors -> stderr

        // 7z 密码传递：使用 -p<password> 格式
        // 注意：密码会出现在命令行参数中，但这是 7z CLI 唯一支持的方式
        // 为了减少暴露时间，我们使用非交互模式，进程会快速结束
        command.arg(format!("-o{}", output_dir.to_string_lossy()));
        command.arg("--");
        command.arg(file_path);

        // Never let an unexpected archive prompt wait invisibly for terminal
        // input. Password state is resolved before this process starts.
        command.stdin(Stdio::null());

        // 我们需要捕获 stdout 来解析进度
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command.spawn()?;

        let stdout = child.stdout.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture 7z stdout"))?;
        let stderr = child.stderr.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture 7z stderr"))?;

        let mut stdout = stdout;
        let stderr_task = tokio::spawn(async move {
            let mut stderr = stderr;
            let mut bytes = Vec::new();
            stderr.read_to_end(&mut bytes).await?;
            Ok::<Vec<u8>, std::io::Error>(bytes)
        });

        let cancel_flag = is_cancelled.clone();

        let mut last_resource_check = std::time::Instant::now();
        let mut last_disk_check = std::time::Instant::now();
        let mut chunk = [0u8; 16 * 1024];
        let mut pending_record = Vec::new();
        let mut last_file = None;
        // 7z 的进度使用回车符刷新同一行，而文件日志使用换行符。
        // 同时解析 \r 与 \n，避免大文件直到进程结束才刷新界面。
        loop {
            if cancel_flag.load(Ordering::SeqCst) {
                let _ = child.kill().await;
                return Err(CompressionError::Cancelled.into());
            }

            tokio::select! {
                read = stdout.read(&mut chunk) => {
                    match read {
                        Ok(0) => {
                            if !pending_record.is_empty() {
                                let text = String::from_utf8_lossy(&pending_record);
                                Self::publish_progress_record(&text, &on_progress, &on_log, &mut last_file);
                            }
                            break;
                        },
                        Ok(count) => {
                            for byte in &chunk[..count] {
                                if *byte == b'\r' || *byte == b'\n' {
                                    if !pending_record.is_empty() {
                                        let text = String::from_utf8_lossy(&pending_record);
                                        Self::publish_progress_record(&text, &on_progress, &on_log, &mut last_file);
                                        pending_record.clear();
                                    }
                                } else {
                                    pending_record.push(*byte);
                                }
                            }
                        },
                        Err(error) => {
                            let _ = child.kill().await;
                            return Err(anyhow::anyhow!("Failed to read 7z progress stream: {}", error));
                        },
                    }
                },
                _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                    if last_disk_check.elapsed() >= std::time::Duration::from_secs(1) {
                        if let Err(error) = crate::services::extraction_transaction::validate_staging_disk_reserve(output_dir) {
                            let _ = child.kill().await;
                            return Err(error);
                        }
                        last_disk_check = std::time::Instant::now();
                    }
                    if last_resource_check.elapsed() >= Self::RESOURCE_SCAN_INTERVAL {
                        if let Err(error) = crate::services::extraction_transaction::validate_staged_resources(file_path, output_dir) {
                            let _ = child.kill().await;
                            return Err(error);
                        }
                        last_resource_check = std::time::Instant::now();
                    }
                }
            }
        }

        let status = child.wait().await?;
        let err_bytes = stderr_task
            .await
            .map_err(|error| anyhow::anyhow!("Failed to join 7z error stream: {}", error))??;
        let err_msg = String::from_utf8_lossy(&err_bytes).to_string();
        if !status.success() {
            if err_msg.contains("Wrong password") {
                return Err(CompressionError::InvalidPassword.into());
            }
            if err_msg.contains("Cannot open encrypted archive") {
                return Err(CompressionError::PasswordRequired.into());
            }

            return Err(CompressionError::ExtractionFailed(format!("通用引擎调用失败: {}", err_msg)).into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_7z_progress_parsing() {
        // 正常百分比
        assert_eq!(UniversalCliEngine::parse_progress("  0%"), Some(0.0));
        assert_eq!(UniversalCliEngine::parse_progress(" 12% - path/to/file"), Some(0.12));
        assert_eq!(UniversalCliEngine::parse_progress("100%"), Some(1.0));
        
        // 边界情况
        assert_eq!(UniversalCliEngine::parse_progress("no percent"), None);
        assert_eq!(UniversalCliEngine::parse_progress("%"), None);
        assert_eq!(UniversalCliEngine::parse_progress("abc 50% def"), Some(0.5));
        assert_eq!(UniversalCliEngine::parse_progress(" 12.34% - path/to/file"), Some(0.1234));
        
        // 多个百分号（虽然不常见，但应取第一个）
        assert_eq!(UniversalCliEngine::parse_progress(" 10% ... 20%"), Some(0.1));
    }

    #[test]
    fn parses_current_file_from_carriage_return_progress_records() {
        assert_eq!(
            UniversalCliEngine::current_file_from_progress(" 12% - folder/file.txt"),
            Some("folder/file.txt".to_string())
        );
        assert_eq!(
            UniversalCliEngine::current_file_from_progress("Extracting  folder/second.txt"),
            Some("folder/second.txt".to_string())
        );
    }

    #[test]
    fn plain_listing_text_cannot_become_a_password_false_positive() {
        let plain_listing = "Path = enter password notes.txt\nEncrypted = -\nPath = normal.txt\nEncrypted = -";
        assert!(!UniversalCliEngine::encryption_state_from_listing_text(
            true,
            plain_listing,
            "",
        )
        .expect("plain listing"));
        assert!(UniversalCliEngine::encryption_state_from_listing_text(
            true,
            "Path = secret.txt\nEncrypted = +",
            "",
        )
        .expect("encrypted metadata"));
        assert!(UniversalCliEngine::encryption_state_from_listing_text(
            false,
            "",
            "ERROR: Wrong password",
        )
        .expect("password error"));
    }

    #[tokio::test]
    #[ignore = "requires LONG_REAL_SPLIT_ARCHIVE and performs a cancellable real extraction"]
    async fn real_split_archive_streams_progress_and_current_files() {
        let archive = std::env::var("LONG_REAL_SPLIT_ARCHIVE")
            .expect("set LONG_REAL_SPLIT_ARCHIVE to the first split volume");
        let archive = std::path::PathBuf::from(archive);
        assert!(archive.is_file(), "real split fixture must exist");
        let engine = UniversalCliEngine::new();
        assert!(
            !engine
                .requires_password(&archive)
                .await
                .expect("inspect real split encryption metadata"),
            "the real plain split archive must not enter password discovery"
        );
        let output = tempfile::tempdir().expect("temporary extraction output");
        let progress = Arc::new(std::sync::Mutex::new(Vec::new()));
        let logs = Arc::new(std::sync::Mutex::new(Vec::new()));
        let cancellation = Arc::new(AtomicBool::new(false));

        let progress_capture = progress.clone();
        let log_capture = logs.clone();
        let cancel_after_observation = cancellation.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(25)).await;
            cancel_after_observation.store(true, Ordering::SeqCst);
        });

        let result = engine
            .extract_with_progress(
                &archive,
                output.path(),
                None,
                true,
                Arc::new(move |value| progress_capture.lock().unwrap().push(value)),
                Arc::new(move |message, _| log_capture.lock().unwrap().push(message)),
                cancellation,
            )
            .await;

        assert!(result.is_err(), "observation run should stop through cancellation");
        assert!(
            progress.lock().unwrap().iter().any(|value| *value > 0.0),
            "real extraction must publish progress before cancellation"
        );
        assert!(
            logs.lock()
                .unwrap()
                .iter()
                .any(|message| message.starts_with("正在解压：")),
            "real extraction must publish the current file"
        );
    }
}
