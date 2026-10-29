use super::{
    archive_browser, extraction_transaction as tx, native_extraction::ExtractionRuntime,
    zip_directory_times as dates, zip_read_time,
};
use crate::models::compression::{CompressionOptions, DecompressOptions, TaskLogSeverity};
use anyhow::Result;
use chrono::TimeZone;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    io::Write,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc, Mutex},
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/zip-utc-read-1731")
}
fn base() -> PathBuf {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/zip-utc-read-1731");
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn work() -> PathBuf {
    let p = base().join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir(&p).unwrap();
    p
}
fn load(p: &Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}
fn manifest() -> serde_json::Value {
    load(&root().join("manifest.json"))
}
fn sha(p: &Path) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(p).unwrap()))
}
fn seconds(p: &Path) -> i64 {
    filetime::FileTime::from_last_modification_time(&p.metadata().unwrap()).unix_seconds()
}
fn flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}
fn options() -> DecompressOptions {
    DecompressOptions {
        preserve_paths: true,
        preserve_timestamps: true,
        parallel_zip: false,
        conflict_policy: "rename".into(),
        ..Default::default()
    }
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
impl super::native_compression::CompressionRuntime for Runtime {
    fn cancellation_flag(&self) -> Arc<AtomicBool> {
        flag()
    }
    fn check_cancellation(&self) -> Result<()> {
        ExtractionRuntime::check_cancellation(self)
    }
    fn copy_buffer_size(&self) -> usize {
        65536
    }
    fn emit_log(&self, _: &tauri::Window, _: &str, _: &str, _: TaskLogSeverity) {}
    fn emit_progress(&self, _: &tauri::Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {
    }
}
fn stage(
    runtime: &Runtime,
    input: &Path,
    output: &Path,
    opts: &DecompressOptions,
    password: Option<&str>,
) -> tx::ExtractionStaging {
    let stage = tx::ExtractionStaging::create_for(output).unwrap();
    super::native_extraction::zip::extract_with_optional_window(
        runtime,
        None,
        "utcread1731",
        input.to_str().unwrap(),
        stage.path().to_str().unwrap(),
        password,
        opts,
    )
    .unwrap();
    tx::prepare_staging_layout(input, stage.path(), opts).unwrap();
    stage
}
fn commit(
    runtime: &Runtime,
    input: &Path,
    output: &Path,
    opts: &DecompressOptions,
    password: Option<&str>,
) -> super::lzop::selected::DirectoryTimes {
    let mut stage = stage(runtime, input, output, opts, password);
    let times = dates::for_commit(input, stage.path(), opts, || Ok(())).unwrap();
    tx::commit_staged_extraction(input.to_str().unwrap(), stage.path(), output, opts, |_| {})
        .unwrap();
    stage.cleanup().unwrap();
    assert!(super::lzop::selected::apply_directory_times(output, &times).is_empty());
    times
}
fn dos_time(e: &serde_json::Value) -> i64 {
    let d = e["dos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect::<Vec<_>>();
    chrono::Local
        .with_ymd_and_hms(
            d[0] as i32,
            d[1] as u32,
            d[2] as u32,
            d[3] as u32,
            d[4] as u32,
            d[5] as u32,
        )
        .earliest()
        .unwrap()
        .timestamp()
}
fn exact(output: &Path, entries: &[serde_json::Value], fallback: bool) {
    for e in entries {
        let p = output.join(e["name"].as_str().unwrap());
        assert_eq!(
            p.is_dir(),
            e["directory"].as_bool().unwrap_or(false),
            "{}",
            p.display()
        );
        assert_eq!(
            seconds(&p),
            if fallback || e["seconds"].is_null() {
                dos_time(e)
            } else {
                e["seconds"].as_i64().unwrap()
            },
            "{}",
            p.display()
        );
        if p.is_file() {
            assert_eq!(sha(&p), e["sha256"], "{}", p.display());
        }
    }
    assert_eq!(
        walkdir::WalkDir::new(output).into_iter().count() - 1,
        entries.len()
    );
}

#[tokio::test]
async fn utcread1731_foreign_zip_password_and_volume_dates_match_browse_and_final_files() {
    let m = manifest();
    let run = work();
    let mut proof = Vec::new();
    for (index, (name, numeric, standard)) in [
        ("plain", false, false),
        ("aes", false, false),
        ("plain", true, false),
        ("aes", false, true),
    ]
    .into_iter()
    .enumerate()
    {
        let variant = m["originals"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == name)
            .unwrap()
            .clone();
        let source = root().join(variant["archive"].as_str().unwrap());
        assert_eq!(sha(&source), variant["sha256"]);
        let folder = run.join(format!("case-{index}"));
        std::fs::create_dir(&folder).unwrap();
        let mut parts = Vec::new();
        let input = if numeric {
            for (i, bytes) in std::fs::read(&source).unwrap().chunks(65536).enumerate() {
                let p = folder.join(format!("input.zip.{:03}", i + 1));
                std::fs::write(&p, bytes).unwrap();
                parts.push(p);
            }
            parts[0].clone()
        } else if standard {
            let target = folder.join("standard.zip");
            super::standard_zip_writer::split(&source, &target, 65536, || Ok(()), |_, _| {})
                .unwrap();
            parts = super::archive_volumes::complete_volume_paths(&target).unwrap();
            assert!(parts.len() > 1);
            target
        } else {
            source.clone()
        };
        let password = (name == "aes").then_some("UtcRead1731!");
        let listing = archive_browser::browse_archive_cancellable(&input, password, flag())
            .await
            .unwrap();
        for e in variant["entries"].as_array().unwrap() {
            let actual = listing
                .entries
                .iter()
                .find(|a| a.path.trim_end_matches('/') == e["name"].as_str().unwrap())
                .unwrap();
            assert_eq!(
                chrono::DateTime::parse_from_rfc3339(actual.modified.as_deref().unwrap())
                    .unwrap()
                    .timestamp(),
                e["seconds"].as_i64().unwrap()
            );
        }
        let output = folder.join("restored");
        let i = input.clone();
        let o = output.clone();
        let times = tokio::task::spawn_blocking(move || {
            commit(&Runtime::default(), &i, &o, &options(), password)
        })
        .await
        .unwrap();
        assert_eq!(times.len(), 3);
        exact(&output, variant["entries"].as_array().unwrap(), false);
        assert_eq!(sha(&source), variant["sha256"]);
        proof.push(serde_json::json!({"source":source,"sourceSha256":variant["sha256"],"input":input,"parts":parts,"numeric":numeric,"standard":standard,"encrypted":password.is_some(),"entries":variant["entries"],"listing":listing,"output":output,"directoryTimes":times}));
    }
    std::fs::write(
        base().join("native.json"),
        serde_json::to_vec_pretty(&proof).unwrap(),
    )
    .unwrap();
}

#[test]
fn utcread1731_parallel_selected_flat_dates_off_and_newer_conflict_use_one_policy() {
    let m = manifest();
    let run = work();
    let flat = &m["originals"][2];
    let input = root().join(flat["archive"].as_str().unwrap());
    let runtime = Runtime::default();
    let output = run.join("parallel");
    commit(
        &runtime,
        &input,
        &output,
        &DecompressOptions {
            parallel_zip: true,
            ..options()
        },
        None,
    );
    exact(&output, flat["entries"].as_array().unwrap(), false);
    assert!(
        runtime.threads.lock().unwrap().len() > 1,
        "actual worker threads must be observed"
    );
    let source = root().join("plain.zip");
    let expected = m["originals"][0]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == "docs/note.txt")
        .unwrap();
    let mut choices = Vec::new();
    for mode in ["selected", "flat", "dates-off", "newer", "rename"] {
        let output = run.join(mode);
        let mut opts = DecompressOptions {
            selected_entries: vec!["docs/note.txt".into()],
            ..options()
        };
        if mode == "flat" {
            opts.preserve_paths = false;
        }
        if mode == "dates-off" {
            opts.preserve_timestamps = false;
        }
        if mode == "newer" || mode == "rename" {
            std::fs::create_dir_all(output.join("docs")).unwrap();
            std::fs::write(output.join("docs/note.txt"), b"existing").unwrap();
            filetime::set_file_mtime(
                &output.join("docs/note.txt"),
                filetime::FileTime::from_unix_time(1700001730, 0),
            )
            .unwrap();
        }
        if mode == "newer" {
            opts.extract_only_newer = true;
            opts.overwrite_existing = true;
            opts.conflict_policy = "overwrite".into();
        }
        let times = commit(&Runtime::default(), &source, &output, &opts, None);
        assert!(times.is_empty());
        let target = output.join(if mode == "flat" {
            "note.txt"
        } else if mode == "rename" {
            "docs/note (1).txt"
        } else {
            "docs/note.txt"
        });
        assert_eq!(sha(&target), expected["sha256"]);
        if mode == "dates-off" {
            assert_ne!(seconds(&target), 1700001731);
        } else {
            assert_eq!(seconds(&target), 1700001731);
        }
        if mode == "rename" {
            assert_eq!(
                std::fs::read(output.join("docs/note.txt")).unwrap(),
                b"existing"
            );
        }
        choices.push(serde_json::json!({"mode":mode,"output":output,"target":target,"seconds":seconds(&target),"sha256":sha(&target)}));
    }
    let ask_output = run.join("ask");
    std::fs::create_dir_all(ask_output.join("docs")).unwrap();
    std::fs::write(ask_output.join("docs/note.txt"), b"existing").unwrap();
    let ask_opts = DecompressOptions {
        selected_entries: vec!["docs/note.txt".into()],
        conflict_policy: "ask".into(),
        ..options()
    };
    let mut ask_stage = stage(&Runtime::default(), &source, &ask_output, &ask_opts, None);
    let mut conflicts = Vec::new();
    assert!(tx::commit_staged_extraction(
        source.to_str().unwrap(),
        ask_stage.path(),
        &ask_output,
        &ask_opts,
        |c| conflicts.push(c)
    )
    .is_err());
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].source_modified, 1700001731000);
    tx::commit_staged_extraction_with_resolutions(
        source.to_str().unwrap(),
        ask_stage.path(),
        &ask_output,
        &ask_opts,
        &std::collections::HashMap::new(),
        Some("overwrite"),
        |_| {},
    )
    .unwrap();
    ask_stage.cleanup().unwrap();
    let ask_target = ask_output.join("docs/note.txt");
    assert_eq!(seconds(&ask_target), 1700001731);
    assert_eq!(sha(&ask_target), expected["sha256"]);
    choices.push(serde_json::json!({"mode":"ask","output":ask_output,"target":ask_target,"seconds":seconds(&ask_target),"sha256":sha(&ask_target),"conflictSourceMilliseconds":conflicts[0].source_modified}));
    std::fs::write(base().join("choices.json"),serde_json::to_vec_pretty(&serde_json::json!({"parallelOutput":output,"parallelInput":input,"workerThreads":runtime.threads.lock().unwrap().len(),"parallelEntries":flat["entries"],"choices":choices})).unwrap()).unwrap();
}

#[tokio::test]
async fn utcread1731_optional_invalid_duplicate_and_ambiguous_times_fall_back_without_losing_contents(
) {
    let m = manifest();
    let run = work();
    let mut proof = Vec::new();
    for (name, entries, fallback, password) in [
        (
            "controls.zip",
            m["controls"]["entries"].clone(),
            false,
            None,
        ),
        (
            "aes-malformed-ut.zip",
            m["originals"][1]["entries"].clone(),
            true,
            Some("UtcRead1731!"),
        ),
    ] {
        let source = root().join(name);
        let before = sha(&source);
        let listing = archive_browser::browse_archive_cancellable(&source, password, flag())
            .await
            .unwrap();
        for e in entries.as_array().unwrap() {
            let a = listing
                .entries
                .iter()
                .find(|a| a.path.trim_end_matches('/') == e["name"].as_str().unwrap())
                .unwrap();
            if !fallback && !e["seconds"].is_null() {
                assert_eq!(
                    chrono::DateTime::parse_from_rfc3339(a.modified.as_deref().unwrap())
                        .unwrap()
                        .timestamp(),
                    e["seconds"].as_i64().unwrap()
                );
            } else {
                assert!(!a.modified.as_deref().unwrap().contains('+'));
            }
        }
        let output = run.join(name);
        let i = source.clone();
        let o = output.clone();
        tokio::task::spawn_blocking(move || {
            commit(&Runtime::default(), &i, &o, &options(), password)
        })
        .await
        .unwrap();
        exact(&output, entries.as_array().unwrap(), fallback);
        assert_eq!(sha(&source), before);
        proof.push(serde_json::json!({"source":source,"sourceSha256":before,"entries":entries,"listing":listing,"output":output,"fallbackAll":fallback,"encrypted":password.is_some()}));
    }
    std::fs::write(
        base().join("controls.json"),
        serde_json::to_vec_pretty(&proof).unwrap(),
    )
    .unwrap();
    // Identical UT instants remain identical under display-zone conversions;
    // never mutate the user's Windows timezone for this check.
    for offset in [-12 * 3600, 0, 8 * 3600, 14 * 3600] {
        let utc = chrono::DateTime::<chrono::Utc>::from_timestamp(1700001731, 0).unwrap();
        assert_eq!(
            utc.with_timezone(&chrono::FixedOffset::east_opt(offset).unwrap())
                .timestamp(),
            1700001731
        );
    }
    assert!(zip_read_time::utc_modified(Some(&[0x55, 0x54, 0xff, 0xff, 1, 0, 0, 0, 0])).is_none());
}

#[tokio::test]
async fn utcread1731_zipx_xz_aes_uses_utc_instead_of_the_distinct_dos_calendar() {
    let run = work();
    let source = run.join("utc.zipx");
    let mut writer = zip_aes::ZipWriter::new(std::fs::File::create(&source).unwrap());
    let mut opts = zip_aes::write::FullFileOptions::default()
        .compression_method(zip_aes::CompressionMethod::Xz)
        .compression_level(Some(1))
        .last_modified_time(zip_aes::DateTime::from_date_and_time(2001, 2, 3, 4, 5, 6).unwrap())
        .with_aes_encryption(zip_aes::AesMode::Aes256, "UtcRead1731!");
    let mut ut = vec![1];
    ut.extend_from_slice(&1700001731u32.to_le_bytes());
    opts.add_extra_data(0x5455, ut, false).unwrap();
    writer.start_file("note.txt", opts).unwrap();
    writer.write_all(b"ZIPX XZ UTC source 1731\n").unwrap();
    writer.finish().unwrap();
    let expected = 1700001731i64;
    let listing =
        archive_browser::browse_archive_cancellable(&source, Some("UtcRead1731!"), flag())
            .await
            .unwrap();
    assert_eq!(
        chrono::DateTime::parse_from_rfc3339(listing.entries[0].modified.as_deref().unwrap())
            .unwrap()
            .timestamp(),
        expected
    );
    let output = run.join("restored");
    let i = source.clone();
    let o = output.clone();
    tokio::task::spawn_blocking(move || {
        commit(
            &Runtime::default(),
            &i,
            &o,
            &options(),
            Some("UtcRead1731!"),
        )
    })
    .await
    .unwrap();
    assert_eq!(seconds(&output.join("note.txt")), expected);
    assert_eq!(
        std::fs::read(output.join("note.txt")).unwrap(),
        b"ZIPX XZ UTC source 1731\n"
    );
    std::fs::write(base().join("zipx.json"),serde_json::to_vec_pretty(&serde_json::json!({"source":source,"sourceSha256":sha(&source),"producer":"test-only vendored ZIP writer, XZ/AES with declared UT metadata; not an external original","output":output,"seconds":expected,"listing":listing})).unwrap()).unwrap();
}

#[test]
fn utcread1731_conversion_staging_carries_utc_file_and_directory_dates_to_tar_and_sevenz() {
    let m = manifest();
    let source = root().join("plain.zip");
    let before = sha(&source);
    let run = work();
    let staging = run.join("contents");
    commit(&Runtime::default(), &source, &staging, &options(), None);
    exact(
        &staging,
        m["originals"][0]["entries"].as_array().unwrap(),
        false,
    );
    let files = vec![staging.join("docs").to_string_lossy().into_owned()];
    let mut proof = Vec::new();
    for format in ["tar.gz", "7z"] {
        let output = run.join(format!("converted.{format}"));
        let opts = CompressionOptions {
            format: Some(format.into()),
            level: 1,
            verify_after: true,
            preserve_paths: Some(true),
            password: (format == "7z").then(|| "UtcRead1731!".into()),
            encrypt_filenames: format == "7z",
            ..Default::default()
        };
        let result = if format == "7z" {
            super::native_compression::seven_zip::compress(
                &Runtime::default(),
                None,
                "utc-convert",
                &files,
                output.to_str().unwrap(),
                opts.clone(),
            )
        } else {
            super::native_compression::tar::compress_gzip(
                &Runtime::default(),
                None,
                "utc-convert",
                &files,
                output.to_str().unwrap(),
                opts.clone(),
            )
        };
        result.unwrap();
        assert!(
            super::compression_verification::verify_native_with_zip_encoding(
                super::compression_format::compression_route(format).unwrap(),
                &output,
                opts.password.as_deref(),
                None,
                || false
            )
            .unwrap()
        );
        assert_eq!(sha(&source), before);
        proof.push(serde_json::json!({"source":source,"sourceSha256":before,"inputContents":staging,"output":output,"format":format,"encrypted":format=="7z","entries":m["originals"][0]["entries"]}));
    }
    std::fs::write(
        base().join("conversion.json"),
        serde_json::to_vec_pretty(&proof).unwrap(),
    )
    .unwrap();
}

#[tokio::test]
async fn utcread1731_password_fallback_and_comparison_rows_keep_the_same_instants() {
    use super::archive_engine::ArchiveEngine;
    let m = manifest();
    let run = work();
    let source = root().join("aes.zip");
    let output = run.join("password-fallback");
    std::fs::create_dir(&output).unwrap();
    super::universal_engine::UniversalCliEngine::new()
        .extract_with_progress(
            &source,
            &output,
            Some("UtcRead1731!"),
            false,
            Arc::new(|_| {}),
            Arc::new(|_, _| {}),
            flag(),
        )
        .await
        .unwrap();
    for e in m["originals"][1]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["directory"] != true)
    {
        let p = output.join(e["name"].as_str().unwrap());
        assert_eq!(seconds(&p), e["seconds"].as_i64().unwrap());
        assert_eq!(sha(&p), e["sha256"]);
    }
    let comparison =
        super::source_comparison::read(&root().join("plain.zip"), flag(), |_| {}).unwrap();
    for e in m["originals"][0]["entries"].as_array().unwrap() {
        let actual = comparison
            .entries
            .iter()
            .find(|a| a.path == e["name"].as_str().unwrap())
            .unwrap();
        let expected =
            chrono::DateTime::<chrono::Utc>::from_timestamp(e["seconds"].as_i64().unwrap(), 0)
                .unwrap()
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();
        assert_eq!(actual.modified.as_deref(), Some(expected.as_str()));
    }
    std::fs::write(base().join("consistency.json"),serde_json::to_vec_pretty(&serde_json::json!({"source":source,"sourceSha256":sha(&source),"passwordFallbackOutput":output,"entries":m["originals"][1]["entries"],"comparison":comparison})).unwrap()).unwrap();
}
