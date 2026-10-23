use std::{path::{Path,PathBuf},sync::{Arc,atomic::AtomicBool}};
use sha2::{Digest,Sha256};use base64::Engine;
use super::{archive_preview as preview,image_reader::{self,Control}};
const TEXT:&str="notes/说明 [1].txt";const IMAGE:&str="gallery/01-cover.png";
fn root()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-deflate-preview-1633")}
fn manifest()->serde_json::Value{serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn case(name:&str)->PathBuf{let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();root().join(name).join(c["members"][0]["name"].as_str().unwrap())}
fn hash(b:&[u8])->String{hex::encode(Sha256::digest(b))}
fn image_bytes(v:&preview::ArchiveImagePreview)->Vec<u8>{base64::engine::general_purpose::STANDARD.decode(v.data_url.split(',').nth(1).unwrap()).unwrap()}
fn archive()->(std::fs::File,sevenz_rust::Archive,usize,usize){let mut input=std::fs::File::open(case("deflate")).unwrap();let len=input.metadata().unwrap().len();let a=sevenz_rust::Archive::read(&mut input,len,&[]).unwrap();let i=a.files.iter().position(|e|e.name==TEXT).unwrap();let f=a.stream_map.file_folder_index[i].unwrap();(input,a,i,f)}
async fn text_error(p:&Path,e:&str,key:Option<&str>)->String{match preview::preview_archive_text(p,e,key).await{Err(e)=>format!("{e:#}"),Ok(v)=>panic!("unexpected successful {} bytes",v.byte_size)}}

#[tokio::test]
async fn sevenz1633_original_orders_every_numeric_member_and_complete_text_encodings(){
 let m=manifest();for c in m["cases"].as_array().unwrap(){for member in c["members"].as_array().unwrap(){let p=root().join(c["name"].as_str().unwrap()).join(member["name"].as_str().unwrap());let v=preview::preview_archive_text(&p,TEXT,None).await.unwrap();assert!(!v.truncated);assert_eq!(v.byte_size,m["textBytes"]);assert_eq!(hash(v.content.as_bytes()),m["textSha256"]);}}
 for(e,text)in[("notes/说明 1.txt","NOT the selected bracket path"),("notes/empty.txt",""),("notes/utf16.txt","完整 UTF16 文本"),("notes/gbk.txt","中文编码说明")]{let v=preview::preview_archive_text(&case("deflate"),e,None).await.unwrap();assert!(!v.truncated);assert_eq!(v.content,text);}
 let v=preview::preview_archive_text(&case("deflate"),"notes/exact.txt",None).await.unwrap();assert!(!v.truncated);assert_eq!(v.byte_size,1024*1024);
}
#[tokio::test]
async fn sevenz1633_original_rasters_and_non_first_volume_keep_exact_bytes(){
 let m=manifest();for name in ["deflate","maximum","volumes"]{let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();let members=c["members"].as_array().unwrap();let p=root().join(name).join(members[members.len()/2]["name"].as_str().unwrap());let rev=image_reader::revision(&p).unwrap();for e in m["images"].as_array().unwrap(){let v=image_reader::read(&p,e["path"].as_str().unwrap(),None,None,None,&rev,Control::default()).await.unwrap();assert_eq!(v.width as u64,e["width"]);assert_eq!(v.height as u64,e["height"]);assert_eq!(hash(&image_bytes(&v)),e["sha256"]);}}
}
#[test]
fn sevenz1633_exact_crc_size_pack_coverage_and_parameters_are_required(){
 for which in ["entry-crc","folder-crc","pack-crc","size","mapping","method","properties","extra-pack","missing-crc"] {
  let(mut input,mut a,i,f)=archive();let pack=a.stream_map.folder_first_pack_stream_index[f];
  match which {
   "entry-crc"=>{a.files[i].has_crc=true;a.files[i].crc^=1;},"folder-crc"=>{a.folders[f].has_crc=true;a.folders[f].crc^=1;},
   "pack-crc"=>{a.pack_crcs_defined.insert(pack);a.pack_crcs.resize(a.pack_sizes.len(),0);a.pack_crcs[pack]=0;},
   "size"=>{a.files[i].size-=1;a.folders[f].unpack_sizes[0]-=1;},"mapping"=>a.stream_map.file_folder_index[i]=None,
   "method"=>{a.folders[f].coders[0].id_size=1; a.folders[f].coders[0].decompression_method_id_mut().copy_from_slice(&[0]);},
   "properties"=>a.folders[f].coders[0].properties.push(1),"extra-pack"=>a.pack_sizes[pack]+=1,
   _=>{a.files[i].has_crc=false;a.folders[f].has_crc=false;}
  }
  assert!(sevenz_rust::BlockDecoder::new(f,&a,&[],&mut input).read_deflate_entry(i,1024*1024,||Ok(())).is_err(),"{which}");
 }
 let(mut input,a,i,f)=archive();let mut count=0;let e=sevenz_rust::BlockDecoder::new(f,&a,&[],&mut input).read_deflate_entry(i,1024*1024,||{count+=1;if count==3{Err(sevenz_rust::Error::other("预览已取消"))}else{Ok(())}}).unwrap_err();assert!(e.to_string().contains("取消"));
}
#[tokio::test]
async fn sevenz1633_damaged_deflate_never_displays_complete_body_and_original_is_exact(){
 assert!(text_error(&root().join("bad-data.7z"),TEXT,None).await.contains("Deflate"));
 assert_eq!(hash(preview::preview_archive_text(&root().join("original.7z"),TEXT,None).await.unwrap().content.as_bytes()),manifest()["textSha256"]);
}
#[tokio::test]
async fn sevenz1633_output_image_limits_binary_and_unsupported_scopes_keep_next_steps(){
 assert!(text_error(&case("deflate"),"notes/binary.txt",None).await.contains("binary"));
 for (name,key,reason) in [("solid",None,"Deflate"),("encrypted",manifest()["password"].as_str(),"Deflate"),("deflate64",None,"方法组合")] {assert!(text_error(&root().join(format!("{name}.7z")),TEXT,key).await.contains(reason),"{name}");}
 assert!(text_error(&case("deflate"),"notes/large.txt",None).await.contains("Deflate"));

 let v=preview::preview_archive_image(&root().join("at-limit.7z"),"at-limit.bmp",None).await.unwrap();assert_eq!(image_bytes(&v).len(),8*1024*1024);
 for(p,e,reason)in[(root().join("oversize.7z"),"oversize.bmp","8 MiB"),(root().join("dimensions.7z"),"dimensions.png","dimensions exceed"),(case("deflate"),"gallery/03-bad.png","not a supported raster")]{let error=preview::preview_archive_image(&p,e,None).await.err().unwrap();assert!(format!("{error:#}").contains(reason),"{error:#}");}
}
#[tokio::test]
async fn sevenz1633_cancel_source_revision_missing_tail_and_unrelated_data(){
 let temp=tempfile::tempdir().unwrap();let p=temp.path().join("changed.7z");std::fs::copy(case("deflate"),&p).unwrap();let rev=image_reader::revision(&p).unwrap();let v=preview::preview_archive_text_controlled_at(&p,TEXT,None,None,None,Arc::new(AtomicBool::new(true)),Some(rev.clone())).await;assert!(format!("{:#}",v.err().unwrap()).contains("取消"));
 use std::io::Write;std::fs::OpenOptions::new().append(true).open(&p).unwrap().write_all(b"x").unwrap();let v=preview::preview_archive_text_controlled_at(&p,TEXT,None,None,None,Arc::new(AtomicBool::new(false)),Some(rev)).await;assert!(format!("{:#}",v.err().unwrap()).contains("IMAGE_READER_CHANGED"));
 let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]=="volumes").unwrap();let files:Vec<_>=c["members"].as_array().unwrap().iter().map(|f|{let n=f["name"].as_str().unwrap();let p=temp.path().join(n);std::fs::copy(root().join("volumes").join(n),&p).unwrap();p}).collect();assert!(files.len()>2);let rev=image_reader::revision(&files[1]).unwrap();std::fs::remove_file(files.last().unwrap()).unwrap();assert!(preview::preview_archive_text_controlled_at(&files[1],TEXT,None,None,None,Arc::new(AtomicBool::new(false)),Some(rev)).await.err().unwrap().to_string().contains("IMAGE_READER_CHANGED"));
 let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/sevenz-deflate-preview-1633/large-unrelated.7z");assert!(p.metadata().unwrap().len()>64*1024*1024);assert_eq!(hash(preview::preview_archive_text(&p,TEXT,None).await.unwrap().content.as_bytes()),m["textSha256"]);assert_eq!(hash(&image_bytes(&preview::preview_archive_image(&p,IMAGE,None).await.unwrap())),m["images"][0]["sha256"]);
}
