use crate::services::pdf_analysis::{
    analyze_pdf_input as analyze_pdf_input_service, PdfInputAnalysisReport,
};
use crate::services::pdf_engine::{
    bundled_pdf_resource_root, validate_pdf_engine, PdfEngineStatus,
};
use crate::services::pdf_publish::{
    execute_pdf_publication_transaction_observed, PdfPublicationStage, PublishedPdfOutput,
};
use crate::services::pdf_transform::{PdfOptimizationMode, PdfTransformRequest};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use tauri::Window;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfInputAnalysisRequest {
    path: String,
    password: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PdfCompressionExecutionRequest {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub mode: PdfOptimizationMode,
    #[serde(default)]
    pub confirmed_lossy_image_changes: bool,
    #[serde(default)]
    pub preserve_mark_of_web: bool,
    #[serde(default)]
    pub allow_larger_output: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PdfOptimizationDestinationPlan {
    pub destination: PathBuf,
}

impl PdfPublicationStage {
    fn event_name(self) -> &'static str {
        match self {
            Self::Transforming => "Transforming",
            Self::Validating => "Validating",
            Self::Publishing => "Publishing",
        }
    }

    fn log_message(self) -> &'static str {
        match self {
            Self::Transforming => "正在重新分析并生成隔离 PDF 暂存输出",
            Self::Validating => "正在检查 PDF 结构与源文件一致性",
            Self::Publishing => "验证通过，正在原子发布最终 PDF",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct PdfTaskProgress {
    task_id: String,
    stage: String,
    progress: f32,
    current_file: String,
    processed_bytes: u64,
    total_bytes: u64,
    output_bytes: u64,
    output_bytes_estimated: bool,
}

fn normalized_destination_key(path: &Path) -> String {
    let key = path.to_string_lossy().replace('/', "\\");
    #[cfg(windows)]
    let key = key.to_lowercase();
    key
}

fn plan_pdf_destination(
    source: &Path,
    output_directory: Option<&Path>,
    mode: PdfOptimizationMode,
    reserved_destinations: &[PathBuf],
) -> Result<PdfOptimizationDestinationPlan, String> {
    let metadata = std::fs::metadata(source)
        .map_err(|error| format!("PDF_DESTINATION_SOURCE_UNAVAILABLE: {error}"))?;
    if !metadata.is_file() {
        return Err("PDF_DESTINATION_SOURCE_NOT_FILE".to_string());
    }
    let directory = output_directory
        .map(Path::to_path_buf)
        .or_else(|| source.parent().map(Path::to_path_buf))
        .ok_or_else(|| "PDF_DESTINATION_DIRECTORY_UNAVAILABLE".to_string())?;
    if !directory.is_dir() {
        return Err(format!(
            "PDF_DESTINATION_DIRECTORY_NOT_FOUND: {}",
            directory.display()
        ));
    }
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "PDF_DESTINATION_SOURCE_NAME_INVALID".to_string())?;
    let mode_suffix = match mode {
        PdfOptimizationMode::LosslessOrganization => "organized",
        PdfOptimizationMode::CompatibleImageOptimization => "optimized",
    };
    let reserved = reserved_destinations
        .iter()
        .map(|path| normalized_destination_key(path))
        .collect::<HashSet<_>>();
    for index in 0..10_000_u32 {
        let conflict_suffix = if index == 0 {
            String::new()
        } else {
            format!(" ({index})")
        };
        let destination = directory.join(format!("{stem}.{mode_suffix}{conflict_suffix}.pdf"));
        let key = normalized_destination_key(&destination);
        if !destination.exists() && !reserved.contains(&key) {
            return Ok(PdfOptimizationDestinationPlan { destination });
        }
    }
    Err("PDF_DESTINATION_RENAME_LIMIT_REACHED".to_string())
}

#[tauri::command]
pub fn plan_pdf_optimization_destination(
    source: String,
    output_directory: Option<String>,
    mode: PdfOptimizationMode,
    reserved_destinations: Vec<String>,
) -> Result<PdfOptimizationDestinationPlan, String> {
    let reserved = reserved_destinations
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    plan_pdf_destination(
        Path::new(&source),
        output_directory.as_deref().map(Path::new),
        mode,
        &reserved,
    )
}

async fn run_pdf_compression<F>(
    resource_root: PathBuf,
    task_id: String,
    request: PdfCompressionExecutionRequest,
    observe: F,
) -> Result<PublishedPdfOutput, String>
where
    F: FnMut(PdfPublicationStage),
{
    let cancelled = crate::commands::compression::register_task_cancellation(&task_id)?;
    let _task_guard = crate::commands::compression::TaskCancellationGuard::new(&task_id);
    let validation_root = resource_root.clone();
    tauri::async_runtime::spawn_blocking(move || validate_pdf_engine(&validation_root))
        .await
        .map_err(|error| format!("PDF_ENGINE_PREFLIGHT_JOIN_FAILED: {error}"))?
        .map_err(|error| error.to_string())?;
    if cancelled.load(Ordering::Acquire) {
        return Err("PDF_PUBLISH_CANCELLED".to_string());
    }
    let qpdf = resource_root.join("pdf-engine").join("qpdf.exe");
    let transform_request = PdfTransformRequest {
        source: request.source,
        destination: request.destination,
        password: None,
        mode: request.mode,
        confirmed_lossy_image_changes: request.confirmed_lossy_image_changes,
    };
    execute_pdf_publication_transaction_observed(
        &qpdf,
        &transform_request,
        request.preserve_mark_of_web,
        request.allow_larger_output,
        &cancelled,
        observe,
    )
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn compress_pdf_file(
    app: tauri::AppHandle,
    window: Window,
    task_id: String,
    request: PdfCompressionExecutionRequest,
) -> Result<PublishedPdfOutput, String> {
    let app_resource_dir = app
        .path_resolver()
        .resource_dir()
        .ok_or_else(|| "PDF_ENGINE_RESOURCE_DIRECTORY_UNAVAILABLE".to_string())?;
    let resource_root = bundled_pdf_resource_root(&app_resource_dir);
    let event_task_id = task_id.clone();
    let source = request.source.to_string_lossy().into_owned();
    run_pdf_compression(resource_root, task_id, request, move |stage| {
        let stage_name = stage.event_name().to_string();
        let _ = window.emit(
            "task-log",
            crate::models::compression::TaskLog {
                task_id: event_task_id.clone(),
                timestamp: chrono::Utc::now(),
                message: stage.log_message().to_string(),
                severity: crate::models::compression::TaskLogSeverity::Info,
            },
        );
        let _ = window.emit(
            "task-progress",
            PdfTaskProgress {
                task_id: event_task_id.clone(),
                stage: stage_name,
                progress: 0.0,
                current_file: source.clone(),
                processed_bytes: 0,
                total_bytes: 0,
                output_bytes: 0,
                output_bytes_estimated: true,
            },
        );
    })
    .await
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destination_planner_uses_mode_suffix_and_avoids_real_and_reserved_conflicts() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("中文 sample.pdf");
        std::fs::write(&source, b"%PDF-1.4\n%%EOF\n").unwrap();
        let first = directory.path().join("中文 sample.organized.pdf");
        std::fs::write(&first, b"existing").unwrap();
        let reserved = vec![directory.path().join("中文 sample.organized (1).pdf")];
        let planned = plan_pdf_destination(
            &source,
            None,
            PdfOptimizationMode::LosslessOrganization,
            &reserved,
        )
        .unwrap();
        assert_eq!(
            planned.destination,
            directory.path().join("中文 sample.organized (2).pdf")
        );
        let optimized = plan_pdf_destination(
            &source,
            None,
            PdfOptimizationMode::CompatibleImageOptimization,
            &[],
        )
        .unwrap();
        assert_eq!(
            optimized.destination,
            directory.path().join("中文 sample.optimized.pdf")
        );
    }

    #[tokio::test]
    #[ignore = "run through npm run test:pdf-d04-command:real after generating real PDF fixtures"]
    async fn real_product_command_revalidates_and_publishes_with_truthful_stages() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let fixture_root = root.join("test-results/media-fixture-audit/fixtures/pdfs");
        let output = root.join("test-results/d04-pdf-command/outputs");
        std::fs::create_dir_all(&output).unwrap();
        let fixtures = [
            ("text-vector.pdf", 1_u32),
            ("scanned-image.pdf", 1),
            ("mixed-content.pdf", 1),
            ("transparency.pdf", 1),
            ("chinese-font.pdf", 1),
            ("large-pages.pdf", 300),
            ("large-image.pdf", 1),
            ("form.pdf", 1),
            ("annotation.pdf", 1),
            ("outline.pdf", 2),
            ("attachment.pdf", 1),
        ];
        let modes = [
            PdfOptimizationMode::LosslessOrganization,
            PdfOptimizationMode::CompatibleImageOptimization,
        ];
        let mut reports = Vec::new();
        for (file, expected_pages) in fixtures {
            for mode in modes {
                let source = fixture_root.join(file);
                let source_before = std::fs::read(&source).expect("real PDF fixture");
                let mode_name = match mode {
                    PdfOptimizationMode::LosslessOrganization => "lossless-organization",
                    PdfOptimizationMode::CompatibleImageOptimization => {
                        "compatible-image-optimization"
                    }
                };
                let destination = output.join(format!("{file}.{mode_name}.pdf"));
                if destination.exists() {
                    std::fs::remove_file(&destination).unwrap();
                }
                let stages = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
                let observed = stages.clone();
                let published = run_pdf_compression(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources"),
                    format!("pdf-command-{}", uuid::Uuid::new_v4()),
                    PdfCompressionExecutionRequest {
                        source: source.clone(),
                        destination: destination.clone(),
                        mode,
                        confirmed_lossy_image_changes: mode
                            == PdfOptimizationMode::CompatibleImageOptimization,
                        preserve_mark_of_web: true,
                        allow_larger_output: true,
                    },
                    move |stage| observed.lock().unwrap().push(stage),
                )
                .await
                .unwrap();
                let stages = stages.lock().unwrap().clone();
                assert_eq!(
                    stages,
                    vec![
                        PdfPublicationStage::Transforming,
                        PdfPublicationStage::Validating,
                        PdfPublicationStage::Publishing,
                    ]
                );
                assert_eq!(published.path, destination);
                assert_eq!(published.verified.output_facts.page_count, expected_pages);
                assert_eq!(
                    published.verified.source_facts,
                    published.verified.output_facts
                );
                assert_eq!(
                    published.output_bytes,
                    std::fs::metadata(&destination).unwrap().len()
                );
                assert_eq!(std::fs::read(&source).unwrap(), source_before);
                reports.push(serde_json::json!({
                    "file": file,
                    "mode": mode_name,
                    "stages": stages.iter().map(|stage| stage.event_name()).collect::<Vec<_>>(),
                    "finalOutputExists": destination.exists(),
                    "inputBytes": published.input_bytes,
                    "outputBytes": published.output_bytes,
                    "pageCount": published.verified.output_facts.page_count,
                    "structuralFactsEqual": published.verified.source_facts == published.verified.output_facts,
                    "sourceBytesUnchanged": std::fs::read(&source).unwrap() == source_before,
                    "markOfTheWeb": published.mark_of_the_web,
                }));
            }
        }

        let mut blocked = Vec::new();
        for (file, expected_code) in [
            ("signed.pdf", "PDF_TRANSFORM_SIGNED_DOCUMENT_BLOCKED"),
            ("encrypted.pdf", "PDF_TRANSFORM_ANALYSIS_INCOMPLETE"),
        ] {
            let source = fixture_root.join(file);
            let destination = output.join(format!("blocked-{file}"));
            if destination.exists() {
                std::fs::remove_file(&destination).unwrap();
            }
            let error = run_pdf_compression(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources"),
                format!("pdf-command-blocked-{}", uuid::Uuid::new_v4()),
                PdfCompressionExecutionRequest {
                    source,
                    destination: destination.clone(),
                    mode: PdfOptimizationMode::LosslessOrganization,
                    confirmed_lossy_image_changes: false,
                    preserve_mark_of_web: true,
                    allow_larger_output: true,
                },
                |_| {},
            )
            .await
            .unwrap_err();
            assert!(
                error.starts_with(expected_code),
                "unexpected {file} error: {error}"
            );
            assert!(!destination.exists());
            blocked.push(serde_json::json!({
                "file": file,
                "expectedCode": expected_code,
                "actualError": error,
                "outputAbsent": !destination.exists(),
            }));
        }
        println!(
            "D04_PDF_COMMAND_RESULT={}",
            serde_json::json!({
                "reports": reports,
                "blocked": blocked,
            })
        );
    }
}
