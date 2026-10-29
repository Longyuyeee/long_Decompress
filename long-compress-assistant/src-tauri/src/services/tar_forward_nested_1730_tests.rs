use super::{
    archive_browser,
    archive_entry_open::{ArchiveEntryOpenCache, NestedArchiveMaterializeResult},
    native_extraction::ExtractionRuntime,
    nested_archive,
    tar_lzo_prefix_175_tests::wrapped,
};
use crate::models::compression::{DecompressOptions, TaskLogSeverity};
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/tar-forward-nested-1730")
}
fn base() -> PathBuf {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/tar-forward-nested-1730");
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn sha(path: &Path) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
}
fn flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}
struct Runtime {
    pool: super::io_buffer_pool::IOBufferPool,
    calls: AtomicUsize,
    hook: Arc<dyn Fn(usize) -> Result<()> + Send + Sync>,
}
impl Runtime {
    fn new() -> Self {
        Self {
            pool: super::io_buffer_pool::IOBufferPool::new(Default::default()),
            calls: AtomicUsize::new(0),
            hook: Arc::new(|_| Ok(())),
        }
    }
}
impl ExtractionRuntime for Runtime {
    fn check_cancellation(&self) -> Result<()> {
        (self.hook)(self.calls.fetch_add(1, Ordering::SeqCst) + 1)
    }
    fn buffer_pool(&self) -> &super::io_buffer_pool::IOBufferPool {
        &self.pool
    }
    fn copy_buffer_size(&self) -> usize {
        65536
    }
    fn normalized_archive_path(&self, path: &Path, _: bool) -> Option<PathBuf> {
        Some(path.into())
    }
    fn emit_log(&self, _: &tauri::Window, _: &str, _: &str, _: TaskLogSeverity) {}
    fn emit_progress(&self, _: &tauri::Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {
    }
}
async fn materialize(
    cache: &ArchiveEntryOpenCache,
    rt: Arc<Runtime>,
    source: &Path,
    entry: &str,
    depth: usize,
    revision: Option<String>,
    cancel: Arc<AtomicBool>,
) -> Result<NestedArchiveMaterializeResult> {
    nested_archive::materialize(
        cache,
        rt,
        source.into(),
        entry.into(),
        None,
        None,
        None,
        depth,
        vec![],
        revision,
        cancel,
        |_, _| async { anyhow::bail!("Link must use the confirmed plan") },
    )
    .await
}
fn file_count(path: &Path) -> usize {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count()
}

fn json(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn selected_readback(input: &Path, output: &Path, encrypted: bool) {
    use super::{extraction_transaction as tx, native_extraction as native};
    let selected = if encrypted {
        vec!["secret.txt".into(), "cover.png".into()]
    } else {
        vec!["readme.txt".into(), "gallery/cover.png".into()]
    };
    let opts = DecompressOptions {
        selected_entries: selected,
        preserve_paths: true,
        preserve_timestamps: true,
        conflict_policy: "rename".into(),
        ..Default::default()
    };
    let mut stage = tx::ExtractionStaging::create_for(output).unwrap();
    let runtime = Runtime::new();
    if encrypted {
        native::seven_zip::extract(
            &runtime,
            None,
            "forward-nested",
            input.to_str().unwrap(),
            stage.path().to_str().unwrap(),
            Some("Nested1730!"),
            &opts,
        )
        .unwrap();
    } else {
        native::zip::extract_with_optional_window(
            &runtime,
            None,
            "forward-nested",
            input.to_str().unwrap(),
            stage.path().to_str().unwrap(),
            None,
            &opts,
        )
        .unwrap();
    }
    tx::prepare_staging_layout(input, stage.path(), &opts).unwrap();
    tx::commit_staged_extraction(input.to_str().unwrap(), stage.path(), output, &opts, |_| {})
        .unwrap();
    stage.cleanup().unwrap();
    assert_eq!(file_count(output), 2);
}
#[tokio::test]
async fn nested1730_forward_aliases_open_preview_extract_and_continue_to_layer_three() {
    use base64::Engine;
    let manifest = json(&root().join("manifest.json"));
    let mut cases = Vec::new();
    let mut evidence = Vec::new();
    for variant in manifest["variants"].as_array().unwrap() {
        for alias in variant["aliases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| !a.as_str().unwrap().ends_with("fake.zip"))
        {
            cases.push((variant.clone(), alias.as_str().unwrap().to_string(), false));
        }
    }
    let gnu = manifest["variants"][1].clone();
    let long = gnu["aliases"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned();
    cases.push((gnu, long, true));
    cases.push((
        manifest["variants"][2].clone(),
        "links/加密副本.7z".into(),
        true,
    ));
    let run = base().join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&run).unwrap();
    for (index, (variant, alias, numeric)) in cases.into_iter().enumerate() {
        let work = run.join(format!("case-{index}"));
        std::fs::create_dir(&work).unwrap();
        let source = root().join(variant["archive"].as_str().unwrap());
        assert_eq!(sha(&source), variant["sha256"]);
        let mut parts = Vec::new();
        if numeric {
            for (i, part) in std::fs::read(&source).unwrap().chunks(512).enumerate() {
                let path = work.join(format!("outer.tar.lzo.{:03}", i + 1));
                std::fs::write(&path, part).unwrap();
                parts.push(path);
            }
            assert!(parts.len() > 1);
        }
        let read_source = parts.last().unwrap_or(&source).clone();
        let cache_root = work.join("cache");
        let cache = ArchiveEntryOpenCache::new(cache_root.clone());
        let revision = super::image_reader::revision(&read_source).unwrap();
        let listing = archive_browser::browse_archive_cancellable(&read_source, None, flag())
            .await
            .unwrap();
        let selected = listing.entries.iter().find(|e| e.path == alias).unwrap();
        assert!(selected.link_target.is_some());
        let receipt = materialize(
            &cache,
            Arc::new(Runtime::new()),
            &read_source,
            &alias,
            2,
            Some(revision.clone()),
            flag(),
        )
        .await
        .unwrap();
        let path = Path::new(&receipt.cache_path);
        assert!(path.ends_with(alias.replace('/', std::path::MAIN_SEPARATOR_STR)));
        assert_eq!(receipt.entry_path, alias);
        let expected = variant["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["name"] == alias)
            .unwrap();
        let target_id = expected["targetId"].as_u64().unwrap() as usize;
        let alias_id = variant["entries"]
            .as_array()
            .unwrap()
            .iter()
            .position(|e| e["name"] == alias)
            .unwrap();
        assert!(target_id > alias_id);
        assert_eq!(sha(path), expected["sha256"]);
        assert_eq!(path.metadata().unwrap().len(), selected.size);
        let seconds = filetime::FileTime::from_last_modification_time(&path.metadata().unwrap())
            .unix_seconds();
        assert_eq!(seconds, expected["seconds"].as_i64().unwrap());
        assert_eq!(file_count(&cache_root), 1);
        let encrypted = alias.ends_with(".7z");
        let password = encrypted.then_some("Nested1730!");
        let child = archive_browser::browse_archive_cancellable(path, password, flag())
            .await
            .unwrap();
        if encrypted {
            assert!(
                archive_browser::browse_archive_cancellable(path, Some("wrong"), flag())
                    .await
                    .is_err()
            );
        }
        let text_name = if encrypted {
            "secret.txt"
        } else {
            "readme.txt"
        };
        let image_name = if encrypted {
            "cover.png"
        } else {
            "gallery/cover.png"
        };
        let text = super::archive_preview::preview_archive_text(path, text_name, password)
            .await
            .unwrap();
        let text_sha = format!("{:x}", Sha256::digest(text.content.as_bytes()));
        assert_eq!(
            text_sha,
            manifest[if encrypted {
                "secretSha256"
            } else {
                "textSha256"
            }]
        );
        let image = super::archive_preview::preview_archive_image(path, image_name, password)
            .await
            .unwrap();
        assert_eq!((image.width, image.height), (32, 24));
        let image_bytes = base64::engine::general_purpose::STANDARD
            .decode(image.data_url.split(',').nth(1).unwrap())
            .unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&image_bytes)),
            manifest["imageSha256"]
        );
        let restored = work.join("selected");
        let selected_input = path.to_path_buf();
        let selected_output = restored.clone();
        tokio::task::spawn_blocking(move || {
            selected_readback(&selected_input, &selected_output, encrypted)
        })
        .await
        .unwrap();
        assert_eq!(
            sha(&restored.join(text_name)),
            manifest[if encrypted {
                "secretSha256"
            } else {
                "textSha256"
            }]
        );
        assert_eq!(sha(&restored.join(image_name)), manifest["imageSha256"]);
        let retained = work.join(if encrypted {
            "retained.7z"
        } else {
            "retained.zip"
        });
        std::fs::copy(path, &retained).unwrap();
        let mut third_receipt = None;
        let mut third_retained = None;
        if !encrypted {
            let runtime = Arc::new(Runtime::new());
            let rt = runtime.clone();
            let parent = path.to_path_buf();
            let input = parent.clone();
            let third = nested_archive::materialize(
                &cache,
                runtime,
                parent,
                "docs/next.zip".into(),
                None,
                None,
                None,
                3,
                vec![receipt.parent_sha256.clone()],
                None,
                flag(),
                move |directory, options| async move {
                    tokio::task::spawn_blocking(move || {
                        super::native_extraction::zip::extract_with_optional_window(
                            rt.as_ref(),
                            None,
                            "third",
                            input.to_str().unwrap(),
                            directory.to_str().unwrap(),
                            None,
                            &options,
                        )
                    })
                    .await?
                },
            )
            .await
            .unwrap();
            assert_eq!(third.depth, 3);
            assert_eq!(third.content_sha256, manifest["thirdSha256"]);
            let preview = super::archive_preview::preview_archive_text(
                Path::new(&third.cache_path),
                "third.txt",
                None,
            )
            .await
            .unwrap();
            assert_eq!(preview.content, "Nested third layer 1730\n");
            assert!(nested_archive::materialize(
                &cache,
                Arc::new(Runtime::new()),
                PathBuf::from(&third.cache_path),
                "anything.zip".into(),
                None,
                None,
                None,
                4,
                vec![],
                None,
                flag(),
                |_, _| async { anyhow::bail!("must reject depth first") }
            )
            .await
            .is_err());
            let keep = work.join("third.zip");
            std::fs::copy(&third.cache_path, &keep).unwrap();
            third_retained = Some(keep);
            third_receipt = Some(third);
        }
        assert_eq!(sha(&source), variant["sha256"]);
        drop(cache);
        assert_eq!(file_count(&cache_root), 0);
        assert!(!path.exists());
        evidence.push(serde_json::json!({"variant":variant["format"],"source":source,"readSource":read_source,"parts":parts,"sourceSha256":variant["sha256"],"alias":alias,"receipt":receipt,"retained":retained,"seconds":seconds,"outer":listing,"child":child,"textSha256":text_sha,"imageSha256":manifest["imageSha256"],"selected":restored,"encrypted":encrypted,"thirdReceipt":third_receipt,"thirdRetained":third_retained,"cacheCleaned":true}));
    }
    std::fs::write(
        base().join("native.json"),
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
}

#[tokio::test]
async fn nested1730_source_revision_change_cancellation_and_storage_failure_keep_outer_and_release_private_cache(
) {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("outer.tar.lzo");
    std::fs::copy(root().join("gnu.tar.lzo"), &source).unwrap();
    let cache_root = temp.path().join("cache");
    let cache = ArchiveEntryOpenCache::new(cache_root.clone());
    let old = super::image_reader::revision(&source).unwrap();
    filetime::set_file_mtime(&source, filetime::FileTime::from_unix_time(1900001730, 0)).unwrap();
    let error = materialize(
        &cache,
        Arc::new(Runtime::new()),
        &source,
        "links/copy.zip",
        2,
        Some(old),
        flag(),
    )
    .await
    .unwrap_err();
    assert!(format!("{error:#}").contains("IMAGE_READER_CHANGED"));
    for mode in ["change", "cancel", "storage"] {
        let mut rt = Runtime::new();
        let cancel = flag();
        let signal = cancel.clone();
        let path = source.clone();
        let original = sha(&source);
        rt.hook = Arc::new(move |n| {
            if n == 2 {
                match mode {
                    "change" => filetime::set_file_mtime(
                        &path,
                        filetime::FileTime::from_unix_time(1900001724, 0),
                    )
                    .unwrap(),
                    "cancel" => signal.store(true, Ordering::SeqCst),
                    _ => return Err(std::io::Error::from_raw_os_error(112).into()),
                }
            }
            Ok(())
        });
        let error = materialize(
            &cache,
            Arc::new(rt),
            &source,
            "links/copy.zip",
            2,
            None,
            cancel,
        )
        .await
        .unwrap_err();
        let message = format!("{error:#}");
        assert!(
            message.contains(match mode {
                "change" => "变化",
                "cancel" => "取消",
                _ => "112",
            }) || mode == "cancel" && message.contains("cancel"),
            "{mode}: {message}"
        );
        assert_eq!(sha(&source), original);
        assert_eq!(file_count(&cache_root), 0);
    }
}

#[tokio::test]
async fn nested1730_unrecognized_alias_damaged_tail_missing_volume_and_bad_depth_do_not_leave_results(
) {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("outer.tar.lzo");
    std::fs::copy(root().join("gnu.tar.lzo"), &source).unwrap();
    let cache_root = temp.path().join("cache");
    let cache = ArchiveEntryOpenCache::new(cache_root.clone());
    let error = materialize(
        &cache,
        Arc::new(Runtime::new()),
        &source,
        "links/fake.zip",
        2,
        None,
        flag(),
    )
    .await
    .unwrap_err();
    assert!(format!("{error:#}").contains("不是可识别"));
    for depth in [3, 4] {
        assert!(materialize(
            &cache,
            Arc::new(Runtime::new()),
            &source,
            "links/copy.zip",
            depth,
            None,
            flag()
        )
        .await
        .is_err());
    }
    let raw = std::fs::read(root().join("gnu.tar")).unwrap();
    std::fs::write(&source, wrapped(&raw[..raw.len() / 2], 100003, true)).unwrap();
    assert!(materialize(
        &cache,
        Arc::new(Runtime::new()),
        &source,
        "links/copy.zip",
        2,
        None,
        flag()
    )
    .await
    .is_err());
    std::fs::write(temp.path().join("parts.tar.lzo.001"), b"first incomplete").unwrap();
    std::fs::write(temp.path().join("parts.tar.lzo.003"), b"third").unwrap();
    assert!(materialize(
        &cache,
        Arc::new(Runtime::new()),
        &temp.path().join("parts.tar.lzo.001"),
        "links/copy.zip",
        2,
        None,
        flag()
    )
    .await
    .is_err());
    assert_eq!(file_count(&cache_root), 0);
}

#[tokio::test]
async fn nested1730_ordinary_nested_route_is_preserved_and_hash_validation_can_cancel() {
    let source = root().join("gnu.tar.lzo");
    let temp = tempfile::tempdir().unwrap();
    let cache = ArchiveEntryOpenCache::new(temp.path().join("cache"));
    let runtime = Arc::new(Runtime::new());
    let rt = runtime.clone();
    let path = source.clone();
    let called = Arc::new(AtomicBool::new(false));
    let observed = called.clone();
    let receipt = nested_archive::materialize(
        &cache,
        runtime,
        source,
        "ordinary.zip".into(),
        None,
        None,
        None,
        2,
        vec![],
        None,
        flag(),
        move |directory, options: DecompressOptions| async move {
            observed.store(true, Ordering::SeqCst);
            let plan = super::lzop::selected::Plan::build(&path, &options, &flag())?.unwrap();
            plan.extract(
                rt.as_ref(),
                None,
                "ordinary",
                &path,
                &directory,
                &options,
                &flag(),
            )
        },
    )
    .await
    .unwrap();
    assert!(called.load(Ordering::SeqCst));
    assert_eq!(
        archive_browser::detect_format(Path::new(&receipt.cache_path)).unwrap(),
        super::archive_format::ArchiveFormat::Zip
    );
    let calls = AtomicUsize::new(0);
    let error = super::archive_entry_open::validate_nested_archive_identity_controlled(
        &root().join("gnu.tar"),
        Path::new(&receipt.cache_path),
        2,
        &[],
        || {
            anyhow::ensure!(
                calls.fetch_add(1, Ordering::SeqCst) < 2,
                "cancel during hash"
            );
            Ok(())
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("cancel during hash"));
}
