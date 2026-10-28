use super::{lzop::recovery,tar_lzo_prefix_175_tests::wrapped};
use sha2::{Digest,Sha256};
use std::{path::{Path,PathBuf},io::{Read},sync::{atomic::{AtomicBool,Ordering},Arc}};
fn flag()->AtomicBool {AtomicBool::new(false)}
fn root()->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/tar-hardlink-1717")}
fn base()->PathBuf {let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/tar-hardlink-1717");std::fs::create_dir_all(&p).unwrap();p}
fn manifest()->serde_json::Value {serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn sha(data:&[u8])->String {format!("{:x}",Sha256::digest(data))}
fn digest(path:&Path)->String {let mut file=std::fs::File::open(path).unwrap();let mut buf=[0;65536];let mut hash=Sha256::new();loop{let n=file.read(&mut buf).unwrap();if n==0{break}hash.update(&buf[..n])}format!("{:x}",hash.finalize())}
fn scan(path:&Path)->recovery::Report {recovery::scan(path,None,&flag(),|_,_|{}).unwrap()}
fn no_stage(path:&Path){assert!(!std::fs::read_dir(path).unwrap().any(|e|e.unwrap().file_name().to_string_lossy().starts_with(".long-lzop-recovery")))}
fn exact(result:&recovery::Outcome){for e in &result.entries{let p=Path::new(&result.output_directory).join(&e.name);if e.is_directory{assert!(p.is_dir());continue}assert_eq!(p.metadata().unwrap().len(),e.bytes);assert_eq!(digest(&p),e.sha256);assert!(e.time_warning.is_none());if let Some(stored)=e.stored_bytes{assert!(stored<e.bytes);#[cfg(windows)]{use std::os::windows::fs::MetadataExt;assert_ne!(p.metadata().unwrap().file_attributes()&0x200,0);}}}}
fn rewrite(data:&mut [u8],at:usize,mutate:impl FnOnce(&mut tar::Header)){let mut h=tar::Header::from_byte_slice(&data[at..at+512]).clone();mutate(&mut h);h.set_cksum();data[at..at+512].copy_from_slice(h.as_bytes());}


fn fields(record:&serde_json::Value)->super::tar_pax::Fields{record["pax"].as_object().unwrap().iter().map(|(k,v)|(k.clone(),v.as_str().unwrap().into())).collect()}
fn control(fields:&super::tar_pax::Fields)->Vec<u8>{let mut b=tar::Builder::new(Vec::new());b.append_pax_extensions(fields.iter().map(|(k,v)|(k.as_str(),v.as_bytes()))).unwrap();let mut v=b.into_inner().unwrap();v.truncate(v.len()-1024);v}
fn replace_control(raw:&[u8],record:&serde_json::Value,fields:&super::tar_pax::Fields)->Vec<u8>{let mut v=raw.to_vec();let end=record["headerOffset"].as_u64().unwrap()as usize;let start=record["control"]["offset"].as_u64().map_or(end,|n|n as usize);v.splice(start..end,control(fields));v}

#[test]
fn hard1717_complete_producer_resolves_prior_targets_and_local_pax_names_without_metadata_files(){
 let m=manifest();let source=root().join("hardlinks.tar.lzo");assert_eq!(sha(&std::fs::read(&source).unwrap()),m["sha256"].as_str().unwrap());let report=scan(&source);assert!(report.complete,"{report:?}");assert_eq!(report.entries.len(),14);assert_eq!(report.entries.iter().filter(|e|e.link_target.is_some()).count(),5);assert_eq!(report.entries.iter().filter(|e|e.sparse.is_some()).count(),4);
 for(e,x)in report.entries.iter().zip(m["entries"].as_array().unwrap()){assert_eq!(e.name,x["name"].as_str().unwrap());assert_eq!(e.bytes,x["bytes"].as_u64().unwrap());assert_eq!(e.sha256,x["sha256"].as_str().unwrap());assert_eq!(e.link_target.as_deref(),x["linkTarget"].as_str());assert!(!e.name.contains("GNUSparseFile"));}assert_eq!(report.entries[7].link_source,Some(6));assert_eq!(report.entries[11].link_source,Some(10));assert_eq!(report.entries[9].bytes,0);
}

#[tokio::test]
async fn hard1717_tail_checksum_and_end_failures_materialize_alias_only_and_full_selections(){
 let m=manifest();let original=std::fs::read(root().join("hardlinks.tar.lzo")).unwrap();let run=base().join(uuid::Uuid::new_v4().to_string());std::fs::create_dir_all(&run).unwrap();let mut evidence=Vec::new();
 for mode in["tail","checksum","end"]{let mut bytes=original.clone();match mode{"tail"=>bytes.truncate(bytes.len()-400000),"checksum"=>{let raw=std::fs::read(root().join("hardlinks.tar")).unwrap();bytes=wrapped(&raw,100003,true);let header=wrapped(&[],100003,true).len()-4;let at=header+((raw.len()-1)/100003)*(100003+12)+8;bytes[at]^=1;},_=>bytes.truncate(bytes.len()-2)}let path=run.join(format!("{mode}.tar.lzo"));std::fs::write(&path,bytes).unwrap();let report=scan(&path);let count=if mode=="end"{14}else{13};assert!(!report.complete);assert_eq!(report.entries.len(),count,"{report:?}");let ids=report.entries.iter().map(|e|e.id).collect::<Vec<_>>();let mut last=0;let result=recovery::recover(&path,&run,&report.revision,&ids,None,&flag(),|phase,done,_|if phase=="recovering"{assert!(done>=last);last=done}).unwrap();exact(&result);assert_eq!(last,result.recovered_bytes);no_stage(&run);
  let mut record=serde_json::json!({"mode":mode,"source":path,"report":report,"result":result,"entries":&m["entries"].as_array().unwrap()[..count]});if mode=="tail"{let diagnosis=super::archive_diagnostics::diagnose_archive(&path,None,Arc::new(flag())).await.unwrap();assert_eq!(diagnosis.status,"partial_recovery");assert_eq!(diagnosis.total_files,11);record["diagnosis"]=serde_json::to_value(diagnosis).unwrap();}record["sourceKind"]=serde_json::json!(if mode=="checksum"{"CRC construction around GNU TAR"}else{"damaged GNU/lzop producer original"});evidence.push(record);
  if mode=="tail"{let ids=[2,4,5,7,9,11];let selected=recovery::recover(&path,&run,&report.revision,&ids,None,&flag(),|_,_,_|{}).unwrap();exact(&selected);assert_eq!(selected.recovered_files,5);assert_eq!(selected.recovered_directories,1);assert_eq!(selected.recovered_bytes,25565830);assert!(!Path::new(&selected.output_directory).join("source.bin").exists());evidence.push(serde_json::json!({"mode":"selected","source":path,"report":report,"result":selected,"entries":ids.iter().map(|id|m["entries"][id].clone()).collect::<Vec<_>>()}));}
 }assert_eq!(std::fs::read(root().join("hardlinks.tar.lzo")).unwrap(),original);std::fs::write(base().join("native.json"),serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
}

#[test]
fn hard1717_unconfirmed_ambiguous_chain_external_and_conflicting_targets_stop_at_their_headers(){
 let m=manifest();let raw=std::fs::read(root().join("hardlinks.tar")).unwrap();let a=&m["records"][4];let at=a["headerOffset"].as_u64().unwrap()as usize;let mut cases=Vec::new();
 for target in["missing.bin","after.txt","alias-a.bin","资料","SOURCE.BIN","../source.bin"]{let mut bad=raw.clone();rewrite(&mut bad,at,|h|h.set_link_name(target).unwrap());cases.push((bad,4));}
 for kind in[tar::EntryType::Symlink,tar::EntryType::Fifo]{let mut bad=raw.clone();rewrite(&mut bad,at,|h|h.set_entry_type(kind));cases.push((bad,4));}
 let mut bad=raw.clone();rewrite(&mut bad,at,|h|h.set_size(1));cases.push((bad,4));let mut bad=raw.clone();rewrite(&mut bad,at,|h|h.as_mut_bytes()[157..257].fill(0));cases.push((bad,4));
 let mut bad=raw.clone();rewrite(&mut bad,m["records"][5]["headerOffset"].as_u64().unwrap()as usize,|h|h.set_link_name("alias-a.bin").unwrap());cases.push((bad,5));
 let long=&m["records"][7];for value in["","missing.bin","../source.bin","alias-a.bin"]{let mut f=fields(long);f.iter_mut().find(|(k,_)|k=="linkpath").unwrap().1=value.into();cases.push((replace_control(&raw,long,&f),7));}
 let mut f=fields(long);f.push(("linkpath".into(),"source.bin".into()));cases.push((replace_control(&raw,long,&f),7));
 cases.push((replace_control(&raw,&m["records"][3],&vec![("linkpath".into(),"source.bin".into())]),3));
 for value in["1","0"]{let mut bad=raw.clone();if value=="0"{rewrite(&mut bad,at,|h|h.set_size(1));}cases.push((replace_control(&bad,a,&vec![("size".into(),value.into())]),4));}
 let mut bad=replace_control(&raw,a,&vec![("linkpath".into(),"source.bin".into())]);rewrite(&mut bad,at,|h|h.set_entry_type(tar::EntryType::XGlobalHeader));cases.push((bad,4));
 let mut b=tar::Builder::new(Vec::new());let mut h=tar::Header::new_ustar();h.set_path("source.bin").unwrap();h.set_mode(0o644);h.set_size(4);h.set_cksum();b.append(&h,b"same".as_slice()).unwrap();let mut extra=b.into_inner().unwrap();extra.truncate(extra.len()-1024);let mut bad=raw.clone();bad.splice(at..at,extra);cases.push((bad,5));
 let mut h=tar::Header::new_gnu();h.set_path("././@LongLink").unwrap();h.set_entry_type(tar::EntryType::GNULongLink);h.set_size(11);h.set_cksum();let mut b=tar::Builder::new(Vec::new());b.append(&h,b"source.bin\0".as_slice()).unwrap();let mut extra=b.into_inner().unwrap();extra.truncate(extra.len()-1024);let mut bad=raw.clone();bad.splice(at..at,extra);cases.push((bad,4));
 let temp=tempfile::tempdir().unwrap();std::fs::write(temp.path().join("missing.bin"),b"external data").unwrap();let path=temp.path().join("bad.tar.lzo");let mut reasons=Vec::new();for(bad,count)in cases{std::fs::write(&path,wrapped(&bad,137,true)).unwrap();let report=scan(&path);assert_eq!(report.entries.len(),count,"{report:?}");assert!(!report.complete);reasons.push(report.stop_reason);}std::fs::write(base().join("invalid-targets.json"),serde_json::to_vec_pretty(&reasons).unwrap()).unwrap();
}

#[test]
fn hard1717_target_data_padding_and_link_metadata_must_be_complete_before_alias_confirmation(){
 let m=manifest();let raw=std::fs::read(root().join("hardlinks.tar")).unwrap();let temp=tempfile::tempdir().unwrap();let path=temp.path().join("boundary.tar.lzo");let mut checks=Vec::new();let source=&m["records"][3];let end=source["dataOffset"].as_u64().unwrap()as usize+source["physicalBytes"].as_u64().unwrap()as usize;let a=m["records"][4]["headerOffset"].as_u64().unwrap()as usize;let long=&m["records"][7];let start=long["control"]["offset"].as_u64().unwrap()as usize;let size=long["control"]["bytes"].as_u64().unwrap()as usize;let header=long["headerOffset"].as_u64().unwrap()as usize;
 for(cut,count)in[(end-1,3),(end,3),(a-1,3),(a+511,4),(start+512+size-1,7),(header-1,7),(header+511,7)]{std::fs::write(&path,wrapped(&raw[..cut],137,true)).unwrap();let r=scan(&path);assert_eq!(r.entries.len(),count,"cut {cut}: {r:?}");checks.push(serde_json::json!({"cut":cut,"count":count,"reason":r.stop_reason}));}
 for(at,count)in[(a-1,3),(header-1,7)]{let mut bad=raw.clone();bad[at]=1;std::fs::write(&path,wrapped(&bad,137,true)).unwrap();assert_eq!(scan(&path).entries.len(),count);}
 std::fs::write(base().join("boundaries.json"),serde_json::to_vec_pretty(&checks).unwrap()).unwrap();
}

#[test]
fn hard1717_original_names_bind_revision_even_when_windows_names_and_logical_bytes_match(){
 fn archive(swap:bool)->Vec<u8>{let mut b=tar::Builder::new(Vec::new());for name in if swap{["item*","item?"]}else{["item?","item*"]}{let mut h=tar::Header::new_ustar();h.set_size(4);h.set_mode(0o644);h.set_mtime(1700001717);h.set_cksum();b.append_data(&mut h,name,b"same".as_slice()).unwrap();}let mut h=tar::Header::new_ustar();h.set_path("copy.bin").unwrap();h.set_link_name("item?").unwrap();h.set_entry_type(tar::EntryType::Link);h.set_size(0);h.set_mtime(1700001717);h.set_cksum();b.append(&h,std::io::empty()).unwrap();b.into_inner().unwrap()}
 let temp=tempfile::tempdir().unwrap();let path=temp.path().join("revision.tar.lzo");let data=wrapped(&archive(false),137,true);std::fs::write(&path,&data[..data.len()-2]).unwrap();let before=scan(&path);assert_eq!(before.entries.len(),3);let time=filetime::FileTime::from_last_modification_time(&path.metadata().unwrap());let changed=wrapped(&archive(true),137,true);assert_eq!(data.len(),changed.len());std::fs::write(&path,&changed[..changed.len()-2]).unwrap();filetime::set_file_mtime(&path,time).unwrap();let after=scan(&path);assert_ne!(before.revision,after.revision);let mut left=serde_json::to_value(&before).unwrap();let mut right=serde_json::to_value(&after).unwrap();left["revision"]="".into();right["revision"]="".into();assert_eq!(left,right);assert_ne!(before.entries[2].link_source,after.entries[2].link_source);
 assert!(recovery::recover(&path,temp.path(),&before.revision,&[2],None,&flag(),|_,_,_|{}).unwrap_err().to_string().contains("重新检查"));let result=recovery::recover(&path,temp.path(),&after.revision,&[2],None,&flag(),|_,_,_|{}).unwrap();assert_eq!(std::fs::read(Path::new(&result.output_directory).join("copy.bin")).unwrap(),b"same");no_stage(temp.path());
}

#[test]
fn hard1717_aliases_are_independent_and_cancellation_during_staged_replay_publishes_nothing(){
 let original=std::fs::read(root().join("hardlinks.tar.lzo")).unwrap();let temp=tempfile::tempdir().unwrap();let path=temp.path().join("copy.tar.lzo");std::fs::write(&path,&original[..original.len()-2]).unwrap();let report=scan(&path);let cancel=flag();let mut active=false;let error=recovery::recover(&path,temp.path(),&report.revision,&[4,5],None,&cancel,|phase,done,_|if phase=="recovering"&&done>200003{active=true;cancel.store(true,Ordering::SeqCst)}).unwrap_err();assert!(active);assert!(format!("{error:#}").contains("cancelled"));no_stage(temp.path());assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(),1);
 let result=recovery::recover(&path,temp.path(),&report.revision,&[3,4,5],None,&flag(),|_,_,_|{}).unwrap();exact(&result);let out=Path::new(&result.output_directory);let b=std::fs::read(out.join("资料/alias-b.bin")).unwrap();std::fs::write(out.join("alias-a.bin"),b"changed independently").unwrap();assert_eq!(std::fs::read(out.join("资料/alias-b.bin")).unwrap(),b);assert_eq!(std::fs::read(out.join("source.bin")).unwrap(),b);assert_eq!(std::fs::read(&path).unwrap(),&original[..original.len()-2]);
}

#[test]
fn hard1717_missing_volume_keeps_only_confirmed_link_targets_and_rechecks_after_repair(){
 let original=std::fs::read(root().join("hardlinks.tar.lzo")).unwrap();let temp=tempfile::tempdir().unwrap();let mut paths=Vec::new();for(i,bytes)in original.chunks(100003).enumerate(){let p=temp.path().join(format!("links.tar.lzo.{:03}",i+1));std::fs::write(&p,bytes).unwrap();paths.push(p);}let missing=std::fs::read(&paths[8]).unwrap();std::fs::remove_file(&paths[8]).unwrap();let path=paths.last().unwrap();let r=scan(path);assert_eq!(r.available_volumes,8);assert!(r.ignored_volumes>0);assert_eq!(r.entries.len(),13,"{r:?}");let result=recovery::recover(path,temp.path(),&r.revision,&[4,5,7,9,11],None,&flag(),|_,_,_|{}).unwrap();exact(&result);assert_eq!(result.recovered_files,5);std::fs::write(&paths[8],missing).unwrap();assert!(recovery::recover(path,temp.path(),&r.revision,&[4],None,&flag(),|_,_,_|{}).unwrap_err().to_string().contains("重新检查"));assert!(scan(path).complete);no_stage(temp.path());
}
