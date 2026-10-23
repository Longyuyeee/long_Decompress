use super::{archive_preview as preview, image_reader};
use std::{io::Read, path::{Path,PathBuf}, sync::{Arc,atomic::AtomicBool}};
use sha2::Digest;
const SELECTED:&str="docs/02-说明 [1].txt";
const LIMIT:u64=64*1024*1024;
fn root()->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-solid-text-1619")}
fn manifest()->serde_json::Value {serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn case(name:&str)->PathBuf {let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();root().join(name).join(c["members"][0]["name"].as_str().unwrap())}
fn archive()->(std::fs::File,sevenz_rust::Archive,usize,usize) {
 let mut f=std::fs::File::open(case("lzma2")).unwrap();let size=f.metadata().unwrap().len();let a=sevenz_rust::Archive::read(&mut f,size,&[]).unwrap();let i=a.files.iter().position(|e|e.name==SELECTED).unwrap();let b=a.stream_map.file_folder_index[i].unwrap();(f,a,i,b)
}

#[tokio::test]
async fn sevenz1619_original_shared_methods_and_every_numeric_member_show_complete_selected_text() {
 let m=manifest();
 for name in ["lzma","lzma2","volumes"] {
  let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();
  for member in c["members"].as_array().unwrap() {
   let p=root().join(name).join(member["name"].as_str().unwrap());
   let value=preview::preview_archive_text(&p,SELECTED,None).await.unwrap();
   assert!(!value.truncated);assert_eq!(value.byte_size,value.total_size);
   assert_eq!(hex::encode(sha2::Sha256::digest(value.content.as_bytes())),m["textSha256"].as_str().unwrap());
  }
 }
 for file in m["files"].as_array().unwrap().iter().filter(|f|f["path"].as_str().unwrap().ends_with(".txt")) {
  let value=preview::preview_archive_text(&case("lzma2"),file["path"].as_str().unwrap(),None).await.unwrap();
  assert_eq!(hex::encode(sha2::Sha256::digest(value.content.as_bytes())),file["sha256"].as_str().unwrap());
 }
 let old=Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-text-1617/solid/资料.7z");
 assert!(!preview::preview_archive_text(&old,"notes/说明 [1].txt",None).await.unwrap().truncated);
}

#[test]
fn sevenz1619_checks_earlier_later_and_folder_crcs_and_complete_decoder_end() {
 for which in ["before","after","folder","size","mapping","packed-crc"] {
  let(mut f,mut a,i,b)=archive();
  let indices:Vec<_>=a.files.iter().enumerate().filter(|(n,e)|e.has_stream&&a.stream_map.file_folder_index[*n]==Some(b)).map(|(n,_)|n).collect();
  assert!(indices.len()>2);
  match which {
   "before"=>{let n=*indices.iter().find(|n|**n<i).unwrap();a.files[n].has_crc=true;a.files[n].crc^=1;},
   "after"=>{let n=*indices.iter().find(|n|**n>i).unwrap();a.files[n].has_crc=true;a.files[n].crc^=1;},
   "folder"=>{a.folders[b].has_crc=true;a.folders[b].crc^=1;},
   "size"=>a.files[*indices.last().unwrap()].size+=1,
   "mapping"=>a.stream_map.file_folder_index[*indices.last().unwrap()]=None,
   _=>{let n=a.stream_map.folder_first_pack_stream_index[b];a.pack_crcs_defined.insert(n);a.pack_crcs.resize(a.pack_sizes.len(),0);a.pack_crcs[n]=0;}
  }
  assert!(sevenz_rust::BlockDecoder::new(b,&a,&[],&mut f).read_solid_entry(i,1024*1024,LIMIT,||Ok(())).is_err(),"{which}");
 }
 // Terminal damage cannot publish selected bytes even when every preceding byte decodes.
 let error=super::sevenz_preview::read(&root().join("bad-end.7z"),SELECTED,None,Arc::new(AtomicBool::new(false))).unwrap_err();
 assert!(format!("{error:#}").contains("CRC"),"{error:#}");
}

#[tokio::test]
async fn sevenz1619_shared_limits_and_encryption_keep_clear_extraction_next_steps() {
 for (p,entry,key,reason) in [(root().join("oversize-shared.7z"),"readme.txt",None,"64 MiB"),(case("encrypted"),SELECTED,Some("7Z固实公开测试1619!"),"加密固实")] {
  let e=preview::preview_archive_text(&p,entry,key).await.unwrap_err();let text=format!("{e:#}");assert!(text.contains(reason)&&text.contains("先提取"),"{text}");assert!(!text.contains("[sevenz-preview:password]"));
 }
 let(mut f,mut a,i,b)=archive();let n=a.stream_map.folder_first_pack_stream_index[b];a.pack_sizes[n]=LIMIT+1;
 assert!(sevenz_rust::BlockDecoder::new(b,&a,&[],&mut f).read_solid_entry(i,1024*1024,LIMIT,||Ok(())).is_err());
 let big=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/sevenz-solid-text-1619/unrelated.7z");assert!(big.metadata().unwrap().len()>LIMIT);
 assert!(!preview::preview_archive_text(&big,SELECTED,None).await.unwrap().truncated);
 let image=Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-image-1618/solid/相册.7z");
 assert_eq!(preview::preview_archive_image(&image,"gallery/01-cover.png",None).await.unwrap().width,320);
}

#[test]
fn sevenz1619_cancel_during_buffered_output_stops_without_retaining_other_files() {
 let(mut f,a,i,b)=archive();let mut checks=0;
 let e=sevenz_rust::BlockDecoder::new(b,&a,&[],&mut f).read_solid_entry(i,1024*1024,LIMIT,||{checks+=1;if checks==3 {Err(sevenz_rust::Error::io(std::io::Error::other("7Z 预览已取消")))} else {Ok(())}}).unwrap_err();
 assert!(e.to_string().contains("取消"));assert_eq!(checks,3);
 // Reject unverified dependencies rather than displaying a verified selected CRC alone.
 let(mut f,mut a,i,b)=archive();a.folders[b].has_crc=false;let other=a.files.iter().enumerate().find(|(n,e)|*n!=i&&e.has_stream&&a.stream_map.file_folder_index[*n]==Some(b)).unwrap().0;a.files[other].has_crc=false;
 assert!(sevenz_rust::BlockDecoder::new(b,&a,&[],&mut f).read_solid_entry(i,1024*1024,LIMIT,||Ok(())).is_err());
}

#[tokio::test]
async fn sevenz1619_source_revision_cancellation_and_missing_shared_volume_are_distinct() {
 let temp=tempfile::tempdir().unwrap();let changed=temp.path().join("changed.7z");std::fs::copy(case("lzma2"),&changed).unwrap();let rev=image_reader::revision(&changed).unwrap();
 use std::io::Write;std::fs::OpenOptions::new().append(true).open(&changed).unwrap().write_all(&[0]).unwrap();
 let e=preview::preview_archive_text_controlled_at(&changed,SELECTED,None,None,None,Arc::new(AtomicBool::new(false)),Some(rev)).await.unwrap_err();assert!(format!("{e:#}").contains("IMAGE_READER_CHANGED"));
 let e=preview::preview_archive_text_controlled_at(&case("lzma2"),SELECTED,None,None,None,Arc::new(AtomicBool::new(true)),None).await.unwrap_err();assert!(e.to_string().contains("取消"));
 let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]=="volumes").unwrap();let members:Vec<_>=c["members"].as_array().unwrap().iter().map(|f|{let n=f["name"].as_str().unwrap();let p=temp.path().join(n);std::fs::copy(root().join("volumes").join(n),&p).unwrap();p}).collect();
 assert!(members.len()>2);let rev=image_reader::revision(&members[1]).unwrap();std::fs::remove_file(members.last().unwrap()).unwrap();
 let e=preview::preview_archive_text_controlled_at(&members[1],SELECTED,None,None,None,Arc::new(AtomicBool::new(false)),Some(rev)).await.unwrap_err();assert!(format!("{e:#}").contains("IMAGE_READER_CHANGED"));
 assert!(preview::preview_archive_text(&members[1],SELECTED,None).await.is_err());
 // Keep the trait in scope to make direct readers available when auditing bytes.
 let mut f=std::fs::File::open(case("lzma2")).unwrap();let mut signature=[0;6];f.read_exact(&mut signature).unwrap();assert_eq!(&signature,b"7z\xBC\xAF\x27\x1C");
}
