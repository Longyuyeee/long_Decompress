//! Local wall-clock schedules share the directory workflow queue and executor.
use super::workflow_triggers::{self as queue, Item, State};
use anyhow::{ensure, Context, Result};
use chrono::{Datelike, Local, LocalResult, NaiveDate, NaiveTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::Path;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schedule {
    pub id: String,
    pub name: String,
    pub plan_id: String,
    pub paths: Vec<String>,
    pub output_directory: String,
    pub kind: String,
    pub time: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub weekdays: Vec<u32>,
    pub busy_policy: String,
    pub enabled: bool,
    #[serde(default)]
    pub next_at: Option<String>,
    #[serde(default)]
    pub checked_at: i64,
    #[serde(default)]
    pub note: String,
}
pub async fn init_tables(pool: &SqlitePool) -> Result<()> {
    sqlx::query("CREATE TABLE IF NOT EXISTS workflow_schedule_rules(id TEXT PRIMARY KEY,definition TEXT NOT NULL)").execute(pool).await?;
    Ok(())
}
pub async fn list(pool: &SqlitePool) -> Result<Vec<Schedule>> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT definition FROM workflow_schedule_rules ORDER BY id")
            .fetch_all(pool)
            .await?;
    rows.into_iter()
        .map(|r| Ok(serde_json::from_str(&r.0)?))
        .collect()
}
async fn put(pool: &SqlitePool, rule: &Schedule) -> Result<()> {
    sqlx::query("INSERT INTO workflow_schedule_rules VALUES(?,?) ON CONFLICT(id) DO UPDATE SET definition=excluded.definition").bind(&rule.id).bind(serde_json::to_string(rule)?).execute(pool).await?;
    Ok(())
}
fn millis(value: &str) -> Result<i64> {
    Ok(chrono::DateTime::parse_from_rfc3339(value)?.timestamp_millis())
}
fn stamp(at: i64) -> String {
    Utc.timestamp_millis_opt(at).single().unwrap().to_rfc3339()
}
// Recompute after 'after', so missed days coalesce instead of silently replaying.
fn next<T: TimeZone>(rule: &Schedule, after: i64, zone: &T) -> Result<Option<String>> {
    let time = NaiveTime::parse_from_str(&rule.time, "%H:%M").context("请选择有效时间")?;
    let local = zone
        .timestamp_millis_opt(after)
        .single()
        .context("时间无效")?;
    let first = if rule.kind == "once" {
        NaiveDate::parse_from_str(&rule.date, "%Y-%m-%d").context("请选择单次日期")?
    } else {
        local.date_naive()
    };
    for offset in 0..=370 {
        if rule.kind == "once" && offset > 0 {
            break;
        }
        let date = first
            .checked_add_signed(chrono::Duration::days(offset))
            .context("日期超出范围")?;
        if rule.kind == "weekly" && !rule.weekdays.contains(&date.weekday().number_from_monday()) {
            continue;
        }
        // A DST gap has no real instant; a repeated wall time runs only once.
        let instant = match zone.from_local_datetime(&date.and_time(time)) {
            LocalResult::Single(v) => v,
            LocalResult::Ambiguous(a, b) => {
                if a.timestamp_millis() < b.timestamp_millis() {
                    a
                } else {
                    b
                }
            }
            LocalResult::None => continue,
        };
        if instant.timestamp_millis() > after {
            return Ok(Some(instant.with_timezone(&Utc).to_rfc3339()));
        }
    }
    Ok(None)
}
pub async fn save(pool: &SqlitePool, mut rule: Schedule) -> Result<State> {
    let _guard = queue::RUNTIME.lock().await;
    let all = list(pool).await?;
    ensure!(
        all.iter().any(|r| r.id == rule.id) || all.len() < 30,
        "最多保存 30 条定时规则"
    );
    ensure!(!rule.id.is_empty() && rule.id.len() <= 100, "规则标识无效");
    rule.name = rule.name.trim().into();
    ensure!(
        !rule.name.is_empty() && rule.name.chars().count() <= 60,
        "请填写 1～60 字规则名称"
    );
    ensure!(
        matches!(rule.kind.as_str(), "once" | "daily" | "weekly"),
        "请选择单次、每天或每周"
    );
    ensure!(
        matches!(rule.busy_policy.as_str(), "queue" | "skip"),
        "请选择忙碌时的处理方式"
    );
    ensure!(
        rule.kind != "weekly"
            || !rule.weekdays.is_empty() && rule.weekdays.iter().all(|d| (1..=7).contains(d)),
        "请至少选择一个星期"
    );
    ensure!(
        !rule.paths.is_empty() && rule.paths.len() <= 32,
        "请选择 1～32 份固定来源"
    );
    queue::plan(pool, &rule.plan_id).await?;
    let output = queue::path_dir(&rule.output_directory)?;
    let mut normalized = Vec::new();
    for path in &rule.paths {
        let meta = std::fs::symlink_metadata(path).context("来源无法访问，请重新选择")?;
        ensure!(
            !super::task_template::metadata_is_link_or_reparse_point(&meta),
            "请选择普通文件或文件夹"
        );
        let source = std::fs::canonicalize(path)?;
        ensure!(
            !meta.is_dir() || !output.starts_with(&source) && !source.starts_with(&output),
            "保存位置与来源文件夹不能互相包含，请另选目录"
        );
        normalized.push(queue::display_path(&source));
    }
    normalized.sort();
    normalized.dedup();
    rule.paths = normalized;
    rule.output_directory = queue::display_path(&output);
    let at = Utc::now().timestamp_millis();
    rule.next_at = next(&rule, at, &Local)?;
    ensure!(
        rule.kind != "once" || rule.next_at.is_some(),
        "单次时间已过去，请选择未来时间"
    );
    rule.checked_at = at;
    rule.note.clear();
    // Also exclude scheduled outputs from directory observers.
    sqlx::query("INSERT OR IGNORE INTO workflow_trigger_outputs VALUES(?)")
        .bind(&rule.output_directory)
        .execute(pool)
        .await?;
    put(pool, &rule).await?;
    queue::snapshot(pool).await
}
pub(crate) fn overlap(a: &[String], b: &[String]) -> bool {
    a.iter().any(|a| {
        b.iter().any(|b| {
            let a = queue::key(Path::new(a));
            let b = queue::key(Path::new(b));
            a == b || a.starts_with(&(b.clone() + "/")) || b.starts_with(&(a + "/"))
        })
    })
}
async fn enqueue(
    pool: &SqlitePool,
    rule: &Schedule,
    at: i64,
    explicit: bool,
    missed: bool,
    busy: bool,
) -> Result<Item> {
    let all = queue::items(pool).await?;
    // A pending occurrence represents the current request; never build an unbounded catch-up queue.
    if let Some(item) = all
        .iter()
        .find(|i| i.rule_id == rule.id && !queue::terminal(&i.state) && i.state != "running")
    {
        return Ok(item.clone());
    }
    let pending = all
        .iter()
        .filter(|i| !queue::terminal(&i.state) && i.state != "running")
        .count();
    ensure!(pending < 200, "已有 200 份等待来源，请处理或取消后再继续");
    let plan = queue::plan(pool, &rule.plan_id).await?;
    let mut item = Item {
        id: uuid::Uuid::new_v4().to_string(),
        rule_id: rule.id.clone(),
        rule_name: rule.name.clone(),
        source_key: format!("schedule:{}:{at}", rule.id),
        paths: rule.paths.clone(),
        revision: String::new(),
        plan,
        output_directory: rule.output_directory.clone(),
        state: "queued".into(),
        reason: "已到约定时间，按队列顺序执行".into(),
        created_at: stamp(Utc::now().timestamp_millis()),
        run_id: None,
        original_run_id: None,
        trigger: "schedule".into(),
        scheduled_at: Some(stamp(at)),
        explicit,
    };
    let duplicate = all.iter().any(|i| {
        i.plan.id == rule.plan_id && !queue::terminal(&i.state) && overlap(&i.paths, &rule.paths)
    });
    if missed {
        item.state = "missed".into();
        item.reason="此时段应用未运行、休眠或长时间挂起，尚未执行。错过的多次合并为一条，请选择现在补跑或跳过".into();
    } else if duplicate || !explicit && rule.busy_policy == "skip" && busy {
        item.state = "skipped".into();
        item.reason = if duplicate {
            "相同方案与来源已在目录或定时队列中，本次不重复派发".into()
        } else {
            "到点时同一方案仍在执行，已按规则跳过本次".into()
        };
    } else {
        prepare(pool, &mut item).await;
    }
    queue::put_item(pool, &item).await?;
    Ok(item)
}
async fn prepare(pool: &SqlitePool, item: &mut Item) {
    match queue::schedule_sources(pool, &item.paths).await {
        Ok((paths, revision)) => {
            item.paths = paths;
            item.revision = revision;
            item.state = if item.plan.steps.iter().any(|s| s.password_required) {
                "needs_input"
            } else {
                "queued"
            }
            .into();
            item.reason = "本次来源已检查，等待队列执行；使用触发时的方案和保存位置".into();
        }
        Err(e) => {
            item.state = "failed".into();
            item.reason = format!("暂未执行：{e:#}。处理后可重新加入队列");
        }
    }
}
pub(crate) async fn check_item(pool: &SqlitePool, item: &Item) -> Result<()> {
    queue::path_dir(&item.output_directory)?;
    let (paths, revision) = queue::schedule_sources(pool, &item.paths).await?;
    ensure!(
        revision == item.revision && paths == item.paths,
        "排队期间来源已变化，请重新加入队列以使用当前资料"
    );
    Ok(())
}
pub(crate) async fn prepare_missed(pool: &SqlitePool, item: &mut Item) -> Result<()> {
    ensure!(
        list(pool).await?.iter().any(|r| r.id == item.rule_id),
        "定时规则已删除"
    );
    let other = queue::items(pool).await?.into_iter().any(|i| {
        i.id != item.id
            && !queue::terminal(&i.state)
            && i.plan.id == item.plan.id
            && overlap(&i.paths, &item.paths)
    });
    ensure!(!other, "同方案与来源已在队列中，请先处理该项");
    item.explicit = true;
    prepare(pool, item).await;
    Ok(())
}
pub(crate) async fn enqueue_by_id(
    pool: &SqlitePool,
    id: &str,
    at: i64,
    explicit: bool,
    busy: bool,
) -> Result<Item> {
    let rule = list(pool)
        .await?
        .into_iter()
        .find(|r| r.id == id)
        .context("定时规则已删除")?;
    enqueue(pool, &rule, at, explicit, false, busy).await
}
pub(crate) async fn poll_at(pool: &SqlitePool, at: i64, restart: bool, _busy: bool) -> Result<()> {
    for mut rule in list(pool).await? {
        if !rule.enabled {
            continue;
        }
        let Some(due) = rule.next_at.as_deref().map(millis).transpose()? else {
            continue;
        };
        if due > at {
            continue;
        }
        let running: Vec<(String,)> =
            sqlx::query_as("SELECT record FROM workflow_runs ORDER BY started_at DESC LIMIT 100")
                .fetch_all(pool)
                .await?;
        let active: Vec<serde_json::Value> = running
            .into_iter()
            .filter_map(|r| serde_json::from_str::<serde_json::Value>(&r.0).ok())
            .filter(|r| r["status"] == "running")
            .collect();
        let missed = restart
            || at.saturating_sub(rule.checked_at) > 90_000
            || at.saturating_sub(due) > 90_000;
        let busy = active.iter().any(|r| r["planId"] == rule.plan_id)
            || queue::items(pool)
                .await?
                .iter()
                .any(|i| i.plan.id == rule.plan_id && i.state == "running");
        match enqueue(pool, &rule, due, false, missed, busy).await {
            Ok(_) => {
                rule.next_at = next(&rule, at, &Local)?;
                rule.note.clear();
            }
            Err(e) => {
                rule.note = format!("尚未执行：{e:#}");
            }
        }
        rule.checked_at = at;
        put(pool, &rule).await?;
    }
    // Persist only a lightweight heartbeat at most once per minute, needed for suspend detection.
    for mut rule in list(pool).await? {
        if rule.enabled && at.saturating_sub(rule.checked_at) >= 60_000 {
            rule.checked_at = at;
            put(pool, &rule).await?;
        }
    }
    Ok(())
}
pub async fn action(pool: &SqlitePool, id: &str, action: &str) -> Result<State> {
    let _guard = queue::RUNTIME.lock().await;
    let mut rule = list(pool)
        .await?
        .into_iter()
        .find(|r| r.id == id)
        .context("定时规则已删除")?;
    let at = Utc::now().timestamp_millis();
    match action {
        "run" => {
            let mut item = enqueue(pool, &rule, at, true, false, false).await?;
            if item.state == "missed" {
                prepare_missed(pool, &mut item).await?;
            }
            if !queue::terminal(&item.state) && item.state != "running" {
                item.explicit = true;
                queue::put_item(pool, &item).await?;
            }
        }
        "enable" => {
            rule.enabled = true;
            rule.checked_at = at;
            rule.next_at = next(&rule, at, &Local)?;
            ensure!(
                rule.kind != "once" || rule.next_at.is_some(),
                "单次时间已过去，请编辑规则选择未来时间"
            );
            put(pool, &rule).await?;
        }
        "pause" => {
            rule.enabled = false;
            put(pool, &rule).await?;
        }
        "delete" => {
            sqlx::query("DELETE FROM workflow_schedule_rules WHERE id=?")
                .bind(id)
                .execute(pool)
                .await?;
            for mut item in queue::items(pool)
                .await?
                .into_iter()
                .filter(|i| i.rule_id == id && !queue::terminal(&i.state) && i.state != "running")
            {
                item.state = "cancelled".into();
                item.reason = "定时规则已删除，未执行项已取消；原记录和文件保留".into();
                queue::put_item(pool, &item).await?;
            }
        }
        _ => anyhow::bail!("未知定时规则操作"),
    }
    queue::snapshot(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn setup() -> (SqlitePool, tempfile::TempDir, Schedule) {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        queue::init_tables(&pool).await.unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("out")).unwrap();
        std::fs::write(dir.path().join("source.txt"), b"scheduled-data").unwrap();
        let plan=serde_json::from_value(serde_json::json!({"id":"p","name":"归档","steps":[{"id":"pack","name":"打包","kind":"compress","settings":{"format":"zip","filename":"{name}","level":0}}]})).unwrap();
        super::super::workflows::save_plan(&pool, plan)
            .await
            .unwrap();
        let rule = Schedule {
            id: uuid::Uuid::new_v4().to_string(),
            name: "定时".into(),
            plan_id: "p".into(),
            paths: vec![dir.path().join("source.txt").to_string_lossy().into()],
            output_directory: dir.path().join("out").to_string_lossy().into(),
            kind: "daily".into(),
            time: "18:00".into(),
            date: "2030-01-01".into(),
            weekdays: vec![1, 3, 5],
            busy_policy: "queue".into(),
            enabled: true,
            next_at: None,
            checked_at: 0,
            note: String::new(),
        };
        (pool, dir, rule)
    }
    #[test]
    fn workflow1553_local_once_daily_weekly_and_no_replay() {
        let rule = Schedule {
            id: "r".into(),
            name: "r".into(),
            plan_id: "p".into(),
            paths: vec![],
            output_directory: "out".into(),
            kind: "daily".into(),
            time: "18:00".into(),
            date: "2026-10-21".into(),
            weekdays: vec![1, 5],
            busy_policy: "queue".into(),
            enabled: true,
            next_at: None,
            checked_at: 0,
            note: String::new(),
        };
        let zone = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
        let at = millis("2026-10-21T10:00:00Z").unwrap();
        assert_eq!(
            next(&rule, at, &zone).unwrap().unwrap(),
            "2026-10-22T10:00:00+00:00"
        );
        let mut weekly = rule.clone();
        weekly.kind = "weekly".into();
        assert_eq!(
            next(&weekly, at, &zone).unwrap().unwrap(),
            "2026-10-23T10:00:00+00:00"
        );
        weekly.kind = "once".into();
        assert!(next(&weekly, at, &zone).unwrap().is_none());
        assert_eq!(
            next(&weekly, at - 1, &zone).unwrap().unwrap(),
            "2026-10-21T10:00:00+00:00"
        );
        assert!(overlap(
            &["C:/a/source".into()],
            &["c:/a/source/file.zip".into()]
        ));
        assert!(!overlap(
            &["C:/a/source".into()],
            &["c:/a/source-copy/file.zip".into()]
        ));
    }
    #[tokio::test]
    async fn workflow1553_restart_missed_coalesces_and_explicit_skip() {
        let (pool, _dir, mut rule) = setup().await;
        let at = Utc::now().timestamp_millis();
        rule.next_at = Some(stamp(at - 3 * 86400000));
        rule.checked_at = at - 4 * 86400000;
        put(&pool, &rule).await.unwrap();
        poll_at(&pool, at, true, false).await.unwrap();
        let first = queue::items(&pool).await.unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].state, "missed");
        let mut saved = list(&pool).await.unwrap()[0].clone();
        assert!(millis(saved.next_at.as_deref().unwrap()).unwrap() > at);
        saved.next_at = Some(stamp(at + 1000));
        saved.checked_at = at;
        put(&pool, &saved).await.unwrap();
        poll_at(&pool, at + 1000, false, false).await.unwrap();
        assert_eq!(queue::items(&pool).await.unwrap().len(), 1);
        queue::action(&pool, &first[0].id, "skip").await.unwrap();
        assert_eq!(queue::items(&pool).await.unwrap()[0].state, "skipped");
    }
    #[tokio::test]
    async fn workflow1553_live_due_claim_once_and_immutable_snapshot() {
        let (pool, _dir, mut rule) = setup().await;
        let at = Utc::now().timestamp_millis();
        rule.next_at = Some(stamp(at));
        rule.checked_at = at - 2000;
        put(&pool, &rule).await.unwrap();
        poll_at(&pool, at, false, false).await.unwrap();
        poll_at(&pool, at + 2000, false, false).await.unwrap();
        let items = queue::items(&pool).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].state, "queued");
        let mut plan = queue::plan(&pool, "p").await.unwrap();
        plan.name = "新方案".into();
        super::super::workflows::save_plan(&pool, plan)
            .await
            .unwrap();
        let claimed = queue::claim(&pool, &items[0].id, "run-one").await.unwrap();
        assert_eq!(claimed.plan.name, "归档");
        assert!(queue::claim(&pool, &items[0].id, "run-two").await.is_err());
        queue::initialize(&pool).await.unwrap();
        assert_eq!(queue::items(&pool).await.unwrap()[0].state, "interrupted");
    }
    #[tokio::test]
    async fn workflow1553_paused_rule_now_and_global_queue_pause() {
        let (pool, _dir, mut rule) = setup().await;
        rule.enabled = false;
        put(&pool, &rule).await.unwrap();
        action(&pool, &rule.id, "run").await.unwrap();
        let item = queue::items(&pool).await.unwrap()[0].clone();
        assert!(item.explicit);
        queue::pause(&pool, true).await.unwrap();
        assert!(queue::claim(&pool, &item.id, "r").await.is_err());
        queue::pause(&pool, false).await.unwrap();
        assert!(queue::claim(&pool, &item.id, "r").await.is_ok());
    }
    #[tokio::test]
    async fn workflow1553_busy_skip_queue_and_cross_trigger_dedup() {
        let (pool, _dir, mut rule) = setup().await;
        rule.busy_policy = "skip".into();
        let at = Utc::now().timestamp_millis();
        let skipped = enqueue(&pool, &rule, at, false, false, true).await.unwrap();
        assert_eq!(skipped.state, "skipped");
        rule.busy_policy = "queue".into();
        let queued = enqueue(&pool, &rule, at + 1, false, false, true)
            .await
            .unwrap();
        assert_eq!(queued.state, "queued");
        let mut other = rule.clone();
        other.id = "other".into();
        let skipped = enqueue(&pool, &other, at + 2, false, false, false)
            .await
            .unwrap();
        assert_eq!(skipped.state, "skipped");
        assert!(action(&pool, &rule.id, "run").await.is_err()); // rule not yet saved
        put(&pool, &rule).await.unwrap();
        action(&pool, &rule.id, "run").await.unwrap();
        assert_eq!(
            queue::items(&pool)
                .await
                .unwrap()
                .iter()
                .filter(|i| i.state == "queued")
                .count(),
            1
        );
    }
    #[tokio::test]
    async fn workflow1553_changed_missing_sources_fail_without_run_and_can_retry() {
        let (pool, dir, rule) = setup().await;
        put(&pool, &rule).await.unwrap();
        let at = Utc::now().timestamp_millis();
        let first = enqueue(&pool, &rule, at, true, false, false).await.unwrap();
        std::fs::write(&rule.paths[0], b"changed scheduled data").unwrap();
        assert!(queue::claim(&pool, &first.id, "r").await.is_err());
        let failed = queue::items(&pool).await.unwrap()[0].clone();
        assert_eq!(failed.state, "failed");
        assert!(failed.run_id.is_none());
        queue::action(&pool, &first.id, "retry").await.unwrap();
        assert_eq!(queue::items(&pool).await.unwrap().len(), 2);
        let next = queue::items(&pool)
            .await
            .unwrap()
            .into_iter()
            .find(|i| i.state == "queued")
            .unwrap();
        std::fs::remove_file(dir.path().join("source.txt")).unwrap();
        assert!(queue::claim(&pool, &next.id, "r2").await.is_err());
    }
    #[tokio::test]
    async fn workflow1553_session_password_marker_and_missed_run_keep_trigger_snapshot() {
        let (pool, _dir, rule) = setup().await;
        put(&pool, &rule).await.unwrap();
        let mut plan = queue::plan(&pool, "p").await.unwrap();
        plan.steps[0].password_required = true;
        super::super::workflows::save_plan(&pool, plan)
            .await
            .unwrap();
        let item = enqueue(
            &pool,
            &rule,
            Utc::now().timestamp_millis(),
            false,
            true,
            false,
        )
        .await
        .unwrap();
        queue::action(&pool, &item.id, "run_missed").await.unwrap();
        let ready = queue::items(&pool).await.unwrap()[0].clone();
        assert_eq!(ready.state, "needs_input");
        assert!(ready.explicit);
        assert_eq!(ready.scheduled_at, item.scheduled_at);
        assert!(serde_json::to_string(&ready)
            .unwrap()
            .contains("passwordRequired"));
        action(&pool, &rule.id, "delete").await.unwrap();
        let state = queue::snapshot(&pool).await.unwrap();
        assert!(state.schedules.is_empty());
        assert_eq!(state.items[0].state, "cancelled");
    }
    #[tokio::test]
    async fn workflow1553_upgrade17_preserves_directory_rules_and_queue() {
        let (pool, _dir, rule) = setup().await;
        crate::database::migrations::init_tables(&pool)
            .await
            .unwrap();
        let receipt = enqueue(
            &pool,
            &rule,
            Utc::now().timestamp_millis(),
            true,
            false,
            false,
        )
        .await
        .unwrap();
        sqlx::query("DELETE FROM schema_version WHERE version>=18")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DROP TABLE workflow_schedule_rules")
            .execute(&pool)
            .await
            .unwrap();
        crate::database::migrations::init_tables(&pool)
            .await
            .unwrap();
        assert!(list(&pool).await.unwrap().is_empty());
        assert_eq!(queue::items(&pool).await.unwrap()[0].id, receipt.id);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT MAX(version) FROM schema_version")
                .fetch_one(&pool)
                .await
                .unwrap(),
            18
        );

        let bad = Schedule {
            kind: "once".into(),
            date: "2000-01-01".into(),
            ..rule
        };
        assert!(save(&pool, bad).await.is_err());
    }
}

#[cfg(test)]
mod timing_tests {
    use super::*;
    #[tokio::test]
    async fn workflow1553_shared_due_busy_policy_and_suspend_detection() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        queue::init_tables(&pool).await.unwrap();
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.txt");
        std::fs::write(&source, b"source").unwrap();
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();
        let plan=serde_json::from_value(serde_json::json!({"id":"busy-p","name":"归档","steps":[{"id":"s","name":"打包","kind":"compress","settings":{"format":"zip","filename":"{name}"}}]})).unwrap();
        super::super::workflows::save_plan(&pool, plan)
            .await
            .unwrap();
        let at = Utc::now().timestamp_millis();
        let mut rule = Schedule {
            id: "busy".into(),
            name: "到点忙碌".into(),
            plan_id: "busy-p".into(),
            paths: vec![source.to_string_lossy().into()],
            output_directory: out.to_string_lossy().into(),
            kind: "daily".into(),
            time: "18:00".into(),
            date: String::new(),
            weekdays: vec![],
            busy_policy: "skip".into(),
            enabled: true,
            next_at: Some(stamp(at)),
            checked_at: at - 2000,
            note: String::new(),
        };
        put(&pool, &rule).await.unwrap();
        sqlx::query("INSERT INTO workflow_runs VALUES('manual',?,?)")
            .bind(
                serde_json::json!({"id":"manual","planId":"busy-p","status":"running"}).to_string(),
            )
            .bind(stamp(at))
            .execute(&pool)
            .await
            .unwrap();
        poll_at(&pool, at, false, true).await.unwrap();
        assert_eq!(queue::items(&pool).await.unwrap()[0].state, "skipped");
        rule.id = "suspended".into();
        rule.checked_at = at - 180000;
        rule.next_at = Some(stamp(at));
        put(&pool, &rule).await.unwrap();
        poll_at(&pool, at, false, false).await.unwrap();
        assert!(queue::items(&pool)
            .await
            .unwrap()
            .iter()
            .any(|i| i.rule_id == "suspended" && i.state == "missed"));
    }
    #[tokio::test]
    async fn workflow1553_fixed_inputs_can_use_prior_outputs_without_observer_exclusion() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        queue::init_tables(&pool).await.unwrap();
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("previous-result.txt");
        std::fs::write(&source, b"previous result").unwrap();
        sqlx::query("INSERT INTO workflow_trigger_outputs VALUES(?)")
            .bind(dir.path().to_string_lossy().to_string())
            .execute(&pool)
            .await
            .unwrap();
        let (paths, revision) = queue::schedule_sources(&pool, &[source.to_string_lossy().into()])
            .await
            .unwrap();
        assert_eq!(paths.len(), 1);
        assert!(!revision.is_empty());
    }
}
