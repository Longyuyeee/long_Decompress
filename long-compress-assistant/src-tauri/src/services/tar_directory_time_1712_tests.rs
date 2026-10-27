use super::{lzop::recovery,tar_recovery_time::ModificationTime,tar_lzo_prefix_175_tests::wrapped};
use sha2::{Digest,Sha256};
use std::{path::{Path,PathBuf},sync::atomic::{AtomicBool,Ordering},io::Write};
fn flag()->AtomicBool {AtomicBool::new(false)}
fn root()->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/tar-directory-time-1712")}
fn base()->PathBuf {let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/tar-directory-time-1712");std::fs::create_dir_all(&p).unwrap();p}
fn manifest()->serde_json::Value {serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn sha(b:&[u8])->String {format!("{:x}",Sha256::digest(b))}
fn scan(p:&Path)->recovery::Report {recovery::scan(p,None,&flag(),|_,_|{}).unwrap()}
fn no_stage(p:&Path){assert!(!std::fs::read_dir(p).unwrap().any(|e|e.unwrap().file_name().to_string_lossy().starts_with(".long-lzop-recovery")))}
fn exact(result:&recovery::Outcome){
 for e in &result.entries {let path=Path::new(&result.output_directory).join(&e.name);if e.is_directory{assert!(path.is_dir())}else{assert_eq!(sha(&std::fs::read(&path).unwrap()),e.sha256)}
  if e.time_warning.is_none(){let expected=e.modified_time.as_ref().unwrap();let actual=filetime::FileTime::from_last_modification_time(&path.metadata().unwrap());assert_eq!((actual.unix_seconds(),actual.nanoseconds()),(expected.seconds,expected.nanoseconds),"{}",e.name);}
 }
}

#[test]
fn directory1712_independent_originals_apply_explicit_directory_times_after_all_children_and_keep_selection_scope(){
 let m=manifest();let run=base().join(uuid::Uuid::new_v4().to_string());std::fs::create_dir_all(&run).unwrap();let mut evidence=Vec::new();
 for case in m["cases"].as_array().unwrap(){let bytes=std::fs::read(root().join(case["archive"].as_str().unwrap())).unwrap();assert_eq!(sha(&bytes),case["sha256"].as_str().unwrap());let path=run.join(format!("{}-tail.tar.lzo",case["name"].as_str().unwrap()));std::fs::write(&path,&bytes[..bytes.len()-2]).unwrap();let report=scan(&path);assert!(!report.complete);assert_eq!(report.entries.len(),9);
  for(e,x)in report.entries.iter().zip(case["entries"].as_array().unwrap()){assert_eq!(e.name,x["name"].as_str().unwrap());assert_eq!(serde_json::to_value(&e.modified_time).unwrap(),x["modifiedTime"]);}
  let ids=report.entries.iter().map(|e|e.id).collect::<Vec<_>>();let result=recovery::recover(&path,&run,&report.revision,&ids,None,&flag(),|_,_,_|{}).unwrap();assert_eq!(result.recovered_directories,6);assert_eq!(result.recovered_files,3);exact(&result);
  let times=result.time_preservation.as_ref().unwrap();assert_eq!(times.preserved_files,3);assert_eq!(times.preserved_directories.unwrap()+times.unpreserved_directories.unwrap(),6);
  #[cfg(windows)] {assert_eq!(times.unpreserved_directories,Some(if case["name"]=="gnu"{1}else{2}));}
  assert!(Path::new(&result.output_directory).join("implicit-only").is_dir());assert!(!result.entries.iter().any(|e|e.name=="implicit-only"));no_stage(&run);
  evidence.push(serde_json::json!({"case":case["name"],"mode":"full","source":path,"report":report,"result":result,"entries":case["entries"]}));
  if case["name"]=="pax" {
   for(mode,ids)in[("selected",vec![2,4]),("directory-only",vec![1])]{let result=recovery::recover(&path,&run,&report.revision,&ids,None,&flag(),|_,_,_|{}).unwrap();exact(&result);assert_eq!(result.recovered_directories,1);assert_eq!(result.recovered_files,usize::from(mode=="selected"));assert_eq!(result.entries.len(),ids.len());
    let root_time=filetime::FileTime::from_last_modification_time(&Path::new(&result.output_directory).join("资料").metadata().unwrap());assert_ne!(root_time.unix_seconds(),1700001712);assert!(!result.entries.iter().any(|e|e.name=="资料"));
    let entries=ids.iter().map(|id|case["entries"][id].clone()).collect::<Vec<_>>();evidence.push(serde_json::json!({"case":case["name"],"mode":mode,"source":path,"report":report,"result":result,"entries":entries}));
   }
  }
  assert_eq!(std::fs::read(root().join(case["archive"].as_str().unwrap())).unwrap(),bytes);
 }
 std::fs::write(base().join("native.json"),serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
}

#[test]
fn directory1712_local_global_time_deletion_unknown_time_and_directory_open_failure_do_not_lose_contents(){
 let mut builder=tar::Builder::new(Vec::new());
 fn control(builder:&mut tar::Builder<Vec<u8>>,global:bool,fields:&[(&str,&[u8])]){let mut record=tar::Builder::new(Vec::new());record.append_pax_extensions(fields.iter().copied()).unwrap();let mut bytes=record.into_inner().unwrap();bytes.truncate(bytes.len()-1024);if global{bytes[156]=b'g';bytes[148..156].fill(b' ');let sum:usize=bytes[..512].iter().map(|b|*b as usize).sum();bytes[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());}builder.get_mut().extend(bytes);}
 control(&mut builder,true,&[("mtime",b"99.25")]);
 for(name,time,local)in[("local-delete",42,Some(b"".as_slice())),("inherit",43,None),("global-delete",44,None),("unknown",u64::MAX,None),("extra-precision",45,Some(b"1.1234567891".as_slice()))]{
  if name=="global-delete"{control(&mut builder,true,&[("mtime",b"")]);}
  if let Some(local)=local {control(&mut builder,false,&[("mtime",local)]);}
  let mut h=tar::Header::new_gnu();h.set_path(name).unwrap();h.set_entry_type(tar::EntryType::Directory);h.set_size(0);h.set_mode(0o755);h.set_mtime(time);h.set_cksum();builder.append(&h,&[][..]).unwrap();
 }
 let raw=builder.into_inner().unwrap();let data=wrapped(&raw,137,true);let temp=tempfile::tempdir().unwrap();let path=temp.path().join("states.tar.lzo");std::fs::write(&path,&data[..data.len()-2]).unwrap();let report=scan(&path);assert_eq!(report.entries.len(),5);
 for(id,value)in[(0,"42"),(1,"99.25"),(2,"44")] {assert_eq!(report.entries[id].modified_time,ModificationTime::pax(value));}
 assert!(report.entries[3..].iter().all(|e|e.modified_time.is_none()));let result=recovery::recover(&path,temp.path(),&report.revision,&[0,1,2,3,4],None,&flag(),|_,_,_|{}).unwrap();exact(&result);assert_eq!(result.time_preservation.as_ref().unwrap().unpreserved_directories,Some(2));assert_eq!(result.recovered_files,0);assert_eq!(result.recovered_directories,5);no_stage(temp.path());
 assert!(ModificationTime::pax("42").unwrap().apply_directory(&temp.path().join("missing")).is_err());
}

#[test]
fn directory1712_cancellation_during_directory_times_and_changed_source_remove_the_entire_stage(){
 let data=std::fs::read(root().join("gnu.tar.lzo")).unwrap();let temp=tempfile::tempdir().unwrap();let path=temp.path().join("original.tar.lzo");std::fs::write(&path,&data[..data.len()-2]).unwrap();let report=scan(&path);let ids=report.entries.iter().filter(|e|e.is_directory).map(|e|e.id).collect::<Vec<_>>();let cancel=flag();let mut active=false;
 let err=recovery::recover(&path,temp.path(),&report.revision,&ids,None,&cancel,|phase,_,_|{if phase=="recovering"{active=true;let stage=std::fs::read_dir(temp.path()).unwrap().map(|e|e.unwrap().path()).find(|p|p.file_name().unwrap().to_string_lossy().starts_with(".long-lzop-recovery")).unwrap();let deepest=stage.join("资料/deep/nested/empty");assert_eq!(filetime::FileTime::from_last_modification_time(&deepest.metadata().unwrap()).unix_seconds(),0);cancel.store(true,Ordering::SeqCst)}}).unwrap_err();assert!(active);assert!(format!("{err:#}").contains("cancelled"));no_stage(temp.path());
 let mut changed=false;let err=recovery::recover(&path,temp.path(),&report.revision,&ids,None,&flag(),|phase,_,_|{if phase=="recovering"&&!changed{changed=true;std::fs::OpenOptions::new().append(true).open(&path).unwrap().write_all(&[0]).unwrap();}}).unwrap_err();assert!(changed);assert!(err.to_string().contains("重新检查"));no_stage(temp.path());assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(),1);
}

#[test]
fn directory1712_missing_middle_volume_keeps_verified_directory_times_and_rechecks_after_repair(){
 let data=std::fs::read(root().join("pax.tar.lzo")).unwrap();let temp=tempfile::tempdir().unwrap();let mut parts=Vec::new();for(i,chunk)in data.chunks(100003).enumerate(){let path=temp.path().join(format!("dirs.tar.lzo.{:03}",i+1));std::fs::write(&path,chunk).unwrap();parts.push(path);}
 let withheld=std::fs::read(&parts[3]).unwrap();std::fs::remove_file(&parts[3]).unwrap();let path=parts.last().unwrap();let report=scan(path);assert_eq!(report.available_volumes,3);assert!(report.ignored_volumes>0);assert_eq!(report.entries.len(),7);
 let result=recovery::recover(path,temp.path(),&report.revision,&[0,1,2,3,4,5,6],None,&flag(),|_,_,_|{}).unwrap();assert_eq!(result.recovered_directories,6);assert_eq!(result.recovered_files,1);exact(&result);assert_eq!(result.time_preservation.as_ref().unwrap().unpreserved_directories,Some(2));
 std::fs::write(&parts[3],withheld).unwrap();assert!(recovery::recover(path,temp.path(),&report.revision,&[0],None,&flag(),|_,_,_|{}).unwrap_err().to_string().contains("重新检查"));assert!(scan(path).complete);no_stage(temp.path());assert_eq!(parts.iter().flat_map(|p|std::fs::read(p).unwrap()).collect::<Vec<_>>(),data);
}
