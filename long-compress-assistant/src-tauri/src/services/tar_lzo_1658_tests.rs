use super::{
    archive_browser, archive_diagnostics, archive_format::ArchiveFormat, archive_preview,
    codec_stream, compression_format, compression_verification, tar_compatibility,
    universal_engine::UniversalCliEngine,
};
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
};
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/tar-lzo-read-1658")
}
fn manifest() -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()
}
fn sha(p: &Path) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(p).unwrap()))
}

#[test]
fn tarlzo1658_tar_parse_errors_are_structural_without_masking_storage_errors() {
    use std::io::{Error, ErrorKind};
    for message in [
        "archive header checksum mismatch",
        "failed to read entire block",
        "unexpected EOF during skip",
    ] {
        let error =
            tar_compatibility::inspection_error(Error::new(ErrorKind::Other, message).into());
        assert_eq!(
            error.downcast_ref::<Error>().unwrap().kind(),
            ErrorKind::InvalidData
        );
        assert!(error
            .to_string()
            .contains("[archive-inspection:damaged] TAR"));
        assert!(error.to_string().contains(message));
    }
    for kind in [
        ErrorKind::PermissionDenied,
        ErrorKind::NotFound,
        ErrorKind::StorageFull,
        ErrorKind::Other,
    ] {
        let error =
            tar_compatibility::inspection_error(Error::new(kind, "original device failure").into());
        assert_eq!(error.downcast_ref::<Error>().unwrap().kind(), kind);
        assert!(!error.to_string().contains("[archive-inspection:damaged]"));
    }
}
struct Runtime(super::io_buffer_pool::IOBufferPool);
impl super::native_extraction::ExtractionRuntime for Runtime {
    fn check_cancellation(&self) -> anyhow::Result<()> {
        Ok(())
    }
    fn buffer_pool(&self) -> &super::io_buffer_pool::IOBufferPool {
        &self.0
    }
    fn copy_buffer_size(&self) -> usize {
        64 * 1024
    }
    fn normalized_archive_path(&self, p: &Path, _: bool) -> Option<PathBuf> {
        Some(
            p.components()
                .filter(|c| matches!(c, std::path::Component::Normal(_)))
                .collect(),
        )
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
#[tokio::test]
async fn tarlzo1658_independent_originals_browse_verify_and_restore_every_file() {
    let m = manifest();
    let runtime = Runtime(super::io_buffer_pool::IOBufferPool::new(Default::default()));
    for case in m["cases"].as_array().unwrap() {
        let name = case["archive"].as_str().unwrap();
        let p = root().join(name);
        assert_eq!(sha(&p), case["archiveSha256"].as_str().unwrap());
        let list = archive_browser::browse_archive(&p, None).await.unwrap();
        assert_eq!(list.format, "TAR.LZO");
        let expected = case["expected"].as_array().unwrap();
        assert_eq!(list.total_files, expected.len());
        assert_eq!(
            list.total_uncompressed_size,
            expected
                .iter()
                .map(|e| e["bytes"].as_u64().unwrap())
                .sum::<u64>()
        );
        UniversalCliEngine::test_integrity(&p, None).await.unwrap();
        let mut raw = Vec::new();
        codec_stream::decoder(&p, "lzo")
            .unwrap()
            .read_to_end(&mut raw)
            .unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&raw)),
            case["tarSha256"].as_str().unwrap()
        );
        let temp = tempfile::tempdir().unwrap();
        super::native_extraction::tar::extract_inner(
            &runtime,
            None,
            "tarlzo1658",
            p.to_str().unwrap(),
            temp.path(),
            Some(codec_stream::decoder(&p, "lzo").unwrap()),
            &Default::default(),
        )
        .unwrap();
        for e in expected {
            let entry = e["path"].as_str().unwrap();
            assert!(
                list.entries
                    .iter()
                    .any(|f| f.path.trim_start_matches("./").trim_end_matches('/') == entry),
                "{name}: {entry}"
            );
            assert_eq!(sha(&temp.path().join(entry)), e["sha256"].as_str().unwrap());
        }
        if name.starts_with("directory-") {
            assert!(temp.path().join("empty-dir").is_dir());
            let bytes = std::fs::read(&p).unwrap();
            let start = 38 + bytes[33] as usize;
            assert!(
                u32::from_be_bytes(bytes[start + 4..start + 8].try_into().unwrap())
                    < u32::from_be_bytes(bytes[start..start + 4].try_into().unwrap())
            );
        }
    }
}
#[tokio::test]
async fn tarlzo1658_text_image_prefix_diagnostics_and_selection() {
    let p = root().join("directory-1.tar.lzo");
    let preview = archive_preview::preview_archive_text(&p, "notes/说明 [1].txt", None)
        .await
        .unwrap();
    assert!(preview.content.contains("收到 TAR.LZO"));
    assert!(!preview.truncated);
    let prefix = archive_preview::preview_archive_text(&p, "notes/large.txt", None)
        .await
        .unwrap();
    assert!(prefix.truncated);
    assert_eq!(prefix.content.len(), 1024 * 1024);
    let image = archive_preview::preview_archive_image(&p, "gallery/cover.png", None)
        .await
        .unwrap();
    assert_eq!((image.width, image.height), (32, 32));
    let d = archive_diagnostics::diagnose_archive(&p, None, Arc::new(AtomicBool::new(false)))
        .await
        .unwrap();
    assert_eq!(d.actual_format, "TAR.LZO");
    assert_eq!(d.status, "healthy");
    assert!(d.integrity_tested);
    assert!(!d.can_repair);
    let runtime = Runtime(super::io_buffer_pool::IOBufferPool::new(Default::default()));
    for (selection, filter) in [("notes/说明 [1].txt", None), ("notes", Some("*.txt"))] {
        let temp = tempfile::tempdir().unwrap();
        let options = crate::models::compression::DecompressOptions {
            selected_entries: vec![selection.into()],
            file_filter: filter.map(str::to_owned),
            ..Default::default()
        };
        super::native_extraction::tar::extract_inner(
            &runtime,
            None,
            "tarlzo1658",
            p.to_str().unwrap(),
            temp.path(),
            Some(codec_stream::decoder(&p, "lzo").unwrap()),
            &options,
        )
        .unwrap();
        assert!(temp.path().join("notes/说明 [1].txt").is_file());
        assert!(!temp.path().join("gallery").exists());
        assert!(!temp.path().join("data.bin").exists());
        assert_eq!(
            temp.path().join("notes/large.txt").exists(),
            filter.is_some()
        );
    }
}
#[tokio::test]
async fn tarlzo1658_magic_and_suffix_both_required_and_creation_stays_disabled() {
    let cap = compression_format::find_compression_format_capability("tar.lzo").unwrap();
    assert!(cap.can_extract);
    assert!(!cap.can_compress);
    assert!(!cap.supports_split);
    assert!(!cap.supports_password_extract);
    assert!(!cap.requires_7za);
    assert!(!cap.single_file_only);
    assert!(compression_format::compression_route("tar.lzo").is_none());
    let options = crate::models::compression::CompressionOptions {
        format: Some("tar.lzo".into()),
        ..Default::default()
    };
    assert!(
        compression_format::validate_compression_request(&[], "output.tar.lzo", &options).is_err()
    );
    assert!(tar_compatibility::is_tar_lzo(
        Path::new("BACKUP.TAR.LZO"),
        &ArchiveFormat::Lzop
    ));
    assert!(!tar_compatibility::is_tar_lzo(
        Path::new("backup.lzo"),
        &ArchiveFormat::Lzop
    ));
    assert!(!tar_compatibility::is_tar_lzo(
        Path::new("backup.tar.lzo"),
        &ArchiveFormat::Gzip
    ));
    let temp = tempfile::tempdir().unwrap();
    let ordinary = temp.path().join("backup.lzo");
    std::fs::copy(root().join("directory-1.tar.lzo"), &ordinary).unwrap();
    let list = archive_browser::browse_archive(&ordinary, None)
        .await
        .unwrap();
    assert_eq!(list.format, "LZOP");
    assert_eq!(list.entries.len(), 1);
    assert_eq!(list.entries[0].path, "backup");
    let raw = temp.path().join("raw.tar.lzo");
    let mut bytes = Vec::new();
    codec_stream::decoder(&root().join("directory-1.tar.lzo"), "lzo")
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    std::fs::write(&raw, bytes).unwrap();
    assert_eq!(
        archive_browser::browse_archive(&raw, None)
            .await
            .unwrap()
            .format,
        "TAR"
    );
    UniversalCliEngine::test_integrity(&raw, None)
        .await
        .unwrap();
}
#[tokio::test]
async fn tarlzo1658_invalid_inner_and_damaged_outer_never_report_healthy() {
    for case in manifest()["rejected"].as_array().unwrap() {
        let p = root().join(case["archive"].as_str().unwrap());
        assert!(archive_browser::browse_archive(&p, None).await.is_err());
        assert!(UniversalCliEngine::test_integrity(&p, None).await.is_err());
        let d = archive_diagnostics::diagnose_archive(&p, None, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap();
        assert_ne!(d.status, "healthy");
        assert!(
            matches!(
                d.status.as_str(),
                "damaged" | "unsupported_feature" | "crc_error" | "truncated"
            ),
            "{}: {}",
            p.display(),
            d.status
        );
    }
    let temp = tempfile::tempdir().unwrap();
    let original = std::fs::read(root().join("directory-1.tar.lzo")).unwrap();
    for name in ["short.tar.lzo", "checksum.tar.lzo"] {
        let mut bytes = original.clone();
        if name.starts_with("short") {
            bytes.pop();
        } else {
            let start = 38 + bytes[33] as usize;
            bytes[start + 8] ^= 1;
        }
        let p = temp.path().join(name);
        std::fs::write(&p, bytes).unwrap();
        assert!(UniversalCliEngine::test_integrity(&p, None).await.is_err());
        let d = archive_diagnostics::diagnose_archive(&p, None, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap();
        assert_ne!(d.status, "healthy");
    }
    let p = temp.path().join("parts.tar.lzo.001");
    std::fs::write(&p, original).unwrap();
    UniversalCliEngine::test_integrity(&p, None).await.unwrap();
    assert!(compression_verification::verify_tar_reader(
        codec_stream::decoder(&root().join("directory-1.tar.lzo"), "lzo").unwrap(),
        &|| true
    )
    .is_err());
}
