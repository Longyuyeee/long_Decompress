use super::{archive_preview as preview,image_reader,compression_service::{CompressionService,CompressionServiceConfig},encrypted_password_service::EncryptedPasswordService,io_buffer_pool::IOBufferPool,rar_support::RarSupportService,universal_engine::UniversalCliEngine};
use std::{path::{Path,PathBuf},sync::{Arc,atomic::AtomicBool}};
use sha2::Digest;
const TEXT:&str="docs/02-说明 [1].txt";
const KEY:&str="7Z加密固实1621!";
fn root()->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-encrypted-solid-text-1621")}
fn manifest()->serde_json::Value {serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn case(name:&str)->PathBuf {let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();root().join(name).join(c["members"][0]["name"].as_str().unwrap())}
fn hash(text:&str)->String {hex::encode(sha2::Sha256::digest(text.as_bytes()))}

#[tokio::test]
async fn sevenz1621_original_content_header_methods_and_numeric_members_show_complete_text(){
 let m=manifest();
 for c in m["cases"].as_array().unwrap(){for member in c["members"].as_array().unwrap(){
  let p=root().join(c["name"].as_str().unwrap()).join(member["name"].as_str().unwrap());let v=preview::preview_archive_text(&p,TEXT,Some(KEY)).await.unwrap();assert!(!v.truncated);assert_eq!(v.byte_size,v.total_size);assert_eq!(hash(&v.content),m["textSha256"].as_str().unwrap());
 }}
 for (entry,text) in [("docs/empty.txt",""),("docs/02-说明 1.txt","NOT the selected bracket path")]{assert_eq!(preview::preview_archive_text(&case("headers"),entry,Some(KEY)).await.unwrap().content,text);}
}

#[tokio::test]
async fn sevenz1621_vault_metadata_and_selected_text_never_record_whole_archive_success(){
 use crate::models::password::{PasswordEntry,PasswordCategory};
 let temp=tempfile::tempdir().unwrap();let mut vault=EncryptedPasswordService::new(temp.path());vault.initialize("owned-public-test").await.unwrap();
 for (name,key) in [("a wrong","wrong"),("b public",KEY)]{vault.add_password(PasswordEntry::new(name.into(),key.into(),PasswordCategory::Other)).await.unwrap();}
 let service=CompressionService::new(CompressionServiceConfig::default(),Arc::new(IOBufferPool::default()),Arc::new(RarSupportService::new()),Arc::new(UniversalCliEngine::new()),Arc::new(vault));
 for name in ["content-lzma2","headers"]{let p=case(name);let listing=service.browse_sevenz_with_vault(p.to_str().unwrap(),None).await.unwrap();assert!(listing.entries.iter().find(|e|e.path==TEXT).unwrap().compressed_size.is_none());let v=service.preview_sevenz_text_with_vault(p.to_str().unwrap(),TEXT,None,None).await.unwrap();assert_eq!(hash(&v.content),manifest()["textSha256"].as_str().unwrap());assert!(service.preview_sevenz_text_with_vault(p.to_str().unwrap(),TEXT,Some("wrong manual"),None).await.is_err());}
 assert!(service.browse_sevenz_with_vault(case("headers").to_str().unwrap(),Some("wrong manual")).await.is_err());
 assert!(service.encrypted_password_service.list_passwords().await.unwrap().iter().all(|e|e.use_count==0));
}

#[tokio::test]
async fn sevenz1621_corrupt_unrelated_large_block_is_skipped_by_metadata_and_text(){
 let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/sevenz-encrypted-solid-text-1621/unrelated.7z");assert!(p.metadata().unwrap().len()>64*1024*1024);
 let listing=super::archive_browser::browse_archive_cancellable(&p,None,Arc::new(AtomicBool::new(false))).await.unwrap();assert!(listing.entries.iter().any(|e|e.path==TEXT));
 assert_eq!(hash(&preview::preview_archive_text(&p,TEXT,Some(KEY)).await.unwrap().content),manifest()["textSha256"].as_str().unwrap());
}

#[tokio::test]
async fn sevenz1621_wrong_password_damage_limits_and_encrypted_images_keep_distinct_next_steps(){
 for key in [None,Some("wrong"),Some(KEY)]{let e=preview::preview_archive_text(&root().join("damaged-data.7z"),TEXT,key).await.unwrap_err();assert!(format!("{e:#}").contains("sevenz-preview:password"),"{e:#}");}
 let e=preview::preview_archive_text(&root().join("oversize-shared.7z"),"readme.txt",Some(KEY)).await.unwrap_err();assert!(format!("{e:#}").contains("64 MiB"),"{e:#}");assert!(!format!("{e:#}").contains("sevenz-preview:password"));
 let image=Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-solid-image-1620/encrypted/相册.7z");assert!(preview::preview_archive_image(&image,"gallery/01-cover.png",Some("7Z固实图片1620!")).await.unwrap_err().to_string().contains("加密固实图片"));
 // Existing non-solid encrypted data now uses the same opt-in AES completion path.
 let previous=Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-text-1617/contents/资料.7z");assert!(!preview::preview_archive_text(&previous,"notes/说明 [1].txt",Some("7Z预览密码1617!")).await.unwrap().truncated);
}

#[tokio::test]
async fn sevenz1621_source_revision_cancel_and_missing_encrypted_volume_do_not_publish_text(){
 let p=case("headers");let revision=image_reader::revision(&p).unwrap();let e=preview::preview_archive_text_controlled_at(&p,TEXT,Some(KEY),None,None,Arc::new(AtomicBool::new(true)),Some(revision)).await.unwrap_err();assert!(e.to_string().contains("取消"));
 let temp=tempfile::tempdir().unwrap();let changed=temp.path().join("changed.7z");std::fs::copy(&p,&changed).unwrap();let revision=image_reader::revision(&changed).unwrap();use std::io::Write;std::fs::OpenOptions::new().append(true).open(&changed).unwrap().write_all(&[0]).unwrap();let e=preview::preview_archive_text_controlled_at(&changed,TEXT,Some(KEY),None,None,Arc::new(AtomicBool::new(false)),Some(revision)).await.unwrap_err();assert!(format!("{e:#}").contains("IMAGE_READER_CHANGED"));
 let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]=="headers-volumes").unwrap();let members:Vec<_>=c["members"].as_array().unwrap().iter().map(|f|{let n=f["name"].as_str().unwrap();let p=temp.path().join(n);std::fs::copy(root().join("headers-volumes").join(n),&p).unwrap();p}).collect();assert!(members.len()>2);let rev=image_reader::revision(&members[1]).unwrap();std::fs::remove_file(members.last().unwrap()).unwrap();let e=preview::preview_archive_text_controlled_at(&members[1],TEXT,Some(KEY),None,None,Arc::new(AtomicBool::new(false)),Some(rev)).await.unwrap_err();assert!(format!("{e:#}").contains("IMAGE_READER_CHANGED"));assert!(preview::preview_archive_text(&members[1],TEXT,Some(KEY)).await.is_err());
}
