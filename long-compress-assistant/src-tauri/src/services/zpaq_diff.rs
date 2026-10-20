//! Compare catalog references first; decode only the candidates explicitly requested.
use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, io::Read, path::Path, sync::{Arc, atomic::{AtomicBool,Ordering}}};
use crate::models::compression::ZpaqSnapshot;
use super::zpaq_engine::{self, Catalog, Entry};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Side { pub size:u64, pub date:i64, pub directory:bool, pub selectable:bool }
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Change { pub path:String, pub kind:String, pub reason:String, pub before:Option<Side>, pub after:Option<Side>, pub verified:bool }
#[derive(Debug, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Comparison { pub before:ZpaqSnapshot, pub after:ZpaqSnapshot, pub before_catalog_id:String, pub after_catalog_id:String, pub revision:String, pub entries:Vec<Change>, pub counts:BTreeMap<String,usize> }
#[derive(Clone,Serialize)]
#[serde(rename_all="camelCase")]
pub struct DiffProgress { pub phase:&'static str, pub processed_bytes:u64, pub total_bytes:u64 }
pub type Progress=Arc<dyn Fn(DiffProgress)+Send+Sync>;
fn check(flag:&AtomicBool)->Result<()> {anyhow::ensure!(!flag.load(Ordering::Acquire),"快照对比已停止，未创建或执行提取任务");Ok(())}
fn notify(progress:&Progress,phase:&'static str,done:u64,total:u64){progress(DiffProgress{phase,processed_bytes:done,total_bytes:total})}
fn map(catalog:&Catalog)->Result<BTreeMap<String,Entry>> {
    let mut map=BTreeMap::new();
    for entry in &catalog.entries {let key=entry.path.trim_end_matches('/').to_owned();anyhow::ensure!(map.insert(key,entry.clone()).is_none(),"快照包含重复路径，无法可靠对比");}Ok(map)
}
fn side(entry:&Entry,map:&BTreeMap<String,Entry>)->Side {
    let prefix=entry.path.trim_end_matches('/').to_owned()+"/";
    let children=map.range(prefix.clone()..).next().is_some_and(|(key,_)|key.starts_with(&prefix));
    Side{size:entry.size as u64,date:entry.date,directory:entry.directory,selectable:!entry.directory||!children}
}
fn compare(before:ZpaqSnapshot,after:ZpaqSnapshot,old:&Catalog,new:&Catalog)->Result<Comparison> {
    let (left,right)=(map(old)?,map(new)?);let keys:BTreeSet<_>=left.keys().chain(right.keys()).cloned().collect();
    let mut entries=Vec::new();
    for path in keys {
        let (a,b)=(left.get(&path),right.get(&path));
        let (kind,reason)=match (a,b) {
            (None,Some(_))=>("added","仅新版存在"), (Some(_),None)=>("removed","仅旧版存在；取回时读取旧版"),
            (Some(a),Some(b)) if a.directory!=b.directory=>("changed","文件与目录类型变化；目录内容请分别选择"),
            (Some(a),Some(b)) if a.size!=b.size=>("changed","文件大小变化"),
            (Some(a),Some(b)) if !a.directory&&a.content_ref!=b.content_ref=>("candidate","内容片段引用变化；尚未逐字节核对"),
            (Some(a),Some(b)) if a.date!=b.date=>("metadata","内容引用一致，记录时间变化"),
            _=>("unchanged","内容引用、类型、大小与时间记录一致"),
        };
        entries.push(Change{path,kind:kind.into(),reason:reason.into(),before:a.map(|e|side(e,&left)),after:b.map(|e|side(e,&right)),verified:false});
    }
    let revision=format!("{:x}",Sha256::digest(format!("{}:{}",old.id,new.id)));
    let mut result=Comparison{before,after,before_catalog_id:old.id.clone(),after_catalog_id:new.id.clone(),revision,entries,counts:BTreeMap::new()};recount(&mut result);Ok(result)
}
fn recount(result:&mut Comparison){result.counts.clear();for e in &result.entries{*result.counts.entry(e.kind.clone()).or_default()+=1}}
async fn hash_version(path:&Path,password:Option<&str>,snapshot:&ZpaqSnapshot,entries:&[(String,u64)],flag:Arc<AtomicBool>,progress:Progress,decode_phase:&'static str,hash_phase:&'static str)->Result<BTreeMap<String,String>> {
    check(&flag)?;let total=entries.iter().try_fold(0u64,|n,(_,size)|n.checked_add(*size).context("所选核对文件大小溢出"))?;
    let temp=tempfile::Builder::new().prefix("long-zpaq-compare-").tempdir()?;
    if let Some(available)=super::storage_preflight::available_disk_space(temp.path()){anyhow::ensure!(available>=total.saturating_add(super::storage_preflight::DISK_SAFETY_RESERVE),"临时核对空间不足，请减少所选候选文件后再核对");}
    notify(&progress,decode_phase,0,0);
    let selected:Vec<_>=entries.iter().map(|(p,_)|p.clone()).collect();
    zpaq_engine::extract_at(path,temp.path(),password,&selected,Some(snapshot),flag.clone()).await?;
    let root=temp.path().to_owned();let entries=entries.to_vec();
    tokio::task::spawn_blocking(move||->Result<BTreeMap<String,String>>{
        let mut hashes=BTreeMap::new();let mut done=0;let mut gate=super::progress_gate::ProgressGate::new();notify(&progress,hash_phase,0,total);
        for (path,size) in entries {
            check(&flag)?;let mut file=std::fs::File::open(root.join(&path))?;anyhow::ensure!(file.metadata()?.len()==size,"核对时文件大小不一致，请重新读取归档");
            let mut sha=Sha256::new();let mut buffer=[0u8;65536];
            loop{check(&flag)?;let n=file.read(&mut buffer)?;if n==0{break}sha.update(&buffer[..n]);done+=n as u64;if gate.allow(done==total){notify(&progress,hash_phase,done,total)}}
            hashes.insert(path,format!("{:x}",sha.finalize()));
        }check(&flag)?;Ok(hashes)
    }).await?
}
pub async fn run(path:&Path,password:Option<&str>,before:ZpaqSnapshot,after:ZpaqSnapshot,verify_paths:&[String],expected:Option<&str>,flag:Arc<AtomicBool>,progress:Progress)->Result<Comparison>{
    anyhow::ensure!(before.version<after.version,"请选择较早和较新的两个不同快照");check(&flag)?;notify(&progress,"catalog",0,0);
    let old=zpaq_engine::catalog(path,password,&before,flag.clone()).await?;
    let new=zpaq_engine::catalog(path,password,&after,flag.clone()).await?;
    anyhow::ensure!(old.history.snapshots==new.history.snapshots,"读取期间归档历史已变化，请重新对比");
    let mut result=compare(before,after,&old,&new)?;
    if let Some(expected)=expected{anyhow::ensure!(expected==result.revision,"来源已变化，请重新比较目录后再核对");}
    if !verify_paths.is_empty(){
        anyhow::ensure!(expected.is_some()&&verify_paths.len()<=2000,"请先比较目录，再选择最多2000个候选文件核对");
        let wanted:BTreeSet<_>=verify_paths.iter().collect();let mut entries=Vec::new();
        for path in wanted{let row=result.entries.iter().find(|e|&e.path==path).context("所选候选已不在这组快照中")?;
            anyhow::ensure!(row.kind=="candidate","仅对所选内容变化候选做精确核对：{path}");entries.push((path.clone(),row.before.as_ref().unwrap().size));}
        let a=hash_version(path,password,&before,&entries,flag.clone(),progress.clone(),"before-decode","before-hash").await?;
        let b=hash_version(path,password,&after,&entries,flag.clone(),progress.clone(),"after-decode","after-hash").await?;
        for row in &mut result.entries{if let(Some(a),Some(b))=(a.get(&row.path),b.get(&row.path)){row.verified=true;if a!=b{row.kind="changed".into();row.reason="已实际提取并核对SHA256，内容不同".into()}else if row.before.as_ref().unwrap().date!=row.after.as_ref().unwrap().date{row.kind="metadata".into();row.reason="已核对SHA256一致，仅记录时间变化".into()}else{row.kind="unchanged".into();row.reason="已实际提取并核对SHA256一致".into()}}}
        recount(&mut result);
    }
    zpaq_engine::check_catalog(path,password,&before,&result.before_catalog_id,flag.clone()).await?;check(&flag)?;notify(&progress,"complete",0,0);Ok(result)
}

#[cfg(test)]
#[path="zpaq_diff_1537_tests.rs"]
mod tests;
