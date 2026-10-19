use super::file_integrity_service::FileIntegrityService;
use anyhow::Result;

#[test]
fn checksum1519_report_distinguishes_real_file_failures() -> Result<()> {
    let temp = tempfile::tempdir()?;
    std::fs::write(temp.path().join("匹配.txt"), b"Hello, World!")?;
    std::fs::write(temp.path().join("内容改变.txt"), b"different")?;
    std::fs::create_dir(temp.path().join("实际是目录.txt"))?;
    let manifest = temp.path().join("清单.md5");
    std::fs::write(&manifest, ["匹配.txt", "内容改变.txt", "缺少.zip.002", "实际是目录.txt"]
        .map(|name| format!("65a8e27d8879283831b664bd8b7f0ad4  {name}\n")).join(""))?;
    let result = serde_json::to_value(FileIntegrityService::verify_checksum_file(&manifest, temp.path())?)?;
    let statuses: Vec<_> = result["entries"].as_array().expect("per-file entries instead of one boolean")
        .iter().map(|entry| entry["status"].as_str().unwrap()).collect();
    assert_eq!(statuses, ["matched", "mismatch", "missing", "unreadable"]);
    Ok(())
}

#[test]
fn checksum1519_reads_independent_sfv_md5_sha256_with_bom_spaces_and_binary_marker() -> Result<()> {
    let temp = tempfile::tempdir()?;
    std::fs::write(temp.path().join("中文 空格.txt"), b"Hello, World!")?;
    for (extension, digest) in [("sfv", "EC4AC3D0"), ("md5", "65a8e27d8879283831b664bd8b7f0ad4"),
        ("sha256", "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f")] {
        let path = temp.path().join(format!("checks.{extension}"));
        let row = if extension == "sfv" { format!("中文 空格.txt {digest}") } else { format!("{digest} *中文 空格.txt") };
        std::fs::write(&path, format!("\u{feff}; 独立已知摘要\r\n{row}\r\n"))?;
        let report = FileIntegrityService::verify_checksum_file(&path, temp.path())?;
        assert!(report.valid, "{report:?}");
        assert_eq!(report.entries[0].line, 2);
        assert_eq!(report.entries[0].actual_checksum.as_deref(), Some(digest.to_ascii_lowercase().as_str()));
    }
    Ok(())
}

#[test]
fn checksum1519_selective_pass_does_not_read_unselected_files_or_claim_full_success() -> Result<()> {
    use super::checksum_manifest::ChecksumEntryStatus as S;
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("checks.md5");
    std::fs::write(temp.path().join("可读.txt"), b"Hello, World!")?;
    std::fs::write(&path, "65a8e27d8879283831b664bd8b7f0ad4  可读.txt\n65a8e27d8879283831b664bd8b7f0ad4  不存在.txt\n")?;
    let report = FileIntegrityService::verify_checksum_entries(&path, temp.path(), Some(&[1]))?;
    assert!(!report.valid);
    assert_eq!(report.entries[0].status, S::Matched);
    assert_eq!(report.entries[1].status, S::NotChecked);
    assert_eq!(report.entries[1].checked_at, None);
    assert_eq!(report.entries[1].actual_checksum, None);
    let none = FileIntegrityService::verify_checksum_entries(&path, temp.path(), Some(&[]))?;
    assert!(!none.valid && none.entries.iter().all(|row| row.status == S::NotChecked));
    Ok(())
}

#[test]
fn checksum1519_manifest_identity_changes_before_selective_results_can_be_merged() -> Result<()> {
    use super::checksum_manifest::ChecksumEntryStatus as S;
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("checks.md5");
    std::fs::write(temp.path().join("one.txt"), b"Hello, World!")?;
    std::fs::write(&path, "65a8e27d8879283831b664bd8b7f0ad4  one.txt\n")?;
    let old = FileIntegrityService::verify_checksum_file(&path, temp.path())?;
    assert!(old.valid);
    std::fs::write(&path, "# changed\n65a8e27d8879283831b664bd8b7f0ad4  one.txt\n")?;
    let new = FileIntegrityService::verify_checksum_entries(&path, temp.path(), Some(&[1]))?;
    assert_ne!(old.manifest_id, new.manifest_id);
    assert!(!new.valid);
    assert_eq!(new.entries[0].status, S::NotChecked);
    Ok(())
}

#[test]
fn checksum1519_malformed_rows_remain_visible_and_empty_manifests_do_not_pass() -> Result<()> {
    use super::checksum_manifest::ChecksumEntryStatus as S;
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("checks.md5");
    std::fs::write(&path, "not-a-row\nzzzzzzzz  文件.txt\n65a8e27d8879283831b664bd8b7f0ad4  ../outside\n")?;
    let report = FileIntegrityService::verify_checksum_file(&path, temp.path())?;
    assert!(!report.valid);
    assert_eq!(report.entries.len(), 3);
    assert!(report.entries.iter().all(|row| row.status == S::InvalidEntry && row.path.is_empty()));
    std::fs::write(&path, "# only comment\n; nothing\n")?;
    let empty = FileIntegrityService::verify_checksum_file(&path, temp.path())?;
    assert!(!empty.valid && empty.entries.is_empty());
    Ok(())
}

#[cfg(windows)]
#[test]
fn checksum1519_actual_windows_lock_has_its_reason_and_recovers_without_reading_other_files() -> Result<()> {
    use std::os::windows::fs::OpenOptionsExt;
    use super::checksum_manifest::ChecksumEntryStatus as S;
    let temp = tempfile::tempdir()?;
    let file = temp.path().join("占用.txt");
    std::fs::write(&file, b"Hello, World!")?;
    let path = temp.path().join("checks.md5");
    std::fs::write(&path, "65a8e27d8879283831b664bd8b7f0ad4  占用.txt\n65a8e27d8879283831b664bd8b7f0ad4  不存在.txt\n")?;
    let lock = std::fs::OpenOptions::new().read(true).share_mode(0).open(&file)?;
    let report = FileIntegrityService::verify_checksum_entries(&path, temp.path(), Some(&[1]))?;
    assert_eq!(report.entries[0].status, S::Unreadable);
    assert!(report.entries[0].reason.contains("os error 32"));
    assert_eq!(report.entries[1].status, S::NotChecked);
    drop(lock);
    let recovered = FileIntegrityService::verify_checksum_entries(&path, temp.path(), Some(&[1]))?;
    assert_eq!(recovered.entries[0].status, S::Matched);
    assert_eq!(recovered.entries[1].status, S::NotChecked);
    Ok(())
}

#[test]
fn checksum1519_generated_lists_roundtrip_all_three_algorithms() -> Result<()> {
    use super::file_integrity_service::ChecksumAlgorithm as A;
    let temp = tempfile::tempdir()?;
    let file = temp.path().join("中文 空格.txt");
    std::fs::write(&file, b"Hello, World!")?;
    for (extension, algorithm) in [("sfv", A::CRC32), ("md5", A::MD5), ("sha256", A::SHA256)] {
        let path = temp.path().join(format!("export.{extension}"));
        FileIntegrityService::generate_checksum_file(&[&file], algorithm, &path)?;
        assert!(FileIntegrityService::verify_checksum_file(&path, temp.path())?.valid);
    }
    Ok(())
}
