use super::{
    compression_entries, extraction_transaction as tx,
    native_compression::tar as create,
    native_extraction::tar as extract,
    tar_compatibility as tar, tar_directory_times as dates,
    zip_directory_time_1725_tests::{records, sources, Runtime},
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
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/tar-directory-layout-1726");
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn workspace() -> PathBuf {
    let p = base().join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn sha(p: &Path) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(p).unwrap()))
}
fn options() -> DecompressOptions {
    DecompressOptions {
        preserve_paths: true,
        preserve_timestamps: true,
        conflict_policy: "rename".into(),
        ..Default::default()
    }
}
fn route(format: &str) -> super::compression_format::CompressionRoute {
    super::compression_format::compression_route(format).unwrap()
}
fn decoder(input: &Path, format: &str) -> Option<Box<dyn std::io::Read + Send>> {
    if format == "tar" {
        None
    } else {
        Some(super::codec_stream::decoder(input, format.strip_prefix("tar.").unwrap()).unwrap())
    }
}
fn build(files: &[String], output: &Path, format: &str) {
    let opts = CompressionOptions {
        format: Some(format.into()),
        level: 1,
        preserve_paths: Some(true),
        ..Default::default()
    };
    let runtime = Runtime::default();
    match format {
        "tar" => create::compress_tar(
            &runtime,
            None,
            "tar-root",
            files,
            output.to_str().unwrap(),
            opts,
        ),
        "tar.gz" => create::compress_gzip(
            &runtime,
            None,
            "tar-root",
            files,
            output.to_str().unwrap(),
            opts,
        ),
        "tar.bz2" => create::compress_bzip2(
            &runtime,
            None,
            "tar-root",
            files,
            output.to_str().unwrap(),
            opts,
        ),
        "tar.xz" => create::compress_xz(
            &runtime,
            None,
            "tar-root",
            files,
            output.to_str().unwrap(),
            opts,
        ),
        "tar.zst" => create::compress_zstd(
            &runtime,
            None,
            "tar-root",
            files,
            output.to_str().unwrap(),
            opts,
        ),
        _ => super::codec_stream::compress(
            &runtime,
            None,
            "tar-root",
            files,
            output.to_str().unwrap(),
            opts,
            format,
        ),
    }
    .unwrap();
}
fn stage(
    input: &Path,
    format: &str,
    target: &Path,
    opts: &DecompressOptions,
) -> (tx::ExtractionStaging, super::lzop::selected::DirectoryTimes) {
    let plan = tar::plan(input, route(format), opts, || Ok(())).unwrap();
    let staging = tx::ExtractionStaging::create_for(target).unwrap();
    extract::extract_inner(
        &Runtime::default(),
        None,
        "tar-root",
        input.to_str().unwrap(),
        staging.path(),
        decoder(input, format),
        opts,
    )
    .unwrap();
    tx::prepare_staging_layout_with_tar_plan(input, staging.path(), opts, Some(&plan)).unwrap();
    let times = dates::retained(&plan.directory_times, staging.path(), opts);
    (staging, times)
}
fn commit(
    input: &Path,
    format: &str,
    target: &Path,
    opts: &DecompressOptions,
) -> super::lzop::selected::DirectoryTimes {
    let (mut staging, times) = stage(input, format, target, opts);
    tx::commit_staged_extraction(
        input.to_str().unwrap(),
        staging.path(),
        target,
        opts,
        |_| {},
    )
    .unwrap();
    staging.cleanup().unwrap();
    assert!(super::lzop::selected::apply_directory_times(target, &times).is_empty());
    times
}
fn exact(output: &Path, expected: &[serde_json::Value]) {
    for e in expected {
        let p = output.join(e["name"].as_str().unwrap());
        assert_eq!(
            p.is_dir(),
            e["directory"].as_bool().unwrap(),
            "{}",
            p.display()
        );
        assert_eq!(
            filetime::FileTime::from_last_modification_time(&p.metadata().unwrap()).unix_seconds(),
            e["seconds"].as_i64().unwrap(),
            "{}",
            p.display()
        );
        if !p.is_dir() {
            assert_eq!(sha(&p), e["sha256"]);
        }
    }
    assert_eq!(
        walkdir::WalkDir::new(output).into_iter().count() - 1,
        expected.len()
    );
}

#[test]
fn tardir1726_root_names_shared_with_zip_and_legacy_flat_and_sevenz_are_unchanged() {
    let root = workspace();
    let files = sources(&root);
    let names = |entries: Vec<compression_entries::CompressionEntry>| {
        entries
            .into_iter()
            .map(|e| e.archive_name)
            .collect::<Vec<_>>()
    };
    let rooted = names(compression_entries::collect_rooted(&files, true).unwrap());
    assert_eq!(rooted.len(), 13);
    assert!(rooted.contains(&"资料 (1)/nested/note.txt".into()));
    assert_eq!(
        rooted,
        names(compression_entries::collect_zip(&files, true).unwrap())
    );
    assert_eq!(
        names(compression_entries::collect_rooted(&files, false).unwrap()),
        names(compression_entries::collect(&files, false, true).unwrap())
    );
    assert!(
        !names(compression_entries::collect(&files, true, true).unwrap()).contains(&"资料".into())
    );
}

#[test]
fn tardir1726_all_tar_writers_numeric_volumes_and_final_directory_dates_round_trip() {
    let root = workspace();
    let files = sources(&root);
    let expected = records(&files, true);
    let mut evidence = Vec::new();
    for format in [
        "tar", "tar.gz", "tar.bz2", "tar.xz", "tar.zst", "tar.lzma", "tar.lz", "tar.lz4", "tar.br",
        "tar.lzo",
    ] {
        let created = root.join(format!("created.{format}"));
        build(&files, &created, format);
        assert!(super::compression_verification::verify_native(
            route(format),
            &created,
            None,
            || false
        )
        .unwrap());
        let mut inputs = vec![(created.clone(), vec![created.clone()], false)];
        if matches!(format, "tar" | "tar.lzo") {
            let output = root.join(format!("numeric.{format}"));
            super::split_compression::SplitCompressionService::split_archive_file_with_progress(
                &created,
                &output,
                65536,
                || Ok(()),
                |_, _| {},
            )
            .unwrap();
            let parts = super::archive_volumes::complete_volume_paths(&output).unwrap();
            assert!(parts.len() > 1);
            inputs.push((parts[0].clone(), parts, true));
        }
        for (input, parts, numeric) in inputs {
            let detected = super::archive_browser::detect_format(&input).unwrap();
            assert_eq!(
                tar::extraction_route(&input, &detected),
                Some(route(format))
            );
            let target = root.join(format!("restored-{}-{}", format.replace('.', "-"), numeric));
            let times = commit(&input, format, &target, &options());
            assert_eq!(times.len(), 7);
            exact(&target, &expected);
            assert_eq!(records(&files, true), expected);
            evidence.push(serde_json::json!({"format":format,"input":input,"parts":parts,"numeric":numeric,"sources":expected,"restored":target,"directoryTimes":times}));
        }
    }
    std::fs::write(
        base().join("native.json"),
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
}

#[test]
fn tardir1726_selection_filter_file_only_flattening_and_time_switch_use_original_archive_paths() {
    let root = workspace();
    let files = sources(&root);
    let input = root.join("selection.tar.gz");
    build(&files, &input, "tar.gz");
    let mut evidence = Vec::new();
    for (mode, opts) in [
        (
            "directory",
            DecompressOptions {
                selected_entries: vec!["资料/nested".into()],
                ..options()
            },
        ),
        (
            "file",
            DecompressOptions {
                selected_entries: vec!["资料/nested/note.txt".into()],
                ..options()
            },
        ),
        (
            "filter",
            DecompressOptions {
                selected_entries: vec!["资料".into()],
                file_filter: Some("*.txt".into()),
                ..options()
            },
        ),
        (
            "flat",
            DecompressOptions {
                selected_entries: vec!["资料/nested".into()],
                preserve_paths: false,
                ..options()
            },
        ),
        (
            "off",
            DecompressOptions {
                selected_entries: vec!["empty-root".into()],
                preserve_timestamps: false,
                ..options()
            },
        ),
    ] {
        let target = root.join(mode);
        let times = commit(&input, "tar.gz", &target, &opts);
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
                assert!(target.join("note.txt").is_file());
                assert!(!target.join("资料").exists());
            }
            _ => assert!(times.is_empty()),
        };
        for (name, t) in &times {
            assert_eq!(
                filetime::FileTime::from_last_modification_time(
                    &target.join(name).metadata().unwrap()
                )
                .unix_seconds(),
                t.seconds
            );
        }
        evidence.push(serde_json::json!({"mode":mode,"restored":target,"times":times}));
    }
    std::fs::write(
        base().join("selection.json"),
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
}

#[test]
fn tardir1726_conflict_resume_reuses_preflight_dates_without_reading_the_source_again() {
    let root = workspace();
    let files = sources(&root);
    let input = root.join("source.tar");
    build(&files, &input, "tar");
    let target = root.join("final");
    std::fs::create_dir_all(target.join("资料/nested")).unwrap();
    std::fs::write(target.join("资料/nested/note.txt"), b"existing").unwrap();
    std::fs::write(target.join("unrelated.txt"), b"keep").unwrap();
    let opts = DecompressOptions {
        conflict_policy: "ask".into(),
        overwrite_existing: false,
        ..options()
    };
    let (mut staging, times) = stage(&input, "tar", &target, &opts);
    let mut conflicts = 0;
    assert!(tx::commit_staged_extraction(
        input.to_str().unwrap(),
        staging.path(),
        &target,
        &opts,
        |_| conflicts += 1
    )
    .is_err());
    assert_eq!(conflicts, 1);
    std::fs::rename(&input, root.join("original-kept.tar")).unwrap();
    tx::commit_staged_extraction_with_resolutions(
        input.to_str().unwrap(),
        staging.path(),
        &target,
        &opts,
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
    for (name, t) in &times {
        assert_eq!(
            filetime::FileTime::from_last_modification_time(&target.join(name).metadata().unwrap())
                .unix_seconds(),
            t.seconds
        );
    }
}

#[test]
fn tardir1726_pax_dates_implicit_ambiguous_directories_cancel_and_bad_footer_do_not_invent_dates() {
    let root = workspace();
    let input = root.join("dates.tar");
    let mut b = ::tar::Builder::new(std::fs::File::create(&input).unwrap());
    b.append_pax_extensions([("path", b"real/".as_slice()), ("mtime", b"1700001726.125")])
        .unwrap();
    let mut h = ::tar::Header::new_gnu();
    h.set_entry_type(::tar::EntryType::Directory);
    h.set_size(0);
    h.set_mode(0o755);
    h.set_mtime(42);
    h.set_cksum();
    b.append_data(&mut h, "placeholder", std::io::empty())
        .unwrap();
    for name in ["ambiguous/", "AMBIGUOUS/"] {
        b.append_data(&mut h, name, std::io::empty()).unwrap();
    }
    let mut f = ::tar::Header::new_gnu();
    f.set_size(3);
    f.set_mode(0o644);
    f.set_mtime(1700001726);
    f.set_cksum();
    b.append_data(&mut f, "real/file.txt", b"abc".as_slice())
        .unwrap();
    b.append_data(&mut f, "implicit/file.txt", b"def".as_slice())
        .unwrap();
    b.finish().unwrap();
    drop(b);
    let plan = tar::plan(&input, route("tar"), &options(), || Ok(())).unwrap();
    assert_eq!(plan.directory_times.len(), 1);
    assert_eq!(plan.directory_times[0].0, "real");
    assert_eq!(plan.directory_times[0].1.nanoseconds, 125_000_000);
    let target = root.join("restored");
    let times = commit(&input, "tar", &target, &options());
    assert_eq!(times.len(), 1);
    assert_eq!(
        filetime::FileTime::from_last_modification_time(&target.join("real").metadata().unwrap())
            .nanoseconds(),
        125_000_000
    );
    let calls = AtomicUsize::new(0);
    assert!(tar::plan(&input, route("tar"), &options(), || {
        anyhow::ensure!(
            calls.fetch_add(1, Ordering::Relaxed) < 2,
            "cancelled tar metadata"
        );
        Ok(())
    })
    .unwrap_err()
    .to_string()
    .contains("cancelled tar metadata"));
    let files = sources(&root.join("source"));
    let broken = root.join("broken.tar.gz");
    build(&files, &broken, "tar.gz");
    let mut bytes = std::fs::read(&broken).unwrap();
    bytes.truncate(bytes.len() - 2);
    std::fs::write(&broken, bytes).unwrap();
    assert!(tar::plan(&broken, route("tar.gz"), &options(), || Ok(())).is_err());
}

#[test]
fn tardir1726_verified_conversion_keeps_tar_directory_identity_and_times() {
    let root = workspace();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/archives/tar-browse-links-1719/gnu.tar.lzo");
    let before = sha(&source);
    let mut evidence = Vec::new();
    for format in ["tar", "tar.gz"] {
        let output = root.join(format!("converted.{format}"));
        let prepared = super::tar_link_conversion::Prepared::open(
            &source,
            &output,
            options(),
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap()
        .unwrap();
        let files = prepared
            .read(&Runtime::default(), None, "tar-convert")
            .unwrap();
        let expected = records(&files, true);
        build(&files, prepared.rebuilt(), format);
        assert!(super::compression_verification::verify_native(
            route(format),
            prepared.rebuilt(),
            None,
            || false
        )
        .unwrap());
        prepared.publish_verified(false).unwrap();
        drop(prepared);
        let target = root.join(format!("restored-{format}"));
        let times = commit(&output, format, &target, &options());
        exact(&target, &expected);
        assert_eq!(times.len(), 2);
        assert_eq!(sha(&source), before);
        evidence.push(serde_json::json!({"format":format,"input":output,"parts":[output],"sources":expected,"sourceTar":source.with_file_name("gnu.tar"),"sourceSha256":before,"restored":target,"directoryTimes":times}));
    }
    std::fs::write(
        base().join("conversion.json"),
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
}
