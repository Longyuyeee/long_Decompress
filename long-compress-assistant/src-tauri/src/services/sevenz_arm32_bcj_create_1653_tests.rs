//! New creation methods: independent output readback and persistence, not old decoder matrices.
use super::{native_compression::{self,CompressionRuntime},seven_zip_method::{SevenZipMethod as M,SevenZipFilter as F,sample_size_filtered}};
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
#[tokio::test]
async fn sevenz1653_create_bcj_password_solid_numeric_and_independent_readback(){
 let base=root().join("test-results/sevenz-arm32-bcj-create-1653");std::fs::create_dir_all(&base).unwrap();
 let source=base.join("source");if !source.exists(){
  let fixture=root().join("tests/fixtures/archives/sevenz-bzip2-preview-1644");let manifest:serde_json::Value=serde_json::from_slice(&std::fs::read(fixture.join("manifest.json")).unwrap()).unwrap();let case=manifest["cases"].as_array().unwrap().iter().find(|c|c["name"]=="plain").unwrap();
  independent(&["x".into(),"-y".into(),format!("-o{}",source.display()),fixture.join("plain").join(case["members"][0]["name"].as_str().unwrap()).to_string_lossy().into()]);
  std::fs::create_dir_all(source.join("empty-dir")).unwrap();std::fs::write(source.join("empty.txt"),[]).unwrap();
 }std::fs::write(source.join("crc-zero.bin"),[0x9d,0x0a,0xd9,0x6d]).unwrap();assert!(source.join("notes/说明 [1].txt").exists());
 std::fs::create_dir_all(source.join("code")).unwrap();
 for name in ["sample-arm32.elf","input.bin"] {std::fs::copy(root().join("tests/fixtures/archives/sevenz-arm32-bcj-create-1653").join(name),source.join("code").join(name)).unwrap();}
 let elf=std::fs::read(source.join("code/sample-arm32.elf")).unwrap();assert_eq!(&elf[..7],b"\x7fELF\x01\x01\x01");assert_eq!(u16::from_le_bytes(elf[18..20].try_into().unwrap()),40);
 let expected=hashes(&source);let runtime=Runtime(Arc::new(AtomicBool::new(false)));let mut proof=Vec::new();
 let run_directory=base.join(format!("creation-{}",uuid::Uuid::new_v4()));
 for method in [M::Lzma2,M::Lzma]{for (kind,password,header,solid,split) in [("plain",None,false,false,None),("contents",Some("公开密码1653!"),false,true,None),("headers",Some("公开密码1653!"),true,false,None),("volumes",Some("公开密码1653!"),true,true,Some(8192))]{
  let name=format!("{}-{kind}",method.as_str());let folder=run_directory.join(&name);std::fs::create_dir_all(&folder).unwrap();let output=folder.join("资料.7z");
  native_compression::seven_zip::compress(&runtime,None,&name,&[source.to_string_lossy().into()],&output.to_string_lossy(),CompressionOptions{format:Some("7z".into()),level:match kind {"plain"=>1,"contents"=>3,"headers"=>1,_=>0},seven_zip_method:method,seven_zip_filter:F::BcjArm32,seven_zip_delta_distance:if kind=="contents"{256}else if kind=="headers"{4}else{1},password:password.map(str::to_owned),encrypt_filenames:header,create_solid_archive:solid,split_size:split,..Default::default()}).unwrap();
  let primary=if split.is_some(){output.with_extension("7z.001")}else{output.clone()};
  let pw=format!("-p{}",password.unwrap_or(""));let listing=independent(&["l".into(),"-slt".into(),pw.clone(),primary.to_string_lossy().into()]);
  let label=match method{M::Lzma2=>"LZMA2",M::Lzma=>"LZMA:",M::Bzip2=>"BZip2",M::Deflate=>"Deflate",M::Ppmd=>"PPMD",M::Copy=>"Copy"};assert!(listing.contains("ARM"),"{listing}");assert!(listing.contains(label),"{name}: {listing}");
  if password.is_some(){let bad=Command::new(root().join("src-tauri/resources/archive-engine/7z.exe")).args(["t","-pwrong",&primary.to_string_lossy()]).output().unwrap();assert!(!bad.status.success());}
  let restored=folder.join("restored");independent(&["x".into(),"-y".into(),pw,format!("-o{}",restored.display()),primary.to_string_lossy().into()]);
  assert_eq!(hashes(&restored.join("source")),expected,"{name}");assert!(restored.join("source/empty-dir").is_dir());
  let members:Vec<_>=std::fs::read_dir(&folder).unwrap().map(Result::unwrap).filter(|e|e.file_type().unwrap().is_file()).map(|e|e.path().to_string_lossy().into_owned()).collect();
  if split.is_some(){assert!(members.iter().any(|p|p.ends_with(".002")));}
  let input=members.iter().find(|p|p.ends_with(".002")).map(PathBuf::from).unwrap_or(primary.clone());
  let text=super::archive_preview::preview_archive_text(&input,"source/notes/说明 [1].txt",password).await.unwrap();assert_eq!(text.content.as_bytes(),std::fs::read(source.join("notes/说明 [1].txt")).unwrap());
  let empty=super::archive_preview::preview_archive_text(&input,"source/empty.txt",password).await.unwrap();assert!(empty.content.is_empty());
  let prefix=super::archive_preview::preview_archive_text(&input,"source/notes/large.txt",password).await.unwrap();assert!(prefix.truncated);assert_eq!(prefix.content.as_bytes(),&std::fs::read(source.join("notes/large.txt")).unwrap()[..1024*1024]);
  use base64::Engine;let image=super::image_reader::read(&input,"source/gallery/01-cover.png",password,None,None,&super::image_reader::revision(&input).unwrap(),super::image_reader::Control::default()).await.unwrap();assert_eq!(base64::engine::general_purpose::STANDARD.decode(image.data_url.split(',').nth(1).unwrap()).unwrap(),std::fs::read(source.join("gallery/01-cover.png")).unwrap());
  if password.is_some(){assert!(super::archive_preview::preview_archive_text(&input,"source/notes/说明 [1].txt",Some("wrong")).await.is_err());}
  proof.push(serde_json::json!({"filter":"bcj_arm32","distance":if kind=="contents"{256}else if kind=="headers"{4}else{1},"nativeTextImagePrefix":true,"method":method,"kind":kind,"primary":primary,"members":members,"files":expected.len(),"independentReadback":true,"emptyDirectoryPreserved":true}));
 }}
 std::fs::write(base.join("creation.json"),serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
}
#[test]
fn sevenz1653_defaults_inactive_filter_analysis_and_preflight_cancel_conflict(){
 let default:CompressionOptions=serde_json::from_value(serde_json::json!({"format":"7z","level":6})).unwrap();assert_eq!(default.seven_zip_filter,F::None);assert_eq!(default.seven_zip_delta_distance,1);
 assert!(F::BcjArm32.configuration(M::Copy,4).unwrap().is_none());assert!(F::BcjArm32.configuration(M::Bzip2,4).unwrap().is_none());for n in [0,257]{assert!(F::Delta.configuration(M::Lzma2,n).is_err());assert!(F::BcjArm32.configuration(M::Lzma2,n).unwrap().is_some());}
 let dir=tempfile::tempdir().unwrap();let file=dir.path().join("program.arm32");let data=std::fs::read(root().join("tests/fixtures/archives/sevenz-arm32-bcj-create-1653/input.bin")).unwrap().repeat(8);std::fs::write(&file,&data).unwrap();let active=AtomicBool::new(false);
 let mut sample=data[..65536].to_vec();sample.extend_from_slice(&data[data.len()/2..data.len()/2+65536]);sample.extend_from_slice(&data[data.len()-65536..]);
 for method in [M::Lzma2,M::Lzma]{for distance in [256]{let format=format!("7z-{}-bcj-arm32",method.as_str());let result=super::compression_analysis::analyze_compression(&[file.to_string_lossy().into()],&format,1,&active).unwrap();let actual=sample_size_filtered(&sample,method,1,F::BcjArm32,distance,&active).unwrap();let ratio=(actual as f64/sample.len() as f64).clamp(0.02,1.08);assert!((result.estimated_ratio-ratio).abs()<1e-12);}}
 let output=dir.path().join("invalid.7z");let runtime=Runtime(Arc::new(AtomicBool::new(false)));let bad=CompressionOptions{seven_zip_filter:F::Delta,seven_zip_delta_distance:257,..Default::default()};assert!(native_compression::seven_zip::compress(&runtime,None,"invalid",&[file.to_string_lossy().into()],&output.to_string_lossy(),bad).is_err());assert!(!output.exists());
 let part=dir.path().join("out.7z.001");std::fs::write(&part,b"keep").unwrap();let output=dir.path().join("out.7z");let options=CompressionOptions{seven_zip_filter:F::BcjArm32,seven_zip_delta_distance:4,split_size:Some(1024),..Default::default()};assert!(native_compression::seven_zip::compress(&runtime,None,"conflict",&[file.to_string_lossy().into()],&output.to_string_lossy(),options.clone()).is_err());assert_eq!(std::fs::read(&part).unwrap(),b"keep");runtime.0.store(true,Ordering::Release);assert!(native_compression::seven_zip::compress(&runtime,None,"cancel",&[file.to_string_lossy().into()],&output.to_string_lossy(),options).is_err());assert_eq!(std::fs::read(&file).unwrap(),data);
}
#[tokio::test]async fn sevenz1653_workflow_database_roundtrip(){
 use super::workflows::*;let pool=sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();init_tables(&pool).await.unwrap();let plan=Plan{id:"delta1653".into(),name:"Delta".into(),updated_at:String::new(),steps:vec![Step{id:"create".into(),name:"create".into(),kind:"compress".into(),extensions:String::new(),password_required:true,settings:serde_json::json!({"format":"7z","sevenZipMethod":"lzma2","sevenZipFilter":"bcj_arm32","sevenZipDeltaDistance":256})}]};save_plan(&pool,plan.clone()).await.unwrap();assert_eq!(state(&pool).await.unwrap().plans[0].steps[0].settings["sevenZipDeltaDistance"],256);let mut bad=plan;bad.steps[0].settings["sevenZipDeltaDistance"]=257.into();assert!(save_plan(&pool,bad).await.is_err());
}
