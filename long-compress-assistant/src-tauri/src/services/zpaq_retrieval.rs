//! Explicit, verified subset retrieval. The original extraction transaction owns publication.
use super::*;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
#[serde(rename_all="camelCase")]
pub struct Selection { pub snapshot:ZpaqSnapshot,pub revision:String,pub paths:Vec<String> }
impl Selection {
    pub fn validate(&self)->Result<()> {
        anyhow::ensure!(self.snapshot.version>0&&self.snapshot.version<=9_999_999&&(19000101000000..=29991231235959).contains(&self.snapshot.date),"部分取回的快照无效，请重新诊断");
        anyhow::ensure!(self.revision.len()==64&&self.revision.bytes().all(|b|b.is_ascii_hexdigit()),"部分取回缺少诊断身份，请重新诊断");
        anyhow::ensure!(!self.paths.is_empty()&&self.paths.len()<=2000&&serde_json::to_vec(&self.paths)?.len()<=60000,"请选择 1～2000 个已检查通过的文件，或分批取回");
        let mut seen=HashSet::new();
        for path in &self.paths {anyhow::ensure!(super::super::archive_entry_open::normalize_safe_entry_path(path)?==*path&&seen.insert(path.to_lowercase()),"部分取回的所选路径重复或无效");}
        Ok(())
    }
    pub fn validate_options(&self,options:&crate::models::compression::DecompressOptions)->Result<()> {
        self.validate()?;
        anyhow::ensure!(options.zpaq_snapshot.as_ref()==Some(&self.snapshot)&&options.selected_entries==self.paths&&options.zpaq_catalog_id.is_none()&&options.reader_selection.is_none()&&options.library_source_id.is_none(),"部分取回范围已变化，请重新诊断并选择");
        anyhow::ensure!(options.preserve_paths&&!options.delete_after&&!options.skip_corrupted&&!options.extract_only_newer&&options.file_filter.as_deref().is_none_or(|s|s.is_empty()),"部分取回需要保留路径与原件，不能混用过滤或跳过损坏设置");
        Ok(())
    }
}

#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Outcome {pub path:String,pub size:u64,pub state:String}
#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Receipt {pub schema_version:u32,pub kind:String,pub source:String,pub snapshot:ZpaqSnapshot,pub revision:String,pub entries:Vec<Outcome>}

// A deterministic leaf outside the chosen file names, including case-insensitive Windows names.
pub fn receipt_name(selection:&Selection)->String {
    let roots=selection.paths.iter().map(|p|p.split('/').next().unwrap_or(p).to_lowercase()).collect::<HashSet<_>>();
    for n in 0..=selection.paths.len() {let name=if n==0{"Long解压-部分取回说明.json".to_string()}else{format!("Long解压-部分取回说明-{n}.json")};if !roots.contains(&name.to_lowercase()){return name}}
    unreachable!()
}
pub fn result_directory(parent:&Path)->PathBuf {
    loop {let suffix=uuid::Uuid::new_v4().simple().to_string();let path=parent.join(format!("ZPAQ-部分取回-{}-{}",chrono::Local::now().format("%Y%m%d-%H%M%S"),&suffix[..8]));if !path.exists(){return path}}
}

/// Password candidates are checked against the requested view and exact files, never unrelated damaged blocks.
pub async fn try_password(path:&Path,password:&str,selection:&Selection,flag:Arc<AtomicBool>)->Result<bool>{
    selection.validate()?;
    let check=async {
        let directory=browse_at(path,Some(password),Some(&selection.snapshot),flag.clone()).await?;
        for name in &selection.paths {anyhow::ensure!(directory.entries.iter().any(|e|!e.is_dir&&e.path==*name),"所选文件已不在此快照，请重新诊断");}
        run(path,Some(password),None,true,&selection.paths,flag,Some(&selection.snapshot)).await?;
        Ok::<_,anyhow::Error>(())
    }.await;
    match check {Ok(())=>Ok(true),Err(e) if e.to_string().contains("[archive-inspection:ambiguous-data]")=>Ok(false),Err(e)=>Err(e)}
}

pub async fn extract(path:&Path,output:&Path,password:Option<&str>,selection:&Selection,flag:Arc<AtomicBool>)->Result<Receipt>{
    selection.validate()?;
    let stamp=catalog_source_stamp(path)?;
    let report=blocks::diagnose(path,password,Some(&selection.snapshot),Some(&stamp),flag.clone()).await?;
    anyhow::ensure!(report.revision==selection.revision,"[archive-inspection:snapshot-unavailable] 来源或诊断结果已变化，请重新诊断并选择；未沿用旧文件清单");
    let wanted=selection.paths.iter().collect::<HashSet<_>>();
    for name in &selection.paths {anyhow::ensure!(report.entries.iter().any(|e|e.path==*name&&e.state==blocks::FileState::Verified),"所选文件尚未通过内容检查，请重新诊断；未取回受影响或未检查内容");}
    extract_at(path,output,password,&selection.paths,Some(&selection.snapshot),flag.clone()).await?;
    anyhow::ensure!(!flag.load(std::sync::atomic::Ordering::Relaxed),"部分取回已取消");
    anyhow::ensure!(stamp==catalog_source_stamp(path)?,"[archive-inspection:snapshot-unavailable] 取回期间来源已变化，未发布部分文件");
    for e in report.entries.iter().filter(|e|wanted.contains(&e.path)) {anyhow::ensure!(std::fs::metadata(output.join(&e.path))?.len()==e.size,"取回文件大小不一致，未发布结果");}
    let source=super::super::zpaq_multipart::details(path)?.and_then(|g|g.members.first().map(|p|p.path.to_string_lossy().into_owned())).unwrap_or_else(||path.to_string_lossy().into_owned());
    Ok(Receipt{schema_version:1,kind:"zpaq-partial-retrieval".into(),source,snapshot:selection.snapshot,revision:selection.revision.clone(),entries:report.entries.into_iter().map(|e|{let state=if wanted.contains(&e.path){"retrieved"}else{match e.state{blocks::FileState::Verified=>"not_selected",blocks::FileState::Affected=>"affected",blocks::FileState::Unchecked=>"unchecked"}};Outcome{path:e.path,size:e.size,state:state.into()}}).collect()})
}
pub fn write_receipt(output:&Path,selection:&Selection,receipt:&Receipt)->Result<()> {
    use std::io::Write;
    let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(output.join(receipt_name(selection)))?;
    file.write_all(&serde_json::to_vec_pretty(receipt)?)?;file.sync_all()?;Ok(())
}
pub fn read_receipt(output:&Path,selection:&Selection)->Result<Receipt>{
    selection.validate()?;
    let path=output.join(receipt_name(selection));anyhow::ensure!(path.metadata()?.len()<=64*1024*1024,"取回说明过大，请在输出目录查看");
    let report:Receipt=serde_json::from_slice(&std::fs::read(path)?)?;
    anyhow::ensure!(report.kind=="zpaq-partial-retrieval"&&report.schema_version==1&&report.revision==selection.revision&&report.snapshot==selection.snapshot,"取回说明与任务范围不符");Ok(report)
}

#[cfg(test)]
#[path="zpaq_retrieval_164_tests.rs"]
mod tests_164;
