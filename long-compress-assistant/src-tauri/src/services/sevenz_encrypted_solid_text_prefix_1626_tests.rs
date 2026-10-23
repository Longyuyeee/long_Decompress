use std::{path::{Path,PathBuf},sync::{Arc,atomic::AtomicBool}};
use sha2::{Digest,Sha256};
use super::{archive_preview as preview,image_reader};
const CAP:usize=1024*1024;const LIMIT:u64=64*1024*1024;
const SELECTED:&str="logs/20-session [1].txt";
fn root()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-encrypted-solid-text-prefix-1626")}
fn manifest()->serde_json::Value{serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn key()->String{manifest()["password"].as_str().unwrap().into()}
fn case(name:&str)->PathBuf{let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();root().join(name).join(c["members"][0]["name"].as_str().unwrap())}
fn hash(s:&str)->String{hex::encode(Sha256::digest(s.as_bytes()))}
fn archive()->(std::fs::File,sevenz_rust::Archive,usize,usize,sevenz_rust::Password){let mut input=std::fs::File::open(case("lzma2")).unwrap();let len=input.metadata().unwrap().len();let pwd=sevenz_rust::Password::from(key().as_str());let a=sevenz_rust::Archive::read(&mut input,len,pwd.as_slice()).unwrap();let i=a.files.iter().position(|e|e.name==SELECTED).unwrap();let b=a.stream_map.file_folder_index[i].unwrap();(input,a,i,b,pwd)}
async fn error(path:&Path,entry:&str,password:Option<&str>)->String{match preview::preview_archive_text(path,entry,password).await{Err(e)=>format!("{e:#}"),Ok(v)=>panic!("unexpected successful {} bytes",v.byte_size)}}

#[tokio::test]
async fn sevenz1626_original_aes_methods_headers_every_numeric_member_and_boundaries_are_exact(){
 let m=manifest();for c in m["cases"].as_array().unwrap(){for member in c["members"].as_array().unwrap(){let path=root().join(c["name"].as_str().unwrap()).join(member["name"].as_str().unwrap());let v=preview::preview_archive_text(&path,SELECTED,Some(&key())).await.unwrap();assert!(v.truncated);assert_eq!(v.byte_size,m["renderedPrefixBytes"]);assert_eq!(v.total_size,m["totalSize"]);assert_eq!(hash(&v.content),m["renderedPrefixSha256"]);}}
 let v=preview::preview_archive_text(&case("lzma2"),m["first"].as_str().unwrap(),Some(&key())).await.unwrap();assert!(v.truncated);assert_eq!(hash(&v.content),m["firstPrefixSha256"]);
 assert_eq!(preview::preview_archive_text(&case("lzma2"),"logs/20-session 1.txt",Some(&key())).await.unwrap().content,"NOT the selected bracket path");
 for e in m["boundaries"].as_array().unwrap(){let v=preview::preview_archive_text(&case("lzma2"),e["path"].as_str().unwrap(),Some(&key())).await.unwrap();assert!(v.truncated);assert_eq!(v.byte_size,e["bytes"]);assert_eq!(v.encoding,e["encoding"]);assert_eq!(hash(&v.content),e["sha256"]);assert!(!v.content.contains('�'));}
}
#[tokio::test]
async fn sevenz1626_vault_manual_password_and_stats_keep_partial_semantics(){
 use super::{compression_service::{CompressionService,CompressionServiceConfig},encrypted_password_service::EncryptedPasswordService,io_buffer_pool::IOBufferPool,rar_support::RarSupportService,universal_engine::UniversalCliEngine};use crate::models::password::{PasswordEntry,PasswordCategory};
 let temp=tempfile::tempdir().unwrap();let mut vault=EncryptedPasswordService::new(temp.path());vault.initialize("owned-public-test-key").await.unwrap();for(n,k)in[("a wrong","wrong".to_string()),("b public",key())]{vault.add_password(PasswordEntry::new(n.into(),k,PasswordCategory::Other)).await.unwrap();}
 let service=CompressionService::new(CompressionServiceConfig::default(),Arc::new(IOBufferPool::default()),Arc::new(RarSupportService::new()),Arc::new(UniversalCliEngine::new()),Arc::new(vault));
 for name in ["lzma","headers","volumes"]{let p=case(name);let listing=service.browse_sevenz_with_vault(p.to_str().unwrap(),None).await.unwrap();assert!(listing.entries.iter().find(|e|e.path==SELECTED).unwrap().compressed_size.is_none());let v=service.preview_sevenz_text_with_vault(p.to_str().unwrap(),SELECTED,None,None).await.unwrap();assert!(v.truncated);assert_eq!(hash(&v.content),manifest()["renderedPrefixSha256"]);assert!(service.preview_sevenz_text_with_vault(p.to_str().unwrap(),SELECTED,Some("wrong manual"),None).await.is_err());}
 assert!(service.encrypted_password_service.list_passwords().await.unwrap().iter().all(|p|p.use_count==0));
}
#[test]
fn sevenz1626_mapping_aes_alignment_short_output_and_skip_cancellation_reject(){
 for which in ["mapping","sum","aes-size","short","cancel"] {
  let(mut input,mut a,i,b,pwd)=archive();let mut checks=0;
  let order:Vec<_>=a.folders[b].ordered_coder_iter().map(|(i,_)|i).collect();assert_eq!(order.len(),2);
  match which {
   "mapping"=>{let previous=a.files.iter().enumerate().find(|(n,e)|*n<i&&e.has_stream).unwrap().0;a.stream_map.file_folder_index[previous]=None;},
   "sum"=>a.files[i].size+=1,
   "aes-size"=>a.folders[b].unpack_sizes[order[0]]+=16,
   "short"=>{let last=a.files.iter().rposition(|e|e.has_stream).unwrap();let total=a.folders[b].get_unpack_size();a.files[..i].iter_mut().find(|e|e.has_stream).unwrap().size+=total+3*CAP as u64;a.files[last].size=0;a.folders[b].unpack_sizes[order[1]]=a.files.iter().filter(|e|e.has_stream).map(|e|e.size).sum();},_=>{}
  }
  let v=sevenz_rust::BlockDecoder::new(b,&a,pwd.as_slice(),&mut input).read_solid_entry_prefix(i,CAP,LIMIT,||{checks+=1;if which=="cancel"&&checks==4{Err(sevenz_rust::Error::other("预览已取消"))}else{Ok(())}});
  assert!(v.is_err(),"{which}");if which=="cancel"{assert_eq!(checks,4);}
 }
}
#[tokio::test]
async fn sevenz1626_shared_large_total_and_exact_dependency_boundary_remain_bounded(){
 let p=root().join("large-shared.7z");let v=preview::preview_archive_text(&p,"00-huge.txt",Some(&key())).await.unwrap();assert!(v.truncated);assert_eq!(v.total_size,65*CAP as u64);assert_eq!(v.content.len(),CAP);assert!(v.content.bytes().all(|b|b==b'L'));assert!(error(&p,"99-selected.txt",Some(&key())).await.contains("前置内容与开头合计超过 64 MiB"));
 let(mut input,a,i,b,pwd)=archive();let offset:u64=a.files[..i].iter().filter(|e|e.has_stream).map(|e|e.size).sum();let allowed=offset+CAP as u64;assert_eq!(sevenz_rust::BlockDecoder::new(b,&a,pwd.as_slice(),&mut input).read_solid_entry_prefix(i,CAP,allowed,||Ok(())).unwrap().len(),CAP);
 let(mut input,a,i,b,pwd)=archive();assert!(sevenz_rust::BlockDecoder::new(b,&a,pwd.as_slice(),&mut input).read_solid_entry_prefix(i,CAP,allowed-1,||Ok(())).is_err());
}
#[tokio::test]
async fn sevenz1626_partial_tail_does_not_waive_full_shared_aes_or_crc(){
 let v=preview::preview_archive_text(&root().join("tail-damaged.7z"),SELECTED,Some(&key())).await.unwrap();assert!(v.truncated);assert_eq!(hash(&v.content),manifest()["renderedPrefixSha256"]);assert!(error(&root().join("tail-damaged.7z"),"logs/readme.txt",Some(&key())).await.contains("password"));
 for which in ["before-crc","selected-crc","after-crc","folder-crc"] {let(mut input,mut a,i,b,pwd)=archive();let indices:Vec<_>=a.files.iter().enumerate().filter(|(n,e)|e.has_stream&&a.stream_map.file_folder_index[*n]==Some(b)).map(|(i,_)|i).collect();if which=="folder-crc"{a.folders[b].has_crc=true;a.folders[b].crc^=1;}else{let n=match which{"before-crc"=>indices[0],"selected-crc"=>i,_=>*indices.last().unwrap()};a.files[n].has_crc=true;a.files[n].crc^=1;}
  assert!(sevenz_rust::BlockDecoder::new(b,&a,pwd.as_slice(),&mut input).read_solid_entry_prefix(i,CAP,LIMIT,||Ok(())).is_ok());assert!(sevenz_rust::BlockDecoder::new(b,&a,pwd.as_slice(),&mut input).read_solid_entry(i,4*CAP,LIMIT,||Ok(())).is_err(),"{which}");}
 assert!(error(&case("lzma2"),"logs/30-binary.txt",Some(&key())).await.contains("binary"));assert!(error(&root().join("bad-start.7z"),SELECTED,Some(&key())).await.contains("password"));
 assert!(!preview::preview_archive_text(&case("headers"),"logs/readme.txt",Some(&key())).await.unwrap().truncated);
}
#[tokio::test]
async fn sevenz1626_cancel_changed_source_missing_volume_and_fresh_shared_prefix(){
 let temp=tempfile::tempdir().unwrap();let p=temp.path().join("changed.7z");std::fs::copy(case("headers"),&p).unwrap();let rev=image_reader::revision(&p).unwrap();let v=preview::preview_archive_text_controlled_at(&p,SELECTED,Some(&key()),None,None,Arc::new(AtomicBool::new(true)),Some(rev.clone())).await;assert!(format!("{:#}",v.err().unwrap()).contains("取消"));
 use std::io::Write;std::fs::OpenOptions::new().append(true).open(&p).unwrap().write_all(b"x").unwrap();let v=preview::preview_archive_text_controlled_at(&p,SELECTED,Some(&key()),None,None,Arc::new(AtomicBool::new(false)),Some(rev)).await;assert!(format!("{:#}",v.err().unwrap()).contains("IMAGE_READER_CHANGED"));
 let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]=="volumes").unwrap();let files:Vec<_>=c["members"].as_array().unwrap().iter().map(|f|{let n=f["name"].as_str().unwrap();let p=temp.path().join(n);std::fs::copy(root().join("volumes").join(n),&p).unwrap();p}).collect();assert!(files.len()>2);let revision=image_reader::revision(&files[1]).unwrap();std::fs::remove_file(files.last().unwrap()).unwrap();let v=preview::preview_archive_text_controlled_at(&files[1],SELECTED,Some(&key()),None,None,Arc::new(AtomicBool::new(false)),Some(revision)).await;assert!(format!("{:#}",v.err().unwrap()).contains("IMAGE_READER_CHANGED"));assert!(preview::preview_archive_text(&files[1],SELECTED,Some(&key())).await.is_err());assert!(preview::preview_archive_text(&case("headers"),SELECTED,Some(&key())).await.unwrap().truncated);
}
