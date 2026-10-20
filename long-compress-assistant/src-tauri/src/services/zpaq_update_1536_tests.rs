use super::*;
use sha2::{Digest,Sha256};
use std::collections::BTreeMap;
fn root()->PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().into() }
fn fixtures()->PathBuf {root().join("tests/fixtures/archives/zpaq-snapshots-1530")}
fn manifest()->serde_json::Value {serde_json::from_slice(&fs::read(fixtures().join("manifest.json")).unwrap()).unwrap()}
fn flag()->Arc<AtomicBool> {Arc::new(AtomicBool::new(false))}
fn quiet()->Progress {Arc::new(|_|{})}
fn hash(data:&[u8])->String {hex::encode(Sha256::digest(data))}
fn tree(path:&Path)->BTreeMap<String,String> {walkdir::WalkDir::new(path).into_iter().map(Result::unwrap).filter(|e|e.file_type().is_file()).map(|e|(e.path().strip_prefix(path).unwrap().to_string_lossy().replace('\\',"/"),hash(&fs::read(e.path()).unwrap()))).collect()}
fn join(parts:&[PathBuf])->Vec<u8> {parts.iter().flat_map(|p|fs::read(p).unwrap()).collect()}
fn parts(file:&Path)->Vec<PathBuf> {super::super::archive_volumes::complete_volume_paths(file).unwrap()}
fn independent(file:&Path,output:&Path,password:Option<&str>,version:Option<u32>) {
    let mut command=crate::utils::process::command(root().join("test-results/zpaq-volumes-1526/upstream/zpaq64.exe"));
    command.arg("x").arg(file).arg("-to").arg(output).args(["-threads","1","-noattributes"]);
    if let Some(p)=password {command.arg("-key").arg(p);} if let Some(v)=version {command.arg("-until").arg(v.to_string());}
    let result=command.output().unwrap();assert!(result.status.success(),"{} {}",String::from_utf8_lossy(&result.stdout),String::from_utf8_lossy(&result.stderr));
}
#[tokio::test]
async fn zpaq1536_incremental_plain_password_single_numeric_retains_history_and_no_change() {
    let m=manifest();
    for (encrypted,source_split,target_split) in [(false,false,false),(false,true,true),(true,false,true),(true,true,false)] {
        let t=tempfile::tempdir().unwrap();let c=&m["cases"][usize::from(encrypted)];let original=fixtures().join(c["file"].as_str().unwrap());let before=fs::read(&original).unwrap();
        let password=encrypted.then(||m["password"].as_str().unwrap());
        let source=if source_split {let prefix=t.path().join("原备份.zpaq");let mut offset=0;let mut members=Vec::new();while offset<before.len(){let end=(offset+if members.is_empty(){7}else if members.len()==1{13}else{8191}).min(before.len());let p=PathBuf::from(format!("{}.{:03}",prefix.display(),members.len()+1));fs::write(&p,&before[offset..end]).unwrap();members.push(p);offset=end;}members[members.len()/2].clone()} else {let p=t.path().join("原备份.zpaq");fs::write(&p,&before).unwrap();p};
        let old=zpaq_engine::browse(&source,password,flag()).await.unwrap();let info=old.entries.iter().find(|e|e.path=="资料/说明.txt").unwrap();
        let input=t.path().join("说明.txt");fs::write(&input,vec![b'X';info.size as usize]).unwrap();
        let date=chrono::NaiveDateTime::parse_from_str(info.modified.as_ref().unwrap(),"%Y-%m-%d %H:%M:%S UTC").unwrap().and_utc().timestamp();
        fs::OpenOptions::new().write(true).open(&input).unwrap().set_modified(std::time::UNIX_EPOCH+Duration::from_secs(date as u64)).unwrap();
        let new=t.path().join("新增.txt");fs::write(&new,"新增内容与空目录。\n").unwrap();let empty=t.path().join("新增空目录");fs::create_dir(&empty).unwrap();
        let inputs=[input.to_string_lossy().into_owned(),new.to_string_lossy().into_owned(),empty.to_string_lossy().into_owned()];
        let output=t.path().join("更新.zpaq");let summary=update(&source,&inputs,"资料",&output,password,1,target_split.then_some(16384),flag(),quiet()).await.unwrap();
        assert_eq!((summary.base_snapshots,summary.snapshots),(3,4));assert_eq!(summary.updated_entries,3);assert!(summary.new_fragments>0);assert!(!summary.unchanged);assert_eq!(summary.copied_bytes,before.len() as u64);
        let produced=if target_split {parts(&output)}else{vec![output.clone()]};let bytes=join(&produced);assert!(bytes.starts_with(&before));assert_eq!(fs::read(&original).unwrap(),before);
        let oracle=t.path().join("独立回解.zpaq");fs::write(&oracle,&bytes).unwrap();let dest=t.path().join("最新");independent(&oracle,&dest,password,None);
        let mut expected:BTreeMap<String,String>=serde_json::from_value(c["snapshots"][2]["expected"].clone()).unwrap();expected.insert("资料/说明.txt".into(),hash(&fs::read(&input).unwrap()));expected.insert("资料/新增.txt".into(),hash(&fs::read(&new).unwrap()));assert_eq!(tree(&dest),expected);assert!(dest.join("资料/新增空目录").is_dir());
        let old_dest=t.path().join("第一版");independent(&oracle,&old_dest,password,Some(1));assert_eq!(tree(&old_dest),serde_json::from_value(c["snapshots"][0]["expected"].clone()).unwrap());assert!(old_dest.join("资料/后来删除.txt").exists());
        let no_change=t.path().join("同内容副本.zpaq");let same=update(produced.last().unwrap(),&inputs,"资料",&no_change,password,1,None,flag(),quiet()).await.unwrap();assert!(same.unchanged);assert_eq!((same.updated_entries,same.new_fragments,same.snapshots),(0,0,4));assert_eq!(fs::read(no_change).unwrap(),bytes);
        println!("ZPAQ update encrypted={encrypted}, source numeric={source_split}, target numeric={target_split}: old prefix retained, complete independent latest/old hashes and unchanged copy OK");
    }
}
#[tokio::test]
async fn zpaq1536_copy_cancellation_preserves_original_and_cleans_only_staging() {
    let t=tempfile::tempdir().unwrap();let source=fixtures().join("journal.zpaq");let before=fs::read(&source).unwrap();let input=t.path().join("添加.txt");fs::write(&input,b"addition").unwrap();let output=t.path().join("new.zpaq");let cancel=flag();let trigger=cancel.clone();
    let result=update(&source,&[input.to_string_lossy().into_owned()],"",&output,None,1,None,cancel,Arc::new(move|e|if e.phase=="copying"&&e.processed_bytes>0{trigger.store(true,Ordering::Release)})).await;
    assert!(result.unwrap_err().to_string().contains("取消"));assert!(!output.exists());assert_eq!(fs::read(source).unwrap(),before);assert_eq!(fs::read_dir(t.path()).unwrap().count(),1);
}
#[tokio::test]
async fn zpaq1536_incremental_child_cancellation_stops_real_work_and_discards_copy() {
    let t=tempfile::tempdir().unwrap();let source=fixtures().join("journal.zpaq");let before=fs::read(&source).unwrap();let input=t.path().join("new.bin");let payload:Vec<u8>=(0..131072).flat_map(|n|Sha256::digest(format!("1536:{n}").as_bytes())).collect();fs::write(&input,payload).unwrap();let output=t.path().join("new.zpaq");let cancel=flag();let trigger=cancel.clone();
    let result=update(&source,&[input.to_string_lossy().into_owned()],"",&output,None,6,None,cancel,Arc::new(move|e|if e.phase=="updating"&&e.processed_bytes>0{trigger.store(true,Ordering::Release)})).await;
    assert!(result.unwrap_err().to_string().contains("取消"));assert!(!output.exists());assert_eq!(fs::read(source).unwrap(),before);assert_eq!(fs::read_dir(t.path()).unwrap().count(),1);
}
#[tokio::test]
async fn zpaq1536_incomplete_password_and_mapping_failures_keep_existing_results() {
    let t=tempfile::tempdir().unwrap();let source=fixtures().join("journal-key.zpaq");let original=fs::read(&source).unwrap();let input=t.path().join("added.txt");fs::write(&input,b"input").unwrap();let output=t.path().join("new.zpaq");
    assert!(update(&source,&[input.to_string_lossy().into_owned()],"",&output,Some("wrong"),1,None,flag(),quiet()).await.is_err());assert!(!output.exists());
    fs::write(&output,b"old result").unwrap();assert!(update(&source,&[input.to_string_lossy().into_owned()],"",&output,None,1,None,flag(),quiet()).await.is_err());assert_eq!(fs::read(&output).unwrap(),b"old result");
    let output2=t.path().join("conflict.zpaq");let conflicting=t.path().join("资料");fs::write(&conflicting,b"not a directory").unwrap();assert!(update(&source,&[conflicting.to_string_lossy().into_owned()],"",&output2,Some(manifest()["password"].as_str().unwrap()),1,None,flag(),quiet()).await.unwrap_err().to_string().contains("冲突"));assert!(!output2.exists());
    let prefix=t.path().join("split.zpaq");let group=super::super::split_compression::SplitCompressionService::split_archive_file(&source,&prefix,16384,||Ok(())).unwrap();fs::remove_file(&group.part_files[1]).unwrap();
    assert!(update(&group.part_files[0],&[input.to_string_lossy().into_owned()],"",&output2,Some(manifest()["password"].as_str().unwrap()),1,None,flag(),quiet()).await.is_err());assert!(!output2.exists());assert_eq!(fs::read(&source).unwrap(),original);
}
