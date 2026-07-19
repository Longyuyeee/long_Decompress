use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
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
                    if std::io::copy(&mut entry, &mut sink).is_err() {
                        return Ok(false);
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
        std::fs::create_dir_all(output_dir)?;

        for index in 0..archive.len() {
            if is_cancelled.load(Ordering::SeqCst) {
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
                    crate::services::compression_service::CompressionService::zip_system_time(
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
                std::io::copy(&mut entry, &mut output)?;
                drop(output);
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
            while start_idx > 0 && text.as_bytes()[start_idx - 1].is_ascii_digit() {
                start_idx -= 1;
            }
            if start_idx < idx {
                if let Ok(percent) = text[start_idx..idx].parse::<f32>() {
                    return Some(percent / 100.0);
                }
            }
        }
        None
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

    /// 尝试修复损坏的 ZIP 文件（通过 7z CLI）
    pub async fn repair_zip(file_path: &Path) -> Result<String> {
        let repaired = file_path.with_extension("repaired.zip");
        let args = vec![
            "r".to_string(),
            file_path.to_string_lossy().to_string(),
            "-o".to_string(),
            repaired.parent().unwrap_or(Path::new(".")).to_string_lossy().to_string(),
        ];

        let output = Self::run_7z_command(&args).await?;

        if output.status.success() && repaired.exists() {
            Ok(repaired.to_string_lossy().to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("ZIP repair failed: {}", stderr.trim()))
        }
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

        // 无密码尝试列出内容或测试
        let output = crate::utils::process::async_command(cmd)
            .arg("t")
            .arg("-y")
            .arg(file_path)
            .output()
            .await?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{}\n{}", stdout, stderr);

        // 7z CLI 会在遇到需要密码的归档时提示 Enter password (在终端) 
        // 并在带 -y 参数时报错 "Cannot open encrypted archive" 或 "Data Error in encrypted file"
        if combined.contains("Cannot open encrypted archive")
            || combined.contains("Can not open encrypted archive")
            || combined.contains("Enter password")
            || combined.contains("Data Error in encrypted file")
            || combined.contains("Wrong password") {
            return Ok(true);
        }

        Ok(false)
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

        // 7z 密码传递：使用 -p<password> 格式
        // 注意：密码会出现在命令行参数中，但这是 7z CLI 唯一支持的方式
        // 为了减少暴露时间，我们使用非交互模式，进程会快速结束
        command.arg(format!("-o{}", output_dir.to_string_lossy()));
        command.arg(file_path);

        // 开启进度输出
        command.arg("-bsp1");

        // 我们需要捕获 stdout 来解析进度
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command.spawn()?;

        let stdout = child.stdout.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture 7z stdout"))?;
        let stderr = child.stderr.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture 7z stderr"))?;

        let mut reader = BufReader::new(stdout).lines();
        let mut err_reader = BufReader::new(stderr).lines();

        let cancel_flag = is_cancelled.clone();

        let mut last_resource_check = std::time::Instant::now();
        // 解析标准输出流以提取进度。定时分支保证即使子进程没有
        // 输出新行，取消和资源配额仍会被及时检查。
        loop {
            if cancel_flag.load(Ordering::SeqCst) {
                let _ = child.kill().await;
                return Err(CompressionError::Cancelled.into());
            }

            tokio::select! {
                line = reader.next_line() => {
                    match line {
                        Ok(Some(text)) => {
                            if let Some(progress) = Self::parse_progress(&text) {
                                on_progress(progress);
                            }
                            // 同时记录提取的文件
                            if let Some(stripped) = text.strip_prefix("- ") {
                                on_log(stripped.to_string(), TaskLogSeverity::Info);
                            }
                        },
                        Ok(None) => break, // EOF
                        Err(_) => break,
                    }
                },
                _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                    if last_resource_check.elapsed() >= std::time::Duration::from_secs(1) {
                        if let Err(error) = crate::services::compression_service::CompressionService::validate_staged_resources(file_path, output_dir) {
                            let _ = child.kill().await;
                            return Err(error);
                        }
                        last_resource_check = std::time::Instant::now();
                    }
                }
            }
        }

        let status = child.wait().await?;
        if !status.success() {
            // 读取可能的错误信息
            let mut err_msg = String::new();
            while let Ok(Some(line)) = err_reader.next_line().await {
                err_msg.push_str(&line);
                err_msg.push('\n');
            }
            
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
        
        // 多个百分号（虽然不常见，但应取第一个）
        assert_eq!(UniversalCliEngine::parse_progress(" 10% ... 20%"), Some(0.1));
    }
}
