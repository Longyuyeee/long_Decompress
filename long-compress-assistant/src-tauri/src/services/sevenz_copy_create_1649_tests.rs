//! New creation methods: independent output readback and persistence, not old decoder matrices.
use super::{native_compression::{self,CompressionRuntime},seven_zip_method::{SevenZipMethod as M,sample_size}};
use crate::models::compression::{CompressionOptions,TaskLogSeverity};
use std::{path::{Path,PathBuf},sync::{Arc,atomic::{AtomicBool,Ordering}},process::Command};
struct Runtime(Arc<AtomicBool>);
impl CompressionRuntime for Runtime {
 fn check_cancellation(&self)->anyhow::Result<()>{anyhow::ensure!(!self.0.load(Ordering::Relaxed),"cancelled");Ok(())}
 fn cancellation_flag(&self)->Arc<AtomicBool>{self.0.clone()}
 fn copy_buffer_size(&self)->usize{65536}
 fn emit_log(&self,_:&tauri::Window,_:&str,_:&str,_:TaskLogSeverity){}
 fn emit_progress(&self,_:&tauri::Window,_:&str,_:f32,_:Option<String>,_:u64,_:u64){}
}
fn root()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_owned()}
fn independent(args:&[String])->String{
 let output=Command::new(root().join("src-tauri/resources/archive-engine/7z.exe")).args(args).arg("-sccUTF-8").output().unwrap();
 assert!(output.status.success(),"{} {}",String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr));String::from_utf8(output.stdout).unwrap()
}
fn hashes(dir:&Path)->std::collections::BTreeMap<String,String>{walkdir::WalkDir::new(dir).into_iter().map(Result::unwrap).filter(|e|e.file_type().is_file()).map(|e|(e.path().strip_prefix(dir).unwrap().to_string_lossy().replace('\\',"/"),blake3::hash(&std::fs::read(e.path()).unwrap()).to_hex().to_string())).collect()}
#[test]
fn sevenz1649_create_copy_password_solid_numeric_and_independent_readback(){
 let base=root().join("test-results/sevenz-copy-create-1649");std::fs::create_dir_all(&base).unwrap();
 let source=base.join("source");if !source.exists(){
  let fixture=root().join("tests/fixtures/archives/sevenz-bzip2-preview-1644");let manifest:serde_json::Value=serde_json::from_slice(&std::fs::read(fixture.join("manifest.json")).unwrap()).unwrap();let case=manifest["cases"].as_array().unwrap().iter().find(|c|c["name"]=="plain").unwrap();
  independent(&["x".into(),"-y".into(),format!("-o{}",source.display()),fixture.join("plain").join(case["members"][0]["name"].as_str().unwrap()).to_string_lossy().into()]);
  std::fs::create_dir_all(source.join("empty-dir")).unwrap();std::fs::write(source.join("empty.txt"),[]).unwrap();
 }assert!(source.join("notes/说明 [1].txt").exists());
 let expected=hashes(&source);let runtime=Runtime(Arc::new(AtomicBool::new(false)));let mut proof=Vec::new();
 let run_directory=base.join(format!("creation-{}",uuid::Uuid::new_v4()));
 for method in [M::Copy]{for (kind,password,header,solid,split) in [("plain",None,false,false,None),("contents",Some("公开密码1649!"),false,true,None),("headers",Some("公开密码1649!"),true,false,None),("volumes",Some("公开密码1649!"),true,true,Some(8192))]{
  let name=format!("{}-{kind}",method.as_str());let folder=run_directory.join(&name);std::fs::create_dir_all(&folder).unwrap();let output=folder.join("资料.7z");
  native_compression::seven_zip::compress(&runtime,None,&name,&[source.to_string_lossy().into()],&output.to_string_lossy(),CompressionOptions{format:Some("7z".into()),level:match kind {"plain"=>1,"contents"=>6,"headers"=>9,_=>0},seven_zip_method:method,password:password.map(str::to_owned),encrypt_filenames:header,create_solid_archive:solid,split_size:split,..Default::default()}).unwrap();
  let primary=if split.is_some(){output.with_extension("7z.001")}else{output.clone()};
  let pw=format!("-p{}",password.unwrap_or(""));let listing=independent(&["l".into(),"-slt".into(),pw.clone(),primary.to_string_lossy().into()]);
  let label=match method{M::Lzma2=>"LZMA2",M::Lzma=>"LZMA:",M::Bzip2=>"BZip2",M::Deflate=>"Deflate",M::Ppmd=>"PPMD",M::Copy=>"Copy"};assert!(listing.contains(label),"{name}: {listing}");
  if password.is_some(){let bad=Command::new(root().join("src-tauri/resources/archive-engine/7z.exe")).args(["t","-pwrong",&primary.to_string_lossy()]).output().unwrap();assert!(!bad.status.success());}
  let restored=folder.join("restored");independent(&["x".into(),"-y".into(),pw,format!("-o{}",restored.display()),primary.to_string_lossy().into()]);
  assert_eq!(hashes(&restored.join("source")),expected,"{name}");assert!(restored.join("source/empty-dir").is_dir());
  let members:Vec<_>=std::fs::read_dir(&folder).unwrap().map(Result::unwrap).filter(|e|e.file_type().unwrap().is_file()).map(|e|e.path().to_string_lossy().into_owned()).collect();
  if split.is_some(){assert!(members.iter().any(|p|p.ends_with(".002")));}
  proof.push(serde_json::json!({"method":method,"kind":kind,"primary":primary,"members":members,"files":expected.len(),"independentReadback":true,"emptyDirectoryPreserved":true}));
 }}
 std::fs::write(base.join("creation.json"),serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
}
#[test]
fn sevenz1649_default_analysis_and_cancel_collision(){
 let mut legacy=serde_json::to_value(CompressionOptions::default()).unwrap();legacy.as_object_mut().unwrap().remove("seven_zip_method");let default:CompressionOptions=serde_json::from_value(legacy.clone()).unwrap();assert_eq!(default.seven_zip_method,M::Lzma2);
 assert!(serde_json::from_value::<CompressionOptions>({legacy["seven_zip_method"]="unknown".into();legacy}).is_err());
 let body="真实编码 sample 1649\n".as_bytes().repeat(4096);let flag=AtomicBool::new(false);
 for method in [M::Copy]{assert!(sample_size(&body,method,1,&flag).unwrap()>body.len());assert!(sample_size(&[],method,1,&flag).unwrap()>32);}
 flag.store(true,Ordering::SeqCst);assert!(sample_size(&body,M::Copy,1,&flag).is_err());
 let dir=tempfile::tempdir().unwrap();let source=dir.path().join("input.txt");std::fs::write(&source,&body).unwrap();let out=dir.path().join("out.7z");let part=out.with_extension("7z.001");std::fs::write(&part,b"keep existing").unwrap();
 let measured:Vec<u8>=(0..32768).map(|i|((i*31+i/8)%95+32) as u8).collect();let sampled=dir.path().join("sample.txt");std::fs::write(&sampled,&measured).unwrap();let active=AtomicBool::new(false);let result=super::compression_analysis::analyze_compression(&[sampled.to_string_lossy().into()],"7z-copy",6,&active).unwrap();let expected=(sample_size(&measured,M::Copy,6,&active).unwrap() as f64 / measured.len() as f64).max(1.0);assert!((result.estimated_ratio-expected).abs()<1e-12);std::fs::remove_file(sampled).unwrap();
 let opts=CompressionOptions{format:Some("7z".into()),seven_zip_method:M::Copy,split_size:Some(32),..Default::default()};
 let runtime=Runtime(Arc::new(AtomicBool::new(false)));assert!(native_compression::seven_zip::compress(&runtime,None,"collision",&[source.to_string_lossy().into()],&out.to_string_lossy(),opts.clone()).is_err());assert_eq!(std::fs::read(&part).unwrap(),b"keep existing");assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(),2);
 runtime.0.store(true,Ordering::SeqCst);assert!(native_compression::seven_zip::compress(&runtime,None,"cancel",&[source.to_string_lossy().into()],&out.to_string_lossy(),opts).is_err());assert_eq!(std::fs::read(&source).unwrap(),body);
}


#[tokio::test]async fn sevenz1649_workflow_database_roundtrip(){
 use super::workflows::*;let pool=sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();init_tables(&pool).await.unwrap();
 for method in ["copy"]{let plan=Plan{id:format!("method-{method}"),name:method.into(),updated_at:String::new(),steps:vec![Step{id:"create".into(),name:"create".into(),kind:"compress".into(),extensions:String::new(),password_required:false,settings:serde_json::json!({"format":"7z","sevenZipMethod":method})}]};save_plan(&pool,plan.clone()).await.unwrap();let mut bad=plan;bad.steps[0].settings["sevenZipMethod"]="unknown".into();assert!(save_plan(&pool,bad).await.is_err());}
 let plans=state(&pool).await.unwrap().plans;assert_eq!(plans.len(),1);for p in plans{assert_eq!(p.steps[0].settings["sevenZipMethod"],p.name);}
}

#[test]
fn sevenz1649_low_gain_advice_uses_content_and_sample_threshold() {
 let dir=tempfile::tempdir().unwrap();let random=dir.path().join("unknown.bin");let photo=dir.path().join("photo.jpg");let tiny=dir.path().join("tiny.bin");
 let data:Vec<u8>=(0u64..65539).map(|i|blake3::hash(&i.to_le_bytes()).as_bytes()[0]).collect();
 std::fs::write(&random,&data).unwrap();std::fs::write(&photo,b"highly compressible text ".repeat(4096)).unwrap();std::fs::write(&tiny,&data[..100]).unwrap();
 let active=AtomicBool::new(false);
 let analyze=|p:&Path,f:&str|super::compression_analysis::analyze_compression(&[p.to_string_lossy().into()],f,6,&active).unwrap();
 assert_eq!(analyze(&random,"7z").recommended_seven_zip_method.as_deref(),Some("copy"));
 assert!(analyze(&photo,"7z").recommended_seven_zip_method.is_none());
 assert!(analyze(&tiny,"7z").recommended_seven_zip_method.is_none());
 for p in [&random,&photo]{let result=analyze(p,"7z-copy");assert!(result.estimated_ratio>=1.0);assert!(result.estimated_size>=result.total_size);assert!(result.recommended_seven_zip_method.is_none());}
}
#[test]
fn sevenz1649_copy_cancellation_during_source_read_is_returned() {
 let dir=tempfile::tempdir().unwrap();let input=dir.path().join("large.bin");let output=dir.path().join("cancel.7z");
 std::fs::File::create(&input).unwrap().set_len(128*1024*1024).unwrap();
 let runtime=Runtime(Arc::new(AtomicBool::new(false)));let completed=AtomicBool::new(false);
 std::thread::scope(|scope|{
  scope.spawn(||{while !completed.load(Ordering::Acquire){if std::fs::metadata(&output).is_ok_and(|m|m.len()>=1024*1024){runtime.0.store(true,Ordering::Release);return;}std::thread::yield_now();}});
  let result=native_compression::seven_zip::compress(&runtime,None,"mid-copy",&[input.to_string_lossy().into()],&output.to_string_lossy(),CompressionOptions{format:Some("7z".into()),seven_zip_method:M::Copy,..Default::default()});
  completed.store(true,Ordering::Release);assert!(result.is_err());assert!(runtime.0.load(Ordering::Acquire));
 });
 assert_eq!(std::fs::metadata(input).unwrap().len(),128*1024*1024);assert!(std::fs::metadata(output).unwrap().len()<128*1024*1024);
}

#[tokio::test]
async fn sevenz1649_copy_preview_complete_prefix_password_volumes_and_crc_failure() {
 use super::{archive_preview as preview,image_reader::{self,Control}};use base64::Engine;
 let base=root().join("test-results/sevenz-copy-create-1649");
 let cases:serde_json::Value=serde_json::from_slice(&std::fs::read(base.join("creation.json")).unwrap()).unwrap();
 for c in cases.as_array().unwrap() {
  let primary=PathBuf::from(c["primary"].as_str().unwrap());let volume=c["members"].as_array().unwrap().iter().find_map(|p|p.as_str().filter(|s|s.ends_with(".002"))).map(PathBuf::from).unwrap_or(primary.clone());
  let key=if c["kind"]=="plain"{None}else{Some("公开密码1649!")};
  let text=preview::preview_archive_text(&volume,"source/notes/说明 [1].txt",key).await.unwrap();assert!(!text.truncated);assert_eq!(text.content.as_bytes(),std::fs::read(base.join("source/notes/说明 [1].txt")).unwrap());
  let empty=preview::preview_archive_text(&volume,"source/empty.txt",key).await.unwrap();assert!(empty.content.is_empty());assert!(!empty.truncated);
  let prefix=preview::preview_archive_text(&volume,"source/notes/large.txt",key).await.unwrap();assert!(prefix.truncated);assert_eq!(prefix.content.as_bytes(),&std::fs::read(base.join("source/notes/large.txt")).unwrap()[..1024*1024]);
  let image=image_reader::read(&volume,"source/gallery/01-cover.png",key,None,None,&image_reader::revision(&volume).unwrap(),Control::default()).await.unwrap();assert_eq!(base64::engine::general_purpose::STANDARD.decode(image.data_url.split(',').nth(1).unwrap()).unwrap(),std::fs::read(base.join("source/gallery/01-cover.png")).unwrap());
  if key.is_some(){assert!(preview::preview_archive_text(&volume,"source/notes/说明 [1].txt",Some("wrong")).await.is_err());}
  if c["kind"]=="plain" {
   let mut input=std::fs::File::open(&primary).unwrap();let len=input.metadata().unwrap().len();let a=sevenz_rust::Archive::read(&mut input,len,&[]).unwrap();let index=a.files.iter().position(|e|e.name=="source/notes/说明 [1].txt").unwrap();let folder=a.stream_map.file_folder_index[index].unwrap();let pack=a.stream_map.folder_first_pack_stream_index[folder];let offset=32+a.pack_pos+a.stream_map.pack_stream_offsets[pack];
   let mut bytes=std::fs::read(primary).unwrap();bytes[offset as usize]^=1;let temp=tempfile::tempdir().unwrap();let broken=temp.path().join("bad-crc.7z");std::fs::write(&broken,bytes).unwrap();assert!(preview::preview_archive_text(&broken,"source/notes/说明 [1].txt",None).await.is_err());
  }
 }
}
