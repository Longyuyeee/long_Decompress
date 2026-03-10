fn main() {
    println!("测试ZIP库基本功能");
    
    // 测试zip库是否可用
    use zip::write::FileOptions;
    use zip::CompressionMethod;
    
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(6);
    
    println!("ZIP选项创建成功: {:?}", options.compression_level());
    
    // 测试压缩级别转换
    let level: i32 = 6;
    println!("压缩级别: {}", level);
}
