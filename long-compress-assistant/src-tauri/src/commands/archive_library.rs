use crate::services::archive_library as library;
use tauri::command;
async fn pool()->Result<sqlx::SqlitePool,String>{crate::database::connection::get_pool().await.map_err(|e|e.to_string())}
#[command]
pub async fn library_sources()->Result<Vec<library::Source>,String>{library::list(&pool().await?).await.map_err(|e|format!("{e:#}"))}
#[command]
pub async fn library_add(paths:Vec<String>)->Result<Vec<library::Source>,String>{library::add(&pool().await?,paths).await.map_err(|e|format!("{e:#}"))}
#[command]
pub async fn library_remove(source_id:String)->Result<(),String>{library::remove(&pool().await?,&source_id).await.map_err(|e|format!("{e:#}"))}
#[command]
pub async fn library_index(source_id:String)->Result<(),String>{library::index(&pool().await?,&source_id).await.map_err(|e|format!("{e:#}"))}
#[command]
pub async fn library_search(request:library::Search)->Result<library::Results,String>{library::search(&pool().await?,request).await.map_err(|e|format!("{e:#}"))}
#[command]
pub async fn library_check(source_id:String,entry_path:Option<String>)->Result<library::Source,String>{library::check_source(&pool().await?,&source_id,entry_path.as_deref()).await.map_err(|e|format!("{e:#}"))}
#[command]
pub async fn library_cancel(job_id:String){library::cancel(&job_id)}
#[command]
pub async fn library_scan(directory:String,job_id:String)->Result<library::Scan,String>{let job=library::Job::start(&job_id).map_err(|e|e.to_string())?;tauri::async_runtime::spawn_blocking(move||library::scan(std::path::Path::new(&directory),&job.flag)).await.map_err(|e|e.to_string())?.map_err(|e|format!("{e:#}"))}
