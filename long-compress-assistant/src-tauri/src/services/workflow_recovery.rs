//! Content receipts for user-initiated workflow continuation. No credentials.
use super::workflows::{Output, Run};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::Metadata,
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
    time::{Instant, UNIX_EPOCH},
};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Policy {
    pub excludes: Vec<String>,
    pub extensions: Option<String>,
    pub files_only: bool,
    pub archive_members: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Proof {
    pub digest: String,
    pub metadata: String,
    pub files: u64,
    pub bytes: u64,
    pub policy: Policy,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub files: u64,
    pub bytes: u64,
    pub path: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub index: usize,
    pub reason: String,
}
#[derive(Serialize)]
pub struct Report {
    pub reusable: usize,
    pub issue: Option<Issue>,
}
fn stopped(job: &super::archive_library::Job) -> Result<()> {
    ensure!(
        !job.flag.load(Ordering::SeqCst),
        "已停止核对，尚未创建新的任务"
    );
    Ok(())
}
fn metadata_key(meta: &Metadata) -> String {
    format!(
        "{}:{}:{}",
        meta.is_dir(),
        meta.len(),
        meta.modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    )
}
fn field(hash: &mut blake3::Hasher, value: &[u8]) {
    hash.update(&(value.len() as u64).to_le_bytes());
    hash.update(value);
}
fn paths(outputs: &[Output]) -> Vec<String> {
    outputs
        .iter()
        .flat_map(|o| {
            if o.files.is_empty() {
                vec![o.path.clone()]
            } else {
                o.files.iter().map(|f| f.path.clone()).collect()
            }
        })
        .collect()
}
struct Entry {
    path: PathBuf,
    key: String,
    meta: Metadata,
}
fn entries(
    roots: &[String],
    policy: &Policy,
    job: &super::archive_library::Job,
) -> Result<Vec<Entry>> {
    ensure!(
        roots.len() <= 50000 && policy.excludes.len() <= 16,
        "超过本版接续记录范围，请从此步重新处理"
    );
    let excludes = policy
        .excludes
        .iter()
        .filter_map(|p| std::fs::canonicalize(p).ok())
        .collect::<Vec<_>>();
    let extensions = policy
        .extensions
        .as_deref()
        .unwrap_or("")
        .split([',', '，', ';', '；', ' '])
        .map(|s| s.trim().trim_start_matches("*.").to_lowercase())
        .filter(|v| !v.is_empty())
        .collect::<HashSet<_>>();
    let mut expanded = vec![];
    for root in roots {
        stopped(job)?;
        if policy.archive_members {
            if let Some(group) =
                super::split_archive_detector::SplitArchiveDetector::detect_split_archive(
                    Path::new(root),
                )?
            {
                ensure!(
                    group.is_complete,
                    "缺少分卷：{}",
                    group.missing_part_names()
                );
                expanded.extend(group.parts);
                continue;
            }
        }
        expanded.push(PathBuf::from(root));
    }
    let mut selected = BTreeMap::new();
    let mut visited = 0;
    for root in expanded {
        stopped(job)?;
        let root = std::fs::canonicalize(&root)
            .with_context(|| format!("找不到或无法访问：{}", root.display()))?;
        let omitted = excludes
            .iter()
            .filter(|e| !root.starts_with(e))
            .collect::<Vec<_>>();
        for entry in walkdir::WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !omitted.iter().any(|x| e.path().starts_with(x)))
        {
            stopped(job)?;
            let entry = entry.context("读取接续资料失败")?;
            visited += 1;
            ensure!(visited <= 200000, "核对超过 20 万个位置，请从此步重新处理");
            ensure!(
                !entry.file_type().is_symlink(),
                "资料中有链接，无法沿用此步结果：{}",
                entry.path().display()
            );
            let meta = entry.metadata()?;
            if meta.is_dir() && policy.files_only {
                continue;
            }
            if !meta.is_dir() && !meta.is_file() {
                continue;
            }
            if meta.is_file()
                && !extensions.is_empty()
                && !extensions.contains(
                    &entry
                        .path()
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase(),
                )
            {
                continue;
            }
            let key = entry.path().to_string_lossy().replace('\\', "/");
            selected.entry(key.to_lowercase()).or_insert(Entry {
                path: entry.path().into(),
                key,
                meta,
            });
            ensure!(selected.len() <= 100000, "核对条目过多，请从此步重新处理");
        }
    }
    ensure!(
        !selected.is_empty(),
        "没有可核对的来源或产物，请从此步重新处理"
    );
    Ok(selected.into_values().collect())
}
fn fingerprint(
    roots: &[String],
    policy: &Policy,
    job: &super::archive_library::Job,
    content: bool,
    cache: &mut HashMap<String, String>,
    progress: &mut dyn FnMut(Progress),
) -> Result<Proof> {
    let entries = entries(roots, policy, job)?;
    let mut metadata = blake3::Hasher::new();
    for e in &entries {
        field(&mut metadata, e.key.as_bytes());
        field(&mut metadata, metadata_key(&e.meta).as_bytes());
    }
    let metadata = metadata.finalize().to_hex().to_string();
    let files = entries.iter().filter(|e| e.meta.is_file()).count() as u64;
    let bytes = entries
        .iter()
        .filter(|e| e.meta.is_file())
        .map(|e| e.meta.len())
        .sum();
    if !content {
        return Ok(Proof {
            digest: String::new(),
            metadata,
            files,
            bytes,
            policy: policy.clone(),
        });
    }
    if let Some(digest) = cache.get(&metadata) {
        return Ok(Proof {
            digest: digest.clone(),
            metadata,
            files,
            bytes,
            policy: policy.clone(),
        });
    }
    let mut digest = blake3::Hasher::new();
    let mut done = Progress {
        files: 0,
        bytes: 0,
        path: String::new(),
    };
    let mut buffer = vec![0u8; 1024 * 1024];
    let mut tick = Instant::now();
    for e in entries {
        stopped(job)?;
        field(&mut digest, e.key.as_bytes());
        field(
            &mut digest,
            if e.meta.is_dir() {
                b"directory"
            } else {
                b"file"
            },
        );
        if e.meta.is_file() {
            field(&mut digest, &e.meta.len().to_le_bytes());
            let mut file = std::fs::File::open(&e.path)
                .with_context(|| format!("无法读取：{}", e.path.display()))?;
            ensure!(
                metadata_key(&file.metadata()?) == metadata_key(&e.meta),
                "读取前文件已变化：{}",
                e.path.display()
            );
            loop {
                stopped(job)?;
                let n = file.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                digest.update(&buffer[..n]);
                done.bytes += n as u64;
                if tick.elapsed().as_millis() >= 150 {
                    done.path = e.key.clone();
                    progress(done.clone());
                    tick = Instant::now();
                }
            }
            ensure!(
                metadata_key(&file.metadata()?) == metadata_key(&e.meta)
                    && metadata_key(&std::fs::metadata(&e.path)?) == metadata_key(&e.meta),
                "读取期间文件已变化：{}",
                e.path.display()
            );
            done.files += 1;
        }
    }
    progress(Progress {
        files,
        bytes,
        path: String::new(),
    });
    let digest = digest.finalize().to_hex().to_string();
    cache.insert(metadata.clone(), digest.clone());
    Ok(Proof {
        digest,
        metadata,
        files,
        bytes,
        policy: policy.clone(),
    })
}
pub fn witness(
    roots: Vec<String>,
    policy: Policy,
    job_id: &str,
    metadata_only: bool,
    progress: &mut dyn FnMut(Progress),
) -> Result<Proof> {
    let job = super::archive_library::Job::start(job_id)?;
    fingerprint(
        &roots,
        &policy,
        &job,
        !metadata_only,
        &mut HashMap::new(),
        progress,
    )
}
pub fn check(
    run: &Run,
    count: usize,
    job_id: &str,
    progress: &mut dyn FnMut(Progress),
) -> Result<Report> {
    ensure!(count <= run.steps.len(), "接续步骤无效");
    let job = super::archive_library::Job::start(job_id)?;
    let mut cache = HashMap::new();
    let mut observed = vec![];
    for (index, step) in run.steps.iter().take(count).enumerate() {
        let result = (|| -> Result<()> {
            ensure!(
                run.plan.is_some() && step.status == "completed" && step.reuse_issue.is_none(),
                "没有完整接续依据，请从此步重新处理"
            );
            for (roots, proof, label) in [
                (step.inputs.clone(), step.input_proof.as_ref(), "输入"),
                (paths(&step.outputs), step.output_proof.as_ref(), "产物"),
            ] {
                let proof = proof.context("旧记录缺少内容依据")?;
                ensure!(!proof.digest.is_empty(), "缺少内容指纹");
                let current = fingerprint(&roots, &proof.policy, &job, true, &mut cache, progress)
                    .with_context(|| format!("{label}无法核对"))?;
                ensure!(
                    current.digest == proof.digest,
                    "{label}的内容或目录结构已变化"
                );
                observed.push((index, roots, current));
            }
            Ok(())
        })();
        if let Err(error) = result {
            stopped(&job)?;
            return Ok(Report {
                reusable: index,
                issue: Some(Issue {
                    index,
                    reason: format!("第 {} 步「{}」：{error:#}", index + 1, step.name),
                }),
            });
        }
    }
    // A long read must not validate an earlier file that changed during later reads.
    for (index, roots, proof) in observed {
        let current = fingerprint(&roots, &proof.policy, &job, false, &mut cache, progress);
        match current {
            Ok(value) if value.metadata == proof.metadata => {}
            _ => {
                stopped(&job)?;
                return Ok(Report {
                    reusable: index,
                    issue: Some(Issue {
                        index,
                        reason: format!(
                            "第 {} 步资料在核对期间发生变化，请重新核对或从此步处理",
                            index + 1
                        ),
                    }),
                });
            }
        }
    }
    Ok(Report {
        reusable: count,
        issue: None,
    })
}

#[cfg(test)]
mod workflow1551_tests {
    use super::*;
    fn path(p: &Path) -> String {
        p.to_string_lossy().into()
    }
    fn proof(p: &Path) -> Proof {
        witness(
            vec![path(p)],
            Policy::default(),
            &uuid::Uuid::new_v4().to_string(),
            false,
            &mut |_| {},
        )
        .unwrap()
    }
    fn run(input: &Path, output: &Path) -> Run {
        let mut run:Run=serde_json::from_value(serde_json::json!({"id":"run","planId":"plan","name":"恢复","status":"failed","startedAt":"now","directory":"old","inputs":[path(input)],"plan":{"id":"plan","name":"恢复","steps":[{"id":"first","name":"第一步","kind":"compress"},{"id":"last","name":"失败步骤","kind":"convert"}]},"steps":[{"id":"first","name":"第一步","kind":"compress","status":"completed","inputs":[path(input)],"outputs":[{"path":path(output),"directory":false}]},{"id":"last","name":"失败步骤","kind":"convert","status":"failed"}]})).unwrap();
        run.steps[0].input_proof = Some(proof(input));
        run.steps[0].output_proof = Some(proof(output));
        run
    }
    #[test]
    fn workflow1551_same_size_and_timestamp_content_change_blocks_reuse() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("source");
        let output = dir.path().join("result");
        std::fs::write(&input, b"first").unwrap();
        std::fs::write(&output, b"result").unwrap();
        let run = run(&input, &output);
        assert!(check(&run, 1, "workflow1551-ok", &mut |_| {})
            .unwrap()
            .issue
            .is_none());
        let stamp = std::fs::metadata(&input).unwrap().modified().unwrap();
        std::fs::write(&input, b"other").unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&input)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(stamp))
            .unwrap();
        let report = check(&run, 1, "workflow1551-changed", &mut |_| {}).unwrap();
        assert_eq!(report.issue.as_ref().unwrap().index, 0);
        assert!(report.issue.unwrap().reason.contains("输入的内容"));
        assert_eq!(std::fs::read(&output).unwrap(), b"result");
    }
    #[test]
    fn workflow1551_missing_output_and_legacy_receipt_never_count_as_success() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("source");
        let output = dir.path().join("result");
        std::fs::write(&input, b"first").unwrap();
        std::fs::write(&output, b"result").unwrap();
        let mut run = run(&input, &output);
        std::fs::remove_file(&output).unwrap();
        assert!(check(&run, 1, "workflow1551-missing", &mut |_| {})
            .unwrap()
            .issue
            .unwrap()
            .reason
            .contains("产物"));
        run.plan = None;
        run.steps[0].input_proof = None;
        assert!(check(&run, 1, "workflow1551-legacy", &mut |_| {})
            .unwrap()
            .issue
            .is_some());
        assert!(check(&run, 0, "workflow1551-from-zero", &mut |_| {})
            .unwrap()
            .issue
            .is_none());
    }
    #[test]
    fn workflow1551_directory_structure_filter_scope_and_output_exclusion() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("output");
        std::fs::create_dir(&output).unwrap();
        std::fs::write(dir.path().join("keep.TXT"), b"keep").unwrap();
        std::fs::write(dir.path().join("skip.png"), b"skip").unwrap();
        std::fs::write(output.join("old.txt"), b"old").unwrap();
        let policy = Policy {
            excludes: vec![path(&output)],
            extensions: Some("txt".into()),
            files_only: true,
            ..Default::default()
        };
        let before = witness(
            vec![path(dir.path())],
            policy.clone(),
            "workflow1551-filter-before",
            false,
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(before.files, 1);
        std::fs::write(output.join("new.txt"), b"new").unwrap();
        std::fs::write(dir.path().join("skip.png"), b"different").unwrap();
        let after = witness(
            vec![path(dir.path())],
            policy,
            "workflow1551-filter-after",
            false,
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(before.digest, after.digest);
        let first = proof(&output);
        std::fs::create_dir(output.join("empty")).unwrap();
        assert_ne!(first.digest, proof(&output).digest);
    }
    #[test]
    fn workflow1551_cancelled_read_never_returns_a_content_proof() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("source"), b"x").unwrap();
        let job = super::super::archive_library::Job::start("workflow1551-stop").unwrap();
        job.flag.store(true, Ordering::SeqCst);
        assert!(fingerprint(
            &[path(dir.path())],
            &Policy::default(),
            &job,
            true,
            &mut HashMap::new(),
            &mut |_| {}
        )
        .unwrap_err()
        .to_string()
        .contains("停止"));
    }
    #[tokio::test]
    async fn workflow1551_snapshot_persists_without_passwords_and_old_summaries_remain_readable() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        super::super::workflows::init_tables(&pool).await.unwrap();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input");
        let output = dir.path().join("output");
        std::fs::write(&input, b"x").unwrap();
        std::fs::write(&output, b"y").unwrap();
        let mut run = run(&input, &output);
        run.plan.as_mut().unwrap().steps[0].settings =
            serde_json::json!({"password":"session-only","deleteAfter":true,"format":"zip"});
        super::super::workflows::save_run(&pool, run).await.unwrap();
        let state = super::super::workflows::state(&pool).await.unwrap();
        assert!(state.runs[0].plan.as_ref().unwrap().steps[0].password_required);
        assert!(!serde_json::to_string(&state.runs)
            .unwrap()
            .contains("session-only"));
        assert!(state.runs[0].steps[0].output_proof.is_some());
        let old:Run=serde_json::from_value(serde_json::json!({"id":"old","planId":"old","name":"旧摘要","status":"completed","startedAt":"2026","directory":"old","inputs":[],"steps":[]})).unwrap();
        assert!(old.plan.is_none());
        super::super::workflows::save_run(&pool, old).await.unwrap();
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
