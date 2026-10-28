use super::{lzop::recovery,tar_lzo_prefix_175_tests::wrapped};
use sha2::{Digest,Sha256};
use std::{path::{Path,PathBuf},io::{Read},sync::{atomic::{AtomicBool,Ordering},Arc}};
fn flag()->AtomicBool {AtomicBool::new(false)}
fn root()->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/tar-gnu-longlink-1718")}
fn base()->PathBuf {let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/tar-gnu-longlink-1718");std::fs::create_dir_all(&p).unwrap();p}
fn manifest()->serde_json::Value {serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn sha(data:&[u8])->String {format!("{:x}",Sha256::digest(data))}
fn digest(path:&Path)->String {let mut file=std::fs::File::open(path).unwrap();let mut buf=[0;65536];let mut hash=Sha256::new();loop{let n=file.read(&mut buf).unwrap();if n==0{break}hash.update(&buf[..n])}format!("{:x}",hash.finalize())}
fn scan(path:&Path)->recovery::Report {recovery::scan(path,None,&flag(),|_,_|{}).unwrap()}
fn no_stage(path:&Path){assert!(!std::fs::read_dir(path).unwrap().any(|e|e.unwrap().file_name().to_string_lossy().starts_with(".long-lzop-recovery")))}
fn exact(result:&recovery::Outcome){for e in &result.entries{let p=Path::new(&result.output_directory).join(&e.name);if e.is_directory{assert!(p.is_dir());continue}assert_eq!(p.metadata().unwrap().len(),e.bytes);assert_eq!(digest(&p),e.sha256);assert!(e.time_warning.is_none());if let Some(stored)=e.stored_bytes{assert!(stored<e.bytes);#[cfg(windows)]{use std::os::windows::fs::MetadataExt;assert_ne!(p.metadata().unwrap().file_attributes()&0x200,0);}}}}
fn rewrite(data:&mut [u8],at:usize,mutate:impl FnOnce(&mut tar::Header)){let mut h=tar::Header::from_byte_slice(&data[at..at+512]).clone();mutate(&mut h);h.set_cksum();data[at..at+512].copy_from_slice(h.as_bytes());}

fn ext(kind:tar::EntryType,bytes:&[u8])->Vec<u8>{let mut h=tar::Header::new_gnu();h.set_path("././@LongLink").unwrap();h.set_entry_type(kind);h.set_size(bytes.len()as u64);h.set_cksum();let mut b=tar::Builder::new(Vec::new());b.append(&h,bytes).unwrap();let mut out=b.into_inner().unwrap();out.truncate(out.len()-1024);out}
fn pax(target:&str)->Vec<u8>{let mut b=tar::Builder::new(Vec::new());b.append_pax_extensions([("linkpath",target.as_bytes())]).unwrap();let mut out=b.into_inner().unwrap();out.truncate(out.len()-1024);out}
fn long_record()->(Vec<u8>,usize,usize,usize){let m=manifest();let r=&m["records"][7];(std::fs::read(root().join("gnu-links.tar")).unwrap(),r["metadata"][0]["offset"].as_u64().unwrap()as usize,r["metadata"][1]["offset"].as_u64().unwrap()as usize,r["headerOffset"].as_u64().unwrap()as usize)}

#[test]
fn gnu1718_long_own_name_and_target_work_in_both_orders_and_matching_pax_applies_once(){
 let (raw,k,l,h)=long_record();let target=manifest()["entries"][7]["linkTarget"].as_str().unwrap().to_owned();let mut arrangements=vec![raw[k..h].to_vec()];let mut reverse=raw[l..h].to_vec();reverse.extend(&raw[k..l]);arrangements.push(reverse);
 for order in [true,false]{let mut meta=raw[k..h].to_vec();if order{meta.extend(pax(&target));}else{let mut prefix=pax(&target);prefix.extend(meta);meta=prefix;}arrangements.push(meta);}
 let temp=tempfile::tempdir().unwrap();let path=temp.path().join("orders.tar.lzo");for metadata in arrangements{let mut data=raw.clone();data.splice(k..h,metadata);std::fs::write(&path,wrapped(&data,137,true)).unwrap();let report=scan(&path);assert!(report.complete,"{report:?}");assert_eq!(report.entries.len(),15);assert_eq!(report.entries[7].name,manifest()["entries"][7]["name"].as_str().unwrap());assert_eq!(report.entries[7].link_target.as_deref(),Some(target.as_str()));assert!(report.entries[8].link_target.is_none());assert_eq!(report.entries[12].link_source,Some(6));}
}

#[test]
fn gnu1718_orphan_duplicate_conflicting_malformed_and_nonlink_targets_stop_before_confirmation(){
 let(raw,k,l,h)=long_record();let mut cases=Vec::new();
 for payload in [b"\0".as_slice(),b"unterminated",b"target\0extra",b"missing.bin\0",b"../source.bin\0",b"alias-a.bin\0",b"after.txt\0",b"\xff\0"]{let mut bad=raw.clone();bad.splice(k..l,ext(tar::EntryType::GNULongLink,payload));cases.push(bad);}
 let mut bad=raw.clone();bad.splice(k..k,raw[k..l].iter().copied());cases.push(bad);
 let mut bad=raw.clone();bad[l-1]=1;cases.push(bad);
 let mut bad=raw[..l].to_vec();bad.extend([0;1024]);cases.push(bad);
 for kind in [tar::EntryType::Regular,tar::EntryType::Directory,tar::EntryType::Symlink,tar::EntryType::GNUSparse]{let mut bad=raw.clone();rewrite(&mut bad,h,|header|header.set_entry_type(kind));cases.push(bad);}
 for first in [true,false]{let mut bad=raw.clone();let at=if first{k}else{h};bad.splice(at..at,pax("source.bin"));cases.push(bad);}
 let temp=tempfile::tempdir().unwrap();let path=temp.path().join("invalid.tar.lzo");let mut reasons=Vec::new();for bad in cases{std::fs::write(&path,wrapped(&bad,137,true)).unwrap();let report=scan(&path);assert!(!report.complete);assert_eq!(report.entries.len(),7,"{report:?}");reasons.push(report.stop_reason);}std::fs::write(base().join("invalid-metadata.json"),serde_json::to_vec_pretty(&reasons).unwrap()).unwrap();
}

#[test]
fn gnu1718_target_and_name_metadata_payload_padding_and_link_header_require_full_verified_prefix(){
 let(raw,k,l,h)=long_record();let m=manifest();let temp=tempfile::tempdir().unwrap();let path=temp.path().join("boundary.tar.lzo");let mut checks=Vec::new();
 for cut in [k+511,k+512+m["records"][7]["metadata"][0]["bytes"].as_u64().unwrap()as usize-1,l-1,l+511,h-1,h+511]{std::fs::write(&path,wrapped(&raw[..cut],137,true)).unwrap();let report=scan(&path);assert!(!report.complete);assert_eq!(report.entries.len(),7,"cut {cut}: {report:?}");checks.push(serde_json::json!({"cut":cut,"count":7,"reason":report.stop_reason}));}
 std::fs::write(&path,wrapped(&raw,137,true)).unwrap();let cancel=flag();let mut triggered=false;let error=recovery::scan(&path,None,&cancel,|done,_|if done>360000&&!triggered{triggered=true;cancel.store(true,Ordering::SeqCst)}).unwrap_err();assert!(triggered);assert!(format!("{error:#}").contains("cancelled"));assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(),1);
 std::fs::write(base().join("boundaries.json"),serde_json::to_vec_pretty(&checks).unwrap()).unwrap();
}


#[test]
fn gnu1718_complete_producer_resolves_prior_targets_and_local_pax_names_without_metadata_files(){
 let m=manifest();let source=root().join("gnu-links.tar.lzo");assert_eq!(sha(&std::fs::read(&source).unwrap()),m["sha256"].as_str().unwrap());let report=scan(&source);assert!(report.complete,"{report:?}");assert_eq!(report.entries.len(),15);assert_eq!(report.entries.iter().filter(|e|e.link_target.is_some()).count(),6);assert_eq!(report.entries.iter().filter(|e|e.sparse.is_some()).count(),5);
 for(e,x)in report.entries.iter().zip(m["entries"].as_array().unwrap()){assert_eq!(e.name,x["name"].as_str().unwrap());assert_eq!(e.bytes,x["bytes"].as_u64().unwrap());assert_eq!(e.sha256,x["sha256"].as_str().unwrap());assert_eq!(e.link_target.as_deref(),x["linkTarget"].as_str());assert!(!e.name.contains("GNUSparseFile"));}assert_eq!(report.entries[7].link_source,Some(6));assert_eq!(report.entries[11].link_source,Some(10));assert_eq!(report.entries[9].bytes,0);
}

#[tokio::test]
async fn gnu1718_tail_checksum_and_end_failures_materialize_alias_only_and_full_selections(){
 let m=manifest();let original=std::fs::read(root().join("gnu-links.tar.lzo")).unwrap();let run=base().join(uuid::Uuid::new_v4().to_string());std::fs::create_dir_all(&run).unwrap();let mut evidence=Vec::new();
 for mode in["tail","checksum","end"]{let mut bytes=original.clone();match mode{"tail"=>bytes.truncate(bytes.len()-400000),"checksum"=>{let raw=std::fs::read(root().join("gnu-links.tar")).unwrap();bytes=wrapped(&raw,100003,true);let header=wrapped(&[],100003,true).len()-4;let at=header+((raw.len()-1)/100003)*(100003+12)+8;bytes[at]^=1;},_=>bytes.truncate(bytes.len()-2)}let path=run.join(format!("{mode}.tar.lzo"));std::fs::write(&path,bytes).unwrap();let report=scan(&path);let count=if mode=="end"{15}else{14};assert!(!report.complete);assert_eq!(report.entries.len(),count,"{report:?}");let ids=report.entries.iter().map(|e|e.id).collect::<Vec<_>>();let mut last=0;let result=recovery::recover(&path,&run,&report.revision,&ids,None,&flag(),|phase,done,_|if phase=="recovering"{assert!(done>=last);last=done}).unwrap();exact(&result);assert_eq!(last,result.recovered_bytes);no_stage(&run);
  let mut record=serde_json::json!({"mode":mode,"source":path,"report":report,"result":result,"entries":&m["entries"].as_array().unwrap()[..count]});if mode=="tail"{let diagnosis=super::archive_diagnostics::diagnose_archive(&path,None,Arc::new(flag())).await.unwrap();assert_eq!(diagnosis.status,"partial_recovery");assert_eq!(diagnosis.total_files,12);record["diagnosis"]=serde_json::to_value(diagnosis).unwrap();}record["sourceKind"]=serde_json::json!(if mode=="checksum"{"CRC construction around GNU TAR"}else{"damaged GNU/lzop producer original"});evidence.push(record);
  if mode=="tail"{let ids=[2,4,5,7,9,11,12];let selected=recovery::recover(&path,&run,&report.revision,&ids,None,&flag(),|_,_,_|{}).unwrap();exact(&selected);assert_eq!(selected.recovered_files,6);assert_eq!(selected.recovered_directories,1);assert_eq!(selected.recovered_bytes,33954438);assert!(!Path::new(&selected.output_directory).join("source.bin").exists());evidence.push(serde_json::json!({"mode":"selected","source":path,"report":report,"result":selected,"entries":ids.iter().map(|id|m["entries"][id].clone()).collect::<Vec<_>>()}));}
 }assert_eq!(std::fs::read(root().join("gnu-links.tar.lzo")).unwrap(),original);std::fs::write(base().join("native.json"),serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
}

#[test]
fn gnu1718_original_names_bind_revision_even_when_windows_names_and_logical_bytes_match(){
 fn archive(swap:bool)->Vec<u8>{let mut b=tar::Builder::new(Vec::new());for name in if swap{["item*","item?"]}else{["item?","item*"]}{let mut h=tar::Header::new_ustar();h.set_size(4);h.set_mode(0o644);h.set_mtime(1700001718);h.set_cksum();b.append_data(&mut h,name,b"same".as_slice()).unwrap();}let mut h=tar::Header::new_ustar();h.set_path("copy.bin").unwrap();h.set_link_name("ignored").unwrap();h.set_entry_type(tar::EntryType::Link);h.set_size(0);h.set_mtime(1700001718);h.set_cksum();let mut data=b.into_inner().unwrap();data.truncate(data.len()-1024);data.extend(ext(tar::EntryType::GNULongLink,b"item?\0"));data.extend(h.as_bytes());data.extend([0;1024]);data}
 let temp=tempfile::tempdir().unwrap();let path=temp.path().join("revision.tar.lzo");let data=wrapped(&archive(false),137,true);std::fs::write(&path,&data[..data.len()-2]).unwrap();let before=scan(&path);assert_eq!(before.entries.len(),3);let time=filetime::FileTime::from_last_modification_time(&path.metadata().unwrap());let changed=wrapped(&archive(true),137,true);assert_eq!(data.len(),changed.len());std::fs::write(&path,&changed[..changed.len()-2]).unwrap();filetime::set_file_mtime(&path,time).unwrap();let after=scan(&path);assert_ne!(before.revision,after.revision);let mut left=serde_json::to_value(&before).unwrap();let mut right=serde_json::to_value(&after).unwrap();left["revision"]="".into();right["revision"]="".into();assert_eq!(left,right);assert_ne!(before.entries[2].link_source,after.entries[2].link_source);
 assert!(recovery::recover(&path,temp.path(),&before.revision,&[2],None,&flag(),|_,_,_|{}).unwrap_err().to_string().contains("重新检查"));let result=recovery::recover(&path,temp.path(),&after.revision,&[2],None,&flag(),|_,_,_|{}).unwrap();assert_eq!(std::fs::read(Path::new(&result.output_directory).join("copy.bin")).unwrap(),b"same");no_stage(temp.path());
}

#[test]
fn gnu1718_missing_volume_keeps_only_confirmed_link_targets_and_rechecks_after_repair(){
 let original=std::fs::read(root().join("gnu-links.tar.lzo")).unwrap();let temp=tempfile::tempdir().unwrap();let mut paths=Vec::new();for(i,bytes)in original.chunks(100003).enumerate(){let p=temp.path().join(format!("links.tar.lzo.{:03}",i+1));std::fs::write(&p,bytes).unwrap();paths.push(p);}let missing=std::fs::read(&paths[8]).unwrap();std::fs::remove_file(&paths[8]).unwrap();let path=paths.last().unwrap();let r=scan(path);assert_eq!(r.available_volumes,8);assert!(r.ignored_volumes>0);assert_eq!(r.entries.len(),14,"{r:?}");let result=recovery::recover(path,temp.path(),&r.revision,&[4,5,7,9,11,12],None,&flag(),|_,_,_|{}).unwrap();exact(&result);assert_eq!(result.recovered_files,6);std::fs::write(&paths[8],missing).unwrap();assert!(recovery::recover(path,temp.path(),&r.revision,&[4],None,&flag(),|_,_,_|{}).unwrap_err().to_string().contains("重新检查"));assert!(scan(path).complete);no_stage(temp.path());
}
