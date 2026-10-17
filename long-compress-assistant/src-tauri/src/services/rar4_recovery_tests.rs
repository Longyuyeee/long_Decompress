use super::*;
fn fixture(name: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/rar4-recovery-1419").join(name) }
fn copy_case(name: &str, root: &Path) -> PathBuf {
    fs::create_dir_all(root).unwrap();
    for e in fs::read_dir(fixture(name)).unwrap() { let p = e.unwrap().path(); fs::copy(&p, root.join(p.file_name().unwrap())).unwrap(); }
    root.join(if name.starts_with("modern") { "原始归档.part1.rar" } else if name == "no-record" { "无恢复记录.rar" } else { "原始归档.rar" })
}
fn flag() -> Arc<AtomicBool> { Arc::new(AtomicBool::new(false)) }
fn snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = fs::read_dir(root).unwrap().map(|e| { let p = e.unwrap().path(); let b = fs::read(&p).unwrap(); (p, b) }).collect::<Vec<_>>();
    files.sort_by_key(|v| v.0.clone()); files
}
fn assert_originals(files: &[(PathBuf, Vec<u8>)]) { for (p, b) in files { assert_eq!(&fs::read(p).unwrap(), b, "original changed: {}", p.display()); } }

#[test]
fn real_rar4_recovery_groups_resolve_from_every_member() {
    for style in ["old", "modern"] { for mode in ["none", "content", "headers"] {
        for entry in fs::read_dir(fixture(&format!("{style}-{mode}"))).unwrap() {
            let mut g = collect(&entry.unwrap().path()).unwrap(); check_rar4_recovery(&mut g, &flag()).unwrap();
            assert!(g.rar4); assert_eq!(g.data.len(), 4); assert_eq!(g.recovery.len(), 2);
            assert_eq!(g.recovery_data_count, Some(4)); assert!(recovery_missing(&g).is_empty());
        }
    }}
}
#[test]
fn invalid_recovery_is_not_counted_or_copied() {
    let dir = tempfile::tempdir().unwrap(); let first = copy_case("old-none", dir.path());
    fs::write(dir.path().join("原始归档1.rev"), b"short").unwrap();
    let mut g = collect(&first).unwrap(); check_rar4_recovery(&mut g, &flag()).unwrap();
    assert_eq!(g.recovery.len(), 1); assert_eq!(g.rejected_recovery.len(), 1);
    assert!(!g.files.contains(&dir.path().join("原始归档1.rev")));
    let second = dir.path().join("原始归档2.rev"); let mut b = fs::read(&second).unwrap(); b[50] ^= 1; fs::write(second, b).unwrap();
    let mut g = collect(&first).unwrap(); check_rar4_recovery(&mut g, &flag()).unwrap();
    assert!(g.recovery.is_empty()); assert_eq!(g.rejected_recovery.len(), 2);
}
#[tokio::test]
async fn duplicate_indices_are_rejected_and_invalid_recovery_has_one_complete_issue() {
    let dir = tempfile::tempdir().unwrap(); let first = copy_case("old-none", dir.path());
    fs::copy(dir.path().join("原始归档1.rev"), dir.path().join("原始归档2.rev")).unwrap();
    let mut g = collect(&first).unwrap(); assert!(check_rar4_recovery(&mut g, &flag()).unwrap_err().to_string().contains("序号重复"));
    for name in ["原始归档1.rev", "原始归档2.rev"] { fs::write(dir.path().join(name), b"short").unwrap(); }
    let r = diagnose(&first, None, flag()).await.unwrap();
    let issues = r.issues.iter().filter(|i| i.code == "invalid_recovery_volume").collect::<Vec<_>>();
    assert_eq!(issues.len(), 1); assert!(issues[0].detail.contains("1.rev") && issues[0].detail.contains("2.rev"));
}
#[test]
fn ambiguous_trailing_digits_do_not_mix_families() {
    let dir = tempfile::tempdir().unwrap(); let first = copy_case("old-none", dir.path());
    fs::copy(&first, dir.path().join("原始归档1.rar")).unwrap();
    assert!(collect(&dir.path().join("原始归档12.rev")).unwrap_err().to_string().contains("多个"));
    assert!(collect(&first).unwrap_err().to_string().contains("多个"));
}
#[tokio::test]
async fn rev_footer_reports_missing_tail_and_insufficient_recovery_before_password() {
    let dir = tempfile::tempdir().unwrap(); let first = copy_case("old-headers", dir.path());
    fs::remove_file(dir.path().join("原始归档.r02")).unwrap();
    let r = diagnose(&first, Some("wrong"), flag()).await.unwrap();
    assert_eq!(r.status, "missing_volume"); assert!(r.missing_volumes.iter().any(|p| p.ends_with(".r02")));
    for n in ["原始归档.r00", "原始归档.r01"] { fs::remove_file(dir.path().join(n)).unwrap(); }
    let r = diagnose(&first, None, flag()).await.unwrap(); assert!(!r.can_repair); assert_eq!(r.missing_volumes.len(), 3);
    let out = dir.path().join("out"); fs::create_dir(&out).unwrap();
    assert!(repair(&first, &out, None, flag()).await.unwrap_err().to_string().contains("恢复卷不足"));
    assert_eq!(fs::read_dir(out).unwrap().count(), 0);
}
#[tokio::test]
#[ignore = "Requires current external Rar.exe; genuine pinned RAR4 record repair"]
async fn real_rar4_record_repair_retains_all_password_modes() {
    for style in ["single", "old", "modern"] { for mode in ["none", "content", "headers"] {
        let dir = tempfile::tempdir().unwrap(); let input = dir.path().join("input"); let first = copy_case(&format!("{style}-{mode}"), &input);
        for p in collect(&first).unwrap().recovery { fs::remove_file(p).unwrap(); }
        let damaged = if style == "single" { first.clone() } else { collect(&first).unwrap().data[1].clone() };
        let mut b = fs::read(&damaged).unwrap(); b[3000] ^= 0x55; fs::write(damaged, b).unwrap();
        let originals = snapshot(&input); let out = dir.path().join("out"); fs::create_dir(&out).unwrap();
        let password = (mode != "none").then_some("Rar4-1419-中文!");
        let r = repair(&first, &out, password, flag()).await.unwrap_or_else(|e| panic!("{style}-{mode}: {e:#}"));
        assert!(r.verified); assert!(!r.partial); assert_eq!(r.recovered_files, 2); assert_eq!(r.output_encrypted, password.is_some()); assert_originals(&originals);
        assert!(repair(&first, &out, password, flag()).await.is_err());
    }}
}
#[tokio::test]
#[ignore = "Requires current external Rar.exe; missing first/middle/last/two original RAR4 disks"]
async fn real_rar4_rev_rebuilds_both_naming_styles_and_all_password_modes() {
    for style in ["old", "modern"] { for mode in ["none", "content", "headers"] { for missing in [&[0][..], &[1][..], &[3][..], &[0, 2][..]] {
        let dir = tempfile::tempdir().unwrap(); let input = dir.path().join("input"); let first = copy_case(&format!("{style}-{mode}"), &input);
        let g = collect(&first).unwrap(); let expected = snapshot(&input);
        for &i in missing { fs::remove_file(&g.data[i]).unwrap(); }
        let originals = snapshot(&input); let out = dir.path().join("out"); fs::create_dir(&out).unwrap();
        let password = (mode != "none").then_some("Rar4-1419-中文!");
        let report = diagnose(&g.recovery[0], password, flag()).await.unwrap(); assert!(report.can_repair, "{style}-{mode}-{missing:?}: {report:?}");
        let r = repair(&g.recovery[0], &out, password, flag()).await.unwrap_or_else(|e| panic!("{style}-{mode}-{missing:?}: {e:#}"));
        assert_eq!(r.output_files.len(), 4); assert_eq!(r.recovered_files, 2); assert_eq!(r.output_encrypted, password.is_some()); assert_originals(&originals);
        for &i in missing { assert!(!g.data[i].exists()); }
        for file in &r.output_files { let p = Path::new(file); let original = expected.iter().find(|(o, _)| o.file_name() == p.file_name()).unwrap(); assert_eq!(fs::read(p).unwrap(), original.1, "{style}-{mode}-{missing:?}: {}", p.display()); }
    }}}
}
#[tokio::test]
#[ignore = "Requires external Rar.exe; damaged data without recovery and wrong passwords publish nothing"]
async fn unrecoverable_rar4_and_wrong_password_publish_nothing() {
    for name in ["no-record", "single-headers"] {
        let dir = tempfile::tempdir().unwrap(); let input = dir.path().join("input"); let first = copy_case(name, &input);
        let mut b = fs::read(&first).unwrap(); b[3000] ^= 0x55; fs::write(&first, b).unwrap();
        let originals = snapshot(&input); let out = dir.path().join("out"); fs::create_dir(&out).unwrap();
        assert!(repair(&first, &out, (name == "single-headers").then_some("wrong"), flag()).await.is_err());
        assert_eq!(fs::read_dir(&out).unwrap().count(), 0); assert_originals(&originals);
        assert!(repair(&first, &out, None, Arc::new(AtomicBool::new(true))).await.is_err()); assert_eq!(fs::read_dir(&out).unwrap().count(), 0);
    }
}

#[tokio::test]
#[ignore = "Requires current external Rar.exe to create genuine RAR4 recovery volumes"]
async fn current_tool_creates_rar4_recovery_and_ignores_a_truncated_sibling() {
    for style in ["old", "modern"] {
        let dir = tempfile::tempdir().unwrap(); let input = dir.path().join("input"); let first = copy_case(&format!("{style}-headers"), &input);
        let original_group = collect(&first).unwrap();
        for p in original_group.recovery { fs::remove_file(p).unwrap(); }
        let originals = snapshot(&input); let saved = dir.path().join("recovery"); fs::create_dir(&saved).unwrap();
        let r = create_recovery_volumes(&first, &saved, 2, Some("Rar4-1419-中文!"), flag()).await.unwrap();
        assert_eq!(r.output_files.len(), 2); assert_originals(&originals);
        for file in r.output_files { let p = Path::new(&file); fs::copy(p, input.join(p.file_name().unwrap())).unwrap(); }
        let g = collect(&first).unwrap(); fs::write(&g.recovery[0], b"short").unwrap(); fs::remove_file(&g.data[3]).unwrap();
        let report = diagnose(&first, Some("Rar4-1419-中文!"), flag()).await.unwrap();
        assert!(report.can_repair); assert!(report.issues.iter().any(|i| i.code == "invalid_recovery_volume"));
        let kept = snapshot(&input); let out = dir.path().join("out"); fs::create_dir(&out).unwrap();
        let repaired = repair(&first, &out, Some("Rar4-1419-中文!"), flag()).await.unwrap();
        assert_eq!(repaired.recovered_files, 2); assert_originals(&kept);
        let wrong = dir.path().join("wrong"); fs::create_dir(&wrong).unwrap();
        assert!(repair(&first, &wrong, Some("wrong"), flag()).await.is_err());
        assert_eq!(fs::read_dir(wrong).unwrap().count(), 0);
    }
}
