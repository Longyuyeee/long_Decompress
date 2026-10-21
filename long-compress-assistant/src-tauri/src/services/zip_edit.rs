//! Session-based ZIP add/replace/save-as. No in-place updates or implicit conflict decisions.
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, HashMap, HashSet}, fs::{File, OpenOptions}, io::{Read, Write, Seek, SeekFrom}, path::{Path, PathBuf}, sync::{Arc, atomic::{AtomicBool, Ordering}}};
use zip_aes::{ZipArchive, ZipWriter, CompressionMethod, HasZipMetadata, write::{SimpleFileOptions, FullFileOptions}};
use zip_aes::unstable::write::FileOptionsExt;
use super::zip_password::ZipPasswordEncoding;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum Encryption { #[default] None, Zipcrypto, Aes128, Aes192, Aes256 }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum OutputMode { #[default] Keep, None, Zipcrypto, Aes256 }
#[derive(Clone, Default, Deserialize)]
#[serde(rename_all="camelCase",default)]
pub struct Credentials {pub source_password:String,pub source_encoding:Option<ZipPasswordEncoding>,pub output_mode:OutputMode,pub output_password:String,pub output_encoding:ZipPasswordEncoding}
fn entry_encryption<R:Read>(entry:&zip_aes::read::ZipFile<'_,R>)->Encryption {
    if !entry.encrypted(){return Encryption::None}
    match entry.get_metadata().aes_mode.map(|(mode,_,_)|mode){Some(zip_aes::AesMode::Aes128)=>Encryption::Aes128,Some(zip_aes::AesMode::Aes192)=>Encryption::Aes192,Some(zip_aes::AesMode::Aes256)=>Encryption::Aes256,None=>Encryption::Zipcrypto}
}
fn output_encryption(mode:OutputMode,original:Encryption)->Encryption{match mode{OutputMode::Keep=>original,OutputMode::None=>Encryption::None,OutputMode::Zipcrypto=>Encryption::Zipcrypto,OutputMode::Aes256=>Encryption::Aes256}}
fn write_options<'a>(method:CompressionMethod,size:u64,date:Option<zip_aes::DateTime>,permissions:Option<u32>,comment:&str,encryption:Encryption,password:&'a[u8],level:Option<i64>)->Result<FullFileOptions<'a>>{
    let mut options=SimpleFileOptions::default().compression_method(method).compression_level(level).large_file(size>=u32::MAX as u64).last_modified_time(date.filter(zip_aes::DateTime::is_valid).unwrap_or_default()).into_full_options().with_file_comment(comment.to_string());
    if let Some(mode)=permissions{options=options.unix_permissions(mode);}
    Ok(match encryption{Encryption::None=>options,Encryption::Zipcrypto=>options.with_deprecated_encryption(password)?,Encryption::Aes128=>options.with_aes_encryption_bytes(zip_aes::AesMode::Aes128,password),Encryption::Aes192=>options.with_aes_encryption_bytes(zip_aes::AesMode::Aes192,password),Encryption::Aes256=>options.with_aes_encryption_bytes(zip_aes::AesMode::Aes256,password)})
}
const MAX_ADDITIONS: usize = 10_000;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Entry { pub path: String, pub is_dir: bool, pub size: u64 }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Catalog { pub revision: String, pub entries: Vec<Entry>, pub methods: Vec<String>, pub encrypted:bool, pub default_encryption:Encryption, pub encryptions:HashMap<String,Encryption> }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Input { pub source_path: String, pub target_path: String, pub is_dir: bool, pub size: u64, pub source_revision: String }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Change { #[serde(flatten)] pub input: Input, pub action: String, #[serde(default)] pub replaces_path:Option<String> }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct OriginalEdit { pub source_path: String, pub target_path: Option<String> }
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Progress { pub phase: &'static str, pub progress: f32, pub current_file: String, pub processed_bytes: u64, pub total_bytes: u64, pub output_bytes: Option<u64> }
pub type Observer = Arc<dyn Fn(Progress) + Send + Sync>;
fn progress(observer: &Observer, phase: &'static str, fraction: f32, file: &str, processed: u64, total: u64, output: Option<u64>) {
    observer(Progress {phase,progress:fraction,current_file:file.into(),processed_bytes:processed,total_bytes:total,output_bytes:output});
}
fn checkpoint(flag: &Arc<AtomicBool>) -> Result<()> {
    super::task_control::wait_if_paused(flag);
    ensure!(!flag.load(Ordering::SeqCst), "ZIP_EDIT_CANCELLED：已停止，原包和待保存修改保留"); Ok(())
}
pub fn normalized_path(path: &str, directory: bool) -> Result<String> {
    let value=path.replace('\\',"/"); let value=value.trim_end_matches('/');
    ensure!(!value.is_empty() && !value.starts_with('/'), "包内位置无效：{path}");
    for part in value.split('/') {
        ensure!(!part.is_empty() && part!="." && part!=".." && !part.ends_with(['.',' ']) && !part.chars().any(|c|c.is_control()||"<>:\"|?*".contains(c)), "包内名称不能直接编辑：{path}");
        let stem=part.split('.').next().unwrap_or("").to_uppercase();
        ensure!(!["CON","PRN","AUX","NUL","COM1","COM2","COM3","COM4","COM5","COM6","COM7","COM8","COM9","LPT1","LPT2","LPT3","LPT4","LPT5","LPT6","LPT7","LPT8","LPT9"].contains(&stem.as_str()), "包内名称与系统保留名冲突：{path}");
    }
    Ok(format!("{value}{}",if directory {"/"}else{""}))
}
fn key(path: &str) -> String { path.trim_end_matches('/').to_lowercase() }
fn file_revision(path: &Path, directory: bool) -> Result<String> {
    let metadata=std::fs::symlink_metadata(path).with_context(||format!("找不到来源：{}",path.display()))?;
    ensure!(!metadata.file_type().is_symlink() && (if directory {metadata.is_dir()} else {metadata.is_file()}), "来源不再是原来的普通文件/目录：{}",path.display());
    Ok(if directory {"directory".into()} else {format!("{}:{:?}",metadata.len(),metadata.modified()?)})
}
// zip-rs indexes equal raw names by their last occurrence. Compare EOCD count so no older item can disappear silently.
pub(crate) fn declared_count(file: &mut File) -> Result<u64> {
    declared_count_reader(file)
}
pub(crate) fn declared_count_reader<R: Read+Seek>(file: &mut R) -> Result<u64> {
    let length=file.seek(SeekFrom::End(0))?; let take=length.min(65557);file.seek(SeekFrom::End(-(take as i64)))?;
    let mut tail=vec![0;take as usize];file.read_exact(&mut tail)?;
    let at=(0..tail.len().saturating_sub(21)).rev().find(|&at|tail[at..].starts_with(b"PK\x05\x06")&&at+22+u16::from_le_bytes(tail[at+20..at+22].try_into().unwrap()) as usize==tail.len()).context("ZIP 目录不完整，不能编辑")?;
    let u16at=|offset|u16::from_le_bytes(tail[at+offset..at+offset+2].try_into().unwrap());
    ensure!(u16at(4)==0&&u16at(6)==0&&u16at(8)==u16at(10),"分卷 ZIP 请继续使用浏览/提取；本阶段编辑支持独立 ZIP");
    let count=u16at(10);if count!=u16::MAX {return Ok(count as u64)}
    let eocd=length-take+at as u64;ensure!(eocd>=20,"ZIP64 目录不完整");file.seek(SeekFrom::Start(eocd-20))?;
    let mut locator=[0;20];file.read_exact(&mut locator)?;ensure!(&locator[..4]==b"PK\x06\x07"&&u32::from_le_bytes(locator[4..8].try_into()?)==0&&u32::from_le_bytes(locator[16..20].try_into()?)==1,"ZIP64 分卷暂不支持编辑");
    file.seek(SeekFrom::Start(u64::from_le_bytes(locator[8..16].try_into()?)))?;let mut record=[0;56];file.read_exact(&mut record)?;
    ensure!(&record[..4]==b"PK\x06\x06"&&record[16..24]==[0;8],"ZIP64 目录不完整或不是独立 ZIP");
    let count=u64::from_le_bytes(record[32..40].try_into()?);ensure!(count==u64::from_le_bytes(record[24..32].try_into()?),"ZIP64 分卷暂不支持编辑");Ok(count)
}
fn register_entry(map: &mut BTreeMap<String,Entry>, entry: Entry) -> Result<()> {
    let parts:Vec<_>=entry.path.trim_end_matches('/').split('/').collect();
    for i in 1..parts.len() {let path=format!("{}/",parts[..i].join("/"));let k=key(&path);
        if let Some(old)=map.get(&k) {ensure!(old.is_dir && old.path==path,"包内文件/目录或大小写位置冲突：{path}");}
        else {map.insert(k,Entry{path,is_dir:true,size:0});}
    }
    let k=key(&entry.path);if let Some(old)=map.get(&k) {ensure!(old.is_dir&&entry.is_dir&&old.path==entry.path,"包内同名或大小写冲突：{}",entry.path);}
    map.insert(k,entry);Ok(())
}
fn open_catalog(path: &Path) -> Result<(ZipArchive<File>,Catalog)> {
    let reader=super::numeric_volume_reader::ZipInputReader::open(path)?;
    let mut file=match reader {super::numeric_volume_reader::ZipInputReader::File(file)=>file,_=>anyhow::bail!("本阶段编辑支持独立 ZIP；分卷和自解压请继续使用浏览/提取")};
    let count=declared_count(&mut file)?;let mut archive=ZipArchive::new(file).context("不是可编辑的完整 ZIP")?;
    ensure!(archive.offset()==0,"自解压/带前置数据的 ZIP 暂不支持编辑");
    ensure!(archive.len() as u64==count,"ZIP 包内含重复名称，不能在另存时丢弃旧内容；请先提取整理");
    ensure!(archive.len()<=200_000,"包内条目过多，请先提取需要整理的部分");
    let mut map=BTreeMap::new();let mut names=HashSet::new();let mut methods=HashSet::new();let mut encryptions=HashMap::new();let mut digest=Sha256::new();digest.update(file_revision(path,false)?.as_bytes());
    for index in 0..archive.len() {
        let file=archive.by_index_raw(index)?;
        let mode=file.unix_mode().unwrap_or(0)&0o170000;ensure!(mode==0||mode==0o100000||mode==0o040000,"ZIP 含链接或特殊条目，暂不支持编辑：{}",file.name());
        ensure!(matches!(file.compression(),CompressionMethod::Stored|CompressionMethod::Deflated),"本阶段编辑支持原方法为 Store/Deflate 的普通 ZIP；条目 {} 使用 {:?}，仍可浏览/提取",file.name(),file.compression());
        let path=normalized_path(file.name(),file.is_dir())?;ensure!(names.insert(key(&path)),"ZIP 包内同名或大小写冲突：{path}");
        digest.update(format!("{}:{:?}:{}:{}:{}:{:?}:{:?}",path,file.compression(),file.size(),file.compressed_size(),file.crc32(),file.last_modified(),file.unix_mode()).as_bytes());
        encryptions.insert(path.clone(),entry_encryption(&file));methods.insert(format!("{:?}",file.compression()));register_entry(&mut map,Entry{path,is_dir:file.is_dir(),size:file.size()})?;
    }
    let mut methods:Vec<_>=methods.into_iter().collect();methods.sort();
    let schemes:HashSet<_>=encryptions.values().copied().filter(|scheme|*scheme!=Encryption::None).collect();
    let default_encryption=if schemes.len()==1{*schemes.iter().next().unwrap()}else if schemes.is_empty(){Encryption::None}else{Encryption::Aes256};
    Ok((archive,Catalog{revision:format!("{:x}",digest.finalize()),entries:map.into_values().collect(),methods,encrypted:!schemes.is_empty(),default_encryption,encryptions}))
}
pub fn inspect(path: &Path) -> Result<Catalog> {Ok(open_catalog(path)?.1)}
pub fn prepare(sources: &[String], directory: &str) -> Result<Vec<Input>> {
    ensure!(!sources.is_empty()&&sources.len()<=1000,"请选择需要添加的文件或目录");
    let prefix=if directory.is_empty(){String::new()}else{normalized_path(directory,true)?};let mut inputs=Vec::new();
    fn walk(source: &Path,target: String,inputs:&mut Vec<Input>)->Result<()> {
        ensure!(inputs.len()<MAX_ADDITIONS,"一次最多添加 10000 项，请分批整理");
        let metadata=std::fs::symlink_metadata(source)?;let is_dir=metadata.is_dir();let source_revision=file_revision(source,is_dir)?;let target_path=normalized_path(&target,is_dir)?;
        inputs.push(Input{source_path:source.to_string_lossy().into_owned(),target_path:target_path.clone(),is_dir,size:if is_dir{0}else{metadata.len()},source_revision});
        if is_dir {let mut children=std::fs::read_dir(source)?.map(|e|e.map(|e|e.path())).collect::<std::io::Result<Vec<_>>>()?;children.sort();
            for child in children {let name=child.file_name().context("来源名称无效")?.to_str().context("来源文件名不能表示为 Unicode")?;walk(&child,format!("{target_path}{name}"),inputs)?;}
        }Ok(())
    }
    for source in sources {let source=Path::new(source);let name=source.file_name().context("请选择文件/目录而不是磁盘根目录")?.to_str().context("来源名称无效")?;walk(source,format!("{prefix}{name}"),&mut inputs)?;}
    Ok(inputs)
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct InputIssue {pub index:usize,pub issue:String}
pub fn check_inputs(inputs:&[Input])->Result<Vec<InputIssue>>{
    ensure!(inputs.len()<=MAX_ADDITIONS,"编辑来源超过 10000 项");
    Ok(inputs.iter().enumerate().filter_map(|(index,input)|match file_revision(Path::new(&input.source_path),input.is_dir){Ok(revision)if revision==input.source_revision=>None,Ok(_)=>Some(InputIssue{index,issue:"来源已变化，请重新读取或重新定位后确认".into()}),Err(_)=>Some(InputIssue{index,issue:"来源缺失或无法访问，请重新定位".into()})}).collect())
}
struct ControlledFile { file:File, flag:Arc<AtomicBool>, observer:Option<Observer>, position:u64, high:u64, estimate:u64 }
impl ControlledFile {
    fn check(&self)->std::io::Result<()>{checkpoint(&self.flag).map_err(|e|std::io::Error::new(std::io::ErrorKind::Other,e.to_string()))}
}
impl Read for ControlledFile {fn read(&mut self,b:&mut[u8])->std::io::Result<usize>{self.check()?;self.file.read(b)}}
impl Seek for ControlledFile {fn seek(&mut self,p:SeekFrom)->std::io::Result<u64>{self.check()?;let p=self.file.seek(p)?;self.position=p;Ok(p)}}
impl Write for ControlledFile {
    fn write(&mut self,b:&[u8])->std::io::Result<usize>{self.check()?;let n=self.file.write(b)?;self.position+=n as u64;self.high=self.high.max(self.position);if let Some(observer)=&self.observer{progress(observer,"writing",0.3+0.4*(self.high as f64/self.estimate.max(1) as f64).min(1.0) as f32,"",self.high,self.estimate,Some(self.high));}Ok(n)}
    fn flush(&mut self)->std::io::Result<()>{self.file.flush()}
}
struct Staging(PathBuf);impl Drop for Staging {fn drop(&mut self){let _=std::fs::remove_file(&self.0);}}
fn validate_contents<R:Read+Seek>(archive:&mut ZipArchive<R>,flag:&Arc<AtomicBool>,observer:&Observer,phase:&'static str,start:f32,weight:f32,password:&str,encoding:Option<ZipPasswordEncoding>)->Result<HashMap<String,(u64,u32)>> {
    let total=(0..archive.len()).try_fold(0u64,|n,i|->Result<_>{Ok(n.saturating_add(archive.by_index_raw(i)?.size()))})?;
    let mut processed=0u64;let mut map=HashMap::new();let mut buffer=vec![0u8;256*1024];
    for index in 0..archive.len(){checkpoint(flag)?;let mut entry=super::zip_password::open_entry_with_encoding(archive,index,password,encoding).context("无法读取 ZIP 条目，请核对原包密码与密码编码")?;let name=normalized_path(entry.name(),entry.is_dir())?;let mut crc=crc32fast::Hasher::new();
        loop {checkpoint(flag)?;let n=entry.read(&mut buffer).with_context(||format!("条目无法完整读取：{name}"))?;if n==0{break}crc.update(&buffer[..n]);processed+=n as u64;progress(observer,phase,start+weight*(processed as f64/total.max(1) as f64) as f32,&name,processed,total,None);}
        ensure!(map.insert(name,(entry.size(),crc.finalize())).is_none(),"新包存在重复条目");
    }progress(observer,phase,start+weight,"",processed,total,None);Ok(map)
}
#[cfg(test)]
pub fn save(source:&Path,revision:&str,changes:&[Change],output:&Path,method:&str,level:u32,flag:Arc<AtomicBool>,observer:Observer)->Result<()> {
    save_with_edits(source,revision,changes,&[],output,method,level,flag,observer)
}
#[cfg(test)]
pub fn save_with_edits(source:&Path,revision:&str,changes:&[Change],original_edits:&[OriginalEdit],output:&Path,method:&str,level:u32,flag:Arc<AtomicBool>,observer:Observer)->Result<()> {
    save_encrypted(source,revision,changes,original_edits,output,method,level,&Credentials::default(),flag,observer)
}
pub fn save_encrypted(source:&Path,revision:&str,changes:&[Change],original_edits:&[OriginalEdit],output:&Path,method:&str,level:u32,credentials:&Credentials,flag:Arc<AtomicBool>,observer:Observer)->Result<()> {
    ensure!((!changes.is_empty()||!original_edits.is_empty()||credentials.output_mode!=OutputMode::Keep)&&changes.len()<=MAX_ADDITIONS&&original_edits.len()<=200_000,"没有需要保存的修改，或一次修改超过支持范围");
    ensure!(matches!(method,"deflate"|"store")&&level<=9,"新增内容请使用 Deflate 0～9 或仅存储");
    ensure!(output.extension().is_some_and(|s|s.eq_ignore_ascii_case("zip")),"请另存为 .zip 文件");
    ensure!(!output.exists(),"[archive-output:Pre-checking:output-conflict] 输出文件已存在，请更换名称或保存位置；已有文件保留");
    let parent=output.parent().context("请选择新包保存位置")?;ensure!(parent.is_dir(),"输出目录不存在，请选择可用目录");
    let source_absolute=source.canonicalize()?;let target_absolute=parent.canonicalize()?.join(output.file_name().context("新包名称为空")?);
    ensure!(!source_absolute.to_string_lossy().eq_ignore_ascii_case(&target_absolute.to_string_lossy()),"请另存新名称，不能覆盖原压缩包");
    let (_,catalog)=open_catalog(source)?;ensure!(catalog.revision==revision,"原包已变化，请保留修改清单，重新打开并确认来源后再编辑");
    ensure!(!catalog.encrypted||!credentials.source_password.is_empty(),"请填写原包密码，再保存密码 ZIP；修改清单保留");
    let output_password=match credentials.output_mode{OutputMode::Keep=>credentials.source_password.as_str(),OutputMode::None=>"",_=>{ensure!(!credentials.output_password.is_empty(),"请填写新包密码，或明确选择不加密");credentials.output_password.as_str()}};
    let output_bytes=if !output_password.is_empty(){super::zip_password::encode(output_password,credentials.output_encoding)?}else{Vec::new()};
    let originals:HashMap<_,_>=catalog.entries.iter().map(|e|(e.path.as_str(),e)).collect();
    let mut edits=HashMap::new();
    for edit in original_edits {
        let original=originals.get(edit.source_path.as_str()).with_context(||format!("整理来源已变化，请重新读取：{}",edit.source_path))?;
        if let Some(target)=&edit.target_path {ensure!(normalized_path(target,original.is_dir)?==*target,"新包内路径不规范：{target}");}
        ensure!(edits.insert(edit.source_path.as_str(),edit.target_path.as_deref()).is_none(),"同一原条目重复整理：{}",edit.source_path);
    }
    let mut expected=BTreeMap::new();
    for entry in &catalog.entries {
        let target=edits.get(entry.path.as_str()).copied().unwrap_or(Some(entry.path.as_str()));
        if let Some(path)=target {register_entry(&mut expected,Entry{path:path.into(),is_dir:entry.is_dir,size:entry.size})?;}
    }
    let mut targets=HashSet::new();
    for change in changes {let input=&change.input;let normalized=normalized_path(&input.target_path,input.is_dir)?;ensure!(normalized==input.target_path,"修改目标不是规范包内位置");ensure!(targets.insert(key(&normalized)),"多个修改指向相同位置：{normalized}");
        ensure!(file_revision(Path::new(&input.source_path),input.is_dir)?==input.source_revision,"添加来源已变化，请撤销并重新添加：{}",input.source_path);
        let old=expected.get(&key(&normalized));match change.action.as_str(){"replace"=>ensure!(old.is_some_and(|e|!e.is_dir&&!input.is_dir&&e.path==normalized),"替换位置已变化，请重新确认：{normalized}"),"add"=>ensure!(old.is_none(),"同名项尚未处理：{normalized}"),_=>anyhow::bail!("请选择替换、跳过或保留两份")};
        if change.action=="replace"{expected.remove(&key(&normalized));}
        register_entry(&mut expected,Entry{path:normalized,is_dir:input.is_dir,size:input.size})?;
    }
    if (catalog.encrypted&&credentials.output_mode==OutputMode::Keep)||matches!(credentials.output_mode,OutputMode::Zipcrypto|OutputMode::Aes256){ensure!(expected.values().any(|entry|!entry.is_dir),"空 ZIP 没有可加密文件，请明确选择新包不加密后另存");}
    checkpoint(&flag)?;
    let mut original=ZipArchive::new(ControlledFile{file:File::open(source)?,flag:flag.clone(),observer:None,position:0,high:0,estimate:0})?;
    let original_facts=validate_contents(&mut original,&flag,&observer,"reading",0.0,0.3,&credentials.source_password,credentials.source_encoding)?;
    let staged=Staging(super::output_publish_transaction::staged_output_path(output,"zip-edit")?);
    let estimate=source.metadata()?.len().saturating_add(changes.iter().fold(0u64,|n,e|n.saturating_add(e.input.size)));
    let file=OpenOptions::new().create_new(true).write(true).read(true).open(&staged.0)?;
    let mut writer=ZipWriter::new(ControlledFile{file,flag:flag.clone(),observer:Some(observer.clone()),position:0,high:0,estimate});writer.set_raw_comment(original.comment().to_vec().into_boxed_slice())?;
    let replacements:HashSet<_>=changes.iter().filter(|e|e.action=="replace").map(|e|key(&e.input.target_path)).collect();
    let mut retained_facts=HashMap::new();let mut written_paths=HashSet::new();
    for index in 0..original.len(){checkpoint(&flag)?;let entry=original.by_index_raw(index)?;let old_path=normalized_path(entry.name(),entry.is_dir())?;
        let target=edits.get(old_path.as_str()).copied().unwrap_or(Some(old_path.as_str()));
        if let Some(path)=target {if !replacements.contains(&key(path)){
            retained_facts.insert(path.to_string(),original_facts[&old_path]);written_paths.insert(path.to_string());
            progress(&observer,"writing",0.3,path,0,estimate,None);
            let encryption=output_encryption(credentials.output_mode,entry_encryption(&entry));
            if !entry.encrypted()&&(encryption==Encryption::None||entry.is_dir()) {writer.raw_copy_file_rename(entry,path)?;}
            else {
                let method=entry.compression();let size=entry.size();let date=entry.last_modified();let permissions=entry.unix_mode();let comment=entry.comment().to_string();let is_dir=entry.is_dir();drop(entry);
                let mut decoded=super::zip_password::open_entry_with_encoding(&mut original,index,&credentials.source_password,credentials.source_encoding).context("无法解锁原条目，请核对密码和编码")?;
                let options=write_options(method,size,date,permissions,&comment,if is_dir&&credentials.output_mode!=OutputMode::Keep{Encryption::None}else{encryption},&output_bytes,None)?;
                writer.start_file(path,options)?;let mut block=vec![0u8;256*1024];loop{checkpoint(&flag)?;let n=decoded.read(&mut block)?;if n==0{break}writer.write_all(&block[..n])?;}
            }
        }}
    }
    let mut buffer=vec![0u8;256*1024];
    for change in changes {checkpoint(&flag)?;let input=&change.input;progress(&observer,"writing",0.3,&input.target_path,0,estimate,None);
        written_paths.insert(input.target_path.clone());
        let metadata=std::fs::metadata(&input.source_path)?;
        let date=chrono::DateTime::<chrono::Local>::from(metadata.modified()?);
        use chrono::{Datelike,Timelike};
        let timestamp=zip_aes::DateTime::from_date_and_time(date.year().clamp(1980,2107) as u16,date.month() as u8,date.day() as u8,date.hour() as u8,date.minute() as u8,date.second() as u8).unwrap_or_default();
        let inherited=change.replaces_path.as_ref().map(|path|catalog.encryptions.get(path).copied().with_context(||format!("替换原位置已变化：{path}"))).transpose()?.unwrap_or(catalog.default_encryption);
        let encryption=if input.is_dir{Encryption::None}else{output_encryption(credentials.output_mode,inherited)};
        let options=write_options(if method=="store"{CompressionMethod::Stored}else{CompressionMethod::Deflated},input.size,Some(timestamp),None,"",encryption,&output_bytes,if method=="store"{None}else{Some(level as i64)})?;
        if input.is_dir {writer.add_directory(&input.target_path,options)?;continue}
        ensure!(file_revision(Path::new(&input.source_path),false)?==input.source_revision,"添加来源已变化，请重新添加：{}",input.source_path);
        writer.start_file(&input.target_path,options)?;let mut file=File::open(&input.source_path)?;let mut copied=0u64;
        loop{checkpoint(&flag)?;let n=file.read(&mut buffer)?;if n==0{break}writer.write_all(&buffer[..n])?;copied+=n as u64;}
        ensure!(copied==input.size&&file_revision(Path::new(&input.source_path),false)?==input.source_revision,"添加来源在保存期间变化，请重新添加：{}",input.source_path);
    }
    // An implicit directory can become empty after removing its last file. Keep the projected directory usable.
    fn mark_parents(path:&str,represented:&mut HashSet<String>){let parts:Vec<_>=path.trim_end_matches('/').split('/').collect();for i in 1..parts.len(){represented.insert(format!("{}/",parts[..i].join("/")));}if path.ends_with('/'){represented.insert(path.into());}}
    let mut represented=HashSet::new();for path in &written_paths{mark_parents(path,&mut represented);}
    let mut directories:Vec<_>=expected.values().filter(|entry|entry.is_dir).collect();directories.sort_by_key(|entry|std::cmp::Reverse(entry.path.len()));
    for directory in directories {checkpoint(&flag)?;if !represented.contains(&directory.path){writer.add_directory(&directory.path,SimpleFileOptions::default().compression_method(CompressionMethod::Stored))?;written_paths.insert(directory.path.clone());mark_parents(&directory.path,&mut represented);}}
    let file=writer.finish()?;file.file.sync_all()?;drop(file);checkpoint(&flag)?;
    let mut rebuilt=ZipArchive::new(ControlledFile{file:File::open(&staged.0)?,flag:flag.clone(),observer:None,position:0,high:0,estimate:0})?;
    let facts=validate_contents(&mut rebuilt,&flag,&observer,"verifying",0.75,0.23,output_password,Some(credentials.output_encoding))?;
    for (path,fact) in &retained_facts {ensure!(facts.get(path)==Some(fact),"保留条目的内容或新位置不一致：{path}");}
    ensure!(facts.keys().cloned().collect::<HashSet<_>>()==written_paths,"新包条目与确认的修改清单不一致");
    for change in changes{ensure!(facts.get(&change.input.target_path).is_some_and(|(size,_)|*size==change.input.size),"新增内容大小不一致：{}",change.input.target_path);}
    drop(rebuilt);ensure!(inspect(source)?.revision==revision,"原包在保存期间变化，请重新确认后再编辑");
    checkpoint(&flag)?;progress(&observer,"publishing",0.99,"",0,0,None);
    super::output_publish_transaction::transfer_no_replace(&staged.0,output)?;
    progress(&observer,"completed",1.0,"",0,0,Some(output.metadata()?.len()));Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn zip(path:&Path){let mut writer=ZipWriter::new(File::create(path).unwrap());writer.set_comment("保留备注").unwrap();
        let options=SimpleFileOptions::default().compression_method(CompressionMethod::Deflated).last_modified_time(zip_aes::DateTime::from_date_and_time(2020,2,3,4,5,6).unwrap());
        writer.start_file("资料/说明.txt",options).unwrap();writer.write_all(b"keep original contents").unwrap();writer.start_file("替换.txt",options).unwrap();writer.write_all(b"old").unwrap();writer.add_directory("空目录/",options).unwrap();writer.finish().unwrap();}
    fn observer()->Observer{Arc::new(|_|{})}
    fn changes(paths:&[String],target:&str,action:&str)->Vec<Change>{prepare(paths,target).unwrap().into_iter().map(|input|Change{input,action:action.into(),replaces_path:None}).collect()}
    #[test]
    fn zip_edit1544_add_replace_retains_original_content_names_empty_directories_and_timestamp(){
        let temp=tempfile::tempdir().unwrap();let source=temp.path().join("源.zip");zip(&source);let source_bytes=std::fs::read(&source).unwrap();let catalog=inspect(&source).unwrap();
        let new=temp.path().join("新增.txt");std::fs::write(&new,"新增内容").unwrap();let replacement=temp.path().join("替换.txt");std::fs::write(&replacement,"替换内容").unwrap();
        let mut changes=changes(&[new.to_string_lossy().into()],"资料","add");changes.extend(self::changes(&[replacement.to_string_lossy().into()],"","replace"));
        let folder=temp.path().join("新空目录");std::fs::create_dir(&folder).unwrap();changes.extend(self::changes(&[folder.to_string_lossy().into()],"","add"));
        let output=temp.path().join("新.zip");save(&source,&catalog.revision,&changes,&output,"deflate",6,Arc::new(AtomicBool::new(false)),observer()).unwrap();
        assert_eq!(std::fs::read(&source).unwrap(),source_bytes);let mut result=ZipArchive::new(File::open(output).unwrap()).unwrap();assert_eq!(result.comment(),"保留备注".as_bytes());assert!(result.by_name("空目录/").unwrap().is_dir());assert!(result.by_name("新空目录/").unwrap().is_dir());
        let mut entry=result.by_name("资料/说明.txt").unwrap();assert_eq!(entry.last_modified().unwrap().year(),2020);assert_eq!(entry.compression(),CompressionMethod::Deflated);let mut content=String::new();entry.read_to_string(&mut content).unwrap();assert_eq!(content,"keep original contents");drop(entry);
        let mut content=String::new();result.by_name("替换.txt").unwrap().read_to_string(&mut content).unwrap();assert_eq!(content,"替换内容");
    }
    #[test]
    fn zip_edit1544_output_conflict_and_changed_inputs_preserve_files(){
        let temp=tempfile::tempdir().unwrap();let source=temp.path().join("source.zip");zip(&source);let before=std::fs::read(&source).unwrap();let catalog=inspect(&source).unwrap();let input=temp.path().join("new.txt");std::fs::write(&input,b"new").unwrap();let changes=changes(&[input.to_string_lossy().into()],"","add");let output=temp.path().join("output.zip");std::fs::write(&output,b"existing").unwrap();
        assert!(save(&source,&catalog.revision,&changes,&output,"store",0,Arc::new(AtomicBool::new(false)),observer()).unwrap_err().to_string().contains("[archive-output:Pre-checking:output-conflict]"));assert_eq!(std::fs::read(&output).unwrap(),b"existing");
        std::fs::remove_file(&output).unwrap();std::fs::write(&input,b"changed input").unwrap();assert!(save(&source,&catalog.revision,&changes,&output,"store",0,Arc::new(AtomicBool::new(false)),observer()).is_err());assert!(!output.exists());assert_eq!(std::fs::read(source).unwrap(),before);
    }
    #[test]
    fn zip_edit1544_cancel_during_writing_removes_staging_and_keeps_original(){
        let temp=tempfile::tempdir().unwrap();let source=temp.path().join("source.zip");zip(&source);let catalog=inspect(&source).unwrap();let input=temp.path().join("new.bin");std::fs::write(&input,vec![42;1024*1024]).unwrap();let changes=changes(&[input.to_string_lossy().into()],"","add");let flag=Arc::new(AtomicBool::new(false));let signal=flag.clone();let output=temp.path().join("cancelled.zip");
        let result=save(&source,&catalog.revision,&changes,&output,"store",0,flag,Arc::new(move|e|{if e.phase=="writing"{signal.store(true,Ordering::SeqCst)}}));assert!(result.is_err());assert!(!output.exists());assert!(std::fs::read_dir(temp.path()).unwrap().all(|e|!e.unwrap().file_name().to_string_lossy().starts_with(".zip-edit-")));
    }
    #[test]
    fn zip_edit1544_catalog_encryption_and_rejects_duplicate_names_and_conflicting_paths(){
        let temp=tempfile::tempdir().unwrap();let path=temp.path().join("encrypted.zip");let mut writer=ZipWriter::new(File::create(&path).unwrap());writer.start_file("secret.txt",SimpleFileOptions::default().with_aes_encryption(zip_aes::AesMode::Aes256,"password")).unwrap();writer.write_all(b"secret").unwrap();writer.finish().unwrap();assert!(inspect(&path).unwrap().encrypted);
        let mut writer=zip::ZipWriter::new(File::create(&path).unwrap());for _ in 0..2{writer.start_file("same.txt",zip::write::FileOptions::default()).unwrap();writer.write_all(b"same").unwrap();}writer.finish().unwrap();assert!(inspect(&path).unwrap_err().to_string().contains("重复"));
        for name in ["../escape","/absolute","folder/../file","folder/file.","CON.txt"]{assert!(normalized_path(name,false).is_err(),"{name}");}
    }
    #[test]
    fn zip_edit1544_stored_addition_to_empty_zip_and_manifest_conflict(){
        let temp=tempfile::tempdir().unwrap();let source=temp.path().join("empty.zip");ZipWriter::new(File::create(&source).unwrap()).finish().unwrap();let catalog=inspect(&source).unwrap();let input=temp.path().join("file.txt");std::fs::write(&input,b"stored").unwrap();let mut changes=changes(&[input.to_string_lossy().into()],"","add");let output=temp.path().join("stored.zip");save(&source,&catalog.revision,&changes,&output,"store",0,Arc::new(AtomicBool::new(false)),observer()).unwrap();assert_eq!(ZipArchive::new(File::open(output).unwrap()).unwrap().by_index(0).unwrap().compression(),CompressionMethod::Stored);
        changes.push(changes[0].clone());assert!(save(&source,&catalog.revision,&changes,&temp.path().join("duplicate.zip"),"deflate",6,Arc::new(AtomicBool::new(false)),observer()).is_err());
    }
    fn edit(source:&str,target:Option<&str>)->OriginalEdit{OriginalEdit{source_path:source.into(),target_path:target.map(str::to_string)}}
    #[test]
    fn zip_edit1545_combines_move_replace_add_delete_and_retains_raw_facts(){
        let temp=tempfile::tempdir().unwrap();let source=temp.path().join("原包.zip");zip(&source);let before=std::fs::read(&source).unwrap();let catalog=inspect(&source).unwrap();
        let input=temp.path().join("新增.txt");std::fs::write(&input,"新内容").unwrap();let mut additions=changes(&[input.to_string_lossy().into()],"新资料","add");
        additions[0].input.target_path="说明.txt".into();
        let original_edits=[edit("资料/",Some("交付/")),edit("资料/说明.txt",Some("交付/中文说明.txt")),edit("替换.txt",None),edit("空目录/",Some("新的空目录/"))];
        let output=temp.path().join("整理.zip");save_with_edits(&source,&catalog.revision,&additions,&original_edits,&output,"deflate",6,Arc::new(AtomicBool::new(false)),observer()).unwrap();
        assert_eq!(std::fs::read(&source).unwrap(),before);let mut archive=ZipArchive::new(File::open(output).unwrap()).unwrap();assert_eq!(archive.comment(),"保留备注".as_bytes());assert!(archive.by_name("替换.txt").is_err());assert!(archive.by_name("新的空目录/").unwrap().is_dir());
        let mut kept=archive.by_name("交付/中文说明.txt").unwrap();assert_eq!(kept.last_modified().unwrap().year(),2020);assert_eq!(kept.compression(),CompressionMethod::Deflated);let mut content=String::new();kept.read_to_string(&mut content).unwrap();assert_eq!(content,"keep original contents");drop(kept);
        let mut content=String::new();archive.by_name("说明.txt").unwrap().read_to_string(&mut content).unwrap();assert_eq!(content,"新内容");
    }
    #[test]
    fn zip_edit1545_swaps_names_and_keeps_an_implicit_directory_that_becomes_empty(){
        let temp=tempfile::tempdir().unwrap();let source=temp.path().join("source.zip");let mut writer=ZipWriter::new(File::create(&source).unwrap());for(name,content)in[("A.txt","aaa"),("B.txt","bbb"),("implicit/last.txt","last")]{writer.start_file(name,SimpleFileOptions::default().compression_method(CompressionMethod::Stored)).unwrap();writer.write_all(content.as_bytes()).unwrap();}writer.finish().unwrap();
        let catalog=inspect(&source).unwrap();let output=temp.path().join("swap.zip");save_with_edits(&source,&catalog.revision,&[],&[edit("A.txt",Some("B.txt")),edit("B.txt",Some("a.txt")),edit("implicit/last.txt",None)],&output,"store",0,Arc::new(AtomicBool::new(false)),observer()).unwrap();
        let mut archive=ZipArchive::new(File::open(output).unwrap()).unwrap();for(name,expected)in[("B.txt","aaa"),("a.txt","bbb")]{let mut content=String::new();archive.by_name(name).unwrap().read_to_string(&mut content).unwrap();assert_eq!(content,expected);}assert!(archive.by_name("implicit/").unwrap().is_dir());assert_eq!(archive.len(),3);
    }
    #[test]
    fn zip_edit1545_delete_everything_creates_a_readable_empty_zip(){
        let temp=tempfile::tempdir().unwrap();let source=temp.path().join("source.zip");zip(&source);let catalog=inspect(&source).unwrap();let edits:Vec<_>=catalog.entries.iter().map(|entry|edit(&entry.path,None)).collect();let output=temp.path().join("empty.zip");
        save_with_edits(&source,&catalog.revision,&[],&edits,&output,"deflate",6,Arc::new(AtomicBool::new(false)),observer()).unwrap();assert_eq!(ZipArchive::new(File::open(output).unwrap()).unwrap().len(),0);
    }
    #[test]
    fn zip_edit1545_rejects_unknown_duplicate_conflicting_and_invalid_transforms_before_output(){
        let temp=tempfile::tempdir().unwrap();let source=temp.path().join("source.zip");zip(&source);let before=std::fs::read(&source).unwrap();let catalog=inspect(&source).unwrap();let output=temp.path().join("invalid.zip");
        for edits in [vec![edit("missing",None)],vec![edit("替换.txt",None),edit("替换.txt",None)],vec![edit("替换.txt",Some("资料/说明.txt"))],vec![edit("替换.txt",Some("资料"))],vec![edit("替换.txt",Some("../escape.txt"))]]{
            assert!(save_with_edits(&source,&catalog.revision,&[],&edits,&output,"deflate",6,Arc::new(AtomicBool::new(false)),observer()).is_err());assert!(!output.exists());assert_eq!(std::fs::read(&source).unwrap(),before);
        }
    }
    #[test]
    fn zip_edit1545_stopping_a_rename_keeps_the_source_and_no_partial_output(){
        let temp=tempfile::tempdir().unwrap();let source=temp.path().join("source.zip");zip(&source);let before=std::fs::read(&source).unwrap();let catalog=inspect(&source).unwrap();let flag=Arc::new(AtomicBool::new(false));let signal=flag.clone();let output=temp.path().join("cancelled.zip");
        assert!(save_with_edits(&source,&catalog.revision,&[],&[edit("替换.txt",Some("new.txt"))],&output,"store",0,flag,Arc::new(move|event|{if event.phase=="writing"{signal.store(true,Ordering::SeqCst)}})).is_err());assert!(!output.exists());assert_eq!(std::fs::read(source).unwrap(),before);assert!(std::fs::read_dir(temp.path()).unwrap().all(|entry|!entry.unwrap().file_name().to_string_lossy().starts_with(".zip-edit-")));
    }

    fn protected_zip(path:&Path,scheme:Encryption,password:&[u8]){let mut writer=ZipWriter::new(File::create(path).unwrap());writer.set_comment("加密包备注").unwrap();for(name,content,method)in[("secret.txt","protected contents",CompressionMethod::Deflated),("stored.txt","stored contents",CompressionMethod::Stored)]{let options=write_options(method,content.len() as u64,Some(zip_aes::DateTime::from_date_and_time(2021,3,4,5,6,8).unwrap()),None,"条目备注",scheme,password,None).unwrap();writer.start_file(name,options).unwrap();writer.write_all(content.as_bytes()).unwrap();}writer.start_file("public.txt",SimpleFileOptions::default()).unwrap();writer.write_all(b"public contents").unwrap();writer.add_directory("empty/",SimpleFileOptions::default()).unwrap();writer.finish().unwrap();}
    fn read_password(archive:&mut ZipArchive<File>,name:&str,password:&str,encoding:ZipPasswordEncoding)->String{let index=(0..archive.len()).find(|index|archive.by_index_raw(*index).unwrap().name()==name).unwrap();let mut content=String::new();super::super::zip_password::open_entry_with_encoding(archive,index,password,Some(encoding)).unwrap().read_to_string(&mut content).unwrap();content}
    #[test]
    fn zip_edit1546_preserves_aes_strengths_zipcrypto_mixed_public_entries_and_replacement_encryption(){
        for scheme in [Encryption::Aes128,Encryption::Aes192,Encryption::Aes256,Encryption::Zipcrypto]{let temp=tempfile::tempdir().unwrap();let source=temp.path().join("source.zip");protected_zip(&source,scheme,b"original-password");let before=std::fs::read(&source).unwrap();let catalog=inspect(&source).unwrap();assert_eq!(catalog.default_encryption,scheme);
            let replacement=temp.path().join("new.txt");std::fs::write(&replacement,b"replacement").unwrap();let mut changes=changes(&[replacement.to_string_lossy().into()],"","add");changes[0].input.target_path="renamed.txt".into();changes[0].replaces_path=Some("secret.txt".into());let output=temp.path().join("edited.zip");let credentials=Credentials{source_password:"original-password".into(),..Default::default()};
            save_encrypted(&source,&catalog.revision,&changes,&[edit("secret.txt",None)],&output,"deflate",6,&credentials,Arc::new(AtomicBool::new(false)),observer()).unwrap();let output_catalog=inspect(&output).unwrap();assert_eq!(output_catalog.encryptions["renamed.txt"],scheme);assert_eq!(output_catalog.encryptions["stored.txt"],scheme);assert_eq!(output_catalog.encryptions["public.txt"],Encryption::None);
            let mut archive=ZipArchive::new(File::open(output).unwrap()).unwrap();assert_eq!(read_password(&mut archive,"renamed.txt","original-password",ZipPasswordEncoding::Utf8),"replacement");assert_eq!(read_password(&mut archive,"stored.txt","original-password",ZipPasswordEncoding::Utf8),"stored contents");let stored_index=(0..archive.len()).find(|index|archive.by_index_raw(*index).unwrap().name()=="stored.txt").unwrap();let entry=archive.by_index_raw(stored_index).unwrap();assert_eq!(entry.compression(),CompressionMethod::Stored);assert_eq!(entry.last_modified().unwrap().year(),2021);assert_eq!(entry.comment(),"条目备注");assert_eq!(std::fs::read(&source).unwrap(),before);
        }
    }
    #[test]
    fn zip_edit1546_reencrypts_aes_as_zipcrypto_with_new_cp936_password_and_actual_content_crc(){
        let temp=tempfile::tempdir().unwrap();let source=temp.path().join("aes.zip");protected_zip(&source,Encryption::Aes256,b"old");let catalog=inspect(&source).unwrap();let output=temp.path().join("new.zip");let credentials=Credentials{source_password:"old".into(),output_mode:OutputMode::Zipcrypto,output_password:"交付密码".into(),output_encoding:ZipPasswordEncoding::Cp936,..Default::default()};
        save_encrypted(&source,&catalog.revision,&[],&[],&output,"deflate",6,&credentials,Arc::new(AtomicBool::new(false)),observer()).unwrap();let catalog=inspect(&output).unwrap();assert_eq!(catalog.encryptions["secret.txt"],Encryption::Zipcrypto);assert_eq!(catalog.encryptions["public.txt"],Encryption::Zipcrypto);let mut archive=ZipArchive::new(File::open(output).unwrap()).unwrap();assert_eq!(read_password(&mut archive,"secret.txt","交付密码",ZipPasswordEncoding::Cp936),"protected contents");
    }
    #[test]
    fn zip_edit1546_reads_legacy_password_encoding_and_requires_explicit_plain_output(){
        let temp=tempfile::tempdir().unwrap();let source=temp.path().join("local.zip");let password=super::super::zip_password::encode("原包密码",ZipPasswordEncoding::Cp936).unwrap();protected_zip(&source,Encryption::Zipcrypto,&password);let catalog=inspect(&source).unwrap();let output=temp.path().join("plain.zip");let credentials=Credentials{source_password:"原包密码".into(),source_encoding:Some(ZipPasswordEncoding::Cp936),output_mode:OutputMode::None,..Default::default()};
        save_encrypted(&source,&catalog.revision,&[],&[edit("secret.txt",Some("renamed.txt"))],&output,"store",0,&credentials,Arc::new(AtomicBool::new(false)),observer()).unwrap();assert!(!inspect(&output).unwrap().encrypted);let mut archive=ZipArchive::new(File::open(output).unwrap()).unwrap();let mut content=String::new();archive.by_name("renamed.txt").unwrap().read_to_string(&mut content).unwrap();assert_eq!(content,"protected contents");assert_eq!(archive.by_name("renamed.txt").unwrap().compression(),CompressionMethod::Deflated);
        let kept=temp.path().join("kept.zip");let mut keep=credentials;keep.output_mode=OutputMode::Keep;save_encrypted(&source,&catalog.revision,&[],&[edit("secret.txt",Some("renamed.txt"))],&kept,"deflate",6,&keep,Arc::new(AtomicBool::new(false)),observer()).unwrap();let mut kept=ZipArchive::new(File::open(kept).unwrap()).unwrap();assert_eq!(read_password(&mut kept,"renamed.txt","原包密码",ZipPasswordEncoding::Utf8),"protected contents");
    }
    #[test]
    fn zip_edit1546_wrong_missing_password_and_unrepresentable_output_do_not_publish(){
        let temp=tempfile::tempdir().unwrap();let source=temp.path().join("source.zip");protected_zip(&source,Encryption::Aes256,b"old");let before=std::fs::read(&source).unwrap();let catalog=inspect(&source).unwrap();let output=temp.path().join("failed.zip");
        for credentials in [Credentials::default(),Credentials{source_password:"wrong".into(),..Default::default()},Credentials{source_password:"old".into(),output_mode:OutputMode::Aes256,..Default::default()},Credentials{source_password:"old".into(),output_mode:OutputMode::Aes256,output_password:"密码😀".into(),output_encoding:ZipPasswordEncoding::Cp936,..Default::default()}]{assert!(save_encrypted(&source,&catalog.revision,&[],&[edit("secret.txt",Some("renamed.txt"))],&output,"deflate",6,&credentials,Arc::new(AtomicBool::new(false)),observer()).is_err());assert!(!output.exists());assert_eq!(std::fs::read(&source).unwrap(),before);}
    }
    #[test]
    fn zip_edit1546_empty_password_zip_needs_explicit_unencrypted_choice(){let temp=tempfile::tempdir().unwrap();let source=temp.path().join("source.zip");protected_zip(&source,Encryption::Aes256,b"old");let catalog=inspect(&source).unwrap();let edits:Vec<_>=catalog.entries.iter().map(|entry|edit(&entry.path,None)).collect();let output=temp.path().join("empty.zip");let mut credentials=Credentials{source_password:"old".into(),..Default::default()};assert!(save_encrypted(&source,&catalog.revision,&[],&edits,&output,"store",0,&credentials,Arc::new(AtomicBool::new(false)),observer()).unwrap_err().to_string().contains("空 ZIP"));credentials.output_mode=OutputMode::None;save_encrypted(&source,&catalog.revision,&[],&edits,&output,"store",0,&credentials,Arc::new(AtomicBool::new(false)),observer()).unwrap();assert_eq!(ZipArchive::new(File::open(output).unwrap()).unwrap().len(),0);}
    #[test]
    fn zip_edit1546_reports_changed_and_missing_input_references(){let temp=tempfile::tempdir().unwrap();let first=temp.path().join("first.txt");let second=temp.path().join("second.txt");std::fs::write(&first,b"one").unwrap();std::fs::write(&second,b"two").unwrap();let inputs=prepare(&[first.to_string_lossy().into(),second.to_string_lossy().into()],"").unwrap();assert!(check_inputs(&inputs).unwrap().is_empty());std::fs::write(first,b"changed").unwrap();std::fs::remove_file(second).unwrap();let issues=check_inputs(&inputs).unwrap();assert_eq!(issues.len(),2);assert_eq!(issues[0].index,0);assert!(issues[1].issue.contains("缺失"));}

}
