use super::{lzop_writer, lzop, compression_format::{self,CompressionRoute as Route}, compression_verification,archive_browser,archive_preview};
use crate::models::compression::CompressionOptions;
use std::{io::{Read,Write},path::{Path,PathBuf},sync::{Arc,atomic::{AtomicBool,AtomicUsize,Ordering}}};
use sha2::{Digest,Sha256};
fn sha(b:&[u8])->String{format!("{:x}",Sha256::digest(b))}
fn base()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/lzop-create-1662/generated")}
struct Runtime{cancel:Arc<AtomicBool>}
impl super::native_compression::CompressionRuntime for Runtime{
 fn check_cancellation(&self)->anyhow::Result<()>{anyhow::ensure!(!self.cancel.load(Ordering::SeqCst),"操作已取消");Ok(())}
 fn cancellation_flag(&self)->Arc<AtomicBool>{self.cancel.clone()}
 fn copy_buffer_size(&self)->usize{65536}
 fn emit_log(&self,_:&tauri::Window,_:&str,_:&str,_:crate::models::compression::TaskLogSeverity){}
 fn emit_progress(&self,_:&tauri::Window,_:&str,_:f32,_:Option<String>,_:u64,_:u64){}
}
#[tokio::test]
async fn lzop1662_new_outputs_roundtrip_paths_previews_and_official_verifier_inputs(){
 let root=base();std::fs::create_dir_all(&root).unwrap();let image=std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/aes-codecs-1521/原文件/图标.png")).unwrap();
 let mut state=1662u64;let random:Vec<u8>=(0..700001).map(|_|{state^=state<<13;state^=state>>7;state^=state<<17;state as u8}).collect();
 let cases=vec![("empty.txt".to_owned(),vec![]),("notes.txt".into(),b"bounded LZOP content\n".repeat(140000)),("资料.txt".into(),"中文路径和正文均保持原样。\n".repeat(100).into_bytes()),("cover.png".into(),image),("random.bin".into(),random),(format!("{}.txt","a".repeat(190)),b"long name retained".to_vec()),(format!("{}.txt","文".repeat(90)),b"UTF-8 header limit uses output filename".to_vec())];
 let runtime=Runtime{cancel:Arc::new(AtomicBool::new(false))};let mut proof=Vec::new();
 for (i,(name,payload)) in cases.into_iter().enumerate(){
  let source=root.join(&name);std::fs::write(&source,&payload).unwrap();
  let archive=root.join(format!("case-{i}.lzo"));let options=CompressionOptions{format:Some("lzo".into()),..Default::default()};
  compression_format::validate_compression_request(&[source.to_string_lossy().into_owned()],&archive.to_string_lossy(),&options).unwrap();
  lzop_writer::compress(&runtime,None,"1662",&[source.to_string_lossy().into_owned()],&archive.to_string_lossy(),options).unwrap();
  assert!(compression_verification::verify_native(Route::Lzop,&archive,None,||false).unwrap());
  let mut actual=Vec::new();lzop::open(&archive).unwrap().read_to_end(&mut actual).unwrap();assert_eq!(actual,payload);
  let list=archive_browser::browse_archive(&archive,None).await.unwrap();assert_eq!(list.total_files,1);let entry=&list.entries.iter().find(|e|!e.is_dir).unwrap().path;
  if name.len()<=253 {assert_eq!(entry,&name)}else{assert_eq!(entry,&format!("case-{i}"));}
  if name=="资料.txt" {assert!(archive_preview::preview_archive_text(&archive,entry,None).await.unwrap().content.contains("中文路径"));}
  if name=="cover.png" {let p=archive_preview::preview_archive_image(&archive,entry,None).await.unwrap();assert_eq!((p.width,p.height),(32,32));}
  proof.push(serde_json::json!({"archive":archive,"source":source,"sourceBytes":payload.len(),"sourceSha256":sha(&payload),"archiveSha256":sha(&std::fs::read(&archive).unwrap()),"displayedPath":entry}));
 }
 std::fs::write(root.join("manifest.json"),serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
}
#[test]
fn lzop1662_bounded_short_reads_stored_blocks_and_integrity_are_not_level_dependent(){
 struct Short(std::io::Cursor<Vec<u8>>);impl Read for Short{fn read(&mut self,b:&mut[u8])->std::io::Result<usize>{let n=b.len().min(17);self.0.read(&mut b[..n])}}
 let data:Vec<u8>=(0..600000).map(|i|((i*127+i/31)%256)as u8).collect();let mut bytes=Vec::new();let stats=lzop_writer::write_stream(&mut Short(std::io::Cursor::new(data.clone())),&mut bytes,Some("data.bin"),|_|Ok(())).unwrap();assert_eq!(stats.blocks,3);assert_eq!(stats.input_bytes,data.len()as u64);
 let mut restored=Vec::new();lzop::LzopReader::new(std::io::Cursor::new(bytes.clone())).unwrap().read_to_end(&mut restored).unwrap();assert_eq!(restored,data);
 let block=38+bytes[33]as usize;bytes[block+8]^=1;assert!(lzop::LzopReader::new(std::io::Cursor::new(bytes)).unwrap().read_to_end(&mut Vec::new()).is_err());
 let mut output=Vec::new();let stats=lzop_writer::write_stream(&mut std::io::Cursor::new([1u8,2]),&mut output,None,|_|Ok(())).unwrap();assert_eq!(stats.stored_blocks,1);
}
#[test]
fn lzop1662_cancel_and_output_errors_keep_their_actual_causes(){
 let t=tempfile::tempdir().unwrap();let source=t.path().join("source.txt");std::fs::write(&source,b"new output directory").unwrap();let target=t.path().join("new/nested/result.lzo");
 lzop_writer::compress(&Runtime{cancel:Arc::new(AtomicBool::new(false))},None,"1662-new-parent",&[source.to_string_lossy().into_owned()],&target.to_string_lossy(),Default::default()).unwrap();
 let mut plain=Vec::new();lzop::open(&target).unwrap().read_to_end(&mut plain).unwrap();assert_eq!(plain,b"new output directory");
 let count=AtomicUsize::new(0);let mut output=Vec::new();let error=lzop_writer::write_stream(&mut std::io::Cursor::new(vec![1;3*lzop_writer::BLOCK_BYTES]),&mut output,None,|_|{anyhow::ensure!(count.fetch_add(1,Ordering::SeqCst)<7,"操作已取消");Ok(())}).unwrap_err();assert!(error.to_string().contains("取消"));assert!(lzop::LzopReader::new(std::io::Cursor::new(output)).and_then(|mut r|r.read_to_end(&mut Vec::new())).is_err());
 struct Failed;impl Write for Failed{fn write(&mut self,_:&[u8])->std::io::Result<usize>{Err(std::io::Error::from_raw_os_error(112))}fn flush(&mut self)->std::io::Result<()>{Ok(())}}
 let e=lzop_writer::write_stream(&mut std::io::Cursor::new([0u8;10]),&mut Failed,None,|_|Ok(())).unwrap_err();assert_eq!(e.downcast_ref::<std::io::Error>().unwrap().raw_os_error(),Some(112));
}
#[test]
fn lzop1662_rejects_directory_password_and_volume_requests(){
 let t=tempfile::tempdir().unwrap();let a=t.path().join("one.txt");let b=t.path().join("two.txt");std::fs::write(&a,b"one").unwrap();std::fs::write(&b,b"two").unwrap();let a=a.to_string_lossy().into_owned();let b=b.to_string_lossy().into_owned();
 let options=CompressionOptions{format:Some("lzo".into()),..Default::default()};
 assert!(compression_format::validate_compression_request(&[a.clone(),b],"out.lzo",&options).is_ok());
 for sources in [vec![t.path().to_string_lossy().into_owned()]]{assert!(compression_format::validate_compression_request(&sources,"out.lzo",&options).is_err());}
 for changed in [CompressionOptions{password:Some("not supported".into()),..options.clone()},CompressionOptions{split_size:Some(1024),..options.clone()}]{assert!(compression_format::validate_compression_request(&[a.clone()],"out.lzo",&changed).is_err());}
}
#[test]
fn lzop1662_analysis_uses_the_fixed_real_encoder_and_rejects_ineligible_sources(){
 let t=tempfile::tempdir().unwrap();let p=t.path().join("notes.txt");std::fs::write(&p,b"real LZOP sampling\n".repeat(50000)).unwrap();let paths=vec![p.to_string_lossy().into_owned()];
 let a=super::compression_analysis::analyze_compression(&paths,"lzo",1,&AtomicBool::new(false)).unwrap();let b=super::compression_analysis::analyze_compression(&paths,"lzo",9,&AtomicBool::new(false)).unwrap();assert_eq!(a.estimated_size,b.estimated_size);assert_eq!(a.estimated_seconds_high,b.estimated_seconds_high);assert!(a.estimated_ratio<0.2);
 assert!(super::compression_analysis::analyze_compression(&[paths[0].clone(),paths[0].clone()],"lzo",9,&AtomicBool::new(false)).is_ok());
}
