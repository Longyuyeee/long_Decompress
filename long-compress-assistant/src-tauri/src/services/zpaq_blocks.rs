//! Read-only block verification for one explicitly identified journaling view.
use super::*;
use serde::Serialize;

#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
#[serde(rename_all="lowercase")]
pub enum BlockState { Verified, Failed, Unchecked }
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
#[serde(rename_all="lowercase")]
pub enum FileState { Verified, Affected, Unchecked }
#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Block { pub id:u32,pub offset:u64,pub compressed_size:u64,pub state:BlockState,pub files:usize }
#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct File { pub path:String,pub size:u64,pub state:FileState,pub blocks:Vec<u32> }
#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Report { pub snapshot:ZpaqSnapshot,pub blocks:Vec<Block>,pub entries:Vec<File>,#[serde(default)] pub revision:String }
impl Report {
    pub fn affected(&self)->usize { self.entries.iter().filter(|e|e.state==FileState::Affected).count() }
    pub fn unchecked(&self)->usize { self.entries.iter().filter(|e|e.state==FileState::Unchecked).count() }
}
pub async fn diagnose(path:&Path,password:Option<&str>,snapshot:Option<&ZpaqSnapshot>,expected_stamp:Option<&[u8]>,flag:Arc<AtomicBool>)->Result<Report>{
    let stamp=catalog_source_stamp(path)?;
    anyhow::ensure!(expected_stamp.is_none_or(|expected|expected==stamp.as_slice()),"诊断期间来源已变化，请重新检查；未沿用旧目录结论");
    let bytes=run_catalog_mode(path,password,None,false,&[],flag.clone(),None,snapshot,false,true).await?;
    let mut report: Report = {
        let text = String::from_utf8(bytes)?.replace("\r\n", "\n");
        let json = text
            .split_once("\nLONG_ZPAQ_BLOCKS\n")
            .context("缺少 ZPAQ 数据块诊断响应")?
            .1;
        serde_json::from_str(json)?
    };
    anyhow::ensure!(snapshot.is_none_or(|s|*s==report.snapshot),"所选快照已变化，请重新检查");
    for entry in &mut report.entries { entry.path=super::super::archive_entry_open::normalize_safe_entry_path(&entry.path)?; }
    anyhow::ensure!(!flag.load(std::sync::atomic::Ordering::Relaxed),"ZPAQ 诊断已取消");
    anyhow::ensure!(stamp==catalog_source_stamp(path)?,"诊断期间来源已变化，请重新检查；未保存混合来源报告");
    report.revision = revision_from_json(&stamp, &report)?;
    Ok(report)
}

#[cfg(test)]
#[path="zpaq_blocks_163_tests.rs"]
mod tests_163;
