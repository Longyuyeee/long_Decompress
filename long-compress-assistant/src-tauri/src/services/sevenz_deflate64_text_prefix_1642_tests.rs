use std::{path::{Path,PathBuf},sync::{Arc,atomic::AtomicBool}};
use sha2::{Digest,Sha256};
use super::{archive_preview as preview,image_reader};
const SELECTED:&str="logs/session [1].txt";
fn root()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-deflate64-text-prefix-1642")}
fn manifest()->serde_json::Value{serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn case(name:&str)->PathBuf{let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();root().join(name).join(c["members"][0]["name"].as_str().unwrap())}
fn key()->String{manifest()["password"].as_str().unwrap().into()}
fn hash(s:&str)->String{hex::encode(Sha256::digest(s.as_bytes()))}
async fn error(p:&Path,entry:&str,password:Option<&str>)->String{match preview::preview_archive_text(p,entry,password).await{Err(e)=>format!("{e:#}"),Ok(v)=>panic!("unexpected successful {} bytes",v.byte_size)}}

#[tokio::test]
async fn sevenz1642_original_aes_methods_headers_and_every_numeric_member_read_exact_prefix(){
 let m=manifest();let password=key();for c in m["cases"].as_array().unwrap(){for member in c["members"].as_array().unwrap(){let p=root().join(c["name"].as_str().unwrap()).join(member["name"].as_str().unwrap());let v=preview::preview_archive_text(&p,SELECTED,c["encrypted"].as_bool().unwrap().then_some(password.as_str())).await.unwrap();assert!(v.truncated);assert_eq!(v.byte_size,m["renderedPrefixBytes"].as_u64().unwrap());assert_eq!(v.total_size,m["totalSize"].as_u64().unwrap());assert_eq!(hash(&v.content),m["renderedPrefixSha256"].as_str().unwrap());}}
 let exact=preview::preview_archive_text(&case("contents"),"exact.txt",Some(&key())).await.unwrap();assert!(!exact.truncated);assert_eq!(exact.byte_size,1048576);
 let over=preview::preview_archive_text(&case("contents"),"just-over.txt",Some(&key())).await.unwrap();assert!(over.truncated);assert_eq!(over.byte_size,1048576);assert_eq!(over.total_size,1048577);

 for e in m["boundaries"].as_array().unwrap(){let v=preview::preview_archive_text(&case("contents"),e["path"].as_str().unwrap(),Some(&key())).await.unwrap();assert!(v.truncated);assert_eq!(v.encoding,e["encoding"]);assert_eq!(v.byte_size,e["bytes"]);assert_eq!(hash(&v.content),e["sha256"]);assert!(!v.content.contains('�'));}
}
#[tokio::test]
async fn sevenz1642_vault_candidates_and_manual_wrong_key_keep_partial_out_of_success_stats(){
 use super::{compression_service::{CompressionService,CompressionServiceConfig},encrypted_password_service::EncryptedPasswordService,io_buffer_pool::IOBufferPool,rar_support::RarSupportService,universal_engine::UniversalCliEngine};use crate::models::password::{PasswordEntry,PasswordCategory};
 let temp=tempfile::tempdir().unwrap();let mut vault=EncryptedPasswordService::new(temp.path());vault.initialize("owned-public-key").await.unwrap();for (n,k) in [("a wrong","wrong".to_string()),("b public",key())]{vault.add_password(PasswordEntry::new(n.into(),k,PasswordCategory::Other)).await.unwrap();}
 let service=CompressionService::new(CompressionServiceConfig::default(),Arc::new(IOBufferPool::default()),Arc::new(RarSupportService::new()),Arc::new(UniversalCliEngine::new()),Arc::new(vault));
 for name in ["contents","headers","volumes"]{let p=case(name);let v=service.preview_sevenz_text_with_vault(p.to_str().unwrap(),SELECTED,None,None).await.unwrap();assert_eq!(hash(&v.content),manifest()["renderedPrefixSha256"]);assert!(v.truncated);assert!(service.preview_sevenz_text_with_vault(p.to_str().unwrap(),SELECTED,Some("wrong manual"),None).await.is_err());}
 assert!(service.encrypted_password_service.list_passwords().await.unwrap().iter().all(|p|p.use_count==0));
}
#[tokio::test]
async fn sevenz1642_partial_tail_is_not_full_integrity_and_full_small_reads_stay_strict(){
 for(name,k)in[("tail-damaged.7z",Some(key())),("plain-tail-damaged.7z",None)]{
  let p=root().join(name);let v=preview::preview_archive_text(&p,SELECTED,k.as_deref()).await.unwrap();assert!(v.truncated);assert_eq!(hash(&v.content),manifest()["renderedPrefixSha256"]);
  let mut input=std::fs::File::open(&p).unwrap();let len=input.metadata().unwrap().len();let password=k.as_deref().map(sevenz_rust::Password::from).unwrap_or_default();let a=sevenz_rust::Archive::read(&mut input,len,password.as_slice()).unwrap();let i=a.files.iter().position(|e|e.name==SELECTED).unwrap();let f=a.stream_map.file_folder_index[i].unwrap();assert!(sevenz_rust::BlockDecoder::new(f,&a,password.as_slice(),&mut input).read_deflate64_entry(i,4*1024*1024,||Ok(())).is_err());
 }
 let password=sevenz_rust::Password::from(key().as_str());
 let p=case("contents");let v=preview::preview_archive_text(&p,"readme.txt",Some(&key())).await.unwrap();assert!(!v.truncated);assert_eq!(v.content,"完整小文本仍需完整读取。");
 let mut input=std::fs::File::open(&p).unwrap();let len=input.metadata().unwrap().len();let mut a=sevenz_rust::Archive::read(&mut input,len,password.as_slice()).unwrap();let i=a.files.iter().position(|e|e.name=="readme.txt").unwrap();a.files[i].crc^=1;let f=a.stream_map.file_folder_index[i].unwrap();assert!(sevenz_rust::BlockDecoder::new(f,&a,password.as_slice(),&mut input).read_deflate64_entry(i,1024*1024,||Ok(())).is_err());
 for (name,entry,reason) in [("bad-start.7z",SELECTED,"password"),("store.7z",SELECTED,"方法组合")]{assert!(error(&root().join(name),entry,Some(&key())).await.contains(reason),"{name}");}

 assert!(error(&p,"logs/binary.txt",Some(&key())).await.contains("binary"));assert!(error(&p,SELECTED,Some("wrong")).await.contains("password"));
}
#[tokio::test]
async fn sevenz1642_cancel_source_revision_missing_tail_and_large_total_are_bounded(){
 let temp=tempfile::tempdir().unwrap();let p=temp.path().join("changed.7z");std::fs::copy(case("contents"),&p).unwrap();let rev=image_reader::revision(&p).unwrap();let e=preview::preview_archive_text_controlled_at(&p,SELECTED,Some(&key()),None,None,Arc::new(AtomicBool::new(true)),Some(rev.clone())).await.err().unwrap();assert!(format!("{e:#}").contains("取消"));
 std::fs::OpenOptions::new().append(true).open(&p).unwrap().write_all(b"x").unwrap();let e=preview::preview_archive_text_controlled_at(&p,SELECTED,Some(&key()),None,None,Arc::new(AtomicBool::new(false)),Some(rev)).await.err().unwrap();assert!(format!("{e:#}").contains("IMAGE_READER_CHANGED"));
 let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]=="volumes").unwrap();let files:Vec<_>=c["members"].as_array().unwrap().iter().map(|f|{let n=f["name"].as_str().unwrap();let p=temp.path().join(n);std::fs::copy(root().join("volumes").join(n),&p).unwrap();p}).collect();assert!(files.len()>2);std::fs::remove_file(files.last().unwrap()).unwrap();assert!(preview::preview_archive_text(&files[1],SELECTED,Some(&key())).await.is_err());

 let v=preview::preview_archive_text(&root().join("large-total.7z"),"huge.txt",Some(&key())).await.unwrap();assert_eq!(v.total_size,65*1048576);assert_eq!(v.byte_size,1048576);
}
use std::io::Write;

#[test]
fn sevenz1642_prefix_mapping_parameters_mid_read_cancel_and_unverified_crc_are_explicit(){
 fn archive()->(std::fs::File,sevenz_rust::Archive,usize,usize){let mut input=std::fs::File::open(case("contents")).unwrap();let len=input.metadata().unwrap().len();let a=sevenz_rust::Archive::read(&mut input,len,sevenz_rust::Password::from(key().as_str()).as_slice()).unwrap();let i=a.files.iter().position(|e|e.name==SELECTED).unwrap();let f=a.stream_map.file_folder_index[i].unwrap();(input,a,i,f)}
 for which in ["props","mapping","declared","shared","aes-size","power","aes-props","packed-map","cap"]{
  let(mut input,mut a,i,f)=archive();let ppmd=a.folders[f].coders.iter().position(|c|c.decompression_method_id()==sevenz_rust::SevenZMethod::ID_DEFLATE64).unwrap();let aes=a.folders[f].coders.iter().position(|c|c.decompression_method_id()==sevenz_rust::SevenZMethod::ID_AES256SHA256).unwrap();
  match which{"props"=>a.folders[f].coders[ppmd].properties.push(1),"mapping"=>a.stream_map.file_folder_index[i]=None,"declared"=>a.files[i].size-=1,"shared"=>a.folders[f].num_unpack_sub_streams=2,"aes-size"=>a.folders[f].unpack_sizes[aes]+=16,"power"=>a.folders[f].coders[aes].properties[0]=(a.folders[f].coders[aes].properties[0]&0xc0)|21,"aes-props"=>{a.folders[f].coders[aes].properties.pop();},"packed-map"=>a.folders[f].packed_streams[0]=ppmd as u64,_=>()}
  assert!(sevenz_rust::with_preview_limits::<_,sevenz_rust::Error>(64*1024*1024,20,||Ok(()),||sevenz_rust::BlockDecoder::new(f,&a,sevenz_rust::Password::from(key().as_str()).as_slice(),&mut input).read_deflate64_entry_prefix(i,if which=="cap"{1048577}else{1048576},||Ok(()))).is_err(),"{which}");
 }
 let(mut input,mut a,i,f)=archive();a.files[i].crc^=1;a.folders[f].has_crc=true;a.folders[f].crc=0;
 let bytes=sevenz_rust::with_preview_limits::<_,sevenz_rust::Error>(64*1024*1024,20,||Ok(()),||sevenz_rust::BlockDecoder::new(f,&a,sevenz_rust::Password::from(key().as_str()).as_slice(),&mut input).read_deflate64_entry_prefix(i,1048576,||Ok(()))).unwrap();let rendered=std::str::from_utf8(&bytes[..manifest()["renderedPrefixBytes"].as_u64().unwrap() as usize]).unwrap();assert_eq!(hash(rendered),manifest()["renderedPrefixSha256"]);
 let(mut input,a,i,f)=archive();let mut n=0;let e=sevenz_rust::BlockDecoder::new(f,&a,sevenz_rust::Password::from(key().as_str()).as_slice(),&mut input).read_deflate64_entry_prefix(i,1048576,||{n+=1;if n==3{Err(sevenz_rust::Error::other("预览已取消"))}else{Ok(())}}).unwrap_err();assert!(e.to_string().contains("取消"));
}

#[tokio::test]
async fn sevenz1642_complete_image_and_shared_text_dispatch_stay_on_full_paths(){
 use base64::Engine;
 let fixtures=Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives");
 for name in ["sevenz-password-deflate64-preview-1639","sevenz-password-solid-deflate64-preview-1641"]{
  let old=fixtures.join(name);let m:serde_json::Value=serde_json::from_slice(&std::fs::read(old.join("manifest.json")).unwrap()).unwrap();let c=&m["cases"][0];let p=old.join(c["name"].as_str().unwrap()).join(c["members"][0]["name"].as_str().unwrap());let key=m["password"].as_str();
  let text=preview::preview_archive_text(&p,"notes/说明 [1].txt",key).await.unwrap();assert!(!text.truncated);assert_eq!(hash(&text.content),m["textSha256"]);
  let im=preview::preview_archive_image(&p,"gallery/01-cover.png",key).await.unwrap();let bytes=base64::engine::general_purpose::STANDARD.decode(im.data_url.split(',').nth(1).unwrap()).unwrap();assert_eq!(hex::encode(Sha256::digest(bytes)),m["images"][0]["sha256"]);
 }
}
