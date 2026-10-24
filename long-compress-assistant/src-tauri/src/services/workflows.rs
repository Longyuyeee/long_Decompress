//! Stored manual workflow definitions and bounded local input selection.
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::{collections::HashSet, path::Path, sync::atomic::Ordering};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub id: String,
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub extensions: String,
    #[serde(default)]
    pub password_required: bool,
    #[serde(default)]
    pub settings: serde_json::Value,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub id: String,
    pub name: String,
    pub steps: Vec<Step>,
    #[serde(default)]
    pub updated_at: String,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct File {
    pub path: String,
    pub bytes: u64,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    pub path: String,
    pub directory: bool,
    #[serde(default)]
    pub files: Vec<File>,
    #[serde(default)]
    pub total_bytes: u64,
    #[serde(default)]
    pub split: bool,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepRun {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub status: String,
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<Output>,
    #[serde(default)]
    pub task_ids: Vec<String>,
    #[serde(default)]
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_proof: Option<super::workflow_recovery::Proof>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_proof: Option<super::workflow_recovery::Proof>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reuse_issue: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reused_from: Option<String>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub id: String,
    pub plan_id: String,
    pub name: String,
    pub status: String,
    pub started_at: String,
    #[serde(default)]
    pub finished_at: Option<String>,
    pub directory: String,
    pub inputs: Vec<String>,
    pub steps: Vec<StepRun>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<Plan>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_directory: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_encoding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_from: Option<usize>,
}
#[derive(Serialize)]
pub struct State {
    pub plans: Vec<Plan>,
    pub runs: Vec<Run>,
}
pub async fn init_tables(pool: &SqlitePool) -> Result<()> {
    sqlx::query("CREATE TABLE IF NOT EXISTS workflow_plans(id TEXT PRIMARY KEY,name TEXT NOT NULL,definition TEXT NOT NULL,updated_at TEXT NOT NULL)").execute(pool).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS workflow_runs(id TEXT PRIMARY KEY,record TEXT NOT NULL,started_at TEXT NOT NULL)").execute(pool).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS workflow_run_links(child_id TEXT PRIMARY KEY,parent_id TEXT NOT NULL,started_at TEXT NOT NULL)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS workflow_run_parent ON workflow_run_links(parent_id,started_at)").execute(pool).await?;
    Ok(())
}
fn clean_plan(mut plan: Plan) -> Result<Plan> {
    ensure!(!plan.id.is_empty() && plan.id.len() <= 100, "方案标识无效");
    plan.name = plan.name.trim().into();
    ensure!(
        !plan.name.is_empty() && plan.name.chars().count() <= 60,
        "方案名称请控制在 1～60 字"
    );
    ensure!(
        !plan.steps.is_empty() && plan.steps.len() <= 8,
        "请选择 1～8 个顺序步骤"
    );
    let mut ids = HashSet::new();
    for step in &mut plan.steps {
        ensure!(
            !step.id.is_empty() && ids.insert(step.id.clone()) && step.id.len() <= 100,
            "步骤标识重复，请重新添加该步骤"
        );
        ensure!(
            matches!(
                step.kind.as_str(),
                "filter" | "extract" | "compress" | "convert"
            ),
            "不支持的步骤类型"
        );
        ensure!(
            step.name.chars().count() <= 60 && step.extensions.len() <= 512,
            "步骤名称或扩展名过长"
        );
        let incoming = step.settings.as_object();
        step.password_required |= incoming
            .and_then(|s| s.get("password"))
            .and_then(|v| v.as_str())
            .is_some_and(|p| !p.is_empty());
        let mut settings = serde_json::Map::new();
        for (key, kind) in [
            ("format", 's'),
            ("level", 'n'),
            ("filename", 's'),
            ("splitArchive", 'b'),
            ("splitSize", 's'),
            ("keepStructure", 'b'),
            ("verifyAfter", 'b'),
            ("createSolidArchive", 'b'),
            ("encryptFilenames", 'b'),
            ("rarRecoveryPercent", 'n'),
            ("rarRecoveryVolumes", 'n'),
            ("zipPasswordEncoding", 's'),
            ("zipMethod", 's'),
            ("sevenZipMethod", 's'),
            ("sevenZipFilter", 's'),
            ("sevenZipDeltaDistance", 'n'),
            ("zipEncryption", 's'),
            ("zipSplitMode", 's'),
            ("wimMethod", 's'),
        ] {
            if let Some(value) = incoming.and_then(|m| m.get(key)) {
                ensure!(
                    match kind {
                        's' => value.as_str().is_some_and(|v| v.len() <= 500),
                        'b' => value.is_boolean(),
                        _ => value
                            .as_f64()
                            .is_some_and(|v| v.is_finite() && v >= 0.0 && v <= 1048576.0),
                    },
                    "步骤配置「{}」无效",
                    key
                );
                settings.insert(key.into(), value.clone());
            }
        }
        if let Some(method)=settings.get("sevenZipMethod") {
            ensure!(method.as_str().and_then(super::seven_zip_method::SevenZipMethod::from_setting).is_some(),"请选择支持的 7Z 压缩方法：LZMA2、LZMA、Bzip2、Deflate、PPMd 或仅归档（Copy）");
        }
        if let Some(s)=settings.get("sevenZipFilter"){ensure!(s.as_str().and_then(super::seven_zip_method::SevenZipFilter::from_setting).is_some(),"7Z 预处理请选择无、Delta 或 x86 BCJ");}
        if let Some(n)=settings.get("sevenZipDeltaDistance"){ensure!(n.as_u64().is_some_and(|n|(1..=256).contains(&n)),"Delta 距离请输入 1～256 的整数");}
        settings.insert("deleteAfter".into(), false.into());
        step.settings = serde_json::Value::Object(settings);
    }
    plan.updated_at = chrono::Utc::now().to_rfc3339();
    Ok(plan)
}
pub async fn save_plan(pool: &SqlitePool, plan: Plan) -> Result<Plan> {
    let plan = clean_plan(plan)?;
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM workflow_plans WHERE id<>?")
        .bind(&plan.id)
        .fetch_one(pool)
        .await?;
    ensure!(count < 100, "最多保存 100 份方案，请先删除不用的方案");
    sqlx::query("INSERT INTO workflow_plans VALUES(?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,definition=excluded.definition,updated_at=excluded.updated_at").bind(&plan.id).bind(&plan.name).bind(serde_json::to_string(&plan)?).bind(&plan.updated_at).execute(pool).await?;
    Ok(plan)
}
pub async fn delete_plan(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM workflow_plans WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
pub async fn save_run(pool: &SqlitePool, mut run: Run) -> Result<()> {
    ensure!(
        run.steps.len() <= 8
            && run.inputs.len() <= 1000
            && run.steps.iter().all(|s| s.inputs.len() <= 50000
                && s.outputs.len() <= 50000
                && s.task_ids.len() <= 1000),
        "运行记录超过当前范围"
    );
    if let Some(plan) = run.plan.take() {
        let stamp = plan.updated_at.clone();
        let mut plan = clean_plan(plan)?;
        plan.updated_at = stamp;
        ensure!(
            plan.steps.len() == run.steps.len(),
            "执行快照与步骤记录不对应"
        );
        run.plan = Some(plan);
    }
    let record = serde_json::to_string(&run)?;
    ensure!(
        record.len() <= 16 * 1024 * 1024,
        "运行记录过大，请缩小来源范围"
    );
    sqlx::query("INSERT INTO workflow_runs VALUES(?,?,?) ON CONFLICT(id) DO UPDATE SET record=excluded.record").bind(&run.id).bind(record).bind(&run.started_at).execute(pool).await?;
    if let Some(parent) = run
        .parent_run_id
        .as_ref()
        .filter(|parent| *parent != &run.id)
    {
        sqlx::query("INSERT INTO workflow_run_links VALUES(?,?,?) ON CONFLICT(child_id) DO UPDATE SET parent_id=excluded.parent_id").bind(&run.id).bind(parent).bind(&run.started_at).execute(pool).await?;
    }
    super::workflow_triggers::RUNS_CHANGED.store(true, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}
pub async fn state(pool: &SqlitePool) -> Result<State> {
    let plans: Vec<(String,)> =
        sqlx::query_as("SELECT definition FROM workflow_plans ORDER BY updated_at DESC,id")
            .fetch_all(pool)
            .await?;
    let runs: Vec<(String,)> =
        sqlx::query_as("SELECT record FROM workflow_runs ORDER BY started_at DESC,id LIMIT 100")
            .fetch_all(pool)
            .await?;
    Ok(State {
        plans: plans
            .into_iter()
            .map(|(v,)| Ok(serde_json::from_str(&v)?))
            .collect::<Result<_>>()?,
        runs: runs
            .into_iter()
            .map(|(v,)| Ok(serde_json::from_str(&v)?))
            .collect::<Result<_>>()?,
    })
}
pub fn begin_directory(parent: &Path, name: &str) -> Result<String> {
    ensure!(parent.is_dir(), "请选择已存在的输出目录");
    let name = name.trim();
    ensure!(
        !name.is_empty()
            && name.len() <= 180
            && !name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
            && name != "."
            && name != "..",
        "本次目录名称无效"
    );
    let target = parent.join(name);
    std::fs::create_dir(&target).context("无法新建本次运行目录，请更换输出位置；已有结果保留")?;
    Ok(target.to_string_lossy().into())
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub paths: Vec<String>,
    pub visited: usize,
    pub bytes: u64,
}
pub fn collect(
    paths: Vec<String>,
    extensions: &str,
    exclude: &Path,
    job_id: &str,
    output_base: Option<&Path>,
) -> Result<Collection> {
    ensure!(paths.len() <= 50000, "来源过多，请分批处理");
    let job = super::archive_library::Job::start(job_id)?;
    let extensions = extensions
        .split([',', '，', ';', '；', ' '])
        .filter(|v| !v.trim().is_empty())
        .map(|s| s.trim().trim_start_matches("*.").to_lowercase())
        .collect::<HashSet<_>>();
    ensure!(
        extensions.len() <= 100
            && extensions
                .iter()
                .all(|e| e.len() <= 30 && !e.contains(['/', '\\', '*', '?'])),
        "请用逗号填写扩展名，例如 txt,pdf,jpg，留空选择所有文件"
    );
    let mut result = Collection {
        paths: vec![],
        visited: 0,
        bytes: 0,
    };
    let mut seen = HashSet::new();
    let excluded = std::fs::canonicalize(exclude).ok();
    let output_base = output_base.and_then(|p| std::fs::canonicalize(p).ok());
    for root in paths {
        let root = std::fs::canonicalize(&root).with_context(|| format!("来源无法访问：{root}"))?;
        let internal = excluded.as_ref().is_some_and(|e| root.starts_with(e));
        let skip_base = output_base.as_ref().filter(|base| !root.starts_with(base));
        let walker = walkdir::WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                (internal || !excluded.as_ref().is_some_and(|x| e.path().starts_with(x)))
                    && !skip_base.is_some_and(|base| e.path().starts_with(base))
            });
        for entry in walker {
            ensure!(
                !job.flag.load(Ordering::SeqCst),
                "已停止筛选，后续步骤未开始"
            );
            let entry = entry.context("部分来源无法读取，请缩小范围或处理后重新运行")?;
            result.visited += 1;
            ensure!(
                result.visited <= 100000,
                "本次扫描超过 10 万位置，请缩小来源范围"
            );
            if !entry.file_type().is_file() {
                continue;
            }
            if !extensions.is_empty()
                && !extensions.contains(
                    &entry
                        .path()
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase(),
                )
            {
                continue;
            }
            if seen.insert(entry.path().to_string_lossy().to_lowercase()) {
                ensure!(
                    result.paths.len() < 50000,
                    "所选文件超过 5 万个，请分批处理"
                );
                result.bytes += entry.metadata()?.len();
                result
                    .paths
                    .push(entry.path().to_string_lossy().to_string());
            }
        }
    }
    result.paths.sort();
    ensure!(
        !result.paths.is_empty(),
        "没有符合扩展名的文件，请调整筛选条件；后续步骤未开始"
    );
    Ok(result)
}
pub fn children(path: &Path) -> Result<Vec<String>> {
    let mut result = vec![];
    for entry in std::fs::read_dir(path)? {
        let e = entry?;
        ensure!(
            !e.file_type()?.is_symlink(),
            "来源中有链接，请从原压缩中心选择实际文件"
        );
        result.push(e.path().to_string_lossy().into());
        ensure!(result.len() <= 50000, "目录内容过多，请分批选择");
    }
    ensure!(!result.is_empty(), "上一步的输出目录为空，没有可打包的内容");
    Ok(result)
}

#[cfg(test)]
mod workflow1550_tests {
    use super::*;
    fn plan() -> Plan {
        serde_json::from_value(serde_json::json!({"id":"p","name":" 按日归档 ","steps":[{"id":"s","name":"压缩","kind":"compress","settings":{"format":"zip","level":6,"password":"session-secret","deleteAfter":true,"unknown":"secret"}}]})).unwrap()
    }
    #[test]
    fn workflow1550_plan_keeps_parameters_without_credentials_or_source_deletion() {
        let p = clean_plan(plan()).unwrap();
        assert_eq!(p.name, "按日归档");
        assert!(p.steps[0].password_required);
        assert_eq!(p.steps[0].settings["format"], "zip");
        assert_eq!(p.steps[0].settings["deleteAfter"], false);
        let json = serde_json::to_string(&p).unwrap();
        assert!(!json.contains("session-secret"));
        assert!(!json.contains("unknown"));
        let mut p = plan();
        p.steps.push(p.steps[0].clone());
        assert!(clean_plan(p).is_err());
    }
    #[tokio::test]
    async fn workflow1550_persistence_and_v15_migration_preserve_library() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::database::migrations::init_tables(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO archive_library_sources(id,path,name,format,status) VALUES('old','old.zip','old.zip','ZIP','ready')").execute(&pool).await.unwrap();
        sqlx::query("DROP TABLE workflow_plans")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DROP TABLE workflow_runs")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM schema_version WHERE version>=16")
            .execute(&pool)
            .await
            .unwrap();
        crate::database::migrations::init_tables(&pool)
            .await
            .unwrap();
        crate::database::migrations::init_tables(&pool)
            .await
            .unwrap();
        save_plan(&pool, plan()).await.unwrap();
        assert_eq!(state(&pool).await.unwrap().plans.len(), 1);
        let run:Run=serde_json::from_value(serde_json::json!({"id":"r","planId":"p","name":"归档","status":"completed","startedAt":"2026-10-21","directory":"out","inputs":["a.zip"],"steps":[{"id":"s","name":"压缩","kind":"compress","status":"completed","outputs":[{"path":"actual.zip.001","directory":false,"files":[{"path":"actual.zip.001","bytes":12}],"totalBytes":12,"split":true}],"password":"ignored"}]})).unwrap();
        save_run(&pool, run).await.unwrap();
        delete_plan(&pool, "p").await.unwrap();
        let state = state(&pool).await.unwrap();
        assert!(state.plans.is_empty());
        assert_eq!(state.runs[0].steps[0].outputs[0].path, "actual.zip.001");
        assert!(!serde_json::to_string(&state.runs)
            .unwrap()
            .contains("ignored"));
        assert_eq!(
            crate::services::archive_library::source(&pool, "old")
                .await
                .unwrap()
                .status,
            "ready"
        );
    }
    #[test]
    fn workflow1550_filter_uses_real_files_and_excludes_current_output() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("input");
        std::fs::create_dir(&input).unwrap();
        std::fs::write(input.join("report.TXT"), b"report").unwrap();
        std::fs::write(input.join("skip.png"), b"skip").unwrap();
        let output = begin_directory(&input, "output").unwrap();
        std::fs::write(Path::new(&output).join("old.txt"), b"old").unwrap();
        let result = collect(
            vec![
                input.to_string_lossy().into(),
                input.join("report.TXT").to_string_lossy().into(),
            ],
            "*.txt,md",
            Path::new(&output),
            "workflow1550-filter",
            None,
        )
        .unwrap();
        assert_eq!(result.paths.len(), 1);
        assert_eq!(result.bytes, 6);
        assert!(result.paths[0].ends_with("report.TXT"));
        let inside = collect(
            vec![output.clone()],
            "txt",
            Path::new(&output),
            "workflow1550-inside",
            None,
        )
        .unwrap();
        assert_eq!(inside.paths.len(), 1);
        let old = Path::new(&output).join("prior");
        std::fs::create_dir(&old).unwrap();
        std::fs::write(old.join("report.txt"), b"old-run").unwrap();
        let current = begin_directory(Path::new(&output), "current").unwrap();
        let repeated = collect(
            vec![input.to_string_lossy().into()],
            "txt",
            Path::new(&current),
            "workflow1550-output-base",
            Some(Path::new(&output)),
        )
        .unwrap();
        assert_eq!(repeated.paths.len(), 1);
        let downstream = collect(
            vec![old.to_string_lossy().into()],
            "txt",
            Path::new(&current),
            "workflow1550-downstream",
            Some(Path::new(&output)),
        )
        .unwrap();
        assert_eq!(downstream.paths.len(), 1);

        assert!(collect(
            vec![input.to_string_lossy().into()],
            "pdf",
            Path::new(&output),
            "workflow1550-empty",
            None
        )
        .is_err());
        assert!(begin_directory(&input, "output").is_err());
        assert_eq!(
            std::fs::read(Path::new(&output).join("old.txt")).unwrap(),
            b"old"
        );
        assert!(begin_directory(&input, "../elsewhere").is_err());
    }
    #[test]
    fn workflow1550_child_selection_rejects_empty_output() {
        let temp = tempfile::tempdir().unwrap();
        assert!(children(temp.path()).is_err());
        std::fs::write(temp.path().join("one.txt"), b"one").unwrap();
        assert_eq!(children(temp.path()).unwrap().len(), 1);
    }
}
