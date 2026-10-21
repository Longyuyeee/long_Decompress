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
pub async fn library_index(source_id:String,credentials:Option<library::Credentials>,force:Option<bool>)->Result<library::IndexOutcome,String>{library::index_with_credentials(&pool().await?,&source_id,&credentials.unwrap_or_default(),force.unwrap_or(false)).await.map_err(|e|format!("{e:#}"))}
#[command]
pub async fn library_search(request:library::Search)->Result<library::Results,String>{library::search(&pool().await?,request).await.map_err(|e|format!("{e:#}"))}
#[command]
pub async fn library_check(source_id:String,entry_path:Option<String>)->Result<library::Source,String>{library::check_source(&pool().await?,&source_id,entry_path.as_deref()).await.map_err(|e|format!("{e:#}"))}
#[command]
pub async fn library_cancel(job_id:String){library::cancel(&job_id)}
#[command]
pub async fn library_scan(directory:String,job_id:String)->Result<library::Scan,String>{let job=library::Job::start(&job_id).map_err(|e|e.to_string())?;tauri::async_runtime::spawn_blocking(move||library::scan(std::path::Path::new(&directory),&job.flag)).await.map_err(|e|e.to_string())?.map_err(|e|format!("{e:#}"))}

#[command]
pub async fn library_audit(source_id:String)->Result<library::Source,String>{library::audit(&pool().await?,&source_id).await.map_err(|e|format!("{e:#}"))}
#[command]
pub async fn library_relocate(source_id:String,path:String)->Result<library::Source,String>{library::relocate(&pool().await?,&source_id,std::path::Path::new(&path)).await.map_err(|e|format!("{e:#}"))}

#[command]
pub async fn library_organization()->Result<crate::services::library_organization::OrganizationState,String>{crate::services::library_organization::state(&pool().await?).await.map_err(|e|format!("{e:#}"))}
#[command]
pub async fn library_organize(value:crate::services::library_organization::Organization)->Result<crate::services::library_organization::OrganizationState,String>{crate::services::library_organization::organize(&pool().await?,value).await.map_err(|e|format!("{e:#}"))}
#[command]
pub async fn library_save_search(name:String,filters:library::Search)->Result<crate::services::library_organization::OrganizationState,String>{crate::services::library_organization::save_search(&pool().await?,&name,filters).await.map_err(|e|format!("{e:#}"))}
#[command]
pub async fn library_delete_search(id:String)->Result<crate::services::library_organization::OrganizationState,String>{crate::services::library_organization::delete_search(&pool().await?,&id).await.map_err(|e|format!("{e:#}"))}
#[command]
pub async fn library_selection(source_id:String,revision:String,paths:Vec<String>)->Result<library::Source,String>{crate::services::library_organization::selection(&pool().await?,&source_id,&revision,&paths,None).await.map_err(|e|format!("{e:#}"))}
