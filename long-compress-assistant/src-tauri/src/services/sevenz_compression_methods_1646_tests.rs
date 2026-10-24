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
fn sevenz1646_create_three_methods_password_solid_numeric_and_independent_readback(){
 let base=root().join("test-results/sevenz-compression-methods-1646");std::fs::create_dir_all(&base).unwrap();
 let source=base.join("source");if !source.exists(){
  let fixture=root().join("tests/fixtures/archives/sevenz-bzip2-preview-1644");let manifest:serde_json::Value=serde_json::from_slice(&std::fs::read(fixture.join("manifest.json")).unwrap()).unwrap();let case=manifest["cases"].as_array().unwrap().iter().find(|c|c["name"]=="plain").unwrap();
  independent(&["x".into(),"-y".into(),format!("-o{}",source.display()),fixture.join("plain").join(case["members"][0]["name"].as_str().unwrap()).to_string_lossy().into()]);
  std::fs::create_dir_all(source.join("empty-dir")).unwrap();std::fs::write(source.join("empty.txt"),[]).unwrap();
 }assert!(source.join("notes/说明 [1].txt").exists());
 let expected=hashes(&source);let runtime=Runtime(Arc::new(AtomicBool::new(false)));let mut proof=Vec::new();
 let run_directory=base.join(format!("creation-{}",uuid::Uuid::new_v4()));
 for method in [M::Lzma2,M::Lzma,M::Bzip2]{for (kind,password,header,solid,split) in [("plain",None,false,false,None),("contents",Some("公开密码1646!"),false,true,None),("headers",Some("公开密码1646!"),true,false,None),("volumes",Some("公开密码1646!"),true,true,Some(8192))]{
  let name=format!("{}-{kind}",method.as_str());let folder=run_directory.join(&name);std::fs::create_dir_all(&folder).unwrap();let output=folder.join("资料.7z");
  native_compression::seven_zip::compress(&runtime,None,&name,&[source.to_string_lossy().into()],&output.to_string_lossy(),CompressionOptions{format:Some("7z".into()),level:if method==M::Bzip2{9}else{1},seven_zip_method:method,password:password.map(str::to_owned),encrypt_filenames:header,create_solid_archive:solid,split_size:split,..Default::default()}).unwrap();
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
fn sevenz1646_default_analysis_and_cancel_collision(){
 let mut legacy=serde_json::to_value(CompressionOptions::default()).unwrap();legacy.as_object_mut().unwrap().remove("seven_zip_method");let default:CompressionOptions=serde_json::from_value(legacy.clone()).unwrap();assert_eq!(default.seven_zip_method,M::Lzma2);
 assert!(serde_json::from_value::<CompressionOptions>({legacy["seven_zip_method"]="unknown".into();legacy}).is_err());
 let body="真实编码 sample 1646\n".as_bytes().repeat(4096);let flag=AtomicBool::new(false);
 for method in [M::Lzma2,M::Lzma,M::Bzip2]{assert!(sample_size(&body,method,1,&flag).unwrap()<body.len());assert!(sample_size(&[],method,1,&flag).unwrap()>32);}
 flag.store(true,Ordering::SeqCst);assert!(sample_size(&body,M::Bzip2,1,&flag).is_err());
 let dir=tempfile::tempdir().unwrap();let source=dir.path().join("input.txt");std::fs::write(&source,&body).unwrap();let out=dir.path().join("out.7z");let part=out.with_extension("7z.001");std::fs::write(&part,b"keep existing").unwrap();
 let opts=CompressionOptions{format:Some("7z".into()),seven_zip_method:M::Bzip2,split_size:Some(32),..Default::default()};
 let runtime=Runtime(Arc::new(AtomicBool::new(false)));assert!(native_compression::seven_zip::compress(&runtime,None,"collision",&[source.to_string_lossy().into()],&out.to_string_lossy(),opts.clone()).is_err());assert_eq!(std::fs::read(&part).unwrap(),b"keep existing");assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(),2);
 runtime.0.store(true,Ordering::SeqCst);assert!(native_compression::seven_zip::compress(&runtime,None,"cancel",&[source.to_string_lossy().into()],&out.to_string_lossy(),opts).is_err());assert_eq!(std::fs::read(&source).unwrap(),body);
}


#[tokio::test]async fn sevenz1646_workflow_database_roundtrip(){
 use super::workflows::*;let pool=sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();init_tables(&pool).await.unwrap();
 for method in ["lzma2","lzma","bzip2"]{let plan=Plan{id:format!("method-{method}"),name:method.into(),updated_at:String::new(),steps:vec![Step{id:"create".into(),name:"create".into(),kind:"compress".into(),extensions:String::new(),password_required:false,settings:serde_json::json!({"format":"7z","sevenZipMethod":method})}]};save_plan(&pool,plan.clone()).await.unwrap();let mut bad=plan;bad.steps[0].settings["sevenZipMethod"]="unknown".into();assert!(save_plan(&pool,bad).await.is_err());}
 let plans=state(&pool).await.unwrap().plans;assert_eq!(plans.len(),3);for p in plans{assert_eq!(p.steps[0].settings["sevenZipMethod"],p.name);}
}
