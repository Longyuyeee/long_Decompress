use super::{
    archive_browser, archive_diagnostics, archive_format::ArchiveFormat, archive_preview,
    compression_format, lzop, universal_engine::UniversalCliEngine,
};
use sha2::{Digest, Sha256};
use std::{
    io::{Cursor, Read},
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
};
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/lzop-read-1657")
}
fn text() -> Vec<u8> {
    std::fs::read(fixtures().join("../aes-codecs-1521/原文件/说明.txt")).unwrap()
}
fn decode(bytes: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut out = Vec::new();
    lzop::LzopReader::new(Cursor::new(bytes))?.read_to_end(&mut out)?;
    Ok(out)
}
fn header_end(b: &[u8]) -> usize {
    38 + b[33] as usize
}
fn repair_header(b: &mut [u8]) {
    let end = header_end(b);
    let crc = u32::from_be_bytes(b[17..21].try_into().unwrap()) & 0x1000 != 0;
    let value = if crc {
        crc32fast::hash(&b[9..end - 4])
    } else {
        let (mut a, mut c) = (1u32, 0u32);
        for &x in &b[9..end - 4] {
            a = (a + x as u32) % 65521;
            c = (c + a) % 65521;
        }
        c << 16 | a
    };
    b[end - 4..end].copy_from_slice(&value.to_be_bytes());
}

#[test]
fn lzop1657_official_methods_checksums_stored_multiblock_and_short_reads() {
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixtures().join("manifest.json")).unwrap()).unwrap();
    for case in manifest["cases"].as_array().unwrap() {
        let p = fixtures().join(case["file"].as_str().unwrap());
        let packed = std::fs::read(&p).unwrap();
        let plain = decode(&packed).unwrap();
        assert_eq!(plain.len() as u64, case["size"].as_u64().unwrap());
        assert_eq!(
            format!("{:x}", Sha256::digest(&plain)),
            case["sha256"].as_str().unwrap()
        );
        assert_eq!(ArchiveFormat::from_magic(&packed), ArchiveFormat::Lzop);
        let mut short = lzop::LzopReader::new(Cursor::new(packed)).unwrap();
        let mut actual = Vec::new();
        let mut bytes = [0; 17];
        while let n @ 1..=17 = short.read(&mut bytes).unwrap() {
            actual.extend_from_slice(&bytes[..n]);
        }
        assert_eq!(actual, plain);
        assert_eq!(short.read(&mut bytes).unwrap(), 0);
        lzop::verify(&p, || false).unwrap();
        assert!(lzop::verify(&p, || true).is_err());
    }
    for (name, method) in [
        ("method1.txt.lzo", 1),
        ("method2.txt.lzo", 2),
        ("method3.txt.lzo", 3),
    ] {
        assert_eq!(std::fs::read(fixtures().join(name)).unwrap()[15], method);
    }
    let stored = std::fs::read(fixtures().join("stored.bin.lzo")).unwrap();
    let start = header_end(&stored);
    assert_eq!(&stored[start..start + 4], &stored[start + 4..start + 8]);
}

#[test]
fn lzop1657_damage_unsupported_limits_and_numeric_slices_remain_explicit() {
    let original = std::fs::read(fixtures().join("crc.txt.lzo")).unwrap();
    let start = header_end(&original);
    for pos in [
        22,
        start + 8,
        start + 12,
        start + 16,
        start + 24,
        original.len() - 5,
    ] {
        let mut bad = original.clone();
        bad[pos] ^= 0x80;
        let error = decode(&bad).unwrap_err().to_string();
        assert!(
            error.contains("[archive-inspection:damaged]"),
            "{pos}: {error}"
        );
    }
    for cut in [0, 8, 20, start - 1, start + 2, original.len() - 1] {
        assert!(decode(&original[..cut])
            .unwrap_err()
            .to_string()
            .contains("damaged"));
    }
    let mut trailing = original.clone();
    trailing.push(42);
    assert!(decode(&trailing)
        .unwrap_err()
        .to_string()
        .contains("damaged"));
    let mut members = original.clone();
    members.extend_from_slice(&original);
    assert!(decode(&members)
        .unwrap_err()
        .to_string()
        .contains("多个成员"));
    for (pos, value) in [(15, 4), (16, 10), (18, 0x01), (19, 0x04)] {
        let mut bad = original.clone();
        bad[pos] = value;
        repair_header(&mut bad);
        assert!(
            decode(&bad)
                .unwrap_err()
                .to_string()
                .contains("unsupported"),
            "{pos}"
        );
    }
    for size in [0xffffffffu32, 64 * 1024 * 1024 + 1] {
        let mut bad = original.clone();
        bad[start..start + 4].copy_from_slice(&size.to_be_bytes());
        assert!(decode(&bad).is_err());
    }
    let mut bad = original.clone();
    bad[start + 8] ^= 1;
    let mut reader = lzop::LzopReader::new(Cursor::new(bad)).unwrap();
    let mut bytes = [0; 1];
    let first = reader.read(&mut bytes).unwrap_err().to_string();
    assert_eq!(reader.read(&mut bytes).unwrap_err().to_string(), first);
    let temp = tempfile::tempdir().unwrap();
    let part = temp.path().join("file.lzo.001");
    std::fs::write(&part, original).unwrap();
    assert!(lzop::open(&part)
        .err()
        .unwrap()
        .to_string()
        .contains("先合并"));
}

#[tokio::test]
async fn lzop1657_browse_preview_integrity_and_large_size_boundary() {
    let image = archive_preview::preview_archive_image(
        &fixtures().join("picture.png.lzo"),
        "picture.png",
        None,
    )
    .await
    .unwrap();
    assert!(image.width > 0 && image.height > 0);
    for name in [
        "method1.txt.lzo",
        "method2.txt.lzo",
        "method3.txt.lzo",
        "empty.txt.lzo",
        "directory.tar.lzo",
    ] {
        let p = fixtures().join(name);
        let result = archive_browser::browse_archive(&p, None).await.unwrap();
        assert_eq!(result.format, "LZOP");
        assert_eq!(result.entries.len(), 1);
        let entry = &result.entries[0];
        assert_eq!(entry.path, name.trim_end_matches(".lzo"));
        assert_eq!(entry.size_known, Some(true));
        assert!(!entry.encrypted);
        if name.starts_with("method") {
            let preview = archive_preview::preview_archive_text(&p, &entry.path, None)
                .await
                .unwrap();
            assert_eq!(
                preview.content,
                String::from_utf8(text()).unwrap().replace("\r\n", "\n")
            );
            assert!(!preview.truncated);
        }
        UniversalCliEngine::test_integrity_with_zip_encoding(&p, None, None)
            .await
            .unwrap();
    }
    let p = fixtures().join("large.txt.lzo");
    let result = archive_browser::browse_archive(&p, None).await.unwrap();
    assert_eq!(result.entries[0].size_known, Some(false));
    let preview = archive_preview::preview_archive_text(&p, "large.txt", None)
        .await
        .unwrap();
    assert!(preview.truncated);
    assert_eq!(preview.content.len(), 1024 * 1024);
    let diagnosis = archive_diagnostics::diagnose_archive(
        &fixtures().join("crc.txt.lzo"),
        None,
        Arc::new(AtomicBool::new(false)),
    )
    .await
    .unwrap();
    assert_eq!(diagnosis.actual_format, "LZOP");
    assert_eq!(diagnosis.status, "healthy");
    assert!(diagnosis.integrity_tested);
    assert!(!diagnosis.can_repair);
    let temp = tempfile::tempdir().unwrap();
    let renamed = temp.path().join("renamed.zip");
    std::fs::copy(fixtures().join("method1.txt.lzo"), &renamed).unwrap();
    assert_eq!(
        archive_browser::browse_archive(&renamed, None)
            .await
            .unwrap()
            .format,
        "LZOP"
    );
    let bad = temp.path().join("damaged.lzo");
    let mut bytes = std::fs::read(fixtures().join("crc.txt.lzo")).unwrap();
    bytes.pop();
    std::fs::write(&bad, bytes).unwrap();
    let report =
        archive_diagnostics::diagnose_archive(&bad, None, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap();
    assert_ne!(report.status, "healthy");
    assert!(
        UniversalCliEngine::test_integrity_with_zip_encoding(&bad, None, None)
            .await
            .is_err()
    );
}

#[test]
fn lzop1657_single_stream_selection_and_file_filter_are_respected() {
    use super::{
        io_buffer_pool::IOBufferPool,
        native_extraction::{single_stream, ExtractionRuntime},
    };
    use crate::models::compression::DecompressOptions;
    struct Runtime(IOBufferPool);
    impl ExtractionRuntime for Runtime {
        fn check_cancellation(&self) -> anyhow::Result<()> {
            Ok(())
        }
        fn buffer_pool(&self) -> &IOBufferPool {
            &self.0
        }
        fn copy_buffer_size(&self) -> usize {
            64 * 1024
        }
        fn normalized_archive_path(&self, p: &Path, _: bool) -> Option<PathBuf> {
            Some(p.to_path_buf())
        }
        fn emit_log(
            &self,
            _: &tauri::Window,
            _: &str,
            _: &str,
            _: crate::models::compression::TaskLogSeverity,
        ) {
        }
        fn emit_progress(
            &self,
            _: &tauri::Window,
            _: &str,
            _: f32,
            _: Option<String>,
            _: u64,
            _: u64,
        ) {
        }
    }
    let service = Runtime(IOBufferPool::new(Default::default()));
    let temp = tempfile::tempdir().unwrap();
    for (selected, filter, expected) in [
        ("method1.txt", None, true),
        ("unrelated.txt", None, false),
        ("method1.txt", Some("*.png"), false),
    ] {
        let output = temp
            .path()
            .join(format!("{selected}-{expected}-{}", filter.is_some()));
        let options = DecompressOptions {
            selected_entries: vec![selected.into()],
            file_filter: filter.map(str::to_owned),
            ..Default::default()
        };
        single_stream::extract_inner(
            &service,
            None,
            "lzop1657",
            lzop::open(&fixtures().join("method1.txt.lzo")).unwrap(),
            &output,
            "method1.txt".into(),
            &options,
        )
        .unwrap();
        assert_eq!(output.join("method1.txt").exists(), expected);
        if expected {
            assert_eq!(std::fs::read(output.join("method1.txt")).unwrap(), text());
        }
    }
}

#[test]
fn lzop1657_read_only_capability_never_falls_back_to_another_creation_codec() {
    let cap = compression_format::COMPRESSION_FORMAT_CAPABILITIES
        .iter()
        .find(|c| c.format == "lzo")
        .unwrap();
    assert!(cap.can_extract);
    assert!(!cap.can_compress);
    assert!(!cap.supports_split);
    assert!(!cap.requires_7za);
    assert!(!cap.supports_password_extract);
    let options = crate::models::compression::CompressionOptions {
        format: Some("lzo".into()),
        ..Default::default()
    };
    assert!(compression_format::validate_compression_request(
        &[fixtures()
            .join("method1.txt.lzo")
            .to_string_lossy()
            .into_owned()],
        "out.lzo",
        &options
    )
    .is_err());
    assert!(compression_format::compression_route("lzo").is_none());
}
