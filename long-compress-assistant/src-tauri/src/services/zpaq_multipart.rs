//! Explicit, immutable local source references for native ZPAQ transaction parts.
//! A reference contains paths and file stamps, never data or passwords. It is
//! retained outside preview-cache so pending tasks survive a normal restart.
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, io::{Read, Write}, path::{Path, PathBuf}};
use super::split_archive_detector::{SplitArchiveInfo, SplitFormat};
const MAGIC: &[u8] = b"LONG_ZPAQ_GROUP_1\n";
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all="camelCase")]
pub struct Member { pub path: PathBuf, pub size: u64, pub stamp: String }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Group { pub name: String, pub members: Vec<Member>, pub missing: Vec<String> }
fn numbered(path:&Path)->Option<(String,usize,usize)> {
    let stem=path.file_stem()?.to_str()?;
    if !path.extension()?.eq_ignore_ascii_case("zpaq") { return None; }
    let start=stem.trim_end_matches(|c:char|c.is_ascii_digit()).len();
    if start==stem.len() {return None;}
    let digits=&stem[start..];let number=digits.parse().ok()?;
    Some((stem[..start].to_string(),digits.len(),number))
}
fn familiar(path:PathBuf)->PathBuf{
    let text=path.to_string_lossy();
    if let Some(tail)=text.strip_prefix(r"\\?\UNC\"){PathBuf::from(format!(r"\\{tail}"))}else{PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(&text))}
}
fn member(path:PathBuf)->Result<Member>{
    let metadata=fs::metadata(&path).with_context(||format!("[archive-inspection:incomplete] 缺少 ZPAQ 部分：{}；请补回原文件后重试",path.display()))?;
    ensure!(metadata.is_file()&&metadata.len()>0,"ZPAQ 部分为空或不是文件：{}",path.display());
    Ok(Member{stamp:format!("{:?}",metadata.modified()?),size:metadata.len(),path})
}
/// Suggestions do not affect any reader. Even independently valid files with
/// the same prefix remain independent until the user explicitly confirms.
pub fn suggest(path:&Path)->Result<Option<Group>>{
    if read_reference(path)?.is_some(){return Ok(None);}
    let Some((prefix,width,_))=numbered(path)else{return Ok(None)};
    let mut found=Vec::new();
    for entry in fs::read_dir(path.parent().context("找不到来源目录")?)? {
        let entry=entry?;
        if let Some((p,_w,n))=numbered(&entry.path()){
            if p.eq_ignore_ascii_case(&prefix)&&entry.file_type()?.is_file(){found.push((n,entry.path()));}
        }
    }
    if found.len()<2{return Ok(None);}
    found.sort_by_key(|(n,_)|*n);
    // Date-suffixed independent backups are common; do not force them into an impossible numbered group.
    if found.last().unwrap().0>100000 {return Ok(None);}
    ensure!(found[0].0>0&&found.windows(2).all(|p|p[0].0!=p[1].0),"ZPAQ 部分编号为 0、重复或过大，请整理来源名称后重试");
    let total=found.last().unwrap().0;
    let present:std::collections::HashSet<usize>=found.iter().map(|(n,_)|*n).collect();
    let missing=(1..=total).filter(|n|!present.contains(n)).map(|n|format!("{prefix}{n:0width$}.zpaq")).collect();
    let members=found.into_iter().map(|(_,p)|member(familiar(p.canonicalize()?))).collect::<Result<Vec<_>>>()?;
    Ok(Some(Group{name:format!("{}.zpaq",prefix.trim_end_matches(['.','-','_'])),members,missing}))
}
fn read_reference(path:&Path)->Result<Option<Group>>{
    if !path.extension().is_some_and(|e|e.eq_ignore_ascii_case("zpaq"))||!path.is_file(){return Ok(None);}
    let mut file=fs::File::open(path)?;let mut magic=vec![0;MAGIC.len()];
    if file.read_exact(&mut magic).is_err()||magic!=MAGIC{return Ok(None);}
    ensure!(file.metadata()?.len()<=16*1024*1024,"ZPAQ 来源记录过大，请重新选择成员确认整组");
    let group:Group=serde_json::from_reader(file).context("ZPAQ 来源记录无法读取，请重新选择成员确认整组")?;
    validate_members(&group)?;Ok(Some(group))
}
fn validate_members(group:&Group)->Result<()> {
    ensure!((1..=100000).contains(&group.members.len())&&group.missing.is_empty(),"[archive-inspection:incomplete] 缺少 ZPAQ 部分：{}",group.missing.join("、"));
    if group.members.len()==1 { ensure!(group.members[0].path.is_absolute(),"来源路径无效"); return Ok(()); }
    let (prefix,_width,_)=numbered(&group.members[0].path).context("ZPAQ 来源编号无效，请重新确认整组")?;
    let parent=group.members[0].path.parent();
    for (i,item) in group.members.iter().enumerate(){
        ensure!(item.path.is_absolute()&&item.path.parent()==parent&&numbered(&item.path).is_some_and(|(p,_w,n)|p.eq_ignore_ascii_case(&prefix)&&n==i+1),"ZPAQ 部分次序异常，请从首部分重新确认整组；没有自动改序或读取单个部分");
    }Ok(())
}
pub fn confirm(root:&Path,group:Group)->Result<PathBuf>{
    validate_members(&group)?;
    for item in &group.members{ensure!(&member(item.path.clone())?==item,"[archive-inspection:changed] ZPAQ 来源在确认期间已变化，请重新选择成员确认整组");}
    let bytes=serde_json::to_vec(&group)?;let id=blake3::hash(&bytes).to_hex();
    let directory=root.join("zpaq-sources").join(id.as_str());fs::create_dir_all(&directory)?;
    // A familiar archive name in tasks, with an immutable id in its parent.
    let name=group.members[0].path.file_name().context("来源名称无效")?;
    let reference=directory.join(name);
    if reference.exists(){ensure!(fs::read(&reference)?==[MAGIC,bytes.as_slice()].concat(),"已确认的 ZPAQ 来源记录不一致，请重新选择整组");}
    else{let mut temporary=tempfile::NamedTempFile::new_in(directory)?;temporary.write_all(MAGIC)?;temporary.write_all(&bytes)?;temporary.as_file().sync_all()?;temporary.persist_noclobber(&reference).map_err(|e|e.error)?;}
    Ok(reference)
}
pub fn inspect(path:&Path)->Result<Option<SplitArchiveInfo>>{
    let Some(group)=read_reference(path)?else{return Ok(None)};
    for item in &group.members {ensure!(&member(item.path.clone())?==item,"[archive-inspection:changed] ZPAQ 来源已变化：{}；请重新选择任一成员确认整组，再建立新任务；旧任务没有改读其他来源",item.path.display());}
    Ok(Some(SplitArchiveInfo{format:SplitFormat::ZpaqNativeSplit,base_name:group.name.trim_end_matches(".zpaq").into(),first_part:path.into(),
        total_parts:group.members.len(),total_size:group.members.iter().map(|m|m.size).sum(),is_complete:true,missing_parts:vec![],parts:group.members.into_iter().map(|m|m.path).collect()}))
}
pub fn details(path:&Path)->Result<Option<Group>>{read_reference(path)}
