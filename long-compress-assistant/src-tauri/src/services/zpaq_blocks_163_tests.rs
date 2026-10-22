use super::*;
use std::fs;
fn root()->PathBuf{PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")}
fn flag()->Arc<AtomicBool>{Arc::new(AtomicBool::new(false))}
fn manifest()->serde_json::Value{serde_json::from_slice(&fs::read(root().join("tests/fixtures/archives/zpaq-damaged-163/manifest.json")).unwrap()).unwrap()}
fn sha(p:&Path)->String{hex::encode(Sha256::digest(fs::read(p).unwrap()))}
fn copy(t:&Path,c:&serde_json::Value)->(PathBuf,Vec<PathBuf>){
    let base=root().join("tests/fixtures/archives/zpaq-damaged-163");
    let parts=c["parts"].as_array().unwrap().iter().map(|p|{let source=base.join(p["file"].as_str().unwrap());assert_eq!(sha(&source),p["sha256"]);let out=t.join(source.file_name().unwrap());fs::copy(source,&out).unwrap();out}).collect::<Vec<_>>();
    let source=if c["native"]==true {super::super::super::zpaq_multipart::confirm(&t.join("profile"),super::super::super::zpaq_multipart::suggest(&parts[0]).unwrap().unwrap()).unwrap()}else{parts[0].clone()};(source,parts)
}
#[tokio::test]
async fn zpaq163_four_modes_match_official_affected_shared_block_and_older_scope(){
    let m=manifest();
    for c in m["cases"].as_array().unwrap(){
        let t=tempfile::tempdir().unwrap();let(source,parts)=copy(t.path(),c);let key=(c["encrypted"]==true).then(||m["password"].as_str().unwrap());let before=parts.iter().map(|p|sha(p)).collect::<Vec<_>>();
        let report=super::super::super::archive_diagnostics::diagnose_archive_at(&source,key,None,flag(),None).await.unwrap();assert_eq!(report.status,"zpaq_affected");assert!(!report.can_repair);assert!(report.integrity_tested);
        let blocks=report.zpaq_blocks.unwrap();assert_eq!(blocks.snapshot.version,3);assert_eq!(blocks.revision.len(),64);assert_eq!((blocks.affected(),blocks.unchecked()),(2,0));
        let expected=c["affected"].as_array().unwrap().iter().map(|s|s.as_str().unwrap().to_string()).collect::<std::collections::BTreeSet<_>>();let actual=blocks.entries.iter().filter(|e|e.state==FileState::Affected).map(|e|e.path.clone()).collect::<std::collections::BTreeSet<_>>();assert_eq!(actual,expected);
        let failed=blocks.blocks.iter().filter(|b|b.state==BlockState::Failed).collect::<Vec<_>>();assert_eq!(failed.len(),1);assert_eq!(failed[0].files,2);for row in c["independent"].as_array().unwrap(){let item=blocks.entries.iter().find(|e|e.path==row["path"].as_str().unwrap()).unwrap();assert_eq!(item.state==FileState::Verified,row["completeHashMatched"].as_bool().unwrap());}
        let old:ZpaqSnapshot=serde_json::from_value(c["snapshots"][0].clone()).unwrap();let view=diagnose(&source,key,Some(&old),None,flag()).await.unwrap();assert!(view.entries.iter().all(|e|e.state==FileState::Verified));assert!(view.blocks.iter().all(|b|b.state==BlockState::Verified));assert_eq!(view.snapshot,old);assert_eq!(before,parts.iter().map(|p|sha(p)).collect::<Vec<_>>());
        assert!(!t.path().join("output").exists());
    }
}
#[tokio::test]
async fn zpaq163_password_directory_tail_missing_member_and_cancel_do_not_infer_usable_files(){
    let m=manifest();let t=tempfile::tempdir().unwrap();let(source,_)=copy(t.path(),&m["cases"][2]);
    for (key,status) in [(None,"password_required"),(Some("wrong"),"ambiguous_data")]{let r=super::super::super::archive_diagnostics::diagnose_archive_at(&source,key,None,flag(),None).await.unwrap();assert_eq!(r.status,status);assert!(r.zpaq_blocks.is_none());assert!(!r.integrity_tested);}
    let cancelled=flag();let stop=cancelled.clone();let timer=tokio::spawn(async move {tokio::time::sleep(Duration::from_millis(20)).await;stop.store(true,std::sync::atomic::Ordering::Relaxed);});assert!(diagnose(&source,Some(m["password"].as_str().unwrap()),None,None,cancelled).await.is_err());timer.await.unwrap();
    let native=tempfile::tempdir().unwrap();let(group,members)=copy(native.path(),&m["cases"][3]);fs::remove_file(&members[1]).unwrap();assert!(diagnose(&group,Some(m["password"].as_str().unwrap()),None,None,flag()).await.is_err());
    let plain=tempfile::tempdir().unwrap();let(file,_)=copy(plain.path(),&m["cases"][0]);let stamp=catalog_source_stamp(&file).unwrap();let bytes=fs::read(&file).unwrap();fs::write(&file,&bytes[..bytes.len()-30]).unwrap();assert!(diagnose(&file,None,None,Some(&stamp),flag()).await.is_err());let r=super::super::super::archive_diagnostics::diagnose_archive_at(&file,None,None,flag(),None).await.unwrap();assert!(r.zpaq_blocks.is_none());assert_ne!(r.status,"healthy");
    let tail=root().join("tests/fixtures/archives/zpaq-interrupted-162/plain-single/backup.zpaq");let r=super::super::super::archive_diagnostics::diagnose_archive_at(&tail,None,None,flag(),None).await.unwrap();assert_eq!(r.status,"interrupted_tail");assert!(r.zpaq_blocks.is_none());assert!(!r.integrity_tested);
}
