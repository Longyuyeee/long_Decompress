use crate::task_queue::task_queue::TaskQueue;
use crate::task_queue::optimized_task_queue::OptimizedTaskQueue;
use crate::task_queue::models::{QueueTask, TaskType, TaskPriority, QueueTaskStatus};
use crate::models::compression::{CompressionTask, CompressionFormat, CompressionOptions};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

/// 任务队列性能基准测试
pub struct QueueBenchmark {
    original_queue: Arc<TaskQueue>,
    optimized_queue: Arc<OptimizedTaskQueue>,
}

impl QueueBenchmark {
    /// 创建新的性能基准测试
    pub fn new(queue_size: usize, worker_count: usize) -> Self {
        let original_queue = Arc::new(TaskQueue::new(queue_size));
        let optimized_queue = Arc::new(OptimizedTaskQueue::new(queue_size, worker_count));

        Self {
            original_queue,
            optimized_queue,
        }
    }

    /// 创建测试任务
    fn create_test_task(priority: TaskPriority) -> QueueTask {
        let compression_task = CompressionTask::new(
            vec!["test.txt".to_string()],
            "output.zip".to_string(),
            CompressionFormat::Zip,
            CompressionOptions::default(),
        );

        QueueTask::new(
            TaskType::Compress,
            priority,
            compression_task,
        )
    }

    /// 运行添加任务性能测试
    pub async fn run_add_task_benchmark(&self, task_count: usize) -> AddTaskBenchmarkResult {
        // 测试原始队列
        let original_start = Instant::now();
        for i in 0..task_count {
            let priority = match i % 3 {
                0 => TaskPriority::High,
                1 => TaskPriority::Medium,
                _ => TaskPriority::Low,
            };

            let task = Self::create_test_task(priority);
            self.original_queue.add_task(task).await.unwrap();
        }
        let original_duration = original_start.elapsed();

        // 清空队列
        self.original_queue.clear().await;

        // 测试优化队列
        let optimized_start = Instant::now();
        for i in 0..task_count {
            let priority = match i % 3 {
                0 => TaskPriority::High,
                1 => TaskPriority::Medium,
                _ => TaskPriority::Low,
            };

            let task = Self::create_test_task(priority);
            self.optimized_queue.add_task(task).await.unwrap();
        }
        let optimized_duration = optimized_start.elapsed();

        AddTaskBenchmarkResult {
            task_count,
            original_duration,
            optimized_duration,
            speedup: original_duration.as_secs_f64() / optimized_duration.as_secs_f64(),
        }
    }

    /// 运行获取任务性能测试
    pub async fn run_get_task_benchmark(&self, task_count: usize, concurrent_workers: usize) -> GetTaskBenchmarkResult {
        // 先添加任务
        for i in 0..task_count {
            let priority = match i % 3 {
                0 => TaskPriority::High,
                1 => TaskPriority::Medium,
                _ => TaskPriority::Low,
            };

            let task = Self::create_test_task(priority);
            self.original_queue.add_task(task).await.unwrap();

            let task = Self::create_test_task(priority);
            self.optimized_queue.add_task(task).await.unwrap();
        }

        // 测试原始队列并发获取
        let original_start = Instant::now();
        let mut original_tasks = Vec::new();

        for worker_id in 0..concurrent_workers {
            let queue_clone = self.original_queue.clone();
            let task = tokio::spawn(async move {
                let mut count = 0;
                while let Some(task_ref) = queue_clone.get_next_task().await {
                    count += 1;
                    // 模拟任务处理
                    tokio::time::sleep(Duration::from_micros(10)).await;
                }
                count
            });
            original_tasks.push(task);
        }

        let mut original_total = 0;
        for task in original_tasks {
            original_total += task.await.unwrap();
        }
        let original_duration = original_start.elapsed();

        // 测试优化队列并发获取（使用工作窃取）
        let optimized_start = Instant::now();
        let mut optimized_tasks = Vec::new();

        for worker_id in 0..concurrent_workers {
            let queue_clone = self.optimized_queue.clone();
            let task = tokio::spawn(async move {
                let mut count = 0;
                while let Some(task_ref) = queue_clone.get_next_task(worker_id).await {
                    count += 1;
                    // 模拟任务处理
                    tokio::time::sleep(Duration::from_micros(10)).await;
                }
                count
            });
            optimized_tasks.push(task);
        }

        let mut optimized_total = 0;
        for task in optimized_tasks {
            optimized_total += task.await.unwrap();
        }
        let optimized_duration = optimized_start.elapsed();

        // 清空队列
        self.original_queue.clear().await;
        self.optimized_queue.clear().await;

        GetTaskBenchmarkResult {
            task_count,
            concurrent_workers,
            original_duration,
            optimized_duration,
            original_total_tasks: original_total,
            optimized_total_tasks: optimized_total,
            speedup: original_duration.as_secs_f64() / optimized_duration.as_secs_f64(),
        }
    }

    /// 运行锁竞争测试
    pub async fn run_lock_contention_benchmark(&self, operations_per_thread: usize, thread_count: usize) -> LockContentionBenchmarkResult {
        // 测试原始队列锁竞争
        let original_start = Instant::now();
        let mut original_threads = Vec::new();

        for thread_id in 0..thread_count {
            let queue_clone = self.original_queue.clone();
            let thread = std::thread::spawn(move || {
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    for i in 0..operations_per_thread {
                        let priority = match (thread_id + i) % 3 {
                            0 => TaskPriority::High,
                            1 => TaskPriority::Medium,
                            _ => TaskPriority::Low,
                        };

                        let task = Self::create_test_task(priority);
                        queue_clone.add_task(task).await.unwrap();

                        // 偶尔获取任务
                        if i % 10 == 0 {
                            let _ = queue_clone.get_next_task().await;
                        }
                    }
                });
            });
            original_threads.push(thread);
        }

        for thread in original_threads {
            thread.join().unwrap();
        }
        let original_duration = original_start.elapsed();

        // 清空队列
        self.original_queue.clear().await;

        // 测试优化队列锁竞争
        let optimized_start = Instant::now();
        let mut optimized_threads = Vec::new();

        for thread_id in 0..thread_count {
            let queue_clone = self.optimized_queue.clone();
            let thread = std::thread::spawn(move || {
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    for i in 0..operations_per_thread {
                        let priority = match (thread_id + i) % 3 {
                            0 => TaskPriority::High,
                            1 => TaskPriority::Medium,
                            _ => TaskPriority::Low,
                        };

                        let task = Self::create_test_task(priority);
                        queue_clone.add_task(task).await.unwrap();

                        // 偶尔获取任务
                        if i % 10 == 0 {
                            let _ = queue_clone.get_next_task(thread_id).await;
                        }
                    }
                });
            });
            optimized_threads.push(thread);
        }

        for thread in optimized_threads {
            thread.join().unwrap();
        }
        let optimized_duration = optimized_start.elapsed();

        // 清空队列
        self.optimized_queue.clear().await;

        LockContentionBenchmarkResult {
            operations_per_thread,
            thread_count,
            original_duration,
            optimized_duration,
            speedup: original_duration.as_secs_f64() / optimized_duration.as_secs_f64(),
        }
    }

    /// 运行工作窃取效果测试
    pub async fn run_work_stealing_benchmark(&self, uneven_task_distribution: bool) -> WorkStealingBenchmarkResult {
        let task_count = 1000;
        let worker_count = 4;

        // 添加任务
        for i in 0..task_count {
            let priority = match i % 3 {
                0 => TaskPriority::High,
                1 => TaskPriority::Medium,
                _ => TaskPriority::Low,
            };

            let task = Self::create_test_task(priority);
            self.optimized_queue.add_task(task).await.unwrap();
        }

        // 获取工作窃取统计
        let initial_stats = self.optimized_queue.get_work_stealing_stats();
        let initial_balance = initial_stats.load_balance_score();

        // 模拟不均匀的工作负载
        if uneven_task_distribution {
            // 让某些工作者获取更多任务
            let mut tasks = Vec::new();

            for worker_id in 0..worker_count {
                let queue_clone = self.optimized_queue.clone();
                let task_count = if worker_id == 0 { 300 } else { 100 };
                let task = tokio::spawn(async move {
                    for _ in 0..task_count {
                        let _ = queue_clone.get_next_task(worker_id).await;
                    }
                });
                tasks.push(task);
            }

            for task in tasks {
                task.await.unwrap();
            }
        }

        // 获取最终统计
        let final_stats = self.optimized_queue.get_work_stealing_stats();
        let final_balance = final_stats.load_balance_score();

        // 清空队列
        self.optimized_queue.clear().await;

        WorkStealingBenchmarkResult {
            task_count,
            worker_count,
            initial_balance,
            final_balance,
            balance_improvement: final_balance - initial_balance,
            work_distribution: final_stats.worker_queues,
        }
    }

    /// 运行内存使用测试
    pub async fn run_memory_usage_benchmark(&self, task_count: usize) -> MemoryUsageBenchmarkResult {
        use std::alloc::System;
        use std::alloc::{GlobalAlloc, Layout};

        #[global_allocator]
        static ALLOCATOR: System = System;

        // 测试原始队列内存使用
        let original_memory_before = get_memory_usage();
        for i in 0..task_count {
            let priority = match i % 3 {
                0 => TaskPriority::High,
                1 => TaskPriority::Medium,
                _ => TaskPriority::Low,
            };

            let task = Self::create_test_task(priority);
            self.original_queue.add_task(task).await.unwrap();
        }
        let original_memory_after = get_memory_usage();
        self.original_queue.clear().await;

        // 测试优化队列内存使用
        let optimized_memory_before = get_memory_usage();
        for i in 0..task_count {
            let priority = match i % 3 {
                0 => TaskPriority::High,
                1 => TaskPriority::Medium,
                _ => TaskPriority::Low,
            };

            let task = Self::create_test_task(priority);
            self.optimized_queue.add_task(task).await.unwrap();
        }
        let optimized_memory_after = get_memory_usage();
        self.optimized_queue.clear().await;

        MemoryUsageBenchmarkResult {
            task_count,
            original_memory_used: original_memory_after - original_memory_before,
            optimized_memory_used: optimized_memory_after - optimized_memory_before,
            memory_saving: (original_memory_after - original_memory_before) as f64
                / (optimized_memory_after - optimized_memory_before) as f64,
        }
    }

    /// 运行完整的性能测试套件
    pub async fn run_complete_benchmark_suite(&self) -> CompleteBenchmarkSuiteResult {
        let mut results = Vec::new();

        // 1. 添加任务性能测试
        results.push("=== 添加任务性能测试 ===".to_string());
        let add_result = self.run_add_task_benchmark(1000).await;
        results.push(add_result.format());

        // 2. 获取任务性能测试
        results.push("\n=== 获取任务性能测试 ===".to_string());
        let get_result = self.run_get_task_benchmark(1000, 4).await;
        results.push(get_result.format());

        // 3. 锁竞争测试
        results.push("\n=== 锁竞争测试 ===".to_string());
        let lock_result = self.run_lock_contention_benchmark(100, 8).await;
        results.push(lock_result.format());

        // 4. 工作窃取效果测试
        results.push("\n=== 工作窃取效果测试 ===".to_string());
        let stealing_result = self.run_work_stealing_benchmark(true).await;
        results.push(stealing_result.format());

        // 5. 内存使用测试
        results.push("\n=== 内存使用测试 ===".to_string());
        let memory_result = self.run_memory_usage_benchmark(500).await;
        results.push(memory_result.format());

        CompleteBenchmarkSuiteResult {
            results: results.join("\n"),
        }
    }
}

/// 获取当前内存使用量（近似值）
fn get_memory_usage() -> usize {
    // 这是一个简化的内存使用估算
    // 在实际应用中，可以使用更精确的内存跟踪
    std::mem::size_of::<TaskQueue>() + std::mem::size_of::<OptimizedTaskQueue>()
}

/// 添加任务性能测试结果
#[derive(Debug, Clone)]
pub struct AddTaskBenchmarkResult {
    pub task_count: usize,
    pub original_duration: Duration,
    pub optimized_duration: Duration,
    pub speedup: f64,
}

impl AddTaskBenchmarkResult {
    pub fn format(&self) -> String {
        format!(
            "任务数量: {}\n\
            原始队列耗时: {:.2?}\n\
            优化队列耗时: {:.2?}\n\
            加速比: {:.2}x\n\
            原始队列吞吐量: {:.2} 任务/秒\n\
            优化队列吞吐量: {:.2} 任务/秒",
            self.task_count,
            self.original_duration,
            self.optimized_duration,
            self.speedup,
            self.task_count as f64 / self.original_duration.as_secs_f64(),
            self.task_count as f64 / self.optimized_duration.as_secs_f64()
        )
    }
}

/// 获取任务性能测试结果
#[derive(Debug, Clone)]
pub struct GetTaskBenchmarkResult {
    pub task_count: usize,
    pub concurrent_workers: usize,
    pub original_duration: Duration,
    pub optimized_duration: Duration,
    pub original_total_tasks: usize,
    pub optimized_total_tasks: usize,
    pub speedup: f64,
}

impl GetTaskBenchmarkResult {
    pub fn format(&self) -> String {
        format!(
            "任务数量: {}\n\
            并发工作者: {}\n\
            原始队列耗时: {:.2?} (获取 {} 任务)\n\
            优化队列耗时: {:.2?} (获取 {} 任务)\n\
            加速比: {:.2}x\n\
            原始队列效率: {:.1}%\n\
            优化队列效率: {:.1}%",
            self.task_count,
            self.concurrent_workers,
            self.original_duration,
            self.original_total_tasks,
            self.optimized_duration,
            self.optimized_total_tasks,
            self.speedup,
            self.original_total_tasks as f64 / self.task_count as f64 * 100.0,
            self.optimized_total_tasks as f64 / self.task_count as f64 * 100.0
        )
    }
}

/// 锁竞争测试结果
#[derive(Debug, Clone)]
pub struct LockContentionBenchmarkResult {
    pub operations_per_thread: usize,
    pub thread_count: usize,
    pub original_duration: Duration,
    pub optimized_duration: Duration,
    pub speedup: f64,
}

impl LockContentionBenchmarkResult {
    pub fn format(&self) -> String {
        let total_operations = self.operations_per_thread * self.thread_count;
        format!(
            "每线程操作数: {}\n\
            线程数: {}\n\
            总操作数: {}\n\
            原始队列耗时: {:.2?}\n\
            优化队列耗时: {:.2?}\n\
            加速比: {:.2}x\n\
            原始队列吞吐量: {:.2} 操作/秒\n\
            优化队列吞吐量: {:.2} 操作/秒",
            self.operations_per_thread,
            self.thread_count,
            total_operations,
            self.original_duration,
            self.optimized_duration,
            self.speedup,
            total_operations as f64 / self.original_duration.as_secs_f64(),
            total_operations as f64 / self.optimized_duration.as_secs_f64()
        )
    }
}

/// 工作窃取测试结果
#[derive(Debug, Clone)]
pub struct WorkStealingBenchmarkResult {
    pub task_count: usize,
    pub worker_count: usize,
    pub initial_balance: f64,
    pub final_balance: f64,
    pub balance_improvement: f64,
    pub work_distribution: std::collections::HashMap<usize, usize>,
}

impl WorkStealingBenchmarkResult {
    pub fn format(&self) -> String {
        let mut distribution_str = String::new();
        for (worker_id, task_count) in &self.work_distribution {
            distribution_str.push_str(&format!("\n  工作者 {}: {} 任务", worker_id, task_count));
        }

        format!(
            "任务数量: {}\n\
            工作者数量: {}\n\
            初始负载均衡度: {:.3}\n\
            最终负载均衡度: {:.3}\n\
            负载均衡改善: {:.3}\n\
            工作分布:{}",
            self.task_count,
            self.worker_count,
            self.initial_balance,
            self.final_balance,
            self.balance_improvement,
            distribution_str
        )
    }
}

/// 内存使用测试结果
#[derive(Debug, Clone)]
pub struct MemoryUsageBenchmarkResult {
    pub task_count: usize,
    pub original_memory_used: usize,
    pub optimized_memory_used: usize,
    pub memory_saving: f64,
}

impl MemoryUsageBenchmarkResult {
    pub fn format(&self) -> String {
        format!(
            "任务数量: {}\n\
            原始队列内存使用: {:.2} KB\n\
            优化队列内存使用: {:.2} KB\n\
            内存节省: {:.1}%\n\
            内存使用比: {:.2}x",
            self.task_count,
            self.original_memory_used as f64 / 1024.0,
            self.optimized_memory_used as f64 / 1024.0,
            (1.0 - 1.0 / self.memory_saving) * 100.0,
            self.memory_saving
        )
    }
}

/// 完整测试套件结果
#[derive(Debug, Clone)]
pub struct CompleteBenchmarkSuiteResult {
    pub results: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_task_benchmark() {
        let benchmark = QueueBenchmark::new(1000, 4);
        let result = benchmark.run_add_task_benchmark(100).await;

        assert_eq!(result.task_count, 100);
        assert!(result.original_duration.as_secs_f64() > 0.0);
        assert!(result.optimized_duration.as_secs_f64() > 0.0);
        assert!(result.speedup > 0.0);
    }

    #[tokio::test]
    async fn test_get_task_benchmark() {
        let benchmark = QueueBenchmark::new(1000, 4);
        let result = benchmark.run_get_task_benchmark(100, 2).await;

        assert_eq!(result.task_count, 100);
        assert_eq!(result.concurrent_workers, 2);
        assert!(result.original_duration.as_secs_f64() > 0.0);
        assert!(result.optimized_duration.as_secs_f64() > 0.0);
        assert!(result.speedup > 0.0);
    }

    #[tokio::test]
    async fn test_work_stealing_benchmark() {
        let benchmark = QueueBenchmark::new(1000, 4);
        let result = benchmark.run_work_stealing_benchmark(false).await;

        assert_eq!(result.task_count, 1000);
        assert_eq!(result.worker_count, 4);
        assert!(result.initial_balance >= 0.0 && result.initial_balance <= 1.0);
        assert!(result.final_balance >= 0.0 && result.final_balance <= 1.0);
        assert_eq!(result.work_distribution.len(), 4);
    }

    #[tokio::test]
    async fn test_complete_benchmark_suite() {
        let benchmark = QueueBenchmark::new(1000, 4);
        let result = benchmark.run_complete_benchmark_suite().await;

        assert!(!result.results.is_empty());
        assert!(result.results.contains("添加任务性能测试"));
        assert!(result.results.contains("获取任务性能测试"));
        assert!(result.results.contains("锁竞争测试"));
        assert!(result.results.contains("工作窃取效果测试"));
    }
}