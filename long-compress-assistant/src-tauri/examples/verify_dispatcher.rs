use long_compress_assistant::services::compression_service::ArchiveFormat;
use std::io::{Write, Cursor};

fn main() {
    println!("--- 开始分发器 (Dispatcher) 嗅探逻辑验证 ---");

    // 1. 验证 ZIP 签名 (PK\x03\x04)
    let zip_header = [0x50, 0x4B, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00];
    let format = ArchiveFormat::from_magic(&zip_header);
    println!("ZIP 识别结果: {:?}", format);
    assert_eq!(format, ArchiveFormat::Zip);

    // 2. 验证 7z 签名 (7z\xBC\xAF\x27\x1C)
    let seven_z_header = [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C, 0x00, 0x00];
    let format = ArchiveFormat::from_magic(&seven_z_header);
    println!("7z 识别结果: {:?}", format);
    assert_eq!(format, ArchiveFormat::SevenZip);

    // 3. 验证 RAR5 签名 (Rar!\x1a\x07\x01\x00)
    let rar5_header = [0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x01, 0x00];
    let format = ArchiveFormat::from_magic(&rar5_header);
    println!("RAR5 识别结果: {:?}", format);
    assert_eq!(format, ArchiveFormat::Rar);

    // 4. 验证 Gzip 签名 (\x1F\x8B)
    let gzip_header = [0x1F, 0x8B, 0x08, 0x00];
    let format = ArchiveFormat::from_magic(&gzip_header);
    println!("Gzip 识别结果: {:?}", format);
    assert_eq!(format, ArchiveFormat::Gzip);

    // 5. 验证未知格式
    let unknown_header = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let format = ArchiveFormat::from_magic(&unknown_header);
    println!("未知格式识别结果: {:?}", format);
    assert_eq!(format, ArchiveFormat::Unknown);

    println!("\n分发器核心嗅探逻辑验证成功！");
}
