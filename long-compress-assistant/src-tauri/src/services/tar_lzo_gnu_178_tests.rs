use super::{lzop::{self, recovery}, tar_lzo_prefix_175_tests::wrapped};
use sha2::{Digest, Sha256};
use std::{io::Read, path::{Path, PathBuf}, sync::{atomic::AtomicBool, Arc}};

fn flag() -> AtomicBool { AtomicBool::new(false) }
fn root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/tar-lzo-gnu-178") }
fn manifest() -> serde_json::Value { serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap() }
fn sha(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }
fn scan(path: &Path) -> recovery::Report { recovery::scan(path, None, &flag(), |_, _| {}).unwrap() }
fn raw() -> Vec<u8> {
    let mut bytes = Vec::new();
    super::codec_stream::decoder(&root().join("complete.tar.lzo"), "lzo").unwrap().read_to_end(&mut bytes).unwrap();
    assert_eq!(sha(&bytes), manifest()["tarSha256"].as_str().unwrap());
    bytes
}
fn exact(entries: &[recovery::Entry], directory: &Path) {
    let m = manifest();
    for entry in entries {
        let expected = m["entries"].as_array().unwrap().iter().find(|item| item["name"] == entry.name).unwrap();
        assert_eq!(entry.is_directory, expected["isDirectory"].as_bool().unwrap());
        assert_eq!(entry.bytes, expected["bytes"].as_u64().unwrap());
        assert_eq!(entry.sha256, expected["sha256"].as_str().unwrap());
        let path = directory.join(&entry.name);
        if entry.is_directory { assert!(path.is_dir()); }
        else { assert_eq!(sha(&std::fs::read(path).unwrap()), entry.sha256); }
    }
}
fn recover(path: &Path, destination: &Path, report: &recovery::Report, ids: &[usize]) -> recovery::Outcome {
    let result = recovery::recover(path, destination, &report.revision, ids, None, &flag(), |_, _, _| {}).unwrap();
    exact(&result.entries, Path::new(&result.output_directory));
    assert!(!std::fs::read_dir(destination).unwrap().any(|item| item.unwrap().file_name().to_string_lossy().starts_with(".long-lzop-recovery")));
    result
}

#[test]
fn tarlzo178_official_gnu_prefix_restores_long_paths_once_without_metadata_files() {
    let data = std::fs::read(root().join("complete.tar.lzo")).unwrap();
    let m = manifest();
    assert_eq!(sha(&data), m["sha256"].as_str().unwrap());
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/tar-lzo-gnu-178");
    let work = base.join("generated").join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&work).unwrap();
    let mut outputs = Vec::new();
    for case in ["tail", "checksum", "end"] {
        let mut damaged = data.clone();
        match case {
            "tail" => damaged.truncate(data.len() - 400_000),
            "checksum" => { let at = damaged.len() - 400_000; damaged[at] ^= 1; }
            _ => damaged.truncate(damaged.len() - 2),
        }
        let path = work.join(format!("{case}.tar.lzo"));
        std::fs::write(&path, &damaged).unwrap();
        let report = scan(&path);
        assert!(!report.complete, "{report:?}");
        assert_eq!(report.entries.len(), if case == "end" { 9 } else { 8 }, "{case}: {report:?}");
        for (entry, expected) in report.entries.iter().zip(m["entries"].as_array().unwrap()) {
            assert_eq!(entry.name, expected["name"].as_str().unwrap());
            assert_eq!(entry.sha256, expected["sha256"].as_str().unwrap());
        }
        assert!(report.entries.iter().any(|entry| entry.name == "after.txt"));
        assert!(!report.entries.iter().any(|entry| entry.name.contains("@LongLink")));
        assert!(lzop::verify(&path, || false).is_err());
        let ids: Vec<usize> = report.entries.iter().map(|entry| entry.id).collect();
        let result = recover(&path, &work, &report, &ids);
        assert!(result.partial);
        assert_eq!(result.recovered_directories, 2);
        assert_eq!(std::fs::read(&path).unwrap(), damaged);
        outputs.push(serde_json::json!({"source": path, "outputDirectory": result.output_directory, "entries": result.entries}));
        if case == "tail" {
            std::fs::write(base.join("native.json"), serde_json::to_vec_pretty(&serde_json::json!({"source": path, "report": report, "result": result})).unwrap()).unwrap();
        }
    }
    std::fs::write(base.join("native-outputs.json"), serde_json::to_vec_pretty(&outputs).unwrap()).unwrap();
}

#[test]
fn tarlzo178_missing_numeric_volume_keeps_long_names_and_replacement_requires_recheck() {
    let data = std::fs::read(root().join("complete.tar.lzo")).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let mut paths = Vec::new();
    for (index, part) in data.chunks(100003).enumerate() {
        let path = temp.path().join(format!("长目录.tar.lzo.{:03}", index + 1));
        std::fs::write(&path, part).unwrap(); paths.push(path);
    }
    let missing = paths[4].clone();
    let withheld = std::fs::read(&missing).unwrap();
    std::fs::remove_file(&missing).unwrap();
    let path = paths.last().unwrap();
    let report = scan(path);
    assert_eq!(report.available_volumes, 4);
    assert!(report.ignored_volumes > 0);
    assert_eq!(report.entries.len(), 8, "{report:?}");
    let result = recover(path, temp.path(), &report, &[1, 2, 3, 6]);
    assert_eq!(result.recovered_files, 2);
    assert_eq!(result.recovered_directories, 2);
    std::fs::write(missing, withheld).unwrap();
    assert!(recovery::recover(path, temp.path(), &report.revision, &[6], None, &flag(), |_, _, _| {}).unwrap_err().to_string().contains("重新检查"));
    assert!(scan(path).complete);
}

#[test]
fn tarlzo178_long_metadata_requires_checked_header_payload_padding_and_a_following_entry() {
    let bytes = raw();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("boundary.tar.lzo");
    // The official first L record is at 1024, with a 207-byte payload.
    for cut in [1124, 1024 + 512 + 200, 1024 + 512 + 208, 2047] {
        std::fs::write(&path, wrapped(&bytes[..cut], 137, true)).unwrap();
        let report = scan(&path);
        assert!(!report.complete);
        assert_eq!(report.entries.len(), 1, "cut {cut}: {report:?}");
        assert_eq!(report.entries[0].name, "prefix.txt");
        recover(&path, temp.path(), &report, &[0]);
    }
    let mut orphan = bytes[..2048].to_vec(); orphan.extend([0; 1024]);
    std::fs::write(&path, wrapped(&orphan, 137, true)).unwrap();
    let report = scan(&path);
    assert_eq!(report.entries.len(), 1);
    assert!(report.stop_reason.contains("缺少对应条目"));
}

#[test]
fn tarlzo178_malformed_repeated_mixed_and_unsafe_names_stop_at_the_original_prefix() {
    let bytes = raw();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("metadata.tar.lzo");
    let mut cases = Vec::new();
    let mut no_end = bytes.clone(); no_end[1536..1743].fill(b'a'); cases.push(("missing terminator", no_end));
    let mut empty = bytes.clone(); empty[1536..1743].fill(0); cases.push(("empty name", empty));
    let mut trailing = bytes.clone(); trailing[1742] = 0; trailing[1537] = 0; cases.push(("data after terminator", trailing));
    let mut unsafe_name = bytes.clone(); unsafe_name[1536..1743].fill(0); unsafe_name[1536..1546].copy_from_slice(b"../outside"); cases.push(("unsafe path", unsafe_name));
    let mut repeated = bytes[..2048].to_vec(); repeated.extend_from_slice(&bytes[1024..]); cases.push(("repeated GNU", repeated));
    let mut pax = tar::Builder::new(Vec::new()); pax.append_pax_extensions([("path", b"override.txt".as_slice())]).unwrap();
    let pax = pax.into_inner().unwrap();
    let mut mixed = bytes[..2048].to_vec(); mixed.extend_from_slice(&pax[..pax.len()-1024]); mixed.extend_from_slice(&bytes[2048..]); cases.push(("mixed GNU/PAX", mixed));
    for (label, data) in cases {
        std::fs::write(&path, wrapped(&data, 333, true)).unwrap();
        let report = scan(&path);
        assert!(!report.complete, "{label}");
        assert_eq!(report.entries.len(), 1, "{label}: {report:?}");
        assert!(!report.stop_reason.is_empty());
        recover(&path, temp.path(), &report, &[0]);
    }
}

#[cfg(windows)]
#[test]
fn tarlzo178_explicit_gnu_name_encoding_is_respected_without_replacement_characters() {
    let mut bytes = raw();
    bytes[1536..1743].fill(0);
    bytes[1536..1540].copy_from_slice(&[0xd7, 0xca, 0xc1, 0xcf]); // 资料 in CP936
    // Do not append later original long paths whose UTF-8 bytes are a different encoding.
    bytes.truncate(2560); bytes.extend([0; 1024]);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("local.tar.lzo");
    std::fs::write(&path, wrapped(&bytes, 137, true)).unwrap();
    let report = scan(&path);
    assert_eq!(report.entries.len(), 1);
    assert!(report.stop_reason.contains("lzop-encoding"));
    let report = recovery::scan(&path, Some(super::zip_password::ZipPasswordEncoding::Cp936), &flag(), |_, _| {}).unwrap();
    assert!(report.complete, "{report:?}");
    assert_eq!(report.entries[1].name, "资料");
}

#[tokio::test]
async fn tarlzo178_complete_normal_browse_extract_preview_and_partial_diagnosis_agree() {
    let path = root().join("complete.tar.lzo");
    let m = manifest();
    assert!(scan(&path).complete);
    let list = super::archive_browser::browse_archive(&path, None).await.unwrap();
    assert_eq!(list.entries.len(), 9);
    assert_eq!(list.total_files, 7);
    for (entry, expected) in list.entries.iter().zip(m["entries"].as_array().unwrap()) {
        assert_eq!(entry.path.trim_end_matches('/'), expected["name"].as_str().unwrap());
    }
    super::universal_engine::UniversalCliEngine::test_integrity(&path, None).await.unwrap();
    let runtime = super::tar_lzo_1658_tests::Runtime(super::io_buffer_pool::IOBufferPool::new(Default::default()));
    let temp = tempfile::tempdir().unwrap();
    super::native_extraction::tar::extract_inner(&runtime, None, "tarlzo178", path.to_str().unwrap(), temp.path(), Some(super::codec_stream::decoder(&path, "lzo").unwrap()), &Default::default()).unwrap();
    for expected in m["entries"].as_array().unwrap() {
        let item = temp.path().join(expected["name"].as_str().unwrap());
        if expected["isDirectory"].as_bool().unwrap() { assert!(item.is_dir()); }
        else { assert_eq!(sha(&std::fs::read(item).unwrap()), expected["sha256"].as_str().unwrap()); }
    }
    let preview = super::archive_preview::preview_archive_text(&path, m["longFile"].as_str().unwrap(), None).await.unwrap();
    assert!(preview.content.contains("GNU long UTF-8 file name"));
    let healthy = super::archive_diagnostics::diagnose_archive(&path, None, Arc::new(flag())).await.unwrap();
    assert_eq!(healthy.status, "healthy"); assert!(healthy.integrity_tested); assert_eq!(healthy.recoverability, "not_needed");
    let damaged = temp.path().join("damaged.tar.lzo");
    let data = std::fs::read(path).unwrap(); std::fs::write(&damaged, &data[..data.len()-400000]).unwrap();
    let report = super::archive_diagnostics::diagnose_archive(&damaged, None, Arc::new(flag())).await.unwrap();
    assert_eq!(report.status, "partial_recovery");
    // Diagnosis includes implicit parent folders; recovery counts selected
    // explicit directory entries separately (two in this GNU original).
    assert_eq!(report.total_files, 6); assert_eq!(report.total_directories, 4);
    assert!(!report.integrity_tested);
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/tar-lzo-gnu-178");
    std::fs::write(base.join("diagnostic-records.json"), serde_json::to_vec_pretty(&serde_json::json!({"healthy": healthy, "partial": report})).unwrap()).unwrap();
}
