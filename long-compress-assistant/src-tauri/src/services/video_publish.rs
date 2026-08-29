use crate::services::mark_of_web::{self, PropagationStatus};
use crate::services::output_publish_transaction::{publish_verified_file, PublishError};
use crate::services::video_encoding::StagedVideoOutput;
use crate::services::video_output_validation::VerifiedVideoOutput;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum VideoPublishError {
    #[error("VIDEO_PUBLISH_CANCELLED")]
    Cancelled,
    #[error("VIDEO_PUBLISH_SOURCE_CHANGED: expected={0} actual={1}")]
    SourceChanged(u64, u64),
    #[error("VIDEO_PUBLISH_STAGING_CHANGED: expected={0} actual={1}")]
    StagingChanged(u64, u64),
    #[error("VIDEO_PUBLISH_MARK_OF_WEB_READ_FAILED: {0}")]
    MarkOfWebReadFailed(String),
    #[error("VIDEO_PUBLISH_MARK_OF_WEB_PROPAGATION_FAILED: {0}")]
    MarkOfWebPropagationFailed(String),
    #[error("VIDEO_PUBLISH_TARGET_APPEARED: {0}")]
    TargetAppeared(PathBuf),
    #[error("VIDEO_PUBLISH_TRANSACTION_FAILED: {0}")]
    TransactionFailed(String),
    #[error("VIDEO_PUBLISH_FINAL_METADATA_FAILED: {0}")]
    FinalMetadataFailed(String),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedVideoOutput {
    pub path: PathBuf,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub savings_ratio: f64,
    pub mark_of_the_web: String,
    pub verified: VerifiedVideoOutput,
}

fn map_publish_error(error: PublishError) -> VideoPublishError {
    match error {
        PublishError::Cancelled => VideoPublishError::Cancelled,
        PublishError::TargetAppeared(path) => VideoPublishError::TargetAppeared(path),
        other => VideoPublishError::TransactionFailed(other.to_string()),
    }
}

pub fn publish_validated_video_output(
    staged: StagedVideoOutput,
    verified: VerifiedVideoOutput,
    source: &Path,
    final_output: &Path,
    preserve_mark_of_web: bool,
    cancelled: &AtomicBool,
) -> Result<PublishedVideoOutput, VideoPublishError> {
    if cancelled.load(Ordering::Acquire) {
        return Err(VideoPublishError::Cancelled);
    }
    let source_bytes = std::fs::metadata(source)
        .map_err(|error| VideoPublishError::FinalMetadataFailed(error.to_string()))?
        .len();
    if source_bytes != staged.input_bytes() {
        return Err(VideoPublishError::SourceChanged(
            staged.input_bytes(),
            source_bytes,
        ));
    }
    let staged_bytes = std::fs::metadata(staged.path())
        .map_err(|error| VideoPublishError::FinalMetadataFailed(error.to_string()))?
        .len();
    if staged_bytes != verified.encoded_bytes || staged_bytes != staged.output_bytes() {
        return Err(VideoPublishError::StagingChanged(
            verified.encoded_bytes,
            staged_bytes,
        ));
    }

    let mark_status = if preserve_mark_of_web {
        match mark_of_web::read_from(source)
            .map_err(|error| VideoPublishError::MarkOfWebReadFailed(error.to_string()))?
        {
            Some(mark) => match mark_of_web::propagate_to_tree(staged.path(), &mark, || {
                cancelled.load(Ordering::Acquire)
            })
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::Interrupted {
                    VideoPublishError::Cancelled
                } else {
                    VideoPublishError::MarkOfWebPropagationFailed(error.to_string())
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
        return Err(VideoPublishError::Cancelled);
    }
    publish_verified_file(staged.path(), final_output, || {
        cancelled.load(Ordering::Acquire)
    })
    .map_err(map_publish_error)?;

    let final_metadata = std::fs::symlink_metadata(final_output)
        .map_err(|error| VideoPublishError::FinalMetadataFailed(error.to_string()))?;
    if !final_metadata.file_type().is_file()
        || final_metadata.file_type().is_symlink()
        || final_metadata.len() != verified.encoded_bytes
    {
        return Err(VideoPublishError::FinalMetadataFailed(
            "published file identity differs from the verified staging file".to_string(),
        ));
    }
    let output_bytes = final_metadata.len();
    let input_bytes = staged.input_bytes();
    let savings_ratio = if input_bytes == 0 {
        0.0
    } else {
        (input_bytes as f64 - output_bytes as f64) / input_bytes as f64
    };

    Ok(PublishedVideoOutput {
        path: final_output.to_path_buf(),
        input_bytes,
        output_bytes,
        savings_ratio,
        mark_of_the_web: mark_status.to_string(),
        verified,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::video_compression_plan::{
        build_video_compression_plan, VideoCompressionPlan, VideoCompressionPlanRequest,
        VideoCompressionPreset,
    };
    use crate::services::video_encoding::encode_video_to_staging;
    use crate::services::video_output_validation::validate_staged_video_output;
    use crate::services::video_probe::probe_video_file;
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn resource(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/video-engine")
            .join(name)
    }

    fn fixture() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/media/videos/h264-vfr-audio-rotation-subtitles.mp4")
    }

    async fn staged_and_verified(
        source: &Path,
        final_output: &Path,
    ) -> (VideoCompressionPlan, StagedVideoOutput, VerifiedVideoOutput) {
        let input = probe_video_file(&resource("ffprobe.exe"), source)
            .await
            .expect("probe input");
        let plan = build_video_compression_plan(
            input,
            &VideoCompressionPlanRequest {
                path: source.to_string_lossy().into_owned(),
                preset: VideoCompressionPreset::Balanced,
                max_width: None,
                max_height: None,
            },
        )
        .expect("plan input");
        let staged = encode_video_to_staging(
            &resource("ffmpeg.exe"),
            &plan,
            final_output,
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .await
        .expect("encode staging");
        let verified = validate_staged_video_output(&resource("ffprobe.exe"), &plan, &staged)
            .await
            .expect("validate staging");
        (plan, staged, verified)
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn validated_real_output_publishes_atomically_with_final_filesystem_facts() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("来源 & publish.mp4");
        std::fs::copy(fixture(), &source).expect("copy source");
        let final_output = directory.path().join("发布 & final.mp4");
        let (_plan, staged, verified) = staged_and_verified(&source, &final_output).await;
        let expected_bytes = verified.encoded_bytes;

        let published = publish_validated_video_output(
            staged,
            verified,
            &source,
            &final_output,
            true,
            &AtomicBool::new(false),
        )
        .expect("publish verified output");

        assert_eq!(published.path, final_output);
        assert_eq!(published.output_bytes, expected_bytes);
        assert_eq!(
            published.output_bytes,
            std::fs::metadata(&published.path).unwrap().len()
        );
        assert_eq!(published.mark_of_the_web, "not-present");
        assert_eq!(published.verified.video_codec, "h264");
        assert!(
            source.exists(),
            "source recycling is a later explicit action"
        );
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn target_race_preserves_existing_bytes_and_cleans_staging() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("source.mp4");
        std::fs::copy(fixture(), &source).expect("copy source");
        let final_output = directory.path().join("race.mp4");
        let (_plan, staged, verified) = staged_and_verified(&source, &final_output).await;
        let staged_path = staged.path().to_path_buf();
        std::fs::write(&final_output, b"existing-user-bytes").expect("create target race");

        let error = publish_validated_video_output(
            staged,
            verified,
            &source,
            &final_output,
            true,
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert!(matches!(error, VideoPublishError::TargetAppeared(_)));
        assert_eq!(
            std::fs::read(&final_output).unwrap(),
            b"existing-user-bytes"
        );
        assert!(!staged_path.exists());
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn cancellation_after_validation_publishes_nothing_and_cleans_staging() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("source.mp4");
        std::fs::copy(fixture(), &source).expect("copy source");
        let final_output = directory.path().join("cancelled.mp4");
        let (_plan, staged, verified) = staged_and_verified(&source, &final_output).await;
        let staged_path = staged.path().to_path_buf();
        let cancelled = AtomicBool::new(true);

        let error = publish_validated_video_output(
            staged,
            verified,
            &source,
            &final_output,
            true,
            &cancelled,
        )
        .unwrap_err();
        assert_eq!(error, VideoPublishError::Cancelled);
        assert!(!final_output.exists());
        assert!(!staged_path.exists());
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn real_mark_of_web_is_preserved_only_after_validation() {
        use std::os::windows::ffi::{OsStrExt, OsStringExt};

        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("downloaded.mp4");
        std::fs::copy(fixture(), &source).expect("copy source");
        let mut source_stream: Vec<u16> = source.as_os_str().encode_wide().collect();
        source_stream.extend(":Zone.Identifier".encode_utf16());
        let zone = b"[ZoneTransfer]\r\nZoneId=3\r\nHostUrl=https://example.test/video.mp4\r\n";
        std::fs::write(
            PathBuf::from(std::ffi::OsString::from_wide(&source_stream)),
            zone,
        )
        .expect("write source zone");
        let final_output = directory.path().join("published.mp4");
        let (_plan, staged, verified) = staged_and_verified(&source, &final_output).await;

        let published = publish_validated_video_output(
            staged,
            verified,
            &source,
            &final_output,
            true,
            &AtomicBool::new(false),
        )
        .expect("publish marked video");
        let mut final_stream: Vec<u16> = final_output.as_os_str().encode_wide().collect();
        final_stream.extend(":Zone.Identifier".encode_utf16());
        assert_eq!(published.mark_of_the_web, "applied");
        assert_eq!(
            std::fs::read(PathBuf::from(std::ffi::OsString::from_wide(&final_stream))).unwrap(),
            zone
        );
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn source_change_after_validation_prevents_publication() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("source-changes.mp4");
        std::fs::copy(fixture(), &source).expect("copy source");
        let final_output = directory.path().join("source-change-must-not-publish.mp4");
        let (_plan, staged, verified) = staged_and_verified(&source, &final_output).await;
        let staged_path = staged.path().to_path_buf();
        std::fs::OpenOptions::new()
            .append(true)
            .open(&source)
            .unwrap()
            .write_all(b"changed-after-validation")
            .unwrap();

        let error = publish_validated_video_output(
            staged,
            verified,
            &source,
            &final_output,
            true,
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert!(matches!(error, VideoPublishError::SourceChanged(_, _)));
        assert!(!final_output.exists());
        assert!(!staged_path.exists());
    }
}
