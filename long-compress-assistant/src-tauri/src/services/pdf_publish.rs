use crate::services::mark_of_web::{self, PropagationStatus};
use crate::services::output_publish_transaction::{publish_verified_file, PublishError};
use crate::services::pdf_output_validation::{
    validate_staged_pdf_output, PdfOutputValidationError, VerifiedPdfOutput,
};
use crate::services::pdf_transform::{
    transform_pdf_to_staging, PdfStagedOutput, PdfTransformError, PdfTransformRequest,
};
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;

static ACTIVE_PDF_OUTPUTS: Lazy<DashMap<String, String>> = Lazy::new(DashMap::new);

#[derive(Debug, Error)]
pub enum PdfPublishError {
    #[error("PDF_PUBLISH_OUTPUT_LOCKED: {0}")]
    OutputLocked(PathBuf),
    #[error("PDF_PUBLISH_DESTINATION_INVALID: {0}")]
    InvalidDestination(String),
    #[error("PDF_PUBLISH_CANCELLED")]
    Cancelled,
    #[error("PDF_PUBLISH_SOURCE_CHANGED")]
    SourceChanged,
    #[error("PDF_PUBLISH_STAGING_CHANGED")]
    StagingChanged,
    #[error("PDF_PUBLISH_MARK_OF_WEB_READ_FAILED: {0}")]
    MarkOfWebReadFailed(String),
    #[error("PDF_PUBLISH_MARK_OF_WEB_PROPAGATION_FAILED: {0}")]
    MarkOfWebPropagationFailed(String),
    #[error("PDF_PUBLISH_TARGET_APPEARED: {0}")]
    TargetAppeared(PathBuf),
    #[error("PDF_PUBLISH_TRANSACTION_FAILED: {0}")]
    TransactionFailed(String),
    #[error("PDF_PUBLISH_FINAL_IDENTITY_FAILED")]
    FinalIdentityFailed,
    #[error(transparent)]
    Transform(#[from] PdfTransformError),
    #[error(transparent)]
    Validation(#[from] PdfOutputValidationError),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedPdfOutput {
    pub path: PathBuf,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub savings_ratio: f64,
    pub output_sha256: String,
    pub mark_of_the_web: String,
    pub verified: VerifiedPdfOutput,
}

fn normalized_output_key(path: &Path) -> Result<String, PdfPublishError> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| PdfPublishError::InvalidDestination(error.to_string()))?
            .join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    let key = normalized.to_string_lossy().replace('/', "\\");
    #[cfg(windows)]
    let key = key.to_lowercase();
    Ok(key)
}

#[derive(Debug)]
struct PdfOutputReservation {
    key: String,
    owner: String,
    destination: PathBuf,
}

impl PdfOutputReservation {
    fn acquire(destination: &Path) -> Result<Self, PdfPublishError> {
        let key = normalized_output_key(destination)?;
        let owner = uuid::Uuid::new_v4().to_string();
        match ACTIVE_PDF_OUTPUTS.entry(key.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(owner.clone());
                Ok(Self {
                    key,
                    owner,
                    destination: destination.to_path_buf(),
                })
            }
            Entry::Occupied(_) => Err(PdfPublishError::OutputLocked(destination.to_path_buf())),
        }
    }
}

impl Drop for PdfOutputReservation {
    fn drop(&mut self) {
        if ACTIVE_PDF_OUTPUTS
            .get(&self.key)
            .is_some_and(|owner| owner.value() == &self.owner)
        {
            ACTIVE_PDF_OUTPUTS.remove(&self.key);
        }
    }
}

fn file_identity(path: &Path) -> Result<(u64, String), PdfPublishError> {
    let metadata =
        std::fs::symlink_metadata(path).map_err(|_| PdfPublishError::FinalIdentityFailed)?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(PdfPublishError::FinalIdentityFailed);
    }
    let mut reader =
        BufReader::new(File::open(path).map_err(|_| PdfPublishError::FinalIdentityFailed)?);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| PdfPublishError::FinalIdentityFailed)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok((metadata.len(), hex::encode(hasher.finalize())))
}

fn map_publish_error(error: PublishError) -> PdfPublishError {
    match error {
        PublishError::Cancelled => PdfPublishError::Cancelled,
        PublishError::TargetAppeared(path) => PdfPublishError::TargetAppeared(path),
        other => PdfPublishError::TransactionFailed(other.to_string()),
    }
}

fn publish_validated_pdf_output(
    reservation: PdfOutputReservation,
    staged: PdfStagedOutput,
    verified: VerifiedPdfOutput,
    preserve_mark_of_web: bool,
    cancelled: &AtomicBool,
) -> Result<PublishedPdfOutput, PdfPublishError> {
    if cancelled.load(Ordering::Acquire) {
        return Err(PdfPublishError::Cancelled);
    }
    if staged.destination() != reservation.destination {
        return Err(PdfPublishError::InvalidDestination(
            "staging destination differs from the held output reservation".into(),
        ));
    }
    let source = &staged.source_report().source;
    let (source_bytes, source_sha256) = file_identity(source)?;
    if source_bytes != staged.source_report().input_bytes || source_sha256 != staged.source_sha256()
    {
        return Err(PdfPublishError::SourceChanged);
    }
    let (staged_bytes, staged_sha256) = file_identity(staged.path())?;
    if staged_bytes != verified.output_bytes || staged_sha256 != verified.output_sha256 {
        return Err(PdfPublishError::StagingChanged);
    }

    let mark_status = if preserve_mark_of_web {
        match mark_of_web::read_from(source)
            .map_err(|error| PdfPublishError::MarkOfWebReadFailed(error.to_string()))?
        {
            Some(mark) => match mark_of_web::propagate_to_tree(staged.path(), &mark, || {
                cancelled.load(Ordering::Acquire)
            })
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::Interrupted {
                    PdfPublishError::Cancelled
                } else {
                    PdfPublishError::MarkOfWebPropagationFailed(error.to_string())
                }
            })? {
                PropagationStatus::Applied(_) => "applied",
                PropagationStatus::Unsupported => "unsupported",
            },
            None => "not-present",
        }
    } else {
        "not-requested"
    };

    if cancelled.load(Ordering::Acquire) {
        return Err(PdfPublishError::Cancelled);
    }
    let (source_bytes_before_commit, source_sha256_before_commit) = file_identity(source)?;
    if source_bytes_before_commit != staged.source_report().input_bytes
        || source_sha256_before_commit != staged.source_sha256()
    {
        return Err(PdfPublishError::SourceChanged);
    }
    let (staged_bytes_before_commit, staged_sha256_before_commit) = file_identity(staged.path())?;
    if staged_bytes_before_commit != verified.output_bytes
        || staged_sha256_before_commit != verified.output_sha256
    {
        return Err(PdfPublishError::StagingChanged);
    }
    let final_output = reservation.destination.clone();
    publish_verified_file(staged.path(), &final_output, || {
        cancelled.load(Ordering::Acquire)
    })
    .map_err(map_publish_error)?;

    let (output_bytes, output_sha256) = file_identity(&final_output)?;
    if output_bytes != verified.output_bytes || output_sha256 != verified.output_sha256 {
        return Err(PdfPublishError::FinalIdentityFailed);
    }
    let input_bytes = staged.source_report().input_bytes;
    let savings_ratio = if input_bytes == 0 {
        0.0
    } else {
        (input_bytes as f64 - output_bytes as f64) / input_bytes as f64
    };
    Ok(PublishedPdfOutput {
        path: final_output,
        input_bytes,
        output_bytes,
        savings_ratio,
        output_sha256,
        mark_of_the_web: mark_status.to_string(),
        verified,
    })
}

pub async fn execute_pdf_publication_transaction(
    qpdf: &Path,
    request: &PdfTransformRequest,
    preserve_mark_of_web: bool,
    cancelled: &AtomicBool,
) -> Result<PublishedPdfOutput, PdfPublishError> {
    let reservation = PdfOutputReservation::acquire(&request.destination)?;
    if cancelled.load(Ordering::Acquire) {
        return Err(PdfPublishError::Cancelled);
    }
    let staged = transform_pdf_to_staging(qpdf, request, cancelled).await?;
    let verified = validate_staged_pdf_output(qpdf, &staged, cancelled).await?;
    publish_validated_pdf_output(
        reservation,
        staged,
        verified,
        preserve_mark_of_web,
        cancelled,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::pdf_transform::PdfOptimizationMode;
    use std::ffi::OsString;
    use std::process::Stdio;

    fn qpdf() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/pdf-engine/qpdf.exe")
    }

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../test-results/media-fixture-audit/fixtures/pdfs")
            .join(name)
    }

    fn independent_facts(path: &Path) -> serde_json::Value {
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
            "independent published PDF inspection failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn write_minimal_real_pdf(path: &Path) {
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>",
            "<< /Length 0 >>\nstream\n\nendstream",
        ];
        let mut bytes = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(bytes.len());
            bytes
                .extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
        }
        let xref = bytes.len();
        bytes.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
        bytes.extend_from_slice(b"0000000000 65535 f \n");
        for offset in offsets {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                objects.len() + 1
            )
            .as_bytes(),
        );
        std::fs::write(path, bytes).expect("write minimal real PDF");
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
    fn equivalent_destinations_share_one_cross_task_reservation() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("locked.pdf");
        let equivalent = directory.path().join(".").join("locked.pdf");
        let first = PdfOutputReservation::acquire(&destination).unwrap();
        assert!(matches!(
            PdfOutputReservation::acquire(&equivalent),
            Err(PdfPublishError::OutputLocked(_))
        ));
        drop(first);
        assert!(PdfOutputReservation::acquire(&equivalent).is_ok());
    }

    #[tokio::test]
    #[ignore = "run through scripts/run-d03-pdf-publication.mjs with real qpdf fixtures"]
    async fn publishes_real_pdf_matrix_and_rejects_post_validation_failures() {
        let directory = tempfile::tempdir().unwrap();
        let cancelled = AtomicBool::new(false);
        let cases = [
            "text-vector.pdf",
            "scanned-image.pdf",
            "mixed-content.pdf",
            "transparency.pdf",
            "form.pdf",
            "annotation.pdf",
            "outline.pdf",
            "attachment.pdf",
        ];
        let modes = [
            PdfOptimizationMode::LosslessOrganization,
            PdfOptimizationMode::CompatibleImageOptimization,
        ];
        let mut reports = Vec::new();
        for name in cases {
            for mode in modes {
                let source = fixture(name);
                let destination = directory.path().join(format!(
                    "{}-{mode:?}.published.pdf",
                    name.trim_end_matches(".pdf")
                ));
                let source_before = file_identity(&source).unwrap().1;
                let source_independent = independent_facts(&source);
                let published = execute_pdf_publication_transaction(
                    &qpdf(),
                    &request(source.clone(), destination.clone(), mode),
                    true,
                    &cancelled,
                )
                .await
                .unwrap_or_else(|error| panic!("{name} {mode:?}: {error}"));
                let published_independent = independent_facts(&destination);
                reports.push(serde_json::json!({
                    "file": name,
                    "mode": mode,
                    "inputBytes": published.input_bytes,
                    "outputBytes": published.output_bytes,
                    "outputSha256": published.output_sha256,
                    "finalSha256": file_identity(&destination).unwrap().1,
                    "sourceHashUnchanged": file_identity(&source).unwrap().1 == source_before,
                    "independentFactsEqual": source_independent == published_independent,
                    "markOfTheWeb": published.mark_of_the_web,
                    "finalOutputExists": destination.exists(),
                    "stagingCount": std::fs::read_dir(directory.path()).unwrap()
                        .flatten()
                        .filter(|entry| entry.file_name().to_string_lossy().starts_with(".pdf-transform-"))
                        .count()
                }));
            }
        }

        let source_change_destination = directory.path().join("source-change.pdf");
        let source_change_source = directory.path().join("source-change-input.pdf");
        std::fs::copy(fixture("text-vector.pdf"), &source_change_source).unwrap();
        let reservation = PdfOutputReservation::acquire(&source_change_destination).unwrap();
        let staged = transform_pdf_to_staging(
            &qpdf(),
            &request(
                source_change_source.clone(),
                source_change_destination.clone(),
                PdfOptimizationMode::LosslessOrganization,
            ),
            &cancelled,
        )
        .await
        .unwrap();
        let verified = validate_staged_pdf_output(&qpdf(), &staged, &cancelled)
            .await
            .unwrap();
        std::fs::write(&source_change_source, b"changed after validation").unwrap();
        let source_change_error =
            publish_validated_pdf_output(reservation, staged, verified, false, &cancelled)
                .unwrap_err();

        let staging_change_destination = directory.path().join("staging-change.pdf");
        let reservation = PdfOutputReservation::acquire(&staging_change_destination).unwrap();
        let staged = transform_pdf_to_staging(
            &qpdf(),
            &request(
                fixture("text-vector.pdf"),
                staging_change_destination.clone(),
                PdfOptimizationMode::LosslessOrganization,
            ),
            &cancelled,
        )
        .await
        .unwrap();
        let verified = validate_staged_pdf_output(&qpdf(), &staged, &cancelled)
            .await
            .unwrap();
        std::fs::write(staged.path(), vec![0_u8; verified.output_bytes as usize]).unwrap();
        let staging_change_error =
            publish_validated_pdf_output(reservation, staged, verified, false, &cancelled)
                .unwrap_err();

        let target_destination = directory.path().join("target-race.pdf");
        let reservation = PdfOutputReservation::acquire(&target_destination).unwrap();
        let staged = transform_pdf_to_staging(
            &qpdf(),
            &request(
                fixture("text-vector.pdf"),
                target_destination.clone(),
                PdfOptimizationMode::LosslessOrganization,
            ),
            &cancelled,
        )
        .await
        .unwrap();
        let verified = validate_staged_pdf_output(&qpdf(), &staged, &cancelled)
            .await
            .unwrap();
        std::fs::write(&target_destination, b"existing user bytes").unwrap();
        let target_error =
            publish_validated_pdf_output(reservation, staged, verified, false, &cancelled)
                .unwrap_err();

        let cancel_destination = directory.path().join("cancel.pdf");
        let reservation = PdfOutputReservation::acquire(&cancel_destination).unwrap();
        let staged = transform_pdf_to_staging(
            &qpdf(),
            &request(
                fixture("text-vector.pdf"),
                cancel_destination.clone(),
                PdfOptimizationMode::LosslessOrganization,
            ),
            &cancelled,
        )
        .await
        .unwrap();
        let verified = validate_staged_pdf_output(&qpdf(), &staged, &cancelled)
            .await
            .unwrap();
        let publish_cancelled = AtomicBool::new(true);
        let cancel_error =
            publish_validated_pdf_output(reservation, staged, verified, false, &publish_cancelled)
                .unwrap_err();

        let motw_source = directory.path().join("downloaded.pdf");
        std::fs::copy(fixture("text-vector.pdf"), &motw_source).unwrap();
        #[cfg(windows)]
        {
            use std::os::windows::ffi::{OsStrExt, OsStringExt};
            let mut encoded: Vec<u16> = motw_source.as_os_str().encode_wide().collect();
            encoded.extend(":Zone.Identifier".encode_utf16());
            std::fs::write(
                PathBuf::from(OsString::from_wide(&encoded)),
                b"[ZoneTransfer]\r\nZoneId=3\r\nHostUrl=https://example.test/input.pdf\r\n",
            )
            .unwrap();
        }
        let motw_destination = directory.path().join("marked-output.pdf");
        let motw = execute_pdf_publication_transaction(
            &qpdf(),
            &request(
                motw_source,
                motw_destination.clone(),
                PdfOptimizationMode::LosslessOrganization,
            ),
            true,
            &cancelled,
        )
        .await
        .unwrap();
        let motw_zone = mark_of_web::read_from(&motw_destination)
            .unwrap()
            .map(|mark| mark.zone_id());

        let lock_destination = directory.path().join("transaction-lock.pdf");
        let lock_equivalent = directory.path().join(".").join("transaction-lock.pdf");
        let held_lock = PdfOutputReservation::acquire(&lock_destination).unwrap();
        let duplicate_lock_rejected = matches!(
            PdfOutputReservation::acquire(&lock_equivalent),
            Err(PdfPublishError::OutputLocked(_))
        );
        drop(held_lock);
        let lock_released_after_drop = PdfOutputReservation::acquire(&lock_equivalent).is_ok();

        println!(
            "D03_PDF_PUBLICATION_RESULT={}",
            serde_json::json!({
                "reports": reports,
                "sourceChangeError": source_change_error.to_string(),
                "sourceChangeOutputAbsent": !source_change_destination.exists(),
                "stagingChangeError": staging_change_error.to_string(),
                "stagingChangeOutputAbsent": !staging_change_destination.exists(),
                "targetRaceError": target_error.to_string().split(':').next().unwrap_or_default(),
                "targetRaceBytesPreserved": std::fs::read(&target_destination).unwrap() == b"existing user bytes",
                "cancelError": cancel_error.to_string(),
                "cancelOutputAbsent": !cancel_destination.exists(),
                "motwStatus": motw.mark_of_the_web,
                "motwFinalZone": motw_zone,
                "motwOutputExists": motw_destination.exists(),
                "duplicateLockRejected": duplicate_lock_rejected,
                "lockReleasedAfterDrop": lock_released_after_drop,
                "stagingFilesRemaining": std::fs::read_dir(directory.path()).unwrap()
                    .flatten()
                    .filter(|entry| entry.file_name().to_string_lossy().starts_with(".pdf-transform-"))
                    .count(),
            })
        );
    }

    #[tokio::test]
    #[cfg(windows)]
    #[ignore = "requires LONG_D03_LOW_CAPACITY_PATH pointing to an isolated real NTFS volume"]
    async fn real_low_capacity_volume_blocks_pdf_transaction_without_artifacts() {
        use crate::services::storage_preflight::{probe_storage, DISK_SAFETY_RESERVE};

        let volume = PathBuf::from(
            std::env::var("LONG_D03_LOW_CAPACITY_PATH")
                .expect("isolated low-capacity volume path is required"),
        );
        assert!(volume.is_dir(), "isolated volume must be mounted");
        let probe_file = volume.join("real-write-probe.bin");
        std::fs::write(&probe_file, vec![0x5a; 1024 * 1024])
            .expect("isolated volume must accept a real write");
        let target = probe_storage(&volume);
        let total_bytes = target.total_bytes.expect("real volume total bytes");
        let available_bytes = target.available_bytes.expect("real volume available bytes");
        assert!(
            total_bytes < DISK_SAFETY_RESERVE,
            "test volume must be smaller than the production safety reserve"
        );

        let source_directory = tempfile::tempdir().expect("real source directory");
        let source = source_directory.path().join("minimal-real.pdf");
        write_minimal_real_pdf(&source);
        let source_before = file_identity(&source).unwrap().1;
        let destination = volume.join("must-not-publish.pdf");
        let error = execute_pdf_publication_transaction(
            &qpdf(),
            &request(
                source.clone(),
                destination.clone(),
                PdfOptimizationMode::LosslessOrganization,
            ),
            true,
            &AtomicBool::new(false),
        )
        .await
        .expect_err("production preflight must reject the real low-capacity volume");
        let source_after = file_identity(&source).unwrap().1;
        let staging_files: Vec<_> = std::fs::read_dir(&volume)
            .unwrap()
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".pdf-transform-")
            })
            .map(|entry| entry.path())
            .collect();
        std::fs::remove_file(&probe_file).expect("remove real write probe");

        println!(
            "D03_PDF_LOW_CAPACITY_RESULT={}",
            serde_json::json!({
                "fileSystem": target.file_system,
                "mountPoint": target.mount_point,
                "totalBytes": total_bytes,
                "availableBytes": available_bytes,
                "reserveBytes": DISK_SAFETY_RESERVE,
                "realWriteProbeBytes": 1024 * 1024,
                "error": error.to_string().split(':').next().unwrap_or_default(),
                "finalOutputExists": destination.exists(),
                "stagingFiles": staging_files,
                "sourceHashUnchanged": source_before == source_after,
            })
        );
    }

    #[test]
    #[cfg(windows)]
    fn github_actions_runs_real_low_capacity_volume_gate() {
        if std::env::var("GITHUB_ACTIONS").as_deref() != Ok("true") {
            return;
        }
        if std::env::var("LONG_D03_LOW_CAPACITY_PATH").is_ok() {
            return;
        }
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let status = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(root.join("scripts/test-d03-pdf-low-capacity.ps1"))
            .current_dir(&root)
            .status()
            .expect("launch isolated low-capacity VHD gate");
        assert!(status.success(), "isolated low-capacity VHD gate failed");
    }
}
