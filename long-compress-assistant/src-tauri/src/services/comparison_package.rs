//! Comparison references reuse ZIP editing; decoded inputs live only for one save.
use super::{source_comparison as comparison, zip_edit as edit};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, io::{Read, Write}, path::Path, sync::{Arc, atomic::{AtomicBool, Ordering}}};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Reference { pub source: comparison::SourceRef, pub path: String, #[serde(default,skip_serializing_if="Option::is_none")] pub encoding: Option<super::zip_password::ZipPasswordEncoding> }
#[derive(Clone, Default, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Credential { pub path: String, #[serde(default)] pub password: String, pub encoding: Option<super::zip_password::ZipPasswordEncoding> }
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Part { pub source: comparison::SourceRef, pub paths: Option<Vec<String>>, pub encoding: Option<super::zip_password::ZipPasswordEncoding> }
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Prepared { pub source_path: String, pub inputs: Vec<edit::Input> }
fn check(flag:&Arc<AtomicBool>)->Result<()> {
    super::task_control::wait_if_paused(flag);
    ensure!(!flag.load(Ordering::SeqCst),"ZIP_EDIT_CANCELLED：已停止组包，选择与原件保留"); Ok(())
}
fn current(reference:&comparison::SourceRef,flag:Arc<AtomicBool>)->Result<comparison::Source>{
    let source=comparison::read(Path::new(&reference.path),flag,|_|{})?;
    ensure!(source.source.kind==reference.kind && source.source.revision==reference.revision,"组包来源已变化，请重新读取并确认该来源；修改清单保留：{}",reference.path);
    Ok(source)
}
fn entry<'a>(source:&'a comparison::Source,reference:&Reference,input:&edit::Input)->Result<&'a comparison::Entry>{
    let entry=source.entries.iter().find(|e|e.path==reference.path&&(e.selectable||e.directory)).context("组包所选路径已缺失，或不再是可用文件/空目录；请调整清单")?;
    ensure!(entry.directory==input.is_dir&&entry.size==input.size&&input.source_revision==reference.source.revision,"组包来源类型、大小或版本不一致，请重新读取并确认");
    Ok(entry)
}
pub fn check_inputs(inputs:&[edit::Input])->HashMap<String,std::result::Result<String,String>>{
    let mut result=HashMap::new();
    for input in inputs {if let Some(reference)=&input.comparison {if !result.contains_key(&reference.source.path){
        let checked=(||->Result<String>{let source=current(&reference.source,Arc::new(AtomicBool::new(false)))?;
            for input in inputs {if let Some(other)=&input.comparison {if other.source.path==reference.source.path {ensure!(other.source.revision==source.source.revision,"同一来源包含不同版本");entry(&source,other,input)?;}}}Ok(reference.source.revision.clone())})();
        result.insert(reference.source.path.clone(),checked.map_err(|e|format!("{e:#}")));
    }}}result
}
pub fn prepare(parts:&[Part],directory:&Path,flag:Arc<AtomicBool>)->Result<Prepared>{
    ensure!(!parts.is_empty()&&parts.len()<=3,"请选择基础内容及需要加入的来源");
    let mut inputs=Vec::new();
    for part in parts {
        let source=current(&part.source,flag.clone())?;
        if let Some(paths)=&part.paths {ensure!(paths.len()<=2000&&serde_json::to_string(paths)?.len()<=60000,"所选范围过大，请分批组包");for path in paths {ensure!(source.entries.iter().any(|e|&e.path==path&&e.selectable),"所选路径不可用：{path}");}}
        for e in source.entries.iter().filter(|e|part.paths.as_ref().is_none_or(|paths|paths.contains(&e.path)||e.directory&&paths.iter().any(|path|path.starts_with(&(e.path.clone()+"/"))))) {
            check(&flag)?;
            inputs.push(edit::Input{source_path:part.source.path.clone(),target_path:edit::normalized_path(&e.path,e.directory)?,is_dir:e.directory,size:e.size,source_revision:source.source.revision.clone(),comparison:Some(Reference{source:source.source.clone(),path:e.path.clone(),encoding:part.encoding})});
            ensure!(inputs.len()<=10000,"基础与补充内容合计超过 10000 项，请改用仅所选或缩小范围");
        }
    }
    ensure!(!inputs.is_empty(),"没有可组包的文件或空目录，请重新选择");check(&flag)?;
    std::fs::create_dir_all(directory)?;
    let path=directory.join(format!("所选资料-{}.zip",uuid::Uuid::new_v4()));
    zip_aes::ZipWriter::new(File::create(&path)?).finish()?;
    Ok(Prepared{source_path:path.to_string_lossy().into_owned(),inputs})
}
fn sources(inputs:&[edit::Input],flag:Arc<AtomicBool>)->Result<HashMap<String,comparison::Source>>{
    let mut result=HashMap::new();
    for input in inputs {if let Some(reference)=&input.comparison {
        if !result.contains_key(&reference.source.path){result.insert(reference.source.path.clone(),current(&reference.source,flag.clone())?);}
        let source=&result[&reference.source.path];
        ensure!(source.source.revision==reference.source.revision,"同一组包来源包含不同版本，请重新选择");entry(source,reference,input)?;
    }}Ok(result)
}
pub fn validate_output(inputs:&[edit::Input],output:&Path)->Result<()> {
    let parent=output.parent().context("请选择新 ZIP 保存位置")?.canonicalize()?;
    let output=parent.join(output.file_name().context("新 ZIP 名称为空")?);
    let target=super::workflow_triggers::key(&output);
    for input in inputs {if let Some(reference)=&input.comparison {
        let source=Path::new(&reference.source.path).canonicalize()?;
        let root=super::workflow_triggers::key(&source);
        ensure!(target!=root && (reference.source.kind!="directory"||!target.starts_with(&(root.trim_end_matches('/').to_string()+"/"))),"新 ZIP 请保存到来源文件夹之外；原件保留");
    }}Ok(())
}
pub fn save(source:&Path,revision:&str,changes:&[edit::Change],original_edits:&[edit::OriginalEdit],output:&Path,method:&str,level:u32,credentials:&edit::Credentials,source_credentials:&[Credential],flag:Arc<AtomicBool>,observer:edit::Observer)->Result<()> {
    if !changes.iter().any(|c|c.input.comparison.is_some()){return edit::save_encrypted(source,revision,changes,original_edits,output,method,level,credentials,flag,observer)}
    let inputs:Vec<_>=changes.iter().map(|c|c.input.clone()).collect();validate_output(&inputs,output)?;
    let originals=sources(&inputs,flag.clone())?;
    let staging=tempfile::Builder::new().prefix(".long-compose-").tempdir_in(output.parent().context("请选择保存位置")?)?;
    let mut materialized=changes.to_vec();
    for (source_path,catalog) in &originals {
        let secret=source_credentials.iter().find(|c|c.path==*source_path);
        let mut archive=if catalog.source.kind=="zip"{Some(comparison::open_zip(catalog)?)}else{None};
        for (index,change) in materialized.iter_mut().enumerate() {
            let input=&mut change.input;
            let Some(reference)=&input.comparison else{continue};if reference.source.path!=*source_path{continue}
            check(&flag)?;let e=entry(catalog,reference,input)?;
            observer(edit::Progress{phase:"reading",progress:0.0,current_file:e.path.clone(),processed_bytes:0,total_bytes:input.size,output_bytes:None});
            let path=staging.path().join(index.to_string());
            if input.is_dir{std::fs::create_dir(&path)?;}else{
                let mut reader:Box<dyn Read+'_>=if let Some(archive)=&mut archive{Box::new(comparison::open_entry(archive,&e.raw,secret.map(|c|c.password.as_str()),secret.and_then(|c|c.encoding).or(reference.encoding))?)}else{Box::new(File::open(Path::new(source_path).join(&e.path))?)};
                let mut file=File::create(&path)?;let mut copied=0;let mut buffer=[0u8;65536];
                loop {check(&flag)?;let n=reader.read(&mut buffer).with_context(||format!("组包内容读取失败：{}；请核对来源密码和编码",e.path))?;if n==0{break}file.write_all(&buffer[..n])?;copied+=n as u64;observer(edit::Progress{phase:"reading",progress:0.0,current_file:e.path.clone(),processed_bytes:copied,total_bytes:input.size,output_bytes:None});}
                ensure!(copied==input.size,"组包来源大小已变化，请重新确认");
            }
            let local=edit::prepare(&[path.to_string_lossy().into_owned()],"")?.remove(0);
            input.source_path=local.source_path;input.source_revision=local.source_revision;input.comparison=None;
        }
    }
    sources(&inputs,flag.clone())?;
    edit::save_encrypted_guarded(source,revision,&materialized,original_edits,output,method,level,credentials,flag.clone(),observer,||{sources(&inputs,flag)?;Ok(())})
}
