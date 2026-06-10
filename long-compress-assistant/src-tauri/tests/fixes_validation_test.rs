//! 修复验证集成测试
//!
//! 验证本轮审计修复的核心功能：
//! - Zip Slip 路径穿越防护
//! - 分卷压缩写入文件内容
//! - 压缩验证逻辑
//! - 通用引擎覆盖模式
//! - 密码脱敏

use long_compress_assistant::services::split_compression::SplitCompressionService;
use long_compress_assistant::services::compression_service::CompressionService;
use long_compress_assistant::services::universal_engine::UniversalCliEngine;
use long_compress_assistant::models::compression::CompressionOptions;
use long_compress_assistant::utils::file_utils;
use tempfile::tempdir;
use std::io::Write;
use std::fs::File;
use std::path::Path;

/// 验证 normalize_archive_path 阻止路径穿越攻击
#[test]
fn test_path_traversal_prevention() {
    // 基本的 .. 攻击
    assert!(file_utils::normalize_archive_path(Path::new("../../etc/passwd"), true).is_some());
    assert!(file_utils::normalize_archive_path(Path::new(".."), true).is_none());

    // 绝对路径应该被规范化（RootDir 被剥离）
    assert!(file_utils::normalize_archive_path(Path::new("/etc/passwd"), true).is_some());

    // 正常路径应保持不变
    let result = file_utils::normalize_archive_path(Path::new("folder/file.txt"), true);
    assert!(result.is_some());
    let normalized = result.unwrap().to_string_lossy().replace('\\', "/");
    assert_eq!(normalized, "folder/file.txt");

    // 仅文件名模式
    let result = file_utils::normalize_archive_path(Path::new("deep/nested/file.txt"), false);
    assert!(result.is_some());
    assert_eq!(result.unwrap().to_string_lossy(), "file.txt");
}

/// 验证 verify_extract_path 将结果锚定在输出目录内
#[test]
fn test_extract_path_anchoring() {
    let output = Path::new("/tmp/output");

    // 安全路径
    let safe = file_utils::verify_extract_path(Path::new("file.txt"), output, true);
    assert!(safe.is_some());
    assert!(safe.unwrap().starts_with(output));

    // 穿越路径中的 .. 被剥离，结果是安全的锚定路径
    let sanitized = file_utils::verify_extract_path(Path::new("../../etc/passwd"), output, true);
    assert!(sanitized.is_some());
    // 验证结果仍然在输出目录内
    assert!(sanitized.unwrap().starts_with(output));
}

/// 验证分卷压缩确实写入文件内容（而非空壳ZIP）
#[tokio::test]
async fn test_split_compression_writes_content() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("data.txt");
    let mut f = File::create(&source).unwrap();
    f.write_all(b"Hello, split compression! This is test content.").unwrap();

    let output = temp.path().join("archive.zip");
    let service = SplitCompressionService::new();
    let files = vec![source.to_string_lossy().to_string()];
    let options = CompressionOptions {
        split_size: Some(1024 * 1024),
        ..Default::default()
    };

    let result: anyhow::Result<_> = service.compress_to_split_zips(&files, &output, options).await;
    assert!(result.is_ok(), "分卷压缩应成功: {:?}", result.err());

    let result = result.unwrap();
    assert_eq!(result.part_count, 1, "小文件应产生单个分卷");
    assert!(result.part_files[0].exists(), "分卷文件应存在");

    let metadata = std::fs::metadata(&result.part_files[0]).unwrap();
    assert!(metadata.len() > 22, "ZIP文件应有内容（至少含有 local file header）");
}

/// 验证压缩请求的格式校验
#[test]
fn test_compression_validation_rules() {
    // 单文件流格式只支持一个常规文件
    let multi = CompressionService::validate_compression_request(
        &["a.txt".to_string(), "b.txt".to_string()],
        "out.gz",
        &CompressionOptions { format: Some("gz".to_string()), ..Default::default() }
    );
    assert!(multi.is_err(), "多文件 gzip 应被拒绝");

    // ZIP 不支持密码压缩
    let zip_pwd = CompressionService::validate_compression_request(
        &["a.txt".to_string()],
        "out.zip",
        &CompressionOptions { format: Some("zip".to_string()), password: Some("secret".to_string()), ..Default::default() }
    );
    assert!(zip_pwd.is_err(), "ZIP 密码压缩应被拒绝");

    // 7Z 应支持密码压缩
    let sevenz_pwd = CompressionService::validate_compression_request(
        &["a.txt".to_string()],
        "out.7z",
        &CompressionOptions { format: Some("7z".to_string()), password: Some("secret".to_string()), ..Default::default() }
    );
    assert!(sevenz_pwd.is_ok(), "7Z 密码压缩应被支持");
}

/// 验证通用引擎覆盖模式参数
#[test]
fn test_universal_engine_overwrite_modes() {
    assert_eq!(UniversalCliEngine::overwrite_mode_arg(false), "-aou");
    assert_eq!(UniversalCliEngine::overwrite_mode_arg(true), "-aoa");
}

/// 验证压缩完成后源文件的清理逻辑
#[test]
fn test_removable_sources_logic() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("source.txt");
    let output = temp.path().join("archive.zip");
    let extra = temp.path().join("extra.txt");
    File::create(&source).unwrap();
    File::create(&output).unwrap();
    File::create(&extra).unwrap();

    let removable = CompressionService::removable_compressed_sources(
        &[
            source.to_string_lossy().to_string(),
            temp.path().to_string_lossy().to_string(), // directory: skipped
            output.to_string_lossy().to_string(),       // same as output: skipped
            extra.to_string_lossy().to_string(),
        ],
        &output.to_string_lossy().to_string(),
    );
    assert!(removable.is_ok());
    let removable = removable.unwrap();
    // source.txt and extra.txt should be removable; output archive should not
    assert!(removable.iter().any(|p| p.ends_with("source.txt")));
    assert!(removable.iter().any(|p| p.ends_with("extra.txt")));
    assert!(!removable.iter().any(|p| p.ends_with("archive.zip")));
}
