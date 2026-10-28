use super::{
    archive_browser, archive_preview, codec_stream, compression_entries, compression_format,
    compression_verification, lzop_writer, output_publish_transaction as publication,
    split_compression::SplitCompressionService as Split,
};
use crate::models::compression::CompressionOptions;
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

struct Runtime(Arc<AtomicBool>);
impl super::native_compression::CompressionRuntime for Runtime {
    fn check_cancellation(&self) -> anyhow::Result<()> {
        anyhow::ensure!(!self.0.load(Ordering::SeqCst), "操作已取消");
        Ok(())
    }
    fn cancellation_flag(&self) -> Arc<AtomicBool> {
        self.0.clone()
    }
    fn copy_buffer_size(&self) -> usize {
        65536
    }
    fn emit_log(
        &self,
        _: &tauri::Window,
        _: &str,
        _: &str,
        _: crate::models::compression::TaskLogSeverity,
    ) {
    }
    fn emit_progress(&self, _: &tauri::Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {
    }
}
fn runtime() -> Runtime {
    Runtime(Arc::new(AtomicBool::new(false)))
}
fn sha(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
fn write(path: &Path, data: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, data).unwrap();
}
fn options(format: &str, split: Option<u64>) -> CompressionOptions {
    CompressionOptions {
        format: Some(format.into()),
        split_size: split,
        preserve_paths: Some(true),
        ..Default::default()
    }
}
fn create(sources: &[String], output: &Path, format: &str) {
    if format == "lzo" {
        lzop_writer::compress(
            &runtime(),
            None,
            "173",
            sources,
            &output.to_string_lossy(),
            options(format, None),
        )
        .unwrap();
    } else {
        codec_stream::compress(
            &runtime(),
            None,
            "173",
            sources,
            &output.to_string_lossy(),
            options(format, None),
            format,
        )
        .unwrap();
    }
}
fn parts(path: &Path) -> Vec<PathBuf> {
    super::archive_volumes::complete_volume_paths(path).unwrap()
}

#[tokio::test]
async fn lzop173_fresh_numeric_groups_publish_and_open_from_any_volume_with_all_content() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test-results/lzop-numeric-create-173/generated");
    let root = base.join(uuid::Uuid::new_v4().to_string());
    let folder = root.join("source/资料目录");
    write(
        &folder.join("说明.txt"),
        "跨卷内容完整保留。\n".repeat(9000).as_bytes(),
    );
    write(&folder.join("空文件.txt"), b"");
    let mut state = 173u64;
    let random: Vec<u8> = (0..2_300_173)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect();
    write(&folder.join("random.bin"), &random);
    let image = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/archives/aes-codecs-1521/原文件/图标.png"),
    )
    .unwrap();
    write(&folder.join("图标.png"), &image);
    std::fs::create_dir_all(folder.join("空目录/下一层")).unwrap();
    let duplicate = root.join("source/extra/说明.txt");
    write(&duplicate, b"second same basename");
    let cases = [
        ("single", "lzo", vec![folder.join("说明.txt")]),
        (
            "multiple",
            "lzo",
            vec![
                folder.join("说明.txt"),
                duplicate,
                folder.join("random.bin"),
                folder.join("图标.png"),
                folder.join("空文件.txt"),
            ],
        ),
        ("directory", "tar.lzo", vec![folder.clone()]),
        ("empty-header", "lzo", vec![folder.join("空文件.txt")]),
    ];
    let mut proof = Vec::new();
    for (case, format, inputs) in cases {
        let sources: Vec<String> = inputs
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        let target = root.join(format!("{case}.{format}"));
        let staged = publication::staged_output_path(&target, "numeric173").unwrap();
        let size = if case == "empty-header" { 31 } else { 65536 };
        assert_eq!(
            compression_format::validate_compression_request(
                &sources,
                &target.to_string_lossy(),
                &options(format, Some(size))
            )
            .unwrap(),
            format
        );
        create(&sources, &staged, format);
        let stream = std::fs::read(&staged).unwrap();
        let result = Split::split_archive_file(&staged, &staged, size, || Ok(())).unwrap();
        assert_eq!(result.part_count, stream.len().div_ceil(size as usize));
        assert!(result
            .part_files
            .iter()
            .all(|p| p.metadata().unwrap().len() > 0 && p.metadata().unwrap().len() <= size));
        std::fs::remove_file(&staged).unwrap();
        let stage_first = super::archive_volumes::first_volume_path(&staged);
        let route = compression_format::compression_route(format).unwrap();
        assert!(
            compression_verification::verify_native(route, &stage_first, None, || false).unwrap()
        );
        publication::publish_verified_volumes(&staged, &target, || false).unwrap();
        assert!(!target.exists());
        let volumes = parts(&target);
        assert_eq!(volumes.len(), result.part_count);
        let combined: Vec<u8> = volumes
            .iter()
            .flat_map(|p| std::fs::read(p).unwrap())
            .collect();
        assert_eq!(combined, stream);
        let expected_entries = if format == "lzo" {
            lzop_writer::prepare_members(&sources).unwrap()
        } else {
            compression_entries::collect_rooted(&sources, true).unwrap()
        };
        let expected: Vec<_> = expected_entries.iter().map(|e|serde_json::json!({"path":e.archive_name.trim_end_matches('/'),"directory":e.is_dir,"bytes":if e.is_dir {0} else {e.path.metadata().unwrap().len()},"sha256":if e.is_dir {String::new()} else {sha(&std::fs::read(&e.path).unwrap())}})).collect();
        for volume in [
            &volumes[0],
            &volumes[volumes.len() / 2],
            volumes.last().unwrap(),
        ] {
            let list = archive_browser::browse_archive(volume, None).await.unwrap();
            for entry in &expected {
                assert!(list
                    .entries
                    .iter()
                    .any(|e| e.path.trim_end_matches('/') == entry["path"].as_str().unwrap()));
            }
        }
        let last = volumes.last().unwrap();
        if case != "empty-header" {
            let text = if format == "lzo" {
                "说明.txt"
            } else {
                "资料目录/说明.txt"
            };
            assert!(archive_preview::preview_archive_text(last, text, None)
                .await
                .unwrap()
                .content
                .contains("跨卷内容"));
        }
        if case == "multiple" || case == "directory" {
            let name = if format == "lzo" {
                "图标.png"
            } else {
                "资料目录/图标.png"
            };
            assert_eq!(
                archive_preview::preview_archive_image(last, name, None)
                    .await
                    .unwrap()
                    .width,
                32
            );
        }
        proof.push(serde_json::json!({"case":case,"format":format,"archive":target,"parts":volumes,"splitSize":size,"streamSha256":sha(&stream),"entries":expected}));
    }
    std::fs::write(
        base.join("manifest.json"),
        serde_json::to_vec_pretty(&proof).unwrap(),
    )
    .unwrap();
}

#[test]
fn lzop173_exact_and_undersized_volumes_have_no_empty_tail_and_cut_cancel_keeps_sources() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("empty.txt");
    write(&source, b"");
    let whole = temp.path().join("whole.lzo");
    create(&[source.to_string_lossy().into_owned()], &whole, "lzo");
    let bytes = std::fs::read(&whole).unwrap();
    for (case, size) in [
        ("exact", bytes.len() as u64),
        ("larger", bytes.len() as u64 + 1),
    ] {
        let out = temp.path().join(format!("{case}.lzo"));
        let result = Split::split_archive_file(&whole, &out, size, || Ok(())).unwrap();
        assert_eq!(result.part_count, 1);
        assert_eq!(std::fs::read(&result.part_files[0]).unwrap(), bytes);
        assert!(!temp.path().join(format!("{case}.lzo.002")).exists());
    }
    let big = temp.path().join("big.lzo");
    write(&big, &vec![173; 200000]);
    let cancelled = AtomicBool::new(false);
    let cut = temp.path().join("cut.lzo");
    let error = Split::split_archive_file_with_progress(
        &big,
        &cut,
        65536,
        || {
            anyhow::ensure!(!cancelled.load(Ordering::SeqCst), "操作已取消");
            Ok(())
        },
        |processed, _| {
            if processed > 0 {
                cancelled.store(true, Ordering::SeqCst)
            }
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("取消"));
    assert!(!temp.path().join("cut.lzo.001").exists());
    assert_eq!(std::fs::metadata(&big).unwrap().len(), 200000);
    assert!(Split::split_archive_file(&big, &cut, 65536, || Ok(())).is_ok());
}

#[test]
fn lzop173_incomplete_new_groups_do_not_pass_verification_and_existing_targets_remain() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.bin");
    let mut state = 173u64;
    let data: Vec<u8> = (0..180000)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect();
    write(&source, &data);
    let whole = temp.path().join("whole.lzo");
    create(&[source.to_string_lossy().into_owned()], &whole, "lzo");
    for missing in [1, 2] {
        let out = temp.path().join(format!("missing-{missing}.lzo"));
        let result = Split::split_archive_file(&whole, &out, 65536, || Ok(())).unwrap();
        assert_eq!(result.part_count, 3);
        std::fs::remove_file(&result.part_files[missing]).unwrap();
        assert!(super::lzop::verify(&result.part_files[0], || false).is_err());
    }
    let target = temp.path().join("keep.lzo");
    let first = super::archive_volumes::first_volume_path(&target);
    write(&first, b"existing user output");
    let stage = publication::staged_output_path(&target, "conflict173").unwrap();
    std::fs::copy(&whole, &stage).unwrap();
    Split::split_archive_file(&stage, &stage, 65536, || Ok(())).unwrap();
    std::fs::remove_file(&stage).unwrap();
    assert!(publication::publish_verified_volumes(&stage, &target, || false).is_err());
    assert_eq!(std::fs::read(&first).unwrap(), b"existing user output");
    assert_eq!(std::fs::read(&source).unwrap(), data);
}
