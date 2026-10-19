use super::{archive_browser, archive_preview, compression_format::CompressionRoute, compression_verification, tar_compatibility};
use crate::models::compression::{DecompressOptions, TaskLogSeverity};
use super::native_extraction::{self, ExtractionRuntime};
use super::io_buffer_pool::IOBufferPool;
use std::io::Read;
use std::path::{Path, PathBuf, Component};
use sha2::{Digest, Sha256};
use tauri::Window;

#[derive(Default)]
struct Runtime { pool: IOBufferPool }
impl ExtractionRuntime for Runtime {
    fn check_cancellation(&self) -> anyhow::Result<()> { Ok(()) }
    fn buffer_pool(&self) -> &IOBufferPool { &self.pool }
    fn copy_buffer_size(&self) -> usize { 65536 }
    fn normalized_archive_path(&self, p: &Path, _: bool) -> Option<PathBuf> {
        Some(p.components().filter_map(|c|if let Component::Normal(s)=c {Some(s)}else{None}).collect())
    }
    fn emit_log(&self, _: &Window, _: &str, _: &str, _: TaskLogSeverity) {}
    fn emit_progress(&self, _: &Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {}
}
fn fixture() -> (PathBuf, serde_json::Value) {
    let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/multistream-1517");
    let m=serde_json::from_slice(&std::fs::read(p.join("manifest.json")).unwrap()).unwrap(); (p,m)
}
fn route(codec: &str, tar: bool) -> CompressionRoute {
    match (codec,tar) {
        ("gz",true)=>CompressionRoute::TarGzip, ("bz2",true)=>CompressionRoute::TarBzip2,
        ("xz",true)=>CompressionRoute::TarXz, ("zst",true)=>CompressionRoute::TarZstd,
        ("gz",false)=>CompressionRoute::Gzip, ("bz2",false)=>CompressionRoute::Bzip2,
        ("xz",false)=>CompressionRoute::Xz, _=>CompressionRoute::Zstd,
    }
}
async fn tar_flow(codec: &str) {
    let (base,m)=fixture(); let p=base.join(format!("跨段.tar.{codec}"));
    let expected=m["expected"].as_array().unwrap();
    let browse=archive_browser::browse_archive(&p,None).await.unwrap();
    assert_eq!(browse.total_files,3,"{codec}: all members must contribute entries");
    let preview=archive_preview::preview_archive_text(&p,"末尾.txt",None).await.unwrap();
    assert!(preview.content.ends_with("内容不能丢。\n"));
    let plan=tar_compatibility::plan(&p,route(codec,true),&DecompressOptions::default(),||Ok(())).unwrap();
    assert_eq!(plan.logical_bytes,expected.iter().map(|v|v["bytes"].as_u64().unwrap()).sum::<u64>());
    assert!(compression_verification::verify_native(route(codec,true),&p,None,||false).unwrap());
    let out=tempfile::tempdir().unwrap();
    native_extraction::tar::extract_inner(&Runtime::default(),None,"test",p.to_str().unwrap(),out.path(),
        Some(super::codec_stream::decoder(&p,codec).unwrap()),&DecompressOptions::default()).unwrap();
    for file in expected {
        let bytes=std::fs::read(out.path().join(file["path"].as_str().unwrap())).unwrap();
        assert_eq!(hex::encode(Sha256::digest(&bytes)),file["sha256"].as_str().unwrap());
    }
    assert!(out.path().join("空目录").is_dir());
}
#[tokio::test] async fn stream1517_gzip_tar_reads_tail_across_members() { tar_flow("gz").await; }
#[tokio::test] async fn stream1517_bzip2_tar_reads_tail_across_members() { tar_flow("bz2").await; }
#[tokio::test] async fn stream1517_xz_tar_reads_tail_across_members_and_padding() { tar_flow("xz").await; }
#[tokio::test] async fn stream1517_zstd_tar_keeps_existing_multiframe_support() { tar_flow("zst").await; }

#[test]
fn stream1517_later_member_damage_is_not_success() {
    let (base,m)=fixture();
    for c in m["cases"].as_array().unwrap() {
        for fault in ["truncated","corrupt"] {
            let p=base.join(c[fault].as_str().unwrap());
            assert!(compression_verification::verify_native(route(c["codec"].as_str().unwrap(),c["kind"]=="tar"),&p,None,||false).is_err(),"{}: {fault}",p.display());
        }
    }
}
#[tokio::test]
async fn stream1517_plain_stream_has_complete_size_and_preview() {
    let (base,m)=fixture();
    for c in m["cases"].as_array().unwrap().iter().filter(|c|c["kind"]=="plain") {
        let p=base.join(c["archive"].as_str().unwrap());
        let browse=archive_browser::browse_archive(&p,None).await.unwrap();
        assert_eq!(browse.total_files,1); assert_eq!(browse.total_uncompressed_size,c["decodedBytes"].as_u64().unwrap());
        let preview=archive_preview::preview_archive_text(&p,"说明.txt",None).await.unwrap();
        assert_eq!(preview.content.as_bytes(),std::fs::read(base.join("说明.txt")).unwrap());
    }
}
#[test]
fn stream1517_separate_tar_concatenation_is_explicit_not_partial_success() {
    let (base,_)=fixture();
    for codec in ["gz","bz2","xz","zst"] {
        let p=base.join(format!("两个完整TAR.tar.{codec}"));
        let e=compression_verification::verify_native(route(codec,true),&p,None,||false).unwrap_err();
        assert!(e.to_string().contains("[archive-inspection:unsupported]"),"{codec}: {e:#}");
    }
}

#[test]
fn stream1517_complete_decoding_keeps_raw_payload_hash() {
    let (base,m)=fixture();
    for c in m["cases"].as_array().unwrap() {
        let mut bytes=Vec::new();
        super::codec_stream::decoder(&base.join(c["archive"].as_str().unwrap()),c["codec"].as_str().unwrap()).unwrap().read_to_end(&mut bytes).unwrap();
        assert_eq!(hex::encode(Sha256::digest(&bytes)),c["decodedSha256"].as_str().unwrap());
    }
}
