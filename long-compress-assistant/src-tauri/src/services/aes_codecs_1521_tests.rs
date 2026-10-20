use super::{aes_wrapper::AesWrapper, aes_container::AuthenticatedInner, compression_format as formats, compression_verification, archive_browser, archive_preview, universal_engine::UniversalCliEngine};
use crate::models::compression::{CompressionOptions,TaskLogSeverity};
use std::{fs,path::{Path,PathBuf}};
use sha2::{Digest,Sha256};
struct Runtime;
impl super::native_compression::CompressionRuntime for Runtime {
    fn check_cancellation(&self)->anyhow::Result<()> { Ok(()) }
    fn cancellation_flag(&self)->std::sync::Arc<std::sync::atomic::AtomicBool> { Default::default() }
    fn copy_buffer_size(&self)->usize { 65536 }
    fn emit_log(&self,_:&tauri::Window,_:&str,_:&str,_:TaskLogSeverity) {}
    fn emit_progress(&self,_:&tauri::Window,_:&str,_:f32,_:Option<String>,_:u64,_:u64) {}
}
fn fixtures()->PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/aes-codecs-1521") }
fn manifest()->serde_json::Value { serde_json::from_slice(&fs::read(fixtures().join("manifest.json")).unwrap()).unwrap() }
fn sha(bytes:&[u8])->String { format!("{:x}",Sha256::digest(bytes)) }
fn split(file:&Path,root:&Path)->Vec<PathBuf> {
    fs::read(file).unwrap().chunks(32749).enumerate().map(|(i,b)| {
        let p=root.join(format!("{}.{:03}",file.file_name().unwrap().to_str().unwrap(),i+1));fs::write(&p,b).unwrap();p
    }).collect()
}
#[test]
fn aes1521_capabilities_preserve_password_split_and_single_file_boundaries() {
    let source=fixtures().join("原文件/大文本.txt").to_string_lossy().into_owned();
    for f in ["lz4.aes","br.aes","lz.aes","tar.lz4.aes","tar.br.aes","tar.lz.aes","tlz.aes"] {
        let o=CompressionOptions{format:Some(f.into()),password:Some("test".into()),split_size:Some(8191),..Default::default()};
        let result=formats::validate_compression_request(std::slice::from_ref(&source),&format!("结果.{f}"),&o).unwrap();
        assert_eq!(result,if f=="tlz.aes" {"tar.lz.aes"} else {f});
        assert!(formats::has_native_password_container(f));
        assert!(formats::compression_route(f).is_some());
        assert_eq!(formats::find_compression_format_capability(f).unwrap().single_file_only,!f.starts_with("tar.")&&f!="tlz.aes");
        assert_eq!(formats::infer_compression_format(&format!("结果.{f}"),None),result);
        if !f.starts_with("tar.")&&f!="tlz.aes" {
            assert!(formats::validate_compression_request(&[fixtures().join("原文件").to_string_lossy().into_owned()],&format!("目录.{f}"),&o).is_err());
        }
    }
}
#[tokio::test]
async fn aes1521_independent_originals_middle_member_browse_previews_and_real_integrity() {
    let m=manifest();let password=m["password"].as_str().unwrap();
    for c in m["cases"].as_array().unwrap() {
        let temp=tempfile::tempdir().unwrap();let file=fixtures().join(c["archive"].as_str().unwrap());
        assert_eq!(sha(&fs::read(&file).unwrap()),c["sha256"].as_str().unwrap());
        let parts=split(&file,temp.path());assert!(parts.len()>2);let member=&parts[parts.len()/2];
        let inner=AuthenticatedInner::open(member,Some(password),||Ok(())).unwrap();
        assert_eq!(sha(&fs::read(&inner.path).unwrap()),c["innerSha256"].as_str().unwrap());
        let browse=archive_browser::browse_archive(member,Some(password)).await.unwrap();
        assert!(browse.encrypted&&browse.format.starts_with("LONG AES."));
        let tar=c["tar"].as_bool().unwrap();
        let entry=if tar {"说明.txt"} else {"大文本.txt"};
        let preview=archive_preview::preview_archive_text(member,entry,Some(password)).await.unwrap();
        assert_eq!(sha(preview.content.as_bytes()),m["expectedFiles"][entry].as_str().unwrap());
        if tar {let image=archive_preview::preview_archive_image(parts.last().unwrap(),"图标.png",Some(password)).await.unwrap();assert_eq!(image.mime_type,"image/png");}
        UniversalCliEngine::test_integrity(member,Some(password)).await.unwrap();
    }
}
#[test]
fn aes1521_native_creation_verifies_each_inner_and_never_splits_unencrypted_temporary() {
    use super::native_compression::aes::{self,AesCompressionKind as K};
    for (f,kind) in [("lz4.aes",K::Lz4),("br.aes",K::Brotli),("lz.aes",K::Lzip),("tar.lz4.aes",K::TarLz4),("tar.br.aes",K::TarBrotli),("tar.lz.aes",K::TarLzip)] {
        let temp=tempfile::tempdir().unwrap();let output=temp.path().join(format!("创建.{f}"));
        let source=fixtures().join(if f.starts_with("tar.") {"原文件"} else {"原文件/大文本.txt"});
        aes::compress(&Runtime,None,"aes1521",&[source.to_string_lossy().into_owned()],&output.to_string_lossy(),CompressionOptions{password:Some("password".into()),split_size:Some(8191),level:1,..Default::default()},kind).unwrap();
        let parts=split(&output,temp.path());let route=formats::compression_route(f).unwrap();
        assert!(compression_verification::verify_native(route,&parts[0],Some("password"),||false).unwrap());
        if !f.starts_with("tar.") { let inner=AuthenticatedInner::open(&parts[1],Some("password"),||Ok(())).unwrap();let mut bytes=Vec::new();use std::io::Read;super::codec_stream::decoder(&inner.path,f.trim_end_matches(".aes")).unwrap().read_to_end(&mut bytes).unwrap();assert_eq!(bytes,fs::read(source).unwrap()); }
    }
}
#[tokio::test]
async fn aes1521_plain_codecs_use_correct_public_integrity_route_including_lzip_regression() {
    let m=manifest();for c in m["cases"].as_array().unwrap().iter().filter(|c|!c["tar"].as_bool().unwrap()) {
        let file=fixtures().join(c["archive"].as_str().unwrap());let temp=tempfile::tempdir().unwrap();
        let plain=temp.path().join(file.file_stem().unwrap());AesWrapper::decrypt_file(&file,&plain,m["password"].as_str().unwrap()).unwrap();
        UniversalCliEngine::test_integrity(&plain,None).await.unwrap();
    }
}
#[test]
fn aes1521_known_new_inner_rejects_wrong_password_late_auth_and_authenticated_garbage() {
    let m=manifest();let c=m["cases"].as_array().unwrap().iter().find(|c|c["format"]=="tar.lz.aes").unwrap();
    let file=fixtures().join(c["archive"].as_str().unwrap());let temp=tempfile::tempdir().unwrap();let parts=split(&file,temp.path());
    assert!(AuthenticatedInner::open(&parts[1],Some("wrong"),||Ok(())).is_err());
    let tail=parts.last().unwrap();let mut bytes=fs::read(tail).unwrap();*bytes.last_mut().unwrap()^=1;fs::write(tail,bytes).unwrap();
    let error=AuthenticatedInner::open(&parts[1],m["password"].as_str(),||Ok(())).err().unwrap();assert!(format!("{error:#}").contains("后续分块"));
    let plain=temp.path().join("garbage");fs::write(&plain,b"not a LZIP stream").unwrap();let bad=temp.path().join("已认证但内层损坏.tar.lz.aes");AesWrapper::encrypt_file(&plain,&bad,"test").unwrap();
    assert!(compression_verification::verify_native(formats::CompressionRoute::TarLzipAes,&bad,Some("test"),||false).is_err());
}

#[test]
fn aes1521_history_uses_canonical_wrapped_format_including_tlz_alias() {
    use super::{compression_service::CompressionService,archive_format::ArchiveFormat};
    for (name,expected) in [("资料.tar.lz4.aes.002","tar.lz4.aes"),("资料.tlz.aes.004","tar.lz.aes"),("资料.tgz.aes","tar.gz.aes"),("文件.lz.aes","lz.aes")] {
        assert_eq!(CompressionService::extraction_history_format(Path::new(name),&ArchiveFormat::AesEncrypted),expected);
    }
}
