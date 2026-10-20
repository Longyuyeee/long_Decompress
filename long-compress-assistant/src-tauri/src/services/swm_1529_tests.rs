use super::{archive_browser, archive_diagnostics, archive_preview, split_archive_detector::{SplitArchiveDetector, SplitFormat}, universal_engine::UniversalCliEngine};
use std::{collections::BTreeMap, fs, path::{Path, PathBuf}, sync::{Arc, atomic::AtomicBool}};
use sha2::{Digest, Sha256};
fn flag() -> Arc<AtomicBool> { Arc::new(AtomicBool::new(false)) }
fn hash(bytes: &[u8]) -> String { hex::encode(Sha256::digest(bytes)) }
fn root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/native-swm-1529") }
fn case(method: &str) -> serde_json::Value {
    let manifest: serde_json::Value = serde_json::from_slice(&fs::read(root().join("manifest.json")).unwrap()).unwrap();
    manifest["cases"].as_array().unwrap().iter().find(|c| c["method"] == method).unwrap().clone()
}
fn expected(c: &serde_json::Value) -> BTreeMap<String,String> { serde_json::from_value(c["expected"].clone()).unwrap() }
fn tree(path: &Path) -> BTreeMap<String,String> {
    walkdir::WalkDir::new(path).into_iter().map(Result::unwrap).filter(|p| p.file_type().is_file())
        .map(|p|(p.path().strip_prefix(path).unwrap().to_string_lossy().replace('\\',"/"),hash(&fs::read(p.path()).unwrap()))).collect()
}
fn parts(directory: &Path, method: &str, stem: &str) -> Vec<PathBuf> {
    fs::create_dir_all(directory).unwrap();
    case(method)["members"].as_array().unwrap().iter().enumerate().map(|(index,part)| {
        let bytes=fs::read(root().join(part["file"].as_str().unwrap())).unwrap(); assert_eq!(hash(&bytes),part["sha256"]);
        let name=if index==0 {format!("{stem}.swm")} else {format!("{stem}{}.swm",index+1)};
        let path=directory.join(name);fs::write(&path,bytes).unwrap();path
    }).collect()
}
async fn positive(method: &str) {
    let temp=tempfile::tempdir().unwrap();let c=case(method);let files=parts(&temp.path().join("parts"),method,"镜像2026");
    for index in [0,files.len()/2,files.len()-1] {
        let group=SplitArchiveDetector::detect_split_archive(&files[index]).unwrap().unwrap();
        assert_eq!(group.format,SplitFormat::WimSplit);assert!(group.is_complete);assert_eq!(group.parts,files);assert_eq!(group.first_part,files[0]);
        assert_eq!(group.total_size,files.iter().map(|p|p.metadata().unwrap().len()).sum::<u64>());
        let result=archive_browser::browse_archive(&files[index],None).await.unwrap();assert_eq!(result.format,"WIM");
        assert_eq!(result.entries.iter().filter(|e|!e.is_dir).count(),expected(&c).len());
        assert!(result.entries.iter().filter(|e|!e.is_dir).all(|e|expected(&c).contains_key(&e.path)));
    }
    let selected=files.last().unwrap();UniversalCliEngine::test_integrity(selected,None).await.unwrap();
    let report=archive_diagnostics::diagnose_archive(selected,None,flag()).await.unwrap();assert_eq!(report.status,"healthy");assert!(report.integrity_tested&&report.split_archive);
    let prefix=if c["images"]==2 {"2/"} else {""};
    let text=archive_preview::preview_archive_text(selected,&format!("{prefix}说明.txt"),None).await.unwrap();assert!(text.content.contains("原生 SWM 独立镜像"));
    let image=archive_preview::preview_archive_image(selected,&format!("{prefix}图标.png"),None).await.unwrap();assert_eq!((image.width,image.height),(32,32));
    let entry=format!("{prefix}项目/数据17.bin");let output=temp.path().join("selected");
    UniversalCliEngine::new().extract_wim_with_progress(selected,&output,false,Arc::new(|_|{}),Arc::new(|_,_|{}),flag(),&[entry.clone()]).await.unwrap();
    assert_eq!(tree(&output),expected(&c).into_iter().filter(|(p,_)|p==&entry).collect());
    let full=temp.path().join("full");
    UniversalCliEngine::new().extract_wim_with_progress(&files[files.len()/2],&full,false,Arc::new(|_|{}),Arc::new(|_,_|{}),flag(),&[]).await.unwrap();
    assert_eq!(tree(&full),expected(&c));for dir in c["directories"].as_array().unwrap(){assert!(full.join(dir.as_str().unwrap()).is_dir());}
    assert_eq!(fs::read_dir(temp.path().join("parts")).unwrap().count(),files.len(),"No joined WIM is created");
}
#[tokio::test] async fn swm_1529_copy_actual_parts_all_flows() {positive("none").await;}
#[tokio::test] async fn swm_1529_xpress_compressed_parts_all_flows() {positive("xpress").await;}
#[tokio::test] async fn swm_1529_lzx_multi_image_parts_all_flows() {positive("lzx").await;}

#[tokio::test]
async fn swm_1529_missing_first_middle_tail_and_restore() {
    let temp=tempfile::tempdir().unwrap();let files=parts(temp.path(),"lzx","install");
    for index in [0,files.len()/2,files.len()-1] {
        let bytes=fs::read(&files[index]).unwrap();fs::remove_file(&files[index]).unwrap();
        let selected=if index==files.len()-1 {&files[1]} else {files.last().unwrap()};
        let group=SplitArchiveDetector::detect_split_archive(selected).unwrap().unwrap();assert!(!group.is_complete);assert_eq!(group.missing_parts,vec![files[index].clone()]);
        let error=archive_browser::browse_archive(selected,None).await.unwrap_err().to_string();assert!(error.contains("incomplete")&&error.contains(files[index].file_name().unwrap().to_str().unwrap()),"{error}");
        assert!(UniversalCliEngine::test_integrity(selected,None).await.is_err());fs::write(&files[index],bytes).unwrap();
    }
    assert!(archive_browser::browse_archive(files.last().unwrap(),None).await.is_ok());
}

#[tokio::test]
async fn swm_1529_mixed_guid_wrong_number_truncated_and_corrupt_data() {
    let temp=tempfile::tempdir().unwrap();let files=parts(temp.path(),"xpress","install");let original=fs::read(&files[1]).unwrap();
    for (index,value,cause) in [(24,original[24]^1,"不同镜像"),(40,1,"内部卷号")] {
        let mut bytes=original.clone();bytes[index]=value;fs::write(&files[1],bytes).unwrap();
        let error=SplitArchiveDetector::complete_source(&files[0]).unwrap_err().to_string();assert!(error.contains(cause),"{error}");fs::write(&files[1],&original).unwrap();
    }
    fs::write(&files[1],&original[..original.len()/2]).unwrap();let error=SplitArchiveDetector::complete_source(&files[0]).unwrap_err().to_string();assert!(error.contains("截断"),"{error}");
    let mut bytes=original.clone();bytes[1024]^=0x55;fs::write(&files[1],bytes).unwrap();assert!(UniversalCliEngine::test_integrity(&files[0],None).await.is_err());
}

#[test]
fn swm_1529_case_insensitive_names_and_renamed_member_boundary() {
    let temp=tempfile::tempdir().unwrap();let files=parts(temp.path(),"none","Install");let original=&files[2];let renamed=temp.path().join("INSTALL3.SWM");fs::rename(original,&renamed).unwrap();
    let group=SplitArchiveDetector::detect_split_archive(&renamed).unwrap().unwrap();assert!(group.is_complete);assert_eq!(group.first_part,files[0]);
    let wrong=temp.path().join("download.swm");fs::rename(&renamed,&wrong).unwrap();let error=SplitArchiveDetector::complete_source(&wrong).unwrap_err().to_string();assert!(error.contains("原始名称"),"{error}");
}
