use super::{
    archive_browser, archive_diagnostics, archive_preview, lzop,
    universal_engine::UniversalCliEngine,
};
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
};
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/lzop-members-1660")
}
fn manifest() -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn plain() -> Vec<u8> {
    std::fs::read(
        root()
            .parent()
            .unwrap()
            .join("lzop-read-1657/method1.txt.lzo"),
    )
    .unwrap()
}
fn renamed(bytes: &[u8], name: &[u8]) -> Vec<u8> {
    let end = 34 + bytes[33] as usize;
    let mut header = bytes[9..34].to_vec();
    header[24] = name.len() as u8;
    // Independently update the header CRC using the existing CRC library.
    header[8..12].copy_from_slice(
        &(u32::from_be_bytes(bytes[17..21].try_into().unwrap()) | 0x1000).to_be_bytes(),
    );
    header.extend_from_slice(name);
    let crc = crc32fast::hash(&header);
    let mut out = bytes[..9].to_vec();
    out.extend(header);
    out.extend(crc.to_be_bytes());
    out.extend_from_slice(&bytes[end + 4..]);
    out
}
struct Runtime(super::io_buffer_pool::IOBufferPool);
impl super::native_extraction::ExtractionRuntime for Runtime {
    fn check_cancellation(&self) -> anyhow::Result<()> {
        Ok(())
    }
    fn buffer_pool(&self) -> &super::io_buffer_pool::IOBufferPool {
        &self.0
    }
    fn copy_buffer_size(&self) -> usize {
        64 * 1024
    }
    fn normalized_archive_path(&self, p: &Path, _: bool) -> Option<PathBuf> {
        Some(
            p.components()
                .filter(|c| matches!(c, std::path::Component::Normal(_)))
                .collect(),
        )
    }
    fn emit_log(
        &self,
        _: &tauri::Window,
        _: &str,
        _: &str,
        _: crate::models::compression::TaskLogSeverity,
    ) {
    }
    fn emit_progress(&self, _: &tauri::Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {
    }
}

#[tokio::test]
async fn lzop1660_official_members_browse_verify_preview_and_extract_each_file() {
    let runtime = Runtime(super::io_buffer_pool::IOBufferPool::new(Default::default()));
    for case in manifest()["cases"].as_array().unwrap() {
        let p = root().join(case["archive"].as_str().unwrap());
        assert_eq!(
            sha(&std::fs::read(&p).unwrap()),
            case["archiveSha256"].as_str().unwrap()
        );
        let list = archive_browser::browse_archive(&p, None).await.unwrap();
        assert_eq!(list.format, "LZOP");
        assert_eq!(list.total_files, 5);
        let index = lzop::multi_index(&p, || Ok(())).unwrap().unwrap();
        assert_eq!(index.len(), 5);
        UniversalCliEngine::test_integrity(&p, None).await.unwrap();
        let out = tempfile::tempdir().unwrap();
        assert!(
            lzop::extract_members(&runtime, None, "1660", &p, out.path(), &Default::default())
                .unwrap()
        );
        for e in case["expected"].as_array().unwrap() {
            let name = e["path"].as_str().unwrap();
            let m = index.iter().find(|m| m.name == name).unwrap();
            assert_eq!(m.size, e["bytes"].as_u64().unwrap());
            assert!(list.entries.iter().any(|v| v.path == name));
            let mut data = Vec::new();
            lzop::member_reader(&p, m)
                .unwrap()
                .read_to_end(&mut data)
                .unwrap();
            assert_eq!(sha(&data), e["sha256"].as_str().unwrap());
            assert_eq!(
                sha(&std::fs::read(out.path().join(name)).unwrap()),
                e["sha256"].as_str().unwrap()
            );
        }
    }
    let p = root().join("members-1.lzo");
    let text = archive_preview::preview_archive_text(&p, "notes.txt", None)
        .await
        .unwrap();
    assert!(text.content.contains("收到多成员"));
    let large = archive_preview::preview_archive_text(&p, "large.txt", None)
        .await
        .unwrap();
    assert!(large.truncated);
    assert_eq!(
        large.content,
        "LZOP member prefix preview\n".repeat(60000)[..1024 * 1024]
    );
    let image = archive_preview::preview_archive_image(&p, "cover.png", None)
        .await
        .unwrap();
    assert_eq!((image.width, image.height), (32, 32));
    let d = archive_diagnostics::diagnose_archive(&p, None, Arc::new(AtomicBool::new(false)))
        .await
        .unwrap();
    assert_eq!(d.status, "healthy");
    assert!(d.integrity_tested);
    let out = tempfile::tempdir().unwrap();
    let options = crate::models::compression::DecompressOptions {
        selected_entries: vec!["notes.txt".into(), "cover.png".into()],
        file_filter: Some("*.txt".into()),
        ..Default::default()
    };
    lzop::extract_members(&runtime, None, "1660", &p, out.path(), &options).unwrap();
    assert!(out.path().join("notes.txt").exists());
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 1);
}
#[tokio::test]
async fn lzop1660_names_duplicates_and_unnamed_members_keep_distinct_contents() {
    let bytes = plain();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("names.lzo");
    let names: [&[u8]; 6] = [
        b"../same.txt",
        b"SAME.txt",
        b"",
        b"CON",
        b"C:\\path\\same.txt",
        b"\xff.txt",
    ];
    std::fs::write(
        &path,
        names
            .iter()
            .map(|n| renamed(&bytes, n))
            .flatten()
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let list = archive_browser::browse_archive(&path, None).await.unwrap();
    let rows: Vec<_> = list.entries.iter().map(|e| e.path.as_str()).collect();
    assert_eq!(
        rows,
        vec![
            "same.txt",
            "SAME (member 2).txt",
            "member-003.bin",
            "member-4-CON",
            "same (member 5).txt",
            "member-006.bin"
        ]
    );
    let runtime = Runtime(super::io_buffer_pool::IOBufferPool::new(Default::default()));
    let out = tempfile::tempdir().unwrap();
    lzop::extract_members(
        &runtime,
        None,
        "1660",
        &path,
        out.path(),
        &Default::default(),
    )
    .unwrap();
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 6);
    UniversalCliEngine::test_integrity(&path, None)
        .await
        .unwrap();
    for row in rows.iter().filter(|p| p.ends_with(".txt")) {
        let p = archive_preview::preview_archive_text(&path, row, None)
            .await
            .unwrap();
        assert!(p.content.contains("先填写密码"));
    }
}
#[tokio::test]
async fn lzop1660_numeric_slices_second_member_damage_and_cancel_are_not_healthy() {
    let bytes = std::fs::read(root().join("members-3.lzo")).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let mut parts = Vec::new();
    let cuts = [0, 4, bytes.len() / 2, bytes.len() - 2, bytes.len()];
    for (i, w) in cuts.windows(2).enumerate() {
        let p = temp.path().join(format!("group.lzo.{:03}", i + 1));
        std::fs::write(&p, &bytes[w[0]..w[1]]).unwrap();
        parts.push(p);
    }
    let source = &parts[1];
    assert_eq!(
        archive_browser::browse_archive(source, None)
            .await
            .unwrap()
            .total_files,
        5
    );
    assert!(
        archive_preview::preview_archive_text(source, "notes.txt", None)
            .await
            .unwrap()
            .content
            .contains("收到多成员")
    );
    UniversalCliEngine::test_integrity(source, None)
        .await
        .unwrap();
    std::fs::remove_file(&parts[2]).unwrap();
    assert!(
        format!("{:#}", lzop::multi_index(source, || Ok(())).err().unwrap())
            .contains("group.lzo.003")
    );
    std::fs::write(&parts[2], &bytes[cuts[2]..cuts[3]]).unwrap();
    std::fs::remove_file(&parts[3]).unwrap();
    assert!(format!(
        "{:#}",
        UniversalCliEngine::test_integrity(source, None)
            .await
            .unwrap_err()
    )
    .contains("可能缺少后续卷"));
    std::fs::write(&parts[3], &bytes[cuts[3]..]).unwrap();
    UniversalCliEngine::test_integrity(source, None)
        .await
        .unwrap();
    let bad = temp.path().join("bad.lzo");
    let mut damaged = bytes.clone();
    let second = bytes
        .windows(9)
        .enumerate()
        .filter(|(_, w)| *w == b"\x89LZO\0\r\n\x1a\n")
        .nth(1)
        .unwrap()
        .0;
    let block = second + 38 + bytes[second + 33] as usize;
    damaged[block + 8] ^= 1;
    std::fs::write(&bad, damaged).unwrap();
    assert!(lzop::multi_index(&bad, || Ok(())).is_ok());
    let error = format!(
        "{:#}",
        UniversalCliEngine::test_integrity(&bad, None)
            .await
            .unwrap_err()
    );
    assert!(error.contains("large.txt"), "{error}");
    let d = archive_diagnostics::diagnose_archive(&bad, None, Arc::new(AtomicBool::new(false)))
        .await
        .unwrap();
    assert_ne!(d.status, "healthy");
    assert!(lzop::verify(source, || true)
        .unwrap_err()
        .to_string()
        .contains("取消"));
    let mut checks = 0;
    assert!(lzop::multi_index(source, || {
        checks += 1;
        anyhow::ensure!(checks < 4, "取消索引");
        Ok(())
    })
    .unwrap_err()
    .to_string()
    .contains("取消索引"));
    let extra = temp.path().join("extra.lzo");
    let mut trailing = bytes.clone();
    trailing.push(1);
    std::fs::write(&extra, trailing).unwrap();
    assert!(lzop::multi_index(&extra, || Ok(())).is_err());
    // Multiple independent files must not be guessed to be one TAR stream.
    let misleading = temp.path().join("misleading.tar.lzo");
    std::fs::write(&misleading, bytes).unwrap();
    assert!(archive_browser::browse_archive(&misleading, None)
        .await
        .is_err());
}

#[tokio::test]
async fn lzop1660_explicit_latin1_precedes_utf8_guess_for_the_same_name_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("encodings.lzo");
    let renamed = renamed(&plain(), b"\xc3\xa9.txt");
    let end = 34 + renamed[33] as usize;
    let mut sequence = Vec::new();
    for charset in [0x00100000u32, 0x00500000] {
        let mut member = renamed.clone();
        let flags = u32::from_be_bytes(member[17..21].try_into().unwrap());
        member[17..21].copy_from_slice(&((flags & !0x00f00000) | charset).to_be_bytes());
        let crc = crc32fast::hash(&member[9..end]);
        member[end..end + 4].copy_from_slice(&crc.to_be_bytes());
        sequence.extend(member);
    }
    std::fs::write(&path, sequence).unwrap();
    let list = archive_browser::browse_archive(&path, None).await.unwrap();
    assert_eq!(
        list.entries
            .iter()
            .map(|e| e.path.as_str())
            .collect::<Vec<_>>(),
        vec!["Ã©.txt", "é.txt"]
    );
    for name in ["Ã©.txt", "é.txt"] {
        assert!(archive_preview::preview_archive_text(&path, name, None)
            .await
            .unwrap()
            .content
            .contains("先填写密码"));
    }
    let runtime = Runtime(super::io_buffer_pool::IOBufferPool::new(Default::default()));
    let out = tempfile::tempdir().unwrap();
    lzop::extract_members(
        &runtime,
        None,
        "1660",
        &path,
        out.path(),
        &Default::default(),
    )
    .unwrap();
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 2);
    assert_eq!(
        std::fs::read(out.path().join("Ã©.txt")).unwrap(),
        std::fs::read(out.path().join("é.txt")).unwrap()
    );
}
