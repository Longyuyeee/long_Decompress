use super::{archive_preview as preview,image_reader::{self,Control},compression_service::{CompressionService,CompressionServiceConfig},encrypted_password_service::EncryptedPasswordService,io_buffer_pool::IOBufferPool,rar_support::RarSupportService,universal_engine::UniversalCliEngine};
use std::{path::{Path,PathBuf},sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}}};
use base64::Engine;
use sha2::Digest;
fn fixtures()->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-image-1618")}
fn manifest()->serde_json::Value {serde_json::from_slice(&std::fs::read(fixtures().join("manifest.json")).unwrap()).unwrap()}
fn case(name:&str)->PathBuf {let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();fixtures().join(name).join(c["members"][0]["name"].as_str().unwrap())}
fn bytes(p:&preview::ArchiveImagePreview)->Vec<u8>{base64::engine::general_purpose::STANDARD.decode(p.data_url.split(',').nth(1).unwrap()).unwrap()}
const IMAGE:&str="gallery/01-cover.png";
const KEY:&str="7Z图片密码1618!";

#[tokio::test]
async fn sevenz1618_original_lzma_lzma2_images_have_exact_complete_bytes() {
 let m=manifest();let p=case("lzma2");let revision=image_reader::revision(&p).unwrap();
 for e in m["images"].as_array().unwrap() {
  let value=image_reader::read(&p,e["path"].as_str().unwrap(),None,None,None,&revision,Control::default()).await.unwrap();
  assert_eq!((value.width,value.height),(e["width"].as_u64().unwrap() as u32,e["height"].as_u64().unwrap() as u32));
  assert_eq!(hex::encode(sha2::Sha256::digest(bytes(&value))),e["sha256"].as_str().unwrap());
 }
 let value=preview::preview_archive_image(&case("lzma"),IMAGE,None).await.unwrap();assert_eq!(hex::encode(sha2::Sha256::digest(bytes(&value))),m["images"][0]["sha256"].as_str().unwrap());
 let large=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/sevenz-image-1618/large-unrelated.7z");assert!(large.metadata().unwrap().len()>64*1024*1024);
 assert_eq!(preview::preview_archive_image(&large,"preview.png",None).await.unwrap().width,320);
}

#[tokio::test]
async fn sevenz1618_limits_end_marker_and_shared_streams_do_not_change_text_limits() {
 let at=preview::preview_archive_image(&fixtures().join("at-limit.7z"),"at-limit.bmp",None).await.unwrap();assert_eq!(at.byte_size,8*1024*1024);
 let m=manifest();assert_eq!(hex::encode(sha2::Sha256::digest(bytes(&at))),m["limits"]["at-limit.bmp"]["sha256"].as_str().unwrap());
 for (p,entry,reason) in [(fixtures().join("oversize.7z"),"oversize.bmp","8 MiB"),(fixtures().join("dimensions.7z"),"dimensions.png","dimensions exceed"),(case("lzma2"),"gallery/03-bad.png","not a supported raster"),(case("solid"),IMAGE,"固实"),(fixtures().join("bad-end.7z"),"preview.png","CRC")] {
  let error=preview::preview_archive_image(&p,entry,None).await.unwrap_err();assert!(format!("{error:#}").contains(reason),"{error:#}");
 }
 let error=preview::preview_archive_text(&fixtures().join("at-limit.7z"),"at-limit.bmp",None).await.unwrap_err();assert!(error.to_string().contains("1 MiB"));
 let text=preview::preview_archive_text(&case("lzma2"),"readme.txt",None).await.unwrap();assert!(!text.truncated&&text.content.starts_with("7Z 图片"));
}

#[tokio::test]
async fn sevenz1618_selected_vault_header_volumes_and_progress_are_connected() {
 use crate::models::password::{PasswordEntry,PasswordCategory};
 let temp=tempfile::tempdir().unwrap();let mut vault=EncryptedPasswordService::new(temp.path());vault.initialize("owned-public-test-key").await.unwrap();
 for (name,key) in [("a wrong","wrong"),("b public",KEY)]{vault.add_password(PasswordEntry::new(name.into(),key.into(),PasswordCategory::Other)).await.unwrap();}
 let service=CompressionService::new(CompressionServiceConfig::default(),Arc::new(IOBufferPool::default()),Arc::new(RarSupportService::new()),Arc::new(UniversalCliEngine::new()),Arc::new(vault));
 let events=Arc::new(Mutex::new(vec![]));let observer=events.clone();let control=Control{observer:Arc::new(move|p|observer.lock().unwrap().push(p.phase)),..Default::default()};
 for name in ["contents","headers-volumes"] {
  let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();
  for member in c["members"].as_array().unwrap() {
   let p=fixtures().join(name).join(member["name"].as_str().unwrap());let revision=image_reader::revision(&p).unwrap();
   let value=service.preview_sevenz_image_with_vault(p.to_str().unwrap(),IMAGE,None,Some(revision),control.clone()).await.unwrap();
   assert_eq!(hex::encode(sha2::Sha256::digest(bytes(&value))),m["images"][0]["sha256"].as_str().unwrap());
  }
 }
 assert!(service.preview_sevenz_image_with_vault(case("contents").to_str().unwrap(),IMAGE,Some("wrong manual"),None,Control::default()).await.is_err());
 assert!(service.encrypted_password_service.list_passwords().await.unwrap().iter().all(|p|p.use_count==0));
 let phases=events.lock().unwrap();assert!(phases.contains(&"reading")&&phases.contains(&"decoding"));
}

#[tokio::test]
async fn sevenz1618_cancel_source_mutation_and_missing_member_cannot_publish_an_image() {
 let p=case("lzma2");let rev=image_reader::revision(&p).unwrap();let control=Control{flag:Arc::new(AtomicBool::new(true)),..Default::default()};
 assert!(image_reader::read(&p,IMAGE,None,None,None,&rev,control).await.unwrap_err().to_string().contains("CANCELLED"));
 let temp=tempfile::tempdir().unwrap();let changed=temp.path().join("changed.7z");std::fs::copy(&p,&changed).unwrap();let rev=image_reader::revision(&changed).unwrap();
 let once=Arc::new(AtomicBool::new(false));let mutated=changed.clone();let control=Control{observer:Arc::new(move|p|{if p.phase=="decoding"&&!once.swap(true,Ordering::AcqRel){use std::io::Write;std::fs::OpenOptions::new().append(true).open(&mutated).unwrap().write_all(&[0]).unwrap();}}),..Default::default()};
 let error=preview::preview_archive_image_bound_at(&changed,IMAGE,None,None,None,control,Some(rev)).await.unwrap_err();assert!(format!("{error:#}").contains("IMAGE_READER_CHANGED"));
 let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]=="headers-volumes").unwrap();let members:Vec<_>=c["members"].as_array().unwrap().iter().map(|f|{let n=f["name"].as_str().unwrap();let p=temp.path().join(n);std::fs::copy(fixtures().join("headers-volumes").join(n),&p).unwrap();p}).collect();
 let p=&members[1];let rev=image_reader::revision(p).unwrap();std::fs::remove_file(members.last().unwrap()).unwrap();
 assert!(image_reader::read(p,IMAGE,Some(KEY),None,None,&rev,Control::default()).await.is_err());
 assert!(preview::preview_archive_image(p,IMAGE,Some(KEY)).await.is_err());
}
