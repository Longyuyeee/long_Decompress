use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::process::Command;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use crate::models::compression::TaskLogSeverity;
use super::archive_engine::ArchiveEngine;
use crate::services::compression_service::CompressionError;

pub struct UniversalCliEngine;

impl UniversalCliEngine {
    pub fn new() -> Self {
        Self
    }

    /// 检查系统中是否安装了 7z 或 7za
    fn get_7z_command() -> Option<&'static str> {
        if std::process::Command::new("7z").arg("--help").output().is_ok() {
            Some("7z")
        } else if std::process::Command::new("7za").arg("--help").output().is_ok() {
            Some("7za")
        } else {
            None
        }
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
        let cmd = match Self::get_7z_command() {
            Some(c) => c,
            None => return Ok(false),
        };

        // 7z t -p<password> <file> 测试归档
        let output = Command::new(cmd)
            .arg("t")
            .arg(format!("-p{}", password))
            .arg("-y") // 假定所有提示为yes
            .arg(file_path)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // 如果包含 "Everything is Ok"，则密码正确
        if stdout.contains("Everything is Ok") {
            return Ok(true);
        }

        // 如果报错包含密码相关，则说明密码错误
        if stderr.contains("Wrong password") || stderr.contains("Data Error in encrypted file") {
            return Ok(false);
        }

        // 无法识别的错误，或者不是密码问题（例如格式根本不支持）
        Ok(false)
    }

    async fn requires_password(&self, file_path: &Path) -> Result<bool> {
        let cmd = match Self::get_7z_command() {
            Some(c) => c,
            None => return Ok(false),
        };

        // 无密码尝试列出内容或测试
        let output = Command::new(cmd)
            .arg("t")
            .arg("-y")
            .arg(file_path)
            .output()
            .await?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // 7z CLI 会在遇到需要密码的归档时提示 Enter password (在终端) 
        // 并在带 -y 参数时报错 "Cannot open encrypted archive" 或 "Data Error in encrypted file"
        if stderr.contains("Cannot open encrypted archive") 
            || stdout.contains("Enter password")
            || stderr.contains("Data Error in encrypted file") {
            return Ok(true);
        }

        Ok(false)
    }

    async fn extract_with_progress(
        &self,
        file_path: &Path,
        output_dir: &Path,
        password: Option<&str>,
        on_progress: Arc<dyn Fn(f32) + Send + Sync>,
        on_log: Arc<dyn Fn(String, TaskLogSeverity) + Send + Sync>,
        is_cancelled: Arc<AtomicBool>,
    ) -> Result<()> {
        let cmd = Self::get_7z_command().ok_or_else(|| {
            anyhow::anyhow!("系统中未找到 7z 命令行工具，通用格式解压不可用。")
        })?;

        let mut command = Command::new(cmd);
        command.arg("x"); // extract with full paths
        command.arg("-y"); // yes to all
        
        if let Some(pwd) = password {
            command.arg(format!("-p{}", pwd));
        }

        command.arg(format!("-o{}", output_dir.to_string_lossy()));
        command.arg(file_path);
        
        // 开启进度输出
        command.arg("-bsp1");

        // 我们需要捕获 stdout 来解析进度
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command.spawn()?;

        let stdout = child.stdout.take().expect("未能获取 stdout");
        let stderr = child.stderr.take().expect("未能获取 stderr");

        let mut reader = BufReader::new(stdout).lines();
        let mut err_reader = BufReader::new(stderr).lines();

        let cancel_flag = is_cancelled.clone();
        
        // 解析标准输出流以提取进度
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
                            if text.starts_with("- ") {
                                on_log(text[2..].to_string(), TaskLogSeverity::Info);
                            }
                        },
                        Ok(None) => break, // EOF
                        Err(_) => break,
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
