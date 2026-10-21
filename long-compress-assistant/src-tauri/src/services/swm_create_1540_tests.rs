use super::*;
use std::{fs, collections::BTreeMap};
use sha2::{Digest, Sha256};
use crate::services::{archive_browser, archive_preview, archive_volumes, output_publish_transaction, universal_engine::UniversalCliEngine};
fn flag() -> Arc<AtomicBool> { Arc::new(AtomicBool::new(false)) }
fn tree(path: &Path) -> BTreeMap<String,String> {
    walkdir::WalkDir::new(path).into_iter().map(|e|e.unwrap()).filter(|e|e.file_type().is_file())
        .map(|e|(e.path().strip_prefix(path).unwrap().to_string_lossy().replace('\\',"/"),hex::encode(Sha256::digest(fs::read(e.path()).unwrap())))).collect()
}
fn seed(base: &Path) -> PathBuf {
    let source=base.join("资料");fs::create_dir_all(source.join("项目/空目录")).unwrap();
    fs::write(source.join("说明.txt"),"原生 SWM 创建，中文与空目录保留。").unwrap();fs::write(source.join("空.txt"),[]).unwrap();
    fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("icons/32x32.png"),source.join("图标.png")).unwrap();
    for file in 0..5 { let data:Vec<u8>=(0..3072).flat_map(|n|Sha256::digest(format!("{file}:{n}").as_bytes()).to_vec()).collect();fs::write(source.join(format!("项目/数据{file}.bin")),data).unwrap(); }
    source
}
fn independent(parts: &[PathBuf], base: &Path) -> PathBuf {
    let joined=base.join("joined.wim");let out=crate::utils::process::command(command_path().unwrap()).arg("join").arg(&joined).args(parts).output().unwrap();assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));
    let decoded=base.join("independent");let out=crate::utils::process::command(Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/archive-engine/7z.exe")).args(["x","-y"]).arg(joined).arg(format!("-o{}",decoded.display())).output().unwrap();assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));decoded
}
#[tokio::test]
async fn swm1540_three_methods_headers_independent_complete_decode_and_new_member_flows() {
    for (method, bits) in [(WimMethod::Store,0),(WimMethod::Xpress,0x20002),(WimMethod::Lzx,0x40002)] {
        let temp=tempfile::tempdir().unwrap();let source=seed(temp.path());let output=temp.path().join("install2026.swm");
        let reported=std::sync::Mutex::new(Vec::new());create_swm(&[source.to_string_lossy().into_owned()],&output,true,method,65536,flag(),|p,_|reported.lock().unwrap().push(p)).unwrap();
        let parts=archive_volumes::complete_volume_paths(&output).unwrap();assert!(parts.len()>2);let head=fs::read(&parts[0]).unwrap();
        for (i,part) in parts.iter().enumerate(){let b=fs::read(part).unwrap();assert_eq!(&b[..8],b"MSWIM\0\0\0");assert_eq!(&b[24..40],&head[24..40]);assert_eq!(u16::from_le_bytes(b[40..42].try_into().unwrap())as usize,i+1);assert_eq!(u16::from_le_bytes(b[42..44].try_into().unwrap())as usize,parts.len());assert_eq!(u32::from_le_bytes(b[16..20].try_into().unwrap())&0x003f0002,bits);}
        let decoded=independent(&parts,temp.path());assert_eq!(tree(&decoded.join("资料")),tree(&source));assert!(decoded.join("资料/项目/空目录").is_dir());
        let member=parts.last().unwrap();let browse=archive_browser::browse_archive(member,None).await.unwrap();assert_eq!(browse.entries.iter().filter(|e|!e.is_dir).count(),tree(&source).len());UniversalCliEngine::test_integrity(member,None).await.unwrap();
        assert!(archive_preview::preview_archive_text(member,"资料/说明.txt",None).await.unwrap().content.contains("SWM"));assert_eq!(archive_preview::preview_archive_image(member,"资料/图标.png",None).await.unwrap().width,32);
        let selected=temp.path().join("selected");UniversalCliEngine::new().extract_wim_with_progress(member,&selected,false,Arc::new(|_|{}),Arc::new(|_,_|{}),flag(),&["资料/说明.txt".into()]).await.unwrap();assert_eq!(tree(&selected).len(),1);
        let full=temp.path().join("full");UniversalCliEngine::new().extract_wim_with_progress(&parts[parts.len()/2],&full,false,Arc::new(|_|{}),Arc::new(|_,_|{}),flag(),&[]).await.unwrap();assert_eq!(tree(&full),tree(&decoded));
        assert!(reported.lock().unwrap().iter().any(|p|*p>=0.75));assert!(parts.iter().any(|p|p.metadata().unwrap().len()>65536));
    }
}
#[tokio::test]
async fn swm1540_publication_results_missing_first_middle_tail_and_restore() {
    let temp=tempfile::tempdir().unwrap();let source=seed(temp.path());let final_path=temp.path().join("交付.swm");let staged=output_publish_transaction::staged_output_path(&final_path,"archive").unwrap();
    create_swm(&[source.to_string_lossy().into_owned()],&staged,true,WimMethod::Xpress,65536,flag(),|_,_|{}).unwrap();output_publish_transaction::publish_verified_volumes(&staged,&final_path,||false).unwrap();
    let parts=archive_volumes::complete_volume_paths(&final_path).unwrap();assert_eq!(parts[1].file_name().unwrap(),"交付2.swm");assert!(!staged.exists());
    for i in [0,parts.len()/2,parts.len()-1] { let saved=fs::read(&parts[i]).unwrap();fs::remove_file(&parts[i]).unwrap();let member=if i==0 {&parts[1]}else{&parts[0]};let e=archive_browser::browse_archive(member,None).await.unwrap_err().to_string();assert!(e.contains(parts[i].file_name().unwrap().to_str().unwrap()),"{e}");fs::write(&parts[i],saved).unwrap(); }
    UniversalCliEngine::test_integrity(parts.last().unwrap(),None).await.unwrap();
}
#[test]
fn swm1540_cancel_split_cleanup_and_collision_keeps_existing_parts() {
    let temp=tempfile::tempdir().unwrap();let source=seed(temp.path());let output=temp.path().join("stop.swm");let cancelled=flag();
    let result=create_swm(&[source.to_string_lossy().into_owned()],&output,true,WimMethod::Lzx,65536,cancelled.clone(),|p,_|if p>=0.75 {cancelled.store(true,Ordering::Relaxed)});assert!(result.is_err());assert!(archive_volumes::volume_paths(&output).unwrap().is_empty());
    let final_path=temp.path().join("taken.swm");let staged=output_publish_transaction::staged_output_path(&final_path,"archive").unwrap();create_swm(&[source.to_string_lossy().into_owned()],&staged,true,WimMethod::Store,65536,flag(),|_,_|{}).unwrap();
    let occupied=temp.path().join("taken2.swm");fs::write(&occupied,b"previous").unwrap();assert!(output_publish_transaction::publish_verified_volumes(&staged,&final_path,||false).is_err());assert_eq!(fs::read(occupied).unwrap(),b"previous");assert!(!final_path.exists());assert!(archive_volumes::volume_paths(&staged).unwrap().is_empty());
}
#[test]
fn swm1540_rejects_solid_lzms_password_and_missing_size_before_output() {
    let temp=tempfile::tempdir().unwrap();let source=seed(temp.path());let output=temp.path().join("invalid.swm");let sources=vec![source.to_string_lossy().into_owned()];
    let mut options=crate::models::compression::CompressionOptions{format:Some("swm".into()),split_size:Some(65536),..Default::default()};assert!(crate::services::compression_format::validate_compression_request(&sources,&output.to_string_lossy(),&options).is_ok());
    for method in [WimMethod::Solid,WimMethod::Lzms]{options.wim_method=method;assert!(crate::services::compression_format::validate_compression_request(&sources,&output.to_string_lossy(),&options).is_err());}
    options.wim_method=WimMethod::Store;options.split_size=None;assert!(crate::services::compression_format::validate_compression_request(&sources,&output.to_string_lossy(),&options).is_err());options.split_size=Some(65536);options.password=Some("secret".into());assert!(crate::services::compression_format::validate_compression_request(&sources,&output.to_string_lossy(),&options).is_err());assert!(!output.exists());
}

#[test]
fn swm1540_source_byte_count_is_announced_at_split_transition() {
    let temp=tempfile::tempdir().unwrap();let source=seed(temp.path());let output=temp.path().join("nested/new.swm");let events=std::sync::Mutex::new(Vec::new());
    let bytes=create_swm(&[source.to_string_lossy().into_owned()],&output,true,WimMethod::Xpress,65536,flag(),|p,b|events.lock().unwrap().push((p,b))).unwrap();
    let events=events.lock().unwrap();let split=events.iter().find(|(p,_)|*p>=0.75).unwrap();assert_eq!(split.1,bytes);assert!(bytes>400000);assert!(output.is_file());
}
