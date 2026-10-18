//! Native EGG IDs and .volN.egg framing; shared with the isolated helper.
use anyhow::{ensure, Context, Result};
use std::{collections::{BTreeMap, HashSet}, fs::File, io::{self, Read, Seek, SeekFrom}, path::{Path, PathBuf}};

#[derive(Debug, Clone)]
pub struct Group {
    pub first: PathBuf, pub parts: Vec<PathBuf>, pub missing: Vec<PathBuf>,
    pub total: usize, pub bytes: u64, pub stem: String,
}
#[derive(Clone)]
struct Header { id: u32, prev: u32, next: u32, split: bool, start: u64, size: u64 }
fn named(path: &Path) -> Option<(String, usize)> {
    let name = path.file_name()?.to_str()?;
    let lower = name.to_ascii_lowercase();
    let p = lower.strip_suffix(".egg")?.rfind(".vol")?;
    let digits = &lower[p+4..lower.len()-4];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) { return None; }
    Some((name[..p].to_string(), digits.parse().ok()?))
}
pub fn is_candidate(path: &Path) -> bool {
    let mut head = [0;4];
    File::open(path).and_then(|mut f| f.read_exact(&mut head)).is_ok() && &head == b"EGGA"
        || named(path).is_some()
}
fn header(path: &Path) -> Result<Header> {
    let mut f = File::open(path).with_context(|| format!("[archive-inspection:io] [egg-io] 无法打开 EGG 卷：{}", path.display()))?;
    let size = f.metadata()?.len();
    ensure!(size >= 22, "[archive-inspection:damaged] EGG 卷头/尾不完整：{}", path.display());
    let mut head = [0;14]; f.read_exact(&mut head)?;
    ensure!(&head[..4] == b"EGGA", "[archive-inspection:damaged] EGG 卷头损坏或混入其他文件：{}", path.display());
    ensure!(u16::from_le_bytes(head[4..6].try_into()?) == 256, "[archive-inspection:unsupported] EGG 版本当前不受支持");
    let mut h = Header { id:u32::from_le_bytes(head[6..10].try_into()?), prev:0,next:0,split:false,start:0,size };
    loop {
        let mut sig = [0;4]; f.read_exact(&mut sig).context("[archive-inspection:damaged] EGG 卷前缀截断")?;
        if u32::from_le_bytes(sig) == 0x08E28222 { h.start = f.stream_position()?; break; }
        let mut flag = [0]; f.read_exact(&mut flag)?;
        let len = if flag[0]&1 != 0 { let mut b=[0;4];f.read_exact(&mut b)?;u32::from_le_bytes(b) as u64 }
            else { let mut b=[0;2];f.read_exact(&mut b)?;u16::from_le_bytes(b) as u64 };
        let start=f.stream_position()?;
        ensure!(start.checked_add(len).is_some_and(|v| v <= size), "[archive-inspection:damaged] EGG 卷前缀超出文件范围");
        if u32::from_le_bytes(sig) == 0x24F5A262 {
            ensure!(len >= 8 && !h.split, "[archive-inspection:damaged] EGG 分卷链接不完整或重复");
            let mut ids=[0;8];f.read_exact(&mut ids)?;
            h.prev=u32::from_le_bytes(ids[..4].try_into()?);h.next=u32::from_le_bytes(ids[4..].try_into()?);h.split=true;
        }
        f.seek(SeekFrom::Start(start+len))?;
    }
    Ok(h)
}
pub fn inspect(path: &Path) -> Result<Option<Group>> {
    if !is_candidate(path) { return Ok(None); }
    let parent=path.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let mut members=BTreeMap::new();
    let (stem, split_name)=if let Some((stem, n))=named(path) {
        ensure!((1..=1000).contains(&n), "[archive-inspection:unsupported] EGG 原生卷仅支持 1 至 1000 卷");
        for e in std::fs::read_dir(parent)? {
            let e=e?; let p=e.path();
            if let Some((s,n))=named(&p) { if s.eq_ignore_ascii_case(&stem) {
                ensure!((1..=1000).contains(&n), "[archive-inspection:unsupported] EGG 卷号超出范围");
                ensure!(e.file_type()?.is_file() && members.insert(n,p).is_none(), "[archive-inspection:damaged] EGG 卷号重复或不是文件");
            } }
        }
        (stem,true)
    } else { members.insert(1,path.to_path_buf()); (path.file_stem().unwrap_or_default().to_string_lossy().to_string(),false) };
    let highest=*members.keys().next_back().context("EGG 原卷不存在")?;
    let mut missing=(1..=highest).filter(|n|!members.contains_key(n)).map(|n|parent.join(format!("{stem}.vol{n}.egg"))).collect::<Vec<_>>();
    let mut headers=BTreeMap::new();let mut bytes=0u64;let mut ids=HashSet::new();
    for (&n,p) in &members {
        let h=header(p)?;
        ensure!(ids.insert(h.id), "[archive-inspection:damaged] EGG 卷 ID 重复，可能混卷");
        ensure!(!split_name || h.split, "[archive-inspection:damaged] EGG 卷名为分卷，但没有原生分卷链接");
        ensure!(n != 1 || h.prev == 0, "[archive-inspection:damaged] EGG 首卷前向链接不为零，可能混卷");
        ensure!(n == 1 || h.prev != 0, "[archive-inspection:damaged] EGG 中间卷声明自己是首卷，可能混卷");
        ensure!(n == highest || h.next != 0, "[archive-inspection:damaged] EGG 已结束卷后仍有同名卷，可能混卷");
        bytes=bytes.checked_add(h.size).context("EGG 总大小溢出")?;headers.insert(n,h);
    }
    for (&n,h) in &headers { if let Some(next)=headers.get(&(n+1)) {
        ensure!(h.next==next.id && next.prev==h.id, "[archive-inspection:damaged] EGG 相邻卷 ID/双向链接不符，可能混卷");
    }
        if h.next!=0 && ids.contains(&h.next) {ensure!(headers.get(&(n+1)).is_some_and(|v|v.id==h.next),"[archive-inspection:damaged] EGG 下一卷链接形成循环或跳过卷号");}
        if h.prev!=0 && ids.contains(&h.prev) {ensure!(headers.get(&(n-1)).is_some_and(|v|v.id==h.prev),"[archive-inspection:damaged] EGG 上一卷链接形成循环或跳过卷号");}
    }
    let last=&headers[&highest];
    if last.next!=0 { missing.push(parent.join(format!("{stem}.vol{}.egg",highest+1))); }
    if !split_name {
        ensure!(headers[&1].prev==0, "[archive-inspection:incomplete] 这不是 EGG 首卷；请保留原始 .volN.egg 名称并补齐首卷");
        ensure!(!headers[&1].split || last.next==0, "[archive-inspection:incomplete] EGG 原生分卷名称已丢失；请恢复原始 .volN.egg 卷组后重试");
    }
    if missing.is_empty() {
        let last_path=members.get(&highest).unwrap();let mut f=File::open(last_path)?;f.seek(SeekFrom::End(-4))?;
        let mut tail=[0;4];f.read_exact(&mut tail)?;
        ensure!(u32::from_le_bytes(tail)==0x08E28222, "[archive-inspection:damaged] EGG 末卷缺少结束标记，文件可能下载不完整");
    }
    let first=members.get(&1).cloned().unwrap_or_else(||parent.join(format!("{stem}.vol1.egg")));
    Ok(Some(Group{first,parts:members.into_values().collect(),total:highest+usize::from(last.next!=0),missing,bytes,stem}))
}
pub fn complete(path: &Path) -> Result<Group> {
    let group=inspect(path)?.context("[archive-inspection:damaged] EGG 文件头无效或截断")?;
    ensure!(group.missing.is_empty(), "[archive-inspection:incomplete] 缺少 EGG 原生分卷：{}；请补齐原卷后重试，不必反复尝试密码",group.missing.iter().map(|p|p.file_name().unwrap_or_default().to_string_lossy()).collect::<Vec<_>>().join("、"));
    Ok(group)
}
pub struct Reader { segments:Vec<(File,u64,u64,u64)>,pos:u64,len:u64 }
impl Reader {
    pub fn open(group:&Group)->Result<Self> {
        let mut segments=Vec::new();let mut len=0u64;
        for (i,p) in group.parts.iter().enumerate() {
            let h=header(p)?;let skip=if i==0 {0} else {h.start};
            segments.push((File::open(p)?,skip,len,h.size-skip));len+=h.size-skip;
        }
        Ok(Self{segments,pos:0,len})
    }
    pub fn len(&self)->u64 { self.len }
}
impl Read for Reader {
    fn read(&mut self,b:&mut[u8])->io::Result<usize> {
        if self.pos>=self.len {return Ok(0)}
        let s=self.segments.iter_mut().find(|s|self.pos>=s.2&&self.pos<s.2+s.3).ok_or_else(||io::Error::other("EGG invalid segment"))?;
        s.0.seek(SeekFrom::Start(s.1+self.pos-s.2))?;
        let n=(s.2+s.3-self.pos).min(b.len() as u64) as usize;let n=s.0.read(&mut b[..n])?;
        if n==0&&!b.is_empty(){return Err(io::Error::new(io::ErrorKind::UnexpectedEof,"EGG volume truncated during reading"))}
        self.pos+=n as u64;Ok(n)
    }
}
impl Seek for Reader {
    fn seek(&mut self,p:SeekFrom)->io::Result<u64> {
        let n=match p {SeekFrom::Start(v)=>v as i128,SeekFrom::Current(v)=>self.pos as i128+v as i128,SeekFrom::End(v)=>self.len as i128+v as i128};
        if !(0..=self.len as i128).contains(&n){return Err(io::Error::new(io::ErrorKind::InvalidInput,"EGG seek beyond source"))}
        self.pos=n as u64;Ok(self.pos)
    }
}
