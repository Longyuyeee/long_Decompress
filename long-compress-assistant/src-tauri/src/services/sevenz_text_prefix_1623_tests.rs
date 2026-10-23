use super::{archive_preview as preview,image_reader};
use std::{path::{Path,PathBuf},sync::{Arc,atomic::AtomicBool}};
use sha2::Digest;
const SELECTED:&str="logs/session [1].txt";
fn root()->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-text-prefix-1623")}
fn manifest()->serde_json::Value {serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn case(name:&str)->PathBuf {let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();root().join(name).join(c["members"][0]["name"].as_str().unwrap())}
fn hash(s:&str)->String {hex::encode(sha2::Sha256::digest(s.as_bytes()))}

#[tokio::test]
async fn sevenz1623_complete_integrity_rejects_lzma2_bad_terminal_and_keeps_valid_aes_shared_images(){
 use super::universal_engine::UniversalCliEngine;
 UniversalCliEngine::test_integrity(&case("lzma2"),None).await.unwrap();
 let error=UniversalCliEngine::test_integrity(&root().join("tail-damaged.7z"),None).await.unwrap_err();assert!(!error.to_string().is_empty());
 let encrypted=Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-encrypted-solid-image-1622/headers/相册.7z");
 UniversalCliEngine::test_integrity(&encrypted,Some("7Z加密相册1622!")).await.unwrap();
 assert_eq!(preview::preview_archive_image(&encrypted,"gallery/01-cover.png",Some("7Z加密相册1622!")).await.unwrap().width,320);
}

#[tokio::test]
async fn sevenz1623_original_methods_and_each_numeric_member_read_only_the_exact_prefix(){
 let m=manifest();
 for c in m["cases"].as_array().unwrap(){for member in c["members"].as_array().unwrap(){
  let p=root().join(c["name"].as_str().unwrap()).join(member["name"].as_str().unwrap());let v=preview::preview_archive_text(&p,SELECTED,None).await.unwrap();
  assert!(v.truncated);assert_eq!(v.byte_size,m["renderedPrefixBytes"].as_u64().unwrap());assert_eq!(v.total_size,m["totalSize"].as_u64().unwrap());assert_eq!(hash(&v.content),m["renderedPrefixSha256"].as_str().unwrap());
 }}
 let v=preview::preview_archive_text(&root().join("large-total.7z"),"huge.txt",None).await.unwrap();assert!(v.truncated);assert_eq!(v.total_size,65*1024*1024);assert_eq!(v.byte_size,1024*1024);assert_eq!(v.content,"L".repeat(1024*1024));
}

#[tokio::test]
async fn sevenz1623_character_boundaries_keep_original_encoding_without_replacement(){
 for e in manifest()["boundaries"].as_array().unwrap(){
  let v=preview::preview_archive_text(&case("lzma2"),e["path"].as_str().unwrap(),None).await.unwrap();assert!(v.truncated);assert_eq!(v.encoding,e["encoding"].as_str().unwrap());assert_eq!(v.byte_size,e["bytes"].as_u64().unwrap());assert_eq!(hash(&v.content),e["sha256"].as_str().unwrap());assert!(!v.content.contains('�'));
 }
 assert!(preview::preview_archive_text(&case("lzma2"),"logs/binary.txt",None).await.unwrap_err().to_string().contains("binary"));
}

#[tokio::test]
async fn sevenz1623_unread_damaged_tail_is_explicitly_partial_and_complete_paths_stay_strict(){
 let p=root().join("tail-damaged.7z");let v=preview::preview_archive_text(&p,SELECTED,None).await.unwrap();assert!(v.truncated);assert_eq!(hash(&v.content),manifest()["renderedPrefixSha256"].as_str().unwrap());
 let mut input=std::fs::File::open(&p).unwrap();let len=input.metadata().unwrap().len();let a=sevenz_rust::Archive::read(&mut input,len,&[]).unwrap();let i=a.files.iter().position(|e|e.name==SELECTED).unwrap();let b=a.stream_map.file_folder_index[i].unwrap();assert!(sevenz_rust::BlockDecoder::new(b,&a,&[],&mut input).read_single_entry(4*1024*1024).is_err());
 for (name,key,reason) in [("solid.7z",None,"固实大文本"),("store.7z",None,"方法组合"),("bad-start.7z",None,"CRC")]{let e=match preview::preview_archive_text(&root().join(name),SELECTED,key).await{Err(e)=>e,Ok(v)=>panic!("{name} unexpectedly returned {} bytes",v.byte_size)};assert!(format!("{e:#}").contains(reason),"{name}: {e:#}");}
 for (entry,size) in [("readme.txt",0),("exact.txt",1024*1024)]{let v=preview::preview_archive_text(&case("lzma2"),entry,None).await.unwrap();assert!(!v.truncated);if size!=0{assert_eq!(v.byte_size,size)}}
 assert_eq!(preview::preview_archive_text(&case("lzma2"),"logs/session 1.txt",None).await.unwrap().content,"NOT the selected bracket path");
}

#[tokio::test]
async fn sevenz1623_cancel_changed_and_missing_group_discard_prefix_before_fresh_request(){
 let p=case("lzma2");let rev=image_reader::revision(&p).unwrap();let e=preview::preview_archive_text_controlled_at(&p,SELECTED,None,None,None,Arc::new(AtomicBool::new(true)),Some(rev)).await.unwrap_err();assert!(e.to_string().contains("取消"));
 let temp=tempfile::tempdir().unwrap();let changed=temp.path().join("changed.7z");std::fs::copy(&p,&changed).unwrap();let rev=image_reader::revision(&changed).unwrap();use std::io::Write;std::fs::OpenOptions::new().append(true).open(&changed).unwrap().write_all(&[0]).unwrap();let e=preview::preview_archive_text_controlled_at(&changed,SELECTED,None,None,None,Arc::new(AtomicBool::new(false)),Some(rev)).await.unwrap_err();assert!(format!("{e:#}").contains("IMAGE_READER_CHANGED"));
 let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]=="volumes").unwrap();let members:Vec<_>=c["members"].as_array().unwrap().iter().map(|f|{let n=f["name"].as_str().unwrap();let p=temp.path().join(n);std::fs::copy(root().join("volumes").join(n),&p).unwrap();p}).collect();assert!(members.len()>2);let rev=image_reader::revision(&members[1]).unwrap();std::fs::remove_file(members.last().unwrap()).unwrap();let e=preview::preview_archive_text_controlled_at(&members[1],SELECTED,None,None,None,Arc::new(AtomicBool::new(false)),Some(rev)).await.unwrap_err();assert!(format!("{e:#}").contains("IMAGE_READER_CHANGED"));assert!(preview::preview_archive_text(&members[1],SELECTED,None).await.is_err());
 assert!(preview::preview_archive_text(&p,SELECTED,None).await.unwrap().truncated);
}

#[tokio::test]
async fn sevenz1623_prefix_skips_unrelated_corrupt_data_and_stops_on_short_output_or_cancellation(){
 let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/sevenz-text-prefix-1623/unrelated.7z");assert!(p.metadata().unwrap().len()>64*1024*1024);assert_eq!(hash(&preview::preview_archive_text(&p,SELECTED,None).await.unwrap().content),manifest()["renderedPrefixSha256"].as_str().unwrap());
 // Controlled mapping on an original short stream proves a short decoder
 // cannot return a successful partial prefix. This is not an original file.
 let mut input=std::fs::File::open(case("lzma2")).unwrap();let len=input.metadata().unwrap().len();let mut a=sevenz_rust::Archive::read(&mut input,len,&[]).unwrap();let i=a.files.iter().position(|e|e.name=="readme.txt").unwrap();let b=a.stream_map.file_folder_index[i].unwrap();a.files[i].size=2*1024*1024;a.folders[b].unpack_sizes[0]=2*1024*1024;
 let e=sevenz_rust::BlockDecoder::new(b,&a,&[],&mut input).read_single_entry_prefix(i,1024*1024,||Ok(())).unwrap_err();assert!(e.to_string().contains("声明大小"),"{e}");
 let mut input=std::fs::File::open(case("lzma2")).unwrap();let len=input.metadata().unwrap().len();let a=sevenz_rust::Archive::read(&mut input,len,&[]).unwrap();let i=a.files.iter().position(|e|e.name==SELECTED).unwrap();let b=a.stream_map.file_folder_index[i].unwrap();let mut count=0;assert!(sevenz_rust::BlockDecoder::new(b,&a,&[],&mut input).read_single_entry_prefix(i,1024*1024,||{count+=1;if count==3{Err(sevenz_rust::Error::other("prefix cancelled"))}else{Ok(())}}).unwrap_err().to_string().contains("cancelled"));
}
