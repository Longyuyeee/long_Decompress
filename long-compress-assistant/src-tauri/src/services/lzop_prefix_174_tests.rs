use super::{
    lzop::{self, recovery},
    lzop_writer,
};
use sha2::{Digest, Sha256};
use std::{
    io::Cursor,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
fn flag() -> AtomicBool {
    AtomicBool::new(false)
}
fn fixture() -> (Vec<u8>, serde_json::Value) {
    let p =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/lzop-prefix-174");
    (
        std::fs::read(p.join("complete.lzo")).unwrap(),
        serde_json::from_slice(&std::fs::read(p.join("manifest.json")).unwrap()).unwrap(),
    )
}
fn scan(p: &Path) -> recovery::Report {
    recovery::scan(p, None, &flag(), |_, _| {}).unwrap()
}
fn recover(p: &Path, out: &Path, r: &recovery::Report, ids: &[usize]) -> recovery::Outcome {
    recovery::recover(p, out, &r.revision, ids, None, &flag(), |_, _, _| {}).unwrap()
}
fn sha(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
fn assert_content(out: &recovery::Outcome, r: &recovery::Report) {
    for e in &out.entries {
        let data = std::fs::read(Path::new(&out.output_directory).join(&e.name)).unwrap();
        assert_eq!(data.len() as u64, e.bytes);
        assert_eq!(sha(&data), e.sha256);
        assert!(r.entries.iter().any(|v| v.name == e.name));
    }
}
fn no_stage(p: &Path) {
    assert!(!std::fs::read_dir(p).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".long-lzop-recovery")));
}
#[test]
fn lzop174_official_prefix_stops_at_bad_header_data_and_tail_with_selected_paths_and_empty_file() {
    let (data, m) = fixture();
    assert_eq!(sha(&data), m["sha256"].as_str().unwrap());
    let offset = m["entries"][4]["offset"].as_u64().unwrap() as usize;
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test-results/lzop-prefix-rescue-174/generated")
        .join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&base).unwrap();
    let mut evidence = Vec::new();
    for case in ["header", "data", "tail", "end"] {
        let mut damaged = data.clone();
        match case {
            "header" => damaged[offset + 1] ^= 1,
            "data" => {
                let i = damaged.len() - 20;
                damaged[i] ^= 1
            }
            "tail" => damaged.truncate(offset + 80),
            _ => damaged.truncate(damaged.len() - 2),
        }
        let source = base.join(format!("{case}.lzo"));
        std::fs::write(&source, &damaged).unwrap();
        let r = scan(&source);
        assert!(!r.complete);
        assert_eq!(r.entries.len(), 4, "{case}: {r:?}");
        assert!(!r.stop_reason.is_empty());
        assert!(lzop::verify(&source, || false).is_err());
        for (e, expected) in r.entries.iter().zip(m["entries"].as_array().unwrap()) {
            assert_eq!(e.name, expected["name"].as_str().unwrap());
            assert_eq!(e.sha256, expected["sha256"].as_str().unwrap());
        }
        let out = recover(&source, &base, &r, &[0, 1, 2]);
        assert!(out.partial);
        assert_eq!(out.recovered_files, 3);
        assert_content(&out, &r);
        assert!(!Path::new(&out.output_directory).join("late.bin").exists());
        assert_eq!(std::fs::read(&source).unwrap(), damaged);
        no_stage(&base);
        evidence.push(serde_json::json!({"outputDirectory":out.output_directory,"entries":out.entries.iter().map(|e|&e.name).collect::<Vec<_>>()}));
    }
    let healthy = base.join("healthy.lzo");
    std::fs::write(&healthy, &data).unwrap();
    assert!(scan(&healthy).complete);
    assert!(lzop::verify(&healthy, || false).is_ok());
    let r = scan(&healthy);
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
    let mut empty = Vec::new();
    lzop_writer::write_stream(
        &mut Cursor::new(Vec::<u8>::new()),
        &mut empty,
        Some("empty.txt"),
        |_| Ok(()),
    )
    .unwrap();
    empty.extend(&data[offset..offset + 12]);
    let p = base.join("empty.lzo");
    std::fs::write(&p, empty).unwrap();
    let r = scan(&p);
    assert_eq!(r.entries.len(), 1);
    assert_eq!(r.entries[0].bytes, 0);
    assert_content(&recover(&p, &base, &r, &[0]), &r);
    std::fs::write(
        base.parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("native-outputs.json"),
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
    std::fs::write(base.parent().unwrap().parent().unwrap().join("native-fixture.json"),serde_json::to_vec_pretty(&serde_json::json!({"source":base.join("tail.lzo"),"healthy":healthy,"base":base,"entries":m["entries"]})).unwrap()).unwrap();
}
#[test]
fn lzop174_numeric_prefix_never_bridges_gaps_and_repaired_source_requires_a_new_check() {
    let (data, m) = fixture();
    let temp = tempfile::tempdir().unwrap();
    let offset = m["entries"][4]["offset"].as_u64().unwrap() as usize;
    let sliced = &data[..offset + 100];
    let size = 257;
    let parts: Vec<PathBuf> = sliced
        .chunks(size)
        .enumerate()
        .map(|(i, b)| {
            let p = temp.path().join(format!("partial.lzo.{:03}", i + 1));
            std::fs::write(&p, b).unwrap();
            p
        })
        .collect();
    let r = scan(parts.last().unwrap());
    assert_eq!(r.entries.len(), 4);
    assert!(r.available_volumes > 2);
    assert!(!r.complete);
    assert_content(
        &recover(parts.last().unwrap(), temp.path(), &r, &[0, 1, 3]),
        &r,
    );
    let missing = parts[4].clone();
    let withheld = std::fs::read(&missing).unwrap();
    std::fs::remove_file(&missing).unwrap();
    let g = scan(parts.last().unwrap());
    assert_eq!(g.entries.len(), 2);
    assert_eq!(g.available_volumes, 4);
    assert!(g.ignored_volumes > 0);
    assert!(!g.complete);
    assert_content(&recover(&parts[0], temp.path(), &g, &[0, 1]), &g);
    assert!(lzop::verify(&parts[0], || false).is_err());
    std::fs::write(&missing, withheld).unwrap();
    assert!(recovery::recover(
        &parts[0],
        temp.path(),
        &g.revision,
        &[0],
        None,
        &flag(),
        |_, _, _| {}
    )
    .is_err());
    std::fs::remove_file(&parts[0]).unwrap();
    assert!(
        recovery::scan(parts.last().unwrap(), None, &flag(), |_, _| {})
            .unwrap_err()
            .to_string()
            .contains("首卷")
    );
    let tar = temp.path().join("archive.tar.lzo");
    std::fs::write(&tar, &data).unwrap();
    assert!(recovery::scan(&tar, None, &flag(), |_, _| {}).is_err());
}
#[test]
fn lzop174_duplicate_names_unchecked_members_and_runtime_cancel_do_not_publish_partial_files() {
    let (data, m) = fixture();
    let temp = tempfile::tempdir().unwrap();
    let first = m["entries"][0]["encodedBytes"].as_u64().unwrap() as usize;
    let later = m["entries"][4]["offset"].as_u64().unwrap() as usize;
    let mut duplicate = data[..first].to_vec();
    duplicate.extend(&data[..first]);
    duplicate.extend(&data[later..later + 40]);
    let p = temp.path().join("duplicate.lzo");
    std::fs::write(&p, &duplicate).unwrap();
    let r = scan(&p);
    assert_eq!(r.entries.len(), 2);
    assert_ne!(
        r.entries[0].name.to_lowercase(),
        r.entries[1].name.to_lowercase()
    );
    let out = recover(&p, temp.path(), &r, &[1]);
    assert_content(&out, &r);
    assert!(!Path::new(&out.output_directory)
        .join(&r.entries[0].name)
        .exists());
    // A valid header and stored payload without a declared data checksum must stop the prefix.
    let mut no_check = Vec::new();
    lzop_writer::write_stream(
        &mut Cursor::new(b"unchecked"),
        &mut no_check,
        Some("unchecked.txt"),
        |_| Ok(()),
    )
    .unwrap();
    let flags = u32::from_be_bytes(no_check[17..21].try_into().unwrap()) & !0x300;
    no_check[17..21].copy_from_slice(&flags.to_be_bytes());
    let header_end = 34 + no_check[33] as usize;
    let checksum = crc32fast::hash(&no_check[9..header_end]);
    no_check[header_end..header_end + 4].copy_from_slice(&checksum.to_be_bytes());
    no_check.drain(header_end + 12..header_end + 16);
    let mut bytes = data[..first].to_vec();
    bytes.extend(no_check);
    bytes.extend(&data[first..]);
    std::fs::write(&p, bytes).unwrap();
    let unchecked = scan(&p);
    assert_eq!(unchecked.entries.len(), 1);
    assert!(unchecked.stop_reason.contains("内容校验"));
    std::fs::write(&p, &data[..data.len() - 2]).unwrap();
    let r = scan(&p);
    let sentinel = temp.path().join("old.txt");
    std::fs::write(&sentinel, b"keep").unwrap();
    let cancel = Arc::new(flag());
    let f = cancel.clone();
    let result = recovery::recover(
        &p,
        temp.path(),
        &r.revision,
        &[0, 1, 2],
        None,
        &cancel,
        move |phase, b, _| {
            if phase == "recovering" && b > 0 {
                f.store(true, Ordering::SeqCst)
            }
        },
    );
    assert!(result.is_err());
    no_stage(temp.path());
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"keep");
    assert_content(&recover(&p, temp.path(), &r, &[2]), &r);
    let f = flag();
    assert!(recovery::scan(&p, None, &f, |b, _| if b > 0 {
        f.store(true, Ordering::SeqCst)
    })
    .is_err());
    let empty = temp.path().join("bad.lzo");
    std::fs::write(&empty, b"").unwrap();
    let r = scan(&empty);
    assert!(!r.complete && r.entries.is_empty());
}
#[tokio::test]
async fn lzop174_diagnosis_and_history_keep_partial_results_distinct() {
    let (data, m) = fixture();
    let temp = tempfile::tempdir().unwrap();
    let p = temp.path().join("bad.lzo");
    std::fs::write(&p, &data[..data.len() - 2]).unwrap();
    let report = super::archive_diagnostics::diagnose_archive(&p, None, Arc::new(flag()))
        .await
        .unwrap();
    assert_eq!(report.actual_format, "LZOP");
    assert_eq!(report.status, "partial_recovery");
    assert_eq!(report.recoverability, "lzop_prefix");
    assert!(
        !report.can_repair && !report.integrity_tested && !report.total_uncompressed_size_known
    );
    assert_eq!(report.lzop_prefix.unwrap().entries.len(), 4);
    let mut bad = data.clone();
    bad[0] = 0;
    std::fs::write(&p, bad).unwrap();
    let report = super::archive_diagnostics::diagnose_archive(&p, None, Arc::new(flag()))
        .await
        .unwrap();
    assert_eq!(report.total_files, 0);
    assert_eq!(report.status, "damaged");
    assert!(m["officialReadback"].as_bool().unwrap());
}

#[tokio::test]
async fn lzop174_diagnosis_keeps_directory_counts_and_unsupported_method_classification(){
 let(data,_)=fixture();let temp=tempfile::tempdir().unwrap();let p=temp.path().join("healthy.lzo");std::fs::write(&p,data).unwrap();let report=super::archive_diagnostics::diagnose_archive(&p,None,Arc::new(flag())).await.unwrap();assert_eq!(report.total_directories,2);assert_eq!(report.status,"healthy");
 let mut bad=Vec::new();lzop_writer::write_stream(&mut Cursor::new(b"payload"),&mut bad,Some("note.txt"),|_|Ok(())).unwrap();bad[15]=7;let end=34+bad[33]as usize;let sum=crc32fast::hash(&bad[9..end]);bad[end..end+4].copy_from_slice(&sum.to_be_bytes());std::fs::write(&p,bad).unwrap();let report=super::archive_diagnostics::diagnose_archive(&p,None,Arc::new(flag())).await.unwrap();assert_eq!(report.status,"unsupported_feature");assert!(!report.can_repair&&report.total_files==0);
}
