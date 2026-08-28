use crate::services::video_probe::{VideoFrameRateMode, VideoProbeReport};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAX_OUTPUT_PIXELS: u64 = 3_840 * 2_160;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum VideoCompressionPreset {
    Clear,
    Balanced,
    Small,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoCompressionPlanRequest {
    pub path: String,
    pub preset: VideoCompressionPreset,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoPresetFacts {
    pub preset: VideoCompressionPreset,
    pub label: &'static str,
    pub video_bits_per_pixel_milli: u32,
    pub minimum_video_bit_rate: u64,
    pub maximum_video_bit_rate: u64,
    pub audio_bit_rate: u64,
    pub default_max_width: u32,
    pub default_max_height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoSizeEstimate {
    pub is_estimate: bool,
    pub low_bytes: u64,
    pub high_bytes: u64,
    pub basis: &'static str,
    pub disclaimer: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoCompressionPlan {
    pub probe: VideoProbeReport,
    pub preset: VideoPresetFacts,
    pub effective_max_width: u32,
    pub effective_max_height: u32,
    pub output_width: u32,
    pub output_height: u32,
    pub will_resize: bool,
    pub will_upscale: bool,
    pub aspect_ratio_policy: &'static str,
    pub target_video_bit_rate: u64,
    pub target_audio_bit_rate: Option<u64>,
    pub estimated_output: VideoSizeEstimate,
    pub stream_changes: Vec<String>,
    pub requires_explicit_confirmation: bool,
    pub can_encode: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum VideoCompressionPlanError {
    #[error("VIDEO_PLAN_MAX_RESOLUTION_PAIR_REQUIRED")]
    MaxResolutionPairRequired,
    #[error("VIDEO_PLAN_MAX_RESOLUTION_INVALID: width and height must each be at least 2 pixels")]
    MaxResolutionInvalid,
    #[error("VIDEO_PLAN_MAX_RESOLUTION_TOO_LARGE: maximum output area is 3840x2160 pixels")]
    MaxResolutionTooLarge,
    #[error("VIDEO_PLAN_INPUT_TOO_SMALL: visible width and height must each be at least 2 pixels")]
    InputTooSmall,
}

impl VideoCompressionPreset {
    fn facts(self, portrait: bool) -> VideoPresetFacts {
        let (label, bpp_milli, minimum, maximum, audio, landscape_width, landscape_height) =
            match self {
                Self::Clear => ("clear", 120, 1_500_000, 12_000_000, 192_000, 1_920, 1_080),
                Self::Balanced => ("balanced", 75, 800_000, 8_000_000, 128_000, 1_280, 720),
                Self::Small => ("small", 45, 400_000, 4_000_000, 96_000, 854, 480),
            };
        let (default_max_width, default_max_height) = if portrait {
            (landscape_height, landscape_width)
        } else {
            (landscape_width, landscape_height)
        };
        VideoPresetFacts {
            preset: self,
            label,
            video_bits_per_pixel_milli: bpp_milli,
            minimum_video_bit_rate: minimum,
            maximum_video_bit_rate: maximum,
            audio_bit_rate: audio,
            default_max_width,
            default_max_height,
        }
    }
}

fn effective_maximum(
    request: &VideoCompressionPlanRequest,
    preset: &VideoPresetFacts,
) -> Result<(u32, u32), VideoCompressionPlanError> {
    let dimensions = match (request.max_width, request.max_height) {
        (None, None) => (preset.default_max_width, preset.default_max_height),
        (Some(width), Some(height)) => (width, height),
        _ => return Err(VideoCompressionPlanError::MaxResolutionPairRequired),
    };
    if dimensions.0 < 2 || dimensions.1 < 2 {
        return Err(VideoCompressionPlanError::MaxResolutionInvalid);
    }
    if dimensions.0 > 3_840
        || dimensions.1 > 3_840
        || u64::from(dimensions.0) * u64::from(dimensions.1) > MAX_OUTPUT_PIXELS
    {
        return Err(VideoCompressionPlanError::MaxResolutionTooLarge);
    }
    Ok(dimensions)
}

fn even_floor(value: f64) -> u32 {
    let rounded_down = value.floor() as u32;
    rounded_down - rounded_down % 2
}

fn output_dimensions(
    input_width: u32,
    input_height: u32,
    max_width: u32,
    max_height: u32,
) -> Result<(u32, u32), VideoCompressionPlanError> {
    if input_width < 2 || input_height < 2 {
        return Err(VideoCompressionPlanError::InputTooSmall);
    }
    let scale = 1.0_f64
        .min(f64::from(max_width) / f64::from(input_width))
        .min(f64::from(max_height) / f64::from(input_height));
    let width = even_floor(f64::from(input_width) * scale).max(2);
    let height = even_floor(f64::from(input_height) * scale).max(2);
    Ok((width, height))
}

fn estimate_bytes(duration_ms: u64, total_bit_rate: u64) -> VideoSizeEstimate {
    let nominal_bytes = u128::from(duration_ms).saturating_mul(u128::from(total_bit_rate)) / 8_000;
    let low = nominal_bytes.saturating_mul(80) / 100;
    let high = nominal_bytes.saturating_mul(125) / 100;
    VideoSizeEstimate {
        is_estimate: true,
        low_bytes: u64::try_from(low).unwrap_or(u64::MAX),
        high_bytes: u64::try_from(high).unwrap_or(u64::MAX),
        basis: "duration-output-pixels-average-frame-rate-and-preset-bitrate-envelope",
        disclaimer: "estimate-only; source complexity, VFR timing and encoder behavior can change the final size",
    }
}

pub fn build_video_compression_plan(
    probe: VideoProbeReport,
    request: &VideoCompressionPlanRequest,
) -> Result<VideoCompressionPlan, VideoCompressionPlanError> {
    let input_width = probe.primary_video.visible_width;
    let input_height = probe.primary_video.visible_height;
    let preset = request.preset.facts(input_height > input_width);
    let (effective_max_width, effective_max_height) = effective_maximum(request, &preset)?;
    let (output_width, output_height) = output_dimensions(
        input_width,
        input_height,
        effective_max_width,
        effective_max_height,
    )?;
    let average_fps = probe
        .primary_video
        .average_frame_rate_milli
        .map(|rate| f64::from(rate) / 1_000.0)
        .filter(|rate| rate.is_finite() && *rate > 0.0)
        .unwrap_or(30.0);
    let raw_video_bit_rate = f64::from(output_width)
        * f64::from(output_height)
        * average_fps
        * (f64::from(preset.video_bits_per_pixel_milli) / 1_000.0);
    let target_video_bit_rate = (raw_video_bit_rate.round() as u64)
        .clamp(preset.minimum_video_bit_rate, preset.maximum_video_bit_rate);
    let target_audio_bit_rate = (!probe.audio_streams.is_empty()).then_some(preset.audio_bit_rate);
    let total_bit_rate = target_video_bit_rate.saturating_add(target_audio_bit_rate.unwrap_or(0));

    let mut stream_changes = vec![
        "VIDEO_PLAN_CONTAINER_CHANGE: output will use MP4".to_string(),
        "VIDEO_PLAN_VIDEO_CODEC_CHANGE: primary video will be encoded as H.264".to_string(),
    ];
    if (output_width, output_height) != (input_width, input_height) {
        stream_changes.push(format!(
            "VIDEO_PLAN_RESIZE: visible dimensions will change from {input_width}x{input_height} to {output_width}x{output_height}"
        ));
    }
    if probe.primary_video.rotation_degrees != 0 {
        stream_changes.push(format!(
            "VIDEO_PLAN_ROTATION_NORMALIZED: {} degree metadata will be applied to visible pixels",
            probe.primary_video.rotation_degrees
        ));
    }
    if probe.primary_video.frame_rate_mode == VideoFrameRateMode::Variable {
        stream_changes.push(
            "VIDEO_PLAN_VFR_TIMESTAMPS_PRESERVED: output timing follows input timestamps"
                .to_string(),
        );
    }
    if target_audio_bit_rate.is_some() {
        stream_changes.push(
            "VIDEO_PLAN_PRIMARY_AUDIO_CHANGE: primary audio will be encoded as AAC".to_string(),
        );
    }
    stream_changes.extend(probe.warnings.iter().cloned());
    stream_changes.extend(probe.blocking_reasons.iter().cloned());
    let requires_explicit_confirmation = !probe.warnings.is_empty();
    let can_encode = probe.blocking_reasons.is_empty();

    Ok(VideoCompressionPlan {
        estimated_output: estimate_bytes(probe.duration_ms, total_bit_rate),
        probe,
        preset,
        effective_max_width,
        effective_max_height,
        output_width,
        output_height,
        will_resize: (output_width, output_height) != (input_width, input_height),
        will_upscale: false,
        aspect_ratio_policy: "preserve-within-even-dimension-rounding",
        target_video_bit_rate,
        target_audio_bit_rate,
        stream_changes,
        requires_explicit_confirmation,
        can_encode,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::video_probe::{probe_video_file, VideoFirstReleasePolicy};
    use std::path::{Path, PathBuf};

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/media/videos")
            .join(name)
    }

    fn ffprobe() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/video-engine/ffprobe.exe")
    }

    fn request(path: &Path, preset: VideoCompressionPreset) -> VideoCompressionPlanRequest {
        VideoCompressionPlanRequest {
            path: path.display().to_string(),
            preset,
            max_width: None,
            max_height: None,
        }
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn plans_rotated_vfr_input_without_upscaling_and_labels_estimate() {
        let source = fixture("h264-vfr-audio-rotation-subtitles.mp4");
        let probe = probe_video_file(&ffprobe(), &source)
            .await
            .expect("probe fixture");
        let plan = build_video_compression_plan(
            probe,
            &request(&source, VideoCompressionPreset::Balanced),
        )
        .expect("plan fixture");
        assert_eq!((plan.output_width, plan.output_height), (360, 640));
        assert!(!plan.will_resize);
        assert!(!plan.will_upscale);
        assert_eq!(plan.target_audio_bit_rate, Some(128_000));
        assert!(plan.estimated_output.is_estimate);
        assert!(plan.estimated_output.low_bytes < plan.estimated_output.high_bytes);
        assert!(plan.requires_explicit_confirmation);
        assert!(plan
            .stream_changes
            .iter()
            .any(|change| change.starts_with("VIDEO_PLAN_ROTATION_NORMALIZED:")));
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn no_audio_input_has_no_invented_audio_bitrate() {
        let source = fixture("h265.mp4");
        let probe = probe_video_file(&ffprobe(), &source)
            .await
            .expect("probe fixture");
        let plan =
            build_video_compression_plan(probe, &request(&source, VideoCompressionPreset::Small))
                .expect("plan fixture");
        assert_eq!(plan.target_audio_bit_rate, None);
        assert!(!plan
            .stream_changes
            .iter()
            .any(|change| change.starts_with("VIDEO_PLAN_PRIMARY_AUDIO_CHANGE:")));
    }

    #[test]
    fn custom_maximum_preserves_aspect_and_never_upscales() {
        let probe = sample_report(3_840, 2_160);
        let plan = build_video_compression_plan(
            probe,
            &VideoCompressionPlanRequest {
                path: "4k.mp4".to_string(),
                preset: VideoCompressionPreset::Balanced,
                max_width: Some(1_000),
                max_height: Some(1_000),
            },
        )
        .expect("custom plan");
        assert_eq!((plan.output_width, plan.output_height), (1_000, 562));
        assert!(plan.will_resize);
        assert!(!plan.will_upscale);
    }

    #[test]
    fn refuses_partial_or_oversized_maximums() {
        let probe = sample_report(1_920, 1_080);
        let partial = build_video_compression_plan(
            probe.clone(),
            &VideoCompressionPlanRequest {
                path: "video.mp4".to_string(),
                preset: VideoCompressionPreset::Clear,
                max_width: Some(1_280),
                max_height: None,
            },
        );
        assert_eq!(
            partial.unwrap_err(),
            VideoCompressionPlanError::MaxResolutionPairRequired
        );
        let oversized = build_video_compression_plan(
            probe,
            &VideoCompressionPlanRequest {
                path: "video.mp4".to_string(),
                preset: VideoCompressionPreset::Clear,
                max_width: Some(3_840),
                max_height: Some(3_840),
            },
        );
        assert_eq!(
            oversized.unwrap_err(),
            VideoCompressionPlanError::MaxResolutionTooLarge
        );
    }

    #[test]
    fn presets_are_ordered_and_portrait_defaults_are_rotated() {
        let clear = VideoCompressionPreset::Clear.facts(true);
        let balanced = VideoCompressionPreset::Balanced.facts(true);
        let small = VideoCompressionPreset::Small.facts(true);
        assert!(clear.video_bits_per_pixel_milli > balanced.video_bits_per_pixel_milli);
        assert!(balanced.video_bits_per_pixel_milli > small.video_bits_per_pixel_milli);
        assert!(clear.audio_bit_rate > balanced.audio_bit_rate);
        assert!(balanced.audio_bit_rate > small.audio_bit_rate);
        assert_eq!(
            (clear.default_max_width, clear.default_max_height),
            (1_080, 1_920)
        );
        assert_eq!(
            (balanced.default_max_width, balanced.default_max_height),
            (720, 1_280)
        );
        assert_eq!(
            (small.default_max_width, small.default_max_height),
            (480, 854)
        );
    }

    #[test]
    fn tauri_json_contract_uses_camel_case_and_estimate_marker() {
        let request: VideoCompressionPlanRequest = serde_json::from_value(serde_json::json!({
            "path": "video.mp4",
            "preset": "balanced",
            "maxWidth": null,
            "maxHeight": null
        }))
        .expect("frontend request must deserialize");
        let plan = build_video_compression_plan(sample_report(1_920, 1_080), &request)
            .expect("serialize plan");
        let json = serde_json::to_value(plan).expect("plan JSON");
        assert_eq!(json["preset"]["preset"], "balanced");
        assert_eq!(json["effectiveMaxWidth"], 1_280);
        assert_eq!(json["estimatedOutput"]["isEstimate"], true);
        assert_eq!(json["willUpscale"], false);
        assert!(json.get("effective_max_width").is_none());
    }

    fn sample_report(width: u32, height: u32) -> VideoProbeReport {
        use crate::services::video_probe::VideoStreamFacts;
        VideoProbeReport {
            source: PathBuf::from("video.mp4"),
            input_bytes: 100_000_000,
            container: Some("mov,mp4".to_string()),
            duration_ms: 60_000,
            overall_bit_rate: Some(8_000_000),
            primary_video: VideoStreamFacts {
                index: 0,
                codec: Some("h264".to_string()),
                profile: None,
                encoded_width: width,
                encoded_height: height,
                visible_width: width,
                visible_height: height,
                rotation_degrees: 0,
                pixel_format: Some("yuv420p".to_string()),
                color_transfer: None,
                hdr: false,
                nominal_frame_rate: Some("30/1".to_string()),
                average_frame_rate: Some("30/1".to_string()),
                average_frame_rate_milli: Some(30_000),
                frame_rate_mode: VideoFrameRateMode::ConstantOrUndetermined,
                bit_rate: Some(8_000_000),
                default: true,
            },
            video_stream_count: 1,
            audio_streams: Vec::new(),
            subtitle_streams: Vec::new(),
            chapter_count: 0,
            attached_picture_count: 0,
            policy: VideoFirstReleasePolicy::default(),
            warnings: Vec::new(),
            blocking_reasons: Vec::new(),
        }
    }
}
