//! GNU S and local PAX 0.0/0.1/1.0 maps inside the checked TAR prefix.
use anyhow::{Context,Result};
use serde::Serialize;
use sha2::{Digest,Sha256};
use std::{fs::File,io::{Read,Seek,SeekFrom,Write},sync::atomic::AtomicBool};
use super::{extraction_transaction::MAX_EXTRACTED_BYTES,lzop::recovery::check};

fn decimal(value:&str)->Result<u64>{
    anyhow::ensure!(!value.is_empty()&&value.len()<=20&&value.bytes().all(|b|b.is_ascii_digit()),"GNU.sparse 数值无效");
    value.parse().context("GNU.sparse 数值溢出")
}
fn attribute<'a>(fields:&'a super::tar_pax::Fields,key:&str)->Result<&'a str>{
    fields.iter().find(|(k,_)|k==key).map(|(_,v)|v.as_str()).with_context(||format!("GNU.sparse 缺少 {key}"))
}
/// Sparse attributes are local-only. Global PAX retains the stricter common rules.
#[derive(Clone,Copy)]
pub(crate) enum PaxVersion { Pairs00, Map01, Text10 }
pub(crate) fn validate_local(fields:&super::tar_pax::Fields)->Result<Option<PaxVersion>>{
    let sparse=fields.iter().any(|(k,_)|k.starts_with("GNU.sparse."));
    // Older formats have no major/minor. Keep each attribute family separate;
    // mixed declarations must never change the interpretation of body bytes.
    let version=if fields.iter().any(|(k,_)|k=="GNU.sparse.map"){PaxVersion::Map01}
        else if fields.iter().any(|(k,_)|matches!(k.as_str(),"GNU.sparse.major"|"GNU.sparse.minor"|"GNU.sparse.realsize")){PaxVersion::Text10}
        else if fields.iter().any(|(k,_)|matches!(k.as_str(),"GNU.sparse.size"|"GNU.sparse.numblocks"|"GNU.sparse.offset"|"GNU.sparse.numbytes")){PaxVersion::Pairs00}
        else{PaxVersion::Text10};
    let mut common=Vec::new();
    for(k,v)in fields{
        if k.starts_with("GNU.sparse."){
            let supported=match version{PaxVersion::Pairs00=>matches!(k.as_str(),"GNU.sparse.size"|"GNU.sparse.numblocks"|"GNU.sparse.offset"|"GNU.sparse.numbytes"),PaxVersion::Map01=>matches!(k.as_str(),"GNU.sparse.size"|"GNU.sparse.numblocks"|"GNU.sparse.name"|"GNU.sparse.map"),PaxVersion::Text10=>matches!(k.as_str(),"GNU.sparse.major"|"GNU.sparse.minor"|"GNU.sparse.name"|"GNU.sparse.realsize")};
            anyhow::ensure!(supported,"[archive-inspection:unsupported] 暂不支持或冲突的 GNU.sparse 属性 {k}");
        }else{common.push((k.clone(),v.clone()));}
    }
    super::tar_pax::common(&common)?;
    if sparse{
        let size_key=match version{
            PaxVersion::Pairs00|PaxVersion::Map01=>{anyhow::ensure!(decimal(attribute(fields,"GNU.sparse.numblocks")?)?<=65_536,"[archive-inspection:unsupported] GNU.sparse 段数超过支持范围");"GNU.sparse.size"},
            PaxVersion::Text10=>{anyhow::ensure!(decimal(attribute(fields,"GNU.sparse.major")?)?==1&&decimal(attribute(fields,"GNU.sparse.minor")?)?==0,"[archive-inspection:unsupported] GNU.sparse 部分取回目前支持 PAX 0.0 / 0.1 / 1.0");"GNU.sparse.realsize"}
        };
        anyhow::ensure!(decimal(attribute(fields,size_key)?)?<=MAX_EXTRACTED_BYTES,"[archive-inspection:unsupported] GNU.sparse 完整大小超过支持范围");
        if !matches!(version,PaxVersion::Pairs00){
            let name=attribute(fields,"GNU.sparse.name")?;
            anyhow::ensure!(!name.is_empty()&&!name.contains(['\0','\n']),"GNU.sparse 文件名无效");
        }
    }
    Ok(sparse.then_some(version))
}
struct TextMap<'a,R>{input:&'a mut R,block:[u8;512],cursor:usize,consumed:u64,physical:u64}
impl<R:Read> TextMap<'_,R>{
    fn number(&mut self)->Result<u64>{
        let mut digits=Vec::new();
        loop{
            if self.cursor==512{
                anyhow::ensure!(self.consumed+512<=self.physical&&self.consumed+512<=8*1024*1024,"GNU.sparse 文本映射超出条目边界或支持范围");
                self.input.read_exact(&mut self.block).context("GNU.sparse 文本映射未完整到达")?;
                self.consumed+=512;self.cursor=0;
            }
            let b=self.block[self.cursor];self.cursor+=1;
            if b==b'\n'{break;}
            anyhow::ensure!(b.is_ascii_digit()&&digits.len()<20,"GNU.sparse 文本映射数值无效");digits.push(b);
        }
        decimal(std::str::from_utf8(&digits)?)
    }
}

#[derive(Debug,Clone,Serialize)]
pub(crate) struct Layout { pub logical:u64, pub stored:u64, pub ranges:Vec<(u64,u64)> }
impl Layout {
    pub(crate) fn parse_pax00(fields:&super::tar_pax::Fields,physical:u64)->Result<Self>{
        let logical=decimal(attribute(fields,"GNU.sparse.size")?)?;
        let count=decimal(attribute(fields,"GNU.sparse.numblocks")?)?;
        anyhow::ensure!(count<=65_536,"[archive-inspection:unsupported] GNU.sparse 段数超过支持范围");
        // Preserve order and repetitions; a dictionary would silently lose all
        // but the final range. Common PAX attributes are not range components.
        let mut pieces=fields.iter().filter(|(k,_)|matches!(k.as_str(),"GNU.sparse.offset"|"GNU.sparse.numbytes"));
        let mut ranges=Vec::new();
        for _ in 0..count{
            let(offset,value)=pieces.next().context("GNU.sparse 映射缺少偏移")?;
            anyhow::ensure!(offset=="GNU.sparse.offset","GNU.sparse 映射偏移与长度顺序无效");let offset=decimal(value)?;
            let(length,value)=pieces.next().context("GNU.sparse 映射缺少长度")?;
            anyhow::ensure!(length=="GNU.sparse.numbytes","GNU.sparse 映射偏移与长度顺序无效");ranges.push((offset,decimal(value)?));
        }
        anyhow::ensure!(pieces.next().is_none(),"GNU.sparse 映射与声明段数不一致");
        Self::checked(logical,physical,ranges)
    }
    pub(crate) fn parse_pax01(fields:&super::tar_pax::Fields,physical:u64)->Result<Self>{
        let logical=decimal(attribute(fields,"GNU.sparse.size")?)?;
        let count=decimal(attribute(fields,"GNU.sparse.numblocks")?)?;
        anyhow::ensure!(count<=65_536,"[archive-inspection:unsupported] GNU.sparse 段数超过支持范围");
        let text=attribute(fields,"GNU.sparse.map")?;let mut values=text.split(',');let mut ranges=Vec::new();
        if count==0{anyhow::ensure!(text.is_empty(),"GNU.sparse 零段映射含多余内容");}
        else{
            for _ in 0..count{
                let offset=decimal(values.next().context("GNU.sparse 映射缺少偏移")?)?;
                let length=decimal(values.next().context("GNU.sparse 映射缺少长度")?)?;ranges.push((offset,length));
            }
            anyhow::ensure!(values.next().is_none(),"GNU.sparse 映射与声明段数不一致");
        }
        Self::checked(logical,physical,ranges)
    }
    pub(crate) fn parse(header:&tar::Header,input:&mut impl Read)->Result<Self>{
        let gnu=header.as_gnu().context("[archive-inspection:unsupported] 旧 GNU 稀疏条目缺少 GNU 文件头")?;
        let logical=gnu.real_size()?;let stored=header.entry_size()?;
        anyhow::ensure!(logical<=MAX_EXTRACTED_BYTES,"[archive-inspection:unsupported] 稀疏文件完整大小超过支持范围");
        let mut ranges=Vec::new();let mut unused=false;
        fn slots(slots:&[tar::GnuSparseHeader],ranges:&mut Vec<(u64,u64)>,unused:&mut bool)->Result<()>{
            for slot in slots {
                let empty_offset=slot.offset.iter().all(|b|*b==0);let empty_length=slot.numbytes.iter().all(|b|*b==0);
                if empty_offset&&empty_length{*unused=true;continue;}
                anyhow::ensure!(!empty_offset&&!empty_length&&!*unused,"TAR 稀疏映射包含不完整或乱序槽位");
                anyhow::ensure!(ranges.len()<65_536,"[archive-inspection:unsupported] TAR 稀疏段数超过支持范围");
                ranges.push((slot.offset()?,slot.length()?));
            }Ok(())
        }
        slots(&gnu.sparse,&mut ranges,&mut unused)?;
        let mut extended=gnu.isextended[0];let mut extensions=0;
        loop {
            anyhow::ensure!(extended<=1,"TAR 稀疏扩展标记无效");
            if extended==0{break;}
            anyhow::ensure!(!unused,"TAR 稀疏扩展链槽位不完整");
            anyhow::ensure!(extensions<4096,"[archive-inspection:unsupported] TAR 稀疏扩展链超过支持范围");
            let mut ext=tar::GnuExtSparseHeader::new();input.read_exact(ext.as_mut_bytes()).context("TAR 稀疏扩展块未完整到达")?;
            anyhow::ensure!(ext.padding.iter().all(|b|*b==0),"TAR 稀疏扩展填充无效");
            slots(ext.sparse(),&mut ranges,&mut unused)?;extended=ext.isextended[0];extensions+=1;
        }
        Self::checked(logical,stored,ranges)
    }
    pub(crate) fn parse_pax10(fields:&super::tar_pax::Fields,physical:u64,input:&mut impl Read)->Result<Self>{
        let logical=decimal(attribute(fields,"GNU.sparse.realsize")?)?;
        let mut map=TextMap{input,block:[0;512],cursor:512,consumed:0,physical};
        let count=map.number()?;
        anyhow::ensure!(count<=65_536,"[archive-inspection:unsupported] GNU.sparse 段数超过支持范围");
        let mut ranges=Vec::new();for _ in 0..count{ranges.push((map.number()?,map.number()?));}
        anyhow::ensure!(map.block[map.cursor..].iter().all(|b|*b==0),"GNU.sparse 文本映射填充无效");
        Self::checked(logical,physical-map.consumed,ranges)
    }
    fn checked(logical:u64,stored:u64,ranges:Vec<(u64,u64)>)->Result<Self>{
        let(mut previous,mut total,mut eof)=(0u64,0u64,false);
        for &(offset,length)in &ranges {
            let end=offset.checked_add(length).context("TAR 稀疏范围溢出")?;
            anyhow::ensure!(!eof&&offset>=previous&&end<=logical,"TAR 稀疏范围重叠、乱序或超出完整大小");
            if length==0{anyhow::ensure!(offset==logical,"TAR 稀疏结束段位置无效");eof=true;}
            total=total.checked_add(length).context("TAR 稀疏存储大小溢出")?;previous=end;
        }
        anyhow::ensure!(total==stored,"TAR 稀疏映射与包内数据大小不一致");
        Ok(Self{logical,stored,ranges})
    }
    /// Hash the reconstructed logical bytes. At output, seek over holes instead
    /// of writing zero buffers; source bytes remain a sequential stored stream.
    pub(crate) fn process(&self,input:&mut impl Read,mut output:Option<&mut File>,cancelled:&AtomicBool,mut progress:impl FnMut(u64))->Result<String>{
        if let Some(file)=output.as_deref_mut(){
            if self.logical>self.stored {mark_sparse(file)?;}
            file.set_len(self.logical)?;
        }
        let mut hash=Sha256::new();let mut position=0u64;let mut buffer=[0u8;65536];let zeros=[0u8;65536];let mut reported=0;
        let tail=(self.logical,0);
        for &(offset,length) in self.ranges.iter().chain(std::iter::once(&tail)) {
            while position<offset{
                check(cancelled)?;let n=(offset-position).min(zeros.len()as u64)as usize;hash.update(&zeros[..n]);position+=n as u64;
                if position-reported>=16*1024*1024{progress(position);reported=position;}
            }
            if let Some(file)=output.as_deref_mut(){file.seek(SeekFrom::Start(offset))?;}
            let mut remaining=length;
            while remaining>0{
                check(cancelled)?;let n=remaining.min(buffer.len()as u64)as usize;input.read_exact(&mut buffer[..n]).context("TAR 稀疏文件数据未完整到达")?;
                if let Some(file)=output.as_deref_mut(){file.write_all(&buffer[..n])?;}
                hash.update(&buffer[..n]);remaining-=n as u64;position+=n as u64;
                if position-reported>=16*1024*1024{progress(position);reported=position;}
            }
        }
        check(cancelled)?;progress(self.logical);Ok(format!("{:x}",hash.finalize()))
    }
}
#[cfg(windows)]
fn mark_sparse(file:&File)->Result<()>{
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::{IO::DeviceIoControl,Ioctl::FSCTL_SET_SPARSE};
    let mut returned=0;
    let ok=unsafe{DeviceIoControl(file.as_raw_handle(),FSCTL_SET_SPARSE,std::ptr::null(),0,std::ptr::null_mut(),0,&mut returned,std::ptr::null_mut())};
    if ok==0{return Err(std::io::Error::last_os_error()).context("保存位置无法建立稀疏文件，请改选支持稀疏文件的目录后重试");}
    Ok(())
}
#[cfg(not(windows))]
fn mark_sparse(_file:&File)->Result<()>{Ok(())}
