use crate::models::compression::TaskLogSeverity;
use crate::services::io_buffer_pool::IOBufferPool;
use anyhow::Result;
use std::path::{Path, PathBuf};
use tauri::Window;

pub(crate) mod single_stream;
pub(crate) mod tar;
pub(crate) mod zip;

pub(crate) trait ExtractionRuntime {
    fn check_cancellation(&self) -> Result<()>;
    fn buffer_pool(&self) -> &IOBufferPool;
    fn copy_buffer_size(&self) -> usize;
    fn normalized_archive_path(&self, path: &Path, preserve_paths: bool) -> Option<PathBuf>;
    fn emit_log(&self, window: &Window, task_id: &str, message: &str, severity: TaskLogSeverity);
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
