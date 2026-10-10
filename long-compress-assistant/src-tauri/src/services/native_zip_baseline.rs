//! Explicit, reproducible engine measurements; not desktop/transaction timings.
use super::{compression_service::CompressionService, native_compression, native_extraction};
use crate::models::compression::{CompressionOptions, DecompressOptions};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::Path;
use std::time::Instant;

fn bounded_env(name: &str, default: usize, minimum: usize, maximum: usize) -> usize {
    std::env::var(name)
        .ok()
        .map(|value| value.parse::<usize>().expect("numeric workload parameter"))
        .unwrap_or(default)
        .clamp(minimum, maximum)
}

fn digest(path: &Path) -> String {
    let mut file = std::fs::File::open(path).unwrap();
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).unwrap();
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    format!("{:x}", hash.finalize())
}

fn random_bytes(state: &mut u32, buffer: &mut [u8]) {
    for byte in buffer {
        *state ^= *state << 13;
        *state ^= *state >> 17;
        *state ^= *state << 5;
        *byte = *state as u8;
    }
}

#[test]
fn headless_engine_preserves_filter_timestamp_and_cancellation() {
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("input.zip");
    let output = temp.path().join("output");
    let timestamp = zip::DateTime::from_date_and_time(2020, 5, 6, 12, 34, 56).unwrap();
    let mut writer = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
    for name in ["资料/保留.txt", "资料/排除.bin"] {
        writer
            .start_file(
                name,
                zip::write::FileOptions::default().last_modified_time(timestamp),
            )
            .unwrap();
        writer.write_all(b"real file contents").unwrap();
    }
    writer.finish().unwrap();
    let service = CompressionService::for_testing();
    let options = DecompressOptions {
        preserve_paths: true,
        preserve_timestamps: true,
        file_filter: Some("*.txt".into()),
        ..Default::default()
    };
    native_extraction::zip::extract_with_optional_window(
        &service,
        None,
        "test",
        archive.to_str().unwrap(),
        output.to_str().unwrap(),
        None,
        &options,
    )
    .unwrap();
    let kept = output.join("资料/保留.txt");
    assert_eq!(std::fs::read(&kept).unwrap(), b"real file contents");
    assert!(!output.join("资料/排除.bin").exists());
    assert_eq!(
        std::fs::metadata(kept).unwrap().modified().unwrap(),
        native_extraction::zip::system_time(2020, 5, 6, 12, 34, 56).unwrap()
    );

    service.cancel();
    let cancelled_output = temp.path().join("cancelled");
    let error = native_extraction::zip::extract_with_optional_window(
        &service,
        None,
        "cancelled",
        archive.to_str().unwrap(),
        cancelled_output.to_str().unwrap(),
        None,
        &options,
    )
    .unwrap_err();
    assert!(matches!(
        error.downcast_ref::<super::compression_service::CompressionError>(),
        Some(super::compression_service::CompressionError::Cancelled)
    ));
    assert!(!cancelled_output.exists());
}

#[test]
fn headless_engine_rejects_invalid_archive_without_creating_output() {
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("broken.zip");
    let output = temp.path().join("output");
    std::fs::write(&archive, b"not a ZIP archive").unwrap();
    let service = CompressionService::for_testing();
    assert!(native_extraction::zip::extract_with_optional_window(
        &service,
        None,
        "broken",
        archive.to_str().unwrap(),
        output.to_str().unwrap(),
        None,
        &DecompressOptions::default()
    )
    .is_err());
    assert!(!output.exists());
}

#[test]
#[ignore = "explicit native ZIP benchmark; no UI or transaction timings"]
fn native_zip_engine_baseline() {
    let large_mib = bounded_env("LONG_NATIVE_ZIP_MIB", 32, 16, 2048);
    let small_count = bounded_env("LONG_NATIVE_ZIP_FILES", 1000, 100, 50000);
    let root = std::env::var_os("LONG_NATIVE_ZIP_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    assert!(root.is_dir(), "benchmark root must already exist");
    for scenario in ["large", "small", "mixed_media"] {
        let temp = tempfile::Builder::new()
            .prefix("long-native-zip-")
            .tempdir_in(&root)
            .unwrap();
        let source = temp.path().join("source");
        let output = temp.path().join("output");
        let archive = temp.path().join("fixture.zip");
        std::fs::create_dir(&source).unwrap();
        let mut state = 0x6D2B_79F5u32;
        match scenario {
            "large" => {
                let mut file = std::fs::File::create(source.join("large.bin")).unwrap();
                let mut buffer = vec![0u8; 1024 * 1024];
                for _ in 0..large_mib {
                    random_bytes(&mut state, &mut buffer);
                    file.write_all(&buffer).unwrap();
                }
                file.flush().unwrap();
            }
            "small" => {
                let mut buffer = [0u8; 4096];
                for index in 0..small_count {
                    random_bytes(&mut state, &mut buffer);
                    std::fs::write(source.join(format!("小文件-{index:05}.bin")), buffer).unwrap();
                }
            }
            _ => {
                // Real encoded PNG/JPEG and the repository's small H.265 fixture,
                // plus compressible text. No personal media or fake extensions.
                let pixels = image::RgbImage::from_fn(512, 512, |_, _| {
                    let mut rgb = [0u8; 3];
                    random_bytes(&mut state, &mut rgb);
                    image::Rgb(rgb)
                });
                pixels.save(source.join("图片.png")).unwrap();
                pixels.save(source.join("照片.jpg")).unwrap();
                let video = Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../tests/fixtures/media/videos/h265.mp4");
                assert!(video.is_file(), "missing tracked video fixture");
                std::fs::copy(video, source.join("视频.mp4")).unwrap();
                for index in 0..32 {
                    std::fs::write(
                        source.join(format!("说明-{index:02}.txt")),
                        "这是项目说明，不应遗漏输入文件。\n".repeat(1024),
                    )
                    .unwrap();
                }
            }
        }
        let mut inputs: Vec<_> = std::fs::read_dir(&source)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        inputs.sort();
        let manifest: Vec<_> = inputs
            .iter()
            .map(|path| {
                serde_json::json!({
                    "name": path.file_name().unwrap().to_str().unwrap(),
                    "bytes": std::fs::metadata(path).unwrap().len(),
                    "sha256": digest(path),
                })
            })
            .collect();
        let fixture_hash = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&manifest).unwrap())
        );
        let input_bytes: u64 = inputs
            .iter()
            .map(|path| std::fs::metadata(path).unwrap().len())
            .sum();
        let sources: Vec<_> = inputs
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect();
        let service = CompressionService::for_testing();
        let compression_started = Instant::now();
        native_compression::zip::compress(
            &service,
            None,
            "native-zip-baseline",
            &sources,
            archive.to_str().unwrap(),
            CompressionOptions {
                level: 6,
                preserve_paths: Some(false),
                ..Default::default()
            },
        )
        .expect("product ZIP compression");
        let compression_ms = compression_started.elapsed().as_secs_f64() * 1000.0;
        let extraction_started = Instant::now();
        native_extraction::zip::extract_with_optional_window(
            &service,
            None,
            "native-zip-baseline",
            archive.to_str().unwrap(),
            output.to_str().unwrap(),
            None,
            &DecompressOptions::default(),
        )
        .expect("product ZIP extraction");
        let extraction_ms = extraction_started.elapsed().as_secs_f64() * 1000.0;
        // Verify actual files after stopping the timer, not just bytes read from ZIP.
        assert_eq!(
            walkdir::WalkDir::new(&output)
                .into_iter()
                .map(Result::unwrap)
                .filter(|entry| entry.file_type().is_file())
                .count(),
            inputs.len()
        );
        for entry in &manifest {
            let extracted = output.join(entry["name"].as_str().unwrap());
            assert_eq!(
                std::fs::metadata(&extracted).unwrap().len(),
                entry["bytes"].as_u64().unwrap()
            );
            assert_eq!(digest(&extracted), entry["sha256"].as_str().unwrap());
        }
        println!(
            "NATIVE_ZIP_JSON {}",
            serde_json::json!({
                "schema_version": 1, "scenario": scenario,
                "scope": "native_zip_engine_without_window_or_transaction",
                "compression": { "method": "deflate", "level": 6, "encrypted": false, "preserve_paths": false },
                "fixture_sha256": fixture_hash, "files": manifest,
                "input_bytes": input_bytes, "file_count": inputs.len(),
                "archive_bytes": std::fs::metadata(archive).unwrap().len(),
                "compression_ms": compression_ms, "extraction_ms": extraction_ms,
                "verified": "output_file_count_size_sha256",
            })
        );
    }
}
