use super::{
    compression_entries, extraction_transaction as tx,
    native_compression::seven_zip as create,
    native_extraction::{seven_zip as extract, ExtractionRuntime},
    seven_zip_directory_times as dates,
    zip_directory_time_1725_tests::Runtime,
};
use crate::models::compression::{CompressionOptions, DecompressOptions};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

fn base() -> PathBuf {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test-results/seven-zip-directory-layout-1727");
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn workspace() -> PathBuf {
    let p = base().join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn sha(path: &Path) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
}
fn opts() -> DecompressOptions {
    DecompressOptions {
        preserve_paths: true,
        preserve_timestamps: true,
        conflict_policy: "rename".into(),
        ..Default::default()
    }
}
fn compression() -> CompressionOptions {
    CompressionOptions {
        format: Some("7z".into()),
        level: 1,
        preserve_paths: Some(true),
        ..Default::default()
    }
}
fn sources(root: &Path) -> Vec<String> {
    let files = super::zip_directory_time_1725_tests::sources(root);
    for entry in compression_entries::collect_rooted(&files, true).unwrap() {
        let t = filetime::FileTime::from_last_modification_time(&entry.path.metadata().unwrap());
        filetime::set_file_mtime(
            &entry.path,
            filetime::FileTime::from_unix_time(t.unix_seconds(), 123_456_700),
        )
        .unwrap();
    }
    files
}
fn records(files: &[String], preserve: bool) -> Vec<serde_json::Value> {
    compression_entries::collect_rooted(files,preserve).unwrap().into_iter().map(|entry| {
        let time=filetime::FileTime::from_last_modification_time(&entry.path.metadata().unwrap());
        serde_json::json!({"name":entry.archive_name,"path":entry.path,"directory":entry.is_dir,"seconds":time.unix_seconds(),"nanoseconds":time.nanoseconds(),"sha256":if entry.is_dir{None}else{Some(sha(&entry.path))}})
    }).collect()
}
fn build(files: &[String], output: &Path, options: CompressionOptions) {
    create::compress(
        &Runtime::default(),
        None,
        "7zdir1727",
        files,
        output.to_str().unwrap(),
        options,
    )
    .unwrap();
}
fn stage(
    input: &Path,
    target: &Path,
    options: &DecompressOptions,
    password: Option<&str>,
) -> (tx::ExtractionStaging, super::lzop::selected::DirectoryTimes) {
    let staging = tx::ExtractionStaging::create_for(target).unwrap();
    let times = extract::extract_with_directory_times(
        &Runtime::default(),
        None,
        "7zdir1727",
        input.to_str().unwrap(),
        staging.path().to_str().unwrap(),
        password,
        options,
    )
    .unwrap();
    tx::prepare_staging_layout(input, staging.path(), options).unwrap();
    let times = dates::retained(&times, staging.path(), options);
    (staging, times)
}
fn commit(
    input: &Path,
    target: &Path,
    options: &DecompressOptions,
    password: Option<&str>,
) -> super::lzop::selected::DirectoryTimes {
    let (mut staging, times) = stage(input, target, options, password);
    tx::commit_staged_extraction(
        input.to_str().unwrap(),
        staging.path(),
        target,
        options,
        |_| {},
    )
    .unwrap();
    staging.cleanup().unwrap();
    assert!(super::lzop::selected::apply_directory_times(target, &times).is_empty());
    times
}
fn exact(target: &Path, expected: &[serde_json::Value]) {
    assert_eq!(
        walkdir::WalkDir::new(target).into_iter().count() - 1,
        expected.len()
    );
    for source in expected {
        let path = target.join(source["name"].as_str().unwrap());
        assert_eq!(
            path.is_dir(),
            source["directory"].as_bool().unwrap(),
            "{}",
            path.display()
        );
        let time = filetime::FileTime::from_last_modification_time(&path.metadata().unwrap());
        assert_eq!(
            time.unix_seconds(),
            source["seconds"].as_i64().unwrap(),
            "{}",
            path.display()
        );
        assert_eq!(
            time.nanoseconds(),
            source["nanoseconds"].as_u64().unwrap() as u32,
            "{}",
            path.display()
        );
        if !path.is_dir() {
            assert_eq!(sha(&path), source["sha256"]);
        }
    }
}
fn verify(input: &Path, password: Option<&str>) {
    assert!(super::compression_verification::verify_native(
        super::compression_format::CompressionRoute::SevenZip,
        input,
        password,
        || false
    )
    .unwrap());
}
fn evidence(name: &str, items: &[serde_json::Value]) {
    std::fs::write(
        base().join(format!("{name}.json")),
        serde_json::to_vec_pretty(items).unwrap(),
    )
    .unwrap();
}

#[test]
fn sevenzdir1727_root_folder_file_collisions_share_one_subtree_identity() {
    let root = workspace();
    let mut files = sources(&root.join("sources"));
    std::fs::create_dir_all(root.join("c")).unwrap();
    std::fs::write(root.join("c/资料"), b"standalone").unwrap();
    files.insert(2, root.join("c/资料").to_string_lossy().into_owned());
    let output = root.join("roots.7z");
    build(&files, &output, compression());
    let reader = sevenz_rust::SevenZReader::open(&output, sevenz_rust::Password::empty()).unwrap();
    let names = reader
        .archive()
        .files
        .iter()
        .map(|e| e.name().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(names.len(), 14);
    for name in [
        "资料",
        "资料 (1)",
        "资料 (2)",
        "资料 (1)/nested/note.txt",
        "empty-root",
    ] {
        assert!(names.contains(&name.into()), "{name}");
    }
    assert!(!reader
        .archive()
        .files
        .iter()
        .find(|e| e.name() == "资料 (2)")
        .unwrap()
        .is_directory());
}

#[test]
fn sevenzdir1727_plain_solid_password_numeric_forum_and_flat_layout_round_trip() {
    let root = workspace();
    let files = sources(&root.join("source"));
    let original = records(&files, true);
    let mut proof = Vec::new();
    for (name, solid, password, hidden, numeric, preserve) in [
        ("plain", false, None, false, false, true),
        ("solid", true, None, false, false, true),
        ("password", false, Some("SevenDir1727!"), false, false, true),
        ("hidden", true, Some("SevenDir1727!"), true, false, true),
        ("numeric", true, Some("SevenDir1727!"), true, true, true),
        ("flat-create", false, None, false, false, false),
    ] {
        let output = root.join(format!("{name}.7z"));
        build(
            &files,
            &output,
            CompressionOptions {
                create_solid_archive: solid,
                password: password.map(str::to_owned),
                encrypt_filenames: hidden,
                split_size: if numeric { Some(100_000) } else { None },
                preserve_paths: Some(preserve),
                ..compression()
            },
        );
        let input = if numeric {
            output.with_file_name(format!("{name}.7z.001"))
        } else {
            output.clone()
        };
        let parts = if numeric {
            super::archive_volumes::complete_volume_paths(&output).unwrap()
        } else {
            vec![output.clone()]
        };
        verify(&input, password);
        if password.is_some() {
            assert!(super::compression_verification::verify_native(
                super::compression_format::CompressionRoute::SevenZip,
                &input,
                Some("wrong"),
                || false
            )
            .is_err());
        }
        let expected = records(&files, preserve);
        let target = root.join(format!("restored-{name}"));
        let times = commit(&input, &target, &opts(), password);
        exact(&target, &expected);
        assert_eq!(
            times.len(),
            expected.iter().filter(|e| e["directory"] == true).count()
        );
        assert_eq!(records(&files, true), original);
        proof.push(serde_json::json!({"name":name,"input":input,"parts":parts,"password":password,"sources":expected,"restored":target,"directoryTimes":times}));
        if numeric {
            let forum = root.join("forum");
            std::fs::create_dir_all(&forum).unwrap();
            let mut renamed = Vec::new();
            for (index, part) in parts.iter().enumerate() {
                let to = forum.join(format!("资料.haozip{:02}.7z", index + 1));
                std::fs::copy(part, &to).unwrap();
                renamed.push(to);
            }
            let input = &renamed[0];
            verify(input, password);
            let target = root.join("restored-forum");
            let times = commit(input, &target, &opts(), password);
            exact(&target, &expected);
            proof.push(serde_json::json!({"name":"forum-names","input":input,"parts":renamed,"password":password,"sources":expected,"restored":target,"directoryTimes":times}));
        }
    }
    evidence("native", &proof);
}

#[test]
fn sevenzdir1727_selected_directory_file_filter_flattening_and_time_switch_use_original_names() {
    let root = workspace();
    let files = sources(&root.join("source"));
    let input = root.join("selected.7z");
    build(
        &files,
        &input,
        CompressionOptions {
            create_solid_archive: true,
            password: Some("SevenDir1727!".into()),
            encrypt_filenames: true,
            ..compression()
        },
    );
    let mut proof = Vec::new();
    for (mode, options) in [
        (
            "directory",
            DecompressOptions {
                selected_entries: vec!["资料/nested".into()],
                ..opts()
            },
        ),
        (
            "file",
            DecompressOptions {
                selected_entries: vec!["资料/nested/note.txt".into()],
                ..opts()
            },
        ),
        (
            "filter",
            DecompressOptions {
                selected_entries: vec!["资料".into()],
                file_filter: Some("*.txt".into()),
                ..opts()
            },
        ),
        (
            "flat",
            DecompressOptions {
                selected_entries: vec!["资料/nested/note.txt".into()],
                preserve_paths: false,
                ..opts()
            },
        ),
        (
            "off",
            DecompressOptions {
                selected_entries: vec!["empty-root".into()],
                preserve_timestamps: false,
                ..opts()
            },
        ),
    ] {
        let target = root.join(mode);
        let times = commit(&input, &target, &options, Some("SevenDir1727!"));
        match mode {
            "directory" => {
                assert_eq!(times.len(), 2);
                assert!(target.join("资料/nested/empty").is_dir());
                assert!(!target.join("资料/file.bin").exists());
            }
            "filter" => {
                assert_eq!(times.len(), 2);
                assert!(!target.join("资料/file.bin").exists());
            }
            "flat" => {
                assert!(times.is_empty());
                assert_eq!(std::fs::read(target.join("note.txt")).unwrap(), b"a");
                assert!(!target.join("资料").exists());
            }
            _ => assert!(times.is_empty()),
        }
        for (name, t) in &times {
            assert_eq!(
                filetime::FileTime::from_last_modification_time(
                    &target.join(name).metadata().unwrap()
                ),
                filetime::FileTime::from_unix_time(t.seconds, t.nanoseconds)
            );
        }
        proof.push(serde_json::json!({"mode":mode,"restored":target,"times":times}));
    }
    evidence("selection", &proof);
}

#[test]
fn sevenzdir1727_conflict_resume_reuses_captured_dates_without_reading_the_source() {
    let root = workspace();
    let files = sources(&root.join("source"));
    let input = root.join("conflict.7z");
    build(&files, &input, compression());
    let target = root.join("final");
    std::fs::create_dir_all(target.join("资料/nested")).unwrap();
    std::fs::write(target.join("资料/nested/note.txt"), b"existing").unwrap();
    std::fs::write(target.join("unrelated.txt"), b"keep").unwrap();
    let options = DecompressOptions {
        conflict_policy: "ask".into(),
        overwrite_existing: false,
        ..opts()
    };
    let (mut staging, times) = stage(&input, &target, &options, None);
    let mut conflicts = 0;
    assert!(tx::commit_staged_extraction(
        input.to_str().unwrap(),
        staging.path(),
        &target,
        &options,
        |_| conflicts += 1
    )
    .is_err());
    assert_eq!(conflicts, 1);
    std::fs::rename(&input, root.join("original-kept.7z")).unwrap();
    tx::commit_staged_extraction_with_resolutions(
        input.to_str().unwrap(),
        staging.path(),
        &target,
        &options,
        &std::collections::HashMap::new(),
        Some("rename"),
        |_| {},
    )
    .unwrap();
    staging.cleanup().unwrap();
    assert!(super::lzop::selected::apply_directory_times(&target, &times).is_empty());
    assert_eq!(
        std::fs::read(target.join("资料/nested/note.txt")).unwrap(),
        b"existing"
    );
    assert_eq!(
        std::fs::read(target.join("资料/nested/note (1).txt")).unwrap(),
        b"a"
    );
    assert_eq!(
        std::fs::read(target.join("unrelated.txt")).unwrap(),
        b"keep"
    );
    for (name, t) in times {
        assert_eq!(
            filetime::FileTime::from_last_modification_time(&target.join(name).metadata().unwrap()),
            filetime::FileTime::from_unix_time(t.seconds, t.nanoseconds)
        );
    }
}

#[derive(Default)]
struct CancelRuntime {
    calls: AtomicUsize,
    inner: Runtime,
}
impl ExtractionRuntime for CancelRuntime {
    fn check_cancellation(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.calls.fetch_add(1, Ordering::Relaxed) < 3,
            "cancelled 7Z dates"
        );
        Ok(())
    }
    fn copy_buffer_size(&self) -> usize {
        65536
    }
    fn buffer_pool(&self) -> &super::io_buffer_pool::IOBufferPool {
        ExtractionRuntime::buffer_pool(&self.inner)
    }
    fn normalized_archive_path(&self, path: &Path, preserve: bool) -> Option<PathBuf> {
        self.inner.normalized_archive_path(path, preserve)
    }
    fn emit_log(
        &self,
        _: &tauri::Window,
        _: &str,
        _: &str,
        _: crate::models::compression::TaskLogSeverity,
    ) {
    }
    fn emit_progress(&self, _: &tauri::Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {
    }
}

#[test]
fn sevenzdir1727_explicit_100ns_dates_missing_ambiguous_directories_and_cancel() {
    let root = workspace();
    let folder = root.join("folder");
    std::fs::create_dir(&folder).unwrap();
    let timestamp = filetime::FileTime::from_unix_time(1_700_001_727, 123_456_700);
    filetime::set_file_mtime(&folder, timestamp).unwrap();
    let input = root.join("metadata.7z");
    let payload = root.join("payload");
    std::fs::write(&payload, b"abc").unwrap();
    let mut writer = sevenz_rust::SevenZWriter::create(&input).unwrap();
    for name in ["dated", "ambiguous", "AMBIGUOUS", "no-date"] {
        let mut entry = sevenz_rust::SevenZArchiveEntry::from_path(&folder, name.into());
        entry.is_anti_item = true;
        if name == "no-date" {
            entry.has_last_modified_date = false;
        }
        writer
            .push_archive_entry::<std::io::Empty>(entry, None)
            .unwrap();
    }
    for name in ["dated/file.txt", "implicit/file.txt"] {
        let mut entry = sevenz_rust::SevenZArchiveEntry::from_path(&payload, name.into());
        entry.is_anti_item = true;
        writer
            .push_archive_entry(entry, Some(std::fs::File::open(&payload).unwrap()))
            .unwrap();
    }
    writer.finish().unwrap();
    let target = root.join("restored");
    let times = commit(&input, &target, &opts(), None);
    assert_eq!(times.len(), 1);
    assert_eq!(times[0].0, "dated");
    assert_eq!(times[0].1.nanoseconds, 123_456_700);
    assert_eq!(
        filetime::FileTime::from_last_modification_time(&target.join("dated").metadata().unwrap()),
        timestamp
    );
    let files = sources(&root.join("source"));
    let large = root.join("cancel.7z");
    build(&files, &large, compression());
    let staging = tx::ExtractionStaging::create_for(&root.join("cancelled-result")).unwrap();
    assert!(extract::extract_with_directory_times(
        &CancelRuntime::default(),
        None,
        "cancel",
        large.to_str().unwrap(),
        staging.path().to_str().unwrap(),
        None,
        &opts()
    )
    .is_err());
    assert!(!root.join("cancelled-result").exists());
}

#[test]
fn sevenzdir1727_verified_tar_lzo_conversion_keeps_root_directory_dates() {
    let root = workspace();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/archives/tar-browse-links-1719/gnu.tar.lzo");
    let before = sha(&source);
    let mut proof = Vec::new();
    for (name, split) in [("hidden", false), ("numeric", true)] {
        let output = root.join(format!("{name}.7z"));
        let prepared = super::tar_link_conversion::Prepared::open(
            &source,
            &output,
            opts(),
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap()
        .unwrap();
        let files = prepared
            .read(&Runtime::default(), None, "convert7z")
            .unwrap();
        let expected = records(&files, true);
        build(
            &files,
            prepared.rebuilt(),
            CompressionOptions {
                password: Some("SevenDir1727!".into()),
                encrypt_filenames: true,
                create_solid_archive: true,
                split_size: if split { Some(256) } else { None },
                ..compression()
            },
        );
        let working = if split {
            prepared.rebuilt().with_file_name(format!(
                "{}.001",
                prepared.rebuilt().file_name().unwrap().to_string_lossy()
            ))
        } else {
            prepared.rebuilt().to_path_buf()
        };
        verify(&working, Some("SevenDir1727!"));
        prepared.publish_verified(split).unwrap();
        drop(prepared);
        let input = if split {
            output.with_file_name(format!("{name}.7z.001"))
        } else {
            output.clone()
        };
        let parts = if split {
            super::archive_volumes::complete_volume_paths(&output).unwrap()
        } else {
            vec![output]
        };
        let target = root.join(format!("restored-{name}"));
        let times = commit(&input, &target, &opts(), Some("SevenDir1727!"));
        exact(&target, &expected);
        assert_eq!(times.len(), 2);
        assert_eq!(sha(&source), before);
        proof.push(serde_json::json!({"name":name,"input":input,"parts":parts,"password":"SevenDir1727!","sources":expected,"sourceTar":source.with_file_name("gnu.tar"),"sourceSha256":before,"restored":target,"directoryTimes":times}));
    }
    evidence("conversion", &proof);
}
