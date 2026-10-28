//! Old GNU S maps, consumed only inside the checksum-confirmed TAR prefix.
use anyhow::{Context,Result};
use serde::Serialize;
use sha2::{Digest,Sha256};
use std::{fs::File,io::{Read,Seek,SeekFrom,Write},sync::atomic::AtomicBool};
use super::{extraction_transaction::MAX_EXTRACTED_BYTES,lzop::recovery::check};

#[derive(Debug,Clone,Serialize)]
pub(crate) struct Layout { pub logical:u64, pub stored:u64, pub ranges:Vec<(u64,u64)> }
impl Layout {
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
