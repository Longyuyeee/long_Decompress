//! One nested entry, kept in the existing session cache and browser workspace.
use super::{archive_entry_open::{self as cache,ArchiveEntryOpenCache,NestedArchiveMaterializeResult},image_reader,native_extraction::ExtractionRuntime};
use crate::models::compression::{DecompressOptions,ZpaqSnapshot};
use anyhow::{Context,Result};
use std::{future::Future,path::{Path,PathBuf},sync::{Arc,atomic::{AtomicBool,Ordering}}};

struct UncommittedDirectory(PathBuf);
impl Drop for UncommittedDirectory{fn drop(&mut self){if !self.0.as_os_str().is_empty(){let _=std::fs::remove_dir_all(&self.0);}}}
fn check(path:&Path,revision:&str,flag:&AtomicBool)->Result<()>{
    anyhow::ensure!(!flag.load(Ordering::Acquire),"ARCHIVE_BROWSE_CANCELLED|已取消进入压缩包");
    image_reader::check_source(path,revision)
}

pub(crate) async fn materialize<R,F,Fut>(cache:&ArchiveEntryOpenCache,runtime:Arc<R>,archive:PathBuf,entry:String,password:Option<String>,encoding:Option<super::zip_password::ZipPasswordEncoding>,snapshot:Option<ZpaqSnapshot>,depth:usize,ancestors:Vec<String>,revision:Option<String>,flag:Arc<AtomicBool>,extract:F)->Result<NestedArchiveMaterializeResult>
where R:ExtractionRuntime+Send+Sync+'static,F:FnOnce(PathBuf,DecompressOptions)->Fut,Fut:Future<Output=Result<()>>{
    let entry=cache::normalize_safe_entry_path(&entry)?;
    anyhow::ensure!((2..=cache::MAX_NESTED_ARCHIVE_DEPTH).contains(&depth),"嵌套归档最多允许 3 层");
    let revision=match revision{Some(value)=>value,None=>image_reader::revision(&archive)?};
    check(&archive,&revision,&flag)?;
    let metadata=super::archive_browser::browse_archive_cancellable_at(&archive,password.as_deref(),flag.clone(),encoding,snapshot).await?;
    check(&archive,&revision,&flag)?;
    let selected=metadata.entries.iter().find(|e|!e.is_dir&&e.path.replace('\\',"/")==entry).context("所选嵌套归档不存在，请刷新目录")?;
    let (directory,reservation)=cache.create_entry_dir(selected.size)?;
    let mut cleanup=UncommittedDirectory(directory.clone());
    let options=DecompressOptions{zpaq_snapshot:snapshot,zip_password_encoding:encoding,parallel_zip:false,preserve_paths:true,overwrite_existing:false,delete_after:false,preserve_timestamps:true,skip_corrupted:false,extract_only_newer:false,create_subdirectory:false,preserve_mark_of_web:true,selected_entries:vec![entry.clone()],conflict_policy:"rename".into(),enable_bruteforce:false,..Default::default()};
    if selected.link_target.is_some(){
        let path=archive.clone();let out=directory.clone();let cancelled=flag.clone();let expected=revision.clone();
        tokio::task::spawn_blocking(move||->Result<()>{
            check(&path,&expected,&cancelled)?;
            let plan=super::lzop::selected::Plan::build(&path,&options,&cancelled)?.context("此链接尚不支持嵌套浏览，请先提取为独立文件")?;
            // A complete plan resolves either direction before materializing the selected alias.
            plan.extract(runtime.as_ref(),None,"archive-nested-link",&path,&out,&options,&cancelled)?;
            plan.prepare_stage(&path,&out,&options)?;
            plan.check_source(&path,&cancelled)
        }).await??;
    }else{extract(directory.clone(),options).await?;}
    check(&archive,&revision,&flag)?;
    let extracted=cache::validate_extracted_file(&directory,&entry,selected.size)?;
    // A link alias must contain an actual archive header, not just an archive suffix.
    if selected.link_target.is_some(){
        use std::io::Read;let mut bytes=[0;560];let count=std::fs::File::open(&extracted)?.read(&mut bytes)?;
        anyhow::ensure!(super::archive_format::ArchiveFormat::from_magic(&bytes[..count])!=super::archive_format::ArchiveFormat::Unknown,"所选链接内容不是可识别的压缩包，请刷新目录或提取后查看");
    }
    let source=archive.clone();let child=extracted.clone();let cancelled=flag.clone();
    let (parent_sha256,content_sha256)=tokio::task::spawn_blocking(move||cache::validate_nested_archive_identity_controlled(&source,&child,depth,&ancestors,||{
        anyhow::ensure!(!cancelled.load(Ordering::Acquire),"ARCHIVE_BROWSE_CANCELLED|已取消进入压缩包");Ok(())
    })).await??;
    check(&archive,&revision,&flag)?;
    cache.register_nested_archive(&archive,&extracted,&parent_sha256,&content_sha256,depth)?;
    reservation.commit();cleanup.0=PathBuf::new();
    Ok(NestedArchiveMaterializeResult{entry_path:entry,cache_path:extracted.to_string_lossy().into_owned(),parent_sha256,content_sha256,depth})
}
