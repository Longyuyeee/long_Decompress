use super::{archive_browser, archive_preview, lzop, zip_password::ZipPasswordEncoding as Encoding};
use crate::models::compression::DecompressOptions;
use sha2::{Digest, Sha256};
use std::{io::Read, path::{Path,PathBuf}};
fn root()->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/lzop-paths-1661")}
fn sha(bytes:&[u8])->String {format!("{:x}",Sha256::digest(bytes))}
fn manifest()->serde_json::Value {serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()}
struct Runtime(super::io_buffer_pool::IOBufferPool);
impl super::native_extraction::ExtractionRuntime for Runtime {
 fn check_cancellation(&self)->anyhow::Result<()> {Ok(())}
 fn buffer_pool(&self)->&super::io_buffer_pool::IOBufferPool {&self.0}
 fn copy_buffer_size(&self)->usize {65536}
 fn normalized_archive_path(&self,p:&Path,_:bool)->Option<PathBuf>{Some(p.to_owned())}
 fn emit_log(&self,_:&tauri::Window,_:&str,_:&str,_:crate::models::compression::TaskLogSeverity){}
 fn emit_progress(&self,_:&tauri::Window,_:&str,_:f32,_:Option<String>,_:u64,_:u64){}
}
fn runtime()->Runtime {Runtime(super::io_buffer_pool::IOBufferPool::new(Default::default()))}
// Derived renamed headers; these are intentionally not presented as external originals.
fn member(name:&[u8],charset:u32,paths:bool)->Vec<u8>{
 let b=std::fs::read(root().parent().unwrap().join("lzop-read-1657/method1.txt.lzo")).unwrap();
 let end=34+b[33] as usize;let mut h=b[9..34].to_vec();h[24]=u8::try_from(name.len()).unwrap();
 let flags=(u32::from_be_bytes(b[17..21].try_into().unwrap())&!0x00f02000)|0x1400|charset|if paths{0x2000}else{0};
 h[8..12].copy_from_slice(&flags.to_be_bytes());h.extend(name);let crc=crc32fast::hash(&h);
 let mut out=b[..9].to_vec();out.extend(h);out.extend(crc.to_be_bytes());out.extend_from_slice(&b[end+4..]);out
}
#[tokio::test]
async fn lzop1661_official_paths_directories_previews_selection_flat_and_complete_readback(){
 let p=root().join("paths.lzo");let manifest=manifest();assert_eq!(sha(&std::fs::read(&p).unwrap()),manifest["archiveSha256"]);
 let list=archive_browser::browse_archive(&p,None).await.unwrap();assert_eq!(list.total_files,4);
 for d in ["docs/","docs/more/","pictures/"] {assert!(list.entries.iter().any(|e|e.path==d&&e.is_dir));}
 let text=archive_preview::preview_archive_text(&p,"docs/notes.txt",None).await.unwrap();assert!(text.content.contains("按目录浏览"));
 let image=archive_preview::preview_archive_image(&p,"pictures/cover.png",None).await.unwrap();assert_eq!((image.width,image.height),(32,32));
 lzop::verify(&p,||false).unwrap();let out=tempfile::tempdir().unwrap();
 let options=DecompressOptions{preserve_paths:true,..Default::default()};
 lzop::extract_members(&runtime(),None,"1661",&p,out.path(),&options).unwrap();
 for e in manifest["expected"].as_array().unwrap(){assert_eq!(sha(&std::fs::read(out.path().join(e["path"].as_str().unwrap())).unwrap()),e["sha256"]);}
 let selected=tempfile::tempdir().unwrap();let options=DecompressOptions{preserve_paths:true,selected_entries:vec!["docs/".into()],file_filter:Some("*.txt".into()),..Default::default()};
 lzop::extract_members(&runtime(),None,"1661",&p,selected.path(),&options).unwrap();assert!(selected.path().join("docs/more/notes.txt").exists());assert!(!selected.path().join("pictures").exists());
 let flat=tempfile::tempdir().unwrap();let options=DecompressOptions{selected_entries:vec!["docs/more/notes.txt".into()],..Default::default()};
 lzop::extract_members(&runtime(),None,"1661",&p,flat.path(),&options).unwrap();assert!(flat.path().join("notes (member 2).txt").exists());assert_eq!(std::fs::read_dir(flat.path()).unwrap().count(),1);
}
#[tokio::test]
async fn lzop1661_legacy_codepages_share_names_between_listing_preview_and_retrieval(){
 let t=tempfile::tempdir().unwrap();let p=t.path().join("legacy.lzo");
 for (page,raw,name) in [(Encoding::Cp936,b"docs/\xd7\xca\xc1\xcf.txt".as_slice(),"docs/资料.txt"),(Encoding::Cp932,b"docs/\x93\xfa\x96\x7b.txt".as_slice(),"docs/日本.txt"),(Encoding::Cp950,b"docs/\xb8\xea\xae\xc6.txt".as_slice(),"docs/資料.txt"),(Encoding::Cp437,b"docs/caf\x82.txt".as_slice(),"docs/café.txt"),(Encoding::Cp850,b"docs/\x9b.txt".as_slice(),"docs/ø.txt")] {
  std::fs::write(&p,member(raw,0x00300000,true)).unwrap();
  let auto=lzop::member_index(&p,||Ok(())).unwrap();assert_eq!(auto[0].name,"member-001.bin");
  let list=archive_browser::browse_archive_with_encoding(&p,None,Some(page)).await.unwrap();assert!(list.entries.iter().any(|e|e.path==name),"{page:?}: {list:?}");
  let preview=archive_preview::preview_archive_text_with_encoding(&p,name,None,Some(page)).await.unwrap();assert!(!preview.content.is_empty());
  let out=tempfile::tempdir().unwrap();let options=DecompressOptions{preserve_paths:true,zip_password_encoding:Some(page),selected_entries:vec!["docs/".into()],..Default::default()};
  lzop::extract_members(&runtime(),None,"1661",&p,out.path(),&options).unwrap();assert!(out.path().join(name).exists());
 }
 std::fs::write(&p,member(b"\x81.txt",0x00300000,false)).unwrap();
 let error=format!("{:#}",lzop::member_index_with_encoding(&p,Some(Encoding::Cp932),||Ok(())).unwrap_err());assert!(error.contains("lzop-encoding")&&!error.contains("password"),"{error}");
}
#[test]
fn lzop1661_paths_case_collisions_intersections_unknown_and_long_names_remain_distinct(){
 let t=tempfile::tempdir().unwrap();let p=t.path().join("names.lzo");let long=format!("{}.txt","a".repeat(190));
 let names=["a","a/b.txt","a/c.txt","A/B.TXT","dir/x.txt","dir","../outside.txt","C:/outside.txt","/root.txt","CON .txt",&long];
 std::fs::write(&p,names.iter().flat_map(|n|member(n.as_bytes(),0x00500000,true)).collect::<Vec<_>>()).unwrap();
 let index=lzop::member_index(&p,||Ok(())).unwrap();let actual:Vec<_>=index.iter().map(|m|m.name.as_str()).collect();
 assert_eq!(actual,vec!["a","a (member 2)/b.txt","a (member 2)/c.txt","a (member 2)/B (member 4).TXT","dir/x.txt","dir (member 6)","outside.txt","outside (member 8).txt","root.txt","member-10-CON .txt",&long]);
 let out=tempfile::tempdir().unwrap();lzop::extract_members(&runtime(),None,"1661",&p,out.path(),&DecompressOptions{preserve_paths:true,..Default::default()}).unwrap();
 for m in &index{let mut bytes=Vec::new();lzop::member_reader(&p,m).unwrap().read_to_end(&mut bytes).unwrap();assert_eq!(std::fs::read(out.path().join(&m.name)).unwrap(),bytes);}
}
#[tokio::test]
async fn lzop1661_path_original_numeric_non_first_damaged_tail_and_cancellation(){
 let t=tempfile::tempdir().unwrap();let bytes=std::fs::read(root().join("paths.lzo")).unwrap();let cuts=[0,7,bytes.len()-2,bytes.len()];let mut parts=Vec::new();
 for(i,w)in cuts.windows(2).enumerate(){let p=t.path().join(format!("paths.lzo.{:03}",i+1));std::fs::write(&p,&bytes[w[0]..w[1]]).unwrap();parts.push(p);}
 let list=archive_browser::browse_archive(&parts[1],None).await.unwrap();assert!(list.entries.iter().any(|e|e.path=="docs/more/notes.txt"));
 lzop::verify(&parts[1],||false).unwrap();assert!(lzop::verify(&parts[1],||true).is_err());
 let mut checks=0;assert!(lzop::member_index(&parts[1],||{checks+=1;anyhow::ensure!(checks<4,"取消索引");Ok(())}).is_err());
 std::fs::remove_file(&parts[2]).unwrap();assert!(format!("{:#}",lzop::verify(&parts[1],||false).unwrap_err()).contains("可能缺少后续卷"));
}
