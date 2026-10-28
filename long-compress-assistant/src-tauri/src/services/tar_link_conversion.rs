//! TAR.LZO link conversion keeps expanded inputs and rebuilt output private until publication.
use super::{archive_browser,archive_format::ArchiveFormat,lzop::selected::{self,Plan},native_extraction::ExtractionRuntime,output_publish_transaction as publish};
use crate::models::compression::DecompressOptions;
use anyhow::{Context,Result};
use std::{path::{Path,PathBuf},sync::{Arc,atomic::{AtomicBool,Ordering}}};

pub(crate) struct Prepared {
    contents:tempfile::TempDir,
    source:PathBuf,
    output:PathBuf,
    rebuilt:PathBuf,
    options:DecompressOptions,
    plan:Plan,
    cancelled:Arc<AtomicBool>,
}
impl Prepared {
    pub(crate) fn open(source:&Path,output:&Path,options:DecompressOptions,cancelled:Arc<AtomicBool>)->Result<Option<Self>> {
        if !super::lzop::recovery::is_tar(source)||archive_browser::detect_format(source)?!=ArchiveFormat::Lzop{return Ok(None);}
        anyhow::ensure!(options.zpaq_snapshot.is_none(),"TAR.LZO 不适用所选 ZPAQ 快照，请重新确认来源");
        anyhow::ensure!(options.selected_entries.is_empty()&&options.preserve_paths&&options.preserve_timestamps&&!options.delete_after&&!options.skip_corrupted&&options.file_filter.is_none(),"转换必须读取完整原包并保留结构和时间");
        let Some(plan)=Plan::for_extraction(source,&options,&cancelled)? else{return Ok(None)};
        let parent=output.parent().context("请选择输出目录")?;
        let contents=tempfile::Builder::new().prefix(".long-convert-input-").tempdir_in(parent)?;
        let rebuilt=publish::staged_output_path(output,"long-convert")?;
        Ok(Some(Self{contents,source:source.into(),output:output.into(),rebuilt,options,plan,cancelled}))
    }
    pub(crate) fn check_source(&self)->Result<()> {
        self.check_source_at("Extracting")
    }
    fn check_source_at(&self,stage:&'static str)->Result<()> {
        super::task_control::wait_if_paused(&self.cancelled);
        self.plan.check_source(&self.source,&self.cancelled).map_err(|error| {
            use super::compression_service::CompressionError;
            if self.cancelled.load(Ordering::SeqCst){CompressionError::Cancelled.into()}
            else{CompressionError::SourceAccess{stage,code:"source-changed",detail:format!("转换原包已变化、离线或缺卷，本次未完成，原包保留；请重新读取：{error:#}")}.into()}
        })
    }
    pub(crate) fn read<R:ExtractionRuntime>(&self,runtime:&R,window:Option<&tauri::Window>,task:&str)->Result<Vec<String>> {
        self.plan.extract(runtime,window,task,&self.source,self.contents.path(),&self.options,&self.cancelled)?;
        self.plan.prepare_stage(&self.source,self.contents.path(),&self.options)?;
        let warnings=selected::apply_directory_times(self.contents.path(),&self.plan.directory_times(&self.options));
        if let Some(window)=window {for warning in warnings{runtime.emit_log(window,task,&warning,crate::models::compression::TaskLogSeverity::Warning);}}
        self.check_source()?;
        let mut files=std::fs::read_dir(self.contents.path())?.map(|entry|entry.map(|entry|entry.path().to_string_lossy().into_owned())).collect::<std::io::Result<Vec<_>>>()?;
        files.sort();anyhow::ensure!(!files.is_empty(),"原包没有可转换的条目");Ok(files)
    }
    pub(crate) fn rebuilt(&self)->&Path {&self.rebuilt}
    /// The caller has successfully rebuilt and verified this owned output family.
    pub(crate) fn publish_verified(&self,split:bool)->Result<()> {
        self.check_source_at("Publishing")?;
        let guard_error=std::cell::RefCell::new(None);
        let check=||match self.check_source_at("Publishing"){Ok(())=>false,Err(error)=>{*guard_error.borrow_mut()=Some(error);true}};
        let result=if split {publish::publish_verified_volumes(&self.rebuilt,&self.output,check)}else{publish::publish_verified_file(&self.rebuilt,&self.output,check)};
        match (result,guard_error.into_inner()) {
            (Ok(()),_)=>Ok(()),
            (Err(_),Some(source))=>Err(source),
            (Err(error),None)=>Err(super::compression_service::CompressionService::publication_error(error)),
        }
    }
    #[cfg(test)]
    pub(crate) fn contents(&self)->&Path {self.contents.path()}
}
impl Drop for Prepared {
    fn drop(&mut self){publish::cleanup_staged_output_family(&self.rebuilt);}
}
