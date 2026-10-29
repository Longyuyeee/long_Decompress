use super::{
    native_extraction::ExtractionRuntime, tar_link_conversion::Prepared,
    tar_lzo_prefix_175_tests::wrapped,
};
use crate::models::compression::{DecompressOptions, TaskLogSeverity};
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives")
}
fn base() -> PathBuf {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/tar-forward-convert-1729");
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}
fn json(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn sha(path: &Path) -> String {
    let mut file = std::fs::File::open(path).unwrap();
    let mut buf = [0; 65536];
    let mut h = Sha256::new();
    loop {
        let n = file.read(&mut buf).unwrap();
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    format!("{:x}", h.finalize())
}
fn options(names: Vec<String>) -> DecompressOptions {
    DecompressOptions {
        selected_entries: names,
        preserve_paths: true,
        preserve_timestamps: true,
        conflict_policy: "rename".into(),
        ..Default::default()
    }
}
struct Runtime {
    pool: super::io_buffer_pool::IOBufferPool,
    flag: Arc<AtomicBool>,
    calls: AtomicUsize,
    hook: Arc<dyn Fn(usize) -> Result<()> + Send + Sync>,
}
impl Runtime {
    fn new(flag: Arc<AtomicBool>) -> Self {
        Self {
            pool: super::io_buffer_pool::IOBufferPool::new(Default::default()),
            flag,
            calls: AtomicUsize::new(0),
            hook: Arc::new(|_| Ok(())),
        }
    }
}
impl ExtractionRuntime for Runtime {
    fn check_cancellation(&self) -> Result<()> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        (self.hook)(n)?;
        anyhow::ensure!(!self.flag.load(Ordering::SeqCst), "test cancelled");
        Ok(())
    }
    fn buffer_pool(&self) -> &super::io_buffer_pool::IOBufferPool {
        &self.pool
    }
    fn copy_buffer_size(&self) -> usize {
        65536
    }
    fn normalized_archive_path(&self, path: &Path, preserve: bool) -> Option<PathBuf> {
        if preserve {
            Some(path.to_path_buf())
        } else {
            path.file_name().map(PathBuf::from)
        }
    }
    fn emit_log(&self, _: &tauri::Window, _: &str, _: &str, _: TaskLogSeverity) {}
    fn emit_progress(&self, _: &tauri::Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {
    }
}
impl super::native_compression::CompressionRuntime for Runtime {
    fn check_cancellation(&self) -> Result<()> {
        ExtractionRuntime::check_cancellation(self)
    }
    fn cancellation_flag(&self) -> Arc<AtomicBool> {
        self.flag.clone()
    }
    fn copy_buffer_size(&self) -> usize {
        65536
    }
    fn emit_log(&self, _: &tauri::Window, _: &str, _: &str, _: TaskLogSeverity) {}
    fn emit_progress(&self, _: &tauri::Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {
    }
}
fn original(folder: &str, file: &str) -> (PathBuf, serde_json::Value) {
    let path = root().join(folder).join(file);
    let m = json(&root().join(folder).join("manifest.json"));
    let variant = m["variants"]
        .as_array()
        .and_then(|v| v.iter().find(|v| v["archive"] == file))
        .unwrap_or(&m)
        .clone();
    (path, variant)
}
fn build(
    runtime: &Runtime,
    files: &[String],
    output: &Path,
    opts: &crate::models::compression::CompressionOptions,
) {
    use super::native_compression as n;
    match opts.format.as_deref().unwrap() {
        "zip" | "zipx" => n::zip::compress(
            runtime,
            None,
            "convert1729",
            files,
            &output.to_string_lossy(),
            opts.clone(),
        )
        .unwrap(),
        "7z" => n::seven_zip::compress(
            runtime,
            None,
            "convert1729",
            files,
            &output.to_string_lossy(),
            opts.clone(),
        )
        .unwrap(),
        "tar" => n::tar::compress_tar(
            runtime,
            None,
            "convert1729",
            files,
            &output.to_string_lossy(),
            opts.clone(),
        )
        .unwrap(),
        "tar.gz" => n::tar::compress_gzip(
            runtime,
            None,
            "convert1729",
            files,
            &output.to_string_lossy(),
            opts.clone(),
        )
        .unwrap(),
        _ => unreachable!(),
    }
}
fn verify(path: &Path, opts: &crate::models::compression::CompressionOptions) -> Result<bool> {
    let route =
        super::compression_format::compression_route(opts.format.as_deref().unwrap()).unwrap();
    let primary = if opts.split_size.is_some() {
        super::split_archive_detector::SplitArchiveDetector::complete_source(
            &super::archive_volumes::complete_volume_paths(path)?[0],
        )?
    } else {
        path.into()
    };
    super::compression_verification::verify_native_with_zip_encoding(
        route,
        &primary,
        opts.password.as_deref(),
        Some(opts.zip_password_encoding),
        || false,
    )
}
fn exact_readback(restored: &Path, entries: &serde_json::Value) {
    use std::collections::BTreeSet;
    let mut names = BTreeSet::new();
    for e in entries.as_array().unwrap() {
        let name = e["name"].as_str().unwrap();
        names.insert(name.to_string());
        names.extend(
            Path::new(name)
                .ancestors()
                .skip(1)
                .filter(|p| !p.as_os_str().is_empty())
                .map(|p| p.to_string_lossy().replace('\\', "/")),
        );
        let path = restored.join(name);
        assert_eq!(path.is_dir(), e["isDirectory"].as_bool().unwrap(), "{name}");
        assert_eq!(
            filetime::FileTime::from_last_modification_time(&path.metadata().unwrap())
                .unix_seconds(),
            e["seconds"].as_i64().unwrap(),
            "{name}"
        );
        if !path.is_dir() {
            assert_eq!(path.metadata().unwrap().len(), e["bytes"].as_u64().unwrap());
            assert_eq!(sha(&path), e["sha256"], "{name}");
        }
    }
    let actual = walkdir::WalkDir::new(restored)
        .into_iter()
        .map(Result::unwrap)
        .filter(|e| e.path() != restored)
        .map(|e| {
            e.path()
                .strip_prefix(restored)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, names);
}
fn readback(input: &Path, output: &Path, format: &str, password: Option<&str>) {
    use super::{extraction_transaction as tx, lzop::selected, native_extraction as native};
    let opts = options(vec![]);
    let runtime = Runtime::new(flag());
    let mut stage = tx::ExtractionStaging::create_for(output).unwrap();
    let times = match format {
        "zip" | "zipx" => {
            native::zip::extract_with_optional_window(
                &runtime,
                None,
                "convert1729",
                input.to_str().unwrap(),
                stage.path().to_str().unwrap(),
                password,
                &opts,
            )
            .unwrap();
            tx::prepare_staging_layout(input, stage.path(), &opts).unwrap();
            super::zip_directory_times::for_commit(input, stage.path(), &opts, || Ok(())).unwrap()
        }
        "7z" => {
            let times = native::seven_zip::extract_with_directory_times(
                &runtime,
                None,
                "convert1729",
                input.to_str().unwrap(),
                stage.path().to_str().unwrap(),
                password,
                &opts,
            )
            .unwrap();
            tx::prepare_staging_layout(input, stage.path(), &opts).unwrap();
            super::seven_zip_directory_times::retained(&times, stage.path(), &opts)
        }
        "tar" | "tar.gz" => {
            let route = super::compression_format::compression_route(format).unwrap();
            let plan = super::tar_compatibility::plan(input, route, &opts, || Ok(())).unwrap();
            let decoder = if format == "tar" {
                None
            } else {
                Some(super::codec_stream::decoder(input, "gz").unwrap())
            };
            native::tar::extract_inner(
                &runtime,
                None,
                "convert1729",
                input.to_str().unwrap(),
                stage.path(),
                decoder,
                &opts,
            )
            .unwrap();
            tx::prepare_staging_layout_with_tar_plan(input, stage.path(), &opts, Some(&plan))
                .unwrap();
            super::tar_directory_times::retained(&plan.directory_times, stage.path(), &opts)
        }
        _ => unreachable!(),
    };
    tx::commit_staged_extraction(input.to_str().unwrap(), stage.path(), output, &opts, |_| {})
        .unwrap();
    stage.cleanup().unwrap();
    assert!(selected::apply_directory_times(output, &times).is_empty());
}
#[test]
fn convert1729_forward_contents_keep_roots_dates_passwords_and_split_results() {
    use super::{
        standard_zip_writer::ZipSplitMode, zip_method::ZipMethod, zip_password::ZipEncryption,
    };
    use crate::models::compression::CompressionOptions;
    let run = base().join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&run).unwrap();
    let mut cases = Vec::new();
    for (i, (variant, format, encrypted, split, standard, numeric_input)) in [
        ("ustar", "zip", false, false, false, false),
        ("gnu", "zip", true, true, true, false),
        ("pax", "zip", true, true, false, true),
        ("gnu", "zipx", true, false, false, false),
        ("pax", "7z", true, true, false, true),
        ("gnu", "tar", false, false, false, false),
        ("pax", "tar.gz", false, false, false, false),
    ]
    .into_iter()
    .enumerate()
    {
        let dir = run.join(format!("case-{i}"));
        std::fs::create_dir(&dir).unwrap();
        let (source, m) = original("tar-forward-links-1728", &format!("{variant}.tar.lzo"));
        let source_hash = sha(&source);
        assert_eq!(source_hash, m["sha256"]);
        let mut parts = Vec::new();
        if numeric_input {
            for (n, bytes) in std::fs::read(&source).unwrap().chunks(2048).enumerate() {
                let path = dir.join(format!("source.tar.lzo.{:03}", n + 1));
                std::fs::write(&path, bytes).unwrap();
                parts.push(path);
            }
        }
        let read_source = parts.last().unwrap_or(&source).clone();
        let output = dir.join(format!("converted.{format}"));
        let cancel = flag();
        let runtime = Runtime::new(cancel.clone());
        let prepared = Prepared::open(&read_source, &output, options(vec![]), cancel)
            .unwrap()
            .unwrap();
        let scratch = prepared.contents().to_path_buf();
        let rebuilt = prepared.rebuilt().to_path_buf();
        let files = prepared.read(&runtime, None, "convert1729").unwrap();
        exact_readback(&scratch, &m["entries"]);
        let config = CompressionOptions {
            format: Some(format.into()),
            level: 1,
            verify_after: true,
            preserve_paths: Some(true),
            password: encrypted.then(|| "ForwardConvert1729!".into()),
            encrypt_filenames: encrypted && format == "7z",
            zip_encryption: if i == 2 {
                ZipEncryption::Zipcrypto
            } else {
                ZipEncryption::Aes256
            },
            zip_method: if standard {
                ZipMethod::Store
            } else {
                ZipMethod::Deflate
            },
            split_size: split.then_some(if standard { 262144 } else { 1024 }),
            zip_split_mode: if standard {
                ZipSplitMode::Standard
            } else {
                ZipSplitMode::Numeric
            },
            ..Default::default()
        };
        super::compression_service::CompressionService::validate_compression_request(
            &files,
            rebuilt.to_str().unwrap(),
            &config,
        )
        .unwrap();
        prepared.check_source().unwrap();
        build(&runtime, &files, &rebuilt, &config);
        assert!(verify(&rebuilt, &config).unwrap());
        assert!(!output.exists());
        prepared.publish_verified(split).unwrap();
        let receipt = serde_json::to_value(
            crate::commands::compression::published_archive_output(&output, split).unwrap(),
        )
        .unwrap();
        for part in receipt["files"].as_array().unwrap() {
            let path = Path::new(part["path"].as_str().unwrap());
            assert_eq!(
                path.metadata().unwrap().len(),
                part["bytes"].as_u64().unwrap()
            );
        }
        if split {
            assert!(receipt["files"].as_array().unwrap().len() > 1);
        }
        let restored = dir.join("native-restored");
        readback(
            Path::new(receipt["primaryPath"].as_str().unwrap()),
            &restored,
            format,
            config.password.as_deref(),
        );
        exact_readback(&restored, &m["entries"]);
        assert_eq!(sha(&source), source_hash);
        drop(prepared);
        assert!(!scratch.exists());
        assert!(!rebuilt.exists());
        assert!(!std::fs::read_dir(&dir).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".long-")));
        cases.push(serde_json::json!({"variant":variant,"source":source,"readSource":read_source,"inputParts":parts,"sourceSha256":source_hash,"format":format,"encrypted":encrypted,"zipEncryption":if i==2{"zipcrypto"}else{"aes256"},"split":split,"standard":standard,"output":receipt,"nativeRestored":restored,"entries":m["entries"],"privateFilesCleaned":true}));
    }
    std::fs::write(
        base().join("native.json"),
        serde_json::to_vec_pretty(&cases).unwrap(),
    )
    .unwrap();
}
#[test]
fn convert1729_source_changed_before_writing_or_publishing_cannot_become_a_completed_result() {
    use crate::models::compression::CompressionOptions;
    let temp = tempfile::tempdir().unwrap();
    let original = original("tar-forward-links-1728", "gnu.tar.lzo").0;
    for phase in ["reading", "writing"] {
        let source = temp.path().join(format!("{phase}.tar.lzo"));
        std::fs::copy(&original, &source).unwrap();
        let before = sha(&source);
        let output = temp.path().join(format!("{phase}.zip"));
        let cancel = flag();
        let prepared = Prepared::open(&source, &output, options(vec![]), cancel.clone())
            .unwrap()
            .unwrap();
        let scratch = prepared.contents().to_owned();
        let rebuilt = prepared.rebuilt().to_owned();
        let files = prepared
            .read(&Runtime::new(cancel.clone()), None, phase)
            .unwrap();
        if phase == "writing" {
            let opts = CompressionOptions {
                format: Some("zip".into()),
                ..Default::default()
            };
            build(&Runtime::new(cancel.clone()), &files, &rebuilt, &opts);
            verify(&rebuilt, &opts).unwrap();
        }
        filetime::set_file_mtime(&source, filetime::FileTime::from_unix_time(1900001729, 0))
            .unwrap();
        let error = if phase == "reading" {
            prepared.check_source().unwrap_err()
        } else {
            prepared.publish_verified(false).unwrap_err()
        };
        let text = super::archive_io::message(&error);
        assert!(
            text.contains("[archive-source:") && text.contains("source-changed"),
            "{text}"
        );
        assert!(!text.contains("cancelled"));
        assert!(!output.exists());
        drop(prepared);
        assert!(!scratch.exists());
        assert!(!rebuilt.exists());
        assert_eq!(sha(&source), before);
    }
}
#[test]
fn convert1729_late_target_single_volume_family_and_cancel_do_not_overwrite_user_files() {
    use crate::models::compression::CompressionOptions;
    let temp = tempfile::tempdir().unwrap();
    let source = original("tar-forward-links-1728", "gnu.tar.lzo").0;
    for kind in ["single", "volumes", "cancel"] {
        let output = temp.path().join(format!("{kind}.zip"));
        let cancel = flag();
        let prepared = Prepared::open(&source, &output, options(vec![]), cancel.clone())
            .unwrap()
            .unwrap();
        let scratch = prepared.contents().to_owned();
        let rebuilt = prepared.rebuilt().to_owned();
        let runtime = Runtime::new(cancel.clone());
        let files = prepared.read(&runtime, None, kind).unwrap();
        let split = kind == "volumes";
        let opts = CompressionOptions {
            format: Some("zip".into()),
            zip_method: super::zip_method::ZipMethod::Store,
            split_size: split.then_some(65536),
            ..Default::default()
        };
        build(&runtime, &files, &rebuilt, &opts);
        verify(&rebuilt, &opts).unwrap();
        let occupied = if split {
            output.with_file_name("volumes.zip.002")
        } else {
            output.clone()
        };
        if kind == "cancel" {
            cancel.store(true, Ordering::SeqCst);
        } else {
            std::fs::write(&occupied, b"user data").unwrap();
        }
        let error = prepared.publish_verified(split).unwrap_err();
        let message = super::archive_io::message(&error);
        if kind == "cancel" {
            assert!(message.to_ascii_lowercase().contains("cancel") || message.contains("已取消"));
            assert!(!output.exists());
        } else {
            assert!(
                message.contains("[archive-output:Publishing:output-conflict]"),
                "{message}"
            );
            assert_eq!(std::fs::read(&occupied).unwrap(), b"user data");
        }
        drop(prepared);
        assert!(!scratch.exists());
        assert!(!rebuilt.exists());
        if split {
            assert!(!output.with_file_name("volumes.zip.001").exists());
        }
    }
}
#[test]
fn convert1729_bad_source_missing_volume_ordinary_fallback_and_single_file_target_are_explicit() {
    let temp = tempfile::tempdir().unwrap();
    let (source, manifest) = original("tar-forward-links-1728", "gnu.tar.lzo");
    let raw = std::fs::read(source.with_extension("")).unwrap();
    let last = manifest["entries"].as_array().unwrap().last().unwrap();
    // Sparse logical length is not the stored TAR payload span. Cut inside
    // the final encoded body rather than deriving an offset from logical bytes.
    let mut tail = raw.clone();
    tail.truncate(last["dataOffset"].as_u64().unwrap() as usize + 1);
    let mut bad = wrapped(&raw, 1000, true);
    let last = bad.len() - 6;
    bad[last] ^= 1;
    for (i, data) in [wrapped(&tail, 1000, true), bad].into_iter().enumerate() {
        let path = temp.path().join(format!("bad-{i}.tar.lzo"));
        std::fs::write(&path, data).unwrap();
        assert!(Prepared::open(
            &path,
            &temp.path().join(format!("out-{i}.zip")),
            options(vec![]),
            flag()
        )
        .is_err());
    }
    let first = temp.path().join("split.tar.lzo.001");
    let bytes = std::fs::read(&source).unwrap();
    std::fs::write(&first, &bytes[..bytes.len() / 3]).unwrap();
    std::fs::write(
        temp.path().join("split.tar.lzo.003"),
        &bytes[bytes.len() * 2 / 3..],
    )
    .unwrap();
    assert!(Prepared::open(
        &first,
        &temp.path().join("gap.zip"),
        options(vec![]),
        flag()
    )
    .is_err());
    let ordinary = root().join("tar-lzo-read-1658/directory-1.tar.lzo");
    assert!(Prepared::open(
        &ordinary,
        &temp.path().join("ordinary.zip"),
        options(vec![]),
        flag()
    )
    .unwrap()
    .is_none());
    let prepared = Prepared::open(
        &source,
        &temp.path().join("single.lz"),
        options(vec![]),
        flag(),
    )
    .unwrap()
    .unwrap();
    let files = prepared
        .read(&Runtime::new(flag()), None, "unsupported")
        .unwrap();
    let request = crate::models::compression::CompressionOptions {
        format: Some("lz".into()),
        ..Default::default()
    };
    assert!(
        super::compression_service::CompressionService::validate_compression_request(
            &files,
            &prepared.rebuilt().to_string_lossy(),
            &request
        )
        .is_err()
    );
    drop(prepared);
    assert!(!temp.path().join("single.lz").exists());
    assert!(!std::fs::read_dir(temp.path()).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".long-")));
}
#[test]
fn convert1729_failed_read_or_writer_and_failed_verification_drop_all_private_state() {
    use crate::models::compression::CompressionOptions;
    let temp = tempfile::tempdir().unwrap();
    let (source, _) = original("tar-forward-links-1728", "gnu.tar.lzo");
    let before = sha(&source);
    for phase in ["reading", "writing", "verifying"] {
        let output = temp.path().join(format!("{phase}.zip"));
        let cancel = flag();
        let prepared = Prepared::open(&source, &output, options(vec![]), cancel.clone())
            .unwrap()
            .unwrap();
        let scratch = prepared.contents().to_owned();
        let rebuilt = prepared.rebuilt().to_owned();
        let mut fail = Runtime::new(cancel.clone());
        fail.hook = Arc::new(|n| {
            if n == 6 {
                return Err(std::io::Error::from_raw_os_error(112).into());
            }
            Ok(())
        });
        if phase == "reading" {
            let error = prepared.read(&fail, None, phase).unwrap_err();
            assert!(super::archive_io::message(&error).contains("disk-full"));
        } else {
            let files = prepared
                .read(&Runtime::new(cancel.clone()), None, phase)
                .unwrap();
            let opts = CompressionOptions {
                format: Some("zip".into()),
                ..Default::default()
            };
            if phase == "writing" {
                let error = super::native_compression::zip::compress(
                    &fail,
                    None,
                    phase,
                    &files,
                    &rebuilt.to_string_lossy(),
                    opts,
                )
                .unwrap_err();
                assert!(super::archive_io::message(&error).contains("disk-full"));
            } else {
                build(&Runtime::new(cancel.clone()), &files, &rebuilt, &opts);
                let bytes = std::fs::read(&rebuilt).unwrap();
                std::fs::write(&rebuilt, &bytes[..bytes.len() / 2]).unwrap();
                assert!(verify(&rebuilt, &opts).is_err());
            }
        }
        assert!(!output.exists());
        drop(prepared);
        assert!(!scratch.exists());
        assert!(!rebuilt.exists());
        assert_eq!(sha(&source), before);
    }
}
