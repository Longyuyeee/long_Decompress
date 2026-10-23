use std::{path::{Path,PathBuf},sync::{Arc,atomic::AtomicBool}};
use sha2::{Digest,Sha256};use base64::Engine;
use super::{archive_preview as preview,image_reader::{self,Control}};
const TEXT:&str="notes/说明 [1].txt";const IMAGE:&str="gallery/01-cover.png";const LIMIT:u64=64*1024*1024;
fn root()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-solid-ppmd-preview-1629")}
fn manifest()->serde_json::Value{serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn case(name:&str)->PathBuf{let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();root().join(name).join(c["members"][0]["name"].as_str().unwrap())}
fn hash(b:&[u8])->String{hex::encode(Sha256::digest(b))}
fn image_hash(v:&preview::ArchiveImagePreview)->String{hash(&base64::engine::general_purpose::STANDARD.decode(v.data_url.split(',').nth(1).unwrap()).unwrap())}
async fn text_error(p:&Path,e:&str,key:Option<&str>)->String{match preview::preview_archive_text(p,e,key).await{Err(e)=>format!("{e:#}"),Ok(v)=>panic!("unexpected successful {} bytes",v.byte_size)}}
fn archive()->(std::fs::File,sevenz_rust::Archive,usize,usize){let mut input=std::fs::File::open(case("ppmd")).unwrap();let len=input.metadata().unwrap().len();let a=sevenz_rust::Archive::read(&mut input,len,&[]).unwrap();let i=a.files.iter().position(|e|e.name==TEXT).unwrap();let f=a.stream_map.file_folder_index[i].unwrap();(input,a,i,f)}

#[tokio::test]
async fn sevenz1629_original_solid_orders_every_numeric_member_and_complete_text_encodings(){
 let m=manifest();for c in m["cases"].as_array().unwrap(){for member in c["members"].as_array().unwrap(){let p=root().join(c["name"].as_str().unwrap()).join(member["name"].as_str().unwrap());let v=preview::preview_archive_text(&p,TEXT,None).await.unwrap();assert!(!v.truncated);assert_eq!(v.byte_size,m["textBytes"]);assert_eq!(hash(v.content.as_bytes()),m["textSha256"]);}}
 for(e,text)in[("00-first.txt","First shared text\n".repeat(200)),("notes/说明 1.txt","NOT the selected bracket path".into()),("notes/empty.txt","".into()),("notes/utf16.txt","完整 UTF16 文本".into()),("notes/gbk.txt","中文编码说明".into())]{let v=preview::preview_archive_text(&case("ppmd"),e,None).await.unwrap();assert!(!v.truncated);assert_eq!(v.content,text);}
 let v=preview::preview_archive_text(&case("ppmd"),"notes/exact.txt",None).await.unwrap();assert!(!v.truncated);assert_eq!(v.byte_size,1024*1024);
 // A single affected-routing continuity check; no old password/volume matrix.
 let old=Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-password-ppmd-preview-1628");let m:serde_json::Value=serde_json::from_slice(&std::fs::read(old.join("manifest.json")).unwrap()).unwrap();let p=old.join("contents").join(m["cases"][0]["members"][0]["name"].as_str().unwrap());let v=preview::preview_archive_text(&p,TEXT,m["password"].as_str()).await.unwrap();assert!(!v.truncated);assert_eq!(hash(v.content.as_bytes()),m["textSha256"]);
}
#[tokio::test]
async fn sevenz1629_original_shared_rasters_and_non_first_volume_keep_exact_bytes(){
 let m=manifest();for name in ["ppmd","order12","volumes"]{let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();let members=c["members"].as_array().unwrap();let p=root().join(name).join(members[members.len()/2]["name"].as_str().unwrap());let rev=image_reader::revision(&p).unwrap();for e in m["images"].as_array().unwrap(){let v=image_reader::read(&p,e["path"].as_str().unwrap(),None,None,None,&rev,Control::default()).await.unwrap();assert_eq!(v.width as u64,e["width"]);assert_eq!(v.height as u64,e["height"]);assert_eq!(image_hash(&v),e["sha256"]);}}
}
#[test]
fn sevenz1629_shared_before_after_block_pack_crc_mapping_parameters_and_cancel_are_required(){
 for which in ["before-crc","after-crc","block-crc","pack-crc","size","mapping","reorder","order","memory","properties","extra-pack","missing-crc","packed-map"] {
  let(mut input,mut a,i,f)=archive();let pack=a.stream_map.folder_first_pack_stream_index[f];let indices:Vec<_>=a.files.iter().enumerate().filter(|(n,e)|e.has_stream&&a.stream_map.file_folder_index[*n]==Some(f)).map(|(n,_)|n).collect();let before=*indices.iter().find(|n|**n<i).unwrap();let after=*indices.iter().find(|n|**n>i).unwrap();
  match which {
   "before-crc"=>{a.files[before].has_crc=true;a.files[before].crc^=1;},"after-crc"=>{a.files[after].has_crc=true;a.files[after].crc^=1;},"block-crc"=>{a.folders[f].has_crc=true;a.folders[f].crc^=1;},"pack-crc"=>{a.pack_crcs_defined.insert(pack);a.pack_crcs.resize(a.pack_sizes.len(),0);a.pack_crcs[pack]=0;},"size"=>a.files[after].size+=1,"mapping"=>a.stream_map.file_folder_index[after]=None,"reorder"=>a.files.swap(i,before),
   "order"=>a.folders[f].coders[0].properties[0]=1,"memory"=>a.folders[f].coders[0].properties[1..5].copy_from_slice(&(65u32*1024*1024).to_le_bytes()),"properties"=>{a.folders[f].coders[0].properties.pop();},"extra-pack"=>a.pack_sizes[pack]+=1,"missing-crc"=>{a.files[before].has_crc=false;a.folders[f].has_crc=false;},_=>a.folders[f].packed_streams[0]=1
  }
  assert!(sevenz_rust::BlockDecoder::new(f,&a,&[],&mut input).read_solid_ppmd_entry(i,1024*1024,LIMIT,||Ok(())).is_err(),"{which}");
 }
 // A valid folder-only CRC may cover all shared files; selected CRC alone may not.
 let(mut input,mut a,i,f)=archive();let mut crc=crc32fast::Hasher::new();for(n,e)in a.files.iter().enumerate().filter(|(n,e)|e.has_stream&&a.stream_map.file_folder_index[*n]==Some(f)){assert!(a.files[n].has_crc);crc.combine(&crc32fast::Hasher::new_with_initial_len(e.crc as u32,e.size));}a.folders[f].has_crc=true;a.folders[f].crc=crc.finalize() as u64;for(n,e)in a.files.iter_mut().enumerate(){if a.stream_map.file_folder_index[n]==Some(f){e.has_crc=false;}}let v=sevenz_rust::BlockDecoder::new(f,&a,&[],&mut input).read_solid_ppmd_entry(i,1024*1024,LIMIT,||Ok(())).unwrap();assert_eq!(hash(&v),manifest()["textSha256"]);
 let(mut input,a,i,f)=archive();let mut count=0;let e=sevenz_rust::BlockDecoder::new(f,&a,&[],&mut input).read_solid_ppmd_entry(i,1024*1024,LIMIT,||{count+=1;if count==3{Err(sevenz_rust::Error::other("预览已取消"))}else{Ok(())}}).unwrap_err();assert!(e.to_string().contains("取消"));
}
#[tokio::test]
async fn sevenz1629_readable_shared_bad_flush_never_displays_text_or_pixels_and_unrelated_block_is_skipped(){
 let p=root().join("bad-flush.7z");let e=text_error(&p,TEXT,None).await;assert!(e.contains("range coder did not finish"),"{e}");let e=preview::preview_archive_image(&p,IMAGE,None).await.err().unwrap();assert!(format!("{e:#}").contains("range coder did not finish"));
 let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/sevenz-solid-ppmd-preview-1629/large-unrelated.7z");assert!(p.metadata().unwrap().len()>LIMIT);let m=manifest();assert_eq!(hash(preview::preview_archive_text(&p,TEXT,None).await.unwrap().content.as_bytes()),m["textSha256"]);assert_eq!(image_hash(&preview::preview_archive_image(&p,IMAGE,None).await.unwrap()),m["images"][0]["sha256"]);
}
#[tokio::test]
async fn sevenz1629_shared_and_image_boundaries_binary_large_and_password_scopes_keep_next_steps(){
 let v=preview::preview_archive_text(&root().join("at-shared-limit.7z"),"readme.txt",None).await.unwrap();assert_eq!(v.content,"Valid selected shared text");assert!(!v.truncated);let e=text_error(&root().join("oversize-shared.7z"),"readme.txt",None).await;assert!(e.contains("64 MiB")&&e.contains("先提取"),"{e}");
 assert!(text_error(&case("ppmd"),"notes/large.txt",None).await.contains("PPMd"));assert!(text_error(&case("ppmd"),"notes/binary.txt",None).await.contains("binary"));let e=text_error(&root().join("encrypted.7z"),TEXT,Some("PPMd固实公开密码1629!")).await;assert!(e.contains("PPMd")&&e.contains("先提取"),"{e}");
 let v=preview::preview_archive_image(&root().join("at-limit.7z"),"at-limit.bmp",None).await.unwrap();assert_eq!(v.byte_size,8*1024*1024);
 for(p,e,reason)in[(root().join("oversize.7z"),"oversize.bmp","8 MiB"),(root().join("dimensions.7z"),"dimensions.png","dimensions exceed"),(case("ppmd"),"gallery/03-bad.png","not a supported raster")]{let error=preview::preview_archive_image(&p,e,None).await.err().unwrap();assert!(format!("{error:#}").contains(reason),"{error:#}");}
 let(mut input,mut a,i,f)=archive();let pack=a.stream_map.folder_first_pack_stream_index[f];a.pack_sizes[pack]=LIMIT+1;assert!(sevenz_rust::BlockDecoder::new(f,&a,&[],&mut input).read_solid_ppmd_entry(i,1024*1024,LIMIT,||Ok(())).is_err());
}
#[tokio::test]
async fn sevenz1629_cancel_source_revision_missing_shared_tail_and_fresh_request(){
 let temp=tempfile::tempdir().unwrap();let p=temp.path().join("changed.7z");std::fs::copy(case("ppmd"),&p).unwrap();let rev=image_reader::revision(&p).unwrap();let v=preview::preview_archive_text_controlled_at(&p,TEXT,None,None,None,Arc::new(AtomicBool::new(true)),Some(rev.clone())).await;assert!(format!("{:#}",v.err().unwrap()).contains("取消"));
 use std::io::Write;std::fs::OpenOptions::new().append(true).open(&p).unwrap().write_all(b"x").unwrap();let v=preview::preview_archive_text_controlled_at(&p,TEXT,None,None,None,Arc::new(AtomicBool::new(false)),Some(rev)).await;assert!(format!("{:#}",v.err().unwrap()).contains("IMAGE_READER_CHANGED"));
 let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]=="volumes").unwrap();let files:Vec<_>=c["members"].as_array().unwrap().iter().map(|f|{let n=f["name"].as_str().unwrap();let p=temp.path().join(n);std::fs::copy(root().join("volumes").join(n),&p).unwrap();p}).collect();assert!(files.len()>2);let rev=image_reader::revision(&files[1]).unwrap();std::fs::remove_file(files.last().unwrap()).unwrap();let e=preview::preview_archive_text_controlled_at(&files[1],TEXT,None,None,None,Arc::new(AtomicBool::new(false)),Some(rev)).await.err().unwrap();assert!(format!("{e:#}").contains("IMAGE_READER_CHANGED"));let v=preview::preview_archive_text(&case("ppmd"),TEXT,None).await.unwrap();assert!(!v.truncated);assert_eq!(hash(v.content.as_bytes()),m["textSha256"]);
}
