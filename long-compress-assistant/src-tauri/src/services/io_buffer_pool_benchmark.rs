use crate::services::io_buffer_pool::{IOBufferPool, IOBufferPoolConfig};
use std::time::{Duration, Instant};

/// 缓冲区池性能基准测试
pub struct IOBufferPoolBenchmark {
    pool: IOBufferPool,
}

impl IOBufferPoolBenchmark {
    /// 创建新的性能基准测试
    pub fn new(config: IOBufferPoolConfig) -> Self {
        let pool = IOBufferPool::new(config);
        Self { pool }
    }

    /// 使用默认配置创建性能基准测试
    pub fn default() -> Self {
        Self::new(IOBufferPoolConfig::default())
    }

    /// 运行单线程获取和释放测试
    pub async fn run_single_thread_test(&self, iterations: usize) -> BenchmarkResult {
        let start_time = Instant::now();
        let mut total_buffer_size = 0;

        for i in 0..iterations {
            // 根据迭代次数动态调整缓冲区大小
            let buffer_size = 64 * 1024 * ((i % 4) + 1); // 64KB, 128KB, 192KB, 256KB
            let mut handle = self.pool.acquire(Some(buffer_size)).await;

            // 模拟使用缓冲区
            let buffer = handle.buffer_mut();
            buffer.set_size(buffer_size / 2);
            total_buffer_size += buffer.capacity();

            // 立即释放缓冲区
            handle.release().await;
        }

        let duration = start_time.elapsed();
        let throughput = iterations as f64 / duration.as_secs_f64();

        BenchmarkResult {
            test_name: "单线程获取释放测试".to_string(),
            iterations,
            duration,
            throughput,
            average_latency: duration.as_secs_f64() / iterations as f64,
            total_buffer_size,
        }
    }

    /// 运行并发获取测试
    pub async fn run_concurrent_test(&self, concurrent_tasks: usize, iterations_per_task: usize) -> BenchmarkResult {
        let start_time = Instant::now();
        let mut total_buffer_size = 0;

        let mut tasks = Vec::new();

        for _task_id in 0..concurrent_tasks {
            let pool_clone = self.pool.clone();
            let task = tokio::spawn(async move {
                let mut task_total_size = 0;

                for i in 0..iterations_per_task {
                    let buffer_size = 64 * 1024 * ((i % 4) + 1);
                    let mut handle = pool_clone.acquire(Some(buffer_size)).await;

                    let buffer = handle.buffer_mut();
                    buffer.set_size(buffer_size / 2);
                    task_total_size += buffer.capacity();

                    handle.release().await;
                }

                task_total_size
            });

            tasks.push(task);
        }

        // 等待所有任务完成
        for task in tasks {
            total_buffer_size += task.await.unwrap();
        }

        let duration = start_time.elapsed();
        let total_iterations = concurrent_tasks * iterations_per_task;
        let throughput = total_iterations as f64 / duration.as_secs_f64();

        BenchmarkResult {
            test_name: format!("并发测试 ({} 任务)", concurrent_tasks),
            iterations: total_iterations,
            duration,
            throughput,
            average_latency: duration.as_secs_f64() / total_iterations as f64,
            total_buffer_size,
        }
    }

    /// 运行内存分配对比测试
    pub async fn run_memory_allocation_comparison(&self, iterations: usize) -> MemoryAllocationComparison {
        // 测试使用缓冲区池的性能
        let pool_start_time = Instant::now();
        let mut pool_total_memory = 0;

        for i in 0..iterations {
            let buffer_size = 64 * 1024 * ((i % 4) + 1);
            let mut handle = self.pool.acquire(Some(buffer_size)).await;

            let buffer = handle.buffer_mut();
            buffer.set_size(buffer_size / 2);
            pool_total_memory += buffer.capacity();

            handle.release().await;
        }

        let pool_duration = pool_start_time.elapsed();

        // 测试直接分配内存的性能
        let direct_start_time = Instant::now();
        let mut direct_total_memory = 0;

        for i in 0..iterations {
            let buffer_size = 64 * 1024 * ((i % 4) + 1);
            let buffer = vec![0u8; buffer_size];

            direct_total_memory += buffer.capacity();

            // 模拟使用缓冲区
            let _used_size = buffer_size / 2;
        }

        let direct_duration = direct_start_time.elapsed();

        MemoryAllocationComparison {
            iterations,
            pool_duration,
            direct_duration,
            pool_total_memory,
            direct_total_memory,
            speedup: direct_duration.as_secs_f64() / pool_duration.as_secs_f64(),
            memory_saving: (direct_total_memory - pool_total_memory) as f64 / direct_total_memory as f64 * 100.0,
        }
    }

    /// 运行缓冲区大小调整测试
    pub async fn run_buffer_resizing_test(&self, test_cases: &[usize]) -> BufferResizingTestResult {
        let start_time = Instant::now();
        let mut results = Vec::new();

        for &file_size in test_cases {
            let recommended_size = self.pool.recommend_buffer_size(file_size as u64);

            let handle = self.pool.acquire(Some(recommended_size)).await;
            let actual_size = handle.capacity();
            handle.release().await;

            results.push(BufferSizeRecommendation {
                file_size,
                recommended_size,
                actual_size,
                efficiency: if file_size > 0 {
                    (recommended_size as f64 / file_size as f64).min(1.0) * 100.0
                } else {
                    0.0
                },
            });
        }

        let duration = start_time.elapsed();

        BufferResizingTestResult {
            test_cases: test_cases.len(),
            duration,
            recommendations: results,
        }
    }

    /// 获取缓冲区池统计信息
    pub async fn get_pool_statistics(&self) -> String {
        let stats = self.pool.get_statistics().await;
        format!(
            "缓冲区池统计:\n\
            - 总分配次数: {}\n\
            - 总释放次数: {}\n\
            - 缓冲区命中率: {:.2}%\n\
            - 平均缓冲区大小: {:.2} KB\n\
            - 峰值缓冲区数量: {}\n\
            - 总缓冲区大小: {:.2} MB",
            stats.total_allocations,
            stats.total_releases,
            if stats.total_allocations > 0 {
                stats.buffer_hits as f64 / stats.total_allocations as f64 * 100.0
            } else {
                0.0
            },
            stats.average_buffer_size / 1024.0,
            stats.peak_buffer_count,
            stats.total_buffer_size as f64 / (1024.0 * 1024.0)
        )
    }
}

/// 基准测试结果
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub test_name: String,
    pub iterations: usize,
    pub duration: Duration,
    pub throughput: f64, // 操作/秒
    pub average_latency: f64, // 秒/操作
    pub total_buffer_size: usize,
}

/// 内存分配对比结果
#[derive(Debug, Clone)]
pub struct MemoryAllocationComparison {
    pub iterations: usize,
    pub pool_duration: Duration,
    pub direct_duration: Duration,
    pub pool_total_memory: usize,
    pub direct_total_memory: usize,
    pub speedup: f64, // 加速比
    pub memory_saving: f64, // 内存节省百分比
}

/// 缓冲区大小推荐结果
#[derive(Debug, Clone)]
pub struct BufferSizeRecommendation {
    pub file_size: usize,
    pub recommended_size: usize,
    pub actual_size: usize,
    pub efficiency: f64, // 效率百分比
}

/// 缓冲区大小调整测试结果
#[derive(Debug, Clone)]
pub struct BufferResizingTestResult {
    pub test_cases: usize,
    pub duration: Duration,
    pub recommendations: Vec<BufferSizeRecommendation>,
}

impl BenchmarkResult {
    /// 格式化输出结果
    pub fn format(&self) -> String {
        format!(
            "测试: {}\n\
            迭代次数: {}\n\
            总耗时: {:.2?}\n\
            吞吐量: {:.2} 操作/秒\n\
            平均延迟: {:.4} 毫秒/操作\n\
            总缓冲区大小: {:.2} MB",
            self.test_name,
            self.iterations,
            self.duration,
            self.throughput,
            self.average_latency * 1000.0,
            self.total_buffer_size as f64 / (1024.0 * 1024.0)
        )
    }
}

impl MemoryAllocationComparison {
    /// 格式化输出结果
    pub fn format(&self) -> String {
        format!(
            "内存分配对比测试:\n\
            迭代次数: {}\n\
            缓冲区池耗时: {:.2?}\n\
            直接分配耗时: {:.2?}\n\
            加速比: {:.2}x\n\
            缓冲区池总内存: {:.2} MB\n\
            直接分配总内存: {:.2} MB\n\
            内存节省: {:.2}%",
            self.iterations,
            self.pool_duration,
            self.direct_duration,
            self.speedup,
            self.pool_total_memory as f64 / (1024.0 * 1024.0),
            self.direct_total_memory as f64 / (1024.0 * 1024.0),
            self.memory_saving
        )
    }
}

/// 运行完整的性能基准测试套件
pub async fn run_complete_benchmark_suite() -> String {
    let mut results = Vec::new();

    // 创建缓冲区池
    let benchmark = IOBufferPoolBenchmark::default();

    // 1. 单线程测试
    results.push("=== 单线程性能测试 ===".to_string());
    let single_thread_result = benchmark.run_single_thread_test(1000).await;
    results.push(single_thread_result.format());

    // 2. 并发测试
    results.push("\n=== 并发性能测试 ===".to_string());
    let concurrent_result = benchmark.run_concurrent_test(4, 250).await;
    results.push(concurrent_result.format());

    // 3. 内存分配对比测试
    results.push("\n=== 内存分配对比测试 ===".to_string());
    let memory_comparison = benchmark.run_memory_allocation_comparison(1000).await;
    results.push(memory_comparison.format());

    // 4. 缓冲区大小调整测试
    results.push("\n=== 缓冲区大小调整测试 ===".to_string());
    let test_cases = vec![
        1024,           // 1KB
        1024 * 1024,    // 1MB
        10 * 1024 * 1024, // 10MB
        100 * 1024 * 1024, // 100MB
        1024 * 1024 * 1024, // 1GB
    ];

    let resizing_result = benchmark.run_buffer_resizing_test(&test_cases).await;
    results.push(format!(
        "测试用例数: {}\n总耗时: {:.2?}",
        resizing_result.test_cases, resizing_result.duration
    ));

    for recommendation in &resizing_result.recommendations {
        results.push(format!(
            "文件大小: {} -> 推荐缓冲区: {} (实际: {}) 效率: {:.1}%",
            format_file_size(recommendation.file_size as u64),
            format_file_size(recommendation.recommended_size as u64),
            format_file_size(recommendation.actual_size as u64),
            recommendation.efficiency
        ));
    }

    // 5. 获取统计信息
    results.push("\n=== 缓冲区池统计信息 ===".to_string());
    let stats = benchmark.get_pool_statistics().await;
    results.push(stats);

    results.join("\n")
}

/// 格式化文件大小
fn format_file_size(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];

    if size == 0 {
        return "0 B".to_string();
    }

    let base = 1024_f64;
    let size_f64 = size as f64;
    let exponent = (size_f64.log10() / base.log10()).floor() as i32;
    let unit_index = exponent.min(4).max(0) as usize;

    let formatted_size = size_f64 / base.powi(exponent);

    format!("{:.1} {}", formatted_size, UNITS[unit_index])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_single_thread_benchmark() {
        let benchmark = IOBufferPoolBenchmark::default();
        let result = benchmark.run_single_thread_test(100).await;

        assert_eq!(result.iterations, 100);
        assert!(result.duration.as_secs_f64() > 0.0);
        assert!(result.throughput > 0.0);
    }

    #[tokio::test]
    async fn test_concurrent_benchmark() {
        let benchmark = IOBufferPoolBenchmark::default();
        let result = benchmark.run_concurrent_test(2, 50).await;

        assert_eq!(result.iterations, 100);
        assert!(result.duration.as_secs_f64() > 0.0);
        assert!(result.throughput > 0.0);
    }

    #[tokio::test]
    async fn test_memory_allocation_comparison() {
        let benchmark = IOBufferPoolBenchmark::default();
        let result = benchmark.run_memory_allocation_comparison(100).await;

        assert_eq!(result.iterations, 100);
        assert!(result.pool_duration.as_secs_f64() > 0.0);
        assert!(result.direct_duration.as_secs_f64() > 0.0);
        assert!(result.speedup > 0.0);
    }

    #[tokio::test]
    async fn test_buffer_resizing() {
        let benchmark = IOBufferPoolBenchmark::default();
        let test_cases = vec![1024, 1024 * 1024, 10 * 1024 * 1024];
        let result = benchmark.run_buffer_resizing_test(&test_cases).await;

        assert_eq!(result.test_cases, 3);
        assert!(result.duration.as_secs_f64() > 0.0);
        assert_eq!(result.recommendations.len(), 3);
    }

    #[tokio::test]
    async fn test_complete_benchmark_suite() {
        let result = run_complete_benchmark_suite().await;
        assert!(!result.is_empty());
        assert!(result.contains("单线程性能测试"));
        assert!(result.contains("并发性能测试"));
        assert!(result.contains("内存分配对比测试"));
    }
}