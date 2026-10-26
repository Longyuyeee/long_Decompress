use super::{archive_browser, archive_preview, codec_stream, compression_entries, compression_format, compression_verification};
use crate::models::compression::CompressionOptions;
use std::{path::Path, sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}}};
use sha2::{Digest, Sha256};

struct Runtime { checks: AtomicUsize, cancel_at: usize, cancelled: Arc<AtomicBool> }
impl super::native_compression::CompressionRuntime for Runtime {
    fn check_cancellation(&self) -> anyhow::Result<()> {
        if self.checks.fetch_add(1, Ordering::SeqCst) >= self.cancel_at { self.cancelled.store(true, Ordering::SeqCst); }
        if self.cancelled.load(Ordering::SeqCst) { return Err(super::compression_service::CompressionError::Cancelled.into()); }
        Ok(())
    }
    fn cancellation_flag(&self) -> Arc<AtomicBool> { self.cancelled.clone() }
    fn copy_buffer_size(&self) -> usize { 65536 }
    fn emit_log(&self, _: &tauri::Window, _: &str, _: &str, _: crate::models::compression::TaskLogSeverity) {}
    fn emit_progress(&self, _: &tauri::Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {}
}
fn runtime(cancel_at: usize) -> Runtime { Runtime { checks: AtomicUsize::new(0), cancel_at, cancelled: Arc::new(AtomicBool::new(false)) } }
fn options() -> CompressionOptions { CompressionOptions { format: Some("tar.lzo".into()), preserve_paths: Some(true), ..Default::default() } }
fn sha(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }
fn write(path: &Path, bytes: &[u8]) { std::fs::create_dir_all(path.parent().unwrap()).unwrap(); std::fs::write(path, bytes).unwrap(); }

#[tokio::test]
async fn tarlzo171_new_outputs_preserve_every_file_directory_and_preview() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/tar-lzo-create-171/generated");
    let source = root.join("source/资料目录");
    write(&source.join("说明.txt"), "目录与内容完整保留。\n".repeat(8000).as_bytes());
    write(&source.join("空文件.txt"), b"");
    write(&source.join(format!("深层/{}.txt", "长名称".repeat(25))), b"GNU long filename survives");
    let mut state = 171u64;
    let random: Vec<u8> = (0..700001).map(|_| { state ^= state << 13; state ^= state >> 7; state ^= state << 17; state as u8 }).collect();
    write(&source.join("random.bin"), &random);
    let image = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/aes-codecs-1521/原文件/图标.png")).unwrap();
    write(&source.join("图标.png"), &image);
    std::fs::create_dir_all(source.join("空目录/下一级")).unwrap();
    let extra = root.join("source/单独说明.md"); write(&extra, b"multiple sources are all included");
    let empty = root.join("source/只有空目录"); std::fs::create_dir_all(&empty).unwrap();
    let cases = [("directory", vec![source.clone()]), ("multiple", vec![source.clone(), extra]), ("empty-directory", vec![empty])];
    let mut proof = Vec::new();
    for (name, paths) in cases {
        let sources: Vec<String> = paths.iter().map(|path| path.to_string_lossy().into_owned()).collect();
        let archive = root.join(format!("{name}.tar.lzo"));
        assert_eq!(compression_format::validate_compression_request(&sources, &archive.to_string_lossy(), &options()).unwrap(), "tar.lzo");
        let entries = compression_entries::collect(&sources, true, true).unwrap();
        let expected: Vec<_> = entries.iter().map(|entry| serde_json::json!({"path": entry.archive_name.trim_end_matches('/'), "directory": entry.is_dir,
            "sha256": if entry.is_dir { String::new() } else { sha(&std::fs::read(&entry.path).unwrap()) }})).collect();
        codec_stream::compress(&runtime(usize::MAX), None, "171", &sources, &archive.to_string_lossy(), options(), "tar.lzo").unwrap();
        assert!(compression_verification::verify_native(compression_format::CompressionRoute::TarLzop, &archive, None, || false).unwrap());
        let list = archive_browser::browse_archive(&archive, None).await.unwrap();
        for entry in &expected { assert!(list.entries.iter().any(|actual| actual.path.trim_end_matches('/') == entry["path"].as_str().unwrap()), "missing {entry}"); }
        if name == "directory" {
            assert!(archive_preview::preview_archive_text(&archive, "资料目录/说明.txt", None).await.unwrap().content.contains("内容完整保留"));
            let preview = archive_preview::preview_archive_image(&archive, "资料目录/图标.png", None).await.unwrap(); assert_eq!((preview.width, preview.height), (32,32));
        }
        proof.push(serde_json::json!({"archive":archive,"archiveSha256":sha(&std::fs::read(&archive).unwrap()),"entries":expected}));
    }
    std::fs::write(root.join("manifest.json"), serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
}

#[test]
fn tarlzo171_fixed_analysis_and_target_options_preserve_sources() {
    let temp = tempfile::tempdir().unwrap(); let source = temp.path().join("notes.txt"); write(&source, &b"actual LZO encoder sample\n".repeat(50000));
    let sources = vec![source.to_string_lossy().into_owned()];
    for changed in [CompressionOptions { password:Some("unsupported".into()), ..options() }] {
        assert!(compression_format::validate_compression_request(&sources, "output.tar.lzo", &changed).is_err());
    }
    let a = super::compression_analysis::analyze_compression(&sources, "tar.lzo", 1, &AtomicBool::new(false)).unwrap();
    let b = super::compression_analysis::analyze_compression(&sources, "tar.lzo", 9, &AtomicBool::new(false)).unwrap();
    assert_eq!(a.estimated_size, b.estimated_size); assert_eq!(a.estimated_seconds_high, b.estimated_seconds_high); assert!(a.estimated_ratio < 0.2);
    assert_eq!(compression_format::infer_compression_format("output.tar.lzo", None), "tar.lzo");
}

#[test]
fn tarlzo171_cancel_and_output_failure_keep_sources_and_do_not_publish() {
    let temp = tempfile::tempdir().unwrap(); let source = temp.path().join("source.bin"); write(&source, &vec![42; 4 * 1024 * 1024]);
    let before = sha(&std::fs::read(&source).unwrap()); let sources = vec![source.to_string_lossy().into_owned()];
    let stage = tempfile::tempdir_in(temp.path()).unwrap(); let output = stage.path().join("pending.tar.lzo");
    let error = codec_stream::compress(&runtime(20), None, "cancel", &sources, &output.to_string_lossy(), options(), "tar.lzo").unwrap_err();
    assert!(matches!(error.downcast_ref::<super::compression_service::CompressionError>(), Some(super::compression_service::CompressionError::Cancelled)), "{error:?}");
    drop(stage); assert!(!output.exists());
    assert!(codec_stream::compress(&runtime(usize::MAX), None, "bad-output", &sources, &temp.path().to_string_lossy(), options(), "tar.lzo").is_err());
    assert_eq!(sha(&std::fs::read(&source).unwrap()), before);
    assert!(!temp.path().join("published.tar.lzo").exists());
}
