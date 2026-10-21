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

#[tauri::command]
pub async fn workflow_witness(
    window: tauri::Window,
    paths: Vec<String>,
    policy: crate::services::workflow_recovery::Policy,
    job_id: String,
    metadata_only: bool,
) -> Result<crate::services::workflow_recovery::Proof, String> {
    tauri::async_runtime::spawn_blocking(move||{let id=job_id.clone();crate::services::workflow_recovery::witness(paths,policy,&job_id,metadata_only,&mut |progress|{let _=window.emit("workflow-proof-progress",serde_json::json!({"jobId":id,"files":progress.files,"bytes":progress.bytes,"path":progress.path}));})}).await.map_err(|e|e.to_string())?.map_err(|e|format!("{e:#}"))
}
#[tauri::command]
pub async fn workflow_check_reuse(
    window: tauri::Window,
    run_id: String,
    count: usize,
    job_id: String,
) -> Result<crate::services::workflow_recovery::Report, String> {
    let record: (String,) = sqlx::query_as("SELECT record FROM workflow_runs WHERE id=?")
        .bind(run_id)
        .fetch_one(&pool().await?)
        .await
        .map_err(|e| e.to_string())?;
    let run: workflow::Run = serde_json::from_str(&record.0).map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move||{let id=job_id.clone();crate::services::workflow_recovery::check(&run,count,&job_id,&mut |progress|{let _=window.emit("workflow-proof-progress",serde_json::json!({"jobId":id,"files":progress.files,"bytes":progress.bytes,"path":progress.path}));})}).await.map_err(|e|e.to_string())?.map_err(|e|format!("{e:#}"))
}

#[tauri::command]
pub async fn workflow_run(id: String) -> Result<workflow::Run, String> {
    let record: (String,) = sqlx::query_as("SELECT record FROM workflow_runs WHERE id=?")
        .bind(id)
        .fetch_one(&pool().await?)
        .await
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&record.0).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn workflow_trigger_state() -> Result<crate::services::workflow_triggers::State, String> {
    crate::services::workflow_triggers::initialize(&pool().await?)
        .await
        .map_err(|e| format!("{e:#}"))
}
#[tauri::command]
pub async fn workflow_trigger_poll(
    busy: Option<bool>,
) -> Result<crate::services::workflow_triggers::State, String> {
    crate::services::workflow_triggers::poll(&pool().await?, busy.unwrap_or(false))
        .await
        .map_err(|e| format!("{e:#}"))
}
#[tauri::command]
pub async fn workflow_trigger_save(
    rule: crate::services::workflow_triggers::Rule,
) -> Result<crate::services::workflow_triggers::State, String> {
    crate::services::workflow_triggers::save_rule(&pool().await?, rule)
        .await
        .map_err(|e| format!("{e:#}"))
}
#[tauri::command]
pub async fn workflow_trigger_enabled(
    id: String,
    enabled: bool,
) -> Result<crate::services::workflow_triggers::State, String> {
    crate::services::workflow_triggers::set_enabled(&pool().await?, &id, enabled)
        .await
        .map_err(|e| format!("{e:#}"))
}
#[tauri::command]
pub async fn workflow_trigger_delete(
    id: String,
) -> Result<crate::services::workflow_triggers::State, String> {
    crate::services::workflow_triggers::delete_rule(&pool().await?, &id)
        .await
        .map_err(|e| format!("{e:#}"))
}
#[tauri::command]
pub async fn workflow_trigger_pause(
    paused: bool,
) -> Result<crate::services::workflow_triggers::State, String> {
    crate::services::workflow_triggers::pause(&pool().await?, paused)
        .await
        .map_err(|e| format!("{e:#}"))
}
#[tauri::command]
pub async fn workflow_trigger_claim(
    id: String,
    run_id: String,
) -> Result<crate::services::workflow_triggers::Item, String> {
    crate::services::workflow_triggers::claim(&pool().await?, &id, &run_id)
        .await
        .map_err(|e| format!("{e:#}"))
}
#[tauri::command]
pub async fn workflow_trigger_action(
    id: String,
    action: String,
) -> Result<crate::services::workflow_triggers::State, String> {
    crate::services::workflow_triggers::action(&pool().await?, &id, &action)
        .await
        .map_err(|e| format!("{e:#}"))
}
#[tauri::command]
pub async fn workflow_trigger_finish() -> Result<crate::services::workflow_triggers::State, String>
{
    crate::services::workflow_triggers::finish(&pool().await?)
        .await
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn workflow_schedule_save(
    rule: crate::services::workflow_schedules::Schedule,
) -> Result<crate::services::workflow_triggers::State, String> {
    crate::services::workflow_schedules::save(&pool().await?, rule)
        .await
        .map_err(|e| format!("{e:#}"))
}
#[tauri::command]
pub async fn workflow_schedule_action(
    id: String,
    action: String,
) -> Result<crate::services::workflow_triggers::State, String> {
    crate::services::workflow_schedules::action(&pool().await?, &id, &action)
        .await
        .map_err(|e| format!("{e:#}"))
}
