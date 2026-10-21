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
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all="camelCase")]
pub struct Source { pub id:String, pub path:String, pub name:String, pub format:String, pub status:String, pub note:String, pub revision:String, pub indexed_at:Option<String>, pub entry_count:i64 }
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
pub struct Hit { pub source_id:String, pub source_path:String, pub source_name:String, pub format:String, pub path:String, pub name:String, pub extension:String, pub size:i64, pub modified:Option<String> }
#[derive(Serialize)]
pub struct Results { pub items:Vec<Hit>, pub total:i64, pub page:u32 }

pub struct Job { id:String, pub flag:Arc<AtomicBool> }
impl Job { pub fn start(id:&str)->Result<Self>{let mut jobs=JOBS.lock().unwrap();ensure!(!jobs.contains_key(id),"这份来源正在处理中，请稍候");let flag=Arc::new(AtomicBool::new(false));jobs.insert(id.into(),flag.clone());Ok(Self{id:id.into(),flag})} }
impl Drop for Job {fn drop(&mut self){JOBS.lock().unwrap().remove(&self.id);}}
pub fn cancel(id:&str){if let Some(flag)=JOBS.lock().unwrap().get(id){flag.store(true,Ordering::SeqCst)}}
fn checkpoint(flag:&AtomicBool)->Result<()>{ensure!(!flag.load(Ordering::SeqCst),"已停止建库，已完成来源保留");Ok(())}
fn candidate(path:&Path)->Option<Candidate>{
    let extension=path.extension()?.to_str()?.to_ascii_lowercase();
    let format=match extension.as_str(){"zip"=>"ZIP","tar"=>"TAR",_=>return None};
    Some(Candidate{path:path.to_string_lossy().into(),name:path.file_name()?.to_string_lossy().into(),format:format.into()})
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
    result.candidates.sort_by(|a,b|a.path.to_lowercase().cmp(&b.path.to_lowercase()));Ok(result)
}
fn canonical(path:&Path)->Result<PathBuf>{
    let meta=std::fs::symlink_metadata(path).with_context(||format!("找不到来源：{}",path.display()))?;
    ensure!(meta.is_file()&&!meta.file_type().is_symlink(),"来源必须是普通 ZIP/TAR 文件");
    let value=std::fs::canonicalize(path)?;
    // Keep familiar paths in the UI; filesystem APIs retain long-path support.
    let display=value.to_string_lossy();
    Ok(if let Some(unc)=display.strip_prefix(r"\\?\UNC\"){PathBuf::from(format!(r"\\{unc}"))}else{PathBuf::from(display.strip_prefix(r"\\?\").unwrap_or(&display))})
}
pub fn revision(path:&Path)->Result<String>{let meta=std::fs::metadata(path).context("原包缺失或无法访问，请重新选择来源")?;ensure!(meta.is_file(),"原包不再是文件");Ok(format!("{}:{:?}",meta.len(),meta.modified()?))}
pub async fn add(pool:&SqlitePool,paths:Vec<String>)->Result<Vec<Source>>{
    ensure!(paths.len()<=10_000,"一次最多添加 10000 份来源，请分批选择");
    let mut items=vec![];
    for path in paths{let path=canonical(Path::new(&path))?;items.push(candidate(&path).context("本版资源库先支持独立普通 ZIP/TAR，请选择对应来源")?)}
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

pub async fn index(pool:&SqlitePool,id:&str)->Result<()> {
    let job=Job::start(id)?;let item=source(pool,id).await?;
    sqlx::query("UPDATE archive_library_sources SET status='indexing',note='' WHERE id=?").bind(id).execute(pool).await?;
    let result=index_contents(pool,&item,job.flag.clone()).await;
    if let Err(error)=&result{let status=if job.flag.load(Ordering::SeqCst){"interrupted"}else{"failed"};sqlx::query("UPDATE archive_library_sources SET status=?,note=? WHERE id=?").bind(status).bind(format!("{error:#}")).bind(id).execute(pool).await?;}
    result
}
async fn index_contents(pool:&SqlitePool,item:&Source,flag:Arc<AtomicBool>)->Result<()> {
    let path=PathBuf::from(&item.path);checkpoint(&flag)?;let before=revision(&path)?;
    ensure!(super::split_archive_detector::SplitArchiveDetector::detect_split_archive(&path)?.is_none(),"分卷资料索引将在下一阶段接入，请先使用原浏览/提取");
    use std::io::Read;
    let mut header=[0u8;4];std::fs::File::open(&path)?.read_exact(&mut header)?;
    ensure!(&header[..2]!=b"MZ","自解压来源请继续使用原浏览/提取");
    let declared=if item.format=="ZIP" {Some(super::zip_edit::declared_count(&mut std::fs::File::open(&path)?).context("ZIP 目录必须完整且来自独立包")?)}else{None};
    let result=super::archive_browser::browse_archive_cancellable_at(&path,None,flag.clone(),None,None).await?;
    ensure!(result.format==item.format&&["ZIP","TAR"].contains(&result.format.as_str()),"实际内容不是独立普通 ZIP/TAR，请使用原浏览器");
    if let Some(count)=declared{ensure!(count==result.entries.len() as u64,"ZIP 存在重复目录条目，不能唯一索引，请使用原浏览/提取");}
    ensure!(!result.encrypted,"密码资料索引将在下一阶段接入，请先使用原浏览/提取");
    ensure!(result.entries.len()<=MAX_ENTRIES,"单包超过 100 万条，请先按资料范围拆分");
    ensure!(before==revision(&path)?,"建库时原包发生变化，请重新刷新");checkpoint(&flag)?;
    let mut tx=pool.begin().await?;
    sqlx::query("DELETE FROM archive_library_entries WHERE source_id=?").bind(&item.id).execute(&mut *tx).await?;
    let mut paths=HashSet::new();let mut count=0;
    for entry in result.entries.into_iter().filter(|entry|!entry.is_dir){checkpoint(&flag)?;ensure!(paths.insert(entry.path.clone()),"包内位置重复，无法唯一定位，请继续使用原浏览/提取");
        let extension=Path::new(&entry.name).extension().and_then(|v|v.to_str()).unwrap_or("").to_lowercase();
        sqlx::query("INSERT INTO archive_library_entries(source_id,path,name,search_path,extension,size,modified) VALUES(?,?,?,?,?,?,?)").bind(&item.id).bind(&entry.path).bind(entry.name).bind(entry.path.to_lowercase()).bind(extension).bind(i64::try_from(entry.size)?).bind(entry.modified).execute(&mut *tx).await?;count+=1;
    }
    checkpoint(&flag)?;ensure!(before==revision(&path)?,"建立索引期间原包发生变化，请重新刷新");
    sqlx::query("UPDATE archive_library_sources SET status='ready',note='',revision=?,indexed_at=?,entry_count=? WHERE id=?").bind(before).bind(chrono::Utc::now().to_rfc3339()).bind(count).bind(&item.id).execute(&mut *tx).await?;
    tx.commit().await?;Ok(())
}
pub async fn check_source(pool:&SqlitePool,id:&str,entry:Option<&str>)->Result<Source>{
    let item=source(pool,id).await?;ensure!(item.status=="ready","这份来源尚未完成建库，请先刷新索引");
    ensure!(revision(Path::new(&item.path))?==item.revision,"原包大小或修改时间已变化，请刷新这份来源后再查看/取回");
    if let Some(entry)=entry{let (count,):(i64,)=sqlx::query_as("SELECT COUNT(*) FROM archive_library_entries WHERE source_id=? AND path=?").bind(id).bind(entry).fetch_one(pool).await?;ensure!(count==1,"所选条目已不在当前索引中，请重新搜索");}Ok(item)
}
fn category_extensions(category:&str)->&'static [&'static str]{match category{"image"=>&["png","jpg","jpeg","gif","webp","bmp","svg","avif","heic"],"document"=>&["txt","md","pdf","doc","docx","xls","xlsx","ppt","pptx","csv","rtf"],"audio"=>&["mp3","flac","wav","aac","ogg","m4a"],"video"=>&["mp4","mkv","mov","avi","webm","wmv"],"archive"=>&["zip","tar","7z","rar","gz","xz","bz2","zst"],_=>&[]}}
pub async fn search(pool:&SqlitePool,request:Search)->Result<Results>{
    ensure!(request.query.chars().count()<=512,"搜索文字请控制在 512 字以内");
    let query=request.query.trim().to_lowercase();let escaped=query.replace('\\',"\\\\").replace('%',"\\%").replace('_',"\\_");
    let mut filter=String::from(" FROM archive_library_entries e JOIN archive_library_sources s ON s.id=e.source_id WHERE s.status='ready' AND (e.search_path LIKE ? ESCAPE '\\') AND (?='' OR s.id=?)");
    let extensions=category_extensions(&request.category);if !extensions.is_empty(){filter.push_str(" AND e.extension IN (");filter.push_str(&vec!["?";extensions.len()].join(","));filter.push(')');}
    let pattern=format!("%{escaped}%");
    let count_sql=format!("SELECT COUNT(*){filter}");
    let mut count=sqlx::query_as::<_,(i64,)>(&count_sql).bind(&pattern).bind(&request.source_id).bind(&request.source_id);for value in extensions{count=count.bind(value)}let (total,)=count.fetch_one(pool).await?;
    let page=request.page.max(1).min(((total.max(1) as u64+49)/50) as u32);
    let rows_sql=format!("SELECT s.id source_id,s.path source_path,s.name source_name,s.format,e.path,e.name,e.extension,e.size,e.modified{filter} ORDER BY e.name COLLATE NOCASE,s.name COLLATE NOCASE,s.id,e.path LIMIT 50 OFFSET ?");
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
    async fn library1547_changed_missing_refresh_failure_and_recovery(){let dir=tempfile::tempdir().unwrap();let path=dir.path().join("changes.zip");zip(&path,&["old.txt"],false);let pool=db().await;let item=add_one(&pool,&path).await;index(&pool,&item.id).await.unwrap();check_source(&pool,&item.id,Some("old.txt")).await.unwrap();assert!(check_source(&pool,&item.id,Some("not-indexed.txt")).await.is_err());std::fs::write(&path,b"invalid changed archive").unwrap();assert!(check_source(&pool,&item.id,None).await.is_err());assert!(index(&pool,&item.id).await.is_err());assert_eq!(source(&pool,&item.id).await.unwrap().status,"failed");assert_eq!(search(&pool,Search::default()).await.unwrap().total,0);zip(&path,&["new.txt"],false);index(&pool,&item.id).await.unwrap();assert_eq!(search(&pool,Search::default()).await.unwrap().items[0].path,"new.txt");std::fs::remove_file(path).unwrap();assert!(check_source(&pool,&item.id,None).await.is_err());}
    #[test]
    fn library1547_recursive_scan_is_partial_stoppable_and_bounded_to_formats(){let dir=tempfile::tempdir().unwrap();std::fs::create_dir(dir.path().join("child")).unwrap();for name in ["a.zip","child/b.tar","child/c.7z","child/d.zip.001"]{std::fs::write(dir.path().join(name),b"candidate").unwrap();}let flag=AtomicBool::new(false);let result=scan(dir.path(),&flag).unwrap();assert_eq!(result.candidates.len(),2);assert!(!result.stopped);flag.store(true,Ordering::SeqCst);let stopped=scan(dir.path(),&flag).unwrap();assert!(stopped.stopped);assert!(stopped.candidates.is_empty());}
    #[tokio::test]
    async fn library1547_password_rejected_and_other_ready_sources_survive(){let dir=tempfile::tempdir().unwrap();let path=dir.path().join("password.zip");let ready=dir.path().join("ready.tar");zip(&path,&["secret.txt"],true);tar(&ready);let pool=db().await;let a=add_one(&pool,&path).await;let b=add_one(&pool,&ready).await;index(&pool,&b.id).await.unwrap();assert!(index(&pool,&a.id).await.unwrap_err().to_string().contains("密码"));assert_eq!(search(&pool,Search::default()).await.unwrap().total,1);assert_eq!(source(&pool,&a.id).await.unwrap().entry_count,0);}
    #[tokio::test]
    async fn library1547_cancel_keeps_committed_index_and_interrupted_state_is_explicit(){let dir=tempfile::tempdir().unwrap();let path=dir.path().join("cancel.zip");zip(&path,&["kept.txt"],false);let pool=db().await;let item=add_one(&pool,&path).await;index(&pool,&item.id).await.unwrap();let job=Job::start(&item.id).unwrap();cancel(&item.id);assert!(index_contents(&pool,&source(&pool,&item.id).await.unwrap(),job.flag.clone()).await.is_err());assert!(remove(&pool,&item.id).await.is_err());drop(job);assert_eq!(search(&pool,Search::default()).await.unwrap().total,1);sqlx::query("UPDATE archive_library_sources SET status='indexing' WHERE id=?").bind(&item.id).execute(&pool).await.unwrap();assert_eq!(list(&pool).await.unwrap()[0].status,"interrupted");index(&pool,&item.id).await.unwrap();assert_eq!(search(&pool,Search::default()).await.unwrap().total,1);}
    #[tokio::test]
    async fn library1547_persisted_index_reopens_without_rebuilding(){let dir=tempfile::tempdir().unwrap();let path=dir.path().join("persistent.zip");zip(&path,&["kept.txt"],false);let options=sqlx::sqlite::SqliteConnectOptions::new().filename(dir.path().join("library.db")).create_if_missing(true);let pool=sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(options.clone()).await.unwrap();init_tables(&pool).await.unwrap();let item=add_one(&pool,&path).await;index(&pool,&item.id).await.unwrap();pool.close().await;let reopened=sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(options).await.unwrap();init_tables(&reopened).await.unwrap();assert_eq!(search(&reopened,Search::default()).await.unwrap().total,1);check_source(&reopened,&item.id,Some("kept.txt")).await.unwrap();}
}
