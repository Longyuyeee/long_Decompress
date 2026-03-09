use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, FromRow};
use uuid::Uuid;

use crate::database::connection::get_connection;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasswordCategory {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCategoryRequest {
    pub id: String,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CategoryStatistics {
    pub category_id: String,
    pub category_name: String,
    pub display_name: String,
    pub password_count: i64,
    pub average_strength: f64,
    pub expired_count: i64,
    pub favorite_count: i64,
}

pub struct PasswordCategoryService;

impl PasswordCategoryService {
    pub fn new() -> Self {
        Self
    }

    /// 获取所有密码分类
    pub async fn get_all_categories(&self) -> Result<Vec<PasswordCategory>> {
        let db = get_connection().await?;
        let pool = db.pool();

        let categories: Vec<PasswordCategory> = sqlx::query_as(
            r#"
            SELECT
                id, name, display_name, description, icon, color, sort_order, created_at, updated_at
            FROM password_categories
            ORDER BY sort_order ASC, name ASC
            "#
        )
        .fetch_all(pool)
        .await
        .context("获取密码分类失败")?;

        Ok(categories)
    }

    /// 根据ID获取密码分类
    pub async fn get_category_by_id(&self, id: &str) -> Result<Option<PasswordCategory>> {
        let db = get_connection().await?;
        let pool = db.pool();

        let category: Option<PasswordCategory> = sqlx::query_as(
            r#"
            SELECT
                id, name, display_name, description, icon, color, sort_order, created_at, updated_at
            FROM password_categories
            WHERE id = ?
            "#
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("根据ID获取密码分类失败")?;

        Ok(category)
    }

    /// 根据名称获取密码分类
    pub async fn get_category_by_name(&self, name: &str) -> Result<Option<PasswordCategory>> {
        let db = get_connection().await?;
        let pool = db.pool();

        let category: Option<PasswordCategory> = sqlx::query_as(
            r#"
            SELECT
                id, name, display_name, description, icon, color, sort_order, created_at, updated_at
            FROM password_categories
            WHERE name = ?
            "#
        )
        .bind(name)
        .fetch_optional(pool)
        .await
        .context("根据名称获取密码分类失败")?;

        Ok(category)
    }

    /// 创建新的密码分类
    pub async fn create_category(&self, request: CreateCategoryRequest) -> Result<PasswordCategory> {
        let db = get_connection().await?;
        let pool = db.pool();

        // 检查分类名称是否已存在
        let existing = self.get_category_by_name(&request.name).await?;
        if existing.is_some() {
            return Err(anyhow::anyhow!("分类名称已存在: {}", request.name));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let sort_order = request.sort_order.unwrap_or(0);

        let category = PasswordCategory {
            id: id.clone(),
            name: request.name,
            display_name: request.display_name,
            description: request.description,
            icon: request.icon,
            color: request.color,
            sort_order,
            created_at: now,
            updated_at: now,
        };

        sqlx::query(
            r#"
            INSERT INTO password_categories
            (id, name, display_name, description, icon, color, sort_order, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&category.id)
        .bind(&category.name)
        .bind(&category.display_name)
        .bind(&category.description)
        .bind(&category.icon)
        .bind(&category.color)
        .bind(category.sort_order)
        .bind(category.created_at)
        .bind(category.updated_at)
        .execute(pool)
        .await
        .context("创建密码分类失败")?;

        Ok(category)
    }

    /// 更新密码分类
    pub async fn update_category(&self, request: UpdateCategoryRequest) -> Result<PasswordCategory> {
        let db = get_connection().await?;
        let pool = db.pool();

        // 获取现有分类
        let mut category = self.get_category_by_id(&request.id).await?
            .ok_or_else(|| anyhow::anyhow!("分类不存在: {}", request.id))?;

        // 更新字段
        if let Some(name) = request.name {
            // 检查新名称是否与其他分类冲突
            if name != category.name {
                let existing = self.get_category_by_name(&name).await?;
                if existing.is_some() {
                    return Err(anyhow::anyhow!("分类名称已存在: {}", name));
                }
                category.name = name;
            }
        }

        if let Some(display_name) = request.display_name {
            category.display_name = display_name;
        }

        if let Some(description) = request.description {
            category.description = Some(description);
        }

        if let Some(icon) = request.icon {
            category.icon = Some(icon);
        }

        if let Some(color) = request.color {
            category.color = Some(color);
        }

        if let Some(sort_order) = request.sort_order {
            category.sort_order = sort_order;
        }

        category.updated_at = Utc::now();

        // 更新数据库
        sqlx::query(
            r#"
            UPDATE password_categories SET
                name = ?, display_name = ?, description = ?, icon = ?, color = ?,
                sort_order = ?, updated_at = ?
            WHERE id = ?
            "#
        )
        .bind(&category.name)
        .bind(&category.display_name)
        .bind(&category.description)
        .bind(&category.icon)
        .bind(&category.color)
        .bind(category.sort_order)
        .bind(category.updated_at)
        .bind(&category.id)
        .execute(pool)
        .await
        .context("更新密码分类失败")?;

        Ok(category)
    }

    /// 删除密码分类
    pub async fn delete_category(&self, id: &str) -> Result<()> {
        let db = get_connection().await?;
        let pool = db.pool();

        // 检查分类是否存在
        let category = self.get_category_by_id(id).await?;
        if category.is_none() {
            return Err(anyhow::anyhow!("分类不存在: {}", id));
        }

        // 检查是否有密码使用此分类
        let password_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM password_entries WHERE category = ? AND deleted = FALSE"
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .context("检查分类使用情况失败")?;

        if password_count.0 > 0 {
            return Err(anyhow::anyhow!("分类正在被 {} 个密码使用，无法删除", password_count.0));
        }

        // 删除分类
        sqlx::query(
            "DELETE FROM password_categories WHERE id = ?"
        )
        .bind(id)
        .execute(pool)
        .await
        .context("删除密码分类失败")?;

        Ok(())
    }

    /// 重新排序分类
    pub async fn reorder_categories(&self, category_orders: Vec<(String, i32)>) -> Result<()> {
        let db = get_connection().await?;
        let pool = db.pool();

        // 开始事务
        let mut transaction = pool.begin().await.context("开始事务失败")?;

        for (category_id, sort_order) in category_orders {
            sqlx::query(
                "UPDATE password_categories SET sort_order = ?, updated_at = ? WHERE id = ?"
            )
            .bind(sort_order)
            .bind(Utc::now())
            .bind(&category_id)
            .execute(&mut *transaction)
            .await
            .context("更新分类排序失败")?;
        }

        transaction.commit().await.context("提交事务失败")?;

        Ok(())
    }

    /// 获取分类统计信息
    pub async fn get_category_statistics(&self) -> Result<Vec<CategoryStatistics>> {
        let db = get_connection().await?;
        let pool = db.pool();

        let stats: Vec<CategoryStatistics> = sqlx::query_as(
            r#"
            SELECT
                pc.id as category_id,
                pc.name as category_name,
                pc.display_name as display_name,
                COUNT(pe.id) as password_count,
                AVG(CASE
                    WHEN pe.strength = 'VeryWeak' THEN 1
                    WHEN pe.strength = 'Weak' THEN 2
                    WHEN pe.strength = 'Medium' THEN 3
                    WHEN pe.strength = 'Strong' THEN 4
                    WHEN pe.strength = 'VeryStrong' THEN 5
                    ELSE 0
                END) as average_strength,
                SUM(CASE WHEN pe.expires_at IS NOT NULL AND pe.expires_at < ? THEN 1 ELSE 0 END) as expired_count,
                SUM(CASE WHEN pe.favorite = TRUE THEN 1 ELSE 0 END) as favorite_count
            FROM password_categories pc
            LEFT JOIN password_entries pe ON pc.name = pe.category AND pe.deleted = FALSE
            GROUP BY pc.id, pc.name, pc.display_name
            ORDER BY pc.sort_order ASC, pc.name ASC
            "#
        )
        .bind(Utc::now())
        .fetch_all(pool)
        .await
        .context("获取分类统计信息失败")?;

        Ok(stats)
    }

    /// 获取分类使用情况（按时间统计）
    pub async fn get_category_usage_over_time(&self, days: i32) -> Result<Vec<(String, Vec<(String, i32)>)>> {
        let db = get_connection().await?;
        let pool = db.pool();

        let start_date = Utc::now() - chrono::Duration::days(days as i64);

        let usage: Vec<(String, String, i32)> = sqlx::query_as(
            r#"
            SELECT
                pc.name as category_name,
                DATE(pe.created_at) as date,
                COUNT(pe.id) as count
            FROM password_categories pc
            LEFT JOIN password_entries pe ON pc.name = pe.category
                AND pe.deleted = FALSE
                AND pe.created_at >= ?
            WHERE pe.created_at IS NOT NULL
            GROUP BY pc.name, DATE(pe.created_at)
            ORDER BY pc.name, DATE(pe.created_at)
            "#
        )
        .bind(start_date)
        .fetch_all(pool)
        .await
        .context("获取分类使用情况失败")?;

        // 按分类分组
        let mut result: std::collections::HashMap<String, Vec<(String, i32)>> = std::collections::HashMap::new();

        for (category_name, date, count) in usage {
            result.entry(category_name)
                .or_insert_with(Vec::new)
                .push((date, count));
        }

        Ok(result.into_iter().collect())
    }

    /// 批量更新密码分类
    pub async fn batch_update_passwords_category(&self, password_ids: Vec<String>, new_category_id: &str) -> Result<u64> {
        let db = get_connection().await?;
        let pool = db.pool();

        // 检查新分类是否存在
        let new_category = self.get_category_by_id(new_category_id).await?;
        if new_category.is_none() {
            return Err(anyhow::anyhow!("目标分类不存在: {}", new_category_id));
        }

        let new_category_name = new_category.unwrap().name;
        let now = Utc::now();

        // 开始事务
        let mut transaction = pool.begin().await.context("开始事务失败")?;

        let mut updated_count = 0;

        for password_id in password_ids {
            let result = sqlx::query(
                r#"
                UPDATE password_entries
                SET category = ?, updated_at = ?
                WHERE id = ? AND deleted = FALSE
                "#
            )
            .bind(&new_category_name)
            .bind(now)
            .bind(&password_id)
            .execute(&mut *transaction)
            .await
            .context("批量更新密码分类失败")?;

            updated_count += result.rows_affected();
        }

        transaction.commit().await.context("提交事务失败")?;

        Ok(updated_count)
    }

    /// 导出分类配置
    pub async fn export_categories(&self) -> Result<Vec<PasswordCategory>> {
        self.get_all_categories().await
    }

    /// 导入分类配置
    pub async fn import_categories(&self, categories: Vec<PasswordCategory>) -> Result<u64> {
        let db = get_connection().await?;
        let pool = db.pool();

        let mut imported_count = 0;
        let now = Utc::now();

        // 开始事务
        let mut transaction = pool.begin().await.context("开始事务失败")?;

        for category in categories {
            // 检查是否已存在
            let existing: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM password_categories WHERE id = ? OR name = ?"
            )
            .bind(&category.id)
            .bind(&category.name)
            .fetch_optional(&mut *transaction)
            .await
            .context("检查分类是否存在失败")?;

            if existing.is_some() {
                // 更新现有分类
                let result = sqlx::query(
                    r#"
                    UPDATE password_categories SET
                        name = ?, display_name = ?, description = ?, icon = ?, color = ?,
                        sort_order = ?, updated_at = ?
                    WHERE id = ?
                    "#
                )
                .bind(&category.name)
                .bind(&category.display_name)
                .bind(&category.description)
                .bind(&category.icon)
                .bind(&category.color)
                .bind(category.sort_order)
                .bind(now)
                .bind(&category.id)
                .execute(&mut *transaction)
                .await
                .context("更新导入分类失败")?;

                imported_count += result.rows_affected();
            } else {
                // 插入新分类
                let result = sqlx::query(
                    r#"
                    INSERT INTO password_categories
                    (id, name, display_name, description, icon, color, sort_order, created_at, updated_at)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#
                )
                .bind(&category.id)
                .bind(&category.name)
                .bind(&category.display_name)
                .bind(&category.description)
                .bind(&category.icon)
                .bind(&category.color)
                .bind(category.sort_order)
                .bind(now)
                .bind(now)
                .execute(&mut *transaction)
                .await
                .context("插入导入分类失败")?;

                imported_count += result.rows_affected();
            }
        }

        transaction.commit().await.context("提交事务失败")?;

        Ok(imported_count)
    }
}

impl Default for PasswordCategoryService {
    fn default() -> Self {
        Self::new()
    }
}