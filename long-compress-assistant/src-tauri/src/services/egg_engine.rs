//! One EGG native reader for browse, selection, password confirmation and extraction.
use anyhow::{Context,Result};
use std::{path::{Path,PathBuf},sync::{Arc,atomic::AtomicBool},time::Duration};
use crate::models::compression::{ArchiveBrowseResult,DecompressOptions};
use super::zip_password::ZipPasswordEncoding;
fn source_encoding(e:Option<ZipPasswordEncoding>)->Result<String>{
    if e!=Some(ZipPasswordEncoding::WindowsLocal){return Ok(e.map(|v|v.as_str()).unwrap_or("utf8").into());}
    #[cfg(windows)]let cp=unsafe{windows_sys::Win32::Globalization::GetACP()};
    #[cfg(not(windows))]let cp=0;
    Ok(match cp{65001=>"utf8",936=>"cp936",950=>"cp950",932=>"cp932",949=>"cp949",_=>anyhow::bail!("[egg-password-encoding] Windows 代码页 {cp} 不在 EGG 本版范围，请选择原包密码编码")}.into())
}
pub fn command_path()->Option<PathBuf>{
    let mut roots=vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources")];
    if let Ok(exe)=std::env::current_exe(){if let Some(p)=exe.parent(){roots.insert(0,p.join("resources"));roots.insert(0,p.to_path_buf());}}
    roots.into_iter().map(|p|p.join("egg-engine/long-egg.exe")).find(|p|p.is_file())
}
async fn run(op:&str,path:&Path,pwd:Option<&str>,output:Option<&Path>,selected:&[String],cancelled:Arc<AtomicBool>,preserve_timestamps:bool,encoding:Option<ZipPasswordEncoding>)->Result<serde_json::Value>{
    let group=super::egg_volumes::complete(path)?;
    let tool=command_path().context("[archive-inspection:engine-unavailable] 缺少随附 EGG 读取引擎，请修复安装")?;
    let mut cmd=crate::utils::process::async_command(tool);cmd.arg(op).arg(group.first);
    let input=serde_json::to_vec(&serde_json::json!({"password":pwd,"output":output,"selected":selected,"preserve_timestamps":preserve_timestamps,"encoding":source_encoding(encoding)?}))?;
    let result=crate::utils::process::cancellable_archive_output_with_stdin(&mut cmd,&cancelled,Duration::from_secs(24*60*60),Some(input)).await.context("EGG 引擎取消或运行失败")?;
    let response:serde_json::Value=serde_json::from_slice(&result.stdout).context("EGG 引擎没有返回有效结果")?;
    if let Some(e)=response.get("error").and_then(|v|v.as_str()){anyhow::bail!("{e}")}
    anyhow::ensure!(result.status.success(),"EGG 引擎异常退出：{}",String::from_utf8_lossy(&result.stderr));
    response.get("result").cloned().context("EGG 引擎缺少结果")
}
pub async fn browse(path:&Path,cancelled:Arc<AtomicBool>)->Result<ArchiveBrowseResult>{
    let d:ArchiveBrowseResult=serde_json::from_value(run("list",path,None,None,&[],cancelled,false,None).await?)?;
    for e in &d.entries{super::archive_entry_open::normalize_safe_entry_path(&e.path)?;}
    super::extraction_transaction::validate_resource_limits(path,d.entries.len(),d.total_uncompressed_size)?;Ok(d)
}
pub async fn plan(path:&Path,options:&DecompressOptions,cancelled:Arc<AtomicBool>)->Result<ArchiveBrowseResult>{
    let d=browse(path,cancelled.clone()).await?;let filter=super::extraction_transaction::compile_file_filter(options.file_filter.as_deref());
    let selected=d.entries.iter().filter(|e|super::extraction_transaction::matches_selected_entries(Path::new(&e.path),&options.selected_entries)&&super::extraction_transaction::matches_compiled_file_filter(Path::new(&e.path),&filter)).map(|e|e.path.clone()).collect::<Vec<_>>();
    anyhow::ensure!(!selected.is_empty()||!super::zip_selection::has_selection(options),"所选范围内没有 EGG 条目，请重新选择文件或修改筛选条件");
    serde_json::from_value(run("plan",path,None,None,&selected,cancelled,false,options.zip_password_encoding).await?).context("EGG 预检结果无效")
}
pub async fn requires_password(path:&Path,options:&DecompressOptions,cancelled:Arc<AtomicBool>)->Result<bool>{Ok(plan(path,options,cancelled).await?.encrypted)}
pub async fn verify(path:&Path,pwd:Option<&str>,options:&DecompressOptions,cancelled:Arc<AtomicBool>)->Result<()>{
    let d=plan(path,options,cancelled.clone()).await?;let selected=d.entries.iter().map(|e|e.path.clone()).collect::<Vec<_>>();
    run("verify",path,pwd,None,&selected,cancelled,false,options.zip_password_encoding).await?;Ok(())
}
pub async fn try_password(path:&Path,pwd:&str,options:&DecompressOptions,cancelled:Arc<AtomicBool>)->Result<bool>{
    if !requires_password(path,options,cancelled.clone()).await?{return Ok(false)}
    match verify(path,Some(pwd),options,cancelled).await{Ok(())=>Ok(true),Err(e)if e.to_string().contains("[egg-password]")||e.to_string().contains("[archive-inspection:ambiguous-data]")=>Ok(false),Err(e)=>Err(e)}
}
pub async fn extract(path:&Path,output:&Path,pwd:Option<&str>,options:&DecompressOptions,cancelled:Arc<AtomicBool>)->Result<()>{
    let d=plan(path,options,cancelled.clone()).await?;let selected=d.entries.iter().map(|e|e.path.clone()).collect::<Vec<_>>();
    run("extract",path,pwd,Some(output),&selected,cancelled,options.preserve_timestamps,options.zip_password_encoding).await?;Ok(())
}

#[cfg(test)]mod tests{
    use super::*;use sha2::Digest;
    fn root()->PathBuf{PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/egg-153")}
    fn flag()->Arc<AtomicBool>{Arc::new(AtomicBool::new(false))}
    fn sha(p:&Path)->String{format!("{:x}",sha2::Sha256::digest(std::fs::read(p).unwrap()))}
    #[tokio::test]async fn egg_genuine_matrix_matches_independent_payloads(){
        let m:serde_json::Value=serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap();
        for c in m["cases"].as_array().unwrap(){
            let p=root().join("upstream").join(c["archive"].as_str().unwrap());let d=browse(&p,flag()).await.unwrap();assert_eq!(d.format,"EGG");
            let out=tempfile::tempdir().unwrap();let pwd=c["password"].as_str();
            verify(&p,pwd,&Default::default(),flag()).await.unwrap();extract(&p,out.path(),pwd,&Default::default(),flag()).await.unwrap();
            for e in c["expected"].as_array().unwrap(){let f=out.path().join(e["path"].as_str().unwrap());assert_eq!(std::fs::metadata(&f).unwrap().len(),e["bytes"].as_u64().unwrap());assert_eq!(sha(&f),e["sha256"].as_str().unwrap());}
        }
    }
    #[tokio::test]async fn egg_any_member_has_one_group_and_directory(){
        let p=root().join("upstream/split_store.vol1.egg");let d=browse(&p,flag()).await.unwrap();
        for n in 1..=3 {let p=p.with_file_name(format!("split_store.vol{n}.egg"));let g=super::super::split_archive_detector::SplitArchiveDetector::detect_split_archive(&p).unwrap().unwrap();assert!(g.is_complete);assert_eq!(g.total_parts,3);assert_eq!(g.first_part.file_name().unwrap(),"split_store.vol1.egg");assert_eq!(browse(&p,flag()).await.unwrap(),d);}
    }
    #[tokio::test]async fn egg_original_alzip_unicode_names_and_cp936_password(){
        let base=root().join("original-alzip");let m:serde_json::Value=serde_json::from_slice(&std::fs::read(base.join("manifest.json")).unwrap()).unwrap();
        for c in m["cases"].as_array().unwrap(){let p=base.join(c["archive"].as_str().unwrap());let opts=DecompressOptions{zip_password_encoding:Some(ZipPasswordEncoding::Cp936),..Default::default()};
            let d=browse(&p,flag()).await.unwrap();assert!(d.entries.iter().any(|e|e.path=="说明 文件.txt"));
            let out=tempfile::tempdir().unwrap();extract(&p,out.path(),c["password"].as_str(),&opts,flag()).await.unwrap();
            for e in c["expected"].as_array().unwrap(){assert_eq!(sha(&out.path().join(e["path"].as_str().unwrap())),e["sha256"].as_str().unwrap());}assert!(out.path().join("空目录").is_dir());
        }
        let p=base.join("native-cn-key.egg");assert!(!try_password(&p,"中文密码123!",&Default::default(),flag()).await.unwrap());
        let opts=DecompressOptions{zip_password_encoding:Some(ZipPasswordEncoding::Cp936),..Default::default()};assert!(try_password(&p,"中文密码123!",&opts,flag()).await.unwrap());
    }
    fn copies(dest:&Path)->Vec<PathBuf>{(1..=3).map(|n|{let name=format!("split_store.vol{n}.egg");let p=dest.join(&name);std::fs::copy(root().join("upstream").join(name),&p).unwrap();p}).collect()}
    #[tokio::test]async fn egg_missing_first_middle_final_precede_password(){
        for absent in 0..3{let dir=tempfile::tempdir().unwrap();let p=copies(dir.path());std::fs::remove_file(&p[absent]).unwrap();let source=&p[if absent==0{1}else{0}];let e=requires_password(source,&Default::default(),flag()).await.unwrap_err();assert!(e.to_string().contains("[archive-inspection:incomplete]"),"{e}");let r=super::super::archive_diagnostics::diagnose_archive(source,None,flag()).await.unwrap();assert_eq!(r.status,"missing_volume");std::fs::copy(root().join("upstream").join(p[absent].file_name().unwrap()),&p[absent]).unwrap();verify(source,None,&Default::default(),flag()).await.unwrap();}
    }
    #[tokio::test]async fn egg_mixed_links_version_and_truncation_have_final_causes(){
        for change in [0,1,2,3]{let dir=tempfile::tempdir().unwrap();let p=copies(dir.path());let mut b=std::fs::read(&p[1]).unwrap();match change{0=>b[6]^=1,1=>b[21]^=1,2=>b[4]=17,_=>b.truncate(12)}std::fs::write(&p[1],b).unwrap();let r=super::super::archive_diagnostics::diagnose_archive(&p[0],Some("irrelevant"),flag()).await.unwrap();assert!(matches!(r.status.as_str(),"damaged"|"unsupported_feature"),"{r:?}");assert!(!r.integrity_tested);}
    }
    #[tokio::test]async fn egg_unsupported_complex_features_never_request_password(){
        for name in ["azo.egg","solid.egg","aes128.egg","aes256.egg","lea128.egg","lea256.egg"]{let p=root().join("upstream").join(name);let r=super::super::archive_diagnostics::diagnose_archive(&p,None,flag()).await.unwrap();assert_eq!(r.status,"unsupported_feature","{name}: {r:?}");assert!(!r.integrity_tested);assert!(try_password(&p,"test1234",&Default::default(),flag()).await.unwrap_err().to_string().contains("unsupported"));}
    }
    #[tokio::test]async fn egg_password_is_full_content_not_short_header_or_plain_success(){
        let p=root().join("upstream/zip20.egg");assert!(try_password(&p,"test1234",&Default::default(),flag()).await.unwrap());assert!(!try_password(&p,"wrong",&Default::default(),flag()).await.unwrap());assert!(!try_password(&root().join("upstream/store.egg"),"anything",&Default::default(),flag()).await.unwrap());
        let dir=tempfile::tempdir().unwrap();let bad=dir.path().join("damaged.egg");let mut b=std::fs::read(&p).unwrap();let n=b.len();b[n-8]^=9;std::fs::write(&bad,b).unwrap();assert!(!try_password(&bad,"test1234",&Default::default(),flag()).await.unwrap());
        let r=super::super::archive_diagnostics::diagnose_archive(&bad,Some("test1234"),flag()).await.unwrap();assert_eq!(r.status,"ambiguous_data");
    }
    #[tokio::test]async fn egg_selection_filter_and_cancel_preserve_scope(){
        let p=root().join("upstream/store5.egg");let d=browse(&p,flag()).await.unwrap();let e=d.entries.iter().find(|e|!e.is_dir&&e.size>0).unwrap();
        let opts=DecompressOptions{selected_entries:vec![e.path.clone()],..Default::default()};let out=tempfile::tempdir().unwrap();extract(&p,out.path(),None,&opts,flag()).await.unwrap();assert!(out.path().join(&e.path).is_file());assert_eq!(plan(&p,&opts,flag()).await.unwrap().entries.len(),1);
        let opts=DecompressOptions{selected_entries:vec!["missing".into()],..Default::default()};assert!(plan(&p,&opts,flag()).await.is_err());
        let cancelled=Arc::new(AtomicBool::new(true));assert!(browse(&p,cancelled).await.unwrap_err().to_string().contains("取消"));
    }
}
