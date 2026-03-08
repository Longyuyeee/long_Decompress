#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::file::*;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tempfile::tempdir;

    #[test]
    fn test_file_entry_from_path() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // 创建测试文件
        std::fs::write(&file_path, "test content").unwrap();

        // 从路径创建FileEntry
        let file_entry = FileEntry::from_path(&file_path).unwrap();

        // 验证基本属性
        assert_eq!(file_entry.name, "test.txt");
        assert!(file_entry.path.contains("test.txt"));
        assert!(file_entry.absolute_path.contains("test.txt"));
        assert_eq!(file_entry.size, 12); // "test content"的长度
        assert!(!file_entry.is_dir);
        assert!(file_entry.is_file);
        assert!(!file_entry.is_symlink);
        assert!(!file_entry.is_hidden); // 不是隐藏文件
        assert_eq!(file_entry.extension, Some("txt".to_string()));

        // 验证权限
        assert!(file_entry.permissions.readable);
        // writable 取决于文件权限，这里不严格验证

        // 验证时间戳存在
        assert!(file_entry.modified.is_some());
        assert!(file_entry.created.is_some());
        assert!(file_entry.accessed.is_some());
    }

    #[test]
    fn test_file_entry_from_directory() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path().join("test_dir");

        // 创建测试目录
        std::fs::create_dir(&dir_path).unwrap();

        // 从目录路径创建FileEntry
        let dir_entry = FileEntry::from_path(&dir_path).unwrap();

        // 验证目录属性
        assert_eq!(dir_entry.name, "test_dir");
        assert!(dir_entry.is_dir);
        assert!(!dir_entry.is_file);
        assert_eq!(dir_entry.size, 0); // 目录大小为0
    }

    #[test]
    fn test_file_entry_get_icon() {
        // 测试目录图标
        let mut dir_entry = FileEntry {
            name: "test".to_string(),
            path: "test".to_string(),
            absolute_path: "test".to_string(),
            size: 0,
            is_dir: true,
            is_file: false,
            is_symlink: false,
            is_hidden: false,
            permissions: FilePermissions {
                readable: true,
                writable: true,
                executable: false,
            },
            modified: None,
            created: None,
            accessed: None,
            extension: None,
            mime_type: None,
            icon: None,
        };

        assert_eq!(dir_entry.get_icon(), "📁");

        // 测试各种文件类型的图标
        let test_cases = vec![
            ("test.txt", "txt", "📄"),
            ("image.jpg", "jpg", "🖼️"),
            ("audio.mp3", "mp3", "🎵"),
            ("video.mp4", "mp4", "🎬"),
            ("archive.zip", "zip", "📦"),
            ("document.pdf", "pdf", "📕"),
            ("document.docx", "docx", "📘"),
            ("spreadsheet.xlsx", "xlsx", "📗"),
            ("presentation.pptx", "pptx", "📙"),
            ("program.exe", "exe", "⚙️"),
            ("page.html", "html", "🌐"),
            ("style.css", "css", "🎨"),
            ("script.js", "js", "📜"),
            ("rust.rs", "rs", "🦀"),
            ("python.py", "py", "🐍"),
            ("java.java", "java", "☕"),
            ("code.cpp", "cpp", "🔧"),
            ("unknown.xyz", "xyz", "📄"), // 默认图标
        ];

        for (name, extension, expected_icon) in test_cases {
            dir_entry.is_dir = false;
            dir_entry.is_file = true;
            dir_entry.name = name.to_string();
            dir_entry.extension = Some(extension.to_string());

            assert_eq!(dir_entry.get_icon(), expected_icon, "Failed for {}", name);
        }
    }

    #[test]
    fn test_file_entry_format_size() {
        let mut file_entry = FileEntry {
            name: "test.txt".to_string(),
            path: "test.txt".to_string(),
            absolute_path: "test.txt".to_string(),
            size: 0,
            is_dir: false,
            is_file: true,
            is_symlink: false,
            is_hidden: false,
            permissions: FilePermissions {
                readable: true,
                writable: true,
                executable: false,
            },
            modified: None,
            created: None,
            accessed: None,
            extension: Some("txt".to_string()),
            mime_type: None,
            icon: None,
        };

        // 测试0字节
        file_entry.size = 0;
        assert_eq!(file_entry.format_size(), "0 B");

        // 测试小文件
        file_entry.size = 500;
        assert_eq!(file_entry.format_size(), "500.00 B");

        // 测试KB
        file_entry.size = 1500;
        assert_eq!(file_entry.format_size(), "1.46 KB");

        // 测试MB
        file_entry.size = 5 * 1024 * 1024; // 5MB
        assert_eq!(file_entry.format_size(), "5.00 MB");

        // 测试GB
        file_entry.size = 2 * 1024 * 1024 * 1024; // 2GB
        assert_eq!(file_entry.format_size(), "2.00 GB");

        // 测试目录
        file_entry.is_dir = true;
        file_entry.is_file = false;
        file_entry.size = 1024 * 1024; // 1MB目录
        assert_eq!(file_entry.format_size(), "文件夹");
    }

    #[test]
    fn test_file_permissions() {
        let permissions = FilePermissions {
            readable: true,
            writable: false,
            executable: true,
        };

        assert!(permissions.readable);
        assert!(!permissions.writable);
        assert!(permissions.executable);
    }

    #[test]
    fn test_directory_info() {
        let now = SystemTime::now();
        let dir_info = DirectoryInfo {
            path: "/test/path".to_string(),
            total_files: 100,
            total_dirs: 10,
            total_size: 1024 * 1024 * 500, // 500MB
            depth: 3,
            modified: Some(now),
        };

        assert_eq!(dir_info.path, "/test/path");
        assert_eq!(dir_info.total_files, 100);
        assert_eq!(dir_info.total_dirs, 10);
        assert_eq!(dir_info.total_size, 1024 * 1024 * 500);
        assert_eq!(dir_info.depth, 3);
        assert_eq!(dir_info.modified, Some(now));
    }

    #[test]
    fn test_file_operation_new() {
        let source_paths = vec!["file1.txt".to_string(), "file2.txt".to_string()];
        let destination_path = Some("dest/".to_string());

        let operation = FileOperation::new(
            FileOperationType::Copy,
            source_paths.clone(),
            destination_path.clone(),
        );

        assert!(!operation.id.is_empty());
        assert_eq!(operation.operation_type, FileOperationType::Copy);
        assert_eq!(operation.source_paths, source_paths);
        assert_eq!(operation.destination_path, destination_path);
        assert_eq!(operation.status, FileOperationStatus::Pending);
        assert_eq!(operation.progress, 0.0);
        assert_eq!(operation.total_size, 0);
        assert_eq!(operation.processed_size, 0);
        assert_eq!(operation.started_at, None);
        assert_eq!(operation.completed_at, None);
        assert_eq!(operation.error_message, None);
    }

    #[test]
    fn test_file_operation_lifecycle() {
        let mut operation = FileOperation::new(
            FileOperationType::Move,
            vec!["source.txt".to_string()],
            Some("dest.txt".to_string()),
        );

        // 开始操作
        operation.start();
        assert_eq!(operation.status, FileOperationStatus::InProgress);
        assert!(operation.started_at.is_some());

        // 更新进度
        operation.update_progress(500, 1000);
        assert_eq!(operation.processed_size, 500);
        assert_eq!(operation.total_size, 1000);
        assert_eq!(operation.progress, 50.0);

        // 完成操作（成功）
        operation.complete(true, None);
        assert_eq!(operation.status, FileOperationStatus::Completed);
        assert_eq!(operation.progress, 100.0);
        assert_eq!(operation.error_message, None);
        assert!(operation.completed_at.is_some());

        // 测试失败情况
        let mut failed_operation = FileOperation::new(
            FileOperationType::Delete,
            vec!["file.txt".to_string()],
            None,
        );

        failed_operation.start();
        failed_operation.complete(false, Some("权限不足".to_string()));

        assert_eq!(failed_operation.status, FileOperationStatus::Failed);
        assert_eq!(failed_operation.error_message, Some("权限不足".to_string()));
    }

    #[test]
    fn test_file_operation_cancel() {
        let mut operation = FileOperation::new(
            FileOperationType::Copy,
            vec!["file.txt".to_string()],
            Some("copy.txt".to_string()),
        );

        operation.start();
        operation.cancel();

        assert_eq!(operation.status, FileOperationStatus::Cancelled);
        assert!(operation.completed_at.is_some());
    }

    #[test]
    fn test_file_operation_type_equality() {
        assert_eq!(FileOperationType::Copy, FileOperationType::Copy);
        assert_eq!(FileOperationType::Move, FileOperationType::Move);
        assert_eq!(FileOperationType::Delete, FileOperationType::Delete);
        assert_eq!(FileOperationType::Rename, FileOperationType::Rename);
        assert_eq!(FileOperationType::CreateDirectory, FileOperationType::CreateDirectory);
        assert_eq!(FileOperationType::CreateFile, FileOperationType::CreateFile);

        assert_ne!(FileOperationType::Copy, FileOperationType::Move);
        assert_ne!(FileOperationType::Delete, FileOperationType::Rename);
    }

    #[test]
    fn test_file_operation_status_equality() {
        assert_eq!(FileOperationStatus::Pending, FileOperationStatus::Pending);
        assert_eq!(FileOperationStatus::InProgress, FileOperationStatus::InProgress);
        assert_eq!(FileOperationStatus::Completed, FileOperationStatus::Completed);
        assert_eq!(FileOperationStatus::Failed, FileOperationStatus::Failed);
        assert_eq!(FileOperationStatus::Cancelled, FileOperationStatus::Cancelled);

        assert_ne!(FileOperationStatus::Pending, FileOperationStatus::Completed);
        assert_ne!(FileOperationStatus::InProgress, FileOperationStatus::Failed);
    }

    #[test]
    fn test_search_criteria() {
        let modified_time = SystemTime::now();
        let criteria = SearchCriteria {
            name_pattern: Some("*.txt".to_string()),
            content_pattern: Some("test".to_string()),
            min_size: Some(1024), // 1KB
            max_size: Some(1024 * 1024), // 1MB
            file_types: vec!["txt".to_string(), "md".to_string()],
            exclude_patterns: vec!["*.tmp".to_string(), "*.log".to_string()],
            modified_after: Some(modified_time),
            modified_before: Some(modified_time),
            recursive: true,
            case_sensitive: false,
        };

        assert_eq!(criteria.name_pattern, Some("*.txt".to_string()));
        assert_eq!(criteria.content_pattern, Some("test".to_string()));
        assert_eq!(criteria.min_size, Some(1024));
        assert_eq!(criteria.max_size, Some(1024 * 1024));
        assert_eq!(criteria.file_types, vec!["txt".to_string(), "md".to_string()]);
        assert_eq!(criteria.exclude_patterns, vec!["*.tmp".to_string(), "*.log".to_string()]);
        assert_eq!(criteria.modified_after, Some(modified_time));
        assert_eq!(criteria.modified_before, Some(modified_time));
        assert!(criteria.recursive);
        assert!(!criteria.case_sensitive);
    }

    #[test]
    fn test_search_result() {
        let file_entry = FileEntry {
            name: "test.txt".to_string(),
            path: "test.txt".to_string(),
            absolute_path: "test.txt".to_string(),
            size: 1024,
            is_dir: false,
            is_file: true,
            is_symlink: false,
            is_hidden: false,
            permissions: FilePermissions {
                readable: true,
                writable: true,
                executable: false,
            },
            modified: None,
            created: None,
            accessed: None,
            extension: Some("txt".to_string()),
            mime_type: None,
            icon: None,
        };

        let match_positions = vec![
            MatchPosition {
                line: 1,
                column: 5,
                length: 4,
                matched_text: "test".to_string(),
            },
            MatchPosition {
                line: 2,
                column: 10,
                length: 6,
                matched_text: "search".to_string(),
            },
        ];

        let result = SearchResult {
            file_entry: file_entry.clone(),
            relevance_score: 0.85,
            match_positions: match_positions.clone(),
        };

        assert_eq!(result.file_entry.name, "test.txt");
        assert_eq!(result.relevance_score, 0.85);
        assert_eq!(result.match_positions.len(), 2);
        assert_eq!(result.match_positions[0].line, 1);
        assert_eq!(result.match_positions[0].matched_text, "test");
        assert_eq!(result.match_positions[1].line, 2);
        assert_eq!(result.match_positions[1].matched_text, "search");
    }

    #[test]
    fn test_file_preview() {
        let preview = FilePreview {
            path: "test.txt".to_string(),
            content: "Hello, World!\nThis is a test.".to_string(),
            truncated: false,
            line_count: 2,
            encoding: "UTF-8".to_string(),
            language: Some("plaintext".to_string()),
        };

        assert_eq!(preview.path, "test.txt");
        assert_eq!(preview.content, "Hello, World!\nThis is a test.");
        assert!(!preview.truncated);
        assert_eq!(preview.line_count, 2);
        assert_eq!(preview.encoding, "UTF-8");
        assert_eq!(preview.language, Some("plaintext".to_string()));
    }

    #[test]
    fn test_serialization_deserialization() {
        use serde_json;

        // 测试FileEntry序列化
        let file_entry = FileEntry {
            name: "test.txt".to_string(),
            path: "/path/to/test.txt".to_string(),
            absolute_path: "/absolute/path/to/test.txt".to_string(),
            size: 1024,
            is_dir: false,
            is_file: true,
            is_symlink: false,
            is_hidden: false,
            permissions: FilePermissions {
                readable: true,
                writable: true,
                executable: false,
            },
            modified: None,
            created: None,
            accessed: None,
            extension: Some("txt".to_string()),
            mime_type: None,
            icon: None,
        };

        let serialized = serde_json::to_string(&file_entry).unwrap();
        let deserialized: FileEntry = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.name, file_entry.name);
        assert_eq!(deserialized.size, file_entry.size);
        assert_eq!(deserialized.is_dir, file_entry.is_dir);
        assert_eq!(deserialized.is_file, file_entry.is_file);
        assert_eq!(deserialized.extension, file_entry.extension);
        assert!(deserialized.permissions.readable);
        assert!(deserialized.permissions.writable);
        assert!(!deserialized.permissions.executable);
    }
}