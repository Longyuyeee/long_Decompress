use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use long_compress_assistant::models::compression::{CompressionOptions, TaskLogSeverity};
use long_compress_assistant::services::archive_engine::ArchiveEngine;
use long_compress_assistant::services::compression_service::CompressionService;
use long_compress_assistant::services::universal_engine::UniversalCliEngine;
use tempfile::tempdir;

#[tokio::test]
async fn compress_extract_and_compare_real_file_contents() {
    let temp = tempdir().expect("temp dir");
    let source = temp.path().join("pipeline-source.txt");
    let archive = temp.path().join("pipeline.zip");
    let output = temp.path().join("output");
    let expected = b"real archive pipeline\nwith binary-safe content: \0\x01\x02";
    std::fs::write(&source, expected).expect("source fixture");

    CompressionService::for_testing()
        .compress_zip_enhanced(
            &[source.to_string_lossy().to_string()],
            archive.to_str().expect("archive path"),
            CompressionOptions::default(),
        )
        .await
        .expect("compress real ZIP");

    let progress = Arc::new(Mutex::new(Vec::new()));
    let recorded_progress = progress.clone();
    UniversalCliEngine::new()
        .extract_with_progress(
            &archive,
            &output,
            None,
            true,
            Arc::new(move |value| recorded_progress.lock().unwrap().push(value)),
            Arc::new(|_message: String, _severity: TaskLogSeverity| {}),
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .expect("extract real ZIP");

    assert_eq!(
        std::fs::read(output.join("pipeline-source.txt")).expect("extracted file"),
        expected
    );
    assert!(archive.metadata().expect("archive metadata").len() > 22);
}

#[tokio::test]
async fn real_pipeline_rejects_missing_sources_without_creating_an_archive() {
    let temp = tempdir().expect("temp dir");
    let missing = temp.path().join("missing.txt");
    let archive = temp.path().join("must-not-exist.zip");

    let result = CompressionService::for_testing()
        .compress_zip_enhanced(
            &[missing.to_string_lossy().to_string()],
            archive.to_str().expect("archive path"),
            CompressionOptions::default(),
        )
        .await;

    assert!(result.is_err());
    assert!(!archive.exists());
}
