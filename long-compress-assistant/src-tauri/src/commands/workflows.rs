use crate::services::workflows as workflow;
async fn pool() -> Result<sqlx::SqlitePool, String> {
    crate::database::connection::get_pool()
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn workflow_state() -> Result<workflow::State, String> {
    workflow::state(&pool().await?)
        .await
        .map_err(|e| format!("{e:#}"))
}
#[tauri::command]
pub async fn workflow_save_plan(plan: workflow::Plan) -> Result<workflow::Plan, String> {
    workflow::save_plan(&pool().await?, plan)
        .await
        .map_err(|e| format!("{e:#}"))
}
#[tauri::command]
pub async fn workflow_delete_plan(id: String) -> Result<(), String> {
    workflow::delete_plan(&pool().await?, &id)
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn workflow_save_run(run: workflow::Run) -> Result<(), String> {
    workflow::save_run(&pool().await?, run)
        .await
        .map_err(|e| format!("{e:#}"))
}
#[tauri::command]
pub async fn workflow_begin_directory(parent: String, name: String) -> Result<String, String> {
    workflow::begin_directory(std::path::Path::new(&parent), &name).map_err(|e| format!("{e:#}"))
}
#[tauri::command]
pub async fn workflow_collect(
    paths: Vec<String>,
    extensions: String,
    exclude: String,
    job_id: String,
    output_base: Option<String>,
) -> Result<workflow::Collection, String> {
    tauri::async_runtime::spawn_blocking(move || {
        workflow::collect(
            paths,
            &extensions,
            std::path::Path::new(&exclude),
            &job_id,
            output_base.as_deref().map(std::path::Path::new),
        )
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))
}
#[tauri::command]
pub async fn workflow_children(path: String) -> Result<Vec<String>, String> {
    workflow::children(std::path::Path::new(&path)).map_err(|e| format!("{e:#}"))
}
#[tauri::command]
pub async fn workflow_cancel_inputs(job_id: String) {
    crate::services::archive_library::cancel(&job_id)
}
