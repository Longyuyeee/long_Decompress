use super::{lzop::recovery,tar_lzo_prefix_175_tests::wrapped};
use sha2::{Digest,Sha256};
use std::{path::{Path,PathBuf},io::{Read,Cursor},sync::{atomic::{AtomicBool,Ordering},Arc}};
fn flag()->AtomicBool {AtomicBool::new(false)}
fn root()->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/tar-sparse-1713")}
fn base()->PathBuf {let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/tar-sparse-1713");std::fs::create_dir_all(&p).unwrap();p}
fn manifest()->serde_json::Value {serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn sha(data:&[u8])->String {format!("{:x}",Sha256::digest(data))}
fn digest(path:&Path)->String {let mut file=std::fs::File::open(path).unwrap();let mut buf=[0;65536];let mut hash=Sha256::new();loop{let n=file.read(&mut buf).unwrap();if n==0{break}hash.update(&buf[..n])}format!("{:x}",hash.finalize())}
fn scan(path:&Path)->recovery::Report {recovery::scan(path,None,&flag(),|_,_|{}).unwrap()}
fn no_stage(path:&Path){assert!(!std::fs::read_dir(path).unwrap().any(|e|e.unwrap().file_name().to_string_lossy().starts_with(".long-lzop-recovery")))}
fn exact(result:&recovery::Outcome){for e in &result.entries{let p=Path::new(&result.output_directory).join(&e.name);if e.is_directory{assert!(p.is_dir());continue}assert_eq!(p.metadata().unwrap().len(),e.bytes);assert_eq!(digest(&p),e.sha256);assert!(e.time_warning.is_none());if let Some(stored)=e.stored_bytes{assert!(stored<e.bytes);#[cfg(windows)]{use std::os::windows::fs::MetadataExt;assert_ne!(p.metadata().unwrap().file_attributes()&0x200,0);}}}}
fn rewrite(data:&mut [u8],at:usize,mutate:impl FnOnce(&mut tar::Header)){let mut h=tar::Header::from_byte_slice(&data[at..at+512]).clone();mutate(&mut h);h.set_cksum();data[at..at+512].copy_from_slice(h.as_bytes());}

#[tokio::test]
async fn sparse1713_complete_original_keeps_normal_browse_integrity_and_healthy_diagnosis(){
 let m=manifest();let source=root().join(m["archive"].as_str().unwrap());assert_eq!(sha(&std::fs::read(&source).unwrap()),m["sha256"].as_str().unwrap());let report=scan(&source);assert!(report.complete,"{report:?}");assert_eq!(report.entries.len(),9);assert_eq!(report.entries.iter().filter(|e|e.sparse.is_some()).count(),4);
 for(e,x)in report.entries.iter().zip(m["entries"].as_array().unwrap()){assert_eq!(e.name,x["name"].as_str().unwrap());assert_eq!(e.bytes,x["bytes"].as_u64().unwrap());assert_eq!(e.sha256,x["sha256"].as_str().unwrap());}
 let list=super::archive_browser::browse_archive(&source,None).await.unwrap();assert_eq!(list.entries.len(),9);for(e,x)in list.entries.iter().zip(m["entries"].as_array().unwrap()){assert_eq!(e.path.trim_end_matches('/'),x["name"].as_str().unwrap());assert_eq!(e.size,x["bytes"].as_u64().unwrap());}
 super::universal_engine::UniversalCliEngine::test_integrity(&source,None).await.unwrap();let diagnosis=super::archive_diagnostics::diagnose_archive(&source,None,Arc::new(flag())).await.unwrap();assert_eq!(diagnosis.status,"healthy");
}

#[tokio::test]
async fn sparse1713_tail_checksum_and_end_marker_failures_recover_full_sparse_contents_and_following_file(){
 let m=manifest();let original=std::fs::read(root().join(m["archive"].as_str().unwrap())).unwrap();let run=base().join(uuid::Uuid::new_v4().to_string());std::fs::create_dir_all(&run).unwrap();let mut evidence=Vec::new();
 for mode in ["tail","checksum","end"]{let mut bytes=original.clone();match mode{"tail"=>bytes.truncate(bytes.len()-400000),"checksum"=>{let n=bytes.len()-5;bytes[n]^=1;},_=>bytes.truncate(bytes.len()-2)}let path=run.join(format!("{mode}.tar.lzo"));std::fs::write(&path,&bytes).unwrap();let report=scan(&path);let count=if mode=="end"{9}else{8};assert!(!report.complete);assert_eq!(report.entries.len(),count,"{mode}: {report:?}");let ids=report.entries.iter().map(|e|e.id).collect::<Vec<_>>();let result=recovery::recover(&path,&run,&report.revision,&ids,None,&flag(),|_,_,_|{}).unwrap();exact(&result);assert!(Path::new(&result.output_directory).join("after.txt").is_file());assert_eq!(result.time_preservation.as_ref().unwrap().preserved_directories,Some(2));no_stage(&run);
  let mut record=serde_json::json!({"mode":mode,"source":path,"report":report,"result":result,"entries":&m["entries"].as_array().unwrap()[..count]});if mode=="tail"{let diagnosis=super::archive_diagnostics::diagnose_archive(&path,None,Arc::new(flag())).await.unwrap();assert_eq!(diagnosis.status,"partial_recovery");assert_eq!(diagnosis.total_files,6);record["diagnosis"]=serde_json::to_value(diagnosis).unwrap();}
  evidence.push(record);
  if mode=="tail"{let ids=[2,4,6];let selected=recovery::recover(&path,&run,&report.revision,&ids,None,&flag(),|_,_,_|{}).unwrap();exact(&selected);assert_eq!(selected.recovered_files,2);assert_eq!(selected.recovered_directories,1);evidence.push(serde_json::json!({"mode":"selected","source":path,"report":report,"result":selected,"entries":ids.iter().map(|id|m["entries"][id].clone()).collect::<Vec<_>>()}));}
 }
 assert_eq!(std::fs::read(root().join(m["archive"].as_str().unwrap())).unwrap(),original);std::fs::write(base().join("native.json"),serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
}

#[test]
fn sparse1713_incomplete_maps_extents_and_padding_and_invalid_ranges_stop_at_confirmed_prefix(){
 let m=manifest();let raw=std::fs::read(root().join(m["tar"].as_str().unwrap())).unwrap();let record=m["records"].as_array().unwrap().iter().find(|r|r["logicalIndex"]==4).unwrap();let at=record["headerOffset"].as_u64().unwrap()as usize;let data=record["dataOffset"].as_u64().unwrap()as usize;let stored=record["storedBytes"].as_u64().unwrap()as usize;let extensions=record["extensionOffsets"].as_array().unwrap();assert!(extensions.len()>=2);let temp=tempfile::tempdir().unwrap();let path=temp.path().join("boundary.tar.lzo");let mut checks=Vec::new();
 for cut in [at+100,extensions[0].as_u64().unwrap()as usize+100,data-1,data+stored/2,data+stored-1,data+stored.div_ceil(512)*512-1]{std::fs::write(&path,wrapped(&raw[..cut],137,true)).unwrap();let report=scan(&path);assert_eq!(report.entries.len(),4,"cut {cut}: {report:?}");assert!(!report.complete);checks.push(serde_json::json!({"cut":cut,"confirmed":4,"reason":report.stop_reason}));}
 let mut cases=Vec::new();let mut bad=raw.clone();rewrite(&mut bad,at,|h|h.as_gnu_mut().unwrap().isextended[0]=2);cases.push(bad);
 let mut bad=raw.clone();rewrite(&mut bad,at,|h|h.as_gnu_mut().unwrap().sparse[0].offset.fill(0));cases.push(bad);
 let mut bad=raw.clone();rewrite(&mut bad,at,|h|h.as_gnu_mut().unwrap().sparse[1].set_offset(0));cases.push(bad);
 let mut bad=raw.clone();rewrite(&mut bad,at,|h|h.as_gnu_mut().unwrap().sparse[1].set_offset(9*1024*1024));cases.push(bad);
 let mut bad=raw.clone();rewrite(&mut bad,at,|h|h.set_size(stored as u64+1));cases.push(bad);
 let mut bad=raw.clone();rewrite(&mut bad,at,|h|h.as_gnu_mut().unwrap().set_real_size(super::extraction_transaction::MAX_EXTRACTED_BYTES+1));cases.push(bad);
 let last=extensions.last().unwrap().as_u64().unwrap()as usize;let mut bad=raw.clone();bad[last+504]=2;cases.push(bad);let mut bad=raw.clone();bad[last+505]=1;cases.push(bad);
 let mut pax=tar::Builder::new(Vec::new());pax.append_pax_extensions([("size",stored.to_string().as_bytes())]).unwrap();let mut control=pax.into_inner().unwrap();control.truncate(control.len()-1024);let mut bad=raw.clone();bad.splice(at..at,control);cases.push(bad);
 for bytes in cases{std::fs::write(&path,wrapped(&bytes,137,true)).unwrap();let report=scan(&path);assert_eq!(report.entries.len(),4,"{report:?}");assert!(!report.complete);checks.push(serde_json::json!({"confirmed":4,"reason":report.stop_reason}));}
 let report=scan(&path);let result=recovery::recover(&path,temp.path(),&report.revision,&[0,1,2,3],None,&flag(),|_,_,_|{}).unwrap();exact(&result);assert_eq!(result.recovered_files,2);no_stage(temp.path());std::fs::write(base().join("boundaries.json"),serde_json::to_vec_pretty(&checks).unwrap()).unwrap();
}

#[test]
fn sparse1713_hole_hash_and_copy_cancel_and_destination_failure_leave_no_published_half_file(){
 let m=manifest();let original=std::fs::read(root().join(m["archive"].as_str().unwrap())).unwrap();let temp=tempfile::tempdir().unwrap();let path=temp.path().join("cancel.tar.lzo");std::fs::write(&path,&original[..original.len()-2]).unwrap();let report=scan(&path);let entry=&report.entries[6];let layout=entry.sparse.as_ref().unwrap();let raw=std::fs::read(root().join("gnu.tar")).unwrap();let offset=m["records"].as_array().unwrap().iter().find(|r|r["logicalIndex"]==6).unwrap()["dataOffset"].as_u64().unwrap()as usize;let mut input=Cursor::new(&raw[offset..]);let cancel=flag();let error=layout.process(&mut input,None,&cancel,|logical|if logical>=16*1024*1024{cancel.store(true,Ordering::SeqCst)}).unwrap_err();assert!(format!("{error:#}").contains("cancelled"));assert_eq!(input.position(),0);
 let cancel=flag();let mut active=false;let error=recovery::recover(&path,temp.path(),&report.revision,&[6],None,&cancel,|phase,done,_|if phase=="recovering"&&done>=16*1024*1024{active=true;cancel.store(true,Ordering::SeqCst)}).unwrap_err();assert!(active);assert!(format!("{error:#}").contains("cancelled"));no_stage(temp.path());assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(),1);
 let target=temp.path().join("readonly-target");std::fs::write(&target,[]).unwrap();let mut file=std::fs::File::open(&target).unwrap();let error=layout.process(&mut Cursor::new([]),Some(&mut file),&flag(),|_|{}).unwrap_err();assert!(!format!("{error:#}").contains("cancelled"));assert_eq!(std::fs::read(&path).unwrap(),&original[..original.len()-2]);
}

#[test]
fn sparse1713_map_revision_binds_equal_logical_contents_even_when_source_size_and_time_match(){
 let raw=std::fs::read(root().join("gnu.tar")).unwrap();let temp=tempfile::tempdir().unwrap();let path=temp.path().join("revision.tar.lzo");let data=wrapped(&raw,137,true);std::fs::write(&path,&data[..data.len()-2]).unwrap();let before=scan(&path);let time=filetime::FileTime::from_last_modification_time(&path.metadata().unwrap());let m=manifest();let at=m["records"].as_array().unwrap().iter().find(|r|r["logicalIndex"]==5).unwrap()["headerOffset"].as_u64().unwrap()as usize;
 let mut changed=raw;rewrite(&mut changed,at,|h|for slot in &mut h.as_gnu_mut().unwrap().sparse{slot.offset.fill(0);slot.numbytes.fill(0)});let data=wrapped(&changed,137,true);std::fs::write(&path,&data[..data.len()-2]).unwrap();filetime::set_file_mtime(&path,time).unwrap();let after=scan(&path);assert_ne!(before.revision,after.revision);let mut left=serde_json::to_value(&before).unwrap();let mut right=serde_json::to_value(&after).unwrap();left["revision"]="".into();right["revision"]="".into();assert_eq!(left,right);
 assert!(recovery::recover(&path,temp.path(),&before.revision,&[5],None,&flag(),|_,_,_|{}).unwrap_err().to_string().contains("重新检查"));no_stage(temp.path());
}

#[test]
fn sparse1713_missing_middle_volume_recovers_sparse_prefix_and_rechecks_after_repair(){
 let original=std::fs::read(root().join("gnu.tar.lzo")).unwrap();let temp=tempfile::tempdir().unwrap();let mut paths=Vec::new();for(i,bytes)in original.chunks(100003).enumerate(){let path=temp.path().join(format!("sparse.tar.lzo.{:03}",i+1));std::fs::write(&path,bytes).unwrap();paths.push(path);}let missing=std::fs::read(&paths[3]).unwrap();std::fs::remove_file(&paths[3]).unwrap();let path=paths.last().unwrap();let report=scan(path);assert_eq!(report.available_volumes,3);assert!(report.ignored_volumes>0);assert_eq!(report.entries.len(),8,"{report:?}");let result=recovery::recover(path,temp.path(),&report.revision,&[3,4,5,6,7],None,&flag(),|_,_,_|{}).unwrap();exact(&result);assert_eq!(result.recovered_files,5);
 std::fs::write(&paths[3],missing).unwrap();assert!(recovery::recover(path,temp.path(),&report.revision,&[0],None,&flag(),|_,_,_|{}).unwrap_err().to_string().contains("重新检查"));assert!(scan(path).complete);assert_eq!(paths.iter().flat_map(|p|std::fs::read(p).unwrap()).collect::<Vec<_>>(),original);no_stage(temp.path());
}

#[test]
fn sparse1713_stored_payload_padding_is_required_even_when_logical_holes_are_complete(){
 let mut h=tar::Header::new_gnu();h.set_path("partial-block.bin").unwrap();h.set_entry_type(tar::EntryType::GNUSparse);h.set_mode(0o644);h.set_size(3);h.set_mtime(1700001713);let g=h.as_gnu_mut().unwrap();g.set_real_size(10);g.sparse[0].set_offset(2);g.sparse[0].set_length(3);h.set_cksum();let mut builder=tar::Builder::new(Vec::new());builder.append(&h,b"abc".as_slice()).unwrap();let raw=builder.into_inner().unwrap();let temp=tempfile::tempdir().unwrap();let path=temp.path().join("padding.tar.lzo");
 for cut in[515,1023]{std::fs::write(&path,wrapped(&raw[..cut],137,true)).unwrap();assert!(scan(&path).entries.is_empty());}
 let mut bad=raw.clone();bad[1023]=1;std::fs::write(&path,wrapped(&bad,137,true)).unwrap();assert!(scan(&path).entries.is_empty());
 let data=wrapped(&raw,137,true);std::fs::write(&path,&data[..data.len()-2]).unwrap();let report=scan(&path);assert_eq!(report.entries.len(),1);let result=recovery::recover(&path,temp.path(),&report.revision,&[0],None,&flag(),|_,_,_|{}).unwrap();assert_eq!(std::fs::read(Path::new(&result.output_directory).join("partial-block.bin")).unwrap(),b"\0\0abc\0\0\0\0\0");exact(&result);no_stage(temp.path());
}
