use super::{
    archive_browser, archive_preview, extraction_transaction as tx,
    image_reader::Control,
    lzop::{
        recovery,
        selected::{self, Plan},
    },
    tar_browse_links,
    tar_lzo_prefix_175_tests::wrapped,
    zip_directory_time_1725_tests::Runtime,
};
use crate::models::compression::DecompressOptions;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/tar-forward-links-1728")
}
fn base() -> PathBuf {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/tar-forward-links-1728");
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn workspace() -> PathBuf {
    let p = base().join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn load(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn manifest() -> serde_json::Value {
    load(&root().join("manifest.json"))
}
fn flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}
fn sha(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
fn digest(path: &Path) -> String {
    sha(&std::fs::read(path).unwrap())
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
fn save(name: &str, data: serde_json::Value) {
    std::fs::write(
        base().join(format!("{name}.json")),
        serde_json::to_vec_pretty(&data).unwrap(),
    )
    .unwrap();
}
fn numeric(source: &Path, folder: &Path) -> Vec<PathBuf> {
    std::fs::create_dir_all(folder).unwrap();
    std::fs::read(source)
        .unwrap()
        .chunks(2048)
        .enumerate()
        .map(|(index, bytes)| {
            let p = folder.join(format!("source.tar.lzo.{:03}", index + 1));
            std::fs::write(&p, bytes).unwrap();
            p
        })
        .collect()
}
fn stage(input: &Path, target: &Path, opts: &DecompressOptions) -> (Plan, tx::ExtractionStaging) {
    let cancel = flag();
    let plan = Plan::for_extraction(input, opts, &cancel).unwrap().unwrap();
    let stage = tx::ExtractionStaging::create_for(target).unwrap();
    plan.extract(
        &Runtime::default(),
        None,
        "forward1728",
        input,
        stage.path(),
        opts,
        &cancel,
    )
    .unwrap();
    plan.prepare_stage(input, stage.path(), opts).unwrap();
    (plan, stage)
}
fn publish(input: &Path, target: &Path, opts: &DecompressOptions) -> serde_json::Value {
    let (plan, mut stage) = stage(input, target, opts);
    assert!(plan.has_forward_links());
    let bytes =
        tx::commit_staged_extraction(input.to_str().unwrap(), stage.path(), target, opts, |_| {})
            .unwrap();
    stage.cleanup().unwrap();
    let times = plan.directory_times(opts);
    assert!(selected::apply_directory_times(target, &times).is_empty());
    serde_json::json!({"entries":plan.stats.entries,"logicalBytes":plan.stats.logical_bytes,"payloadBytes":plan.stats.payload_bytes,"publishedBytes":bytes,"directoryTimes":times})
}
fn exact(target: &Path, entries: &[serde_json::Value], times: bool) {
    let mut names = HashSet::new();
    for e in entries {
        let name = e["name"].as_str().unwrap();
        names.insert(name.to_owned());
        for parent in Path::new(name)
            .ancestors()
            .skip(1)
            .filter(|p| !p.as_os_str().is_empty())
        {
            names.insert(parent.to_string_lossy().replace('\\', "/"));
        }
        let path = target.join(name);
        assert_eq!(path.is_dir(), e["isDirectory"].as_bool().unwrap(), "{name}");
        if times {
            assert_eq!(
                filetime::FileTime::from_last_modification_time(&path.metadata().unwrap())
                    .unix_seconds(),
                e["seconds"].as_i64().unwrap(),
                "{name}"
            );
        }
        if !path.is_dir() {
            assert_eq!(path.metadata().unwrap().len(), e["bytes"].as_u64().unwrap());
            assert_eq!(digest(&path), e["sha256"], "{name}");
        }
    }
    let actual = walkdir::WalkDir::new(target)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path() != target)
        .map(|e| {
            e.path()
                .strip_prefix(target)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<HashSet<_>>();
    assert_eq!(actual, names);
}

#[tokio::test]
async fn forward1728_real_ustar_gnu_pax_browse_preview_integrity_and_diagnosis_agree() {
    let m = manifest();
    let work = workspace();
    let mut proof = Vec::new();
    for v in m["variants"].as_array().unwrap() {
        let source = root().join(v["archive"].as_str().unwrap());
        assert_eq!(digest(&source), v["sha256"]);
        let entries = v["entries"].as_array().unwrap();
        let listing = archive_browser::browse_archive_cancellable(&source, None, flag())
            .await
            .unwrap();
        assert_eq!(listing.entries.len(), entries.len());
        for (actual, expected) in listing.entries.iter().zip(entries) {
            assert_eq!(
                actual.path.trim_end_matches('/'),
                expected["name"].as_str().unwrap()
            );
            assert_eq!(actual.size, expected["bytes"].as_u64().unwrap());
            assert_eq!(
                actual.link_target.as_deref(),
                expected["linkTarget"].as_str()
            );
        }
        let parts = numeric(&source, &work.join(v["format"].as_str().unwrap()));
        let numeric_listing =
            archive_browser::browse_archive_cancellable(parts.last().unwrap(), None, flag())
                .await
                .unwrap();
        assert_eq!(numeric_listing.entries.len(), entries.len());
        assert_eq!(
            numeric_listing.total_uncompressed_size,
            listing.total_uncompressed_size
        );
        let mut texts = Vec::new();
        for entry in entries.iter().filter(|e| {
            e["linkTarget"].is_string() && e["name"].as_str().unwrap().ends_with(".txt")
        }) {
            let preview = archive_preview::preview_archive_text(
                &source,
                entry["name"].as_str().unwrap(),
                None,
            )
            .await
            .unwrap();
            assert_eq!(preview.total_size, entry["bytes"].as_u64().unwrap());
            assert_eq!(sha(preview.content.as_bytes()), entry["prefixSha256"]);
            texts.push(preview);
        }
        let image = archive_preview::preview_archive_image(&source, "gallery/封面副本.png", None)
            .await
            .unwrap();
        assert_eq!((image.width, image.height), (32, 24));
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(image.data_url.split(',').nth(1).unwrap())
            .unwrap();
        assert_eq!(
            sha(&bytes),
            entries
                .iter()
                .find(|e| e["name"] == "gallery/封面副本.png")
                .unwrap()["sha256"]
        );
        if let Some(sparse) = entries.iter().find(|e| e["name"] == "sparse-copy.bin") {
            let (bytes, size) = tar_browse_links::preview(
                &source,
                "sparse-copy.bin",
                1048576,
                true,
                &Control::default(),
            )
            .unwrap();
            assert_eq!(size, sparse["bytes"].as_u64().unwrap());
            assert_eq!(sha(&bytes), sparse["prefixSha256"]);
        }
        super::universal_engine::UniversalCliEngine::test_integrity(&source, None)
            .await
            .unwrap();
        assert!(super::compression_verification::verify_native(
            super::compression_format::CompressionRoute::TarLzop,
            &source,
            None,
            || false
        )
        .unwrap());
        let diagnosis = super::archive_diagnostics::diagnose_archive(&source, None, flag())
            .await
            .unwrap();
        assert_eq!(diagnosis.status, "healthy");
        assert!(diagnosis.integrity_tested);
        assert_eq!(
            diagnosis.total_uncompressed_size,
            listing.total_uncompressed_size
        );
        proof.push(serde_json::json!({"source":source,"listing":listing,"texts":texts,"image":image,"diagnosis":diagnosis,"numericParts":parts}));
    }
    save("browse", serde_json::json!(proof));
}

#[test]
fn forward1728_complete_selected_and_numeric_publish_independent_contents_and_own_dates() {
    let m = manifest();
    let work = workspace();
    let mut proof = Vec::new();
    for v in m["variants"].as_array().unwrap() {
        let source = root().join(v["archive"].as_str().unwrap());
        let entries = v["entries"].as_array().unwrap();
        let parts = numeric(
            &source,
            &work.join(format!("{}-volumes", v["format"].as_str().unwrap())),
        );
        for mode in ["full", "numeric", "selected"] {
            let input = if mode == "full" {
                source.clone()
            } else {
                parts.last().unwrap().clone()
            };
            let expected = entries
                .iter()
                .enumerate()
                .filter(|(index, e)| {
                    mode != "selected"
                        || e["targetId"]
                            .as_u64()
                            .is_some_and(|target| target > *index as u64)
                })
                .map(|(_, e)| e.clone())
                .collect::<Vec<_>>();
            let opts = options(if mode == "selected" {
                expected
                    .iter()
                    .map(|e| e["name"].as_str().unwrap().into())
                    .collect()
            } else {
                vec![]
            });
            let output = work.join(format!("{}-{mode}", v["format"].as_str().unwrap()));
            let stats = publish(&input, &output, &opts);
            exact(&output, &expected, true);
            assert!(stats["payloadBytes"].as_u64().unwrap() <= stats["logicalBytes"].as_u64().unwrap());
            if mode != "selected" {
                assert!(stats["payloadBytes"].as_u64().unwrap() < stats["logicalBytes"].as_u64().unwrap());
            }
            assert_eq!(digest(&source), v["sha256"]);
            proof.push(serde_json::json!({"mode":mode,"format":v["format"],"source":source,"sourceSha256":v["sha256"],"readSource":input,"outputDirectory":output,"entries":expected,"stats":stats}));
        }
    }
    save("native", serde_json::json!(proof));
}

#[test]
fn forward1728_directory_filter_flat_time_switch_and_conflict_resume_keep_exact_choices() {
    let work = workspace();
    let source = root().join("pax.tar.lzo");
    let m = manifest();
    let variant = &m["variants"][2];
    let entries = variant["entries"].as_array().unwrap();
    let mut proof = Vec::new();
    for (mode, opts) in [
        ("directory", options(vec!["docs/".into()])),
        (
            "filter",
            DecompressOptions {
                file_filter: Some("*.txt".into()),
                ..options(vec!["阅读副本.txt".into()])
            },
        ),
        (
            "flat",
            DecompressOptions {
                preserve_paths: false,
                ..options(vec!["gallery/封面副本.png".into()])
            },
        ),
        (
            "off",
            DecompressOptions {
                preserve_timestamps: false,
                ..options(vec!["empty-copy.txt".into()])
            },
        ),
    ] {
        let target = work.join(mode);
        let stats = publish(&source, &target, &opts);
        if mode == "directory" {
            let selected = entries
                .iter()
                .filter(|e| e["name"] == "docs" || e["name"].as_str().unwrap().starts_with("docs/"))
                .cloned()
                .collect::<Vec<_>>();
            exact(&target, &selected, true);
        } else if mode == "flat" {
            assert_eq!(
                digest(&target.join("封面副本.png")),
                entries
                    .iter()
                    .find(|e| e["name"] == "gallery/封面副本.png")
                    .unwrap()["sha256"]
            );
            assert!(!target.join("gallery").exists());
        } else {
            let name = if mode == "off" {
                "empty-copy.txt"
            } else {
                "阅读副本.txt"
            };
            let e = entries.iter().find(|e| e["name"] == name).unwrap();
            exact(&target, &[e.clone()], mode != "off");
        }
        proof.push(serde_json::json!({"mode":mode,"outputDirectory":target,"stats":stats}));
    }
    let input = work.join("source.tar.lzo");
    std::fs::copy(&source, &input).unwrap();
    let target = work.join("conflict");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("阅读副本.txt"), b"existing").unwrap();
    std::fs::write(target.join("unrelated.txt"), b"keep").unwrap();
    let opts = DecompressOptions {
        conflict_policy: "ask".into(),
        overwrite_existing: false,
        ..options(vec!["阅读副本.txt".into()])
    };
    let (plan, mut stage) = stage(&input, &target, &opts);
    let mut conflicts = 0;
    assert!(tx::commit_staged_extraction(
        input.to_str().unwrap(),
        stage.path(),
        &target,
        &opts,
        |_| conflicts += 1
    )
    .is_err());
    assert_eq!(conflicts, 1);
    std::fs::rename(&input, work.join("original-kept.tar.lzo")).unwrap();
    tx::commit_staged_extraction_with_resolutions(
        input.to_str().unwrap(),
        stage.path(),
        &target,
        &opts,
        &Default::default(),
        Some("rename"),
        |_| {},
    )
    .unwrap();
    stage.cleanup().unwrap();
    assert!(selected::apply_directory_times(&target, &plan.directory_times(&opts)).is_empty());
    assert_eq!(
        std::fs::read(target.join("阅读副本.txt")).unwrap(),
        b"existing"
    );
    assert_eq!(
        std::fs::read(target.join("unrelated.txt")).unwrap(),
        b"keep"
    );
    assert_eq!(
        digest(&target.join("阅读副本 (1).txt")),
        entries
            .iter()
            .find(|e| e["name"] == "阅读副本.txt")
            .unwrap()["sha256"]
    );
    let independent = work.join("independent");
    publish(
        &source,
        &independent,
        &options(vec!["阅读副本.txt".into(), "docs/readme.txt".into()]),
    );
    std::fs::write(independent.join("阅读副本.txt"), b"changed alias only").unwrap();
    assert_eq!(
        digest(&independent.join("docs/readme.txt")),
        entries
            .iter()
            .find(|e| e["name"] == "docs/readme.txt")
            .unwrap()["sha256"]
    );
    save("choices", serde_json::json!(proof));
}

#[tokio::test]
async fn forward1728_bad_tail_missing_volume_source_change_and_cancel_publish_nothing() {
    let work = workspace();
    let original = root().join("gnu.tar.lzo");
    let raw = std::fs::read(&original).unwrap();
    let source = work.join("source.tar.lzo");
    let opts = options(vec!["阅读副本.txt".into()]);
    std::fs::write(&source, &raw[..raw.len() - 2]).unwrap();
    assert!(Plan::for_extraction(&source, &opts, &flag()).is_err());
    assert!(
        archive_preview::preview_archive_text(&source, "阅读副本.txt", None)
            .await
            .is_err()
    );
    let parts = numeric(&original, &work.join("gap"));
    assert!(parts.len() > 2);
    std::fs::rename(&parts[1], parts[1].with_extension("kept")).unwrap();
    assert!(Plan::for_extraction(&parts[0], &opts, &flag()).is_err());
    std::fs::write(&source, &raw).unwrap();
    let plan = Plan::for_extraction(&source, &opts, &flag())
        .unwrap()
        .unwrap();
    filetime::set_file_mtime(
        &source,
        filetime::FileTime::from_unix_time(1_900_001_728, 0),
    )
    .unwrap();
    let target = work.join("not-published");
    let stage = tx::ExtractionStaging::create_for(&target).unwrap();
    assert!(plan
        .extract(
            &Runtime::default(),
            None,
            "changed",
            &source,
            stage.path(),
            &opts,
            &flag()
        )
        .is_err());
    assert_eq!(std::fs::read_dir(stage.path()).unwrap().count(), 0);
    assert!(!target.exists());
    let cancel = flag();
    let triggered = cancel.clone();
    let control = Control {
        flag: cancel,
        observer: Arc::new(move |p| {
            if p.processed_bytes > 4096 {
                triggered.store(true, Ordering::SeqCst)
            }
        }),
    };
    assert!(archive_preview::preview_archive_image_controlled_at(
        &original,
        "gallery/封面副本.png",
        None,
        None,
        None,
        control
    )
    .await
    .is_err());
    let prefix = recovery::scan(&original, None, &AtomicBool::new(false), |_, _| {}).unwrap();
    assert!(!prefix.complete);
    assert!(prefix
        .entries
        .iter()
        .all(|e| e.archive_name.as_deref() != Some("阅读副本.txt")));
    assert!(prefix.stop_reason.contains("尚未确认"));
    assert_eq!(digest(&original), manifest()["variants"][1]["sha256"]);
}

fn constructed(records: &[(&str, u8, &str, &[u8])]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (name, kind, target, data) in records {
        let mut h = tar::Header::new_gnu();
        h.set_path(name).unwrap();
        h.set_entry_type(tar::EntryType::new(*kind));
        h.set_size(data.len() as u64);
        h.set_mtime(1_700_001_728);
        h.set_mode(0o644);
        if !target.is_empty() {
            h.set_link_name(target).unwrap();
        }
        h.set_cksum();
        builder.append(&h, *data).unwrap();
    }
    builder.into_inner().unwrap()
}
#[tokio::test]
async fn forward1728_missing_ambiguous_chained_cyclic_directory_and_body_targets_never_become_empty_success(
) {
    let work = workspace();
    let mut reasons = Vec::new();
    for (index, records) in [
        vec![("alias", b'1', "missing", b"".as_slice())],
        vec![
            ("alias", b'1', "later", b"".as_slice()),
            ("later", b'0', "", b"a"),
            ("later", b'0', "", b"b"),
        ],
        vec![
            ("alias", b'1', "chain", b"".as_slice()),
            ("chain", b'1', "later", b""),
            ("later", b'0', "", b"a"),
        ],
        vec![
            ("alias", b'1', "cycle", b"".as_slice()),
            ("cycle", b'1', "alias", b""),
        ],
        vec![
            ("alias", b'1', "later", b"".as_slice()),
            ("later", b'5', "", b""),
        ],
        vec![
            ("alias", b'1', "later", b"unexpected".as_slice()),
            ("later", b'0', "", b"a"),
        ],
    ]
    .into_iter()
    .enumerate()
    {
        let source = work.join(format!("bad-{index}.tar.lzo"));
        std::fs::write(&source, wrapped(&constructed(&records), 127, true)).unwrap();
        let error = Plan::for_extraction(&source, &options(vec![]), &flag())
            .err()
            .unwrap();
        assert!(format!("{error:#}").contains("[archive-inspection:unsupported]"));
        assert!(
            archive_browser::browse_archive_cancellable(&source, None, flag())
                .await
                .is_err()
        );
        assert!(
            tar_browse_links::preview(&source, "alias", 1024, true, &Control::default()).is_err()
        );
        assert!(
            super::universal_engine::UniversalCliEngine::test_integrity(&source, None)
                .await
                .is_err()
        );
        let diagnosis = super::archive_diagnostics::diagnose_archive(&source, None, flag())
            .await
            .unwrap();
        assert_ne!(diagnosis.status, "healthy");
        reasons.push(format!("{error:#}"));
    }
    save("invalid", serde_json::json!(reasons));
}

#[tokio::test]
async fn forward1728_nonarchive_forward_alias_does_not_leave_nested_outputs() {
    let work = workspace();
    let source = root().join("gnu.tar.lzo");
    let cache = super::archive_entry_open::ArchiveEntryOpenCache::new(work.join("cache"));
    let error = super::nested_archive::materialize(
        &cache,
        Arc::new(Runtime::default()),
        source.clone(),
        "阅读副本.txt".into(),
        None,
        None,
        None,
        2,
        vec![],
        None,
        flag(),
        |_, _| async { anyhow::bail!("forward alias must not use generic extraction") },
    )
    .await
    .unwrap_err();
    assert!(format!("{error:#}").contains("不是可识别的压缩包"));
    assert_eq!(
        walkdir::WalkDir::new(work.join("cache"))
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .count(),
        0
    );
    assert_eq!(digest(&source), manifest()["variants"][1]["sha256"]);
}
