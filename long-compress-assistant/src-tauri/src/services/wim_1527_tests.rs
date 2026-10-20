use super::{archive_browser, archive_diagnostics, archive_preview, compression_format, compression_service::CompressionService, split_archive_detector::SplitArchiveDetector, universal_engine::UniversalCliEngine, wim_preview};
use std::{collections::BTreeMap, fs, path::{Path, PathBuf}, sync::{Arc, atomic::AtomicBool}};
use sha2::{Digest, Sha256};

fn flag() -> Arc<AtomicBool> { Arc::new(AtomicBool::new(false)) }
fn hash(bytes: &[u8]) -> String { hex::encode(Sha256::digest(bytes)) }
fn root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/wim-volumes-1527") }
fn manifest() -> serde_json::Value { serde_json::from_slice(&fs::read(root().join("manifest.json")).unwrap()).unwrap() }
fn case(name: &str) -> serde_json::Value { manifest()["cases"].as_array().unwrap().iter().find(|c| c["method"] == name).unwrap().clone() }
fn expected(c: &serde_json::Value) -> BTreeMap<String,String> { serde_json::from_value(c["expected"].clone()).unwrap() }
fn tree(path: &Path) -> BTreeMap<String,String> {
    walkdir::WalkDir::new(path).into_iter().map(Result::unwrap).filter(|p| p.file_type().is_file())
        .map(|p|(p.path().strip_prefix(path).unwrap().to_string_lossy().replace('\\',"/"),hash(&fs::read(p.path()).unwrap()))).collect()
}
fn parts(directory: &Path, name: &str) -> Vec<PathBuf> {
    fs::create_dir_all(directory).unwrap();
    let c=case(name);let bytes=fs::read(root().join(c["file"].as_str().unwrap())).unwrap();assert_eq!(hash(&bytes),c["sha256"]);
    let mut files=Vec::new();let mut offset=0;
    while offset<bytes.len() { let size=if files.is_empty(){7}else if files.len()==1{13}else{4093};let end=(offset+size).min(bytes.len());let path=directory.join(format!("镜像.wim.{:03}",files.len()+1));fs::write(&path,&bytes[offset..end]).unwrap();files.push(path);offset=end; }
    files
}

#[tokio::test]
async fn wim_1527_four_independent_methods_and_images_across_numeric_members() {
    for method in ["none","xpress","lzx","lzms"] {
        let temp=tempfile::tempdir().unwrap();let files=parts(temp.path(),method);let c=case(method);
        for index in [0,files.len()/2,files.len()-1] {
            let result=archive_browser::browse_archive(&files[index],None).await.unwrap();
            assert_eq!(result.format,"WIM");let paths:Vec<_>=result.entries.iter().filter(|e|!e.is_dir).map(|e|e.path.clone()).collect();
            assert_eq!(paths.len(),expected(&c).len());assert!(paths.iter().all(|p|expected(&c).contains_key(p)),"{paths:?}");
        }
        UniversalCliEngine::test_integrity(&files[files.len()/2],None).await.unwrap();
        assert_eq!(SplitArchiveDetector::complete_source(files.last().unwrap()).unwrap(),files[0]);
        assert!(fs::read_dir(temp.path()).unwrap().all(|p|p.unwrap().path().extension().unwrap().to_str().unwrap().chars().all(|c|c.is_ascii_digit())));
    }
}

#[tokio::test]
async fn wim_1527_selected_image_and_directory_are_extracted_without_other_images() {
    let temp=tempfile::tempdir().unwrap();let files=parts(&temp.path().join("parts"),"lzx");
    for (selection,label) in [(vec!["2/说明.txt".into()],"file"),(vec!["1/项目".into()],"directory")] {
        let output=temp.path().join(label);
        UniversalCliEngine::new().extract_wim_with_progress(&files[0],&output,false,Arc::new(|_|{}),Arc::new(|_,_|{}),flag(),&selection).await.unwrap();
        let wanted=expected(&case("lzx")).into_iter().filter(|(name,_)|name==&selection[0]||name.starts_with(&(selection[0].clone()+"/"))).collect();
        assert_eq!(tree(&output),wanted);
    }
}

#[tokio::test]
async fn wim_1527_missing_and_truncated_members_keep_actionable_causes() {
    let temp=tempfile::tempdir().unwrap();let files=parts(temp.path(),"xpress");let bytes=fs::read(&files[1]).unwrap();fs::remove_file(&files[1]).unwrap();
    let error=archive_browser::browse_archive(files.last().unwrap(),None).await.unwrap_err().to_string();assert!(error.contains("incomplete"),"{error}");
    fs::write(&files[1],bytes).unwrap();let last=files.last().unwrap();let mut bytes=fs::read(last).unwrap();bytes.truncate(bytes.len()/2);fs::write(last,bytes).unwrap();
    assert!(UniversalCliEngine::test_integrity(&files[0],None).await.is_err());
}

#[tokio::test]
async fn wim_1527_preview_bounds_cancel_and_password_are_explicit() {
    let temp=tempfile::tempdir().unwrap();let files=parts(temp.path(),"xpress");
    let error=wim_preview::preview_entry(&files[0],"说明.txt",None,1,1,flag()).await.unwrap_err().to_string();assert!(error.contains("预览范围"),"{error}");
    assert!(wim_preview::preview_entry(&files[0],"说明.txt",Some("password"),1024,1024,flag()).await.unwrap_err().to_string().contains("不支持原生密码"));
    let error=wim_preview::preview_entry(&files[0],"说明.txt",None,1024,1024,Arc::new(AtomicBool::new(true))).await.unwrap_err().to_string();assert!(error.contains("取消"));
}

#[tokio::test]
async fn wim_1527_public_diagnostics_stats_and_creation_capabilities() {
    let temp=tempfile::tempdir().unwrap();let files=parts(temp.path(),"lzx");
    let report=archive_diagnostics::diagnose_archive(&files[files.len()/2],None,flag()).await.unwrap();assert_eq!(report.actual_format,"WIM");assert_eq!(report.status,"healthy");assert!(report.integrity_tested&&report.split_archive);
    assert_eq!(UniversalCliEngine::archive_uncompressed_stats(&files[0]).await.unwrap().0,expected(&case("lzx")).len()+6);
    let input=temp.path().join("file.txt");fs::write(&input,b"source").unwrap();let options=crate::models::compression::CompressionOptions {format:Some("wim".into()),split_size:Some(65537),..Default::default()};
    let source=vec![input.to_string_lossy().into_owned()];let output=temp.path().join("new.wim");assert_eq!(compression_format::validate_compression_request(&source,&output.to_string_lossy(),&options).unwrap(),"wim");
    let encrypted=crate::models::compression::CompressionOptions {password:Some("password".into()),..options};assert!(CompressionService::validate_compression_request(&source,&output.to_string_lossy(),&encrypted).is_err());
}

async fn preview(method: &str) {
    let temp=tempfile::tempdir().unwrap();let files=parts(temp.path(),method);let prefix=if method=="lzx"{"1/"}else{""};
    let text=archive_preview::preview_archive_text(files.last().unwrap(),&format!("{prefix}说明.txt"),None).await.unwrap();assert!(text.content.contains("独立 WIM 镜像 1"));assert!(!text.truncated);
    let image=archive_preview::preview_archive_image(&files[files.len()/2],&format!("{prefix}图标.png"),None).await.unwrap();assert_eq!((image.width,image.height),(32,32));
    use base64::Engine;
    assert_eq!(base64::engine::general_purpose::STANDARD.decode(image.data_url.split(',').nth(1).unwrap()).unwrap(),fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("icons/32x32.png")).unwrap());
}
#[tokio::test] async fn wim_1527_preview_copy() {preview("none").await;}
#[tokio::test] async fn wim_1527_preview_xpress() {preview("xpress").await;}
#[tokio::test] async fn wim_1527_preview_lzx_multi_image() {preview("lzx").await;}
#[tokio::test] async fn wim_1527_preview_lzms_solid() {preview("lzms").await;}
