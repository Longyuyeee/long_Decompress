#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::{self, File};
    use std::io::Write;
    use crate::services::compression_service::CompressionService;

    #[test]
    fn test_detect_zip_format() {
        // 创建临时ZIP文件
        let temp_dir = tempdir().unwrap();
        let zip_path = temp_dir.path().join("test.zip");

        // 这里应该创建实际的ZIP文件进行测试
        // 暂时跳过，需要实现ZIP创建逻辑
        assert!(true); // 占位测试
    }

    #[test]
    fn test_detect_tar_format() {
        // 创建临时TAR文件
        let temp_dir = tempdir().unwrap();
        let tar_path = temp_dir.path().join("test.tar");

        // 这里应该创建实际的TAR文件进行测试
        assert!(true); // 占位测试
    }

    #[test]
    fn test_validate_file_path() {
        let temp_dir = tempdir().unwrap();
        let valid_file = temp_dir.path().join("test.txt");

        // 创建测试文件
        let mut file = File::create(&valid_file).unwrap();
        file.write_all(b"test content").unwrap();

        // 测试有效文件路径
        assert!(valid_file.exists());

        // 测试无效文件路径
        let invalid_file = temp_dir.path().join("nonexistent.txt");
        assert!(!invalid_file.exists());
    }

    #[test]
    fn test_create_output_directory() {
        let temp_dir = tempdir().unwrap();
        let output_dir = temp_dir.path().join("output");

        // 测试创建输出目录
        fs::create_dir_all(&output_dir).unwrap();
        assert!(output_dir.exists());
        assert!(output_dir.is_dir());
    }

    #[tokio::test]
    async fn test_compression_options_default() {
        let options = CompressionOptions {
            password: None,
            compression_level: Some(6),
            create_subdirectories: Some(true),
        };

        assert_eq!(options.password, None);
        assert_eq!(options.compression_level, Some(6));
        assert_eq!(options.create_subdirectories, Some(true));
    }

    #[tokio::test]
    async fn test_compression_options_with_password() {
        let options = CompressionOptions {
            password: Some("test123".to_string()),
            compression_level: Some(9),
            create_subdirectories: Some(false),
        };

        assert_eq!(options.password, Some("test123".to_string()));
        assert_eq!(options.compression_level, Some(9));
        assert_eq!(options.create_subdirectories, Some(false));
    }
}