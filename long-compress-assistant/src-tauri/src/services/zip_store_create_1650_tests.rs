//! Store creation: inspect actual methods and compare independent complete readback.
use super::{native_compression::{self,CompressionRuntime},zip_method::{self,ZipMethod as M},zip_password::ZipEncryption,standard_zip_writer::ZipSplitMode};
use crate::models::compression::{CompressionOptions,TaskLogSeverity};
use std::{path::{Path,PathBuf},sync::{Arc,atomic::{AtomicBool,Ordering}},io::{Read,Write},collections::BTreeMap};
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
 let output=crate::utils::process::command(root().join("src-tauri/resources/archive-engine/7z.exe")).args(args).arg("-sccUTF-8").output().unwrap();
 assert!(output.status.success(),"{} {}",String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr));String::from_utf8(output.stdout).unwrap()
}
fn hashes(dir:&Path)->BTreeMap<String,String>{walkdir::WalkDir::new(dir).into_iter().map(Result::unwrap).filter(|e|e.file_type().is_file()).map(|e|(e.path().strip_prefix(dir).unwrap().to_string_lossy().replace('\\',"/"),blake3::hash(&std::fs::read(e.path()).unwrap()).to_hex().to_string())).collect()}
fn seed(base:&Path)->PathBuf{
 let source=base.join("source");
 if !source.exists(){let fixture=root().join("tests/fixtures/archives/sevenz-bzip2-preview-1644");let manifest:serde_json::Value=serde_json::from_slice(&std::fs::read(fixture.join("manifest.json")).unwrap()).unwrap();let case=manifest["cases"].as_array().unwrap().iter().find(|c|c["name"]=="plain").unwrap();independent(&["x".into(),"-y".into(),format!("-o{}",source.display()),fixture.join("plain").join(case["members"][0]["name"].as_str().unwrap()).to_string_lossy().into()]);}
 std::fs::create_dir_all(source.join("empty-dir")).unwrap();std::fs::write(source.join("empty.txt"),[]).unwrap();std::fs::write(source.join("crc-zero.bin"),[0x9d,0x0a,0xd9,0x6d]).unwrap();source
}
#[tokio::test]
async fn zip1650_store_plain_aes_zipcrypto_numeric_standard_and_native_preview(){
 use super::{archive_preview as preview,image_reader::{self,Control}};use base64::Engine;
 let base=root().join("test-results/zip-store-create-1650");std::fs::create_dir_all(&base).unwrap();let source=seed(&base);let expected=hashes(&source);let runtime=Runtime(Arc::new(AtomicBool::new(false)));let run=base.join(format!("creation-{}",uuid::Uuid::new_v4()));let mut proof=Vec::new();
 for password in [None,Some(ZipEncryption::Aes256),Some(ZipEncryption::Zipcrypto)]{for split in [None,Some(ZipSplitMode::Numeric),Some(ZipSplitMode::Standard)]{
  let folder=run.join(format!("{password:?}-{split:?}"));std::fs::create_dir_all(&folder).unwrap();let output=folder.join("资料.zip");let secret=password.map(|_|"StoreKey1650!");
  native_compression::zip::compress(&runtime,None,"store1650",&[source.to_string_lossy().into()],&output.to_string_lossy(),CompressionOptions{format:Some("zip".into()),zip_method:M::Store,level:9,password:secret.map(str::to_owned),zip_encryption:password.unwrap_or_default(),split_size:split.map(|_|65536),zip_split_mode:split.unwrap_or_default(),..Default::default()}).unwrap();
  let parts=if split.is_some(){super::archive_volumes::complete_volume_paths(&output).unwrap()}else{vec![output.clone()]};if split.is_some(){assert!(parts.len()>2)}let primary=super::split_archive_detector::SplitArchiveDetector::complete_source(&parts[parts.len()/2]).unwrap();
  let reader=super::numeric_volume_reader::ZipInputReader::open(&primary).unwrap();let mut archive=zip_aes::ZipArchive::new(reader).unwrap();let mut restored=BTreeMap::new();
  for i in 0..archive.len(){let mut f=if let Some(secret)=secret{archive.by_index_decrypt(i,secret.as_bytes()).unwrap()}else{archive.by_index(i).unwrap()};assert_eq!(f.compression(),zip_aes::CompressionMethod::Stored);if f.is_dir(){continue}let mut data=Vec::new();f.read_to_end(&mut data).unwrap();if f.name().ends_with("crc-zero.bin"){assert_eq!(crc32fast::hash(&data),0)}restored.insert(f.name().to_owned(),blake3::hash(&data).to_hex().to_string());}
  let expected:BTreeMap<_,_>=expected.iter().map(|(p,h)|(format!("source/{p}"),h.clone())).collect();assert_eq!(restored,expected);
  let oracle=if split==Some(ZipSplitMode::Numeric){let joined=folder.join("oracle.zip");let mut f=std::fs::File::create(&joined).unwrap();for part in &parts{f.write_all(&std::fs::read(part).unwrap()).unwrap()}joined}else{output.clone()};let pw=format!("-p{}",secret.unwrap_or(""));
  let listing=independent(&["l".into(),"-slt".into(),pw.clone(),oracle.to_string_lossy().into()]);assert!(listing.contains("Store"));let extracted=folder.join("independent");independent(&["x".into(),"-y".into(),pw,format!("-o{}",extracted.display()),oracle.to_string_lossy().into()]);assert_eq!(hashes(&extracted),expected);assert!(extracted.join("source/empty-dir").is_dir());
  let input=&parts[parts.len()/2];let text=preview::preview_archive_text(input,"source/notes/说明 [1].txt",secret).await.unwrap();assert_eq!(text.content.as_bytes(),std::fs::read(source.join("notes/说明 [1].txt")).unwrap());assert!(!text.truncated);
  let empty=preview::preview_archive_text(input,"source/empty.txt",secret).await.unwrap();assert!(empty.content.is_empty());let prefix=preview::preview_archive_text(input,"source/notes/large.txt",secret).await.unwrap();assert!(prefix.truncated);assert_eq!(prefix.content.as_bytes(),&std::fs::read(source.join("notes/large.txt")).unwrap()[..1024*1024]);
  let image=image_reader::read(input,"source/gallery/01-cover.png",secret,None,None,&image_reader::revision(input).unwrap(),Control::default()).await.unwrap();assert_eq!(base64::engine::general_purpose::STANDARD.decode(image.data_url.split(',').nth(1).unwrap()).unwrap(),std::fs::read(source.join("gallery/01-cover.png")).unwrap());
  if secret.is_some(){assert!(preview::preview_archive_text(input,"source/notes/说明 [1].txt",Some("wrong")).await.is_err());}
  if password.is_none()&&split.is_none(){let offset=archive.by_name("source/notes/说明 [1].txt").unwrap().data_start().unwrap() as usize;let mut bytes=std::fs::read(&output).unwrap();bytes[offset]^=1;let bad=folder.join("bad-crc.zip");std::fs::write(&bad,bytes).unwrap();assert!(preview::preview_archive_text(&bad,"source/notes/说明 [1].txt",None).await.is_err());}
  proof.push(serde_json::json!({"encryption":password,"split":split,"levelRetained":9,"primary":primary,"members":parts,"files":expected.len(),"independentReadback":true,"method":"Stored (0)","emptyEntriesAndCrcZero":true,"nativeTextImageAndPrefix":true}));
 }}std::fs::write(base.join("creation.json"),serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
}
#[test]
fn zip1650_legacy_methods_and_zipx_keep_meaning(){
 let legacy:CompressionOptions=serde_json::from_value(serde_json::json!({"level":0})).unwrap();assert_eq!(legacy.zip_method,M::Deflate);
 for (method,expected) in [(M::Store,zip_aes::CompressionMethod::Stored),(M::Deflate,zip_aes::CompressionMethod::Stored),(M::Zstd,zip_aes::CompressionMethod::Zstd),(M::Bzip2,zip_aes::CompressionMethod::Bzip2),(M::Ppmd,zip_aes::CompressionMethod::Ppmd),(M::Lzma,zip_aes::CompressionMethod::Lzma)]{
  let mut z=zip_aes::ZipWriter::new(std::io::Cursor::new(Vec::new()));z.start_file("sample",zip_method::file_options(false,method,0)).unwrap();z.write_all(b"legacy method continuity").unwrap();let mut z=zip_aes::ZipArchive::new(z.finish().unwrap()).unwrap();assert_eq!(z.by_index(0).unwrap().compression(),expected);
 }
 for (zipx,method,expected) in [(false,M::Deflate,zip_aes::CompressionMethod::Deflated),(true,M::Store,zip_aes::CompressionMethod::Xz)]{
  let mut z=zip_aes::ZipWriter::new(std::io::Cursor::new(Vec::new()));z.start_file("sample",zip_method::file_options(zipx,method,6)).unwrap();z.write_all(b"default or explicit ZIPX").unwrap();let mut z=zip_aes::ZipArchive::new(z.finish().unwrap()).unwrap();assert_eq!(z.by_index(0).unwrap().compression(),expected);
 }
}
#[test]
fn zip1650_analysis_real_store_content_advice_and_cancel(){
 let dir=tempfile::tempdir().unwrap();let random=dir.path().join("unknown.bin");let photo=dir.path().join("photo.jpg");let tiny=dir.path().join("tiny.bin");let data:Vec<u8>=(0u64..65539).map(|i|blake3::hash(&i.to_le_bytes()).as_bytes()[0]).collect();std::fs::write(&random,&data).unwrap();std::fs::write(&photo,b"compressible text ".repeat(4096)).unwrap();std::fs::write(&tiny,&data[..100]).unwrap();let active=AtomicBool::new(false);
 let analyze=|p:&Path,f:&str,l|super::compression_analysis::analyze_compression(&[p.to_string_lossy().into()],f,l,&active).unwrap();
 for f in ["zip","zip-zstd","zip-bzip2","zip-lzma","zip-ppmd"]{assert_eq!(analyze(&random,f,6).recommended_zip_method.as_deref(),Some("store"));assert!(analyze(&photo,f,6).recommended_zip_method.is_none());assert!(analyze(&tiny,f,6).recommended_zip_method.is_none());}
 for p in [&random,&photo]{for f in ["zip-store","zip"]{let r=analyze(p,f,if f=="zip"{0}else{9});assert!(r.estimated_ratio>=1.0);assert!(r.estimated_size>=r.total_size);assert!(r.recommended_zip_method.is_none());}}
 active.store(true,Ordering::SeqCst);assert!(super::compression_analysis::analyze_compression(&[random.to_string_lossy().into()],"zip-store",9,&active).is_err());
}
#[test]
fn zip1650_store_cancel_and_split_conflict_keep_existing_files(){
 let dir=tempfile::tempdir().unwrap();let input=dir.path().join("input.bin");let data=vec![42;1024*1024];std::fs::write(&input,&data).unwrap();let output=dir.path().join("out.zip");let existing=dir.path().join("out.zip.001");std::fs::write(&existing,b"keep").unwrap();let runtime=Runtime(Arc::new(AtomicBool::new(false)));let opts=CompressionOptions{zip_method:M::Store,split_size:Some(65536),..Default::default()};
 assert!(native_compression::zip::compress(&runtime,None,"conflict",&[input.to_string_lossy().into()],&output.to_string_lossy(),opts.clone()).is_err());assert_eq!(std::fs::read(&existing).unwrap(),b"keep");assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(),2);
 runtime.0.store(true,Ordering::SeqCst);assert!(native_compression::zip::compress(&runtime,None,"cancel",&[input.to_string_lossy().into()],&output.to_string_lossy(),opts).is_err());assert_eq!(std::fs::read(&input).unwrap(),data);
}
#[tokio::test]
async fn zip1650_workflow_database_roundtrip(){
 use super::workflows::*;let pool=sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();init_tables(&pool).await.unwrap();let plan=Plan{id:"store1650".into(),name:"Store".into(),updated_at:String::new(),steps:vec![Step{id:"create".into(),name:"create".into(),kind:"compress".into(),extensions:String::new(),password_required:true,settings:serde_json::json!({"format":"zip","zipMethod":"store","level":9,"splitArchive":true,"splitSize":"2","zipSplitMode":"standard","zipEncryption":"zipcrypto"})}]};save_plan(&pool,plan).await.unwrap();let plans=state(&pool).await.unwrap().plans;assert_eq!(plans[0].steps[0].settings["zipMethod"],"store");assert_eq!(plans[0].steps[0].settings["level"],9);
}
