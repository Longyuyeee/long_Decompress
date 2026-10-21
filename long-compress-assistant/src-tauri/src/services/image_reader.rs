//! A reading session reuses bounded previews and binds every page to one source revision.
use anyhow::{ensure,Context,Result};
use serde::{Serialize,Deserialize};
use std::{path::Path,sync::{Arc,atomic::{AtomicBool,Ordering}}};
#[derive(Clone,Serialize)]
#[serde(rename_all="camelCase")]
pub struct Progress {pub phase:&'static str,pub processed_bytes:u64,pub total_bytes:u64}
#[derive(Clone)]
pub struct Control {pub flag:Arc<AtomicBool>,pub observer:Arc<dyn Fn(Progress)+Send+Sync>}
impl Default for Control {fn default()->Self{Self{flag:Arc::new(AtomicBool::new(false)),observer:Arc::new(|_|{})}}}
impl Control {
 pub fn check(&self)->Result<()>{ensure!(!self.flag.load(Ordering::Acquire),"IMAGE_READER_CANCELLED|已停止读取图片");Ok(())}
 pub fn update(&self,phase:&'static str,processed:u64,total:u64){(self.observer)(Progress{phase,processed_bytes:processed,total_bytes:total})}
}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Selection {pub revision:String,pub paths:Vec<String>,pub keep_structure:bool,pub conflict_policy:String}
pub fn source_path(path:&Path)->Result<String>{
 let source=super::split_archive_detector::SplitArchiveDetector::complete_source(path)?.canonicalize()?.to_string_lossy().into_owned();
 Ok(if let Some(tail)=source.strip_prefix(r"\\?\UNC\"){format!(r"\\{tail}")}else{source.strip_prefix(r"\\?\").unwrap_or(&source).to_string()})
}
impl Selection {
 pub fn validate(&self)->Result<()>{
  ensure!(self.revision.len()==64&&self.revision.bytes().all(|b|b.is_ascii_hexdigit()),"阅读来源版本无效，请重新读取");
  ensure!(!self.paths.is_empty()&&self.paths.len()<=2000&&serde_json::to_string(&self.paths)?.len()<=240000,"阅读所选范围无效，请返回阅读器重新选择");
  let unique:std::collections::HashSet<_>=self.paths.iter().collect();
  ensure!(unique.len()==self.paths.len()&&self.paths.iter().all(|p|!p.is_empty()&&p.len()<=131072),"阅读所选路径无效，请重新选择");
  ensure!(matches!(self.conflict_policy.as_str(),"rename"|"skip"),"请重新确认所选图片的同名处理方式");Ok(())
 }
}
pub fn apply_selection(path:&Path,options:&mut crate::models::compression::DecompressOptions)->Result<()>{
 let Some(selection)=options.reader_selection.clone()else{return Ok(())};selection.validate()?;
 check_source(path,&selection.revision).map_err(|_|anyhow::anyhow!("阅读来源已变化、离线或缺卷，请返回浏览中心刷新并重新选择图片；原选择不会扩大为整包提取"))?;
 options.selected_entries=selection.paths;options.preserve_paths=selection.keep_structure;options.conflict_policy=selection.conflict_policy;
 ensure!(!options.delete_after,"所选图片取回保留原包，请关闭回收来源选项");
 Ok(())
}
pub fn revision(path:&Path)->Result<String>{
 let source=super::split_archive_detector::SplitArchiveDetector::complete_source(path)?;
 let identity=super::archive_library::source_revision(&source)?.0;
 Ok(blake3::hash(format!("{}:{identity}",source.canonicalize()?.to_string_lossy()).as_bytes()).to_hex().to_string())
}
pub fn check_source(path:&Path,expected:&str)->Result<()>{ensure!(!expected.is_empty()&&revision(path).context("IMAGE_READER_CHANGED|来源无法确认，请返回刷新来源后重新阅读")?==expected,"IMAGE_READER_CHANGED|来源已变化，请返回刷新来源后重新阅读");Ok(())}
pub async fn read(path:&Path,entry:&str,password:Option<&str>,encoding:Option<super::zip_password::ZipPasswordEncoding>,snapshot:Option<crate::models::compression::ZpaqSnapshot>,expected:&str,control:Control)->Result<super::archive_preview::ArchiveImagePreview>{
 control.check()?;check_source(path,expected)?;
 let result=super::archive_preview::preview_archive_image_controlled_at(path,entry,password,encoding,snapshot,control.clone()).await?;
 control.check()?;check_source(path,expected)?;Ok(result)
}

#[cfg(test)]mod tests{
 use super::*;use std::{fs,io::Write};
 fn bmp()->Vec<u8>{let mut b=vec![37;256*1024];b[..2].copy_from_slice(b"BM");b[18..22].copy_from_slice(&64i32.to_le_bytes());b[22..26].copy_from_slice(&64i32.to_le_bytes());b}
 fn zip(path:&Path,password:Option<&str>){let mut z=zip_aes::ZipWriter::new(fs::File::create(path).unwrap());let option=zip_aes::write::SimpleFileOptions::default().compression_method(zip_aes::CompressionMethod::Deflated);z.start_file("page2.bmp",if let Some(p)=password{option.with_aes_encryption(zip_aes::AesMode::Aes256,p)}else{option}).unwrap();z.write_all(&bmp()).unwrap();z.finish().unwrap();}
 #[tokio::test]async fn reader1557_zip_has_actual_byte_progress_and_original_untouched(){let d=tempfile::tempdir().unwrap();let p=d.path().join("pages.zip");zip(&p,None);let before=fs::read(&p).unwrap();let count=Arc::new(std::sync::atomic::AtomicU64::new(0));let observed=count.clone();let control=Control{observer:Arc::new(move|event|{if event.phase=="reading"{observed.fetch_max(event.processed_bytes,Ordering::SeqCst);}}),..Default::default()};let result=read(&p,"page2.bmp",None,None,None,&revision(&p).unwrap(),control).await.unwrap();assert_eq!((result.width,result.height,result.byte_size),(64,64,256*1024));assert_eq!(count.load(Ordering::SeqCst),256*1024);assert_eq!(fs::read(p).unwrap(),before);}
 #[tokio::test]async fn reader1557_zip_cancellation_stops_after_a_bounded_chunk(){let d=tempfile::tempdir().unwrap();let p=d.path().join("pages.zip");zip(&p,None);let flag=Arc::new(AtomicBool::new(false));let signal=flag.clone();let control=Control{flag,observer:Arc::new(move|event|{if event.phase=="reading"&&event.processed_bytes>=65536{signal.store(true,Ordering::Release)}})};let error=read(&p,"page2.bmp",None,None,None,&revision(&p).unwrap(),control).await.unwrap_err();assert!(format!("{error:#}").contains("IMAGE_READER_CANCELLED"));}
 #[tokio::test]async fn reader1557_tar_cancellation_during_locating_is_not_a_finished_page(){let d=tempfile::tempdir().unwrap();let p=d.path().join("pages.tar");let mut tar=tar::Builder::new(fs::File::create(&p).unwrap());for(name,bytes)in[("before.bin",vec![31;2*1024*1024]),("page2.bmp",bmp())]{let mut h=tar::Header::new_gnu();h.set_size(bytes.len()as u64);h.set_mode(0o644);h.set_cksum();tar.append_data(&mut h,name,bytes.as_slice()).unwrap();}tar.finish().unwrap();drop(tar);let flag=Arc::new(AtomicBool::new(false));let signal=flag.clone();let control=Control{flag,observer:Arc::new(move|event|{if event.phase=="locating"&&event.processed_bytes>=65536{signal.store(true,Ordering::Release)}})};let error=read(&p,"page2.bmp",None,None,None,&revision(&p).unwrap(),control).await.unwrap_err();assert!(format!("{error:#}").contains("IMAGE_READER_CANCELLED"));}
 #[tokio::test]async fn reader1557_source_changes_before_or_during_page_are_explicit(){let d=tempfile::tempdir().unwrap();let p=d.path().join("pages.zip");zip(&p,None);let expected=revision(&p).unwrap();let changed=p.clone();let control=Control{observer:Arc::new(move|event|{if event.phase=="decoding"{fs::write(&changed,b"changed source").unwrap()}}),..Default::default()};assert!(format!("{:#}",read(&p,"page2.bmp",None,None,None,&expected,control).await.unwrap_err()).contains("IMAGE_READER_CHANGED"));assert!(check_source(&p,&expected).is_err());}
 #[tokio::test]async fn reader1557_password_numeric_source_keeps_exact_version_and_selected_page(){let d=tempfile::tempdir().unwrap();let p=d.path().join("pages.zip");zip(&p,Some("reader-password"));let bytes=fs::read(&p).unwrap();fs::remove_file(p).unwrap();let chunks:Vec<_>=bytes.chunks(120).collect();for(index,chunk)in chunks.iter().enumerate(){fs::write(d.path().join(format!("pages.zip.{:03}",index+1)),chunk).unwrap();}let member=d.path().join("pages.zip.002");let version=revision(&member).unwrap();assert!(read(&member,"page2.bmp",None,None,None,&version,Control::default()).await.is_err());assert_eq!(read(&member,"page2.bmp",Some("reader-password"),None,None,&version,Control::default()).await.unwrap().byte_size,256*1024);fs::remove_file(d.path().join("pages.zip.001")).unwrap();assert!(format!("{:#}",check_source(&member,&version).unwrap_err()).contains("IMAGE_READER_CHANGED"));}
 #[test]fn reader1557_missing_and_replaced_source_cannot_reuse_a_cache_revision(){let d=tempfile::tempdir().unwrap();let p=d.path().join("pages.zip");zip(&p,None);let version=revision(&p).unwrap();assert!(check_source(&p,&version).is_ok());fs::remove_file(p.clone()).unwrap();assert!(format!("{:#}",check_source(&p,&version).unwrap_err()).contains("IMAGE_READER_CHANGED"));}
 #[tokio::test]async fn reader1557_tar_page_progress_does_not_return_to_locating_after_reading(){let d=tempfile::tempdir().unwrap();let p=d.path().join("pages.tar");let mut tar=tar::Builder::new(fs::File::create(&p).unwrap());let b=bmp();let mut h=tar::Header::new_gnu();h.set_size(b.len()as u64);h.set_mode(0o644);h.set_cksum();tar.append_data(&mut h,"page2.bmp",b.as_slice()).unwrap();tar.finish().unwrap();drop(tar);let phases=Arc::new(std::sync::Mutex::new(Vec::new()));let events=phases.clone();let control=Control{observer:Arc::new(move|event|events.lock().unwrap().push(event.phase)),..Default::default()};read(&p,"page2.bmp",None,None,None,&revision(&p).unwrap(),control).await.unwrap();let phases=phases.lock().unwrap();let reading=phases.iter().position(|p|*p=="reading").unwrap();assert!(!phases[reading..].contains(&"locating"));}

}

#[cfg(test)]
mod reader1558_tests {
 use super::*;
 use crate::models::compression::DecompressOptions;
 fn source(root:&Path)->std::path::PathBuf{let path=root.join("pages.zip");std::fs::write(&path,b"fixture-metadata-for-selection").unwrap();path}
 fn selection(path:&Path)->Selection{Selection{revision:revision(path).unwrap(),paths:vec!["pages/2.png".into(),"other/10.png".into()],keep_structure:false,conflict_policy:"skip".into()}}
 #[test]fn reader1558_selection_keeps_exact_paths_and_original_options(){let root=tempfile::tempdir().unwrap();let path=source(root.path());let expected=selection(&path);let mut options=DecompressOptions{reader_selection:Some(expected.clone()),..Default::default()};apply_selection(&path,&mut options).unwrap();assert_eq!(options.selected_entries,expected.paths);assert!(!options.preserve_paths);assert_eq!(options.conflict_policy,"skip");assert!(!options.delete_after);}
 #[test]fn reader1558_changed_source_cannot_reuse_old_selection(){let root=tempfile::tempdir().unwrap();let path=source(root.path());let mut options=DecompressOptions{reader_selection:Some(selection(&path)),..Default::default()};std::fs::write(&path,b"replacement-source-with-new-size").unwrap();assert!(apply_selection(&path,&mut options).unwrap_err().to_string().contains("重新选择图片"));assert!(options.selected_entries.is_empty());}
 #[test]fn reader1558_empty_or_duplicate_selection_never_extracts_whole_archive(){let root=tempfile::tempdir().unwrap();let path=source(root.path());for paths in [vec![],vec!["same.png".into(),"same.png".into()]]{let mut selected=selection(&path);selected.paths=paths;let mut options=DecompressOptions{reader_selection:Some(selected),..Default::default()};assert!(apply_selection(&path,&mut options).is_err());}}
 #[test]fn reader1558_any_numeric_member_has_one_reading_source(){let root=tempfile::tempdir().unwrap();for suffix in ["001","002","003"]{std::fs::write(root.path().join(format!("pages.zip.{suffix}")),b"volume").unwrap();}let first=root.path().join("pages.zip.001");let last=root.path().join("pages.zip.003");assert_eq!(source_path(&first).unwrap(),source_path(&last).unwrap());assert_eq!(revision(&first).unwrap(),revision(&last).unwrap());}
 #[test]fn reader1558_old_options_remain_unchanged_without_reading_reference(){let root=tempfile::tempdir().unwrap();let path=source(root.path());let mut options=DecompressOptions{preserve_paths:true,selected_entries:vec!["old.txt".into()],..Default::default()};apply_selection(&path,&mut options).unwrap();assert_eq!(options.selected_entries,vec!["old.txt"]);assert!(options.preserve_paths);}
}
