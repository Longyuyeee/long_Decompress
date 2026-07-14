use crate::models::decompression_profile::{
    DecompressionProfile, DecompressionConfig, AutoApplyRule, AutoApplyMode,
    PasswordAttemptStrategyConfig, ProfileStats, OutputMode, OverwritePolicy,
    create_default_profiles
};
use anyhow::{Context, Result};
use sqlx::{SqlitePool, query, query_as, Row};
use chrono::Utc;
use log;

/// 解压配置组服务
pub struct DecompressionProfileService {
    pool: SqlitePool,
}

impl DecompressionProfileService {
    /// 创建新的解压配置组服务
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 初始化数据库表
    pub async fn init_table(&self) -> Result<()> {
        query(
            r#"
            CREATE TABLE IF NOT EXISTS decompression_profiles (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                icon TEXT NOT NULL,
                description TEXT NOT NULL,

                -- DecompressionConfig
                output_mode TEXT NOT NULL,
                output_mode_path TEXT,
                create_subdirectory INTEGER NOT NULL,
                subdirectory_template TEXT,
                preserve_paths INTEGER NOT NULL,
                overwrite_policy TEXT NOT NULL,
                preserve_timestamps INTEGER NOT NULL,
                delete_after INTEGER NOT NULL,
                skip_corrupted INTEGER NOT NULL,
                extract_only_newer INTEGER NOT NULL,
                file_filter TEXT,
                extra_params TEXT NOT NULL,

                -- AutoApplyRule
                auto_apply_enabled INTEGER NOT NULL,
                auto_apply_mode TEXT NOT NULL,
                auto_apply_extension_patterns TEXT NOT NULL,
                auto_apply_size_range_min INTEGER,
                auto_apply_size_range_max INTEGER,
                auto_apply_filename_patterns TEXT NOT NULL,

                -- PasswordAttemptStrategyConfig
                password_attempt_enabled INTEGER NOT NULL,
                password_attempt_parallel INTEGER NOT NULL,
                password_attempt_parallelism INTEGER NOT NULL,
                password_attempt_strategies TEXT NOT NULL,
                password_attempt_try_known INTEGER NOT NULL,
                password_attempt_vault_strategy TEXT NOT NULL,
                password_attempt_try_wordlists INTEGER NOT NULL,
                password_attempt_wordlist_paths TEXT NOT NULL,
                password_attempt_max_attempts INTEGER,

                -- ProfileStats
                stats_use_count INTEGER NOT NULL DEFAULT 0,
                stats_success_count INTEGER NOT NULL DEFAULT 0,
                stats_failure_count INTEGER NOT NULL DEFAULT 0,
                stats_total_files_processed INTEGER NOT NULL DEFAULT 0,
                stats_total_bytes_processed INTEGER NOT NULL DEFAULT 0,
                stats_avg_extraction_time REAL,

                created_at INTEGER NOT NULL,
                last_used_at INTEGER,
                display_order INTEGER NOT NULL DEFAULT 0
            )
            "#
        )
        .execute(&self.pool)
        .await
        .context("创建 decompression_profiles 表失败")?;

        log::info!("解压配置组表初始化完成");
        Ok(())
    }

    /// 初始化默认解压配置组（如果数据库为空）
    pub async fn init_default_profiles(&self) -> Result<()> {
        let count: (i64,) = query_as("SELECT COUNT(*) FROM decompression_profiles")
            .fetch_one(&self.pool)
            .await
            .context("检查解压配置组数量失败")?;

        if count.0 == 0 {
            log::info!("数据库中无解压配置组，初始化默认配置组");
            let default_profiles = create_default_profiles();

            for (index, profile) in default_profiles.into_iter().enumerate() {
                self.create_profile_internal(profile, index as i32).await?;
            }

            log::info!("已初始化 {} 个默认解压配置组", 5);
        }

        Ok(())
    }

    /// 内部创建配置组（带 display_order）
    async fn create_profile_internal(&self, profile: DecompressionProfile, display_order: i32) -> Result<()> {
        let output_mode_str = match &profile.config.output_mode {
            OutputMode::SameAsSource => "same_as_source",
            OutputMode::FixedDirectory(_) => "fixed_directory",
            OutputMode::AutoGenerate => "auto_generate",
            OutputMode::AskUser => "ask_user",
        };

        let output_mode_path = match &profile.config.output_mode {
            OutputMode::FixedDirectory(path) => Some(path.clone()),
            _ => None,
        };

        let overwrite_policy_str = match profile.config.overwrite_policy {
            OverwritePolicy::AlwaysOverwrite => "always_overwrite",
            OverwritePolicy::AlwaysSkip => "always_skip",
            OverwritePolicy::KeepNewer => "keep_newer",
            OverwritePolicy::RenameNew => "rename_new",
            OverwritePolicy::AskUser => "ask_user",
        };

        let auto_apply_mode_str = match profile.auto_apply.mode {
            AutoApplyMode::None => "none",
            AutoApplyMode::All => "all",
            AutoApplyMode::Extension => "extension",
            AutoApplyMode::SizeRange => "size_range",
            AutoApplyMode::Filename => "filename",
        };

        query(
            r#"
            INSERT INTO decompression_profiles (
                id, name, icon, description,
                output_mode, output_mode_path, create_subdirectory, subdirectory_template,
                preserve_paths, overwrite_policy, preserve_timestamps, delete_after,
                skip_corrupted, extract_only_newer, file_filter, extra_params,
                auto_apply_enabled, auto_apply_mode, auto_apply_extension_patterns,
                auto_apply_size_range_min, auto_apply_size_range_max, auto_apply_filename_patterns,
                password_attempt_enabled, password_attempt_parallel, password_attempt_parallelism,
                password_attempt_strategies, password_attempt_try_known, password_attempt_vault_strategy,
                password_attempt_try_wordlists, password_attempt_wordlist_paths, password_attempt_max_attempts,
                stats_use_count, stats_success_count, stats_failure_count,
                stats_total_files_processed, stats_total_bytes_processed, stats_avg_extraction_time,
                created_at, last_used_at, display_order
            ) VALUES (
                ?, ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?
            )
            "#
        )
        .bind(&profile.id)
        .bind(&profile.name)
        .bind(&profile.icon)
        .bind(&profile.description)
        .bind(output_mode_str)
        .bind(output_mode_path)
        .bind(profile.config.create_subdirectory)
        .bind(profile.config.subdirectory_template)
        .bind(profile.config.preserve_paths)
        .bind(overwrite_policy_str)
        .bind(profile.config.preserve_timestamps)
        .bind(profile.config.delete_after)
        .bind(profile.config.skip_corrupted)
        .bind(profile.config.extract_only_newer)
        .bind(profile.config.file_filter)
        .bind(serde_json::to_string(&profile.config.extra_params)?)
        .bind(profile.auto_apply.enabled)
        .bind(auto_apply_mode_str)
        .bind(serde_json::to_string(&profile.auto_apply.extension_patterns)?)
        .bind(profile.auto_apply.size_range.map(|(min, _)| min as i64))
        .bind(profile.auto_apply.size_range.map(|(_, max)| max as i64))
        .bind(serde_json::to_string(&profile.auto_apply.filename_patterns)?)
        .bind(profile.password_attempt_strategy.enabled)
        .bind(profile.password_attempt_strategy.enable_parallel)
        .bind(profile.password_attempt_strategy.parallelism as i64)
        .bind(serde_json::to_string(&profile.password_attempt_strategy.strategies)?)
        .bind(profile.password_attempt_strategy.try_known_passwords)
        .bind(&profile.password_attempt_strategy.password_vault_strategy)
        .bind(profile.password_attempt_strategy.try_wordlists)
        .bind(serde_json::to_string(&profile.password_attempt_strategy.wordlist_paths)?)
        .bind(profile.password_attempt_strategy.max_attempts.map(|v| v as i64))
        .bind(profile.stats.use_count as i64)
        .bind(profile.stats.success_count as i64)
        .bind(profile.stats.failure_count as i64)
        .bind(profile.stats.total_files_processed as i64)
        .bind(profile.stats.total_bytes_processed as i64)
        .bind(profile.stats.avg_extraction_time)
        .bind(profile.created_at)
        .bind(profile.last_used_at)
        .bind(display_order)
        .execute(&self.pool)
        .await
        .context("插入解压配置组失败")?;

        Ok(())
    }

    /// 获取所有解压配置组
    pub async fn get_all_profiles(&self) -> Result<Vec<DecompressionProfile>> {
        let rows = query(
            r#"
            SELECT * FROM decompression_profiles
            ORDER BY display_order ASC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("获取解压配置组列表失败")?;

        let mut profiles = Vec::new();
        for row in rows {
            profiles.push(self.row_to_profile(&row)?);
        }

        Ok(profiles)
    }

    /// 根据 ID 获取配置组
    pub async fn get_profile_by_id(&self, id: &str) -> Result<Option<DecompressionProfile>> {
        let row = query("SELECT * FROM decompression_profiles WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .context("获取解压配置组失败")?;

        match row {
            Some(r) => Ok(Some(self.row_to_profile(&r)?)),
            None => Ok(None),
        }
    }

    /// 创建新配置组
    pub async fn create_profile(&self, profile: DecompressionProfile) -> Result<()> {
        let max_order: Option<(i32,)> = query_as("SELECT MAX(display_order) FROM decompression_profiles")
            .fetch_optional(&self.pool)
            .await?;

        let next_order = max_order.and_then(|r| Some(r.0)).unwrap_or(-1) + 1;
        self.create_profile_internal(profile, next_order).await
    }

    /// 更新配置组
    pub async fn update_profile(&self, profile: DecompressionProfile) -> Result<()> {
        let output_mode_str = match &profile.config.output_mode {
            OutputMode::SameAsSource => "same_as_source",
            OutputMode::FixedDirectory(_) => "fixed_directory",
            OutputMode::AutoGenerate => "auto_generate",
            OutputMode::AskUser => "ask_user",
        };

        let output_mode_path = match &profile.config.output_mode {
            OutputMode::FixedDirectory(path) => Some(path.clone()),
            _ => None,
        };

        let overwrite_policy_str = match profile.config.overwrite_policy {
            OverwritePolicy::AlwaysOverwrite => "always_overwrite",
            OverwritePolicy::AlwaysSkip => "always_skip",
            OverwritePolicy::KeepNewer => "keep_newer",
            OverwritePolicy::RenameNew => "rename_new",
            OverwritePolicy::AskUser => "ask_user",
        };

        let auto_apply_mode_str = match profile.auto_apply.mode {
            AutoApplyMode::None => "none",
            AutoApplyMode::All => "all",
            AutoApplyMode::Extension => "extension",
            AutoApplyMode::SizeRange => "size_range",
            AutoApplyMode::Filename => "filename",
        };

        query(
            r#"
            UPDATE decompression_profiles SET
                name = ?, icon = ?, description = ?,
                output_mode = ?, output_mode_path = ?, create_subdirectory = ?, subdirectory_template = ?,
                preserve_paths = ?, overwrite_policy = ?, preserve_timestamps = ?, delete_after = ?,
                skip_corrupted = ?, extract_only_newer = ?, file_filter = ?, extra_params = ?,
                auto_apply_enabled = ?, auto_apply_mode = ?, auto_apply_extension_patterns = ?,
                auto_apply_size_range_min = ?, auto_apply_size_range_max = ?, auto_apply_filename_patterns = ?,
                password_attempt_enabled = ?, password_attempt_parallel = ?, password_attempt_parallelism = ?,
                password_attempt_strategies = ?, password_attempt_try_known = ?, password_attempt_vault_strategy = ?,
                password_attempt_try_wordlists = ?, password_attempt_wordlist_paths = ?, password_attempt_max_attempts = ?
            WHERE id = ?
            "#
        )
        .bind(&profile.name)
        .bind(&profile.icon)
        .bind(&profile.description)
        .bind(output_mode_str)
        .bind(output_mode_path)
        .bind(profile.config.create_subdirectory)
        .bind(profile.config.subdirectory_template)
        .bind(profile.config.preserve_paths)
        .bind(overwrite_policy_str)
        .bind(profile.config.preserve_timestamps)
        .bind(profile.config.delete_after)
        .bind(profile.config.skip_corrupted)
        .bind(profile.config.extract_only_newer)
        .bind(profile.config.file_filter)
        .bind(serde_json::to_string(&profile.config.extra_params)?)
        .bind(profile.auto_apply.enabled)
        .bind(auto_apply_mode_str)
        .bind(serde_json::to_string(&profile.auto_apply.extension_patterns)?)
        .bind(profile.auto_apply.size_range.map(|(min, _)| min as i64))
        .bind(profile.auto_apply.size_range.map(|(_, max)| max as i64))
        .bind(serde_json::to_string(&profile.auto_apply.filename_patterns)?)
        .bind(profile.password_attempt_strategy.enabled)
        .bind(profile.password_attempt_strategy.enable_parallel)
        .bind(profile.password_attempt_strategy.parallelism as i64)
        .bind(serde_json::to_string(&profile.password_attempt_strategy.strategies)?)
        .bind(profile.password_attempt_strategy.try_known_passwords)
        .bind(&profile.password_attempt_strategy.password_vault_strategy)
        .bind(profile.password_attempt_strategy.try_wordlists)
        .bind(serde_json::to_string(&profile.password_attempt_strategy.wordlist_paths)?)
        .bind(profile.password_attempt_strategy.max_attempts.map(|v| v as i64))
        .bind(&profile.id)
        .execute(&self.pool)
        .await
        .context("更新解压配置组失败")?;

        Ok(())
    }

    /// 删除配置组
    pub async fn delete_profile(&self, id: &str) -> Result<()> {
        query("DELETE FROM decompression_profiles WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("删除解压配置组失败")?;

        Ok(())
    }

    /// 更新使用统计
    pub async fn update_stats(&self, id: &str, success: bool, files_processed: u64, bytes_processed: u64, extraction_time: f64) -> Result<()> {
        if success {
            query(
                r#"
                UPDATE decompression_profiles SET
                    stats_use_count = stats_use_count + 1,
                    stats_success_count = stats_success_count + 1,
                    stats_total_files_processed = stats_total_files_processed + ?,
                    stats_total_bytes_processed = stats_total_bytes_processed + ?,
                    stats_avg_extraction_time = CASE
                        WHEN stats_avg_extraction_time IS NULL THEN ?
                        ELSE (stats_avg_extraction_time * stats_success_count + ?) / (stats_success_count + 1)
                    END,
                    last_used_at = ?
                WHERE id = ?
                "#
            )
            .bind(files_processed as i64)
            .bind(bytes_processed as i64)
            .bind(extraction_time)
            .bind(extraction_time)
            .bind(Utc::now().timestamp())
            .bind(id)
            .execute(&self.pool)
            .await?;
        } else {
            query(
                r#"
                UPDATE decompression_profiles SET
                    stats_use_count = stats_use_count + 1,
                    stats_failure_count = stats_failure_count + 1,
                    last_used_at = ?
                WHERE id = ?
                "#
            )
            .bind(Utc::now().timestamp())
            .bind(id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// 将数据库行转换为 DecompressionProfile
    fn row_to_profile(&self, row: &sqlx::sqlite::SqliteRow) -> Result<DecompressionProfile> {
        let output_mode_str: String = row.try_get("output_mode")?;
        let output_mode_path: Option<String> = row.try_get("output_mode_path")?;
        let output_mode = match output_mode_str.as_str() {
            "same_as_source" => OutputMode::SameAsSource,
            "fixed_directory" => OutputMode::FixedDirectory(output_mode_path.unwrap_or_default()),
            "auto_generate" => OutputMode::AutoGenerate,
            "ask_user" => OutputMode::AskUser,
            _ => OutputMode::SameAsSource,
        };

        let overwrite_policy_str: String = row.try_get("overwrite_policy")?;
        let overwrite_policy = match overwrite_policy_str.as_str() {
            "always_overwrite" => OverwritePolicy::AlwaysOverwrite,
            "always_skip" => OverwritePolicy::AlwaysSkip,
            "keep_newer" => OverwritePolicy::KeepNewer,
            "rename_new" => OverwritePolicy::RenameNew,
            "ask_user" => OverwritePolicy::AskUser,
            _ => OverwritePolicy::KeepNewer,
        };

        let auto_apply_mode_str: String = row.try_get("auto_apply_mode")?;
        let auto_apply_mode = match auto_apply_mode_str.as_str() {
            "none" => AutoApplyMode::None,
            "all" => AutoApplyMode::All,
            "extension" => AutoApplyMode::Extension,
            "size_range" => AutoApplyMode::SizeRange,
            "filename" => AutoApplyMode::Filename,
            _ => AutoApplyMode::None,
        };

        let extra_params_str: String = row.try_get("extra_params")?;
        let extension_patterns_str: String = row.try_get("auto_apply_extension_patterns")?;
        let filename_patterns_str: String = row.try_get("auto_apply_filename_patterns")?;
        let strategies_str: String = row.try_get("password_attempt_strategies")?;
        let wordlist_paths_str: String = row.try_get("password_attempt_wordlist_paths")?;

        let size_range_min: Option<i64> = row.try_get("auto_apply_size_range_min")?;
        let size_range_max: Option<i64> = row.try_get("auto_apply_size_range_max")?;
        let size_range = match (size_range_min, size_range_max) {
            (Some(min), Some(max)) => Some((min as u64, max as u64)),
            _ => None,
        };

        Ok(DecompressionProfile {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            icon: row.try_get("icon")?,
            description: row.try_get("description")?,
            config: DecompressionConfig {
                output_mode,
                create_subdirectory: row.try_get("create_subdirectory")?,
                subdirectory_template: row.try_get("subdirectory_template")?,
                preserve_paths: row.try_get("preserve_paths")?,
                overwrite_policy,
                preserve_timestamps: row.try_get("preserve_timestamps")?,
                delete_after: row.try_get("delete_after")?,
                skip_corrupted: row.try_get("skip_corrupted")?,
                extract_only_newer: row.try_get("extract_only_newer")?,
                file_filter: row.try_get("file_filter")?,
                extra_params: serde_json::from_str(&extra_params_str).unwrap_or_default(),
            },
            auto_apply: AutoApplyRule {
                enabled: row.try_get("auto_apply_enabled")?,
                mode: auto_apply_mode,
                extension_patterns: serde_json::from_str(&extension_patterns_str).unwrap_or_default(),
                size_range,
                filename_patterns: serde_json::from_str(&filename_patterns_str).unwrap_or_default(),
            },
            password_attempt_strategy: PasswordAttemptStrategyConfig {
                enabled: row.try_get("password_attempt_enabled")?,
                enable_parallel: row.try_get("password_attempt_parallel")?,
                parallelism: row.try_get::<i64, _>("password_attempt_parallelism")? as usize,
                strategies: serde_json::from_str(&strategies_str).unwrap_or_default(),
                try_known_passwords: row.try_get("password_attempt_try_known")?,
                password_vault_strategy: row.try_get("password_attempt_vault_strategy")?,
                try_wordlists: row.try_get("password_attempt_try_wordlists")?,
                wordlist_paths: serde_json::from_str(&wordlist_paths_str).unwrap_or_default(),
                max_attempts: row.try_get::<Option<i64>, _>("password_attempt_max_attempts")?.map(|v| v as usize),
            },
            stats: ProfileStats {
                use_count: row.try_get::<i64, _>("stats_use_count")? as u32,
                success_count: row.try_get::<i64, _>("stats_success_count")? as u32,
                failure_count: row.try_get::<i64, _>("stats_failure_count")? as u32,
                total_files_processed: row.try_get::<i64, _>("stats_total_files_processed")? as u64,
                total_bytes_processed: row.try_get::<i64, _>("stats_total_bytes_processed")? as u64,
                avg_extraction_time: row.try_get("stats_avg_extraction_time")?,
            },
            created_at: row.try_get("created_at")?,
            last_used_at: row.try_get("last_used_at")?,
        })
    }
}
