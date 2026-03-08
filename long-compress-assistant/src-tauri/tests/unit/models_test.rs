#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::compression::*;
    use chrono::{DateTime, Utc};

    #[test]
    fn test_compression_format_from_extension() {
        // 测试有效的扩展名
        assert_eq!(
            CompressionFormat::from_extension("zip"),
            Some(CompressionFormat::Zip)
        );
        assert_eq!(
            CompressionFormat::from_extension("tar.gz"),
            Some(CompressionFormat::TarGz)
        );
        assert_eq!(
            CompressionFormat::from_extension("7z"),
            Some(CompressionFormat::SevenZip)
        );
        assert_eq!(
            CompressionFormat::from_extension("rar"),
            Some(CompressionFormat::Rar)
        );

        // 测试无效的扩展名
        assert_eq!(CompressionFormat::from_extension("unknown"), None);
        assert_eq!(CompressionFormat::from_extension(""), None);
    }

    #[test]
    fn test_compression_format_extension() {
        assert_eq!(CompressionFormat::Zip.extension(), "zip");
        assert_eq!(CompressionFormat::TarGz.extension(), "tar.gz");
        assert_eq!(CompressionFormat::SevenZip.extension(), "7z");
        assert_eq!(CompressionFormat::Rar.extension(), "rar");
    }

    #[test]
    fn test_compression_format_name() {
        assert_eq!(CompressionFormat::Zip.name(), "ZIP");
        assert_eq!(CompressionFormat::TarGz.name(), "TAR.GZ");
        assert_eq!(CompressionFormat::SevenZip.name(), "7-Zip");
        assert_eq!(CompressionFormat::Rar.name(), "RAR");
    }

    #[test]
    fn test_compression_format_supports_password() {
        // 支持密码的格式
        assert!(CompressionFormat::Zip.supports_password());
        assert!(CompressionFormat::SevenZip.supports_password());
        assert!(CompressionFormat::Rar.supports_password());

        // 不支持密码的格式
        assert!(!CompressionFormat::Tar.supports_password());
        assert!(!CompressionFormat::Gz.supports_password());
        assert!(!CompressionFormat::TarGz.supports_password());
    }

    #[test]
    fn test_compression_format_supports_compression_level() {
        // 支持压缩级别的格式
        assert!(CompressionFormat::Zip.supports_compression_level());
        assert!(CompressionFormat::Gz.supports_compression_level());
        assert!(CompressionFormat::SevenZip.supports_compression_level());

        // 不支持压缩级别的格式
        assert!(!CompressionFormat::Tar.supports_compression_level());
        assert!(!CompressionFormat::Rar.supports_compression_level());
    }

    #[test]
    fn test_compression_options_default() {
        let options = CompressionOptions::default();

        assert_eq!(options.password, None);
        assert_eq!(options.compression_level, 6);
        assert_eq!(options.split_size, None);
        assert_eq!(options.preserve_paths, true);
        assert_eq!(options.exclude_patterns, Vec::<String>::new());
        assert_eq!(options.include_patterns, Vec::<String>::new());
        assert_eq!(options.create_subdirectories, true);
        assert_eq!(options.overwrite_existing, false);
    }

    #[test]
    fn test_compression_status_is_active() {
        // 活跃状态
        assert!(CompressionStatus::Preparing.is_active());
        assert!(CompressionStatus::Compressing.is_active());
        assert!(CompressionStatus::Extracting.is_active());
        assert!(CompressionStatus::Finalizing.is_active());

        // 非活跃状态
        assert!(!CompressionStatus::Pending.is_active());
        assert!(!CompressionStatus::Completed.is_active());
        assert!(!CompressionStatus::Failed.is_active());
        assert!(!CompressionStatus::Cancelled.is_active());
    }

    #[test]
    fn test_compression_status_is_finished() {
        // 完成状态
        assert!(CompressionStatus::Completed.is_finished());
        assert!(CompressionStatus::Failed.is_finished());
        assert!(CompressionStatus::Cancelled.is_finished());

        // 未完成状态
        assert!(!CompressionStatus::Pending.is_finished());
        assert!(!CompressionStatus::Preparing.is_finished());
        assert!(!CompressionStatus::Compressing.is_finished());
        assert!(!CompressionStatus::Extracting.is_finished());
        assert!(!CompressionStatus::Finalizing.is_finished());
    }

    #[test]
    fn test_compression_task_new() {
        let source_files = vec!["file1.txt".to_string(), "file2.txt".to_string()];
        let output_path = "output.zip".to_string();
        let format = CompressionFormat::Zip;
        let options = CompressionOptions::default();

        let task = CompressionTask::new(
            source_files.clone(),
            output_path.clone(),
            format.clone(),
            options.clone(),
        );

        // 验证基本属性
        assert!(!task.id.is_empty());
        assert_eq!(task.source_files, source_files);
        assert_eq!(task.output_path, output_path);
        assert_eq!(task.format, format);
        assert_eq!(task.options.password, options.password);
        assert_eq!(task.options.compression_level, options.compression_level);

        // 验证默认状态
        assert_eq!(task.status, CompressionStatus::Pending);
        assert_eq!(task.progress, 0.0);
        assert!(task.created_at <= Utc::now());
        assert_eq!(task.started_at, None);
        assert_eq!(task.completed_at, None);
        assert_eq!(task.error_message, None);
        assert_eq!(task.total_size, 0);
        assert_eq!(task.processed_size, 0);
    }

    #[test]
    fn test_compression_task_update_progress() {
        let mut task = CompressionTask::new(
            vec!["test.txt".to_string()],
            "output.zip".to_string(),
            CompressionFormat::Zip,
            CompressionOptions::default(),
        );

        // 更新进度
        task.update_progress(500, 1000);

        assert_eq!(task.processed_size, 500);
        assert_eq!(task.total_size, 1000);
        assert_eq!(task.progress, 50.0);

        // 测试除零情况
        task.update_progress(0, 0);
        assert_eq!(task.progress, 0.0);
    }

    #[test]
    fn test_compression_task_start() {
        let mut task = CompressionTask::new(
            vec!["test.txt".to_string()],
            "output.zip".to_string(),
            CompressionFormat::Zip,
            CompressionOptions::default(),
        );

        task.start();

        assert_eq!(task.status, CompressionStatus::Preparing);
        assert!(task.started_at.is_some());
        assert!(task.started_at.unwrap() <= Utc::now());
    }

    #[test]
    fn test_compression_task_complete_success() {
        let mut task = CompressionTask::new(
            vec!["test.txt".to_string()],
            "output.zip".to_string(),
            CompressionFormat::Zip,
            CompressionOptions::default(),
        );

        task.start();
        task.complete(true, None);

        assert_eq!(task.status, CompressionStatus::Completed);
        assert_eq!(task.progress, 100.0);
        assert_eq!(task.error_message, None);
        assert!(task.completed_at.is_some());
        assert!(task.completed_at.unwrap() >= task.started_at.unwrap());
    }

    #[test]
    fn test_compression_task_complete_failure() {
        let mut task = CompressionTask::new(
            vec!["test.txt".to_string()],
            "output.zip".to_string(),
            CompressionFormat::Zip,
            CompressionOptions::default(),
        );

        let error_message = "磁盘空间不足".to_string();
        task.start();
        task.complete(false, Some(error_message.clone()));

        assert_eq!(task.status, CompressionStatus::Failed);
        assert_eq!(task.error_message, Some(error_message));
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_compression_task_cancel() {
        let mut task = CompressionTask::new(
            vec!["test.txt".to_string()],
            "output.zip".to_string(),
            CompressionFormat::Zip,
            CompressionOptions::default(),
        );

        task.start();
        task.cancel();

        assert_eq!(task.status, CompressionStatus::Cancelled);
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_operation_type_equality() {
        assert_eq!(OperationType::Compress, OperationType::Compress);
        assert_eq!(OperationType::Extract, OperationType::Extract);
        assert_ne!(OperationType::Compress, OperationType::Extract);
    }

    #[test]
    fn test_compression_history_creation() {
        let history = CompressionHistory {
            id: "test-id".to_string(),
            task_id: "task-123".to_string(),
            operation_type: OperationType::Compress,
            source_paths: vec!["file1.txt".to_string(), "file2.txt".to_string()],
            output_path: "output.zip".to_string(),
            format: CompressionFormat::Zip,
            size_before: 1024,
            size_after: 512,
            compression_ratio: 0.5,
            duration_seconds: 2.5,
            created_at: Utc::now(),
            success: true,
            error_message: None,
        };

        assert_eq!(history.id, "test-id");
        assert_eq!(history.task_id, "task-123");
        assert_eq!(history.operation_type, OperationType::Compress);
        assert_eq!(history.source_paths.len(), 2);
        assert_eq!(history.output_path, "output.zip");
        assert_eq!(history.format, CompressionFormat::Zip);
        assert_eq!(history.size_before, 1024);
        assert_eq!(history.size_after, 512);
        assert_eq!(history.compression_ratio, 0.5);
        assert_eq!(history.duration_seconds, 2.5);
        assert!(history.success);
        assert_eq!(history.error_message, None);
    }

    #[test]
    fn test_compression_statistics_creation() {
        let stats = CompressionStatistics {
            total_operations: 100,
            successful_operations: 95,
            failed_operations: 5,
            total_compressed_size: 1024 * 1024 * 100, // 100MB
            total_extracted_size: 1024 * 1024 * 150,  // 150MB
            average_compression_ratio: 0.65,
            most_used_format: CompressionFormat::Zip,
            last_operation_time: Some(Utc::now()),
        };

        assert_eq!(stats.total_operations, 100);
        assert_eq!(stats.successful_operations, 95);
        assert_eq!(stats.failed_operations, 5);
        assert_eq!(stats.total_compressed_size, 1024 * 1024 * 100);
        assert_eq!(stats.total_extracted_size, 1024 * 1024 * 150);
        assert_eq!(stats.average_compression_ratio, 0.65);
        assert_eq!(stats.most_used_format, CompressionFormat::Zip);
        assert!(stats.last_operation_time.is_some());
    }

    #[test]
    fn test_serialization_deserialization() {
        use serde_json;

        // 测试CompressionOptions序列化
        let options = CompressionOptions {
            password: Some("secret".to_string()),
            compression_level: 9,
            split_size: Some(1024 * 1024 * 100), // 100MB
            preserve_paths: true,
            exclude_patterns: vec!["*.tmp".to_string(), "*.log".to_string()],
            include_patterns: vec!["*.txt".to_string(), "*.md".to_string()],
            create_subdirectories: false,
            overwrite_existing: true,
        };

        let serialized = serde_json::to_string(&options).unwrap();
        let deserialized: CompressionOptions = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.password, options.password);
        assert_eq!(deserialized.compression_level, options.compression_level);
        assert_eq!(deserialized.split_size, options.split_size);
        assert_eq!(deserialized.preserve_paths, options.preserve_paths);
        assert_eq!(deserialized.exclude_patterns, options.exclude_patterns);
        assert_eq!(deserialized.include_patterns, options.include_patterns);
        assert_eq!(deserialized.create_subdirectories, options.create_subdirectories);
        assert_eq!(deserialized.overwrite_existing, options.overwrite_existing);
    }

    #[test]
    fn test_compression_format_ordering() {
        // 测试格式比较
        let zip = CompressionFormat::Zip;
        let tar = CompressionFormat::Tar;
        let seven_zip = CompressionFormat::SevenZip;

        // 相同格式比较
        assert_eq!(zip, CompressionFormat::Zip);
        assert_ne!(zip, tar);

        // 不同格式比较
        assert_ne!(zip, seven_zip);
        assert_ne!(tar, seven_zip);
    }
}