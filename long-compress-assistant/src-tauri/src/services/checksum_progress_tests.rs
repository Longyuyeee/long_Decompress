use super::{checksum_manifest::{verify_with_progress, ChecksumEntryStatus as S}, file_integrity_service::{ChecksumAlgorithm as A, FileIntegrityService as F}, checksum_progress::calculate_batch};
use std::cell::Cell;

#[test]
fn checksum1522_real_chunks_cancel_all_algorithms_without_digest() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?; let path = dir.path().join("large.bin");
    std::fs::write(&path, vec![7u8; 200_000])?;
    for algorithm in [A::CRC32, A::MD5, A::SHA256] {
        let mut chunks = vec![];
        assert!(F::calculate_checksum_with_progress(&path, algorithm, &mut |bytes, size| { chunks.push(bytes); assert_eq!(size, 200_000); bytes < 65_536 })?.is_none());
        assert_eq!(chunks, [0, 65_536]);
        let mut total = 0;
        let result = F::calculate_checksum_with_progress(&path, algorithm, &mut |bytes, _| { assert!(bytes >= total); total = bytes; true })?.unwrap();
        assert_eq!(total, 200_000); assert_eq!(result.checksum, F::calculate_checksum(&path, algorithm)?.checksum);
    }
    Ok(())
}

#[test]
fn checksum1522_cancel_retains_finished_and_resume_reads_only_remaining() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("done.txt"), b"Hello, World!")?;
    std::fs::write(dir.path().join("large.bin"), vec![8u8; 200_000])?;
    let large_hash = F::calculate_checksum(&dir.path().join("large.bin"), A::MD5)?.checksum;
    let path = dir.path().join("checks.md5");
    std::fs::write(&path, format!("65a8e27d8879283831b664bd8b7f0ad4  done.txt\n{large_hash}  large.bin\n{large_hash}  later.bin\n"))?;
    let stopped = Cell::new(false);
    let report = verify_with_progress(&path, dir.path(), None, None, &|| stopped.get(), &mut |p| {
        assert!(p.completed_bytes <= p.total_bytes);
        if p.current_file.ends_with("large.bin") && p.file_bytes >= 65_536 { stopped.set(true); }
    })?;
    assert!(report.cancelled); assert!(!report.valid);
    assert_eq!(report.entries.iter().map(|r| r.status).collect::<Vec<_>>(), [S::Matched, S::Interrupted, S::NotChecked]);
    assert!(report.entries[1].actual_checksum.is_none());
    std::fs::remove_file(dir.path().join("done.txt"))?;
    let mut observed = vec![];
    let resumed = verify_with_progress(&path, dir.path(), Some(&[2, 3]), Some(&report.manifest_id), &|| false, &mut |p| { if !p.current_file.is_empty() { observed.push(p.current_file); } })?;
    assert_eq!(resumed.entries.iter().map(|r| r.status).collect::<Vec<_>>(), [S::NotChecked, S::Matched, S::Missing]);
    assert!(observed.iter().all(|p| !p.ends_with("done.txt")));
    Ok(())
}

#[test]
fn checksum1522_changed_manifest_never_reads_old_line_selection() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?; let path = dir.path().join("checks.md5");
    std::fs::write(&path, "65a8e27d8879283831b664bd8b7f0ad4  other.txt\n")?;
    let mut calls = 0;
    let report = verify_with_progress(&path, dir.path(), Some(&[1]), Some("old-manifest"), &|| false, &mut |p| { calls += 1; assert!(p.current_file.is_empty()); assert_eq!(p.total_bytes, 0); })?;
    assert_eq!(calls, 1); assert_eq!(report.entries[0].status, S::NotChecked); assert!(!report.valid);
    Ok(())
}

#[test]
fn checksum1522_changing_file_during_read_is_not_matched() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?; let path = dir.path().join("changing.bin");
    std::fs::write(&path, vec![1u8; 200_000])?;
    let mut changed = false;
    let error = F::calculate_checksum_with_progress(&path, A::SHA256, &mut |bytes, _| {
        if bytes > 0 && !changed { changed = true; std::fs::write(&path, b"changed").unwrap(); } true
    }).unwrap_err();
    assert!(error.to_string().contains("发生变化"));
    Ok(())
}

#[test]
fn checksum1522_batch_keeps_completed_and_interrupts_current() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let paths: Vec<_> = ["first", "second", "third"].iter().map(|s| dir.path().join(s).to_string_lossy().into_owned()).collect();
    for p in &paths { std::fs::write(p, vec![9u8; 200_000])?; }
    let stop = Cell::new(false);
    let rows = calculate_batch(&paths, A::SHA256, "sha256", &|| stop.get(), &mut |p| {
        if p.current_file == paths[1] && p.file_bytes > 0 { stop.set(true); }
    });
    assert_eq!(rows.iter().map(|r| r.status.as_str()).collect::<Vec<_>>(), ["success", "interrupted", "pending"]);
    assert!(rows[1].checksum.is_empty()); assert!(rows[2].checksum.is_empty());
    Ok(())
}
