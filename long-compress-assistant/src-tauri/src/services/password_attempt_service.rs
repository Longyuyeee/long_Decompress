use crate::services::password_query_service::{PasswordQueryService, PasswordQueryRequest};
use crate::models::password::PasswordEntry;
use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use log;
use tokio::sync::Semaphore;

/// 密码尝试策略
#[derive(Debug, Clone)]
pub enum PasswordAttemptStrategy {
    /// 尝试所有密码
    All,
    /// 尝试最近使用的密码
    Recent(u32), // 最近N个密码
    /// 尝试特定分类的密码
    Category(String),
    /// 尝试匹配名称的密码
    NameMatch(String),
    /// 自定义密码列表
    Custom(Vec<String>),
}

/// 密码尝试结果
#[derive(Debug, Clone)]
pub struct PasswordAttemptResult {
    pub success: bool,
    pub password: Option<String>,
    pub attempts: usize,
    pub total_passwords: usize,
    pub matched_entry: Option<PasswordEntry>,
    pub error_message: Option<String>,
}

/// 密码尝试进度回调
pub type ProgressCallback = Arc<dyn Fn(usize, usize) + Send + Sync>;

/// 密码尝试配置
#[derive(Clone)]
pub struct PasswordAttemptConfig {
    /// 是否启用并行尝试
    pub enable_parallel: bool,
    /// 并行度（同时尝试的密码数量）
    pub parallelism: usize,
    /// 进度回调
    pub on_progress: Option<ProgressCallback>,
}

impl Default for PasswordAttemptConfig {
    fn default() -> Self {
        Self {
            enable_parallel: true,
            parallelism: 4, // 默认4个并行任务
            on_progress: None,
        }
    }
}

/// 密码尝试服务
pub struct PasswordAttemptService {
    query_service: Arc<PasswordQueryService>,
}

impl PasswordAttemptService {
    /// 创建新的密码尝试服务
    pub fn new(query_service: Arc<PasswordQueryService>) -> Self {
        Self { query_service }
    }

    /// 尝试解压ZIP文件，自动从密码本尝试密码
    pub async fn attempt_extract_with_passwords(
        &self,
        zip_path: &str,
        output_dir: &str,
        strategy: PasswordAttemptStrategy,
    ) -> Result<PasswordAttemptResult> {
        self.attempt_extract_with_passwords_config(
            zip_path,
            output_dir,
            strategy,
            PasswordAttemptConfig::default(),
        ).await
    }

    /// 尝试解压ZIP文件，支持并行化和进度回调
    pub async fn attempt_extract_with_passwords_config(
        &self,
        zip_path: &str,
        output_dir: &str,
        strategy: PasswordAttemptStrategy,
        config: PasswordAttemptConfig,
    ) -> Result<PasswordAttemptResult> {
        log::info!("开始尝试解压归档文件: {}, 策略: {:?}, 并行: {}",
            zip_path, strategy, config.enable_parallel);

        // 获取要尝试的密码列表
        let passwords = self.get_passwords_for_strategy(&strategy).await?;

        if passwords.is_empty() {
            return Ok(PasswordAttemptResult {
                success: false,
                password: None,
                attempts: 0,
                total_passwords: 0,
                matched_entry: None,
                error_message: Some("密码本中没有找到密码".to_string()),
            });
        }

        log::info!("获取到 {} 个密码进行尝试", passwords.len());
        let total = passwords.len();

        // 如果启用并行尝试
        if config.enable_parallel && passwords.len() > 1 {
            self.attempt_parallel(zip_path, output_dir, passwords, config).await
        } else {
            self.attempt_sequential(zip_path, output_dir, passwords, config.on_progress, total).await
        }
    }

    /// 顺序尝试密码
    async fn attempt_sequential(
        &self,
        zip_path: &str,
        output_dir: &str,
        passwords: Vec<(String, Option<PasswordEntry>)>,
        on_progress: Option<ProgressCallback>,
        total: usize,
    ) -> Result<PasswordAttemptResult> {
        for (index, (password, entry)) in passwords.iter().enumerate() {
            log::debug!("尝试第 {} 个密码 (长度: {} 字符)", index + 1, password.len());

            // 调用进度回调
            if let Some(ref callback) = on_progress {
                callback(index + 1, total);
            }

            let attempt_result = self.try_extract_with_password(zip_path, output_dir, password).await;

            match attempt_result {
                Ok(true) => {
                    log::info!("解压成功! 使用的密码来自条目: {}",
                        entry.as_ref().map_or("未知", |e| &e.name)
                    );

                    if let Some(entry) = entry {
                        let _ = self.update_password_usage(&entry.id).await;
                    }

                    return Ok(PasswordAttemptResult {
                        success: true,
                        password: Some(password.clone()),
                        attempts: index + 1,
                        total_passwords: total,
                        matched_entry: entry.clone(),
                        error_message: None,
                    });
                }
                Ok(false) => continue,
                Err(e) => {
                    log::warn!("解压尝试出错: {}", e);
                    return Ok(PasswordAttemptResult {
                        success: false,
                        password: None,
                        attempts: index + 1,
                        total_passwords: total,
                        matched_entry: None,
                        error_message: Some(format!("解压过程出错: {}", e)),
                    });
                }
            }
        }

        log::warn!("所有 {} 个密码尝试失败", total);
        Ok(PasswordAttemptResult {
            success: false,
            password: None,
            attempts: total,
            total_passwords: total,
            matched_entry: None,
            error_message: Some(format!("尝试了 {} 个密码，全部失败", total)),
        })
    }

    /// 并行尝试密码
    async fn attempt_parallel(
        &self,
        zip_path: &str,
        output_dir: &str,
        passwords: Vec<(String, Option<PasswordEntry>)>,
        config: PasswordAttemptConfig,
    ) -> Result<PasswordAttemptResult> {
        let total = passwords.len();
        let parallelism = config.parallelism.min(total).max(1);

        log::info!("并行密码尝试: {} 个密码, 并行度: {}", total, parallelism);

        // 共享状态
        let success = Arc::new(AtomicBool::new(false));
        let attempts = Arc::new(AtomicUsize::new(0));
        let semaphore = Arc::new(Semaphore::new(parallelism));

        // 结果存储
        let result_password = Arc::new(tokio::sync::Mutex::new(None::<String>));
        let result_entry = Arc::new(tokio::sync::Mutex::new(None::<PasswordEntry>));
        let result_attempts = Arc::new(tokio::sync::Mutex::new(0usize));

        let mut tasks = Vec::new();

        for (index, (password, entry)) in passwords.into_iter().enumerate() {
            let zip_path = zip_path.to_string();
            let _output_dir = output_dir.to_string();
            let success_flag = success.clone();
            let attempts_counter = attempts.clone();
            let semaphore = semaphore.clone();
            let result_password = result_password.clone();
            let result_entry = result_entry.clone();
            let result_attempts = result_attempts.clone();
            let on_progress = config.on_progress.clone();
            let query_service = self.query_service.clone();

            let task = tokio::spawn(async move {
                // 如果已经找到正确密码，直接返回
                if success_flag.load(Ordering::SeqCst) {
                    return;
                }

                // 获取信号量许可
                let _permit = semaphore.acquire().await.unwrap();

                // 再次检查是否已经成功
                if success_flag.load(Ordering::SeqCst) {
                    return;
                }

                let current_attempt = attempts_counter.fetch_add(1, Ordering::SeqCst) + 1;

                // 调用进度回调
                if let Some(ref callback) = on_progress {
                    callback(current_attempt, total);
                }

                log::debug!("并行尝试第 {} 个密码 (索引: {}, 长度: {} 字符)",
                    current_attempt, index, password.len());

                // 创建临时服务实例进行密码测试
                let compression_service =
                    crate::services::compression_service::CompressionService::new_with_defaults().await;

                match compression_service.test_archive_password(&zip_path, &password).await {
                    Ok(true) => {
                        // 找到正确密码
                        if !success_flag.swap(true, Ordering::SeqCst) {
                            log::info!("并行尝试成功! 第 {} 次尝试, 密码来自: {}",
                                current_attempt,
                                entry.as_ref().map_or("未知", |e| &e.name)
                            );

                            *result_password.lock().await = Some(password.clone());
                            *result_entry.lock().await = entry.clone();
                            *result_attempts.lock().await = current_attempt;

                            // 更新密码使用记录
                            if let Some(ref e) = entry {
                                let _ = query_service.increment_use_count(&e.id).await;
                            }
                        }
                    }
                    Ok(false) => {
                        // 密码错误，继续
                    }
                    Err(e) => {
                        log::warn!("密码测试出错: {}", e);
                    }
                }
            });

            tasks.push(task);
        }

        // 等待所有任务完成
        for task in tasks {
            let _ = task.await;
        }

        // 检查结果
        if success.load(Ordering::SeqCst) {
            let final_password = result_password.lock().await.clone();
            let final_entry = result_entry.lock().await.clone();
            let final_attempts = *result_attempts.lock().await;

            Ok(PasswordAttemptResult {
                success: true,
                password: final_password,
                attempts: final_attempts,
                total_passwords: total,
                matched_entry: final_entry,
                error_message: None,
            })
        } else {
            log::warn!("并行尝试: 所有 {} 个密码失败", total);
            Ok(PasswordAttemptResult {
                success: false,
                password: None,
                attempts: total,
                total_passwords: total,
                matched_entry: None,
                error_message: Some(format!("尝试了 {} 个密码，全部失败", total)),
            })
        }
    }

    /// 根据策略获取密码列表（带智能排序）
    async fn get_passwords_for_strategy(
        &self,
        strategy: &PasswordAttemptStrategy,
    ) -> Result<Vec<(String, Option<PasswordEntry>)>> {
        let mut passwords = match strategy {
            PasswordAttemptStrategy::All => {
                self.get_all_passwords().await
            }
            PasswordAttemptStrategy::Recent(limit) => {
                self.get_recent_passwords(*limit).await
            }
            PasswordAttemptStrategy::Category(category) => {
                self.get_passwords_by_category(category).await
            }
            PasswordAttemptStrategy::NameMatch(name_pattern) => {
                self.get_passwords_by_name_pattern(name_pattern).await
            }
            PasswordAttemptStrategy::Custom(passwords) => {
                Ok(passwords.iter()
                    .map(|p| (p.clone(), None))
                    .collect())
            }
        }?;

        // 对密码进行智能排序（除了自定义策略）
        if !matches!(strategy, PasswordAttemptStrategy::Custom(_)) {
            self.sort_passwords_by_priority(&mut passwords);
        }

        Ok(passwords)
    }

    /// 按优先级对密码排序
    /// 排序规则：
    /// 1. 使用频率高的优先
    /// 2. 最近使用的优先
    /// 3. 收藏的优先
    /// 4. 密码强度高的优先（可能更常用）
    fn sort_passwords_by_priority(&self, passwords: &mut [(String, Option<PasswordEntry>)]) {
        passwords.sort_by(|a, b| {
            let entry_a = a.1.as_ref();
            let entry_b = b.1.as_ref();

            // 如果没有条目信息，保持原顺序
            if entry_a.is_none() || entry_b.is_none() {
                return std::cmp::Ordering::Equal;
            }

            let ea = entry_a.unwrap();
            let eb = entry_b.unwrap();

            // 1. 优先按使用次数排序（降序）
            let use_count_cmp = eb.use_count.cmp(&ea.use_count);
            if use_count_cmp != std::cmp::Ordering::Equal {
                return use_count_cmp;
            }

            // 2. 按最后使用时间排序（降序，最近的优先）
            if let (Some(last_a), Some(last_b)) = (&ea.last_used, &eb.last_used) {
                let last_used_cmp = last_b.cmp(last_a);
                if last_used_cmp != std::cmp::Ordering::Equal {
                    return last_used_cmp;
                }
            }

            // 3. 收藏的优先
            let fav_cmp = eb.favorite.cmp(&ea.favorite);
            if fav_cmp != std::cmp::Ordering::Equal {
                return fav_cmp;
            }

            // 4. 按密码强度排序（强度高的可能更常用）
            let strength_a = Self::strength_score(&ea.strength);
            let strength_b = Self::strength_score(&eb.strength);
            let strength_cmp = strength_b.cmp(&strength_a);
            if strength_cmp != std::cmp::Ordering::Equal {
                return strength_cmp;
            }

            // 5. 最后按创建时间排序（新的优先）
            eb.created_at.cmp(&ea.created_at)
        });

        log::debug!("密码优先级排序完成，前5个密码使用次数: {:?}",
            passwords.iter().take(5).map(|(_, e)| {
                e.as_ref().map(|entry| entry.use_count).unwrap_or(0)
            }).collect::<Vec<_>>()
        );
    }

    /// 将密码强度转换为分数
    fn strength_score(strength: &crate::models::password::PasswordStrength) -> u8 {
        use crate::models::password::PasswordStrength;
        match strength {
            PasswordStrength::VeryWeak => 1,
            PasswordStrength::Weak => 2,
            PasswordStrength::Medium => 3,
            PasswordStrength::Strong => 4,
            PasswordStrength::VeryStrong => 5,
        }
    }

    /// 获取所有密码
    async fn get_all_passwords(&self) -> Result<Vec<(String, Option<PasswordEntry>)>> {
        let request = PasswordQueryRequest {
            include_decrypted: true, // 需要解密密码
            page_size: Some(1000),   // 获取大量密码
            ..Default::default()
        };

        let response = self.query_service.search_passwords(&request).await?;

        Ok(response.data.into_iter()
            .map(|entry| (entry.password.clone(), Some(entry)))
            .collect())
    }

    /// 获取最近使用的密码
    async fn get_recent_passwords(&self, limit: u32) -> Result<Vec<(String, Option<PasswordEntry>)>> {
        let request = PasswordQueryRequest {
            include_decrypted: true,
            page_size: Some(limit),
            ..Default::default()
        };

        let response = self.query_service.search_passwords(&request).await?;

        Ok(response.data.into_iter()
            .map(|entry| (entry.password.clone(), Some(entry)))
            .collect())
    }

    /// 获取特定分类的密码
    async fn get_passwords_by_category(&self, category: &str) -> Result<Vec<(String, Option<PasswordEntry>)>> {
        use crate::models::password::PasswordCategory;

        // 尝试解析分类
        let category_enum = match category.to_lowercase().as_str() {
            "personal" | "个人" => PasswordCategory::Personal,
            "work" | "工作" => PasswordCategory::Work,
            "finance" | "金融" => PasswordCategory::Finance,
            "social" | "社交" => PasswordCategory::Social,
            "shopping" | "购物" => PasswordCategory::Shopping,
            "entertainment" | "娱乐" => PasswordCategory::Entertainment,
            "education" | "教育" => PasswordCategory::Education,
            "travel" | "旅行" => PasswordCategory::Travel,
            "health" | "健康" => PasswordCategory::Health,
            _ => PasswordCategory::Other,
        };

        let request = PasswordQueryRequest {
            category: Some(category_enum),
            include_decrypted: true,
            page_size: Some(100),
            ..Default::default()
        };

        let response = self.query_service.search_passwords(&request).await?;

        Ok(response.data.into_iter()
            .map(|entry| (entry.password.clone(), Some(entry)))
            .collect())
    }

    /// 获取匹配名称模式的密码
    async fn get_passwords_by_name_pattern(&self, pattern: &str) -> Result<Vec<(String, Option<PasswordEntry>)>> {
        let request = PasswordQueryRequest {
            query: Some(pattern.to_string()),
            include_decrypted: true,
            page_size: Some(100),
            ..Default::default()
        };

        let response = self.query_service.search_passwords(&request).await?;

        Ok(response.data.into_iter()
            .map(|entry| (entry.password.clone(), Some(entry)))
            .collect())
    }

    /// 尝试使用密码解压归档文件（支持所有加密格式）
    pub async fn try_extract_with_password(
        &self,
        archive_path: &str,
        _output_dir: &str, // 实际测试密码不需要解压到目录，只是验证
        password: &str,
    ) -> Result<bool> {
        use crate::services::compression_service::CompressionService;

        // 调用新的通用密码测试功能
        let compression_service = CompressionService::new_with_defaults().await;
        match compression_service.test_archive_password(
            archive_path,
            password,
        ).await {
            Ok(is_valid) => Ok(is_valid),
            Err(e) => {
                // 检查错误类型，如果是其他非密码错误则抛出
                let error_msg: String = e.to_string();
                if error_msg.contains("密码错误") || error_msg.contains("InvalidPassword") {
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// 更新密码使用记录
    async fn update_password_usage(&self, password_id: &str) -> Result<()> {
        log::info!("更新密码使用记录: {}", password_id);
        self.query_service.increment_use_count(password_id).await?;
        Ok(())
    }

    /// 智能猜测密码（基于文件名、路径等上下文）
    pub async fn guess_passwords_from_context(
        &self,
        context: &PasswordGuessContext,
    ) -> Result<Vec<String>> {
        let mut guesses = Vec::new();

        // 基于文件名的猜测
        if let Some(filename) = &context.filename {
            guesses.extend(self.guess_from_filename(filename).await?);
        }

        // 基于路径的猜测
        if let Some(path) = &context.filepath {
            guesses.extend(self.guess_from_filepath(path).await?);
        }

        // 基于创建日期的猜测
        if let Some(date) = &context.creation_date {
            guesses.extend(self.guess_from_date(date).await?);
        }

        // 去重
        guesses.sort();
        guesses.dedup();

        Ok(guesses)
    }

    /// 从文件名猜测密码
    async fn guess_from_filename(&self, filename: &str) -> Result<Vec<String>> {
        let mut guesses = Vec::new();

        // 移除扩展名
        let name_without_ext = filename.split('.').next().unwrap_or(filename);

        // 常见密码模式
        guesses.push(name_without_ext.to_string());
        guesses.push(format!("{}123", name_without_ext));
        guesses.push(format!("{}123456", name_without_ext));
        guesses.push(format!("{}@123", name_without_ext));
        guesses.push(name_without_ext.to_lowercase());
        guesses.push(name_without_ext.to_uppercase());

        // 从密码本中查找相关密码
        let request = PasswordQueryRequest {
            query: Some(name_without_ext.to_string()),
            include_decrypted: true,
            page_size: Some(10),
            ..Default::default()
        };

        if let Ok(response) = self.query_service.search_passwords(&request).await {
            for entry in response.data {
                guesses.push(entry.password);
            }
        }

        Ok(guesses)
    }

    /// 从文件路径猜测密码
    async fn guess_from_filepath(&self, path: &str) -> Result<Vec<String>> {
        let mut guesses = Vec::new();

        // 提取路径中的目录名
        let path_components: Vec<&str> = path.split(['/', '\\']).collect();

        for component in path_components {
            if component.len() > 2 && !component.contains('.') {
                guesses.push(component.to_string());
                guesses.push(format!("{}123", component));
            }
        }

        Ok(guesses)
    }

    /// 从日期猜测密码
    async fn guess_from_date(&self, _date: &str) -> Result<Vec<String>> {
        let mut guesses = Vec::new();

        // 常见日期格式
        let _date_formats = ["YYYYMMDD", "YYYY-MM-DD", "DDMMYYYY", "MMDDYYYY",
            "YYMMDD", "YY-MM-DD", "DDMMYY", "MMDDYY"];

        // 这里应该解析日期并生成各种格式
        // 暂时添加一些常见日期密码
        guesses.push("123456".to_string());
        guesses.push("12345678".to_string());
        guesses.push("111111".to_string());
        guesses.push("000000".to_string());

        Ok(guesses)
    }
}

/// 密码猜测上下文
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct PasswordGuessContext {
    pub filename: Option<String>,
    pub filepath: Option<String>,
    pub creation_date: Option<String>,
    pub file_size: Option<u64>,
    pub file_type: Option<String>,
    pub tags: Vec<String>,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_password_attempt_service() {
        // 创建模拟的查询服务
        // 在实际测试中，应该使用真实的或模拟的服务

        println!("密码尝试服务测试框架就绪");
    }

    #[test]
    fn test_password_attempt_strategy() {
        let strategy1 = PasswordAttemptStrategy::All;
        let strategy2 = PasswordAttemptStrategy::Recent(10);
        let strategy3 = PasswordAttemptStrategy::Category("工作".to_string());
        let strategy4 = PasswordAttemptStrategy::NameMatch("项目".to_string());
        let strategy5 = PasswordAttemptStrategy::Custom(vec!["password123".to_string()]);

        assert!(matches!(strategy1, PasswordAttemptStrategy::All));
        assert!(matches!(strategy2, PasswordAttemptStrategy::Recent(10)));
        assert!(matches!(strategy3, PasswordAttemptStrategy::Category(_)));
        assert!(matches!(strategy4, PasswordAttemptStrategy::NameMatch(_)));
        assert!(matches!(strategy5, PasswordAttemptStrategy::Custom(_)));
    }
}
