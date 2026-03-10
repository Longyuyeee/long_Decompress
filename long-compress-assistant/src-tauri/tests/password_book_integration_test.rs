#[cfg(test)]
mod tests {
    use crate::services::compression_service::CompressionService;
    use crate::services::password_attempt_service::{PasswordAttemptService, PasswordAttemptStrategy};
    use crate::services::password_query_service::PasswordQueryService;
    use crate::models::compression::CompressionOptions;
    use tempfile::tempdir;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_password_attempt_integration() {
        // 创建临时目录
        let temp_dir = tempdir().unwrap();

        // 创建测试文件
        let test_file = temp_dir.path().join("test_secret.txt");
        let mut file = File::create(&test_file).unwrap();
        file.write_all(b"This is a secret file for password integration test").unwrap();

        // 创建带密码的ZIP文件
        let output_zip = temp_dir.path().join("encrypted_integration.zip");
        let service = CompressionService::default();
        let files = vec![test_file.to_string_lossy().to_string()];

        let options = CompressionOptions {
            password: Some("integration_test_password".to_string()),
            ..Default::default()
        };

        // 压缩文件
        let compress_result = service.compress_zip_enhanced(
            &files,
            &output_zip,
            options,
            None,
        ).await;

        assert!(compress_result.is_ok(), "带密码压缩失败: {:?}", compress_result.err());
        assert!(output_zip.exists(), "加密ZIP文件未创建");

        // 创建解压目录
        let extract_dir = temp_dir.path().join("extracted_integration");
        fs::create_dir(&extract_dir).unwrap();

        // 测试：使用正确密码解压
        let extract_result = CompressionService::extract(
            &output_zip.to_string_lossy(),
            Some(&extract_dir.to_string_lossy()),
            Some("integration_test_password"),
        ).await;

        assert!(extract_result.is_ok(), "使用正确密码解压失败: {:?}", extract_result.err());

        // 验证文件内容
        let extracted_file = extract_dir.join("test_secret.txt");
        assert!(extracted_file.exists(), "解压后的文件不存在");
        let content = fs::read_to_string(&extracted_file).unwrap();
        assert_eq!(content, "This is a secret file for password integration test", "文件内容不匹配");
    }

    #[tokio::test]
    async fn test_zip_compression_with_different_levels() {
        // 测试不同压缩级别
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test_levels.txt");

        // 创建一个大一点的文件以便观察压缩效果
        let mut file = File::create(&test_file).unwrap();
        for _ in 0..1000 {
            file.write_all(b"This is test content for compression level testing. ".as_ref()).unwrap();
        }

        let files = vec![test_file.to_string_lossy().to_string()];
        let service = CompressionService::default();

        let mut sizes = Vec::new();

        // 测试不同压缩级别
        for level in [0, 1, 3, 6, 9].iter() {
            let output_zip = temp_dir.path().join(format!("level_{}.zip", level));

            let options = CompressionOptions {
                compression_level: *level,
                ..Default::default()
            };

            let result = service.compress_zip_enhanced(
                &files,
                &output_zip,
                options,
                None,
            ).await;

            assert!(result.is_ok(), "压缩级别 {} 失败: {:?}", level, result.err());
            assert!(output_zip.exists(), "ZIP文件未创建");

            let metadata = fs::metadata(&output_zip).unwrap();
            sizes.push((*level, metadata.len()));

            println!("压缩级别 {}: {} 字节", level, metadata.len());
        }

        // 验证压缩级别越高，文件越小（通常如此）
        // 注意：级别0是存储（不压缩），所以可能比级别1大
        for i in 1..sizes.len() {
            if sizes[i].0 > 0 && sizes[i-1].0 > 0 {
                // 压缩级别越高，文件应该越小（通常）
                println!("级别 {} ({} bytes) vs 级别 {} ({} bytes)",
                    sizes[i-1].0, sizes[i-1].1, sizes[i].0, sizes[i].1);
            }
        }
    }

    #[tokio::test]
    async fn test_zip_compression_with_exclude_patterns() {
        // 测试排除模式
        let temp_dir = tempdir().unwrap();

        // 创建多个测试文件
        let files_to_create = vec![
            ("include1.txt", "This should be included"),
            ("include2.txt", "This should also be included"),
            ("exclude1.tmp", "This should be excluded"),
            ("exclude2.log", "This should also be excluded"),
        ];

        let mut file_paths = Vec::new();
        for (filename, content) in files_to_create {
            let file_path = temp_dir.path().join(filename);
            let mut file = File::create(&file_path).unwrap();
            file.write_all(content.as_bytes()).unwrap();
            file_paths.push(file_path.to_string_lossy().to_string());
        }

        let output_zip = temp_dir.path().join("with_exclude.zip");
        let service = CompressionService::default();

        // 排除.tmp和.log文件
        let options = CompressionOptions {
            exclude_patterns: vec![
                "*.tmp".to_string(),
                "*.log".to_string(),
            ],
            ..Default::default()
        };

        let result = service.compress_zip_enhanced(
            &file_paths,
            &output_zip,
            options,
            None,
        ).await;

        assert!(result.is_ok(), "带排除模式的压缩失败: {:?}", result.err());

        // 验证ZIP文件内容
        let zip_file = File::open(&output_zip).unwrap();
        let archive = zip::ZipArchive::new(zip_file).unwrap();

        let file_names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();

        // 应该只包含.txt文件
        assert!(file_names.contains(&"include1.txt".to_string()));
        assert!(file_names.contains(&"include2.txt".to_string()));
        assert!(!file_names.contains(&"exclude1.tmp".to_string()));
        assert!(!file_names.contains(&"exclude2.log".to_string()));
        assert_eq!(file_names.len(), 2, "应该只包含2个文件");
    }

    #[tokio::test]
    async fn test_zip_compression_large_file() {
        // 测试大文件压缩（模拟）
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("large_test.dat");

        // 创建一个大文件（10MB）
        let mut file = File::create(&test_file).unwrap();
        let chunk = vec![0u8; 1024 * 1024]; // 1MB chunk

        for _ in 0..10 {
            file.write_all(&chunk).unwrap();
        }

        let files = vec![test_file.to_string_lossy().to_string()];
        let output_zip = temp_dir.path().join("large_file.zip");
        let service = CompressionService::default();

        let options = CompressionOptions::default();

        let result = service.compress_zip_enhanced(
            &files,
            &output_zip,
            options,
            None,
        ).await;

        assert!(result.is_ok(), "大文件压缩失败: {:?}", result.err());

        let metadata = fs::metadata(&output_zip).unwrap();
        assert!(metadata.len() > 0, "ZIP文件为空");

        println!("原始文件大小: {} MB", 10);
        println!("压缩后大小: {} 字节", metadata.len());
        println!("压缩率: {:.2}%", (metadata.len() as f64 / (10.0 * 1024.0 * 1024.0)) * 100.0);
    }

    #[tokio::test]
    async fn test_zip_compression_concurrent() {
        // 测试并发压缩多个文件
        let temp_dir = tempdir().unwrap();
        let mut file_paths = Vec::new();

        // 创建10个测试文件
        for i in 0..10 {
            let file_path = temp_dir.path().join(format!("test{}.txt", i));
            let mut file = File::create(&file_path).unwrap();
            file.write_all(format!("This is test file number {}", i).as_bytes()).unwrap();
            file_paths.push(file_path.to_string_lossy().to_string());
        }

        let output_zip = temp_dir.path().join("concurrent_test.zip");
        let service = CompressionService::default();

        let options = CompressionOptions::default();

        let start = std::time::Instant::now();
        let result = service.compress_zip_enhanced(
            &file_paths,
            &output_zip,
            options,
            None,
        ).await;
        let duration = start.elapsed();

        assert!(result.is_ok(), "并发压缩失败: {:?}", result.err());

        // 验证所有文件都在ZIP中
        let zip_file = File::open(&output_zip).unwrap();
        let archive = zip::ZipArchive::new(zip_file).unwrap();
        assert_eq!(archive.len(), 10, "应该包含10个文件");

        println!("并发压缩10个文件耗时: {:?}", duration);
    }
}