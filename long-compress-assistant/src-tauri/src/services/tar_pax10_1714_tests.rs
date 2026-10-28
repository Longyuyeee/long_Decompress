use super::{lzop::recovery,tar_lzo_prefix_175_tests::wrapped};
use sha2::{Digest,Sha256};
use std::{path::{Path,PathBuf},io::{Read,Cursor},sync::{atomic::{AtomicBool,Ordering},Arc}};
fn flag()->AtomicBool {AtomicBool::new(false)}
fn root()->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/tar-pax10-1714")}
fn base()->PathBuf {let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/tar-pax10-1714");std::fs::create_dir_all(&p).unwrap();p}
fn manifest()->serde_json::Value {serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn sha(data:&[u8])->String {format!("{:x}",Sha256::digest(data))}
fn digest(path:&Path)->String {let mut file=std::fs::File::open(path).unwrap();let mut buf=[0;65536];let mut hash=Sha256::new();loop{let n=file.read(&mut buf).unwrap();if n==0{break}hash.update(&buf[..n])}format!("{:x}",hash.finalize())}
fn scan(path:&Path)->recovery::Report {recovery::scan(path,None,&flag(),|_,_|{}).unwrap()}
fn no_stage(path:&Path){assert!(!std::fs::read_dir(path).unwrap().any(|e|e.unwrap().file_name().to_string_lossy().starts_with(".long-lzop-recovery")))}
fn exact(result:&recovery::Outcome){for e in &result.entries{let p=Path::new(&result.output_directory).join(&e.name);if e.is_directory{assert!(p.is_dir());continue}assert_eq!(p.metadata().unwrap().len(),e.bytes);assert_eq!(digest(&p),e.sha256);assert!(e.time_warning.is_none());if let Some(stored)=e.stored_bytes{assert!(stored<e.bytes);#[cfg(windows)]{use std::os::windows::fs::MetadataExt;assert_ne!(p.metadata().unwrap().file_attributes()&0x200,0);}}}}
fn rewrite(data:&mut [u8],at:usize,mutate:impl FnOnce(&mut tar::Header)){let mut h=tar::Header::from_byte_slice(&data[at..at+512]).clone();mutate(&mut h);h.set_cksum();data[at..at+512].copy_from_slice(h.as_bytes());}

#[tokio::test]
async fn pax1714_complete_original_keeps_normal_browse_integrity_and_healthy_diagnosis(){
 let m=manifest();let source=root().join(m["archive"].as_str().unwrap());assert_eq!(sha(&std::fs::read(&source).unwrap()),m["sha256"].as_str().unwrap());let report=scan(&source);assert!(report.complete,"{report:?}");assert_eq!(report.entries.len(),10);assert_eq!(report.entries.iter().filter(|e|e.sparse.is_some()).count(),5);
 for(e,x)in report.entries.iter().zip(m["entries"].as_array().unwrap()){assert_eq!(e.name,x["name"].as_str().unwrap());assert_eq!(e.bytes,x["bytes"].as_u64().unwrap());assert_eq!(e.sha256,x["sha256"].as_str().unwrap());}
 let list=super::archive_browser::browse_archive(&source,None).await.unwrap();assert_eq!(list.entries.len(),10);for(e,x)in list.entries.iter().zip(m["entries"].as_array().unwrap()){assert_eq!(e.path.trim_end_matches('/'),x["name"].as_str().unwrap());assert_eq!(e.size,x["bytes"].as_u64().unwrap());}
 super::universal_engine::UniversalCliEngine::test_integrity(&source,None).await.unwrap();let diagnosis=super::archive_diagnostics::diagnose_archive(&source,None,Arc::new(flag())).await.unwrap();assert_eq!(diagnosis.status,"healthy");
}

#[tokio::test]
async fn pax1714_tail_checksum_and_end_marker_failures_recover_full_sparse_contents_and_following_file(){
 let m=manifest();let original=std::fs::read(root().join(m["archive"].as_str().unwrap())).unwrap();let run=base().join(uuid::Uuid::new_v4().to_string());std::fs::create_dir_all(&run).unwrap();let mut evidence=Vec::new();
 for mode in ["tail","checksum","end"]{let mut bytes=original.clone();match mode{"tail"=>bytes.truncate(bytes.len()-400000),"checksum"=>{let n=bytes.len()-5;bytes[n]^=1;},_=>bytes.truncate(bytes.len()-2)}let path=run.join(format!("{mode}.tar.lzo"));std::fs::write(&path,&bytes).unwrap();let report=scan(&path);let count=if mode=="end"{10}else{9};assert!(!report.complete);assert_eq!(report.entries.len(),count,"{mode}: {report:?}");let ids=report.entries.iter().map(|e|e.id).collect::<Vec<_>>();let result=recovery::recover(&path,&run,&report.revision,&ids,None,&flag(),|_,_,_|{}).unwrap();exact(&result);assert!(Path::new(&result.output_directory).join("after.txt").is_file());assert_eq!(result.time_preservation.as_ref().unwrap().preserved_directories,Some(2));no_stage(&run);
  let mut record=serde_json::json!({"mode":mode,"source":path,"report":report,"result":result,"entries":&m["entries"].as_array().unwrap()[..count]});if mode=="tail"{let diagnosis=super::archive_diagnostics::diagnose_archive(&path,None,Arc::new(flag())).await.unwrap();assert_eq!(diagnosis.status,"partial_recovery");assert_eq!(diagnosis.total_files,7);record["diagnosis"]=serde_json::to_value(diagnosis).unwrap();}
  evidence.push(record);
  if mode=="tail"{let ids=[2,4,6,7];let selected=recovery::recover(&path,&run,&report.revision,&ids,None,&flag(),|_,_,_|{}).unwrap();exact(&selected);assert_eq!(selected.recovered_files,3);assert_eq!(selected.recovered_directories,1);evidence.push(serde_json::json!({"mode":"selected","source":path,"report":report,"result":selected,"entries":ids.iter().map(|id|m["entries"][id].clone()).collect::<Vec<_>>()}));}
 }
 assert_eq!(std::fs::read(root().join(m["archive"].as_str().unwrap())).unwrap(),original);std::fs::write(base().join("native.json"),serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
}

#[test]
fn pax1714_hole_hash_and_copy_cancel_and_destination_failure_leave_no_published_half_file(){
 let m=manifest();let original=std::fs::read(root().join(m["archive"].as_str().unwrap())).unwrap();let temp=tempfile::tempdir().unwrap();let path=temp.path().join("cancel.tar.lzo");std::fs::write(&path,&original[..original.len()-2]).unwrap();let report=scan(&path);let entry=&report.entries[6];let layout=entry.sparse.as_ref().unwrap();let raw=std::fs::read(root().join("pax10.tar")).unwrap();let offset=m["records"].as_array().unwrap().iter().find(|r|r["logicalIndex"]==6).unwrap()["dataOffset"].as_u64().unwrap()as usize;let mut input=Cursor::new(&raw[offset..]);let cancel=flag();let error=layout.process(&mut input,None,&cancel,|logical|if logical>=16*1024*1024{cancel.store(true,Ordering::SeqCst)}).unwrap_err();assert!(format!("{error:#}").contains("cancelled"));assert_eq!(input.position(),0);
 let cancel=flag();let mut active=false;let error=recovery::recover(&path,temp.path(),&report.revision,&[6],None,&cancel,|phase,done,_|if phase=="recovering"&&done>=16*1024*1024{active=true;cancel.store(true,Ordering::SeqCst)}).unwrap_err();assert!(active);assert!(format!("{error:#}").contains("cancelled"));no_stage(temp.path());assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(),1);
 let target=temp.path().join("readonly-target");std::fs::write(&target,[]).unwrap();let mut file=std::fs::File::open(&target).unwrap();let error=layout.process(&mut Cursor::new([]),Some(&mut file),&flag(),|_|{}).unwrap_err();assert!(!format!("{error:#}").contains("cancelled"));assert_eq!(std::fs::read(&path).unwrap(),&original[..original.len()-2]);
}

#[test]
fn pax1714_missing_middle_volume_recovers_sparse_prefix_and_rechecks_after_repair(){
 let original=std::fs::read(root().join("pax10.tar.lzo")).unwrap();let temp=tempfile::tempdir().unwrap();let mut paths=Vec::new();for(i,bytes)in original.chunks(100003).enumerate(){let path=temp.path().join(format!("sparse.tar.lzo.{:03}",i+1));std::fs::write(&path,bytes).unwrap();paths.push(path);}let missing=std::fs::read(&paths[8]).unwrap();std::fs::remove_file(&paths[8]).unwrap();let path=paths.last().unwrap();let report=scan(path);assert_eq!(report.available_volumes,8);assert!(report.ignored_volumes>0);assert_eq!(report.entries.len(),9,"{report:?}");let result=recovery::recover(path,temp.path(),&report.revision,&[3,4,5,6,7,8],None,&flag(),|_,_,_|{}).unwrap();exact(&result);assert_eq!(result.recovered_files,6);
 std::fs::write(&paths[8],missing).unwrap();assert!(recovery::recover(path,temp.path(),&report.revision,&[0],None,&flag(),|_,_,_|{}).unwrap_err().to_string().contains("重新检查"));assert!(scan(path).complete);assert_eq!(paths.iter().flat_map(|p|std::fs::read(p).unwrap()).collect::<Vec<_>>(),original);no_stage(temp.path());
}

fn fields(record:&serde_json::Value)->super::tar_pax::Fields{record["pax"].as_object().unwrap().iter().map(|(k,v)|(k.clone(),v.as_str().unwrap().to_owned())).collect()}
fn control(fields:&super::tar_pax::Fields)->Vec<u8>{let mut b=tar::Builder::new(Vec::new());b.append_pax_extensions(fields.iter().map(|(k,v)|(k.as_str(),v.as_bytes()))).unwrap();let mut v=b.into_inner().unwrap();v.truncate(v.len()-1024);v}
fn replace_control(raw:&[u8],record:&serde_json::Value,fields:&super::tar_pax::Fields)->Vec<u8>{let mut v=raw.to_vec();v.splice(record["control"]["offset"].as_u64().unwrap()as usize..record["headerOffset"].as_u64().unwrap()as usize,control(fields));v}
fn map_block(raw:&[u8],at:usize,text:&[u8])->Vec<u8>{let mut v=raw.to_vec();v[at..at+512].fill(0);v[at..at+text.len()].copy_from_slice(text);v}

#[test]
fn pax1714_map_and_control_boundaries_never_expose_placeholders_or_incomplete_members(){
 let m=manifest();let raw=std::fs::read(root().join("pax10.tar")).unwrap();let r=&m["records"][7];let body=r["bodyOffset"].as_u64().unwrap()as usize;let data=r["dataOffset"].as_u64().unwrap()as usize;let end=body+r["physicalBytes"].as_u64().unwrap()as usize;assert!(data-body>512);let temp=tempfile::tempdir().unwrap();let path=temp.path().join("boundary.tar.lzo");let mut evidence=Vec::new();
 for cut in[r["control"]["offset"].as_u64().unwrap()as usize+520,r["headerOffset"].as_u64().unwrap()as usize-1,body+511,body+700,data-1,data+100,end-1]{std::fs::write(&path,wrapped(&raw[..cut],137,true)).unwrap();let report=scan(&path);assert_eq!(report.entries.len(),7,"cut {cut}: {report:?}");assert!(!report.complete);assert!(report.entries.iter().all(|e|!e.name.contains("GNUSparseFile")));evidence.push(serde_json::json!({"cut":cut,"reason":report.stop_reason}));}
 let r=&m["records"][3];let at=r["bodyOffset"].as_u64().unwrap()as usize;
 for text in[b"-1\n".as_slice(),b"65537\n",b"1\n0\n999999999999999999999\n",b"2\n9\n4096\n1\n4096\n",b"1\n524288\n8192\n",b"1\n0\n1\n",b"1\n0\n0\n",b"1\n0\r\n8192\n",b"1\n0\n8192\nX"]{let bad=map_block(&raw,at,text);std::fs::write(&path,wrapped(&bad,137,true)).unwrap();let report=scan(&path);assert_eq!(report.entries.len(),3,"{report:?}");assert!(!report.complete);evidence.push(serde_json::json!({"invalidMap":String::from_utf8_lossy(text),"reason":report.stop_reason}));}
 std::fs::write(base().join("boundaries.json"),serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
}

#[test]
fn pax1714_only_local_version10_file_metadata_and_unambiguous_physical_sizes_are_supported(){
 let m=manifest();let raw=std::fs::read(root().join("pax10.tar")).unwrap();let r=&m["records"][3];let original=fields(r);let mut cases=Vec::new();
 for(key,value)in[("GNU.sparse.major","0"),("GNU.sparse.minor","1"),("GNU.sparse.name","../escape"),("GNU.sparse.realsize","-1"),("GNU.sparse.realsize","99999999999999999999"),("GNU.sparse.map","0,4096"),("size","524288")]{let mut f=original.clone();if let Some(pair)=f.iter_mut().find(|(k,_)|k==key){pair.1=value.into()}else{f.push((key.into(),value.into()))}cases.push(replace_control(&raw,r,&f));}
 let f=original.iter().filter(|(k,_)|k!="GNU.sparse.name").cloned().collect();cases.push(replace_control(&raw,r,&f));
 let mut f=original.clone();f.push(("GNU.sparse.major".into(),"1".into()));cases.push(replace_control(&raw,r,&f));
 let at=r["headerOffset"].as_u64().unwrap()as usize;let mut bad=raw.clone();rewrite(&mut bad,at,|h|h.set_entry_type(tar::EntryType::Directory));cases.push(bad);
 let mut bad=raw.clone();rewrite(&mut bad,r["control"]["offset"].as_u64().unwrap()as usize,|h|h.set_entry_type(tar::EntryType::XGlobalHeader));cases.push(bad);
 let mut b=tar::Builder::new(Vec::new());let mut h=tar::Header::new_gnu();h.set_path("././@LongLink").unwrap();h.set_entry_type(tar::EntryType::GNULongName);let name=b"actual.bin\0";h.set_size(name.len()as u64);h.set_cksum();b.append(&h,name.as_slice()).unwrap();let mut gnu=b.into_inner().unwrap();gnu.truncate(gnu.len()-1024);let mut bad=raw.clone();bad.splice(at..at,gnu);cases.push(bad);
 let temp=tempfile::tempdir().unwrap();let path=temp.path().join("unsupported.tar.lzo");for bad in cases{std::fs::write(&path,wrapped(&bad,137,true)).unwrap();let report=scan(&path);assert_eq!(report.entries.len(),3,"{report:?}");assert!(!report.complete);}
 let mut f=original;f.push(("size".into(),r["physicalBytes"].as_u64().unwrap().to_string()));let good=replace_control(&raw,r,&f);std::fs::write(&path,wrapped(&good,137,true)).unwrap();assert!(scan(&path).complete);
}

#[test]
fn pax1714_map_revision_binds_equal_logical_contents_with_equal_source_stamp(){
 let raw=std::fs::read(root().join("pax10.tar")).unwrap();let temp=tempfile::tempdir().unwrap();let path=temp.path().join("revision.tar.lzo");let data=wrapped(&raw,137,true);std::fs::write(&path,&data[..data.len()-2]).unwrap();let before=scan(&path);let time=filetime::FileTime::from_last_modification_time(&path.metadata().unwrap());let at=manifest()["records"][5]["bodyOffset"].as_u64().unwrap()as usize;
 let changed=map_block(&raw,at,b"0\n");let data=wrapped(&changed,137,true);std::fs::write(&path,&data[..data.len()-2]).unwrap();filetime::set_file_mtime(&path,time).unwrap();let after=scan(&path);assert_ne!(before.revision,after.revision);let mut left=serde_json::to_value(&before).unwrap();let mut right=serde_json::to_value(&after).unwrap();left["revision"]="".into();right["revision"]="".into();assert_eq!(left,right);
 assert!(recovery::recover(&path,temp.path(),&before.revision,&[5],None,&flag(),|_,_,_|{}).unwrap_err().to_string().contains("重新检查"));no_stage(temp.path());
}

#[test]
fn pax1714_map_padding_and_payload_padding_are_separate_and_scan_cancellation_is_respected(){
 let f=vec![("GNU.sparse.major".into(),"1".into()),("GNU.sparse.minor".into(),"0".into()),("GNU.sparse.name".into(),"actual.bin".into()),("GNU.sparse.realsize".into(),"10".into())];let mut raw=control(&f);let mut h=tar::Header::new_ustar();h.set_path("GNUSparseFile.placeholder").unwrap();h.set_size(515);h.set_mtime(1700001714);h.set_mode(0o644);h.set_cksum();raw.extend_from_slice(h.as_bytes());let body=raw.len();raw.resize(body+512,0);raw[body..body+6].copy_from_slice(b"1\n2\n3\n");raw.extend_from_slice(b"abc");raw.resize(body+1024+1024,0);let temp=tempfile::tempdir().unwrap();let path=temp.path().join("small.tar.lzo");
 for cut in[body+511,body+514,body+1023]{std::fs::write(&path,wrapped(&raw[..cut],137,true)).unwrap();assert!(scan(&path).entries.is_empty());}
 for at in[body+511,body+1023]{let mut bad=raw.clone();bad[at]=1;std::fs::write(&path,wrapped(&bad,137,true)).unwrap();assert!(scan(&path).entries.is_empty());}
 let data=wrapped(&raw,137,true);std::fs::write(&path,&data[..data.len()-2]).unwrap();let report=scan(&path);assert_eq!(report.entries.len(),1,"{report:?}");assert_eq!(report.entries[0].stored_bytes,Some(3));let result=recovery::recover(&path,temp.path(),&report.revision,&[0],None,&flag(),|_,_,_|{}).unwrap();assert_eq!(std::fs::read(Path::new(&result.output_directory).join("actual.bin")).unwrap(),b"\0\0abc\0\0\0\0\0");exact(&result);
 let m=manifest();let source=root().join("pax10.tar.lzo");let cancel=flag();let mut callbacks=0;let error=recovery::scan(&source,None,&cancel,|_,_|{callbacks+=1;if callbacks==20{cancel.store(true,Ordering::SeqCst)}}).unwrap_err();assert!(format!("{error:#}").contains("cancelled"));assert!(m["records"][7]["mapBytes"].as_u64().unwrap()>512);no_stage(temp.path());
}
