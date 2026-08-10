use crate::models::compression_profile::{
    CompressionProfile, CompressionConfig, AutoApplyRule, AutoApplyMode,
    PasswordStrategy, ProfileStats, create_default_profiles
};
use anyhow::{Context, Result};
use sqlx::{SqlitePool, query, query_as};
use std::collections::HashMap;
use chrono::Utc;
use log;

/// 配置组服务
pub struct CompressionProfileService {
    pool: SqlitePool,
}

impl CompressionProfileService {
    /// 创建新的配置组服务
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 初始化默认配置组（如果数据库为空）
    pub async fn init_default_profiles(&self) -> Result<()> {
        // 检查是否已有配置组
        let count: (i64,) = query_as("SELECT COUNT(*) FROM compression_profiles")
            .fetch_one(&self.pool)
            .await
            .context("检查配置组数量失败")?;

        if count.0 == 0 {
            log::info!("数据库中无配置组，初始化默认配置组");
            let default_profiles = create_default_profiles();

            for (index, profile) in default_profiles.into_iter().enumerate() {
                self.create_profile_internal(profile, index as i32).await?;
            }

            log::info!("已初始化 5 个默认配置组");
        }

        Ok(())
    }

    /// 获取所有配置组（按 display_order 排序）
    pub async fn get_all_profiles(&self) -> Result<Vec<CompressionProfile>> {
        let rows = query(
            r#"
            SELECT id, name, icon, description,
                   config_format, config_level, config_password,
                   config_split_archive, config_split_size, config_keep_structure,
                   config_delete_after, config_verify_after, config_create_solid_archive,
                   config_filename_template, config_extra_params,
                   auto_apply_enabled, auto_apply_mode, auto_apply_file_patterns,
                   auto_apply_size_range_min, auto_apply_size_range_max,
                   password_strategy, password_strategy_category_id,
                   password_strategy_length, password_strategy_save_to_vault,
                   stats_use_count, stats_success_count, stats_failure_count,
                   stats_total_files_processed, stats_total_bytes_processed,
                   created_at, last_used_at
            FROM compression_profiles
            ORDER BY display_order ASC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("获取配置组列表失败")?;

        let mut profiles = Vec::new();
        for row in rows {
            let profile = self.row_to_profile(&row)?;
            profiles.push(profile);
        }

        Ok(profiles)
    }

    /// 根据 ID 获取配置组
    pub async fn get_profile_by_id(&self, id: &str) -> Result<Option<CompressionProfile>> {
        let row = query(
            r#"
            SELECT id, name, icon, description,
                   config_format, config_level, config_password,
                   config_split_archive, config_split_size, config_keep_structure,
                   config_delete_after, config_verify_after, config_create_solid_archive,
                   config_filename_template, config_extra_params,
                   auto_apply_enabled, auto_apply_mode, auto_apply_file_patterns,
                   auto_apply_size_range_min, auto_apply_size_range_max,
                   password_strategy, password_strategy_category_id,
                   password_strategy_length, password_strategy_save_to_vault,
                   stats_use_count, stats_success_count, stats_failure_count,
                   stats_total_files_processed, stats_total_bytes_processed,
                   created_at, last_used_at
            FROM compression_profiles
            WHERE id = ?
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("获取配置组失败")?;

        if let Some(row) = row {
            let profile = self.row_to_profile(&row)?;
            Ok(Some(profile))
        } else {
            Ok(None)
        }
    }

    /// 创建新配置组
    pub async fn create_profile(&self, profile: CompressionProfile) -> Result<String> {
        // 获取当前最大 display_order
        let max_order: (Option<i32>,) = query_as(
            "SELECT MAX(display_order) FROM compression_profiles"
        )
        .fetch_one(&self.pool)
        .await
        .context("获取最大排序值失败")?;

        let display_order = max_order.0.unwrap_or(-1) + 1;

        self.create_profile_internal(profile.clone(), display_order).await?;

        Ok(profile.id)
    }

    /// 内部创建配置组方法（带 display_order）
    async fn create_profile_internal(&self, profile: CompressionProfile, display_order: i32) -> Result<()> {
        let extra_params = serde_json::to_string(&profile.config.extra_params)
            .context("序列化 extra_params 失败")?;
        let file_patterns = serde_json::to_string(&profile.auto_apply.file_patterns)
            .context("序列化 file_patterns 失败")?;

        let (password_strategy_type, category_id, pwd_length, save_to_vault) =
            self.serialize_password_strategy(&profile.password_strategy);

        query(
            r#"
            INSERT INTO compression_profiles (
                id, name, icon, description,
                config_format, config_level, config_password,
                config_split_archive, config_split_size, config_keep_structure,
                config_delete_after, config_verify_after, config_create_solid_archive,
                config_filename_template, config_extra_params,
                auto_apply_enabled, auto_apply_mode, auto_apply_file_patterns,
                auto_apply_size_range_min, auto_apply_size_range_max,
                password_strategy, password_strategy_category_id,
                password_strategy_length, password_strategy_save_to_vault,
                stats_use_count, stats_success_count, stats_failure_count,
                stats_total_files_processed, stats_total_bytes_processed,
                display_order, created_at, last_used_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&profile.id)
        .bind(&profile.name)
        .bind(&profile.icon)
        .bind(&profile.description)
        .bind(&profile.config.format)
        .bind(profile.config.level as i32)
        .bind(&profile.config.password)
        .bind(profile.config.split_archive)
        .bind(profile.config.split_size.map(|s| s as i64))
        .bind(profile.config.keep_structure)
        .bind(profile.config.delete_after)
        .bind(profile.config.verify_after)
        .bind(profile.config.create_solid_archive)
        .bind(&profile.config.filename_template)
        .bind(&extra_params)
        .bind(profile.auto_apply.enabled)
        .bind(format!("{:?}", profile.auto_apply.mode).to_lowercase())
        .bind(&file_patterns)
        .bind(profile.auto_apply.size_range.map(|(min, _)| min as i64))
        .bind(profile.auto_apply.size_range.map(|(_, max)| max as i64))
        .bind(&password_strategy_type)
        .bind(&category_id)
        .bind(pwd_length.map(|l| l as i32))
        .bind(save_to_vault)
        .bind(profile.stats.use_count as i64)
        .bind(profile.stats.success_count as i64)
        .bind(profile.stats.failure_count as i64)
        .bind(profile.stats.total_files_processed as i64)
        .bind(profile.stats.total_bytes_processed as i64)
        .bind(display_order)
        .bind(profile.created_at)
        .bind(profile.last_used_at)
        .execute(&self.pool)
        .await
        .context("插入配置组失败")?;

        Ok(())
    }

    /// 更新配置组
    pub async fn update_profile(&self, id: &str, profile: CompressionProfile) -> Result<()> {
        let extra_params = serde_json::to_string(&profile.config.extra_params)
            .context("序列化 extra_params 失败")?;
        let file_patterns = serde_json::to_string(&profile.auto_apply.file_patterns)
            .context("序列化 file_patterns 失败")?;

        let (password_strategy_type, category_id, pwd_length, save_to_vault) =
            self.serialize_password_strategy(&profile.password_strategy);

        query(
            r#"
            UPDATE compression_profiles SET
                name = ?, icon = ?, description = ?,
                config_format = ?, config_level = ?, config_password = ?,
                config_split_archive = ?, config_split_size = ?, config_keep_structure = ?,
                config_delete_after = ?, config_verify_after = ?, config_create_solid_archive = ?,
                config_filename_template = ?, config_extra_params = ?,
                auto_apply_enabled = ?, auto_apply_mode = ?, auto_apply_file_patterns = ?,
                auto_apply_size_range_min = ?, auto_apply_size_range_max = ?,
                password_strategy = ?, password_strategy_category_id = ?,
                password_strategy_length = ?, password_strategy_save_to_vault = ?
            WHERE id = ?
            "#
        )
        .bind(&profile.name)
        .bind(&profile.icon)
        .bind(&profile.description)
        .bind(&profile.config.format)
        .bind(profile.config.level as i32)
        .bind(&profile.config.password)
        .bind(profile.config.split_archive)
        .bind(profile.config.split_size.map(|s| s as i64))
        .bind(profile.config.keep_structure)
        .bind(profile.config.delete_after)
        .bind(profile.config.verify_after)
        .bind(profile.config.create_solid_archive)
        .bind(&profile.config.filename_template)
        .bind(&extra_params)
        .bind(profile.auto_apply.enabled)
        .bind(format!("{:?}", profile.auto_apply.mode).to_lowercase())
        .bind(&file_patterns)
        .bind(profile.auto_apply.size_range.map(|(min, _)| min as i64))
        .bind(profile.auto_apply.size_range.map(|(_, max)| max as i64))
        .bind(&password_strategy_type)
        .bind(&category_id)
        .bind(pwd_length.map(|l| l as i32))
        .bind(save_to_vault)
        .bind(id)
        .execute(&self.pool)
        .await
        .context("更新配置组失败")?;

        Ok(())
    }

    /// 删除配置组
    pub async fn delete_profile(&self, id: &str) -> Result<()> {
        query("DELETE FROM compression_profiles WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("删除配置组失败")?;

        Ok(())
    }

    /// 重新排序配置组
    pub async fn reorder_profiles(&self, ids: Vec<String>) -> Result<()> {
        // 使用事务确保原子性
        let mut tx = self.pool.begin().await.context("开始事务失败")?;

        for (index, id) in ids.iter().enumerate() {
            query("UPDATE compression_profiles SET display_order = ? WHERE id = ?")
                .bind(index as i32)
                .bind(id)
                .execute(&mut *tx)
                .await
                .context("更新排序失败")?;
        }

        tx.commit().await.context("提交事务失败")?;

        Ok(())
    }

    /// 更新配置组使用统计
    pub async fn update_profile_stats(
        &self,
        id: &str,
        success: bool,
        files_count: u64,
        bytes_processed: u64,
    ) -> Result<()> {
        let now = Utc::now().timestamp();

        if success {
            query(
                r#"
                UPDATE compression_profiles SET
                    stats_use_count = stats_use_count + 1,
                    stats_success_count = stats_success_count + 1,
                    stats_total_files_processed = stats_total_files_processed + ?,
                    stats_total_bytes_processed = stats_total_bytes_processed + ?,
                    last_used_at = ?
                WHERE id = ?
                "#
            )
            .bind(files_count as i64)
            .bind(bytes_processed as i64)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await
            .context("更新配置组统计失败")?;
        } else {
            query(
                r#"
                UPDATE compression_profiles SET
                    stats_use_count = stats_use_count + 1,
                    stats_failure_count = stats_failure_count + 1,
                    stats_total_files_processed = stats_total_files_processed + ?,
                    stats_total_bytes_processed = stats_total_bytes_processed + ?,
                    last_used_at = ?
                WHERE id = ?
                "#
            )
            .bind(files_count as i64)
            .bind(bytes_processed as i64)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await
            .context("更新配置组统计失败")?;
        }

        Ok(())
    }

    /// 推荐配置组（根据文件名和大小）
    pub async fn suggest_profile_for_file(
        &self,
        file_path: &str,
        file_size: u64,
    ) -> Result<Option<CompressionProfile>> {
        let all_profiles = self.get_all_profiles().await?;

        // 按优先级排序：使用次数高 → 成功率高 → 最近使用
        let mut matching_profiles: Vec<&CompressionProfile> = all_profiles
            .iter()
            .filter(|p| p.matches_auto_apply(file_path, file_size))
            .collect();

        if matching_profiles.is_empty() {
            return Ok(None);
        }

        // 排序：成功率 DESC, 使用次数 DESC, 最后使用时间 DESC
        matching_profiles.sort_by(|a, b| {
            let success_rate_a = if a.stats.use_count > 0 {
                (a.stats.success_count as f64) / (a.stats.use_count as f64)
            } else {
                0.0
            };
            let success_rate_b = if b.stats.use_count > 0 {
                (b.stats.success_count as f64) / (b.stats.use_count as f64)
            } else {
                0.0
            };

            success_rate_b
                .partial_cmp(&success_rate_a)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.stats.use_count.cmp(&a.stats.use_count))
                .then_with(|| b.last_used_at.cmp(&a.last_used_at))
        });

        Ok(matching_profiles.first().map(|p| (*p).clone()))
    }

    /// 将数据库行转换为 CompressionProfile
    fn row_to_profile(&self, row: &sqlx::sqlite::SqliteRow) -> Result<CompressionProfile> {
        use sqlx::Row;

        let id: String = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let icon: String = row.try_get("icon")?;
        let description: String = row.try_get("description")?;

        // Config
        let config_format: String = row.try_get("config_format")?;
        let config_level: i32 = row.try_get("config_level")?;
        let config_password: Option<String> = row.try_get("config_password")?;
        let config_split_archive: bool = row.try_get("config_split_archive")?;
        let config_split_size: Option<i64> = row.try_get("config_split_size")?;
        let config_keep_structure: bool = row.try_get("config_keep_structure")?;
        let config_delete_after: bool = row.try_get("config_delete_after")?;
        let config_verify_after: bool = row.try_get("config_verify_after")?;
        let config_create_solid_archive: bool = row.try_get("config_create_solid_archive")?;
        let config_filename_template: Option<String> = row.try_get("config_filename_template")?;
        let config_extra_params: String = row.try_get("config_extra_params")?;

        let extra_params: HashMap<String, String> = serde_json::from_str(&config_extra_params)
            .unwrap_or_default();

        let config = CompressionConfig {
            format: config_format,
            level: config_level as u8,
            password: config_password,
            split_archive: config_split_archive,
            split_size: config_split_size.map(|s| s as u32),
            keep_structure: config_keep_structure,
            delete_after: config_delete_after,
            verify_after: config_verify_after,
            create_solid_archive: config_create_solid_archive,
            filename_template: config_filename_template,
            extra_params,
        };

        // Auto Apply
        let auto_apply_enabled: bool = row.try_get("auto_apply_enabled")?;
        let auto_apply_mode: String = row.try_get("auto_apply_mode")?;
        let auto_apply_file_patterns: String = row.try_get("auto_apply_file_patterns")?;
        let auto_apply_size_range_min: Option<i64> = row.try_get("auto_apply_size_range_min")?;
        let auto_apply_size_range_max: Option<i64> = row.try_get("auto_apply_size_range_max")?;

        let file_patterns: Vec<String> = serde_json::from_str(&auto_apply_file_patterns)
            .unwrap_or_default();

        let mode = match auto_apply_mode.as_str() {
            "all" => AutoApplyMode::All,
            "pattern" => AutoApplyMode::Pattern,
            "size_range" | "sizerange" => AutoApplyMode::SizeRange,
            _ => AutoApplyMode::None,
        };

        let size_range = if let (Some(min), Some(max)) = (auto_apply_size_range_min, auto_apply_size_range_max) {
            Some((min as u64, max as u64))
        } else {
            None
        };

        let auto_apply = AutoApplyRule {
            enabled: auto_apply_enabled,
            mode,
            file_patterns,
            size_range,
        };

        // Password Strategy
        let password_strategy_type: String = row.try_get("password_strategy")?;
        let password_strategy_category_id: Option<String> = row.try_get("password_strategy_category_id")?;
        let password_strategy_length: Option<i32> = row.try_get("password_strategy_length")?;
        let password_strategy_save_to_vault: Option<bool> = row.try_get("password_strategy_save_to_vault")?;

        let password_strategy = self.deserialize_password_strategy(
            &password_strategy_type,
            password_strategy_category_id,
            password_strategy_length,
            password_strategy_save_to_vault,
        );

        // Stats
        let stats_use_count: i64 = row.try_get("stats_use_count")?;
        let stats_success_count: i64 = row.try_get("stats_success_count")?;
        let stats_failure_count: i64 = row.try_get("stats_failure_count")?;
        let stats_total_files_processed: i64 = row.try_get("stats_total_files_processed")?;
        let stats_total_bytes_processed: i64 = row.try_get("stats_total_bytes_processed")?;

        let stats = ProfileStats {
            use_count: stats_use_count as u64,
            success_count: stats_success_count as u64,
            failure_count: stats_failure_count as u64,
            total_files_processed: stats_total_files_processed as u64,
            total_bytes_processed: stats_total_bytes_processed as u64,
        };

        let created_at: i64 = row.try_get("created_at")?;
        let last_used_at: Option<i64> = row.try_get("last_used_at")?;

        Ok(CompressionProfile {
            id,
            name,
            icon,
            description,
            config,
            auto_apply,
            password_strategy,
            stats,
            created_at,
            last_used_at,
        })
    }

    /// 序列化密码策略
    fn serialize_password_strategy(
        &self,
        strategy: &PasswordStrategy,
    ) -> (String, Option<String>, Option<u8>, Option<bool>) {
        match strategy {
            PasswordStrategy::None => ("none".to_string(), None, None, None),
            PasswordStrategy::Fixed => ("fixed".to_string(), None, None, None),
            PasswordStrategy::FromVault { category_id } => {
                ("from_vault".to_string(), category_id.clone(), None, None)
            }
            PasswordStrategy::AutoGenerate { length, save_to_vault } => {
                ("auto_generate".to_string(), None, Some(*length), Some(*save_to_vault))
            }
        }
    }

    /// 反序列化密码策略
    fn deserialize_password_strategy(
        &self,
        strategy_type: &str,
        category_id: Option<String>,
        length: Option<i32>,
        save_to_vault: Option<bool>,
    ) -> PasswordStrategy {
        match strategy_type {
            "fixed" => PasswordStrategy::Fixed,
            "from_vault" => PasswordStrategy::FromVault { category_id },
            "auto_generate" => PasswordStrategy::AutoGenerate {
                length: length.unwrap_or(16) as u8,
                save_to_vault: save_to_vault.unwrap_or(false),
            },
            _ => PasswordStrategy::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn verify_after_round_trips_through_profile_storage() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::database::migrations::init_tables(&pool).await.unwrap();
        let service = CompressionProfileService::new(pool);
        service.init_default_profiles().await.unwrap();

        let mut profiles = service.get_all_profiles().await.unwrap();
        assert_eq!(profiles.len(), 5);
        assert!(profiles.iter().all(|profile| profile.config.verify_after));

        let mut profile = profiles.remove(0);
        profile.config.verify_after = false;
        service
            .update_profile(&profile.id, profile.clone())
            .await
            .unwrap();
        let reloaded = service
            .get_profile_by_id(&profile.id)
            .await
            .unwrap()
            .unwrap();
        assert!(!reloaded.config.verify_after);
    }
}
