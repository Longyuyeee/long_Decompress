use super::{archive_browser, archive_diagnostics, archive_preview, compression_format, compression_service::CompressionService, split_archive_detector::SplitArchiveDetector, zpaq_engine};
use std::{fs, path::{Path, PathBuf}, sync::{Arc, atomic::AtomicBool}};
fn flag() -> Arc<AtomicBool> { Arc::new(AtomicBool::new(false)) }
fn fixture(name: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/zpaq").join(name) }
fn parts(root: &Path, source: &Path, tiny: bool) -> Vec<PathBuf> {
    let bytes=fs::read(source).unwrap(); let mut offset=0; let mut files=Vec::new();
    while offset<bytes.len() {
        let size=if tiny {29} else if files.is_empty() {7} else if files.len()==1 {13} else {4096};
        let end=(offset+size).min(bytes.len()); let file=root.join(format!("资料.zpaq.{:03}",files.len()+1));
        fs::write(&file,&bytes[offset..end]).unwrap(); files.push(file); offset=end;
    } files
}

#[tokio::test]
async fn zpaq_1526_independent_journal_stream_and_native_password_slices() {
    for name in ["journal.zpaq","journal-key.zpaq","stream.zpaq","stream-key.zpaq"] {
        let root=tempfile::tempdir().unwrap();let files=parts(root.path(),&fixture(name),false);
        let password=name.contains("-key").then_some("中文 密码!-钥匙");
        for member in [&files[0],&files[files.len()/2],files.last().unwrap()] {
            assert_eq!(archive_browser::detect_format(member).unwrap(),super::archive_format::ArchiveFormat::Zpaq);
            assert_eq!(zpaq_engine::requires_password(member).unwrap(),password.is_some());
            let listing=archive_browser::browse_archive_cancellable(member,password,flag()).await.unwrap();
            assert_eq!(listing.total_files,3,"{name}");assert!(!listing.entries.iter().any(|entry|entry.path.contains("已删除")));
            assert_eq!(listing.total_compressed_size,fs::metadata(fixture(name)).unwrap().len());
        }
        zpaq_engine::verify(&files[files.len()/2],password,flag()).await.unwrap();
        let output=tempfile::tempdir().unwrap();
        zpaq_engine::extract(files.last().unwrap(),output.path(),password,&["项目 文档/说明.txt".into()],flag()).await.unwrap();
        assert_eq!(fs::read_to_string(output.path().join("项目 文档/说明.txt")).unwrap(),"第二版：当前文件视图，不混入旧版本。\r\n");
        assert!(!output.path().join("项目 文档/数据.bin").exists());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(),files.len(),"No joined archive or extracted preview beside sources");
    }
}

#[tokio::test]
async fn zpaq_1526_more_than_999_and_salt_across_members() {
    let root=tempfile::tempdir().unwrap();let files=parts(root.path(),&fixture("stream-key.zpaq"),true);
    assert!(files.len()>1000);assert!(files[999].ends_with("资料.zpaq.1000"));
    zpaq_engine::verify(&files[1000],Some("中文 密码!-钥匙"),flag()).await.unwrap();
    let output=tempfile::tempdir().unwrap();
    zpaq_engine::extract(&files[999],output.path(),Some("中文 密码!-钥匙"),&[],flag()).await.unwrap();
    assert_eq!(fs::read(output.path().join("项目 文档/数据.bin")).unwrap(),(0..32768).map(|i|(i*37)as u8).collect::<Vec<_>>());
}

#[tokio::test]
async fn zpaq_1526_missing_damage_and_password_causes() {
    let root=tempfile::tempdir().unwrap();let files=parts(root.path(),&fixture("journal-key.zpaq"),false);
    assert!(zpaq_engine::verify(files.last().unwrap(),Some("wrong"),flag()).await.unwrap_err().to_string().contains("ambiguous-data"));
    let withdrawn=fs::read(&files[1]).unwrap();fs::remove_file(&files[1]).unwrap();
    let error=zpaq_engine::browse(files.last().unwrap(),Some("wrong"),flag()).await.unwrap_err().to_string();
    assert!(error.contains("incomplete"),"{error}");assert!(!error.contains("ambiguous-data"));
    fs::write(&files[1],withdrawn).unwrap();
    let last=files.last().unwrap();let mut bytes=fs::read(last).unwrap();bytes.truncate(bytes.len()-20);fs::write(last,bytes).unwrap();
    let error=zpaq_engine::verify(&files[0],Some("中文 密码!-钥匙"),flag()).await.unwrap_err().to_string();
    assert!(error.contains("damaged"),"{error}");
}

#[tokio::test]
async fn zpaq_1526_diagnosis_and_password_candidate_use_logical_format() {
    let root=tempfile::tempdir().unwrap();let files=parts(root.path(),&fixture("journal-key.zpaq"),false);
    let service=CompressionService::new_with_defaults().await;
    assert!(service.verify_archive_password_candidate(&files[1].to_string_lossy(),"中文 密码!-钥匙").await.unwrap());
    let report=archive_diagnostics::diagnose_archive(&files[1],Some("中文 密码!-钥匙"),flag()).await.unwrap();
    assert_eq!(report.actual_format,"ZPAQ");assert_eq!(report.status,"healthy");assert!(report.integrity_tested&&report.encrypted&&report.split_archive);
    let group=SplitArchiveDetector::detect_split_archive(&files[1]).unwrap().unwrap();assert_eq!(group.parts,files);
}

#[tokio::test]
async fn zpaq_1526_bounded_text_preview_and_unknown_stream_size() {
    let root=tempfile::tempdir().unwrap();let files=parts(root.path(),&fixture("journal-key.zpaq"),false);
    let preview=archive_preview::preview_archive_text(&files[1],"项目 文档/说明.txt",Some("中文 密码!-钥匙")).await.unwrap();
    assert!(preview.content.contains("第二版"));assert!(!preview.truncated);
    assert!(zpaq_engine::preview_entry(&files[0],"项目 文档/说明.txt",Some("中文 密码!-钥匙"),1,1).await.unwrap_err().to_string().contains("预览范围"));
    let stream=tempfile::tempdir().unwrap();let files=parts(stream.path(),&fixture("stream.zpaq"),false);
    assert!(archive_preview::preview_archive_text(&files[1],"项目 文档/说明.txt",None).await.unwrap_err().to_string().contains("大小未知"));
}

#[tokio::test]
async fn zpaq_1526_created_password_archive_image_and_request_capability() {
    use crate::models::compression::CompressionOptions;
    let root=tempfile::tempdir().unwrap();let input=root.path().join("图标.png");
    fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("icons/32x32.png"),&input).unwrap();
    let options=CompressionOptions{format:Some("zpaq".into()),password:Some("原生 密码".into()),split_size:Some(37),..Default::default()};
    assert_eq!(compression_format::validate_compression_request(&[input.to_string_lossy().into_owned()],&root.path().join("新建.zpaq").to_string_lossy(),&options).unwrap(),"zpaq");
    let original=root.path().join("新建.zpaq");zpaq_engine::create(&[input.to_string_lossy().into_owned()],&original,&options,flag()).await.unwrap();
    let split=root.path().join("slices");fs::create_dir(&split).unwrap();let files=parts(&split,&original,false);
    let preview=archive_preview::preview_archive_image(files.last().unwrap(),"图标.png",Some("原生 密码")).await.unwrap();
    assert_eq!((preview.width,preview.height),(32,32));
    use base64::Engine;
    assert_eq!(base64::engine::general_purpose::STANDARD.decode(preview.data_url.split(',').nth(1).unwrap()).unwrap(),fs::read(input).unwrap());
}
