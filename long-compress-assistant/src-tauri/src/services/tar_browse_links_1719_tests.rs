use super::{archive_browser,archive_preview,image_reader::{self,Control},tar_browse_links,tar_lzo_prefix_175_tests::wrapped};
use std::{path::{Path,PathBuf},sync::{Arc,atomic::{AtomicBool,Ordering}}};
use sha2::{Digest,Sha256};
fn root()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives")}
fn base()->PathBuf{let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/tar-browse-links-1719");std::fs::create_dir_all(&p).unwrap();p}
fn manifest(folder:&str)->serde_json::Value{serde_json::from_slice(&std::fs::read(root().join(folder).join("manifest.json")).unwrap()).unwrap()}
fn flag()->Arc<AtomicBool>{Arc::new(AtomicBool::new(false))}
fn sha(bytes:&[u8])->String{format!("{:x}",Sha256::digest(bytes))}
fn rewrite(bytes:&mut[u8],at:usize,mutate:impl FnOnce(&mut tar::Header)){let mut h=tar::Header::from_byte_slice(&bytes[at..at+512]).clone();mutate(&mut h);h.set_cksum();bytes[at..at+512].copy_from_slice(h.as_bytes());}
fn ext(kind:tar::EntryType,bytes:&[u8])->Vec<u8>{let mut h=tar::Header::new_gnu();h.set_path("././@LongLink").unwrap();h.set_entry_type(kind);h.set_size(bytes.len()as u64);h.set_cksum();let mut b=tar::Builder::new(Vec::new());b.append(&h,bytes).unwrap();let mut bytes=b.into_inner().unwrap();bytes.truncate(bytes.len()-1024);bytes}
fn pax(value:&str)->Vec<u8>{let mut b=tar::Builder::new(Vec::new());b.append_pax_extensions([("linkpath",value.as_bytes())]).unwrap();let mut bytes=b.into_inner().unwrap();bytes.truncate(bytes.len()-1024);bytes}

#[tokio::test]
async fn browse1719_prior_gnu_pax_and_sparse_links_keep_logical_sizes_names_and_independent_oracle_prefixes(){
 let m=manifest("tar-browse-links-1719");let mut reports=Vec::new();
 for folder in ["tar-hardlink-1717","tar-gnu-longlink-1718"]{let old=manifest(folder);let path=root().join(folder).join(old["archive"].as_str().unwrap());let report=archive_browser::browse_archive_cancellable(&path,None,flag()).await.unwrap();assert_eq!(report.entries.len(),old["entries"].as_array().unwrap().len());
  for(e,x)in report.entries.iter().zip(old["entries"].as_array().unwrap()){assert_eq!(e.path.trim_end_matches('/'),x["name"].as_str().unwrap());assert_eq!(e.size,x["bytes"].as_u64().unwrap());assert_eq!(e.link_target.as_deref(),x["linkTarget"].as_str());}
  for x in m["priorSparseLinkOracles"].as_array().unwrap().iter().filter(|x|x["folder"]==folder){let(bytes,total)=tar_browse_links::preview(&path,x["name"].as_str().unwrap(),1048576,true,&Default::default()).unwrap();assert_eq!(total,x["size"].as_u64().unwrap());assert_eq!(sha(&bytes),x["prefixSha256"].as_str().unwrap());}
  reports.push(serde_json::json!({"source":path,"report":report}));
 }std::fs::write(base().join("prior.json"),serde_json::to_vec_pretty(&reports).unwrap()).unwrap();
}

#[tokio::test]
async fn browse1719_gnu_pax_text_image_empty_and_long_name_previews_use_selected_alias_and_target_content(){
 let m=manifest("tar-browse-links-1719");let mut evidence=Vec::new();
 for v in m["variants"].as_array().unwrap(){let path=root().join("tar-browse-links-1719").join(v["archive"].as_str().unwrap());assert_eq!(sha(&std::fs::read(&path).unwrap()),v["sha256"].as_str().unwrap());let report=archive_browser::browse_archive_cancellable(&path,None,flag()).await.unwrap();assert_eq!(report.entries.len(),11);assert_eq!(report.entries.iter().filter(|e|e.link_target.is_some()).count(),4);
  for(e,x)in report.entries.iter().zip(v["entries"].as_array().unwrap()){assert_eq!(e.path.trim_end_matches('/'),x["name"].as_str().unwrap());assert_eq!(e.size,x["bytes"].as_u64().unwrap());assert_eq!(e.link_target.as_deref(),x["linkTarget"].as_str());}
  let mut texts=Vec::new();for id in [6,8,9]{let name=v["entries"][id]["name"].as_str().unwrap();let preview=archive_preview::preview_archive_text(&path,name,None).await.unwrap();assert_eq!(preview.entry_path,name);assert_eq!(preview.total_size,v["entries"][id]["bytes"].as_u64().unwrap());assert_eq!(sha(preview.content.as_bytes()),v["entries"][id]["prefixSha256"].as_str().unwrap());assert_eq!(preview.truncated,id==6);texts.push(preview);}
  let image=archive_preview::preview_archive_image(&path,"gallery/封面副本.png",None).await.unwrap();assert_eq!((image.width,image.height),(32,24));assert_eq!(image.entry_path,"gallery/封面副本.png");use base64::Engine;let data=base64::engine::general_purpose::STANDARD.decode(image.data_url.split(',').nth(1).unwrap()).unwrap();assert_eq!(sha(&data),v["entries"][7]["sha256"].as_str().unwrap());
  evidence.push(serde_json::json!({"source":path,"report":report,"texts":texts,"image":image,"revision":image_reader::revision(&path).unwrap()}));
 }std::fs::write(base().join("native.json"),serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
}

#[tokio::test]
async fn browse1719_missing_ambiguous_forward_chain_and_nonfile_targets_are_not_shown_as_empty_files(){
 let m=manifest("tar-gnu-longlink-1718");let original=std::fs::read(root().join("tar-gnu-longlink-1718/gnu-links.tar")).unwrap();let at=m["records"][4]["headerOffset"].as_u64().unwrap()as usize;let mut cases=Vec::new();
 for name in ["missing.bin","after.txt","alias-a.bin","资料","SOURCE.BIN","../source.bin"]{let mut bad=original.clone();rewrite(&mut bad,at,|h|h.set_link_name(name).unwrap());cases.push(bad);}
 for kind in [tar::EntryType::Symlink,tar::EntryType::Fifo]{let mut bad=original.clone();rewrite(&mut bad,at,|h|h.set_entry_type(kind));cases.push(bad);}
 let mut bad=original.clone();rewrite(&mut bad,at,|h|h.set_size(1));cases.push(bad);let mut bad=original.clone();rewrite(&mut bad,m["records"][5]["headerOffset"].as_u64().unwrap()as usize,|h|h.set_link_name("alias-a.bin").unwrap());cases.push(bad);
 let mut bad=original.clone();rewrite(&mut bad,m["records"][3]["headerOffset"].as_u64().unwrap()as usize,|h|h.set_path("prefix.txt").unwrap());cases.push(bad);
 let temp=tempfile::tempdir().unwrap();let path=temp.path().join("invalid.tar.lzo");let mut reasons=Vec::new();for bad in cases{std::fs::write(&path,wrapped(&bad,100003,true)).unwrap();let error=archive_browser::browse_archive_cancellable(&path,None,flag()).await.unwrap_err();assert!(format!("{error:#}").contains("[archive-inspection:unsupported]"));reasons.push(format!("{error:#}"));}std::fs::write(base().join("invalid-targets.json"),serde_json::to_vec_pretty(&reasons).unwrap()).unwrap();
}

#[tokio::test]
async fn browse1719_gnu_controls_validate_complete_payloads_both_orders_and_matching_pax_once(){
 let m=manifest("tar-gnu-longlink-1718");let raw=std::fs::read(root().join("tar-gnu-longlink-1718/gnu-links.tar")).unwrap();let r=&m["records"][7];let k=r["metadata"][0]["offset"].as_u64().unwrap()as usize;let l=r["metadata"][1]["offset"].as_u64().unwrap()as usize;let h=r["headerOffset"].as_u64().unwrap()as usize;let target=m["entries"][7]["linkTarget"].as_str().unwrap();let mut good=Vec::new();let mut swap=raw.clone();swap.splice(k..h,[&raw[l..h],&raw[k..l]].concat());good.push(swap);
 for first in [true,false]{let mut data=raw.clone();let at=if first{k}else{h};data.splice(at..at,pax(target));good.push(data);}
 let temp=tempfile::tempdir().unwrap();let path=temp.path().join("controls.tar.lzo");for data in good{std::fs::write(&path,wrapped(&data,100003,true)).unwrap();let list=archive_browser::browse_archive_cancellable(&path,None,flag()).await.unwrap();assert_eq!(list.entries.len(),15);assert_eq!(list.entries[7].link_target.as_deref(),Some(target));assert!(list.entries[8].link_target.is_none());}
 let mut bads=Vec::new();for payload in [b"\0".as_slice(),b"missing-nul",b"name\0extra"]{let mut bad=raw.clone();bad.splice(k..l,ext(tar::EntryType::GNULongLink,payload));bads.push(bad);}let mut bad=raw.clone();bad[l-1]=1;bads.push(bad);let mut bad=raw.clone();bad.splice(k..k,raw[k..l].iter().copied());bads.push(bad);let mut bad=raw.clone();bad.splice(h..h,pax("source.bin"));bads.push(bad);let mut bad=raw[..l].to_vec();bad.extend([0;1024]);bads.push(bad);let mut bad=raw.clone();rewrite(&mut bad,h,|e|e.set_entry_type(tar::EntryType::Regular));bads.push(bad);
 for cut in [k+511,l-1,h+511]{bads.push(raw[..cut].to_vec());}for data in bads{std::fs::write(&path,wrapped(&data,100003,true)).unwrap();assert!(archive_browser::browse_archive_cancellable(&path,None,flag()).await.is_err());}
 std::fs::write(&path,wrapped(&raw,100003,true)).unwrap();let bytes=std::fs::read(&path).unwrap();std::fs::write(&path,&bytes[..bytes.len()-2]).unwrap();assert!(archive_browser::browse_archive_cancellable(&path,None,flag()).await.is_err());
}

#[test]
fn browse1719_replay_shares_scan_budget_and_large_images_are_rejected_before_materializing(){
 let path=root().join("tar-browse-links-1719/gnu.tar.lzo");let control=Control::default();let normal=tar_browse_links::preview_with_budget(&path,"gallery/cover.png",8*1024*1024,false,&control,2*1024*1024).unwrap();assert!(!normal.0.is_empty());let error=tar_browse_links::preview_with_budget(&path,"gallery/封面副本.png",8*1024*1024,false,&control,2*1024*1024).unwrap_err();assert!(format!("{error:#}").contains("累计扫描"));
 let error=tar_browse_links::preview(&root().join("tar-gnu-longlink-1718/gnu-links.tar.lzo"),"稀疏副本.bin",8*1024*1024,false,&control).unwrap_err();assert!(format!("{error:#}").contains("8 MiB"));
}

#[tokio::test]
async fn browse1719_cancellation_and_source_change_during_alias_lookup_leave_no_cached_outputs(){
 let temp=tempfile::tempdir().unwrap();let path=temp.path().join("source.tar.lzo");std::fs::copy(root().join("tar-browse-links-1719/gnu.tar.lzo"),&path).unwrap();let cancel=flag();let triggered=cancel.clone();let control=Control{flag:cancel.clone(),observer:Arc::new(move|p|if p.processed_bytes>4096{triggered.store(true,Ordering::SeqCst)})};let error=archive_preview::preview_archive_image_controlled_at(&path,"gallery/封面副本.png",None,None,None,control).await.unwrap_err();assert!(format!("{error:#}").contains("CANCELLED"));
 let changed=path.clone();let once=Arc::new(AtomicBool::new(false));let time=filetime::FileTime::from_unix_time(1800001719,0);let control=Control{observer:Arc::new(move|p|if p.processed_bytes>4096&&!once.swap(true,Ordering::SeqCst){filetime::set_file_mtime(&changed,time).unwrap()}),..Default::default()};let error=archive_preview::preview_archive_image_controlled_at(&path,"gallery/封面副本.png",None,None,None,control).await.unwrap_err();assert!(format!("{error:#}").contains("IMAGE_READER_CHANGED"));
 assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(),1);let old="a".repeat(64);let error=archive_preview::preview_archive_text_controlled_at(&path,"阅读副本.txt",None,None,None,flag(),Some(old)).await.unwrap_err();assert!(format!("{error:#}").contains("IMAGE_READER_CHANGED"));
}

#[test]
fn browse1719_readonly_links_do_not_silently_enable_normal_extraction_or_create_system_links(){
 let path=root().join("tar-browse-links-1719/gnu.tar.lzo");let runtime=super::tar_lzo_1658_tests::Runtime(super::io_buffer_pool::IOBufferPool::new(Default::default()));let temp=tempfile::tempdir().unwrap();let options=crate::models::compression::DecompressOptions{selected_entries:vec!["阅读副本.txt".into()],..Default::default()};let error=super::native_extraction::tar::extract_inner(&runtime,None,"browse1719",path.to_str().unwrap(),temp.path(),Some(super::codec_stream::decoder(&path,"lzo").unwrap()),&options).unwrap_err();assert!(format!("{error:#}").contains("硬链接"));assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(),0);
}
