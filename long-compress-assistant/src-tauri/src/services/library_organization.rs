//! User organization and reusable search conditions; no archive contents or credentials.
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use super::archive_library::{self as library, Search, Source};

pub async fn init_tables(pool: &SqlitePool) -> Result<()> {
    sqlx::query("CREATE TABLE IF NOT EXISTS archive_library_organization(source_id TEXT PRIMARY KEY REFERENCES archive_library_sources(id) ON DELETE CASCADE, favorite BOOLEAN NOT NULL DEFAULT 0,tags TEXT NOT NULL DEFAULT '[]')").execute(pool).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS archive_library_searches(id TEXT PRIMARY KEY,name TEXT NOT NULL UNIQUE,filters TEXT NOT NULL)").execute(pool).await?;
    Ok(())
}
#[derive(Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Organization { pub source_id:String,pub favorite:bool,pub tags:Vec<String> }
#[derive(Serialize,Deserialize)]
pub struct SavedSearch { pub id:String,pub name:String,pub filters:Search }
#[derive(Default,Serialize)]
pub struct OrganizationState { pub sources:Vec<Organization>,pub searches:Vec<SavedSearch> }
pub async fn state(pool:&SqlitePool)->Result<OrganizationState> {
    let rows:Vec<(String,bool,String)>=sqlx::query_as("SELECT source_id,favorite,tags FROM archive_library_organization").fetch_all(pool).await?;
    let searches:Vec<(String,String,String)>=sqlx::query_as("SELECT id,name,filters FROM archive_library_searches ORDER BY name COLLATE NOCASE,id").fetch_all(pool).await?;
    Ok(OrganizationState{sources:rows.into_iter().map(|(source_id,favorite,tags)|Organization{source_id,favorite,tags:serde_json::from_str(&tags).unwrap_or_default()}).collect(),searches:searches.into_iter().map(|(id,name,filters)|Ok(SavedSearch{id,name,filters:serde_json::from_str(&filters)?})).collect::<Result<_>>()?})
}
pub async fn organize(pool:&SqlitePool,mut value:Organization)->Result<OrganizationState> {
    library::source(pool,&value.source_id).await?;
    let mut names=std::collections::HashSet::new();
    value.tags=value.tags.into_iter().map(|s|s.trim().to_string()).filter(|s|!s.is_empty()&&names.insert(s.to_lowercase())).collect();
    ensure!(value.tags.len()<=12&&value.tags.iter().all(|s|s.chars().count()<=30),"每份资料最多 12 个标签，每个标签最多 30 字");
    sqlx::query("INSERT INTO archive_library_organization(source_id,favorite,tags) VALUES(?,?,?) ON CONFLICT(source_id) DO UPDATE SET favorite=excluded.favorite,tags=excluded.tags").bind(value.source_id).bind(value.favorite).bind(serde_json::to_string(&value.tags)?).execute(pool).await?;
    state(pool).await
}
pub async fn save_search(pool:&SqlitePool,name:&str,filters:Search)->Result<OrganizationState> {
    let name=name.trim();ensure!(!name.is_empty()&&name.chars().count()<=40,"请填写 1～40 字的搜索名称");
    ensure!(filters.query.chars().count()<=512&&filters.tag.chars().count()<=30,"搜索条件过长，请缩短后再保存");
    if !filters.source_id.is_empty(){library::source(pool,&filters.source_id).await?;}
    let(count,):(i64,)=sqlx::query_as("SELECT COUNT(*) FROM archive_library_searches WHERE name<>?").bind(name).fetch_one(pool).await?;
    ensure!(count<100,"最多保留 100 份常用搜索，请先删除不用的条件");
    sqlx::query("INSERT INTO archive_library_searches(id,name,filters) VALUES(?,?,?) ON CONFLICT(name) DO UPDATE SET filters=excluded.filters").bind(uuid::Uuid::new_v4().to_string()).bind(name).bind(serde_json::to_string(&filters)?).execute(pool).await?;
    state(pool).await
}
pub async fn delete_search(pool:&SqlitePool,id:&str)->Result<OrganizationState>{sqlx::query("DELETE FROM archive_library_searches WHERE id=?").bind(id).execute(pool).await?;state(pool).await}

pub async fn selection(pool:&SqlitePool,id:&str,revision:&str,paths:&[String],source_path:Option<&str>)->Result<Source>{
    ensure!(!paths.is_empty()&&paths.len()<=2000&&serde_json::to_string(paths)?.len()<=60000,"每次请选择 1～2000 个文件，路径过多时请分批取回");
    let item=library::check_source(pool,id,None).await?;
    ensure!(!revision.is_empty()&&revision==item.revision,"索引已更新，请重新搜索并选择这份资料中的文件，其他来源保留");
    if let Some(path)=source_path{ensure!(path.eq_ignore_ascii_case(&item.path),"来源位置已更新，请从资料库重新选择文件，其他来源保留");}
    let unique=paths.iter().collect::<std::collections::HashSet<_>>();
    let(count,):(i64,)=sqlx::query_as("SELECT COUNT(*) FROM archive_library_entries WHERE source_id=? AND path IN(SELECT value FROM json_each(?))").bind(id).bind(serde_json::to_string(paths)?).fetch_one(pool).await?;
    ensure!(count as usize==unique.len(),"部分所选文件已不在当前索引，请重新搜索并选择；不会改为整包取回");
    Ok(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::Path,fs::File,io::Write};
    async fn db()->SqlitePool{let pool=sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();sqlx::query("PRAGMA foreign_keys=ON").execute(&pool).await.unwrap();library::init_tables(&pool).await.unwrap();pool}
    fn zip(path:&Path,names:&[&str]){let mut zip=zip_aes::ZipWriter::new(File::create(path).unwrap());for name in names{zip.start_file(*name,zip_aes::write::SimpleFileOptions::default()).unwrap();zip.write_all(b"selected library data").unwrap();}zip.finish().unwrap();}
    async fn add(pool:&SqlitePool,path:&Path)->Source{library::add(pool,vec![path.to_string_lossy().into()]).await.unwrap();let row=library::list(pool).await.unwrap().into_iter().find(|r|r.path==path.to_string_lossy()).unwrap();library::index(pool,&row.id).await.unwrap();library::source(pool,&row.id).await.unwrap()}
    #[tokio::test]
    async fn library1549_labels_favorites_and_saved_conditions_round_trip_without_reindex(){
        let dir=tempfile::tempdir().unwrap();let a=dir.path().join("a.zip");let b=dir.path().join("b.zip");zip(&a,&["资料/合同.txt","photo.jpg"]);zip(&b,&["资料/合同.txt"]);let pool=db().await;let a=add(&pool,&a).await;let b=add(&pool,&b).await;
        organize(&pool,Organization{source_id:a.id.clone(),favorite:true,tags:vec![" 项目 ".into(),"项目".into(),"École".into(),"éCOLE".into()]}).await.unwrap();organize(&pool,Organization{source_id:b.id.clone(),favorite:false,tags:vec!["其他".into()]}).await.unwrap();
        let filters=Search{query:"合同".into(),tag:"项目".into(),favorites_only:true,category:"document".into(),..Default::default()};assert_eq!(library::search(&pool,filters.clone()).await.unwrap().total,1);
        save_search(&pool,"项目合同",filters.clone()).await.unwrap();save_search(&pool,"项目合同",filters.clone()).await.unwrap();init_tables(&pool).await.unwrap();let saved=state(&pool).await.unwrap();assert_eq!(saved.searches.len(),1);assert_eq!(saved.sources.iter().find(|r|r.source_id==a.id).unwrap().tags,vec!["项目","École"]);assert_eq!(library::search(&pool,saved.searches[0].filters.clone()).await.unwrap().items[0].source_id,a.id);assert_eq!(library::source(&pool,&a.id).await.unwrap().indexed_at,a.indexed_at);
        delete_search(&pool,&saved.searches[0].id).await.unwrap();assert!(state(&pool).await.unwrap().searches.is_empty());
        library::remove(&pool,&a.id).await.unwrap();assert!(state(&pool).await.unwrap().sources.iter().all(|r|r.source_id!=a.id));assert!(Path::new(&a.path).exists());
    }
    #[tokio::test]
    async fn library1549_selection_keeps_exact_revision_paths_and_rejects_empty_or_missing(){
        let dir=tempfile::tempdir().unwrap();let path=dir.path().join("source.zip");zip(&path,&["docs/one.txt","docs/two.txt"]);let pool=db().await;let row=add(&pool,&path).await;
        assert_eq!(selection(&pool,&row.id,&row.revision,&["docs/one.txt".into()],Some(&row.path)).await.unwrap().id,row.id);
        for paths in [vec![],vec!["absent.txt".into()]]{assert!(selection(&pool,&row.id,&row.revision,&paths,None).await.is_err());}
        assert!(selection(&pool,&row.id,"old-revision",&["docs/one.txt".into()],None).await.is_err());assert!(selection(&pool,&row.id,&row.revision,&["docs/one.txt".into()],Some("other.zip")).await.is_err());
        let moved=dir.path().join("moved.zip");std::fs::rename(&path,&moved).unwrap();assert!(selection(&pool,&row.id,&row.revision,&["docs/one.txt".into()],None).await.is_err());assert_eq!(library::search(&pool,Search::default()).await.unwrap().total,2);
    }
    #[tokio::test]
    async fn library1549_empty_label_filter_does_not_change_old_search_and_unknown_tags_stay_empty(){let dir=tempfile::tempdir().unwrap();let path=dir.path().join("source.zip");zip(&path,&["100%_资料.txt"]);let pool=db().await;add(&pool,&path).await;assert_eq!(library::search(&pool,Search{query:"%_".into(),..Default::default()}).await.unwrap().total,1);for filters in [Search{tag:"不存在".into(),..Default::default()},Search{favorites_only:true,..Default::default()}]{assert_eq!(library::search(&pool,filters).await.unwrap().total,0);}}
}
