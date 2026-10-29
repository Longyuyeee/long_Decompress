//! Single XZ streams and real descriptor boundaries, through the existing rescue workflow.
use super::{
    archive_diagnostics::{diagnose_archive, repair_zip_to_new},
    zip_zstd_rescue_1736_tests::{clean, recovered},
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/zip-xz-rescue-1738")
}
fn base() -> PathBuf {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/zip-xz-rescue-1738");
    fs::create_dir_all(&p).unwrap();
    p
}
fn cases() -> Vec<Value> {
    let v: Value =
        serde_json::from_slice(&fs::read(fixtures().join("manifest.json")).unwrap()).unwrap();
    v["cases"].as_array().unwrap().clone()
}
fn work() -> PathBuf {
    let p = base().join(uuid::Uuid::new_v4().to_string());
    fs::create_dir(&p).unwrap();
    p
}
fn save(name: &str, v: Value) {
    fs::write(base().join(name), serde_json::to_vec_pretty(&v).unwrap()).unwrap();
}
fn lost(spec: &Value) -> Vec<u8> {
    let raw = fs::read(fixtures().join(spec["name"].as_str().unwrap())).unwrap();
    assert_eq!(format!("{:x}", Sha256::digest(&raw)), spec["sha256"]);
    let bytes = raw[..spec["central"].as_u64().unwrap() as usize].to_vec();
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), spec["lostSha256"]);
    bytes
}

#[tokio::test]
async fn xzrescue1738_full_members_zip64_unsigned_empty_stream_and_mixed_method() {
    let mut results = Vec::new();
    for spec in cases() {
        let full = fixtures().join(spec["name"].as_str().unwrap());
        let healthy = diagnose_archive(&full, None, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap();
        assert_eq!(healthy.status, "healthy", "{healthy:?}");
        let parent = work();
        let source = parent.join("lost.zip");
        fs::write(&source, lost(&spec)).unwrap();
        let report = diagnose_archive(&source, None, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap();
        assert_eq!(report.status, "directory_lost", "{report:?}");
        assert!(
            report.can_repair && !report.integrity_tested && !report.total_uncompressed_size_known
        );
        let entries = spec["entries"].as_array().unwrap();
        let mut result = recovered(&source, &parent, entries, &[source.clone()]);
        result["variant"] = spec["name"].clone();
        result["diagnosis"] = json!(report);
        results.push(result);
    }
    save("native.json", json!(results));
}

#[tokio::test]
async fn xzrescue1738_actual_numeric_slices_split_stream_and_descriptor_headers() {
    let spec = cases()
        .into_iter()
        .find(|s| s["name"] == "xz64-signed.zipx")
        .unwrap();
    let bytes = lost(&spec);
    let blob = &spec["entries"][3];
    let cut = blob["data"].as_u64().unwrap() as usize + 2;
    let mut bounds = vec![
        0,
        bytes.len(),
        blob["descriptor"].as_u64().unwrap() as usize + 2,
    ];
    bounds.extend((cut..bytes.len()).step_by(65536));
    bounds.sort_unstable();
    bounds.dedup();
    let parent = work();
    let parts = bounds
        .windows(2)
        .enumerate()
        .map(|(i, span)| {
            let p = parent.join(format!("多帧资料.zip.{:03}", i + 1));
            fs::write(&p, &bytes[span[0]..span[1]]).unwrap();
            p
        })
        .collect::<Vec<_>>();
    assert!(parts.len() >= 4);
    let selected = parts.last().unwrap();
    let report = diagnose_archive(selected, None, Arc::new(AtomicBool::new(false)))
        .await
        .unwrap();
    assert_eq!(report.status, "directory_lost");
    assert!(report.can_repair && report.split_archive && !report.total_uncompressed_size_known);
    let mut result = recovered(
        selected,
        &parent,
        spec["entries"].as_array().unwrap(),
        &parts,
    );
    result["variant"] = json!("numeric-frame-and-descriptor-cuts");
    result["diagnosis"] = json!(report);
    let withheld = parent.join("withheld.bin");
    fs::rename(&parts[1], &withheld).unwrap();
    let absent = parent.join("must-not-exist.zip");
    assert!(repair_zip_to_new(&parts[0], &absent, &AtomicBool::new(false)).is_err());
    assert!(!absent.exists());
    fs::rename(withheld, &parts[1]).unwrap();
    clean(&parent);
    result["missingMiddleRejected"] = json!(true);
    save("numeric.json", json!([result]));
}

#[test]
fn xzrescue1738_bad_stream_check_index_footer_and_descriptor_stop_at_the_member() {
    let mut results = Vec::new();
    for wide in [false, true] {
        let spec = cases()
            .into_iter()
            .find(|s| s["name"] == format!("xz{}-signed.zipx", if wide { 64 } else { 32 }))
            .unwrap();
        let raw = lost(&spec);
        let entries = spec["entries"].as_array().unwrap();
        let blob = &entries[3];
        let local = blob["local"].as_u64().unwrap() as usize;
        let start = blob["data"].as_u64().unwrap() as usize;
        let desc = blob["descriptor"].as_u64().unwrap() as usize;
        for kind in [
            "header",
            "truncated",
            "block-check",
            "index",
            "footer",
            "bad-crc",
            "compressed-size",
            "expanded-size",
            "descriptor-truncated",
            "gap",
            "local-size",
            "concatenated",
            "padding",
            "memory",
        ] {
            let mut bytes = raw.clone();
            match kind {
                "header" => bytes[start] ^= 1,
                "truncated" => bytes.truncate(start + 100),
                "block-check" => {
                    let backward =
                        u32::from_le_bytes(bytes[desc - 8..desc - 4].try_into().unwrap()) as usize;
                    let index = desc - 12 - (backward + 1) * 4;
                    bytes[index - 1] ^= 1;
                }
                "index" => bytes[desc - 16] ^= 1,
                "footer" => bytes[desc - 12] ^= 1,
                "bad-crc" => bytes[desc + 4] ^= 1,
                "compressed-size" => bytes[desc + 8] ^= 1,
                "expanded-size" => bytes[desc + if wide { 16 } else { 12 }] ^= 1,
                "descriptor-truncated" => bytes.truncate(desc + 5),
                "gap" => {
                    bytes.splice(start..start, b"junkPK\x03\x04".iter().copied());
                }
                "local-size" => {
                    if wide {
                        bytes[start - 16..start - 8].copy_from_slice(&1u64.to_le_bytes());
                    } else {
                        bytes[local + 22..local + 26].copy_from_slice(&1u32.to_le_bytes());
                    }
                }
                "concatenated" => {
                    let e = &entries[2];
                    let empty = raw[e["data"].as_u64().unwrap() as usize
                        ..e["descriptor"].as_u64().unwrap() as usize]
                        .to_vec();
                    bytes.splice(desc..desc, empty);
                }
                "padding" => {
                    bytes.splice(desc..desc, [0u8; 4]);
                }
                "memory" => {
                    assert_eq!(&bytes[start + 14..start + 16], &[0x21, 1]);
                    bytes[start + 16] = 40;
                    let end = start + 12 + (bytes[start + 12] as usize + 1) * 4;
                    let crc = crc32fast::hash(&bytes[start + 12..end - 4]);
                    bytes[end - 4..end].copy_from_slice(&crc.to_le_bytes());
                }
                _ => unreachable!(),
            }
            let parent = work();
            let source = parent.join("bad-later-frame.zip");
            fs::write(&source, bytes).unwrap();
            let mut result = recovered(&source, &parent, &entries[..3], &[source.clone()]);
            assert!(result["result"]["entryResults"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["name"] == blob["name"] && r["status"] == "unknown"));
            result["variant"] = json!(format!("{kind}-{}", if wide { 64 } else { 32 }));
            results.push(result);
        }
    }
    save("stops.json", json!(results));
}

#[tokio::test]
async fn xzrescue1738_incomplete_first_member_is_not_repairable_then_fixed_source_retries() {
    let spec = cases()
        .into_iter()
        .find(|s| s["name"] == "xz32-signed.zipx")
        .unwrap();
    let raw = lost(&spec);
    let note = &spec["entries"][1];
    let mut failures = Vec::new();
    for kind in ["bad-first-stream", "bad-whole-member-crc"] {
        let mut bytes = raw.clone();
        if kind == "bad-first-stream" {
            bytes[note["data"].as_u64().unwrap() as usize] ^= 1;
        } else {
            bytes[note["descriptor"].as_u64().unwrap() as usize + 4] ^= 1;
        }
        let parent = work();
        let source = parent.join("first-member.zip");
        let output = parent.join("recovered.zip");
        fs::write(&source, &bytes).unwrap();
        let report = diagnose_archive(&source, None, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap();
        assert_eq!(report.status, "directory_lost");
        assert_eq!(report.total_files, 0);
        assert!(!report.can_repair && !report.total_uncompressed_size_known);
        assert!(repair_zip_to_new(&source, &output, &AtomicBool::new(false)).is_err());
        assert!(!output.exists());
        assert_eq!(fs::read(&source).unwrap(), bytes);
        clean(&parent);
        failures.push(json!({"variant":kind,"noNewOutput":true,"originalUnchanged":true,"stagingCleaned":true}));
        if kind == "bad-first-stream" {
            fs::write(&source, &raw).unwrap();
            let mut result = recovered(
                &source,
                &parent,
                spec["entries"].as_array().unwrap(),
                &[source.clone()],
            );
            result["variant"] = json!("retry-after-whole-member-restored");
            save("retry.json", result);
        }
    }
    save("failures.json", json!(failures));
}
