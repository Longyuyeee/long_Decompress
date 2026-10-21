//! Actual content evidence for one explicitly requested pair of source revisions.
use super::source_comparison::{self as catalog, Entry, Source, SourceRef};
use super::zip_password::ZipPasswordEncoding;
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

#[derive(Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    #[serde(default)]
    pub password: String,
    pub encoding: Option<ZipPasswordEncoding>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub left: SourceRef,
    pub right: SourceRef,
    pub paths: Vec<String>,
    #[serde(default)]
    pub left_credentials: Credentials,
    #[serde(default)]
    pub right_credentials: Credentials,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Verdict {
    pub path: String,
    pub kind: String,
    pub reason: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Batch {
    pub entries: Vec<Verdict>,
    pub stopped: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub processed_bytes: u64,
    pub total_bytes: u64,
    pub completed: usize,
    pub total: usize,
    pub path: String,
    pub side: String,
}
fn check(flag: &AtomicBool) -> Result<()> {
    ensure!(!flag.load(Ordering::SeqCst), "内容核对已停止");
    Ok(())
}
fn current(reference: &SourceRef, flag: Arc<AtomicBool>) -> Result<Source> {
    let source = catalog::read(Path::new(&reference.path), flag, |_| {})?;
    ensure!(
        source.source.kind == reference.kind && source.source.revision == reference.revision,
        "[comparison:changed] 来源已变化，请重新读取后核对；旧内容结论已撤销"
    );
    Ok(source)
}
type Archive = zip_aes::ZipArchive<super::numeric_volume_reader::ZipInputReader>;
fn digest(
    source: &Source,
    entry: &Entry,
    zip: &mut Option<Archive>,
    credentials: &Credentials,
    flag: &AtomicBool,
    mut on_bytes: impl FnMut(u64),
) -> Result<Vec<u8>> {
    check(flag)?;
    let file_path = Path::new(&source.source.path).join(&entry.path);
    if zip.is_none() {
        ensure!(
            catalog::identity(&fs::symlink_metadata(&file_path)?)? == entry.fingerprint,
            "[comparison:changed] 文件已变化，请重新读取来源"
        );
    }
    let is_zip = zip.is_some();
    let mut input: Box<dyn Read> = if let Some(zip) = zip.as_mut() {
        Box::new(catalog::open_entry(
            zip,
            &entry.raw,
            Some(&credentials.password),
            credentials.encoding,
        )?)
    } else {
        Box::new(fs::File::open(&file_path)?)
    };
    let mut hash = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 65536];
    loop {
        check(flag)?;
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        bytes = bytes.checked_add(n as u64).context("读取字节超出范围")?;
        ensure!(
            bytes <= entry.size,
            if is_zip { "ZIP 条目实际大小与目录记录不一致，本项无法完成内容核对；请重新读取或提取可用内容" } else { "[comparison:changed] 内容大小已变化，请重新读取来源" }
        );
        hash.update(&buffer[..n]);
        on_bytes(n as u64);
    }
    ensure!(
        bytes == entry.size,
        if is_zip { "ZIP 条目实际大小与目录记录不一致，本项无法完成内容核对；请重新读取或提取可用内容" } else { "[comparison:changed] 内容大小已变化，请重新读取来源" }
    );
    drop(input);
    if zip.is_none() {
        ensure!(
            catalog::identity(&fs::symlink_metadata(file_path)?)? == entry.fingerprint,
            "[comparison:changed] 核对期间文件已变化，请重新读取来源"
        );
    }
    Ok(hash.finalize().to_vec())
}
pub fn run(
    request: &Request,
    flag: Arc<AtomicBool>,
    mut progress: impl FnMut(Progress),
    mut result: impl FnMut(Verdict),
) -> Result<Batch> {
    ensure!(
        !request.paths.is_empty()
            && request.paths.len() <= 2000
            && serde_json::to_vec(&request.paths)?.len() <= 60000,
        "每次最多核对 2000 项，请缩小筛选或所选范围"
    );
    let unique: BTreeSet<_> = request.paths.iter().collect();
    ensure!(
        unique.len() == request.paths.len(),
        "核对范围有重复路径，请重新选择"
    );
    check(&flag)?;
    let left = current(&request.left, flag.clone())?;
    let right = current(&request.right, flag.clone())?;
    let a: std::collections::BTreeMap<_, _> =
        left.entries.iter().map(|e| (e.path.as_str(), e)).collect();
    let b: std::collections::BTreeMap<_, _> =
        right.entries.iter().map(|e| (e.path.as_str(), e)).collect();
    let pairs = request
        .paths
        .iter()
        .map(|path| {
            let l = a
                .get(path.as_str())
                .context("左侧所选路径不存在，请重新读取")?;
            let r = b
                .get(path.as_str())
                .context("右侧所选路径不存在，请重新读取")?;
            ensure!(
                !l.directory && !r.directory,
                "内容核对请选择两边均存在的普通文件"
            );
            Ok((path, *l, *r))
        })
        .collect::<Result<Vec<_>>>()?;
    let total_bytes = pairs
        .iter()
        .filter(|(_, l, r)| l.size == r.size)
        .try_fold(0u64, |n, (_, l, r)| {
            n.checked_add(l.size)?.checked_add(r.size)
        })
        .context("核对大小超出范围")?;
    let mut left_zip = if left.source.kind == "zip" {
        Some(catalog::open_zip(&left)?)
    } else {
        None
    };
    let mut right_zip = if right.source.kind == "zip" {
        Some(catalog::open_zip(&right)?)
    } else {
        None
    };
    let mut processed_bytes = 0;
    let mut completed = 0;
    let mut entries = Vec::new();
    let mut gate = super::progress_gate::ProgressGate::new();
    progress(Progress {
        processed_bytes,
        total_bytes,
        completed,
        total: pairs.len(),
        path: String::new(),
        side: String::new(),
    });
    for (path, l, r) in &pairs {
        if flag.load(Ordering::SeqCst) {
            break;
        }
        let value = if l.size != r.size {
            Ok(("different", "文件大小不同，内容必然不同"))
        } else {
            let mut read_side = |source: &Source,
                                 entry: &Entry,
                                 zip: &mut Option<Archive>,
                                 credentials: &Credentials,
                                 side: &str| {
                digest(source, entry, zip, credentials, &flag, |count| {
                    processed_bytes += count;
                    if gate.allow(false) {
                        progress(Progress {
                            processed_bytes,
                            total_bytes,
                            completed,
                            total: pairs.len(),
                            path: (*path).clone(),
                            side: side.into(),
                        })
                    }
                })
            };
            read_side(&left, l, &mut left_zip, &request.left_credentials, "left").and_then(
                |lhash| {
                    read_side(
                        &right,
                        r,
                        &mut right_zip,
                        &request.right_credentials,
                        "right",
                    )
                    .map(|rhash| {
                        if lhash != rhash {
                            ("different", "已读取双方完整内容，SHA-256 不同")
                        } else if l.modified != r.modified {
                            (
                                "metadata",
                                "已读取双方完整内容，SHA-256 相同；仅记录时间不同或缺失",
                            )
                        } else {
                            ("same", "已读取双方完整内容，SHA-256 相同")
                        }
                    })
                },
            )
        };
        if flag.load(Ordering::SeqCst) {
            break;
        }
        let verdict = match value {
            Ok((kind, reason)) => Verdict {
                path: (*path).clone(),
                kind: kind.into(),
                reason: reason.into(),
            },
            Err(error) => {
                let reason = format!("{error:#}");
                if reason.contains("[comparison:changed]") {
                    return Err(error);
                }
                Verdict {
                    path: (*path).clone(),
                    kind: "unreadable".into(),
                    reason: format!("本项未完成核对：{reason}；可补密码、补卷或重新读取后继续"),
                }
            }
        };
        completed += 1;
        result(verdict.clone());
        entries.push(verdict);
        progress(Progress {
            processed_bytes,
            total_bytes,
            completed,
            total: pairs.len(),
            path: (*path).clone(),
            side: String::new(),
        });
    }
    let stopped = flag.load(Ordering::SeqCst);
    if !stopped {
        current(&request.left, flag.clone())?;
        current(&request.right, flag.clone())?;
    }
    Ok(Batch { entries, stopped })
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip_aes::unstable::write::FileOptionsExt;
    use std::{
        io::Write,
        path::{Path, PathBuf},
    };
    use zip_aes::{write::SimpleFileOptions, CompressionMethod, ZipWriter};
    fn flag() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }
    fn folders() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("left");
        let b = dir.path().join("right");
        fs::create_dir(&a).unwrap();
        fs::create_dir(&b).unwrap();
        (dir, a, b)
    }
    fn put(root: &Path, name: &str, bytes: &[u8], time: i64) {
        fs::write(root.join(name), bytes).unwrap();
        filetime::set_file_mtime(root.join(name), filetime::FileTime::from_unix_time(time, 0))
            .unwrap();
    }
    fn request(a: &Path, b: &Path, paths: &[&str]) -> Request {
        Request {
            left: catalog::read(a, flag(), |_| {}).unwrap().source,
            right: catalog::read(b, flag(), |_| {}).unwrap().source,
            paths: paths.iter().map(|s| s.to_string()).collect(),
            left_credentials: Credentials::default(),
            right_credentials: Credentials::default(),
        }
    }
    fn protected(path: &Path, password: &[u8], aes: bool, payload: &[u8]) {
        let mut zip = ZipWriter::new(fs::File::create(path).unwrap());
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let options = if aes {
            options.with_aes_encryption_bytes(zip_aes::AesMode::Aes256, password)
        } else {
            options.with_deprecated_encryption(password).unwrap()
        };
        zip.start_file("paired.bin", options).unwrap();
        zip.write_all(payload).unwrap();
        zip.start_file("public.txt", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"public").unwrap();
        zip.finish().unwrap();
    }
    #[test]
    fn comparison1555_content_distinguishes_equal_metadata_and_equal_content_with_other_times() {
        let (_dir, a, b) = folders();
        for (name, one, two, ta, tb) in [
            ("different", b"aaaa".as_slice(), b"bbbb".as_slice(), 10, 10),
            ("same", b"data", b"data", 10, 10),
            ("metadata", b"equal", b"equal", 10, 20),
            ("size", b"a", b"two", 10, 10),
        ] {
            put(&a, name, one, ta);
            put(&b, name, two, tb);
        }
        let request = request(&a, &b, &["different", "same", "metadata", "size"]);
        let mut bytes = 0;
        let batch = run(&request, flag(), |p| bytes = p.processed_bytes, |_| {}).unwrap();
        assert_eq!(
            batch
                .entries
                .iter()
                .map(|e| e.kind.as_str())
                .collect::<Vec<_>>(),
            ["different", "same", "metadata", "different"]
        );
        assert_eq!(bytes, 26);
        assert!(!batch.stopped);
    }
    #[test]
    fn comparison1555_password_errors_can_be_corrected_and_selected_content_is_retrievable() {
        for aes in [false, true] {
            let (dir, a, _) = folders();
            put(&a, "paired.bin", b"protected payload", 10);
            let zip = dir.path().join("source.zip");
            let password =
                super::super::zip_password::encode("原包密码", ZipPasswordEncoding::Cp936).unwrap();
            protected(&zip, &password, aes, b"protected payload");
            let original = fs::read(&zip).unwrap();
            let mut request = request(&a, &zip, &["paired.bin"]);
            let failed = run(&request, flag(), |_| {}, |_| {}).unwrap();
            assert_eq!(failed.entries[0].kind, "unreadable");
            request.right_credentials = Credentials {
                password: "原包密码".into(),
                encoding: Some(ZipPasswordEncoding::Cp936),
            };
            let fixed = run(&request, flag(), |_| {}, |_| {}).unwrap();
            assert!(matches!(
                fixed.entries[0].kind.as_str(),
                "same" | "metadata"
            ));
            let selection = catalog::Selection {
                source: request.right.clone(),
                paths: vec!["paired.bin".into()],
                keep_structure: true,
                conflict_policy: "rename".into(),
                zip_password_encoding: Some(ZipPasswordEncoding::Cp936),
            };
            let out = dir.path().join("out");
            let source = catalog::validate(&selection, &out, flag()).unwrap();
            assert!(catalog::validate_password(&selection, &source, None).is_err());
            catalog::validate_password(&selection, &source, Some("原包密码")).unwrap();
            catalog::retrieve_with_password(&selection, &out, Some("原包密码"), flag(), |_| {})
                .unwrap();
            assert_eq!(
                fs::read(out.join("paired.bin")).unwrap(),
                b"protected payload"
            );
            assert!(!out.join("public.txt").exists());
            assert_eq!(fs::read(zip).unwrap(), original);
        }
    }
    #[test]
    fn comparison1555_numeric_and_standard_password_volumes_use_any_member_and_detect_changes() {
        for standard in [false, true] {
            let (dir, a, _) = folders();
            let payload = vec![81; 150000];
            put(&a, "paired.bin", &payload, 10);
            let zip = dir.path().join("original.zip");
            protected(&zip, b"key", true, &payload);
            let main = dir.path().join("group.zip");
            let members = if standard {
                super::super::standard_zip_writer::split(&zip, &main, 65536, || Ok(()), |_, _| {})
                    .unwrap();
                super::super::split_archive_detector::SplitArchiveDetector::detect_split_archive(
                    &main,
                )
                .unwrap()
                .unwrap()
                .parts
            } else {
                let bytes = fs::read(&zip).unwrap();
                bytes
                    .chunks(55000)
                    .enumerate()
                    .map(|(i, data)| {
                        let path = dir.path().join(format!("group.zip.{:03}", i + 1));
                        fs::write(&path, data).unwrap();
                        path
                    })
                    .collect::<Vec<_>>()
            };
            let mut request = request(&a, members.last().unwrap(), &["paired.bin"]);
            request.right_credentials.password = "key".into();
            let checked = run(&request, flag(), |_| {}, |_| {}).unwrap();
            assert!(matches!(
                checked.entries[0].kind.as_str(),
                "same" | "metadata"
            ));
            let selection = catalog::Selection {
                source: request.right.clone(),
                paths: vec!["paired.bin".into()],
                keep_structure: true,
                conflict_policy: "rename".into(),
                zip_password_encoding: None,
            };
            let output = dir.path().join("output");
            catalog::retrieve_with_password(&selection, &output, Some("key"), flag(), |_| {})
                .unwrap();
            assert_eq!(fs::read(output.join("paired.bin")).unwrap(), payload);
            let changed = &members[1];
            filetime::set_file_mtime(changed, filetime::FileTime::from_unix_time(77, 0)).unwrap();
            assert!(run(&request, flag(), |_| {}, |_| {}).is_err());
            fs::remove_file(changed).unwrap();
            assert!(catalog::read(members.first().unwrap(), flag(), |_| {}).is_err());
        }
    }
    #[test]
    fn comparison1555_stop_keeps_completed_verdicts_and_remaining_scope_can_continue() {
        let (_dir, a, b) = folders();
        for name in ["one", "two", "three"] {
            put(&a, name, b"data", 10);
            put(&b, name, b"data", 10);
        }
        let mut request = request(&a, &b, &["one", "two", "three"]);
        let stop = flag();
        let control = stop.clone();
        let batch = run(
            &request,
            stop,
            |_| {},
            |_| control.store(true, Ordering::SeqCst),
        )
        .unwrap();
        assert!(batch.stopped);
        assert_eq!(batch.entries.len(), 1);
        assert_eq!(batch.entries[0].kind, "same");
        request.paths = vec!["two".into(), "three".into()];
        assert_eq!(
            run(&request, flag(), |_| {}, |_| {}).unwrap().entries.len(),
            2
        );
    }
    #[test]
    fn comparison1555_source_change_during_read_revokes_the_batch() {
        let (_dir, a, b) = folders();
        put(&a, "large", &vec![3; 4 * 1024 * 1024], 10);
        put(&b, "large", &vec![3; 4 * 1024 * 1024], 10);
        let request = request(&a, &b, &["large"]);
        let mut changed = false;
        let outcome = run(
            &request,
            flag(),
            |p| {
                if p.processed_bytes > 0 && !changed {
                    changed = true;
                    filetime::set_file_mtime(
                        a.join("large"),
                        filetime::FileTime::from_unix_time(123, 0),
                    )
                    .unwrap();
                }
            },
            |_| {},
        );
        assert!(outcome
            .err()
            .unwrap()
            .to_string()
            .contains("[comparison:changed]"));
    }
    #[test]
    fn comparison1555_public_files_in_mixed_zip_do_not_require_other_entries_password() {
        let (dir, a, _) = folders();
        put(&a, "public.txt", b"public", 10);
        let zip = dir.path().join("mixed.zip");
        protected(&zip, b"unknown", false, b"private");
        let request = request(&a, &zip, &["public.txt"]);
        let batch = run(&request, flag(), |_| {}, |_| {}).unwrap();
        assert!(matches!(
            batch.entries[0].kind.as_str(),
            "same" | "metadata"
        ));
    }
    #[test]
    fn comparison1555_invalid_or_stale_scope_does_not_claim_content_equality() {
        let (_dir, a, b) = folders();
        put(&a, "one", b"data", 10);
        put(&b, "one", b"data", 10);
        let mut request = request(&a, &b, &["one", "one"]);
        assert!(run(&request, flag(), |_| {}, |_| {}).is_err());
        request.paths = vec!["missing".into()];
        assert!(run(&request, flag(), |_| {}, |_| {}).is_err());
        request.paths = vec!["one".into()];
        put(&b, "one", b"new data", 20);
        assert!(run(&request, flag(), |_| {}, |_| {}).is_err());
    }
}
