use super::{checksum_export::{self, ChecksumExportEntry}, checksum_manifest::{self, ChecksumEntryStatus}, file_integrity_service::{ChecksumAlgorithm, FileIntegrityService}};
use std::{fs, path::Path};

fn completed(path: &Path, algorithm: ChecksumAlgorithm) -> ChecksumExportEntry {
    ChecksumExportEntry { path:path.to_string_lossy().into_owned(), checksum:FileIntegrityService::calculate_checksum(path, algorithm).unwrap().checksum, status:"success".into() }
}

#[test]
fn result_checksum_1525_nested_duplicate_names_roundtrip_all_algorithms() {
    let root=tempfile::tempdir().unwrap();
    let a=root.path().join("甲/同名 文件.txt"); let b=root.path().join("乙/同名 文件.txt");
    fs::create_dir_all(a.parent().unwrap()).unwrap(); fs::create_dir_all(b.parent().unwrap()).unwrap();
    fs::write(&a,b"first observation").unwrap(); fs::write(&b,b"different observation").unwrap();
    for (algorithm,ext) in [(ChecksumAlgorithm::CRC32,"sfv"),(ChecksumAlgorithm::MD5,"md5"),(ChecksumAlgorithm::SHA256,"sha256")] {
        let manifest=root.path().join(format!("checksums.{ext}"));
        checksum_export::export(&manifest,&[completed(&a,algorithm),completed(&b,algorithm)],algorithm).unwrap();
        let text=fs::read_to_string(&manifest).unwrap(); assert!(text.contains("甲/同名 文件.txt")); assert!(text.contains("乙/同名 文件.txt"));
        let report=checksum_manifest::verify(&manifest,root.path(),None).unwrap();
        assert!(report.valid,"{report:?}"); assert_eq!(report.entries.len(),2);
    }
}

#[test]
fn result_checksum_1525_outside_base_does_not_truncate_manifest() {
    let root=tempfile::tempdir().unwrap(); let source=root.path().join("result.zip"); fs::write(&source,b"archive").unwrap();
    let target=root.path().join("other"); fs::create_dir(&target).unwrap(); let manifest=target.join("checksums.sha256"); fs::write(&manifest,b"original manifest").unwrap();
    let error=checksum_export::export(&manifest,&[completed(&source,ChecksumAlgorithm::SHA256)],ChecksumAlgorithm::SHA256).unwrap_err();
    assert!(error.contains("共同上级目录") && error.contains("跨磁盘"),"{error}"); assert_eq!(fs::read(&manifest).unwrap(),b"original manifest");
}

#[test]
fn result_checksum_1525_exports_only_completed_without_reopening() {
    let root=tempfile::tempdir().unwrap(); let source=root.path().join("result.001"); fs::write(&source,b"finished").unwrap();
    let entry=completed(&source,ChecksumAlgorithm::MD5); fs::remove_file(&source).unwrap();
    let manifest=root.path().join("checksums.md5");
    let entries=vec![entry,ChecksumExportEntry {path:root.path().join("result.002").to_string_lossy().into_owned(),checksum:"partial".into(),status:"interrupted".into()},ChecksumExportEntry{path:"missing".into(),checksum:String::new(),status:"pending".into()}];
    checksum_export::export(&manifest,&entries,ChecksumAlgorithm::MD5).unwrap();
    assert_eq!(fs::read_to_string(&manifest).unwrap().lines().count(),1);
    let report=checksum_manifest::verify(&manifest,root.path(),None).unwrap(); assert_eq!(report.entries[0].status,ChecksumEntryStatus::Missing);
}

#[test]
fn result_checksum_1525_invalid_digest_and_self_overwrite_preserve_source() {
    let root=tempfile::tempdir().unwrap(); let source=root.path().join("checksums.md5"); fs::write(&source,b"source bytes").unwrap();
    let entry=completed(&source,ChecksumAlgorithm::MD5);
    assert!(checksum_export::export(&source,&[entry],ChecksumAlgorithm::MD5).unwrap_err().contains("不能覆盖"));
    let invalid=ChecksumExportEntry{path:source.to_string_lossy().into_owned(),checksum:"abcd".into(),status:"success".into()};
    let target=root.path().join("other.md5"); fs::write(&target,b"existing").unwrap();
    assert!(checksum_export::export(&target,&[invalid],ChecksumAlgorithm::MD5).unwrap_err().contains("摘要不完整"));
    assert_eq!(fs::read(&source).unwrap(),b"source bytes"); assert_eq!(fs::read(&target).unwrap(),b"existing");
}

#[test]
fn result_checksum_1525_unrepresentable_sfv_and_empty_results_publish_nothing() {
    let root=tempfile::tempdir().unwrap(); let source=root.path().join("#注释名.txt"); fs::write(&source,b"file").unwrap();
    let manifest=root.path().join("checksums.sfv");
    assert!(checksum_export::export(&manifest,&[completed(&source,ChecksumAlgorithm::CRC32)],ChecksumAlgorithm::CRC32).unwrap_err().contains("重命名"));
    assert!(!manifest.exists());
    assert!(checksum_export::export(&manifest,&[],ChecksumAlgorithm::CRC32).unwrap_err().contains("没有完整成功"));
    assert!(!manifest.exists());
    let md5=root.path().join("checksums.md5"); checksum_export::export(&md5,&[completed(&source,ChecksumAlgorithm::MD5)],ChecksumAlgorithm::MD5).unwrap();
    assert!(checksum_manifest::verify(&md5,root.path(),None).unwrap().valid);
}
