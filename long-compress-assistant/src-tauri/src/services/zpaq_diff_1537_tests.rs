use super::*;
use std::{fs,path::PathBuf};
fn root()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().into()}
fn manifest()->serde_json::Value{serde_json::from_slice(&fs::read(root().join("tests/fixtures/archives/zpaq-snapshots-1530/manifest.json")).unwrap()).unwrap()}
fn flag()->Arc<AtomicBool>{Arc::new(AtomicBool::new(false))}
fn quiet()->Progress{Arc::new(|_|{})}
fn snapshot(case:&serde_json::Value,n:usize)->ZpaqSnapshot{ZpaqSnapshot{version:n as u32,date:case["snapshots"][n-1]["date"].as_i64().unwrap()}}
#[tokio::test]
async fn zpaq1537_plain_password_numeric_catalog_and_exact_selected_changes_match_independent_content(){
    let m=manifest();
    for (encrypted,numeric) in [(false,false),(true,false),(true,true)]{
        let case=&m["cases"][usize::from(encrypted)];let original=root().join("tests/fixtures/archives/zpaq-snapshots-1530").join(case["file"].as_str().unwrap());let bytes=fs::read(&original).unwrap();let t=tempfile::tempdir().unwrap();
        let source=if numeric {let prefix=t.path().join("journal.zpaq");let p=super::super::split_compression::SplitCompressionService::split_archive_file(&original,&prefix,16384,||Ok(())).unwrap();p.part_files[2].clone()}else{original.clone()};
        let password=encrypted.then(||m["password"].as_str().unwrap());let (before,after)=(snapshot(case,1),snapshot(case,2));
        let result=run(&source,password,before,after,&[],None,flag(),quiet()).await.unwrap();
        assert_eq!(result.entries.iter().find(|e|e.path=="资料/后来删除.txt").unwrap().kind,"removed");assert_eq!(result.entries.iter().find(|e|e.path=="资料/新增.txt").unwrap().kind,"added");
        let candidates:Vec<_>=result.entries.iter().filter(|e|e.kind=="candidate").map(|e|e.path.clone()).collect();assert!(!candidates.is_empty());
        let checked=run(&source,password,before,after,&candidates,Some(&result.revision),flag(),quiet()).await.unwrap();
        for row in checked.entries.iter().filter(|e|e.before.as_ref().is_some_and(|s|!s.directory)&&e.after.as_ref().is_some_and(|s|!s.directory)) {
            let a=&case["snapshots"][0]["expected"][&row.path];let b=&case["snapshots"][1]["expected"][&row.path];
            if a!=b{assert_eq!(row.kind,"changed","{}",row.path)}else{assert!(matches!(row.kind.as_str(),"metadata"|"unchanged"))}
        }
        assert!(checked.entries.iter().filter(|e|candidates.contains(&e.path)).all(|e|e.verified));assert_eq!(fs::read(original).unwrap(),bytes);
        let out=t.path().join("old-selected");fs::create_dir(&out).unwrap();zpaq_engine::extract_checked_at(&source,&out,password,&["资料/后来删除.txt".into()],Some(&before),Some(&result.before_catalog_id),flag()).await.unwrap();assert_eq!(format!("{:x}",Sha256::digest(fs::read(out.join("资料/后来删除.txt")).unwrap())),case["snapshots"][0]["expected"]["资料/后来删除.txt"].as_str().unwrap());
    }
}
#[tokio::test]
async fn zpaq1537_wrong_identity_source_mutation_and_missing_volume_do_not_fall_back(){
    let m=manifest();let case=&m["cases"][0];let t=tempfile::tempdir().unwrap();let source=t.path().join("copy.zpaq");fs::copy(root().join("tests/fixtures/archives/zpaq-snapshots-1530/journal.zpaq"),&source).unwrap();let before=snapshot(case,1);let after=snapshot(case,3);
    let result=run(&source,None,before,after,&[],None,flag(),quiet()).await.unwrap();let output=t.path().join("output");fs::create_dir(&output).unwrap();
    let mut wrong=before;wrong.date+=1;assert!(run(&source,None,wrong,after,&[],None,flag(),quiet()).await.is_err());
    fs::OpenOptions::new().write(true).open(&source).unwrap().set_modified(fs::metadata(&source).unwrap().modified().unwrap()+std::time::Duration::from_secs(1)).unwrap();
    assert!(zpaq_engine::extract_checked_at(&source,&output,None,&["资料/说明.txt".into()],Some(&before),Some(&result.before_catalog_id),flag()).await.unwrap_err().to_string().contains("变化"));assert_eq!(fs::read_dir(&output).unwrap().count(),0);
    assert!(run(&source,None,before,after,&["资料/说明.txt".into()],Some(&result.revision),flag(),quiet()).await.unwrap_err().to_string().contains("变化"));
    let parts=super::super::split_compression::SplitCompressionService::split_archive_file(&source,&t.path().join("split.zpaq"),16384,||Ok(())).unwrap();fs::remove_file(&parts.part_files[1]).unwrap();assert!(run(&parts.part_files[2],None,before,after,&[],None,flag(),quiet()).await.is_err());
}
#[tokio::test]
async fn zpaq1537_actual_hash_progress_can_stop_and_metadata_never_claims_exact_content(){
    let m=manifest();let case=&m["cases"][0];let source=root().join("tests/fixtures/archives/zpaq-snapshots-1530/journal.zpaq");let (a,b)=(snapshot(case,1),snapshot(case,2));let result=run(&source,None,a,b,&[],None,flag(),quiet()).await.unwrap();
    assert!(result.entries.iter().all(|e|!e.verified));assert!(result.entries.iter().any(|e|e.kind=="candidate"&&e.reason.contains("尚未")));
    let paths:Vec<_>=result.entries.iter().filter(|e|e.kind=="candidate").map(|e|e.path.clone()).collect();let cancel=flag();let stop=cancel.clone();
    assert!(run(&source,None,a,b,&paths,Some(&result.revision),cancel,Arc::new(move|e|if e.phase=="before-hash"&&e.processed_bytes>0{stop.store(true,Ordering::Release)})).await.unwrap_err().to_string().contains("停止"));
}

#[tokio::test]
async fn zpaq1537_final_same_size_time_content_change_and_metadata_only_have_distinct_meanings(){
    let m=manifest();let original=root().join("tests/fixtures/archives/zpaq-snapshots-1530/journal.zpaq");let t=tempfile::tempdir().unwrap();let data=t.path().join("input");fs::create_dir(&data).unwrap();
    let latest=snapshot(&m["cases"][0],3);zpaq_engine::extract_at(&original,&data,None,&["资料/说明.txt".into()],Some(&latest),flag()).await.unwrap();let file=data.join("资料/说明.txt");let mut bytes=fs::read(&file).unwrap();bytes[0]^=1;fs::write(&file,bytes).unwrap();
    let old=zpaq_engine::catalog(&original,None,&latest,flag()).await.unwrap();let entry=old.entries.iter().find(|e|e.path=="资料/说明.txt").unwrap();
    let seconds=chrono::NaiveDateTime::parse_from_str(&entry.date.to_string(),"%Y%m%d%H%M%S").unwrap().and_utc().timestamp() as u64;
    let time=std::time::UNIX_EPOCH+std::time::Duration::from_secs(seconds);fs::File::options().write(true).open(&file).unwrap().set_modified(time).unwrap();
    let updated=t.path().join("updated.zpaq");super::super::zpaq_update::update(&original,&[file.to_string_lossy().into_owned()],"资料",&updated,None,1,None,flag(),Arc::new(|_|{})).await.unwrap();
    let fourth=zpaq_engine::browse(&updated,None,flag()).await.unwrap().zpaq_history.unwrap().selected;
    let diff=run(&updated,None,latest,fourth,&[],None,flag(),quiet()).await.unwrap();let row=diff.entries.iter().find(|e|e.path=="资料/说明.txt").unwrap();assert_eq!(row.kind,"candidate");assert_eq!(row.before.as_ref().unwrap().size,row.after.as_ref().unwrap().size);assert_eq!(row.before.as_ref().unwrap().date,row.after.as_ref().unwrap().date);
    let verified=run(&updated,None,latest,fourth,&[row.path.clone()],Some(&diff.revision),flag(),quiet()).await.unwrap();assert_eq!(verified.entries.iter().find(|e|e.path==row.path).unwrap().kind,"changed");
    fs::File::options().write(true).open(&file).unwrap().set_modified(time+std::time::Duration::from_secs(1)).unwrap();let metadata=t.path().join("metadata.zpaq");super::super::zpaq_update::update(&updated,&[file.to_string_lossy().into_owned()],"资料",&metadata,None,1,None,flag(),Arc::new(|_|{})).await.unwrap();
    let fifth=zpaq_engine::browse(&metadata,None,flag()).await.unwrap().zpaq_history.unwrap().selected;let only_metadata=run(&metadata,None,fourth,fifth,&[],None,flag(),quiet()).await.unwrap();assert_eq!(only_metadata.entries.iter().find(|e|e.path==row.path).unwrap().kind,"metadata");
}
