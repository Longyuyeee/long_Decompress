use super::*;
use crate::models::compression::{TaskLogSeverity};
use crate::services::{archive_browser, archive_preview, compression_format, compression_verification, split_archive_detector::SplitArchiveDetector};
use std::sync::{Arc, atomic::AtomicBool};

struct Runtime;
impl CompressionRuntime for Runtime {
    fn check_cancellation(&self) -> Result<()> { Ok(()) }
    fn cancellation_flag(&self) -> Arc<AtomicBool> { Arc::new(AtomicBool::new(false)) }
    fn copy_buffer_size(&self) -> usize { 64 * 1024 }
    fn emit_log(&self, _: &Window, _: &str, _: &str, _: TaskLogSeverity) {}
    fn emit_progress(&self, _: &Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {}
}

fn fixtures() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/lzip-1412")
}
fn slices(root: &Path, name: &str, bytes: &[u8], size: usize) -> Vec<std::path::PathBuf> {
    bytes.chunks(size).enumerate().map(|(index, bytes)| {
        let path = root.join(format!("{name}.{:03}", index + 1));
        std::fs::write(&path, bytes).unwrap(); path
    }).collect()
}

#[test]
fn lzip1520_creation_validation_retains_format_and_rejects_password() {
    let source = fixtures().join("原文件/说明.txt").to_string_lossy().into_owned();
    for format in ["lz", "tar.lz", "tlz"] {
        let output = format!("结果.{format}");
        let mut options = CompressionOptions { format: Some(format.into()), split_size: Some(19), ..Default::default() };
        let result = compression_format::validate_compression_request(std::slice::from_ref(&source), &output, &options).unwrap();
        assert_eq!(result, if format == "tlz" { "tar.lz" } else { format });
        options.password = Some("no native password".into());
        assert!(compression_format::validate_compression_request(std::slice::from_ref(&source), &output, &options).is_err());
    }
    let options = CompressionOptions { format: Some("lz".into()), split_size: Some(19), ..Default::default() };
    assert!(compression_format::validate_compression_request(&[fixtures().join("原文件").to_string_lossy().into_owned()], "目录.lz", &options).is_err());
}

#[tokio::test]
async fn lzip1520_gnu_originals_browse_preview_and_full_verification_from_any_member() {
    let temp = tempfile::tempdir().unwrap();
    for name in ["说明.txt.lz", "资料.tar.lz", "资料别名.tlz", "多成员.txt.lz"] {
        let original = std::fs::read(fixtures().join(name)).unwrap();
        // Header and trailer both cross volumes; members are byte slices, not LZIP members.
        let parts = slices(temp.path(), name, &original, 5);
        let tar = name.ends_with("tar.lz") || name.ends_with("tlz");
        let expected = archive_browser::browse_archive(&fixtures().join(name), None).await.unwrap();
        for member in [&parts[0], &parts[parts.len()/2], parts.last().unwrap()] {
            let first = SplitArchiveDetector::complete_source(member).unwrap();
            let result = archive_browser::browse_archive(member, None).await.unwrap();
            assert_eq!(result.format, expected.format);
            assert_eq!(result.total_uncompressed_size, expected.total_uncompressed_size);
            let route = if tar { compression_format::CompressionRoute::TarLzip } else { compression_format::CompressionRoute::Lzip };
            assert!(compression_verification::verify_native(route, &first, None, || false).unwrap());
            if tar {
                let preview = archive_preview::preview_archive_text(member, "说明.txt", None).await.unwrap();
                assert_eq!(preview.content, String::from_utf8(std::fs::read(fixtures().join("原文件/说明.txt")).unwrap()).unwrap().replace("\r\n", "\n"));
                assert!(!preview.truncated);
            }
            let mut bytes = Vec::new(); decoder(&first, "lz").unwrap().read_to_end(&mut bytes).unwrap();
            let mut unsplit = Vec::new(); decoder(&fixtures().join(name), "lz").unwrap().read_to_end(&mut unsplit).unwrap();
            assert_eq!(bytes, unsplit);
        }
        assert_eq!(parts.iter().flat_map(|part| std::fs::read(part).unwrap()).collect::<Vec<_>>(), original);
    }
}

#[test]
fn lzip1520_creation_uses_transactional_split_and_verifies_complete_stream() {
    use crate::services::compression_service::CompressionService;
    for format in ["lz", "tar.lz"] {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join(format!("应用创建.{format}"));
        let staged = CompressionService::temporary_compression_output(&target).unwrap();
        let source = fixtures().join(if format == "lz" { "原文件/说明.txt" } else { "原文件" });
        compress(&Runtime, None, "lzip1520", &[source.to_string_lossy().into_owned()], &staged.to_string_lossy(), CompressionOptions { split_size: Some(19), level: 1, ..Default::default() }, format).unwrap();
        let first = super::super::archive_volumes::first_volume_path(&staged);
        let route = if format == "lz" { compression_format::CompressionRoute::Lzip } else { compression_format::CompressionRoute::TarLzip };
        assert!(compression_verification::verify_native(route, &first, None, || false).unwrap());
        CompressionService::finalize_compression_output(Ok(()), &staged, &target, true, false).unwrap();
        let parts = super::super::archive_volumes::complete_volume_paths(&target).unwrap();
        assert!(parts.len() > 1); assert!(!target.exists());
        let mut bytes = Vec::new(); decoder(&parts[0], "lz").unwrap().read_to_end(&mut bytes).unwrap();
        if format == "lz" { assert_eq!(bytes, std::fs::read(source).unwrap()); }
        else {
            let mut archive = ::tar::Archive::new(bytes.as_slice());
            let names = archive.entries().unwrap().map(|entry| entry.unwrap().path().unwrap().to_string_lossy().into_owned()).collect::<Vec<_>>();
            assert!(names.iter().any(|name| name.contains("空目录")));
            assert!(names.iter().any(|name| name.ends_with("说明.txt")));
        }
    }
}

#[tokio::test]
async fn lzip1520_missing_middle_and_tail_never_report_healthy() {
    let temp = tempfile::tempdir().unwrap();
    let original = std::fs::read(fixtures().join("资料.tar.lz")).unwrap();
    let parts = slices(temp.path(), "资料.tar.lz", &original, 97);
    for index in [0, parts.len()/2] {
        let bytes = std::fs::read(&parts[index]).unwrap(); std::fs::remove_file(&parts[index]).unwrap();
        let error = archive_browser::browse_archive(parts.last().unwrap(), None).await.unwrap_err().to_string();
        assert!(error.contains("[archive-inspection:incomplete]"), "{error}");
        assert!(error.contains(parts[index].file_name().unwrap().to_str().unwrap()), "{error}");
        std::fs::write(&parts[index], bytes).unwrap();
    }
    let tail = parts.last().unwrap(); let bytes = std::fs::read(tail).unwrap();
    std::fs::write(tail, &bytes[..bytes.len()-1]).unwrap();
    let error = compression_verification::verify_native(compression_format::CompressionRoute::TarLzip, &parts[0], None, || false).unwrap_err();
    assert!(format!("{error:#}").contains("damaged"));
    // Same source becomes readable after the missing bytes are restored.
    std::fs::write(tail, bytes).unwrap();
    assert!(compression_verification::verify_native(compression_format::CompressionRoute::TarLzip, &parts[0], None, || false).unwrap());
}

#[tokio::test]
async fn lzip1520_plain_multimember_preview_is_bounded_and_checks_late_crc() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("大文本.txt");
    let text = "A".repeat(1024 * 1024 + 73);
    std::fs::write(&source, &text).unwrap();
    let archive = temp.path().join("大文本.txt.lz");
    compress(&Runtime, None, "preview", &[source.to_string_lossy().into_owned()], &archive.to_string_lossy(), CompressionOptions { level: 1, ..Default::default() }, "lz").unwrap();
    let mut bytes = std::fs::read(&archive).unwrap();
    bytes.extend(std::fs::read(fixtures().join("说明.txt.lz")).unwrap());
    let parts = slices(temp.path(), "大文本.txt.lz", &bytes, 19);
    let result = archive_preview::preview_archive_text(parts.last().unwrap(), "大文本.txt", None).await.unwrap();
    assert!(result.truncated); assert_eq!(result.content, "A".repeat(1024 * 1024));
    let crc = bytes.len()-20; bytes[crc] ^= 1;
    slices(temp.path(), "大文本.txt.lz", &bytes, 19);
    assert!(archive_preview::preview_archive_text(&parts[0], "大文本.txt", None).await.is_err());
    assert!(archive_preview::preview_archive_text(&parts[0], "wrong.txt", None).await.is_err());
}
