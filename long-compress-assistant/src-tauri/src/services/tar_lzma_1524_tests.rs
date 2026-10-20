use super::{archive_browser, archive_preview, aes_container::AuthenticatedInner, codec_stream,
    compression_format::{self as formats, CompressionRoute as R}, compression_verification,
    compression_service::CompressionService, universal_engine::UniversalCliEngine};
use crate::models::compression::{CompressionOptions, DecompressOptions};
use std::{fs, io::Read, path::{Path, PathBuf}};
use sha2::{Digest, Sha256};
fn fixtures() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/tar-lzma-1524") }
fn manifest() -> serde_json::Value { serde_json::from_slice(&fs::read(fixtures().join("manifest.json")).unwrap()).unwrap() }
fn sha(b: &[u8]) -> String { format!("{:x}", Sha256::digest(b)) }
fn parts(file: &Path, dir: &Path) -> Vec<PathBuf> {
    let bytes=fs::read(file).unwrap(); bytes.chunks(16381).enumerate().map(|(i,b)| {
        let path=dir.join(format!("{}.{:03}",file.file_name().unwrap().to_str().unwrap(),i+1));fs::write(&path,b).unwrap();path
    }).collect()
}
#[test]
fn lzma1524_requests_keep_format_password_and_single_file_boundaries() {
    let source=fixtures().join("原文件/大文本.txt").to_string_lossy().into_owned();
    for (format,route) in [("tar.lzma",R::TarLzma),("lzma.aes",R::LzmaAes),("tar.lzma.aes",R::TarLzmaAes)] {
        let encrypted=format.ends_with(".aes");
        let options=CompressionOptions{format:Some(format.into()),password:encrypted.then(||"test".into()),split_size:Some(8191),..Default::default()};
        assert_eq!(formats::validate_compression_request(std::slice::from_ref(&source),&format!("结果.{format}"),&options).unwrap(),format);
        assert_eq!(formats::compression_route(format),Some(route));
        assert_eq!(formats::infer_compression_format(&format!("结果.{format}"),None),format);
        assert_eq!(formats::has_native_password_container(format),encrypted);
        let folder=vec![fixtures().join("原文件").to_string_lossy().into_owned()];
        assert_eq!(formats::validate_compression_request(&folder,&format!("结果.{format}"),&options).is_ok(),format!="lzma.aes");
    }
    use super::{archive_format::ArchiveFormat as F,tar_compatibility::integrity_route};
    assert_eq!(integrity_route(Path::new("资料.tar.lzma.001"),&F::Lzma),Some(R::TarLzma));
    assert_eq!(integrity_route(Path::new("资料.tar.lzma"),&F::Zip),None);
}
#[tokio::test]
async fn lzma1524_external_originals_all_members_browse_preview_and_public_check() {
    let m=manifest();let password=m["password"].as_str();
    for c in m["cases"].as_array().unwrap() {
        let file=fixtures().join(c["archive"].as_str().unwrap()); assert_eq!(sha(&fs::read(&file).unwrap()),c["sha256"].as_str().unwrap());
        let temp=tempfile::tempdir().unwrap();let group=parts(&file,temp.path()); assert!(group.len()>2);
        for member in [&group[0],&group[group.len()/2],group.last().unwrap()] {
            let browse=archive_browser::browse_archive(member,password).await.unwrap();
            assert_eq!(browse.encrypted,c["encrypted"].as_bool().unwrap());
            assert_eq!(browse.format,if c["encrypted"]==true {if c["tar"]==true {"LONG AES.TAR.LZMA"}else{"LONG AES.LZMA"}} else {"TAR.LZMA"});
            let name=if c["tar"]==true {"说明.txt"}else{"大文本.txt"};
            let preview=archive_preview::preview_archive_text(member,name,password).await.unwrap();
            assert_eq!(sha(preview.content.as_bytes()),m["expectedFiles"][name].as_str().unwrap());
            UniversalCliEngine::test_integrity(member,password).await.unwrap();
        }
    }
}
#[test]
fn lzma1524_native_creation_streams_tar_and_preserves_content_and_empty_directory() {
    let m=manifest();let runtime=CompressionService::for_testing();
    for format in ["tar.lzma","lzma.aes","tar.lzma.aes"] {
        let temp=tempfile::tempdir().unwrap();let output=temp.path().join(format!("创建.{format}"));
        let source=if format=="lzma.aes" {fixtures().join("原文件/大文本.txt")} else {
            let source=temp.path().join("原文件");
            for name in m["expectedFiles"].as_object().unwrap().keys() {
                let target=source.join(name);fs::create_dir_all(target.parent().unwrap()).unwrap();
                fs::copy(fixtures().join("原文件").join(name),target).unwrap();
            }
            // Git does not retain empty directories in the source tree.
            fs::create_dir_all(source.join(m["emptyDirectory"].as_str().unwrap())).unwrap();source
        };
        let sources=vec![source.to_string_lossy().into_owned()];let options=CompressionOptions{level:1,password:Some("test".into()),..Default::default()};
        use super::native_compression::aes::{self,AesCompressionKind as K};
        match format {
            "tar.lzma"=>codec_stream::compress(&runtime,None,"lzma1524",&sources,&output.to_string_lossy(),CompressionOptions{password:None,..options},format).unwrap(),
            _=>aes::compress(&runtime,None,"lzma1524",&sources,&output.to_string_lossy(),options,if format=="lzma.aes" {K::Lzma}else{K::TarLzma}).unwrap(),
        }
        let group=parts(&output,temp.path());let inner;
        let input=if format.ends_with(".aes") {inner=AuthenticatedInner::open(&group[1],Some("test"),||Ok(())).unwrap();inner.path.as_path()}else{group[0].as_path()};
        let reader=codec_stream::decoder(input,"lzma").unwrap();
        if format=="lzma.aes" {let mut bytes=vec![];reader.take(u64::MAX).read_to_end(&mut bytes).unwrap();assert_eq!(sha(&bytes),m["expectedFiles"]["大文本.txt"].as_str().unwrap());}
        else {
            let mut tar=tar::Archive::new(reader);let mut files=0;let mut empty=false;
            for e in tar.entries().unwrap() {let mut e=e.unwrap();let name=e.path().unwrap().to_string_lossy().into_owned();let relative=name.strip_prefix("原文件/").unwrap_or(&name);
                if e.header().entry_type().is_file() {let mut bytes=vec![];e.read_to_end(&mut bytes).unwrap();assert_eq!(sha(&bytes),m["expectedFiles"][relative].as_str().unwrap());files+=1;}
                if e.header().entry_type().is_dir()&&relative.trim_end_matches('/')==m["emptyDirectory"].as_str().unwrap() {empty=true;}
            }assert_eq!(files,5);assert!(empty);super::tar_compatibility::finish_stream(tar.into_inner(),||Ok(())).unwrap();
        }
        assert!(compression_verification::verify_native(formats::compression_route(format).unwrap(),&group[0],Some("test"),||false).unwrap());
    }
}
#[tokio::test]
async fn lzma1524_wrong_password_tail_auth_and_authenticated_bad_inner_keep_real_causes() {
    let m=manifest();let password=m["password"].as_str();let temp=tempfile::tempdir().unwrap();
    let file=fixtures().join("资料.tar.lzma.aes");let group=parts(&file,temp.path());
    assert!(AuthenticatedInner::open(&group[1],Some("wrong"),||Ok(())).is_err());
    let tail=group.last().unwrap();let mut bytes=fs::read(tail).unwrap();*bytes.last_mut().unwrap()^=1;fs::write(tail,bytes).unwrap();
    let error=AuthenticatedInner::open(&group[1],password,||Ok(())).err().unwrap();assert!(format!("{error:#}").contains("后续分块"));
    let plain=temp.path().join("broken.tar.lzma");let original=fs::read(fixtures().join("资料.tar.lzma")).unwrap();fs::write(&plain,&original[..original.len()-6]).unwrap();
    let encrypted=temp.path().join("坏内层.tar.lzma.aes");super::aes_wrapper::AesWrapper::encrypt_file(&plain,&encrypted,"test").unwrap();
    for (p,password) in [(&plain,None),(&encrypted,Some("test"))] {
        let error=UniversalCliEngine::test_integrity(p,password).await.unwrap_err();assert!(format!("{error:#}").contains("[archive-inspection:damaged]"),"{error:#}");
    }
}
#[tokio::test]
async fn lzma1524_authenticated_tar_image_and_selected_extraction_keep_last_entry() {
    let m=manifest();let file=fixtures().join("资料.tar.lzma.aes");let password=m["password"].as_str();
    let image=archive_preview::preview_archive_image(&file,"图标.png",password).await.unwrap();assert!(image.data_url.starts_with("data:image/png;base64,"));
    let temp=tempfile::tempdir().unwrap();let inner=AuthenticatedInner::open(&file,password,||Ok(())).unwrap();
    let mut options=DecompressOptions{preserve_paths:true,..Default::default()};options.selected_entries=vec!["中文目录/数据.bin".into()];
    super::native_extraction::tar::extract_inner(&CompressionService::for_testing(),None,"lzma1524",&inner.path.to_string_lossy(),temp.path(),Some(codec_stream::decoder(&inner.path,"lzma").unwrap()),&options).unwrap();
    assert_eq!(sha(&fs::read(temp.path().join("中文目录/数据.bin")).unwrap()),m["expectedFiles"]["中文目录/数据.bin"].as_str().unwrap());assert!(!temp.path().join("大文本.txt").exists());
}
#[test]
fn lzma1524_history_analysis_and_engine_policy_expose_actual_formats() {
    use super::archive_format::ArchiveFormat as F;
    for (name,format,detected) in [("资料.tar.lzma.002","tar.lzma",F::Lzma),("资料.tar.lzma.aes.002","tar.lzma.aes",F::AesEncrypted),("文件.lzma.aes","lzma.aes",F::AesEncrypted)] {
        assert_eq!(CompressionService::extraction_history_format(Path::new(name),&detected),format);
    }
    assert!(!formats::find_compression_format_capability("tar.lzma").unwrap().requires_7za);
    assert!(formats::is_tar_wrapped_archive(Path::new("资料.tar.lzma.001"),"tar.lzma"));
}
