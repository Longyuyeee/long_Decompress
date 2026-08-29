use crate::services::pdf_engine::{
    bundled_pdf_resource_root, validate_pdf_engine, PdfEngineStatus,
};

#[tauri::command]
pub async fn preflight_pdf_engine(app: tauri::AppHandle) -> Result<PdfEngineStatus, String> {
    let app_resource_dir = app
        .path_resolver()
        .resource_dir()
        .ok_or_else(|| "PDF_ENGINE_RESOURCE_DIRECTORY_UNAVAILABLE".to_string())?;
    let resource_root = bundled_pdf_resource_root(&app_resource_dir);
    tauri::async_runtime::spawn_blocking(move || validate_pdf_engine(&resource_root))
        .await
        .map_err(|error| format!("PDF_ENGINE_PREFLIGHT_JOIN_FAILED: {error}"))?
        .map_err(|error| error.to_string())
}
