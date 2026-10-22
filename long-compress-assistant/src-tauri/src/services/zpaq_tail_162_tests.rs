use super::*;
use sha2::{Digest,Sha256};
fn root()->PathBuf{PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")}
fn flag()->Arc<AtomicBool>{Arc::new(AtomicBool::new(false))}
fn hash(p:&Path)->String{hex::encode(Sha256::digest(std::fs::read(p).unwrap()))}
fn cases()->serde_json::Value{serde_json::from_slice(&std::fs::read(root().join("tests/fixtures/archives/zpaq-interrupted-162/manifest.json")).unwrap()).unwrap()}
fn source(temp:&Path,c:&serde_json::Value)->(PathBuf,Vec<PathBuf>){
    let folder=root().join("tests/fixtures/archives/zpaq-interrupted-162");
    let parts=c["parts"].as_array().unwrap().iter().map(|p|{
        let src=folder.join(p["file"].as_str().unwrap());assert_eq!(hash(&src),p["sha256"]);
        let dest=temp.join(src.file_name().unwrap());std::fs::copy(src,&dest).unwrap();dest
    }).collect::<Vec<_>>();
    let path=if c["native"]==true{super::super::zpaq_multipart::confirm(&temp.join("profile"),super::super::zpaq_multipart::suggest(&parts[0]).unwrap().unwrap()).unwrap()}else{parts[0].clone()};
    (path,parts)
}
#[tokio::test]
async fn zpaq162_real_interruption_four_modes_twelve_snapshots(){
    let m=cases();
    for c in m["cases"].as_array().unwrap(){
        let t=tempfile::tempdir().unwrap();let(path,parts)=source(t.path(),c);
        let before=parts.iter().map(|p|hash(p)).collect::<Vec<_>>();
        let key=(c["encrypted"]==true).then(||m["password"].as_str().unwrap());
        let view=browse(&path,key,flag()).await.unwrap();let h=view.zpaq_history.unwrap();
        assert_eq!(h.snapshots.len(),3);assert_eq!(h.interrupted_tail.unwrap().offset,c["transactionOffset"].as_u64().unwrap());
        let unwanted=t.path().join("no-automatic-fallback");
        assert!(extract_at(&path,&unwanted,key,&[],None,flag()).await.is_err());assert!(!unwanted.exists());
        for s in c["snapshots"].as_array().unwrap(){
            let snapshot=ZpaqSnapshot{version:s["version"].as_u64().unwrap() as u32,date:s["date"].as_i64().unwrap()};
            let output=t.path().join(format!("v{}",snapshot.version));
            extract_at(&path,&output,key,&[],Some(&snapshot),flag()).await.unwrap();
            let actual=walkdir::WalkDir::new(&output).into_iter().map(Result::unwrap).filter(|e|e.file_type().is_file()).map(|e|(e.path().strip_prefix(&output).unwrap().to_string_lossy().replace('\\',"/"),hash(e.path()))).collect::<std::collections::BTreeMap<_,_>>();
            let expected=serde_json::from_value::<std::collections::BTreeMap<String,String>>(s["expected"].clone()).unwrap();assert_eq!(actual,expected);
            assert!(!output.join("interrupted.bin").exists());
        }
        assert_eq!(before,parts.iter().map(|p|hash(p)).collect::<Vec<_>>());
        if key.is_some(){assert!(try_password(&path,key.unwrap(),flag()).await.unwrap());assert!(!try_password(&path,"wrong-key",flag()).await.unwrap());}
        let missing=ZpaqSnapshot{version:4,date:20261022010000};assert!(browse_at(&path,key,Some(&missing),flag()).await.unwrap_err().to_string().contains("snapshot-unavailable"));
    }
}
#[tokio::test]
async fn zpaq162_unconfirmed_damage_missing_member_and_cancel_are_not_recovery(){
    let m=cases();let t=tempfile::tempdir().unwrap();let(path,parts)=source(t.path(),&m["cases"][0]);
    let original=std::fs::read(&parts[0]).unwrap();let boundary=m["cases"][0]["transactionOffset"].as_u64().unwrap() as usize;
    // Explicitly artificial boundaries, separate from the official originals.
    for (name,bytes) in [("cut-header",original[..boundary+30].to_vec()),("cut-index",original[..boundary-30].to_vec()),("no-old-snapshot",original[boundary..].to_vec())]{
        let broken=t.path().join(format!("{name}.zpaq"));std::fs::write(&broken,bytes).unwrap();assert!(browse(&broken,None,flag()).await.is_err(),"{name}");
    }
    let cancelled=flag();cancelled.store(true,std::sync::atomic::Ordering::Relaxed);assert!(browse(&path,None,cancelled).await.is_err());
    let native=tempfile::tempdir().unwrap();let(group,members)=source(native.path(),&m["cases"][1]);std::fs::remove_file(&members[3]).unwrap();assert!(browse(&group,None,flag()).await.is_err());
}
