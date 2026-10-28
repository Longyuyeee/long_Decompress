//! Complete TAR.LZO selected contents, using the confirmed-prefix copier.
use super::recovery::{self,Entry,Source};
use crate::{models::compression::{DecompressOptions,TaskLogSeverity},services::{extraction_transaction as tx,native_extraction::ExtractionRuntime,tar_recovery_time::ModificationTime}};
use anyhow::{Context,Result};
use std::{collections::HashSet,path::Path,sync::{Arc,atomic::AtomicBool}};
use tauri::Window;

pub(crate) type DirectoryTimes=Vec<(String,ModificationTime)>;
#[derive(Clone)]
pub(crate) struct Plan {
    source:Source, revision:String, entries:Vec<Entry>,
    pub(crate) stats:crate::services::tar_compatibility::Plan,
}
impl Plan {
    pub(crate) fn build(path:&Path,options:&DecompressOptions,cancelled:&Arc<AtomicBool>)->Result<Option<Self>> {
        if options.selected_entries.is_empty()||!recovery::is_tar(path){return Ok(None);}
        crate::services::task_control::wait_if_paused(cancelled);recovery::check(cancelled)?;
        let source=recovery::source(path)?;let revision=crate::services::image_reader::revision(path)?;
        let report=recovery::scan_source(&source,options.zip_password_encoding,cancelled,&mut|_,_|crate::services::task_control::wait_if_paused(cancelled))?;
        let selected=|entry:&Entry|tx::matches_selected_entries(Path::new(entry.archive_name.as_deref().unwrap_or(&entry.name)),&options.selected_entries);
        if !report.complete {
            // Keep ordinary selected extraction's previous routing. A selected
            // link never falls back to zero bytes or unchecked target contents.
            if !report.entries.iter().any(|e|e.link_target.is_some()&&selected(e)){return Ok(None);}
            anyhow::bail!("TAR.LZO 所选链接需要完整且可确认的原包，请补卷或使用部分取回：{}",report.stop_reason);
        }
        anyhow::ensure!(source.stamp==recovery::source(path)?.stamp,"来源在准备期间已变化，请重新读取目录");
        crate::services::image_reader::check_source(path,&revision)?;
        for path in &options.selected_entries {
            anyhow::ensure!(report.entries.iter().any(|e|tx::matches_selected_entries(Path::new(e.archive_name.as_deref().unwrap_or(&e.name)),std::slice::from_ref(path))),"所选条目「{path}」已不存在，请刷新目录；未扩大选取范围");
        }
        let filter=tx::compile_file_filter(options.file_filter.as_deref());
        let entries=report.entries.iter().filter(|e|selected(e)).filter(|e|{
            let path=Path::new(&e.name);let relative=if options.preserve_paths{path}else{Path::new(path.file_name().unwrap_or_default())};
            tx::matches_compiled_file_filter(relative,&filter)
        }).cloned().collect::<Vec<_>>();
        anyhow::ensure!(!entries.is_empty(),"所选范围没有符合筛选条件的内容，请调整筛选后重试");
        let mut sources=HashSet::new();let mut logical_bytes=0u64;let mut payload_bytes=0u64;
        for e in &entries {logical_bytes=logical_bytes.checked_add(e.bytes).context("所选逻辑大小溢出")?;if !e.is_directory&&sources.insert(e.link_source.unwrap_or(e.id)){payload_bytes=payload_bytes.checked_add(e.stored_bytes.unwrap_or(e.bytes)).context("所选数据大小溢出")?;}}
        let stats=crate::services::tar_compatibility::Plan{entries:entries.len(),logical_bytes,payload_bytes};
        tx::validate_sparse_resource_limits(path,stats.entries,stats.logical_bytes,stats.payload_bytes)?;
        Ok(Some(Self{source,revision,entries,stats}))
    }
    pub(crate) fn directory_times(&self,options:&DecompressOptions)->DirectoryTimes {
        if !options.preserve_paths||!options.preserve_timestamps{return Vec::new();}
        self.entries.iter().filter(|e|e.is_directory).filter_map(|e|e.modified_time.clone().map(|time|(e.name.clone(),time))).collect()
    }
    pub(crate) fn stage_options(&self,options:&DecompressOptions)->DecompressOptions {
        let mut staged=options.clone();
        // Selection and filtering have already used archive identities. Name
        // repair must not cause the selected output to be filtered out again.
        staged.selected_entries=self.entries.iter().map(|e|e.name.clone()).collect();staged.file_filter=None;staged
    }
    pub(crate) fn prepare_stage(&self,path:&Path,stage:&Path,options:&DecompressOptions)->Result<()> {
        tx::prepare_staging_layout_with_tar_plan(path,stage,&self.stage_options(options),Some(&self.stats))
    }
    pub(crate) fn extract<R:ExtractionRuntime>(&self,runtime:&R,window:Option<&Window>,task:&str,path:&Path,stage:&Path,options:&DecompressOptions,cancelled:&Arc<AtomicBool>)->Result<()> {
        crate::services::task_control::wait_if_paused(cancelled);runtime.check_cancellation()?;crate::services::image_reader::check_source(path,&self.revision)?;
        anyhow::ensure!(self.source.stamp==recovery::source(path)?.stamp,"来源已变化，请刷新目录后重新选择");
        anyhow::ensure!(stage.is_dir()&&std::fs::read_dir(stage)?.next().is_none(),"所选内容必须写入本次空暂存目录");
        tx::validate_disk_capacity(stage,self.stats.logical_bytes)?;
        let mut entries=self.entries.clone();if !options.preserve_timestamps {for e in &mut entries{e.modified_time=None;}}
        let mut last_reserve=0;let result=super::tar_recovery::copy_selected_controlled(&self.source,stage,&mut entries,cancelled,&mut|bytes|{
            crate::services::task_control::wait_if_paused(cancelled);runtime.check_cancellation()?;
            if bytes.saturating_sub(last_reserve)>=16*1024*1024{tx::validate_staging_disk_reserve(stage)?;last_reserve=bytes;}
            if let Some(window)=window{let total=self.stats.logical_bytes;runtime.emit_progress(window,task,if total==0{1.0}else{bytes as f32/total as f32},None,bytes,total);}
            Ok(())
        });
        runtime.check_cancellation()?;result?;
        crate::services::image_reader::check_source(path,&self.revision)?;
        anyhow::ensure!(self.source.stamp==recovery::source(path)?.stamp,"来源在提取期间已变化，请重新读取");
        if options.preserve_timestamps {if let Some(window)=window{for e in entries.iter().filter(|e|e.time_warning.is_some()){runtime.emit_log(window,task,&format!("{}：{}",e.name,e.time_warning.as_deref().unwrap()),TaskLogSeverity::Warning);}}}
        Ok(())
    }
}
pub(crate) fn apply_directory_times(output:&Path,times:&DirectoryTimes)->Vec<String>{
    let mut order=times.iter().collect::<Vec<_>>();order.sort_by_key(|(name,_)|std::cmp::Reverse(Path::new(name).components().count()));
    order.into_iter().filter_map(|(name,time)|time.apply_directory(&output.join(name)).err().map(|why|format!("{name}：{why}"))).collect()
}
