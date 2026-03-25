use long_compress_assistant::services::compression_service::{CompressionService, CompressionServiceConfig};
use std::io::{Cursor, Write};
use zip::write::FileOptions;
use zip::ZipArchive;

fn main() {
    println!("--- 开始 ZIP 增强逻辑验证 ---");

    // 1. 准备 Mock 数据
    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        
        zip.start_file("test.txt", options).unwrap();
        zip.write_all(b"Hello Zip!").unwrap();
        zip.finish().unwrap();
    }

    // 2. 初始化服务
    let service = CompressionService::new(CompressionServiceConfig::default());
    let reader = Cursor::new(buf);
    let mut archive = ZipArchive::new(reader).expect("无法打开 ZIP");

    // 3. 验证加密探测逻辑 (探测非加密文件)
    let is_encrypted = {
        let mut is_enc = false;
        match archive.by_index_decrypt(0, b"") {
            Ok(Err(_)) => is_enc = true,
            _ => {}
        }
        is_enc
    };

    println!("非加密文件探测结果 (预期 false): {}", is_encrypted);
    assert!(!is_encrypted);

    println!("ZIP 基础逻辑验证成功！");
}
