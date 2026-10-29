//! Independent non-seekable CPython originals through the real recovery flow.
use super::{
    archive_diagnostics::{diagnose_archive, repair_zip_to_new},
    zip_recovery_outcome::RecoveryStatus,
    zip_recovery_reader::RecoveryReader,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/zip-bzip2-rescue-1733")
}
fn base() -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/zip-bzip2-rescue-1733");
    fs::create_dir_all(&path).unwrap();
    path
}
fn work() -> PathBuf {
    let p = base().join(uuid::Uuid::new_v4().to_string());
    fs::create_dir(&p).unwrap();
    p
}
fn save(name: &str, value: Value) {
    fs::write(
        base().join(name),
        serde_json::to_vec_pretty(&value).unwrap(),
    )
    .unwrap();
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn producer(zip64: bool) -> Value {
    let r: Value =
        serde_json::from_slice(&fs::read(fixtures().join("producer.json")).unwrap()).unwrap();
    r["originals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["zip64"] == zip64)
        .unwrap()
        .clone()
}
fn clean(parent: &Path) {
    assert!(!fs::read_dir(parent).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".long-repair")));
}
fn verify(output: &Path, entries: &[Value]) {
    let mut zip = zip_aes::ZipArchive::new(fs::File::open(output).unwrap()).unwrap();
    assert_eq!(zip.len(), entries.len());
    for e in entries {
        let mut member = zip.by_name(e["name"].as_str().unwrap()).unwrap();
        assert_eq!(member.is_dir(), e["directory"].as_bool().unwrap());
        let mut content = Vec::new();
        member.read_to_end(&mut content).unwrap();
        assert_eq!(content.len() as u64, e["size"].as_u64().unwrap());
        assert_eq!(sha(&content), e["sha256"].as_str().unwrap());
    }
}
fn recovered(source: &Path, parent: &Path, entries: &[Value], sources: &[PathBuf]) -> Value {
    let originals = sources
        .iter()
        .map(|p| json!({"path": p, "sha256": sha(&fs::read(p).unwrap())}))
        .collect::<Vec<_>>();
    let output = parent.join("recovered.zip");
    let result = repair_zip_to_new(source, &output, &AtomicBool::new(false)).unwrap();
    assert!(
        result.verified && result.partial && result.directory_rebuilt && !result.output_encrypted
    );
    assert_eq!(
        result.recovered_files,
        entries.iter().filter(|e| e["directory"] == false).count()
    );
    assert_eq!(
        result.recovered_directories,
        entries.iter().filter(|e| e["directory"] == true).count()
    );
    assert!(result
        .entry_results
        .iter()
        .any(|e| e.name.is_none() && e.status == RecoveryStatus::Unknown));
    assert_eq!(result.recovered_entries.len(), result.recovered_files);
    verify(&output, entries);
    for e in &originals {
        assert_eq!(
            sha(&fs::read(e["path"].as_str().unwrap()).unwrap()),
            e["sha256"].as_str().unwrap()
        );
    }
    clean(parent);
    json!({"input":source, "output":output, "sources":originals, "expected":entries, "result":result})
}

#[tokio::test]
async fn bziprescue1733_real_streaming_zip32_zip64_signed_and_unsigned_recover_complete_members() {
    let mut results = Vec::new();
    for zip64 in [false, true] {
        let original = producer(zip64);
        let source = fixtures().join(original["name"].as_str().unwrap());
        assert_eq!(
            sha(&fs::read(&source).unwrap()),
            original["sha256"].as_str().unwrap()
        );
        let healthy = diagnose_archive(&source, None, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap();
        assert_eq!(healthy.status, "healthy", "{healthy:?}");
        for style in ["lost", "unsigned"] {
            let parent = work();
            let input = parent.join("broken.zip");
            fs::copy(
                fixtures().join(format!("{style}{}.zip", if zip64 { 64 } else { 32 })),
                &input,
            )
            .unwrap();
            let report = diagnose_archive(&input, None, Arc::new(AtomicBool::new(false)))
                .await
                .unwrap();
            assert_eq!(report.status, "directory_lost", "{report:?}");
            assert!(
                report.can_repair
                    && !report.integrity_tested
                    && !report.total_uncompressed_size_known
            );
            assert_eq!((report.total_files, report.total_directories), (4, 2));
            let reader = RecoveryReader::open(&input, &AtomicBool::new(false)).unwrap();
            assert_eq!(reader.names.len(), 6);
            assert!(reader.directory_rebuilt && !reader.encrypted);
            let mut result = recovered(
                &input,
                &parent,
                original["entries"].as_array().unwrap(),
                &[input.clone()],
            );
            result["diagnosis"] = json!(report);
            result["variant"] = json!(format!("{style}-{}", if zip64 { 64 } else { 32 }));
            results.push(result);
        }
    }
    save("native.json", json!(results));
}

#[tokio::test]
async fn bziprescue1733_actual_complete_numeric_parts_cross_buffers_and_reject_a_missing_middle() {
    let mut results = Vec::new();
    for zip64 in [false, true] {
        let original = producer(zip64);
        let parent = work();
        let bytes =
            fs::read(fixtures().join(format!("lost{}.zip", if zip64 { 64 } else { 32 }))).unwrap();
        let parts = bytes
            .chunks(65536)
            .enumerate()
            .map(|(i, bytes)| {
                let path = parent.join(format!("资料.zip.{:03}", i + 1));
                fs::write(&path, bytes).unwrap();
                path
            })
            .collect::<Vec<_>>();
        assert!(parts.len() >= 4);
        let selected = if zip64 {
            parts.last().unwrap()
        } else {
            &parts[1]
        };
        let report = diagnose_archive(selected, None, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap();
        assert_eq!(report.status, "directory_lost", "{report:?}");
        assert!(report.can_repair && report.split_archive && !report.total_uncompressed_size_known);
        let mut result = recovered(
            selected,
            &parent,
            original["entries"].as_array().unwrap(),
            &parts,
        );
        result["diagnosis"] = json!(report);
        result["variant"] = json!(format!("numeric-{}", if zip64 { 64 } else { 32 }));
        let withheld = parent.join("withheld.bin");
        fs::rename(&parts[1], &withheld).unwrap();
        let absent = parent.join("must-not-exist.zip");
        assert!(repair_zip_to_new(&parts[0], &absent, &AtomicBool::new(false)).is_err());
        assert!(!absent.exists());
        fs::rename(withheld, &parts[1]).unwrap();
        for source in result["sources"].as_array().unwrap() {
            assert_eq!(
                sha(&fs::read(source["path"].as_str().unwrap()).unwrap()),
                source["sha256"].as_str().unwrap()
            );
        }
        clean(&parent);
        result["missingMiddleRejected"] = json!(true);
        results.push(result);
    }
    save("numeric.json", json!(results));
}

#[test]
fn bziprescue1733_damaged_stream_descriptor_and_gaps_keep_only_the_confirmed_prefix() {
    let mut results = Vec::new();
    for zip64 in [false, true] {
        let original = producer(zip64);
        let entries = original["entries"].as_array().unwrap();
        let raw =
            fs::read(fixtures().join(format!("lost{}.zip", if zip64 { 64 } else { 32 }))).unwrap();
        let blob = &entries[3];
        let start = blob["data"].as_u64().unwrap() as usize;
        let local = blob["local"].as_u64().unwrap() as usize;
        let desc = blob["descriptor"].as_u64().unwrap() as usize;
        for kind in [
            "truncated-stream",
            "bad-stream",
            "bad-crc",
            "compressed-size",
            "expanded-size",
            "truncated-descriptor",
            "gap",
            "unknown-method",
            "local-size",
        ] {
            let mut bytes = raw.clone();
            match kind {
                "truncated-stream" => bytes.truncate(start + 500),
                "bad-stream" => bytes[start + 30] ^= 0x80,
                "bad-crc" => bytes[desc + 4] ^= 1,
                "compressed-size" => bytes[desc + 8] ^= 1,
                "expanded-size" => bytes[desc + if zip64 { 16 } else { 12 }] ^= 1,
                "truncated-descriptor" => bytes.truncate(desc + 5),
                "gap" => {
                    bytes.splice(local..local, b"junkPK\x03\x04".iter().copied());
                }
                "unknown-method" => {
                    bytes[local + 8..local + 10].copy_from_slice(&222u16.to_le_bytes())
                }
                "local-size" => {
                    if zip64 {
                        let n =
                            u16::from_le_bytes(bytes[local + 26..local + 28].try_into().unwrap())
                                as usize;
                        bytes[local + 30 + n + 4..local + 30 + n + 12]
                            .copy_from_slice(&1u64.to_le_bytes());
                    } else {
                        bytes[local + 22..local + 26].copy_from_slice(&1u32.to_le_bytes());
                    }
                }
                _ => unreachable!(),
            }
            let parent = work();
            let source = parent.join("damaged.zip");
            fs::write(&source, &bytes).unwrap();
            let mut result = recovered(&source, &parent, &entries[..3], &[source.clone()]);
            assert!(!result["result"]["skippedEntries"]
                .as_array()
                .unwrap()
                .is_empty());
            if kind != "gap" {
                assert!(result["result"]["entryResults"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|e| e["name"] == blob["name"] && e["status"] == "unknown"));
            }
            result["variant"] = json!(format!("{kind}-{}", if zip64 { 64 } else { 32 }));
            results.push(result);
        }
    }
    save("stops.json", json!(results));
}

#[tokio::test]
async fn bziprescue1733_no_complete_file_cancel_existing_and_unknown_encryption_never_publish_then_retry(
) {
    let original = producer(false);
    let entries = original["entries"].as_array().unwrap();
    let raw = fs::read(fixtures().join("lost32.zip")).unwrap();
    let mut results = Vec::new();
    for kind in [
        "first-stream",
        "first-descriptor",
        "cancel",
        "existing",
        "encrypted",
    ] {
        let parent = work();
        let source = parent.join("broken.zip");
        let output = parent.join("recovered.zip");
        let mut bytes = raw.clone();
        match kind {
            "first-stream" => bytes[entries[1]["data"].as_u64().unwrap() as usize] ^= 1,
            "first-descriptor" => {
                bytes.truncate(entries[1]["descriptor"].as_u64().unwrap() as usize + 5)
            }
            "existing" => fs::write(&output, b"keep existing").unwrap(),
            "encrypted" => {
                let at = entries[3]["local"].as_u64().unwrap() as usize;
                bytes[at + 6] |= 1;
            }
            _ => {}
        }
        fs::write(&source, &bytes).unwrap();
        if matches!(kind, "first-stream" | "first-descriptor") {
            let report = diagnose_archive(&source, None, Arc::new(AtomicBool::new(false)))
                .await
                .unwrap();
            assert_eq!(report.status, "directory_lost", "{report:?}");
            assert_eq!(report.total_files, 0);
            assert!(!report.can_repair && !report.total_uncompressed_size_known);
            assert!(report
                .issues
                .iter()
                .any(|issue| issue.title.contains("尚未找到")));
        }
        assert!(
            repair_zip_to_new(&source, &output, &AtomicBool::new(kind == "cancel")).is_err(),
            "{kind}"
        );
        if kind == "existing" {
            assert_eq!(fs::read(&output).unwrap(), b"keep existing");
        } else {
            assert!(!output.exists());
        }
        assert_eq!(fs::read(&source).unwrap(), bytes);
        clean(&parent);
        results.push(json!({"variant":kind,"noNewOutput":true,"originalUnchanged":true,"stagingCleaned":true}));
        if kind == "first-stream" {
            fs::write(&source, &raw).unwrap();
            let mut result = recovered(&source, &parent, entries, &[source.clone()]);
            result["variant"] = json!("retry-after-source-restored");
            save("retry.json", result);
        }
    }
    save("failures.json", json!(results));
}
