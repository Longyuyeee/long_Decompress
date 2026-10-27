use super::{lzop::recovery, tar_recovery_time::ModificationTime, tar_lzo_prefix_175_tests::wrapped};
use sha2::{Digest, Sha256};
use std::{path::{Path,PathBuf}, sync::atomic::{AtomicBool,Ordering}};
fn flag()->AtomicBool {AtomicBool::new(false)}
fn root()->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/tar-time-1711")}
fn base()->PathBuf {let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/tar-time-1711");std::fs::create_dir_all(&p).unwrap();p}
fn sha(data:&[u8])->String {format!("{:x}",Sha256::digest(data))}
fn scan(path:&Path)->recovery::Report {recovery::scan(path,None,&flag(),|_,_|{}).unwrap()}
fn no_stage(path:&Path){assert!(!std::fs::read_dir(path).unwrap().any(|e|e.unwrap().file_name().to_string_lossy().starts_with(".long-lzop-recovery")))}

#[test]
fn time1711_exact_signed_decimal_and_gnu_binary_headers_do_not_round_through_float(){
 for(value,seconds,nanoseconds)in[("1700001711.123456700",1700001711,123456700),("-1.123456700",-2,876543300),("-0.25",-1,750000000),("-1",-1,0),("0",0,0),("1.0000000000",1,0)]{
  assert_eq!(ModificationTime::pax(value),Some(ModificationTime{seconds,nanoseconds}));
 }
 for value in["NaN","inf","1e3","1.1234567891","--1","1.-2","","1000000000001"]{assert!(ModificationTime::pax(value).is_none(),"{value}")}
 let mut h=tar::Header::new_gnu();h.as_gnu_mut().unwrap().mtime.fill(255);h.as_gnu_mut().unwrap().mtime[11]=254;
 assert_eq!(ModificationTime::header(&h),Some(ModificationTime{seconds:-2,nanoseconds:0}));
 h.set_mtime(1700001711);assert_eq!(ModificationTime::header(&h).unwrap().seconds,1700001711);
 let file=tempfile::tempfile().unwrap();let read_only=std::fs::File::open(std::env::current_exe().unwrap()).unwrap();
 assert!(ModificationTime{seconds:1700001711,nanoseconds:0}.apply(&read_only).is_err());
 assert!(ModificationTime{seconds:1700001711,nanoseconds:0}.apply(&file).is_ok());
}

#[test]
fn time1711_independent_gnu_pax_originals_preserve_real_outputs_and_keep_time_limits_separate(){
 let m:serde_json::Value=serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap();
 let run=base().join(uuid::Uuid::new_v4().to_string());std::fs::create_dir_all(&run).unwrap();let mut evidence=Vec::new();
 for case in m["cases"].as_array().unwrap(){
  let original=root().join(case["archive"].as_str().unwrap());let bytes=std::fs::read(&original).unwrap();assert_eq!(sha(&bytes),case["sha256"].as_str().unwrap());
  let source=run.join(format!("{}-tail.tar.lzo",case["name"].as_str().unwrap()));std::fs::write(&source,&bytes[..bytes.len()-2]).unwrap();let report=scan(&source);assert!(!report.complete);assert_eq!(report.entries.len(),8);
  for(e,x)in report.entries.iter().zip(case["entries"].as_array().unwrap()) {assert_eq!(e.name,x["name"].as_str().unwrap());if !e.is_directory{assert_eq!(serde_json::to_value(&e.modified_time).unwrap(),x["modifiedTime"]);}}
  let ids=report.entries.iter().map(|e|e.id).collect::<Vec<_>>();let result=recovery::recover(&source,&run,&report.revision,&ids,None,&flag(),|_,_,_|{}).unwrap();assert_eq!(result.recovered_files,7);assert_eq!(result.recovered_directories,1);
  let mut actual=Vec::new();for e in &result.entries {let path=Path::new(&result.output_directory).join(&e.name);if e.is_directory{assert!(path.is_dir());continue;}assert_eq!(sha(&std::fs::read(&path).unwrap()),e.sha256);
   let mt=filetime::FileTime::from_last_modification_time(&path.metadata().unwrap());let expected=e.modified_time.as_ref().unwrap();
   if e.time_warning.is_none(){assert_eq!((mt.unix_seconds(),mt.nanoseconds()),(expected.seconds,expected.nanoseconds));}
   #[cfg(windows)] if case["name"]=="pax"&&(e.name=="precision.txt"||e.name=="outside-windows.txt"){assert!(e.time_warning.is_some());}
   actual.push(serde_json::json!({"name":e.name,"actualSeconds":mt.unix_seconds(),"actualNanoseconds":mt.nanoseconds(),"warning":e.time_warning}));
  }
  let times=result.time_preservation.as_ref().unwrap();assert_eq!(times.preserved_files+times.unpreserved_files,7);assert_eq!(std::fs::read(&original).unwrap(),bytes);no_stage(&run);
  evidence.push(serde_json::json!({"case":case["name"],"source":source,"report":report,"result":result,"actualTimes":actual,"entries":case["entries"]}));
 }
 std::fs::write(base().join("native.json"),serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
}

#[test]
fn time1711_global_local_deletion_and_inheritance_use_effective_times_and_selected_files_only(){
 let old=Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/tar-pax-179");let m:serde_json::Value=serde_json::from_slice(&std::fs::read(old.join("manifest.json")).unwrap()).unwrap();let case=m["cases"].as_array().unwrap().iter().find(|c|c["name"]=="state").unwrap();let data=std::fs::read(old.join(case["archive"].as_str().unwrap())).unwrap();assert_eq!(sha(&data),case["sha256"].as_str().unwrap());
 let temp=tempfile::tempdir().unwrap();let path=temp.path().join("state.tar.lzo");std::fs::write(&path,&data[..data.len()-2]).unwrap();let report=scan(&path);assert_eq!(report.entries.len(),case["entries"].as_array().unwrap().len());
 for(e,x)in report.entries.iter().zip(case["entries"].as_array().unwrap()){if !e.is_directory {let t=e.modified_time.as_ref().unwrap();assert_eq!(t, &ModificationTime::pax(&x["mtime"].to_string()).unwrap());}}
 let ids=report.entries.iter().filter(|e|!e.is_directory&&e.name!="late.bin").map(|e|e.id).collect::<Vec<_>>();let result=recovery::recover(&path,temp.path(),&report.revision,&ids,None,&flag(),|_,_,_|{}).unwrap();assert_eq!(result.recovered_files,ids.len());assert_eq!(result.time_preservation.as_ref().unwrap().unpreserved_files,0);
 for e in &result.entries{let p=Path::new(&result.output_directory).join(&e.name);assert_eq!(sha(&std::fs::read(&p).unwrap()),e.sha256);let t=filetime::FileTime::from_last_modification_time(&p.metadata().unwrap());let x=e.modified_time.as_ref().unwrap();assert_eq!((t.unix_seconds(),t.nanoseconds()),(x.seconds,x.nanoseconds));}
 let dir=report.entries.iter().find(|e|e.is_directory).unwrap();let result=recovery::recover(&path,temp.path(),&report.revision,&[dir.id],None,&flag(),|_,_,_|{}).unwrap();assert_eq!(result.time_preservation.unwrap().preserved_files,0);assert_eq!(result.recovered_files,0);no_stage(temp.path());
}

#[test]
fn time1711_active_cancel_and_source_replacement_discard_times_and_staged_content(){
 let raw=std::fs::read(root().join("pax.tar")).unwrap();let temp=tempfile::tempdir().unwrap();let path=temp.path().join("cancel.tar.lzo");let data=wrapped(&raw,137,true);std::fs::write(&path,&data[..data.len()-2]).unwrap();let report=scan(&path);let id=report.entries.iter().find(|e|e.name=="late.bin").unwrap().id;
 let cancel=flag();let mut active=false;let err=recovery::recover(&path,temp.path(),&report.revision,&[id],None,&cancel,|phase,done,_|{if phase=="recovering"&&done>65536{active=true;cancel.store(true,Ordering::SeqCst)}}).unwrap_err();assert!(active);assert!(format!("{err:#}").contains("cancelled"));no_stage(temp.path());
 let err=recovery::recover(&path,temp.path(),&report.revision,&[id],None,&flag(),|phase,done,total|{if phase=="recovering"&&done==total{std::fs::OpenOptions::new().append(true).open(&path).unwrap().write_all(&[0]).unwrap();}}).unwrap_err();assert!(err.to_string().contains("重新检查"));no_stage(temp.path());
 use std::io::Write;
 assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(),1);
}

#[test]
fn time1711_unsupported_time_precision_and_huge_header_keep_content_recoverable(){
 let mut builder=tar::Builder::new(Vec::new());
 for (name,pax) in [("huge.txt",false),("extra-precision.txt",true)] {
  if pax {builder.append_pax_extensions([("mtime",b"1.1234567891".as_slice())]).unwrap();}
  let mut h=tar::Header::new_gnu();h.set_path(name).unwrap();h.set_size(7);h.set_mode(0o644);h.set_mtime(u64::MAX);h.set_cksum();builder.append(&h,b"content".as_slice()).unwrap();
 }
 let raw=builder.into_inner().unwrap();let data=wrapped(&raw,137,true);let temp=tempfile::tempdir().unwrap();let path=temp.path().join("times.tar.lzo");std::fs::write(&path,&data[..data.len()-2]).unwrap();let report=scan(&path);assert_eq!(report.entries.len(),2);assert!(report.entries.iter().all(|e|e.modified_time.is_none()));
 let result=recovery::recover(&path,temp.path(),&report.revision,&[0,1],None,&flag(),|_,_,_|{}).unwrap();assert_eq!(result.time_preservation.as_ref().unwrap().unpreserved_files,2);assert!(result.entries.iter().all(|e|e.time_warning.is_some()));for e in &result.entries{assert_eq!(std::fs::read(Path::new(&result.output_directory).join(&e.name)).unwrap(),b"content");}no_stage(temp.path());
}
