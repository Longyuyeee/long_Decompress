use super::{lzop::recovery, tar_lzo_prefix_175_tests::wrapped, tar_pax};
use sha2::{Digest,Sha256};
use std::{io::{Cursor,Read}, path::{Path,PathBuf}, sync::{atomic::{AtomicBool,Ordering},Arc}};
fn flag() -> AtomicBool { AtomicBool::new(false) }
fn root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/tar-pax-179") }
fn base() -> PathBuf { let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/tar-pax-179"); std::fs::create_dir_all(&p).unwrap(); p }
fn manifest() -> serde_json::Value { serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap() }
fn sha(data:&[u8]) -> String { format!("{:x}",Sha256::digest(data)) }
fn exact(output:&Path, expected:&[serde_json::Value]) {
    for e in expected { let p=output.join(e["name"].as_str().unwrap());
        if e["isDirectory"].as_bool().unwrap() { assert!(p.is_dir()); }
        else { assert_eq!(sha(&std::fs::read(&p).unwrap()),e["sha256"].as_str().unwrap(),"{}",p.display()); }
    }
    assert_eq!(walkdir::WalkDir::new(output).into_iter().filter_map(Result::ok).filter(|e|e.file_type().is_file()).count(),expected.iter().filter(|e|!e["isDirectory"].as_bool().unwrap()).count());
}
fn scan(path:&Path) -> recovery::Report { recovery::scan(path,None,&flag(),|_,_|{}).unwrap() }
fn view(bytes:Vec<u8>) -> anyhow::Result<Vec<(String,Vec<u8>)>> {
    let mut archive=tar::Archive::new(tar_pax::Reader::new(Cursor::new(bytes))); let mut out=Vec::new();
    for entry in archive.entries()? { let mut e=super::tar_compatibility::LogicalEntry::new(entry?)?; let mut bytes=Vec::new(); e.read_to_end(&mut bytes)?; out.push((e.path().to_string_lossy().replace('\\',"/"),bytes)); }
    super::tar_compatibility::finish_stream(archive.into_inner(),||Ok(()))?; Ok(out)
}
struct Writer(Arc<AtomicBool>);
impl super::native_compression::CompressionRuntime for Writer {
    fn check_cancellation(&self)->anyhow::Result<()> { anyhow::ensure!(!self.0.load(Ordering::Relaxed),"cancelled"); Ok(()) }
    fn cancellation_flag(&self)->Arc<AtomicBool> { self.0.clone() }
    fn copy_buffer_size(&self)->usize { 65536 }
    fn emit_log(&self,_:&tauri::Window,_:&str,_:&str,_:crate::models::compression::TaskLogSeverity) {}
    fn emit_progress(&self,_:&tauri::Window,_:&str,_:f32,_:Option<String>,_:u64,_:u64) {}
}

#[tokio::test]
async fn tarpax179_complete_originals_and_explicit_state_browse_preview_extract_convert_and_diagnose() {
    let manifest=manifest(); let run=base().join("complete").join(uuid::Uuid::new_v4().to_string()); std::fs::create_dir_all(&run).unwrap();
    let runtime=super::tar_lzo_1658_tests::Runtime(super::io_buffer_pool::IOBufferPool::new(Default::default())); let mut evidence=Vec::new();
    for case in manifest["cases"].as_array().unwrap() {
        let expected=case["entries"].as_array().unwrap();
        for (key,wrapped) in [("tar",false),("archive",true)] {
            let path=root().join(case[key].as_str().unwrap());
            assert_eq!(sha(&std::fs::read(&path).unwrap()),case[if wrapped {"sha256"} else {"tarSha256"}].as_str().unwrap());
            let list=super::archive_browser::browse_archive(&path,None).await.unwrap();
            assert_eq!(list.entries.len(),expected.len(),"{path:?}");
            for (item,e) in list.entries.iter().zip(expected) {
                assert_eq!(item.path.trim_end_matches('/'),e["name"].as_str().unwrap()); assert_eq!(item.size,e["bytes"].as_u64().unwrap());
                let actual=chrono::DateTime::parse_from_rfc3339(item.modified.as_ref().unwrap()).unwrap();
                assert!((actual.timestamp_micros() as f64/1e6-e["mtime"].as_f64().unwrap()).abs()<0.001);
            }
            super::universal_engine::UniversalCliEngine::test_integrity(&path,None).await.unwrap();
            let preview=super::archive_preview::preview_archive_text(&path,if case["name"]=="state" {"local/说明.txt"} else {"说明.txt"},None).await.unwrap(); assert!(!preview.content.is_empty());
            let out=run.join(format!("{}-{key}",case["name"].as_str().unwrap())); std::fs::create_dir_all(&out).unwrap();
            let options=crate::models::compression::DecompressOptions {preserve_paths:true,preserve_timestamps:true,..Default::default()};
            let decoder=if wrapped {Some(super::codec_stream::decoder(&path,"lzo").unwrap())} else {None};
            super::native_extraction::tar::extract_inner(&runtime,None,"pax179",path.to_str().unwrap(),&out,decoder,&options).unwrap(); exact(&out,expected);
            // Same extraction/rebuild engines used by convert_archive; no Window
            // or desktop harness is constructed for this native continuation.
            if wrapped {
                let sources=std::fs::read_dir(&out).unwrap().map(|p|p.unwrap().path().to_string_lossy().into_owned()).collect::<Vec<_>>();
                let target=run.join(format!("{}-converted.zip",case["name"].as_str().unwrap()));
                let options=crate::models::compression::CompressionOptions {format:Some("zip".into()),password:Some("pax-179".into()),preserve_paths:Some(true),..Default::default()};
                super::native_compression::zip::compress(&Writer(Arc::new(flag())),None,"179",&sources,target.to_str().unwrap(),options).unwrap();
                super::compression_verification::verify_native(super::compression_format::CompressionRoute::Zip,&target,Some("pax-179"),||false).unwrap();
                let mut zip=zip_aes::ZipArchive::new(std::fs::File::open(&target).unwrap()).unwrap();
                for e in expected.iter().filter(|e|!e["isDirectory"].as_bool().unwrap()) {
                    let mut entry=zip.by_name_decrypt(e["name"].as_str().unwrap(),b"pax-179").unwrap(); let mut bytes=Vec::new();entry.read_to_end(&mut bytes).unwrap();assert_eq!(sha(&bytes),e["sha256"].as_str().unwrap());
                }
                let diagnosis=super::archive_diagnostics::diagnose_archive(&path,None,Arc::new(flag())).await.unwrap(); assert_eq!(diagnosis.status,"healthy"); assert_eq!(diagnosis.recoverability,"not_needed");
                assert!(scan(&path).complete);
            }
            evidence.push(serde_json::json!({"case":case["name"],"source":path,"outputDirectory":out,"entries":expected}));
        }
    }
    std::fs::write(base().join("full-outputs.json"),serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
}

#[tokio::test]
async fn tarpax179_checked_prefix_results_and_diagnosis_keep_global_names_without_metadata_files() {
    let manifest=manifest(); let work=base().join("prefix").join(uuid::Uuid::new_v4().to_string()); std::fs::create_dir_all(&work).unwrap(); let mut outputs=Vec::new();
    for case in manifest["cases"].as_array().unwrap() {
        let original=std::fs::read(root().join(case["archive"].as_str().unwrap())).unwrap();
        for mode in ["tail","checksum","end"] {
            let mut bytes=original.clone(); if mode=="tail" {bytes.truncate(bytes.len()-400000)} else if mode=="checksum" {let at=bytes.len()-400000; bytes[at]^=1;} else {bytes.truncate(bytes.len()-2)}
            let path=work.join(format!("{}-{mode}.tar.lzo",case["name"].as_str().unwrap())); std::fs::write(&path,&bytes).unwrap(); let report=scan(&path);
            let expected=case["entries"].as_array().unwrap(); let count=expected.len()-usize::from(mode!="end");
            assert!(!report.complete); assert_eq!(report.entries.len(),count,"{case}: {mode}: {report:?}");
            for (entry,e) in report.entries.iter().zip(expected) {assert_eq!(entry.name,e["name"].as_str().unwrap());assert_eq!(entry.sha256,e["sha256"].as_str().unwrap());}
            let ids=report.entries.iter().map(|e|e.id).collect::<Vec<_>>(); let result=recovery::recover(&path,&work,&report.revision,&ids,None,&flag(),|_,_,_|{}).unwrap();
            assert!(result.partial); exact(Path::new(&result.output_directory),&expected[..count]); assert_eq!(std::fs::read(&path).unwrap(),bytes);
            outputs.push(serde_json::json!({"case":case["name"],"source":path,"outputDirectory":result.output_directory,"entries":&expected[..count]}));
            if case["name"]=="state" && mode=="tail" {
                let diagnosis=super::archive_diagnostics::diagnose_archive(&path,None,Arc::new(flag())).await.unwrap(); assert_eq!(diagnosis.status,"partial_recovery");
                std::fs::write(base().join("native.json"),serde_json::to_vec_pretty(&serde_json::json!({"source":path,"report":report,"result":result,"diagnosis":diagnosis})).unwrap()).unwrap();
            }
        }
    }
    std::fs::write(base().join("recovery-outputs.json"),serde_json::to_vec_pretty(&outputs).unwrap()).unwrap();
}

#[test]
fn tarpax179_metadata_boundaries_are_physical_and_partial_global_records_never_apply() {
    let bytes=std::fs::read(root().join("state.tar")).unwrap(); let temp=tempfile::tempdir().unwrap(); let path=temp.path().join("boundary.tar.lzo");
    for cut in [1100,1600,2047,2100,2559] {
        let partial=bytes[..cut].to_vec(); assert!(view(partial.clone()).is_err());
        std::fs::write(&path,wrapped(&partial,137,true)).unwrap(); let report=scan(&path); assert!(!report.complete); assert_eq!(report.entries.len(),1,"{cut}: {report:?}"); assert_eq!(report.entries[0].name,"prefix.txt");
    }
    // First global size is 700; next global/x controls are shorter and use their
    // own physical sizes. Full listing proves none swallowed the following header.
    assert_eq!(view(bytes).unwrap().len(),9);
}

#[test]
fn tarpax179_unknown_global_and_malformed_attributes_stop_both_views_at_confirmed_prefix() {
    let original=std::fs::read(root().join("state.tar")).unwrap(); let temp=tempfile::tempdir().unwrap(); let path=temp.path().join("bad.tar.lzo");
    let mut cases=Vec::new();
    let mut unknown=original.clone(); let at=unknown.windows(7).position(|w|w==b"comment").unwrap(); unknown[at..at+7].copy_from_slice(b"unknown"); cases.push(unknown);
    let mut sparse=original.clone(); sparse[at..at+7].copy_from_slice(b"GNU.map"); cases.push(sparse);
    let mut bad=original.clone(); bad[1536]=b'X'; cases.push(bad);
    let mut padding=original.clone(); padding[2047]=1; cases.push(padding);
    let mut newline=original.clone(); let value=at+8; newline[value]=b'\n'; cases.push(newline);
    for bytes in cases { assert!(view(bytes.clone()).is_err()); std::fs::write(&path,wrapped(&bytes,137,true)).unwrap(); let report=scan(&path); assert_eq!(report.entries.len(),1); assert!(!report.complete); }
}

#[test]
fn tarpax179_numeric_gap_and_replaced_source_require_recheck_and_cancel_does_not_publish() {
    let bytes=std::fs::read(root().join("state.tar.lzo")).unwrap(); let temp=tempfile::tempdir().unwrap(); let mut paths=Vec::new();
    for (i,part) in bytes.chunks(100003).enumerate() {let p=temp.path().join(format!("state.tar.lzo.{:03}",i+1));std::fs::write(&p,part).unwrap();paths.push(p);}
    let withheld=std::fs::read(&paths[4]).unwrap();std::fs::remove_file(&paths[4]).unwrap();let path=paths.last().unwrap();let report=scan(path);assert_eq!(report.entries.len(),8);assert!(report.ignored_volumes>0);
    let cancelled=AtomicBool::new(true);assert!(recovery::recover(path,temp.path(),&report.revision,&[0],None,&cancelled,|_,_,_|{}).is_err());
    std::fs::write(&paths[4],withheld).unwrap();assert!(recovery::recover(path,temp.path(),&report.revision,&[0],None,&flag(),|_,_,_|{}).unwrap_err().to_string().contains("重新检查"));assert!(scan(path).complete);
    assert!(!std::fs::read_dir(temp.path()).unwrap().any(|p|p.unwrap().file_name().to_string_lossy().starts_with(".long-lzop-recovery")));
}

#[test]
fn tarpax179_selected_extraction_and_preflight_use_effective_names_not_physical_headers() {
    let source=root().join("state.tar"); let temp=tempfile::tempdir().unwrap();
    let runtime=super::tar_lzo_1658_tests::Runtime(super::io_buffer_pool::IOBufferPool::new(Default::default()));
    let options=crate::models::compression::DecompressOptions {preserve_paths:true,selected_entries:vec!["local/说明.txt".into(),"empty".into()],..Default::default()};
    let plan=super::tar_compatibility::plan(&source,super::compression_format::CompressionRoute::Tar,&options,||Ok(())).unwrap();
    assert_eq!(plan.entries,2);assert_eq!(plan.logical_bytes,19);
    super::native_extraction::tar::extract_inner(&runtime,None,"selected179",source.to_str().unwrap(),temp.path(),None,&options).unwrap();
    let m=manifest(); let all=m["cases"][0]["entries"].as_array().unwrap();let selected=all.iter().filter(|e|e["name"]=="local/说明.txt"||e["name"]=="empty").cloned().collect::<Vec<_>>();
    exact(temp.path(),&selected); assert!(!temp.path().join("raw-local").exists());assert!(!temp.path().join("global").exists());
}
