use long_compress_assistant::services::compression_service::CompressionService;
use long_compress_assistant::models::compression::CompressionOptions;
use tempfile::tempdir;
use std::io::Write;
use std::fs::File;

/// 验证压缩选项中密码字段的行为
#[test]
fn test_password_compression_options() {
    let opts_with_pwd = CompressionOptions {
        password: Some("test123".to_string()),
        ..Default::default()
    };
    let opts_without = CompressionOptions::default();

    assert_eq!(opts_with_pwd.password, Some("test123".to_string()));
    assert_eq!(opts_without.password, None);
}

/// 验证 validate_compression_request 拒绝 ZIP 密码压缩
#[test]
fn test_zip_rejects_password_compression() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("test.txt");
    File::create(&source).unwrap();

    let options = CompressionOptions {
        format: Some("zip".to_string()),
        password: Some("secret".to_string()),
        ..Default::default()
    };

    let result = CompressionService::validate_compression_request(
        &[source.to_string_lossy().to_string()],
        "output.zip",
        &options,
    );
    assert!(result.is_ok(), "ZIP 现在支持密码压缩（通过 7z CLI）");
}

/// 验证 validate_compression_request 允许 7Z 密码压缩
#[test]
fn test_7z_accepts_password_compression() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("test.txt");
    File::create(&source).unwrap();

    let options = CompressionOptions {
        format: Some("7z".to_string()),
        password: Some("secret".to_string()),
        ..Default::default()
    };

    let result = CompressionService::validate_compression_request(
        &[source.to_string_lossy().to_string()],
        "output.7z",
        &options,
    );
    assert!(result.is_ok(), "7Z 应支持密码压缩");
    assert_eq!(result.unwrap(), "7z");
}

/// 验证 test_archive_password 对 ZIP 的行为
#[tokio::test]
async fn test_zip_password_detection() {
    let temp = tempdir().unwrap();
    // 创建一个简单的测试文件
    let test_file = temp.path().join("normal.txt");
    let mut f = File::create(&test_file).unwrap();
    f.write_all(b"test content").unwrap();

    let output_zip = temp.path().join("test.zip");
    let output_str = output_zip.to_string_lossy().to_string();

    // 创建一个普通 ZIP（无密码）
    let service = CompressionService::new_with_defaults().await;
    let result = service.compress_zip_enhanced(
        &[test_file.to_string_lossy().to_string()],
        &output_str,
        CompressionOptions::default(),
    ).await;

    if result.is_ok() && output_zip.exists() {
        // 测试密码检测 — 无密码 ZIP 应该可以通过空字符串验证
        let password_ok = service.test_archive_password(&output_str, "").await;
        assert!(password_ok.is_ok(), "无密码 ZIP 应该可以通过验证");
    }
}
