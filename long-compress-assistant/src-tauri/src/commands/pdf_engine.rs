use crate::services::pdf_analysis::{
    analyze_pdf_input as analyze_pdf_input_service, PdfInputAnalysisReport,
};
use crate::services::pdf_engine::{
    bundled_pdf_resource_root, validate_pdf_engine, PdfEngineStatus,
};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfInputAnalysisRequest {
    path: String,
    password: Option<String>,
}

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

#[tauri::command]
pub async fn analyze_pdf_input(
    app: tauri::AppHandle,
    request: PdfInputAnalysisRequest,
) -> Result<PdfInputAnalysisReport, String> {
    let app_resource_dir = app
        .path_resolver()
        .resource_dir()
        .ok_or_else(|| "PDF_ENGINE_RESOURCE_DIRECTORY_UNAVAILABLE".to_string())?;
    let resource_root = bundled_pdf_resource_root(&app_resource_dir);
    let validation_root = resource_root.clone();
    tauri::async_runtime::spawn_blocking(move || validate_pdf_engine(&validation_root))
        .await
        .map_err(|error| format!("PDF_ENGINE_PREFLIGHT_JOIN_FAILED: {error}"))?
        .map_err(|error| error.to_string())?;
    let qpdf = resource_root.join("pdf-engine").join("qpdf.exe");
    analyze_pdf_input_service(&qpdf, Path::new(&request.path), request.password.as_deref())
        .await
        .map_err(|error| error.to_string())
}
