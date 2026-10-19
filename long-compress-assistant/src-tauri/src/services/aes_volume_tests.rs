use super::{aes_stream_v2::{AesStreamKind, AesStreamV2}, compression_format, aes_wrapper::AesWrapper};
use crate::models::compression::CompressionOptions;
use std::{fs, path::{Path, PathBuf}};

fn fixtures() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/aes-1518") }
struct Runtime;
impl super::native_compression::CompressionRuntime for Runtime {
    fn check_cancellation(&self) -> anyhow::Result<()> { Ok(()) }
    fn cancellation_flag(&self) -> std::sync::Arc<std::sync::atomic::AtomicBool> { Default::default() }
    fn copy_buffer_size(&self) -> usize { 65536 }
    fn emit_log(&self, _: &tauri::Window, _: &str, _: &str, _: crate::models::compression::TaskLogSeverity) {}
    fn emit_progress(&self, _: &tauri::Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {}
}
fn split(source: &Path, root: &Path, size: usize) -> Vec<PathBuf> {
    let bytes = fs::read(source).unwrap();
    bytes.chunks(size).enumerate().map(|(i, bytes)| {
        let part=root.join(format!("{}.{:03}",source.file_name().unwrap().to_str().unwrap(),i+1));
        fs::write(&part,bytes).unwrap(); part
    }).collect()
}

#[test]
fn aes1518_existing_creation_reaches_numeric_split_for_all_nine_formats() {
    let temp=tempfile::tempdir().unwrap(); let source=temp.path().join("source.txt");
    fs::write(&source,b"real source").unwrap();
    for format in ["tar.aes","tar.gz.aes","tar.bz2.aes","tar.xz.aes","tar.zst.aes","gz.aes","bz2.aes","xz.aes","zst.aes"] {
        let options=CompressionOptions {format:Some(format.into()),password:Some("Long 分卷-1518".into()),split_size:Some(8191),..Default::default()};
        let output=temp.path().join(format!("new.{format}"));
        let sources=vec![source.to_string_lossy().into_owned()];
        assert_eq!(compression_format::validate_compression_request(&sources,&output.to_string_lossy(),&options).unwrap(),format);
    }
}

#[test]
fn aes1518_independent_original_reads_header_across_tiny_volumes() {
    let temp=tempfile::tempdir().unwrap();
    let parts=split(&fixtures().join("v2-说明.txt.gz.aes"),temp.path(),37);
    let output=temp.path().join("decoded.gz");
    AesStreamV2::decrypt_file(&parts[0],&output,"Long 分卷-1518",AesStreamKind::Generic).unwrap();
    assert_eq!(fs::read(output).unwrap(),fs::read(fixtures().join("../multistream-1517/说明.txt.gz")).unwrap());
}

#[test]
fn aes1518_independent_original_single_numeric_member_control() {
    let temp=tempfile::tempdir().unwrap();
    let parts=split(&fixtures().join("v2-说明.txt.gz.aes"),temp.path(),8191);
    let output=temp.path().join("decoded.gz");
    AesWrapper::decrypt_file(&parts[parts.len()/2],&output,"Long 分卷-1518").unwrap();
    assert_eq!(fs::read(output).unwrap(),fs::read(fixtures().join("../multistream-1517/说明.txt.gz")).unwrap());
}

#[test]
fn aes1518_all_independent_v1_v2_originals_decrypt_from_middle_member() {
    let manifest:serde_json::Value=serde_json::from_slice(&fs::read(fixtures().join("manifest.json")).unwrap()).unwrap();
    for c in manifest["cases"].as_array().unwrap() {
        let temp=tempfile::tempdir().unwrap();
        let parts=split(&fixtures().join(c["archive"].as_str().unwrap()),temp.path(),127);
        assert!(parts.len()>2);
        let output=temp.path().join("decoded");
        let kind=if c["codec"]=="" {AesStreamKind::Tar}else{AesStreamKind::Generic};
        if c["version"]==2 {
            AesStreamV2::decrypt_file(&parts[parts.len()/2],&output,"Long 分卷-1518",kind).unwrap();
        } else if kind==AesStreamKind::Tar {
            super::tar_aes_engine::TarAesEngine::decrypt_to_file(&parts[parts.len()/2],&output,"Long 分卷-1518",||Ok(())).unwrap();
        } else {
            AesWrapper::decrypt_file(&parts[parts.len()/2],&output,"Long 分卷-1518").unwrap();
        }
        assert_eq!(fs::read(output).unwrap(),fs::read(fixtures().join("../multistream-1517").join(c["source"].as_str().unwrap())).unwrap(),"{c:?}");
    }
}

#[test]
fn aes1518_missing_head_middle_tail_and_late_authentication_are_distinct() {
    let original=fixtures().join("v2-资料.tar.gz.aes");
    for member in [0,3,usize::MAX] {
        let temp=tempfile::tempdir().unwrap(); let parts=split(&original,temp.path(),8191);
        let index=if member==usize::MAX {parts.len()-1}else{member};
        fs::remove_file(&parts[index]).unwrap();
        let output=temp.path().join("unpublished");
        let error=AesWrapper::decrypt_file(&parts[1],&output,"Long 分卷-1518").unwrap_err();
        assert!(format!("{error:#}").contains("[archive-inspection:incomplete]"),"{error:#}");
        assert!(!output.exists());
    }
    let temp=tempfile::tempdir().unwrap(); let parts=split(&original,temp.path(),8191);
    let last=parts.last().unwrap(); let mut tail=fs::read(last).unwrap(); *tail.last_mut().unwrap()^=1;fs::write(last,tail).unwrap();
    let output=temp.path().join("unpublished");
    let error=AesWrapper::decrypt_file(&parts[2],&output,"Long 分卷-1518").unwrap_err();
    assert!(format!("{error:#}").contains("[archive-inspection:damaged]"),"{error:#}");
    assert!(!output.exists());
    assert!(AesWrapper::verify_password(&parts[0],"Long 分卷-1518").unwrap_err().to_string().contains("后续分块"));
}

#[test]
fn aes1518_wrong_password_is_ambiguous_and_cancelled_output_is_removed() {
    let temp=tempfile::tempdir().unwrap(); let parts=split(&fixtures().join("v2-资料.tar.gz.aes"),temp.path(),8191);
    let output=temp.path().join("unpublished");
    let error=AesWrapper::decrypt_file(&parts[2],&output,"wrong").unwrap_err();
    assert!(error.to_string().contains("[archive-inspection:ambiguous-data]"));
    assert!(!output.exists());
    let mut count=0;
    let error=AesStreamV2::decrypt_file_cancellable(&parts[2],&output,"Long 分卷-1518",AesStreamKind::Generic,||{
        count+=1; anyhow::ensure!(count<4,"user cancelled"); Ok(())
    }).unwrap_err();
    assert!(error.to_string().contains("cancelled"));
    assert!(!output.exists());
}

#[tokio::test]
async fn aes1518_native_browse_preview_integrity_and_diagnosis_use_complete_container() {
    use super::{archive_browser,archive_preview,archive_diagnostics,universal_engine::UniversalCliEngine};
    let temp=tempfile::tempdir().unwrap();
    let parts=split(&fixtures().join("v2-资料.tar.gz.aes"),temp.path(),8191);
    let middle=&parts[parts.len()/2]; let password=Some("Long 分卷-1518");
    let listed=archive_browser::browse_archive(middle,password).await.unwrap();
    assert!(listed.encrypted && listed.entries.iter().all(|e|e.encrypted));
    assert_eq!(listed.total_files,3);assert_eq!(listed.total_compressed_size,fs::metadata(fixtures().join("v2-资料.tar.gz.aes")).unwrap().len());
    let text=archive_preview::preview_archive_text(middle,"末尾.txt",password).await.unwrap();
    assert!(text.content.ends_with("内容不能丢。\n"));
    UniversalCliEngine::test_integrity(middle,password).await.unwrap();
    let report=archive_diagnostics::diagnose_archive(middle,None,Default::default()).await.unwrap();
    assert!(report.encrypted); assert_eq!(report.status,"password_required");
    let tail=parts.last().unwrap();let mut bytes=fs::read(tail).unwrap();*bytes.last_mut().unwrap()^=1;fs::write(tail,bytes).unwrap();
    let report=archive_diagnostics::diagnose_archive(middle,password,Default::default()).await.unwrap();
    assert_eq!(report.status,"damaged","{report:?}");
    assert!(report.issues.iter().any(|e|e.detail.contains("后续分块")));
}

#[test]
fn aes1518_tar_creation_keeps_multi_source_empty_directory_layout() {
    let temp=tempfile::tempdir().unwrap();
    let source=temp.path().join("sources");fs::create_dir(&source).unwrap();
    fs::write(source.join("说明.txt"),b"actual content").unwrap();fs::create_dir(source.join("空目录")).unwrap();
    let output=temp.path().join("result.tar.aes");
    let sources=fs::read_dir(&source).unwrap().map(|e|e.unwrap().path().to_string_lossy().into_owned()).collect::<Vec<_>>();
    super::native_compression::aes::compress(&Runtime,None,"test",&sources,output.to_str().unwrap(),
        CompressionOptions {format:Some("tar.aes".into()),password:Some("Long 分卷-1518".into()),preserve_paths:Some(true),..Default::default()},
        super::native_compression::aes::AesCompressionKind::Tar).unwrap();
    let inner=super::aes_container::AuthenticatedInner::open(&output,Some("Long 分卷-1518"),||Ok(())).unwrap();
    let mut tar=tar::Archive::new(fs::File::open(&inner.path).unwrap());
    let names=tar.entries().unwrap().map(|e|e.unwrap().path().unwrap().to_string_lossy().into_owned()).collect::<Vec<_>>();
    assert!(names.iter().any(|p|p=="空目录" || p=="空目录/"),"{names:?}");
    assert!(names.iter().any(|p|p=="说明.txt"));
}
