use super::tar::extract_inner;
use super::ExtractionRuntime;
use crate::models::compression::{DecompressOptions, TaskLogSeverity};
use crate::services::io_buffer_pool::IOBufferPool;
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use tauri::Window;

struct Runtime { pool: IOBufferPool }
impl ExtractionRuntime for Runtime {
    fn check_cancellation(&self) -> Result<()> { Ok(()) }
    fn buffer_pool(&self) -> &IOBufferPool { &self.pool }
    fn copy_buffer_size(&self) -> usize { 64 * 1024 }
    fn normalized_archive_path(&self, path: &Path, preserve: bool) -> Option<PathBuf> {
        let path = if preserve { path } else { Path::new(path.file_name()?) };
        let clean: PathBuf = path.components().filter_map(|c| if let Component::Normal(p) = c { Some(p) } else { None }).collect();
        (!clean.as_os_str().is_empty()).then_some(clean)
    }
    fn emit_log(&self, _: &Window, _: &str, _: &str, _: TaskLogSeverity) {}
    fn emit_progress(&self, _: &Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {}
}
fn fixture() -> (PathBuf, serde_json::Value) {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/tar-1513");
    let m = serde_json::from_slice(&std::fs::read(p.join("manifest.json")).unwrap()).unwrap();
    (p,m)
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
    let (base,m) = fixture();
    let runtime = Runtime { pool: IOBufferPool::default() };
    for c in m["cases"].as_array().unwrap() {
        if c["layout"] != layout { continue; }
        let file = base.join(c["archive"].as_str().unwrap());
        let out = tempfile::tempdir().unwrap();
        extract_inner(&runtime,None,"real-tar-original",file.to_str().unwrap(),out.path(),Some(decoder(&file)),&DecompressOptions { preserve_paths:true, ..Default::default() }).unwrap();
        for expected in c["expected"].as_array().unwrap() {
            let p = out.path().join(expected["path"].as_str().unwrap());
            assert!(p.is_file(), "{} omitted {}", c["archive"], expected["path"]);
            let bytes = std::fs::read(&p).unwrap();
            assert_eq!(bytes.len() as u64, expected["bytes"].as_u64().unwrap(), "{}",p.display());
            assert_eq!(format!("{:x}",Sha256::digest(&bytes)),expected["sha256"].as_str().unwrap(),"{}",p.display());
        }
    }
}
#[test]
fn original_old_gnu_sparse_complete_content() { check_original_layout("old-gnu"); }
#[test]
fn original_pax00_sparse_complete_content() { check_original_layout("pax00"); }
#[test]
fn original_pax01_sparse_complete_content() { check_original_layout("pax01"); }
#[test]
fn original_pax10_sparse_complete_content() { check_original_layout("pax10"); }
#[test]
fn existing_pax_ordinary_long_name_is_not_a_new_feature() { check_original_layout("pax-ordinary"); }
