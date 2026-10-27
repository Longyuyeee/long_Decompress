use super::{
    lzop::{self, recovery},
    lzop_writer,
};
use sha2::{Digest, Sha256};
use std::{
    io::Cursor,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
fn flag() -> AtomicBool {
    AtomicBool::new(false)
}
fn fixture() -> (Vec<u8>, serde_json::Value) {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/tar-lzo-prefix-175");
    (
        std::fs::read(root.join("complete.tar.lzo")).unwrap(),
        serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap(),
    )
}
fn scan(p: &Path) -> recovery::Report {
    recovery::scan(p, None, &flag(), |_, _| {}).unwrap()
}
fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn restore(p: &Path, base: &Path, r: &recovery::Report, ids: &[usize]) -> recovery::Outcome {
    recovery::recover(p, base, &r.revision, ids, None, &flag(), |_, _, _| {}).unwrap()
}
fn exact(out: &recovery::Outcome) {
    for e in &out.entries {
        let p = Path::new(&out.output_directory).join(&e.name);
        if e.is_directory {
            assert!(p.is_dir());
        } else {
            let data = std::fs::read(p).unwrap();
            assert_eq!(data.len() as u64, e.bytes);
            assert_eq!(sha(&data), e.sha256);
        }
    }
}
fn no_stage(p: &Path) {
    assert!(!std::fs::read_dir(p).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".long-lzop-recovery")));
}
pub(super) fn wrapped(data: &[u8], chunk: usize, checked: bool) -> Vec<u8> {
    let mut head = Vec::new();
    lzop_writer::write_stream(
        &mut Cursor::new(Vec::<u8>::new()),
        &mut head,
        Some("source.tar"),
        |_| Ok(()),
    )
    .unwrap();
    head.truncate(head.len() - 4);
    if !checked {
        let flags = u32::from_be_bytes(head[17..21].try_into().unwrap()) & !0x300;
        head[17..21].copy_from_slice(&flags.to_be_bytes());
        let n = head.len() - 4;
        let sum = crc32fast::hash(&head[9..n]);
        head[n..].copy_from_slice(&sum.to_be_bytes());
    }
    for b in data.chunks(chunk) {
        head.extend((b.len() as u32).to_be_bytes());
        head.extend((b.len() as u32).to_be_bytes());
        if checked {
            head.extend(crc32fast::hash(b).to_be_bytes());
        }
        head.extend(b);
    }
    head.extend(0u32.to_be_bytes());
    head
}
fn plain_tar() -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (name, bytes) in [
        ("first.txt", b"checked".as_slice()),
        ("empty.txt", b"".as_slice()),
        ("late.bin", b"late".as_slice()),
    ] {
        let mut h = tar::Header::new_ustar();
        h.set_mode(0o644);
        h.set_size(bytes.len() as u64);
        h.set_cksum();
        builder.append_data(&mut h, name, bytes).unwrap();
    }
    builder.into_inner().unwrap()
}

#[test]
fn tarlzo175_official_prefix_keeps_only_complete_entries_and_selected_empty_directories() {
    let (data, manifest) = fixture();
    assert_eq!(sha(&data), manifest["sha256"].as_str().unwrap());
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test-results/tar-lzo-prefix-rescue-175/generated")
        .join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&base).unwrap();
    let mut evidence = Vec::new();
    for case in ["tail", "checksum", "end"] {
        let mut damaged = data.clone();
        match case {
            "tail" => damaged.truncate(data.len() - 400000),
            "checksum" => {
                let n = damaged.len() - 400000;
                damaged[n] ^= 1;
            }
            _ => damaged.truncate(damaged.len() - 2),
        }
        let p = base.join(format!("{case}.tar.lzo"));
        std::fs::write(&p, &damaged).unwrap();
        let r = scan(&p);
        assert!(!r.complete);
        assert_eq!(r.format, "TAR.LZO");
        assert!(r.entries.len() >= 6, "{case}: {r:?}");
        assert!(lzop::verify(&p, || false).is_err());
        for (e, m) in r
            .entries
            .iter()
            .zip(manifest["entries"].as_array().unwrap())
        {
            assert_eq!(e.name, m["name"].as_str().unwrap());
            assert_eq!(e.is_directory, m["isDirectory"].as_bool().unwrap());
            assert_eq!(e.sha256, m["sha256"].as_str().unwrap());
        }
        let out = restore(&p, &base, &r, &[1, 2, 3, 4, 5]);
        assert!(out.partial);
        assert_eq!(out.recovered_files, 4);
        assert_eq!(out.recovered_directories, 1);
        exact(&out);
        assert!(!Path::new(&out.output_directory).join("late.bin").exists());
        assert_eq!(std::fs::read(&p).unwrap(), damaged);
        no_stage(&base);
        evidence.push(serde_json::json!({"source":p,"outputDirectory":out.output_directory,"entries":out.entries.iter().map(|e|&e.name).collect::<Vec<_>>()}));
    }
    let healthy = base.join("healthy.tar.lzo");
    std::fs::write(&healthy, data).unwrap();
    let r = scan(&healthy);
    assert!(r.complete, "{r:?}");
    assert_eq!(r.entries.len(), 7);
    assert!(recovery::recover(
        &healthy,
        &base,
        &r.revision,
        &[0],
        None,
        &flag(),
        |_, _, _| {}
    )
    .is_err());
    std::fs::write(
        base.parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("native-outputs.json"),
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
    std::fs::write(base.parent().unwrap().parent().unwrap().join("native.json"),serde_json::to_vec_pretty(&serde_json::json!({"work":base,"source":base.join("tail.tar.lzo"),"entries":scan(&base.join("tail.tar.lzo")).entries})).unwrap()).unwrap();
}
#[test]
fn tarlzo175_nonfirst_volume_stops_at_gap_and_replacement_requires_recheck() {
    let (data, _) = fixture();
    let temp = tempfile::tempdir().unwrap();
    let mut paths = Vec::new();
    for (n, bytes) in data.chunks(100003).enumerate() {
        let p = temp.path().join(format!("资料.tar.lzo.{:03}", n + 1));
        std::fs::write(&p, bytes).unwrap();
        paths.push(p);
    }
    let missing = paths[4].clone();
    let bytes = std::fs::read(&missing).unwrap();
    std::fs::remove_file(&missing).unwrap();
    let r = scan(paths.last().unwrap());
    assert!(!r.complete);
    assert_eq!(r.available_volumes, 4);
    assert!(r.ignored_volumes > 0);
    assert!(r.entries.iter().any(|e| e.name == "资料/图标.png"));
    assert!(!r.entries.iter().any(|e| e.name == "late.bin"));
    let out = restore(paths.last().unwrap(), temp.path(), &r, &[1, 2, 3]);
    exact(&out);
    std::fs::write(&missing, bytes).unwrap();
    assert!(recovery::recover(
        paths.last().unwrap(),
        temp.path(),
        &r.revision,
        &[2],
        None,
        &flag(),
        |_, _, _| {}
    )
    .unwrap_err()
    .to_string()
    .contains("重新检查"));
    assert!(scan(paths.last().unwrap()).complete);
    lzop::verify(paths.last().unwrap(), || false).unwrap();
    std::fs::remove_file(&paths[0]).unwrap();
    assert!(
        recovery::scan(paths.last().unwrap(), None, &flag(), |_, _| {})
            .unwrap_err()
            .to_string()
            .contains("首卷")
    );
    no_stage(temp.path());
}
#[test]
fn tarlzo175_cross_block_header_data_and_padding_never_publish_unconfirmed_bytes() {
    let tar = plain_tar();
    let temp = tempfile::tempdir().unwrap();
    let p = temp.path().join("cross.tar.lzo");
    // Deliberately unaligned checked blocks exercise TAR headers, payload and padding across blocks.
    let full = wrapped(&tar, 333, true);
    std::fs::write(&p, &full).unwrap();
    assert!(scan(&p).complete);
    for cut in [512usize + 7, 512 + 7 + 200, 1024 + 100, 1536 + 100] {
        let n = (cut / 333) * 333;
        let mut data = wrapped(&tar[..n], 333, true);
        data.truncate(data.len() - 4);
        std::fs::write(&p, data).unwrap();
        let r = scan(&p);
        assert!(!r.complete);
        let expected = if n >= 1024 { 1 } else { 0 };
        assert_eq!(r.entries.len(), expected, "cut {cut}: {r:?}");
        if expected > 0 {
            exact(&restore(&p, temp.path(), &r, &[0]));
        }
    }
    std::fs::write(&p, wrapped(&tar, 333, false)).unwrap();
    let r = scan(&p);
    assert!(r.entries.is_empty() && r.stop_reason.contains("未提供内容校验"));
    // Valid outer checksums cannot make an invalid inner TAR header acceptable.
    let mut bad = tar.clone();
    bad[1024] ^= 1;
    std::fs::write(&p, wrapped(&bad, 333, true)).unwrap();
    let r = scan(&p);
    assert_eq!(r.entries.len(), 1);
    assert!(r.stop_reason.contains("文件头校验"));
    no_stage(temp.path());
}
#[test]
fn tarlzo175_unsupported_metadata_stops_without_skipping_and_copy_cancellation_cleans() {
    let temp = tempfile::tempdir().unwrap();
    let p = temp.path().join("unsupported.tar.lzo");
    let tar = plain_tar();
    for kind in [b'1', b'2', b'S', b'V'] {
        let mut bytes = tar.clone();
        let h = tar::Header::from_byte_slice(&bytes[1024..1536]).clone();
        let mut h = h;
        h.set_entry_type(tar::EntryType::new(kind));
        h.set_cksum();
        bytes[1024..1536].copy_from_slice(h.as_bytes());
        std::fs::write(&p, wrapped(&bytes, 333, true)).unwrap();
        let r = scan(&p);
        assert_eq!(r.entries.len(), 1);
        assert!(r.stop_reason.contains("unsupported"), "{r:?}");
    }
    let mut builder = tar::Builder::new(Vec::new());
    builder
        .append_pax_extensions([("GNU.sparse.name", b"bad".as_slice())])
        .unwrap();
    let mut h = tar::Header::new_ustar();
    h.set_size(0);
    h.set_mode(0o644);
    h.set_cksum();
    builder
        .append_data(&mut h, "dummy", Cursor::new([]))
        .unwrap();
    std::fs::write(&p, wrapped(&builder.into_inner().unwrap(), 333, true)).unwrap();
    let r = scan(&p);
    assert!(r.entries.is_empty() && r.stop_reason.contains("GNU.sparse"));
    let (data, _) = fixture();
    std::fs::write(&p, &data[..data.len() - 400000]).unwrap();
    let r = scan(&p);
    let before = std::fs::read(&p).unwrap();
    let stop = flag();
    let result = recovery::recover(
        &p,
        temp.path(),
        &r.revision,
        &[2],
        None,
        &stop,
        |phase, n, _| {
            if phase == "recovering" && n > 0 {
                stop.store(true, Ordering::SeqCst);
            }
        },
    );
    assert!(result.unwrap_err().to_string().contains("cancelled"));
    assert_eq!(std::fs::read(&p).unwrap(), before);
    no_stage(temp.path());
    exact(&restore(&p, temp.path(), &r, &[2]));
}
#[tokio::test]
async fn tarlzo175_diagnosis_and_history_count_directories_without_full_success() {
    let (data, _) = fixture();
    let temp = tempfile::tempdir().unwrap();
    let p = temp.path().join("check.tar.lzo");
    std::fs::write(&p, &data[..data.len() - 400000]).unwrap();
    let r = super::archive_diagnostics::diagnose_archive(&p, None, Arc::new(flag()))
        .await
        .unwrap();
    assert_eq!(r.actual_format, "TAR.LZO");
    assert_eq!(r.status, "partial_recovery");
    assert_eq!(r.recoverability, "lzop_prefix");
    assert_eq!(r.total_files, 4);
    assert!(r.total_directories >= 3);
    assert!(!r.integrity_tested && !r.can_repair);
    assert!(r.lzop_prefix.is_some());
    let selection:recovery::HistorySelection=serde_json::from_value(serde_json::json!({"recoveredFiles":0,"recoveredDirectories":2,"stopReason":"after prefix","revision":"a".repeat(64)})).unwrap();
    selection.validate().unwrap();
    assert_eq!(
        serde_json::to_value(selection).unwrap()["recoveredDirectories"],
        2
    );
}

#[tokio::test]
async fn tarlzo175_intact_existing_sparse_and_gnu_archives_keep_complete_diagnosis(){
 let base=Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/tar-lzo-read-1658");
 for name in ["old-gnu.tar.lzo","pax10.tar.lzo"]{
 let report=super::archive_diagnostics::diagnose_archive(&base.join(name),None,Arc::new(flag())).await.unwrap();
 assert_eq!(report.status,"healthy","{name}: {report:?}");assert!(report.integrity_tested);assert_eq!(report.recoverability,"not_needed");assert!(report.lzop_prefix.is_none());
 }
}
