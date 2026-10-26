use super::{
    archive_browser, archive_preview, compression_format, compression_verification, lzop,
    lzop_writer,
};
use crate::models::compression::CompressionOptions;
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

struct Runtime {
    checks: AtomicUsize,
    cancel_at: usize,
    cancelled: Arc<AtomicBool>,
}
impl super::native_compression::CompressionRuntime for Runtime {
    fn check_cancellation(&self) -> anyhow::Result<()> {
        if self.checks.fetch_add(1, Ordering::SeqCst) >= self.cancel_at {
            self.cancelled.store(true, Ordering::SeqCst);
        }
        anyhow::ensure!(!self.cancelled.load(Ordering::SeqCst), "操作已取消");
        Ok(())
    }
    fn cancellation_flag(&self) -> Arc<AtomicBool> {
        self.cancelled.clone()
    }
    fn copy_buffer_size(&self) -> usize {
        65536
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
fn runtime(cancel_at: usize) -> Runtime {
    Runtime {
        checks: AtomicUsize::new(0),
        cancel_at,
        cancelled: Arc::new(AtomicBool::new(false)),
    }
}
fn options() -> CompressionOptions {
    CompressionOptions {
        format: Some("lzo".into()),
        ..Default::default()
    }
}
fn sha(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
fn write(path: &Path, data: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, data).unwrap();
}

#[tokio::test]
async fn lzop172_fresh_members_preserve_all_names_payloads_and_previews() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test-results/lzop-multi-create-172/generated");
    let image = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/archives/aes-codecs-1521/原文件/图标.png"),
    )
    .unwrap();
    let mut state = 172u64;
    let random: Vec<u8> = (0..700001)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect();
    let longest = format!("{}aa.txt", "文".repeat(83));
    assert_eq!(longest.len(), 255);
    let fixtures = [
        (
            "one/notes.txt".to_owned(),
            b"first duplicate".to_vec(),
            "notes.txt".to_owned(),
        ),
        (
            "two/notes.txt".to_owned(),
            b"second duplicate".to_vec(),
            "notes (1).txt".to_owned(),
        ),
        (
            "three/NOTES.TXT".to_owned(),
            b"case insensitive duplicate".to_vec(),
            "NOTES (2).TXT".to_owned(),
        ),
        (
            "资料.txt".to_owned(),
            "每个成员独立保存。\n".repeat(9000).into_bytes(),
            "资料.txt".to_owned(),
        ),
        ("empty.txt".to_owned(), vec![], "empty.txt".to_owned()),
        ("cover.png".to_owned(), image, "cover.png".to_owned()),
        ("random.bin".to_owned(), random, "random.bin".to_owned()),
        (longest.clone(), b"255 UTF-8 bytes".to_vec(), longest),
        (
            ".hidden".to_owned(),
            b"hidden file".to_vec(),
            ".hidden".to_owned(),
        ),
    ];
    let mut proof = Vec::new();
    for (case, selected) in [
        ("multiple", (0..fixtures.len()).collect::<Vec<_>>()),
        ("single-renamed", vec![3]),
        ("all-empty", vec![4, 4]),
    ] {
        let sources: Vec<_> = selected
            .iter()
            .map(|&i| {
                let path = root.join("source").join(&fixtures[i].0);
                write(&path, &fixtures[i].1);
                path.to_string_lossy().into_owned()
            })
            .collect();
        let archive = root.join(format!("{case}.lzo"));
        compression_format::validate_compression_request(
            &sources,
            &archive.to_string_lossy(),
            &options(),
        )
        .unwrap();
        lzop_writer::compress(
            &runtime(usize::MAX),
            None,
            "172",
            &sources,
            &archive.to_string_lossy(),
            options(),
        )
        .unwrap();
        assert!(compression_verification::verify_native(
            compression_format::CompressionRoute::Lzop,
            &archive,
            None,
            || false
        )
        .unwrap());
        let list = archive_browser::browse_archive(&archive, None)
            .await
            .unwrap();
        let members = lzop::member_index(&archive, || Ok(())).unwrap();
        assert_eq!(members.len(), selected.len());
        let mut expected = Vec::new();
        for (member_index, (&i, member)) in selected.iter().zip(&members).enumerate() {
            let name = if case == "all-empty" && member_index == 1 {
                "empty (1).txt"
            } else {
                &fixtures[i].2
            };
            assert_eq!(member.name, name);
            assert!(list.entries.iter().any(|e| e.path == name));
            let mut data = Vec::new();
            lzop::member_reader(&archive, member)
                .unwrap()
                .read_to_end(&mut data)
                .unwrap();
            assert_eq!(data, fixtures[i].1);
            expected.push(serde_json::json!({"path":name,"bytes":data.len(),"sha256":sha(&data)}));
        }
        if case == "multiple" {
            assert!(
                archive_preview::preview_archive_text(&archive, "资料.txt", None)
                    .await
                    .unwrap()
                    .content
                    .contains("每个成员")
            );
            assert_eq!(
                archive_preview::preview_archive_image(&archive, "cover.png", None)
                    .await
                    .unwrap()
                    .width,
                32
            );
        }
        proof.push(serde_json::json!({"archive":archive,"archiveSha256":sha(&std::fs::read(&archive).unwrap()),"entries":expected}));
    }
    std::fs::write(
        root.join("manifest.json"),
        serde_json::to_vec_pretty(&proof).unwrap(),
    )
    .unwrap();
}

#[test]
fn lzop172_preflight_rejects_lossy_requests_before_output_and_analysis_respects_members() {
    let temp = tempfile::tempdir().unwrap();
    let a = temp.path().join("a.txt");
    let b = temp.path().join("b.txt");
    write(&a, &b"independent encoder samples\n".repeat(8000));
    write(&b, b"second member");
    let sources = vec![
        a.to_string_lossy().into_owned(),
        b.to_string_lossy().into_owned(),
    ];
    let capability = compression_format::find_compression_format_capability("lzo").unwrap();
    assert!(!capability.single_file_only);
    for changed in [
        CompressionOptions {
            password: Some("not-supported".into()),
            ..options()
        },
        CompressionOptions {
            split_size: Some(1024),
            ..options()
        },
    ] {
        assert!(
            compression_format::validate_compression_request(&sources, "out.lzo", &changed)
                .is_err()
        );
    }
    let x = super::compression_analysis::analyze_compression(
        &sources,
        "lzo",
        1,
        &AtomicBool::new(false),
    )
    .unwrap();
    let y = super::compression_analysis::analyze_compression(
        &sources,
        "lzo",
        9,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(x.file_count, 2);
    assert_eq!(x.estimated_size, y.estimated_size);
    assert_eq!(x.estimated_seconds_high, y.estimated_seconds_high);
    let long = temp.path().join(format!("{}.txt", "文".repeat(85)));
    write(&long, b"never drop a name");
    for bad in [
        vec![
            sources[0].clone(),
            temp.path().to_string_lossy().into_owned(),
        ],
        vec![sources[0].clone(), long.to_string_lossy().into_owned()],
    ] {
        let target = temp.path().join("must-not-exist.lzo");
        assert!(compression_format::validate_compression_request(
            &bad,
            &target.to_string_lossy(),
            &options()
        )
        .unwrap_err()
        .to_string()
        .contains("TAR.LZO"));
        assert!(lzop_writer::compress(
            &runtime(usize::MAX),
            None,
            "bad",
            &bad,
            &target.to_string_lossy(),
            options()
        )
        .is_err());
        assert!(!target.exists());
    }
}

#[test]
fn lzop172_cancellation_in_later_member_discards_staging_and_retry_keeps_sources() {
    let temp = tempfile::tempdir().unwrap();
    let a = temp.path().join("first.txt");
    let b = temp.path().join("later.bin");
    write(&a, b"completed earlier member");
    write(&b, &vec![172; 6 * 1024 * 1024]);
    let sources = vec![
        a.to_string_lossy().into_owned(),
        b.to_string_lossy().into_owned(),
    ];
    let before = sha(&std::fs::read(&b).unwrap());
    let stage = tempfile::tempdir_in(temp.path()).unwrap();
    let pending = stage.path().join("partial.lzo");
    assert!(lzop_writer::compress(
        &runtime(30),
        None,
        "cancel",
        &sources,
        &pending.to_string_lossy(),
        options()
    )
    .unwrap_err()
    .to_string()
    .contains("取消"));
    assert!(lzop::member_index(&pending, || Ok(())).is_err());
    drop(stage);
    assert!(!pending.exists());
    let retry = temp.path().join("retry.lzo");
    lzop_writer::compress(
        &runtime(usize::MAX),
        None,
        "retry",
        &sources,
        &retry.to_string_lossy(),
        options(),
    )
    .unwrap();
    lzop::verify(&retry, || false).unwrap();
    assert_eq!(sha(&std::fs::read(&b).unwrap()), before);
}
