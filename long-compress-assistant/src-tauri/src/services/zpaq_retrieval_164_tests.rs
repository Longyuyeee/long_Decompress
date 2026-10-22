use super::*;
use sha2::{Digest,Sha256};
use crate::models::compression::DecompressOptions;
fn root()->PathBuf{PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")}
fn flag()->Arc<AtomicBool>{Arc::new(AtomicBool::new(false))}
fn hash(p:&Path)->String{hex::encode(Sha256::digest(std::fs::read(p).unwrap()))}
fn manifest()->serde_json::Value{serde_json::from_slice(&std::fs::read(root().join("tests/fixtures/archives/zpaq-damaged-163/manifest.json")).unwrap()).unwrap()}
fn source(temp:&Path,c:&serde_json::Value)->(PathBuf,Vec<PathBuf>){
 let base=root().join("tests/fixtures/archives/zpaq-damaged-163");
 let parts=c["parts"].as_array().unwrap().iter().map(|p|{let src=base.join(p["file"].as_str().unwrap());assert_eq!(hash(&src),p["sha256"]);let dest=temp.join(src.file_name().unwrap());std::fs::copy(src,&dest).unwrap();dest}).collect::<Vec<_>>();
 let path=if c["native"]==true{super::super::super::zpaq_multipart::confirm(&temp.join("profile"),super::super::super::zpaq_multipart::suggest(&parts[0]).unwrap().unwrap()).unwrap()}else{parts[0].clone()};(path,parts)
}
#[tokio::test]
async fn zpaq164_four_modes_scoped_password_extract_and_transactional_receipt(){
 let m=manifest();
 for c in m["cases"].as_array().unwrap(){
  let t=tempfile::tempdir().unwrap();let(path,parts)=source(t.path(),c);let before=parts.iter().map(|p|hash(p)).collect::<Vec<_>>();let key=(c["encrypted"]==true).then(||m["password"].as_str().unwrap());
  let report=blocks::diagnose(&path,key,None,None,flag()).await.unwrap();
  let selection=Selection{snapshot:report.snapshot,revision:report.revision,paths:report.entries.iter().filter(|e|e.state==blocks::FileState::Verified).take(2).map(|e|e.path.clone()).collect()};assert_eq!(selection.paths.len(),2);
  let options=DecompressOptions{zpaq_retrieval:Some(selection.clone()),zpaq_snapshot:Some(selection.snapshot),selected_entries:selection.paths.clone(),preserve_paths:true,create_subdirectory:false,..Default::default()};
  let service=super::super::super::compression_service::CompressionService::new_with_defaults().await;
  if let Some(key)=key {assert!(service.test_extraction_password(path.to_str().unwrap(),key,&options).await.unwrap());assert!(!service.test_extraction_password(path.to_str().unwrap(),"wrong",&options).await.unwrap());assert!(super::super::try_password(&path,key,flag()).await.is_err());}
  let final_dir=result_directory(t.path());let staging=super::super::super::extraction_transaction::ExtractionStaging::create_for(&final_dir).unwrap();
  let receipt=extract(&path,staging.path(),key,&selection,flag()).await.unwrap();assert!(!final_dir.exists());
  super::super::super::extraction_transaction::prepare_staging_layout(&path,staging.path(),&options).unwrap();write_receipt(staging.path(),&selection,&receipt).unwrap();
  super::super::super::extraction_transaction::commit_staged_extraction(path.to_str().unwrap(),staging.path(),&final_dir,&options,|_|{}).unwrap();
  let receipt=read_receipt(&final_dir,&selection).unwrap();assert_eq!(receipt.entries.iter().filter(|e|e.state=="retrieved").count(),2);assert_eq!(receipt.entries.iter().filter(|e|e.state=="affected").count(),2);assert_eq!(receipt.entries.iter().filter(|e|e.state=="not_selected").count(),1);
  for name in &selection.paths {assert_eq!(hash(&final_dir.join(name)),c["snapshots"][2]["expected"][name].as_str().unwrap());}
  let actual=walkdir::WalkDir::new(&final_dir).into_iter().map(Result::unwrap).filter(|e|e.file_type().is_file()).count();assert_eq!(actual,3);assert_eq!(before,parts.iter().map(|p|hash(p)).collect::<Vec<_>>());
 }
}
#[tokio::test]
async fn zpaq164_stale_bad_selection_cancel_and_missing_member_publish_nothing(){
 let m=manifest();let t=tempfile::tempdir().unwrap();let(path,parts)=source(t.path(),&m["cases"][3]);let key=m["password"].as_str().unwrap();let report=blocks::diagnose(&path,Some(key),None,None,flag()).await.unwrap();
 let selection=Selection{snapshot:report.snapshot,revision:report.revision,paths:vec![report.entries.iter().find(|e|e.state==blocks::FileState::Verified).unwrap().path.clone()]};
 let bad=Selection{paths:vec![report.entries.iter().find(|e|e.state==blocks::FileState::Affected).unwrap().path.clone()],..selection.clone()};let output=t.path().join("never-published");assert!(extract(&path,&output,Some(key),&bad,flag()).await.is_err());assert!(!output.exists());
 let stale=Selection{revision:"0".repeat(64),..selection.clone()};assert!(extract(&path,&output,Some(key),&stale,flag()).await.is_err());assert!(!output.exists());
 let stop=flag();let cancel=stop.clone();let timer=tokio::spawn(async move{tokio::time::sleep(Duration::from_millis(20)).await;cancel.store(true,std::sync::atomic::Ordering::Relaxed);});assert!(extract(&path,&output,Some(key),&selection,stop).await.is_err());timer.await.unwrap();assert!(!output.exists());
 std::fs::rename(&parts[1],t.path().join("missing-part")).unwrap();assert!(extract(&path,&output,Some(key),&selection,flag()).await.is_err());assert!(!output.exists());
}
