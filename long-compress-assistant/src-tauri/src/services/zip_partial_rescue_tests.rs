use super::{archive_diagnostics::{diagnose_archive, repair_zip_with_password_to_new}, zip_recovery_outcome::RecoveryStatus};
use std::{io::Read, path::Path, sync::{Arc, atomic::AtomicBool}};
use sha2::{Digest, Sha256};

const PASSWORD: &str = "ForumTest-1411";
fn fixture() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/haozip-forum-encrypted-1411")
}
fn copy_fixture(target: &Path) {
    for index in 1..=3 {
        let name = format!("论坛密码验收.haozip{index:02}.zip");
        std::fs::copy(fixture().join(&name), target.join(name)).unwrap();
    }
}

#[tokio::test]
async fn genuine_forum_missing_first_and_middle_rescues_only_verified_files_from_any_member() {
    for missing in [1, 2] { for selected in (1..=3).filter(|index| *index != missing) {
        let temp = tempfile::tempdir().unwrap(); copy_fixture(temp.path());
        std::fs::remove_file(temp.path().join(format!("论坛密码验收.haozip{missing:02}.zip"))).unwrap();
        let source = temp.path().join(format!("论坛密码验收.haozip{selected:02}.zip"));
        let before = std::fs::read(&source).unwrap();
        let report = diagnose_archive(&source, Some(PASSWORD), Arc::new(AtomicBool::new(false))).await.unwrap();
        assert_eq!(report.status, "partial_recovery", "{report:?}");
        assert!(report.can_repair && report.encrypted && !report.integrity_tested);
        assert_eq!(report.total_files, 1);
        let required = diagnose_archive(&source, None, Arc::new(AtomicBool::new(false))).await.unwrap();
        assert_eq!(required.status, "password_required"); assert!(!required.can_repair);
        let wrong = temp.path().join("wrong.zip");
        assert!(repair_zip_with_password_to_new(&source, &wrong, Some("wrong"), &AtomicBool::new(false)).is_err());
        assert!(!wrong.exists());
        let output = temp.path().join("rescue.zip");
        let result = repair_zip_with_password_to_new(&source, &output, Some(PASSWORD), &AtomicBool::new(false)).unwrap();
        assert!(result.verified && result.partial && result.output_encrypted && !result.directory_rebuilt);
        assert_eq!(result.recovered_entries, ["说明.txt"]);
        assert!(result.entry_results.iter().any(|entry| entry.name.as_deref() == Some("中文目录/原文件.bin") && entry.status == RecoveryStatus::Missing));
        assert!(result.entry_results.iter().any(|entry| entry.name.as_deref() == Some("说明.txt") && entry.status == RecoveryStatus::Recovered));
        let mut archive = zip_aes::ZipArchive::new(std::fs::File::open(&output).unwrap()).unwrap();
        let mut bytes = Vec::new(); archive.by_name_decrypt("说明.txt", PASSWORD.as_bytes()).unwrap().read_to_end(&mut bytes).unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), "b70dfbde85c5ad302aae256db376bb19c84c991ca9c25de18696e93f1178910b");
        assert!(archive.by_name("中文目录/原文件.bin").is_err());
        assert_eq!(std::fs::read(&source).unwrap(), before);
    } }
}

#[tokio::test]
async fn genuine_forum_missing_terminal_or_truncated_directory_is_not_guessed() {
    for missing_terminal in [false, true] {
        let temp = tempfile::tempdir().unwrap(); copy_fixture(temp.path());
        let terminal = temp.path().join("论坛密码验收.haozip03.zip");
        if missing_terminal { std::fs::remove_file(terminal).unwrap(); }
        else {
            std::fs::remove_file(temp.path().join("论坛密码验收.haozip02.zip")).unwrap();
            let mut bytes = std::fs::read(&terminal).unwrap(); bytes.truncate(bytes.len() - 10); std::fs::write(terminal, bytes).unwrap();
        }
        let source = temp.path().join("论坛密码验收.haozip01.zip");
        let output = temp.path().join("must-not-exist.zip");
        assert!(repair_zip_with_password_to_new(&source, &output, Some(PASSWORD), &AtomicBool::new(false)).is_err());
        assert!(!output.exists());
    }
}
