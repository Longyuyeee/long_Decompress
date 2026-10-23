use std::{path::{Path,PathBuf},sync::{Arc,atomic::AtomicBool}};
use sha2::{Digest,Sha256};use base64::Engine;
use super::{archive_preview as preview,image_reader::{self,Control},compression_service::{CompressionService,CompressionServiceConfig},encrypted_password_service::EncryptedPasswordService,io_buffer_pool::IOBufferPool,rar_support::RarSupportService,universal_engine::UniversalCliEngine};
const TEXT:&str="notes/说明 [1].txt";const IMAGE:&str="gallery/01-cover.png";const KEY:&str="PPMd密码固实公开密码1630!";const LIMIT:u64=64*1024*1024;
fn root()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-password-solid-ppmd-preview-1630")}
fn manifest()->serde_json::Value{serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn case(name:&str)->PathBuf{let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();root().join(name).join(c["members"][0]["name"].as_str().unwrap())}
fn hash(b:&[u8])->String{hex::encode(Sha256::digest(b))}
fn image_hash(v:&preview::ArchiveImagePreview)->String{hash(&base64::engine::general_purpose::STANDARD.decode(v.data_url.split(',').nth(1).unwrap()).unwrap())}
async fn text_error(p:&Path,e:&str,key:Option<&str>)->String{match preview::preview_archive_text(p,e,key).await{Err(e)=>format!("{e:#}"),Ok(v)=>panic!("unexpected successful {} bytes",v.byte_size)}}
fn archive()->(std::fs::File,sevenz_rust::Archive,usize,usize){let mut input=std::fs::File::open(case("contents")).unwrap();let len=input.metadata().unwrap().len();let a=sevenz_rust::Archive::read(&mut input,len,sevenz_rust::Password::from(KEY).as_slice()).unwrap();let i=a.files.iter().position(|e|e.name==TEXT).unwrap();let f=a.stream_map.file_folder_index[i].unwrap();(input,a,i,f)}

#[tokio::test]
async fn sevenz1630_original_solid_orders_every_numeric_member_and_complete_text_encodings(){
 let m=manifest();for c in m["cases"].as_array().unwrap(){for member in c["members"].as_array().unwrap(){let p=root().join(c["name"].as_str().unwrap()).join(member["name"].as_str().unwrap());let v=preview::preview_archive_text(&p,TEXT,Some(KEY)).await.unwrap();assert!(!v.truncated);assert_eq!(v.byte_size,m["textBytes"]);assert_eq!(hash(v.content.as_bytes()),m["textSha256"]);}}
 for(e,text)in[("00-first.txt","First shared text\n".repeat(200)),("notes/说明 1.txt","NOT the selected bracket path".into()),("notes/empty.txt","".into()),("notes/utf16.txt","完整 UTF16 文本".into()),("notes/gbk.txt","中文编码说明".into())]{let v=preview::preview_archive_text(&case("contents"),e,Some(KEY)).await.unwrap();assert!(!v.truncated);assert_eq!(v.content,text);}
 let v=preview::preview_archive_text(&case("contents"),"notes/exact.txt",Some(KEY)).await.unwrap();assert!(!v.truncated);assert_eq!(v.byte_size,1024*1024);
 // Only one plain shared continuity case for the changed shared reader.
 let old=Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-solid-ppmd-preview-1629");let m:serde_json::Value=serde_json::from_slice(&std::fs::read(old.join("manifest.json")).unwrap()).unwrap();let p=old.join("ppmd").join(m["cases"][0]["members"][0]["name"].as_str().unwrap());let v=preview::preview_archive_text(&p,TEXT,None).await.unwrap();assert!(!v.truncated);assert_eq!(hash(v.content.as_bytes()),m["textSha256"]);
}
#[tokio::test]
async fn sevenz1630_original_shared_rasters_and_non_first_volume_keep_exact_bytes(){
 let m=manifest();for name in ["contents","headers","headers-volumes"]{let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();let members=c["members"].as_array().unwrap();let p=root().join(name).join(members[members.len()/2]["name"].as_str().unwrap());let rev=image_reader::revision(&p).unwrap();for e in m["images"].as_array().unwrap(){let v=image_reader::read(&p,e["path"].as_str().unwrap(),Some(KEY),None,None,&rev,Control::default()).await.unwrap();assert_eq!(v.width as u64,e["width"]);assert_eq!(v.height as u64,e["height"]);assert_eq!(image_hash(&v),e["sha256"]);}}
}
#[test]
fn sevenz1630_shared_before_after_block_pack_crc_mapping_parameters_and_cancel_are_required(){
 for which in ["before-crc","after-crc","block-crc","pack-crc","size","mapping","reorder","order","memory","properties","extra-pack","missing-crc","packed-map","aes-size","power","aes-properties"] {
  let(mut input,mut a,i,f)=archive();let pack=a.stream_map.folder_first_pack_stream_index[f];let ppmd=a.folders[f].coders.iter().position(|c|c.decompression_method_id()==sevenz_rust::SevenZMethod::ID_PPMD).unwrap();let aes=a.folders[f].coders.iter().position(|c|c.decompression_method_id()==sevenz_rust::SevenZMethod::ID_AES256SHA256).unwrap();let indices:Vec<_>=a.files.iter().enumerate().filter(|(n,e)|e.has_stream&&a.stream_map.file_folder_index[*n]==Some(f)).map(|(n,_)|n).collect();let before=*indices.iter().find(|n|**n<i).unwrap();let after=*indices.iter().find(|n|**n>i).unwrap();
  match which {
   "before-crc"=>{a.files[before].has_crc=true;a.files[before].crc^=1;},"after-crc"=>{a.files[after].has_crc=true;a.files[after].crc^=1;},"block-crc"=>{a.folders[f].has_crc=true;a.folders[f].crc^=1;},"pack-crc"=>{a.pack_crcs_defined.insert(pack);a.pack_crcs.resize(a.pack_sizes.len(),0);a.pack_crcs[pack]=0;},"size"=>a.files[after].size+=1,"mapping"=>a.stream_map.file_folder_index[after]=None,"reorder"=>a.files.swap(i,before),
   "order"=>a.folders[f].coders[ppmd].properties[0]=1,"memory"=>a.folders[f].coders[ppmd].properties[1..5].copy_from_slice(&(65u32*1024*1024).to_le_bytes()),"properties"=>{a.folders[f].coders[ppmd].properties.pop();},"extra-pack"=>a.pack_sizes[pack]+=1,"missing-crc"=>{a.files[before].has_crc=false;a.folders[f].has_crc=false;},"packed-map"=>a.folders[f].packed_streams[0]=ppmd as u64,"aes-size"=>a.folders[f].unpack_sizes[aes]+=16,"power"=>a.folders[f].coders[aes].properties[0]=(a.folders[f].coders[aes].properties[0]&0xc0)|21,_=>{a.folders[f].coders[aes].properties.pop();}
  }
  assert!(sevenz_rust::with_preview_limits::<_,sevenz_rust::Error>(LIMIT,20,||Ok(()),||sevenz_rust::BlockDecoder::new(f,&a,sevenz_rust::Password::from(KEY).as_slice(),&mut input).read_solid_ppmd_entry(i,1024*1024,LIMIT,||Ok(()))).is_err(),"{which}");
 }
 // A valid folder-only CRC may cover all shared files; selected CRC alone may not.
 let(mut input,mut a,i,f)=archive();let mut crc=crc32fast::Hasher::new();for(n,e)in a.files.iter().enumerate().filter(|(n,e)|e.has_stream&&a.stream_map.file_folder_index[*n]==Some(f)){assert!(a.files[n].has_crc);crc.combine(&crc32fast::Hasher::new_with_initial_len(e.crc as u32,e.size));}a.folders[f].has_crc=true;a.folders[f].crc=crc.finalize() as u64;for(n,e)in a.files.iter_mut().enumerate(){if a.stream_map.file_folder_index[n]==Some(f){e.has_crc=false;}}let v=sevenz_rust::BlockDecoder::new(f,&a,sevenz_rust::Password::from(KEY).as_slice(),&mut input).read_solid_ppmd_entry(i,1024*1024,LIMIT,||Ok(())).unwrap();assert_eq!(hash(&v),manifest()["textSha256"]);
 let(mut input,a,i,f)=archive();let mut count=0;let e=sevenz_rust::BlockDecoder::new(f,&a,sevenz_rust::Password::from(KEY).as_slice(),&mut input).read_solid_ppmd_entry(i,1024*1024,LIMIT,||{count+=1;if count==3{Err(sevenz_rust::Error::other("预览已取消"))}else{Ok(())}}).unwrap_err();assert!(e.to_string().contains("取消"));
}
#[tokio::test]
async fn sevenz1630_damaged_shared_cipher_never_displays_text_or_pixels_and_unrelated_block_is_skipped(){
 let p=root().join("damaged-data.7z");let e=text_error(&p,TEXT,Some(KEY)).await;assert!(e.contains("密码")&&e.contains("未通过读取或校验"),"{e}");let e=preview::preview_archive_image(&p,IMAGE,Some(KEY)).await.err().unwrap();assert!(format!("{e:#}").contains("密码"));
 let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/sevenz-password-solid-ppmd-preview-1630/large-unrelated.7z");assert!(p.metadata().unwrap().len()>LIMIT);let m=manifest();assert_eq!(hash(preview::preview_archive_text(&p,TEXT,Some(KEY)).await.unwrap().content.as_bytes()),m["textSha256"]);assert_eq!(image_hash(&preview::preview_archive_image(&p,IMAGE,Some(KEY)).await.unwrap()),m["images"][0]["sha256"]);
}
#[tokio::test]
async fn sevenz1630_shared_and_image_boundaries_binary_large_and_password_scopes_keep_next_steps(){
 let v=preview::preview_archive_text(&root().join("at-shared-limit.7z"),"readme.txt",Some(KEY)).await.unwrap();assert_eq!(v.content,"Valid selected shared text");assert!(!v.truncated);let e=text_error(&root().join("oversize-shared.7z"),"readme.txt",Some(KEY)).await;assert!(e.contains("64 MiB")&&e.contains("先提取"),"{e}");
 assert!(text_error(&case("contents"),"notes/binary.txt",Some(KEY)).await.contains("binary"));
 let v=preview::preview_archive_image(&root().join("at-limit.7z"),"at-limit.bmp",Some(KEY)).await.unwrap();assert_eq!(v.byte_size,8*1024*1024);
 for(p,e,reason)in[(root().join("oversize.7z"),"oversize.bmp","8 MiB"),(root().join("dimensions.7z"),"dimensions.png","dimensions exceed"),(case("contents"),"gallery/03-bad.png","not a supported raster")]{let error=preview::preview_archive_image(&p,e,Some(KEY)).await.err().unwrap();assert!(format!("{error:#}").contains(reason),"{error:#}");}
 let(mut input,mut a,i,f)=archive();let pack=a.stream_map.folder_first_pack_stream_index[f];a.pack_sizes[pack]=LIMIT+1;assert!(sevenz_rust::BlockDecoder::new(f,&a,sevenz_rust::Password::from(KEY).as_slice(),&mut input).read_solid_ppmd_entry(i,1024*1024,LIMIT,||Ok(())).is_err());
}
#[tokio::test]
async fn sevenz1630_cancel_source_revision_missing_shared_tail_and_fresh_request(){
 let temp=tempfile::tempdir().unwrap();let p=temp.path().join("changed.7z");std::fs::copy(case("contents"),&p).unwrap();let rev=image_reader::revision(&p).unwrap();let v=preview::preview_archive_text_controlled_at(&p,TEXT,Some(KEY),None,None,Arc::new(AtomicBool::new(true)),Some(rev.clone())).await;assert!(format!("{:#}",v.err().unwrap()).contains("取消"));
 use std::io::Write;std::fs::OpenOptions::new().append(true).open(&p).unwrap().write_all(b"x").unwrap();let v=preview::preview_archive_text_controlled_at(&p,TEXT,Some(KEY),None,None,Arc::new(AtomicBool::new(false)),Some(rev)).await;assert!(format!("{:#}",v.err().unwrap()).contains("IMAGE_READER_CHANGED"));
 let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]=="headers-volumes").unwrap();let files:Vec<_>=c["members"].as_array().unwrap().iter().map(|f|{let n=f["name"].as_str().unwrap();let p=temp.path().join(n);std::fs::copy(root().join("headers-volumes").join(n),&p).unwrap();p}).collect();assert!(files.len()>2);let rev=image_reader::revision(&files[1]).unwrap();std::fs::remove_file(files.last().unwrap()).unwrap();let e=preview::preview_archive_text_controlled_at(&files[1],TEXT,Some(KEY),None,None,Arc::new(AtomicBool::new(false)),Some(rev)).await.err().unwrap();assert!(format!("{e:#}").contains("IMAGE_READER_CHANGED"));let v=preview::preview_archive_text(&case("contents"),TEXT,Some(KEY)).await.unwrap();assert!(!v.truncated);assert_eq!(hash(v.content.as_bytes()),m["textSha256"]);
}

#[tokio::test]
async fn sevenz1630_vault_selected_only_text_images_and_manual_wrong_never_record_archive_success(){
 use crate::models::password::{PasswordEntry,PasswordCategory};let temp=tempfile::tempdir().unwrap();let mut vault=EncryptedPasswordService::new(temp.path());vault.initialize("owned-public-password-ppmd-test").await.unwrap();
 for(name,key)in[("a wrong","wrong"),("b public",KEY)]{vault.add_password(PasswordEntry::new(name.into(),key.into(),PasswordCategory::Other)).await.unwrap();}
 let service=CompressionService::new(CompressionServiceConfig::default(),Arc::new(IOBufferPool::default()),Arc::new(RarSupportService::new()),Arc::new(UniversalCliEngine::new()),Arc::new(vault));
 let m=manifest();let big=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/sevenz-password-solid-ppmd-preview-1630/large-unrelated.7z");assert!(big.metadata().unwrap().len()>64*1024*1024);
 for p in [case("contents"),case("headers"),big]{let listing=service.browse_sevenz_with_vault(p.to_str().unwrap(),None).await.unwrap();assert!(listing.entries.iter().any(|e|e.path==TEXT));let v=service.preview_sevenz_text_with_vault(p.to_str().unwrap(),TEXT,None,None).await.unwrap();assert!(!v.truncated);assert_eq!(hash(v.content.as_bytes()),m["textSha256"]);let im=service.preview_sevenz_image_with_vault(p.to_str().unwrap(),IMAGE,None,Some(image_reader::revision(&p).unwrap()),Control::default()).await.unwrap();assert_eq!(image_hash(&im),m["images"][0]["sha256"]);assert!(service.preview_sevenz_text_with_vault(p.to_str().unwrap(),TEXT,Some("wrong manual"),None).await.is_err());assert!(service.preview_sevenz_image_with_vault(p.to_str().unwrap(),IMAGE,Some("wrong manual"),None,Control::default()).await.is_err());}
 assert!(service.encrypted_password_service.list_passwords().await.unwrap().iter().all(|e|e.use_count==0));
}
