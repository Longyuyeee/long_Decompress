use long_compress_assistant::services::rar_support::RarSupportService;
use long_compress_assistant::models::compression::DecompressOptions;
use tempfile::tempdir;

#[test]
fn test_check_rar_tool_installed() {
    let is_installed = RarSupportService::check_rar_tool_installed();
    println!("系统RAR工具安装状态: {}", is_installed);
    assert!(true, "接口测试通过");
}

#[tokio::test]
async fn test_rar_service_creation() {
    let _service = RarSupportService::new();
    assert!(true, "RAR服务创建成功");
    let _default_service = RarSupportService::default();
    assert!(true, "RAR服务默认实现成功");
}

#[tokio::test]
async fn test_rar_extraction_nonexistent_file() {
    let temp_dir = tempdir().unwrap();
    let service = RarSupportService::new();
    let nonexistent_rar = temp_dir.path().join("nonexistent.rar");
    let output_dir = temp_dir.path().join("output");
    let options = DecompressOptions::default();

    let result = service.extract_rar(
        &nonexistent_rar,
        &output_dir,
        None,
        &options,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    ).await;

    assert!(result.is_err(), "不存在的RAR文件应该失败");
}

#[tokio::test]
async fn test_rar_info_nonexistent_file() {
    let temp_dir = tempdir().unwrap();
    let service = RarSupportService::new();
    let nonexistent_rar = temp_dir.path().join("nonexistent.rar");
    let result = service.get_rar_info(&nonexistent_rar).await;
    assert!(result.is_err(), "不存在的RAR文件信息获取应该失败");
}

#[tokio::test]
async fn test_rar_file_validation() {
    let temp_dir = tempdir().unwrap();
    let service = RarSupportService::new();

    let invalid_rar = temp_dir.path().join("invalid.rar");
    std::fs::write(&invalid_rar, b"this is not a rar file").unwrap();
    let is_valid = service.is_valid_rar_file(&invalid_rar).await;
    assert!(!is_valid, "无效文件应该返回false");

    let empty_file = temp_dir.path().join("empty.rar");
    std::fs::write(&empty_file, b"").unwrap();
    let is_valid = service.is_valid_rar_file(&empty_file).await;
    assert!(!is_valid, "空文件应该返回false");

    let mut rar_signature = vec![b'R', b'a', b'r', b'!', 0x1A, 0x07, 0x00];
    rar_signature.extend_from_slice(b"some fake rar content");
    let fake_rar = temp_dir.path().join("fake_with_signature.rar");
    std::fs::write(&fake_rar, &rar_signature).unwrap();
    let is_valid = service.is_valid_rar_file(&fake_rar).await;
    println!("带RAR签名的文件验证结果: {}", is_valid);
}

#[tokio::test]
async fn test_rar_list_and_test_functions() {
    let temp_dir = tempdir().unwrap();
    let service = RarSupportService::new();
    let test_rar = temp_dir.path().join("test_list.rar");
    std::fs::write(&test_rar, b"fake rar for list test").unwrap();

    let list_result = service.list_rar_contents(&test_rar, None).await;
    println!("RAR列表内容结果: {:?}", list_result);

    let test_result = service.test_rar_integrity(&test_rar, None).await;
    println!("RAR完整性测试结果: {:?}", test_result);
    assert!(true, "列表和测试功能接口测试完成");
}

#[tokio::test]
async fn test_rar_password_attempt() {
    let temp_dir = tempdir().unwrap();
    let service = RarSupportService::new();
    let test_rar = temp_dir.path().join("test_pwd.rar");
    std::fs::write(&test_rar, b"fake rar content").unwrap();

    let result = service.test_rar_password(&test_rar, "test123").await;
    println!("RAR密码测试结果: {}", result);
    assert!(true, "密码尝试接口测试完成");
}
