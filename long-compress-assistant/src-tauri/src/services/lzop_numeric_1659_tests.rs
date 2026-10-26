use super::{archive_browser, archive_diagnostics, archive_preview, codec_stream, lzop, universal_engine::UniversalCliEngine, split_archive_detector::SplitArchiveDetector};
use sha2::{Digest, Sha256};
use std::{io::Read, path::{Path, PathBuf}, sync::{atomic::AtomicBool, Arc}};
fn fixtures(dir: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives").join(dir) }
fn slices(temp: &Path, name: &str, bytes: &[u8], cuts: &[usize], width: usize) -> Vec<PathBuf> {
    let mut boundaries = vec![0]; boundaries.extend_from_slice(cuts); boundaries.push(bytes.len());
    boundaries.windows(2).enumerate().map(|(i, b)| {
        let p = temp.join(format!("{name}.{:0width$}", i + 1));
        std::fs::write(&p, &bytes[b[0]..b[1]]).unwrap(); p
    }).collect()
}
fn sha(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }
struct Runtime(super::io_buffer_pool::IOBufferPool);
impl super::native_extraction::ExtractionRuntime for Runtime {
    fn check_cancellation(&self) -> anyhow::Result<()> {
        Ok(())
    }
    fn buffer_pool(&self) -> &super::io_buffer_pool::IOBufferPool {
        &self.0
    }
    fn copy_buffer_size(&self) -> usize {
        64 * 1024
    }
    fn normalized_archive_path(&self, p: &Path, _: bool) -> Option<PathBuf> {
        Some(
            p.components()
                .filter(|c| matches!(c, std::path::Component::Normal(_)))
                .collect(),
        )
    }
    fn emit_log(
        &self,
        _: &tauri::Window,
        _: &str,
        _: &str,
        _: crate::models::compression::TaskLogSeverity,
    ) {
    }
    fn emit_progress(&self, _: &tauri::Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {
    }
}

#[tokio::test]
async fn lzop1659_headers_blocks_footer_and_every_member_restore_original_bytes() {
    for (dir, name) in [("lzop-read-1657", "crc.txt.lzo"), ("lzop-read-1657", "stored.bin.lzo"), ("tar-lzo-read-1658", "directory-1.tar.lzo")] {
        let original = fixtures(dir).join(name); let bytes = std::fs::read(&original).unwrap();
        let start = 38 + bytes[33] as usize;
        let temp = tempfile::tempdir().unwrap();
        // Header magic, first block length/checksum, payload and 4-byte end marker straddle boundaries.
        let parts = slices(temp.path(), name, &bytes, &[4, start+2, start+9, bytes.len()/2, bytes.len()-2], 4);
        let mut expected = Vec::new(); codec_stream::decoder(&original,"lzo").unwrap().read_to_end(&mut expected).unwrap();
        for selected in &parts {
            assert_eq!(SplitArchiveDetector::complete_source(selected).unwrap(),parts[0]);
            let mut raw=Vec::new();lzop::open(selected).unwrap().read_to_end(&mut raw).unwrap();assert_eq!(sha(&raw),sha(&expected));
            UniversalCliEngine::test_integrity(selected,None).await.unwrap();
        }
        let browse=archive_browser::browse_archive(parts.last().unwrap(),None).await.unwrap();
        assert_eq!(browse.format,if name.ends_with("tar.lzo") {"TAR.LZO"} else {"LZOP"});
        if name.ends_with("tar.lzo") {
            let runtime=Runtime(super::io_buffer_pool::IOBufferPool::new(Default::default()));let out=tempfile::tempdir().unwrap();
            super::native_extraction::tar::extract_inner(&runtime,None,"lzop1659",parts[0].to_str().unwrap(),out.path(),Some(codec_stream::decoder(parts.last().unwrap(),"lzo").unwrap()),&Default::default()).unwrap();
            let manifest:serde_json::Value=serde_json::from_slice(&std::fs::read(fixtures(dir).join("manifest.json")).unwrap()).unwrap();
            let case=manifest["cases"].as_array().unwrap().iter().find(|c|c["archive"]==name).unwrap();
            for e in case["expected"].as_array().unwrap(){assert_eq!(sha(&std::fs::read(out.path().join(e["path"].as_str().unwrap())).unwrap()),e["sha256"].as_str().unwrap());}
            assert!(out.path().join("empty-dir").is_dir());
        } else { assert_eq!(browse.entries[0].path,name.trim_end_matches(".lzo")); }
    }
}
#[tokio::test]
async fn lzop1659_non_first_text_image_prefix_and_cancellation() {
    for (dir,name,entry,image) in [("lzop-read-1657","picture.png.lzo","picture.png",true),("lzop-read-1657","large.txt.lzo","large.txt",false),("tar-lzo-read-1658","directory-1.tar.lzo","notes/large.txt",false)] {
        let bytes=std::fs::read(fixtures(dir).join(name)).unwrap();let temp=tempfile::tempdir().unwrap();
        let parts=slices(temp.path(),name,&bytes,&[bytes.len()/2,bytes.len()-1],3);let selected=&parts[1];
        if image {let p=archive_preview::preview_archive_image(selected,entry,None).await.unwrap();assert_eq!((p.width,p.height),(32,32));}
        else {let p=archive_preview::preview_archive_text(selected,entry,None).await.unwrap();assert!(p.truncated);assert_eq!(p.content.len(),1024*1024);}
        assert!(lzop::verify(selected,||true).unwrap_err().to_string().contains("取消"));
        let d=archive_diagnostics::diagnose_archive(selected,None,Arc::new(AtomicBool::new(false))).await.unwrap();assert_eq!(d.status,"healthy");assert!(d.integrity_tested);
        if name.starts_with("large") {let b=archive_browser::browse_archive(selected,None).await.unwrap();assert_eq!(b.entries[0].size_known,Some(false));}
    }
    let bytes=std::fs::read(fixtures("tar-lzo-read-1658").join("directory-1.tar.lzo")).unwrap();let temp=tempfile::tempdir().unwrap();let parts=slices(temp.path(),"目录.tar.lzo",&bytes,&[4,1000],3);
    let text=archive_preview::preview_archive_text(&parts[2],"notes/说明 [1].txt",None).await.unwrap();assert!(text.content.contains("收到 TAR.LZO"));
    let image=archive_preview::preview_archive_image(&parts[1],"gallery/cover.png",None).await.unwrap();assert_eq!((image.width,image.height),(32,32));
}
#[tokio::test]
async fn lzop1659_gaps_missing_tail_damage_and_corrected_retry_are_explicit() {
    for (dir,name) in [("lzop-read-1657","crc.txt.lzo"),("tar-lzo-read-1658","directory-1.tar.lzo")] {
        let bytes=std::fs::read(fixtures(dir).join(name)).unwrap();let temp=tempfile::tempdir().unwrap();let cuts=[bytes.len()/3,bytes.len()-2];let parts=slices(temp.path(),name,&bytes,&cuts,3);
        for missing in [0,1] {
            let saved=std::fs::read(&parts[missing]).unwrap();std::fs::remove_file(&parts[missing]).unwrap();
            let error=format!("{:#}",lzop::open(&parts[2]).err().unwrap());assert!(error.contains("[archive-inspection:incomplete]"),"{error}");assert!(error.contains(parts[missing].file_name().unwrap().to_str().unwrap()));
            let d=archive_diagnostics::diagnose_archive(&parts[2],None,Arc::new(AtomicBool::new(false))).await.unwrap();assert_ne!(d.status,"healthy");
            std::fs::write(&parts[missing],saved).unwrap();UniversalCliEngine::test_integrity(&parts[2],None).await.unwrap();
        }
        let tail=std::fs::read(&parts[2]).unwrap();std::fs::remove_file(&parts[2]).unwrap();
        let error=format!("{:#}",UniversalCliEngine::test_integrity(&parts[1],None).await.unwrap_err());assert!(error.contains("可能缺少后续卷"),"{error}");assert!(error.contains("总卷数无法确认"));
        std::fs::write(&parts[2],&tail[..1]).unwrap();assert!(UniversalCliEngine::test_integrity(&parts[1],None).await.is_err());
        std::fs::write(&parts[2],&tail).unwrap();UniversalCliEngine::test_integrity(&parts[1],None).await.unwrap();
        let duplicate=temp.path().join(format!("{name}.0001"));std::fs::copy(&parts[0],&duplicate).unwrap();assert!(format!("{:#}",lzop::open(&parts[1]).err().unwrap()).contains("重复"));std::fs::remove_file(duplicate).unwrap();
        let mut bad=std::fs::read(&parts[1]).unwrap();bad[0]^=1;std::fs::write(&parts[1],bad).unwrap();assert!(UniversalCliEngine::test_integrity(&parts[0],None).await.is_err());
    }
}
