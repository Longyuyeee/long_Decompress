use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use crate::models::compression::TaskLogSeverity;

/// 进度回调类型
pub type ProgressCallback<'a> = Box<dyn Fn(f32) + Send + Sync + 'a>;

/// 日志回调类型
pub type LogCallback<'a> = Box<dyn Fn(&str, TaskLogSeverity) + Send + Sync + 'a>;

/// 统一的归档引擎接口
#[async_trait::async_trait]
pub trait ArchiveEngine: Send + Sync {
    /// 引擎名称，用于日志记录
    fn name(&self) -> &'static str;

    /// 是否支持处理该文件（通常通过探测 Header 或后缀）
    async fn can_handle(&self, header: &[u8], ext: &str) -> bool;

    /// 该引擎是否原生支持密码功能
    fn supports_password(&self) -> bool {
        false
    }

    /// 测试给定密码是否能够解密该归档
    /// 返回 Ok(true) 表示密码正确或文件未加密
    /// 返回 Ok(false) 表示明确的密码错误
    /// 返回 Err 表示文件损坏或其他系统错误
    async fn try_password(&self, _file_path: &Path, _password: &str) -> Result<bool> {
        if !self.supports_password() {
            return Ok(true);
        }
        Ok(false)
    }

    /// 判断文件是否被加密且需要密码
    async fn requires_password(&self, _file_path: &Path) -> Result<bool> {
        Ok(false)
    }

    /// 执行解压操作，并支持进度反馈和中断信号
    async fn extract_with_progress(
        &self,
        file_path: &Path,
        output_dir: &Path,
        password: Option<&str>,
        on_progress: Arc<dyn Fn(f32) + Send + Sync>,
        on_log: Arc<dyn Fn(String, TaskLogSeverity) + Send + Sync>,
        is_cancelled: Arc<AtomicBool>,
    ) -> Result<()>;
}
