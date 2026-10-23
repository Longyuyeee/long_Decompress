use super::{archive_preview as preview, image_reader::{self,Control}, compression_service::{CompressionService,CompressionServiceConfig}, encrypted_password_service::EncryptedPasswordService, io_buffer_pool::IOBufferPool, rar_support::RarSupportService, universal_engine::UniversalCliEngine};
use std::{path::{Path,PathBuf}, sync::{Arc,atomic::{AtomicBool,Ordering}}};
use base64::Engine;
use sha2::Digest;
const IMAGE:&str="gallery/01-cover.png";
const KEY:&str="7Z加密相册1622!";
fn root()->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-encrypted-solid-image-1622")}
fn manifest()->serde_json::Value {serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn case(name:&str)->PathBuf {let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();root().join(name).join(c["members"][0]["name"].as_str().unwrap())}
fn hash(p:&preview::ArchiveImagePreview)->String {hex::encode(sha2::Sha256::digest(base64::engine::general_purpose::STANDARD.decode(p.data_url.split(',').nth(1).unwrap()).unwrap()))}

#[tokio::test]
async fn sevenz1622_original_encrypted_methods_rasters_headers_and_every_numeric_member_are_exact(){
 let m=manifest();
 for name in ["content-lzma","content-lzma2","headers"]{
  let p=case(name);let rev=image_reader::revision(&p).unwrap();
  for e in m["images"].as_array().unwrap(){
   let value=image_reader::read(&p,e["path"].as_str().unwrap(),Some(KEY),None,None,&rev,Control::default()).await.unwrap();
   assert_eq!(hash(&value),e["sha256"].as_str().unwrap());assert_eq!((value.width,value.height),(e["width"].as_u64().unwrap() as u32,e["height"].as_u64().unwrap() as u32));
  }
 }
 let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]=="headers-volumes").unwrap();
 for member in c["members"].as_array().unwrap(){let p=root().join("headers-volumes").join(member["name"].as_str().unwrap());assert_eq!(hash(&preview::preview_archive_image(&p,IMAGE,Some(KEY)).await.unwrap()),m["images"][0]["sha256"].as_str().unwrap());}
}

#[tokio::test]
async fn sevenz1622_selected_vault_image_and_metadata_never_validate_unrelated_data_or_record_success(){
 use crate::models::password::{PasswordEntry,PasswordCategory};
 let temp=tempfile::tempdir().unwrap();let mut vault=EncryptedPasswordService::new(temp.path());vault.initialize("owned-public-image-test").await.unwrap();
 for (name,key) in [("a wrong","wrong"),("b public",KEY)]{vault.add_password(PasswordEntry::new(name.into(),key.into(),PasswordCategory::Other)).await.unwrap();}
 let service=CompressionService::new(CompressionServiceConfig::default(),Arc::new(IOBufferPool::default()),Arc::new(RarSupportService::new()),Arc::new(UniversalCliEngine::new()),Arc::new(vault));
 for name in ["content-lzma2","headers"]{
  let p=case(name);let listing=service.browse_sevenz_with_vault(p.to_str().unwrap(),None).await.unwrap();assert!(listing.entries.iter().find(|e|e.path==IMAGE).unwrap().compressed_size.is_none());
  let value=service.preview_sevenz_image_with_vault(p.to_str().unwrap(),IMAGE,None,Some(image_reader::revision(&p).unwrap()),Control::default()).await.unwrap();assert_eq!(hash(&value),manifest()["images"][0]["sha256"].as_str().unwrap());
  assert!(service.preview_sevenz_image_with_vault(p.to_str().unwrap(),IMAGE,Some("wrong manual"),None,Control::default()).await.is_err());
 }
 assert!(service.encrypted_password_service.list_passwords().await.unwrap().iter().all(|e|e.use_count==0));
}

#[tokio::test]
async fn sevenz1622_encrypted_complete_image_limits_and_damage_never_publish_partial_pixels(){
 let m=manifest();let at=preview::preview_archive_image(&root().join("at-limit.7z"),"at-limit.bmp",Some(KEY)).await.unwrap();assert_eq!(at.byte_size,8*1024*1024);assert_eq!(hash(&at),m["limits"]["at-limit.bmp"]["sha256"].as_str().unwrap());
 for (name,entry,reason) in [("oversize.7z","oversize.bmp","8 MiB"),("oversize-shared.7z","preview.png","64 MiB"),("dimensions.7z","dimensions.png","dimensions exceed")]{
  let e=preview::preview_archive_image(&root().join(name),entry,Some(KEY)).await.unwrap_err();assert!(format!("{e:#}").contains(reason),"{e:#}");assert!(!format!("{e:#}").contains("sevenz-preview:password"));
 }
 for (p,key) in [(case("content-lzma2"),Some("wrong")),(root().join("damaged-data.7z"),Some(KEY))]{let e=preview::preview_archive_image(&p,IMAGE,key).await.unwrap_err();assert!(format!("{e:#}").contains("sevenz-preview:password"),"{e:#}");}
 assert!(preview::preview_archive_image(&case("headers"),"gallery/03-bad.png",Some(KEY)).await.unwrap_err().to_string().contains("not a supported raster"));
}

#[tokio::test]
async fn sevenz1622_encrypted_image_mid_read_cancel_changed_source_and_missing_volume_are_discarded(){
 let p=case("headers");let rev=image_reader::revision(&p).unwrap();let flag=Arc::new(AtomicBool::new(false));let signal=flag.clone();
 let control=Control{flag,observer:Arc::new(move|e|{if e.phase=="reading"&&e.processed_bytes>=4096{signal.store(true,Ordering::Release)}})};
 let e=image_reader::read(&p,IMAGE,Some(KEY),None,None,&rev,control).await.unwrap_err();assert!(format!("{e:#}").contains("取消")||format!("{e:#}").contains("CANCELLED"));
 assert_eq!(image_reader::read(&p,IMAGE,Some(KEY),None,None,&rev,Control::default()).await.unwrap().width,320);
 let temp=tempfile::tempdir().unwrap();let changed=temp.path().join("changed.7z");std::fs::copy(&p,&changed).unwrap();let rev=image_reader::revision(&changed).unwrap();let target=changed.clone();
 let control=Control{observer:Arc::new(move|e|{if e.phase=="decoding"{use std::io::Write;std::fs::OpenOptions::new().append(true).open(&target).unwrap().write_all(&[0]).unwrap();}}),..Default::default()};
 let e=preview::preview_archive_image_bound_at(&changed,IMAGE,Some(KEY),None,None,control,Some(rev)).await.unwrap_err();assert!(format!("{e:#}").contains("IMAGE_READER_CHANGED"));
 let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]=="headers-volumes").unwrap();let members:Vec<_>=c["members"].as_array().unwrap().iter().map(|f|{let n=f["name"].as_str().unwrap();let p=temp.path().join(n);std::fs::copy(root().join("headers-volumes").join(n),&p).unwrap();p}).collect();
 let rev=image_reader::revision(&members[1]).unwrap();std::fs::remove_file(members.last().unwrap()).unwrap();let e=image_reader::read(&members[1],IMAGE,Some(KEY),None,None,&rev,Control::default()).await.unwrap_err();assert!(format!("{e:#}").contains("IMAGE_READER_CHANGED"));assert!(preview::preview_archive_image(&members[1],IMAGE,Some(KEY)).await.is_err());
}

#[tokio::test]
async fn sevenz1622_encrypted_selected_image_skips_corrupt_unrelated_65m_block(){
 let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/sevenz-encrypted-solid-image-1622/unrelated.7z");assert!(p.metadata().unwrap().len()>64*1024*1024);
 let listing=super::archive_browser::browse_archive_cancellable(&p,None,Arc::new(AtomicBool::new(false))).await.unwrap();assert!(listing.entries.iter().any(|e|e.path==IMAGE));
 assert_eq!(hash(&preview::preview_archive_image(&p,IMAGE,Some(KEY)).await.unwrap()),manifest()["images"][0]["sha256"].as_str().unwrap());
}
