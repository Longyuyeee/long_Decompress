use super::{lzop::selected::{self,Plan},extraction_transaction as tx,native_extraction::{self,ExtractionRuntime},tar_lzo_prefix_175_tests::wrapped};
use crate::models::compression::{DecompressOptions,TaskLogSeverity};
use std::{io::Read,path::{Path,PathBuf},sync::{Arc,atomic::{AtomicBool,AtomicUsize,Ordering}}};
use sha2::{Digest,Sha256};
use anyhow::Result;
fn root()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives")}
fn base()->PathBuf{let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/tar-selected-1720");std::fs::create_dir_all(&p).unwrap();p}
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
 let cancel=flag();let plan=Plan::build(path,opts,&cancel).unwrap().unwrap();let mut stage=tx::ExtractionStaging::create_for(output).unwrap();let runtime=Runtime::new(cancel.clone());
 plan.extract(&runtime,None,"selected1720",path,stage.path(),opts,&cancel).unwrap();plan.prepare_stage(path,stage.path(),opts).unwrap();let bytes=tx::commit_staged_extraction(path.to_str().unwrap(),stage.path(),output,opts,|_|{}).unwrap();assert!(selected::apply_directory_times(output,&plan.directory_times(opts)).is_empty());assert!(stage.cleanup_after_commit().is_none());
 (bytes,serde_json::json!({"entries":plan.stats.entries,"logicalBytes":plan.stats.logical_bytes,"payloadBytes":plan.stats.payload_bytes}))
}

#[test]
fn selected1720_real_gnu_pax_and_sparse_aliases_publish_only_selected_independent_contents_and_times(){
 let run=base().join(uuid::Uuid::new_v4().to_string());std::fs::create_dir_all(&run).unwrap();let mut cases=Vec::new();
 for(folder,file,ids)in[("tar-hardlink-1717","hardlinks.tar.lzo",vec![2,4,5,7,9,11]),("tar-gnu-longlink-1718","gnu-links.tar.lzo",vec![2,4,5,7,9,11,12]),("tar-browse-links-1719","gnu.tar.lzo",vec![6,7,8,9]),("tar-browse-links-1719","pax.tar.lzo",vec![6,7,8,9])]{
  let path=root().join(folder).join(file);let m=json(&root().join(folder).join("manifest.json"));let variant=if folder=="tar-browse-links-1719"{m["variants"].as_array().unwrap().iter().find(|v|v["archive"]==file).unwrap()}else{&m};let entries=ids.iter().map(|i|variant["entries"][i].clone()).collect::<Vec<_>>();let opts=options(entries.iter().map(|e|e["name"].as_str().unwrap().to_string()).collect());let output=run.join(format!("{}-{}",folder,file));let(bytes,stats)=publish(&path,&output,&opts);
  assert_eq!(sha(&path),variant["sha256"].as_str().unwrap());assert_eq!(bytes,entries.iter().map(|e|e["bytes"].as_u64().unwrap()).sum::<u64>());for e in &entries{let target=output.join(e["name"].as_str().unwrap());if e["isDirectory"]==true{assert!(target.is_dir())}else{assert_eq!(sha(&target),e["sha256"].as_str().unwrap());}}
  cases.push(serde_json::json!({"source":path,"outputDirectory":output,"entries":entries,"publishedBytes":bytes,"stats":stats,"timeSeconds":if folder=="tar-hardlink-1717"{1700001717}else if folder=="tar-gnu-longlink-1718"{1700001718}else{1700001719}}));
 }std::fs::write(base().join("native.json"),serde_json::to_vec_pretty(&cases).unwrap()).unwrap();
}

#[test]
fn selected1720_originals_and_multiple_aliases_are_independent_with_no_extra_targets_or_cache_files(){
 let temp=tempfile::tempdir().unwrap();let source=root().join("tar-gnu-longlink-1718/gnu-links.tar.lzo");let m=json(&root().join("tar-gnu-longlink-1718/manifest.json"));let ids=[3,4,5,6,7,12];let opts=options(ids.iter().map(|i|m["entries"][i]["name"].as_str().unwrap().into()).collect());let output=temp.path().join("out");let(_,stats)=publish(&source,&output,&opts);assert!(stats["payloadBytes"].as_u64().unwrap()<stats["logicalBytes"].as_u64().unwrap()/2);
 std::fs::write(output.join("alias-a.bin"),b"independent").unwrap();let long=m["entries"][7]["name"].as_str().unwrap();std::fs::write(output.join(long),b"separate sparse copy").unwrap();for i in [3,5,6,12]{let e=&m["entries"][i];assert_eq!(sha(&output.join(e["name"].as_str().unwrap())),e["sha256"].as_str().unwrap());}assert_eq!(walkdir::WalkDir::new(&output).into_iter().filter_map(Result::ok).filter(|e|e.file_type().is_file()).count(),6);
}

fn constructed()->Vec<u8>{
 let mut b=tar::Builder::new(Vec::new());for(name,dir,data)in[("alpha",true,b"".as_slice()),("beta",true,b"".as_slice()),("beta/empty",true,b"".as_slice()),("alpha/note.txt",false,b"selected original content\n".as_slice())]{let mut h=tar::Header::new_gnu();h.set_size(data.len()as u64);h.set_mtime(1700001720);h.set_mode(0o755);if dir{h.set_entry_type(tar::EntryType::Directory)}h.set_cksum();b.append_data(&mut h,name,data).unwrap();}let mut h=tar::Header::new_gnu();h.set_path("beta/note.txt").unwrap();h.set_link_name("alpha/note.txt").unwrap();h.set_entry_type(tar::EntryType::Link);h.set_size(0);h.set_mode(0o644);h.set_mtime(1700001721);h.set_cksum();b.append(&h,std::io::empty()).unwrap();b.into_inner().unwrap()
}
#[test]
fn selected1720_folder_filter_flatten_and_original_conflict_confirmation_keep_exact_scope(){
 let temp=tempfile::tempdir().unwrap();let source=temp.path().join("test.tar.lzo");std::fs::write(&source,wrapped(&constructed(),137,true)).unwrap();
 let output=temp.path().join("folder");let opts=options(vec!["beta/".into()]);publish(&source,&output,&opts);assert!(output.join("beta/empty").is_dir());assert!(!output.join("alpha").exists());assert_eq!(filetime::FileTime::from_last_modification_time(&output.join("beta").metadata().unwrap()).unix_seconds(),1700001720);assert_eq!(filetime::FileTime::from_last_modification_time(&output.join("beta/note.txt").metadata().unwrap()).unix_seconds(),1700001721);
 let mut filtered=opts.clone();filtered.file_filter=Some("*.txt".into());let output=temp.path().join("filtered");publish(&source,&output,&filtered);assert!(output.join("beta/note.txt").is_file());assert!(!output.join("beta/empty").exists());
 for policy in ["skip","rename","overwrite","ask"]{let output=temp.path().join(policy);std::fs::create_dir(&output).unwrap();std::fs::write(output.join("note.txt"),b"old").unwrap();let mut opts=options(vec!["alpha/note.txt".into(),"beta/note.txt".into()]);opts.preserve_paths=false;opts.conflict_policy=policy.into();let cancel=flag();let plan=Plan::build(&source,&opts,&cancel).unwrap().unwrap();let mut stage=tx::ExtractionStaging::create_for(&output).unwrap();plan.extract(&Runtime::new(cancel.clone()),None,"flatten",&source,stage.path(),&opts,&cancel).unwrap();plan.prepare_stage(&source,stage.path(),&opts).unwrap();assert!(stage.path().join("note.txt").is_file());assert!(stage.path().join("note (1).txt").is_file());
  let result=tx::commit_staged_extraction(source.to_str().unwrap(),stage.path(),&output,&opts,|_|{});if policy=="ask"{assert!(result.is_err());assert_eq!(std::fs::read(output.join("note.txt")).unwrap(),b"old");assert!(stage.path().join("note.txt").exists());tx::commit_staged_extraction_with_resolutions(source.to_str().unwrap(),stage.path(),&output,&opts,&Default::default(),Some("rename"),|_|{}).unwrap();}else{result.unwrap();}if policy!="overwrite"{assert_eq!(std::fs::read(output.join("note.txt")).unwrap(),b"old");}else{assert_eq!(std::fs::read(output.join("note.txt")).unwrap(),b"selected original content\n");}assert!(stage.cleanup_after_commit().is_none());
 }
}

#[test]
fn selected1720_cancel_and_controlled_storage_failure_during_private_replay_publish_nothing(){
 let source=root().join("tar-gnu-longlink-1718/gnu-links.tar.lzo");for mode in ["cancel","storage"]{let temp=tempfile::tempdir().unwrap();let output=temp.path().join("existing");std::fs::create_dir(&output).unwrap();std::fs::write(output.join("keep.txt"),b"old file").unwrap();let opts=options(vec!["alias-a.bin".into(),"资料/alias-b.bin".into()]);let cancel=flag();let plan=Plan::build(&source,&opts,&cancel).unwrap().unwrap();let mut stage=tx::ExtractionStaging::create_for(&output).unwrap();let stop=cancel.clone();let mut runtime=Runtime::new(cancel.clone());runtime.hook=Arc::new(move|n|{if n==6{if mode=="cancel"{stop.store(true,Ordering::SeqCst);anyhow::bail!("cancelled during replay")}return Err(std::io::Error::from_raw_os_error(112).into());}Ok(())});let error=plan.extract(&runtime,None,"fault",&source,stage.path(),&opts,&cancel).unwrap_err();assert!(runtime.calls.load(Ordering::SeqCst)>=6);if mode=="storage"{assert!(error.chain().any(|e|e.downcast_ref::<std::io::Error>().is_some_and(|e|e.raw_os_error()==Some(112))));}stage.cleanup().unwrap();assert_eq!(std::fs::read(output.join("keep.txt")).unwrap(),b"old file");assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(),1);}
}

#[test]
fn selected1720_source_changes_missing_volumes_and_bad_tail_require_rechecking_before_publish(){
 let temp=tempfile::tempdir().unwrap();let original=root().join("tar-gnu-longlink-1718/gnu-links.tar.lzo");let source=temp.path().join("copy.tar.lzo");std::fs::copy(&original,&source).unwrap();let opts=options(vec!["alias-a.bin".into()]);let cancel=flag();let plan=Plan::build(&source,&opts,&cancel).unwrap().unwrap();filetime::set_file_mtime(&source,filetime::FileTime::from_unix_time(1800001720,0)).unwrap();let stage=temp.path().join("stage");std::fs::create_dir(&stage).unwrap();assert!(plan.extract(&Runtime::new(cancel.clone()),None,"changed",&source,&stage,&opts,&cancel).is_err());assert_eq!(std::fs::read_dir(&stage).unwrap().count(),0);
 let bytes=std::fs::read(&original).unwrap();std::fs::write(&source,&bytes[..bytes.len()-2]).unwrap();assert!(Plan::build(&source,&opts,&cancel).is_err());let mut parts=Vec::new();for(i,b)in bytes.chunks(100003).enumerate(){let p=temp.path().join(format!("vol.tar.lzo.{:03}",i+1));std::fs::write(&p,b).unwrap();parts.push(p);}let missing=std::fs::read(&parts[8]).unwrap();std::fs::remove_file(&parts[8]).unwrap();assert!(Plan::build(parts.last().unwrap(),&opts,&cancel).is_err());std::fs::write(&parts[8],missing).unwrap();let output=temp.path().join("repaired");publish(parts.last().unwrap(),&output,&opts);assert!(output.join("alias-a.bin").exists());assert!(!output.join("source.bin").exists());
}

#[test]
fn selected1720_repaired_output_names_and_missing_selection_do_not_drop_or_expand_requested_content(){
 let temp=tempfile::tempdir().unwrap();let mut b=tar::Builder::new(Vec::new());let mut h=tar::Header::new_gnu();h.set_size(4);h.set_mode(0o644);h.set_mtime(1700001720);h.set_cksum();b.append_data(&mut h,"data?.txt",b"same".as_slice()).unwrap();h.set_size(0);h.set_entry_type(tar::EntryType::Link);h.set_link_name("data?.txt").unwrap();h.set_cksum();b.append_data(&mut h,"copy?.txt",std::io::empty()).unwrap();let source=temp.path().join("names.tar.lzo");std::fs::write(&source,wrapped(&b.into_inner().unwrap(),137,true)).unwrap();let mut opts=options(vec!["copy?.txt".into()]);opts.file_filter=Some("*.txt".into());let output=temp.path().join("out");publish(&source,&output,&opts);assert_eq!(std::fs::read(output.join("copy_.txt")).unwrap(),b"same");assert_eq!(std::fs::read_dir(&output).unwrap().count(),1);opts.selected_entries.push("missing.txt".into());assert!(Plan::build(&source,&opts,&flag()).is_err());opts.selected_entries.clear();assert!(Plan::build(&source,&opts,&flag()).unwrap().is_none());
}

#[test]
fn selected1720_pause_resume_uses_original_task_control_and_ordinary_unselected_specials_keep_old_route(){
 let temp=tempfile::tempdir().unwrap();let source=root().join("tar-gnu-longlink-1718/gnu-links.tar.lzo");let opts=options(vec!["alias-a.bin".into()]);let id=format!("selected1720-{}",uuid::Uuid::new_v4());let cancel=super::task_control::register(&id).unwrap();let plan=Plan::build(&source,&opts,&cancel).unwrap().unwrap();assert!(super::task_control::pause(&id));let stage=temp.path().join("paused");std::fs::create_dir(&stage).unwrap();let worker_stage=stage.clone();let worker_cancel=cancel.clone();let worker=std::thread::spawn(move||plan.extract(&Runtime::new(worker_cancel.clone()),None,"pause",&source,&worker_stage,&opts,&worker_cancel));std::thread::sleep(std::time::Duration::from_millis(50));assert_eq!(std::fs::read_dir(&stage).unwrap().count(),0);assert!(!worker.is_finished());assert!(super::task_control::resume(&id));worker.join().unwrap().unwrap();super::task_control::cleanup(&id);assert!(stage.join("alias-a.bin").is_file());
 let mut b=tar::Builder::new(Vec::new());let mut h=tar::Header::new_gnu();h.set_size(4);h.set_mode(0o644);h.set_cksum();b.append_data(&mut h,"plain.txt",b"data".as_slice()).unwrap();h.set_entry_type(tar::EntryType::Symlink);h.set_size(0);h.set_link_name("plain.txt").unwrap();h.set_cksum();b.append_data(&mut h,"not-selected",std::io::empty()).unwrap();let path=temp.path().join("fallback.tar.lzo");std::fs::write(&path,wrapped(&b.into_inner().unwrap(),137,true)).unwrap();let opts=options(vec!["plain.txt".into()]);assert!(Plan::build(&path,&opts,&flag()).unwrap().is_none());let output=temp.path().join("ordinary");std::fs::create_dir(&output).unwrap();native_extraction::tar::extract_inner(&Runtime::new(flag()),None,"old",path.to_str().unwrap(),&output,Some(super::codec_stream::decoder(&path,"lzo").unwrap()),&opts).unwrap();assert_eq!(std::fs::read(output.join("plain.txt")).unwrap(),b"data");
}

#[test]
fn selected1720_source_change_after_writing_stays_unpublished_and_timestamp_option_is_respected(){
 let temp=tempfile::tempdir().unwrap();let source=temp.path().join("source.tar.lzo");std::fs::copy(root().join("tar-gnu-longlink-1718/gnu-links.tar.lzo"),&source).unwrap();let opts=options(vec!["alias-a.bin".into()]);let cancel=flag();let plan=Plan::build(&source,&opts,&cancel).unwrap().unwrap();let output=temp.path().join("out");let mut stage=tx::ExtractionStaging::create_for(&output).unwrap();let changed=source.clone();let mut runtime=Runtime::new(cancel.clone());runtime.hook=Arc::new(move|n|{if n==2{filetime::set_file_mtime(&changed,filetime::FileTime::from_unix_time(1800001721,0))?;}Ok(())});let error=plan.extract(&runtime,None,"source-change",&source,stage.path(),&opts,&cancel).unwrap_err();assert!(format!("{error:#}").contains("IMAGE_READER_CHANGED"));assert!(stage.path().join("alias-a.bin").exists());assert!(!output.exists());stage.cleanup().unwrap();
 let mut opts=opts;opts.preserve_timestamps=false;publish(&source,&output,&opts);assert_ne!(filetime::FileTime::from_last_modification_time(&output.join("alias-a.bin").metadata().unwrap()).unix_seconds(),1700001718);
}

#[test]
fn selected1720_single_alias_ui_receipt_matches_actual_committed_output(){
 let source=root().join("tar-browse-links-1719/gnu.tar.lzo");let output=base().join(format!("single-{}",uuid::Uuid::new_v4()));let opts=options(vec!["阅读副本.txt".into()]);let(bytes,stats)=publish(&source,&output,&opts);let manifest=json(&root().join("tar-browse-links-1719/manifest.json"));let entry=manifest["variants"][0]["entries"][6].clone();assert_eq!(std::fs::read_dir(&output).unwrap().count(),1);assert_eq!(sha(&output.join("阅读副本.txt")),entry["sha256"].as_str().unwrap());std::fs::write(base().join("single.json"),serde_json::to_vec_pretty(&serde_json::json!({"source":source,"outputDirectory":output,"publishedBytes":bytes,"stats":stats,"entries":[entry]})).unwrap()).unwrap();
}
