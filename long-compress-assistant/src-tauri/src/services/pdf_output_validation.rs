use crate::services::pdf_transform::PdfStagedOutput;
use crate::utils::process;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::io::AsyncReadExt;

const VALIDATION_TIMEOUT: Duration = Duration::from_secs(2 * 60);
const POLL_INTERVAL: Duration = Duration::from_millis(50);
const MAX_JSON_BYTES: u64 = 32 * 1024 * 1024;
const MAX_STDERR_BYTES: u64 = 1024 * 1024;
const MAX_ATTACHMENT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_ATTACHMENT_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PdfOutputValidationError {
    #[error("PDF_OUTPUT_NOT_REGULAR_FILE")]
    NotRegularFile,
    #[error("PDF_OUTPUT_EMPTY")]
    Empty,
    #[error("PDF_OUTPUT_SIZE_CHANGED: expected={0} actual={1}")]
    SizeChanged(u64, u64),
    #[error("PDF_OUTPUT_CHANGED_DURING_VALIDATION")]
    ChangedDuringValidation,
    #[error("PDF_OUTPUT_TARGET_APPEARED")]
    TargetAppeared,
    #[error("PDF_OUTPUT_LARGER_THAN_SOURCE: source={0} output={1}")]
    LargerThanSource(u64, u64),
    #[error("PDF_OUTPUT_CANCELLED")]
    Cancelled,
    #[error("PDF_OUTPUT_VALIDATION_TIMEOUT")]
    Timeout,
    #[error("PDF_OUTPUT_LAUNCH_FAILED: {0}")]
    LaunchFailed(String),
    #[error("PDF_OUTPUT_QPDF_CHECK_FAILED: {0}")]
    QpdfCheckFailed(String),
    #[error("PDF_OUTPUT_JSON_FAILED: {0}")]
    JsonFailed(String),
    #[error("PDF_OUTPUT_JSON_TOO_LARGE")]
    JsonTooLarge,
    #[error("PDF_OUTPUT_INVALID_FACTS: {0}")]
    InvalidFacts(String),
    #[error("PDF_OUTPUT_PAGE_COUNT_MISMATCH: source={0} output={1}")]
    PageCountMismatch(u32, u32),
    #[error("PDF_OUTPUT_ENCRYPTION_STATE_MISMATCH")]
    EncryptionMismatch,
    #[error("PDF_OUTPUT_PAGE_GEOMETRY_MISMATCH")]
    PageGeometryMismatch,
    #[error("PDF_OUTPUT_FORM_FIELDS_MISMATCH")]
    FormFieldsMismatch,
    #[error("PDF_OUTPUT_ANNOTATIONS_MISMATCH")]
    AnnotationsMismatch,
    #[error("PDF_OUTPUT_OUTLINES_MISMATCH")]
    OutlinesMismatch,
    #[error("PDF_OUTPUT_ATTACHMENTS_MISMATCH")]
    AttachmentsMismatch,
    #[error("PDF_OUTPUT_ATTACHMENT_TOO_LARGE: {0}")]
    AttachmentTooLarge(String),
    #[error("PDF_OUTPUT_ATTACHMENTS_TOTAL_TOO_LARGE")]
    AttachmentsTotalTooLarge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct PdfFormFieldFact {
    pub name: String,
    pub field_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct PdfAnnotationFact {
    pub page: u32,
    pub subtype: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct PdfOutlineFact {
    pub title: String,
    pub page: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct PdfAttachmentFact {
    pub key: String,
    pub name: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfStructuralFacts {
    pub page_count: u32,
    pub encrypted: bool,
    pub page_media_boxes: Vec<Vec<String>>,
    pub form_fields: Vec<PdfFormFieldFact>,
    pub annotations: Vec<PdfAnnotationFact>,
    pub outlines: Vec<PdfOutlineFact>,
    pub attachments: Vec<PdfAttachmentFact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedPdfOutput {
    pub output_bytes: u64,
    pub output_sha256: String,
    pub source_facts: PdfStructuralFacts,
    pub output_facts: PdfStructuralFacts,
}

struct ProcessOutput {
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn bounded_detail(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim()
        .chars()
        .take(2_048)
        .collect()
}

fn file_sha256(path: &Path) -> Result<String, PdfOutputValidationError> {
    let file = File::open(path)
        .map_err(|error| PdfOutputValidationError::InvalidFacts(error.to_string()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| PdfOutputValidationError::InvalidFacts(error.to_string()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

async fn run_qpdf_capture(
    qpdf: &Path,
    arguments: &[OsString],
    cancelled: &AtomicBool,
    maximum_stdout: u64,
) -> Result<ProcessOutput, PdfOutputValidationError> {
    if cancelled.load(Ordering::Acquire) {
        return Err(PdfOutputValidationError::Cancelled);
    }
    let mut child = process::async_command(qpdf)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| PdfOutputValidationError::LaunchFailed(error.to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| PdfOutputValidationError::LaunchFailed("qpdf stdout unavailable".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| PdfOutputValidationError::LaunchFailed("qpdf stderr unavailable".into()))?;
    let stdout_reader = tokio::spawn(async move {
        let mut bytes = Vec::new();
        stdout
            .take(maximum_stdout.saturating_add(1))
            .read_to_end(&mut bytes)
            .await
            .map(|_| bytes)
    });
    let stderr_reader = tokio::spawn(async move {
        let mut bytes = Vec::new();
        stderr
            .take(MAX_STDERR_BYTES + 1)
            .read_to_end(&mut bytes)
            .await
            .map(|_| bytes)
    });
    let started = Instant::now();
    let status = loop {
        if cancelled.load(Ordering::Acquire) {
            let _ = child.kill().await;
            let _ = child.wait().await;
            let _ = stdout_reader.await;
            let _ = stderr_reader.await;
            return Err(PdfOutputValidationError::Cancelled);
        }
        if started.elapsed() >= VALIDATION_TIMEOUT {
            let _ = child.kill().await;
            let _ = child.wait().await;
            let _ = stdout_reader.await;
            let _ = stderr_reader.await;
            return Err(PdfOutputValidationError::Timeout);
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => tokio::time::sleep(POLL_INTERVAL).await,
            Err(error) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                let _ = stdout_reader.await;
                let _ = stderr_reader.await;
                return Err(PdfOutputValidationError::LaunchFailed(error.to_string()));
            }
        }
    };
    let stdout = stdout_reader
        .await
        .map_err(|error| PdfOutputValidationError::LaunchFailed(error.to_string()))?
        .map_err(|error| PdfOutputValidationError::LaunchFailed(error.to_string()))?;
    let stderr = stderr_reader
        .await
        .map_err(|error| PdfOutputValidationError::LaunchFailed(error.to_string()))?
        .map_err(|error| PdfOutputValidationError::LaunchFailed(error.to_string()))?;
    Ok(ProcessOutput {
        code: status.code(),
        stdout,
        stderr,
    })
}

fn detailed_json_arguments(path: &Path) -> Vec<OsString> {
    [
        "--json=2",
        "--json-stream-data=none",
        "--json-key=pages",
        "--json-key=acroform",
        "--json-key=attachments",
        "--json-key=outlines",
        "--json-key=qpdf",
    ]
    .into_iter()
    .map(OsString::from)
    .chain(std::iter::once(path.as_os_str().to_owned()))
    .collect()
}

fn object_map(root: &Value) -> Result<&Map<String, Value>, PdfOutputValidationError> {
    root.get("qpdf")
        .and_then(Value::as_array)
        .and_then(|items| items.get(1))
        .and_then(Value::as_object)
        .ok_or_else(|| PdfOutputValidationError::InvalidFacts("qpdf object map missing".into()))
}

fn object_value<'a>(objects: &'a Map<String, Value>, reference: &str) -> Option<&'a Value> {
    objects.get(&format!("obj:{reference}"))?.get("value")
}

fn canonical_number(value: &Value) -> Option<String> {
    if let Some(number) = value.as_i64() {
        Some(number.to_string())
    } else if let Some(number) = value.as_u64() {
        Some(number.to_string())
    } else {
        value.as_f64().map(|number| format!("{number:.6}"))
    }
}

fn page_facts(
    root: &Value,
    objects: &Map<String, Value>,
) -> Result<(Vec<Vec<String>>, Vec<PdfAnnotationFact>), PdfOutputValidationError> {
    let pages = root
        .get("pages")
        .and_then(Value::as_array)
        .ok_or_else(|| PdfOutputValidationError::InvalidFacts("pages missing".into()))?;
    if pages.is_empty() {
        return Err(PdfOutputValidationError::InvalidFacts("pages empty".into()));
    }
    let mut media_boxes = Vec::with_capacity(pages.len());
    let mut annotations = BTreeSet::new();
    for (index, page) in pages.iter().enumerate() {
        let reference = page
            .get("object")
            .and_then(Value::as_str)
            .ok_or_else(|| PdfOutputValidationError::InvalidFacts("page object missing".into()))?;
        let value = object_value(objects, reference).ok_or_else(|| {
            PdfOutputValidationError::InvalidFacts("page dictionary missing".into())
        })?;
        let media_box = value
            .get("/MediaBox")
            .and_then(Value::as_array)
            .ok_or_else(|| PdfOutputValidationError::InvalidFacts("page media box missing".into()))?
            .iter()
            .map(canonical_number)
            .collect::<Option<Vec<_>>>()
            .filter(|values| values.len() == 4)
            .ok_or_else(|| {
                PdfOutputValidationError::InvalidFacts("page media box invalid".into())
            })?;
        media_boxes.push(media_box);
        for annotation_reference in value
            .get("/Annots")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            let annotation = object_value(objects, annotation_reference).ok_or_else(|| {
                PdfOutputValidationError::InvalidFacts("annotation object missing".into())
            })?;
            let subtype = annotation
                .get("/Subtype")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    PdfOutputValidationError::InvalidFacts("annotation subtype missing".into())
                })?;
            annotations.insert(PdfAnnotationFact {
                page: u32::try_from(index + 1).unwrap_or(u32::MAX),
                subtype: subtype.to_string(),
            });
        }
    }
    Ok((media_boxes, annotations.into_iter().collect()))
}

fn form_facts(root: &Value) -> Vec<PdfFormFieldFact> {
    let mut fields: Vec<_> = root
        .pointer("/acroform/fields")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|field| {
            let name = field.get("fullname")?.as_str()?;
            let field_type = field.get("fieldtype")?.as_str()?;
            Some(PdfFormFieldFact {
                name: name.to_string(),
                field_type: field_type.to_string(),
            })
        })
        .collect();
    fields.sort();
    fields.dedup();
    fields
}

fn collect_outlines(items: &[Value], output: &mut Vec<PdfOutlineFact>) {
    for item in items {
        if let Some(title) = item.get("title").and_then(Value::as_str) {
            output.push(PdfOutlineFact {
                title: title.to_string(),
                page: item
                    .get("destpageposfrom1")
                    .and_then(Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok()),
            });
        }
        if let Some(kids) = item.get("kids").and_then(Value::as_array) {
            collect_outlines(kids, output);
        }
    }
}

async fn attachment_facts(
    qpdf: &Path,
    path: &Path,
    root: &Value,
    cancelled: &AtomicBool,
) -> Result<Vec<PdfAttachmentFact>, PdfOutputValidationError> {
    let mut facts = Vec::new();
    let mut total = 0_u64;
    for (key, attachment) in root
        .get("attachments")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|items| items.iter())
    {
        let name = attachment
            .get("preferredname")
            .and_then(Value::as_str)
            .unwrap_or(key);
        let arguments = [
            OsString::from(format!("--show-attachment={key}")),
            path.as_os_str().to_owned(),
        ];
        let output = run_qpdf_capture(qpdf, &arguments, cancelled, MAX_ATTACHMENT_BYTES).await?;
        if output.stdout.len() as u64 > MAX_ATTACHMENT_BYTES {
            return Err(PdfOutputValidationError::AttachmentTooLarge(
                name.to_string(),
            ));
        }
        if output.code != Some(0) {
            return Err(PdfOutputValidationError::JsonFailed(bounded_detail(
                &output.stderr,
            )));
        }
        total = total.saturating_add(output.stdout.len() as u64);
        if total > MAX_TOTAL_ATTACHMENT_BYTES {
            return Err(PdfOutputValidationError::AttachmentsTotalTooLarge);
        }
        facts.push(PdfAttachmentFact {
            key: key.to_string(),
            name: name.to_string(),
            bytes: output.stdout.len() as u64,
            sha256: hex::encode(Sha256::digest(&output.stdout)),
        });
    }
    facts.sort();
    Ok(facts)
}

async fn is_encrypted(
    qpdf: &Path,
    path: &Path,
    cancelled: &AtomicBool,
) -> Result<bool, PdfOutputValidationError> {
    let arguments = [
        OsString::from("--is-encrypted"),
        path.as_os_str().to_owned(),
    ];
    let output = run_qpdf_capture(qpdf, &arguments, cancelled, 1024).await?;
    match output.code {
        Some(0) => Ok(true),
        Some(2) => Ok(false),
        _ => Err(PdfOutputValidationError::JsonFailed(bounded_detail(
            &output.stderr,
        ))),
    }
}

async fn structural_facts(
    qpdf: &Path,
    path: &Path,
    cancelled: &AtomicBool,
) -> Result<PdfStructuralFacts, PdfOutputValidationError> {
    let output = run_qpdf_capture(
        qpdf,
        &detailed_json_arguments(path),
        cancelled,
        MAX_JSON_BYTES,
    )
    .await?;
    if output.stdout.len() as u64 > MAX_JSON_BYTES {
        return Err(PdfOutputValidationError::JsonTooLarge);
    }
    if !matches!(output.code, Some(0 | 3)) {
        return Err(PdfOutputValidationError::JsonFailed(bounded_detail(
            &output.stderr,
        )));
    }
    let root: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| PdfOutputValidationError::JsonFailed(error.to_string()))?;
    let objects = object_map(&root)?;
    let (page_media_boxes, annotations) = page_facts(&root, objects)?;
    let mut outlines = Vec::new();
    collect_outlines(
        root.get("outlines")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default(),
        &mut outlines,
    );
    outlines.sort();
    let attachments = attachment_facts(qpdf, path, &root, cancelled).await?;
    Ok(PdfStructuralFacts {
        page_count: u32::try_from(page_media_boxes.len()).unwrap_or(u32::MAX),
        encrypted: is_encrypted(qpdf, path, cancelled).await?,
        page_media_boxes,
        form_fields: form_facts(&root),
        annotations,
        outlines,
        attachments,
    })
}

fn compare_facts(
    source: &PdfStructuralFacts,
    output: &PdfStructuralFacts,
) -> Result<(), PdfOutputValidationError> {
    if source.page_count != output.page_count {
        return Err(PdfOutputValidationError::PageCountMismatch(
            source.page_count,
            output.page_count,
        ));
    }
    if source.encrypted != output.encrypted {
        return Err(PdfOutputValidationError::EncryptionMismatch);
    }
    if source.page_media_boxes != output.page_media_boxes {
        return Err(PdfOutputValidationError::PageGeometryMismatch);
    }
    if source.form_fields != output.form_fields {
        return Err(PdfOutputValidationError::FormFieldsMismatch);
    }
    if source.annotations != output.annotations {
        return Err(PdfOutputValidationError::AnnotationsMismatch);
    }
    if source.outlines != output.outlines {
        return Err(PdfOutputValidationError::OutlinesMismatch);
    }
    if source.attachments != output.attachments {
        return Err(PdfOutputValidationError::AttachmentsMismatch);
    }
    Ok(())
}

pub async fn validate_staged_pdf_output(
    qpdf: &Path,
    staged: &PdfStagedOutput,
    cancelled: &AtomicBool,
) -> Result<VerifiedPdfOutput, PdfOutputValidationError> {
    validate_staged_pdf_output_with_size_policy(qpdf, staged, false, cancelled).await
}

pub async fn validate_staged_pdf_output_with_size_policy(
    qpdf: &Path,
    staged: &PdfStagedOutput,
    allow_larger_output: bool,
    cancelled: &AtomicBool,
) -> Result<VerifiedPdfOutput, PdfOutputValidationError> {
    let metadata = std::fs::symlink_metadata(staged.path())
        .map_err(|_| PdfOutputValidationError::NotRegularFile)?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(PdfOutputValidationError::NotRegularFile);
    }
    if metadata.len() == 0 {
        return Err(PdfOutputValidationError::Empty);
    }
    if metadata.len() != staged.encoded_bytes() {
        return Err(PdfOutputValidationError::SizeChanged(
            staged.encoded_bytes(),
            metadata.len(),
        ));
    }
    if staged.destination().exists() {
        return Err(PdfOutputValidationError::TargetAppeared);
    }
    if metadata.len() > staged.source_report().input_bytes && !allow_larger_output {
        return Err(PdfOutputValidationError::LargerThanSource(
            staged.source_report().input_bytes,
            metadata.len(),
        ));
    }
    let output_sha256 = file_sha256(staged.path())?;
    let check = run_qpdf_capture(
        qpdf,
        &[
            OsString::from("--check"),
            staged.path().as_os_str().to_owned(),
        ],
        cancelled,
        64 * 1024,
    )
    .await?;
    if check.code != Some(0) {
        return Err(PdfOutputValidationError::QpdfCheckFailed(bounded_detail(
            &check.stderr,
        )));
    }
    let source_facts = structural_facts(qpdf, &staged.source_report().source, cancelled).await?;
    let output_facts = structural_facts(qpdf, staged.path(), cancelled).await?;
    compare_facts(&source_facts, &output_facts)?;
    if std::fs::metadata(staged.path()).map(|item| item.len()).ok() != Some(metadata.len())
        || file_sha256(staged.path())? != output_sha256
    {
        return Err(PdfOutputValidationError::ChangedDuringValidation);
    }
    Ok(VerifiedPdfOutput {
        output_bytes: metadata.len(),
        output_sha256,
        source_facts,
        output_facts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::pdf_transform::{
        transform_pdf_to_staging, PdfOptimizationMode, PdfTransformRequest,
    };
    use std::path::PathBuf;
    use std::sync::Arc;

    fn independent_facts(path: &Path) -> Value {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let output = std::process::Command::new(
            std::env::var("LONG_MEDIA_FIXTURE_PYTHON").unwrap_or_else(|_| "python".into()),
        )
        .arg(root.join("scripts/inspect-d01-pdf.py"))
        .arg(path)
        .env(
            "PYTHONPATH",
            root.join("test-results/media-fixture-audit/python-packages"),
        )
        .stdin(Stdio::null())
        .output()
        .unwrap();
        assert!(
            output.status.success(),
            "independent PDF inspection failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn qpdf() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/pdf-engine/qpdf.exe")
    }

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../test-results/media-fixture-audit/fixtures/pdfs")
            .join(name)
    }

    fn request(source: PathBuf, destination: PathBuf) -> PdfTransformRequest {
        PdfTransformRequest {
            source,
            destination,
            password: None,
            mode: PdfOptimizationMode::LosslessOrganization,
            confirmed_lossy_image_changes: false,
        }
    }

    #[tokio::test]
    #[cfg(windows)]
    #[ignore = "run through npm run test:pdf-d03-validation:real after generating real PDF fixtures"]
    async fn validates_real_pdf_structure_and_rejects_corruption_races_and_larger_output() {
        let directory = tempfile::tempdir().unwrap();
        let cancelled = AtomicBool::new(false);

        let cases = [
            ("text-vector.pdf", 1_u32),
            ("scanned-image.pdf", 1),
            ("mixed-content.pdf", 1),
            ("transparency.pdf", 1),
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
        for (name, expected_pages) in cases {
            for mode in modes {
                let source = fixture(name);
                let source_independent = independent_facts(&source);
                let destination = directory.path().join(format!(
                    "{}-{:?}.validated.pdf",
                    name.trim_end_matches(".pdf"),
                    mode
                ));
                let mut candidate_request = request(source.clone(), destination.clone());
                candidate_request.mode = mode;
                candidate_request.confirmed_lossy_image_changes =
                    mode == PdfOptimizationMode::CompatibleImageOptimization;
                let staged = transform_pdf_to_staging(&qpdf(), &candidate_request, &cancelled)
                    .await
                    .unwrap_or_else(|error| panic!("{name} {mode:?}: {error}"));
                let staged_path = staged.path().to_path_buf();
                let output_independent = independent_facts(&staged_path);
                assert_eq!(
                    source_independent, output_independent,
                    "{name} {mode:?}: independent preservation facts changed"
                );
                let verified = validate_staged_pdf_output(&qpdf(), &staged, &cancelled)
                    .await
                    .unwrap_or_else(|error| panic!("{name} {mode:?}: {error}"));
                assert_eq!(verified.source_facts, verified.output_facts);
                assert_eq!(verified.output_facts.page_count, expected_pages);
                assert!(!destination.exists());
                reports.push(serde_json::json!({
                    "file": name,
                    "mode": mode,
                    "inputBytes": staged.source_report().input_bytes,
                    "outputBytes": verified.output_bytes,
                    "outputSha256": verified.output_sha256,
                    "pages": verified.output_facts.page_count,
                    "formFields": verified.output_facts.form_fields.len(),
                    "annotations": verified.output_facts.annotations.len(),
                    "outlines": verified.output_facts.outlines.len(),
                    "attachments": verified.output_facts.attachments.len(),
                    "independentFactsEqual": true,
                    "finalOutputExists": destination.exists(),
                    "stagingExistsBeforeDrop": staged_path.exists()
                }));
                drop(staged);
                assert!(!staged_path.exists());
            }
        }

        let destination = directory.path().join("form.validated.pdf");
        let staged = transform_pdf_to_staging(
            &qpdf(),
            &request(fixture("form.pdf"), destination.clone()),
            &cancelled,
        )
        .await
        .unwrap();
        let verified = validate_staged_pdf_output(&qpdf(), &staged, &cancelled)
            .await
            .unwrap();
        assert_eq!(verified.source_facts, verified.output_facts);
        assert_eq!(verified.source_facts.form_fields.len(), 2);
        assert!(!destination.exists());
        drop(staged);

        let corrupt_destination = directory.path().join("corrupt.pdf");
        let corrupt = transform_pdf_to_staging(
            &qpdf(),
            &request(fixture("text-vector.pdf"), corrupt_destination.clone()),
            &cancelled,
        )
        .await
        .unwrap();
        std::fs::write(corrupt.path(), vec![0_u8; corrupt.encoded_bytes() as usize]).unwrap();
        let corrupt_error = validate_staged_pdf_output(&qpdf(), &corrupt, &cancelled)
            .await
            .unwrap_err();
        assert!(matches!(
            corrupt_error,
            PdfOutputValidationError::QpdfCheckFailed(_)
        ));
        let corrupt_staging = corrupt.path().to_path_buf();
        drop(corrupt);
        assert!(!corrupt_staging.exists());
        assert!(!corrupt_destination.exists());

        let race_destination = directory.path().join("race.pdf");
        let race = transform_pdf_to_staging(
            &qpdf(),
            &request(fixture("text-vector.pdf"), race_destination.clone()),
            &cancelled,
        )
        .await
        .unwrap();
        std::fs::write(&race_destination, b"existing user bytes").unwrap();
        let race_error = validate_staged_pdf_output(&qpdf(), &race, &cancelled)
            .await
            .unwrap_err();
        assert_eq!(race_error, PdfOutputValidationError::TargetAppeared);
        assert_eq!(
            std::fs::read(&race_destination).unwrap(),
            b"existing user bytes"
        );
        drop(race);

        let larger_destination = directory.path().join("larger.pdf");
        let mut larger = transform_pdf_to_staging(
            &qpdf(),
            &request(fixture("text-vector.pdf"), larger_destination.clone()),
            &cancelled,
        )
        .await
        .unwrap();
        let source_bytes = larger.source_report().input_bytes;
        let mut bytes = std::fs::read(larger.path()).unwrap();
        bytes.resize(source_bytes as usize + 1, b' ');
        std::fs::write(larger.path(), &bytes).unwrap();
        larger.set_encoded_bytes_for_test(bytes.len() as u64);
        let larger_error = validate_staged_pdf_output(&qpdf(), &larger, &cancelled)
            .await
            .unwrap_err();
        assert_eq!(
            larger_error,
            PdfOutputValidationError::LargerThanSource(source_bytes, source_bytes + 1)
        );
        drop(larger);
        assert!(!larger_destination.exists());

        let cancelled_destination = directory.path().join("cancelled.pdf");
        let cancelled_staged = transform_pdf_to_staging(
            &qpdf(),
            &request(fixture("text-vector.pdf"), cancelled_destination.clone()),
            &cancelled,
        )
        .await
        .unwrap();
        let validation_cancellation = Arc::new(AtomicBool::new(false));
        let cancellation_sender = Arc::clone(&validation_cancellation);
        let cancellation_thread = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(10));
            cancellation_sender.store(true, Ordering::Release);
        });
        let cancelled_error =
            validate_staged_pdf_output(&qpdf(), &cancelled_staged, &validation_cancellation)
                .await
                .unwrap_err();
        cancellation_thread.join().unwrap();
        assert_eq!(cancelled_error, PdfOutputValidationError::Cancelled);
        drop(cancelled_staged);
        assert!(!cancelled_destination.exists());

        let damaged_source = directory.path().join("damaged-input.pdf");
        let damaged_destination = directory.path().join("damaged-output.pdf");
        std::fs::write(&damaged_source, b"%PDF-1.7\nbroken-real-input\n%%EOF\n").unwrap();
        let damaged_error = transform_pdf_to_staging(
            &qpdf(),
            &request(damaged_source, damaged_destination.clone()),
            &cancelled,
        )
        .await
        .unwrap_err();
        assert!(damaged_error.to_string().starts_with("PDF_ANALYSIS_"));
        assert!(!damaged_destination.exists());

        println!(
            "D03_PDF_VALIDATION_RESULT={}",
            serde_json::json!({
                "reports": reports,
                "validatedFormFields": verified.source_facts.form_fields.len(),
                "corruptError": corrupt_error.to_string().split(':').next().unwrap_or_default(),
                "targetRaceError": race_error.to_string(),
                "targetRaceBytesPreserved": std::fs::read(&race_destination).unwrap() == b"existing user bytes",
                "largerError": larger_error.to_string().split(':').next().unwrap_or_default(),
                "cancelledError": cancelled_error.to_string(),
                "damagedInputError": damaged_error.to_string().split(':').next().unwrap_or_default(),
                "finalOutputsAbsent": !destination.exists() && !corrupt_destination.exists() && !larger_destination.exists() && !cancelled_destination.exists() && !damaged_destination.exists()
            })
        );
    }
}
