use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use log;

/// RAR解压错误
#[derive(Debug, thiserror::Error)]
pub enum RarError {
    #[error("RAR文件不存在: {0}")]
    FileNotFound(String),

    #[error("RAR文件无效或损坏: {0}")]
    InvalidRarFile(String),

    #[error("RAR解压失败: {0}")]
    ExtractionFailed(String),

    #[error("密码错误或缺失")]
    PasswordError,

    #[error("RAR文件已加密，需要密码")]
    EncryptionRequired,

    #[error("不支持的RAR版本: {0}")]
    UnsupportedVersion(String),

    #[error("RAR工具未安装")]
    ToolNotInstalled,

    #[error("系统命令执行失败: {0}")]
    CommandFailed(String),

    #[error("磁盘空间不足")]
    DiskSpaceFull,

    #[error("权限不足: {0}")]
    PermissionDenied(String),

    #[error("操作超时")]
    OperationTimeout,

    #[error("文件损坏: {0}")]
    FileCorrupted(String),

    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),
}

/// RAR支持服务
pub struct RarSupportService;

impl RarSupportService {
    /// 创建新的RAR支持服务
    pub fn new() -> Self {
        Self
    }

    /// 检查系统是否安装了RAR解压工具
    pub fn check_rar_tool_installed() -> bool {
        // 检查常见的RAR解压工具
        let tools = ["unrar", "rar", "7z"];

        for tool in tools.iter() {
            if Self::check_tool_exists(tool) {
                log::debug!("找到RAR解压工具: {}", tool);
                return true;
            }
        }

        log::warn!("未找到RAR解压工具");
        false
    }

    /// 检查特定工具是否存在
    fn check_tool_exists(tool: &str) -> bool {
        let output = if cfg!(target_os = "windows") {
            Command::new("where")
                .arg(tool)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
        } else {
            Command::new("which")
                .arg(tool)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
        };

        match output {
            Ok(status) => status.success(),
            Err(_) => false,
        }
    }

    /// 解压RAR文件
    pub async fn extract_rar(
        &self,
        rar_path: &Path,
        output_dir: &Path,
        password: Option<&str>,
    ) -> Result<(), RarError> {
        log::info!("开始解压RAR文件: {:?} -> {:?}", rar_path, output_dir);

        // 验证文件存在
        if !rar_path.exists() {
            return Err(RarError::FileNotFound(rar_path.to_string_lossy().to_string()));
        }

        // 验证是否是有效的RAR文件
        if !self.is_valid_rar_file(rar_path).await {
            return Err(RarError::InvalidRarFile("不是有效的RAR文件".to_string()));
        }

        // 检查RAR工具
        if !Self::check_rar_tool_installed() {
            return Err(RarError::ToolNotInstalled);
        }

        // 创建输出目录
        tokio::fs::create_dir_all(output_dir).await
            .map_err(|e| RarError::ExtractionFailed(format!("创建输出目录失败: {}", e)))?;

        // 尝试使用不同的工具解压
        let result = self.try_extract_with_unrar(rar_path, output_dir, password).await
            .or_else(|_| self.try_extract_with_7z(rar_path, output_dir, password).await);

        match result {
            Ok(_) => {
                log::info!("RAR解压成功: {:?}", rar_path);
                Ok(())
            }
            Err(e) => {
                log::error!("RAR解压失败: {:?}", e);
                Err(e)
            }
        }
    }

    /// 尝试使用unrar解压
    async fn try_extract_with_unrar(
        &self,
        rar_path: &Path,
        output_dir: &Path,
        password: Option<&str>,
    ) -> Result<(), RarError> {
        log::debug!("尝试使用unrar解压");

        let mut command = Command::new("unrar");

        command.arg("x"); // 解压并保留目录结构

        if let Some(pwd) = password {
            command.arg("-p").arg(pwd);
        } else {
            command.arg("-p-"); // 无密码
        }

        command.arg("-y"); // 全部回答Yes
        command.arg(rar_path);
        command.arg(output_dir);

        log::debug!("执行命令: {:?}", command);

        let output = command.output()
            .map_err(|e| RarError::CommandFailed(format!("执行unrar命令失败: {}", e)))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);

            if stderr.contains("password") || stderr.contains("Password") {
                Err(RarError::PasswordError)
            } else {
                Err(RarError::ExtractionFailed(format!("unrar解压失败: {}", stderr)))
            }
        }
    }

    /// 尝试使用7z解压
    async fn try_extract_with_7z(
        &self,
        rar_path: &Path,
        output_dir: &Path,
        password: Option<&str>,
    ) -> Result<(), RarError> {
        log::debug!("尝试使用7z解压");

        let mut command = Command::new("7z");

        command.arg("x"); // 解压

        if let Some(pwd) = password {
            command.arg("-p").arg(pwd);
        }

        command.arg("-y"); // 全部回答Yes
        command.arg("-o").arg(output_dir);
        command.arg(rar_path);

        log::debug!("执行命令: {:?}", command);

        let output = command.output()
            .map_err(|e| RarError::CommandFailed(format!("执行7z命令失败: {}", e)))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);

            if stderr.contains("password") || stderr.contains("Password") ||
               stderr.contains("Wrong password") {
                Err(RarError::PasswordError)
            } else {
                Err(RarError::ExtractionFailed(format!("7z解压失败: {}", stderr)))
            }
        }
    }

    /// 列出RAR文件内容
    pub async fn list_rar_contents(
        &self,
        rar_path: &Path,
        password: Option<&str>,
    ) -> Result<Vec<String>, RarError> {
        log::debug!("列出RAR文件内容: {:?}", rar_path);

        if !Self::check_rar_tool_installed() {
            return Err(RarError::ToolNotInstalled);
        }

        let mut command = Command::new("unrar");

        command.arg("l"); // 列出内容

        if let Some(pwd) = password {
            command.arg("-p").arg(pwd);
        }

        command.arg(rar_path);

        let output = command.output()
            .map_err(|e| RarError::CommandFailed(format!("执行unrar列表命令失败: {}", e)))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let files: Vec<String> = stdout.lines()
                .filter(|line| !line.is_empty())
                .map(|s| s.to_string())
                .collect();

            Ok(files)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);

            if stderr.contains("password") || stderr.contains("Password") {
                Err(RarError::PasswordError)
            } else {
                Err(RarError::ExtractionFailed(format!("列出RAR内容失败: {}", stderr)))
            }
        }
    }

    /// 测试RAR文件完整性
    pub async fn test_rar_integrity(
        &self,
        rar_path: &Path,
        password: Option<&str>,
    ) -> Result<bool, RarError> {
        log::debug!("测试RAR文件完整性: {:?}", rar_path);

        if !Self::check_rar_tool_installed() {
            return Err(RarError::ToolNotInstalled);
        }

        let mut command = Command::new("unrar");

        command.arg("t"); // 测试

        if let Some(pwd) = password {
            command.arg("-p").arg(pwd);
        }

        command.arg(rar_path);

        let output = command.output()
            .map_err(|e| RarError::CommandFailed(format!("执行unrar测试命令失败: {}", e)))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let is_ok = stdout.contains("All OK") || stdout.contains("All ok");

            Ok(is_ok)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);

            if stderr.contains("password") || stderr.contains("Password") {
                Err(RarError::PasswordError)
            } else {
                Err(RarError::ExtractionFailed(format!("测试RAR完整性失败: {}", stderr)))
            }
        }
    }

    /// 检测文件是否是有效的RAR文件
    pub async fn is_valid_rar_file(&self, file_path: &Path) -> bool {
        log::debug!("检测文件是否是有效的RAR文件: {:?}", file_path);

        // 检查文件扩展名
        let extension = file_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        if extension != "rar" && extension != "cbr" {
            log::debug!("文件扩展名不是RAR: {}", extension);
            return false;
        }

        // 检查文件是否存在且有内容
        match tokio::fs::metadata(file_path).await {
            Ok(metadata) => {
                if metadata.len() == 0 {
                    log::debug!("RAR文件大小为0");
                    return false;
                }
            }
            Err(_) => {
                log::debug!("无法获取RAR文件元数据");
                return false;
            }
        }

        // 尝试读取文件头（简化检测）
        // 真正的RAR文件以"Rar!"或"RE~"开头
        match tokio::fs::read(file_path).await {
            Ok(data) if data.len() >= 7 => {
                // 检查RAR文件签名
                let is_rar = (data[0] == b'R' && data[1] == b'a' && data[2] == b'r' && data[3] == b'!' && data[4] == 0x1A && data[5] == 0x07 && data[6] == 0x00) ||
                             (data[0] == b'R' && data[1] == b'E' && data[2] == b'~' && data[3] == b'^' && data[4] == 0x1A && data[5] == 0x07 && data[6] == 0x00);

                if !is_rar {
                    log::debug!("文件头不是有效的RAR签名");
                }

                is_rar
            }
            _ => {
                log::debug!("无法读取RAR文件内容进行验证");
                false
            }
        }
    }

    /// 获取RAR文件信息
    pub async fn get_rar_info(
        &self,
        rar_path: &Path,
    ) -> Result<RarFileInfo, RarError> {
        log::debug!("获取RAR文件信息: {:?}", rar_path);

        // 首先验证是否是有效的RAR文件
        if !self.is_valid_rar_file(rar_path).await {
            return Err(RarError::ExtractionFailed("不是有效的RAR文件".to_string()));
        }

        // 获取文件元数据
        let metadata = tokio::fs::metadata(rar_path).await
            .map_err(|e| RarError::ExtractionFailed(format!("获取文件元数据失败: {}", e)))?;

        // 尝试列出内容以获取更多信息
        let contents = self.list_rar_contents(rar_path, None).await.ok();

        // 检查是否加密（简化检查）
        let is_encrypted = match self.test_rar_integrity(rar_path, None).await {
            Ok(true) => false, // 无密码测试成功，说明未加密
            Ok(false) => false, // 测试失败但不一定是加密
            Err(RarError::PasswordError) => true, // 密码错误说明加密了
            Err(_) => false, // 其他错误
        };

        Ok(RarFileInfo {
            path: rar_path.to_path_buf(),
            size: metadata.len(),
            is_encrypted,
            file_count: contents.as_ref().map(|c| c.len()).unwrap_or(0),
            compression_method: "RAR".to_string(),
            format_version: self.detect_rar_version(rar_path).await.unwrap_or("Unknown".to_string()),
        })
    }

    /// 检测RAR文件版本
    async fn detect_rar_version(&self, rar_path: &Path) -> Option<String> {
        // 尝试从文件头检测RAR版本
        match tokio::fs::read(rar_path).await {
            Ok(data) if data.len() >= 10 => {
                // RAR 5.0+ 签名
                if data[0] == b'R' && data[1] == b'a' && data[2] == b'r' && data[3] == b'!' &&
                   data[4] == 0x1A && data[5] == 0x07 && data[6] == 0x01 && data[7] == 0x00 {
                    return Some("5.0+".to_string());
                }
                // RAR 4.x 签名
                else if data[0] == b'R' && data[1] == b'a' && data[2] == b'r' && data[3] == b'!' &&
                        data[4] == 0x1A && data[5] == 0x07 && data[6] == 0x00 {
                    return Some("4.x".to_string());
                }
                // RAR 1.5-3.x 签名
                else if data[0] == b'R' && data[1] == b'E' && data[2] == b'~' && data[3] == b'^' &&
                        data[4] == 0x1A && data[5] == 0x07 && data[6] == 0x00 {
                    return Some("1.5-3.x".to_string());
                }
            }
            _ => {}
        }

        None
    }
}

/// RAR文件信息
#[derive(Debug, Clone)]
pub struct RarFileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub is_encrypted: bool,
    pub file_count: usize,
    pub compression_method: String,
    pub format_version: String,
}

impl Default for RarSupportService {
    fn default() -> Self {
        Self::new()
    }
}