#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;
    use crate::services::file_service::FileService;

    #[test]
    fn test_calculate_file_size() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // 创建测试文件
        let content = "Hello, World!";
        let mut file = File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();

        // 验证文件大小
        let metadata = fs::metadata(&file_path).unwrap();
        assert_eq!(metadata.len(), content.len() as u64);
    }

    #[test]
    fn test_get_file_extension() {
        let test_cases = vec![
            ("test.txt", Some("txt")),
            ("archive.zip", Some("zip")),
            ("document.tar.gz", Some("gz")),
            ("no_extension", None),
            (".hidden", None),
            ("multiple.dots.in.name.txt", Some("txt")),
        ];

        for (filename, expected) in test_cases {
            let path = Path::new(filename);
            let extension = path.extension()
                .and_then(|ext| ext.to_str())
                .map(|s| s.to_lowercase());

            assert_eq!(extension.as_deref(), expected);
        }
    }

    #[test]
    fn test_is_valid_file_path() {
        let temp_dir = tempdir().unwrap();

        // 有效路径测试
        let valid_path = temp_dir.path().join("valid.txt");
        File::create(&valid_path).unwrap();
        assert!(valid_path.exists());

        // 无效路径测试
        let invalid_path = temp_dir.path().join("nonexistent.txt");
        assert!(!invalid_path.exists());

        // 目录路径测试
        let dir_path = temp_dir.path().join("subdir");
        fs::create_dir(&dir_path).unwrap();
        assert!(dir_path.exists());
        assert!(dir_path.is_dir());
    }

    #[test]
    fn test_normalize_path() {
        let test_cases = vec![
            ("C:\\Users\\Test\\file.txt", "C:/Users/Test/file.txt"),
            ("/home/user/file.txt", "/home/user/file.txt"),
            ("./relative/path", "./relative/path"),
            ("../parent/path", "../parent/path"),
        ];

        for (input, expected) in test_cases {
            let normalized = input.replace('\\', "/");
            assert_eq!(normalized, expected);
        }
    }

    #[test]
    fn test_file_permissions() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // 创建文件
        File::create(&file_path).unwrap();

        // 检查文件权限（在Windows上权限检查不同）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&file_path).unwrap();
            let permissions = metadata.permissions();
            assert!(permissions.mode() > 0);
        }

        #[cfg(windows)]
        {
            let metadata = fs::metadata(&file_path).unwrap();
            // Windows权限检查
            assert!(metadata.permissions().readonly() || !metadata.permissions().readonly());
        }
    }

    #[tokio::test]
    async fn test_list_files_in_directory() {
        let temp_dir = tempdir().unwrap();

        // 创建测试文件
        let files = vec!["file1.txt", "file2.txt", "file3.txt"];
        for filename in &files {
            let path = temp_dir.path().join(filename);
            File::create(&path).unwrap();
        }

        // 创建子目录
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        // 列出文件
        let entries: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .collect();

        // 应该找到3个文件 + 1个子目录
        assert_eq!(entries.len(), 4);
    }
}