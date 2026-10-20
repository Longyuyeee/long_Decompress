use super::{zpaq_engine, archive_preview, archive_browser};
use crate::models::compression::ZpaqSnapshot;
use std::{path::{Path,PathBuf},fs,collections::BTreeMap,sync::{Arc,atomic::AtomicBool}};
use sha2::{Digest,Sha256};
fn flag()->Arc<AtomicBool>{Arc::new(AtomicBool::new(false))}
fn hash(b:&[u8])->String{hex::encode(Sha256::digest(b))}
fn root()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/zpaq-snapshots-1530")}
fn manifest()->serde_json::Value{serde_json::from_slice(&fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn tree(p:&Path)->BTreeMap<String,String>{walkdir::WalkDir::new(p).into_iter().map(Result::unwrap).filter(|e|e.file_type().is_file()).map(|e|(e.path().strip_prefix(p).unwrap().to_string_lossy().replace('\\',"/"),hash(&fs::read(e.path()).unwrap()))).collect()}
fn volumes(dir:&Path,c:&serde_json::Value)->Vec<PathBuf>{let b=fs::read(root().join(c["file"].as_str().unwrap())).unwrap();assert_eq!(hash(&b),c["sha256"]);let mut parts=vec![];let mut offset=0;while offset<b.len(){let end=(offset+if parts.is_empty(){7}else if parts.len()==1{13}else{4093}).min(b.len());let p=dir.join(format!("历史.zpaq.{:03}",parts.len()+1));fs::write(&p,&b[offset..end]).unwrap();parts.push(p);offset=end;}parts}
async fn snapshots(encrypted:bool){
 let m=manifest();let c=&m["cases"][usize::from(encrypted)];let t=tempfile::tempdir().unwrap();let parts=volumes(t.path(),c);let password=encrypted.then(||m["password"].as_str().unwrap());
 for row in c["snapshots"].as_array().unwrap(){
  let snapshot:ZpaqSnapshot=serde_json::from_value(serde_json::json!({"version":row["version"],"date":row["date"]})).unwrap();let expected:BTreeMap<String,String>=serde_json::from_value(row["expected"].clone()).unwrap();
  for member in [&parts[0],&parts[parts.len()/2],parts.last().unwrap()] {let b=archive_browser::browse_archive_cancellable_at(member,password,flag(),None,Some(snapshot)).await.unwrap();let h=b.zpaq_history.unwrap();assert_eq!(h.selected,snapshot);assert_eq!(h.snapshots.len(),3);assert_eq!(b.total_files,expected.len());assert_eq!(b.entries.iter().filter(|e|!e.is_dir).map(|e|e.path.clone()).collect::<std::collections::BTreeSet<_>>(),expected.keys().cloned().collect());}
  let output=t.path().join(format!("snapshot{}",snapshot.version));zpaq_engine::extract_at(parts.last().unwrap(),&output,password,&[],Some(&snapshot),flag()).await.unwrap();assert_eq!(tree(&output),expected);for d in row["directories"].as_array().unwrap(){assert!(output.join(d.as_str().unwrap()).is_dir());}
  zpaq_engine::verify_at(parts.last().unwrap(),password,Some(&snapshot),flag()).await.unwrap();
 }
}
#[tokio::test]async fn zpaq_1530_plain_three_snapshots_and_numeric_members(){snapshots(false).await;}
#[tokio::test]async fn zpaq_1530_password_three_snapshots_and_numeric_members(){snapshots(true).await;}
#[tokio::test]async fn zpaq_1530_historical_text_image_and_exact_selection(){
 let m=manifest();let c=&m["cases"][1];let t=tempfile::tempdir().unwrap();let parts=volumes(t.path(),c);let p=parts.last().unwrap();let password=Some(m["password"].as_str().unwrap());let snapshot=ZpaqSnapshot{version:1,date:20261020130000};
 let text=archive_preview::preview_archive_text_at(p,"资料/说明.txt",password,None,Some(snapshot)).await.unwrap();assert!(text.content.contains("历史快照 1"));let image=archive_preview::preview_archive_image_at(p,"资料/图标.png",password,None,Some(snapshot)).await.unwrap();assert_eq!((image.width,image.height),(32,32));
 let output=t.path().join("selected");zpaq_engine::extract_at(p,&output,password,&["资料/后来删除.txt".into()],Some(&snapshot),flag()).await.unwrap();assert_eq!(tree(&output),BTreeMap::from([("资料/后来删除.txt".into(),c["snapshots"][0]["expected"]["资料/后来删除.txt"].as_str().unwrap().into())]));
 let fresh=archive_preview::preview_archive_image(p,"资料/图标.png",password).await.unwrap();assert_eq!((fresh.width,fresh.height),(128,128));
}
#[tokio::test]async fn zpaq_1530_missing_identity_streaming_password_and_cancel_never_fall_back(){
 let m=manifest();let file=root().join("journal-key.zpaq");let password=Some(m["password"].as_str().unwrap());
 for snapshot in [ZpaqSnapshot{version:9,date:20261020130000},ZpaqSnapshot{version:1,date:20261020130001}] {let t=tempfile::tempdir().unwrap();let error=zpaq_engine::extract_at(&file,t.path(),password,&[],Some(&snapshot),flag()).await.unwrap_err().to_string();assert!(error.contains("snapshot-unavailable"),"{error}");assert_eq!(fs::read_dir(t.path()).unwrap().count(),0);}
 let snapshot=ZpaqSnapshot{version:1,date:20261020130000};let stream=Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/zpaq/stream.zpaq");assert!(zpaq_engine::browse(&stream,None,flag()).await.unwrap().zpaq_history.is_none());let error=zpaq_engine::browse_at(&stream,None,Some(&snapshot),flag()).await.unwrap_err().to_string();assert!(error.contains("snapshot-unavailable"),"{error}");
 assert!(zpaq_engine::browse_at(&file,Some("wrong"),Some(&snapshot),flag()).await.is_err());assert!(zpaq_engine::browse_at(&file,password,Some(&snapshot),Arc::new(AtomicBool::new(true))).await.unwrap_err().to_string().contains("取消"));
}
#[tokio::test]async fn zpaq_1530_contract_diagnosis_and_resource_plan_use_the_chosen_snapshot(){
 let file=root().join("journal.zpaq");let snapshot=ZpaqSnapshot{version:1,date:20261020130000};
 let report=super::archive_diagnostics::diagnose_archive_at(&file,None,None,flag(),Some(snapshot)).await.unwrap();assert_eq!(report.status,"healthy");assert!(report.integrity_tested);assert!(report.evidence.iter().any(|e|e.contains("snapshot 1")));
 let invalid=ZpaqSnapshot{version:8,..snapshot};let report=super::archive_diagnostics::diagnose_archive_at(&file,None,None,flag(),Some(invalid)).await.unwrap();assert_eq!(report.status,"snapshot_unavailable");assert!(!report.integrity_tested);
 let output=tempfile::tempdir().unwrap();let report=super::storage_preflight::preflight_operation_resources_at("decompression",&output.path().to_string_lossy(),&[file.to_string_lossy().into_owned()],None,None,false,Some(snapshot)).await.unwrap();let wanted=fs::metadata(Path::new(env!("CARGO_MANIFEST_DIR")).join("icons/32x32.png")).unwrap().len();assert!(report.estimated_output_bytes.unwrap()>wanted);assert_eq!(report.estimate_source,"archive_metadata");
}
