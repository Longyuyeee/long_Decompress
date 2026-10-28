use super::{archive_browser,archive_entry_open::{ArchiveEntryOpenCache,NestedArchiveMaterializeResult},nested_archive,native_extraction::ExtractionRuntime,tar_lzo_prefix_175_tests::wrapped};
use crate::models::compression::{DecompressOptions,TaskLogSeverity};
use std::{path::{Path,PathBuf},sync::{Arc,atomic::{AtomicBool,AtomicUsize,Ordering}}};
use sha2::{Digest,Sha256};
use anyhow::Result;
fn root()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/tar-nested-links-1723")}
fn base()->PathBuf{let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/tar-nested-links-1723");std::fs::create_dir_all(&p).unwrap();p}
fn sha(path:&Path)->String{format!("{:x}",Sha256::digest(std::fs::read(path).unwrap()))}
fn flag()->Arc<AtomicBool>{Arc::new(AtomicBool::new(false))}
struct Runtime{pool:super::io_buffer_pool::IOBufferPool,calls:AtomicUsize,hook:Arc<dyn Fn(usize)->Result<()>+Send+Sync>}
impl Runtime{fn new()->Self{Self{pool:super::io_buffer_pool::IOBufferPool::new(Default::default()),calls:AtomicUsize::new(0),hook:Arc::new(|_|Ok(()))}}}
impl ExtractionRuntime for Runtime{
 fn check_cancellation(&self)->Result<()>{(self.hook)(self.calls.fetch_add(1,Ordering::SeqCst)+1)}
 fn buffer_pool(&self)->&super::io_buffer_pool::IOBufferPool{&self.pool}
 fn copy_buffer_size(&self)->usize{65536}
 fn normalized_archive_path(&self,path:&Path,_:bool)->Option<PathBuf>{Some(path.into())}
 fn emit_log(&self,_:&tauri::Window,_:&str,_:&str,_:TaskLogSeverity){}
 fn emit_progress(&self,_:&tauri::Window,_:&str,_:f32,_:Option<String>,_:u64,_:u64){}
}
async fn materialize(cache:&ArchiveEntryOpenCache,rt:Arc<Runtime>,source:&Path,entry:&str,depth:usize,revision:Option<String>,cancel:Arc<AtomicBool>)->Result<NestedArchiveMaterializeResult>{
 nested_archive::materialize(cache,rt,source.into(),entry.into(),None,None,None,depth,vec![],revision,cancel,|_,_|async{anyhow::bail!("Link must use the confirmed plan")}).await
}
fn file_count(path:&Path)->usize{walkdir::WalkDir::new(path).into_iter().filter_map(|e|e.ok()).filter(|e|e.file_type().is_file()).count()}

#[tokio::test]
async fn nested1723_real_gnu_pax_aliases_open_exact_zip_encrypted_sevenz_long_names_and_numeric_outer(){
 let manifest:serde_json::Value=serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap();let mut evidence=Vec::new();
 for variant in manifest["variants"].as_array().unwrap(){let source=root().join(variant["archive"].as_str().unwrap());assert_eq!(sha(&source),variant["sha256"]);let cache=ArchiveEntryOpenCache::new(base().join(format!("cache-{}",uuid::Uuid::new_v4())));
  let listing=archive_browser::browse_archive_cancellable(&source,None,flag()).await.unwrap();let revision=super::image_reader::revision(&source).unwrap();
  for alias in manifest["aliases"].as_array().unwrap().iter().filter(|a|!a.as_str().unwrap().ends_with("fake.zip")){
   let alias=alias.as_str().unwrap();let before=sha(&source);let receipt=materialize(&cache,Arc::new(Runtime::new()),&source,alias,2,Some(revision.clone()),flag()).await.unwrap();let path=Path::new(&receipt.cache_path);assert!(path.ends_with(alias.replace('/',std::path::MAIN_SEPARATOR_STR)));assert_eq!(receipt.entry_path,alias);let expected=variant["entries"].as_array().unwrap().iter().find(|e|e["name"]==alias).unwrap();assert_eq!(sha(path),expected["sha256"]);assert_eq!(sha(&source),before);
   let encrypted=alias.ends_with(".7z");let password=if encrypted{Some("Nested1723!")}else{None};let child=archive_browser::browse_archive_cancellable(path,password,flag()).await.unwrap();if encrypted{assert!(archive_browser::browse_archive_cancellable(path,Some("wrong"),flag()).await.is_err());}
   let text=super::archive_preview::preview_archive_text(path,if encrypted{"secret.txt"}else{"readme.txt"},password).await.unwrap();assert!(!text.content.is_empty());
   let retained=base().join(format!("{}-{}-{}",variant["format"].as_str().unwrap(),evidence.len(),path.file_name().unwrap().to_string_lossy()));std::fs::copy(path,&retained).unwrap();
   evidence.push(serde_json::json!({"source":source,"alias":alias,"receipt":receipt,"retained":retained,"outer":listing,"child":child,"text":text,"encrypted":encrypted,"revision":revision}));
  }
 }
 let temp=tempfile::tempdir().unwrap();let bytes=std::fs::read(root().join("gnu.tar.lzo")).unwrap();let amount=bytes.len().div_ceil(3);for(i,part)in bytes.chunks(amount).enumerate(){std::fs::write(temp.path().join(format!("outer.tar.lzo.{:03}",i+1)),part).unwrap();}
 let source=temp.path().join("outer.tar.lzo.001");let cache=ArchiveEntryOpenCache::new(temp.path().join("cache"));let receipt=materialize(&cache,Arc::new(Runtime::new()),&source,"links/copy.zip",2,None,flag()).await.unwrap();assert_eq!(receipt.content_sha256,manifest["childSha256"]);let retained=base().join("numeric-copy.zip");std::fs::copy(&receipt.cache_path,&retained).unwrap();evidence.push(serde_json::json!({"numericOuter":true,"receipt":receipt,"retained":retained,"expectedSha256":manifest["childSha256"]}));
 std::fs::write(base().join("native.json"),serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
}

#[tokio::test]
async fn nested1723_source_revision_change_cancellation_and_storage_failure_keep_outer_and_release_private_cache(){
 let temp=tempfile::tempdir().unwrap();let source=temp.path().join("outer.tar.lzo");std::fs::copy(root().join("gnu.tar.lzo"),&source).unwrap();let cache_root=temp.path().join("cache");let cache=ArchiveEntryOpenCache::new(cache_root.clone());let old=super::image_reader::revision(&source).unwrap();filetime::set_file_mtime(&source,filetime::FileTime::from_unix_time(1900001723,0)).unwrap();let error=materialize(&cache,Arc::new(Runtime::new()),&source,"links/copy.zip",2,Some(old),flag()).await.unwrap_err();assert!(format!("{error:#}").contains("IMAGE_READER_CHANGED"));
 for mode in ["change","cancel","storage"]{let mut rt=Runtime::new();let cancel=flag();let signal=cancel.clone();let path=source.clone();let original=sha(&source);rt.hook=Arc::new(move|n|{if n==2{match mode{"change"=>filetime::set_file_mtime(&path,filetime::FileTime::from_unix_time(1900001724,0)).unwrap(),"cancel"=>signal.store(true,Ordering::SeqCst),_=>return Err(std::io::Error::from_raw_os_error(112).into())}}Ok(())});let error=materialize(&cache,Arc::new(rt),&source,"links/copy.zip",2,None,cancel).await.unwrap_err();let message=format!("{error:#}");assert!(message.contains(match mode{"change"=>"变化","cancel"=>"取消",_=>"112"})||mode=="cancel"&&message.contains("cancel"),"{mode}: {message}");assert_eq!(sha(&source),original);assert_eq!(file_count(&cache_root),0);}
}

#[tokio::test]
async fn nested1723_unrecognized_alias_damaged_tail_missing_volume_and_bad_depth_do_not_leave_results(){
 let temp=tempfile::tempdir().unwrap();let source=temp.path().join("outer.tar.lzo");std::fs::copy(root().join("gnu.tar.lzo"),&source).unwrap();let cache_root=temp.path().join("cache");let cache=ArchiveEntryOpenCache::new(cache_root.clone());
 let error=materialize(&cache,Arc::new(Runtime::new()),&source,"links/fake.zip",2,None,flag()).await.unwrap_err();assert!(format!("{error:#}").contains("不是可识别"));
 for depth in [3,4]{assert!(materialize(&cache,Arc::new(Runtime::new()),&source,"links/copy.zip",depth,None,flag()).await.is_err());}
 let raw=std::fs::read(root().join("gnu.tar")).unwrap();std::fs::write(&source,wrapped(&raw[..raw.len()/2],100003,true)).unwrap();assert!(materialize(&cache,Arc::new(Runtime::new()),&source,"links/copy.zip",2,None,flag()).await.is_err());
 std::fs::write(temp.path().join("parts.tar.lzo.001"),b"first incomplete").unwrap();std::fs::write(temp.path().join("parts.tar.lzo.003"),b"third").unwrap();assert!(materialize(&cache,Arc::new(Runtime::new()),&temp.path().join("parts.tar.lzo.001"),"links/copy.zip",2,None,flag()).await.is_err());assert_eq!(file_count(&cache_root),0);
}

#[tokio::test]
async fn nested1723_ordinary_nested_route_is_preserved_and_hash_validation_can_cancel(){
 let source=root().join("gnu.tar.lzo");let temp=tempfile::tempdir().unwrap();let cache=ArchiveEntryOpenCache::new(temp.path().join("cache"));let runtime=Arc::new(Runtime::new());let rt=runtime.clone();let path=source.clone();let called=Arc::new(AtomicBool::new(false));let observed=called.clone();
 let receipt=nested_archive::materialize(&cache,runtime,source,"payload/plain.data".into(),None,None,None,2,vec![],None,flag(),move|directory,options:DecompressOptions|async move{observed.store(true,Ordering::SeqCst);let plan=super::lzop::selected::Plan::build(&path,&options,&flag())?.unwrap();plan.extract(rt.as_ref(),None,"ordinary",&path,&directory,&options,&flag())}).await.unwrap();assert!(called.load(Ordering::SeqCst));assert_eq!(archive_browser::detect_format(Path::new(&receipt.cache_path)).unwrap(),super::archive_format::ArchiveFormat::Zip);
 let calls=AtomicUsize::new(0);let error=super::archive_entry_open::validate_nested_archive_identity_controlled(&root().join("gnu.tar"),Path::new(&receipt.cache_path),2,&[],||{anyhow::ensure!(calls.fetch_add(1,Ordering::SeqCst)<2,"cancel during hash");Ok(())}).unwrap_err();assert!(error.to_string().contains("cancel during hash"));
}
