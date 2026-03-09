//! 配置管理模块
//!
//! 负责管理应用程序的所有配置项，包括系统设置、用户偏好、压缩参数等。
//! 提供配置的存储、加载、验证、导入导出等功能。

pub mod models;
pub mod repository;
pub mod service;
pub mod validation;
pub mod export_import;
pub mod listeners;
pub mod commands;
pub mod file_loader;

// 重新导出常用类型
pub use models::*;
pub use service::ConfigService;