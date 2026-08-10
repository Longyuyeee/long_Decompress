use crate::services::compression_profile_service::CompressionProfileService;
use crate::services::task_template::{
    metadata_is_link_or_reparse_point, preview_profile_watch_folder, validate_profile_source_rules,
    TaskTemplateDraftCandidate,
};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, UNIX_EPOCH};
use tokio::sync::mpsc;
use uuid::Uuid;

const WATCH_EVENT_DEBOUNCE_MS: u64 = 900;
const MAX_WATCH_FOLDERS: i64 = 20;
const MAX_PENDING_BATCHES_PER_FOLDER: i64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchFolderStatus {
    Active,
    Paused,
    Disabled,
}

impl WatchFolderStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Disabled => "disabled",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "disabled" => Ok(Self::Disabled),
            _ => Err(anyhow!("未知监控状态: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchFolderRegistration {
    pub id: String,
    pub profile_id: String,
    pub profile_name: String,
    pub folder_path: String,
    pub status: WatchFolderStatus,
    pub pending_batch_count: u64,
    pub created_at: String,
    pub updated_at: String,
    pub last_event_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchFolderDraftBatch {
    pub id: String,
    pub watch_folder_id: String,
    pub profile_id: String,
    pub profile_name: String,
    pub root_path: String,
    pub candidates: Vec<TaskTemplateDraftCandidate>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileFingerprint {
    size: u64,
    modified_nanos: u128,
}

struct WatchRuntime {
    _watcher: RecommendedWatcher,
    seen: HashMap<String, FileFingerprint>,
}

#[derive(Clone)]
pub struct WatchFolderService {
    pool: SqlitePool,
    runtimes: Arc<StdMutex<HashMap<String, WatchRuntime>>>,
    event_tx: mpsc::UnboundedSender<String>,
}

impl WatchFolderService {
    pub fn new(pool: SqlitePool) -> Self {
        let runtimes = Arc::new(StdMutex::new(HashMap::new()));
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let worker_pool = pool.clone();
        let worker_runtimes = Arc::clone(&runtimes);
        tauri::async_runtime::spawn(async move {
            run_event_worker(worker_pool, worker_runtimes, event_rx).await;
        });
        Self {
            pool,
            runtimes,
            event_tx,
        }
    }

    pub async fn restore_active(&self) -> Result<()> {
        let ids = sqlx::query("SELECT id FROM task_template_watch_folders WHERE status = 'active'")
            .fetch_all(&self.pool)
            .await
            .context("读取需要恢复的监控目录失败")?
            .into_iter()
            .map(|row| row.get::<String, _>("id"))
            .collect::<Vec<_>>();

        for id in ids {
            if let Err(error) = self.activate(&id).await {
                log::warn!("恢复监控目录 {id} 失败，已安全暂停: {error}");
                let now = Utc::now().to_rfc3339();
                sqlx::query(
                    "UPDATE task_template_watch_folders SET status = 'paused', updated_at = ? WHERE id = ?",
                )
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn list_watch_folders(&self) -> Result<Vec<WatchFolderRegistration>> {
        let rows = sqlx::query(
            r#"
            SELECT w.id, w.profile_id, p.name AS profile_name, w.folder_path, w.status,
                   w.created_at, w.updated_at, w.last_event_at,
                   (SELECT COUNT(*) FROM task_template_watch_batches b WHERE b.watch_folder_id = w.id) AS pending_batch_count
            FROM task_template_watch_folders w
            JOIN compression_profiles p ON p.id = w.profile_id
            ORDER BY w.created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("读取监控目录失败")?;
        rows.into_iter().map(row_to_registration).collect()
    }

    pub async fn create_watch_folder(
        &self,
        profile_id: &str,
        folder_path: &str,
    ) -> Result<WatchFolderRegistration> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM task_template_watch_folders")
            .fetch_one(&self.pool)
            .await?;
        if count >= MAX_WATCH_FOLDERS {
            return Err(anyhow!("监控目录最多保存 {MAX_WATCH_FOLDERS} 个"));
        }

        let profile = load_profile(&self.pool, profile_id).await?;
        validate_profile_source_rules(&profile.auto_apply)?;
        if matches!(
            profile.auto_apply.mode,
            crate::models::compression_profile::AutoApplyMode::None
        ) {
            return Err(anyhow!("该配置组没有可用于监控的源文件规则"));
        }

        let source_metadata = fs::symlink_metadata(folder_path)
            .with_context(|| format!("无法读取要授权的目录: {folder_path}"))?;
        if !source_metadata.is_dir() || metadata_is_link_or_reparse_point(&source_metadata) {
            return Err(anyhow!(
                "监控授权目标必须是普通目录，不能是文件、符号链接或重解析点"
            ));
        }

        let canonical = fs::canonicalize(folder_path)
            .with_context(|| format!("无法读取要授权的目录: {folder_path}"))?;
        let metadata = fs::symlink_metadata(&canonical)?;
        if !metadata.is_dir() || metadata_is_link_or_reparse_point(&metadata) {
            return Err(anyhow!(
                "监控授权目标必须是普通目录，不能是文件、符号链接或重解析点"
            ));
        }
        let canonical_path = canonical.to_string_lossy().to_string();
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO task_template_watch_folders
                (id, profile_id, folder_path, status, created_at, updated_at)
            VALUES (?, ?, ?, 'paused', ?, ?)
            "#,
        )
        .bind(&id)
        .bind(profile_id)
        .bind(&canonical_path)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            if error.to_string().contains("UNIQUE constraint failed") {
                anyhow!("该配置组已经授权监控这个目录")
            } else {
                anyhow!("保存监控目录授权失败: {error}")
            }
        })?;

        if let Err(error) = self.activate(&id).await {
            sqlx::query("DELETE FROM task_template_watch_folders WHERE id = ?")
                .bind(&id)
                .execute(&self.pool)
                .await?;
            return Err(error);
        }
        self.get_watch_folder(&id).await
    }

    pub async fn set_status(
        &self,
        id: &str,
        status: WatchFolderStatus,
    ) -> Result<WatchFolderRegistration> {
        match status {
            WatchFolderStatus::Active => self.activate(id).await?,
            WatchFolderStatus::Paused | WatchFolderStatus::Disabled => {
                self.stop_runtime(id)?;
                let now = Utc::now().to_rfc3339();
                let changed = sqlx::query(
                    "UPDATE task_template_watch_folders SET status = ?, updated_at = ? WHERE id = ?",
                )
                .bind(status.as_str())
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
                if changed.rows_affected() == 0 {
                    return Err(anyhow!("监控目录不存在"));
                }
            }
        }
        self.get_watch_folder(id).await
    }

    pub async fn delete_watch_folder(&self, id: &str) -> Result<()> {
        self.stop_runtime(id)?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM task_template_watch_batches WHERE watch_folder_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let deleted = sqlx::query("DELETE FROM task_template_watch_folders WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        if deleted.rows_affected() == 0 {
            return Err(anyhow!("监控目录不存在"));
        }
        Ok(())
    }

    pub async fn list_pending_batches(&self) -> Result<Vec<WatchFolderDraftBatch>> {
        let rows = sqlx::query(
            r#"
            SELECT id, watch_folder_id, profile_id, profile_name, root_path, candidates, created_at
            FROM task_template_watch_batches
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("读取监控草稿批次失败")?;
        rows.into_iter()
            .map(|row| {
                let candidates_json: String = row.get("candidates");
                Ok(WatchFolderDraftBatch {
                    id: row.get("id"),
                    watch_folder_id: row.get("watch_folder_id"),
                    profile_id: row.get("profile_id"),
                    profile_name: row.get("profile_name"),
                    root_path: row.get("root_path"),
                    candidates: serde_json::from_str(&candidates_json)
                        .context("监控草稿批次内容损坏")?,
                    created_at: row.get("created_at"),
                })
            })
            .collect()
    }

    pub async fn has_watch_folders_for_profile(&self, profile_id: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM task_template_watch_folders WHERE profile_id = ?",
        )
        .bind(profile_id)
        .fetch_one(&self.pool)
        .await
        .context("检查配置组监控授权失败")?;
        Ok(count > 0)
    }

    pub async fn acknowledge_batch(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM task_template_watch_batches WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("确认监控草稿批次失败")?;
        Ok(())
    }

    async fn get_watch_folder(&self, id: &str) -> Result<WatchFolderRegistration> {
        let row = sqlx::query(
            r#"
            SELECT w.id, w.profile_id, p.name AS profile_name, w.folder_path, w.status,
                   w.created_at, w.updated_at, w.last_event_at,
                   (SELECT COUNT(*) FROM task_template_watch_batches b WHERE b.watch_folder_id = w.id) AS pending_batch_count
            FROM task_template_watch_folders w
            JOIN compression_profiles p ON p.id = w.profile_id
            WHERE w.id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow!("监控目录不存在"))?;
        row_to_registration(row)
    }

    async fn activate(&self, id: &str) -> Result<()> {
        if self
            .runtimes
            .lock()
            .map_err(|_| anyhow!("监控运行状态不可用"))?
            .contains_key(id)
        {
            return Ok(());
        }

        let row = sqlx::query(
            "SELECT profile_id, folder_path FROM task_template_watch_folders WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow!("监控目录不存在"))?;
        let profile_id: String = row.get("profile_id");
        let folder_path: String = row.get("folder_path");
        let profile = load_profile(&self.pool, &profile_id).await?;
        validate_profile_source_rules(&profile.auto_apply)?;

        let profile_for_baseline = profile.clone();
        let root_for_baseline = PathBuf::from(&folder_path);
        let baseline = tokio::task::spawn_blocking(move || {
            preview_profile_watch_folder(&profile_for_baseline, &root_for_baseline)
        })
        .await
        .context("监控目录基线任务异常退出")??;
        let mut seen = HashMap::new();
        let _ = deduplicate_candidates(&mut seen, &baseline.accepted);

        let watch_id = id.to_string();
        let event_tx = self.event_tx.clone();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                let Ok(event) = event else { return };
                if !matches!(event.kind, EventKind::Access(_)) {
                    let _ = event_tx.send(watch_id.clone());
                }
            })
            .context("创建文件系统监控器失败")?;
        watcher
            .watch(Path::new(&folder_path), RecursiveMode::Recursive)
            .with_context(|| format!("无法启动目录监控: {folder_path}"))?;

        self.runtimes
            .lock()
            .map_err(|_| anyhow!("监控运行状态不可用"))?
            .insert(
                id.to_string(),
                WatchRuntime {
                    _watcher: watcher,
                    seen,
                },
            );
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE task_template_watch_folders SET status = 'active', updated_at = ? WHERE id = ?",
        )
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        // Close the baseline-to-watcher handoff window with one bounded rescan.
        // Baseline files remain deduplicated, while a concurrent new file cannot
        // be silently missed just because no later filesystem event arrives.
        let _ = self.event_tx.send(id.to_string());
        Ok(())
    }

    fn stop_runtime(&self, id: &str) -> Result<()> {
        self.runtimes
            .lock()
            .map_err(|_| anyhow!("监控运行状态不可用"))?
            .remove(id);
        Ok(())
    }
}

fn row_to_registration(row: sqlx::sqlite::SqliteRow) -> Result<WatchFolderRegistration> {
    let status: String = row.get("status");
    let pending: i64 = row.get("pending_batch_count");
    Ok(WatchFolderRegistration {
        id: row.get("id"),
        profile_id: row.get("profile_id"),
        profile_name: row.get("profile_name"),
        folder_path: row.get("folder_path"),
        status: WatchFolderStatus::parse(&status)?,
        pending_batch_count: pending.max(0) as u64,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        last_event_at: row.get("last_event_at"),
    })
}

async fn load_profile(
    pool: &SqlitePool,
    profile_id: &str,
) -> Result<crate::models::compression_profile::CompressionProfile> {
    CompressionProfileService::new(pool.clone())
        .get_profile_by_id(profile_id)
        .await?
        .ok_or_else(|| anyhow!("监控使用的配置组不存在"))
}

fn normalized_path(path: &str) -> String {
    let normalized = Path::new(path)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(path))
        .to_string_lossy()
        .replace('/', "\\");
    #[cfg(windows)]
    {
        normalized.to_lowercase()
    }
    #[cfg(not(windows))]
    {
        normalized
    }
}

fn fingerprint(candidate: &TaskTemplateDraftCandidate) -> Option<(String, FileFingerprint)> {
    let metadata = fs::symlink_metadata(&candidate.path).ok()?;
    if !metadata.is_file() || metadata_is_link_or_reparse_point(&metadata) {
        return None;
    }
    let modified_nanos = metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos();
    Some((
        normalized_path(&candidate.path),
        FileFingerprint {
            size: metadata.len(),
            modified_nanos,
        },
    ))
}

fn deduplicate_candidates(
    seen: &mut HashMap<String, FileFingerprint>,
    candidates: &[TaskTemplateDraftCandidate],
) -> Vec<TaskTemplateDraftCandidate> {
    let mut current = HashSet::new();
    let mut fresh = Vec::new();
    for candidate in candidates {
        let Some((key, identity)) = fingerprint(candidate) else {
            continue;
        };
        current.insert(key.clone());
        if seen.get(&key) != Some(&identity) {
            seen.insert(key, identity);
            fresh.push(candidate.clone());
        }
    }
    seen.retain(|path, _| current.contains(path));
    fresh
}

async fn run_event_worker(
    pool: SqlitePool,
    runtimes: Arc<StdMutex<HashMap<String, WatchRuntime>>>,
    mut event_rx: mpsc::UnboundedReceiver<String>,
) {
    while let Some(first_id) = event_rx.recv().await {
        let mut watch_ids = HashSet::from([first_id]);
        let delay = tokio::time::sleep(Duration::from_millis(WATCH_EVENT_DEBOUNCE_MS));
        tokio::pin!(delay);
        loop {
            tokio::select! {
                _ = &mut delay => break,
                next = event_rx.recv() => {
                    let Some(next_id) = next else { break };
                    watch_ids.insert(next_id);
                }
            }
        }

        for watch_id in watch_ids {
            if let Err(error) = process_watch_folder_event(&pool, &runtimes, &watch_id).await {
                log::warn!("处理监控目录事件 {watch_id} 失败: {error}");
            }
        }
    }
}

async fn process_watch_folder_event(
    pool: &SqlitePool,
    runtimes: &Arc<StdMutex<HashMap<String, WatchRuntime>>>,
    watch_id: &str,
) -> Result<()> {
    let row = sqlx::query(
        r#"
        SELECT w.profile_id, w.folder_path, w.status, p.name AS profile_name
        FROM task_template_watch_folders w
        JOIN compression_profiles p ON p.id = w.profile_id
        WHERE w.id = ?
        "#,
    )
    .bind(watch_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow!("监控目录不存在"))?;
    let status: String = row.get("status");
    if status != WatchFolderStatus::Active.as_str() {
        return Ok(());
    }
    let profile_id: String = row.get("profile_id");
    let profile_name: String = row.get("profile_name");
    let folder_path: String = row.get("folder_path");
    let profile = load_profile(pool, &profile_id).await?;
    let profile_for_scan = profile.clone();
    let root_for_scan = PathBuf::from(&folder_path);
    let preview = tokio::task::spawn_blocking(move || {
        preview_profile_watch_folder(&profile_for_scan, &root_for_scan)
    })
    .await
    .context("监控目录稳定性复查任务异常退出")??;

    let (fresh, next_seen) = {
        let mut runtime_guard = runtimes.lock().map_err(|_| anyhow!("监控运行状态不可用"))?;
        let Some(runtime) = runtime_guard.get_mut(watch_id) else {
            return Ok(());
        };
        let mut proposed_seen = runtime.seen.clone();
        let fresh = deduplicate_candidates(&mut proposed_seen, &preview.accepted);
        (fresh, proposed_seen)
    };
    if fresh.is_empty() {
        if let Some(runtime) = runtimes
            .lock()
            .map_err(|_| anyhow!("监控运行状态不可用"))?
            .get_mut(watch_id)
        {
            runtime.seen = next_seen;
        }
        return Ok(());
    }

    let now = Utc::now().to_rfc3339();
    let batch_id = Uuid::new_v4().to_string();
    let candidates = serde_json::to_string(&fresh)?;
    let mut tx = pool.begin().await?;
    sqlx::query(
        r#"
        INSERT INTO task_template_watch_batches
            (id, watch_folder_id, profile_id, profile_name, root_path, candidates, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&batch_id)
    .bind(watch_id)
    .bind(&profile_id)
    .bind(&profile_name)
    .bind(&folder_path)
    .bind(candidates)
    .bind(&now)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        DELETE FROM task_template_watch_batches
        WHERE watch_folder_id = ? AND id NOT IN (
            SELECT id FROM task_template_watch_batches
            WHERE watch_folder_id = ?
            ORDER BY created_at DESC
            LIMIT ?
        )
        "#,
    )
    .bind(watch_id)
    .bind(watch_id)
    .bind(MAX_PENDING_BATCHES_PER_FOLDER)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE task_template_watch_folders SET last_event_at = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&now)
    .bind(&now)
    .bind(watch_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    // Only advance fingerprints after the durable batch commits. A transient
    // SQLite failure must leave the candidate eligible for a later retry.
    if let Some(runtime) = runtimes
        .lock()
        .map_err(|_| anyhow!("监控运行状态不可用"))?
        .get_mut(watch_id)
    {
        runtime.seen = next_seen;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migrations::init_tables;
    use crate::models::compression_profile::{
        AutoApplyMode, CompressionConfig, CompressionProfile,
    };
    use sqlx::sqlite::SqlitePoolOptions;
    use tempfile::tempdir;

    async fn setup() -> (SqlitePool, WatchFolderService, String, tempfile::TempDir) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        init_tables(&pool).await.unwrap();
        let profile_service = CompressionProfileService::new(pool.clone());
        let mut profile = CompressionProfile::new(
            "日志归档".to_string(),
            "📦".to_string(),
            String::new(),
            CompressionConfig {
                format: "7z".to_string(),
                level: 7,
                password: Some("must-not-propagate".to_string()),
                split_archive: false,
                split_size: None,
                keep_structure: true,
                delete_after: true,
                verify_after: true,
                create_solid_archive: true,
                filename_template: Some("{name}-{date}".to_string()),
                extra_params: HashMap::from([(
                    "unsafe".to_string(),
                    "must-not-propagate".to_string(),
                )]),
            },
        );
        profile.auto_apply.mode = AutoApplyMode::Pattern;
        profile.auto_apply.file_patterns = vec!["*.log".to_string()];
        profile.auto_apply.exclude_patterns = vec!["*.tmp".to_string()];
        let profile_id = profile.id.clone();
        profile_service.create_profile(profile).await.unwrap();
        let service = WatchFolderService::new(pool.clone());
        (pool, service, profile_id, tempdir().unwrap())
    }

    #[tokio::test]
    async fn watch_folder_lifecycle_persists_explicit_statuses() {
        let (_pool, service, profile_id, directory) = setup().await;
        let created = service
            .create_watch_folder(&profile_id, directory.path().to_str().unwrap())
            .await
            .unwrap();
        assert_eq!(created.status, WatchFolderStatus::Active);
        assert!(service
            .has_watch_folders_for_profile(&profile_id)
            .await
            .unwrap());
        let paused = service
            .set_status(&created.id, WatchFolderStatus::Paused)
            .await
            .unwrap();
        assert_eq!(paused.status, WatchFolderStatus::Paused);
        let active = service
            .set_status(&created.id, WatchFolderStatus::Active)
            .await
            .unwrap();
        assert_eq!(active.status, WatchFolderStatus::Active);
        let disabled = service
            .set_status(&created.id, WatchFolderStatus::Disabled)
            .await
            .unwrap();
        assert_eq!(disabled.status, WatchFolderStatus::Disabled);
        service.delete_watch_folder(&created.id).await.unwrap();
        assert!(service.list_watch_folders().await.unwrap().is_empty());
        assert!(!service
            .has_watch_folders_for_profile(&profile_id)
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn watch_authorization_rejects_non_directory_roots() {
        let (_pool, service, profile_id, directory) = setup().await;
        let file = directory.path().join("not-a-folder.log");
        fs::write(&file, "content").unwrap();
        let error = service
            .create_watch_folder(&profile_id, file.to_str().unwrap())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("必须是普通目录"));
    }

    #[tokio::test]
    async fn watcher_coalesces_changes_into_one_inert_persistent_batch() {
        let (_pool, service, profile_id, directory) = setup().await;
        let registration = service
            .create_watch_folder(&profile_id, directory.path().to_str().unwrap())
            .await
            .unwrap();
        let path = directory.path().join("fresh.log");
        fs::write(&path, "one").unwrap();
        fs::write(&path, "two-two").unwrap();

        let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
        let batches = loop {
            let batches = service.list_pending_batches().await.unwrap();
            if !batches.is_empty() {
                break batches;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "watch batch timed out"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        };
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].watch_folder_id, registration.id);
        assert_eq!(batches[0].candidates.len(), 1);
        assert_eq!(batches[0].candidates[0].name, "fresh.log");
        service.acknowledge_batch(&batches[0].id).await.unwrap();
        tokio::time::sleep(Duration::from_millis(2_000)).await;
        assert!(service.list_pending_batches().await.unwrap().is_empty());
    }

    #[test]
    fn fingerprints_suppress_duplicate_events_but_accept_real_changes() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("stable.log");
        fs::write(&path, "first").unwrap();
        let candidate = TaskTemplateDraftCandidate {
            path: path.to_string_lossy().to_string(),
            name: "stable.log".to_string(),
            size: 5,
            is_directory: false,
        };
        let mut seen = HashMap::new();
        assert_eq!(
            deduplicate_candidates(&mut seen, std::slice::from_ref(&candidate)).len(),
            1
        );
        assert!(deduplicate_candidates(&mut seen, std::slice::from_ref(&candidate)).is_empty());
        fs::write(&path, "second version").unwrap();
        assert_eq!(deduplicate_candidates(&mut seen, &[candidate]).len(), 1);
    }
}
