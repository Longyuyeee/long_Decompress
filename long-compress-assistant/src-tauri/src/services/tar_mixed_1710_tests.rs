use super::{lzop::recovery,tar_lzo_prefix_175_tests::wrapped};
use sha2::{Digest,Sha256};
use std::{io::{Cursor,Read},path::{Path,PathBuf},sync::{atomic::{AtomicBool,Ordering},Arc}};
fn flag()->AtomicBool{AtomicBool::new(false)}
fn root()->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/tar-mixed-1710")}
fn base()->PathBuf{let p=Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/tar-mixed-1710");std::fs::create_dir_all(&p).unwrap();p}
fn manifest()->serde_json::Value{serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()}
fn sha(b:&[u8])->String{format!("{:x}",Sha256::digest(b))}
fn scan(p:&Path)->recovery::Report{recovery::scan(p,None,&flag(),|_,_|{}).unwrap()}
fn view(bytes:Vec<u8>)->anyhow::Result<Vec<(String,String)>>{
    let mut archive=tar::Archive::new(super::tar_pax::Reader::new(Cursor::new(bytes)));let mut out=Vec::new();
    for item in archive.entries()?{let mut entry=super::tar_compatibility::LogicalEntry::new(item?)?;let mut data=Vec::new();entry.read_to_end(&mut data)?;out.push((entry.path().to_string_lossy().replace('\\',"/"),sha(&data)));}
    super::tar_compatibility::finish_stream(archive.into_inner(),||Ok(()))?;Ok(out)
}
fn exact(output:&Path,entries:&[serde_json::Value]){
    for entry in entries{let path=output.join(entry["name"].as_str().unwrap());if entry["isDirectory"].as_bool().unwrap(){assert!(path.is_dir());}else{assert_eq!(sha(&std::fs::read(path).unwrap()),entry["sha256"].as_str().unwrap());}}
    assert_eq!(walkdir::WalkDir::new(output).into_iter().filter_map(Result::ok).filter(|p|p.file_type().is_file()).count(),entries.iter().filter(|e|!e["isDirectory"].as_bool().unwrap()).count());
}
fn no_stage(path:&Path){assert!(!std::fs::read_dir(path).unwrap().any(|p|p.unwrap().file_name().to_string_lossy().starts_with(".long-lzop-recovery")));}
struct Writer(Arc<AtomicBool>);
impl super::native_compression::CompressionRuntime for Writer{
    fn check_cancellation(&self)->anyhow::Result<()>{anyhow::ensure!(!self.0.load(Ordering::Relaxed),"cancelled");Ok(())}
    fn cancellation_flag(&self)->Arc<AtomicBool>{self.0.clone()}
    fn copy_buffer_size(&self)->usize{65536}
    fn emit_log(&self,_:&tauri::Window,_:&str,_:&str,_:crate::models::compression::TaskLogSeverity){}
    fn emit_progress(&self,_:&tauri::Window,_:&str,_:f32,_:Option<String>,_:u64,_:u64){}
}

#[tokio::test]
async fn mixed1710_independent_orders_browse_preview_extract_select_convert_and_verify_without_metadata_files(){
    let m=manifest();let work=base().join("full").join(uuid::Uuid::new_v4().to_string());std::fs::create_dir_all(&work).unwrap();let mut outputs=Vec::new();
    let runtime=super::tar_lzo_1658_tests::Runtime(super::io_buffer_pool::IOBufferPool::new(Default::default()));
    for case in m["cases"].as_array().unwrap(){let expected=case["entries"].as_array().unwrap();
        for (key,compressed)in[("tar",false),("archive",true)]{
            let path=root().join(case[key].as_str().unwrap());assert_eq!(sha(&std::fs::read(&path).unwrap()),case[if compressed{"sha256"}else{"tarSha256"}].as_str().unwrap());
            let list=super::archive_browser::browse_archive(&path,None).await.unwrap();assert_eq!(list.entries.len(),9);assert_eq!(list.total_files,7);
            for (actual,e)in list.entries.iter().zip(expected){assert_eq!(actual.path.trim_end_matches('/'),e["name"].as_str().unwrap());assert_eq!(actual.size,e["bytes"].as_u64().unwrap());}
            assert!(!list.entries.iter().any(|e|e.path.contains("@LongLink")||e.path.contains("PaxHeaders")));
            let long=expected[6]["name"].as_str().unwrap();assert!(super::archive_preview::preview_archive_text(&path,long,None).await.unwrap().content.contains("GNU long UTF-8 file name"));
            super::universal_engine::UniversalCliEngine::test_integrity(&path,None).await.unwrap();
            let out=work.join(format!("{}-{key}",case["name"].as_str().unwrap()));std::fs::create_dir_all(&out).unwrap();
            let decoder=if compressed{Some(super::codec_stream::decoder(&path,"lzo").unwrap())}else{None};
            super::native_extraction::tar::extract_inner(&runtime,None,"mixed1710",path.to_str().unwrap(),&out,decoder,&Default::default()).unwrap();exact(&out,expected);
            if compressed{assert!(scan(&path).complete);}
            if compressed&&case["name"]=="alternating"{
                let sources=std::fs::read_dir(&out).unwrap().map(|p|p.unwrap().path().to_string_lossy().into_owned()).collect::<Vec<_>>();let target=work.join("mixed-converted.zip");
                let options=crate::models::compression::CompressionOptions{format:Some("zip".into()),password:Some("mixed-1710".into()),preserve_paths:Some(true),..Default::default()};
                super::native_compression::zip::compress(&Writer(Arc::new(flag())),None,"1710",&sources,target.to_str().unwrap(),options).unwrap();
                assert!(super::compression_verification::verify_native(super::compression_format::CompressionRoute::Zip,&target,Some("mixed-1710"),||false).unwrap());
                let mut zip=zip_aes::ZipArchive::new(std::fs::File::open(target).unwrap()).unwrap();
                for e in expected.iter().filter(|e|!e["isDirectory"].as_bool().unwrap()){let mut file=zip.by_name_decrypt(e["name"].as_str().unwrap(),b"mixed-1710").unwrap();let mut data=Vec::new();file.read_to_end(&mut data).unwrap();assert_eq!(sha(&data),e["sha256"].as_str().unwrap());}
                let selected=work.join("selected");std::fs::create_dir_all(&selected).unwrap();let options=crate::models::compression::DecompressOptions{selected_entries:vec![long.into(),"after.txt".into()],preserve_paths:true,..Default::default()};
                super::native_extraction::tar::extract_inner(&runtime,None,"selected1710",path.to_str().unwrap(),&selected,Some(super::codec_stream::decoder(&path,"lzo").unwrap()),&options).unwrap();exact(&selected,&[expected[6].clone(),expected[7].clone()]);
                let diagnosis=super::archive_diagnostics::diagnose_archive(&path,None,Arc::new(flag())).await.unwrap();assert_eq!(diagnosis.status,"healthy");assert_eq!(diagnosis.recoverability,"not_needed");
            }
            outputs.push(serde_json::json!({"case":case["name"],"source":path,"outputDirectory":out,"entries":expected}));
        }
    }
    std::fs::write(base().join("full-outputs.json"),serde_json::to_vec_pretty(&outputs).unwrap()).unwrap();
}

#[tokio::test]
async fn mixed1710_bad_tail_checksum_and_end_restore_only_confirmed_mixed_entries(){
    let m=manifest();let work=base().join("prefix").join(uuid::Uuid::new_v4().to_string());std::fs::create_dir_all(&work).unwrap();let mut outputs=Vec::new();
    for case in m["cases"].as_array().unwrap(){let original=std::fs::read(root().join(case["archive"].as_str().unwrap())).unwrap();let expected=case["entries"].as_array().unwrap();
        for mode in["tail","checksum","end"]{let mut bytes=original.clone();if mode=="tail"{bytes.truncate(bytes.len()-400000)}else if mode=="checksum"{let at=bytes.len()-400000;bytes[at]^=1;}else{bytes.truncate(bytes.len()-2)}
            let path=work.join(format!("{}-{mode}.tar.lzo",case["name"].as_str().unwrap()));std::fs::write(&path,&bytes).unwrap();let report=scan(&path);let count=if mode=="end"{9}else{8};assert!(!report.complete);assert_eq!(report.entries.len(),count,"{case}: {mode}: {report:?}");
            for (entry,e)in report.entries.iter().zip(expected){assert_eq!(entry.name,e["name"].as_str().unwrap());assert_eq!(entry.sha256,e["sha256"].as_str().unwrap());}
            let selected=report.entries.iter().map(|e|e.id).collect::<Vec<_>>();let result=recovery::recover(&path,&work,&report.revision,&selected,None,&flag(),|_,_,_|{}).unwrap();assert!(result.partial);assert_eq!(result.recovered_directories,2);exact(Path::new(&result.output_directory),&expected[..count]);assert_eq!(std::fs::read(&path).unwrap(),bytes);no_stage(&work);
            if case["name"]=="alternating"&&mode=="tail"{let diagnosis=super::archive_diagnostics::diagnose_archive(&path,None,Arc::new(flag())).await.unwrap();assert_eq!(diagnosis.status,"partial_recovery");assert_eq!(diagnosis.total_files,6);std::fs::write(base().join("native.json"),serde_json::to_vec_pretty(&serde_json::json!({"source":path,"report":report,"result":result,"diagnosis":diagnosis})).unwrap()).unwrap();}
            outputs.push(serde_json::json!({"case":case["name"],"source":path,"outputDirectory":result.output_directory,"entries":&expected[..count]}));
        }
    }
    std::fs::write(base().join("recovery-outputs.json"),serde_json::to_vec_pretty(&outputs).unwrap()).unwrap();
}

#[test]
fn mixed1710_both_control_orders_require_complete_headers_payload_padding_and_following_entry(){
    let m=manifest();let temp=tempfile::tempdir().unwrap();let path=temp.path().join("boundary.tar.lzo");
    for case in m["cases"].as_array().unwrap(){let raw=std::fs::read(root().join(case["tar"].as_str().unwrap())).unwrap();let controls=case["records"].as_array().unwrap().iter().filter(|r|r["type"]=="L"||r["type"]=="x").take(2).collect::<Vec<_>>();let mut cuts=Vec::new();let mut end=0;
        for record in controls{let at=record["offset"].as_u64().unwrap()as usize;let size=tar::Header::from_byte_slice(&raw[at..at+512]).entry_size().unwrap()as usize;end=at+512+size.div_ceil(512)*512;cuts.extend([at+100,at+512+size-1,end-1]);}
        cuts.push(end+100);
        for cut in cuts{let data=raw[..cut].to_vec();assert!(view(data.clone()).is_err());std::fs::write(&path,wrapped(&data,137,true)).unwrap();let report=scan(&path);assert_eq!(report.entries.len(),1,"{cut}: {report:?}");assert!(!report.complete);}
        let mut orphan=raw[..end].to_vec();orphan.extend([0;1024]);assert!(view(orphan.clone()).unwrap_err().to_string().contains("缺少对应条目"));std::fs::write(&path,wrapped(&orphan,137,true)).unwrap();let report=scan(&path);assert_eq!(report.entries.len(),1);assert!(report.stop_reason.contains("缺少对应条目"));
    }
}

fn pax(fields:&[(&str,&[u8])])->Vec<u8>{let mut builder=tar::Builder::new(Vec::new());builder.append_pax_extensions(fields.iter().copied()).unwrap();let mut bytes=builder.into_inner().unwrap();bytes.truncate(bytes.len()-1024);bytes}
#[test]
fn mixed1710_conflicting_deleted_repeated_and_malformed_names_stop_before_the_mixed_member(){
    let m=manifest();let temp=tempfile::tempdir().unwrap();let path=temp.path().join("conflict.tar.lzo");
    for case in m["cases"].as_array().unwrap().iter().take(2){let raw=std::fs::read(root().join(case["tar"].as_str().unwrap())).unwrap();let records=case["records"].as_array().unwrap();let l=records.iter().find(|r|r["type"]=="L").unwrap()["offset"].as_u64().unwrap()as usize;let x=records.iter().find(|r|r["type"]=="x").unwrap()["offset"].as_u64().unwrap()as usize;
        let mut cases=Vec::new();for name in[b"different.txt".as_slice(),b"".as_slice()]{let mut data=raw.clone();data.splice(x..x+1024,pax(&[("path",name)]));cases.push(data);}
        let mut duplicate=raw.clone();duplicate.splice(l..l,raw[l..l+1024].iter().copied());cases.push(duplicate);
        let mut duplicate=raw.clone();duplicate.splice(x..x,raw[x..x+1024].iter().copied());cases.push(duplicate);
        let mut invalid=raw.clone();let size=tar::Header::from_byte_slice(&raw[l..l+512]).entry_size().unwrap()as usize;invalid[l+512..l+512+size].fill(b'a');cases.push(invalid);
        let mut padding=raw.clone();padding[l+1023]=1;cases.push(padding);
        for data in cases{assert!(view(data.clone()).is_err());std::fs::write(&path,wrapped(&data,137,true)).unwrap();let report=scan(&path);assert_eq!(report.entries.len(),1,"{report:?}");assert!(!report.complete);let result=recovery::recover(&path,temp.path(),&report.revision,&[0],None,&flag(),|_,_,_|{}).unwrap();assert_eq!(result.entries[0].name,"prefix.txt");no_stage(temp.path());}
    }
}

#[test]
fn mixed1710_numeric_gap_revision_recheck_and_active_copy_cancel_keep_source_and_clean_stage(){
    let original=std::fs::read(root().join("alternating.tar.lzo")).unwrap();let temp=tempfile::tempdir().unwrap();let mut paths=Vec::new();for(i,part)in original.chunks(100003).enumerate(){let path=temp.path().join(format!("mixed.tar.lzo.{:03}",i+1));std::fs::write(&path,part).unwrap();paths.push(path);}
    let withheld=std::fs::read(&paths[4]).unwrap();std::fs::remove_file(&paths[4]).unwrap();let path=paths.last().unwrap();let report=scan(path);assert_eq!(report.entries.len(),8);assert_eq!(report.available_volumes,4);assert!(report.ignored_volumes>0);
    let result=recovery::recover(path,temp.path(),&report.revision,&[1,2,3,6],None,&flag(),|_,_,_|{}).unwrap();assert_eq!(result.recovered_files,2);assert_eq!(result.recovered_directories,2);
    let cancelled=flag();let mut active=false;let error=recovery::recover(path,temp.path(),&report.revision,&[4],None,&cancelled,|phase,done,_|{if phase=="recovering"&&done>0{active=true;cancelled.store(true,Ordering::SeqCst)}}).unwrap_err();assert!(active);assert!(format!("{error:#}").contains("cancelled"));no_stage(temp.path());
    std::fs::write(&paths[4],withheld).unwrap();assert!(recovery::recover(path,temp.path(),&report.revision,&[0],None,&flag(),|_,_,_|{}).unwrap_err().to_string().contains("重新检查"));assert!(scan(path).complete);
    let joined=paths.iter().flat_map(|p|std::fs::read(p).unwrap()).collect::<Vec<_>>();assert_eq!(joined,original);
}

#[test]
fn mixed1710_zero_padded_gnu_payload_uses_one_logical_name_and_original_physical_copy_offsets(){
    let m=manifest();let case=m["cases"].as_array().unwrap().iter().find(|c|c["name"]=="padded").unwrap();let expected=case["entries"].as_array().unwrap();
    let raw=std::fs::read(root().join(case["tar"].as_str().unwrap())).unwrap();let actual=view(raw).unwrap();assert_eq!(actual.len(),9);
    for((name,digest),e)in actual.iter().zip(expected){assert_eq!(name.trim_end_matches('/'),e["name"].as_str().unwrap());assert_eq!(digest,e["sha256"].as_str().unwrap());}
    let archive=root().join(case["archive"].as_str().unwrap());assert!(scan(&archive).complete);let original=std::fs::read(&archive).unwrap();assert_eq!(sha(&original),case["sha256"].as_str().unwrap());
    let work=base().join("padded").join(uuid::Uuid::new_v4().to_string());std::fs::create_dir_all(&work).unwrap();let source=work.join("padded-tail.tar.lzo");std::fs::write(&source,&original[..original.len()-400000]).unwrap();
    let report=scan(&source);assert_eq!(report.entries.len(),8);let ids=report.entries.iter().map(|e|e.id).collect::<Vec<_>>();let result=recovery::recover(&source,&work,&report.revision,&ids,None,&flag(),|_,_,_|{}).unwrap();exact(Path::new(&result.output_directory),&expected[..8]);no_stage(&work);
    assert_eq!(std::fs::read(&archive).unwrap(),original);std::fs::write(base().join("padded-output.json"),serde_json::to_vec_pretty(&serde_json::json!({"case":"padded","source":source,"outputDirectory":result.output_directory,"entries":&expected[..8]})).unwrap()).unwrap();
}
