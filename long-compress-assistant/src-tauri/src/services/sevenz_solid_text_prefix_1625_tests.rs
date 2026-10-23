use std::{path::{Path,PathBuf},sync::{Arc,atomic::AtomicBool}};
use sha2::{Digest,Sha256};
use super::{archive_preview as preview,image_reader};
const CAP:usize=1024*1024;
const LIMIT:u64=64*1024*1024;
const SELECTED:&str="logs/20-session [1].txt";
fn root()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-solid-text-prefix-1625")}
fn manifest()->serde_json::Value{serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn case(name:&str)->PathBuf{let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();root().join(name).join(c["members"][0]["name"].as_str().unwrap())}
fn hash(s:&str)->String{hex::encode(Sha256::digest(s.as_bytes()))}
fn archive()->(std::fs::File,sevenz_rust::Archive,usize,usize){let input=std::fs::File::open(case("lzma2")).unwrap();let len=input.metadata().unwrap().len();let mut input=input;let a=sevenz_rust::Archive::read(&mut input,len,&[]).unwrap();let i=a.files.iter().position(|e|e.name==SELECTED).unwrap();let b=a.stream_map.file_folder_index[i].unwrap();(input,a,i,b)}
async fn error(path:&Path,entry:&str,password:Option<&str>)->String{match preview::preview_archive_text(path,entry,password).await{Err(e)=>format!("{e:#}"),Ok(v)=>panic!("unexpected successful {} bytes",v.byte_size)}}

#[tokio::test]
async fn sevenz1625_original_methods_every_numeric_member_and_first_middle_are_exact(){
 let m=manifest();for c in m["cases"].as_array().unwrap(){for member in c["members"].as_array().unwrap(){let path=root().join(c["name"].as_str().unwrap()).join(member["name"].as_str().unwrap());let v=preview::preview_archive_text(&path,SELECTED,None).await.unwrap();assert!(v.truncated);assert_eq!(v.byte_size,m["renderedPrefixBytes"]);assert_eq!(v.total_size,m["totalSize"]);assert_eq!(hash(&v.content),m["renderedPrefixSha256"]);}}
 let v=preview::preview_archive_text(&case("lzma2"),m["first"].as_str().unwrap(),None).await.unwrap();assert!(v.truncated);assert_eq!(hash(&v.content),m["firstPrefixSha256"]);
 let adjacent=preview::preview_archive_text(&case("lzma2"),"logs/20-session 1.txt",None).await.unwrap();assert!(!adjacent.truncated);assert_eq!(adjacent.content,"NOT the selected bracket path");
 for e in m["boundaries"].as_array().unwrap(){let v=preview::preview_archive_text(&case("lzma2"),e["path"].as_str().unwrap(),None).await.unwrap();assert!(v.truncated);assert_eq!(v.byte_size,e["bytes"]);assert_eq!(v.encoding,e["encoding"]);assert_eq!(hash(&v.content),e["sha256"]);assert!(!v.content.contains('�'));}
}
#[test]
fn sevenz1625_conflicting_mapping_short_output_and_cancellation_during_skip_never_publish(){
 for which in ["mapping","sum","anti","short","cancel"] {
  let(mut input,mut a,i,b)=archive();let mut checks=0;
  match which {
   "mapping"=>{let previous=a.files.iter().enumerate().find(|(n,e)|*n<i&&e.has_stream).unwrap().0;a.stream_map.file_folder_index[previous]=None;},
   "sum"=>a.files[i].size+=1,
   "anti"=>a.files[i].is_anti_item=true,
   "short"=>{let last=a.files.iter().rposition(|e|e.has_stream).unwrap();let total=a.folders[b].get_unpack_size();let moved=3*CAP as u64;a.files[0..i].iter_mut().filter(|e|e.has_stream).next().unwrap().size+=total+moved;a.files[last].size=0;a.folders[b].unpack_sizes[0]=a.files.iter().filter(|e|e.has_stream).map(|e|e.size).sum();},
   _=>{}
  }
  let value=sevenz_rust::BlockDecoder::new(b,&a,&[],&mut input).read_solid_entry_prefix(i,CAP,LIMIT,||{checks+=1;if which=="cancel"&&checks==4{Err(sevenz_rust::Error::other("预览已取消"))}else{Ok(())}});
  assert!(value.is_err(),"{which}");if which=="cancel"{assert_eq!(checks,4);}
 }
}
#[tokio::test]
async fn sevenz1625_large_total_allows_reachable_prefix_but_rejects_oversized_dependencies(){
 let p=root().join("large-shared.7z");let v=preview::preview_archive_text(&p,"00-huge.txt",None).await.unwrap();assert!(v.truncated);assert_eq!(v.total_size,65*CAP as u64);assert_eq!(v.content.len(),CAP);assert!(v.content.bytes().all(|b|b==b'L'));
 assert!(error(&p,"99-selected.txt",None).await.contains("前置内容与开头合计超过 64 MiB"));
 // Boundary uses the exact mapping from an original stream, not a new original
 // sample: the last readable byte must never let output exceed the limit.
 let(mut input,a,i,b)=archive();let offset:u64=a.files[..i].iter().filter(|e|e.has_stream).map(|e|e.size).sum();let allowed=offset+CAP as u64;
 assert_eq!(sevenz_rust::BlockDecoder::new(b,&a,&[],&mut input).read_solid_entry_prefix(i,CAP,allowed,||Ok(())).unwrap().len(),CAP);
 let(mut input,a,i,b)=archive();assert!(sevenz_rust::BlockDecoder::new(b,&a,&[],&mut input).read_solid_entry_prefix(i,CAP,allowed-1,||Ok(())).is_err());
}
#[tokio::test]
async fn sevenz1625_partial_tail_never_weakens_complete_shared_checks(){
 let p=root().join("tail-damaged.7z");let v=preview::preview_archive_text(&p,SELECTED,None).await.unwrap();assert!(v.truncated);assert_eq!(hash(&v.content),manifest()["renderedPrefixSha256"]);
 assert!(error(&p,"logs/readme.txt",None).await.contains("CRC"));
 for which in ["before-crc","selected-crc","after-crc","folder-crc"] {
  let(mut input,mut a,i,b)=archive();let indices:Vec<_>=a.files.iter().enumerate().filter(|(n,e)|e.has_stream&&a.stream_map.file_folder_index[*n]==Some(b)).map(|(i,_)|i).collect();
  if which=="folder-crc"{a.folders[b].has_crc=true;a.folders[b].crc^=1;}else{let n=match which{"before-crc"=>indices[0],"selected-crc"=>i,_=>*indices.last().unwrap()};a.files[n].has_crc=true;a.files[n].crc^=1;}
  // Prefix is intentionally not an integrity test. Complete shared extraction
  // must still reject CRC damage even after the selected prefix is readable.
  assert!(sevenz_rust::BlockDecoder::new(b,&a,&[],&mut input).read_solid_entry_prefix(i,CAP,LIMIT,||Ok(())).is_ok());
  assert!(sevenz_rust::BlockDecoder::new(b,&a,&[],&mut input).read_solid_entry(i,4*CAP,LIMIT,||Ok(())).is_err(),"{which}");
 }
 assert!(error(&case("lzma2"),"logs/30-binary.txt",None).await.contains("binary"));assert!(error(&root().join("bad-start.7z"),SELECTED,None).await.contains("CRC"));
 let old=Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-password-text-prefix-1624/solid.7z");assert!(error(&old,"logs/session [1].txt",Some("日志密码1624!")).await.contains("加密固实大文本"));
}
#[tokio::test]
async fn sevenz1625_cancel_changed_source_missing_volume_and_fresh_prefix_keep_original_contract(){
 let temp=tempfile::tempdir().unwrap();let p=temp.path().join("changed.7z");std::fs::copy(case("lzma2"),&p).unwrap();let rev=image_reader::revision(&p).unwrap();let v=preview::preview_archive_text_controlled_at(&p,SELECTED,None,None,None,Arc::new(AtomicBool::new(true)),Some(rev.clone())).await;assert!(format!("{:#}",v.err().unwrap()).contains("取消"));
 use std::io::Write;std::fs::OpenOptions::new().append(true).open(&p).unwrap().write_all(b"x").unwrap();let v=preview::preview_archive_text_controlled_at(&p,SELECTED,None,None,None,Arc::new(AtomicBool::new(false)),Some(rev)).await;assert!(format!("{:#}",v.err().unwrap()).contains("IMAGE_READER_CHANGED"));
 let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]=="volumes").unwrap();let files:Vec<_>=c["members"].as_array().unwrap().iter().map(|f|{let n=f["name"].as_str().unwrap();let p=temp.path().join(n);std::fs::copy(root().join("volumes").join(n),&p).unwrap();p}).collect();assert!(files.len()>2);let revision=image_reader::revision(&files[1]).unwrap();std::fs::remove_file(files.last().unwrap()).unwrap();let v=preview::preview_archive_text_controlled_at(&files[1],SELECTED,None,None,None,Arc::new(AtomicBool::new(false)),Some(revision)).await;assert!(format!("{:#}",v.err().unwrap()).contains("IMAGE_READER_CHANGED"));assert!(preview::preview_archive_text(&files[1],SELECTED,None).await.is_err());
 assert!(preview::preview_archive_text(&case("lzma2"),SELECTED,None).await.unwrap().truncated);
}
