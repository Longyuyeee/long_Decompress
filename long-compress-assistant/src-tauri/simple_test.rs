use tempfile::tempdir;
use std::fs::File;
use std::io::Write;

fn main() {
    // 创建临时目录
    let temp_dir = tempdir().unwrap();
    
    // 创建测试文件
    let test_file = temp_dir.path().join("test.txt");
    let mut file = File::create(&test_file).unwrap();
    file.write_all(b"Test content").unwrap();
    
    println!("测试文件创建成功: {:?}", test_file);
    println!("文件存在: {}", test_file.exists());
    println!("文件大小: {}", test_file.metadata().unwrap().len());
}
