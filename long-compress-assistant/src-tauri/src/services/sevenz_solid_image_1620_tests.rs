use super::{archive_preview as preview,image_reader::{self,Control}};
use std::{path::{Path,PathBuf},sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}}};
use base64::Engine;
use sha2::Digest;
const IMAGE:&str="gallery/01-cover.png";
fn root()->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sevenz-solid-image-1620")}
fn manifest()->serde_json::Value {serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn case(name:&str)->PathBuf {let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();root().join(name).join(c["members"][0]["name"].as_str().unwrap())}
fn hash(p:&preview::ArchiveImagePreview)->String {hex::encode(sha2::Sha256::digest(base64::engine::general_purpose::STANDARD.decode(p.data_url.split(',').nth(1).unwrap()).unwrap()))}

#[tokio::test]
async fn sevenz1620_solid_original_rasters_methods_and_every_numeric_member_are_exact() {
 let m=manifest();
 for name in ["lzma","lzma2"] {
  let p=case(name);let rev=image_reader::revision(&p).unwrap();
  for e in m["images"].as_array().unwrap() {
   let v=image_reader::read(&p,e["path"].as_str().unwrap(),None,None,None,&rev,Control::default()).await.unwrap();
   assert_eq!(hash(&v),e["sha256"].as_str().unwrap());assert_eq!((v.width,v.height),(e["width"].as_u64().unwrap() as u32,e["height"].as_u64().unwrap() as u32));
  }
 }
 let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]=="volumes").unwrap();
 for member in c["members"].as_array().unwrap() {let p=root().join("volumes").join(member["name"].as_str().unwrap());assert_eq!(hash(&preview::preview_archive_image(&p,IMAGE,None).await.unwrap()),m["images"][0]["sha256"].as_str().unwrap());}
}

#[tokio::test]
async fn sevenz1620_solid_output_shared_limits_bad_end_and_encryption_have_next_steps() {
 let m=manifest();let at=preview::preview_archive_image(&root().join("at-limit.7z"),"at-limit.bmp",None).await.unwrap();assert_eq!(at.byte_size,8*1024*1024);assert_eq!(hash(&at),m["limits"]["at-limit.bmp"]["sha256"].as_str().unwrap());
 for (p,entry,reason) in [(root().join("oversize.7z"),"oversize.bmp","8 MiB"),(root().join("oversize-shared.7z"),"preview.png","64 MiB"),(root().join("dimensions.7z"),"dimensions.png","dimensions exceed"),(root().join("bad-end.7z"),IMAGE,"CRC"),(case("lzma2"),"gallery/03-bad.png","not a supported raster")] {
  let error=preview::preview_archive_image(&p,entry,None).await.unwrap_err();assert!(format!("{error:#}").contains(reason),"{error:#}");
 }

 let large=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/sevenz-solid-image-1620/large-unrelated.7z");assert!(large.metadata().unwrap().len()>64*1024*1024);assert_eq!(hash(&preview::preview_archive_image(&large,IMAGE,None).await.unwrap()),m["images"][0]["sha256"].as_str().unwrap());
}

#[tokio::test]
async fn sevenz1620_shared_image_progress_and_mid_read_cancellation_do_not_publish_bytes() {
 let p=case("lzma2");let rev=image_reader::revision(&p).unwrap();let events=Arc::new(Mutex::new(vec![]));let observed=events.clone();let control=Control{observer:Arc::new(move|e|observed.lock().unwrap().push((e.phase,e.processed_bytes,e.total_bytes))),..Default::default()};
 image_reader::read(&p,IMAGE,None,None,None,&rev,control).await.unwrap();let events=events.lock().unwrap();assert!(events.iter().any(|e|e.0=="reading"&&e.1>0&&e.2==0));assert!(events.iter().any(|e|e.0=="decoding"));drop(events);
 let flag=Arc::new(AtomicBool::new(false));let signal=flag.clone();let control=Control{flag,observer:Arc::new(move|e|{if e.phase=="reading"&&e.processed_bytes>=4096 {signal.store(true,Ordering::Release)}})};
 let e=image_reader::read(&p,IMAGE,None,None,None,&rev,control).await.unwrap_err();assert!(format!("{e:#}").contains("取消")||format!("{e:#}").contains("CANCELLED"),"{e:#}");
 assert_eq!(image_reader::read(&p,IMAGE,None,None,None,&rev,Control::default()).await.unwrap().width,320);
}

#[tokio::test]
async fn sevenz1620_shared_source_changes_and_missing_volume_reject_stale_images() {
 let temp=tempfile::tempdir().unwrap();let p=temp.path().join("changed.7z");std::fs::copy(case("lzma2"),&p).unwrap();let rev=image_reader::revision(&p).unwrap();let changed=p.clone();
 let control=Control{observer:Arc::new(move|e|{if e.phase=="decoding"{use std::io::Write;std::fs::OpenOptions::new().append(true).open(&changed).unwrap().write_all(&[0]).unwrap();}}),..Default::default()};
 let e=preview::preview_archive_image_bound_at(&p,IMAGE,None,None,None,control,Some(rev.clone())).await.unwrap_err();assert!(format!("{e:#}").contains("IMAGE_READER_CHANGED"));assert!(image_reader::read(&p,IMAGE,None,None,None,&rev,Control::default()).await.is_err());
 let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["name"]=="volumes").unwrap();let members:Vec<_>=c["members"].as_array().unwrap().iter().map(|f|{let n=f["name"].as_str().unwrap();let p=temp.path().join(n);std::fs::copy(root().join("volumes").join(n),&p).unwrap();p}).collect();assert!(members.len()>2);let rev=image_reader::revision(&members[1]).unwrap();std::fs::remove_file(members.last().unwrap()).unwrap();let e=image_reader::read(&members[1],IMAGE,None,None,None,&rev,Control::default()).await.unwrap_err();assert!(format!("{e:#}").contains("IMAGE_READER_CHANGED"));assert!(preview::preview_archive_image(&members[1],IMAGE,None).await.is_err());
}
