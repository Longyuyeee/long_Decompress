use anyhow::{Context, Result};
use sqlx::{SqlitePool, query, query_as};
use chrono::{DateTime, Utc};
use crate::database::models::*;

pub struct CompressionTaskRepository {
    pool: SqlitePool,
}

impl CompressionTaskRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 创建压缩任务
    pub async fn create(&self, task: &CompressionTaskDb) -> Result<()> {
        query(
            r#"
            INSERT INTO compression_tasks (
                id, source_files, output_path, format, options, status, progress,
                created_at, started_at, completed_at, error_message, total_size, processed_size
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
            .bind(&task.id)
            .bind(&task.source_files)
            .bind(&task.output_path)
            .bind(&task.format)
            .bind(&task.options)
            .bind(&task.status)
            .bind(task.progress)
            .bind(task.created_at)
            .bind(task.started_at)
            .bind(task.completed_at)
            .bind(&task.error_message)
            .bind(task.total_size)
            .bind(task.processed_size)
            .execute(&self.pool)
            .await
            .context("创建压缩任务失败")?;

        Ok(())
    }

    /// 获取压缩任务
    pub async fn get(&self, id: &str) -> Result<Option<CompressionTaskDb>> {
        let task = query_as::<_, CompressionTaskDb>(
            "SELECT * FROM compression_tasks WHERE id = ?"
        )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .context("获取压缩任务失败")?;

        Ok(task)
    }

    /// 更新压缩任务
    pub async fn update(&self, task: &CompressionTaskDb) -> Result<()> {
        query(
            r#"
            UPDATE compression_tasks SET
                source_files = ?, output_path = ?, format = ?, options = ?, status = ?,
                progress = ?, started_at = ?, completed_at = ?, error_message = ?,
                total_size = ?, processed_size = ?
            WHERE id = ?
            "#
        )
            .bind(&task.source_files)
            .bind(&task.output_path)
            .bind(&task.format)
            .bind(&task.options)
            .bind(&task.status)
            .bind(task.progress)
            .bind(task.started_at)
            .bind(task.completed_at)
            .bind(&task.error_message)
            .bind(task.total_size)
            .bind(task.processed_size)
            .bind(&task.id)
            .execute(&self.pool)
            .await
            .context("更新压缩任务失败")?;

        Ok(())
    }

    /// 删除压缩任务
    pub async fn delete(&self, id: &str) -> Result<()> {
        query("DELETE FROM compression_tasks WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("删除压缩任务失败")?;

        Ok(())
    }

    /// 获取所有压缩任务
    pub async fn list(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<CompressionTaskDb>> {
        let mut query_str = "SELECT * FROM compression_tasks ORDER BY created_at DESC".to_string();

        if let Some(limit) = limit {
            query_str.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = offset {
            query_str.push_str(&format!(" OFFSET {}", offset));
        }

        let tasks = query_as::<_, CompressionTaskDb>(&query_str)
            .fetch_all(&self.pool)
            .await
            .context("获取压缩任务列表失败")?;

        Ok(tasks)
    }

    /// 根据状态获取压缩任务
    pub async fn list_by_status(&self, status: &str) -> Result<Vec<CompressionTaskDb>> {
        let tasks = query_as::<_, CompressionTaskDb>(
            "SELECT * FROM compression_tasks WHERE status = ? ORDER BY created_at DESC"
        )
            .bind(status)
            .fetch_all(&self.pool)
            .await
            .context("根据状态获取压缩任务失败")?;

        Ok(tasks)
    }

    /// 获取进行中的压缩任务
    pub async fn get_active_tasks(&self) -> Result<Vec<CompressionTaskDb>> {
        let tasks = query_as::<_, CompressionTaskDb>(
            "SELECT * FROM compression_tasks WHERE status IN ('Preparing', 'Compressing', 'Extracting', 'Finalizing') ORDER BY created_at DESC"
        )
            .fetch_all(&self.pool)
            .await
            .context("获取进行中任务失败")?;

        Ok(tasks)
    }
}

pub struct CompressionHistoryRepository {
    pool: SqlitePool,
}

impl CompressionHistoryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 创建压缩历史记录
    pub async fn create(&self, history: &CompressionHistoryDb) -> Result<()> {
        query(
            r#"
            INSERT INTO compression_history (
                id, task_id, operation_type, source_paths, output_path, format,
                size_before, size_after, compression_ratio, duration_seconds,
                created_at, success, error_message
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
            .bind(&history.id)
            .bind(&history.task_id)
            .bind(&history.operation_type)
            .bind(&history.source_paths)
            .bind(&history.output_path)
            .bind(&history.format)
            .bind(history.size_before)
            .bind(history.size_after)
            .bind(history.compression_ratio)
            .bind(history.duration_seconds)
            .bind(history.created_at)
            .bind(history.success)
            .bind(&history.error_message)
            .execute(&self.pool)
            .await
            .context("创建压缩历史记录失败")?;

        Ok(())
    }

    /// 获取压缩历史记录
    pub async fn list(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
        success_only: Option<bool>,
    ) -> Result<Vec<CompressionHistoryDb>> {
        let mut query_str = "SELECT * FROM compression_history".to_string();
        let mut conditions = Vec::new();

        if let Some(success_only) = success_only {
            conditions.push(format!("success = {}", success_only));
        }

        if !conditions.is_empty() {
            query_str.push_str(" WHERE ");
            query_str.push_str(&conditions.join(" AND "));
        }

        query_str.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = limit {
            query_str.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = offset {
            query_str.push_str(&format!(" OFFSET {}", offset));
        }

        let history = query_as::<_, CompressionHistoryDb>(&query_str)
            .fetch_all(&self.pool)
            .await
            .context("获取压缩历史记录失败")?;

        Ok(history)
    }

    /// 获取压缩统计信息
    pub async fn get_statistics(&self) -> Result<CompressionStatistics> {
        let total_count: (i64,) = query_as(
            "SELECT COUNT(*) FROM compression_history"
        )
            .fetch_one(&self.pool)
            .await
            .context("获取总操作数失败")?;

        let success_count: (i64,) = query_as(
            "SELECT COUNT(*) FROM compression_history WHERE success = true"
        )
            .fetch_one(&self.pool)
            .await
            .context("获取成功操作数失败")?;

        let avg_ratio: (Option<f64>,) = query_as(
            "SELECT AVG(compression_ratio) FROM compression_history WHERE success = true"
        )
            .fetch_one(&self.pool)
            .await
            .context("获取平均压缩率失败")?;

        let total_compressed: (Option<i64>,) = query_as(
            "SELECT SUM(size_before) FROM compression_history WHERE operation_type = 'Compress' AND success = true"
        )
            .fetch_one(&self.pool)
            .await
            .context("获取总压缩大小失败")?;

        let total_extracted: (Option<i64>,) = query_as(
            "SELECT SUM(size_after) FROM compression_history WHERE operation_type = 'Extract' AND success = true"
        )
            .fetch_one(&self.pool)
            .await
            .context("获取总解压大小失败")?;

        let most_used_format: (Option<String>,) = query_as(
            r#"
            SELECT format FROM compression_history
            WHERE success = true
            GROUP BY format
            ORDER BY COUNT(*) DESC
            LIMIT 1
            "#
        )
            .fetch_one(&self.pool)
            .await
            .context("获取最常用格式失败")?;

        let last_operation: (Option<DateTime<Utc>>,) = query_as(
            "SELECT MAX(created_at) FROM compression_history"
        )
            .fetch_one(&self.pool)
            .await
            .context("获取最后操作时间失败")?;

        Ok(CompressionStatistics {
            total_operations: total_count.0 as u32,
            successful_operations: success_count.0 as u32,
            failed_operations: (total_count.0 - success_count.0) as u32,
            total_compressed_size: total_compressed.0.unwrap_or(0) as u64,
            total_extracted_size: total_extracted.0.unwrap_or(0) as u64,
            average_compression_ratio: avg_ratio.0.unwrap_or(0.0) as f32,
            most_used_format: most_used_format.0.unwrap_or_else(|| "zip".to_string()),
            last_operation_time: last_operation.0,
        })
    }
}

#[derive(Debug)]
pub struct CompressionStatistics {
    pub total_operations: u32,
    pub successful_operations: u32,
    pub failed_operations: u32,
    pub total_compressed_size: u64,
    pub total_extracted_size: u64,
    pub average_compression_ratio: f32,
    pub most_used_format: String,
    pub last_operation_time: Option<DateTime<Utc>>,
}

pub struct PasswordEntryRepository {
    pool: SqlitePool,
}

impl PasswordEntryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 创建密码条目
    pub async fn create(&self, entry: &PasswordEntryDb) -> Result<()> {
        query(
            r#"
            INSERT INTO password_entries (
                id, name, username, password, url, notes, tags, category,
                strength, created_at, updated_at, last_used, expires_at,
                favorite, use_count, custom_fields
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
            .bind(&entry.id)
            .bind(&entry.name)
            .bind(&entry.username)
            .bind(&entry.password)
            .bind(&entry.url)
            .bind(&entry.notes)
            .bind(&entry.tags)
            .bind(&entry.category)
            .bind(&entry.strength)
            .bind(entry.created_at)
            .bind(entry.updated_at)
            .bind(entry.last_used)
            .bind(entry.expires_at)
            .bind(entry.favorite)
            .bind(entry.use_count)
            .bind(&entry.custom_fields)
            .execute(&self.pool)
            .await
            .context("创建密码条目失败")?;

        Ok(())
    }

    /// 获取密码条目
    pub async fn get(&self, id: &str) -> Result<Option<PasswordEntryDb>> {
        let entry = query_as::<_, PasswordEntryDb>(
            "SELECT * FROM password_entries WHERE id = ?"
        )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .context("获取密码条目失败")?;

        Ok(entry)
    }

    /// 更新密码条目
    pub async fn update(&self, entry: &PasswordEntryDb) -> Result<()> {
        query(
            r#"
            UPDATE password_entries SET
                name = ?, username = ?, password = ?, url = ?, notes = ?, tags = ?,
                category = ?, strength = ?, updated_at = ?, last_used = ?,
                expires_at = ?, favorite = ?, use_count = ?, custom_fields = ?
            WHERE id = ?
            "#
        )
            .bind(&entry.name)
            .bind(&entry.username)
            .bind(&entry.password)
            .bind(&entry.url)
            .bind(&entry.notes)
            .bind(&entry.tags)
            .bind(&entry.category)
            .bind(&entry.strength)
            .bind(entry.updated_at)
            .bind(entry.last_used)
            .bind(entry.expires_at)
            .bind(entry.favorite)
            .bind(entry.use_count)
            .bind(&entry.custom_fields)
            .bind(&entry.id)
            .execute(&self.pool)
            .await
            .context("更新密码条目失败")?;

        Ok(())
    }

    /// 删除密码条目
    pub async fn delete(&self, id: &str) -> Result<()> {
        query("DELETE FROM password_entries WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("删除密码条目失败")?;

        Ok(())
    }

    /// 搜索密码条目
    pub async fn search(
        &self,
        query_str: &str,
        category: Option<&str>,
        favorite_only: Option<bool>,
    ) -> Result<Vec<PasswordEntryDb>> {
        let mut sql = "SELECT * FROM password_entries WHERE 1=1".to_string();
        let mut params: Vec<String> = Vec::new();

        if !query_str.is_empty() {
            sql.push_str(" AND (name LIKE ? OR username LIKE ? OR url LIKE ? OR notes LIKE ?)");
            let like_pattern = format!("%{}%", query_str);
            params.extend(vec![like_pattern.clone(), like_pattern.clone(), like_pattern.clone(), like_pattern]);
        }

        if let Some(category) = category {
            sql.push_str(" AND category = ?");
            params.push(category.to_string());
        }

        if let Some(favorite_only) = favorite_only {
            sql.push_str(" AND favorite = ?");
            params.push(favorite_only.to_string());
        }

        sql.push_str(" ORDER BY use_count DESC, updated_at DESC");

        let mut query_builder = query_as::<_, PasswordEntryDb>(&sql);
        for param in params {
            query_builder = query_builder.bind(param);
        }

        let entries = query_builder
            .fetch_all(&self.pool)
            .await
            .context("搜索密码条目失败")?;

        Ok(entries)
    }

    /// 获取所有密码条目
    pub async fn list(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<PasswordEntryDb>> {
        let mut query_str = "SELECT * FROM password_entries ORDER BY use_count DESC, updated_at DESC".to_string();

        if let Some(limit) = limit {
            query_str.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = offset {
            query_str.push_str(&format!(" OFFSET {}", offset));
        }

        let entries = query_as::<_, PasswordEntryDb>(&query_str)
            .fetch_all(&self.pool)
            .await
            .context("获取密码条目列表失败")?;

        Ok(entries)
    }
}

// 其他仓库的实现类似，这里省略以保持代码简洁