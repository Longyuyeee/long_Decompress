use super::*;
use crate::services::{compression_service::CompressionService, zip_method::ZipMethod};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, sync::{Arc, atomic::{AtomicBool, Ordering}}};
use crate::services::{zip_password::ZipEncryption, standard_zip_writer::ZipSplitMode};
fn root() -> std::path::PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().into() }
fn tree(base: &Path) -> BTreeMap<String, String> {
    walkdir::WalkDir::new(base).into_iter().map(|e| e.unwrap()).filter(|e|e.file_type().is_file())
        .map(|e|(e.path().strip_prefix(base).unwrap().to_string_lossy().replace('\\',"/"),hex::encode(Sha256::digest(fs::read(e.path()).unwrap())))).collect()
}
fn seed(base: &Path) -> std::path::PathBuf {
    let source=base.join("资料"); fs::create_dir_all(source.join("项目/空目录")).unwrap();
    fs::write(source.join("说明.txt"),"LZMA 中文内容\n".repeat(100)).unwrap();
    fs::write(source.join("项目/说明.txt"),"同名不同目录\n".repeat(80)).unwrap();
    fs::write(source.join("空文件.txt"),[]).unwrap();
    fs::copy(root().join("src-tauri/icons/32x32.png"),source.join("图标.png")).unwrap();
    let mut data=Vec::new(); for chunk in 0..16 { for n in 0..256 { data.extend_from_slice(&Sha256::digest(format!("{chunk}:{n}").as_bytes())); } data.extend_from_slice(&vec![65+chunk;8192]); }
    fs::write(source.join("项目/内容.bin"),data).unwrap(); source
}
#[test]
fn zip1538_probe_real_method14_header_eos_empty_and_independent_readback() {
    let temp=tempfile::tempdir().unwrap();let source=seed(temp.path());let out=temp.path().join("方法14.zip");
    compress(&CompressionService::for_testing(),None,"1538-probe",&[source.to_string_lossy().into_owned()],&out.to_string_lossy(),CompressionOptions{zip_method:ZipMethod::Lzma,preserve_paths:Some(true),..Default::default()}).unwrap();
    let expected=tree(&source).into_iter().map(|(n,h)|(format!("资料/{n}"),h)).collect::<BTreeMap<_,_>>();
    let request=temp.path().join("request.json");fs::write(&request,serde_json::to_vec(&serde_json::json!({"file":out,"expectedFiles":expected,"emptyDirectory":"资料/项目/空目录"})).unwrap()).unwrap();
    let python=crate::utils::process::command("python").arg(root().join("scripts/verify-zip-lzma-1538.py")).arg(&request).output().unwrap();
    assert!(python.status.success(),"{} {}",String::from_utf8_lossy(&python.stdout),String::from_utf8_lossy(&python.stderr));println!("{}",String::from_utf8_lossy(&python.stdout));
    let destination=temp.path().join("7zip");let result=crate::utils::process::command(root().join("src-tauri/resources/archive-engine/7z.exe")).args(["x","-y","-sccUTF-8"]).arg(&out).arg(format!("-o{}",destination.display())).output().unwrap();
    assert!(result.status.success(),"{} {}",String::from_utf8_lossy(&result.stdout),String::from_utf8_lossy(&result.stderr));assert_eq!(tree(&destination),expected);assert!(destination.join("资料/项目/空目录").is_dir());
    println!("ZIP LZMA independent Python/liblzma + 7-Zip complete content/empty entries passed");
}

#[test]
fn zip1538_lzma_plain_aes_zipcrypto_numeric_and_standard_independent_complete_readback(){
 for password in [None,Some(ZipEncryption::Aes256),Some(ZipEncryption::Zipcrypto)] {for split in [None,Some(ZipSplitMode::Numeric),Some(ZipSplitMode::Standard)]{
  let temp=tempfile::tempdir().unwrap();let source=seed(temp.path());let output=temp.path().join("new.zip");let sources=vec![source.to_string_lossy().into_owned()];let secret=password.map(|_|"LzmaKey1538!");
  let options=CompressionOptions{format:Some("zip".into()),zip_method:ZipMethod::Lzma,level:6,password:secret.map(str::to_string),zip_encryption:password.unwrap_or_default(),split_size:split.map(|_|65536),zip_split_mode:split.unwrap_or_default(),preserve_paths:Some(true),..Default::default()};
  compress(&CompressionService::for_testing(),None,"1538",&sources,&output.to_string_lossy(),options).unwrap();
  let parts=if split.is_some(){crate::services::archive_volumes::complete_volume_paths(&output).unwrap()}else{vec![output.clone()]};if split.is_some(){assert!(parts.len()>2)}
  let normalized=crate::services::split_archive_detector::SplitArchiveDetector::complete_source(&parts[parts.len()/2]).unwrap();let reader=crate::services::numeric_volume_reader::ZipInputReader::open(&normalized).unwrap();let mut archive=zip_aes::ZipArchive::new(reader).unwrap();let mut restored=BTreeMap::new();
  for index in 0..archive.len(){let mut file=if let Some(secret)=secret{archive.by_index_decrypt(index,secret.as_bytes()).unwrap()}else{archive.by_index(index).unwrap()};if file.is_dir(){continue}assert_eq!(file.compression(),zip_aes::CompressionMethod::Lzma);let mut data=Vec::new();file.read_to_end(&mut data).unwrap();restored.insert(file.name().to_string(),hex::encode(Sha256::digest(data)));}
  let expected=tree(&source).into_iter().map(|(n,h)|(format!("资料/{n}"),h)).collect::<BTreeMap<_,_>>();assert_eq!(restored,expected);
  if password.is_some(){let index=archive.index_for_name("资料/项目/内容.bin").unwrap();let failed=match archive.by_index_decrypt(index,b"wrong"){Err(_)=>true,Ok(mut entry)=>entry.read_to_end(&mut Vec::new()).is_err()};assert!(failed,"wrong password must fail")}
  let independent_input=if split==Some(ZipSplitMode::Numeric){let joined=temp.path().join("oracle.zip");let bytes:Vec<u8>=parts.iter().flat_map(|p|fs::read(p).unwrap()).collect();fs::write(&joined,bytes).unwrap();joined}else{output};
  if password==Some(ZipEncryption::Aes256) && split!=Some(ZipSplitMode::Standard) {
   let request=temp.path().join("aes.json");fs::write(&request,serde_json::to_vec(&serde_json::json!({"file":independent_input,"password":secret,"expectedFiles":expected,"emptyDirectory":"资料/项目/空目录"})).unwrap()).unwrap();
   let oracle=crate::utils::process::command("python").arg(root().join("scripts/verify-zip-lzma-1538.py")).arg(&request).output().unwrap();assert!(oracle.status.success(),"{}",String::from_utf8_lossy(&oracle.stderr));
  }
  let destination=temp.path().join("independent");let mut cmd=crate::utils::process::command(root().join("src-tauri/resources/archive-engine/7z.exe"));cmd.args(["x","-y","-sccUTF-8"]).arg(&independent_input).arg(format!("-o{}",destination.display()));if let Some(secret)=secret{cmd.arg(format!("-p{secret}"));}let result=cmd.output().unwrap();assert!(result.status.success(),"{password:?}/{split:?}: {} {}",String::from_utf8_lossy(&result.stdout),String::from_utf8_lossy(&result.stderr));assert_eq!(tree(&destination),expected);assert!(destination.join("资料/项目/空目录").is_dir());println!("LZMA {password:?} {split:?}: {} parts, independent hashes/empty directories OK",parts.len());
 }}
}

#[test]
fn zip1538_levels_encode_real_dictionary_and_raw_copy_preserves_eos() {
    let temp=tempfile::tempdir().unwrap();let source=seed(temp.path());let expected=tree(&source).into_iter().map(|(n,h)|(format!("资料/{n}"),h)).collect::<BTreeMap<_,_>>();
    for (level,dictionary) in [(0,1u32<<20),(6,8<<20),(9,64<<20)] {
        let out=temp.path().join(format!("level-{level}.zip"));
        compress(&CompressionService::for_testing(),None,"1538-level",&[source.to_string_lossy().into_owned()],&out.to_string_lossy(),CompressionOptions{level,zip_method:ZipMethod::Lzma,preserve_paths:Some(true),..Default::default()}).unwrap();
        let bytes=fs::read(&out).unwrap();let mut reader=zip_aes::ZipArchive::new(File::open(&out).unwrap()).unwrap();
        let mut copy=zip_aes::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for index in 0..reader.len() {let entry=reader.by_index(index).unwrap();if !entry.is_dir(){let start=entry.data_start().unwrap() as usize;assert_eq!(u32::from_le_bytes(bytes[start+5..start+9].try_into().unwrap()),dictionary);}copy.raw_copy_file(entry).unwrap();}
        let copied=temp.path().join("copied.zip");fs::write(&copied,copy.finish().unwrap().into_inner()).unwrap();
        let request=temp.path().join("levels.json");fs::write(&request,serde_json::to_vec(&serde_json::json!({"file":copied,"expectedFiles":expected,"emptyDirectory":"资料/项目/空目录"})).unwrap()).unwrap();
        let oracle=crate::utils::process::command("python").arg(root().join("scripts/verify-zip-lzma-1538.py")).arg(&request).output().unwrap();assert!(oracle.status.success(),"{}",String::from_utf8_lossy(&oracle.stderr));
        println!("LZMA strength {level}: dictionary {} MiB; independent decoded raw copy",dictionary>>20);
    }
    let legacy:CompressionOptions=serde_json::from_value(serde_json::json!({"level":0})).unwrap();assert_eq!(legacy.zip_method,ZipMethod::Deflate);
}
struct CancelRuntime{checks:std::cell::Cell<u32>,flag:Arc<AtomicBool>}
impl CompressionRuntime for CancelRuntime{
 fn check_cancellation(&self)->Result<()>{let n=self.checks.get();self.checks.set(n+1);if n>5{self.flag.store(true,Ordering::SeqCst)}if self.flag.load(Ordering::SeqCst){Err(CompressionError::Cancelled.into())}else{Ok(())}}
 fn cancellation_flag(&self)->Arc<AtomicBool>{self.flag.clone()}
 fn copy_buffer_size(&self)->usize{4096}
 fn emit_log(&self,_:&Window,_:&str,_:&str,_:TaskLogSeverity){}
 fn emit_progress(&self,_:&Window,_:&str,_:f32,_:Option<String>,_:u64,_:u64){}
}
#[test]
fn zip1538_lzma_midstream_cancel_never_publishes_and_preserves_existing_output(){
 let temp=tempfile::tempdir().unwrap();let source=seed(temp.path());let output=temp.path().join("old.zip");fs::write(&output,b"previous user file").unwrap();let staged=crate::services::output_publish_transaction::staged_output_path(&output,"lzma-test").unwrap();let runtime=CancelRuntime{checks:std::cell::Cell::new(0),flag:Arc::new(AtomicBool::new(false))};
 let result=compress(&runtime,None,"1538-cancel",&[source.to_string_lossy().into_owned()],&staged.to_string_lossy(),CompressionOptions{zip_method:ZipMethod::Lzma,preserve_paths:Some(true),..Default::default()});assert!(matches!(result.as_ref().unwrap_err().downcast_ref::<CompressionError>(),Some(CompressionError::Cancelled)));
 assert!(CompressionService::finalize_compression_output(result,&staged,&output,false,false).is_err());assert!(!staged.exists());assert_eq!(fs::read(output).unwrap(),b"previous user file");
}
