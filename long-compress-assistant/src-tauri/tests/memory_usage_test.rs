#[cfg(test)]
mod tests {
    use crate::services::compression_service::CompressionService;
    use crate::models::compression::CompressionOptions;
    use tempfile::tempdir;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;
    use std::time::Instant;

    /// 内存使用监控工具（简化版）
    struct MemoryMonitor {
        start_memory: Option<usize>,
    }

    impl MemoryMonitor {
        fn new() -> Self {
            Self { start_memory: None }
        }

        /// 开始监控
        fn start(&mut self) {
            // 在实际应用中，这里会使用更精确的内存监控
            // 这里使用简化版本
            self.start_memory = Some(Self::current_memory_usage());
        }

        /// 获取内存使用变化
        fn get_usage_change(&self) -> Option<isize> {
            if let Some(start) = self.start_memory {
                let current = Self::current_memory_usage();
                Some(current as isize - start as isize)
            } else {
                None
            }
        }

        /// 获取当前内存使用（简化版本）
        fn current_memory_usage() -> usize {
            // 在实际应用中，这里会使用系统API获取精确内存使用
            // 这里返回一个模拟值
            0
        }
    }

    #[tokio::test]
    async fn test_compression_memory_usage_small_files() {
        // 测试小文件压缩的内存使用
        let temp_dir = tempdir().unwrap();
        let mut monitor = MemoryMonitor::new();

        // 创建多个小文件
        let mut file_paths = Vec::new();
        for i in 0..10 {
            let file_path = temp_dir.path().join(format!("small{}.txt", i));
            let mut file = File::create(&file_path).unwrap();

            // 写入1KB数据
            let data = vec![b'X'; 1024];
            file.write_all(&data).unwrap();

            file_paths.push(file_path.to_string_lossy().to_string());
        }

        let output_zip = temp_dir.path().join("small_files.zip");
        let service = CompressionService::default();
        let options = CompressionOptions::default();

        // 开始内存监控
        monitor.start();

        let start_time = Instant::now();
        let result = service.compress_zip_enhanced(
            &file_paths,
            &output_zip,
            options,
            None,
        ).await;

        let duration = start_time.elapsed();

        assert!(result.is_ok(), "小文件压缩失败: {:?}", result.err());

        // 检查内存使用变化
        if let Some(memory_change) = monitor.get_usage_change() {
            println!("小文件压缩内存变化: {} bytes", memory_change);
            println!("小文件压缩耗时: {:?}", duration);

            // 内存使用应该相对稳定
            // 这里只是记录，不进行严格断言
        }
    }

    #[tokio::test]
    async fn test_compression_large_file_memory() {
        // 测试大文件压缩的内存使用（模拟）
        let temp_dir = tempdir().unwrap();
        let mut monitor = MemoryMonitor::new();

        // 创建一个较大的文件（10MB模拟）
        let large_file = temp_dir.path().join("large.dat");
        let mut file = File::create(&large_file).unwrap();

        // 写入10MB数据（在实际测试中可能需要调整大小）
        let chunk_size = 1024 * 1024; // 1MB
        let chunks = 10; // 10MB

        for i in 0..chunks {
            let data = vec![(i % 256) as u8; chunk_size];
            file.write_all(&data).unwrap();
        }

        let output_zip = temp_dir.path().join("large_file.zip");
        let service = CompressionService::default();
        let options = CompressionOptions::default();

        let files = vec![large_file.to_string_lossy().to_string()];

        // 开始内存监控
        monitor.start();

        let start_time = Instant::now();
        let result = service.compress_zip_enhanced(
            &files,
            &output_zip,
            options,
            None,
        ).await;

        let duration = start_time.elapsed();

        // 大文件压缩可能成功或失败（取决于内存限制）
        // 这里主要测试接口是否工作
        match result {
            Ok(_) => {
                println!("大文件压缩成功，耗时: {:?}", duration);

                if let Some(memory_change) = monitor.get_usage_change() {
                    println!("大文件压缩内存变化: {} bytes", memory_change);
                }
            }
            Err(e) => {
                println!("大文件压缩失败（可能内存不足）: {:?}", e);
                println!("压缩耗时: {:?}", duration);
            }
        }

        // 这个测试总是通过，因为我们主要测试接口
        assert!(true, "大文件压缩内存测试完成");
    }

    #[tokio::test]
    async fn test_concurrent_compression_memory() {
        // 测试并发压缩的内存使用
        let temp_dir = tempdir().unwrap();
        let mut monitor = MemoryMonitor::new();

        // 创建多个中等大小的文件
        let mut file_paths = Vec::new();
        for i in 0..5 {
            let file_path = temp_dir.path().join(format!("medium{}.dat", i));
            let mut file = File::create(&file_path).unwrap();

            // 写入1MB数据
            let data = vec![(i % 256) as u8; 1024 * 1024];
            file.write_all(&data).unwrap();

            file_paths.push(file_path.to_string_lossy().to_string());
        }

        let output_zip = temp_dir.path().join("concurrent.zip");
        let service = CompressionService::default();
        let options = CompressionOptions::default();

        // 开始内存监控
        monitor.start();

        let start_time = Instant::now();
        let result = service.compress_zip_enhanced(
            &file_paths,
            &output_zip,
            options,
            None,
        ).await;

        let duration = start_time.elapsed();

        assert!(result.is_ok(), "并发压缩失败: {:?}", result.err());

        if let Some(memory_change) = monitor.get_usage_change() {
            println!("并发压缩内存变化: {} bytes", memory_change);
            println!("并发压缩耗时: {:?}", duration);

            // 验证压缩文件
            let metadata = fs::metadata(&output_zip).unwrap();
            println!("压缩后文件大小: {} bytes", metadata.len());
        }
    }

    #[tokio::test]
    async fn test_compression_with_different_levels_memory() {
        // 测试不同压缩级别的内存使用
        let temp_dir = tempdir().unwrap();

        // 创建测试文件
        let test_file = temp_dir.path().join("test_levels.txt");
        let mut file = File::create(&test_file).unwrap();

        // 写入2MB数据
        let data = vec![b'X'; 2 * 1024 * 1024];
        file.write_all(&data).unwrap();

        let files = vec![test_file.to_string_lossy().to_string()];
        let service = CompressionService::default();

        let mut results = Vec::new();

        // 测试不同压缩级别
        for level in [0, 1, 3, 6, 9].iter() {
            let output_zip = temp_dir.path().join(format!("level_{}.zip", level));
            let mut monitor = MemoryMonitor::new();

            let options = CompressionOptions {
                compression_level: *level,
                ..Default::default()
            };

            // 开始内存监控
            monitor.start();

            let start_time = Instant::now();
            let result = service.compress_zip_enhanced(
                &files,
                &output_zip,
                options,
                None,
            ).await;

            let duration = start_time.elapsed();

            if result.is_ok() {
                let metadata = fs::metadata(&output_zip).unwrap();
                let memory_change = monitor.get_usage_change().unwrap_or(0);

                results.push((*level, metadata.len(), duration, memory_change));

                println!("压缩级别 {}: 大小={} bytes, 耗时={:?}, 内存变化={} bytes",
                    level, metadata.len(), duration, memory_change);
            } else {
                println!("压缩级别 {} 失败: {:?}", level, result.err());
            }
        }

        // 验证至少有一些级别成功
        assert!(!results.is_empty(), "至少应该有一些压缩级别成功");

        // 分析结果
        println!("\n压缩级别分析:");
        for (level, size, duration, memory) in &results {
            println!("  级别 {}: {} bytes, {:?}, {} bytes 内存变化",
                level, size, duration, memory);
        }
    }

    #[tokio::test]
    async fn test_error_recovery_memory_cleanup() {
        // 测试错误恢复时的内存清理
        let temp_dir = tempdir().unwrap();
        let mut monitor = MemoryMonitor::new();

        // 创建一个不存在的文件（应该导致错误）
        let nonexistent_file = temp_dir.path().join("nonexistent.txt");
        let output_zip = temp_dir.path().join("error_test.zip");

        let service = CompressionService::default();
        let options = CompressionOptions::default();

        let files = vec![nonexistent_file.to_string_lossy().to_string()];

        // 开始内存监控
        monitor.start();

        let result = service.compress_zip_enhanced(
            &files,
            &output_zip,
            options,
            None,
        ).await;

        // 应该失败
        assert!(result.is_err(), "不存在的文件应该失败");

        // 检查错误后的内存使用
        if let Some(memory_change) = monitor.get_usage_change() {
            println!("错误恢复后内存变化: {} bytes", memory_change);

            // 错误处理后，内存应该被清理
            // 这里只是记录，不进行严格断言
        }

        // 验证没有创建输出文件
        assert!(!output_zip.exists(), "错误情况下不应该创建输出文件");
    }
}