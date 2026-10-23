use std::{path::{Path,PathBuf},sync::{Arc,atomic::AtomicBool}};
use sha2::{Digest,Sha256};use base64::Engine;
use super::{archive_preview as preview,image_reader::{self,Control},compression_service::{CompressionService,CompressionServiceConfig},encrypted_password_service::EncryptedPasswordService,io_buffer_pool::IOBufferPool,rar_support::RarSupportService,universal_engine::UniversalCliEngine};
const TEXT:&str="notes/说明 [1].txt";const IMAGE:&str="gallery/01-cover.png";const KEY:&str="Deflate64公开测试密码1639!";
fn root()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-password-deflate64-preview-1639")}
fn manifest()->serde_json::Value{serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn case(name:&str)->PathBuf{let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();root().join(name).join(c["members"][0]["name"].as_str().unwrap())}
fn hash(b:&[u8])->String{hex::encode(Sha256::digest(b))}
fn image_hash(v:&preview::ArchiveImagePreview)->String{hash(&base64::engine::general_purpose::STANDARD.decode(v.data_url.split(',').nth(1).unwrap()).unwrap())}
async fn text_error(p:&Path,e:&str,key:Option<&str>)->String{match preview::preview_archive_text(p,e,key).await{Err(e)=>format!("{e:#}"),Ok(v)=>panic!("unexpected successful {} bytes",v.byte_size)}}
fn archive()->(std::fs::File,sevenz_rust::Archive,usize,usize){let mut input=std::fs::File::open(case("contents")).unwrap();let len=input.metadata().unwrap().len();let a=sevenz_rust::Archive::read(&mut input,len,sevenz_rust::Password::from(KEY).as_slice()).unwrap();let i=a.files.iter().position(|e|e.name==TEXT).unwrap();let f=a.stream_map.file_folder_index[i].unwrap();(input,a,i,f)}

#[tokio::test]
async fn sevenz1639_original_content_header_orders_every_numeric_member_and_text_encodings(){
 let m=manifest();for c in m["cases"].as_array().unwrap(){for member in c["members"].as_array().unwrap(){let p=root().join(c["name"].as_str().unwrap()).join(member["name"].as_str().unwrap());let v=preview::preview_archive_text(&p,TEXT,Some(KEY)).await.unwrap();assert!(!v.truncated);assert_eq!(v.byte_size,m["textBytes"]);assert_eq!(hash(v.content.as_bytes()),m["textSha256"]);}}
 for(e,text)in[("notes/说明 1.txt","NOT the selected bracket path"),("notes/empty.txt",""),("notes/utf16.txt","完整 UTF16 文本"),("notes/gbk.txt","中文编码说明")]{let v=preview::preview_archive_text(&case("headers"),e,Some(KEY)).await.unwrap();assert!(!v.truncated);assert_eq!(v.content,text);}
 let v=preview::preview_archive_text(&case("contents"),"notes/exact.txt",Some(KEY)).await.unwrap();assert!(!v.truncated);assert_eq!(v.byte_size,1024*1024);
 // One ordinary Deflate64 continuity case for the extended complete reader.
 let old=Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-deflate64-preview-1638");let om:serde_json::Value=serde_json::from_slice(&std::fs::read(old.join("manifest.json")).unwrap()).unwrap();let oc=&om["cases"][0];let p=old.join(oc["name"].as_str().unwrap()).join(oc["members"][0]["name"].as_str().unwrap());let v=preview::preview_archive_text(&p,TEXT,None).await.unwrap();assert!(!v.truncated);assert_eq!(hash(v.content.as_bytes()),om["textSha256"]);
}
#[tokio::test]
async fn sevenz1639_five_original_rasters_and_non_first_encrypted_volume_keep_exact_bytes(){
 let m=manifest();for name in ["contents","headers","headers-volumes"]{let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();let members=c["members"].as_array().unwrap();let p=root().join(name).join(members[members.len()/2]["name"].as_str().unwrap());let rev=image_reader::revision(&p).unwrap();for e in m["images"].as_array().unwrap(){let v=image_reader::read(&p,e["path"].as_str().unwrap(),Some(KEY),None,None,&rev,Control::default()).await.unwrap();assert_eq!(v.width as u64,e["width"]);assert_eq!(v.height as u64,e["height"]);assert_eq!(image_hash(&v),e["sha256"]);}}
}
#[tokio::test]
async fn sevenz1639_vault_selected_only_text_images_and_manual_wrong_never_record_archive_success(){
 use crate::models::password::{PasswordEntry,PasswordCategory};let temp=tempfile::tempdir().unwrap();let mut vault=EncryptedPasswordService::new(temp.path());vault.initialize("owned-public-password-ppmd-test").await.unwrap();
 for(name,key)in[("a wrong","wrong"),("b public",KEY)]{vault.add_password(PasswordEntry::new(name.into(),key.into(),PasswordCategory::Other)).await.unwrap();}
 let service=CompressionService::new(CompressionServiceConfig::default(),Arc::new(IOBufferPool::default()),Arc::new(RarSupportService::new()),Arc::new(UniversalCliEngine::new()),Arc::new(vault));
 let m=manifest();let big=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/sevenz-password-deflate64-preview-1639/large-unrelated.7z");assert!(big.metadata().unwrap().len()>64*1024*1024);
 for p in [case("contents"),case("headers"),big]{let listing=service.browse_sevenz_with_vault(p.to_str().unwrap(),None).await.unwrap();assert!(listing.entries.iter().any(|e|e.path==TEXT));let v=service.preview_sevenz_text_with_vault(p.to_str().unwrap(),TEXT,None,None).await.unwrap();assert!(!v.truncated);assert_eq!(hash(v.content.as_bytes()),m["textSha256"]);let im=service.preview_sevenz_image_with_vault(p.to_str().unwrap(),IMAGE,None,Some(image_reader::revision(&p).unwrap()),Control::default()).await.unwrap();assert_eq!(image_hash(&im),m["images"][0]["sha256"]);assert!(service.preview_sevenz_text_with_vault(p.to_str().unwrap(),TEXT,Some("wrong manual"),None).await.is_err());assert!(service.preview_sevenz_image_with_vault(p.to_str().unwrap(),IMAGE,Some("wrong manual"),None,Control::default()).await.is_err());}
 assert!(service.encrypted_password_service.list_passwords().await.unwrap().iter().all(|e|e.use_count==0));
}
#[test]
fn sevenz1639_complete_crc_mapping_aes_lengths_parameters_and_mid_read_cancel_are_required(){
 for which in ["entry-crc","folder-crc","pack-crc","size","mapping","properties","packed-map","extra-pack","missing-crc","aes-size","power","aes-properties"] {
  let(mut input,mut a,i,f)=archive();let pack=a.stream_map.folder_first_pack_stream_index[f];let p=a.folders[f].coders.iter().position(|c|c.decompression_method_id()==sevenz_rust::SevenZMethod::ID_DEFLATE64).unwrap();let aes=a.folders[f].coders.iter().position(|c|c.decompression_method_id()==sevenz_rust::SevenZMethod::ID_AES256SHA256).unwrap();
  match which {
   "entry-crc"=>{a.files[i].has_crc=true;a.files[i].crc^=1;},"folder-crc"=>{a.folders[f].has_crc=true;a.folders[f].crc^=1;},"pack-crc"=>{a.pack_crcs_defined.insert(pack);a.pack_crcs.resize(a.pack_sizes.len(),0);a.pack_crcs[pack]=0;},"size"=>{a.files[i].size-=1;a.folders[f].unpack_sizes[p]-=1;},"mapping"=>a.stream_map.file_folder_index[i]=None,
   "properties"=>a.folders[f].coders[p].properties.push(1),"packed-map"=>a.folders[f].packed_streams[0]=p as u64,"extra-pack"=>a.pack_sizes[pack]+=16,"missing-crc"=>{a.files[i].has_crc=false;a.folders[f].has_crc=false;},"aes-size"=>a.folders[f].unpack_sizes[aes]+=16,"power"=>a.folders[f].coders[aes].properties[0]=(a.folders[f].coders[aes].properties[0]&0xc0)|21,_=>{a.folders[f].coders[aes].properties.pop();}
  }
  assert!(sevenz_rust::with_preview_limits::<_,sevenz_rust::Error>(64*1024*1024,20,||Ok(()),||sevenz_rust::BlockDecoder::new(f,&a,sevenz_rust::Password::from(KEY).as_slice(),&mut input).read_deflate64_entry(i,1024*1024,||Ok(()))).is_err(),"{which}");
 }
 let(mut input,a,i,f)=archive();let mut count=0;let e=sevenz_rust::BlockDecoder::new(f,&a,sevenz_rust::Password::from(KEY).as_slice(),&mut input).read_deflate64_entry(i,1024*1024,||{count+=1;if count==3{Err(sevenz_rust::Error::other("预览已取消"))}else{Ok(())}}).unwrap_err();assert!(e.to_string().contains("取消"));
}
#[tokio::test]
async fn sevenz1639_encrypted_limits_binary_solid_large_and_damage_keep_next_steps(){
 assert!(text_error(&case("contents"),"notes/binary.txt",Some(KEY)).await.contains("binary"));let e=text_error(&root().join("damaged-data.7z"),TEXT,Some(KEY)).await;assert!(e.contains("密码")&&e.contains("未通过读取或校验"),"{e}");


 let v=preview::preview_archive_image(&root().join("at-limit.7z"),"at-limit.bmp",Some(KEY)).await.unwrap();assert_eq!(v.byte_size,8*1024*1024);
 for(p,e,reason)in[(root().join("oversize.7z"),"oversize.bmp","8 MiB"),(root().join("dimensions.7z"),"dimensions.png","dimensions exceed"),(case("contents"),"gallery/03-bad.png","not a supported raster")]{let error=preview::preview_archive_image(&p,e,Some(KEY)).await.err().unwrap();assert!(format!("{error:#}").contains(reason),"{error:#}");}
}
#[tokio::test]
async fn sevenz1639_cancel_source_revision_missing_encrypted_tail_and_fresh_request(){
 let temp=tempfile::tempdir().unwrap();let p=temp.path().join("changed.7z");std::fs::copy(case("headers"),&p).unwrap();let rev=image_reader::revision(&p).unwrap();let v=preview::preview_archive_text_controlled_at(&p,TEXT,Some(KEY),None,None,Arc::new(AtomicBool::new(true)),Some(rev.clone())).await;assert!(format!("{:#}",v.err().unwrap()).contains("取消"));
 use std::io::Write;std::fs::OpenOptions::new().append(true).open(&p).unwrap().write_all(b"x").unwrap();let v=preview::preview_archive_text_controlled_at(&p,TEXT,Some(KEY),None,None,Arc::new(AtomicBool::new(false)),Some(rev)).await;assert!(format!("{:#}",v.err().unwrap()).contains("IMAGE_READER_CHANGED"));
 let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]=="headers-volumes").unwrap();let files:Vec<_>=c["members"].as_array().unwrap().iter().map(|f|{let n=f["name"].as_str().unwrap();let p=temp.path().join(n);std::fs::copy(root().join("headers-volumes").join(n),&p).unwrap();p}).collect();assert!(files.len()>2);let rev=image_reader::revision(&files[1]).unwrap();std::fs::remove_file(files.last().unwrap()).unwrap();let e=preview::preview_archive_text_controlled_at(&files[1],TEXT,Some(KEY),None,None,Arc::new(AtomicBool::new(false)),Some(rev)).await.err().unwrap();assert!(format!("{e:#}").contains("IMAGE_READER_CHANGED"));let v=preview::preview_archive_text(&case("headers"),TEXT,Some(KEY)).await.unwrap();assert!(!v.truncated);assert_eq!(hash(v.content.as_bytes()),m["textSha256"]);
}
