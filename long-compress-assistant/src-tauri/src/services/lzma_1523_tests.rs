use super::{archive_browser, archive_preview, codec_stream, compression_format::{self, CompressionRoute as R}, compression_verification, split_archive_detector::SplitArchiveDetector};
use std::{io::Read, path::{Path, PathBuf}, sync::{Arc, atomic::AtomicBool}};
use sha2::{Digest, Sha256};

fn fixtures() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/lzma-1523") }
fn split(dir: &Path, name: &str, bytes: &[u8]) -> Vec<PathBuf> {
    bytes.chunks((bytes.len() / 3).max(1)).enumerate().map(|(i,b)| { let p=dir.join(format!("{name}.{:03}",i+1)); std::fs::write(&p,b).unwrap(); p }).collect()
}

#[test]
fn lzma1523_independent_alone_originals_from_all_members() {
    let manifest: serde_json::Value=serde_json::from_slice(&std::fs::read(fixtures().join("manifest.json")).unwrap()).unwrap();
    for case in manifest["cases"].as_array().unwrap() {
        let name=case["archive"].as_str().unwrap(); let original=fixtures().join(name);
        let temp=tempfile::tempdir().unwrap(); let parts=split(temp.path(),name,&std::fs::read(&original).unwrap());
        for p in [&original,&parts[0],&parts[1],parts.last().unwrap()] {
            let source=SplitArchiveDetector::complete_source(p).unwrap();
            let mut bytes=vec![]; codec_stream::decoder(&source,"lzma").unwrap().read_to_end(&mut bytes).unwrap();
            assert_eq!(bytes.len() as u64,case["size"].as_u64().unwrap());
            assert_eq!(format!("{:x}",Sha256::digest(&bytes)),case["sha256"].as_str().unwrap());
            assert!(compression_verification::verify_native(R::Lzma,&source,None,||false).unwrap());
        }
    }
}

#[tokio::test]
async fn lzma1523_bounded_browse_text_image_and_public_integrity() {
    let file=fixtures().join("说明.txt.lzma");let temp=tempfile::tempdir().unwrap();let parts=split(temp.path(),"说明.txt.lzma",&std::fs::read(&file).unwrap());
    for p in [&parts[0],&parts[1],parts.last().unwrap()] {
        let browse=archive_browser::browse_archive(p,None).await.unwrap(); assert_eq!(browse.format,"LZMA");assert_eq!(browse.entries[0].path,"说明.txt");
        let preview=archive_preview::preview_archive_text(p,"说明.txt",None).await.unwrap(); assert!(preview.content.contains("收到 LZMA")); assert!(!preview.truncated);
        super::universal_engine::UniversalCliEngine::test_integrity(p,None).await.unwrap();
    }
    let large=fixtures().join("大文本.txt.lzma"); let browse=archive_browser::browse_archive(&large,None).await.unwrap();assert_eq!(browse.entries[0].size_known,Some(false));
    let preview=archive_preview::preview_archive_text(&large,"大文本.txt",None).await.unwrap();assert!(preview.truncated);assert!(preview.content.len()<=1024*1024);
    let image=archive_preview::preview_archive_image(&fixtures().join("图标.png.lzma"),"图标.png",None).await.unwrap();assert!(image.data_url.starts_with("data:image/png;base64,"));
}

#[tokio::test]
async fn lzma1523_truncated_trailing_and_missing_are_not_password_failures() {
    let temp=tempfile::tempdir().unwrap();let original=std::fs::read(fixtures().join("大文本.txt.lzma")).unwrap();
    for (name,bytes) in [("截断.txt.lzma",original[..original.len()-6].to_vec()),("尾随.txt.lzma",[original.as_slice(),b"unexpected trailing data"].concat())] {
        let path=temp.path().join(name);std::fs::write(&path,bytes).unwrap();
        let error=super::universal_engine::UniversalCliEngine::test_integrity(&path,None).await.unwrap_err();assert!(format!("{error:#}").contains("[archive-inspection:damaged]"),"{error:#}");
        let report=super::archive_diagnostics::diagnose_archive(&path,None,Arc::new(AtomicBool::new(false))).await.unwrap();assert_eq!(report.status,"damaged");assert!(report.integrity_tested);
    }
    let parts=split(temp.path(),"缺卷.txt.lzma",&original);std::fs::remove_file(&parts[1]).unwrap();let error=SplitArchiveDetector::complete_source(parts.last().unwrap()).unwrap_err();assert!(format!("{error:#}").contains("incomplete"));
}

#[test]
fn lzma1523_cancellation_during_full_verification() {
    let calls=std::cell::Cell::new(0);
    let error=compression_verification::verify_native(R::Lzma,&fixtures().join("大文本.txt.lzma"),None,||{calls.set(calls.get()+1);calls.get()>3}).unwrap_err();
    assert!(matches!(error.downcast_ref::<super::compression_service::CompressionError>(), Some(super::compression_service::CompressionError::Cancelled)), "{error:#}"); assert!(calls.get()>3);
}

#[test]
fn lzma1523_split_capability_and_native_route() {
    let c=compression_format::find_compression_format_capability("lzma").unwrap();assert!(c.supports_split);assert!(!c.requires_7za);assert!(!c.supports_password_extract);
    assert_eq!(compression_format::compression_route("lzma"),Some(R::Lzma));
}
