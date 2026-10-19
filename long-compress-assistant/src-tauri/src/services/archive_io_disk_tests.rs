//! Actual NTFS exhaustion; invoked only on a bounded CI-owned VHD.
use super::{archive_io, compression_service::CompressionService, native_compression as create,
    native_extraction as extract, extraction_transaction::ExtractionStaging, io_buffer_pool::IOBufferPool};
use crate::models::compression::{CompressionOptions, DecompressOptions, TaskLogSeverity};
use anyhow::Result;
use std::{io::Write, path::{Path, PathBuf}, sync::{Arc, atomic::AtomicBool}};
use tauri::Window;

#[derive(Default)]
struct Runtime { pool: IOBufferPool }
impl create::CompressionRuntime for Runtime {
    fn check_cancellation(&self) -> Result<()> { Ok(()) }
    fn cancellation_flag(&self) -> Arc<AtomicBool> { Arc::new(AtomicBool::new(false)) }
    fn copy_buffer_size(&self) -> usize { 256 * 1024 }
    fn emit_log(&self, _: &Window, _: &str, _: &str, _: TaskLogSeverity) {}
    fn emit_progress(&self, _: &Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {}
}
impl extract::ExtractionRuntime for Runtime {
    fn check_cancellation(&self) -> Result<()> { Ok(()) }
    fn buffer_pool(&self) -> &IOBufferPool { &self.pool }
    fn copy_buffer_size(&self) -> usize { 256 * 1024 }
    fn normalized_archive_path(&self, path: &Path, _: bool) -> Option<PathBuf> { Some(path.to_path_buf()) }
    fn emit_log(&self, _: &Window, _: &str, _: &str, _: TaskLogSeverity) {}
    fn emit_progress(&self, _: &Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {}
}
fn pressure(volume: &Path) -> Result<PathBuf> {
    let free = super::storage_preflight::available_disk_space(volume).expect("free space must be observable");
    anyhow::ensure!(free < 512 * 1024 * 1024, "Refuse to fill an unbounded volume");
    let path = volume.join("io1516-owned-filler.bin");
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&path)?;
    let block = vec![37; 1024 * 1024];
    let mut left = free.saturating_sub(512 * 1024);
    while left > 0 {
        let len = (left as usize).min(block.len());
        file.write_all(&block[..len])?;
        left -= len as u64;
    }
    file.sync_all()?;
    Ok(path)
}
fn create(runtime: &Runtime, format: &str, sources: &[String], target: &Path) -> Result<()> {
    let options = CompressionOptions { level: 1, ..CompressionOptions::default() };
    let target = target.to_str().unwrap();
    match format {
        "zip" => create::zip::compress(runtime, None, "io1516", sources, target, options),
        "7z" => create::seven_zip::compress(runtime, None, "io1516", sources, target, options),
        _ => create::tar::compress_tar(runtime, None, "io1516", sources, target, options),
    }
}
fn retains_actual_storage_full(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        // These libraries own their io::Error without exposing it via source().
        let io = cause.downcast_ref::<std::io::Error>()
            .or_else(|| match cause.downcast_ref::<sevenz_rust::Error>() {
                Some(sevenz_rust::Error::Io(io, _)) => Some(io), _ => None,
            })
            .or_else(|| match cause.downcast_ref::<zip_aes::result::ZipError>() {
                Some(zip_aes::result::ZipError::Io(io)) => Some(io), _ => None,
            });
        io.is_some_and(|io| io.kind() == std::io::ErrorKind::StorageFull
            && io.raw_os_error() == Some(112))
    })
}
#[test]
#[ignore = "requires CI-owned 256 MiB NTFS VHD and IO1516_TEST_VOLUME"]
fn io1516_real_disk_full_native_routes_clean_up_and_recover() -> Result<()> {
    anyhow::ensure!(std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true"));
    let volume = PathBuf::from(std::env::var("IO1516_TEST_VOLUME")?);
    let runner = PathBuf::from(std::env::var("RUNNER_TEMP")?);
    anyhow::ensure!(volume.is_absolute() && volume.starts_with(&runner));
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("payload.bin");
    let mut random = 1516u32;
    let payload: Vec<u8> = (0..2 * 1024 * 1024).map(|_| {
        random ^= random << 13; random ^= random >> 17; random ^= random << 5; random as u8
    }).collect();
    std::fs::write(&input, &payload)?;
    let sources = vec![input.to_string_lossy().into_owned()];
    let sentinel = volume.join("existing-user-result.txt");
    std::fs::write(&sentinel, b"original result")?;
    let runtime = Runtime::default();
    let mut records = Vec::new();
    for format in ["zip", "7z", "tar"] {
        let target = volume.join(format!("recovered.{format}"));
        let staged = CompressionService::temporary_compression_output(&target)?;
        let filler = pressure(&volume)?;
        let result = create(&runtime, format, &sources, &staged);
        let error = CompressionService::finalize_compression_output(result, &staged, &target, false, false).unwrap_err();
        assert_eq!(archive_io::kind(&error).unwrap().0, "disk-full", "{error:#}");
        assert!(retains_actual_storage_full(&error), "OS cause must survive: {error:#}");
        assert!(!staged.exists() && !target.exists());
        assert_eq!(std::fs::read(&sentinel)?, b"original result");
        std::fs::remove_file(filler)?;
        let result = create(&runtime, format, &sources, &staged);
        CompressionService::finalize_compression_output(result, &staged, &target, false, false)?;
        let engine = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/archive-engine/7z.exe");
        let decoded = crate::utils::process::command(engine).args(["e", "-so"])
            .arg(&target).arg("payload.bin").output()?;
        anyhow::ensure!(decoded.status.success());
        assert_eq!(decoded.stdout, payload);
        records.push(serde_json::json!({"format":format,"actualOsStorageFull":true,"noPublishedPartial":true,
            "oldResultPreserved":true,"recoveredIndependentFullReadback":true,"failure":archive_io::message(&error)}));
    }
    let original_zip = temp.path().join("source.zip");
    std::fs::copy(volume.join("recovered.zip"), &original_zip)?;
    let output = volume.join("extract-output");
    let staged;
    let filler = pressure(&volume)?;
    {
        let staging = ExtractionStaging::create_for(&output)?;
        staged = staging.path().to_path_buf();
        let error = extract::zip::extract_with_optional_window(&runtime, None, "io1516", original_zip.to_str().unwrap(),
            staging.path().to_str().unwrap(), None, &DecompressOptions::default()).unwrap_err();
        assert_eq!(archive_io::kind(&error).unwrap().0, "disk-full", "{error:#}");
        assert!(retains_actual_storage_full(&error), "OS cause must survive: {error:#}");
    }
    assert!(!staged.exists() && !output.exists());
    std::fs::remove_file(filler)?;
    std::fs::create_dir(&output)?;
    extract::zip::extract_with_optional_window(&runtime, None, "io1516-recovered", original_zip.to_str().unwrap(),
        output.to_str().unwrap(), None, &DecompressOptions::default())?;
    assert_eq!(std::fs::read(output.join("payload.bin"))?, payload);
    records.push(serde_json::json!({"route":"zip-extraction","actualOsStorageFull":true,"stagingRemoved":true,"recoveredFullContent":true}));
    let receipt = PathBuf::from(std::env::var("IO1516_RECEIPT")?);
    std::fs::write(receipt, serde_json::to_vec_pretty(&records)?)?;
    Ok(())
}
