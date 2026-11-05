use super::archive_clipboard::{cleanup, PreparedCopy, CACHE_TTL};
use super::{archive_browser, compression_service::CompressionService, native_extraction::zip};
use crate::models::compression::DecompressOptions;
use std::{
    fs,
    io::{Cursor, Write},
    path::Path,
    time::{Duration, SystemTime},
};

fn zip_source(path: &Path, password: Option<&str>) {
    let mut writer = zip_aes::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in [
        ("资料/a.txt", "中文 content"),
        ("资料/子目录/b.txt", "second"),
        ("outside.txt", "keep outside"),
    ] {
        let options = zip_aes::write::SimpleFileOptions::default();
        let options = password.map_or(options, |p| {
            options.with_aes_encryption(zip_aes::AesMode::Aes256, p)
        });
        writer.start_file(name, options).unwrap();
        writer.write_all(content.as_bytes()).unwrap();
    }
    writer
        .add_directory("空目录/", zip_aes::write::SimpleFileOptions::default())
        .unwrap();
    fs::write(path, writer.finish().unwrap().into_inner()).unwrap();
}
#[tokio::test]
async fn clipboard1740_plain_password_numeric_copy_tree_and_empty_directory() {
    let temp = tempfile::tempdir().unwrap();
    for (label, password, numeric) in [
        ("plain", None, false),
        ("password", Some("密码 test 1740"), false),
        ("numeric", Some("volume"), true),
    ] {
        let source = temp.path().join(format!("{label}.zip"));
        zip_source(&source, password);
        let original = fs::read(&source).unwrap();
        let input = if numeric {
            for (i, bytes) in original.chunks(original.len() / 3).enumerate() {
                fs::write(temp.path().join(format!("numeric.zip.{:03}", i + 1)), bytes).unwrap();
            }
            temp.path().join("numeric.zip.001")
        } else {
            source.clone()
        };
        let listing = archive_browser::browse_archive(&input, password)
            .await
            .unwrap();
        let prepared = PreparedCopy::new(
            &temp.path().join("cache"),
            &listing.entries,
            &["资料/".into(), "资料/a.txt".into(), "空目录/".into()],
        )
        .unwrap();
        assert_eq!(prepared.roots, vec!["空目录", "资料"]);
        let options = DecompressOptions {
            selected_entries: prepared.roots.clone(),
            preserve_paths: true,
            preserve_timestamps: true,
            conflict_policy: "skip".into(),
            ..Default::default()
        };
        let path = prepared.path().to_path_buf();
        let runtime = CompressionService::for_testing();
        std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    zip::extract_with_optional_window(
                        &runtime,
                        None,
                        "clipboard1740",
                        input.to_str().unwrap(),
                        path.to_str().unwrap(),
                        password,
                        &options,
                    )
                    .unwrap()
                })
                .join()
                .unwrap()
        });
        let paths = prepared.validate().unwrap();
        assert_eq!(paths.len(), 2);
        assert!(paths[0].is_dir());
        assert!(!path.join("outside.txt").exists());
        assert_eq!(
            fs::read_to_string(path.join("资料/a.txt")).unwrap(),
            "中文 content"
        );
        assert_eq!(
            fs::read_to_string(path.join("资料/子目录/b.txt")).unwrap(),
            "second"
        );
        // A consumer pastes roots by name, rather than introducing the cache or archive's parent folders.
        let destination = temp.path().join(format!("paste-{label}"));
        fs::create_dir(&destination).unwrap();
        for root in &paths {
            for item in walkdir::WalkDir::new(root) {
                let item = item.unwrap();
                let out = destination
                    .join(root.file_name().unwrap())
                    .join(item.path().strip_prefix(root).unwrap());
                if item.file_type().is_dir() {
                    fs::create_dir_all(out).unwrap();
                } else {
                    fs::copy(item.path(), out).unwrap();
                }
            }
        }
        assert_eq!(
            fs::read(destination.join("资料/a.txt")).unwrap(),
            "中文 content".as_bytes()
        );
        assert!(destination.join("空目录").is_dir());
        prepared.persist();
        assert!(path.exists());
        assert_eq!(fs::read(&source).unwrap(), original);
    }
}
#[tokio::test]
async fn clipboard1740_bad_password_corruption_missing_volume_and_cancel_discard_cache() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.zip");
    zip_source(&source, Some("right"));
    let listing = archive_browser::browse_archive(&source, Some("right"))
        .await
        .unwrap();
    for (label, password, cancelled) in [
        ("password", Some("wrong"), false),
        ("cancel", Some("right"), true),
    ] {
        let prepared = PreparedCopy::new(
            &temp.path().join(label),
            &listing.entries,
            &["资料/a.txt".into()],
        )
        .unwrap();
        let output = prepared.path().to_path_buf();
        let runtime = CompressionService::for_testing();
        runtime
            .cancellation_flag
            .store(cancelled, std::sync::atomic::Ordering::SeqCst);
        let options = DecompressOptions {
            selected_entries: prepared.roots.clone(),
            preserve_paths: true,
            ..Default::default()
        };
        let result = std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    zip::extract_with_optional_window(
                        &runtime,
                        None,
                        "clipboard1740",
                        source.to_str().unwrap(),
                        output.to_str().unwrap(),
                        password,
                        &options,
                    )
                })
                .join()
                .unwrap()
        });
        assert!(result.is_err(), "{label}");
        assert!(prepared.validate().is_err());
        drop(prepared);
        assert!(!output.exists());
    }
    let original = fs::read(&source).unwrap();
    for (i, chunk) in original.chunks(original.len() / 3).enumerate() {
        fs::write(temp.path().join(format!("part.zip.{:03}", i + 1)), chunk).unwrap();
    }
    fs::remove_file(temp.path().join("part.zip.002")).unwrap();
    assert!(
        archive_browser::browse_archive(&temp.path().join("part.zip.001"), Some("right"))
            .await
            .is_err()
    );
    // Metadata still lists this file; a corrupt payload must not become a completed clipboard tree.
    let mut bytes = original.clone();
    let data = zip_aes::ZipArchive::new(Cursor::new(&bytes))
        .unwrap()
        .by_index_raw(0)
        .unwrap()
        .data_start()
        .unwrap() as usize;
    bytes[data + 20] ^= 0x40;
    fs::write(&source, bytes).unwrap();
    let prepared = PreparedCopy::new(
        &temp.path().join("bad"),
        &listing.entries,
        &["资料/a.txt".into()],
    )
    .unwrap();
    let out = prepared.path().to_path_buf();
    let runtime = CompressionService::for_testing();
    assert!(std::thread::scope(|scope| scope
        .spawn(|| zip::extract_with_optional_window(
            &runtime,
            None,
            "clipboard1740",
            source.to_str().unwrap(),
            out.to_str().unwrap(),
            Some("right"),
            &DecompressOptions {
                selected_entries: prepared.roots.clone(),
                preserve_paths: true,
                ..Default::default()
            }
        ))
        .join()
        .unwrap())
    .is_err());
    drop(prepared);
    assert!(!out.exists());
}
#[tokio::test]
async fn clipboard1740_rejects_missing_colliding_changed_and_incomplete_selection() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.zip");
    zip_source(&source, None);
    let listing = archive_browser::browse_archive(&source, None)
        .await
        .unwrap();
    assert!(PreparedCopy::new(temp.path(), &listing.entries, &[]).is_err());
    assert!(PreparedCopy::new(temp.path(), &listing.entries, &["missing.txt".into()]).is_err());
    let mut entries = listing.entries.clone();
    let mut another = entries[0].clone();
    another.path = "else/a.txt".into();
    entries.push(another);
    assert!(PreparedCopy::new(
        temp.path(),
        &entries,
        &["资料/a.txt".into(), "else/a.txt".into()]
    )
    .is_err());
    let prepared =
        PreparedCopy::new(temp.path(), &listing.entries, &["资料/a.txt".into()]).unwrap();
    fs::create_dir_all(prepared.path().join("资料")).unwrap();
    fs::write(prepared.path().join("资料/a.txt"), b"half").unwrap();
    assert!(prepared.validate().is_err());
    let revision = super::image_reader::revision(&source).unwrap();
    fs::write(&source, b"replaced source").unwrap();
    assert!(super::image_reader::check_source(&source, &revision).is_err());
}
#[test]
fn clipboard1740_expired_cache_cleanup_keeps_current_clipboard_and_unrelated_paths() {
    let temp = tempfile::tempdir().unwrap();
    for name in ["copy-old", "copy-active", "other"] {
        fs::create_dir(temp.path().join(name)).unwrap();
        filetime::set_file_mtime(
            temp.path().join(name),
            filetime::FileTime::from_system_time(
                SystemTime::now() - CACHE_TTL - Duration::from_secs(60),
            ),
        )
        .unwrap();
    }
    cleanup(
        temp.path(),
        &[temp.path().join("copy-active/file.txt")],
        SystemTime::now(),
    )
    .unwrap();
    assert!(!temp.path().join("copy-old").exists());
    assert!(temp.path().join("copy-active").exists());
    assert!(temp.path().join("other").exists());
}
#[cfg(windows)]
#[test]
#[ignore = "publishes a real Windows CF_HDROP; run with the preserving STA wrapper"]
fn clipboard1740_windows_file_drop_roundtrip_and_paste_after_cache_drop() {
    use super::archive_clipboard::windows;
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.zip");
    zip_source(&source, None);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let listing = runtime
        .block_on(archive_browser::browse_archive(&source, None))
        .unwrap();
    let external = std::env::var_os("LONG_COPY_TEST_ROOT").map(std::path::PathBuf::from);
    let cache = external
        .as_ref()
        .map(|p| p.join("cache"))
        .unwrap_or_else(|| temp.path().join("cache"));
    let prepared = PreparedCopy::new(
        &cache,
        &listing.entries,
        &["资料/".into(), "空目录/".into()],
    )
    .unwrap();
    let options = DecompressOptions {
        selected_entries: prepared.roots.clone(),
        preserve_paths: true,
        ..Default::default()
    };
    zip::extract_with_optional_window(
        &CompressionService::for_testing(),
        None,
        "clipboard1740",
        source.to_str().unwrap(),
        prepared.path().to_str().unwrap(),
        None,
        &options,
    )
    .unwrap();
    let paths = prepared.validate().unwrap();
    windows::publish(&paths).unwrap();
    prepared.persist();
    assert_eq!(windows::read_files().unwrap(), paths);
    assert_eq!(
        fs::read_to_string(paths[1].join("a.txt")).unwrap(),
        "中文 content"
    );
    assert!(paths[0].is_dir());
    if let Some(root) = external {
        fs::write(
            root.join("clipboard-receipt.json"),
            serde_json::to_vec_pretty(&serde_json::json!({"paths":paths,"text":"中文 content"}))
                .unwrap(),
        )
        .unwrap();
    }
    println!(
        "CF_HDROP UTF-16 paths, multi-selection, persisted files and empty directory: confirmed"
    );
}

#[tokio::test]
async fn clipboard1740_sevenz_password_and_tar_use_the_same_selected_copy_plan() {
    let temp = tempfile::tempdir().unwrap();
    let original = temp.path().join("照片 说明.txt");
    fs::write(&original, "common engines").unwrap();
    for encrypted in [false, true] {
        let path = temp.path().join(if encrypted {
            "protected.7z"
        } else {
            "plain.7z"
        });
        if encrypted {
            sevenz_rust::compress_to_path_encrypted(
                &original,
                &path,
                sevenz_rust::Password::from("seven password"),
            )
            .unwrap()
        } else {
            sevenz_rust::compress_to_path(&original, &path).unwrap()
        }
        let password = encrypted.then_some("seven password");
        let listing = archive_browser::browse_archive(&path, password)
            .await
            .unwrap();
        let chosen = listing.entries.iter().find(|e| !e.is_dir).unwrap();
        let prepared = PreparedCopy::new(
            &temp.path().join("cache"),
            &listing.entries,
            &[chosen.path.clone()],
        )
        .unwrap();
        let options = DecompressOptions {
            selected_entries: prepared.roots.clone(),
            preserve_paths: true,
            ..Default::default()
        };
        let runtime = CompressionService::for_testing();
        std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    super::native_extraction::seven_zip::extract(
                        &runtime,
                        None,
                        "clipboard1740",
                        path.to_str().unwrap(),
                        prepared.path().to_str().unwrap(),
                        password,
                        &options,
                    )
                })
                .join()
                .unwrap()
        })
        .unwrap();
        let paths = prepared.validate().unwrap();
        assert_eq!(fs::read_to_string(&paths[0]).unwrap(), "common engines");
    }
    let path = temp.path().join("sample.tar");
    let mut builder = tar::Builder::new(fs::File::create(&path).unwrap());
    let bytes = b"tar content";
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "docs/readme.txt", &bytes[..])
        .unwrap();
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Directory);
    header.set_size(0);
    header.set_mode(0o755);
    header.set_cksum();
    builder
        .append_data(&mut header, "empty/", std::io::empty())
        .unwrap();
    builder.finish().unwrap();
    drop(builder);
    let listing = archive_browser::browse_archive(&path, None).await.unwrap();
    let prepared = PreparedCopy::new(
        &temp.path().join("cache"),
        &listing.entries,
        &["docs".into(), "empty".into()],
    )
    .unwrap();
    let options = DecompressOptions {
        selected_entries: prepared.roots.clone(),
        preserve_paths: true,
        ..Default::default()
    };
    let runtime = CompressionService::for_testing();
    super::native_extraction::tar::extract_inner(
        &runtime,
        None,
        "clipboard1740",
        path.to_str().unwrap(),
        prepared.path(),
        None,
        &options,
    )
    .unwrap();
    let paths = prepared.validate().unwrap();
    assert!(paths[1].is_dir());
    assert_eq!(
        fs::read_to_string(paths[0].join("readme.txt")).unwrap(),
        "tar content"
    );
}
