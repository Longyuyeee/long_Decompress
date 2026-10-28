use super::{lzop::selected::{self,Plan},extraction_transaction as tx,native_extraction::{self,ExtractionRuntime},tar_lzo_prefix_175_tests::wrapped};
use crate::models::compression::{DecompressOptions,TaskLogSeverity};
use std::{io::Read,path::{Path,PathBuf},sync::{Arc,atomic::{AtomicBool,AtomicUsize,Ordering}}};
use sha2::{Digest,Sha256};
use anyhow::Result;
fn root()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives")}
fn base()->PathBuf{let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/tar-full-links-1721");std::fs::create_dir_all(&p).unwrap();p}
fn flag()->Arc<AtomicBool>{Arc::new(AtomicBool::new(false))}
fn json(path:&Path)->serde_json::Value{serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()}
fn sha(path:&Path)->String{let mut file=std::fs::File::open(path).unwrap();let mut buf=[0;65536];let mut h=Sha256::new();loop{let n=file.read(&mut buf).unwrap();if n==0{break}h.update(&buf[..n]);}format!("{:x}",h.finalize())}
fn options(names:Vec<String>)->DecompressOptions{DecompressOptions{selected_entries:names,preserve_paths:true,preserve_timestamps:true,conflict_policy:"rename".into(),..Default::default()}}
struct Runtime{pool:super::io_buffer_pool::IOBufferPool,flag:Arc<AtomicBool>,calls:AtomicUsize,hook:Arc<dyn Fn(usize)->Result<()>+Send+Sync>}
impl Runtime{fn new(flag:Arc<AtomicBool>)->Self{Self{pool:super::io_buffer_pool::IOBufferPool::new(Default::default()),flag,calls:AtomicUsize::new(0),hook:Arc::new(|_|Ok(()))}}}
impl ExtractionRuntime for Runtime{
 fn check_cancellation(&self)->Result<()>{let n=self.calls.fetch_add(1,Ordering::SeqCst)+1;(self.hook)(n)?;anyhow::ensure!(!self.flag.load(Ordering::SeqCst),"test cancelled");Ok(())}
 fn buffer_pool(&self)->&super::io_buffer_pool::IOBufferPool{&self.pool}
 fn copy_buffer_size(&self)->usize{65536}
 fn normalized_archive_path(&self,path:&Path,preserve:bool)->Option<PathBuf>{if preserve{Some(path.to_path_buf())}else{path.file_name().map(PathBuf::from)}}
 fn emit_log(&self,_:&tauri::Window,_:&str,_:&str,_:TaskLogSeverity){}
 fn emit_progress(&self,_:&tauri::Window,_:&str,_:f32,_:Option<String>,_:u64,_:u64){}
}
fn publish(path:&Path,output:&Path,opts:&DecompressOptions)->(u64,serde_json::Value){
 let cancel=flag();let plan=Plan::for_extraction(path,opts,&cancel).unwrap().unwrap();let mut stage=tx::ExtractionStaging::create_for(output).unwrap();let runtime=Runtime::new(cancel.clone());
 plan.extract(&runtime,None,"full1721",path,stage.path(),opts,&cancel).unwrap();plan.prepare_stage(path,stage.path(),opts).unwrap();let bytes=tx::commit_staged_extraction(path.to_str().unwrap(),stage.path(),output,opts,|_|{}).unwrap();assert!(selected::apply_directory_times(output,&plan.directory_times(opts)).is_empty());assert!(stage.cleanup_after_commit().is_none());
 (bytes,serde_json::json!({"entries":plan.stats.entries,"logicalBytes":plan.stats.logical_bytes,"payloadBytes":plan.stats.payload_bytes}))
}

fn original(folder:&str,file:&str)->(PathBuf,serde_json::Value){
 let path=root().join(folder).join(file);let m=json(&root().join(folder).join("manifest.json"));let variant=m["variants"].as_array().and_then(|vs|vs.iter().find(|v|v["archive"]==file)).unwrap_or(&m).clone();(path,variant)
}
#[test]
fn full1721_real_gnu_pax_and_sparse_originals_publish_complete_independent_contents(){
 let run=base().join(uuid::Uuid::new_v4().to_string());std::fs::create_dir_all(&run).unwrap();let mut cases=Vec::new();
 for(folder,file)in[("tar-hardlink-1717","hardlinks.tar.lzo"),("tar-gnu-longlink-1718","gnu-links.tar.lzo"),("tar-browse-links-1719","gnu.tar.lzo"),("tar-browse-links-1719","pax.tar.lzo")]{
  let(path,m)=original(folder,file);let opts=options(vec![]);let output=run.join(format!("{folder}-{file}"));let(bytes,stats)=publish(&path,&output,&opts);let entries=m["entries"].as_array().unwrap();assert_eq!(stats["entries"].as_u64().unwrap()as usize,entries.len());assert_eq!(bytes,entries.iter().map(|e|e["bytes"].as_u64().unwrap()).sum::<u64>());
  for entry in entries {let target=output.join(entry["name"].as_str().unwrap());if entry["isDirectory"]==true{assert!(target.is_dir())}else{assert_eq!(sha(&target),entry["sha256"].as_str().unwrap());}}
  assert_eq!(sha(&path),m["sha256"].as_str().unwrap());cases.push(serde_json::json!({"source":path,"outputDirectory":output,"entries":entries,"publishedBytes":bytes,"stats":stats}));
 }std::fs::write(base().join("native.json"),serde_json::to_vec_pretty(&cases).unwrap()).unwrap();
}
#[tokio::test]
async fn full1721_public_integrity_native_verification_and_diagnosis_agree_without_output(){
 let mut checks=Vec::new();
 for(folder,file)in[("tar-hardlink-1717","hardlinks.tar.lzo"),("tar-gnu-longlink-1718","gnu-links.tar.lzo"),("tar-browse-links-1719","gnu.tar.lzo"),("tar-browse-links-1719","pax.tar.lzo")]{
  let(path,m)=original(folder,file);let before=sha(&path);
  let result=crate::commands::compression::test_archive_integrity(path.to_string_lossy().into_owned(),None,None,None).await.unwrap();
  assert!(super::compression_verification::verify_native(super::compression_format::CompressionRoute::TarLzop,&path,None,||false).unwrap());
  let report=super::archive_browser::browse_archive(&path,None).await.unwrap();assert_eq!(report.total_files,m["entries"].as_array().unwrap().iter().filter(|e|e["isDirectory"]!=true).count());assert_eq!(report.total_uncompressed_size,m["entries"].as_array().unwrap().iter().map(|e|e["bytes"].as_u64().unwrap()).sum::<u64>());
  let diagnosis=super::archive_diagnostics::diagnose_archive(&path,None,flag()).await.unwrap();assert_eq!(diagnosis.status,"healthy");assert!(diagnosis.integrity_tested);assert!(!diagnosis.can_repair);assert_eq!(diagnosis.total_files,report.total_files);assert_eq!(sha(&path),before);
  checks.push(serde_json::json!({"source":path,"integrity":result,"browse":report,"diagnosis":diagnosis}));
 }std::fs::write(base().join("integrity.json"),serde_json::to_vec_pretty(&checks).unwrap()).unwrap();
}
fn tar(entries:&[(&str,u8,&str,&[u8],u64)])->Vec<u8>{
 let mut builder=tar::Builder::new(Vec::new());for(name,kind,target,bytes,time)in entries{let mut header=tar::Header::new_gnu();header.set_entry_type(tar::EntryType::new(*kind));header.set_size(bytes.len()as u64);header.set_mtime(*time);header.set_mode(0o755);if !target.is_empty(){header.set_link_name(target).unwrap();}header.set_cksum();builder.append_data(&mut header,*name,*bytes).unwrap();}builder.into_inner().unwrap()
}
fn sample()->Vec<u8>{tar(&[("one",b'5',"",b"",1700001720),("two",b'5',"",b"",1700001721),("two/empty",b'5',"",b"",1700001722),("one/note.txt",b'0',"",b"full original\n",1700001723),("two/note.txt",b'1',"one/note.txt",b"",1700001724)])}
fn blocked(path:&Path){
 let output=path.parent().unwrap().join("not-published");let cancel=flag();let opts=options(vec![]);let mut stage=tx::ExtractionStaging::create_for(&output).unwrap();
 let result=(||->Result<()>{if let Some(plan)=Plan::for_extraction(path,&opts,&cancel)?{plan.extract(&Runtime::new(cancel.clone()),None,"full1721",path,stage.path(),&opts,&cancel)?;}else{native_extraction::tar::extract_inner(&Runtime::new(cancel),None,"fallback",path.to_str().unwrap(),stage.path(),Some(super::codec_stream::decoder(path,"lzo")?),&opts)?;}Ok(())})();assert!(result.is_err(),"{}",path.display());assert!(!output.exists());stage.cleanup().unwrap();assert!(!stage.path().exists());
}
#[tokio::test]
async fn full1721_unconfirmed_forward_ambiguous_chain_directory_and_body_links_are_not_healthy(){
 let cases=vec![
  tar(&[("alias",b'1',"later",b"",1),("later",b'0',"",b"later",2)]),
  tar(&[("source",b'0',"",b"a",1),("alias",b'1',"source",b"",2),("chain",b'1',"alias",b"",3)]),
  tar(&[("source",b'0',"",b"a",1),("source",b'0',"",b"b",2),("alias",b'1',"source",b"",3)]),
  tar(&[("dir",b'5',"",b"",1),("alias",b'1',"dir",b"",2)]),
  tar(&[("source",b'0',"",b"a",1),("alias",b'1',"source",b"body",2)]),
  tar(&[("sym",b'2',"outside",b"",1),("source",b'0',"",b"a",2),("alias",b'1',"source",b"",3)]),
  tar(&[("source",b'0',"",b"a",1),("alias",b'1',"missing",b"",2)])
 ];let temp=tempfile::tempdir().unwrap();let mut errors=Vec::new();
 for(i,data)in cases.iter().enumerate(){let path=temp.path().join(format!("bad-{i}.tar.lzo"));std::fs::write(&path,wrapped(data,127,true)).unwrap();let error=crate::commands::compression::test_archive_integrity(path.to_string_lossy().into_owned(),None,None,None).await.unwrap_err();assert!(!error.contains("password_required"));blocked(&path);errors.push(error);}
 std::fs::write(base().join("invalid.json"),serde_json::to_vec_pretty(&errors).unwrap()).unwrap();
}
#[tokio::test]
async fn full1721_bad_tail_checksum_and_numeric_gap_never_publish_partial_results(){
 let temp=tempfile::tempdir().unwrap();let data=sample();let mut damaged=data.clone();damaged.truncate(damaged.len()-600);let mut trailing=data.clone();trailing.extend_from_slice(&[7;512]);let mut bad_checksum=wrapped(&data,1000,true);let last=bad_checksum.len()-6;bad_checksum[last]^=1;let mut concatenated=wrapped(&data,1000,true);concatenated.extend_from_slice(&wrapped(&data,1000,true));
 for(i,bytes)in [wrapped(&damaged,1000,true),wrapped(&trailing,1000,true),bad_checksum,concatenated].into_iter().enumerate(){let path=temp.path().join(format!("tail-{i}.tar.lzo"));std::fs::write(&path,bytes).unwrap();assert!(crate::commands::compression::test_archive_integrity(path.to_string_lossy().into_owned(),None,None,None).await.is_err());blocked(&path);}
 let(source,m)=original("tar-browse-links-1719","gnu.tar.lzo");let bytes=std::fs::read(&source).unwrap();let third=bytes.len()/3;let first=temp.path().join("full.tar.lzo.001");std::fs::write(&first,&bytes[..third]).unwrap();std::fs::write(temp.path().join("full.tar.lzo.003"),&bytes[third*2..]).unwrap();assert!(super::universal_engine::UniversalCliEngine::test_integrity(&first,None).await.is_err());blocked(&first);std::fs::write(temp.path().join("full.tar.lzo.002"),&bytes[third..third*2]).unwrap();super::universal_engine::UniversalCliEngine::test_integrity(&first,None).await.unwrap();let output=base().join(format!("numeric-{}",uuid::Uuid::new_v4()));let(bytes,stats)=publish(&first,&output,&options(vec![]));std::fs::write(base().join("numeric.json"),serde_json::to_vec_pretty(&serde_json::json!({"source":source,"readSource":first,"outputDirectory":output,"publishedBytes":bytes,"stats":stats,"entries":m["entries"]})).unwrap()).unwrap();
}
#[test]
fn full1721_full_filter_flatten_conflicts_and_per_alias_times_keep_original_choices(){
 let temp=tempfile::tempdir().unwrap();let source=temp.path().join("complete.tar.lzo");std::fs::write(&source,wrapped(&sample(),137,true)).unwrap();let opts=options(vec![]);let output=temp.path().join("complete");publish(&source,&output,&opts);assert_eq!(filetime::FileTime::from_last_modification_time(&output.join("one/note.txt").metadata().unwrap()).unix_seconds(),1700001723);assert_eq!(filetime::FileTime::from_last_modification_time(&output.join("two/note.txt").metadata().unwrap()).unix_seconds(),1700001724);assert_eq!(filetime::FileTime::from_last_modification_time(&output.join("two").metadata().unwrap()).unix_seconds(),1700001721);
 for policy in ["skip","rename","overwrite","ask"]{let output=temp.path().join(policy);std::fs::create_dir(&output).unwrap();std::fs::write(output.join("note.txt"),b"old").unwrap();let mut opts=opts.clone();opts.preserve_paths=false;opts.file_filter=Some("*.txt".into());opts.conflict_policy=policy.into();let cancel=flag();let plan=Plan::for_extraction(&source,&opts,&cancel).unwrap().unwrap();assert_eq!(plan.stats.entries,2);let mut stage=tx::ExtractionStaging::create_for(&output).unwrap();plan.extract(&Runtime::new(cancel.clone()),None,"flatten-full",&source,stage.path(),&opts,&cancel).unwrap();plan.prepare_stage(&source,stage.path(),&opts).unwrap();assert!(stage.path().join("note (1).txt").is_file());let result=tx::commit_staged_extraction(source.to_str().unwrap(),stage.path(),&output,&opts,|_|{});if policy=="ask"{assert!(result.is_err());assert_eq!(std::fs::read(output.join("note.txt")).unwrap(),b"old");tx::commit_staged_extraction_with_resolutions(source.to_str().unwrap(),stage.path(),&output,&opts,&Default::default(),Some("rename"),|_|{}).unwrap();}else{result.unwrap();}if policy=="overwrite"{assert_eq!(std::fs::read(output.join("note.txt")).unwrap(),b"full original\n")}else{assert_eq!(std::fs::read(output.join("note.txt")).unwrap(),b"old")}assert!(stage.cleanup_after_commit().is_none());}
 std::fs::write(output.join("one/note.txt"),b"independent").unwrap();assert_eq!(std::fs::read(output.join("two/note.txt")).unwrap(),b"full original\n");
}
#[test]
fn full1721_cancel_storage_and_source_change_leave_existing_output_intact(){
 let temp=tempfile::tempdir().unwrap();let source=temp.path().join("copy.tar.lzo");std::fs::write(&source,wrapped(&sample(),137,true)).unwrap();let opts=options(vec![]);
 for mode in ["cancel","storage","changed"]{let output=temp.path().join(mode);std::fs::create_dir(&output).unwrap();std::fs::write(output.join("keep.txt"),b"keep").unwrap();let cancel=flag();let plan=Plan::for_extraction(&source,&opts,&cancel).unwrap().unwrap();let stop=cancel.clone();let changed=source.clone();let mut runtime=Runtime::new(cancel.clone());runtime.hook=Arc::new(move|n|{if n==4 {match mode{"cancel"=>{stop.store(true,Ordering::SeqCst);anyhow::bail!("cancelled")},"storage"=>return Err(std::io::Error::from_raw_os_error(112).into()),_=>filetime::set_file_mtime(&changed,filetime::FileTime::from_unix_time(1800001721,0))?}}Ok(())});let mut stage=tx::ExtractionStaging::create_for(&output).unwrap();assert!(plan.extract(&runtime,None,mode,&source,stage.path(),&opts,&cancel).is_err());assert_eq!(std::fs::read_dir(&output).unwrap().count(),1);assert_eq!(std::fs::read(output.join("keep.txt")).unwrap(),b"keep");stage.cleanup().unwrap();}
 let calls=AtomicUsize::new(0);assert!(selected::verify_complete(&source,None,||calls.fetch_add(1,Ordering::SeqCst)>2).is_err());
 let calls=AtomicUsize::new(0);let changed=source.clone();let error=selected::verify_complete(&source,None,||{if calls.fetch_add(1,Ordering::SeqCst)==3{filetime::set_file_mtime(&changed,filetime::FileTime::from_unix_time(1900001721,0)).unwrap();}false}).unwrap_err();assert!(format!("{error:#}").contains("IMAGE_READER_CHANGED"));
}
#[tokio::test]
async fn full1721_ordinary_and_no_checksum_compatibility_fallback_remain_available(){
 let temp=tempfile::tempdir().unwrap();let raw=tar(&[("normal.txt",b'0',"",b"ordinary",1)]);let opts=options(vec![]);
 for checked in [true,false]{let path=temp.path().join(format!("ordinary-{checked}.tar.lzo"));std::fs::write(&path,wrapped(&raw,137,checked)).unwrap();assert!(Plan::for_extraction(&path,&opts,&flag()).unwrap().is_none());super::universal_engine::UniversalCliEngine::test_integrity(&path,None).await.unwrap();let output=temp.path().join(format!("ordinary-{checked}"));std::fs::create_dir(&output).unwrap();native_extraction::tar::extract_inner(&Runtime::new(flag()),None,"ordinary",path.to_str().unwrap(),&output,Some(super::codec_stream::decoder(&path,"lzo").unwrap()),&opts).unwrap();assert_eq!(std::fs::read(output.join("normal.txt")).unwrap(),b"ordinary");}
 let path=temp.path().join("unchecked-links.tar.lzo");std::fs::write(&path,wrapped(&sample(),137,false)).unwrap();assert!(super::universal_engine::UniversalCliEngine::test_integrity(&path,None).await.is_err());blocked(&path);
 let p=original("tar-browse-links-1719","gnu.tar.lzo").0;let selected_opts=options(vec!["阅读副本.txt".into()]);let output=temp.path().join("still-selected");publish(&p,&output,&selected_opts);assert_eq!(std::fs::read_dir(output).unwrap().count(),1);
}
