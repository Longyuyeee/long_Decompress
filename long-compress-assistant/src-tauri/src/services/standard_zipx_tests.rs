use super::{numeric_volume_reader::ZipInputReader, split_archive_detector::SplitArchiveDetector};
use std::{io::Read, path::{Path, PathBuf}};
use sha2::{Digest, Sha256};

const PASSWORD: &str = "ZipxTest-1418";
const PAYLOAD_HASH: &str = "ec69f497c6ea5ecc2cf2a2b64bed3a4cc6c243ce1e6d562ec5661f66d76e5575";
fn fixture(mode: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/standard-zipx-1418").join(mode)
}
fn copy_fixture(mode: &str, target: &Path) {
    for entry in std::fs::read_dir(fixture(mode)).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
    }
}
fn check_payload(bytes: &[u8]) {
    assert_eq!(bytes.len(), 170_123);
    assert_eq!(format!("{:x}", Sha256::digest(bytes)), PAYLOAD_HASH);
}

#[tokio::test]
async fn original_bandizip_zipx_disks_browse_preview_password_and_extract() {
    use super::{archive_browser::browse_archive, archive_preview::preview_archive_text,
        universal_engine::UniversalCliEngine, archive_engine::ArchiveEngine};
    use std::sync::{Arc, atomic::AtomicBool};
    for mode in ["none", "password"] {
        let root = fixture(mode);
        let password = (mode == "password").then_some(PASSWORD);
        for extension in ["zx01", "zx02", "zipx"] {
            let source = root.join(format!("真实原包.{extension}"));
            let before = std::fs::read(&source).unwrap();
            let group = SplitArchiveDetector::detect_split_archive(&source).unwrap().unwrap();
            assert!(group.is_complete); assert_eq!(group.parts.len(), 3);
            assert_eq!(group.first_part, root.join("真实原包.zipx"));
            let metadata = browse_archive(&source, password).await.unwrap();
            assert_eq!(metadata.total_files, 2); assert_eq!(metadata.total_directories, 2);
            assert_eq!(metadata.encrypted, password.is_some());
            let preview = preview_archive_text(&source, "说明.txt", password).await.unwrap();
            assert_eq!(preview.content, "真实 ZIPX 标准分卷测试。中文、空目录和跨卷内容。\n".repeat(20));
            let mut archive = zip_aes::ZipArchive::new(ZipInputReader::open(&source).unwrap()).unwrap();
            let mut entry = if let Some(password) = password { archive.by_name_decrypt("中文目录/原文件.bin", password.as_bytes()).unwrap() }
                else { archive.by_name("中文目录/原文件.bin").unwrap() };
            assert_eq!(entry.compression(), zip_aes::CompressionMethod::Xz);
            let mut bytes = Vec::new(); entry.read_to_end(&mut bytes).unwrap(); check_payload(&bytes);
            drop(entry);
            assert_eq!(UniversalCliEngine::zip_requires_password(&source).unwrap(), password.is_some());
            if password.is_some() {
                assert!(!UniversalCliEngine::try_zip_password(&source, "wrong").unwrap());
                assert!(UniversalCliEngine::try_zip_password(&source, PASSWORD).unwrap());
            }
            let temp = tempfile::tempdir().unwrap();
            UniversalCliEngine::new().extract_with_progress(&source, temp.path(), Some(password.unwrap_or("unused")), false,
                Arc::new(|_| {}), Arc::new(|_, _| {}), Arc::new(AtomicBool::new(false))).await.unwrap();
            check_payload(&std::fs::read(temp.path().join("中文目录/原文件.bin")).unwrap());
            assert!(temp.path().join("中文目录/空目录").is_dir());
            assert_eq!(std::fs::read(&source).unwrap(), before);
            assert_eq!(super::compression_service::CompressionService::extraction_history_format(&source, &super::archive_format::ArchiveFormat::Zip), "zipx");
            assert_eq!(super::compression_service::CompressionService::extraction_output_path(&source, Some("out"), true), Path::new("out").join("真实原包"));
        }
    }
}

#[test]
fn zipx_standard_missing_members_are_named_and_do_not_mix_zip_or_numeric_disks() {
    for mode in ["none", "password"] {
        for missing in ["zx01", "zx02", "zipx"] {
            let temp = tempfile::tempdir().unwrap(); copy_fixture(mode, temp.path());
            let absent = temp.path().join(format!("真实原包.{missing}"));
            let bytes = std::fs::read(&absent).unwrap(); std::fs::remove_file(&absent).unwrap();
            // Same-stem ZIP and numeric members cannot fill the missing ZIPX disk.
            std::fs::write(temp.path().join("真实原包.z01"), b"unrelated ZIP").unwrap();
            std::fs::write(temp.path().join("真实原包.zipx.001"), b"unrelated numeric").unwrap();
            let selected = temp.path().join(if missing == "zipx" { "真实原包.zx01" } else { "真实原包.zipx" });
            let group = SplitArchiveDetector::detect_split_archive(&selected).unwrap().unwrap();
            assert!(!group.is_complete); assert_eq!(group.missing_parts, vec![absent.clone()]);
            let error = ZipInputReader::open(&selected).err().unwrap().to_string();
            assert!(error.contains("[archive-inspection:incomplete]")); assert!(error.contains(&format!("真实原包.{missing}")));
            std::fs::write(&absent, &bytes).unwrap();
            assert!(SplitArchiveDetector::detect_split_archive(&selected).unwrap().unwrap().is_complete);
            assert!(zip_aes::ZipArchive::new(ZipInputReader::open(&selected).unwrap()).is_ok());
        }
    }
}

#[tokio::test]
async fn zipx_standard_reuses_only_verified_complete_files_when_middle_disk_is_missing() {
    use std::sync::{Arc, atomic::AtomicBool};
    use super::archive_diagnostics::{diagnose_archive, repair_zip_with_password_to_new};
    for mode in ["none", "password"] {
        let temp = tempfile::tempdir().unwrap(); copy_fixture(mode, temp.path());
        let main = temp.path().join("真实原包.zipx");
        std::fs::remove_file(main.with_extension("zx02")).unwrap();
        let password = (mode == "password").then_some(PASSWORD);
        let before = std::fs::read(&main).unwrap();
        let report = diagnose_archive(&main, password, Arc::new(AtomicBool::new(false))).await.unwrap();
        assert_eq!(report.status, "partial_recovery", "{report:?}");
        assert!(report.can_repair && !report.integrity_tested);
        let output = temp.path().join("仅救回完整说明.zip");
        let result = repair_zip_with_password_to_new(&main, &output, password, &AtomicBool::new(false)).unwrap();
        assert_eq!(result.recovered_files, 1);
        let mut archive = zip_aes::ZipArchive::new(std::fs::File::open(&output).unwrap()).unwrap();
        assert!(archive.index_for_name("中文目录/原文件.bin").is_none());
        let mut entry = if let Some(password) = password { archive.by_name_decrypt("说明.txt", password.as_bytes()).unwrap() }
            else { archive.by_name("说明.txt").unwrap() };
        let mut text = String::new(); entry.read_to_string(&mut text).unwrap();
        assert_eq!(text, "真实 ZIPX 标准分卷测试。中文、空目录和跨卷内容。\n".repeat(20));
        assert_eq!(std::fs::read(&main).unwrap(), before);
    }
}

#[test]
#[ignore = "Requires independent WinRAR via LONG_WINRAR"]
fn zipx_standard_creation_publishes_and_independent_winrar_extracts_all_password_modes() {
    use crate::models::compression::CompressionOptions;
    use super::{compression_service::CompressionService, standard_zip_writer::ZipSplitMode, zip_password::ZipEncryption};
    let tool = std::env::var_os("LONG_WINRAR").expect("LONG_WINRAR");
    for encryption in [None, Some(ZipEncryption::Aes256), Some(ZipEncryption::Zipcrypto)] {
        let temp = tempfile::tempdir().unwrap();
        let mut original = zip_aes::ZipArchive::new(ZipInputReader::open(&fixture("none").join("真实原包.zipx")).unwrap()).unwrap();
        let mut payload = Vec::new(); original.by_name("中文目录/原文件.bin").unwrap().read_to_end(&mut payload).unwrap();
        let folder = temp.path().join("资料"); std::fs::create_dir_all(folder.join("空目录")).unwrap();
        std::fs::write(folder.join("内容.bin"), &payload).unwrap(); std::fs::write(folder.join("空文件.txt"), []).unwrap();
        let output = temp.path().join("分享.zipx");
        let staged = CompressionService::temporary_compression_output(&output).unwrap();
        let password = encryption.map(|_| PASSWORD);
        let options = CompressionOptions { format: Some("zipx".into()), level: 6,
            password: password.map(str::to_string), zip_encryption: encryption.unwrap_or_default(),
            split_size: Some(65_536), zip_split_mode: ZipSplitMode::Standard, preserve_paths: Some(true), ..Default::default() };
        super::native_compression::zip::compress(&CompressionService::for_testing(), None, "1418", &[folder.to_string_lossy().into()], &staged.to_string_lossy(), options).unwrap();
        super::compression_verification::verify_native(super::compression_format::CompressionRoute::Zip, &staged, password, || false).unwrap();
        super::output_publish_transaction::publish_verified_volumes(&staged, &output, || false).unwrap();
        let parts = super::archive_volumes::complete_volume_paths(&output).unwrap();
        assert_eq!(parts.len(), 3); assert_eq!(parts[0], output.with_extension("zx01")); assert_eq!(parts[2], output);
        assert!(!output.with_extension("z01").exists());
        let restored = temp.path().join("独立回解");
        let mut command = std::process::Command::new(&tool);
        command.args(["x", "-y", "-ibck"]);
        command.arg(password.map(|password| format!("-p{password}")).unwrap_or_else(|| "-p-".into()));
        let result = command.arg(&output).arg(format!("{}{}", restored.display(), std::path::MAIN_SEPARATOR)).output().unwrap();
        if !result.status.success() {
            let retained = temp.keep();
            panic!("{:?}; retained {}: {} {}", encryption, retained.display(), String::from_utf8_lossy(&result.stdout), String::from_utf8_lossy(&result.stderr));
        }
        check_payload(&std::fs::read(restored.join("资料/内容.bin")).unwrap());
        assert!(restored.join("资料/空目录").is_dir()); assert_eq!(std::fs::metadata(restored.join("资料/空文件.txt")).unwrap().len(), 0);
        assert_eq!(std::fs::read(folder.join("内容.bin")).unwrap(), payload);
    }
}

#[test]
fn zipx_standard_publication_preserves_existing_files_and_rolls_back_cancel_or_late_collision() {
    for scenario in ["conflict", "late-conflict", "cancel"] {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("分享.zipx");
        let staged = super::output_publish_transaction::staged_output_path(&output, "zipx-1418").unwrap();
        for ext in ["zx01", "zx02", "zipx"] {
            std::fs::copy(fixture("none").join(format!("真实原包.{ext}")), staged.with_extension(ext)).unwrap();
        }
        let conflict = output.with_extension("zx02");
        if scenario == "conflict" { std::fs::write(&conflict, b"existing user data").unwrap(); }
        let mut checks = 0;
        let result = super::output_publish_transaction::publish_verified_volumes(&staged, &output, || {
            checks += 1;
            if scenario == "late-conflict" && checks == 2 { std::fs::write(&conflict, b"existing user data").unwrap(); }
            scenario == "cancel" && checks == 2
        });
        assert!(result.is_err(), "{scenario}");
        assert!(!output.exists()); assert!(!output.with_extension("zx01").exists());
        for ext in ["zx01", "zx02", "zipx"] { assert!(!staged.with_extension(ext).exists()); }
        if scenario != "cancel" { assert_eq!(std::fs::read(conflict).unwrap(), b"existing user data"); }
        else { assert!(!conflict.exists()); }
    }
}
