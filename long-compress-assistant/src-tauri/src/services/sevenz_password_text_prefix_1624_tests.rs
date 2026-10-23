use std::{path::{Path,PathBuf},sync::{Arc,atomic::AtomicBool}};
use sha2::{Digest,Sha256};
use super::{archive_preview as preview,image_reader};
const SELECTED:&str="logs/session [1].txt";
fn root()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-password-text-prefix-1624")}
fn manifest()->serde_json::Value{serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn case(name:&str)->PathBuf{let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();root().join(name).join(c["members"][0]["name"].as_str().unwrap())}
fn key()->String{manifest()["password"].as_str().unwrap().into()}
fn hash(s:&str)->String{hex::encode(Sha256::digest(s.as_bytes()))}
async fn error(p:&Path,entry:&str,password:Option<&str>)->String{match preview::preview_archive_text(p,entry,password).await{Err(e)=>format!("{e:#}"),Ok(v)=>panic!("unexpected successful {} bytes",v.byte_size)}}

#[tokio::test]
async fn sevenz1624_original_aes_methods_headers_and_every_numeric_member_read_exact_prefix(){
 let m=manifest();for c in m["cases"].as_array().unwrap(){for member in c["members"].as_array().unwrap(){let p=root().join(c["name"].as_str().unwrap()).join(member["name"].as_str().unwrap());let v=preview::preview_archive_text(&p,SELECTED,Some(&key())).await.unwrap();assert!(v.truncated);assert_eq!(v.byte_size,m["renderedPrefixBytes"].as_u64().unwrap());assert_eq!(v.total_size,m["totalSize"].as_u64().unwrap());assert_eq!(hash(&v.content),m["renderedPrefixSha256"].as_str().unwrap());}}
 for e in m["boundaries"].as_array().unwrap(){let v=preview::preview_archive_text(&case("lzma2"),e["path"].as_str().unwrap(),Some(&key())).await.unwrap();assert!(v.truncated);assert_eq!(v.encoding,e["encoding"]);assert_eq!(v.byte_size,e["bytes"]);assert_eq!(hash(&v.content),e["sha256"]);assert!(!v.content.contains('�'));}
}
#[tokio::test]
async fn sevenz1624_vault_candidates_and_manual_wrong_key_keep_partial_out_of_success_stats(){
 use super::{compression_service::{CompressionService,CompressionServiceConfig},encrypted_password_service::EncryptedPasswordService,io_buffer_pool::IOBufferPool,rar_support::RarSupportService,universal_engine::UniversalCliEngine};use crate::models::password::{PasswordEntry,PasswordCategory};
 let temp=tempfile::tempdir().unwrap();let mut vault=EncryptedPasswordService::new(temp.path());vault.initialize("owned-public-key").await.unwrap();for (n,k) in [("a wrong","wrong".to_string()),("b public",key())]{vault.add_password(PasswordEntry::new(n.into(),k,PasswordCategory::Other)).await.unwrap();}
 let service=CompressionService::new(CompressionServiceConfig::default(),Arc::new(IOBufferPool::default()),Arc::new(RarSupportService::new()),Arc::new(UniversalCliEngine::new()),Arc::new(vault));
 for name in ["lzma","headers","volumes"]{let p=case(name);let v=service.preview_sevenz_text_with_vault(p.to_str().unwrap(),SELECTED,None,None).await.unwrap();assert_eq!(hash(&v.content),manifest()["renderedPrefixSha256"]);assert!(v.truncated);assert!(service.preview_sevenz_text_with_vault(p.to_str().unwrap(),SELECTED,Some("wrong manual"),None).await.is_err());}
 assert!(service.encrypted_password_service.list_passwords().await.unwrap().iter().all(|p|p.use_count==0));
}
#[tokio::test]
async fn sevenz1624_partial_tail_is_not_full_integrity_and_full_small_reads_stay_strict(){
 let p=root().join("tail-damaged.7z");let v=preview::preview_archive_text(&p,SELECTED,Some(&key())).await.unwrap();assert!(v.truncated);assert_eq!(hash(&v.content),manifest()["renderedPrefixSha256"]);
 let mut input=std::fs::File::open(&p).unwrap();let len=input.metadata().unwrap().len();let password=sevenz_rust::Password::from(key().as_str());let a=sevenz_rust::Archive::read(&mut input,len,password.as_slice()).unwrap();assert!(sevenz_rust::BlockDecoder::new(0,&a,password.as_slice(),&mut input).read_single_entry(4*1024*1024).is_err());
 let p=case("lzma2");let v=preview::preview_archive_text(&p,"readme.txt",Some(&key())).await.unwrap();assert!(!v.truncated);assert_eq!(v.content,"完整小文本仍需完整读取。");
 let mut input=std::fs::File::open(&p).unwrap();let len=input.metadata().unwrap().len();let mut a=sevenz_rust::Archive::read(&mut input,len,password.as_slice()).unwrap();let i=a.files.iter().position(|e|e.name=="readme.txt").unwrap();a.files[i].crc^=1;let f=a.stream_map.file_folder_index[i].unwrap();assert!(sevenz_rust::BlockDecoder::new(f,&a,password.as_slice(),&mut input).read_single_entry(1024*1024).is_err());
 for (name,entry,reason) in [("bad-start.7z",SELECTED,"password"),("store.7z",SELECTED,"方法组合")]{assert!(error(&root().join(name),entry,Some(&key())).await.contains(reason),"{name}");}
 assert!(error(&p,"logs/binary.txt",Some(&key())).await.contains("binary"));assert!(error(&p,SELECTED,Some("wrong")).await.contains("password"));
}
#[tokio::test]
async fn sevenz1624_cancel_source_revision_missing_tail_and_large_unrelated_are_bounded(){
 let temp=tempfile::tempdir().unwrap();let p=temp.path().join("changed.7z");std::fs::copy(case("lzma2"),&p).unwrap();let rev=image_reader::revision(&p).unwrap();let e=preview::preview_archive_text_controlled_at(&p,SELECTED,Some(&key()),None,None,Arc::new(AtomicBool::new(true)),Some(rev.clone())).await.err().unwrap();assert!(format!("{e:#}").contains("取消"));
 std::fs::OpenOptions::new().append(true).open(&p).unwrap().write_all(b"x").unwrap();let e=preview::preview_archive_text_controlled_at(&p,SELECTED,Some(&key()),None,None,Arc::new(AtomicBool::new(false)),Some(rev)).await.err().unwrap();assert!(format!("{e:#}").contains("IMAGE_READER_CHANGED"));
 let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]=="volumes").unwrap();let files:Vec<_>=c["members"].as_array().unwrap().iter().map(|f|{let n=f["name"].as_str().unwrap();let p=temp.path().join(n);std::fs::copy(root().join("volumes").join(n),&p).unwrap();p}).collect();assert!(files.len()>2);std::fs::remove_file(files.last().unwrap()).unwrap();assert!(preview::preview_archive_text(&files[1],SELECTED,Some(&key())).await.is_err());
 let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/sevenz-password-text-prefix-1624/unrelated.7z");assert!(p.metadata().unwrap().len()>64*1024*1024);let v=preview::preview_archive_text(&p,SELECTED,Some(&key())).await.unwrap();assert_eq!(hash(&v.content),m["renderedPrefixSha256"]);
 let v=preview::preview_archive_text(&root().join("large-total.7z"),"huge.txt",Some(&key())).await.unwrap();assert_eq!(v.total_size,65*1048576);assert_eq!(v.byte_size,1048576);
}
use std::io::Write;
