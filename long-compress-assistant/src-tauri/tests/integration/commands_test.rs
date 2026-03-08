#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::file::*;
    use crate::commands::system::*;
    use tempfile::tempdir;
    use std::fs::{self, File};
    use std::io::Write;

    // 测试文件命令

    #[tokio::test]
    async fn test_check_file_exists() {
        let temp_dir = tempdir().unwrap();

        // 测试存在的文件
        let existing_file = temp_dir.path().join("test.txt");
        File::create(&existing_file).unwrap();

        let result = check_file_exists(existing_file.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);

        // 测试不存在的文件
        let non_existing_file = temp_dir.path().join("nonexistent.txt");
        let result = check_file_exists(non_existing_file.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);

        // 测试存在的目录
        let existing_dir = temp_dir.path().join("subdir");
        fs::create_dir(&existing_dir).unwrap();

        let result = check_file_exists(existing_dir.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[tokio::test]
    async fn test_detect_file_type() {
        let temp_dir = tempdir().unwrap();

        // 测试目录
        let dir_path = temp_dir.path().join("test_dir");
        fs::create_dir(&dir_path).unwrap();

        let result = detect_file_type(dir_path.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "directory");

        // 测试文本文件
        let text_file = temp_dir.path().join("test.txt");
        File::create(&text_file).unwrap();

        let result = detect_file_type(text_file.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "text");

        // 测试JSON文件
        let json_file = temp_dir.path().join("data.json");
        File::create(&json_file).unwrap();

        let result = detect_file_type(json_file.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "text");

        // 测试图片文件
        let image_file = temp_dir.path().join("image.jpg");
        File::create(&image_file).unwrap();

        let result = detect_file_type(image_file.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "image");

        // 测试压缩文件
        let archive_file = temp_dir.path().join("archive.zip");
        File::create(&archive_file).unwrap();

        let result = detect_file_type(archive_file.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "archive");

        // 测试PDF文件
        let pdf_file = temp_dir.path().join("document.pdf");
        File::create(&pdf_file).unwrap();

        let result = detect_file_type(pdf_file.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "pdf");

        // 测试不存在的文件
        let non_existing = temp_dir.path().join("nonexistent.xyz");
        let result = detect_file_type(non_existing.to_string_lossy().to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("文件不存在"));

        // 测试未知文件类型
        let unknown_file = temp_dir.path().join("unknown.xyz");
        File::create(&unknown_file).unwrap();

        let result = detect_file_type(unknown_file.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "unknown");
    }

    #[tokio::test]
    async fn test_detect_file_type_various_extensions() {
        let temp_dir = tempdir().unwrap();

        let test_cases = vec![
            ("test.txt", "text"),
            ("readme.md", "text"),
            ("data.json", "text"),
            ("config.yaml", "text"),
            ("config.yml", "text"),
            ("photo.jpg", "image"),
            ("photo.jpeg", "image"),
            ("image.png", "image"),
            ("animation.gif", "image"),
            ("picture.bmp", "image"),
            ("vector.svg", "image"),
            ("music.mp3", "audio"),
            ("sound.wav", "audio"),
            ("audio.flac", "audio"),
            ("audio.ogg", "audio"),
            ("video.mp4", "video"),
            ("movie.avi", "video"),
            ("film.mkv", "video"),
            ("video.mov", "video"),
            ("archive.zip", "archive"),
            ("compressed.rar", "archive"),
            ("data.7z", "archive"),
            ("backup.tar", "archive"),
            ("compressed.gz", "archive"),
            ("archive.bz2", "archive"),
            ("data.xz", "archive"),
            ("document.pdf", "pdf"),
            ("doc.doc", "document"),
            ("doc.docx", "document"),
            ("data.xls", "spreadsheet"),
            ("data.xlsx", "spreadsheet"),
            ("presentation.ppt", "presentation"),
            ("presentation.pptx", "presentation"),
            ("program.exe", "executable"),
            ("installer.msi", "executable"),
        ];

        for (filename, expected_type) in test_cases {
            let file_path = temp_dir.path().join(filename);
            File::create(&file_path).unwrap();

            let result = detect_file_type(file_path.to_string_lossy().to_string()).await;
            assert!(result.is_ok(), "Failed for {}: {:?}", filename, result);
            assert_eq!(
                result.unwrap(),
                expected_type,
                "Wrong type for {}",
                filename
            );
        }
    }

    // 测试系统命令

    #[tokio::test]
    async fn test_get_disk_space() {
        let temp_dir = tempdir().unwrap();
        let test_path = temp_dir.path().to_string_lossy().to_string();

        // 测试存在的路径
        let result = get_disk_space(test_path.clone()).await;
        assert!(result.is_ok());

        let (total, free) = result.unwrap();
        // 根据测试实现，这里返回的是固定值
        assert_eq!(total, 1024 * 1024 * 1024 * 100); // 100GB
        assert_eq!(free, 1024 * 1024 * 1024 * 50);   // 50GB

        // 测试不存在的路径
        let non_existing_path = temp_dir.path().join("nonexistent").to_string_lossy().to_string();
        let result = get_disk_space(non_existing_path).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("获取磁盘空间失败"));
    }

    #[tokio::test]
    async fn test_get_app_version() {
        let result = get_app_version().await;
        assert!(result.is_ok());

        let version = result.unwrap();
        // 版本号应该符合语义化版本规范
        assert!(!version.is_empty());
        assert!(version.contains('.'));

        // 检查是否是有效的版本号格式
        let parts: Vec<&str> = version.split('.').collect();
        assert!(parts.len() >= 2);

        // 主要版本号应该是数字
        assert!(parts[0].chars().all(|c| c.is_ascii_digit()));
    }

    // 测试错误处理

    #[tokio::test]
    async fn test_file_commands_error_handling() {
        // 测试不存在的路径
        let non_existing = "/this/path/does/not/exist/xyz.txt".to_string();

        let exists_result = check_file_exists(non_existing.clone()).await;
        assert!(exists_result.is_ok());
        assert_eq!(exists_result.unwrap(), false);

        let type_result = detect_file_type(non_existing).await;
        assert!(type_result.is_err());
        assert!(type_result.unwrap_err().contains("文件不存在"));
    }

    #[tokio::test]
    async fn test_special_file_names() {
        let temp_dir = tempdir().unwrap();

        // 测试带空格的文件名
        let file_with_spaces = temp_dir.path().join("file with spaces.txt");
        File::create(&file_with_spaces).unwrap();

        let result = detect_file_type(file_with_spaces.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "text");

        // 测试带特殊字符的文件名
        let special_file = temp_dir.path().join("file!@#$%^&().txt");
        File::create(&special_file).unwrap();

        let result = detect_file_type(special_file.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "text");

        // 测试中文文件名
        let chinese_file = temp_dir.path().join("中文文件.txt");
        File::create(&chinese_file).unwrap();

        let result = detect_file_type(chinese_file.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "text");
    }

    #[tokio::test]
    async fn test_case_sensitivity() {
        let temp_dir = tempdir().unwrap();

        // 测试大小写扩展名
        let uppercase_file = temp_dir.path().join("test.TXT");
        File::create(&uppercase_file).unwrap();

        let result = detect_file_type(uppercase_file.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "text");

        let mixed_case_file = temp_dir.path().join("test.JpG");
        File::create(&mixed_case_file).unwrap();

        let result = detect_file_type(mixed_case_file.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "image");

        let uppercase_archive = temp_dir.path().join("archive.ZIP");
        File::create(&uppercase_archive).unwrap();

        let result = detect_file_type(uppercase_archive.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "archive");
    }

    #[tokio::test]
    async fn test_multiple_dots_in_filename() {
        let temp_dir = tempdir().unwrap();

        // 测试多个点的文件名
        let multi_dot_file = temp_dir.path().join("archive.tar.gz");
        File::create(&multi_dot_file).unwrap();

        let result = detect_file_type(multi_dot_file.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        // 注意：当前实现只检查最后一个扩展名，所以.gz被识别为archive
        assert_eq!(result.unwrap(), "archive");

        let versioned_file = temp_dir.path().join("file.v1.2.3.txt");
        File::create(&versioned_file).unwrap();

        let result = detect_file_type(versioned_file.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "text");
    }

    #[tokio::test]
    async fn test_no_extension_files() {
        let temp_dir = tempdir().unwrap();

        // 测试没有扩展名的文件
        let no_extension = temp_dir.path().join("README");
        File::create(&no_extension).unwrap();

        let result = detect_file_type(no_extension.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "unknown");

        // 测试以点开头的文件（隐藏文件）
        let hidden_file = temp_dir.path().join(".gitignore");
        File::create(&hidden_file).unwrap();

        let result = detect_file_type(hidden_file.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        // .gitignore没有扩展名，所以是unknown
        assert_eq!(result.unwrap(), "unknown");
    }

    // 集成测试：多个命令组合使用

    #[tokio::test]
    async fn test_integrated_file_operations() {
        let temp_dir = tempdir().unwrap();

        // 创建测试文件
        let test_file = temp_dir.path().join("integration_test.txt");
        File::create(&test_file).unwrap();

        // 1. 检查文件是否存在
        let exists_result = check_file_exists(test_file.to_string_lossy().to_string()).await;
        assert!(exists_result.is_ok());
        assert!(exists_result.unwrap());

        // 2. 检测文件类型
        let type_result = detect_file_type(test_file.to_string_lossy().to_string()).await;
        assert!(type_result.is_ok());
        assert_eq!(type_result.unwrap(), "text");

        // 3. 检查不存在的文件
        let non_existing = temp_dir.path().join("does_not_exist.txt");
        let non_exists_result = check_file_exists(non_existing.to_string_lossy().to_string()).await;
        assert!(non_exists_result.is_ok());
        assert!(!non_exists_result.unwrap());

        // 4. 检测不存在的文件类型（应该失败）
        let non_type_result = detect_file_type(non_existing.to_string_lossy().to_string()).await;
        assert!(non_type_result.is_err());
    }
}