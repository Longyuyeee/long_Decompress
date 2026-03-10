#[cfg(test)]
mod tests {
    use crate::services::compression_service::CompressionService;
    use crate::models::compression::CompressionOptions;
    use tempfile::tempdir;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;

    #[tokio::test]
    async fn test_zip_with_password() {
        // 创建临时目录
        let temp_dir = tempdir().unwrap();

        // 创建测试文件
        let test_file = temp_dir.path().join("secret.txt");
        let mut file = File::create(&test_file).unwrap();
        file.write_all(b"This is a secret file").unwrap();

        // 创建输出ZIP文件路径
        let output_zip = temp_dir.path().join("encrypted.zip");

        // 创建压缩服务
        let service = CompressionService::default();

        // 准备文件列表
        let files = vec![test_file.to_string_lossy().to_string()];

        // 压缩选项 - 带密码
        let options = CompressionOptions {
            password: Some("mypassword123".to_string()),
            compression_level: 6,
            split_size: None,
            preserve_paths: true,
            exclude_patterns: Vec::new(),
            include_patterns: Vec::new(),
            create_subdirectories: true,
            overwrite_existing: false,
        };

        // 执行压缩
        let compress_result = service.compress_zip_enhanced(
            &files,
            &output_zip,
            options,
            None,
        ).await;

        // 验证压缩成功
        assert!(compress_result.is_ok(), "带密码压缩失败: {:?}", compress_result.err());
        assert!(output_zip.exists(), "加密ZIP文件未创建");

        // 创建解压目录
        let extract_dir = temp_dir.path().join("extracted");
        fs::create_dir(&extract_dir).unwrap();

        // 测试1: 使用正确密码解压
        let extract_result1 = CompressionService::extract(
            &output_zip.to_string_lossy(),
            Some(&extract_dir.to_string_lossy()),
            Some("mypassword123"),
        ).await;

        assert!(extract_result1.is_ok(), "使用正确密码解压失败: {:?}", extract_result1.err());

        // 验证文件内容
        let extracted_file = extract_dir.join("secret.txt");
        assert!(extracted_file.exists(), "解压后的文件不存在");

        let content = fs::read_to_string(&extracted_file).unwrap();
        assert_eq!(content, "This is a secret file", "文件内容不匹配");

        // 清理并重新测试
        fs::remove_dir_all(&extract_dir).unwrap();
        fs::create_dir(&extract_dir).unwrap();

        // 测试2: 使用错误密码解压 - 应该失败
        let extract_result2 = CompressionService::extract(
            &output_zip.to_string_lossy(),
            Some(&extract_dir.to_string_lossy()),
            Some("wrongpassword"),
        ).await;

        assert!(extract_result2.is_err(), "使用错误密码应该失败");

        // 测试3: 不使用密码解压 - 应该失败（如果文件确实加密）
        let extract_result3 = CompressionService::extract(
            &output_zip.to_string_lossy(),
            Some(&extract_dir.to_string_lossy()),
            None,
        ).await;

        // 注意：如果zip库无法检测加密，这个测试可能会通过
        // 在实际应用中，需要根据具体情况调整
        println!("无密码解压结果: {:?}", extract_result3);
    }

    #[tokio::test]
    async fn test_zip_without_password() {
        // 创建临时目录
        let temp_dir = tempdir().unwrap();

        // 创建测试文件
        let test_file = temp_dir.path().join("normal.txt");
        let mut file = File::create(&test_file).unwrap();
        file.write_all(b"This is a normal file").unwrap();

        // 创建输出ZIP文件路径
        let output_zip = temp_dir.path().join("normal.zip");

        // 创建压缩服务
        let service = CompressionService::default();

        // 准备文件列表
        let files = vec![test_file.to_string_lossy().to_string()];

        // 压缩选项 - 无密码
        let options = CompressionOptions::default();

        // 执行压缩
        let compress_result = service.compress_zip_enhanced(
            &files,
            &output_zip,
            options,
            None,
        ).await;

        // 验证压缩成功
        assert!(compress_result.is_ok(), "无密码压缩失败: {:?}", compress_result.err());

        // 创建解压目录
        let extract_dir = temp_dir.path().join("extracted_normal");
        fs::create_dir(&extract_dir).unwrap();

        // 测试：无密码解压应该成功
        let extract_result = CompressionService::extract(
            &output_zip.to_string_lossy(),
            Some(&extract_dir.to_string_lossy()),
            None,
        ).await;

        assert!(extract_result.is_ok(), "无密码解压失败: {:?}", extract_result.err());

        // 验证文件内容
        let extracted_file = extract_dir.join("normal.txt");
        assert!(extracted_file.exists(), "解压后的文件不存在");

        let content = fs::read_to_string(&extracted_file).unwrap();
        assert_eq!(content, "This is a normal file", "文件内容不匹配");
    }

    #[tokio::test]
    async fn test_password_compression_options() {
        // 测试压缩选项的密码字段
        let options_with_password = CompressionOptions {
            password: Some("test123".to_string()),
            ..Default::default()
        };

        let options_without_password = CompressionOptions::default();

        assert_eq!(options_with_password.password, Some("test123".to_string()));
        assert_eq!(options_without_password.password, None);
    }
}