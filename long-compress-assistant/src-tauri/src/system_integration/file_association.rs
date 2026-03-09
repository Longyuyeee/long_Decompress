//! 文件关联模块
//!
//! 提供文件类型关联功能，支持双击文件使用应用打开。

use tauri::{AppHandle, Manager};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::{Result, anyhow};

/// 文件关联配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAssociationConfig {
    pub enabled: bool,
    pub associations: Vec<FileTypeAssociation>,
    pub auto_register: bool,
    pub register_on_startup: bool,
}

impl Default for FileAssociationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            associations: vec![
                FileTypeAssociation {
                    extension: "zip".to_string(),
                    description: "ZIP压缩文件".to_string(),
                    icon: None,
                    open_with_app: true,
                    context_menu_items: vec![
                        ContextMenuItem {
                            id: "extract_here".to_string(),
                            label: "解压到当前文件夹".to_string(),
                            command: "extract_here".to_string(),
                        },
                        ContextMenuItem {
                            id: "extract_to".to_string(),
                            label: "解压到...".to_string(),
                            command: "extract_to".to_string(),
                        },
                        ContextMenuItem::separator(),
                        ContextMenuItem {
                            id: "compress_with".to_string(),
                            label: "使用胧压缩打开".to_string(),
                            command: "open_with_app".to_string(),
                        },
                    ],
                },
                FileTypeAssociation {
                    extension: "rar".to_string(),
                    description: "RAR压缩文件".to_string(),
                    icon: None,
                    open_with_app: true,
                    context_menu_items: vec![
                        ContextMenuItem {
                            id: "extract_here".to_string(),
                            label: "解压到当前文件夹".to_string(),
                            command: "extract_here".to_string(),
                        },
                        ContextMenuItem {
                            id: "extract_to".to_string(),
                            label: "解压到...".to_string(),
                            command: "extract_to".to_string(),
                        },
                    ],
                },
                FileTypeAssociation {
                    extension: "7z".to_string(),
                    description: "7-Zip压缩文件".to_string(),
                    icon: None,
                    open_with_app: true,
                    context_menu_items: vec![
                        ContextMenuItem {
                            id: "extract_here".to_string(),
                            label: "解压到当前文件夹".to_string(),
                            command: "extract_here".to_string(),
                        },
                        ContextMenuItem {
                            id: "extract_to".to_string(),
                            label: "解压到...".to_string(),
                            command: "extract_to".to_string(),
                        },
                    ],
                },
                FileTypeAssociation {
                    extension: "tar".to_string(),
                    description: "TAR归档文件".to_string(),
                    icon: None,
                    open_with_app: true,
                    context_menu_items: vec![
                        ContextMenuItem {
                            id: "extract_here".to_string(),
                            label: "解压到当前文件夹".to_string(),
                            command: "extract_here".to_string(),
                        },
                        ContextMenuItem {
                            id: "extract_to".to_string(),
                            label: "解压到...".to_string(),
                            command: "extract_to".to_string(),
                        },
                    ],
                },
                FileTypeAssociation {
                    extension: "gz".to_string(),
                    description: "GZIP压缩文件".to_string(),
                    icon: None,
                    open_with_app: true,
                    context_menu_items: vec![
                        ContextMenuItem {
                            id: "extract_here".to_string(),
                            label: "解压到当前文件夹".to_string(),
                            command: "extract_here".to_string(),
                        },
                        ContextMenuItem {
                            id: "extract_to".to_string(),
                            label: "解压到...".to_string(),
                            command: "extract_to".to_string(),
                        },
                    ],
                },
                FileTypeAssociation {
                    extension: "bz2".to_string(),
                    description: "BZIP2压缩文件".to_string(),
                    icon: None,
                    open_with_app: true,
                    context_menu_items: vec![
                        ContextMenuItem {
                            id: "extract_here".to_string(),
                            label: "解压到当前文件夹".to_string(),
                            command: "extract_here".to_string(),
                        },
                        ContextMenuItem {
                            id: "extract_to".to_string(),
                            label: "解压到...".to_string(),
                            command: "extract_to".to_string(),
                        },
                    ],
                },
                FileTypeAssociation {
                    extension: "xz".to_string(),
                    description: "XZ压缩文件".to_string(),
                    icon: None,
                    open_with_app: true,
                    context_menu_items: vec![
                        ContextMenuItem {
                            id: "extract_here".to_string(),
                            label: "解压到当前文件夹".to_string(),
                            command: "extract_here".to_string(),
                        },
                        ContextMenuItem {
                            id: "extract_to".to_string(),
                            label: "解压到...".to_string(),
                            command: "extract_to".to_string(),
                        },
                    ],
                },
            ],
            auto_register: true,
            register_on_startup: false,
        }
    }
}

/// 文件类型关联
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTypeAssociation {
    pub extension: String,
    pub description: String,
    pub icon: Option<String>,
    pub open_with_app: bool,
    pub context_menu_items: Vec<ContextMenuItem>,
}

/// 上下文菜单项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMenuItem {
    pub id: String,
    pub label: String,
    pub command: String,
}

impl ContextMenuItem {
    /// 创建分隔符
    pub fn separator() -> Self {
        Self {
            id: "separator".to_string(),
            label: "".to_string(),
            command: "".to_string(),
        }
    }
}

/// 文件关联管理器
pub struct FileAssociationManager {
    app_handle: AppHandle,
    config: FileAssociationConfig,
}

impl FileAssociationManager {
    /// 创建新的文件关联管理器
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            config: FileAssociationConfig::default(),
        }
    }

    /// 注册文件关联
    pub fn register_associations(&self) -> Result<()> {
        if !self.config.enabled {
            log::info!("文件关联功能已禁用");
            return Ok(());
        }

        log::info!("开始注册文件关联");

        for association in &self.config.associations {
            if let Err(e) = self.register_file_type(association) {
                log::warn!("注册文件类型关联失败 {}: {}", association.extension, e);
            }
        }

        log::info!("文件关联注册完成");
        Ok(())
    }

    /// 注册单个文件类型
    fn register_file_type(&self, association: &FileTypeAssociation) -> Result<()> {
        let extension = &association.extension;
        let description = &association.description;

        log::debug!("注册文件类型: .{} ({})", extension, description);

        // 获取应用信息
        let app_name = "胧压缩·方便助手";
        let app_path = std::env::current_exe()?;

        // 平台特定的注册逻辑
        #[cfg(target_os = "windows")]
        {
            self.register_windows_file_type(extension, description, app_name, &app_path, association)?;
        }

        #[cfg(target_os = "macos")]
        {
            self.register_macos_file_type(extension, description, app_name, &app_path, association)?;
        }

        #[cfg(target_os = "linux")]
        {
            self.register_linux_file_type(extension, description, app_name, &app_path, association)?;
        }

        Ok(())
    }

    /// 注册Windows文件类型
    #[cfg(target_os = "windows")]
    fn register_windows_file_type(
        &self,
        extension: &str,
        description: &str,
        app_name: &str,
        app_path: &PathBuf,
        association: &FileTypeAssociation,
    ) -> Result<()> {
        use winreg::{RegKey, enums::*};

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        // 创建文件类型键
        let file_type = format!("{}.File", extension);
        let (file_type_key, _) = hkcu.create_subkey(&format!("Software\\Classes\\.{}", extension))?;
        file_type_key.set_value("", &file_type)?;

        // 创建文件类型定义
        let (type_def_key, _) = hkcu.create_subkey(&format!("Software\\Classes\\{}", file_type))?;
        type_def_key.set_value("", &description)?;

        // 设置默认图标
        if let Some(icon_path) = &association.icon {
            let (default_icon_key, _) = type_def_key.create_subkey("DefaultIcon")?;
            default_icon_key.set_value("", icon_path)?;
        }

        // 设置打开命令
        if association.open_with_app {
            let (shell_key, _) = type_def_key.create_subkey("shell\\open\\command")?;
            let command = format!("\"{}\" \"%1\"", app_path.display());
            shell_key.set_value("", &command)?;
        }

        // 添加上下文菜单项
        for menu_item in &association.context_menu_items {
            if menu_item.id == "separator" {
                continue;
            }

            let menu_key_path = format!("Software\\Classes\\{}\\shell\\{}", file_type, menu_item.id);
            let (menu_key, _) = hkcu.create_subkey(&menu_key_path)?;
            menu_key.set_value("", &menu_item.label)?;

            let (command_key, _) = menu_key.create_subkey("command")?;
            let command = match menu_item.command.as_str() {
                "extract_here" => format!("\"{}\" extract --here \"%1\"", app_path.display()),
                "extract_to" => format!("\"{}\" extract --to \"%1\"", app_path.display()),
                "open_with_app" => format!("\"{}\" open \"%1\"", app_path.display()),
                _ => format!("\"{}\" \"%1\"", app_path.display()),
            };
            command_key.set_value("", &command)?;
        }

        Ok(())
    }

    /// 注册macOS文件类型
    #[cfg(target_os = "macos")]
    fn register_macos_file_type(
        &self,
        extension: &str,
        description: &str,
        app_name: &str,
        app_path: &PathBuf,
        association: &FileTypeAssociation,
    ) -> Result<()> {
        // macOS使用Launch Services注册文件类型
        // 这里简化实现，实际应用中需要使用Objective-C API
        log::warn!("macOS文件关联注册需要平台特定实现");
        Ok(())
    }

    /// 注册Linux文件类型
    #[cfg(target_os = "linux")]
    fn register_linux_file_type(
        &self,
        extension: &str,
        description: &str,
        app_name: &str,
        app_path: &PathBuf,
        association: &FileTypeAssociation,
    ) -> Result<()> {
        // Linux使用.desktop文件和MIME类型
        // 这里简化实现
        log::warn!("Linux文件关联注册需要平台特定实现");
        Ok(())
    }

    /// 取消注册文件关联
    pub fn unregister_associations(&self) -> Result<()> {
        log::info!("开始取消注册文件关联");

        for association in &self.config.associations {
            if let Err(e) = self.unregister_file_type(&association.extension) {
                log::warn!("取消注册文件类型关联失败 {}: {}", association.extension, e);
            }
        }

        log::info!("文件关联取消注册完成");
        Ok(())
    }

    /// 取消注册单个文件类型
    fn unregister_file_type(&self, extension: &str) -> Result<()> {
        log::debug!("取消注册文件类型: .{}", extension);

        // 平台特定的取消注册逻辑
        #[cfg(target_os = "windows")]
        {
            self.unregister_windows_file_type(extension)?;
        }

        #[cfg(target_os = "macos")]
        {
            self.unregister_macos_file_type(extension)?;
        }

        #[cfg(target_os = "linux")]
        {
            self.unregister_linux_file_type(extension)?;
        }

        Ok(())
    }

    /// 取消注册Windows文件类型
    #[cfg(target_os = "windows")]
    fn unregister_windows_file_type(&self, extension: &str) -> Result<()> {
        use winreg::{RegKey, enums::*};

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        // 删除文件类型键
        let extension_key_path = format!("Software\\Classes\\.{}", extension);
        if hkcu.open_subkey(&extension_key_path).is_ok() {
            hkcu.delete_subkey_all(&extension_key_path)?;
        }

        // 删除文件类型定义
        let file_type = format!("{}.File", extension);
        let type_def_key_path = format!("Software\\Classes\\{}", file_type);
        if hkcu.open_subkey(&type_def_key_path).is_ok() {
            hkcu.delete_subkey_all(&type_def_key_path)?;
        }

        Ok(())
    }

    /// 取消注册macOS文件类型
    #[cfg(target_os = "macos")]
    fn unregister_macos_file_type(&self, extension: &str) -> Result<()> {
        // macOS取消注册
        log::warn!("macOS文件关联取消注册需要平台特定实现");
        Ok(())
    }

    /// 取消注册Linux文件类型
    #[cfg(target_os = "linux")]
    fn unregister_linux_file_type(&self, extension: &str) -> Result<()> {
        // Linux取消注册
        log::warn!("Linux文件关联取消注册需要平台特定实现");
        Ok(())
    }

    /// 检查文件关联状态
    pub fn check_association_status(&self, extension: &str) -> Result<bool> {
        #[cfg(target_os = "windows")]
        {
            self.check_windows_association_status(extension)
        }

        #[cfg(target_os = "macos")]
        {
            self.check_macos_association_status(extension)
        }

        #[cfg(target_os = "linux")]
        {
            self.check_linux_association_status(extension)
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            Err(anyhow!("不支持的操作系统"))
        }
    }

    /// 检查Windows文件关联状态
    #[cfg(target_os = "windows")]
    fn check_windows_association_status(&self, extension: &str) -> Result<bool> {
        use winreg::{RegKey, enums::*};

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let extension_key_path = format!("Software\\Classes\\.{}", extension);

        match hkcu.open_subkey(&extension_key_path) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// 检查macOS文件关联状态
    #[cfg(target_os = "macos")]
    fn check_macos_association_status(&self, extension: &str) -> Result<bool> {
        // 简化实现
        Ok(false)
    }

    /// 检查Linux文件关联状态
    #[cfg(target_os = "linux")]
    fn check_linux_association_status(&self, extension: &str) -> Result<bool> {
        // 简化实现
        Ok(false)
    }

    /// 处理文件打开请求
    pub fn handle_file_open(&self, file_path: &str) -> Result<()> {
        log::info!("处理文件打开请求: {}", file_path);

        // 获取文件扩展名
        let path = PathBuf::from(file_path);
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        // 发送事件到前端
        let event_data = serde_json::json!({
            "file_path": file_path,
            "extension": extension,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        if let Err(e) = self.app_handle.emit_all("file_opened", event_data) {
            log::error!("发送文件打开事件失败: {}", e);
        }

        Ok(())
    }

    /// 获取配置
    pub fn get_config(&self) -> &FileAssociationConfig {
        &self.config
    }

    /// 更新配置
    pub fn update_config(&mut self, new_config: FileAssociationConfig) {
        self.config = new_config;
    }

    /// 添加文件类型关联
    pub fn add_association(&mut self, association: FileTypeAssociation) {
        self.config.associations.push(association);
    }

    /// 移除文件类型关联
    pub fn remove_association(&mut self, extension: &str) -> bool {
        let original_len = self.config.associations.len();
        self.config.associations.retain(|assoc| assoc.extension != extension);
        original_len != self.config.associations.len()
    }
}

/// 全局文件关联管理器
pub struct GlobalFileAssociationManager {
    manager: std::sync::Arc<tokio::sync::RwLock<Option<FileAssociationManager>>>,
}

impl GlobalFileAssociationManager {
    /// 创建新的全局文件关联管理器
    pub fn new() -> Self {
        Self {
            manager: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// 初始化全局文件关联管理器
    pub async fn initialize(&self, app_handle: AppHandle) -> Result<()> {
        let mut manager_guard = self.manager.write().await;

        if manager_guard.is_none() {
            let manager = FileAssociationManager::new(app_handle);
            *manager_guard = Some(manager);
            log::info!("全局文件关联管理器初始化完成");
        }

        Ok(())
    }

    /// 获取文件关联管理器实例
    pub async fn get(&self) -> Result<std::sync::Arc<FileAssociationManager>, String> {
        let manager_guard = self.manager.read().await;
        manager_guard
            .as_ref()
            .cloned()
            .ok_or_else(|| "文件关联管理器未初始化".to_string())
    }
}

// 实现默认的全局实例
lazy_static::lazy_static! {
    pub static ref FILE_ASSOCIATION_MANAGER: GlobalFileAssociationManager = GlobalFileAssociationManager::new();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_association_config_default() {
        let config = FileAssociationConfig::default();

        assert!(config.enabled);
        assert!(config.auto_register);
        assert!(!config.register_on_startup);
        assert!(!config.associations.is_empty());

        // 检查默认关联
        let has_zip = config.associations.iter().any(|assoc| assoc.extension == "zip");
        let has_rar = config.associations.iter().any(|assoc| assoc.extension == "rar");
        let has_7z = config.associations.iter().any(|assoc| assoc.extension == "7z");

        assert!(has_zip);
        assert!(has_rar);
        assert!(has_7z);
    }

    #[test]
    fn test_file_type_association() {
        let association = FileTypeAssociation {
            extension: "test".to_string(),
            description: "测试文件".to_string(),
            icon: Some("test.ico".to_string()),
            open_with_app: true,
            context_menu_items: vec![
                ContextMenuItem {
                    id: "open".to_string(),
                    label: "打开".to_string(),
                    command: "open".to_string(),
                },
                ContextMenuItem::separator(),
            ],
        };

        assert_eq!(association.extension, "test");
        assert_eq!(association.description, "测试文件");
        assert!(association.open_with_app);
        assert_eq!(association.context_menu_items.len(), 2);
    }

    #[test]
    fn test_context_menu_item() {
        let normal_item = ContextMenuItem {
            id: "test_item".to_string(),
            label: "测试项".to_string(),
            command: "test_command".to_string(),
        };

        assert_eq!(normal_item.id, "test_item");
        assert_eq!(normal_item.label, "测试项");
        assert_eq!(normal_item.command, "test_command");

        let separator = ContextMenuItem::separator();
        assert_eq!(separator.id, "separator");
        assert!(separator.label.is_empty());
        assert!(separator.command.is_empty());
    }
}