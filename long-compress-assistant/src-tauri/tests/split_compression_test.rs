use long_compress_assistant::services::split_compression::SplitCompressionService;
use long_compress_assistant::models::compression::CompressionOptions;
use tempfile::tempdir;
use std::fs::{self, File};
use std::io::Write;
use anyhow::Result;

#[tokio::test]
async fn test_split_compression_basic() {
    let temp_dir = tempdir().unwrap();
    let test_file1 = temp_dir.path().join("test1.txt");
    let test_file2 = temp_dir.path().join("test2.txt");

    let mut file1 = File::create(&test_file1).unwrap();
    file1.write_all(b"This is test file 1 content").unwrap();
    let mut file2 = File::create(&test_file2).unwrap();
    file2.write_all(b"This is test file 2 content with more data").unwrap();
    drop(file1);
    drop(file2);

    let output_zip = temp_dir.path().join("split_test.zip");
    let service = SplitCompressionService::new();
    let files = vec![
        test_file1.to_string_lossy().to_string(),
        test_file2.to_string_lossy().to_string(),
    ];
    let options = CompressionOptions {
        split_size: Some(50),
        ..Default::default()
    };

    let result: Result<_> = service.compress_to_split_zips(&files, &output_zip, options).await;
    assert!(result.is_ok(), "分卷压缩失败: {:?}", result.err());
    let result = result.unwrap();
    assert!(result.part_count >= 1, "至少应该有一个分卷");
    assert_eq!(result.part_files.len(), result.part_count, "分卷文件数量不匹配");

    for part_file in &result.part_files {
        assert!(part_file.exists(), "分卷文件不存在: {:?}", part_file);
        let metadata = fs::metadata(part_file).unwrap();
        assert!(metadata.len() > 0, "分卷文件为空: {:?}", part_file);
    }
}

#[tokio::test]
async fn test_split_compression_no_split() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    let mut file = File::create(&test_file).unwrap();
    file.write_all(b"Small test file").unwrap();
    drop(file);

    let output_zip = temp_dir.path().join("no_split.zip");
    let service = SplitCompressionService::new();
    let files = vec![test_file.to_string_lossy().to_string()];
    let options = CompressionOptions {
        split_size: None,
        ..Default::default()
    };

    let result: Result<_> = service.compress_to_split_zips(&files, &output_zip, options).await;
    assert!(result.is_ok(), "不分卷压缩失败: {:?}", result.err());
    let result = result.unwrap();
    assert_eq!(result.part_count, 1, "应该只有一个分卷");
    assert_eq!(result.part_files.len(), 1, "应该只有一个分卷文件");
    assert!(result.part_files[0].extension().unwrap_or_default() == "zip", "应该是.zip文件");
}

#[tokio::test]
async fn test_split_compression_zero_split_size() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    let mut file = File::create(&test_file).unwrap();
    file.write_all(b"Test file").unwrap();
    drop(file);

    let output_zip = temp_dir.path().join("zero_split.zip");
    let service = SplitCompressionService::new();
    let files = vec![test_file.to_string_lossy().to_string()];
    let options = CompressionOptions {
        split_size: Some(0),
        ..Default::default()
    };

    let result: Result<_> = service.compress_to_split_zips(&files, &output_zip, options).await;
    assert!(result.is_ok(), "分卷大小为0时压缩失败: {:?}", result.err());
    let result = result.unwrap();
    assert_eq!(result.part_count, 1, "分卷大小为0时应该只有一个分卷");
}

#[tokio::test]
async fn test_split_compression_large_file() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("large.txt");
    let mut file = File::create(&test_file).unwrap();
    let data = vec![b'X'; 1024];
    file.write_all(&data).unwrap();
    drop(file);

    let output_zip = temp_dir.path().join("large_split.zip");
    let service = SplitCompressionService::new();
    let files = vec![test_file.to_string_lossy().to_string()];
    let options = CompressionOptions {
        split_size: Some(500),
        ..Default::default()
    };

    let result: Result<_> = service.compress_to_split_zips(&files, &output_zip, options).await;
    if let Ok(result) = result {
        assert!(result.part_count >= 1, "至少应该有一个分卷");
    }
}

#[tokio::test]
async fn test_split_compression_nonexistent_file() {
    let temp_dir = tempdir().unwrap();
    let output_zip = temp_dir.path().join("error_test.zip");
    let service = SplitCompressionService::new();
    let files = vec!["nonexistent_file.txt".to_string()];
    let options = CompressionOptions {
        split_size: Some(100),
        ..Default::default()
    };

    let result: Result<_> = service.compress_to_split_zips(&files, &output_zip, options).await;
    assert!(result.is_err(), "不存在的文件应该失败");
}

#[tokio::test]
async fn standard_split_zip_roundtrips_from_first_volume() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("payload.txt");
    fs::write(&source, b"standard split payload").unwrap();
    let output = temp.path().join("standard.zip");
    let service = SplitCompressionService::new();
    let result = service
        .compress_to_split_zips(
            &[source.to_string_lossy().to_string()],
            &output,
            CompressionOptions {
                split_size: Some(10),
                ..Default::default()
            },
        )
        .await
        .expect("create standard split ZIP");

    assert!(result.part_files[0].to_string_lossy().ends_with(".zip.001"));
    let extracted = temp.path().join("extracted");
    let engine = long_compress_assistant::utils::archive_tools::find_7z_command()
        .expect("bundled archive engine");
    let status = long_compress_assistant::utils::process::command(engine)
        .arg("x")
        .arg("-y")
        .arg(format!("-o{}", extracted.display()))
        .arg(&result.part_files[0])
        .status()
        .expect("extract standard split ZIP");
    assert!(status.success());
    assert_eq!(fs::read(extracted.join("payload.txt")).unwrap(), b"standard split payload");
}

#[tokio::test]
async fn preexisting_split_volumes_are_never_deleted_on_validation_failure() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("payload.txt");
    fs::write(&source, b"payload").unwrap();
    let output = temp.path().join("existing.zip");
    let existing = temp.path().join("existing.zip.001");
    fs::write(&existing, b"user data").unwrap();
    let result = SplitCompressionService::new()
        .compress_to_split_zips(
            &[source.to_string_lossy().to_string()],
            &output,
            CompressionOptions {
                split_size: Some(10),
                ..Default::default()
            },
        )
        .await;
    assert!(result.is_err());
    assert_eq!(fs::read(existing).unwrap(), b"user data");
}
