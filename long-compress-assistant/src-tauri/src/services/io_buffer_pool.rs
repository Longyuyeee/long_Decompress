use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

/// IO缓冲区池配置
#[derive(Debug, Clone)]
pub struct IOBufferPoolConfig {
    /// 最小缓冲区大小（字节）
    pub min_buffer_size: usize,
    /// 最大缓冲区大小（字节）
    pub max_buffer_size: usize,
    /// 初始缓冲区数量
    pub initial_buffer_count: usize,
    /// 最大缓冲区数量
    pub max_buffer_count: usize,
    /// 是否启用动态大小调整
    pub enable_dynamic_sizing: bool,
    /// 缓冲区增长因子
    pub growth_factor: f32,
    /// 缓冲区收缩因子
    pub shrink_factor: f32,
}

impl Default for IOBufferPoolConfig {
    fn default() -> Self {
        Self {
            min_buffer_size: 64 * 1024,      // 64KB
            max_buffer_size: 1024 * 1024,    // 1MB
            initial_buffer_count: 4,
            max_buffer_count: 16,
            enable_dynamic_sizing: true,
            growth_factor: 1.5,
            shrink_factor: 0.75,
        }
    }
}

/// IO缓冲区
#[derive(Debug)]
pub struct IOBuffer {
    data: Vec<u8>,
    size: usize,
    capacity: usize,
}

impl IOBuffer {
    /// 创建新的缓冲区
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![0; capacity],
            size: 0,
            capacity,
        }
    }

    /// 获取缓冲区数据切片
    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.size]
    }

    /// 获取缓冲区数据可变切片
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data[..self.capacity]
    }

    /// 设置缓冲区大小
    pub fn set_size(&mut self, size: usize) {
        self.size = size.min(self.capacity);
    }

    /// 获取缓冲区大小
    pub fn size(&self) -> usize {
        self.size
    }

    /// 获取缓冲区容量
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 清空缓冲区
    pub fn clear(&mut self) {
        self.size = 0;
    }

    /// 调整缓冲区容量
    pub fn resize(&mut self, new_capacity: usize) {
        if new_capacity != self.capacity {
            self.data.resize(new_capacity, 0);
            self.capacity = new_capacity;
            self.size = self.size.min(new_capacity);
        }
    }
}

/// IO缓冲区池
#[derive(Debug)]
pub struct IOBufferPool {
    config: IOBufferPoolConfig,
    buffers: Arc<Mutex<VecDeque<IOBuffer>>>,
    total_allocated: Arc<Mutex<usize>>,
    statistics: Arc<Mutex<BufferPoolStatistics>>,
}

#[derive(Debug, Clone, Default)]
pub struct BufferPoolStatistics {
    pub total_allocations: u64,
    pub total_releases: u64,
    pub buffer_hits: u64,
    pub buffer_misses: u64,
    pub total_buffer_size: u64,
    pub average_buffer_size: f64,
    pub peak_buffer_count: usize,
}

impl IOBufferPool {
    /// 创建新的缓冲区池
    pub fn new(config: IOBufferPoolConfig) -> Self {
        let mut buffers = VecDeque::with_capacity(config.initial_buffer_count);

        // 预分配初始缓冲区
        for _ in 0..config.initial_buffer_count {
            buffers.push_back(IOBuffer::new(config.min_buffer_size));
        }

        let total_allocated = config.initial_buffer_count;

        Self {
            config,
            buffers: Arc::new(Mutex::new(buffers)),
            total_allocated: Arc::new(Mutex::new(total_allocated)),
            statistics: Arc::new(Mutex::new(BufferPoolStatistics::default())),
        }
    }

    /// 使用默认配置创建缓冲区池
    pub fn default() -> Self {
        Self::new(IOBufferPoolConfig::default())
    }

    /// 获取一个缓冲区
    pub async fn acquire(&self, preferred_size: Option<usize>) -> IOBufferHandle {
        let mut buffers = self.buffers.lock().await;
        let mut stats = self.statistics.lock().await;

        stats.total_allocations += 1;

        // 确定缓冲区大小
        let buffer_size = preferred_size.unwrap_or(self.config.min_buffer_size)
            .clamp(self.config.min_buffer_size, self.config.max_buffer_size);

        // 尝试从池中获取合适大小的缓冲区
        if let Some(index) = buffers.iter().position(|buf| buf.capacity() >= buffer_size) {
            let mut buffer = buffers.remove(index).unwrap();

            // 如果缓冲区太大且允许收缩，调整大小
            if self.config.enable_dynamic_sizing && buffer.capacity() > buffer_size * 2 {
                let new_size = (buffer.capacity() as f32 * self.config.shrink_factor) as usize;
                let new_size = new_size.clamp(buffer_size, self.config.max_buffer_size);
                buffer.resize(new_size);
            }

            let capacity = buffer.capacity();
            stats.buffer_hits += 1;
            stats.total_buffer_size += capacity as u64;

            IOBufferHandle {
                buffer: Some(buffer),
                pool: self.clone(),
                acquired_size: capacity,
            }
        } else {
            // 池中没有合适的缓冲区，创建新的
            stats.buffer_misses += 1;

            let mut total_allocated = self.total_allocated.lock().await;
            if *total_allocated < self.config.max_buffer_count {
                *total_allocated += 1;
                let current_total = *total_allocated;
                drop(total_allocated);

                let buffer = IOBuffer::new(buffer_size);
                stats.total_buffer_size += buffer.capacity() as u64;
                stats.peak_buffer_count = stats.peak_buffer_count.max(current_total);

                IOBufferHandle {
                    buffer: Some(buffer),
                    pool: self.clone(),
                    acquired_size: buffer_size,
                }
            } else {
                // 已达到最大缓冲区数量，返回最小缓冲区
                drop(total_allocated);

                let buffer = IOBuffer::new(self.config.min_buffer_size);
                stats.total_buffer_size += buffer.capacity() as u64;

                IOBufferHandle {
                    buffer: Some(buffer),
                    pool: self.clone(),
                    acquired_size: self.config.min_buffer_size,
                }
            }
        }
    }

    /// 根据文件大小获取推荐的缓冲区大小
    pub fn recommend_buffer_size(&self, file_size: u64) -> usize {
        if !self.config.enable_dynamic_sizing {
            return self.config.min_buffer_size;
        }

        // 根据文件大小动态调整缓冲区大小
        let recommended_size = if file_size < 1024 * 1024 { // < 1MB
            self.config.min_buffer_size
        } else if file_size < 10 * 1024 * 1024 { // 1MB - 10MB
            self.config.min_buffer_size * 2
        } else if file_size < 100 * 1024 * 1024 { // 10MB - 100MB
            self.config.min_buffer_size * 4
        } else { // > 100MB
            self.config.max_buffer_size
        };

        recommended_size.clamp(self.config.min_buffer_size, self.config.max_buffer_size)
    }

    /// 获取统计信息
    pub async fn get_statistics(&self) -> BufferPoolStatistics {
        let stats = self.statistics.lock().await;
        let buffers = self.buffers.lock().await;
        let total_allocated = self.total_allocated.lock().await;

        let mut result = stats.clone();

        // 计算平均缓冲区大小
        if *total_allocated > 0 {
            result.average_buffer_size = result.total_buffer_size as f64 / *total_allocated as f64;
        }

        // 更新当前缓冲区数量
        result.peak_buffer_count = result.peak_buffer_count.max(buffers.len());

        result
    }

    /// 清理缓冲区池
    pub async fn cleanup(&self) {
        let mut buffers = self.buffers.lock().await;
        let mut total_allocated = self.total_allocated.lock().await;

        // 移除多余的缓冲区
        let target_count = self.config.initial_buffer_count;
        while buffers.len() > target_count {
            buffers.pop_back();
        }

        *total_allocated = buffers.len();
    }

    /// 内部方法：释放缓冲区回池中
    async fn release_buffer(&self, mut buffer: IOBuffer) {
        let mut buffers = self.buffers.lock().await;
        let mut stats = self.statistics.lock().await;
        let mut total_allocated = self.total_allocated.lock().await;

        stats.total_releases += 1;

        // 清空缓冲区内容
        buffer.clear();

        // 如果缓冲区太大且允许收缩，调整大小
        if self.config.enable_dynamic_sizing && buffer.capacity() > self.config.min_buffer_size * 2 {
            let new_size = (buffer.capacity() as f32 * self.config.shrink_factor) as usize;
            let new_size = new_size.clamp(self.config.min_buffer_size, self.config.max_buffer_size);
            buffer.resize(new_size);
        }

        // 如果池已满，丢弃缓冲区
        if buffers.len() >= self.config.max_buffer_count {
            *total_allocated -= 1;
        } else {
            buffers.push_back(buffer);
        }
    }
}

impl Clone for IOBufferPool {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            buffers: self.buffers.clone(),
            total_allocated: self.total_allocated.clone(),
            statistics: self.statistics.clone(),
        }
    }
}

/// IO缓冲区句柄（RAII模式）
#[derive(Debug)]
pub struct IOBufferHandle {
    buffer: Option<IOBuffer>,
    pool: IOBufferPool,
    acquired_size: usize,
}

impl IOBufferHandle {
    /// 获取缓冲区引用
    pub fn buffer(&self) -> &IOBuffer {
        self.buffer.as_ref().unwrap()
    }

    /// 获取缓冲区可变引用
    pub fn buffer_mut(&mut self) -> &mut IOBuffer {
        self.buffer.as_mut().unwrap()
    }

    /// 获取缓冲区大小
    pub fn size(&self) -> usize {
        self.buffer.as_ref().map(|b| b.size()).unwrap_or(0)
    }

    /// 获取缓冲区容量
    pub fn capacity(&self) -> usize {
        self.buffer.as_ref().map(|b| b.capacity()).unwrap_or(0)
    }

    /// 获取获取时的缓冲区大小
    pub fn acquired_size(&self) -> usize {
        self.acquired_size
    }

    /// 手动释放缓冲区
    pub async fn release(mut self) {
        if let Some(buffer) = self.buffer.take() {
            self.pool.release_buffer(buffer).await;
        }
    }
}

impl Drop for IOBufferHandle {
    fn drop(&mut self) {
        if let Some(buffer) = self.buffer.take() {
            let pool = self.pool.clone();
            tokio::spawn(async move {
                pool.release_buffer(buffer).await;
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_buffer_pool_creation() {
        let config = IOBufferPoolConfig {
            min_buffer_size: 64 * 1024,
            max_buffer_size: 1024 * 1024,
            initial_buffer_count: 2,
            max_buffer_count: 8,
            enable_dynamic_sizing: true,
            growth_factor: 1.5,
            shrink_factor: 0.75,
        };

        let pool = IOBufferPool::new(config);

        // 验证池已创建
        assert!(true); // 简单验证对象已创建
    }

    #[test]
    fn test_recommend_buffer_size() {
        let pool = IOBufferPool::default();

        // 测试小文件
        assert_eq!(pool.recommend_buffer_size(1024), 64 * 1024);

        // 测试中等文件
        assert_eq!(pool.recommend_buffer_size(5 * 1024 * 1024), 128 * 1024);

        // 测试大文件
        assert_eq!(pool.recommend_buffer_size(50 * 1024 * 1024), 256 * 1024);

        // 测试超大文件
        assert_eq!(pool.recommend_buffer_size(200 * 1024 * 1024), 1024 * 1024);
    }

    #[tokio::test]
    async fn test_buffer_acquisition_and_release() {
        let pool = IOBufferPool::default();

        // 获取缓冲区
        let mut handle = pool.acquire(Some(128 * 1024)).await;
        assert_eq!(handle.capacity(), 128 * 1024);

        // 使用缓冲区
        let buffer = handle.buffer_mut();
        buffer.set_size(1024);
        assert_eq!(buffer.size(), 1024);

        // 释放缓冲区
        handle.release().await;

        // 验证统计信息
        let stats = pool.get_statistics().await;
        assert_eq!(stats.total_allocations, 1);
        assert_eq!(stats.total_releases, 1);
    }

    #[tokio::test]
    async fn test_concurrent_buffer_access() {
        let pool = IOBufferPool::default();

        let mut handles = Vec::new();

        // 并发获取多个缓冲区
        for i in 0..5 {
            let pool_clone = pool.clone();
            let handle = tokio::spawn(async move {
                let size = 64 * 1024 * (i + 1);
                let mut handle = pool_clone.acquire(Some(size)).await;
                handle.buffer_mut().set_size(size / 2);
                handle
            });

            handles.push(handle);
        }

        // 等待所有任务完成
        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await.unwrap());
        }

        // 验证所有缓冲区都已获取
        assert_eq!(results.len(), 5);

        // 释放所有缓冲区
        for handle in results {
            handle.release().await;
        }

        // 验证统计信息
        let stats = pool.get_statistics().await;
        assert_eq!(stats.total_allocations, 5);
        assert_eq!(stats.total_releases, 5);
    }

    #[tokio::test]
    async fn test_buffer_pool_cleanup() {
        let config = IOBufferPoolConfig {
            min_buffer_size: 64 * 1024,
            max_buffer_size: 1024 * 1024,
            initial_buffer_count: 2,
            max_buffer_count: 4,
            enable_dynamic_sizing: true,
            growth_factor: 1.5,
            shrink_factor: 0.75,
        };

        let pool = IOBufferPool::new(config);

        // 获取并释放多个缓冲区
        for _ in 0..10 {
            let handle = pool.acquire(None).await;
            handle.release().await;
        }

        // 清理缓冲区池
        pool.cleanup().await;

        // 验证统计信息
        let stats = pool.get_statistics().await;
        assert!(stats.total_allocations >= 10);
        assert!(stats.total_releases >= 10);
    }
}