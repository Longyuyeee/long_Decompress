use crate::models::compression::TaskLogSeverity;
use anyhow::Result;
use std::sync::{atomic::AtomicBool, Arc};
use tauri::Window;

pub(crate) mod aes;
pub(crate) mod byte_progress;
pub(crate) mod seven_zip;
pub(crate) mod single_stream;
pub(crate) mod tar;
pub(crate) mod zip;

pub(crate) trait CompressionRuntime {
    fn check_cancellation(&self) -> Result<()>;
    fn cancellation_flag(&self) -> Arc<AtomicBool>;
    fn copy_buffer_size(&self) -> usize;
    fn emit_log(&self, window: &Window, task_id: &str, message: &str, severity: TaskLogSeverity);
    fn emit_split_progress(&self, window: &Window, task_id: &str, processed: u64, total: u64) {
        self.emit_progress(window, task_id, if total == 0 { 0.0 } else { processed as f32 / total as f32 }, None, processed, total);
    }
    fn emit_progress(
        &self,
        window: &Window,
        task_id: &str,
        progress: f32,
        current_file: Option<String>,
        processed_bytes: u64,
        total_bytes: u64,
    );
}
