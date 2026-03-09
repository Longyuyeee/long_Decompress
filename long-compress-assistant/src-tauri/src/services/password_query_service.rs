use anyhow::{Context, Result};
use crate::services::encrypted_password_service::EncryptedPasswordService;
use crate::models::password::{
    PasswordEntry, PasswordCategory, PasswordStrength, PasswordGroup,
};
use crate::services::password_strength_service::{
    PasswordAuditResult, PasswordIssue, PasswordIssueType, IssueSeverity
};
use crate::database::models::{
    PasswordEntryDb, PasswordGroupDb, PasswordGroupEntryDb, PasswordAuditDb,
    PasswordUsageHistoryDb, PasswordPolicyDb
};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, query, query_as, FromRow};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use chrono::{DateTime, Utc, TimeZone};
use uuid::Uuid;

/// 密码查询请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordQueryRequest {
    // 搜索条件
    pub query: Option<String>,
    pub category: Option<PasswordCategory>,
    pub strength: Option<Vec<PasswordStrength>>,
    pub tags: Option<Vec<String>>,
    pub group_id: Option<String>,

    // 时间范围
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    pub expires_after: Option<DateTime<Utc>>,
    pub expires_before: Option<DateTime<Utc>>,

    // 状态过滤
    pub favorite: Option<bool>,
    pub archived: Option<bool>,
    pub deleted: Option<bool>,

    // 排序选项
    pub sort_by: Option<SortField>,
    pub sort_order: Option<SortOrder>,

    // 分页
    pub page: Option<u32>,
    pub page_size: Option<u32>,

    // 高级选项
    pub include_decrypted: bool,
    pub include_audit_info: bool,
    pub include_usage_stats: bool,
}

/// 排序字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortField {
    Name,
    Category,
    Strength,
    CreatedAt,
    UpdatedAt,
    LastUsed,
    Favorite,
    UsageCount,
}

/// 排序顺序
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// 密码查询响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordQueryResponse {
    pub success: bool,
    pub data: Vec<PasswordEntry>,
    pub pagination: PaginationInfo,
    pub query_stats: QueryStatistics,
    pub suggestions: Vec<SearchSuggestion>,
}

/// 分页信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

/// 查询统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStatistics {
    pub execution_time_ms: u64,
    pub result_count: usize,
    pub cache_hit: bool,
    pub index_used: Vec<String>,
}

/// 搜索建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestion {
    pub text: String,
    pub suggestion_type: SuggestionType,
    pub relevance: f32,
}

/// 建议类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    Category,
    Tag,
    Name,
    Username,
    Url,
}

/// 密码查询服务
pub struct PasswordQueryService {
    db_pool: SqlitePool,
    encrypted_password_service: Arc<EncryptedPasswordService>,
    query_cache: RwLock<HashMap<String, CachedQueryResult>>,
    search_index: RwLock<SearchIndex>,
}

/// 查询缓存结果
struct CachedQueryResult {
    result: Vec<String>, // 条目ID列表
    timestamp: DateTime<Utc>,
    ttl_seconds: i64,
}

/// 搜索索引
struct SearchIndex {
    // 名称索引（小写，用于不区分大小写搜索）
    name_index: HashMap<String, Vec<String>>,
    // 分类索引
    category_index: HashMap<String, Vec<String>>,
    // 标签索引
    tag_index: HashMap<String, Vec<String>>,
    // 时间索引
    created_index: Vec<(DateTime<Utc>, String)>,
    updated_index: Vec<(DateTime<Utc>, String)>,
}

impl PasswordQueryService {
    /// 创建新的密码查询服务
    pub fn new(
        db_pool: SqlitePool,
        encrypted_password_service: Arc<EncryptedPasswordService>,
    ) -> Self {
        Self {
            db_pool,
            encrypted_password_service,
            query_cache: RwLock::new(HashMap::new()),
            search_index: RwLock::new(SearchIndex {
                name_index: HashMap::new(),
                category_index: HashMap::new(),
                tag_index: HashMap::new(),
                created_index: Vec::new(),
                updated_index: Vec::new(),
            }),
        }
    }

    /// 搜索密码条目
    pub async fn search_passwords(
        &self,
        request: &PasswordQueryRequest,
    ) -> Result<PasswordQueryResponse> {
        let start_time = std::time::Instant::now();

        // 检查缓存
        let cache_key = self.generate_cache_key(request);
        if let Some(cached) = self.get_from_cache(&cache_key).await {
            let entries = self.fetch_entries(&cached.result, request.include_decrypted).await?;
            let pagination = self.calculate_pagination_info(cached.result.len() as u64, request).await?;

            return Ok(PasswordQueryResponse {
                success: true,
                data: entries,
                pagination,
                query_stats: QueryStatistics {
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    result_count: cached.result.len(),
                    cache_hit: true,
                    index_used: vec!["cache".to_string()],
                },
                suggestions: self.generate_suggestions(request).await?,
            });
        }

        // 构建查询
        let (sql, params) = self.build_query_sql(request).await?;

        // 执行查询
        let mut query = sqlx::query_as::<_, (String,)>(&sql);
        for param in &params {
            query = query.bind(param);
        }
        
        let entry_ids: Vec<String> = query
            .fetch_all(&self.db_pool)
            .await?
            .into_iter()
            .map(|row| row.0)
            .collect();

        // 缓存结果
        self.cache_result(&cache_key, &entry_ids).await?;

        // 获取条目数据
        let entries = self.fetch_entries(&entry_ids, request.include_decrypted).await?;

        // 计算分页信息
        let total_count = self.get_total_count(request).await?;
        let pagination = self.calculate_pagination_info(total_count, request).await?;

        // 更新搜索索引
        self.update_search_index(&entries).await?;

        Ok(PasswordQueryResponse {
            success: true,
            data: entries,
            pagination,
            query_stats: QueryStatistics {
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                result_count: entry_ids.len(),
                cache_hit: false,
                index_used: self.get_used_indexes(request).await?,
            },
            suggestions: self.generate_suggestions(request).await?,
        })
    }

    /// 构建查询SQL
    async fn build_query_sql(&self, request: &PasswordQueryRequest) -> Result<(String, Vec<String>)> {
        let mut conditions = Vec::new();
        let mut params = Vec::new();

        // 基本条件：非删除条目
        conditions.push("pe.deleted = ?".to_string());
        params.push("FALSE".to_string());

        // 查询条件
        if let Some(query) = &request.query {
            if !query.is_empty() {
                let search_param = format!("%{}%", query);
                conditions.push("(pe.name LIKE ? OR pe.tags LIKE ?)".to_string());
                params.push(search_param.clone());
                params.push(search_param);
            }
        }

        // 分类条件
        if let Some(category) = &request.category {
            conditions.push("pe.category = ?".to_string());
            params.push(category.to_string());
        }

        // 强度条件
        if let Some(strengths) = &request.strength {
            if !strengths.is_empty() {
                let strength_values: Vec<String> = strengths.iter()
                    .map(|s| s.to_string())
                    .collect();
                let placeholders = vec!["?"; strength_values.len()].join(", ");
                conditions.push(format!("pe.strength IN ({})", placeholders));
                params.extend(strength_values);
            }
        }

        // 标签条件
        if let Some(tags) = &request.tags {
            if !tags.is_empty() {
                for tag in tags {
                    conditions.push("pe.tags LIKE ?".to_string());
                    params.push(format!("%\"{}\"%", tag));
                }
            }
        }

        // 时间范围条件
        if let Some(after) = request.created_after {
            conditions.push("pe.created_at >= ?".to_string());
            params.push(after.to_rfc3339());
        }
        if let Some(before) = request.created_before {
            conditions.push("pe.created_at <= ?".to_string());
            params.push(before.to_rfc3339());
        }

        if let Some(after) = request.updated_after {
            conditions.push("pe.updated_at >= ?".to_string());
            params.push(after.to_rfc3339());
        }
        if let Some(before) = request.updated_before {
            conditions.push("pe.updated_at <= ?".to_string());
            params.push(before.to_rfc3339());
        }

        // 过期时间条件
        if let Some(after) = request.expires_after {
            conditions.push("(pe.expires_at IS NULL OR pe.expires_at >= ?)".to_string());
            params.push(after.to_rfc3339());
        }
        if let Some(before) = request.expires_before {
            conditions.push("(pe.expires_at IS NULL OR pe.expires_at <= ?)".to_string());
            params.push(before.to_rfc3339());
        }

        // 状态条件
        if let Some(favorite) = request.favorite {
            conditions.push("pe.favorite = ?".to_string());
            params.push(favorite.to_string());
        }

        if let Some(archived) = request.archived {
            conditions.push("pe.archived = ?".to_string());
            params.push(archived.to_string());
        }

        // 构建WHERE子句
        let where_clause = if conditions.is_empty() {
            "".to_string()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // 构建排序子句
        let order_clause = self.build_order_clause(request).await?;

        // 构建分页子句
        let (limit, offset) = self.calculate_limit_offset(request).await?;
        let limit_clause = if limit > 0 {
            format!("LIMIT ? OFFSET ?")
        } else {
            "".to_string()
        };

        // 完整SQL
        let sql = format!(
            "SELECT pe.id FROM password_entries pe {} {} {}",
            where_clause, order_clause, limit_clause
        );

        // 添加分页参数
        if limit > 0 {
            params.push(limit.to_string());
            params.push(offset.to_string());
        }

        Ok((sql, params))
    }

    /// 构建排序子句
    async fn build_order_clause(&self, request: &PasswordQueryRequest) -> Result<String> {
        let sort_by = request.sort_by.as_ref().unwrap_or(&SortField::UpdatedAt);
        let sort_order = request.sort_order.as_ref().unwrap_or(&SortOrder::Desc);

        let order_field = match sort_by {
            SortField::Name => "pe.name",
            SortField::Category => "pe.category",
            SortField::Strength => "
                CASE pe.strength
                    WHEN 'VeryStrong' THEN 5
                    WHEN 'Strong' THEN 4
                    WHEN 'Medium' THEN 3
                    WHEN 'Weak' THEN 2
                    ELSE 1
                END",
            SortField::CreatedAt => "pe.created_at",
            SortField::UpdatedAt => "pe.updated_at",
            SortField::LastUsed => "pe.last_used",
            SortField::Favorite => "pe.favorite",
            SortField::UsageCount => {
                // 需要子查询获取使用次数
                return Ok("".to_string());
            }
        };

        let order_direction = match sort_order {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        };

        Ok(format!("ORDER BY {} {}", order_field, order_direction))
    }

    /// 计算分页参数
    async fn calculate_limit_offset(&self, request: &PasswordQueryRequest) -> Result<(i64, i64)> {
        let page = request.page.unwrap_or(1);
        let page_size = request.page_size.unwrap_or(50);

        let limit = page_size as i64;
        let offset = ((page - 1) * page_size) as i64;

        Ok((limit, offset))
    }

    /// 获取总记录数
    async fn get_total_count(&self, request: &PasswordQueryRequest) -> Result<u64> {
        let (sql, params) = self.build_count_sql(request).await?;

        let mut query = sqlx::query_as::<_, (i64,)>(&sql);
        for param in &params {
            query = query.bind(param);
        }

        let count: (i64,) = query
            .fetch_one(&self.db_pool)
            .await?;

        Ok(count.0 as u64)
    }

    /// 构建计数SQL
    async fn build_count_sql(&self, request: &PasswordQueryRequest) -> Result<(String, Vec<String>)> {
        let (where_sql, params) = self.build_where_clause(request).await?;

        let sql = format!("SELECT COUNT(*) FROM password_entries pe {}", where_sql);

        Ok((sql, params))
    }

    /// 构建WHERE子句
    async fn build_where_clause(&self, request: &PasswordQueryRequest) -> Result<(String, Vec<String>)> {
        let mut conditions = Vec::new();
        let mut params = Vec::new();

        conditions.push("pe.deleted = ?".to_string());
        params.push("FALSE".to_string());

        // 这里可以添加更多条件，与build_query_sql类似
        // 为了简化，只添加基本条件

        let where_clause = if conditions.is_empty() {
            "".to_string()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        Ok((where_clause, params))
    }

    /// 获取条目数据
    async fn fetch_entries(&self, entry_ids: &[String], include_decrypted: bool) -> Result<Vec<PasswordEntry>> {
        let mut entries = Vec::new();

        for chunk in entry_ids.chunks(50) {
            let ids_param = chunk.join("','");
            let sql = format!(
                "SELECT * FROM password_entries WHERE id IN ('{}')",
                ids_param
            );

            let db_entries: Vec<PasswordEntryDb> = query_as(&sql)
                .fetch_all(&self.db_pool)
                .await?;

            for db_entry in db_entries {
                let entry: PasswordEntry = db_entry.into();
                entries.push(entry);
            }
        }

        // 如果需要解密数据
        if include_decrypted {
            let mut decrypted_entries = Vec::new();
            for entry in entries {
                if let Some(decrypted) = self.encrypted_password_service.get_password(&entry.id).await? {
                    decrypted_entries.push(decrypted);
                }
            }
            entries = decrypted_entries;
        }

        Ok(entries)
    }

    /// 计算分页信息
    async fn calculate_pagination_info(
        &self,
        total: u64,
        request: &PasswordQueryRequest,
    ) -> Result<PaginationInfo> {
        let page = request.page.unwrap_or(1);
        let page_size = request.page_size.unwrap_or(50);

        let total_pages = if total == 0 {
            0
        } else {
            ((total as f64) / (page_size as f64)).ceil() as u32
        };

        Ok(PaginationInfo {
            total,
            page,
            page_size,
            total_pages,
        })
    }

    /// 生成缓存键
    fn generate_cache_key(&self, request: &PasswordQueryRequest) -> String {
        let serialized = serde_json::to_string(request).unwrap_or_default();
        format!("query:{}", blake3::hash(serialized.as_bytes()).to_hex())
    }

    /// 从缓存获取
    async fn get_from_cache(&self, key: &str) -> Option<CachedQueryResult> {
        let cache = self.query_cache.read().ok()?;
        let cached = cache.get(key)?;

        // 检查是否过期
        let now = Utc::now();
        let age = now.signed_duration_since(cached.timestamp);
        if age.num_seconds() > cached.ttl_seconds {
            return None;
        }

        Some(cached.clone())
    }

    /// 缓存结果
    async fn cache_result(&self, key: &str, result: &[String]) -> Result<()> {
        let mut cache = self.query_cache.write()
            .map_err(|e| anyhow::anyhow!("获取写锁失败: {}", e))?;

        cache.insert(
            key.to_string(),
            CachedQueryResult {
                result: result.to_vec(),
                timestamp: Utc::now(),
                ttl_seconds: 300, // 5分钟
            },
        );

        Ok(())
    }

    /// 获取使用的索引
    async fn get_used_indexes(&self, request: &PasswordQueryRequest) -> Result<Vec<String>> {
        let mut indexes = Vec::new();

        // 根据查询条件判断可能使用的索引
        if request.query.is_some() {
            indexes.push("idx_password_entries_search".to_string());
        }

        if request.category.is_some() {
            indexes.push("idx_password_entries_category".to_string());
        }

        if request.updated_after.is_some() || request.updated_before.is_some() {
            indexes.push("idx_password_entries_updated".to_string());
        }

        if request.favorite.is_some() {
            indexes.push("idx_password_entries_favorite".to_string());
        }

        Ok(indexes)
    }

    /// 生成搜索建议
    async fn generate_suggestions(&self, request: &PasswordQueryRequest) -> Result<Vec<SearchSuggestion>> {
        let mut suggestions = Vec::new();

        if let Some(query) = &request.query {
            if query.len() >= 2 {
                // 生成分类建议
                let category_suggestions = self.generate_category_suggestions(query).await?;
                suggestions.extend(category_suggestions);

                // 生成标签建议
                let tag_suggestions = self.generate_tag_suggestions(query).await?;
                suggestions.extend(tag_suggestions);

                // 生成名称建议
                let name_suggestions = self.generate_name_suggestions(query).await?;
                suggestions.extend(name_suggestions);
            }
        }

        // 按相关性排序
        suggestions.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());

        Ok(suggestions.into_iter().take(10).collect())
    }

    /// 生成分类建议
    async fn generate_category_suggestions(&self, query: &str) -> Result<Vec<SearchSuggestion>> {
        let categories = vec![
            ("Personal", "个人"),
            ("Work", "工作"),
            ("Finance", "金融"),
            ("Social", "社交"),
            ("Shopping", "购物"),
            ("Entertainment", "娱乐"),
            ("Education", "教育"),
            ("Travel", "旅行"),
            ("Health", "健康"),
            ("Other", "其他"),
        ];

        let query_lower = query.to_lowercase();
        let mut suggestions = Vec::new();

        for (en, zh) in categories {
            let en_lower = en.to_lowercase();
            let zh_lower = zh.to_lowercase();

            let mut relevance = 0.0;

            if en_lower.contains(&query_lower) {
                relevance = 0.8;
            } else if zh_lower.contains(&query_lower) {
                relevance = 0.9;
            } else if fuzzy_match(query, en, 0.6) || fuzzy_match(query, zh, 0.6) {
                relevance = 0.7;
            }

            if relevance > 0.0 {
                suggestions.push(SearchSuggestion {
                    text: zh.to_string(),
                    suggestion_type: SuggestionType::Category,
                    relevance,
                });
            }
        }

        Ok(suggestions)
    }

    /// 生成标签建议
    async fn generate_tag_suggestions(&self, query: &str) -> Result<Vec<SearchSuggestion>> {
        // 从数据库获取常用标签
        let sql = r#"
            SELECT DISTINCT json_each.value as tag, COUNT(*) as count
            FROM password_entries,
                 json_each(password_entries.tags)
            WHERE password_entries.deleted = FALSE
            GROUP BY tag
            ORDER BY count DESC
            LIMIT 20
        "#;

        let tag_counts: Vec<(String, i64)> = query_as(sql)
            .fetch_all(&self.db_pool)
            .await?;

        let query_lower = query.to_lowercase();
        let mut suggestions = Vec::new();

        for (tag, count) in tag_counts {
            let tag_lower = tag.to_lowercase();

            let mut relevance = 0.0;

            if tag_lower.contains(&query_lower) {
                relevance = 0.8;
            } else if fuzzy_match(query, &tag, 0.6) {
                relevance = 0.6;
            }

            // 根据使用频率调整相关性
            let frequency_factor = (count as f32).min(100.0) / 100.0;
            relevance *= 0.5 + 0.5 * frequency_factor;

            if relevance > 0.0 {
                suggestions.push(SearchSuggestion {
                    text: tag,
                    suggestion_type: SuggestionType::Tag,
                    relevance,
                });
            }
        }

        Ok(suggestions)
    }

    /// 生成名称建议
    async fn generate_name_suggestions(&self, query: &str) -> Result<Vec<SearchSuggestion>> {
        let sql = r#"
            SELECT DISTINCT name
            FROM password_entries
            WHERE deleted = FALSE
            AND name LIKE ?
            ORDER BY updated_at DESC
            LIMIT 10
        "#;

        let search_param = format!("%{}%", query);
        let names: Vec<(String,)> = query_as(sql)
            .bind(search_param)
            .fetch_all(&self.db_pool)
            .await?;

        let mut suggestions = Vec::new();

        for (name,) in names {
            let relevance = calculate_name_relevance(query, &name);
            if relevance > 0.3 {
                suggestions.push(SearchSuggestion {
                    text: name,
                    suggestion_type: SuggestionType::Name,
                    relevance,
                });
            }
        }

        Ok(suggestions)
    }

    /// 更新搜索索引
    async fn update_search_index(&self, entries: &[PasswordEntry]) -> Result<()> {
        let mut index = self.search_index.write()
            .map_err(|e| anyhow::anyhow!("获取写锁失败: {}", e))?;

        // 清空索引
        index.name_index.clear();
        index.category_index.clear();
        index.tag_index.clear();
        index.created_index.clear();
        index.updated_index.clear();

        // 重建索引
        for entry in entries {
            // 名称索引
            let name_lower = entry.name.to_lowercase();
            index.name_index
                .entry(name_lower)
                .or_insert_with(Vec::new)
                .push(entry.id.clone());

            // 分类索引
            let category_str = entry.category.to_string();
            index.category_index
                .entry(category_str)
                .or_insert_with(Vec::new)
                .push(entry.id.clone());

            // 标签索引
            for tag in &entry.tags {
                index.tag_index
                    .entry(tag.clone())
                    .or_insert_with(Vec::new)
                    .push(entry.id.clone());
            }

            // 时间索引
            index.created_index.push((entry.created_at, entry.id.clone()));
            index.updated_index.push((entry.updated_at, entry.id.clone()));
        }

        // 排序时间索引
        index.created_index.sort_by_key(|(time, _)| *time);
        index.updated_index.sort_by_key(|(time, _)| *time);

        Ok(())
    }

    /// 根据ID获取密码条目
    pub async fn get_password_by_id(
        &self,
        id: &str,
        include_decrypted: bool,
    ) -> Result<Option<PasswordEntry>> {
        let sql = "SELECT * FROM password_entries WHERE id = ? AND deleted = FALSE";

        let db_entry: Option<PasswordEntryDb> = query_as(sql)
            .bind(id)
            .fetch_optional(&self.db_pool)
            .await?;

        if let Some(db_entry) = db_entry {
            let mut entry: PasswordEntry = db_entry.into();

            if include_decrypted {
                if let Some(decrypted) = self.encrypted_password_service.get_password(id).await? {
                    entry = decrypted;
                }
            }

            Ok(Some(entry))
        } else {
            Ok(None)
        }
    }

    /// 获取密码建议（自动完成）
    pub async fn get_password_suggestions(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<SearchSuggestion>> {
        let request = PasswordQueryRequest {
            query: Some(query.to_string()),
            page_size: Some(limit),
            include_decrypted: false,
            ..Default::default()
        };

        let response = self.generate_suggestions(&request).await?;
        Ok(response.into_iter().take(limit as usize).collect())
    }

    /// 获取最近使用的密码
    pub async fn get_recent_passwords(&self, limit: u32) -> Result<Vec<PasswordEntry>> {
        let request = PasswordQueryRequest {
            sort_by: Some(SortField::LastUsed),
            sort_order: Some(SortOrder::Desc),
            page_size: Some(limit),
            include_decrypted: false,
            ..Default::default()
        };

        let response = self.search_passwords(&request).await?;
        Ok(response.data)
    }

    /// 获取收藏的密码
    pub async fn get_favorite_passwords(&self) -> Result<Vec<PasswordEntry>> {
        let request = PasswordQueryRequest {
            favorite: Some(true),
            include_decrypted: false,
            ..Default::default()
        };

        let response = self.search_passwords(&request).await?;
        Ok(response.data)
    }

    /// 获取即将过期的密码
    pub async fn get_expiring_passwords(&self, days: i32) -> Result<Vec<PasswordEntry>> {
        let now = Utc::now();
        let threshold = now + chrono::Duration::days(days as i64);

        let request = PasswordQueryRequest {
            expires_before: Some(threshold),
            expires_after: Some(now),
            include_decrypted: false,
            ..Default::default()
        };

        let response = self.search_passwords(&request).await?;
        Ok(response.data)
    }

    /// 获取弱密码
    pub async fn get_weak_passwords(&self) -> Result<Vec<PasswordEntry>> {
        let request = PasswordQueryRequest {
            strength: Some(vec![
                PasswordStrength::VeryWeak,
                PasswordStrength::Weak,
            ]),
            include_decrypted: false,
            ..Default::default()
        };

        let response = self.search_passwords(&request).await?;
        Ok(response.data)
    }
}

impl Default for PasswordQueryRequest {
    fn default() -> Self {
        Self {
            query: None,
            category: None,
            strength: None,
            tags: None,
            group_id: None,
            created_after: None,
            created_before: None,
            updated_after: None,
            updated_before: None,
            expires_after: None,
            expires_before: None,
            favorite: None,
            archived: None,
            deleted: None,
            sort_by: None,
            sort_order: None,
            page: None,
            page_size: None,
            include_decrypted: false,
            include_audit_info: false,
            include_usage_stats: false,
        }
    }
}

impl Clone for CachedQueryResult {
    fn clone(&self) -> Self {
        Self {
            result: self.result.clone(),
            timestamp: self.timestamp,
            ttl_seconds: self.ttl_seconds,
        }
    }
}

/// 模糊匹配函数
fn fuzzy_match(query: &str, target: &str, threshold: f32) -> bool {
    if query.is_empty() || target.is_empty() {
        return false;
    }

    let query_lower = query.to_lowercase();
    let target_lower = target.to_lowercase();

    // 简单包含检查
    if target_lower.contains(&query_lower) {
        return true;
    }

    // 计算编辑距离
    let distance = edit_distance(&query_lower, &target_lower);
    let max_len = query_lower.len().max(target_lower.len()) as f32;

    if max_len == 0.0 {
        return false;
    }

    let similarity = 1.0 - (distance as f32 / max_len);
    similarity >= threshold
}

/// 计算编辑距离
fn edit_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut dp = vec![vec![0; b_len + 1]; a_len + 1];

    for i in 0..=a_len {
        dp[i][0] = i;
    }
    for j in 0..=b_len {
        dp[0][j] = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };

            dp[i][j] = (dp[i - 1][j] + 1)  // 删除
                .min(dp[i][j - 1] + 1)     // 插入
                .min(dp[i - 1][j - 1] + cost); // 替换
        }
    }

    dp[a_len][b_len]
}

/// 计算名称相关性
fn calculate_name_relevance(query: &str, name: &str) -> f32 {
    let query_lower = query.to_lowercase();
    let name_lower = name.to_lowercase();

    if name_lower == query_lower {
        return 1.0;
    }

    if name_lower.starts_with(&query_lower) {
        return 0.9;
    }

    if name_lower.contains(&query_lower) {
        return 0.8;
    }

    // 计算单词匹配
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();
    let name_words: Vec<&str> = name_lower.split_whitespace().collect();

    let mut matched_words = 0;
    for q_word in &query_words {
        for n_word in &name_words {
            if n_word.contains(q_word) {
                matched_words += 1;
                break;
            }
        }
    }

    if !query_words.is_empty() {
        (matched_words as f32) / (query_words.len() as f32) * 0.7
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_password_query_service() {
        // 创建临时数据库
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.display());

        let pool = SqlitePool::connect(&db_url).await.unwrap();

        // 初始化数据库表
        crate::database::migrations::init_tables(&pool).await.unwrap();

        // 创建加密密码服务（模拟）
        let encrypted_service = Arc::new(EncryptedPasswordService::new(temp_dir.path()));

        // 创建查询服务
        let query_service = PasswordQueryService::new(pool, encrypted_service);

        // 测试空查询
        let request = PasswordQueryRequest::default();
        let response = query_service.search_passwords(&request).await.unwrap();

        assert!(response.success);
        assert_eq!(response.data.len(), 0);
        assert_eq!(response.pagination.total, 0);

        // 测试生成建议
        let suggestions = query_service.get_password_suggestions("工作", 5).await.unwrap();
        assert!(!suggestions.is_empty());

        println!("测试通过!");
    }

    #[test]
    fn test_fuzzy_match() {
        assert!(fuzzy_match("test", "testing", 0.6));
        assert!(fuzzy_match("test", "TESTING", 0.6));
        assert!(fuzzy_match("abc", "abcdef", 0.6));
        assert!(!fuzzy_match("xyz", "abcdef", 0.6));
    }

    #[test]
    fn test_calculate_name_relevance() {
        assert_eq!(calculate_name_relevance("test", "test"), 1.0);
        assert_eq!(calculate_name_relevance("test", "testing"), 0.9);
        assert_eq!(calculate_name_relevance("test", "this is a test"), 0.8);
        assert!(calculate_name_relevance("web site", "example website") > 0.3);
    }
}