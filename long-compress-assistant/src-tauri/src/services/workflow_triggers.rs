//! Directory observations and durable dispatch receipts; execution stays in workflows.
use super::task_template::metadata_is_link_or_reparse_point;
use super::{split_archive_detector::SplitArchiveDetector, workflows::Plan};
use anyhow::{ensure, Context, Result};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant, UNIX_EPOCH},
};
use tokio::sync::Mutex;
const MAX_VISITS: usize = 20000;
const MAX_CANDIDATES: usize = 1000;
pub(crate) const PENDING: &str =
    "'settling','waiting_parts','queued','needs_input','missing','missed'";
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub plan_id: String,
    pub root: String,
    pub output_directory: String,
    pub extensions: String,
    pub recursive: bool,
    pub mode: String,
    pub settle_seconds: u64,
    pub enabled: bool,
    #[serde(default)]
    pub include_existing: bool,
    #[serde(default)]
    pub note: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub source_key: String,
    pub paths: Vec<String>,
    pub revision: String,
    pub plan: Plan,
    pub output_directory: String,
    pub state: String,
    pub reason: String,
    pub created_at: String,
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_run_id: Option<String>,
    #[serde(default)]
    pub trigger: String,
    #[serde(default)]
    pub scheduled_at: Option<String>,
    #[serde(default)]
    pub explicit: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub rules: Vec<Rule>,
    pub schedules: Vec<super::workflow_schedules::Schedule>,
    pub items: Vec<Item>,
    pub paused: bool,
}
struct Watch {
    _watcher: RecommendedWatcher,
    dirty: Arc<AtomicBool>,
    last: Instant,
    signature: String,
}
#[derive(Default)]
pub(crate) struct Runtime {
    watchers: HashMap<String, Watch>,
}
pub(crate) static RUNS_CHANGED: AtomicBool = AtomicBool::new(true);
pub(crate) static RUNTIME: Lazy<Mutex<Runtime>> = Lazy::new(|| Mutex::new(Runtime::default()));
#[derive(Clone, Debug)]
struct Candidate {
    key: String,
    paths: Vec<String>,
    revision: String,
    missing: String,
    files: usize,
    copying: bool,
}
pub(crate) fn display_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
    }
}
pub(crate) fn key(path: &Path) -> String {
    display_path(path).replace('\\', "/").to_lowercase()
}

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
fn stamp() -> String {
    chrono::Utc::now().to_rfc3339()
}
pub(crate) fn terminal(state: &str) -> bool {
    matches!(
        state,
        "completed" | "failed" | "stopped" | "cancelled" | "interrupted" | "skipped"
    )
}
pub async fn init_tables(pool: &SqlitePool) -> Result<()> {
    super::workflows::init_tables(pool).await?;
    for sql in [
  "CREATE TABLE IF NOT EXISTS workflow_trigger_rules(id TEXT PRIMARY KEY,definition TEXT NOT NULL)",
  "CREATE TABLE IF NOT EXISTS workflow_trigger_seen(rule_id TEXT NOT NULL,source_key TEXT NOT NULL,revision TEXT NOT NULL,changed_at INTEGER NOT NULL,ack TEXT NOT NULL DEFAULT '',PRIMARY KEY(rule_id,source_key))",
  "CREATE TABLE IF NOT EXISTS workflow_trigger_queue(id TEXT PRIMARY KEY,rule_id TEXT NOT NULL,source_key TEXT NOT NULL,revision TEXT NOT NULL,state TEXT NOT NULL,record TEXT NOT NULL,created_at TEXT NOT NULL)",
  "CREATE INDEX IF NOT EXISTS workflow_trigger_pending ON workflow_trigger_queue(rule_id,source_key,state)",
  "CREATE TABLE IF NOT EXISTS workflow_trigger_outputs(path TEXT PRIMARY KEY)",
  "CREATE TABLE IF NOT EXISTS workflow_trigger_options(id INTEGER PRIMARY KEY,paused INTEGER NOT NULL)",
  "INSERT OR IGNORE INTO workflow_trigger_options VALUES(1,0)"
 ] {sqlx::query(sql).execute(pool).await?;}
    super::workflow_schedules::init_tables(pool).await?;
    Ok(())
}
async fn rules(pool: &SqlitePool) -> Result<Vec<Rule>> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT definition FROM workflow_trigger_rules ORDER BY id")
            .fetch_all(pool)
            .await?;
    rows.into_iter()
        .map(|r| {
            let mut rule: Rule = serde_json::from_str(&r.0)?;
            rule.root = display_path(Path::new(&rule.root));
            rule.output_directory = display_path(Path::new(&rule.output_directory));
            Ok(rule)
        })
        .collect()
}
pub(crate) async fn items(pool: &SqlitePool) -> Result<Vec<Item>> {
    let rows:Vec<(String,)>=sqlx::query_as("SELECT record FROM workflow_trigger_queue ORDER BY CASE WHEN state IN ('completed','failed','stopped','cancelled','interrupted','skipped') THEN 1 ELSE 0 END,created_at DESC,id LIMIT 701").fetch_all(pool).await?;
    rows.into_iter()
        .map(|r| {
            let mut item: Item = serde_json::from_str(&r.0)?;
            item.paths = item
                .paths
                .iter()
                .map(|p| display_path(Path::new(p)))
                .collect();
            item.output_directory = display_path(Path::new(&item.output_directory));
            Ok(item)
        })
        .collect()
}
async fn put_rule(pool: &SqlitePool, rule: &Rule) -> Result<()> {
    sqlx::query("INSERT INTO workflow_trigger_rules VALUES(?,?) ON CONFLICT(id) DO UPDATE SET definition=excluded.definition").bind(&rule.id).bind(serde_json::to_string(rule)?).execute(pool).await?;
    Ok(())
}
pub(crate) async fn put_item(pool: &SqlitePool, item: &Item) -> Result<()> {
    sqlx::query("INSERT INTO workflow_trigger_queue VALUES(?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,state=excluded.state,record=excluded.record").bind(&item.id).bind(&item.rule_id).bind(&item.source_key).bind(&item.revision).bind(&item.state).bind(serde_json::to_string(item)?).bind(&item.created_at).execute(pool).await?;
    Ok(())
}
async fn acknowledge(pool: &SqlitePool, item: &Item) -> Result<()> {
    sqlx::query(
        "UPDATE workflow_trigger_seen SET ack=? WHERE rule_id=? AND source_key=? AND revision=?",
    )
    .bind(&item.revision)
    .bind(&item.rule_id)
    .bind(&item.source_key)
    .bind(&item.revision)
    .execute(pool)
    .await?;
    Ok(())
}
pub(crate) async fn snapshot(pool: &SqlitePool) -> Result<State> {
    Ok(State {
        rules: rules(pool).await?,
        schedules: super::workflow_schedules::list(pool).await?,
        items: items(pool).await?,
        paused: sqlx::query_scalar::<_, bool>(
            "SELECT paused FROM workflow_trigger_options WHERE id=1",
        )
        .fetch_one(pool)
        .await?,
    })
}
pub(crate) async fn plan(pool: &SqlitePool, id: &str) -> Result<Plan> {
    let row: Option<(String,)> = sqlx::query_as("SELECT definition FROM workflow_plans WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(serde_json::from_str(
        &row.context("方案已删除，请重新选择已保存方案")?.0,
    )?)
}
pub(crate) fn path_dir(path: &str) -> Result<PathBuf> {
    let original = fs::symlink_metadata(path).context("目录无法访问，请重新选择")?;
    ensure!(
        !metadata_is_link_or_reparse_point(&original),
        "请选择普通目录"
    );
    let path = fs::canonicalize(path).context("目录无法访问，请重新选择")?;
    let meta = fs::symlink_metadata(&path)?;
    ensure!(
        meta.is_dir() && !metadata_is_link_or_reparse_point(&meta),
        "请选择可访问的普通目录"
    );
    Ok(path)
}
pub(crate) async fn output_excludes(pool: &SqlitePool, rules: &[Rule]) -> Result<Vec<PathBuf>> {
    let paths: Vec<(String,)> = sqlx::query_as("SELECT path FROM workflow_trigger_outputs")
        .fetch_all(pool)
        .await?;
    Ok(rules
        .iter()
        .map(|r| r.output_directory.clone())
        .chain(paths.into_iter().map(|r| r.0))
        .filter_map(|p| fs::canonicalize(p).ok())
        .collect())
}
fn excluded(path: &Path, excludes: &[PathBuf]) -> bool {
    excludes.iter().any(|base| path.starts_with(base))
}
fn extensions(value: &str) -> HashSet<String> {
    value
        .split([',', '，', ';', '；', ' ', '\n'])
        .map(|s| s.trim().trim_start_matches(['*', '.']).to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}
static NUMERIC_SUFFIX: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"(?:\.part\d+|\.\d{3,})$").unwrap());
static ZIP_SUFFIX: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"\.z\d+$").unwrap());
static RAR_SUFFIX: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"\.r\d+$").unwrap());
fn matches(path: &Path, exts: &HashSet<String>) -> bool {
    if exts.is_empty() {
        return true;
    }
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let logical = NUMERIC_SUFFIX.replace(&name, "");
    exts.iter().any(|ext| {
        name.ends_with(&format!(".{ext}"))
            || logical.ends_with(&format!(".{ext}"))
            || (ext == "zip" && ZIP_SUFFIX.is_match(&name))
            || (ext == "rar" && RAR_SUFFIX.is_match(&name))
    })
}
fn fingerprint(
    paths: &[PathBuf],
    excludes: &[PathBuf],
    visits: &mut usize,
) -> Result<(String, usize, bool)> {
    let mut pending = paths.to_vec();
    let mut entries = BTreeMap::new();
    let mut files = 0;
    let mut copying = false;
    while let Some(path) = pending.pop() {
        *visits += 1;
        ensure!(
            *visits <= MAX_VISITS,
            "本规则超过 2 万个位置，请缩小目录范围"
        );
        if excluded(&path, excludes) {
            continue;
        }
        let meta = fs::symlink_metadata(&path)
            .with_context(|| format!("来源暂时无法读取：{}", path.display()))?;
        ensure!(
            !metadata_is_link_or_reparse_point(&meta),
            "来源含链接或重解析点，请使用普通文件目录"
        );
        let modified = meta
            .modified()?
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        entries.insert(
            key(&path),
            if meta.is_dir() {
                "directory".into()
            } else {
                format!("file:{}:{}", meta.len(), modified)
            },
        );
        if meta.is_dir() {
            for e in fs::read_dir(&path)? {
                pending.push(e?.path())
            }
        } else {
            files += 1;
            #[cfg(windows)]
            {
                use std::os::windows::fs::OpenOptionsExt;
                copying |= fs::OpenOptions::new()
                    .read(true)
                    .share_mode(1)
                    .open(&path)
                    .is_err();
            }
            #[cfg(not(windows))]
            {
                copying |= fs::File::open(&path).is_err();
            }
        }
    }
    let mut hash = blake3::Hasher::new();
    for (k, v) in entries {
        hash.update(k.as_bytes());
        hash.update(&[0]);
        hash.update(v.as_bytes());
        hash.update(&[0]);
    }
    Ok((hash.finalize().to_hex().to_string(), files, copying))
}
fn candidate(path: &Path, excludes: &[PathBuf]) -> Result<Candidate> {
    candidate_with_budget(path, excludes, &mut 0)
}
fn candidate_with_budget(
    path: &Path,
    excludes: &[PathBuf],
    visits: &mut usize,
) -> Result<Candidate> {
    let mut paths = vec![path.to_path_buf()];
    let mut primary = path.to_path_buf();
    let mut missing = String::new();
    if path.is_file() {
        if let Some(group) = SplitArchiveDetector::detect_split_archive(path)? {
            primary = group.first_part.clone();
            paths = group.parts.clone();
            if !group.is_complete {
                missing = format!("等待分卷：{}", group.missing_part_names())
            }
        }
    }
    paths.sort_by_key(|p| key(p));
    paths.dedup();
    let (digest, files, copying) = fingerprint(&paths, excludes, visits)?;
    let revision = blake3::hash(format!("{digest}\0{missing}").as_bytes())
        .to_hex()
        .to_string();
    Ok(Candidate {
        key: key(&primary),
        paths: paths.iter().map(|p| display_path(p)).collect(),
        revision,
        missing,
        files,
        copying,
    })
}
fn scan(rule: &Rule, excludes: &[PathBuf]) -> Result<Vec<Candidate>> {
    let root = path_dir(&rule.root)?;
    ensure!(
        !excluded(&root, excludes),
        "来源目录处于已注册输出目录中，请调整目录，避免循环触发"
    );
    let exts = extensions(&rule.extensions);
    let mut stack = vec![root.clone()];
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    let mut visits = 0;
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            visits += 1;
            ensure!(
                visits <= MAX_VISITS,
                "本规则超过 2 万个位置，请缩小目录范围"
            );
            if excluded(&path, excludes) {
                continue;
            }
            let meta = fs::symlink_metadata(&path)?;
            if metadata_is_link_or_reparse_point(&meta) {
                continue;
            }
            if rule.mode == "folders" {
                if meta.is_dir() && dir == root {
                    let value = candidate_with_budget(&path, excludes, &mut visits)?;
                    result.push(value)
                }
            } else if meta.is_dir() {
                if rule.recursive {
                    stack.push(path)
                }
            } else if matches(&path, &exts)
                || SplitArchiveDetector::is_split_archive(&path)
                || path.extension().is_some_and(|e| {
                    let e = e.to_string_lossy();
                    e.len() >= 3 && e.as_bytes()[1..].iter().all(|b| b.is_ascii_digit())
                })
            {
                let value = candidate_with_budget(&path, excludes, &mut visits)?;
                if (matches(Path::new(&value.key), &exts)
                    || value.paths.iter().any(|p| matches(Path::new(p), &exts)))
                    && seen.insert(value.key.clone())
                {
                    result.push(value)
                }
            }
            ensure!(
                result.len() <= MAX_CANDIDATES,
                "本规则超过 1000 份来源，请缩小范围或筛选扩展名"
            );
        }
    }
    Ok(result)
}
async fn scan_async(rule: Rule, excludes: Vec<PathBuf>) -> Result<Vec<Candidate>> {
    tauri::async_runtime::spawn_blocking(move || scan(&rule, &excludes)).await?
}
async fn sync_runs(pool: &SqlitePool, restart: bool) -> Result<()> {
    if !restart && !RUNS_CHANGED.swap(false, Ordering::SeqCst) {
        return Ok(());
    }
    let result = sync_runs_changed(pool, restart).await;
    if result.is_err() {
        RUNS_CHANGED.store(true, Ordering::SeqCst)
    }
    result
}
async fn sync_runs_changed(pool: &SqlitePool, restart: bool) -> Result<()> {
    let rows:Vec<(String,)>=sqlx::query_as("SELECT record FROM workflow_trigger_queue WHERE state IN ('running','failed','stopped','interrupted','completed')").fetch_all(pool).await?;
    for row in rows {
        let mut item: Item = serde_json::from_str(&row.0)?;
        let before = serde_json::to_string(&item)?;
        if let Some(mut id) = item.run_id.clone() {
            for _ in 0..32 {
                let child:Option<String>=sqlx::query_scalar("SELECT child_id FROM workflow_run_links WHERE parent_id=? ORDER BY started_at DESC,child_id DESC LIMIT 1").bind(&id).fetch_optional(pool).await?;
                if let Some(child) = child {
                    if item.original_run_id.is_none() {
                        item.original_run_id = Some(id.clone())
                    }
                    id = child;
                } else {
                    break;
                }
            }
            if item.run_id.as_deref() == Some(id.as_str()) && item.state != "running" {
                continue;
            }
            item.run_id = Some(id.clone());
            let record: Option<String> =
                sqlx::query_scalar("SELECT record FROM workflow_runs WHERE id=?")
                    .bind(&id)
                    .fetch_optional(pool)
                    .await?;
            if let Some(record) = record {
                let run: super::workflows::Run = serde_json::from_str(&record)?;
                if terminal(&run.status) {
                    item.state = run.status;
                    item.reason = if item.state == "completed" {
                        if item.original_run_id.is_some() {
                            "已完成接续，点击查看最新来源与结果".into()
                        } else {
                            "已完成，点击查看实际结果".into()
                        }
                    } else {
                        "本次未完成，原记录和已有结果保留，可纠正后继续".into()
                    };
                } else if restart {
                    item.state = "interrupted".into();
                    item.reason = "上次执行已中断，请从原运行纠正继续".into();
                } else {
                    item.state = "running".into();
                    item.reason = if item.original_run_id.is_some() {
                        "正在纠正接续，查看本次运行的进度与结果".into()
                    } else {
                        "已进入原工作流执行，进度与结果在对应运行中查看".into()
                    };
                }
            } else if restart {
                item.state = "interrupted".into();
                item.reason = "上次在启动前中断，可明确重新加入队列".into();
            }
        }
        if before != serde_json::to_string(&item)? {
            put_item(pool, &item).await?;
        }
    }
    Ok(())
}

pub async fn initialize(pool: &SqlitePool) -> Result<State> {
    let _guard = RUNTIME.lock().await;
    sync_runs(pool, true).await?;
    super::workflow_schedules::poll_at(pool, now(), true, false).await?;
    snapshot(pool).await
}
pub async fn save_rule(pool: &SqlitePool, mut rule: Rule) -> Result<State> {
    let mut runtime = RUNTIME.lock().await;
    let all = rules(pool).await?;
    ensure!(
        all.iter().any(|r| r.id == rule.id) || all.len() < 20,
        "最多保存 20 条目录规则"
    );
    ensure!(!rule.id.is_empty() && rule.id.len() <= 100, "规则标识无效");
    rule.name = rule.name.trim().into();
    ensure!(
        !rule.name.is_empty() && rule.name.chars().count() <= 60,
        "请填写 1～60 字规则名称"
    );
    ensure!(
        rule.extensions.len() <= 512
            && matches!(rule.mode.as_str(), "files" | "folders")
            && (3..=300).contains(&rule.settle_seconds),
        "请选择有效的来源范围和 3～300 秒稳定时间"
    );
    plan(pool, &rule.plan_id).await?;
    let root = path_dir(&rule.root)?;
    let output = path_dir(&rule.output_directory)?;
    ensure!(
        !root.starts_with(&output),
        "保存位置不能等于或包含来源目录，请另选目录"
    );
    for other in all.iter().filter(|r| r.id != rule.id) {
        let other_root = path_dir(&other.root).ok();
        let other_output = fs::canonicalize(&other.output_directory).ok();
        ensure!(
            !other_output.as_ref().is_some_and(|p| root.starts_with(p))
                && !other_root.as_ref().is_some_and(|p| p.starts_with(&output)),
            "来源与其他规则的输出目录重叠，请调整后保存，避免循环触发"
        );
        ensure!(
            other.plan_id != rule.plan_id || key(Path::new(&other.root)) != key(&root),
            "这个目录已绑定同一方案，请编辑已有规则"
        );
    }
    rule.root = display_path(&root);
    rule.output_directory = display_path(&output);
    rule.note = String::new();
    let old = all.iter().find(|r| r.id == rule.id);
    let reset = old.is_none_or(|r| {
        r.root != rule.root
            || r.mode != rule.mode
            || r.extensions != rule.extensions
            || r.recursive != rule.recursive
    });
    let retired = output_excludes(pool, &all).await?;
    ensure!(
        !excluded(&root, &retired),
        "来源位于注册过的输出位置，请另选目录，避免循环触发"
    );
    if reset {
        let mut next = all
            .iter()
            .filter(|r| r.id != rule.id)
            .cloned()
            .collect::<Vec<_>>();
        next.push(rule.clone());
        let baseline = scan_async(rule.clone(), output_excludes(pool, &next).await?).await?;
        sqlx::query("DELETE FROM workflow_trigger_seen WHERE rule_id=?")
            .bind(&rule.id)
            .execute(pool)
            .await?;
        if !rule.include_existing {
            for value in baseline {
                sqlx::query("INSERT INTO workflow_trigger_seen(rule_id,source_key,revision,changed_at,ack) VALUES(?,?,?,?,?)").bind(&rule.id).bind(&value.key).bind(&value.revision).bind(now()).bind(&value.revision).execute(pool).await?;
            }
        }
    }
    sqlx::query("INSERT OR IGNORE INTO workflow_trigger_outputs VALUES(?)")
        .bind(&rule.output_directory)
        .execute(pool)
        .await?;
    put_rule(pool, &rule).await?;
    runtime.watchers.remove(&rule.id);
    snapshot(pool).await
}
pub async fn set_enabled(pool: &SqlitePool, id: &str, enabled: bool) -> Result<State> {
    let mut runtime = RUNTIME.lock().await;
    let mut rule = rules(pool)
        .await?
        .into_iter()
        .find(|r| r.id == id)
        .context("规则已删除")?;
    rule.enabled = enabled;
    rule.note.clear();
    put_rule(pool, &rule).await?;
    runtime.watchers.remove(id);
    snapshot(pool).await
}
pub async fn delete_rule(pool: &SqlitePool, id: &str) -> Result<State> {
    let mut runtime = RUNTIME.lock().await;
    sqlx::query("DELETE FROM workflow_trigger_rules WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    runtime.watchers.remove(id);
    for mut item in items(pool)
        .await?
        .into_iter()
        .filter(|i| i.rule_id == id && !terminal(&i.state) && i.state != "running")
    {
        item.state = "cancelled".into();
        item.reason = "规则已删除，待执行项已取消；原运行和文件保留".into();
        put_item(pool, &item).await?;
    }
    sqlx::query("DELETE FROM workflow_trigger_seen WHERE rule_id=?")
        .bind(id)
        .execute(pool)
        .await?;
    snapshot(pool).await
}
pub async fn pause(pool: &SqlitePool, paused: bool) -> Result<State> {
    let _guard = RUNTIME.lock().await;
    sqlx::query("UPDATE workflow_trigger_options SET paused=? WHERE id=1")
        .bind(paused)
        .execute(pool)
        .await?;
    snapshot(pool).await
}
async fn observe(
    pool: &SqlitePool,
    rule: &Rule,
    candidates: Vec<Candidate>,
    at: i64,
) -> Result<()> {
    let mut visible = HashSet::new();
    let existing = items(pool).await?;
    for value in candidates {
        visible.insert(value.key.clone());
        let row:Option<(String,i64,String)>=sqlx::query_as("SELECT revision,changed_at,ack FROM workflow_trigger_seen WHERE rule_id=? AND source_key=?").bind(&rule.id).bind(&value.key).fetch_optional(pool).await?;
        let (changed_at, ack) = match row {
            Some((old, since, ack)) if old == value.revision => (since, ack),
            Some((_, _, ack)) => (at, ack),
            None => (at, String::new()),
        };
        sqlx::query("INSERT INTO workflow_trigger_seen VALUES(?,?,?,?,?) ON CONFLICT(rule_id,source_key) DO UPDATE SET revision=excluded.revision,changed_at=excluded.changed_at").bind(&rule.id).bind(&value.key).bind(&value.revision).bind(changed_at).bind(&ack).execute(pool).await?;
        if ack == value.revision {
            continue;
        }
        let pending = existing.iter().find(|i| {
            i.rule_id == rule.id
                && i.source_key == value.key
                && !terminal(&i.state)
                && i.state != "running"
        });
        let mut item = if let Some(item) = pending {
            item.clone()
        } else {
            let count: i64 = sqlx::query_scalar(&format!(
                "SELECT COUNT(*) FROM workflow_trigger_queue WHERE state IN ({PENDING})"
            ))
            .fetch_one(pool)
            .await?;
            ensure!(count < 200, "已有 200 份等待来源，请处理或取消后再继续观察");
            Item {
                id: uuid::Uuid::new_v4().to_string(),
                rule_id: rule.id.clone(),
                rule_name: rule.name.clone(),
                source_key: value.key.clone(),
                paths: vec![],
                revision: String::new(),
                plan: plan(pool, &rule.plan_id).await?,
                output_directory: rule.output_directory.clone(),
                state: String::new(),
                reason: String::new(),
                created_at: stamp(),
                run_id: None,
                original_run_id: None,
                trigger: String::new(),
                scheduled_at: None,
                explicit: false,
            }
        };
        item.paths = value.paths;
        item.revision = value.revision;
        let stable = at.saturating_sub(changed_at) >= rule.settle_seconds as i64 * 1000;
        if !value.missing.is_empty() {
            item.state = "waiting_parts".into();
            item.reason = value.missing;
        } else if !stable || value.files == 0 || value.copying {
            item.state = "settling".into();
            item.reason = if value.copying {
                "文件仍在写入或被占用，释放后继续等待稳定".into()
            } else if value.files == 0 {
                "等待目录中出现文件".into()
            } else {
                format!("等待文件稳定 {} 秒，复制期间不会开始", rule.settle_seconds)
            };
        } else if item.plan.steps.iter().any(|step| step.password_required) {
            item.state = "needs_input".into();
            item.reason = "需要本次输出密码；填写后按队列执行，密码仅在当前会话使用".into();
        } else {
            item.state = "queued".into();
            item.reason = "来源已稳定，等待本次工作流开始".into();
        }
        if matches!(item.state.as_str(), "queued" | "needs_input")
            && existing.iter().any(|other| {
                other.id != item.id
                    && other.trigger == "schedule"
                    && !terminal(&other.state)
                    && other.plan.id == item.plan.id
                    && super::workflow_schedules::overlap(&other.paths, &item.paths)
            })
        {
            item.state = "skipped".into();
            item.reason = "相同方案与来源已在定时队列中，本次目录变化不重复派发".into();
            acknowledge(pool, &item).await?;
        }
        put_item(pool, &item).await?;
    }
    for mut item in existing.into_iter().filter(|i| {
        i.rule_id == rule.id
            && !terminal(&i.state)
            && i.state != "running"
            && !visible.contains(&i.source_key)
    }) {
        item.state = "missing".into();
        item.reason = "来源已移走或不再符合当前规则，请补回、取消或调整规则".into();
        put_item(pool, &item).await?;
        sqlx::query(
            "UPDATE workflow_trigger_seen SET revision='' WHERE rule_id=? AND source_key=?",
        )
        .bind(&rule.id)
        .bind(&item.source_key)
        .execute(pool)
        .await?;
    }
    Ok(())
}
pub async fn poll(pool: &SqlitePool, busy: bool) -> Result<State> {
    let mut runtime = RUNTIME.lock().await;
    sync_runs(pool, false).await?;
    let all = rules(pool).await?;
    let excludes = output_excludes(pool, &all).await?;
    runtime
        .watchers
        .retain(|id, _| all.iter().any(|r| r.id == *id && r.enabled));
    for mut rule in all.iter().filter(|r| r.enabled).cloned() {
        let signature = format!("{}:{}", rule.root, rule.recursive);
        let pending: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM workflow_trigger_queue WHERE rule_id=? AND state IN ({PENDING})"
        ))
        .bind(&rule.id)
        .fetch_one(pool)
        .await?;
        let scan_now = runtime.watchers.get(&rule.id).is_none_or(|watch| {
            watch.signature != signature
                || watch.dirty.swap(false, Ordering::SeqCst)
                || pending > 0
                || watch.last.elapsed() > Duration::from_secs(30)
        });
        if !scan_now {
            continue;
        }
        let outcome = match path_dir(&rule.output_directory) {
            Ok(_) => scan_async(rule.clone(), excludes.clone()).await,
            Err(e) => Err(e.context("保存位置暂不可用")),
        };
        match outcome {
            Ok(candidates) => {
                if let Err(e) = observe(pool, &rule, candidates, now()).await {
                    rule.note = format!("{e:#}")
                } else {
                    rule.note.clear()
                }
                if !runtime.watchers.contains_key(&rule.id) {
                    let dirty = Arc::new(AtomicBool::new(true));
                    let signal = dirty.clone();
                    let watcher =
                        notify::recommended_watcher(move |_: notify::Result<notify::Event>| {
                            signal.store(true, Ordering::SeqCst);
                        });
                    match watcher {
                        Ok(mut watcher) => {
                            if let Err(e) =
                                watcher.watch(Path::new(&rule.root), RecursiveMode::Recursive)
                            {
                                rule.note = format!("文件通知暂不可用，按周期观察：{e}")
                            }
                            runtime.watchers.insert(
                                rule.id.clone(),
                                Watch {
                                    _watcher: watcher,
                                    dirty,
                                    last: Instant::now(),
                                    signature,
                                },
                            );
                        }
                        Err(e) => rule.note = format!("文件通知暂不可用，按周期观察：{e}"),
                    }
                }
                if let Some(watch) = runtime.watchers.get_mut(&rule.id) {
                    watch.last = Instant::now();
                }
            }
            Err(e) => {
                rule.note = format!("暂未观察：{e:#}");
                runtime.watchers.remove(&rule.id);
            }
        }
        if rule.note != all.iter().find(|r| r.id == rule.id).unwrap().note {
            put_rule(pool, &rule).await?;
        }
    }
    super::workflow_schedules::poll_at(pool, now(), false, busy).await?;
    // Keep the latest 500 visible receipts; deduplication remains in observations.
    sqlx::query("DELETE FROM workflow_trigger_queue WHERE state IN ('completed','failed','stopped','cancelled','interrupted','skipped') AND id NOT IN (SELECT id FROM workflow_trigger_queue ORDER BY created_at DESC,id LIMIT 500)").execute(pool).await?;
    snapshot(pool).await
}
pub async fn claim(pool: &SqlitePool, id: &str, run_id: &str) -> Result<Item> {
    let _guard = RUNTIME.lock().await;
    ensure!(!run_id.is_empty() && run_id.len() <= 100, "运行标识无效");
    let state = snapshot(pool).await?;
    ensure!(!state.paused, "队列已暂停");
    let mut item = state
        .items
        .into_iter()
        .find(|i| i.id == id)
        .context("排队项不存在")?;
    ensure!(
        matches!(item.state.as_str(), "queued" | "needs_input"),
        "来源尚未就绪或已经开始"
    );
    if item.trigger == "schedule" {
        let rule = state
            .schedules
            .iter()
            .find(|r| r.id == item.rule_id)
            .context("定时规则已删除")?;
        ensure!(rule.enabled || item.explicit, "定时规则已暂停");
    } else {
        let rule = state
            .rules
            .iter()
            .find(|r| r.id == item.rule_id)
            .context("规则已删除")?;
        ensure!(rule.enabled, "规则已暂停");
    }
    let active: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workflow_trigger_queue WHERE state='running'")
            .fetch_one(pool)
            .await?;
    ensure!(active == 0, "已有工作流正在执行，请等待本次完成");
    if item.trigger == "schedule" {
        if let Err(error) = super::workflow_schedules::check_item(pool, &item).await {
            item.state = "failed".into();
            item.reason = format!("暂未执行：{error:#}。处理后可重新加入队列");
            put_item(pool, &item).await?;
            return Err(error);
        }
    } else {
        let paths = item.paths.iter().map(PathBuf::from).collect::<Vec<_>>();
        let excludes = output_excludes(pool, &state.rules).await?;
        let check_path = paths.first().context("来源为空")?.clone();
        let current =
            tauri::async_runtime::spawn_blocking(move || candidate(&check_path, &excludes))
                .await??;
        ensure!(
            current.revision == item.revision && current.missing.is_empty() && !current.copying,
            "来源已变化或分卷不齐，正在重新等待稳定"
        );
        path_dir(&item.output_directory)?;
    }
    item.state = "running".into();
    item.reason = "已进入原工作流执行，进度与结果在对应运行中查看".into();
    item.run_id = Some(run_id.into());
    put_item(pool, &item).await?;
    acknowledge(pool, &item).await?;
    Ok(item)
}
pub async fn action(pool: &SqlitePool, id: &str, action: &str) -> Result<State> {
    let mut runtime = RUNTIME.lock().await;
    let mut item = items(pool)
        .await?
        .into_iter()
        .find(|i| i.id == id)
        .context("排队项不存在")?;
    match action {
        "cancel" => {
            ensure!(
                !terminal(&item.state) && item.state != "running",
                "此项已结束或正在运行，请在对应运行中停止"
            );
            item.state = "cancelled".into();
            item.reason = "已取消，来源不变时不会再次自动加入".into();
            acknowledge(pool, &item).await?;
        }
        "retry" if item.trigger == "schedule" => {
            ensure!(terminal(&item.state), "此项仍在等待或执行");
            super::workflow_schedules::enqueue_by_id(pool, &item.rule_id, now(), true, false)
                .await?;
        }
        "run_missed" if item.trigger == "schedule" => {
            ensure!(item.state == "missed", "此项不是错过的计划");
            super::workflow_schedules::prepare_missed(pool, &mut item).await?;
        }
        "skip" if item.trigger == "schedule" => {
            ensure!(item.state == "missed", "此项不是错过的计划");
            item.state = "skipped".into();
            item.reason = "已选择跳过错过的时段，后续按下一次时间执行".into();
        }
        "retry" => {
            ensure!(terminal(&item.state), "此项仍在等待或执行");
            ensure!(
                rules(pool).await?.iter().any(|r| r.id == item.rule_id),
                "规则已删除，请新建规则"
            );
            sqlx::query("UPDATE workflow_trigger_seen SET ack='',revision='' WHERE rule_id=? AND source_key=?").bind(&item.rule_id).bind(&item.source_key).execute(pool).await?;
            runtime.watchers.remove(&item.rule_id);
        }
        "dispatch_failed" => {
            ensure!(item.state == "running", "此项尚未派发");
            item.state = "interrupted".into();
            item.reason = "本次尚未开始或派发中断，可查看记录或明确重新加入队列".into();
        }
        _ => anyhow::bail!("未知队列操作"),
    }
    put_item(pool, &item).await?;
    snapshot(pool).await
}
pub async fn finish(pool: &SqlitePool) -> Result<State> {
    let _guard = RUNTIME.lock().await;
    sync_runs(pool, false).await?;
    snapshot(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn setup() -> (SqlitePool, tempfile::TempDir, Rule) {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        super::super::workflows::init_tables(&pool).await.unwrap();
        init_tables(&pool).await.unwrap();
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("in")).unwrap();
        fs::create_dir(dir.path().join("out")).unwrap();
        let definition:Plan=serde_json::from_value(serde_json::json!({"id":"plan","name":"归档","steps":[{"id":"pack","name":"打包","kind":"compress","settings":{"format":"zip","filename":"{name}","level":0}}]})).unwrap();
        super::super::workflows::save_plan(&pool, definition)
            .await
            .unwrap();
        let rule = Rule {
            id: uuid::Uuid::new_v4().to_string(),
            name: "来件".into(),
            plan_id: "plan".into(),
            root: dir.path().join("in").to_string_lossy().into(),
            output_directory: dir.path().join("out").to_string_lossy().into(),
            mode: "files".into(),
            extensions: "txt".into(),
            recursive: true,
            settle_seconds: 3,
            enabled: true,
            include_existing: true,
            note: String::new(),
        };
        (pool, dir, rule)
    }
    async fn observed(pool: &SqlitePool, rule: &Rule, time: i64) {
        observe(pool, rule, scan(rule, &[]).unwrap(), time)
            .await
            .unwrap()
    }
    #[tokio::test]
    async fn workflow1552_stability_change_dedup_cancel_and_explicit_retry() {
        let (pool, dir, rule) = setup().await;
        put_rule(&pool, &rule).await.unwrap();
        let source = dir.path().join("in/report.txt");
        fs::write(&source, b"first").unwrap();
        observed(&pool, &rule, 1000).await;
        let original = items(&pool).await.unwrap()[0].clone();
        assert_eq!(original.state, "settling");
        observed(&pool, &rule, 4500).await;
        assert_eq!(items(&pool).await.unwrap()[0].state, "queued");
        fs::write(&source, b"newer content").unwrap();
        observed(&pool, &rule, 5000).await;
        let changed = items(&pool).await.unwrap();
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].id, original.id);
        assert_eq!(changed[0].state, "settling");
        assert_ne!(changed[0].revision, original.revision);
        observed(&pool, &rule, 8500).await;
        action(&pool, &original.id, "cancel").await.unwrap();
        observed(&pool, &rule, 12000).await;
        assert_eq!(items(&pool).await.unwrap().len(), 1);
        assert_eq!(items(&pool).await.unwrap()[0].state, "cancelled");
        action(&pool, &original.id, "retry").await.unwrap();
        observed(&pool, &rule, 13000).await;
        assert_eq!(items(&pool).await.unwrap().len(), 2);
    }
    #[tokio::test]
    async fn workflow1552_baseline_defaults_to_new_and_output_is_excluded_after_rule_delete() {
        let (pool, dir, mut rule) = setup().await;
        rule.include_existing = false;
        let input = dir.path().join("in");
        let output = input.join("generated");
        fs::create_dir(&output).unwrap();
        rule.output_directory = output.to_string_lossy().into();
        fs::write(input.join("old.txt"), b"old").unwrap();
        save_rule(&pool, rule.clone()).await.unwrap();
        let all = rules(&pool).await.unwrap();
        let exclusions = output_excludes(&pool, &all).await.unwrap();
        observe(&pool, &rule, scan(&rule, &exclusions).unwrap(), 1000)
            .await
            .unwrap();
        assert!(items(&pool).await.unwrap().is_empty());
        fs::write(output.join("result.txt"), b"generated").unwrap();
        assert_eq!(scan(&rule, &exclusions).unwrap().len(), 1);
        delete_rule(&pool, &rule.id).await.unwrap();
        let retired = output_excludes(&pool, &[]).await.unwrap();
        assert_eq!(scan(&rule, &retired).unwrap().len(), 1);
        assert_eq!(fs::read(input.join("old.txt")).unwrap(), b"old");
    }
    #[test]
    fn workflow1552_folder_versions_ignore_output_changes_and_keep_structure() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("project");
        let output = source.join("results");
        fs::create_dir_all(&output).unwrap();
        fs::write(source.join("doc.txt"), b"original").unwrap();
        let before = candidate(&source, &[output.clone()]).unwrap();
        fs::write(output.join("new.zip"), b"out").unwrap();
        assert_eq!(
            before.revision,
            candidate(&source, &[output.clone()]).unwrap().revision
        );
        fs::create_dir(source.join("empty")).unwrap();
        assert_ne!(
            before.revision,
            candidate(&source, &[output]).unwrap().revision
        );
        assert!(matches(Path::new("report.ZIP.001"), &extensions("zip")));
        assert!(matches(Path::new("report.z02"), &extensions("zip")));
        assert!(!matches(Path::new("report.tmp"), &extensions("zip")));
    }
    #[tokio::test]
    async fn workflow1552_claim_checks_pause_current_version_and_single_dispatch() {
        let (pool, dir, rule) = setup().await;
        put_rule(&pool, &rule).await.unwrap();
        let file = dir.path().join("in/doc.txt");
        fs::write(&file, b"source").unwrap();
        observed(&pool, &rule, 1000).await;
        observed(&pool, &rule, 4500).await;
        let entry = items(&pool).await.unwrap()[0].clone();
        pause(&pool, true).await.unwrap();
        assert!(claim(&pool, &entry.id, "r").await.is_err());
        pause(&pool, false).await.unwrap();
        set_enabled(&pool, &rule.id, false).await.unwrap();
        assert!(claim(&pool, &entry.id, "r").await.is_err());
        set_enabled(&pool, &rule.id, true).await.unwrap();
        fs::write(&file, b"changed file").unwrap();
        assert!(claim(&pool, &entry.id, "r").await.is_err());
        observed(&pool, &rule, 6000).await;
        observed(&pool, &rule, 9500).await;
        let claimed = claim(&pool, &entry.id, "r").await.unwrap();
        assert_eq!(claimed.run_id.as_deref(), Some("r"));
        assert!(claim(&pool, &entry.id, "another").await.is_err());
        initialize(&pool).await.unwrap();
        assert_eq!(items(&pool).await.unwrap()[0].state, "interrupted");
        observed(&pool, &rule, 14000).await;
        assert_eq!(items(&pool).await.unwrap().len(), 1);
    }
    #[tokio::test]
    async fn workflow1552_missing_and_parts_wait_restore_without_dispatch() {
        let (pool, dir, rule) = setup().await;
        let file = dir.path().join("in/doc.txt");
        fs::write(&file, b"x").unwrap();
        let value = candidate(&file, &[]).unwrap();
        let mut missing = value.clone();
        missing.missing = "等待分卷：part2".into();
        observe(&pool, &rule, vec![missing], 1000).await.unwrap();
        assert_eq!(items(&pool).await.unwrap()[0].state, "waiting_parts");
        observe(&pool, &rule, vec![], 5000).await.unwrap();
        assert_eq!(items(&pool).await.unwrap()[0].state, "missing");
        observe(&pool, &rule, vec![value.clone()], 6000)
            .await
            .unwrap();
        assert_eq!(items(&pool).await.unwrap()[0].state, "settling");
        observe(&pool, &rule, vec![value], 10000).await.unwrap();
        assert_eq!(items(&pool).await.unwrap()[0].state, "queued");
    }
    #[tokio::test]
    async fn workflow1552_queue_snapshot_is_independent_and_does_not_store_credentials() {
        let (pool, dir, rule) = setup().await;
        put_rule(&pool, &rule).await.unwrap();
        fs::write(dir.path().join("in/a.txt"), b"a").unwrap();
        let mut original = plan(&pool, "plan").await.unwrap();
        original.steps[0].password_required = true;
        original.steps[0].settings["password"] = serde_json::json!("never-persist-1552");
        super::super::workflows::save_plan(&pool, original)
            .await
            .unwrap();
        observed(&pool, &rule, 1000).await;
        observed(&pool, &rule, 5000).await;
        assert_eq!(items(&pool).await.unwrap()[0].state, "needs_input");
        let mut edited = plan(&pool, "plan").await.unwrap();
        edited.name = "edited".into();
        super::super::workflows::save_plan(&pool, edited)
            .await
            .unwrap();
        let record = serde_json::to_string(&items(&pool).await.unwrap()).unwrap();
        assert!(record.contains("归档"));
        assert!(!record.contains("never-persist"));
        assert!(!record.contains("edited"));
    }
    #[tokio::test]
    async fn workflow1552_schema16_upgrade_keeps_existing_workflow_and_old_draft_tables() {
        let (pool, _dir, _rule) = setup().await;
        sqlx::query("CREATE TABLE schema_version(version INTEGER PRIMARY KEY)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO schema_version VALUES(16)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("CREATE TABLE task_template_watch_batches(id TEXT PRIMARY KEY)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO task_template_watch_batches VALUES('old-draft')")
            .execute(&pool)
            .await
            .unwrap();
        crate::database::migrations::init_tables(&pool)
            .await
            .unwrap();
        let version: i64 = sqlx::query_scalar("SELECT MAX(version) FROM schema_version")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(version, 18);
        assert_eq!(
            super::super::workflows::state(&pool)
                .await
                .unwrap()
                .plans
                .len(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT id FROM task_template_watch_batches")
                .fetch_one(&pool)
                .await
                .unwrap(),
            "old-draft"
        );
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn workflow1552_active_writer_stays_waiting_even_when_metadata_is_stable() {
        use std::io::Write;
        let (pool, dir, rule) = setup().await;
        let file = dir.path().join("in/copying.txt");
        let mut writer = fs::File::create(&file).unwrap();
        writer.write_all(b"copy still open").unwrap();
        writer.flush().unwrap();
        observed(&pool, &rule, 1000).await;
        observed(&pool, &rule, 5000).await;
        assert_eq!(items(&pool).await.unwrap()[0].state, "settling");
        assert!(items(&pool).await.unwrap()[0].reason.contains("写入"));
        drop(writer);
        observed(&pool, &rule, 6000).await;
        assert_eq!(items(&pool).await.unwrap()[0].state, "queued");
    }
    #[tokio::test]
    async fn workflow1553_shared_directory_schedule_dedup_in_both_orders() {
        let (pool, dir, rule) = setup().await;
        let source = dir.path().join("in/source.txt");
        fs::write(&source, b"shared input").unwrap();
        save_rule(&pool, rule.clone()).await.unwrap();
        let scheduled:super::super::workflow_schedules::Schedule=serde_json::from_value(serde_json::json!({"id":"schedule-shared","name":"同来源定时","planId":"plan","paths":[source.to_string_lossy()],"outputDirectory":rule.output_directory,"kind":"daily","time":"18:00","busyPolicy":"queue","enabled":true})).unwrap();
        super::super::workflow_schedules::save(&pool, scheduled)
            .await
            .unwrap();
        super::super::workflow_schedules::action(&pool, "schedule-shared", "run")
            .await
            .unwrap();
        observed(&pool, &rule, 1000).await;
        observed(&pool, &rule, 4500).await;
        let all = items(&pool).await.unwrap();
        assert_eq!(all.iter().filter(|i| i.state == "queued").count(), 1);
        assert!(all
            .iter()
            .any(|i| i.rule_id == rule.id && i.state == "skipped"));
        for item in all.iter().filter(|i| i.state == "queued") {
            action(&pool, &item.id, "cancel").await.unwrap();
        }
        fs::write(&source, b"new shared input").unwrap();
        observed(&pool, &rule, 5000).await;
        observed(&pool, &rule, 8500).await;
        super::super::workflow_schedules::action(&pool, "schedule-shared", "run")
            .await
            .unwrap();
        let all = items(&pool).await.unwrap();
        assert_eq!(all.iter().filter(|i| i.state == "queued").count(), 1);
        assert!(all
            .iter()
            .any(|i| i.trigger == "schedule" && i.state == "skipped"));
    }
}

#[cfg(test)]
mod continuation_tests {
    use super::*;
    #[tokio::test]
    async fn workflow1552_queue_follows_corrected_child_result_and_retains_original_trigger() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        init_tables(&pool).await.unwrap();
        let plan:Plan=serde_json::from_value(serde_json::json!({"id":"p","name":"原方案","steps":[{"id":"s","name":"压缩","kind":"compress"}]})).unwrap();
        let run:super::super::workflows::Run=serde_json::from_value(serde_json::json!({"id":"original","planId":"p","name":"触发","status":"failed","startedAt":"2026-10-21","directory":"out","inputs":[],"steps":[]})).unwrap();
        super::super::workflows::save_run(&pool, run.clone())
            .await
            .unwrap();
        let item = Item {
            id: "q".into(),
            rule_id: "r".into(),
            rule_name: "来件".into(),
            source_key: "source".into(),
            paths: vec!["source".into()],
            revision: "v1".into(),
            plan,
            output_directory: "out".into(),
            state: "failed".into(),
            reason: "密码错误".into(),
            created_at: stamp(),
            run_id: Some("original".into()),
            original_run_id: None,
            trigger: String::new(),
            scheduled_at: None,
            explicit: false,
        };
        put_item(&pool, &item).await.unwrap();
        let mut child = run;
        child.id = "corrected".into();
        child.parent_run_id = Some("original".into());
        child.status = "completed".into();
        super::super::workflows::save_run(&pool, child)
            .await
            .unwrap();
        sync_runs(&pool, false).await.unwrap();
        let item = items(&pool).await.unwrap().remove(0);
        assert_eq!(item.state, "completed");
        assert_eq!(item.run_id.as_deref(), Some("corrected"));
        assert_eq!(item.original_run_id.as_deref(), Some("original"));
        assert_eq!(item.paths, ["source"]);
        assert_eq!(
            super::super::workflows::state(&pool)
                .await
                .unwrap()
                .runs
                .len(),
            2
        );
    }
}

#[cfg(test)]
mod path_tests {
    use super::*;
    #[test]
    fn workflow1552_windows_path_prefix_does_not_reach_output_publication() {
        assert_eq!(
            display_path(Path::new(r"\\?\C:\资料\输出")),
            r"C:\资料\输出"
        );
        assert_eq!(
            display_path(Path::new(r"\\?\UNC\server\share\输出")),
            r"\\server\share\输出"
        );
        let normal = display_path(Path::new(r"\\?\C:\资料\输出"));
        let final_path = PathBuf::from(format!("{normal}/run/资料.zip"));
        let staged = super::super::output_publish_transaction::staged_output_path(
            &final_path,
            "trigger-test",
        )
        .unwrap();
        assert_eq!(staged.parent(), final_path.parent());
    }
}

#[cfg(test)]
mod queue_notice_tests {
    use super::*;
    #[tokio::test]
    async fn workflow1552_queue_capacity_notice_does_not_block_ready_sources() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        init_tables(&pool).await.unwrap();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in");
        let output = dir.path().join("out");
        fs::create_dir(&input).unwrap();
        fs::create_dir(&output).unwrap();
        fs::write(input.join("a.txt"), b"ready").unwrap();
        let plan:Plan=serde_json::from_value(serde_json::json!({"id":"p","name":"归档","steps":[{"id":"s","kind":"compress","name":"打包"}]})).unwrap();
        super::super::workflows::save_plan(&pool, plan)
            .await
            .unwrap();
        let rule = Rule {
            id: uuid::Uuid::new_v4().to_string(),
            name: "来件".into(),
            plan_id: "p".into(),
            root: display_path(&input),
            output_directory: display_path(&output),
            extensions: "txt".into(),
            recursive: false,
            mode: "files".into(),
            settle_seconds: 3,
            enabled: true,
            include_existing: true,
            note: "已有 200 份等待来源，请处理或取消后再继续观察".into(),
        };
        put_rule(&pool, &rule).await.unwrap();
        let candidates = scan(&rule, &[]).unwrap();
        observe(&pool, &rule, candidates.clone(), 1000)
            .await
            .unwrap();
        observe(&pool, &rule, candidates, 5000).await.unwrap();
        let id = items(&pool).await.unwrap()[0].id.clone();
        assert_eq!(
            claim(&pool, &id, "ready-run").await.unwrap().state,
            "running"
        );
    }
}

pub(crate) async fn schedule_sources(
    pool: &SqlitePool,
    paths: &[String],
) -> Result<(Vec<String>, String)> {
    // Explicit fixed inputs may intentionally be another rule's results.
    // Only directory observers exclude registered outputs to prevent event loops.
    let excludes: Vec<PathBuf> = Vec::new();
    let _ = pool;
    let paths = paths.to_vec();
    tauri::async_runtime::spawn_blocking(move || {
        let mut values = BTreeMap::new();
        for path in paths {
            let value = candidate(Path::new(&path), &excludes)?;
            ensure!(!value.copying, "来源仍在写入或被占用，请等待复制完成");
            ensure!(value.missing.is_empty(), "{}", value.missing);
            ensure!(value.files > 0, "来源中尚无可处理文件");
            values.insert(value.key.clone(), value);
        }
        let mut hash = blake3::Hasher::new();
        let mut paths = Vec::new();
        for (key, value) in values {
            hash.update(key.as_bytes());
            hash.update(value.revision.as_bytes());
            paths.extend(value.paths);
        }
        paths.sort();
        paths.dedup();
        Ok((paths, hash.finalize().to_hex().to_string()))
    })
    .await?
}
