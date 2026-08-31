use crate::services::output_publish_transaction::{
    cleanup_staged_output_family, staged_output_path, PublishError,
};
use crate::services::pdf_analysis::{analyze_pdf_input, PdfAnalysisError, PdfInputAnalysisReport};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

const TRANSFORM_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const POLL_INTERVAL: Duration = Duration::from_millis(50);
const MAX_ERROR_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PdfOptimizationMode {
    LosslessOrganization,
    CompatibleImageOptimization,
}

#[derive(Debug, Clone)]
pub struct PdfTransformRequest {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub password: Option<String>,
    pub mode: PdfOptimizationMode,
    pub confirmed_lossy_image_changes: bool,
}

#[derive(Debug, Error)]
pub enum PdfTransformError {
    #[error(transparent)]
    Analysis(#[from] PdfAnalysisError),
    #[error("PDF_TRANSFORM_SOURCE_EQUALS_DESTINATION")]
    SourceEqualsDestination,
    #[error("PDF_TRANSFORM_DESTINATION_EXISTS: {0}")]
    DestinationExists(PathBuf),
    #[error("PDF_TRANSFORM_OUTPUT_DIRECTORY_INVALID: {0}")]
    InvalidOutputDirectory(PathBuf),
    #[error("PDF_TRANSFORM_DESTINATION_MUST_BE_PDF: {0}")]
    InvalidDestinationExtension(PathBuf),
    #[error("PDF_TRANSFORM_ANALYSIS_INCOMPLETE")]
    AnalysisIncomplete,
    #[error("PDF_TRANSFORM_SIGNED_DOCUMENT_BLOCKED")]
    SignedDocumentBlocked,
    #[error("PDF_TRANSFORM_ENCRYPTED_DOCUMENT_BLOCKED: preserving encryption without exposing credentials is not implemented")]
    EncryptedDocumentBlocked,
    #[error("PDF_TRANSFORM_LOSSY_CONFIRMATION_REQUIRED")]
    LossyConfirmationRequired,
    #[error("PDF_TRANSFORM_RESOURCE_PREFLIGHT_BLOCKED: {0}")]
    ResourcePreflightBlocked(String),
    #[error("PDF_TRANSFORM_CANCELLED")]
    Cancelled,
    #[error("PDF_TRANSFORM_TIMEOUT: exceeded 600 seconds")]
    Timeout,
    #[error("PDF_TRANSFORM_LAUNCH_FAILED: {0}")]
    LaunchFailed(String),
    #[error("PDF_TRANSFORM_PROCESS_FAILED: {0}")]
    ProcessFailed(String),
    #[error("PDF_TRANSFORM_STAGED_OUTPUT_INVALID")]
    InvalidStagedOutput,
    #[error("PDF_TRANSFORM_SOURCE_CHANGED_DURING_EXECUTION")]
    SourceChanged,
    #[error("PDF_TRANSFORM_PREFLIGHT_FAILED: {0}")]
    PreflightFailed(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("PDF_TRANSFORM_STAGING_PATH_FAILED: {0}")]
    StagingPath(String),
}

impl From<PublishError> for PdfTransformError {
    fn from(error: PublishError) -> Self {
        Self::StagingPath(error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceIdentity {
    bytes: u64,
    sha256: String,
}

#[derive(Debug)]
pub struct PdfStagedOutput {
    path: PathBuf,
    destination: PathBuf,
    source_report: PdfInputAnalysisReport,
    encoded_bytes: u64,
    mode: PdfOptimizationMode,
}

impl PdfStagedOutput {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn destination(&self) -> &Path {
        &self.destination
    }

    pub fn source_report(&self) -> &PdfInputAnalysisReport {
        &self.source_report
    }

    pub fn encoded_bytes(&self) -> u64 {
        self.encoded_bytes
    }

    pub fn mode(&self) -> PdfOptimizationMode {
        self.mode
    }

    #[cfg(test)]
    pub(crate) fn set_encoded_bytes_for_test(&mut self, encoded_bytes: u64) {
        self.encoded_bytes = encoded_bytes;
    }
}

impl Drop for PdfStagedOutput {
    fn drop(&mut self) {
        cleanup_staged_output_family(&self.path);
    }
}

fn paths_match(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        left.to_string_lossy()
            .replace('/', "\\")
            .eq_ignore_ascii_case(&right.to_string_lossy().replace('/', "\\"))
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

fn source_identity(path: &Path) -> Result<SourceIdentity, PdfTransformError> {
    let metadata = std::fs::metadata(path)?;
    let mut reader = BufReader::new(File::open(path)?);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(SourceIdentity {
        bytes: metadata.len(),
        sha256: hex::encode(hasher.finalize()),
    })
}

fn transform_arguments(mode: PdfOptimizationMode, source: &Path, staged: &Path) -> Vec<OsString> {
    let mut arguments = vec![
        OsString::from("--object-streams=generate"),
        OsString::from("--compress-streams=y"),
        OsString::from("--decode-level=generalized"),
        OsString::from("--recompress-flate"),
        OsString::from("--compression-level=9"),
        OsString::from("--newline-before-endstream"),
    ];
    if mode == PdfOptimizationMode::CompatibleImageOptimization {
        arguments.extend([
            OsString::from("--optimize-images"),
            OsString::from("--jpeg-quality=85"),
            OsString::from("--oi-min-width=128"),
            OsString::from("--oi-min-height=128"),
            OsString::from("--oi-min-area=16384"),
        ]);
    }
    arguments.push(source.as_os_str().to_owned());
    arguments.push(staged.as_os_str().to_owned());
    arguments
}

fn validate_request(request: &PdfTransformRequest) -> Result<(), PdfTransformError> {
    if paths_match(&request.source, &request.destination) {
        return Err(PdfTransformError::SourceEqualsDestination);
    }
    if request.destination.exists() {
        return Err(PdfTransformError::DestinationExists(
            request.destination.clone(),
        ));
    }
    if request
        .destination
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case("pdf"))
    {
        return Err(PdfTransformError::InvalidDestinationExtension(
            request.destination.clone(),
        ));
    }
    let parent = request
        .destination
        .parent()
        .unwrap_or_else(|| Path::new("."));
    if !parent.is_dir() {
        return Err(PdfTransformError::InvalidOutputDirectory(
            parent.to_path_buf(),
        ));
    }
    if request.mode == PdfOptimizationMode::CompatibleImageOptimization
        && !request.confirmed_lossy_image_changes
    {
        return Err(PdfTransformError::LossyConfirmationRequired);
    }
    Ok(())
}

fn validate_report(report: &PdfInputAnalysisReport) -> Result<(), PdfTransformError> {
    if !report.analysis_complete {
        return Err(PdfTransformError::AnalysisIncomplete);
    }
    if report.has_digital_signature == Some(true) {
        return Err(PdfTransformError::SignedDocumentBlocked);
    }
    if report.encrypted {
        return Err(PdfTransformError::EncryptedDocumentBlocked);
    }
    Ok(())
}

async fn run_qpdf(
    qpdf: &Path,
    arguments: &[OsString],
    cancelled: &AtomicBool,
) -> Result<(), PdfTransformError> {
    if cancelled.load(Ordering::Acquire) {
        return Err(PdfTransformError::Cancelled);
    }
    let mut child = Command::new(qpdf)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| PdfTransformError::LaunchFailed(error.to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| PdfTransformError::LaunchFailed("qpdf stderr unavailable".into()))?;
    let stderr_reader = tokio::spawn(async move {
        let mut bytes = Vec::new();
        stderr
            .take(MAX_ERROR_BYTES + 1)
            .read_to_end(&mut bytes)
            .await
            .map(|_| bytes)
    });
    let started = Instant::now();
    let status = loop {
        if cancelled.load(Ordering::Acquire) {
            let _ = child.kill().await;
            let _ = child.wait().await;
            let _ = stderr_reader.await;
            return Err(PdfTransformError::Cancelled);
        }
        if started.elapsed() >= TRANSFORM_TIMEOUT {
            let _ = child.kill().await;
            let _ = child.wait().await;
            let _ = stderr_reader.await;
            return Err(PdfTransformError::Timeout);
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => tokio::time::sleep(POLL_INTERVAL).await,
            Err(error) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                let _ = stderr_reader.await;
                return Err(PdfTransformError::LaunchFailed(error.to_string()));
            }
        }
    };
    let stderr = stderr_reader
        .await
        .map_err(|error| PdfTransformError::LaunchFailed(error.to_string()))??;
    if !status.success() {
        let detail: String = String::from_utf8_lossy(&stderr)
            .trim()
            .chars()
            .take(2_048)
            .collect();
        return Err(PdfTransformError::ProcessFailed(detail));
    }
    Ok(())
}

pub async fn transform_pdf_to_staging(
    qpdf: &Path,
    request: &PdfTransformRequest,
    cancelled: &AtomicBool,
) -> Result<PdfStagedOutput, PdfTransformError> {
    validate_request(request)?;
    let source_before = source_identity(&request.source)?;
    let report = analyze_pdf_input(qpdf, &request.source, request.password.as_deref()).await?;
    validate_report(&report)?;
    if cancelled.load(Ordering::Acquire) {
        return Err(PdfTransformError::Cancelled);
    }

    let estimate = report.input_bytes.saturating_mul(2);
    let preflight = crate::services::storage_preflight::preflight_operation_resources(
        "compression",
        &request.destination.to_string_lossy(),
        &[request.source.to_string_lossy().into_owned()],
        None,
        Some(estimate),
        false,
    )
    .await
    .map_err(|error| PdfTransformError::PreflightFailed(error.to_string()))?;
    if !preflight.can_start {
        return Err(PdfTransformError::ResourcePreflightBlocked(
            preflight.summary,
        ));
    }

    let staged = staged_output_path(&request.destination, "pdf-transform")?;
    let mut guard = PdfStagedOutput {
        path: staged.clone(),
        destination: request.destination.clone(),
        source_report: report,
        encoded_bytes: 0,
        mode: request.mode,
    };
    let arguments = transform_arguments(request.mode, &request.source, &staged);
    run_qpdf(qpdf, &arguments, cancelled).await?;
    let metadata =
        std::fs::symlink_metadata(&staged).map_err(|_| PdfTransformError::InvalidStagedOutput)?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() || metadata.len() == 0 {
        return Err(PdfTransformError::InvalidStagedOutput);
    }
    if source_identity(&request.source)? != source_before {
        return Err(PdfTransformError::SourceChanged);
    }
    guard.encoded_bytes = metadata.len();
    Ok(guard)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qpdf() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/pdf-engine/qpdf.exe")
    }

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../test-results/media-fixture-audit/fixtures/pdfs")
            .join(name)
    }

    fn request(
        source: PathBuf,
        destination: PathBuf,
        mode: PdfOptimizationMode,
    ) -> PdfTransformRequest {
        PdfTransformRequest {
            source,
            destination,
            password: None,
            mode,
            confirmed_lossy_image_changes: mode == PdfOptimizationMode::CompatibleImageOptimization,
        }
    }

    #[test]
    fn arguments_are_fixed_and_image_options_are_mode_scoped() {
        let source = Path::new("input.pdf");
        let staged = Path::new("staged.pdf");
        let lossless =
            transform_arguments(PdfOptimizationMode::LosslessOrganization, source, staged);
        let image = transform_arguments(
            PdfOptimizationMode::CompatibleImageOptimization,
            source,
            staged,
        );
        let joined = |values: &[OsString]| {
            values
                .iter()
                .map(|value| value.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ")
        };
        let lossless = joined(&lossless);
        let image = joined(&image);
        assert!(lossless.contains("--object-streams=generate"));
        assert!(!lossless.contains("--optimize-images"));
        assert!(image.contains("--optimize-images --jpeg-quality=85"));
        assert!(!image.contains("--password="));
        assert!(!image.contains("fixture-user"));
    }

    #[tokio::test]
    async fn refuses_lossy_without_confirmation_before_qpdf_launch() {
        let directory = tempfile::tempdir().unwrap();
        let mut candidate = request(
            directory.path().join("missing.pdf"),
            directory.path().join("output.pdf"),
            PdfOptimizationMode::CompatibleImageOptimization,
        );
        candidate.confirmed_lossy_image_changes = false;
        let error = transform_pdf_to_staging(
            Path::new("missing-qpdf"),
            &candidate,
            &AtomicBool::new(false),
        )
        .await
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "PDF_TRANSFORM_LOSSY_CONFIRMATION_REQUIRED"
        );
    }

    #[tokio::test]
    #[cfg(windows)]
    #[ignore = "run through npm run test:pdf-d03-staging:real after generating real PDF fixtures"]
    async fn transforms_real_pdf_modes_to_owned_staging_and_blocks_risks() {
        let directory = tempfile::tempdir().unwrap();
        let cancelled = AtomicBool::new(false);
        let cases = [
            ("text-vector.pdf", PdfOptimizationMode::LosslessOrganization),
            (
                "mixed-content.pdf",
                PdfOptimizationMode::CompatibleImageOptimization,
            ),
        ];
        let mut reports = Vec::new();
        for (name, mode) in cases {
            let source = fixture(name);
            let source_before = source_identity(&source).unwrap();
            let final_output = directory.path().join(format!("{name}.result.pdf"));
            let staged = transform_pdf_to_staging(
                &qpdf(),
                &request(source.clone(), final_output.clone(), mode),
                &cancelled,
            )
            .await
            .unwrap_or_else(|error| panic!("{name}: {error}"));
            let staged_path = staged.path().to_path_buf();
            assert!(staged_path.exists());
            assert_eq!(staged.source_report().page_count, Some(1));
            assert!(staged.encoded_bytes() > 0);
            assert_eq!(staged.mode(), mode);
            assert!(!final_output.exists());
            let output_report = analyze_pdf_input(&qpdf(), staged.path(), None)
                .await
                .unwrap();
            reports.push(serde_json::json!({
                "file": name,
                "mode": mode,
                "inputBytes": source_before.bytes,
                "stagedBytes": staged.encoded_bytes(),
                "inputPages": staged.source_report().page_count,
                "stagedPages": output_report.page_count,
                "sourceChanged": source_identity(&source).unwrap() != source_before,
                "finalOutputExists": final_output.exists(),
                "stagingExistsBeforeDrop": staged_path.exists()
            }));
            drop(staged);
            assert!(!staged_path.exists());
        }

        let signed_request = request(
            fixture("signed.pdf"),
            directory.path().join("signed.result.pdf"),
            PdfOptimizationMode::LosslessOrganization,
        );
        let signed_error = transform_pdf_to_staging(&qpdf(), &signed_request, &cancelled)
            .await
            .unwrap_err();
        assert_eq!(
            signed_error.to_string(),
            "PDF_TRANSFORM_SIGNED_DOCUMENT_BLOCKED"
        );

        let mut encrypted_request = request(
            fixture("encrypted.pdf"),
            directory.path().join("encrypted.result.pdf"),
            PdfOptimizationMode::LosslessOrganization,
        );
        encrypted_request.password = Some("fixture-user".into());
        let encrypted_error = transform_pdf_to_staging(&qpdf(), &encrypted_request, &cancelled)
            .await
            .unwrap_err();
        assert!(encrypted_error
            .to_string()
            .starts_with("PDF_TRANSFORM_ENCRYPTED_DOCUMENT_BLOCKED:"));

        let cancelled_request = request(
            fixture("text-vector.pdf"),
            directory.path().join("cancelled.result.pdf"),
            PdfOptimizationMode::LosslessOrganization,
        );
        let cancelled_error =
            transform_pdf_to_staging(&qpdf(), &cancelled_request, &AtomicBool::new(true))
                .await
                .unwrap_err();
        assert_eq!(cancelled_error.to_string(), "PDF_TRANSFORM_CANCELLED");

        println!(
            "D03_PDF_STAGING_RESULT={}",
            serde_json::json!({
                "reports": reports,
                "signedError": signed_error.to_string(),
                "encryptedError": encrypted_error.to_string(),
                "cancelledError": cancelled_error.to_string()
            })
        );
    }
}
