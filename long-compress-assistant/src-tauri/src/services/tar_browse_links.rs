//! Read-only TAR.LZO link resolution. Targets are archive entries, never OS paths.
use super::{archive_preview::MAX_TAR_PREVIEW_SCAN_BYTES, image_reader::Control, tar_compatibility::LogicalEntry};
use anyhow::{Context, Result};
use std::{collections::{HashMap,HashSet}, io::{self,Read}, path::Path, sync::{Arc,atomic::{AtomicU64,Ordering}}};

fn unsupported(message:&str)->anyhow::Error {anyhow::anyhow!("[archive-inspection:unsupported] TAR.LZO {message}")}
fn key(bytes:&[u8])->Result<String> {
    let name=std::str::from_utf8(bytes).context("[archive-inspection:unsupported] TAR.LZO 名称不是 UTF-8")?;
    anyhow::ensure!(!name.starts_with(['/','\\'])&&!name.contains([':', '\0'])&&!name.split(['/','\\']).any(|p|p==".."),"[archive-inspection:unsupported] TAR.LZO 路径无法确认");
    Ok(name.split('/').filter(|p|!p.is_empty()&&*p!=".").collect::<Vec<_>>().join("/"))
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub(crate) struct Source {pub index:usize,pub key:String,pub size:u64,pub identity:String}
#[derive(Clone)]
pub(crate) struct View {pub index:usize,pub size:u64,pub link_target:Option<String>,pub source:Option<Source>}
struct Recorded {key:String,target:Option<String>,view:View}
#[derive(Default)]
pub(crate) struct Resolver {sources:HashMap<String,Option<Source>>,visible:HashSet<String>,total:u64,records:Vec<Recorded>}
impl Resolver {
    pub(crate) fn observe<R:Read>(&mut self,entry:&LogicalEntry<'_,R>,index:usize)->Result<View> {
        anyhow::ensure!(index<100_000,"[archive-inspection:unsupported] TAR.LZO 目录超过支持范围");
        let name=key(entry.original_path())?;
        let kind=entry.header().entry_type();
        if name.is_empty()&&kind.is_dir(){return Ok(View{index,size:0,link_target:None,source:None});}
        anyhow::ensure!(!name.is_empty(),"[archive-inspection:unsupported] TAR.LZO 条目名称为空");
        let visible=entry.path().to_string_lossy().replace('\\',"/");
        anyhow::ensure!(self.visible.insert(visible),"[archive-inspection:unsupported] TAR.LZO 存在重复可见路径，无法唯一选择内容，请向提供者索取名称明确的原包");
        let (size,link_target,source)=if kind.is_hard_link(){
            anyhow::ensure!(entry.header().entry_size()?==0&&entry.size()==0,"[archive-inspection:unsupported] TAR.LZO 硬链接带正文或冲突大小");
            let target=entry.link_name().context("[archive-inspection:unsupported] TAR.LZO 硬链接缺少目标")?;
            let target_key=key(&target)?;
            anyhow::ensure!(!target_key.is_empty()&&target_key!=name,"[archive-inspection:unsupported] TAR.LZO 硬链接目标为空或指向自身");
            let source=self.sources.get(&target_key).cloned().flatten();
            (source.as_ref().map_or(0,|source|source.size),Some(String::from_utf8(target.into_owned())?),source)
        }else if entry.is_file(){
            (entry.size(),None,Some(Source{index,key:name.clone(),size:entry.size(),identity:entry.view_identity()}))
        }else if kind.is_dir(){(0,None,None)}else{return Err(unsupported("此链接或非普通条目暂不支持浏览和预览"));};
        self.total=self.total.checked_add(size).context("TAR.LZO 逻辑大小溢出")?;
        anyhow::ensure!(self.total<=super::extraction_transaction::MAX_EXTRACTED_BYTES,"[archive-inspection:unsupported] TAR.LZO 逻辑总量超过支持范围");
        let registered=if link_target.is_none(){source.clone()}else{None};
        self.sources.entry(name.clone()).and_modify(|v|*v=None).or_insert(registered);
        let target=link_target.as_ref().map(|target|key(target.as_bytes())).transpose()?;
        let view=View{index,size,link_target,source};
        self.records.push(Recorded{key:name,target,view:view.clone()});
        Ok(view)
    }
    /// Publish no unresolved sizes. All targets must be unique contents in the
    /// complete, checksum-confirmed stream, regardless of physical ordering.
    pub(crate) fn finish(self,mut check:impl FnMut()->Result<()>)->Result<Vec<View>> {
        use super::tar_link_targets::{Record,resolve};
        let records=self.records.iter().map(|r|Record{key:r.key.clone(),target:r.target.clone(),content:r.view.link_target.is_none()&&r.view.source.is_some()}).collect::<Vec<_>>();
        let targets=resolve(&records,&mut check)?;
        let mut total=0u64;let mut views=Vec::with_capacity(self.records.len());
        for(index,target)in targets.into_iter().enumerate(){
            check()?;let mut view=self.records[index].view.clone();
            if let Some(target)=target {
                let source=self.records[target].view.source.clone().context("[archive-inspection:unsupported] TAR.LZO 链接内容无法确认")?;
                view.size=source.size;view.source=Some(source);
            }
            total=total.checked_add(view.size).context("TAR.LZO 逻辑大小溢出")?;
            anyhow::ensure!(total<=super::extraction_transaction::MAX_EXTRACTED_BYTES,"[archive-inspection:unsupported] TAR.LZO 逻辑总量超过支持范围");
            views.push(view);
        }
        Ok(views)
    }
}

struct Limited<R>{inner:R,remaining:Arc<AtomicU64>,budget:u64,control:Control}
impl<R:Read> Read for Limited<R>{
    fn read(&mut self,bytes:&mut[u8])->io::Result<usize>{
        self.control.check().map_err(io::Error::other)?;
        if bytes.is_empty(){return Ok(0);}
        let available=self.remaining.load(Ordering::Relaxed);
        if available==0{return Err(io::Error::other("TAR.LZO 预览超过 64 MiB 累计扫描范围，请提取后查看"));}
        let count=bytes.len().min(available as usize);let read=self.inner.read(&mut bytes[..count])?;
        let left=self.remaining.fetch_sub(read as u64,Ordering::Relaxed)-read as u64;
        self.control.update("locating",self.budget-left,self.budget);Ok(read)
    }
}
pub(crate) fn preview(path:&Path,selected:&str,keep:u64,prefix:bool,control:&Control)->Result<(Vec<u8>,u64)>{
    preview_with_budget(path,selected,keep,prefix,control,MAX_TAR_PREVIEW_SCAN_BYTES)
}
pub(crate) fn preview_with_budget(path:&Path,selected:&str,keep:u64,prefix:bool,control:&Control,budget:u64)->Result<(Vec<u8>,u64)>{
    let revision=super::image_reader::revision(path)?;
    let remaining=Arc::new(AtomicU64::new(budget));
    let open=||->Result<_>{Ok(tar::Archive::new(super::tar_pax::Reader::with_links(Limited{inner:super::codec_stream::decoder(path,"lzo")?,remaining:remaining.clone(),budget,control:control.clone()})))};
    let read=|entry:&mut dyn Read,size:u64|->Result<Vec<u8>>{
        anyhow::ensure!(prefix||size<=keep,"Image preview is limited to 8 MiB after decompression");
        let amount=size.min(keep);let mut bytes=Vec::with_capacity(amount as usize);let mut buffer=[0;65536];
        while (bytes.len()as u64)<amount {control.check()?;let count=((amount-bytes.len()as u64)as usize).min(buffer.len());entry.read_exact(&mut buffer[..count]).context("TAR.LZO 预览内容不完整")?;bytes.extend_from_slice(&buffer[..count]);control.update("reading",bytes.len()as u64,size);}
        Ok(bytes)
    };
    let selected=selected.trim_start_matches("./");let mut resolved=None;let mut forward=None;let mut archive=open()?;let mut resolver=Resolver::default();
    for (index,item)in archive.entries()?.enumerate(){
        control.check()?;let mut entry=LogicalEntry::new(item?)?;let view=resolver.observe(&entry,index)?;
        if entry.path().to_string_lossy().replace('\\',"/")!=selected{continue;}
        let source=if let Some(source)=view.source{source}else{
            anyhow::ensure!(view.link_target.is_some(),"Directories cannot be previewed");
            forward=Some(index);continue;
        };
        if view.link_target.is_none(){let bytes=read(&mut entry,source.size)?;super::image_reader::check_source(path,&revision)?;return Ok((bytes,source.size));}
        anyhow::ensure!(prefix||source.size<=keep,"Image preview is limited to 8 MiB after decompression");
        resolved=Some(source);break;
    }
    if let Some(selected)=forward {
        super::tar_compatibility::finish_stream(archive.into_inner(),||control.check())?;
        resolved=resolver.finish(||control.check())?.into_iter().find(|view|view.index==selected).and_then(|view|view.source);
    }else{drop(archive);}
    let expected=resolved.context("The selected entry was not found in the archive")?;
    anyhow::ensure!(prefix||expected.size<=keep,"Image preview is limited to 8 MiB after decompression");
    super::image_reader::check_source(path,&revision)?;control.check()?;
    // Replay the confirmed content, using the same aggregate scan budget.
    let mut archive=open()?;let mut resolver=Resolver::default();
    for(index,item)in archive.entries()?.enumerate(){
        control.check()?;let mut entry=LogicalEntry::new(item?)?;let view=resolver.observe(&entry,index)?;
        if index!=expected.index{continue;}
        anyhow::ensure!(view.link_target.is_none()&&view.source.as_ref()==Some(&expected),"IMAGE_READER_CHANGED|链接来源已变化，请刷新目录后重新预览");
        let bytes=read(&mut entry,expected.size)?;super::image_reader::check_source(path,&revision)?;return Ok((bytes,expected.size));
    }
    Err(anyhow::anyhow!("IMAGE_READER_CHANGED|链接来源无法重新确认，请刷新目录"))
}
