use super::*;
use sha2::{Digest,Sha256};
use std::{collections::BTreeMap,fs};
use crate::services::{archive_browser,archive_preview,universal_engine::UniversalCliEngine};
fn flag()->Arc<AtomicBool>{Arc::new(AtomicBool::new(false))}
fn root()->PathBuf{PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().into()}
fn tree(path:&Path)->BTreeMap<String,String>{walkdir::WalkDir::new(path).into_iter().map(|entry|entry.unwrap()).filter(|e|e.file_type().is_file()).map(|e|(e.path().strip_prefix(path).unwrap().to_string_lossy().replace('\\',"/"),hex::encode(Sha256::digest(fs::read(e.path()).unwrap())))).collect()}
fn seed(base:&Path)->PathBuf{
    let source=base.join("资料 目录");fs::create_dir_all(source.join("项目/空目录")).unwrap();
    fs::write(source.join("说明.txt"),"XPRESS 实际压缩，中文内容保持。\n".repeat(100)).unwrap();fs::write(source.join("空文件.txt"),[]).unwrap();
    fs::copy(root().join("src-tauri/icons/32x32.png"),source.join("图标.png")).unwrap();
    let mut data=Vec::new();for chunk in 0..16 {for n in 0..512 {data.extend_from_slice(&Sha256::digest(format!("{chunk}:{n}").as_bytes()));}data.extend_from_slice(&vec![65+chunk as u8;16384]);}
    fs::write(source.join("项目/数据.bin"),data).unwrap();
    // Default system-image exclusions must not hide ordinary user files.
    fs::write(source.join("pagefile.sys"),b"ordinary selected file").unwrap();source
}
fn independent_decode(wim:&Path,output:&Path){let out=crate::utils::process::command(root().join("src-tauri/resources/archive-engine/7z.exe")).args(["x","-y","-sccUTF-8"]).arg(wim).arg(format!("-o{}",output.display())).output().unwrap();assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stdout));}
#[test]
fn wim1531_true_xpress_resources_independent_complete_content_and_empty_directories(){
    let temp=tempfile::tempdir().unwrap();let source=seed(temp.path());let output=temp.path().join("新包.wim");let mut reported=Vec::new();
    let progress=std::sync::Mutex::new(&mut reported);create(&[source.to_string_lossy().into_owned()],&output,true,flag(),|p|progress.lock().unwrap().push(p)).unwrap();
    let data=fs::read(&output).unwrap();assert_eq!(&data[..8],b"MSWIM\0\0\0");assert_ne!(u32::from_le_bytes(data[16..20].try_into().unwrap())&0x20000,0);assert_eq!(u32::from_le_bytes(data[20..24].try_into().unwrap()),32768);
    let table=u64::from_le_bytes(data[56..64].try_into().unwrap())as usize;let size=u64::from_le_bytes(data[48..56].try_into().unwrap())&0x00ff_ffff_ffff_ffff;assert_eq!(data[55]&4,0);
    assert!((table..table+size as usize).step_by(50).any(|i|data[i+7]&4!=0&&u64::from_le_bytes(data[i..i+8].try_into().unwrap())&0x00ff_ffff_ffff_ffff < u64::from_le_bytes(data[i+16..i+24].try_into().unwrap())));
    let decoded=temp.path().join("独立回解");independent_decode(&output,&decoded);assert_eq!(tree(&decoded.join("资料 目录")),tree(&source));assert!(decoded.join("资料 目录/项目/空目录").is_dir());assert!(reported.iter().any(|p|*p>0.0));
}
#[tokio::test]
async fn wim1531_created_numeric_members_browse_preview_integrity_and_selected_extract(){
    let temp=tempfile::tempdir().unwrap();let source=seed(temp.path());let output=temp.path().join("new.wim");create(&[source.to_string_lossy().into_owned()],&output,true,flag(),|_|{}).unwrap();
    let data=fs::read(&output).unwrap();let files:Vec<_>=data.chunks(65536).enumerate().map(|(i,b)|{let file=temp.path().join(format!("parts.wim.{:03}",i+1));fs::write(&file,b).unwrap();file}).collect();assert!(files.len()>2);let member=&files[files.len()/2];
    let browse=archive_browser::browse_archive(member,None).await.unwrap();assert_eq!(browse.format,"WIM");assert_eq!(browse.entries.iter().filter(|e|!e.is_dir).count(),tree(&source).len());
    UniversalCliEngine::test_integrity(member,None).await.unwrap();assert!(archive_preview::preview_archive_text(member,"资料 目录/说明.txt",None).await.unwrap().content.contains("XPRESS"));assert_eq!(archive_preview::preview_archive_image(member,"资料 目录/图标.png",None).await.unwrap().width,32);
    let decoded=temp.path().join("selected");UniversalCliEngine::new().extract_wim_with_progress(member,&decoded,false,Arc::new(|_|{}),Arc::new(|_,_|{}),flag(),&["资料 目录/说明.txt".into()]).await.unwrap();assert_eq!(tree(&decoded).len(),1);
}
#[test]
fn wim1531_flatten_keeps_duplicate_files_and_old_request_defaults(){
    let temp=tempfile::tempdir().unwrap();for folder in ["one","two"]{fs::create_dir(temp.path().join(folder)).unwrap();fs::write(temp.path().join(folder).join("same.txt"),folder).unwrap();}fs::create_dir(temp.path().join("空目录")).unwrap();
    let sources=["one","two","空目录"].map(|n|temp.path().join(n).to_string_lossy().into_owned());let output=temp.path().join("flat.wim");create(&sources,&output,false,flag(),|_|{}).unwrap();let decoded=temp.path().join("out");independent_decode(&output,&decoded);assert_eq!(fs::read(decoded.join("same.txt")).unwrap(),b"one");assert_eq!(fs::read(decoded.join("same (1).txt")).unwrap(),b"two");assert!(decoded.join("空目录").is_dir());
    let legacy:crate::models::compression::CompressionOptions=serde_json::from_value(serde_json::json!({"level":6,"password":null,"split_size":null})).unwrap();assert_eq!(legacy.wim_method,WimMethod::Store);assert!(serde_json::from_value::<WimMethod>(serde_json::json!("lzx")).is_err());
}
#[test]
fn wim1531_real_encoder_cancel_removes_unpublished_output_and_preserves_existing_file(){
    let temp=tempfile::tempdir().unwrap();let source=seed(temp.path());let output=temp.path().join("cancel.wim");let cancel=flag();let error=create(&[source.to_string_lossy().into_owned()],&output,true,cancel.clone(),|_|cancel.store(true,Ordering::SeqCst)).unwrap_err();assert!(matches!(error.downcast_ref::<CompressionError>(),Some(CompressionError::Cancelled)),"{error}");assert!(!output.exists());
    fs::write(&output,b"previous").unwrap();assert!(create(&[source.to_string_lossy().into_owned()],&output,true,flag(),|_|{}).is_err());assert_eq!(fs::read(output).unwrap(),b"previous");
}
