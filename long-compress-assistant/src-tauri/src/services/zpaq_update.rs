//! Incremental additions to a private copy. Existing journals and their slices are inputs only.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap, HashSet}, fs, io::{Read, Write}, path::{Path, PathBuf}, sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, time::Duration};
use super::{compression_entries::{self, CompressionEntry}, output_publish_transaction as publish, zpaq_engine};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct UpdateSummary {
    #[serde(default, skip_serializing_if="Option::is_none")]
    pub native_parts: Option<usize>,
    pub updated_entries: u64,
    pub new_fragments: u64,
    pub base_snapshots: usize,
    pub snapshots: usize,
    pub unchanged: bool,
    pub copied_bytes: u64,
    pub input_bytes: u64,
}
#[derive(Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct UpdateProgress { pub phase: &'static str, pub processed_bytes: u64, pub total_bytes: u64 }
pub type Progress = Arc<dyn Fn(UpdateProgress) + Send + Sync>;
fn notify(progress: &Progress, phase: &'static str, processed: u64, total: u64) {
    progress(UpdateProgress { phase, processed_bytes: processed, total_bytes: total });
}
fn check(flag: &Arc<AtomicBool>) -> Result<()> {
    super::task_control::wait_if_paused(flag);
    anyhow::ensure!(!flag.load(Ordering::Acquire), "ZPAQ 更新已取消，原备份保留"); Ok(())
}
struct Staged(PathBuf);
impl Drop for Staged { fn drop(&mut self) { publish::cleanup_staged_output_family(&self.0); } }

fn collect(sources: &[String], prefix: &str, excluded: &HashSet<PathBuf>) -> Result<Vec<CompressionEntry>> {
    let prefix = if prefix.trim().is_empty() { String::new() } else { super::archive_entry_open::normalize_safe_entry_path(prefix.trim().trim_end_matches('/'))? };
    let mut entries = Vec::new(); let mut names = HashSet::new(); let mut seen = HashSet::new();
    for source in sources {
        let canonical = Path::new(source).canonicalize().with_context(|| format!("更新来源不存在或无法访问：{source}"))?;
        anyhow::ensure!(!excluded.contains(&canonical), "原备份分卷不能作为本次添加的内容：{source}");
        if !seen.insert(canonical) { continue; }
        // Collect separately: duplicate destinations are reported, never silently renamed.
        for mut entry in compression_entries::collect(&[source.clone()], true, true)? {
            if excluded.contains(&entry.path.canonicalize()?) { continue; }
            let name = if prefix.is_empty() { entry.archive_name } else { format!("{prefix}/{}",entry.archive_name) };
            entry.archive_name = super::archive_entry_open::normalize_safe_entry_path(&name)?;
            anyhow::ensure!(names.insert(entry.archive_name.to_lowercase()), "本次文件在备份内重名：{}；请选择不同的归档内位置",entry.archive_name);
            entries.push(entry); anyhow::ensure!(entries.len() <= 100000, "本次更新条目超过100000，请分批添加");
        }
    }
    anyhow::ensure!(!entries.is_empty(), "请选择需要添加或更新的文件/文件夹"); Ok(entries)
}
fn validate_mapping(entries: &[CompressionEntry], old: &[crate::models::compression::ArchiveEntryInfo]) -> Result<()> {
    let old: BTreeMap<_,_> = old.iter().map(|e|(e.path.trim_end_matches('/').to_lowercase(),e)).collect();
    for entry in entries {
        let name=&entry.archive_name; let lower=name.to_lowercase();
        if let Some(previous)=old.get(&lower) {
            anyhow::ensure!(previous.path.trim_end_matches('/')==name && previous.is_dir==entry.is_dir,
                "备份路径大小写或文件/文件夹类型冲突：{name}；请调整归档内位置");
        }
        let mut parent=Path::new(name).parent();
        while let Some(p)=parent {
            let path=p.to_string_lossy().replace('\\',"/");
            if let Some(e)=old.get(&path.to_lowercase()) {
                anyhow::ensure!(e.is_dir,"备份中已有同名文件，不能作为目录：{}",e.path);
                anyhow::ensure!(e.path.trim_end_matches('/')==path,"备份目录大小写冲突：{path}；请使用原目录名称 {}",e.path);
            }
            parent=p.parent();
        }
        if !entry.is_dir { let prefix=lower+"/";anyhow::ensure!(!old.range(prefix.clone()..).next().is_some_and(|(p,_)|p.starts_with(&prefix)),"备份中此位置已有子目录内容：{name}"); }
    }
    Ok(())
}
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
struct EngineSummary { updated_entries: u64, removed_entries: u64, new_fragments: u64 }
async fn append(staged: &Path, entries: &[CompressionEntry], password: Option<&str>, level: u32, flag: Arc<AtomicBool>, progress: Progress) -> Result<EngineSummary> {
    let mut data=zpaq_engine::input(password,&[])?; data.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for entry in entries {
        let mut path=entry.path.to_string_lossy().replace('\\',"/"); let mut name=entry.archive_name.clone();
        if entry.is_dir { path.push('/');name.push('/'); }
        for text in [path,name] { anyhow::ensure!(text.len()<=65536&&!text.contains('\0'),"更新路径过长或无效");data.extend_from_slice(&(text.len() as u32).to_le_bytes()); data.extend_from_slice(text.as_bytes()); }
    }
    let method=match level {0=>"04",1..=2=>"14",3..=4=>"24",5..=6=>"34",7..=8=>"44",_=>"54"};
    let mut command=crate::utils::process::async_command(zpaq_engine::command_path().context("缺少随附ZPAQ引擎，请修复安装")?);
    command.arg("a").arg(staged).args(["-long-stdin","-long-update","-method",method,"-threads","2","-noattributes"]);
    let pending=Mutex::new(String::new());
    let on_bytes=Arc::new(move |bytes:&[u8]| {
        let mut pending=pending.lock().unwrap_or_else(|e|e.into_inner());pending.push_str(&String::from_utf8_lossy(bytes));
        while let Some(end)=pending.find('\n') { let line:String=pending.drain(..=end).collect();
            if let Some(values)=line.trim().strip_prefix("LONG_ZPAQ_UPDATE_PROGRESS:") {if let Some((done,total))=values.split_once(':') { if let (Ok(done),Ok(total))=(done.parse::<u64>(),total.parse::<u64>()) {notify(&progress,"updating",done,total);} } }
        }
        if pending.len()>65536 { pending.clear(); }
    });
    let result=crate::utils::process::cancellable_archive_output_with_progress(&mut command,&flag,Duration::from_secs(24*60*60),data,on_bytes).await;
    check(&flag)?; let result=result.context("ZPAQ 更新引擎运行失败")?;
    if !result.status.success() { let detail=format!("{}\n{}",String::from_utf8_lossy(&result.stderr),String::from_utf8_lossy(&result.stdout));if let Some(error)=zpaq_engine::io_failure(&detail){return Err(error)} anyhow::bail!("ZPAQ 添加/更新失败：{}",detail.chars().take(3000).collect::<String>()); }
    let text=String::from_utf8(result.stdout)?.replace("\r\n","\n"); let summary:EngineSummary=serde_json::from_str(text.split_once("\nLONG_ZPAQ_UPDATE\n").context("引擎未返回更新结果")?.1.trim())?;
    anyhow::ensure!(summary.removed_entries==0,"更新未按保留旧内容的约定完成，未发布结果"); Ok(summary)
}

pub async fn update(source: &Path, sources: &[String], prefix: &str, output: &Path, password: Option<&str>, level: u32, split_size: Option<u64>, flag: Arc<AtomicBool>, progress: Progress) -> Result<UpdateSummary> {
    check(&flag)?;
    let native=super::zpaq_multipart::details(source)?;
    if native.is_some() { anyhow::ensure!(!split_size.is_some_and(|n|n>0),"原生多部分更新按事务生成新部分，无需指定数字分卷大小；请清除分卷大小后重试"); }
    anyhow::ensure!(output.is_absolute()&&output.extension().is_some_and(|e|e.to_str().is_some_and(|s|s.eq_ignore_ascii_case("zpaq"))),"新备份需要使用完整的 .zpaq 路径");
    anyhow::ensure!(!output.exists()&&super::archive_volumes::volume_paths(output)?.is_empty(),"新备份名称已存在，请另选名称");
    anyhow::ensure!(level<=9,"压缩等级应为0～9");
    let source=super::split_archive_detector::SplitArchiveDetector::complete_source(source)?;
    let group=super::split_archive_detector::SplitArchiveDetector::detect_split_archive(&source)?;
    let members=group.as_ref().map(|g|g.parts.clone()).unwrap_or_else(||vec![source.clone()]);
    let mut excluded=members.iter().map(|p|p.canonicalize()).collect::<std::io::Result<HashSet<_>>>()?;
    // The future output is not collected; all input entries are fixed before staging starts.
    if let Ok(p)=output.canonicalize(){excluded.insert(p);}
    let entries=collect(sources,prefix,&excluded)?;
    let input_bytes=entries.iter().filter(|e|!e.is_dir).try_fold(0u64,|n,e|Ok::<_,anyhow::Error>(n.checked_add(fs::metadata(&e.path)?.len()).context("来源大小溢出")?))?;
    notify(&progress,"reading",0,0);
    let old=zpaq_engine::browse(&source,password,flag.clone()).await?;
    let history=old.zpaq_history.as_ref().context("仅能更新完整的日志式ZPAQ；流式归档请先转换为新的ZPAQ")?;
    validate_mapping(&entries,&old.entries)?;
    let copied_bytes=old.total_compressed_size;let base_snapshots=history.snapshots.len();
    let parent=output.parent().context("请选择新备份保存目录")?;
    let minimum=copied_bytes.checked_add(super::storage_preflight::DISK_SAFETY_RESERVE).context("备份空间需求溢出")?;
    if let Some(available)=super::storage_preflight::available_disk_space(parent) {anyhow::ensure!(available>=minimum,"复制原备份至少需要{minimum}字节，当前可用{available}字节；请更换保存位置");}
    if let Some(group)=native {
        return native_update::update(&source,&group,&entries,output,if old.encrypted {password}else{None},level,base_snapshots,copied_bytes,input_bytes,flag,progress).await;
    }
    let staged=Staged(publish::staged_output_path(output,"zpaq-update")?);
    let (copy_source,copy_path,copy_flag,copy_progress)=(source.clone(),staged.0.clone(),flag.clone(),progress.clone());
    tokio::task::spawn_blocking(move||->Result<()> {
        let mut reader=super::numeric_volume_reader::ZipInputReader::open(&copy_source)?;
        let mut file=fs::OpenOptions::new().write(true).create_new(true).open(copy_path)?;
        let mut buffer=[0u8;65536];let mut copied=0;let mut gate=super::progress_gate::ProgressGate::new();
        notify(&copy_progress,"copying",0,copied_bytes);
        loop { check(&copy_flag)?;let n=reader.read(&mut buffer)?;if n==0{break}file.write_all(&buffer[..n])?;copied+=n as u64;if gate.allow(copied==copied_bytes){notify(&copy_progress,"copying",copied,copied_bytes);} }
        anyhow::ensure!(copied==copied_bytes,"复制期间原备份大小发生变化，请重新读取后更新");file.sync_all()?;Ok(())
    }).await??;
    check(&flag)?; notify(&progress,"updating",0,input_bytes);
    let secret=if old.encrypted { password } else { None };
    let result=append(&staged.0,&entries,secret,level,flag.clone(),progress.clone()).await?;
    notify(&progress,"verifying",0,0);
    zpaq_engine::verify_at(&staged.0,secret,None,flag.clone()).await?;
    let new=zpaq_engine::browse(&staged.0,secret,flag.clone()).await?;
    let snapshots=new.zpaq_history.as_ref().context("更新结果未保留历史快照")?.snapshots.len();
    anyhow::ensure!(snapshots==base_snapshots+usize::from(result.updated_entries>0),"更新快照数量不一致，未发布新包");
    if let Some(size)=split_size.filter(|s|*s>0) {
        let (path,cut_flag,cut_progress)=(staged.0.clone(),flag.clone(),progress.clone());
        tokio::task::spawn_blocking(move||super::split_compression::SplitCompressionService::split_archive_file_with_progress(&path,&path,size,||check(&cut_flag),|done,total|notify(&cut_progress,"splitting",done,total))).await??;
        check(&flag)?;notify(&progress,"publishing",0,0);publish::publish_verified_volumes(&staged.0,output,||flag.load(Ordering::Acquire))?;
    } else {check(&flag)?;notify(&progress,"publishing",0,0);publish::publish_verified_file(&staged.0,output,||flag.load(Ordering::Acquire))?;}
    notify(&progress,"completed",input_bytes,input_bytes);
    Ok(UpdateSummary { native_parts:None,updated_entries:result.updated_entries,new_fragments:result.new_fragments,base_snapshots,snapshots,unchanged:result.updated_entries==0,copied_bytes,input_bytes })
}

#[path="zpaq_native_update.rs"]
pub(crate) mod native_update;

#[cfg(test)]
#[path="zpaq_update_1536_tests.rs"]
mod tests;

#[cfg(test)]
#[path="zpaq_native_update_161_tests.rs"]
mod native_tests;
