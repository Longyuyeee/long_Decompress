use super::tar::extract_inner;
use super::ExtractionRuntime;
use crate::models::compression::{DecompressOptions, TaskLogSeverity};
use crate::services::io_buffer_pool::IOBufferPool;
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::cell::Cell;
use std::io::Read;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use tauri::Window;

#[derive(Default)]
struct Runtime {
    pool: IOBufferPool,
    written: Cell<u64>,
    cancel_after: Option<u64>,
    write_error: Option<std::io::ErrorKind>,
}
impl ExtractionRuntime for Runtime {
    fn check_cancellation(&self) -> Result<()> {
        if self.cancel_after.is_some_and(|n| self.written.get() >= n) {
            return Err(crate::services::compression_service::CompressionError::Cancelled.into());
        }
        Ok(())
    }
    fn write_extracted_chunk(&self, writer: &mut dyn Write, bytes: &[u8]) -> Result<()> {
        if let Some(kind) = self.write_error {
            return Err(std::io::Error::from(kind).into());
        }
        writer.write_all(bytes)?;
        self.written.set(self.written.get() + bytes.len() as u64);
        Ok(())
    }
    fn buffer_pool(&self) -> &IOBufferPool {
        &self.pool
    }
    fn copy_buffer_size(&self) -> usize {
        64 * 1024
    }
    fn normalized_archive_path(&self, path: &Path, preserve: bool) -> Option<PathBuf> {
        let path = if preserve {
            path
        } else {
            Path::new(path.file_name()?)
        };
        let clean: PathBuf = path
            .components()
            .filter_map(|c| {
                if let Component::Normal(p) = c {
                    Some(p)
                } else {
                    None
                }
            })
            .collect();
        (!clean.as_os_str().is_empty()).then_some(clean)
    }
    fn emit_log(&self, _: &Window, _: &str, _: &str, _: TaskLogSeverity) {}
    fn emit_progress(&self, _: &Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {}
}
fn fixture() -> (PathBuf, serde_json::Value) {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/tar-1513");
    let m = serde_json::from_slice(&std::fs::read(p.join("manifest.json")).unwrap()).unwrap();
    (p, m)
}

#[tokio::test]
async fn original_zip_with_mistaken_tar_suffix_keeps_its_real_integrity_route() {
    let temp = tempfile::tempdir().unwrap();
    let mut writer = zip_aes::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    writer
        .start_file("实际资料.txt", zip_aes::write::SimpleFileOptions::default())
        .unwrap();
    writer
        .write_all("文件内容必须按真实 ZIP 读取".as_bytes())
        .unwrap();
    let original = writer.finish().unwrap().into_inner();
    for name in ["用户误命名.tar", "用户误命名.tar.gz", "用户误命名.tar.xz"] {
        let file = temp.path().join(name);
        std::fs::write(&file, &original).unwrap();
        assert_eq!(
            crate::services::archive_browser::detect_format(&file).unwrap(),
            crate::services::archive_format::ArchiveFormat::Zip
        );
        crate::services::universal_engine::UniversalCliEngine::test_integrity(&file, None)
            .await
            .unwrap();
        let browse = crate::services::archive_browser::browse_archive(&file, None)
            .await
            .unwrap();
        assert_eq!(browse.format, "ZIP");
        assert_eq!(browse.total_files, 1);
        let preview =
            crate::services::archive_preview::preview_archive_text(&file, "实际资料.txt", None)
                .await
                .unwrap();
        assert_eq!(preview.content, "文件内容必须按真实 ZIP 读取");
        assert_eq!(std::fs::read(&file).unwrap(), original);
    }
}

#[tokio::test]
async fn real_tar_signature_with_mistaken_wrapped_or_zip_suffix_stays_raw_tar() {
    let (base, m) = fixture();
    let temp = tempfile::tempdir().unwrap();
    for name in ["实际TAR.zip", "实际TAR.tar.gz", "实际TAR.bin"] {
        let file = temp.path().join(name);
        std::fs::copy(base.join("pax10.tar"), &file).unwrap();
        let browse = crate::services::archive_browser::browse_archive(&file, None)
            .await
            .unwrap();
        assert_eq!(browse.format, "TAR");
        assert_eq!(browse.total_files, 5);
        assert_eq!(browse.total_uncompressed_size, 8521131);
        crate::services::universal_engine::UniversalCliEngine::test_integrity(&file, None)
            .await
            .unwrap();
        let preview = crate::services::archive_preview::preview_archive_text(
            &file,
            m["longName"].as_str().unwrap(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(
            preview.content,
            "用户只要这份长路径说明。\nPAX / GNU long name.\n"
        );
    }
}

#[test]
fn logical_plan_counts_validated_sparse_payload_without_reading_holes() {
    let (base, m) = fixture();
    for c in m["cases"].as_array().unwrap() {
        let file = base.join(c["archive"].as_str().unwrap());
        let plan = crate::services::tar_compatibility::plan(
            &file,
            route(&file),
            &DecompressOptions::default(),
            || Ok(()),
        )
        .unwrap();
        assert_eq!(plan.logical_bytes, 8521131, "{}", file.display());
        assert_eq!(
            plan.payload_bytes,
            if c["layout"] == "pax-ordinary" {
                8521131
            } else {
                30090
            },
            "{}",
            file.display()
        );
        let selected = DecompressOptions {
            selected_entries: vec!["稀疏.bin".into()],
            ..Default::default()
        };
        let plan =
            crate::services::tar_compatibility::plan(&file, route(&file), &selected, || Ok(()))
                .unwrap();
        assert_eq!(plan.entries, 1);
        assert_eq!(plan.logical_bytes, 8388641);
        assert_eq!(
            plan.payload_bytes,
            if c["layout"] == "pax-ordinary" {
                8388641
            } else {
                28672
            }
        );
    }
    let file = base.join("control/control.tar");
    let plan = crate::services::tar_compatibility::plan(
        &file,
        route(&file),
        &DecompressOptions::default(),
        || Ok(()),
    )
    .unwrap();
    assert_eq!(
        (plan.entries, plan.logical_bytes, plan.payload_bytes),
        (1, 2147483665, 16384)
    );
    crate::services::extraction_transaction::validate_sparse_resource_limits(
        &file,
        plan.entries,
        plan.logical_bytes,
        plan.payload_bytes,
    )
    .unwrap();
}

#[test]
fn sparse_plan_rejects_bad_maps_and_preserves_cancellation() {
    let (base, m) = fixture();
    for c in m["faults"].as_array().unwrap() {
        let file = base.join(c["archive"].as_str().unwrap());
        assert!(crate::services::tar_compatibility::plan(
            &file,
            route(&file),
            &DecompressOptions::default(),
            || Ok(())
        )
        .is_err());
    }
    let error = crate::services::tar_compatibility::plan(
        &base.join("control/control.tar"),
        crate::services::compression_format::CompressionRoute::Tar,
        &DecompressOptions::default(),
        || Err(crate::services::compression_service::CompressionError::Cancelled.into()),
    )
    .unwrap_err();
    assert!(matches!(
        error.downcast_ref::<crate::services::compression_service::CompressionError>(),
        Some(crate::services::compression_service::CompressionError::Cancelled)
    ));
}
fn decoder(file: &Path) -> Box<dyn Read + Send> {
    let f = std::fs::File::open(file).unwrap();
    match file.extension().unwrap().to_str().unwrap() {
        "gz" => Box::new(flate2::read::GzDecoder::new(f)),
        "bz2" => Box::new(bzip2::read::BzDecoder::new(f)),
        "xz" => Box::new(xz2::read::XzDecoder::new(f)),
        "zst" => Box::new(zstd::stream::read::Decoder::new(f).unwrap()),
        _ => Box::new(f),
    }
}
fn check_original_layout(layout: &str) {
    let (base, m) = fixture();
    let runtime = Runtime::default();
    for c in m["cases"].as_array().unwrap() {
        if c["layout"] != layout {
            continue;
        }
        let file = base.join(c["archive"].as_str().unwrap());
        let out = tempfile::tempdir().unwrap();
        extract_inner(
            &runtime,
            None,
            "real-tar-original",
            file.to_str().unwrap(),
            out.path(),
            Some(decoder(&file)),
            &DecompressOptions {
                preserve_paths: true,
                ..Default::default()
            },
        )
        .unwrap();
        for expected in c["expected"].as_array().unwrap() {
            let p = out.path().join(expected["path"].as_str().unwrap());
            assert!(p.is_file(), "{} omitted {}", c["archive"], expected["path"]);
            let bytes = std::fs::read(&p).unwrap();
            assert_eq!(
                bytes.len() as u64,
                expected["bytes"].as_u64().unwrap(),
                "{}",
                p.display()
            );
            assert_eq!(
                format!("{:x}", Sha256::digest(&bytes)),
                expected["sha256"].as_str().unwrap(),
                "{}",
                p.display()
            );
        }
        assert_eq!(
            walkdir::WalkDir::new(out.path())
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .count(),
            c["expected"].as_array().unwrap().len()
        );
    }
}
#[test]
fn original_old_gnu_sparse_complete_content() {
    check_original_layout("old-gnu");
}
#[test]
fn original_pax00_sparse_complete_content() {
    check_original_layout("pax00");
}
#[test]
fn original_pax01_sparse_complete_content() {
    check_original_layout("pax01");
}
#[test]
fn original_pax10_sparse_complete_content() {
    check_original_layout("pax10");
}
#[test]
fn existing_pax_ordinary_long_name_is_not_a_new_feature() {
    check_original_layout("pax-ordinary");
}

#[test]
fn selected_sparse_file_does_not_extract_other_files() {
    let (base, m) = fixture();
    for c in m["cases"].as_array().unwrap() {
        let file = base.join(c["archive"].as_str().unwrap());
        let out = tempfile::tempdir().unwrap();
        extract_inner(
            &Runtime::default(),
            None,
            "selected",
            file.to_str().unwrap(),
            out.path(),
            Some(decoder(&file)),
            &DecompressOptions {
                preserve_paths: true,
                selected_entries: vec!["稀疏.bin".into()],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            walkdir::WalkDir::new(out.path())
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .count(),
            1
        );
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(std::fs::read(out.path().join("稀疏.bin")).unwrap())
            ),
            c["expected"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["path"] == "稀疏.bin")
                .unwrap()["sha256"]
                .as_str()
                .unwrap()
        );
    }
}
#[test]
fn non_regular_selection_fails_even_in_tolerance_mode_but_regular_selection_works() {
    let (base, m) = fixture();
    let file = base.join(m["nonRegular"]["archive"].as_str().unwrap());
    for tolerance in [false, true] {
        let out = tempfile::tempdir().unwrap();
        let e = extract_inner(
            &Runtime::default(),
            None,
            "special",
            file.to_str().unwrap(),
            out.path(),
            None,
            &DecompressOptions {
                preserve_paths: true,
                skip_corrupted: tolerance,
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(
            e.to_string().contains("[archive-inspection:unsupported]"),
            "{e:#}"
        );
    }
    let out = tempfile::tempdir().unwrap();
    extract_inner(
        &Runtime::default(),
        None,
        "regular-selected",
        file.to_str().unwrap(),
        out.path(),
        None,
        &DecompressOptions {
            preserve_paths: true,
            selected_entries: vec![m["nonRegular"]["selected"].as_str().unwrap().into()],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        format!(
            "{:x}",
            Sha256::digest(
                std::fs::read(
                    out.path()
                        .join(m["nonRegular"]["selected"].as_str().unwrap())
                )
                .unwrap()
            )
        ),
        m["nonRegular"]["regularSha256"].as_str().unwrap()
    );
}
#[test]
fn derived_mapping_faults_and_compressed_trailer_never_return_success() {
    let (base, m) = fixture();
    for f in m["faults"].as_array().unwrap() {
        let file = base.join(f["archive"].as_str().unwrap());
        let out = tempfile::tempdir().unwrap();
        let e = extract_inner(
            &Runtime::default(),
            None,
            "fault",
            file.to_str().unwrap(),
            out.path(),
            Some(decoder(&file)),
            &DecompressOptions {
                preserve_paths: true,
                ..Default::default()
            },
        )
        .unwrap_err();
        if f["expectedCategory"] == "unsupported" {
            assert!(
                e.to_string().contains("[archive-inspection:unsupported]"),
                "{e:#}"
            );
        } else if !file.to_string_lossy().ends_with(".gz") {
            assert!(
                e.to_string().contains("[archive-inspection:damaged]"),
                "{e:#}"
            );
        }
    }
}
#[test]
fn sparse_hole_expansion_obeys_cancellation_between_real_output_chunks() {
    let (base, _) = fixture();
    let file = base.join("pax10.tar");
    let out = tempfile::tempdir().unwrap();
    let runtime = Runtime {
        cancel_after: Some(65536),
        ..Default::default()
    };
    let e = extract_inner(
        &runtime,
        None,
        "cancel",
        file.to_str().unwrap(),
        out.path(),
        None,
        &DecompressOptions {
            preserve_paths: true,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(matches!(
        e.downcast_ref::<crate::services::compression_service::CompressionError>(),
        Some(crate::services::compression_service::CompressionError::Cancelled)
    ));
    assert!(runtime.written.get() >= 65536);
}
#[test]
fn output_io_errors_are_not_skipped_as_corrupt_entries() {
    let (base, _) = fixture();
    let file = base.join("pax01.tar");
    let out = tempfile::tempdir().unwrap();
    let runtime = Runtime {
        write_error: Some(std::io::ErrorKind::StorageFull),
        ..Default::default()
    };
    let e = extract_inner(
        &runtime,
        None,
        "io",
        file.to_str().unwrap(),
        out.path(),
        None,
        &DecompressOptions {
            preserve_paths: true,
            skip_corrupted: true,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(e
        .chain()
        .filter_map(|c| c.downcast_ref::<std::io::Error>())
        .any(|e| e.kind() == std::io::ErrorKind::StorageFull));
}
#[test]
fn requested_timestamps_are_preserved_for_sparse_and_regular_files() {
    let (base, _) = fixture();
    let file = base.join("pax10.tar");
    let out = tempfile::tempdir().unwrap();
    extract_inner(
        &Runtime::default(),
        None,
        "mtime",
        file.to_str().unwrap(),
        out.path(),
        None,
        &DecompressOptions {
            preserve_paths: true,
            preserve_timestamps: true,
            ..Default::default()
        },
    )
    .unwrap();
    for name in ["稀疏.bin", "末尾.txt"] {
        assert_eq!(
            std::fs::metadata(out.path().join(name))
                .unwrap()
                .modified()
                .unwrap()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            1577934245
        );
    }
}

#[tokio::test]
async fn original_browsing_and_text_preview_share_names_and_logical_sizes() {
    let (base, m) = fixture();
    for c in m["cases"].as_array().unwrap() {
        let file = base.join(c["archive"].as_str().unwrap());
        let result = crate::services::archive_browser::browse_archive(&file, None)
            .await
            .unwrap();
        assert_eq!(result.total_files, 5, "{}", c["archive"]);
        assert_eq!(
            result.total_uncompressed_size,
            c["expected"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| e["bytes"].as_u64().unwrap())
                .sum::<u64>()
        );
        for e in c["expected"].as_array().unwrap() {
            let found = result
                .entries
                .iter()
                .find(|entry| entry.path == e["path"].as_str().unwrap())
                .unwrap_or_else(|| panic!("{} missing {}", c["archive"], e["path"]));
            assert_eq!(found.size, e["bytes"].as_u64().unwrap());
        }
        assert!(result
            .entries
            .iter()
            .all(|e| !e.path.starts_with("./") && !e.path.contains("GNUSparseFile")));
        let preview = crate::services::archive_preview::preview_archive_text(
            &file,
            m["longName"].as_str().unwrap(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(
            preview.content,
            "用户只要这份长路径说明。\nPAX / GNU long name.\n"
        );
    }
}
fn route(file: &Path) -> crate::services::compression_format::CompressionRoute {
    use crate::services::compression_format::CompressionRoute as R;
    match file.extension().unwrap().to_str().unwrap() {
        "gz" => R::TarGzip,
        "bz2" => R::TarBzip2,
        "xz" => R::TarXz,
        _ => R::Tar,
    }
}
#[test]
fn integrity_reads_all_logical_content_and_rejects_derived_bad_maps_and_footer() {
    let (base, m) = fixture();
    for c in m["cases"].as_array().unwrap() {
        let file = base.join(c["archive"].as_str().unwrap());
        assert!(crate::services::compression_verification::verify_native(
            route(&file),
            &file,
            None,
            || false
        )
        .unwrap());
    }
    for c in m["faults"].as_array().unwrap() {
        let file = base.join(c["archive"].as_str().unwrap());
        assert!(
            crate::services::compression_verification::verify_native(
                route(&file),
                &file,
                None,
                || false
            )
            .is_err(),
            "{}",
            file.display()
        );
    }
}

#[tokio::test]
async fn user_integrity_entry_point_uses_native_tar_layout_not_a_7z_fallback() {
    let (base, m) = fixture();
    for c in m["cases"].as_array().unwrap() {
        crate::services::universal_engine::UniversalCliEngine::test_integrity(
            &base.join(c["archive"].as_str().unwrap()),
            None,
        )
        .await
        .unwrap();
    }
    for c in m["faults"].as_array().unwrap() {
        let e = crate::services::universal_engine::UniversalCliEngine::test_integrity(
            &base.join(c["archive"].as_str().unwrap()),
            None,
        )
        .await
        .unwrap_err();
        assert!(
            format!("{e:#}").contains(if c["expectedCategory"] == "unsupported" {
                "[archive-inspection:unsupported]"
            } else {
                "[archive-inspection:damaged]"
            }),
            "{e:#}"
        );
    }
}

#[tokio::test]
async fn diagnosis_retains_actual_tar_mapping_version_or_footer_cause() {
    let (base, m) = fixture();
    for c in m["faults"].as_array().unwrap() {
        let report = crate::services::archive_diagnostics::diagnose_archive(
            &base.join(c["archive"].as_str().unwrap()),
            None,
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        )
        .await
        .unwrap();
        assert_eq!(
            report.status,
            if c["expectedCategory"] == "unsupported" {
                "unsupported_feature"
            } else {
                "damaged"
            }
        );
        assert!(
            report.issues.iter().any(|e| e.detail.contains("TAR")),
            "{report:?}"
        );
    }
}

#[test]
fn bzip2_and_xz_truncated_compressed_tails_are_not_success() {
    let (base, _) = fixture();
    for extension in ["bz2", "xz"] {
        let scratch = tempfile::tempdir().unwrap();
        let file = scratch.path().join(format!("truncated.tar.{extension}"));
        let bytes = std::fs::read(base.join(format!("pax10.tar.{extension}"))).unwrap();
        std::fs::write(&file, &bytes[..bytes.len() - 5]).unwrap();
        let out = tempfile::tempdir().unwrap();
        let result = extract_inner(
            &Runtime::default(),
            None,
            "trailer",
            file.to_str().unwrap(),
            out.path(),
            Some(decoder(&file)),
            &DecompressOptions {
                preserve_paths: true,
                ..Default::default()
            },
        );
        assert!(
            result.is_err(),
            "{extension} extraction must read compressed trailer"
        );
        let checked = crate::services::compression_verification::verify_native(
            route(&file),
            &file,
            None,
            || false,
        );
        assert!(
            checked.is_err(),
            "{extension} integrity must read compressed trailer"
        );
    }
}

#[tokio::test]
async fn preflight_statistics_use_logical_tar_sizes_not_condensed_stream_size() {
    let (base, m) = fixture();
    for c in m["cases"].as_array().unwrap() {
        let (_, bytes) =
            crate::services::universal_engine::UniversalCliEngine::archive_uncompressed_stats(
                &base.join(c["archive"].as_str().unwrap()),
            )
            .await
            .unwrap();
        assert_eq!(
            bytes,
            c["expected"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| e["bytes"].as_u64().unwrap())
                .sum::<u64>(),
            "{}",
            c["archive"]
        );
    }
}
