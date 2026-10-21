//! Local metadata index. Original archives are never modified or cached here.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::{collections::{HashMap, HashSet}, path::{Path, PathBuf}, sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}};

static JOBS: once_cell::sync::Lazy<Mutex<HashMap<String, Arc<AtomicBool>>>> = once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));
const MAX_SOURCES: i64 = 10_000;
const MAX_ENTRIES: usize = 1_000_000;

pub async fn init_tables(pool: &SqlitePool) -> Result<()> {
    sqlx::query("CREATE TABLE IF NOT EXISTS archive_library_sources (id TEXT PRIMARY KEY, path TEXT NOT NULL UNIQUE COLLATE NOCASE, name TEXT NOT NULL, format TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'pending', note TEXT NOT NULL DEFAULT '', revision TEXT NOT NULL DEFAULT '', indexed_at TEXT, entry_count INTEGER NOT NULL DEFAULT 0)").execute(pool).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS archive_library_entries (source_id TEXT NOT NULL REFERENCES archive_library_sources(id) ON DELETE CASCADE, path TEXT NOT NULL, name TEXT NOT NULL, search_path TEXT NOT NULL, extension TEXT NOT NULL, size INTEGER NOT NULL, modified TEXT, PRIMARY KEY(source_id,path))").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_library_entries_type ON archive_library_entries(extension,source_id)").execute(pool).await?;
    upgrade_tables(pool).await?;
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all="camelCase")]
pub struct Source { pub id:String, pub path:String, pub name:String, pub format:String, pub status:String, pub note:String, pub revision:String, pub indexed_at:Option<String>, pub entry_count:i64,
 pub availability:String, pub availability_note:String, pub checked_at:Option<String>, pub encrypted:bool, pub source_encoding:Option<String>, pub member_count:i64 }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Candidate { pub path:String, pub name:String, pub format:String }
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Scan { pub candidates:Vec<Candidate>, pub visited:usize, pub inaccessible:usize, pub stopped:bool, pub limited:bool }
#[derive(Default,Deserialize)]
#[serde(rename_all="camelCase",default)]
pub struct Search { pub query:String, pub source_id:String, pub category:String, pub page:u32 }
#[derive(Clone, Debug, Serialize, FromRow)]
#[serde(rename_all="camelCase")]
pub struct Hit { pub source_id:String, pub source_path:String, pub source_name:String, pub format:String, pub path:String, pub name:String, pub extension:String, pub size:i64, pub modified:Option<String>, pub size_known:bool, pub availability:String, pub source_status:String, pub encrypted:bool, pub indexed_at:Option<String> }
#[derive(Serialize)]
pub struct Results { pub items:Vec<Hit>, pub total:i64, pub page:u32 }

pub struct Job { id:String, pub flag:Arc<AtomicBool> }
impl Job { pub fn start(id:&str)->Result<Self>{let mut jobs=JOBS.lock().unwrap();ensure!(!jobs.contains_key(id),"这份来源正在处理中，请稍候");let flag=Arc::new(AtomicBool::new(false));jobs.insert(id.into(),flag.clone());Ok(Self{id:id.into(),flag})} }
impl Drop for Job {fn drop(&mut self){JOBS.lock().unwrap().remove(&self.id);}}
pub fn cancel(id:&str){if let Some(flag)=JOBS.lock().unwrap().get(id){flag.store(true,Ordering::SeqCst)}}
fn checkpoint(flag:&AtomicBool)->Result<()>{ensure!(!flag.load(Ordering::SeqCst),"已停止建库，已完成来源保留");Ok(())}
static EXTENSIONS:once_cell::sync::Lazy<Vec<String>>=once_cell::sync::Lazy::new(||crate::utils::archive_tools::detect_archive_engine_capabilities().browse_extensions);
fn candidate(path:&Path)->Option<Candidate>{
    let extension=path.extension()?.to_str()?.to_ascii_lowercase();
    if !EXTENSIONS.contains(&extension)&&!super::split_archive_detector::SplitArchiveDetector::is_split_archive(path)&&!super::alz_volumes::is_candidate(path){return None}
    let format=match extension.as_str(){"zip"|"zipx"|"jar"|"epub"|"apk"|"appx"=>"ZIP".to_string(),"tar"|"ova"=>"TAR".to_string(),_=>extension.to_uppercase()};
    Some(Candidate{path:path.to_string_lossy().into(),name:path.file_name()?.to_string_lossy().into(),format})
}
fn group_identity(path:&Path)->Result<PathBuf>{
    let selected=canonical(path)?;
    Ok(super::split_archive_detector::SplitArchiveDetector::detect_split_archive(&selected)?.map(|group|group.first_part).unwrap_or(selected))
}
pub fn source_revision(path:&Path)->Result<(String,i64)>{
    if let Some(group)=super::split_archive_detector::SplitArchiveDetector::detect_split_archive(path)?{
        ensure!(group.is_complete,"缺少分卷：{}；请补齐原卷后重新检查",group.missing_part_names());
        let values=group.parts.iter().map(|part|Ok((part.file_name().unwrap_or_default().to_string_lossy().to_string(),revision(part)?))).collect::<Result<Vec<_>>>()?;
        return Ok((serde_json::to_string(&values)?,values.len() as i64))
    }
    Ok((revision(path)?,1))
}
pub fn scan(directory:&Path,flag:&AtomicBool)->Result<Scan>{
    ensure!(directory.is_dir(),"请选择可访问的资料文件夹");
    let mut stack=vec![directory.to_path_buf()];let mut result=Scan{candidates:vec![],visited:0,inaccessible:0,stopped:false,limited:false};
    while let Some(dir)=stack.pop(){
        if flag.load(Ordering::SeqCst){result.stopped=true;break}
        let entries=match std::fs::read_dir(dir){Ok(value)=>value,Err(_)=>{result.inaccessible+=1;continue}};
        for entry in entries {
            if flag.load(Ordering::SeqCst){result.stopped=true;break}
            result.visited+=1;
            if result.visited>100_000||result.candidates.len()>=10_000{result.limited=true;break}
            let entry=match entry{Ok(value)=>value,Err(_)=>{result.inaccessible+=1;continue}};
            let kind=match entry.file_type(){Ok(value)=>value,Err(_)=>{result.inaccessible+=1;continue}};
            if kind.is_symlink(){continue}
            if kind.is_dir(){stack.push(entry.path())}else if kind.is_file(){if let Some(value)=candidate(&entry.path()){result.candidates.push(value)}}
        }
        if result.stopped||result.limited{break}
    }
    let mut identities=HashSet::new();
    result.candidates.retain(|row|{let identity=group_identity(Path::new(&row.path)).unwrap_or_else(|_|PathBuf::from(&row.path)).to_string_lossy().to_lowercase();identities.insert(identity)});
    result.candidates.sort_by(|a,b|a.path.to_lowercase().cmp(&b.path.to_lowercase()));Ok(result)
}
fn canonical(path:&Path)->Result<PathBuf>{
    let meta=std::fs::symlink_metadata(path).with_context(||format!("找不到来源：{}",path.display()))?;
    ensure!(meta.is_file()&&!meta.file_type().is_symlink(),"来源必须是普通归档文件");
    let value=std::fs::canonicalize(path)?;
    // Keep familiar paths in the UI; filesystem APIs retain long-path support.
    let display=value.to_string_lossy();
    Ok(if let Some(unc)=display.strip_prefix(r"\\?\UNC\"){PathBuf::from(format!(r"\\{unc}"))}else{PathBuf::from(display.strip_prefix(r"\\?\").unwrap_or(&display))})
}
pub fn revision(path:&Path)->Result<String>{let meta=std::fs::metadata(path).context("原包缺失或无法访问，请重新选择来源")?;ensure!(meta.is_file(),"原包不再是文件");Ok(format!("{}:{:?}",meta.len(),meta.modified()?))}
async fn existing_identities(pool:&SqlitePool)->Result<HashMap<String,(String,String)>>{
    let rows:Vec<(String,String)>=sqlx::query_as("SELECT id,path FROM archive_library_sources").fetch_all(pool).await?;
    Ok(rows.into_iter().map(|(id,path)|{let key=group_identity(Path::new(&path)).unwrap_or_else(|_|PathBuf::from(&path)).to_string_lossy().to_lowercase();(key,(id,path))}).collect())
}
pub async fn add(pool:&SqlitePool,paths:Vec<String>)->Result<Vec<Source>>{
    ensure!(paths.len()<=10_000,"一次最多添加 10000 份来源，请分批选择");
    let mut items=vec![];
    for path in paths{let selected=canonical(Path::new(&path))?;let mut item=candidate(&selected).context("这个后缀不在当前可浏览的归档范围，请从原浏览器确认")?;let identity=group_identity(&selected)?;item.path=identity.to_string_lossy().into();item.name=identity.file_name().unwrap_or_default().to_string_lossy().into();items.push(item)}
    let existing=existing_identities(pool).await?;
    for item in &mut items{if let Some((_,path))=existing.get(&item.path.to_lowercase()){item.path=path.clone();}}
    let mut transaction=pool.begin().await?;
    let (count,):(i64,)=sqlx::query_as("SELECT COUNT(*) FROM archive_library_sources").fetch_one(&mut *transaction).await?;
    let mut inserted=0;
    for item in items{let result=sqlx::query("INSERT OR IGNORE INTO archive_library_sources(id,path,name,format) VALUES(?,?,?,?)").bind(uuid::Uuid::new_v4().to_string()).bind(item.path).bind(item.name).bind(item.format).execute(&mut *transaction).await?;inserted+=result.rows_affected() as i64;ensure!(count+inserted<=MAX_SOURCES,"资料库暂支持 10000 份来源，请先移除不用的记录");}
    transaction.commit().await?;list(pool).await
}
pub async fn list(pool:&SqlitePool)->Result<Vec<Source>>{
    let rows:Vec<Source>=sqlx::query_as("SELECT * FROM archive_library_sources ORDER BY name COLLATE NOCASE,path").fetch_all(pool).await?;
    let active=JOBS.lock().unwrap().keys().cloned().collect::<HashSet<_>>();
    let mut result=vec![];
    for mut row in rows{if row.status=="indexing"&&!active.contains(&row.id){row.status="interrupted".into();row.note="上次建库已中断，可手动继续；其他已完成来源可搜索".into();sqlx::query("UPDATE archive_library_sources SET status=?,note=? WHERE id=? AND status='indexing'").bind(&row.status).bind(&row.note).bind(&row.id).execute(pool).await?;}result.push(row)}Ok(result)
}
pub async fn source(pool:&SqlitePool,id:&str)->Result<Source>{sqlx::query_as("SELECT * FROM archive_library_sources WHERE id=?").bind(id).fetch_optional(pool).await?.context("来源已移出资料库，请返回重新选择")}
pub async fn remove(pool:&SqlitePool,id:&str)->Result<()>{ensure!(!JOBS.lock().unwrap().contains_key(id),"请先停止这份来源的建库，再移除");let mut tx=pool.begin().await?;sqlx::query("DELETE FROM archive_library_entries WHERE source_id=?").bind(id).execute(&mut *tx).await?;sqlx::query("DELETE FROM archive_library_sources WHERE id=?").bind(id).execute(&mut *tx).await?;tx.commit().await?;Ok(())}

#[derive(Default,Clone,Deserialize)]
#[serde(rename_all="camelCase",default)]
pub struct Credentials {pub password:String,pub encoding:Option<super::zip_password::ZipPasswordEncoding>}
#[derive(Serialize)]
pub struct IndexOutcome{pub skipped:bool}

pub async fn upgrade_tables(pool:&SqlitePool)->Result<()>{
    for (name,definition) in [("availability","TEXT NOT NULL DEFAULT 'unchecked'"),("availability_note","TEXT NOT NULL DEFAULT ''"),("checked_at","TEXT"),("encrypted","BOOLEAN NOT NULL DEFAULT 0"),("source_encoding","TEXT"),("member_count","INTEGER NOT NULL DEFAULT 1")]{
        let (count,):(i64,)=sqlx::query_as("SELECT COUNT(*) FROM pragma_table_info('archive_library_sources') WHERE name=?").bind(name).fetch_one(pool).await?;
        if count==0{sqlx::query(&format!("ALTER TABLE archive_library_sources ADD COLUMN {name} {definition}")).execute(pool).await?;}
    }
    let (count,):(i64,)=sqlx::query_as("SELECT COUNT(*) FROM pragma_table_info('archive_library_entries') WHERE name='size_known'").fetch_one(pool).await?;
    if count==0{sqlx::query("ALTER TABLE archive_library_entries ADD COLUMN size_known BOOLEAN NOT NULL DEFAULT 1").execute(pool).await?;}Ok(())
}
fn issue(error:&anyhow::Error)->(&'static str,String){
    let note=format!("{error:#}");let lower=note.to_lowercase();
    let state=if note.contains("缺少")&&note.contains("卷"){"incomplete"}else if note.contains("缺失")||note.contains("找不到")||lower.contains("cannot find")||lower.contains("not found"){"missing"}else{"unreadable"};(state,note)
}
async fn observed(pool:&SqlitePool,id:&str,state:&str,note:&str,members:i64)->Result<()>{
    sqlx::query("UPDATE archive_library_sources SET availability=?,availability_note=?,checked_at=?,member_count=? WHERE id=?").bind(state).bind(note).bind(chrono::Utc::now().to_rfc3339()).bind(members).bind(id).execute(pool).await?;Ok(())
}
pub async fn audit(pool:&SqlitePool,id:&str)->Result<Source>{
    let _job=Job::start(id)?;let item=source(pool,id).await?;
    match source_revision(Path::new(&item.path)){
        Ok((current,members))=>{let changed=item.indexed_at.is_some()&&current!=item.revision;observed(pool,id,if changed{"changed"}else{"available"},if changed{"原包或分卷已变化，当前搜索显示上次索引；请刷新后使用"}else{""},members).await?;}
        Err(error)=>{let(state,note)=issue(&error);observed(pool,id,state,&note,item.member_count).await?;}
    }source(pool,id).await
}
pub async fn relocate(pool:&SqlitePool,id:&str,path:&Path)->Result<Source>{
    let _job=Job::start(id)?;let selected=canonical(path)?;let item=candidate(&selected).context("请选择当前可浏览的归档或任意分卷成员")?;let identity=group_identity(&selected)?;let path=identity.to_string_lossy().to_string();
    let aliases=existing_identities(pool).await?;
    ensure!(aliases.get(&path.to_lowercase()).is_none_or(|(other,_)|other==id),"这个位置已在资料库中，请使用已有来源；不会合并覆盖另一份索引");
    source(pool,id).await?;
    sqlx::query("UPDATE archive_library_sources SET path=?,name=?,format=CASE WHEN indexed_at IS NULL THEN ? ELSE format END,revision='',availability='changed',availability_note='已重新定位，搜索保留上次索引；请刷新确认当前资料',checked_at=? WHERE id=?").bind(&path).bind(identity.file_name().unwrap_or_default().to_string_lossy().to_string()).bind(item.format).bind(chrono::Utc::now().to_rfc3339()).bind(id).execute(pool).await?;
    source(pool,id).await
}
#[cfg(test)]
pub async fn index(pool:&SqlitePool,id:&str)->Result<()>{index_with_credentials(pool,id,&Credentials::default(),false).await.map(|_|())}
pub async fn index_with_credentials(pool:&SqlitePool,id:&str,credentials:&Credentials,force:bool)->Result<IndexOutcome>{
    let job=Job::start(id)?;let item=source(pool,id).await?;
    sqlx::query("UPDATE archive_library_sources SET status='indexing',note='' WHERE id=?").bind(id).execute(pool).await?;
    let result=index_contents_with_credentials(pool,&item,job.flag.clone(),credentials,force).await;
    if let Err(error)=&result{
        let note=format!("{error:#}");let lower=note.to_lowercase();
        let locked=note.contains("密码")||lower.contains("password")||lower.contains("encrypted");
        let status=if job.flag.load(Ordering::SeqCst){"interrupted"}else if locked{"locked"}else{"failed"};
        sqlx::query("UPDATE archive_library_sources SET status=?,note=?,encrypted=CASE WHEN ? THEN 1 ELSE encrypted END WHERE id=?").bind(status).bind(note).bind(locked).bind(id).execute(pool).await?;
    }result
}
async fn verify_credentials(path:&Path,format:&str,credentials:&Credentials,flag:Arc<AtomicBool>)->Result<()>{
    ensure!(!credentials.password.is_empty(),"这份资料需要密码，请填写或从保险箱选择后继续");
    let options=crate::models::compression::DecompressOptions{zip_password_encoding:credentials.encoding,..Default::default()};
    let matched=if format=="ZIP"{
        let path=path.to_path_buf();let credentials=credentials.clone();
        tauri::async_runtime::spawn_blocking(move||super::universal_engine::UniversalCliEngine::try_zip_password_for_selection_cancellable(&path,&credentials.password,&options,||checkpoint(&flag))).await??
    }else{
        #[cfg(test)] let mut service=super::compression_service::CompressionService::for_testing();
        #[cfg(not(test))] let mut service=super::compression_service::CompressionService::new_with_defaults().await;
        service.cancellation_flag=flag;
        service.test_extraction_password(path.to_str().context("来源路径无效")?,&credentials.password,&options).await?
    };
    ensure!(matched,"密码未能读取加密内容，请检查密码与所选编码；也可回到原浏览器查看原因");Ok(())
}
#[cfg(test)]
async fn index_contents(pool:&SqlitePool,item:&Source,flag:Arc<AtomicBool>)->Result<()>{index_contents_with_credentials(pool,item,flag,&Credentials::default(),true).await.map(|_|())}
async fn index_contents_with_credentials(pool:&SqlitePool,item:&Source,flag:Arc<AtomicBool>,credentials:&Credentials,force:bool)->Result<IndexOutcome>{
    let path=PathBuf::from(&item.path);checkpoint(&flag)?;
    let (before,members)=match source_revision(&path){Ok(value)=>value,Err(error)=>{let(state,note)=issue(&error);observed(pool,&item.id,state,&note,item.member_count).await?;return Err(error)}};
    observed(pool,&item.id,"available","",members).await?;
    if !force&&item.indexed_at.is_some()&&before==item.revision&&credentials.encoding.map(|v|v.as_str())==item.source_encoding.as_deref(){
        if item.encrypted{verify_credentials(&path,&item.format,credentials,flag.clone()).await?;}
        sqlx::query("UPDATE archive_library_sources SET status='ready',note='',source_encoding=? WHERE id=?").bind(credentials.encoding.map(|v|v.as_str())).bind(&item.id).execute(pool).await?;
        return Ok(IndexOutcome{skipped:true})
    }
    let result=super::archive_browser::browse_archive_cancellable_at(&path,(!credentials.password.is_empty()).then_some(credentials.password.as_str()),flag.clone(),credentials.encoding,None).await?;
    if result.format=="ZIP"&&members==1{
        let mut file=std::fs::File::open(&path)?;use std::io::Read;let mut header=[0u8;2];file.read_exact(&mut header)?;
        if &header!=b"MZ"{let declared=super::zip_edit::declared_count(&mut file).context("ZIP 目录不完整，无法唯一索引")?;ensure!(declared==result.entries.len() as u64,"ZIP 存在重复目录条目，请使用原浏览/提取");}
    }
    ensure!(result.entries.len()<=MAX_ENTRIES,"单包超过 100 万条，请先按资料范围拆分");
    if result.encrypted{verify_credentials(&path,&result.format,credentials,flag.clone()).await?;}
    ensure!(before==source_revision(&path)?.0,"建库时原包或分卷发生变化，请重新刷新");checkpoint(&flag)?;
    let mut tx=pool.begin().await?;
    sqlx::query("DELETE FROM archive_library_entries WHERE source_id=?").bind(&item.id).execute(&mut *tx).await?;
    let mut paths=HashSet::new();let mut count=0;
    for entry in result.entries.into_iter().filter(|entry|!entry.is_dir){checkpoint(&flag)?;ensure!(paths.insert(entry.path.clone()),"包内位置重复，无法唯一定位，请继续使用原浏览/提取");
        let extension=Path::new(&entry.name).extension().and_then(|v|v.to_str()).unwrap_or("").to_lowercase();
        sqlx::query("INSERT INTO archive_library_entries(source_id,path,name,search_path,extension,size,modified,size_known) VALUES(?,?,?,?,?,?,?,?)").bind(&item.id).bind(&entry.path).bind(entry.name).bind(entry.path.to_lowercase()).bind(extension).bind(i64::try_from(entry.size)?).bind(entry.modified).bind(entry.size_known.unwrap_or(true)).execute(&mut *tx).await?;count+=1;
    }
    checkpoint(&flag)?;ensure!(before==source_revision(&path)?.0,"建立索引期间原包或分卷发生变化，请重新刷新");
    sqlx::query("UPDATE archive_library_sources SET status='ready',note='',revision=?,indexed_at=?,entry_count=?,format=?,encrypted=?,source_encoding=?,availability='available',availability_note='',checked_at=?,member_count=? WHERE id=?").bind(before).bind(chrono::Utc::now().to_rfc3339()).bind(count).bind(result.format).bind(result.encrypted).bind(credentials.encoding.map(|v|v.as_str())).bind(chrono::Utc::now().to_rfc3339()).bind(members).bind(&item.id).execute(&mut *tx).await?;
    tx.commit().await?;Ok(IndexOutcome{skipped:false})
}
pub async fn check_source(pool:&SqlitePool,id:&str,entry:Option<&str>)->Result<Source>{
    let item=audit(pool,id).await?;ensure!(item.indexed_at.is_some(),"这份来源尚未完成建库，请先刷新索引");
    ensure!(item.availability=="available","{}",item.availability_note);
    if let Some(entry)=entry{let (count,):(i64,)=sqlx::query_as("SELECT COUNT(*) FROM archive_library_entries WHERE source_id=? AND path=?").bind(id).bind(entry).fetch_one(pool).await?;ensure!(count==1,"所选条目已不在当前索引中，请重新搜索");}Ok(item)
}
fn category_extensions(category:&str)->&'static [&'static str]{match category{"image"=>&["png","jpg","jpeg","gif","webp","bmp","svg","avif","heic"],"document"=>&["txt","md","pdf","doc","docx","xls","xlsx","ppt","pptx","csv","rtf"],"audio"=>&["mp3","flac","wav","aac","ogg","m4a"],"video"=>&["mp4","mkv","mov","avi","webm","wmv"],"archive"=>&["zip","tar","7z","rar","gz","xz","bz2","zst"],_=>&[]}}
pub async fn search(pool:&SqlitePool,request:Search)->Result<Results>{
    ensure!(request.query.chars().count()<=512,"搜索文字请控制在 512 字以内");
    let query=request.query.trim().to_lowercase();let escaped=query.replace('\\',"\\\\").replace('%',"\\%").replace('_',"\\_");
    let mut filter=String::from(" FROM archive_library_entries e JOIN archive_library_sources s ON s.id=e.source_id WHERE s.indexed_at IS NOT NULL AND (e.search_path LIKE ? ESCAPE '\\') AND (?='' OR s.id=?)");
    let extensions=category_extensions(&request.category);if !extensions.is_empty(){filter.push_str(" AND e.extension IN (");filter.push_str(&vec!["?";extensions.len()].join(","));filter.push(')');}
    let pattern=format!("%{escaped}%");
    let count_sql=format!("SELECT COUNT(*){filter}");
    let mut count=sqlx::query_as::<_,(i64,)>(&count_sql).bind(&pattern).bind(&request.source_id).bind(&request.source_id);for value in extensions{count=count.bind(value)}let (total,)=count.fetch_one(pool).await?;
    let page=request.page.max(1).min(((total.max(1) as u64+49)/50) as u32);
    let rows_sql=format!("SELECT s.id source_id,s.path source_path,s.name source_name,s.format,e.path,e.name,e.extension,e.size,e.modified,e.size_known,s.availability,s.status source_status,s.encrypted,s.indexed_at{filter} ORDER BY e.name COLLATE NOCASE,s.name COLLATE NOCASE,s.id,e.path LIMIT 50 OFFSET ?");
    let mut rows=sqlx::query_as::<_,Hit>(&rows_sql).bind(&pattern).bind(&request.source_id).bind(&request.source_id);for value in extensions{rows=rows.bind(value)}let items=rows.bind((page as i64-1)*50).fetch_all(pool).await?;Ok(Results{items,total,page})
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Write};
    async fn db()->SqlitePool {let pool=sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();sqlx::query("PRAGMA foreign_keys=ON").execute(&pool).await.unwrap();init_tables(&pool).await.unwrap();pool}
    fn zip(path:&Path,names:&[&str],password:bool){let mut writer=zip_aes::ZipWriter::new(File::create(path).unwrap());for name in names{let options=zip_aes::write::SimpleFileOptions::default().compression_method(zip_aes::CompressionMethod::Deflated);let options=if password{options.with_aes_encryption(zip_aes::AesMode::Aes256,"fixture")}else{options};writer.start_file(*name,options).unwrap();writer.write_all(b"library fixture contents").unwrap();}writer.finish().unwrap();}
    fn tar(path:&Path){let mut builder=tar::Builder::new(File::create(path).unwrap());let mut header=tar::Header::new_gnu();header.set_size(7);header.set_mode(0o644);header.set_mtime(1_700_000_000);header.set_cksum();builder.append_data(&mut header,"资料/说明.txt",&b"content"[..]).unwrap();builder.finish().unwrap();}
    async fn add_one(pool:&SqlitePool,path:&Path)->Source{add(pool,vec![path.to_string_lossy().into()]).await.unwrap().into_iter().find(|row|Path::new(&row.path)==path).unwrap()}
    #[tokio::test]
    async fn library1547_zip_tar_literal_unicode_search_and_source_filter(){
        let dir=tempfile::tempdir().unwrap();let zip_path=dir.path().join("资料.zip");let tar_path=dir.path().join("资料.tar");zip(&zip_path,&["资料/ÉCOLE_100%.txt","素材/image.png","other.bin"],false);tar(&tar_path);let pool=db().await;
        let a=add_one(&pool,&zip_path).await;let b=add_one(&pool,&tar_path).await;index(&pool,&a.id).await.unwrap();index(&pool,&b.id).await.unwrap();
        let all=search(&pool,Search::default()).await.unwrap();assert_eq!(all.total,4);
        for query in ["école","100%","ÉCOLE_", "资料/"] {let result=search(&pool,Search{query:query.into(),source_id:a.id.clone(),..Default::default()}).await.unwrap();assert_eq!(result.total,1,"{query}");}
        assert_eq!(search(&pool,Search{query:"missing".into(),..Default::default()}).await.unwrap().total,0);
        assert_eq!(search(&pool,Search{category:"image".into(),..Default::default()}).await.unwrap().total,1);
        assert_eq!(search(&pool,Search{source_id:b.id,..Default::default()}).await.unwrap().items[0].modified.as_deref(),Some("2023-11-14T22:13:20+00:00"));
    }
    #[tokio::test]
    async fn library1547_paging_dedup_and_remove_keeps_original(){let dir=tempfile::tempdir().unwrap();let path=dir.path().join("pages.zip");let names=(0..63).map(|i|format!("file{i:03}.txt")).collect::<Vec<_>>();zip(&path,&names.iter().map(String::as_str).collect::<Vec<_>>(),false);let original=std::fs::read(&path).unwrap();let pool=db().await;let item=add_one(&pool,&path).await;assert_eq!(add(&pool,vec![path.to_string_lossy().into()]).await.unwrap().len(),1);index(&pool,&item.id).await.unwrap();let first=search(&pool,Search::default()).await.unwrap();let last=search(&pool,Search{page:99,..Default::default()}).await.unwrap();assert_eq!((first.items.len(),last.page,last.items.len()),(50,2,13));assert!(first.items.iter().all(|a|last.items.iter().all(|b|a.path!=b.path)));remove(&pool,&item.id).await.unwrap();assert_eq!(search(&pool,Search::default()).await.unwrap().total,0);assert_eq!(std::fs::read(path).unwrap(),original);}
    #[tokio::test]
    async fn library1547_changed_missing_refresh_failure_and_recovery(){let dir=tempfile::tempdir().unwrap();let path=dir.path().join("changes.zip");zip(&path,&["old.txt"],false);let pool=db().await;let item=add_one(&pool,&path).await;index(&pool,&item.id).await.unwrap();check_source(&pool,&item.id,Some("old.txt")).await.unwrap();assert!(check_source(&pool,&item.id,Some("not-indexed.txt")).await.is_err());std::fs::write(&path,b"invalid changed archive").unwrap();assert!(check_source(&pool,&item.id,None).await.is_err());assert!(index(&pool,&item.id).await.is_err());assert_eq!(source(&pool,&item.id).await.unwrap().status,"failed");assert_eq!(search(&pool,Search::default()).await.unwrap().total,1);zip(&path,&["new.txt"],false);index(&pool,&item.id).await.unwrap();assert_eq!(search(&pool,Search::default()).await.unwrap().items[0].path,"new.txt");std::fs::remove_file(path).unwrap();assert!(check_source(&pool,&item.id,None).await.is_err());}
    #[test]
    fn library1547_recursive_scan_is_partial_stoppable_and_bounded_to_formats(){let dir=tempfile::tempdir().unwrap();std::fs::create_dir(dir.path().join("child")).unwrap();for name in ["a.zip","child/b.tar","child/c.7z","child/d.zip.001"]{std::fs::write(dir.path().join(name),b"candidate").unwrap();}let flag=AtomicBool::new(false);let result=scan(dir.path(),&flag).unwrap();assert_eq!(result.candidates.len(),4);assert!(!result.stopped);flag.store(true,Ordering::SeqCst);let stopped=scan(dir.path(),&flag).unwrap();assert!(stopped.stopped);assert!(stopped.candidates.is_empty());}
    #[tokio::test]
    async fn library1547_password_rejected_and_other_ready_sources_survive(){let dir=tempfile::tempdir().unwrap();let path=dir.path().join("password.zip");let ready=dir.path().join("ready.tar");zip(&path,&["secret.txt"],true);tar(&ready);let pool=db().await;let a=add_one(&pool,&path).await;let b=add_one(&pool,&ready).await;index(&pool,&b.id).await.unwrap();assert!(index(&pool,&a.id).await.unwrap_err().to_string().contains("密码"));assert_eq!(search(&pool,Search::default()).await.unwrap().total,1);assert_eq!(source(&pool,&a.id).await.unwrap().entry_count,0);}
    #[tokio::test]
    async fn library1547_cancel_keeps_committed_index_and_interrupted_state_is_explicit(){let dir=tempfile::tempdir().unwrap();let path=dir.path().join("cancel.zip");zip(&path,&["kept.txt"],false);let pool=db().await;let item=add_one(&pool,&path).await;index(&pool,&item.id).await.unwrap();let job=Job::start(&item.id).unwrap();cancel(&item.id);assert!(index_contents(&pool,&source(&pool,&item.id).await.unwrap(),job.flag.clone()).await.is_err());assert!(remove(&pool,&item.id).await.is_err());drop(job);assert_eq!(search(&pool,Search::default()).await.unwrap().total,1);sqlx::query("UPDATE archive_library_sources SET status='indexing' WHERE id=?").bind(&item.id).execute(&pool).await.unwrap();assert_eq!(list(&pool).await.unwrap()[0].status,"interrupted");index(&pool,&item.id).await.unwrap();assert_eq!(search(&pool,Search::default()).await.unwrap().total,1);}
    #[tokio::test]
    async fn library1548_password_encoding_unlock_and_credentials_never_persist(){
        let dir=tempfile::tempdir().unwrap();let path=dir.path().join("密码.zip");let mut writer=zip_aes::ZipWriter::new(File::create(&path).unwrap());let bytes=super::super::zip_password::encode("中文密码",super::super::zip_password::ZipPasswordEncoding::Cp936).unwrap();
        writer.start_file("docs/密码说明.txt",zip_aes::write::SimpleFileOptions::default().with_aes_encryption_bytes(zip_aes::AesMode::Aes256,&bytes)).unwrap();writer.write_all(b"protected").unwrap();writer.finish().unwrap();let pool=db().await;let item=add_one(&pool,&path).await;
        assert!(index(&pool,&item.id).await.is_err());assert_eq!(source(&pool,&item.id).await.unwrap().status,"locked");
        let wrong=Credentials{password:"wrong".into(),encoding:None};assert!(index_with_credentials(&pool,&item.id,&wrong,false).await.is_err());
        let correct=Credentials{password:"中文密码".into(),encoding:Some(super::super::zip_password::ZipPasswordEncoding::Cp936)};assert!(!index_with_credentials(&pool,&item.id,&correct,false).await.unwrap().skipped);
        let item=check_source(&pool,&item.id,Some("docs/密码说明.txt")).await.unwrap();assert!(item.encrypted);assert_eq!(item.source_encoding.as_deref(),Some("cp936"));assert!(!serde_json::to_string(&item).unwrap().contains("中文密码"));
        assert!(index_with_credentials(&pool,&item.id,&wrong,false).await.is_err());assert_eq!(search(&pool,Search::default()).await.unwrap().total,1);
        assert!(index_with_credentials(&pool,&item.id,&correct,false).await.unwrap().skipped);
    }
    #[tokio::test]
    async fn library1548_numeric_members_deduplicate_missing_middle_keeps_cached_hits(){
        let dir=tempfile::tempdir().unwrap();let original=dir.path().join("original.zip");zip(&original,&["资料.txt"],false);let bytes=std::fs::read(&original).unwrap();let size=(bytes.len()+2)/3;let mut parts=vec![];
        for (i,chunk) in bytes.chunks(size).enumerate(){let part=dir.path().join(format!("group.zip.{:03}",i+1));std::fs::write(&part,chunk).unwrap();parts.push(part)}
        let pool=db().await;let rows=add(&pool,parts.iter().rev().map(|p|p.to_string_lossy().into()).collect()).await.unwrap();assert_eq!(rows.len(),1);let id=&rows[0].id;index(&pool,id).await.unwrap();assert_eq!(source(&pool,id).await.unwrap().member_count,3);
        let held=std::fs::read(&parts[1]).unwrap();std::fs::remove_file(&parts[1]).unwrap();assert_eq!(audit(&pool,id).await.unwrap().availability,"incomplete");assert_eq!(search(&pool,Search::default()).await.unwrap().items[0].availability,"incomplete");assert!(check_source(&pool,id,None).await.is_err());
        std::fs::write(&parts[1],held).unwrap();assert!(!index_with_credentials(&pool,id,&Credentials::default(),false).await.unwrap().skipped);
    }
    #[tokio::test]
    async fn library1548_skip_unchanged_refresh_only_changes_and_preserve_cache(){
        let dir=tempfile::tempdir().unwrap();let path=dir.path().join("skip.zip");zip(&path,&["old.txt"],false);let pool=db().await;let item=add_one(&pool,&path).await;index(&pool,&item.id).await.unwrap();let indexed=source(&pool,&item.id).await.unwrap().indexed_at;
        assert!(index_with_credentials(&pool,&item.id,&Credentials::default(),false).await.unwrap().skipped);assert_eq!(source(&pool,&item.id).await.unwrap().indexed_at,indexed);
        zip(&path,&["new-name.txt","second.txt"],false);assert_eq!(audit(&pool,&item.id).await.unwrap().availability,"changed");assert_eq!(search(&pool,Search::default()).await.unwrap().items[0].path,"old.txt");assert!(check_source(&pool,&item.id,None).await.is_err());
        index(&pool,&item.id).await.unwrap();assert_eq!(search(&pool,Search::default()).await.unwrap().total,2);
    }
    #[tokio::test]
    async fn library1548_relocate_preserves_identity_and_old_cache_but_requires_refresh(){
        let dir=tempfile::tempdir().unwrap();let a=dir.path().join("old.zip");let b=dir.path().join("moved.zip");let other=dir.path().join("other.zip");zip(&a,&["old.txt"],false);zip(&other,&["other.txt"],false);let pool=db().await;let item=add_one(&pool,&a).await;index(&pool,&item.id).await.unwrap();add_one(&pool,&other).await;std::fs::rename(&a,&b).unwrap();
        assert_eq!(audit(&pool,&item.id).await.unwrap().availability,"missing");assert_eq!(search(&pool,Search::default()).await.unwrap().total,1);let moved=relocate(&pool,&item.id,&b).await.unwrap();assert_eq!(moved.id,item.id);assert_eq!(moved.availability,"changed");assert!(check_source(&pool,&item.id,None).await.is_err());assert!(relocate(&pool,&item.id,&other).await.is_err());
        index(&pool,&item.id).await.unwrap();assert_eq!(check_source(&pool,&item.id,None).await.unwrap().path,b.to_string_lossy());assert!(b.exists()&&other.exists());
    }
    #[tokio::test]
    async fn library1548_existing_sevenz_and_compressed_tar_metadata_use_original_browse_engine(){
        let dir=tempfile::tempdir().unwrap();let tar_path=dir.path().join("files.tar");tar(&tar_path);let gzip=dir.path().join("files.tar.gz");let mut encoder=flate2::write::GzEncoder::new(File::create(&gzip).unwrap(),flate2::Compression::default());encoder.write_all(&std::fs::read(tar_path).unwrap()).unwrap();encoder.finish().unwrap();
        let seven=dir.path().join("files.7z");let input=dir.path().join("说明.txt");std::fs::write(&input,b"sevenz content").unwrap();let output=std::process::Command::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/archive-engine/7z.exe")).args(["a","-t7z"]).arg(&seven).arg(&input).output().unwrap();assert!(output.status.success());
        let pool=db().await;for path in [&gzip,&seven]{let item=add_one(&pool,path).await;index(&pool,&item.id).await.unwrap();}assert_eq!(search(&pool,Search::default()).await.unwrap().total,2);
    }
    #[tokio::test]
    async fn library1548_standard_zip_alias_and_nonfirst_volume_change_are_tracked(){
        let dir=tempfile::tempdir().unwrap();let original=dir.path().join("source.zip");let target=dir.path().join("standard.zip");
        let mut writer=zip_aes::ZipWriter::new(File::create(&original).unwrap());writer.start_file("large.bin",zip_aes::write::SimpleFileOptions::default().compression_method(zip_aes::CompressionMethod::Stored)).unwrap();writer.write_all(&vec![42u8;180_000]).unwrap();writer.finish().unwrap();
        super::super::standard_zip_writer::split(&original,&target,65_536,||Ok(()),|_,_|{}).unwrap();let pool=db().await;
        // v1.5.47 could have retained the .zip identity before neighbouring volumes existed.
        sqlx::query("INSERT INTO archive_library_sources(id,path,name,format) VALUES('legacy',?,'standard.zip','ZIP')").bind(target.to_string_lossy().to_string()).execute(&pool).await.unwrap();
        let rows=add(&pool,vec![dir.path().join("standard.z01").to_string_lossy().into(),target.to_string_lossy().into()]).await.unwrap();assert_eq!(rows.len(),1);assert_eq!(rows[0].id,"legacy");index(&pool,"legacy").await.unwrap();
        let second=dir.path().join("standard.z02");let mut data=std::fs::read(&second).unwrap();data[100]^=1;std::fs::write(&second,data).unwrap();assert_eq!(audit(&pool,"legacy").await.unwrap().availability,"changed");assert!(check_source(&pool,"legacy",None).await.is_err());
    }
    #[tokio::test]
    async fn library1547_persisted_index_reopens_without_rebuilding(){let dir=tempfile::tempdir().unwrap();let path=dir.path().join("persistent.zip");zip(&path,&["kept.txt"],false);let options=sqlx::sqlite::SqliteConnectOptions::new().filename(dir.path().join("library.db")).create_if_missing(true);let pool=sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(options.clone()).await.unwrap();init_tables(&pool).await.unwrap();let item=add_one(&pool,&path).await;index(&pool,&item.id).await.unwrap();pool.close().await;let reopened=sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(options).await.unwrap();init_tables(&reopened).await.unwrap();assert_eq!(search(&reopened,Search::default()).await.unwrap().total,1);check_source(&reopened,&item.id,Some("kept.txt")).await.unwrap();}
}
