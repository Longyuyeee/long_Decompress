use crate::services::source_comparison::{self as comparison, Selection, Source, SourceRef};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};
#[tauri::command]
pub async fn comparison_read(
    window: tauri::Window,
    path: String,
    job_id: String,
) -> Result<Source, String> {
    let job = crate::services::archive_library::Job::start(&job_id).map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        comparison::read(Path::new(&path), job.flag.clone(), |progress| {
            let _ = window.emit(
                "comparison-read-progress",
                serde_json::json!({"jobId":job_id,"progress":progress}),
            );
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))
}
#[tauri::command]
pub async fn comparison_output(sources: Vec<SourceRef>, output: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        comparison::validate_output(&sources, Path::new(&output)).map(|_| ())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))
}
#[tauri::command]
pub async fn comparison_validate(
    request: Selection,
    output: String,
    password: Option<String>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let source = comparison::validate(
            &request,
            Path::new(&output),
            Arc::new(AtomicBool::new(false)),
        )?;
        comparison::validate_password(&request, &source, password.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievalResult {
    path: String,
    output_bytes: u64,
}

#[tauri::command]
pub async fn comparison_retrieve(
    window: tauri::Window,
    task_id: String,
    request: Selection,
    output: String,
    password: Option<String>,
) -> Result<RetrievalResult, String> {
    let flag = super::compression::register_task_cancellation(&task_id)?;
    let guard = super::compression::TaskCancellationGuard::new(&task_id);
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let log = |message: String, severity: &str| {
            let _ = window.emit("task-log", serde_json::json!({
                "task_id":task_id,"message":message,"severity":severity,
                "timestamp":chrono::Utc::now().to_rfc3339()
            }));
        };
        log(format!("资料对比取回：精确选择 {} 项；原件保留", request.paths.len()), "info");
        let result = comparison::retrieve_with_password(&request, Path::new(&output), password.as_deref(), flag, |progress| {
            let percent = if progress.total_bytes > 0 {
                progress.processed_bytes as f64 / progress.total_bytes as f64 * 95.0
            } else { 0.0 };
            let _ = window.emit("task-progress", serde_json::json!({
                "task_id":task_id,"progress":percent,
                "stage":if progress.phase=="publishing" {"Finalizing"} else {"Extracting"},
                "processed_bytes":progress.processed_bytes,"total_bytes":progress.total_bytes,
                "current_file":progress.path
            }));
        });
        match result {
            Ok((path, bytes)) => {
                log(format!("所选范围处理完成，实际提交 {bytes} 字节；同名项按保留/跳过策略处理；结果：{path}"), "success");
                Ok(RetrievalResult {path, output_bytes:bytes})
            }
            Err(error) => {
                let error = crate::services::archive_io::message(&error);
                log(format!("最终失败原因：{error}"), "error");
                Err(error)
            }
        }
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn comparison_verify(
    window: tauri::Window,
    job_id: String,
    request: crate::services::source_comparison_content::Request,
) -> Result<crate::services::source_comparison_content::Batch, String> {
    let job = crate::services::archive_library::Job::start(&job_id).map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        crate::services::source_comparison_content::run(
            &request,
            job.flag.clone(),
            |progress| {
                let _ = window.emit(
                    "comparison-content-progress",
                    serde_json::json!({"jobId":job_id,"progress":progress}),
                );
            },
            |entry| {
                let _ = window.emit(
                    "comparison-content-result",
                    serde_json::json!({"jobId":job_id,"entry":entry}),
                );
            },
        )
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn comparison_package_prepare(parts:Vec<crate::services::comparison_package::Part>,job_id:String)->Result<crate::services::comparison_package::Prepared,String>{
 let job=crate::services::archive_library::Job::start(&job_id).map_err(|e|e.to_string())?;
 tauri::async_runtime::spawn_blocking(move||crate::services::comparison_package::prepare(&parts,&crate::utils::app_paths::app_data_dir().join("comparison-package-seeds"),job.flag.clone())).await.map_err(|e|e.to_string())?.map_err(|e|format!("{e:#}"))
}
