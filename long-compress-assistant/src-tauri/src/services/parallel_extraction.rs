use crate::services::io_buffer_pool::IOBufferPool;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{Read, Write};
use zip::ZipArchive;
use walkdir::WalkDir;
use tokio::fs;

/// ZIP条目信息
#[derive(Debug, Clone)]
pub struct ZipEntryInfo {
    pub index: usize,
    pub name: String,
    pub outpath: PathBuf,
    pub is_dir: bool,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
}

/// 并行解压器
pub struct ParallelExtractor {
    buffer_pool: IOBufferPool,
    max_concurrent_tasks: usize,
}

impl ParallelExtractor {
    /// 创建新的并行解压器
    pub fn new(buffer_pool: IOBufferPool, max_concurrent_tasks: usize) -> Self {
        Self {
            buffer_pool,
            max_concurrent_tasks,
        }
    }

    /// 并行解压ZIP文件
    pub async fn extract_zip_parallel(
        &self,
        zip_path: &Path,
        output_dir: &Path,
        password: Option<&str>,
    ) -> Result<()> {
        // 打开ZIP文件并读取所有条目信息
        let file = File::open(zip_path).context("打开ZIP文件失败")?;
        let archive = ZipArchive::new(file).context("读取ZIP归档失败")?;

        // 收集所有需要处理的文件条目
        let file_count = archive.len();
        let mut file_entries = Vec::with_capacity(file_count);

        for i in 0..file_count {
            let entry = archive.by_index(i).context("获取ZIP文件条目失败")?;
            let is_dir = entry.name().ends_with('/');
            let outpath = output_dir.join(entry.mangled_name());

            file_entries.push(ZipEntryInfo {
                index: i,
                name: entry.name().to_string(),
                outpath,
                is_dir,
                compressed_size: entry.compressed_size(),
                uncompressed_size: entry.size(),
            });
        }

        // 先创建所有目录
        let dir_entries: Vec<_> = file_entries.iter()
            .filter(|e| e.is_dir)
            .collect();

        for dir_entry in dir_entries {
            fs::create_dir_all(&dir_entry.outpath)
                .await
                .context("创建目录失败")?;
        }

        // 并行处理文件
        let file_entries: Vec<_> = file_entries.into_iter()
            .filter(|e| !e.is_dir)
            .collect();

        if !file_entries.is_empty() {
            // 使用rayon并行处理文件
            use rayon::prelude::*;

            let results: Vec<Result<()>> = file_entries.par_iter()
                .map(|entry| {
                    self.process_zip_entry_with_buffer_pool(zip_path, entry, output_dir)
                })
                .collect();

            // 检查所有结果
            for result in results {
                result.context("处理ZIP条目失败")?;
            }
        }

        Ok(())
    }

    /// 使用缓冲区池处理单个ZIP条目
    fn process_zip_entry_with_buffer_pool(
        &self,
        zip_path: &Path,
        entry: &ZipEntryInfo,
        output_dir: &Path,
    ) -> Result<()> {
        // 每个线程独立打开ZIP文件
        let file = File::open(zip_path)
            .context(format!("打开ZIP文件失败: {:?}", zip_path))?;
        let mut archive = ZipArchive::new(file)
            .context("读取ZIP归档失败")?;

        // 获取文件条目
        let mut file = archive.by_index(entry.index)
            .context(format!("获取ZIP文件条目失败: {}", entry.name))?;

        // 创建父目录
        if let Some(parent) = entry.outpath.parent() {
            std::fs::create_dir_all(parent)
                .context(format!("创建父目录失败: {:?}", parent))?;
        }

        // 创建输出文件
        let mut outfile = File::create(&entry.outpath)
            .context(format!("创建输出文件失败: {:?}", entry.outpath))?;

        // 根据文件大小获取推荐的缓冲区大小
        let recommended_buffer_size = self.buffer_pool.recommend_buffer_size(entry.uncompressed_size);

        // 由于这是在并行线程中运行，我们不能直接使用异步缓冲区池
        // 创建一个本地缓冲区
        let mut buffer = vec![0u8; recommended_buffer_size];

        loop {
            let bytes_read = file.read(&mut buffer)
                .context("读取ZIP文件内容失败")?;
            if bytes_read == 0 {
                break;
            }
            outfile.write_all(&buffer[..bytes_read])
                .context("写入文件内容失败")?;
        }

        Ok(())
    }

    /// 获取推荐的并发任务数
    pub fn recommended_concurrent_tasks(&self) -> usize {
        // 根据CPU核心数确定并发任务数
        let num_cpus = num_cpus::get();
        self.max_concurrent_tasks.min(num_cpus * 2)
    }
}

/// 优化的文件复制函数（使用缓冲区池）
pub async fn copy_file_with_buffer_pool(
    buffer_pool: &IOBufferPool,
    src: &Path,
    dst: &Path,
) -> Result<()> {
    // 打开源文件
    let mut source_file = File::open(src)
        .context(format!("打开源文件失败: {:?}", src))?;

    // 获取文件大小
    let file_size = source_file.metadata()
        .context("获取文件元数据失败")?
        .len();

    // 根据文件大小获取推荐的缓冲区大小
    let recommended_buffer_size = buffer_pool.recommend_buffer_size(file_size);

    // 从缓冲区池获取缓冲区
    let mut buffer_handle = buffer_pool.acquire(Some(recommended_buffer_size)).await;
    let buffer = buffer_handle.buffer_mut();

    // 创建目标文件
    let mut dest_file = File::create(dst)
        .context(format!("创建目标文件失败: {:?}", dst))?;

    loop {
        let bytes_read = source_file.read(buffer.as_mut_slice())
            .context("读取文件内容失败")?;

        if bytes_read == 0 {
            break;
        }

        buffer.set_size(bytes_read);
        dest_file.write_all(buffer.as_slice())
            .context("写入文件内容失败")?;
    }

    Ok(())
}

/// 优化的目录复制函数（使用缓冲区池）
pub async fn copy_directory_with_buffer_pool(
    buffer_pool: &IOBufferPool,
    src: &Path,
    dst: &Path,
) -> Result<()> {
    // 创建目标目录
    fs::create_dir_all(dst).await
        .context(format!("创建目标目录失败: {:?}", dst))?;

    // 遍历源目录
    for entry in WalkDir::new(src) {
        let entry = entry.context("读取目录条目失败")?;
        let src_path = entry.path();
        let relative_path = src_path.strip_prefix(src)
            .context("计算相对路径失败")?;
        let dst_path = dst.join(relative_path);

        if entry.file_type().is_dir() {
            // 创建子目录
            fs::create_dir_all(&dst_path).await
                .context(format!("创建子目录失败: {:?}", dst_path))?;
        } else if entry.file_type().is_file() {
            // 复制文件
            copy_file_with_buffer_pool(buffer_pool, src_path, &dst_path).await
                .context(format!("复制文件失败: {:?} -> {:?}", src_path, dst_path))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parallel_extractor_creation() {
        let buffer_pool = IOBufferPool::default();
        let extractor = ParallelExtractor::new(buffer_pool, 4);

        assert_eq!(extractor.max_concurrent_tasks, 4);
    }

    #[tokio::test]
    async fn test_copy_file_with_buffer_pool() {
        let buffer_pool = IOBufferPool::default();
        let temp_dir = TempDir::new().unwrap();

        // 创建测试文件
        let src_path = temp_dir.path().join("source.txt");
        let dst_path = temp_dir.path().join("destination.txt");

        let test_content = b"Hello, World! This is a test file.";
        std::fs::write(&src_path, test_content).unwrap();

        // 复制文件
        let result = copy_file_with_buffer_pool(&buffer_pool, &src_path, &dst_path).await;
        assert!(result.is_ok());

        // 验证文件内容
        let copied_content = std::fs::read(&dst_path).unwrap();
        assert_eq!(copied_content, test_content);
    }

    #[tokio::test]
    async fn test_copy_directory_with_buffer_pool() {
        let buffer_pool = IOBufferPool::default();
        let temp_dir = TempDir::new().unwrap();

        // 创建测试目录结构
        let src_dir = temp_dir.path().join("source");
        let dst_dir = temp_dir.path().join("destination");

        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(src_dir.join("subdir")).unwrap();

        // 创建测试文件
        std::fs::write(src_dir.join("file1.txt"), b"File 1 content").unwrap();
        std::fs::write(src_dir.join("subdir").join("file2.txt"), b"File 2 content").unwrap();

        // 复制目录
        let result = copy_directory_with_buffer_pool(&buffer_pool, &src_dir, &dst_dir).await;
        assert!(result.is_ok());

        // 验证目录结构
        assert!(dst_dir.exists());
        assert!(dst_dir.join("file1.txt").exists());
        assert!(dst_dir.join("subdir").exists());
        assert!(dst_dir.join("subdir").join("file2.txt").exists());

        // 验证文件内容
        let content1 = std::fs::read(dst_dir.join("file1.txt")).unwrap();
        assert_eq!(content1, b"File 1 content");

        let content2 = std::fs::read(dst_dir.join("subdir").join("file2.txt")).unwrap();
        assert_eq!(content2, b"File 2 content");
    }
}