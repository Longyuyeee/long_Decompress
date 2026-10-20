use crate::services::file_integrity_service::{ChecksumAlgorithm, FileIntegrityService};
use serde::Deserialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use crate::services::checksum_manifest::ChecksumVerificationReport;
use crate::services::checksum_progress::{ChecksumProgress, ChecksumCalculationEntry};
use serde::Serialize;
use std::sync::{Arc, Mutex, OnceLock, atomic::{AtomicBool, Ordering}};
use std::collections::HashMap;
use std::time::{Instant, Duration};

static OPERATIONS: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
struct Operation { key: String, cancelled: Arc<AtomicBool> }
impl Operation {
    fn start(window: &tauri::Window, id: &str) -> Result<Self, String> {
        let key = format!("{}:{id}", window.label());
        let mut operations = OPERATIONS.get_or_init(Default::default).lock().map_err(|e| e.to_string())?;
        if operations.contains_key(&key) { return Err("校验请求正在执行".into()); }
        let cancelled = Arc::new(AtomicBool::new(false));
        operations.insert(key.clone(), cancelled.clone());
        Ok(Self { key, cancelled })
    }
}
impl Drop for Operation {
    fn drop(&mut self) { if let Ok(mut operations) = OPERATIONS.get_or_init(Default::default).lock() { operations.remove(&self.key); } }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent { operation_id: String, #[serde(flatten)] progress: ChecksumProgress }
fn emitter(window: tauri::Window, operation_id: String) -> impl FnMut(ChecksumProgress) {
    let mut last = Instant::now() - Duration::from_secs(1);
    let mut current = String::new();
    move |progress| {
        let important = progress.report.is_some() || progress.entry.is_some() || progress.calculation_results.is_some() || progress.current_file != current;
        if important || last.elapsed() >= Duration::from_millis(80) {
            current = progress.current_file.clone(); last = Instant::now();
            let _ = window.emit("checksum-progress", ProgressEvent { operation_id: operation_id.clone(), progress });
        }
    }
}

#[tauri::command]
pub fn cancel_checksum_operation(window: tauri::Window, operation_id: String) -> Result<bool, String> {
    let operations = OPERATIONS.get_or_init(Default::default).lock().map_err(|e| e.to_string())?;
    let flag = operations.get(&format!("{}:{operation_id}", window.label()));
    if let Some(flag) = flag { flag.store(true, Ordering::Relaxed); }
    Ok(flag.is_some())
}

#[tauri::command]
pub async fn calculate_checksum_batch(window: tauri::Window, operation_id: String, paths: Vec<String>, algorithm: String) -> Result<Vec<ChecksumCalculationEntry>, String> {
    let algorithm_kind = parse_algorithm(&algorithm)?;
    let operation = Operation::start(&window, &operation_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        crate::services::checksum_progress::calculate_batch(&paths, algorithm_kind, &algorithm, &|| _operation.cancelled.load(Ordering::Relaxed), &mut emitter(window, operation_id))
    }).await.map_err(|error| error.to_string())
}

#[derive(Debug, Deserialize)]
pub struct ChecksumExportEntry {
    pub file_name: String,
    pub checksum: String,
}

fn parse_algorithm(value: &str) -> Result<ChecksumAlgorithm, String> {
    match value.to_ascii_lowercase().as_str() {
        "crc32" => Ok(ChecksumAlgorithm::CRC32),
        "md5" => Ok(ChecksumAlgorithm::MD5),
        "sha256" => Ok(ChecksumAlgorithm::SHA256),
        _ => Err(format!("Unsupported checksum algorithm: {value}")),
    }
}

#[tauri::command]
pub async fn calculate_checksum(path: String, algorithm: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let algorithm = parse_algorithm(&algorithm)?;
        FileIntegrityService::calculate_checksum(Path::new(&path), algorithm)
            .map(|result| result.checksum)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn export_checksum_file(
    path: String,
    results: Vec<ChecksumExportEntry>,
    algorithm: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let algorithm = parse_algorithm(&algorithm)?;
        let mut output = std::fs::File::create(&path).map_err(|error| error.to_string())?;

        for entry in results {
            let line = match algorithm {
                ChecksumAlgorithm::CRC32 => format!("{} {}\n", entry.file_name, entry.checksum),
                _ => format!("{}  {}\n", entry.checksum, entry.file_name),
            };
            output
                .write_all(line.as_bytes())
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn verify_checksum_file(
    window: tauri::Window,
    checksum_path: String,
    entry_lines: Option<Vec<usize>>,
    operation_id: Option<String>,
    expected_manifest_id: Option<String>,
) -> Result<ChecksumVerificationReport, String> {
    let operation_id = operation_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let operation = Operation::start(&window, &operation_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        let checksum_file = PathBuf::from(&checksum_path);
        let base_dir = checksum_file.parent().unwrap_or_else(|| Path::new("."));
        crate::services::checksum_manifest::verify_with_progress(&checksum_file, base_dir, entry_lines.as_deref(), expected_manifest_id.as_deref(), &|| _operation.cancelled.load(Ordering::Relaxed), &mut emitter(window, operation_id))
            .map_err(|error| format!("{error:#}"))
    })
    .await
    .map_err(|error| error.to_string())?
}
