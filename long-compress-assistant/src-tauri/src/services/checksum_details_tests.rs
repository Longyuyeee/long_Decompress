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
