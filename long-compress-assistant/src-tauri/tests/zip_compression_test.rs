#[cfg(test)]
mod tests {
    use crate::services::compression_service::CompressionService;
    use crate::models::compression::CompressionOptions;
    use tempfile::tempdir;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;

    #[tokio::test]
    async fn test_zip_compression_basic() {
        // 创建临时目录
        let temp_dir = tempdir().unwrap();

        // 创建测试文件
        let test_file1 = temp_dir.path().join("test1.txt");
        let test_file2 = temp_dir.path().join("test2.txt");

        let mut file1 = File::create(&test_file1).unwrap();
        file1.write_all(b"This is test file 1 content").unwrap();

        let mut file2 = File::create(&test_file2).unwrap();
        file2.write_all(b"This is test file 2 content with more data").unwrap();

        // 创建输出ZIP文件路径
        let output_zip = temp_dir.path().join("output.zip");

        // 创建压缩服务
        let service = CompressionService::default();

        // 准备文件列表
        let files = vec![
            test_file1.to_string_lossy().to_string(),
            test_file2.to_string_lossy().to_string(),
        ];

        // 压缩选项
        let options = CompressionOptions::default();

        // 执行压缩
        let result = service.compress_zip_enhanced(
            &files,
            &output_zip,
            options,
            None,
        ).await;

        // 验证结果
        assert!(result.is_ok(), "压缩失败: {:?}", result.err());
        assert!(output_zip.exists(), "ZIP文件未创建");
        assert!(output_zip.metadata().unwrap().len() > 0, "ZIP文件为空");

        // 验证ZIP文件内容
        let zip_file = File::open(&output_zip).unwrap();
        let archive = zip::ZipArchive::new(zip_file);
        assert!(archive.is_ok(), "ZIP文件损坏或无法读取");

        let mut archive = archive.unwrap();
        assert_eq!(archive.len(), 2, "ZIP文件应包含2个文件");

        // 检查文件名
        let file_names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();

        assert!(file_names.contains(&"test1.txt".to_string()));
        assert!(file_names.contains(&"test2.txt".to_string()));
    }

    #[tokio::test]
    async fn test_zip_compression_with_directory() {
        // 创建临时目录
        let temp_dir = tempdir().unwrap();

        // 创建子目录和文件
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        let file1 = subdir.join("file1.txt");
        let file2 = subdir.join("file2.txt");

        let mut f1 = File::create(&file1).unwrap();
        f1.write_all(b"File in subdirectory 1").unwrap();

        let mut f2 = File::create(&file2).unwrap();
        f2.write_all(b"File in subdirectory 2").unwrap();

        // 创建输出ZIP文件路径
        let output_zip = temp_dir.path().join("with_dir.zip");

        // 创建压缩服务
        let service = CompressionService::default();

        // 压缩整个目录
        let files = vec![subdir.to_string_lossy().to_string()];
        let options = CompressionOptions::default();

        // 执行压缩
        let result = service.compress_zip_enhanced(
            &files,
            &output_zip,
            options,
            None,
        ).await;

        // 验证结果
        assert!(result.is_ok(), "目录压缩失败: {:?}", result.err());
        assert!(output_zip.exists(), "ZIP文件未创建");

        // 验证ZIP文件内容
        let zip_file = File::open(&output_zip).unwrap();
        let archive = zip::ZipArchive::new(zip_file).unwrap();

        // 应该至少有3个条目：目录本身 + 2个文件
        assert!(archive.len() >= 3, "ZIP文件应包含至少3个条目");
    }

    #[tokio::test]
    async fn test_zip_compression_empty_file_list() {
        let temp_dir = tempdir().unwrap();
        let output_zip = temp_dir.path().join("empty.zip");

        let service = CompressionService::default();
        let files: Vec<String> = vec![];
        let options = CompressionOptions::default();

        let result = service.compress_zip_enhanced(
            &files,
            &output_zip,
            options,
            None,
        ).await;

        // 空文件列表应该失败
        assert!(result.is_err(), "空文件列表应该失败");

        if let Err(err) = result {
            let err_str = format!("{}", err);
            assert!(err_str.contains("文件列表为空"), "错误信息不正确: {}", err_str);
        }
    }

    #[tokio::test]
    async fn test_zip_compression_nonexistent_file() {
        let temp_dir = tempdir().unwrap();
        let output_zip = temp_dir.path().join("test.zip");

        let service = CompressionService::default();
        let files = vec!["nonexistent_file.txt".to_string()];
        let options = CompressionOptions::default();

        let result = service.compress_zip_enhanced(
            &files,
            &output_zip,
            options,
            None,
        ).await;

        // 不存在的文件应该失败
        assert!(result.is_err(), "不存在的文件应该失败");

        if let Err(err) = result {
            let err_str = format!("{}", err);
            assert!(err_str.contains("文件不存在"), "错误信息不正确: {}", err_str);
        }
    }

    #[tokio::test]
    async fn test_zip_compression_overwrite_existing() {
        let temp_dir = tempdir().unwrap();

        // 先创建一个ZIP文件
        let output_zip = temp_dir.path().join("test.zip");
        let test_file = temp_dir.path().join("test.txt");

        let mut file = File::create(&test_file).unwrap();
        file.write_all(b"Test content").unwrap();

        let service = CompressionService::default();
        let files = vec![test_file.to_string_lossy().to_string()];

        // 第一次压缩 - 应该成功
        let options1 = CompressionOptions {
            overwrite_existing: false,
            ..Default::default()
        };

        let result1 = service.compress_zip_enhanced(
            &files,
            &output_zip,
            options1,
            None,
        ).await;

        assert!(result1.is_ok(), "第一次压缩应该成功");

        // 第二次压缩，不允许覆盖 - 应该失败
        let result2 = service.compress_zip_enhanced(
            &files,
            &output_zip,
            options1,
            None,
        ).await;

        assert!(result2.is_err(), "不允许覆盖时应该失败");

        // 第三次压缩，允许覆盖 - 应该成功
        let options2 = CompressionOptions {
            overwrite_existing: true,
            ..Default::default()
        };

        let result3 = service.compress_zip_enhanced(
            &files,
            &output_zip,
            options2,
            None,
        ).await;

        assert!(result3.is_ok(), "允许覆盖时应该成功");
    }
}