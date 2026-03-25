use long_compress_assistant::services::compression_service::{CompressionService, CompressionServiceConfig, ArchiveFormat};
use std::io::Write;
use tempfile::tempdir;

#[tokio::main]
async fn main() {
    println!("--- 开始 7z 增强逻辑验证 ---");

    // 1. 验证 7z 签名识别
    let seven_z_header = [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C, 0x00, 0x00];
    let format = ArchiveFormat::from_magic(&seven_z_header);
    println!("7z 签名识别结果 (预期 SevenZip): {:?}", format);
    assert_eq!(format, ArchiveFormat::SevenZip);

    // 2. 模拟 do_extract_7z 调用 (面对无效文件)
    let temp_dir = tempdir().unwrap();
    let dummy_path = temp_dir.path().join("fake.7z");
    std::fs::write(&dummy_path, b"not a 7z file").unwrap();

    let service = CompressionService::new(CompressionServiceConfig::default());
    
    // 我们需要一个 Mock Window，但在 Example 中我们直接绕过 GUI 层的 emit
    // 主要是为了验证后端逻辑的连通性
    
    println!("开始调用 test_archive_password 验证逻辑...");
    let result = service.test_archive_password(dummy_path.to_str().unwrap(), "123456").await;
    
    match result {
        Ok(is_correct) => println!("密码测试结果: {}", is_correct),
        Err(e) => println!("预期的错误处理: {:?}", e),
    }

    println!("7z 逻辑验证示例运行结束！");
}
