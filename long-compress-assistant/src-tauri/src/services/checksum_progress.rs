//! Shared byte observations; cancellation never returns a digest for a partial read.
use serde::Serialize;
use std::path::Path;
use super::file_integrity_service::{ChecksumAlgorithm, FileIntegrityService};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecksumCalculationEntry {
    pub path: String,
    pub file_name: String,
    pub algorithm: String,
    pub checksum: String,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecksumProgress {
    pub current_file: String,
    pub file_bytes: u64,
    pub file_size: u64,
    pub completed_bytes: u64,
    pub total_bytes: u64,
    pub completed_files: usize,
    pub total_files: usize,
    pub entry: Option<super::checksum_manifest::ChecksumVerificationEntry>,
    pub report: Option<super::checksum_manifest::ChecksumVerificationReport>,
    pub calculation_results: Option<Vec<ChecksumCalculationEntry>>,
}

pub fn calculate_batch(
    paths: &[String], algorithm: ChecksumAlgorithm, label: &str,
    cancelled: &dyn Fn() -> bool, progress: &mut dyn FnMut(ChecksumProgress),
) -> Vec<ChecksumCalculationEntry> {
    let mut rows: Vec<_> = paths.iter().map(|path| ChecksumCalculationEntry {
        path: path.clone(), file_name: Path::new(path).file_name().unwrap_or_default().to_string_lossy().into_owned(),
        algorithm: label.to_ascii_uppercase(), checksum: String::new(), status: "pending".into(), error: None,
    }).collect();
    let mut state = ChecksumProgress { total_files: paths.len(), total_bytes: paths.iter().filter_map(|p| std::fs::metadata(p).ok()).filter(|m| m.is_file()).map(|m| m.len()).sum(), ..Default::default() };
    state.calculation_results = Some(rows.clone()); progress(state.clone()); state.calculation_results = None;
    for index in 0..rows.len() {
        if cancelled() { break; }
        state.current_file = rows[index].path.clone(); state.file_bytes = 0; state.file_size = 0;
        let before = state.completed_bytes;
        let result = FileIntegrityService::calculate_checksum_with_progress(Path::new(&rows[index].path), algorithm, &mut |bytes, size| {
            state.file_bytes = bytes; state.file_size = size; state.completed_bytes = before + bytes;
            progress(state.clone()); !cancelled()
        });
        match result {
            Ok(Some(result)) => { rows[index].checksum = result.checksum; rows[index].status = "success".into(); }
            Ok(None) => { rows[index].status = "interrupted".into(); rows[index].error = Some("已停止；继续时从头读取此文件".into()); }
            Err(error) => { rows[index].status = "error".into(); rows[index].error = Some(format!("{error:#}")); }
        }
        if rows[index].status != "interrupted" { state.completed_files += 1; }
        state.calculation_results = Some(vec![rows[index].clone()]); progress(state.clone()); state.calculation_results = None;
        if cancelled() { break; }
    }
    rows
}
