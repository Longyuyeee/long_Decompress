use super::{zpaq_multipart as native,zpaq_engine as engine,archive_browser,archive_preview,split_archive_detector::SplitArchiveDetector};
use crate::models::compression::ZpaqSnapshot;
use std::{path::{Path,PathBuf},fs,collections::BTreeMap,sync::{Arc,atomic::AtomicBool}};
use sha2::{Digest,Sha256};
fn flag()->Arc<AtomicBool>{Arc::new(AtomicBool::new(false))}
fn root()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/zpaq-native-1564")}
fn manifest()->serde_json::Value{serde_json::from_slice(&fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn hash(b:&[u8])->String{hex::encode(Sha256::digest(b))}
fn tree(p:&Path)->BTreeMap<String,String>{walkdir::WalkDir::new(p).into_iter().map(Result::unwrap).filter(|e|e.file_type().is_file()).map(|e|(e.path().strip_prefix(p).unwrap().to_string_lossy().replace('\\',"/"),hash(&fs::read(e.path()).unwrap()))).collect()}
fn originals(dir:&Path,encrypted:bool)->(serde_json::Value,Vec<PathBuf>){
 let c=manifest()["cases"][usize::from(encrypted)].clone();let parts=c["parts"].as_array().unwrap().iter().map(|p|{let to=dir.join(p["file"].as_str().unwrap());fs::copy(root().join(p["file"].as_str().unwrap()),&to).unwrap();assert_eq!(hash(&fs::read(&to).unwrap()),p["sha256"]);to}).collect();(c,parts)
}
fn reference(dir:&Path,parts:&[PathBuf])->PathBuf{native::confirm(&dir.join("profile"),native::suggest(parts.last().unwrap()).unwrap().unwrap()).unwrap()}
fn snap(row:&serde_json::Value)->ZpaqSnapshot{ZpaqSnapshot{version:row["version"].as_u64().unwrap() as u32,date:row["date"].as_i64().unwrap()}}
async fn snapshots(encrypted:bool){
 let t=tempfile::tempdir().unwrap();let(c,parts)=originals(t.path(),encrypted);let password=encrypted.then(||manifest()["password"].as_str().unwrap().to_string());
 // Detection alone never turns similar names into a group or silently reads part 1.
 for part in &parts{assert!(SplitArchiveDetector::detect_split_archive(part).unwrap().is_none());assert_eq!(native::suggest(part).unwrap().unwrap().members.len(),3);assert!(engine::browse(part,password.as_deref(),flag()).await.unwrap_err().to_string().contains("multipart-confirmation"));}
 let source=reference(t.path(),&parts);let bytes=fs::read(&source).unwrap();assert!(bytes.len()<4096);assert!(!String::from_utf8_lossy(&bytes).contains(manifest()["password"].as_str().unwrap()));
 assert_eq!(native::confirm(&t.path().join("profile"),native::suggest(&parts[0]).unwrap().unwrap()).unwrap(),source);
 for row in c["snapshots"].as_array().unwrap(){let snapshot=snap(row);let browse=archive_browser::browse_archive_cancellable_at(&source,password.as_deref(),flag(),None,Some(snapshot)).await.unwrap();assert_eq!(browse.zpaq_history.as_ref().unwrap().snapshots.len(),3);assert_eq!(browse.zpaq_history.unwrap().selected,snapshot);assert_eq!(browse.total_compressed_size,c["parts"].as_array().unwrap().iter().map(|p|p["size"].as_u64().unwrap()).sum::<u64>());
  engine::verify_at(&source,password.as_deref(),Some(&snapshot),flag()).await.unwrap();let output=t.path().join(format!("all-{}",snapshot.version));engine::extract_at(&source,&output,password.as_deref(),&[],Some(&snapshot),flag()).await.unwrap();assert_eq!(tree(&output),serde_json::from_value::<BTreeMap<String,String>>(row["expected"].clone()).unwrap());
 }
 let old=snap(&c["snapshots"][0]);let text=archive_preview::preview_archive_text_at(&source,"资料/说明.txt",password.as_deref(),None,Some(old)).await.unwrap();assert!(text.content.contains("历史快照 1"));let image=archive_preview::preview_archive_image_at(&source,"资料/图标.png",password.as_deref(),None,Some(old)).await.unwrap();assert_eq!((image.width,image.height),(32,32));
 let output=t.path().join("selected");engine::extract_at(&source,&output,password.as_deref(),&["资料/后来删除.txt".into()],Some(&old),flag()).await.unwrap();assert_eq!(tree(&output).len(),1);assert_eq!(tree(&output)["资料/后来删除.txt"],c["snapshots"][0]["expected"]["资料/后来删除.txt"]);
 for (part,p) in parts.iter().zip(c["parts"].as_array().unwrap()){assert_eq!(hash(&fs::read(part).unwrap()),p["sha256"]);}
}
#[tokio::test]async fn zpaq1564_plain_genuine_three_part_snapshot_preview_and_retrieval(){snapshots(false).await;}
#[tokio::test]async fn zpaq1564_password_genuine_three_part_snapshot_preview_and_retrieval(){snapshots(true).await;}
#[tokio::test]async fn zpaq1564_missing_changed_reordered_and_bad_snapshot_never_fall_back(){
 let t=tempfile::tempdir().unwrap();let(c,parts)=originals(t.path(),true);let source=reference(t.path(),&parts);let version=super::image_reader::revision(&source).unwrap();let key=manifest()["password"].as_str().unwrap().to_string();
 let missing=parts[1].with_extension("held");fs::rename(&parts[1],&missing).unwrap();let e=engine::browse(&source,Some(&key),flag()).await.unwrap_err().to_string();assert!(e.contains("incomplete"),"{e}");let g=native::suggest(&parts[2]).unwrap().unwrap();assert_eq!(g.missing,vec!["journal-key02.zpaq"]);assert!(native::confirm(&t.path().join("profile"),g).is_err());fs::rename(missing,&parts[1]).unwrap();assert_eq!(super::image_reader::revision(&source).unwrap(),version);
 let mut group=native::details(&source).unwrap().unwrap();group.members.swap(1,2);assert!(native::confirm(t.path(),group).unwrap_err().to_string().contains("次序"));
 let old=snap(&c["snapshots"][0]);assert!(engine::browse_at(&source,Some("wrong"),Some(&old),flag()).await.is_err());let unavailable=ZpaqSnapshot{version:9,..old};assert!(engine::browse_at(&source,Some(&key),Some(&unavailable),flag()).await.unwrap_err().to_string().contains("snapshot-unavailable"));
 let extra=t.path().join("journal-key04.zpaq");fs::write(extra,b"not a confirmed part").unwrap();assert_eq!(native::inspect(&source).unwrap().unwrap().parts.len(),3);
 let mut file=fs::OpenOptions::new().append(true).open(&parts[2]).unwrap();use std::io::Write;file.write_all(b"changed").unwrap();assert!(engine::browse_at(&source,Some(&key),Some(&old),flag()).await.unwrap_err().to_string().contains("changed"));assert!(super::image_reader::check_source(&source,&version).is_err());
}
#[tokio::test]async fn zpaq1564_explicit_single_is_independent(){
 let t=tempfile::tempdir().unwrap();let(_,parts)=originals(t.path(),false);let mut group=native::suggest(&parts[0]).unwrap().unwrap();group.members.truncate(1);let source=native::confirm(&t.path().join("profile"),group).unwrap();let browse=engine::browse(&source,None,flag()).await.unwrap();assert_eq!(browse.zpaq_history.unwrap().snapshots.len(),1);
}

#[tokio::test]async fn zpaq1564_date_named_independent_backups_remain_readable(){let t=tempfile::tempdir().unwrap();for day in [20261021,20261022]{let p=t.path().join(format!("backup{day}.zpaq"));fs::copy(root().join("journal01.zpaq"),&p).unwrap();assert!(native::suggest(&p).unwrap().is_none());assert_eq!(engine::browse(&p,None,flag()).await.unwrap().zpaq_history.unwrap().snapshots.len(),1);}}
