use long_compress_assistant::services::compression_service::CompressionService;
use long_compress_assistant::models::compression::CompressionOptions;
use tempfile::tempdir;
use std::io::Write;
use std::fs::File;

/// 基本 ZIP 压缩功能
#[tokio::test]
async fn test_zip_compression_basic() {
    let temp = tempdir().unwrap();
    let test_file = temp.path().join("hello.txt");
    let mut f = File::create(&test_file).unwrap();
    f.write_all(b"Hello, ZIP compression!").unwrap();

    let output = temp.path().join("basic.zip");
    let output_str = output.to_string_lossy().to_string();

    let service = CompressionService::new_with_defaults().await;
    let result = service.compress_zip_enhanced(
        &[test_file.to_string_lossy().to_string()],
        &output_str,
        CompressionOptions::default(),
    ).await;

    assert!(result.is_ok(), "基本ZIP压缩应成功: {:?}", result.err());
    assert!(output.exists(), "ZIP文件应被创建");
    assert!(output.metadata().unwrap().len() > 0, "ZIP文件不应为空");
}

/// 多文件 ZIP 压缩
#[tokio::test]
async fn test_zip_compression_multiple_files() {
    let temp = tempdir().unwrap();
    let files: Vec<_> = (0..3).map(|i| {
        let path = temp.path().join(format!("file{}.txt", i));
        let mut f = File::create(&path).unwrap();
        f.write_all(format!("Content of file {}", i).as_bytes()).unwrap();
        path.to_string_lossy().to_string()
    }).collect();

    let output = temp.path().join("multi.zip");
    let output_str = output.to_string_lossy().to_string();

    let service = CompressionService::new_with_defaults().await;
    let result = service.compress_zip_enhanced(&files, &output_str, CompressionOptions::default()).await;

    assert!(result.is_ok(), "多文件ZIP压缩应成功: {:?}", result.err());
    assert!(output.exists(), "ZIP文件应被创建");
}

/// 验证压缩选项默认值
#[test]
fn test_compression_options_defaults() {
    let options = CompressionOptions::default();
    assert_eq!(options.level, 0);
    assert_eq!(options.password, None);
    assert_eq!(options.split_size, None);
    assert_eq!(options.format, None);
    assert_eq!(options.delete_after, false);
}

/// 验证 removeable_compressed_sources 逻辑
#[test]
fn test_removable_source_detection() {
    let temp = tempdir().unwrap();
    let src = temp.path().join("src.txt");
    let out = temp.path().join("out.zip");
    File::create(&src).unwrap();
    File::create(&out).unwrap();

    let result = CompressionService::removable_compressed_sources(
        &[src.to_string_lossy().to_string()],
        &out.to_string_lossy().to_string(),
    ).unwrap();

    assert!(!result.is_empty(), "源文件应该在清理列表中");
    assert!(result[0].ends_with("src.txt"), "应包含源文件");
}

/// 验证当输出文件不存在时不返回可删除项
#[test]
fn test_removable_waits_for_output() {
    let temp = tempdir().unwrap();
    let src = temp.path().join("src.txt");
    let out = temp.path().join("missing.zip");
    File::create(&src).unwrap();

    let result = CompressionService::removable_compressed_sources(
        &[src.to_string_lossy().to_string()],
        &out.to_string_lossy().to_string(),
    ).unwrap();

    assert!(result.is_empty(), "输出文件不存在时不应返回可删除项");
}
