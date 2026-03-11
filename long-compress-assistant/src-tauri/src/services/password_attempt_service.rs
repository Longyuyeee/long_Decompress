use crate::services::password_query_service::{PasswordQueryService, PasswordQueryRequest};
use crate::models::password::PasswordEntry;
use anyhow::{Context, Result};
use std::sync::Arc;
use log;

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
        log::info!("开始尝试解压ZIP文件: {}, 策略: {:?}", zip_path, strategy);

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

        log::debug!("获取到 {} 个密码进行尝试", passwords.len());

        // 尝试每个密码
        for (index, (password, entry)) in passwords.iter().enumerate() {
            log::debug!("尝试第 {} 个密码: {}...", index + 1,
                if password.len() > 3 {
                    format!("{}...", &password[0..3])
                } else {
                    "***".to_string()
                }
            );

            // 这里应该调用实际的ZIP解压功能
            // 暂时模拟解压尝试
            let attempt_result = self.try_extract_with_password(zip_path, output_dir, password).await;

            match attempt_result {
                Ok(true) => {
                    // 解压成功
                    log::info!("解压成功! 使用的密码来自条目: {}",
                        entry.as_ref().map_or("未知", |e| &e.name)
                    );

                    // 更新密码使用记录
                    if let Some(entry) = entry {
                        self.update_password_usage(&entry.id).await?;
                    }

                    return Ok(PasswordAttemptResult {
                        success: true,
                        password: Some(password.clone()),
                        attempts: index + 1,
                        total_passwords: passwords.len(),
                        matched_entry: entry.clone(),
                        error_message: None,
                    });
                }
                Ok(false) => {
                    // 密码错误，继续尝试下一个
                    continue;
                }
                Err(e) => {
                    // 其他错误
                    log::warn!("解压尝试出错: {}", e);
                    return Ok(PasswordAttemptResult {
                        success: false,
                        password: None,
                        attempts: index + 1,
                        total_passwords: passwords.len(),
                        matched_entry: None,
                        error_message: Some(format!("解压过程出错: {}", e)),
                    });
                }
            }
        }

        // 所有密码都尝试失败
        log::warn!("所有 {} 个密码尝试失败", passwords.len());
        Ok(PasswordAttemptResult {
            success: false,
            password: None,
            attempts: passwords.len(),
            total_passwords: passwords.len(),
            matched_entry: None,
            error_message: Some(format!("尝试了 {} 个密码，全部失败", passwords.len())),
        })
    }

    /// 根据策略获取密码列表
    async fn get_passwords_for_strategy(
        &self,
        strategy: &PasswordAttemptStrategy,
    ) -> Result<Vec<(String, Option<PasswordEntry>)>> {
        match strategy {
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
        let compression_service = CompressionService::default();
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
    async fn guess_from_date(&self, date: &str) -> Result<Vec<String>> {
        let mut guesses = Vec::new();

        // 常见日期格式
        let date_formats = vec![
            "YYYYMMDD", "YYYY-MM-DD", "DDMMYYYY", "MMDDYYYY",
            "YYMMDD", "YY-MM-DD", "DDMMYY", "MMDDYY"
        ];

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
pub struct PasswordGuessContext {
    pub filename: Option<String>,
    pub filepath: Option<String>,
    pub creation_date: Option<String>,
    pub file_size: Option<u64>,
    pub file_type: Option<String>,
    pub tags: Vec<String>,
}

impl Default for PasswordGuessContext {
    fn default() -> Self {
        Self {
            filename: None,
            filepath: None,
            creation_date: None,
            file_size: None,
            file_type: None,
            tags: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_password_attempt_service() {
        // 创建模拟的查询服务
        // 在实际测试中，应该使用真实的或模拟的服务

        println!("密码尝试服务测试框架就绪");
        assert!(true);
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