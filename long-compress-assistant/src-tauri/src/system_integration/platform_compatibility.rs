//! 平台兼容性模块
//!
//! 提供系统集成功能的跨平台兼容性检查和适配。

use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};

/// 平台类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PlatformType {
    Windows,
    MacOS,
    Linux,
    Unknown,
}

/// 平台功能支持状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FeatureSupport {
    /// 功能完全支持
    FullySupported,
    /// 功能部分支持
    PartiallySupported,
    /// 功能不支持
    NotSupported,
    /// 需要额外配置
    RequiresConfiguration,
    /// 需要管理员权限
    RequiresAdmin,
}

/// 平台功能检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformFeatureCheck {
    pub feature_name: String,
    pub platform: PlatformType,
    pub support_status: FeatureSupport,
    pub description: String,
    pub notes: Vec<String>,
    pub workarounds: Vec<String>,
}

/// 系统集成功能
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SystemIntegrationFeature {
    /// 文件关联
    FileAssociation,
    /// 全局快捷键
    GlobalShortcut,
    /// 系统托盘
    SystemTray,
    /// 右键菜单
    ContextMenu,
    /// 系统通知
    SystemNotification,
    /// 自动启动
    AutoStart,
    /// 文件拖放
    FileDragDrop,
    /// 剪贴板集成
    ClipboardIntegration,
}

/// 平台兼容性检查器
pub struct PlatformCompatibilityChecker;

impl PlatformCompatibilityChecker {
    /// 获取当前平台类型
    pub fn get_current_platform() -> PlatformType {
        if cfg!(target_os = "windows") {
            PlatformType::Windows
        } else if cfg!(target_os = "macos") {
            PlatformType::MacOS
        } else if cfg!(target_os = "linux") {
            PlatformType::Linux
        } else {
            PlatformType::Unknown
        }
    }

    /// 检查特定功能的平台支持
    pub fn check_feature_support(feature: SystemIntegrationFeature) -> PlatformFeatureCheck {
        let platform = Self::get_current_platform();

        match feature {
            SystemIntegrationFeature::FileAssociation => Self::check_file_association_support(platform),
            SystemIntegrationFeature::GlobalShortcut => Self::check_global_shortcut_support(platform),
            SystemIntegrationFeature::SystemTray => Self::check_system_tray_support(platform),
            SystemIntegrationFeature::ContextMenu => Self::check_context_menu_support(platform),
            SystemIntegrationFeature::SystemNotification => Self::check_system_notification_support(platform),
            SystemIntegrationFeature::AutoStart => Self::check_auto_start_support(platform),
            SystemIntegrationFeature::FileDragDrop => Self::check_file_drag_drop_support(platform),
            SystemIntegrationFeature::ClipboardIntegration => Self::check_clipboard_integration_support(platform),
        }
    }

    /// 检查所有功能的平台支持
    pub fn check_all_features() -> Vec<PlatformFeatureCheck> {
        let features = vec![
            SystemIntegrationFeature::FileAssociation,
            SystemIntegrationFeature::GlobalShortcut,
            SystemIntegrationFeature::SystemTray,
            SystemIntegrationFeature::ContextMenu,
            SystemIntegrationFeature::SystemNotification,
            SystemIntegrationFeature::AutoStart,
            SystemIntegrationFeature::FileDragDrop,
            SystemIntegrationFeature::ClipboardIntegration,
        ];

        features.into_iter()
            .map(Self::check_feature_support)
            .collect()
    }

    /// 检查文件关联支持
    fn check_file_association_support(platform: PlatformType) -> PlatformFeatureCheck {
        match platform {
            PlatformType::Windows => PlatformFeatureCheck {
                feature_name: "文件关联".to_string(),
                platform: PlatformType::Windows,
                support_status: FeatureSupport::FullySupported,
                description: "Windows提供完整的文件关联注册支持".to_string(),
                notes: vec![
                    "需要管理员权限修改系统级关联".to_string(),
                    "用户级关联可在HKCU注册表中设置".to_string(),
                ],
                workarounds: vec![
                    "使用用户级注册避免权限问题".to_string(),
                    "提供手动关联设置选项".to_string(),
                ],
            },
            PlatformType::MacOS => PlatformFeatureCheck {
                feature_name: "文件关联".to_string(),
                platform: PlatformType::MacOS,
                support_status: FeatureSupport::PartiallySupported,
                description: "macOS通过Launch Services支持文件关联".to_string(),
                notes: vec![
                    "需要应用签名以获得最佳兼容性".to_string(),
                    "用户需要手动确认关联".to_string(),
                ],
                workarounds: vec![
                    "使用UTI(Uniform Type Identifier)定义文件类型".to_string(),
                    "在Info.plist中声明支持的文件类型".to_string(),
                ],
            },
            PlatformType::Linux => PlatformFeatureCheck {
                feature_name: "文件关联".to_string(),
                platform: PlatformType::Linux,
                support_status: FeatureSupport::FullySupported,
                description: "Linux通过.desktop文件和MIME类型支持文件关联".to_string(),
                notes: vec![
                    "不同桌面环境实现略有差异".to_string(),
                    "需要写入~/.local/share/applications目录".to_string(),
                ],
                workarounds: vec![
                    "为不同桌面环境提供适配".to_string(),
                    "使用xdg-utils工具进行注册".to_string(),
                ],
            },
            PlatformType::Unknown => PlatformFeatureCheck {
                feature_name: "文件关联".to_string(),
                platform: PlatformType::Unknown,
                support_status: FeatureSupport::NotSupported,
                description: "未知平台，文件关联支持不确定".to_string(),
                notes: vec!["平台检测失败".to_string()],
                workarounds: vec!["使用平台检测失败处理".to_string()],
            },
        }
    }

    /// 检查全局快捷键支持
    fn check_global_shortcut_support(platform: PlatformType) -> PlatformFeatureCheck {
        match platform {
            PlatformType::Windows => PlatformFeatureCheck {
                feature_name: "全局快捷键".to_string(),
                platform: PlatformType::Windows,
                support_status: FeatureSupport::FullySupported,
                description: "Windows提供完整的全局快捷键注册支持".to_string(),
                notes: vec![
                    "需要处理快捷键冲突".to_string(),
                    "某些系统快捷键可能被占用".to_string(),
                ],
                workarounds: vec![
                    "提供快捷键冲突检测".to_string(),
                    "允许用户自定义快捷键".to_string(),
                ],
            },
            PlatformType::MacOS => PlatformFeatureCheck {
                feature_name: "全局快捷键".to_string(),
                platform: PlatformType::MacOS,
                support_status: FeatureSupport::RequiresConfiguration,
                description: "macOS需要辅助功能权限才能使用全局快捷键".to_string(),
                notes: vec![
                    "用户需要在系统偏好设置中授权".to_string(),
                    "应用需要明确请求权限".to_string(),
                ],
                workarounds: vec![
                    "引导用户完成权限设置".to_string(),
                    "提供权限检查工具".to_string(),
                ],
            },
            PlatformType::Linux => PlatformFeatureCheck {
                feature_name: "全局快捷键".to_string(),
                platform: PlatformType::Linux,
                support_status: FeatureSupport::PartiallySupported,
                description: "Linux全局快捷键支持取决于桌面环境".to_string(),
                notes: vec![
                    "X11和Wayland支持不同".to_string(),
                    "需要处理多个桌面环境".to_string(),
                ],
                workarounds: vec![
                    "检测当前桌面环境".to_string(),
                    "提供环境特定的实现".to_string(),
                ],
            },
            PlatformType::Unknown => PlatformFeatureCheck {
                feature_name: "全局快捷键".to_string(),
                platform: PlatformType::Unknown,
                support_status: FeatureSupport::NotSupported,
                description: "未知平台，全局快捷键支持不确定".to_string(),
                notes: vec!["平台检测失败".to_string()],
                workarounds: vec!["使用平台检测失败处理".to_string()],
            },
        }
    }

    /// 检查系统托盘支持
    fn check_system_tray_support(platform: PlatformType) -> PlatformFeatureCheck {
        match platform {
            PlatformType::Windows => PlatformFeatureCheck {
                feature_name: "系统托盘".to_string(),
                platform: PlatformType::Windows,
                support_status: FeatureSupport::FullySupported,
                description: "Windows提供完整的系统托盘支持".to_string(),
                notes: vec![
                    "支持托盘图标、菜单和通知".to_string(),
                    "需要处理高DPI显示".to_string(),
                ],
                workarounds: vec![
                    "提供多种图标尺寸".to_string(),
                    "支持深色/浅色模式".to_string(),
                ],
            },
            PlatformType::MacOS => PlatformFeatureCheck {
                feature_name: "系统托盘".to_string(),
                platform: PlatformType::MacOS,
                support_status: FeatureSupport::FullySupported,
                description: "macOS通过状态栏提供系统托盘功能".to_string(),
                notes: vec![
                    "在macOS上称为状态栏(Status Bar)".to_string(),
                    "支持菜单和自定义视图".to_string(),
                ],
                workarounds: vec![
                    "遵循macOS设计规范".to_string(),
                    "提供适当的图标尺寸".to_string(),
                ],
            },
            PlatformType::Linux => PlatformFeatureCheck {
                feature_name: "系统托盘".to_string(),
                platform: PlatformType::Linux,
                support_status: FeatureSupport::FullySupported,
                description: "Linux通过系统托盘协议支持托盘功能".to_string(),
                notes: vec![
                    "需要支持StatusNotifierItem协议".to_string(),
                    "不同桌面环境实现一致".to_string(),
                ],
                workarounds: vec![
                    "使用libappindicator或Qt系统托盘".to_string(),
                    "检测并适配不同协议".to_string(),
                ],
            },
            PlatformType::Unknown => PlatformFeatureCheck {
                feature_name: "系统托盘".to_string(),
                platform: PlatformType::Unknown,
                support_status: FeatureSupport::NotSupported,
                description: "未知平台，系统托盘支持不确定".to_string(),
                notes: vec!["平台检测失败".to_string()],
                workarounds: vec!["使用平台检测失败处理".to_string()],
            },
        }
    }

    /// 检查右键菜单支持
    fn check_context_menu_support(platform: PlatformType) -> PlatformFeatureCheck {
        match platform {
            PlatformType::Windows => PlatformFeatureCheck {
                feature_name: "右键菜单".to_string(),
                platform: PlatformType::Windows,
                support_status: FeatureSupport::FullySupported,
                description: "Windows提供完整的文件右键菜单支持".to_string(),
                notes: vec![
                    "通过注册表添加菜单项".to_string(),
                    "支持图标和子菜单".to_string(),
                ],
                workarounds: vec![
                    "提供菜单项管理界面".to_string(),
                    "支持动态菜单更新".to_string(),
                ],
            },
            PlatformType::MacOS => PlatformFeatureCheck {
                feature_name: "右键菜单".to_string(),
                platform: PlatformType::MacOS,
                support_status: FeatureSupport::PartiallySupported,
                description: "macOS通过服务菜单支持右键菜单".to_string(),
                notes: vec![
                    "需要应用签名".to_string(),
                    "用户需要手动启用".to_string(),
                ],
                workarounds: vec![
                    "使用Automator工作流".to_string(),
                    "提供安装脚本".to_string(),
                ],
            },
            PlatformType::Linux => PlatformFeatureCheck {
                feature_name: "右键菜单".to_string(),
                platform: PlatformType::Linux,
                support_status: FeatureSupport::PartiallySupported,
                description: "Linux右键菜单支持取决于文件管理器和桌面环境".to_string(),
                notes: vec![
                    "不同文件管理器支持不同".to_string(),
                    "需要创建.desktop文件".to_string(),
                ],
                workarounds: vec![
                    "为常见文件管理器提供适配".to_string(),
                    "使用脚本安装菜单项".to_string(),
                ],
            },
            PlatformType::Unknown => PlatformFeatureCheck {
                feature_name: "右键菜单".to_string(),
                platform: PlatformType::Unknown,
                support_status: FeatureSupport::NotSupported,
                description: "未知平台，右键菜单支持不确定".to_string(),
                notes: vec!["平台检测失败".to_string()],
                workarounds: vec!["使用平台检测失败处理".to_string()],
            },
        }
    }

    /// 检查系统通知支持
    fn check_system_notification_support(platform: PlatformType) -> PlatformFeatureCheck {
        match platform {
            PlatformType::Windows => PlatformFeatureCheck {
                feature_name: "系统通知".to_string(),
                platform: PlatformType::Windows,
                support_status: FeatureSupport::FullySupported,
                description: "Windows 10+提供Toast通知支持".to_string(),
                notes: vec![
                    "需要应用ID和快捷方式".to_string(),
                    "支持操作按钮和输入".to_string(),
                ],
                workarounds: vec![
                    "注册应用ID".to_string(),
                    "创建快捷方式".to_string(),
                ],
            },
            PlatformType::MacOS => PlatformFeatureCheck {
                feature_name: "系统通知".to_string(),
                platform: PlatformType::MacOS,
                support_status: FeatureSupport::FullySupported,
                description: "macOS通过UserNotifications框架支持通知".to_string(),
                notes: vec![
                    "需要用户授权".to_string(),
                    "支持自定义操作".to_string(),
                ],
                workarounds: vec![
                    "请求通知权限".to_string(),
                    "提供权限引导".to_string(),
                ],
            },
            PlatformType::Linux => PlatformFeatureCheck {
                feature_name: "系统通知".to_string(),
                platform: PlatformType::Linux,
                support_status: FeatureSupport::FullySupported,
                description: "Linux通过DBus和通知服务器支持通知".to_string(),
                notes: vec![
                    "使用freedesktop.org通知规范".to_string(),
                    "不同通知服务器实现一致".to_string(),
                ],
                workarounds: vec![
                    "使用libnotify或类似库".to_string(),
                    "处理不同通知服务器".to_string(),
                ],
            },
            PlatformType::Unknown => PlatformFeatureCheck {
                feature_name: "系统通知".to_string(),
                platform: PlatformType::Unknown,
                support_status: FeatureSupport::NotSupported,
                description: "未知平台，系统通知支持不确定".to_string(),
                notes: vec!["平台检测失败".to_string()],
                workarounds: vec!["使用平台检测失败处理".to_string()],
            },
        }
    }

    /// 检查自动启动支持
    fn check_auto_start_support(platform: PlatformType) -> PlatformFeatureCheck {
        match platform {
            PlatformType::Windows => PlatformFeatureCheck {
                feature_name: "自动启动".to_string(),
                platform: PlatformType::Windows,
                support_status: FeatureSupport::FullySupported,
                description: "Windows通过注册表或启动文件夹支持自动启动".to_string(),
                notes: vec![
                    "用户级启动在HKCU注册表".to_string(),
                    "系统级启动需要管理员权限".to_string(),
                ],
                workarounds: vec![
                    "使用用户级注册避免权限问题".to_string(),
                    "提供启动选项配置".to_string(),
                ],
            },
            PlatformType::MacOS => PlatformFeatureCheck {
                feature_name: "自动启动".to_string(),
                platform: PlatformType::MacOS,
                support_status: FeatureSupport::FullySupported,
                description: "macOS通过Launch Agents/Daemons支持自动启动".to_string(),
                notes: vec![
                    "用户级启动在~/Library/LaunchAgents".to_string(),
                    "需要创建.plist文件".to_string(),
                ],
                workarounds: vec![
                    "创建.plist配置文件".to_string(),
                    "使用launchctl加载".to_string(),
                ],
            },
            PlatformType::Linux => PlatformFeatureCheck {
                feature_name: "自动启动".to_string(),
                platform: PlatformType::Linux,
                support_status: FeatureSupport::FullySupported,
                description: "Linux通过.autostart文件或systemd支持自动启动".to_string(),
                notes: vec![
                    "用户级启动在~/.config/autostart".to_string(),
                    "需要创建.desktop文件".to_string(),
                ],
                workarounds: vec![
                    "创建.desktop文件".to_string(),
                    "支持不同初始化系统".to_string(),
                ],
            },
            PlatformType::Unknown => PlatformFeatureCheck {
                feature_name: "自动启动".to_string(),
                platform: PlatformType::Unknown,
                support_status: FeatureSupport::NotSupported,
                description: "未知平台，自动启动支持不确定".to_string(),
                notes: vec!["平台检测失败".to_string()],
                workarounds: vec!["使用平台检测失败处理".to_string()],
            },
        }
    }

    /// 检查剪贴板集成支持
    fn check_clipboard_integration_support(platform: PlatformType) -> PlatformFeatureCheck {
        match platform {
            PlatformType::Windows => PlatformFeatureCheck {
                feature_name: "剪贴板集成".to_string(),
                platform: PlatformType::Windows,
                support_status: FeatureSupport::FullySupported,
                description: "Windows提供完整的剪贴板支持".to_string(),
                notes: vec![
                    "支持多种格式（文本、文件、图片）".to_string(),
                    "支持剪贴板历史记录（Win10+）".to_string(),
                ],
                workarounds: vec![
                    "使用Win32 API进行高级剪贴板操作".to_string(),
                ],
            },
            PlatformType::MacOS => PlatformFeatureCheck {
                feature_name: "剪贴板集成".to_string(),
                platform: PlatformType::MacOS,
                support_status: FeatureSupport::FullySupported,
                description: "macOS通过NSPasteboard支持剪贴板".to_string(),
                notes: vec![
                    "支持通用剪贴板（Handoff）".to_string(),
                    "支持多种数据类型".to_string(),
                ],
                workarounds: vec![
                    "使用AppKit访问剪贴板".to_string(),
                ],
            },
            PlatformType::Linux => PlatformFeatureCheck {
                feature_name: "剪贴板集成".to_string(),
                platform: PlatformType::Linux,
                support_status: FeatureSupport::PartiallySupported,
                description: "Linux剪贴板支持取决于窗口系统（X11/Wayland）".to_string(),
                notes: vec![
                    "需要处理不同剪贴板选择（CLIPBOARD, PRIMARY）".to_string(),
                    "Wayland对剪贴板访问有安全限制".to_string(),
                ],
                workarounds: vec![
                    "使用xclip或wl-clipboard工具".to_string(),
                    "适配不同桌面环境的剪贴板服务".to_string(),
                ],
            },
            PlatformType::Unknown => PlatformFeatureCheck {
                feature_name: "剪贴板集成".to_string(),
                platform: PlatformType::Unknown,
                support_status: FeatureSupport::NotSupported,
                description: "未知平台，剪贴板支持不确定".to_string(),
                notes: vec!["平台检测失败".to_string()],
                workarounds: vec!["使用平台检测失败处理".to_string()],
            },
        }
    }
}
