use crate::services::video_compression_plan::VideoCompressionPlan;
use crate::services::video_encoding::StagedVideoOutput;
use crate::services::video_probe::probe_video_file;
use crate::utils::process;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use thiserror::Error;

const FRAME_SCAN_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_FRAME_SCAN_OUTPUT_BYTES: usize = 64 * 1024;
const MAX_ERROR_DETAIL_CHARS: usize = 2_048;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum VideoOutputValidationError {
    #[error("VIDEO_OUTPUT_NOT_REGULAR_FILE")]
    NotRegularFile,
    #[error("VIDEO_OUTPUT_EMPTY")]
    Empty,
    #[error("VIDEO_OUTPUT_SIZE_CHANGED: expected={0} actual={1}")]
    SizeChanged(u64, u64),
    #[error("VIDEO_OUTPUT_PROBE_FAILED: {0}")]
    ProbeFailed(String),
    #[error("VIDEO_OUTPUT_CONTAINER_MISMATCH: {0}")]
    ContainerMismatch(String),
    #[error("VIDEO_OUTPUT_VIDEO_CODEC_MISMATCH: {0}")]
    VideoCodecMismatch(String),
    #[error("VIDEO_OUTPUT_VIDEO_STREAM_COUNT_MISMATCH: {0}")]
    VideoStreamCountMismatch(u32),
    #[error("VIDEO_OUTPUT_DIMENSIONS_MISMATCH: expected={0}x{1} actual={2}x{3}")]
    DimensionsMismatch(u32, u32, u32, u32),
    #[error("VIDEO_OUTPUT_ENCODED_DIMENSIONS_MISMATCH: expected={0}x{1} actual={2}x{3}")]
    EncodedDimensionsMismatch(u32, u32, u32, u32),
    #[error("VIDEO_OUTPUT_ROTATION_NOT_NORMALIZED: {0}")]
    RotationNotNormalized(i32),
    #[error("VIDEO_OUTPUT_AUDIO_MISSING")]
    AudioMissing,
    #[error("VIDEO_OUTPUT_AUDIO_UNEXPECTED")]
    AudioUnexpected,
    #[error("VIDEO_OUTPUT_AUDIO_CODEC_MISMATCH: {0}")]
    AudioCodecMismatch(String),
    #[error("VIDEO_OUTPUT_AUDIO_STREAM_COUNT_MISMATCH: {0}")]
    AudioStreamCountMismatch(usize),
    #[error("VIDEO_OUTPUT_LOSSY_STREAMS_REMAIN: subtitles={0} chapters={1} attachments={2}")]
    LossyStreamsRemain(usize, u32, u32),
    #[error("VIDEO_OUTPUT_DURATION_MISMATCH: expected={0}ms actual={1}ms tolerance={2}ms")]
    DurationMismatch(u64, u64, u64),
    #[error("VIDEO_OUTPUT_FRAME_SCAN_TIMEOUT")]
    FrameScanTimeout,
    #[error("VIDEO_OUTPUT_FRAME_SCAN_FAILED: {0}")]
    FrameScanFailed(String),
    #[error("VIDEO_OUTPUT_FRAME_SCAN_TOO_LARGE")]
    FrameScanTooLarge,
    #[error("VIDEO_OUTPUT_NO_DECODABLE_VIDEO_FRAMES")]
    NoDecodableVideoFrames,
    #[error("VIDEO_OUTPUT_DECODED_FRAME_COUNT_TOO_LOW: actual={0} minimum={1}")]
    DecodedFrameCountTooLow(u64, u64),
    #[error("VIDEO_OUTPUT_DECODED_AUDIO_FRAME_COUNT_TOO_LOW: actual={0} minimum={1}")]
    DecodedAudioFrameCountTooLow(u64, u64),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedVideoOutput {
    pub encoded_bytes: u64,
    pub container: String,
    pub duration_ms: u64,
    pub duration_difference_ms: u64,
    pub duration_tolerance_ms: u64,
    pub video_codec: String,
    pub audio_codec: Option<String>,
    pub encoded_width: u32,
    pub encoded_height: u32,
    pub visible_width: u32,
    pub visible_height: u32,
    pub rotation_degrees: i32,
    pub decoded_video_frames: u64,
}

fn duration_tolerance_ms(duration_ms: u64) -> u64 {
    duration_ms.div_ceil(100).clamp(250, 2_000)
}

fn minimum_decoded_frames(plan: &VideoCompressionPlan) -> u64 {
    let Some(frame_rate_milli) = plan.probe.primary_video.average_frame_rate_milli else {
        return 1;
    };
    let expected =
        u128::from(plan.probe.duration_ms).saturating_mul(u128::from(frame_rate_milli)) / 1_000_000;
    let minimum = expected.saturating_mul(80) / 100;
    u64::try_from(minimum).unwrap_or(u64::MAX).max(1)
}

fn bounded_error_detail(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .trim()
        .chars()
        .take(MAX_ERROR_DETAIL_CHARS)
        .collect()
}

struct DecodedFrameCounts {
    video: u64,
    audio: Option<u64>,
}

async fn count_decodable_frames(
    ffprobe: &Path,
    output: &Path,
) -> Result<DecodedFrameCounts, VideoOutputValidationError> {
    let mut command = process::async_command(ffprobe);
    command
        .args([
            "-v",
            "error",
            "-count_frames",
            "-show_entries",
            "stream=codec_type,nb_read_frames",
            "-of",
            "json",
            "-i",
        ])
        .arg(output)
        .stdin(Stdio::null())
        .kill_on_drop(true);
    let result = tokio::time::timeout(FRAME_SCAN_TIMEOUT, command.output())
        .await
        .map_err(|_| VideoOutputValidationError::FrameScanTimeout)?
        .map_err(|error| VideoOutputValidationError::FrameScanFailed(error.to_string()))?;
    if !result.status.success() {
        return Err(VideoOutputValidationError::FrameScanFailed(
            bounded_error_detail(&result.stderr),
        ));
    }
    if result.stdout.len() > MAX_FRAME_SCAN_OUTPUT_BYTES {
        return Err(VideoOutputValidationError::FrameScanTooLarge);
    }
    let root: Value = serde_json::from_slice(&result.stdout)
        .map_err(|error| VideoOutputValidationError::FrameScanFailed(error.to_string()))?;
    let streams = root
        .get("streams")
        .and_then(Value::as_array)
        .ok_or(VideoOutputValidationError::NoDecodableVideoFrames)?;
    let frames_for = |kind: &str| {
        streams
            .iter()
            .find(|stream| stream.get("codec_type").and_then(Value::as_str) == Some(kind))
            .and_then(|stream| stream.get("nb_read_frames"))
            .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))
    };
    let video = frames_for("video")
        .filter(|frames| *frames > 0)
        .ok_or(VideoOutputValidationError::NoDecodableVideoFrames)?;
    Ok(DecodedFrameCounts {
        video,
        audio: frames_for("audio"),
    })
}

pub async fn validate_staged_video_output(
    ffprobe: &Path,
    plan: &VideoCompressionPlan,
    staged: &StagedVideoOutput,
) -> Result<VerifiedVideoOutput, VideoOutputValidationError> {
    let metadata = std::fs::symlink_metadata(staged.path())
        .map_err(|_| VideoOutputValidationError::NotRegularFile)?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(VideoOutputValidationError::NotRegularFile);
    }
    if metadata.len() == 0 {
        return Err(VideoOutputValidationError::Empty);
    }
    if metadata.len() != staged.output_bytes() {
        return Err(VideoOutputValidationError::SizeChanged(
            staged.output_bytes(),
            metadata.len(),
        ));
    }
    let report = probe_video_file(ffprobe, staged.path())
        .await
        .map_err(|error| VideoOutputValidationError::ProbeFailed(error.to_string()))?;

    let container = report.container.clone().unwrap_or_default();
    if !container.split(',').any(|value| value == "mp4") {
        return Err(VideoOutputValidationError::ContainerMismatch(container));
    }
    let video_codec = report.primary_video.codec.clone().unwrap_or_default();
    if video_codec != "h264" {
        return Err(VideoOutputValidationError::VideoCodecMismatch(video_codec));
    }
    if report.video_stream_count != 1 {
        return Err(VideoOutputValidationError::VideoStreamCountMismatch(
            report.video_stream_count,
        ));
    }
    let actual_dimensions = (
        report.primary_video.visible_width,
        report.primary_video.visible_height,
    );
    if actual_dimensions != (plan.output_width, plan.output_height) {
        return Err(VideoOutputValidationError::DimensionsMismatch(
            plan.output_width,
            plan.output_height,
            actual_dimensions.0,
            actual_dimensions.1,
        ));
    }
    let encoded_dimensions = (
        report.primary_video.encoded_width,
        report.primary_video.encoded_height,
    );
    if encoded_dimensions != (plan.output_width, plan.output_height) {
        return Err(VideoOutputValidationError::EncodedDimensionsMismatch(
            plan.output_width,
            plan.output_height,
            encoded_dimensions.0,
            encoded_dimensions.1,
        ));
    }
    if report.primary_video.rotation_degrees != 0 {
        return Err(VideoOutputValidationError::RotationNotNormalized(
            report.primary_video.rotation_degrees,
        ));
    }

    let expected_audio = plan.target_audio_bit_rate.is_some();
    let audio_codec = match (expected_audio, report.audio_streams.as_slice()) {
        (true, []) => return Err(VideoOutputValidationError::AudioMissing),
        (false, []) => None,
        (false, _) => return Err(VideoOutputValidationError::AudioUnexpected),
        (true, [audio]) => {
            let codec = audio.codec.clone().unwrap_or_default();
            if codec != "aac" {
                return Err(VideoOutputValidationError::AudioCodecMismatch(codec));
            }
            Some(codec)
        }
        (true, streams) => {
            return Err(VideoOutputValidationError::AudioStreamCountMismatch(
                streams.len(),
            ))
        }
    };
    if !report.subtitle_streams.is_empty()
        || report.chapter_count != 0
        || report.attached_picture_count != 0
    {
        return Err(VideoOutputValidationError::LossyStreamsRemain(
            report.subtitle_streams.len(),
            report.chapter_count,
            report.attached_picture_count,
        ));
    }

    let tolerance = duration_tolerance_ms(plan.probe.duration_ms);
    let difference = plan.probe.duration_ms.abs_diff(report.duration_ms);
    if difference > tolerance {
        return Err(VideoOutputValidationError::DurationMismatch(
            plan.probe.duration_ms,
            report.duration_ms,
            tolerance,
        ));
    }
    let decoded_frames = count_decodable_frames(ffprobe, staged.path()).await?;
    let minimum_frames = minimum_decoded_frames(plan);
    if decoded_frames.video < minimum_frames {
        return Err(VideoOutputValidationError::DecodedFrameCountTooLow(
            decoded_frames.video,
            minimum_frames,
        ));
    }
    if expected_audio {
        let sample_rate = report.audio_streams[0].sample_rate.unwrap_or(44_100);
        let expected_audio_frames =
            u128::from(report.duration_ms).saturating_mul(u128::from(sample_rate)) / 1_024_000;
        let minimum_audio_frames = u64::try_from(expected_audio_frames.saturating_mul(80) / 100)
            .unwrap_or(u64::MAX)
            .max(1);
        let actual_audio_frames = decoded_frames.audio.unwrap_or(0);
        if actual_audio_frames < minimum_audio_frames {
            return Err(VideoOutputValidationError::DecodedAudioFrameCountTooLow(
                actual_audio_frames,
                minimum_audio_frames,
            ));
        }
    }

    Ok(VerifiedVideoOutput {
        encoded_bytes: metadata.len(),
        container: "mp4".to_string(),
        duration_ms: report.duration_ms,
        duration_difference_ms: difference,
        duration_tolerance_ms: tolerance,
        video_codec,
        audio_codec,
        encoded_width: report.primary_video.encoded_width,
        encoded_height: report.primary_video.encoded_height,
        visible_width: report.primary_video.visible_width,
        visible_height: report.primary_video.visible_height,
        rotation_degrees: report.primary_video.rotation_degrees,
        decoded_video_frames: decoded_frames.video,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::video_compression_plan::{
        build_video_compression_plan, VideoCompressionPlanRequest, VideoCompressionPreset,
    };
    use crate::services::video_encoding::encode_video_to_staging;
    use crate::services::video_probe::probe_video_file;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    fn resource(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/video-engine")
            .join(name)
    }

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/media/videos")
            .join(name)
    }

    async fn encoded_staging(
        source: &Path,
        final_output: &Path,
    ) -> (VideoCompressionPlan, StagedVideoOutput) {
        let report = probe_video_file(&resource("ffprobe.exe"), source)
            .await
            .expect("probe input");
        let plan = build_video_compression_plan(
            report,
            &VideoCompressionPlanRequest {
                path: source.to_string_lossy().into_owned(),
                preset: VideoCompressionPreset::Balanced,
                quality: 76,
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
        .expect("encode input");
        (plan, staged)
    }

    #[test]
    fn duration_threshold_is_bounded_and_explainable() {
        assert_eq!(duration_tolerance_ms(1_000), 250);
        assert_eq!(duration_tolerance_ms(60_000), 600);
        assert_eq!(duration_tolerance_ms(600_000), 2_000);
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn real_product_output_passes_container_stream_duration_and_frame_scan() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let final_output = directory.path().join("验证后仍未发布 & output.mp4");
        let (plan, staged) = encoded_staging(
            &fixture("h264-vfr-audio-rotation-subtitles.mp4"),
            &final_output,
        )
        .await;
        let verified = validate_staged_video_output(&resource("ffprobe.exe"), &plan, &staged)
            .await
            .expect("validate real output");

        assert_eq!(verified.container, "mp4");
        assert_eq!(verified.video_codec, "h264");
        assert_eq!(verified.audio_codec.as_deref(), Some("aac"));
        assert_eq!(
            (verified.visible_width, verified.visible_height),
            (plan.output_width, plan.output_height)
        );
        assert_eq!(verified.rotation_degrees, 0);
        assert!(verified.decoded_video_frames > 0);
        assert!(!final_output.exists());
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn real_no_audio_output_is_verified_without_inventing_audio() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let final_output = directory.path().join("no-audio.mp4");
        let (plan, staged) = encoded_staging(&fixture("h265.mp4"), &final_output).await;
        let verified = validate_staged_video_output(&resource("ffprobe.exe"), &plan, &staged)
            .await
            .expect("validate no-audio output");
        assert_eq!(verified.audio_codec, None);
        assert!(!final_output.exists());
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn truncated_real_output_is_rejected_and_never_published() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let final_output = directory.path().join("must-not-exist.mp4");
        let (plan, staged) = encoded_staging(
            &fixture("h264-vfr-audio-rotation-subtitles.mp4"),
            &final_output,
        )
        .await;
        let length = std::fs::metadata(staged.path()).unwrap().len();
        std::fs::OpenOptions::new()
            .write(true)
            .open(staged.path())
            .unwrap()
            .set_len(length / 2)
            .unwrap();

        let error = validate_staged_video_output(&resource("ffprobe.exe"), &plan, &staged)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            VideoOutputValidationError::SizeChanged(_, _)
                | VideoOutputValidationError::ProbeFailed(_)
                | VideoOutputValidationError::FrameScanFailed(_)
                | VideoOutputValidationError::NoDecodableVideoFrames
                | VideoOutputValidationError::DecodedFrameCountTooLow(_, _)
                | VideoOutputValidationError::DecodedAudioFrameCountTooLow(_, _)
                | VideoOutputValidationError::DurationMismatch(_, _, _)
        ));
        assert!(!final_output.exists());
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn zeroed_output_after_encoding_is_rejected_and_cleaned_on_drop() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let final_output = directory.path().join("zero-must-not-exist.mp4");
        let (plan, staged) = encoded_staging(&fixture("h265.mp4"), &final_output).await;
        let staged_path = staged.path().to_path_buf();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&staged_path)
            .unwrap()
            .set_len(0)
            .unwrap();

        let error = validate_staged_video_output(&resource("ffprobe.exe"), &plan, &staged)
            .await
            .unwrap_err();
        assert_eq!(error, VideoOutputValidationError::Empty);
        assert!(!final_output.exists());
        drop(staged);
        assert!(!staged_path.exists());
    }
}
