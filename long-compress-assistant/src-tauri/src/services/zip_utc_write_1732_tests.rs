use super::{
    compression_entries, extraction_transaction as tx, native_extraction::ExtractionRuntime,
    standard_zip_writer::ZipSplitMode, zip_method::ZipMethod, zip_password::ZipEncryption,
};
use crate::models::compression::{CompressionOptions, DecompressOptions, TaskLogSeverity};
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::{Duration, UNIX_EPOCH},
};

fn base() -> PathBuf {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/zip-utc-write-1732");
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn work() -> PathBuf {
    let p = base().join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir(&p).unwrap();
    p
}
fn sha(p: &Path) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(p).unwrap()))
}
fn stamp(p: &Path, seconds: i64, nanos: u32) {
    filetime::set_file_mtime(p, filetime::FileTime::from_unix_time(seconds, nanos)).unwrap();
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
        Arc::new(AtomicBool::new(false))
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
fn options() -> DecompressOptions {
    DecompressOptions {
        preserve_paths: true,
        preserve_timestamps: true,
        parallel_zip: false,
        conflict_policy: "rename".into(),
        ..Default::default()
    }
}
fn sources(folder: &Path) -> Vec<String> {
    let docs = folder.join("资料");
    std::fs::create_dir_all(docs.join("空目录")).unwrap();
    std::fs::create_dir_all(folder.join("empty-root")).unwrap();
    let mut seed = 1732u32;
    let data = (0..340000)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed as u8
        })
        .collect::<Vec<_>>();
    for (name, seconds, body) in [
        ("a.bin", 1700001733, data.clone()),
        ("b.bin", 1710054135, data.into_iter().rev().collect()),
        (
            "note.txt",
            1700001835,
            "创建和取回均保留 UTC 奇数秒。\n".as_bytes().to_vec(),
        ),
        ("zero.dat", 1700001937, vec![]),
    ] {
        let p = docs.join(name);
        std::fs::write(&p, body).unwrap();
        stamp(&p, seconds, 123456700);
    }
    stamp(&docs.join("空目录"), 1700001531, 123456700);
    stamp(&docs, 1700001553, 123456700);
    stamp(&folder.join("empty-root"), 1700001565, 123456700);
    vec![
        docs.to_string_lossy().into_owned(),
        folder.join("empty-root").to_string_lossy().into_owned(),
    ]
}
fn records(files: &[String], preserve: bool) -> Vec<serde_json::Value> {
    compression_entries::collect_zip(files,preserve).unwrap().into_iter().map(|e|{let t=filetime::FileTime::from_last_modification_time(&e.path.metadata().unwrap());let s=t.unix_seconds();serde_json::json!({"name":e.archive_name,"path":e.path,"directory":e.is_dir,"seconds":s,"nanos":t.nanoseconds(),"utcExpected":(0..=i32::MAX as i64).contains(&s),"sha256":if e.is_dir{None}else{Some(sha(&e.path))}})}).collect()
}
fn create(files: &[String], output: &Path, config: &CompressionOptions) {
    super::native_compression::zip::compress(
        &Runtime::default(),
        None,
        "utcwrite1732",
        files,
        output.to_str().unwrap(),
        config.clone(),
    )
    .unwrap();
}
fn primary(output: &Path, config: &CompressionOptions) -> PathBuf {
    if config.split_size.is_some() && config.zip_split_mode == ZipSplitMode::Numeric {
        output.with_file_name(format!(
            "{}.001",
            output.file_name().unwrap().to_string_lossy()
        ))
    } else {
        output.into()
    }
}
fn verify(input: &Path, password: Option<&str>) {
    assert!(
        super::compression_verification::verify_native_with_zip_encoding(
            super::compression_format::CompressionRoute::Zip,
            input,
            password,
            None,
            || false
        )
        .unwrap()
    );
}
fn restore(
    runtime: &Runtime,
    input: &Path,
    output: &Path,
    opts: &DecompressOptions,
    password: Option<&str>,
) {
    let mut stage = tx::ExtractionStaging::create_for(output).unwrap();
    super::native_extraction::zip::extract_with_optional_window(
        runtime,
        None,
        "utcwrite1732",
        input.to_str().unwrap(),
        stage.path().to_str().unwrap(),
        password,
        opts,
    )
    .unwrap();
    tx::prepare_staging_layout(input, stage.path(), opts).unwrap();
    let times =
        super::zip_directory_times::for_commit(input, stage.path(), opts, || Ok(())).unwrap();
    tx::commit_staged_extraction(input.to_str().unwrap(), stage.path(), output, opts, |_| {})
        .unwrap();
    stage.cleanup().unwrap();
    assert!(super::lzop::selected::apply_directory_times(output, &times).is_empty());
}
fn exact(output: &Path, entries: &[serde_json::Value]) {
    for e in entries {
        let p = output.join(e["name"].as_str().unwrap());
        assert_eq!(p.is_dir(), e["directory"].as_bool().unwrap());
        if e["utcExpected"] == true {
            let t = filetime::FileTime::from_last_modification_time(&p.metadata().unwrap());
            assert_eq!(
                t.unix_seconds(),
                e["seconds"].as_i64().unwrap(),
                "{}",
                p.display()
            );
            assert_eq!(t.nanoseconds(), 0);
        }
        if p.is_file() {
            assert_eq!(sha(&p), e["sha256"], "{}", p.display());
        }
    }
    assert_eq!(
        walkdir::WalkDir::new(output).into_iter().count() - 1,
        entries.len()
    );
}
fn inspect(input: &Path, expected: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut zip = zip_aes::ZipArchive::new(
        super::numeric_volume_reader::ZipInputReader::open(input).unwrap(),
    )
    .unwrap();
    (0..zip.len()).map(|i|{let e=zip.by_index_raw(i).unwrap();let want=expected.iter().find(|s|s["name"].as_str().unwrap()==e.name().trim_end_matches('/')).unwrap();let actual=super::zip_read_time::utc_modified(e.extra_data()).map(|t|t.duration_since(UNIX_EPOCH).unwrap().as_secs());assert_eq!(actual,if want["utcExpected"]==true{Some(want["seconds"].as_u64().unwrap())}else{None});let d=e.last_modified().unwrap();serde_json::json!({"name":e.name(),"utcSeconds":actual,"dos":[d.year(),d.month() as u16,d.day() as u16,d.hour() as u16,d.minute() as u16,d.second() as u16],"method":format!("{:?}",e.compression())})}).collect()
}
fn receipt(
    name: &str,
    input: &Path,
    output: &Path,
    config: &CompressionOptions,
    expected: &[serde_json::Value],
    restored: Vec<PathBuf>,
) -> serde_json::Value {
    let parts = if config.split_size.is_some() {
        super::archive_volumes::complete_volume_paths(output).unwrap()
    } else {
        vec![output.into()]
    };
    serde_json::json!({"name":name,"input":input,"output":output,"parts":parts,"numeric":config.split_size.is_some()&&config.zip_split_mode==ZipSplitMode::Numeric,"encrypted":config.password.is_some(),"sources":expected,"headers":inspect(input,expected),"restored":restored})
}

#[test]
fn utcwrite1732_confirmed_range_subseconds_and_dos_fallback_are_encoded_explicitly() {
    let run = work();
    let source = run.join("source");
    std::fs::create_dir(&source).unwrap();
    let mut files = Vec::new();
    for (name, s, n) in [
        ("before", -1, 0),
        ("epoch", 0, 0),
        ("odd", 1700001733, 123456700),
        ("fractional", 1, 999999900),
        ("max", i32::MAX as i64, 999999900),
        ("after", i32::MAX as i64 + 1, 0),
        ("clamped", 4417632001, 0),
    ] {
        let p = source.join(format!("{name}.txt"));
        std::fs::write(&p, name).unwrap();
        stamp(&p, s, n);
        files.push(p.to_string_lossy().into_owned());
        let time = if s < 0 {
            UNIX_EPOCH - Duration::from_secs((-s) as u64)
        } else {
            UNIX_EPOCH + Duration::new(s as u64, n)
        };
        assert_eq!(
            super::zip_creation_time::from_system_time(time).utc_seconds,
            if (0..=i32::MAX as i64).contains(&s) {
                Some(s as u32)
            } else {
                None
            }
        );
    }
    let expected = records(&files, true);
    let output = run.join("bounds.zip");
    let config = CompressionOptions {
        format: Some("zip".into()),
        zip_method: ZipMethod::Store,
        ..Default::default()
    };
    create(&files, &output, &config);
    verify(&output, None);
    let restored = run.join("restored");
    restore(&Runtime::default(), &output, &restored, &options(), None);
    exact(&restored, &expected);
    assert_eq!(records(&files, true), expected);
    std::fs::write(
        base().join("bounds.json"),
        serde_json::to_vec_pretty(&receipt(
            "bounds",
            &output,
            &output,
            &config,
            &expected,
            vec![restored],
        ))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn utcwrite1732_zip_zipx_password_and_real_volume_paths_keep_each_file_and_directory_utc() {
    let run = work();
    let files = sources(&run.join("source"));
    let expected = records(&files, true);
    let mut proof = Vec::new();
    for (name, format, method, encryption, split) in [
        ("plain", "zip", ZipMethod::Deflate, None, None),
        (
            "aes-standard",
            "zip",
            ZipMethod::Store,
            Some(ZipEncryption::Aes256),
            Some(ZipSplitMode::Standard),
        ),
        (
            "legacy-numeric",
            "zip",
            ZipMethod::Deflate,
            Some(ZipEncryption::Zipcrypto),
            Some(ZipSplitMode::Numeric),
        ),
        (
            "xz-aes-standard",
            "zipx",
            ZipMethod::Deflate,
            Some(ZipEncryption::Aes256),
            Some(ZipSplitMode::Standard),
        ),
    ] {
        let output = run.join(format!("{name}.{format}"));
        let config = CompressionOptions {
            format: Some(format.into()),
            level: 1,
            zip_method: method,
            password: encryption.map(|_| "UtcWrite1732!".into()),
            zip_encryption: encryption.unwrap_or_default(),
            split_size: split.map(|_| 65536),
            zip_split_mode: split.unwrap_or_default(),
            verify_after: true,
            preserve_paths: Some(true),
            ..Default::default()
        };
        create(&files, &output, &config);
        let input = primary(&output, &config);
        verify(&input, config.password.as_deref());
        if split.is_some() {
            assert!(
                super::archive_volumes::complete_volume_paths(&output)
                    .unwrap()
                    .len()
                    > 1
            );
        }
        let restored = run.join(format!("from-{name}"));
        restore(
            &Runtime::default(),
            &input,
            &restored,
            &options(),
            config.password.as_deref(),
        );
        exact(&restored, &expected);
        assert_eq!(records(&files, true), expected);
        proof.push(receipt(
            name,
            &input,
            &output,
            &config,
            &expected,
            vec![restored],
        ));
    }
    std::fs::write(
        base().join("native.json"),
        serde_json::to_vec_pretty(&proof).unwrap(),
    )
    .unwrap();
}

#[test]
fn utcwrite1732_actual_parallel_and_flat_renamed_sources_retain_individual_utc_seconds() {
    let run = work();
    let all = sources(&run.join("source"));
    let docs = Path::new(&all[0]);
    let files = vec![
        docs.join("a.bin").to_string_lossy().into_owned(),
        docs.join("b.bin").to_string_lossy().into_owned(),
    ];
    let expected = records(&files, true);
    let output = run.join("parallel.zip");
    let config = CompressionOptions {
        format: Some("zip".into()),
        ..Default::default()
    };
    create(&files, &output, &config);
    verify(&output, None);
    let mut restored = Vec::new();
    let mut workers = 0;
    for parallel in [false, true] {
        let runtime = Runtime::default();
        let target = run.join(if parallel { "parallel" } else { "sequential" });
        restore(
            &runtime,
            &output,
            &target,
            &DecompressOptions {
                parallel_zip: parallel,
                ..options()
            },
            None,
        );
        exact(&target, &expected);
        if parallel {
            workers = runtime.threads.lock().unwrap().len();
            assert!(workers > 1);
        }
        restored.push(target);
    }
    let mut proof = vec![receipt(
        "actual-parallel",
        &output,
        &output,
        &config,
        &expected,
        restored,
    )];
    for (name, s) in [("first", 1700001733), ("second", 1700001835)] {
        let p = run.join(name).join("同名.txt");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, name).unwrap();
        stamp(&p, s, 123456700);
    }
    let files = ["first", "second"]
        .iter()
        .map(|n| run.join(n).join("同名.txt").to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let expected = records(&files, false);
    let config = CompressionOptions {
        format: Some("zip".into()),
        preserve_paths: Some(false),
        ..Default::default()
    };
    let output = run.join("flat.zip");
    create(&files, &output, &config);
    verify(&output, None);
    let target = run.join("flat");
    restore(&Runtime::default(), &output, &target, &options(), None);
    exact(&target, &expected);
    assert_eq!(records(&files, false), expected);
    proof.push(receipt(
        "flat-renamed",
        &output,
        &output,
        &config,
        &expected,
        vec![target],
    ));
    std::fs::write(
        base().join("choices.json"),
        serde_json::to_vec_pretty(&serde_json::json!({"workers":workers,"cases":proof})).unwrap(),
    )
    .unwrap();
}

#[test]
fn utcwrite1732_conversion_from_real_utc_zip_publishes_zip_and_zipx_with_original_seconds() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/archives/zip-utc-read-1731/plain.zip");
    let source_hash = sha(&source);
    let run = work();
    let mut proof = Vec::new();
    for format in ["zip", "zipx"] {
        let temp = tempfile::Builder::new()
            .prefix(".long-convert-")
            .tempdir_in(&run)
            .unwrap();
        let contents = temp.path().join("contents");
        restore(&Runtime::default(), &source, &contents, &options(), None);
        let files = std::fs::read_dir(&contents)
            .unwrap()
            .map(|e| e.unwrap().path().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let expected = records(&files, true);
        assert!(expected.iter().all(|e| e["utcExpected"] == true));
        let output = run.join(format!("converted.{format}"));
        let staged =
            super::output_publish_transaction::staged_output_path(&output, "long-convert").unwrap();
        let config = CompressionOptions {
            format: Some(format.into()),
            level: 1,
            verify_after: true,
            preserve_paths: Some(true),
            password: (format == "zip").then(|| "UtcWrite1732!".into()),
            ..Default::default()
        };
        create(&files, &staged, &config);
        verify(&staged, config.password.as_deref());
        assert!(!output.exists());
        super::output_publish_transaction::publish_verified_file(&staged, &output, || false)
            .unwrap();
        let published = serde_json::to_value(
            crate::commands::compression::published_archive_output(&output, false).unwrap(),
        )
        .unwrap();
        assert!(published["files"][0]["bytes"].as_u64().unwrap() > 0);
        let target = run.join(format!("restored-{format}"));
        restore(
            &Runtime::default(),
            &output,
            &target,
            &options(),
            config.password.as_deref(),
        );
        exact(&target, &expected);
        assert_eq!(sha(&source), source_hash);
        let mut value = receipt(
            "conversion",
            &output,
            &output,
            &config,
            &expected,
            vec![target],
        );
        value["originalSource"] = serde_json::json!(source);
        value["originalSourceSha256"] = serde_json::json!(source_hash);
        value["published"] = published;
        let private = temp.path().to_path_buf();
        drop(temp);
        assert!(!private.exists());
        assert!(!staged.exists());
        value["privateFilesCleaned"] = true.into();
        proof.push(value);
    }
    std::fs::write(
        base().join("conversion.json"),
        serde_json::to_vec_pretty(&proof).unwrap(),
    )
    .unwrap();
}
