//! Opt-in synchronous preview scope. Other readers and writers retain defaults.
use std::{cell::RefCell, sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}}};
use crate::{Error,folder::Coder,SevenZMethod as M};

#[derive(Clone)]
struct Limits {
    bytes:u64, aes_power:u8, check:Arc<dyn Fn()->Result<(),Error>+Send+Sync>,
    aes:Arc<Mutex<Vec<Arc<AtomicBool>>>>,
}
thread_local! {static ACTIVE:RefCell<Option<Limits>>=const {RefCell::new(None)};}
struct Scope(Option<Limits>);
impl Drop for Scope {fn drop(&mut self){ACTIVE.with(|v|{v.replace(self.0.take());});}}
fn limits()->Option<Limits>{ACTIVE.with(|v|v.borrow().clone())}
pub(crate) fn active()->bool {ACTIVE.with(|v|v.borrow().is_some())}
pub(crate) fn check()->Result<(),Error>{if let Some(v)=limits(){(v.check)()?;}Ok(())}
pub(crate) fn size(bytes:u64)->Result<(),Error>{if let Some(v)=limits(){(v.check)()?;if bytes>v.bytes {return Err(Error::unsupported("7Z 预览声明大小超过 64 MiB，请先提取后打开"));}}Ok(())}
pub(crate) fn aes(properties:&[u8],password:&[u8])->Result<(),Error>{
    if let Some(v)=limits(){
        (v.check)()?;
        if properties.len()<2 || properties[0]&63>v.aes_power || password.len()>4096 {
            return Err(Error::unsupported("7Z 预览密码参数超过读取范围，请先提取后打开"));
        }
        let p=properties[0];let q=properties[1];let iv=((p>>6&1)+(q&15)) as usize;let salt=((p>>7&1)+(q>>4)) as usize;
        if properties.len()!=2+iv+salt {return Err(Error::unsupported("7Z 预览 AES 参数长度无法确认，请先提取检查"));}
    }Ok(())
}
pub(crate) fn coder(coder:&Coder,unpacked:usize)->Result<(),Error>{
    if !active(){return Ok(());}size(unpacked as u64)?;
    let id=coder.decompression_method_id();let p=&coder.properties;
    let valid=if id==M::ID_LZMA {p.len()==5&&u32::from_le_bytes(p[1..5].try_into().unwrap())<=64*1024*1024}
        else if id==M::ID_LZMA2 {p.len()==1&&p[0]<=28}
        else if id==M::ID_DELTA {p.len()==1}
        else if id==M::ID_BCJ_X86 {p.is_empty() || p.len()==4}
        else if id==M::ID_PPMD {p.len()==5&&u32::from_le_bytes(p[1..5].try_into().unwrap())<=64*1024*1024}
        // Retain the existing bounded-memory header codecs/filters. Selected
        // payload methods are checked separately by the application.
        else {id!=M::ID_ZSTD};
    if !valid || coder.num_in_streams!=1 || coder.num_out_streams!=1 {return Err(Error::unsupported("7Z 预览方法或字典超出读取范围，请先提取后打开"));}Ok(())
}
pub(crate) fn track_aes()->Arc<AtomicBool>{
    let flag=Arc::new(AtomicBool::new(false));
    if let Some(v)=limits(){v.aes.lock().unwrap().push(flag.clone());}flag
}

/// The scope never crosses an await. RAII restores the previous thread context,
/// including on error/panic, and every created AES stream must complete.
pub fn with_preview_limits<T,E:From<Error>>(
    bytes:u64,aes_power:u8,check:impl Fn()->Result<(),Error>+Send+Sync+'static,
    action:impl FnOnce()->Result<T,E>,
)->Result<T,E>{
    let current=Limits{bytes,aes_power,check:Arc::new(check),aes:Arc::new(Mutex::new(vec![]))};
    let _scope=Scope(ACTIVE.with(|v|v.replace(Some(current.clone()))));
    (current.check)().map_err(E::from)?;
    let value=action()?;
    (current.check)().map_err(E::from)?;
    if current.aes.lock().unwrap().iter().any(|v|!v.load(Ordering::Acquire)) {
        return Err(E::from(Error::other("7Z 预览 AES 数据未完整消费，请先提取检查")));
    }
    Ok(value)
}
