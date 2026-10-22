use super::*;
use crate::services::zpaq_multipart as native;
use sha2::{Digest,Sha256};
fn flag()->Arc<AtomicBool>{Arc::new(AtomicBool::new(false))}
fn quiet()->Progress{Arc::new(|_|{})}
fn root()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("..")}
fn hash(path:&Path)->String{hex::encode(Sha256::digest(fs::read(path).unwrap()))}
fn tree(path:&Path)->BTreeMap<String,String>{walkdir::WalkDir::new(path).into_iter().map(Result::unwrap).filter(|e|e.file_type().is_file()).map(|e|(e.path().strip_prefix(path).unwrap().to_string_lossy().replace('\\',"/"),hash(e.path()))).collect()}
fn original(dir:&Path,encrypted:bool)->(PathBuf,serde_json::Value,Vec<PathBuf>,String){
    let fixtures=root().join("tests/fixtures/archives/zpaq-native-1564");let m:serde_json::Value=serde_json::from_slice(&fs::read(fixtures.join("manifest.json")).unwrap()).unwrap();
    let c=m["cases"][usize::from(encrypted)].clone();
    let parts=c["parts"].as_array().unwrap().iter().map(|p|{let out=dir.join(p["file"].as_str().unwrap());fs::copy(fixtures.join(p["file"].as_str().unwrap()),&out).unwrap();out}).collect::<Vec<_>>();
    let source=native::confirm(&dir.join("profile"),native::suggest(&parts[0]).unwrap().unwrap()).unwrap();
    (source,c,parts,m["password"].as_str().unwrap().into())
}
#[test]fn zpaq161_confirm_long_private_source_record_path(){
    let t=tempfile::tempdir().unwrap();let(_,_,parts,_)=original(t.path(),false);
    let root=t.path().join("metadata-".repeat(21));
    let source=native::confirm(&root,native::suggest(&parts[0]).unwrap().unwrap()).unwrap();
    assert!(source.to_string_lossy().len()>260);assert_eq!(native::inspect(&source).unwrap().unwrap().parts,parts);
}
async fn independent(encrypted:bool){
    let t=tempfile::tempdir().unwrap();let(source,c,parts,key)=original(t.path(),encrypted);let password=encrypted.then_some(key.as_str());
    let input=t.path().join("说明.txt");fs::write(&input,"本次更新的说明，未选条目保留。".as_bytes()).unwrap();
    let added=t.path().join("新资料.txt");fs::write(&added,b"new native transaction").unwrap();
    let sources=vec![input.to_string_lossy().into_owned(),added.to_string_lossy().into_owned()];
    let output=t.path().join("独立更新.zpaq");
    let phases=Arc::new(Mutex::new(Vec::new()));let seen=phases.clone();let progress:Progress=Arc::new(move|p|seen.lock().unwrap().push(p.phase));
    let summary=update(&source,&sources,"资料",&output,password,1,None,flag(),progress).await.unwrap();
    assert_eq!((summary.native_parts,summary.base_snapshots,summary.snapshots,summary.unchanged),(Some(4),3,4,false));
    let output_parts=native_update::output_paths(&output,4).unwrap();
    for (i,p) in parts.iter().enumerate(){assert_eq!(hash(p),c["parts"][i]["sha256"]);assert_eq!(hash(p),hash(&output_parts[i]));}
    assert!(!output.exists());assert!(output_parts[3].metadata().unwrap().len()<summary.copied_bytes);
    let phases=phases.lock().unwrap();for p in ["reading","copying","updating","verifying","publishing","completed"]{assert!(phases.contains(&p));}drop(phases);
    for version in 1..=4 {
        let destination=t.path().join(format!("official-{version}"));
        let mut command=crate::utils::process::command(root().join("test-results/zpaq-volumes-1526/upstream/zpaq64.exe"));
        command.arg("x").arg(output.with_file_name("独立更新.part*.zpaq")).arg("-until").arg(version.to_string()).arg("-to").arg(&destination).args(["-threads","1","-noattributes"]);
        if let Some(password)=password{command.arg("-key").arg(password);}
        let r=command.output().unwrap();assert!(r.status.success(),"{} {}",String::from_utf8_lossy(&r.stdout),String::from_utf8_lossy(&r.stderr));
        let mut expected: BTreeMap<String,String>=serde_json::from_value(c["snapshots"][(version.min(3)-1)as usize]["expected"].clone()).unwrap();
        if version==4 {expected.insert("资料/说明.txt".into(),hash(&input));expected.insert("资料/新资料.txt".into(),hash(&added));}
        assert_eq!(tree(&destination),expected);
    }
    let group=native::suggest(&output_parts[3]).unwrap().unwrap();let updated=native::confirm(&t.path().join("profile"),group).unwrap();
    let same=t.path().join("无变化副本.zpaq");let no_change=update(&updated,&sources,"资料",&same,password,1,None,flag(),quiet()).await.unwrap();
    assert_eq!((no_change.native_parts,no_change.snapshots,no_change.unchanged),(Some(4),4,true));
    for (a,b) in output_parts.iter().zip(native_update::output_paths(&same,4).unwrap()){assert_eq!(hash(a),hash(&b));}
    assert!(!same.with_file_name("无变化副本.part5.zpaq").exists());
    assert!(!fs::read_dir(t.path()).unwrap().any(|e|e.unwrap().file_name().to_string_lossy().starts_with(".zpaq-native-update-")));
}
#[tokio::test]async fn zpaq161_native_plain_independent_four_snapshots_and_unchanged(){independent(false).await;}
#[tokio::test]async fn zpaq161_native_password_independent_four_snapshots_and_unchanged(){independent(true).await;}
#[tokio::test]async fn zpaq161_native_password_conflict_source_change_cancel_and_retry(){
    let t=tempfile::tempdir().unwrap();let(source,_c,parts,key)=original(t.path(),true);let input=t.path().join("新增.txt");fs::write(&input,b"added").unwrap();let sources=vec![input.to_string_lossy().into_owned()];let output=t.path().join("新组.zpaq");
    assert!(update(&source,&sources,"",&output,Some("wrong"),1,None,flag(),quiet()).await.is_err());
    assert!(update(&source,&sources,"",&output,Some(&key),1,Some(1024),flag(),quiet()).await.unwrap_err().to_string().contains("数字分卷"));
    let conflict=output.with_file_name("新组.part9.zpaq");fs::write(&conflict,b"existing result").unwrap();
    assert!(update(&source,&sources,"",&output,Some(&key),1,None,flag(),quiet()).await.unwrap_err().to_string().contains("已有部分"));assert_eq!(fs::read(&conflict).unwrap(),b"existing result");fs::remove_file(&conflict).unwrap();
    let stop=flag();let request=stop.clone();let progress:Progress=Arc::new(move|p|if p.phase=="copying"{request.store(true,Ordering::Release)});
    assert!(update(&source,&sources,"",&output,Some(&key),1,None,stop,progress).await.is_err());
    assert!(!native_update::output_paths(&output,4).unwrap().iter().any(|p|p.exists()));
    // An output that appears after verification is neither overwritten nor removed.
    let conflict=output.with_file_name("新组.part2.zpaq");let path=conflict.clone();let progress:Progress=Arc::new(move|p|if p.phase=="publishing"{fs::write(&path,b"arrived later").unwrap()});
    assert!(update(&source,&sources,"",&output,Some(&key),1,None,flag(),progress).await.is_err());assert_eq!(fs::read(&conflict).unwrap(),b"arrived later");fs::remove_file(&conflict).unwrap();
    // Changing a confirmed member during work rejects publication.
    let changed=parts[1].clone();let bytes=fs::read(&changed).unwrap();let progress:Progress=Arc::new(move|p|if p.phase=="verifying"{fs::write(&changed,b"changed").unwrap()});
    assert!(update(&source,&sources,"",&output,Some(&key),1,None,flag(),progress).await.unwrap_err().to_string().contains("来源已变化"));
    fs::write(&parts[1],bytes).unwrap();let fresh=native::confirm(&t.path().join("profile"),native::suggest(&parts[0]).unwrap().unwrap()).unwrap();
    let result=update(&fresh,&sources,"",&output,Some(&key),1,None,flag(),quiet()).await.unwrap();assert_eq!(result.native_parts,Some(4));
    assert!(!fs::read_dir(t.path()).unwrap().any(|e|e.unwrap().file_name().to_string_lossy().starts_with(".zpaq-native-update-")));
}
