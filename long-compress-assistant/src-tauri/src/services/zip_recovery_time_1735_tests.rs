//! Repaired ZIP metadata and committed filesystem dates, using independent sources.
use super::{
    archive_diagnostics::repair_zip_with_password_to_new, extraction_transaction as tx,
    native_extraction::ExtractionRuntime, zip_recovery_outcome::RecoveryStatus,
};
use crate::models::compression::{DecompressOptions, TaskLogSeverity};
use anyhow::Result;
use chrono::TimeZone;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Mutex},
    time::UNIX_EPOCH,
};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/zip-recovery-time-1735")
}
fn base() -> PathBuf {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/zip-recovery-time-1735");
    fs::create_dir_all(&p).unwrap();
    p
}
fn cases() -> Vec<Value> {
    let m: Value =
        serde_json::from_slice(&fs::read(fixtures().join("manifest.json")).unwrap()).unwrap();
    m["cases"].as_array().unwrap().clone()
}
fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn save(name: &str, rows: &[Value]) {
    fs::write(base().join(name), serde_json::to_vec_pretty(rows).unwrap()).unwrap();
}
#[derive(Default)]
struct Runtime {
    pool: super::io_buffer_pool::IOBufferPool,
    threads: Mutex<HashSet<std::thread::ThreadId>>,
}
impl ExtractionRuntime for Runtime {
    fn check_cancellation(&self) -> Result<()> {
        self.threads
            .lock()
            .unwrap()
            .insert(std::thread::current().id());
        Ok(())
    }
    fn buffer_pool(&self) -> &super::io_buffer_pool::IOBufferPool {
        &self.pool
    }
    fn copy_buffer_size(&self) -> usize {
        65536
    }
    fn normalized_archive_path(&self, p: &Path, preserve: bool) -> Option<PathBuf> {
        if preserve {
            Some(p.into())
        } else {
            p.file_name().map(PathBuf::from)
        }
    }
    fn emit_log(&self, _: &tauri::Window, _: &str, _: &str, _: TaskLogSeverity) {}
    fn emit_progress(&self, _: &tauri::Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {
    }
}
fn expected_seconds(e: &Value) -> i64 {
    if let Some(s) = e["utc"].as_i64() {
        return s;
    }
    let d = e["dos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as u32)
        .collect::<Vec<_>>();
    chrono::Local
        .with_ymd_and_hms(d[0] as i32, d[1], d[2], d[3], d[4], d[5])
        .single()
        .unwrap()
        .timestamp()
}
fn restore(
    input: &Path,
    target: &Path,
    entries: &[Value],
    password: Option<&str>,
    parallel: bool,
) -> usize {
    let runtime = Runtime::default();
    let options = DecompressOptions {
        preserve_paths: true,
        preserve_timestamps: true,
        parallel_zip: parallel,
        conflict_policy: "rename".into(),
        ..Default::default()
    };
    let mut stage = tx::ExtractionStaging::create_for(target).unwrap();
    super::native_extraction::zip::extract_with_optional_window(
        &runtime,
        None,
        "recoverytime1735",
        input.to_str().unwrap(),
        stage.path().to_str().unwrap(),
        password,
        &options,
    )
    .unwrap();
    tx::prepare_staging_layout(input, stage.path(), &options).unwrap();
    let times =
        super::zip_directory_times::for_commit(input, stage.path(), &options, || Ok(())).unwrap();
    tx::commit_staged_extraction(
        input.to_str().unwrap(),
        stage.path(),
        target,
        &options,
        |_| {},
    )
    .unwrap();
    stage.cleanup().unwrap();
    assert!(super::lzop::selected::apply_directory_times(target, &times).is_empty());
    assert_eq!(
        walkdir::WalkDir::new(target).into_iter().count() - 1,
        entries.len()
    );
    for e in entries {
        let p = target.join(e["name"].as_str().unwrap());
        assert_eq!(p.is_dir(), e["directory"].as_bool().unwrap());
        if p.is_file() {
            assert_eq!(sha(&fs::read(&p).unwrap()), e["sha256"]);
        }
        let t = filetime::FileTime::from_last_modification_time(&p.metadata().unwrap());
        assert_eq!(t.unix_seconds(), expected_seconds(e), "{}", p.display());
        assert_eq!(t.nanoseconds(), 0);
    }
    let count = runtime.threads.lock().unwrap().len();
    count
}
fn run(spec: &Value) -> Value {
    let parent = base().join(uuid::Uuid::new_v4().to_string());
    let source_dir = parent.join("source");
    fs::create_dir_all(&source_dir).unwrap();
    let original = fs::read(fixtures().join(spec["archive"].as_str().unwrap())).unwrap();
    if let Some(expected) = spec["originalSha256"].as_str() {
        assert_eq!(sha(&original), expected);
    }
    let mut bytes = original.clone();
    match spec["recipe"]["kind"].as_str().unwrap() {
        "prefix" => bytes.truncate(spec["recipe"]["length"].as_u64().unwrap() as usize),
        "xor" => bytes[spec["recipe"]["position"].as_u64().unwrap() as usize] ^= 1,
        "copy" => {}
        _ => unreachable!(),
    }
    assert_eq!(sha(&bytes), spec["sha256"]);
    let single = source_dir.join("source.zip");
    fs::write(&single, &bytes).unwrap();
    let name = spec["name"].as_str().unwrap();
    let sources = if name == "numeric-lost" {
        bytes
            .chunks(65536)
            .enumerate()
            .map(|(i, b)| {
                let p = source_dir.join(format!("volumes.zip.{:03}", i + 1));
                fs::write(&p, b).unwrap();
                p
            })
            .collect::<Vec<_>>()
    } else if name == "standard-crc" {
        super::standard_zip_writer::split(
            &single,
            &source_dir.join("standard.zip"),
            65536,
            || Ok(()),
            |_, _| {},
        )
        .unwrap()
        .part_files
    } else {
        vec![single.clone()]
    };
    let input = if sources.len() > 1 {
        assert!(sources.len() > 2);
        sources[1].clone()
    } else {
        sources[0].clone()
    };
    let originals = sources
        .iter()
        .map(|p| json!({"path":p,"sha256":sha(&fs::read(p).unwrap())}))
        .collect::<Vec<_>>();
    let output = parent.join("recovered.zip");
    let password = spec["password"].as_str();
    if name == "aes-damaged" {
        for bad in [None, Some("wrong-password")] {
            assert!(
                repair_zip_with_password_to_new(&input, &output, bad, &AtomicBool::new(false))
                    .is_err()
            );
            assert!(!output.exists());
        }
    }
    let result =
        repair_zip_with_password_to_new(&input, &output, password, &AtomicBool::new(false))
            .unwrap();
    let entries = spec["entries"].as_array().unwrap();
    assert!(result.verified && result.partial);
    assert_eq!(result.directory_rebuilt, spec["rebuilt"].as_bool().unwrap());
    assert_eq!(
        result.output_encrypted,
        spec["encrypted"].as_bool().unwrap()
    );
    assert_eq!(
        result.recovered_files,
        entries.iter().filter(|e| e["directory"] == false).count()
    );
    assert_eq!(
        result.recovered_directories,
        entries.iter().filter(|e| e["directory"] == true).count()
    );
    for skipped in spec["skipped"].as_array().unwrap() {
        assert!(!result
            .recovered_entries
            .iter()
            .any(|n| n == skipped.as_str().unwrap()));
        assert!(result.entry_results.iter().any(
            |e| e.name.as_deref() == skipped.as_str() && e.status != RecoveryStatus::Recovered
        ));
    }
    let mut zip = zip_aes::ZipArchive::new(fs::File::open(&output).unwrap()).unwrap();
    assert_eq!(zip.len(), entries.len());
    let mut headers = Vec::new();
    for i in 0..zip.len() {
        let mut e = super::zip_password::open_entry_with_encoding(
            &mut zip,
            i,
            password.unwrap_or(""),
            None,
        )
        .unwrap();
        let expected = entries
            .iter()
            .find(|r| r["name"].as_str().unwrap() == e.name().trim_end_matches('/'))
            .unwrap();
        let actual = super::zip_read_time::utc_modified(e.extra_data())
            .map(|t| t.duration_since(UNIX_EPOCH).unwrap().as_secs());
        assert_eq!(actual, expected["utc"].as_u64());
        let d = e.last_modified().unwrap();
        let dos = json!([
            d.year(),
            d.month(),
            d.day(),
            d.hour(),
            d.minute(),
            d.second()
        ]);
        assert_eq!(dos, expected["dos"]);
        let mut content = Vec::new();
        e.read_to_end(&mut content).unwrap();
        assert_eq!(sha(&content), expected["sha256"]);
        headers.push(json!({"name":e.name(),"utc":actual,"dos":dos}));
    }
    drop(zip);
    let target = parent.join("native-restored");
    restore(&output, &target, entries, password, false);
    let mut restored = vec![target];
    let mut workers = 0;
    if name == "parallel-flat" {
        let target = parent.join("native-parallel");
        workers = restore(&output, &target, entries, password, true);
        assert!(workers > 1);
        restored.push(target);
    }
    for p in &originals {
        assert_eq!(
            sha(&fs::read(p["path"].as_str().unwrap()).unwrap()),
            p["sha256"]
        );
    }
    assert_eq!(fs::read(&single).unwrap(), bytes);
    assert_eq!(
        fs::read(fixtures().join(spec["archive"].as_str().unwrap())).unwrap(),
        original
    );
    assert!(!fs::read_dir(&parent).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".long-repair")));
    json!({"name":name,"input":input,"output":output,"sources":originals,"expected":entries,"headers":headers,"restored":restored,"workers":workers,"password":password,"result":result})
}

#[test]
fn recoverytime1735_full_and_rebuilt_metadata_controls_keep_utc_dos_and_fallbacks() {
    let rows = cases()
        .into_iter()
        .filter(|s| s["name"].as_str().unwrap().starts_with("controls-"))
        .map(|s| run(&s))
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 2);
    save("controls.json", &rows);
}
#[test]
fn recoverytime1735_real_plain_password_and_actual_volume_outputs_retain_dates() {
    let rows = cases()
        .into_iter()
        .filter(|s| {
            matches!(
                s["name"].as_str().unwrap(),
                "plain-lost"
                    | "parallel-flat"
                    | "aes-damaged"
                    | "aes-bad-ut-damaged"
                    | "numeric-lost"
                    | "standard-crc"
            )
        })
        .map(|s| run(&s))
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 6);
    save("native.json", &rows);
}
#[test]
fn recoverytime1735_bzip2_lzma_stream_recovery_keeps_dos_dates_without_inventing_utc() {
    let rows = cases()
        .into_iter()
        .filter(|s| s["name"].as_str().unwrap().ends_with("-stream"))
        .map(|s| run(&s))
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 2);
    save("streams.json", &rows);
}
