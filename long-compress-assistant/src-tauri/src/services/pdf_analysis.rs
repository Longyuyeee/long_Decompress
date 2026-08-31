use serde::Serialize;
use serde_json::Value;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

const ANALYSIS_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_ANALYSIS_OUTPUT_BYTES: usize = 32 * 1024 * 1024;
const MAX_ERROR_DETAIL_CHARS: usize = 2_048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PdfPasswordState {
    NotRequired,
    Required,
    Accepted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfInputAnalysisReport {
    pub source: PathBuf,
    pub input_bytes: u64,
    pub analysis_complete: bool,
    pub page_count: Option<u32>,
    pub encrypted: bool,
    pub password_state: PdfPasswordState,
    pub has_digital_signature: Option<bool>,
    pub signature_field_names: Vec<String>,
    pub has_form_fields: Option<bool>,
    pub form_field_names: Vec<String>,
    pub has_attachments: Option<bool>,
    pub attachment_names: Vec<String>,
    pub outline_count: Option<u32>,
    pub warnings: Vec<String>,
    pub blocking_reasons: Vec<String>,
}

#[derive(Debug, Error)]
pub enum PdfAnalysisError {
    #[error("PDF_ANALYSIS_SOURCE_MISSING: {0}")]
    SourceMissing(String),
    #[error("PDF_ANALYSIS_SOURCE_NOT_FILE: {0}")]
    SourceNotFile(String),
    #[error("PDF_ANALYSIS_SOURCE_EMPTY: {0}")]
    SourceEmpty(String),
    #[error("PDF_ANALYSIS_PASSWORD_CONTAINS_LINE_BREAK")]
    PasswordContainsLineBreak,
    #[error("PDF_ANALYSIS_INVALID_PASSWORD")]
    InvalidPassword,
    #[error("PDF_ANALYSIS_LAUNCH_FAILED: {0}")]
    LaunchFailed(String),
    #[error("PDF_ANALYSIS_TIMEOUT: exceeded 30 seconds")]
    Timeout,
    #[error("PDF_ANALYSIS_PROCESS_FAILED: {0}")]
    ProcessFailed(String),
    #[error("PDF_ANALYSIS_OUTPUT_TOO_LARGE: metadata exceeded 32 MiB")]
    OutputTooLarge,
    #[error("PDF_ANALYSIS_INVALID_JSON: {0}")]
    InvalidJson(String),
    #[error("PDF_ANALYSIS_INVALID_PAGE_COUNT")]
    InvalidPageCount,
}

fn bounded_error_detail(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .trim()
        .chars()
        .take(MAX_ERROR_DETAIL_CHARS)
        .collect()
}

fn json_arguments(source: &Path, password_from_stdin: bool) -> Vec<OsString> {
    let mut arguments = Vec::with_capacity(10);
    if password_from_stdin {
        arguments.push(OsString::from("--password-file=-"));
    }
    arguments.extend([
        OsString::from("--json=2"),
        OsString::from("--json-stream-data=none"),
        OsString::from("--json-key=pages"),
        OsString::from("--json-key=acroform"),
        OsString::from("--json-key=attachments"),
        OsString::from("--json-key=outlines"),
        source.as_os_str().to_owned(),
    ]);
    arguments
}

async fn run_boolean_probe(
    qpdf: &Path,
    option: &'static str,
    source: &Path,
) -> Result<bool, PdfAnalysisError> {
    let output = tokio::time::timeout(
        ANALYSIS_TIMEOUT,
        Command::new(qpdf)
            .arg(option)
            .arg(source)
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| PdfAnalysisError::Timeout)?
    .map_err(|error| PdfAnalysisError::LaunchFailed(error.to_string()))?;

    match output.status.code() {
        Some(0) => Ok(true),
        Some(2) => Ok(false),
        _ => Err(PdfAnalysisError::ProcessFailed(bounded_error_detail(
            &output.stderr,
        ))),
    }
}

async fn run_json_probe(
    qpdf: &Path,
    source: &Path,
    password: Option<&str>,
) -> Result<Value, PdfAnalysisError> {
    if password.is_some_and(|value| value.contains(['\r', '\n', '\0'])) {
        return Err(PdfAnalysisError::PasswordContainsLineBreak);
    }

    let mut command = Command::new(qpdf);
    command
        .args(json_arguments(source, password.is_some()))
        .stdin(if password.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|error| PdfAnalysisError::LaunchFailed(error.to_string()))?;

    if let Some(password) = password {
        let mut stdin = child.stdin.take().ok_or_else(|| {
            PdfAnalysisError::LaunchFailed("qpdf password stdin unavailable".to_string())
        })?;
        stdin
            .write_all(password.as_bytes())
            .await
            .map_err(|error| PdfAnalysisError::LaunchFailed(error.to_string()))?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(|error| PdfAnalysisError::LaunchFailed(error.to_string()))?;
        stdin
            .shutdown()
            .await
            .map_err(|error| PdfAnalysisError::LaunchFailed(error.to_string()))?;
    }

    let output = tokio::time::timeout(ANALYSIS_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| PdfAnalysisError::Timeout)?
        .map_err(|error| PdfAnalysisError::LaunchFailed(error.to_string()))?;
    if output.stdout.len() > MAX_ANALYSIS_OUTPUT_BYTES {
        return Err(PdfAnalysisError::OutputTooLarge);
    }
    if !matches!(output.status.code(), Some(0 | 3)) {
        let detail = bounded_error_detail(&output.stderr);
        if detail.to_ascii_lowercase().contains("invalid password") {
            return Err(PdfAnalysisError::InvalidPassword);
        }
        return Err(PdfAnalysisError::ProcessFailed(detail));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| PdfAnalysisError::InvalidJson(error.to_string()))
}

fn field_names(root: &Value, signature_fields: bool) -> Vec<String> {
    let mut names: Vec<String> = root
        .pointer("/acroform/fields")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|field| {
            (field.get("fieldtype").and_then(Value::as_str) == Some("/Sig")) == signature_fields
        })
        .filter_map(|field| field.get("fullname").and_then(Value::as_str))
        .map(str::to_owned)
        .collect();
    names.sort();
    names.dedup();
    names
}

fn attachment_names(root: &Value) -> Vec<String> {
    let mut names: Vec<String> = root
        .get("attachments")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|attachments| attachments.iter())
        .map(|(key, attachment)| {
            attachment
                .get("preferredname")
                .and_then(Value::as_str)
                .unwrap_or(key)
                .to_owned()
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

fn count_outlines(items: Option<&Vec<Value>>) -> u32 {
    items
        .into_iter()
        .flatten()
        .map(|item| {
            1_u32.saturating_add(count_outlines(item.get("kids").and_then(Value::as_array)))
        })
        .fold(0_u32, u32::saturating_add)
}

fn complete_report(
    source: &Path,
    input_bytes: u64,
    encrypted: bool,
    root: &Value,
) -> Result<PdfInputAnalysisReport, PdfAnalysisError> {
    let page_count = root
        .get("pages")
        .and_then(Value::as_array)
        .and_then(|pages| u32::try_from(pages.len()).ok())
        .filter(|count| *count > 0)
        .ok_or(PdfAnalysisError::InvalidPageCount)?;
    let signature_field_names = field_names(root, true);
    let form_field_names = field_names(root, false);
    let attachment_names = attachment_names(root);
    let outline_count = count_outlines(root.get("outlines").and_then(Value::as_array));
    let mut warnings = Vec::new();
    let mut blocking_reasons = Vec::new();
    if !signature_field_names.is_empty() {
        warnings.push("PDF_ANALYSIS_DIGITAL_SIGNATURE_MAY_BE_INVALIDATED".to_string());
        blocking_reasons.push("PDF_ANALYSIS_SIGNED_DOCUMENT_ANALYSIS_ONLY".to_string());
    }
    if encrypted {
        warnings.push("PDF_ANALYSIS_ENCRYPTED_DOCUMENT_REQUIRES_PASSWORD_FOR_PLANNING".to_string());
    }

    Ok(PdfInputAnalysisReport {
        source: source.to_path_buf(),
        input_bytes,
        analysis_complete: true,
        page_count: Some(page_count),
        encrypted,
        password_state: if encrypted {
            PdfPasswordState::Accepted
        } else {
            PdfPasswordState::NotRequired
        },
        has_digital_signature: Some(!signature_field_names.is_empty()),
        signature_field_names,
        has_form_fields: Some(!form_field_names.is_empty()),
        form_field_names,
        has_attachments: Some(!attachment_names.is_empty()),
        attachment_names,
        outline_count: Some(outline_count),
        warnings,
        blocking_reasons,
    })
}

pub async fn analyze_pdf_input(
    qpdf: &Path,
    source: &Path,
    password: Option<&str>,
) -> Result<PdfInputAnalysisReport, PdfAnalysisError> {
    let metadata = std::fs::metadata(source)
        .map_err(|_| PdfAnalysisError::SourceMissing(source.display().to_string()))?;
    if !metadata.is_file() {
        return Err(PdfAnalysisError::SourceNotFile(
            source.display().to_string(),
        ));
    }
    if metadata.len() == 0 {
        return Err(PdfAnalysisError::SourceEmpty(source.display().to_string()));
    }

    let encrypted = run_boolean_probe(qpdf, "--is-encrypted", source).await?;
    let requires_password = run_boolean_probe(qpdf, "--requires-password", source).await?;
    if requires_password && password.is_none() {
        return Ok(PdfInputAnalysisReport {
            source: source.to_path_buf(),
            input_bytes: metadata.len(),
            analysis_complete: false,
            page_count: None,
            encrypted,
            password_state: PdfPasswordState::Required,
            has_digital_signature: None,
            signature_field_names: Vec::new(),
            has_form_fields: None,
            form_field_names: Vec::new(),
            has_attachments: None,
            attachment_names: Vec::new(),
            outline_count: None,
            warnings: vec!["PDF_ANALYSIS_PASSWORD_REQUIRED_FOR_STRUCTURAL_FACTS".to_string()],
            blocking_reasons: vec!["PDF_ANALYSIS_PASSWORD_REQUIRED".to_string()],
        });
    }

    let root = run_json_probe(
        qpdf,
        source,
        if requires_password { password } else { None },
    )
    .await?;
    complete_report(source, metadata.len(), encrypted, &root)
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

    #[test]
    fn parser_separates_signature_fields_from_interactive_form_fields() {
        let root = serde_json::json!({
            "pages": [{}],
            "acroform": { "fields": [
                { "fieldtype": "/Sig", "fullname": "Signature" },
                { "fieldtype": "/Tx", "fullname": "Name" }
            ] },
            "attachments": { "evidence": { "preferredname": "evidence.txt" } },
            "outlines": [{ "kids": [{ "kids": [] }] }]
        });
        let report = complete_report(Path::new("sample.pdf"), 100, false, &root).unwrap();
        assert_eq!(report.page_count, Some(1));
        assert_eq!(report.signature_field_names, ["Signature"]);
        assert_eq!(report.form_field_names, ["Name"]);
        assert_eq!(report.attachment_names, ["evidence.txt"]);
        assert_eq!(report.outline_count, Some(2));
        assert_eq!(
            report.blocking_reasons,
            ["PDF_ANALYSIS_SIGNED_DOCUMENT_ANALYSIS_ONLY"]
        );
    }

    #[test]
    fn password_value_is_never_part_of_the_qpdf_argument_array() {
        let secret = "never-put-this-secret-in-arguments";
        let arguments = json_arguments(Path::new("input.pdf"), true);
        let joined = arguments
            .iter()
            .map(|item| item.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.contains("--password-file=-"));
        assert!(!joined.contains(secret));
        assert!(!joined.contains("--password="));
    }

    #[tokio::test]
    async fn refuses_empty_input_before_launching_qpdf() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("empty.pdf");
        std::fs::File::create(&source).unwrap();
        let error = analyze_pdf_input(Path::new("missing-qpdf"), &source, None)
            .await
            .unwrap_err();
        assert!(error.to_string().starts_with("PDF_ANALYSIS_SOURCE_EMPTY:"));
    }

    #[tokio::test]
    #[cfg(windows)]
    #[ignore = "run through npm run test:pdf-d02-analysis:real after generating real PDF fixtures"]
    async fn probes_real_pdf_fixture_matrix_with_product_runtime() {
        let cases = [
            "text-vector.pdf",
            "scanned-image.pdf",
            "mixed-content.pdf",
            "transparency.pdf",
            "form.pdf",
            "annotation.pdf",
            "outline.pdf",
            "attachment.pdf",
            "signed.pdf",
        ];
        let mut reports = Vec::new();
        for name in cases {
            reports.push(
                analyze_pdf_input(&qpdf(), &fixture(name), None)
                    .await
                    .unwrap_or_else(|error| panic!("{name}: {error}")),
            );
        }

        let encrypted = fixture("encrypted.pdf");
        let locked = analyze_pdf_input(&qpdf(), &encrypted, None).await.unwrap();
        assert!(!locked.analysis_complete);
        assert_eq!(locked.password_state, PdfPasswordState::Required);
        assert_eq!(locked.page_count, None);

        let wrong_password = "synthetic-wrong-password";
        let error = analyze_pdf_input(&qpdf(), &encrypted, Some(wrong_password))
            .await
            .unwrap_err();
        let wrong_password_error = error.to_string();
        assert_eq!(wrong_password_error, "PDF_ANALYSIS_INVALID_PASSWORD");
        assert!(!wrong_password_error.contains(wrong_password));

        let unlocked = analyze_pdf_input(&qpdf(), &encrypted, Some("fixture-user"))
            .await
            .unwrap();
        assert!(unlocked.analysis_complete);
        assert_eq!(unlocked.password_state, PdfPasswordState::Accepted);
        reports.push(unlocked);

        println!(
            "D02_PDF_ANALYSIS_RESULT={}",
            serde_json::json!({
                "reports": reports,
                "lockedEncryptedReport": locked,
                "securityControls": {
                    "wrongPasswordError": wrong_password_error,
                    "wrongPasswordLeaked": wrong_password_error.contains(wrong_password)
                }
            })
        );
    }
}
