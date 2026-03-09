use crate::utils::error::AppError;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tokio::time;

/// 异步任务执行器
pub struct AsyncExecutor {
    max_concurrent: usize,
    semaphore: Arc<Semaphore>,
}

impl AsyncExecutor {
    /// 创建新的异步执行器
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            max_concurrent,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    /// 执行异步任务
    pub async fn execute<F, T>(&self, task: F) -> Result<T, AppError>
    where
        F: Future<Output = Result<T, AppError>>,
    {
        let _permit = self.semaphore
            .acquire()
            .await
            .map_err(|e| AppError::system(format!("获取信号量失败: {}", e)))?;

        task.await
    }

    /// 批量执行异步任务
    pub async fn execute_batch<F, T, I>(&self, tasks: I) -> Vec<Result<T, AppError>>
    where
        F: Future<Output = Result<T, AppError>> + Send + 'static,
        T: Send + 'static,
        I: IntoIterator<Item = F>,
    {
        let mut handles = Vec::new();

        for task in tasks {
            let semaphore = self.semaphore.clone();
            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.ok();
                task.await
            });
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(AppError::system(format!("任务执行失败: {}", e)))),
            }
        }

        results
    }

    /// 获取当前并发数
    pub fn current_concurrency(&self) -> usize {
        self.max_concurrent - self.semaphore.available_permits()
    }

    /// 获取最大并发数
    pub fn max_concurrency(&self) -> usize {
        self.max_concurrent
    }
}

/// 带重试的异步执行
pub struct RetryExecutor {
    max_retries: u32,
    base_delay: Duration,
    max_delay: Duration,
    jitter: bool,
}

impl RetryExecutor {
    /// 创建新的重试执行器
    pub fn new(max_retries: u32, base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            max_retries,
            base_delay,
            max_delay,
            jitter: true,
        }
    }

    /// 禁用抖动
    pub fn without_jitter(mut self) -> Self {
        self.jitter = false;
        self
    }

    /// 执行带重试的异步任务
    pub async fn execute<F, T>(&self, mut task: F) -> Result<T, AppError>
    where
        F: FnMut() -> Pin<Box<dyn Future<Output = Result<T, AppError>> + Send>>,
    {
        let mut retries = 0;
        let mut last_error = None;

        while retries <= self.max_retries {
            match task().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    last_error = Some(err);

                    if retries == self.max_retries {
                        break;
                    }

                    // 计算延迟时间（指数退避）
                    let delay = self.calculate_delay(retries);
                    time::sleep(delay).await;

                    retries += 1;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| AppError::unknown("重试失败")))
    }

    /// 计算延迟时间
    fn calculate_delay(&self, retry_count: u32) -> Duration {
        let exponent = 2u32.pow(retry_count);
        let mut delay = self.base_delay * exponent;

        // 应用最大延迟限制
        if delay > self.max_delay {
            delay = self.max_delay;
        }

        // 应用抖动（随机化延迟）
        if self.jitter {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let jitter = rng.gen_range(0.8..1.2);
            delay = Duration::from_millis((delay.as_millis() as f64 * jitter) as u64);
        }

        delay
    }
}

/// 带超时的异步执行
pub struct TimeoutExecutor {
    timeout: Duration,
}

impl TimeoutExecutor {
    /// 创建新的超时执行器
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// 执行带超时的异步任务
    pub async fn execute<F, T>(&self, task: F) -> Result<T, AppError>
    where
        F: Future<Output = Result<T, AppError>>,
    {
        match time::timeout(self.timeout, task).await {
            Ok(result) => result,
            Err(_) => Err(AppError::timeout(format!(
                "操作超时 ({}秒)",
                self.timeout.as_secs()
            ))),
        }
    }
}

/// 带进度的异步执行
pub struct ProgressExecutor<T> {
    task: Box<dyn FnMut(ProgressReporter) -> Pin<Box<dyn Future<Output = Result<T, AppError>> + Send>> + Send>,
}

impl<T> ProgressExecutor<T> {
    /// 创建新的进度执行器
    pub fn new<F, Fut>(mut task: F) -> Self
    where
        F: FnMut(ProgressReporter) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, AppError>> + Send + 'static,
    {
        Self {
            task: Box::new(move |reporter| Box::pin(task(reporter))),
        }
    }

    /// 执行带进度的异步任务
    pub async fn execute(&mut self) -> Result<T, AppError> {
        let reporter = ProgressReporter::new();
        (self.task)(reporter.clone()).await
    }
}

/// 进度报告器
#[derive(Clone)]
pub struct ProgressReporter {
    progress: Arc<Mutex<ProgressInfo>>,
}

impl ProgressReporter {
    /// 创建新的进度报告器
    pub fn new() -> Self {
        Self {
            progress: Arc::new(Mutex::new(ProgressInfo::default())),
        }
    }

    /// 更新进度
    pub async fn update(&self, current: u64, total: u64) {
        let mut progress = self.progress.lock().await;
        progress.current = current;
        progress.total = total;

        if total > 0 {
            progress.percentage = (current as f64 / total as f64 * 100.0).round() as u8;
        }
    }

    /// 更新消息
    pub async fn update_message(&self, message: String) {
        let mut progress = self.progress.lock().await;
        progress.message = message;
    }

    /// 获取当前进度信息
    pub async fn get_info(&self) -> ProgressInfo {
        self.progress.lock().await.clone()
    }

    /// 检查是否已取消
    pub async fn is_cancelled(&self) -> bool {
        self.progress.lock().await.cancelled
    }

    /// 取消任务
    pub async fn cancel(&self) {
        let mut progress = self.progress.lock().await;
        progress.cancelled = true;
    }
}

/// 进度信息
#[derive(Debug, Clone, Default)]
pub struct ProgressInfo {
    pub current: u64,
    pub total: u64,
    pub percentage: u8,
    pub message: String,
    pub cancelled: bool,
}

/// 异步缓存
pub struct AsyncCache<K, V> {
    cache: Arc<Mutex<std::collections::HashMap<K, CacheEntry<V>>>>,
    ttl: Duration,
}

impl<K, V> AsyncCache<K, V>
where
    K: Eq + std::hash::Hash + Clone + Send + 'static,
    V: Clone + Send + 'static,
{
    /// 创建新的异步缓存
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
            ttl,
        }
    }

    /// 获取或计算值
    pub async fn get_or_compute<F>(&self, key: K, computer: F) -> Result<V, AppError>
    where
        F: Future<Output = Result<V, AppError>>,
    {
        let now = Instant::now();

        // 检查缓存
        {
            let cache = self.cache.lock().await;
            if let Some(entry) = cache.get(&key) {
                if now.duration_since(entry.timestamp) < self.ttl {
                    return Ok(entry.value.clone());
                }
            }
        }

        // 计算新值
        let value = computer.await?;

        // 更新缓存
        {
            let mut cache = self.cache.lock().await;
            cache.insert(key, CacheEntry {
                value: value.clone(),
                timestamp: now,
            });
        }

        Ok(value)
    }

    /// 清除缓存
    pub async fn clear(&self) {
        let mut cache = self.cache.lock().await;
        cache.clear();
    }

    /// 清除过期条目
    pub async fn cleanup(&self) {
        let now = Instant::now();
        let mut cache = self.cache.lock().await;

        cache.retain(|_, entry| now.duration_since(entry.timestamp) < self.ttl);
    }

    /// 获取缓存大小
    pub async fn size(&self) -> usize {
        let cache = self.cache.lock().await;
        cache.len()
    }
}

/// 缓存条目
struct CacheEntry<V> {
    value: V,
    timestamp: Instant,
}

/// 异步限流器
pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
    refill_interval: Duration,
    max_tokens: usize,
    refill_task: tokio::task::JoinHandle<()>,
}

impl RateLimiter {
    /// 创建新的限流器
    pub fn new(max_tokens: usize, refill_interval: Duration) -> Self {
        let semaphore = Arc::new(Semaphore::new(max_tokens));

        // 启动令牌补充任务
        let refill_semaphore = semaphore.clone();
        let refill_task = tokio::spawn(async move {
            let mut interval = time::interval(refill_interval);
            loop {
                interval.tick().await;

                let current_permits = refill_semaphore.available_permits();
                if current_permits < max_tokens {
                    refill_semaphore.add_permits(1);
                }
            }
        });

        Self {
            semaphore,
            refill_interval,
            max_tokens,
            refill_task,
        }
    }

    /// 获取令牌
    pub async fn acquire(&self) -> Result<(), AppError> {
        self.semaphore
            .acquire()
            .await
            .map(|_| ())
            .map_err(|e| AppError::system(format!("获取令牌失败: {}", e)))
    }

    /// 尝试获取令牌（非阻塞）
    pub fn try_acquire(&self) -> bool {
        self.semaphore.try_acquire().is_ok()
    }

    /// 获取当前可用令牌数
    pub fn available_tokens(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// 停止限流器
    pub fn stop(self) {
        self.refill_task.abort();
    }
}

/// 异步批处理器
pub struct BatchProcessor<T> {
    batch_size: usize,
    batch_timeout: Duration,
    buffer: Arc<Mutex<Vec<T>>>,
    processor: Arc<dyn Fn(Vec<T>) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send>> + Send + Sync>,
    flush_task: Option<tokio::task::JoinHandle<()>>,
}

impl<T> BatchProcessor<T>
where
    T: Send + 'static,
{
    /// 创建新的批处理器
    pub fn new<F, Fut>(
        batch_size: usize,
        batch_timeout: Duration,
        processor: F,
    ) -> Self
    where
        F: Fn(Vec<T>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), AppError>> + Send + 'static,
    {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let processor: Arc<dyn Fn(Vec<T>) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send>> + Send + Sync> = 
            Arc::new(move |items| Box::pin(processor(items)));

        // 启动定时刷新任务
        let flush_buffer = buffer.clone();
        let flush_processor = processor.clone();
        let flush_task = tokio::spawn(async move {
            let mut interval = time::interval(batch_timeout);
            loop {
                interval.tick().await;

                let mut buffer = flush_buffer.lock().await;
                if !buffer.is_empty() {
                    let items = std::mem::take(&mut *buffer);
                    let processor = flush_processor.clone();
                    tokio::spawn(async move {
                        let _ = processor(items).await;
                    });
                }
            }
        });

        Self {
            batch_size,
            batch_timeout,
            buffer,
            processor,
            flush_task: Some(flush_task),
        }
    }

    /// 添加项目
    pub async fn add(&self, item: T) -> Result<(), AppError> {
        let mut buffer = self.buffer.lock().await;
        buffer.push(item);

        if buffer.len() >= self.batch_size {
            let items = std::mem::take(&mut *buffer);
            let processor = self.processor.clone();
            tokio::spawn(async move {
                let _ = processor(items).await;
            });
        }

        Ok(())
    }

    /// 强制刷新缓冲区
    pub async fn flush(&self) -> Result<(), AppError> {
        let mut buffer = self.buffer.lock().await;
        if !buffer.is_empty() {
            let items = std::mem::take(&mut *buffer);
            (self.processor)(items).await
        } else {
            Ok(())
        }
    }

    /// 停止批处理器
    pub async fn stop(mut self) -> Result<(), AppError> {
        // 停止刷新任务
        if let Some(task) = self.flush_task.take() {
            task.abort();
        }

        // 刷新剩余项目
        self.flush().await
    }
}