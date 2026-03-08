//! 测试辅助函数库
//! 提供通用的测试工具函数和模拟数据

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

/// 创建临时测试目录
pub fn create_temp_dir() -> TempDir {
    tempdir().expect("Failed to create temporary directory")
}

/// 创建测试文件
pub fn create_test_file<P: AsRef<Path>>(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
    let file_path = dir.join(name);
    let mut file = File::create(&file_path).expect("Failed to create test file");
    file.write_all(content).expect("Failed to write to test file");
    file_path
}

/// 创建大测试文件
pub fn create_large_test_file<P: AsRef<Path>>(dir: &Path, name: &str, size_mb: usize) -> PathBuf {
    let file_path = dir.join(name);
    let content = vec![0u8; size_mb * 1024 * 1024];
    let mut file = File::create(&file_path).expect("Failed to create large test file");
    file.write_all(&content).expect("Failed to write to large test file");
    file_path
}

/// 创建文本测试文件
pub fn create_text_file<P: AsRef<Path>>(dir: &Path, name: &str, content: &str) -> PathBuf {
    create_test_file(dir, name, content.as_bytes())
}

/// 创建JSON测试文件
pub fn create_json_file<P: AsRef<Path>>(dir: &Path, name: &str, data: &serde_json::Value) -> PathBuf {
    let content = serde_json::to_string_pretty(data).expect("Failed to serialize JSON");
    create_text_file(dir, name, &content)
}

/// 创建ZIP测试文件（模拟）
pub fn create_zip_file<P: AsRef<Path>>(dir: &Path, name: &str) -> PathBuf {
    // 创建一些内容文件
    let content_dir = create_temp_dir();
    create_text_file(content_dir.path(), "file1.txt", "Content of file 1");
    create_text_file(content_dir.path(), "file2.txt", "Content of file 2");

    // 创建ZIP文件（这里只是创建空文件，实际项目中应该使用zip库）
    let zip_path = dir.join(name);
    File::create(&zip_path).expect("Failed to create ZIP file");

    // 写入一些ZIP头信息（模拟）
    let mut file = File::create(&zip_path).expect("Failed to write ZIP file");
    file.write_all(b"PK\x03\x04").expect("Failed to write ZIP header");

    zip_path
}

/// 清理测试目录
pub fn cleanup_test_dir<P: AsRef<Path>>(path: P) {
    if path.as_ref().exists() {
        fs::remove_dir_all(path).expect("Failed to cleanup test directory");
    }
}

/// 断言文件存在
pub fn assert_file_exists<P: AsRef<Path>>(path: P) {
    assert!(path.as_ref().exists(), "File does not exist: {:?}", path.as_ref());
}

/// 断言文件内容
pub fn assert_file_content<P: AsRef<Path>>(path: P, expected_content: &[u8]) {
    assert_file_exists(&path);
    let content = fs::read(&path).expect("Failed to read file");
    assert_eq!(content, expected_content, "File content mismatch");
}

/// 断言文件内容为文本
pub fn assert_file_text<P: AsRef<Path>>(path: P, expected_text: &str) {
    assert_file_content(path, expected_text.as_bytes());
}

/// 生成随机测试数据
pub fn generate_random_data(size: usize) -> Vec<u8> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..size).map(|_| rng.gen()).collect()
}

/// 创建嵌套目录结构
pub fn create_nested_directories<P: AsRef<Path>>(base_dir: &Path) -> PathBuf {
    let nested_path = base_dir.join("nested").join("deep").join("path");
    fs::create_dir_all(&nested_path).expect("Failed to create nested directories");

    // 在各个层级创建文件
    create_text_file(base_dir, "root_file.txt", "Root file");
    create_text_file(&base_dir.join("nested"), "level1.txt", "Level 1 file");
    create_text_file(&base_dir.join("nested").join("deep"), "level2.txt", "Level 2 file");
    create_text_file(&nested_path, "deep_file.txt", "Deep file");

    nested_path
}

/// 模拟文件系统操作结果
pub mod mock_fs {
    use mockall::automock;
    use std::io;
    use std::path::Path;

    #[automock]
    pub trait FileSystem {
        fn read_file(&self, path: &Path) -> io::Result<Vec<u8>>;
        fn write_file(&self, path: &Path, data: &[u8]) -> io::Result<()>;
        fn file_exists(&self, path: &Path) -> bool;
        fn delete_file(&self, path: &Path) -> io::Result<()>;
    }

    /// 真实文件系统实现
    pub struct RealFileSystem;

    impl FileSystem for RealFileSystem {
        fn read_file(&self, path: &Path) -> io::Result<Vec<u8>> {
            std::fs::read(path)
        }

        fn write_file(&self, path: &Path, data: &[u8]) -> io::Result<()> {
            std::fs::write(path, data)
        }

        fn file_exists(&self, path: &Path) -> bool {
            path.exists()
        }

        fn delete_file(&self, path: &Path) -> io::Result<()> {
            std::fs::remove_file(path)
        }
    }
}

/// 测试数据库工具
pub mod test_db {
    use sqlx::{SqliteConnection, Connection};

    /// 创建内存测试数据库
    pub async fn create_memory_db() -> SqliteConnection {
        SqliteConnection::connect(":memory:")
            .await
            .expect("Failed to create memory database")
    }

    /// 初始化测试表
    pub async fn init_test_tables(conn: &mut SqliteConnection) {
        // 创建测试表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS test_files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                size INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
        .execute(conn)
        .await
        .expect("Failed to create test table");
    }
}

/// 性能测试工具
pub mod perf {
    use std::time::{Duration, Instant};

    /// 测量函数执行时间
    pub fn measure_time<F, R>(f: F) -> (R, Duration)
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();
        (result, duration)
    }

    /// 断言执行时间在阈值内
    pub fn assert_within_time<F, R>(f: F, max_time: Duration, message: &str)
    where
        F: FnOnce() -> R,
    {
        let (_, duration) = measure_time(f);
        assert!(
            duration <= max_time,
            "{}: Expected within {:?}, took {:?}",
            message,
            max_time,
            duration
        );
    }
}

/// 测试日志工具
pub mod test_log {
    use std::sync::Once;
    use tracing_subscriber::{fmt, EnvFilter};

    static INIT: Once = Once::new();

    /// 初始化测试日志
    pub fn init_test_logging() {
        INIT.call_once(|| {
            let filter = EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"));

            fmt()
                .with_env_filter(filter)
                .with_test_writer()
                .init();
        });
    }
}

/// 测试配置
pub mod config {
    use serde_json::json;

    /// 创建测试配置
    pub fn test_config() -> serde_json::Value {
        json!({
            "app": {
                "name": "Test App",
                "version": "1.0.0"
            },
            "compression": {
                "default_level": 6,
                "max_file_size": 1073741824
            },
            "security": {
                "encryption_enabled": true
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_temp_dir() {
        let temp_dir = create_temp_dir();
        assert!(temp_dir.path().exists());
        assert!(temp_dir.path().is_dir());
    }

    #[test]
    fn test_create_test_file() {
        let temp_dir = create_temp_dir();
        let file_path = create_test_file(temp_dir.path(), "test.txt", b"Hello, World!");

        assert_file_exists(&file_path);
        assert_file_content(&file_path, b"Hello, World!");
    }

    #[test]
    fn test_create_text_file() {
        let temp_dir = create_temp_dir();
        let file_path = create_text_file(temp_dir.path(), "test.txt", "Hello, World!");

        assert_file_exists(&file_path);
        assert_file_text(&file_path, "Hello, World!");
    }

    #[test]
    fn test_create_nested_directories() {
        let temp_dir = create_temp_dir();
        let nested_path = create_nested_directories(temp_dir.path());

        assert!(nested_path.exists());
        assert!(nested_path.is_dir());

        // 检查创建的文件
        let root_file = temp_dir.path().join("root_file.txt");
        let deep_file = nested_path.join("deep_file.txt");

        assert_file_exists(&root_file);
        assert_file_exists(&deep_file);
    }

    #[test]
    fn test_generate_random_data() {
        let data = generate_random_data(100);
        assert_eq!(data.len(), 100);

        // 检查数据不是全零（大概率）
        let all_zeros = data.iter().all(|&b| b == 0);
        assert!(!all_zeros, "Random data should not be all zeros");
    }

    #[test]
    fn test_perf_measure_time() {
        let (result, duration) = perf::measure_time(|| 42);

        assert_eq!(result, 42);
        assert!(duration < Duration::from_secs(1)); // 应该很快
    }
}