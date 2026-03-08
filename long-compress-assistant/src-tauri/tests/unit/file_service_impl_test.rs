#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::file_service::FileService;
    use tempfile::tempdir;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;

    #[tokio::test]
    async fn test_list_files_non_recursive() {
        let temp_dir = tempdir().unwrap();

        // 创建测试文件
        let files = vec!["file1.txt", "file2.txt", "file3.txt"];
        for filename in &files {
            let path = temp_dir.path().join(filename);
            let mut file = File::create(&path).unwrap();
            file.write_all(b"test content").unwrap();
        }

        // 创建子目录（不应该在非递归模式下列出）
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        File::create(subdir.join("subfile.txt")).unwrap();

        // 测试非递归列表
        let result = FileService::list_files(temp_dir.path().to_str().unwrap(), false).await;
        assert!(result.is_ok());

        let file_infos = result.unwrap();
        // 应该只列出直接文件，不包括子目录内容
        assert_eq!(file_infos.len(), 4); // 3个文件 + 1个子目录

        // 验证文件名
        let mut found_files = Vec::new();
        for info in &file_infos {
            found_files.push(info.name.clone());
        }

        assert!(found_files.contains(&"file1.txt".to_string()));
        assert!(found_files.contains(&"file2.txt".to_string()));
        assert!(found_files.contains(&"file3.txt".to_string()));
        assert!(found_files.contains(&"subdir".to_string()));
    }

    #[tokio::test]
    async fn test_list_files_recursive() {
        let temp_dir = tempdir().unwrap();

        // 创建嵌套目录结构
        let dirs = vec![
            temp_dir.path().join("level1"),
            temp_dir.path().join("level1/level2"),
            temp_dir.path().join("level1/level2/level3"),
        ];

        for dir in &dirs {
            fs::create_dir_all(dir).unwrap();
        }

        // 在各个层级创建文件
        let files = vec![
            temp_dir.path().join("root.txt"),
            temp_dir.path().join("level1/file1.txt"),
            temp_dir.path().join("level1/level2/file2.txt"),
            temp_dir.path().join("level1/level2/level3/file3.txt"),
        ];

        for file_path in &files {
            File::create(file_path).unwrap();
        }

        // 测试递归列表
        let result = FileService::list_files(temp_dir.path().to_str().unwrap(), true).await;
        assert!(result.is_ok());

        let file_infos = result.unwrap();
        // 应该包括所有文件和目录
        assert!(file_infos.len() >= 8); // 4个文件 + 4个目录（包括根目录）

        // 验证所有文件都被找到
        let mut found_paths = Vec::new();
        for info in &file_infos {
            found_paths.push(info.path.clone());
        }

        for file_path in &files {
            let path_str = file_path.to_string_lossy().to_string();
            assert!(found_paths.contains(&path_str), "Missing: {}", path_str);
        }
    }

    #[tokio::test]
    async fn test_list_files_invalid_path() {
        // 测试不存在的路径
        let result = FileService::list_files("/this/path/does/not/exist", false).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("目录不存在"));

        // 测试文件路径（不是目录）
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path).unwrap();

        let result = FileService::list_files(file_path.to_str().unwrap(), false).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("路径不是目录"));
    }

    #[tokio::test]
    async fn test_get_file_info() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // 创建测试文件
        let content = "Hello, World!";
        let mut file = File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();

        // 获取文件信息
        let result = FileService::get_file_info(file_path.to_str().unwrap()).await;
        assert!(result.is_ok());

        let file_info = result.unwrap();
        assert_eq!(file_info.name, "test.txt");
        assert_eq!(file_info.path, file_path.to_string_lossy().to_string());
        assert_eq!(file_info.size, content.len() as u64);
        assert!(!file_info.is_dir);
        assert_eq!(file_info.extension, Some("txt".to_string()));
        assert!(file_info.modified.is_some());
        assert!(file_info.created.is_some());
    }

    #[tokio::test]
    async fn test_get_file_info_directory() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path().join("test_dir");

        // 创建测试目录
        fs::create_dir(&dir_path).unwrap();

        // 获取目录信息
        let result = FileService::get_file_info(dir_path.to_str().unwrap()).await;
        assert!(result.is_ok());

        let dir_info = result.unwrap();
        assert_eq!(dir_info.name, "test_dir");
        assert_eq!(dir_info.path, dir_path.to_string_lossy().to_string());
        assert_eq!(dir_info.size, 0); // 目录大小为0
        assert!(dir_info.is_dir);
        assert_eq!(dir_info.extension, None); // 目录没有扩展名
        assert!(dir_info.modified.is_some());
        assert!(dir_info.created.is_some());
    }

    #[tokio::test]
    async fn test_get_file_info_nonexistent() {
        let temp_dir = tempdir().unwrap();
        let non_existing = temp_dir.path().join("nonexistent.txt");

        // 测试不存在的文件
        let result = FileService::get_file_info(non_existing.to_str().unwrap()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("文件不存在"));
    }

    #[tokio::test]
    async fn test_get_file_info_special_characters() {
        let temp_dir = tempdir().unwrap();

        // 测试带空格的文件名
        let spaced_file = temp_dir.path().join("file with spaces.txt");
        File::create(&spaced_file).unwrap();

        let result = FileService::get_file_info(spaced_file.to_str().unwrap()).await;
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.name, "file with spaces.txt");
        assert_eq!(info.extension, Some("txt".to_string()));

        // 测试中文文件名
        let chinese_file = temp_dir.path().join("中文文件.txt");
        File::create(&chinese_file).unwrap();

        let result = FileService::get_file_info(chinese_file.to_str().unwrap()).await;
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.name, "中文文件.txt");
        assert_eq!(info.extension, Some("txt".to_string()));

        // 测试特殊字符文件名
        let special_file = temp_dir.path().join("file!@#$%^&().txt");
        File::create(&special_file).unwrap();

        let result = FileService::get_file_info(special_file.to_str().unwrap()).await;
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.name, "file!@#$%^&().txt");
        assert_eq!(info.extension, Some("txt".to_string()));
    }

    #[tokio::test]
    async fn test_get_file_info_hidden_files() {
        let temp_dir = tempdir().unwrap();

        // 在Unix系统上测试隐藏文件
        #[cfg(unix)]
        {
            let hidden_file = temp_dir.path().join(".hidden");
            File::create(&hidden_file).unwrap();

            let result = FileService::get_file_info(hidden_file.to_str().unwrap()).await;
            assert!(result.is_ok());
            let info = result.unwrap();
            assert_eq!(info.name, ".hidden");
            assert_eq!(info.extension, None); // 隐藏文件没有扩展名
        }

        // 测试没有扩展名的文件
        let no_extension = temp_dir.path().join("README");
        File::create(&no_extension).unwrap();

        let result = FileService::get_file_info(no_extension.to_str().unwrap()).await;
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.name, "README");
        assert_eq!(info.extension, None);
    }

    #[tokio::test]
    async fn test_get_file_info_large_file() {
        let temp_dir = tempdir().unwrap();
        let large_file = temp_dir.path().join("large.bin");

        // 创建大文件（1MB）
        let size = 1024 * 1024; // 1MB
        let mut file = File::create(&large_file).unwrap();
        let data = vec![0u8; 1024]; // 1KB数据块

        for _ in 0..1024 {
            file.write_all(&data).unwrap();
        }

        // 获取大文件信息
        let result = FileService::get_file_info(large_file.to_str().unwrap()).await;
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.name, "large.bin");
        assert_eq!(info.size, size as u64);
        assert_eq!(info.extension, Some("bin".to_string()));
    }

    #[tokio::test]
    async fn test_file_info_serialization() {
        use serde_json;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.json");

        File::create(&file_path).unwrap();

        let file_info = FileService::get_file_info(file_path.to_str().unwrap()).await.unwrap();

        // 测试序列化
        let serialized = serde_json::to_string(&file_info).unwrap();
        assert!(!serialized.is_empty());
        assert!(serialized.contains("test.json"));
        assert!(serialized.contains("size"));
        assert!(serialized.contains("is_dir"));

        // 测试反序列化
        let deserialized: crate::services::file_service::FileInfo =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.name, file_info.name);
        assert_eq!(deserialized.size, file_info.size);
        assert_eq!(deserialized.is_dir, file_info.is_dir);
        assert_eq!(deserialized.extension, file_info.extension);
    }

    #[tokio::test]
    async fn test_empty_directory() {
        let temp_dir = tempdir().unwrap();
        let empty_dir = temp_dir.path().join("empty_dir");

        // 创建空目录
        fs::create_dir(&empty_dir).unwrap();

        // 测试非递归列表
        let result = FileService::list_files(empty_dir.to_str().unwrap(), false).await;
        assert!(result.is_ok());
        let files = result.unwrap();
        assert!(files.is_empty());

        // 测试递归列表
        let result = FileService::list_files(empty_dir.to_str().unwrap(), true).await;
        assert!(result.is_ok());
        let files = result.unwrap();
        // 应该只包含目录本身
        assert_eq!(files.len(), 1);
        assert!(files[0].is_dir);
    }

    #[tokio::test]
    async fn test_symlink_handling() {
        let temp_dir = tempdir().unwrap();

        // 创建源文件
        let source_file = temp_dir.path().join("source.txt");
        File::create(&source_file).unwrap();

        // 创建符号链接
        let symlink_path = temp_dir.path().join("link.txt");

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(&source_file, &symlink_path).unwrap();

            // 测试符号链接
            let result = FileService::get_file_info(symlink_path.to_str().unwrap()).await;
            assert!(result.is_ok());
            let info = result.unwrap();
            assert_eq!(info.name, "link.txt");
            // 符号链接应该被识别为文件
            assert!(!info.is_dir);
        }

        #[cfg(windows)]
        {
            use std::os::windows::fs::symlink_file;
            symlink_file(&source_file, &symlink_path).unwrap();

            // Windows上的符号链接测试
            let result = FileService::get_file_info(symlink_path.to_str().unwrap()).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_permission_errors() {
        // 注意：这个测试可能在某些系统上失败，取决于权限设置
        // 我们主要测试错误处理逻辑

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let temp_dir = tempdir().unwrap();
            let restricted_dir = temp_dir.path().join("restricted");

            // 创建目录并设置无读取权限
            fs::create_dir(&restricted_dir).unwrap();
            let mut perms = fs::metadata(&restricted_dir).unwrap().permissions();
            perms.set_mode(0o000); // 无任何权限
            fs::set_permissions(&restricted_dir, perms).unwrap();

            // 尝试列出无权限的目录
            let result = FileService::list_files(restricted_dir.to_str().unwrap(), false).await;
            // 可能会失败，但我们主要关心错误处理
            if result.is_err() {
                let error = result.unwrap_err().to_string();
                assert!(error.contains("读取目录失败") || error.contains("权限"));
            }

            // 恢复权限以便清理
            let mut perms = fs::metadata(&restricted_dir).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&restricted_dir, perms).unwrap();
        }
    }
}