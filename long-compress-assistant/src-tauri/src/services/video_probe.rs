use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use thiserror::Error;
use tokio::process::Command;

const PROBE_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_PROBE_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_ERROR_DETAIL_CHARS: usize = 2_048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum VideoFrameRateMode {
    Variable,
    ConstantOrUndetermined,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoStreamFacts {
    pub index: u32,
    pub codec: Option<String>,
    pub profile: Option<String>,
    pub encoded_width: u32,
    pub encoded_height: u32,
    pub visible_width: u32,
    pub visible_height: u32,
    pub rotation_degrees: i32,
    pub pixel_format: Option<String>,
    pub color_transfer: Option<String>,
    pub hdr: bool,
    pub nominal_frame_rate: Option<String>,
    pub average_frame_rate: Option<String>,
    pub average_frame_rate_milli: Option<u32>,
    pub frame_rate_mode: VideoFrameRateMode,
    pub bit_rate: Option<u64>,
    pub default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioStreamFacts {
    pub index: u32,
    pub codec: Option<String>,
    pub channels: Option<u32>,
    pub sample_rate: Option<u32>,
    pub bit_rate: Option<u64>,
    pub language: Option<String>,
    pub default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleStreamFacts {
    pub index: u32,
    pub codec: Option<String>,
    pub language: Option<String>,
    pub default: bool,
    pub forced: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFirstReleasePolicy {
    pub container: &'static str,
    pub video: &'static str,
    pub audio: &'static str,
    pub additional_audio: &'static str,
    pub subtitles: &'static str,
    pub chapters: &'static str,
    pub attached_pictures: &'static str,
    pub rotation: &'static str,
    pub variable_frame_rate: &'static str,
    pub hdr: &'static str,
}

impl Default for VideoFirstReleasePolicy {
    fn default() -> Self {
        Self {
            container: "output-mp4",
            video: "transcode-h264-mf-software",
            audio: "preserve-primary-as-aac-when-present",
            additional_audio: "drop-with-explicit-warning",
            subtitles: "drop-with-explicit-warning",
            chapters: "drop-with-explicit-warning",
            attached_pictures: "drop-with-explicit-warning",
            rotation: "normalize-to-visible-pixel-orientation",
            variable_frame_rate: "preserve-input-timestamps",
            hdr: "refuse-before-encoding",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoProbeReport {
    pub source: PathBuf,
    pub input_bytes: u64,
    pub container: Option<String>,
    pub duration_ms: u64,
    pub overall_bit_rate: Option<u64>,
    pub primary_video: VideoStreamFacts,
    pub video_stream_count: u32,
    pub audio_streams: Vec<AudioStreamFacts>,
    pub subtitle_streams: Vec<SubtitleStreamFacts>,
    pub chapter_count: u32,
    pub attached_picture_count: u32,
    pub policy: VideoFirstReleasePolicy,
    pub warnings: Vec<String>,
    pub blocking_reasons: Vec<String>,
}

#[derive(Debug, Error)]
pub enum VideoProbeError {
    #[error("VIDEO_PROBE_SOURCE_MISSING: {0}")]
    SourceMissing(String),
    #[error("VIDEO_PROBE_SOURCE_NOT_FILE: {0}")]
    SourceNotFile(String),
    #[error("VIDEO_PROBE_SOURCE_EMPTY: {0}")]
    SourceEmpty(String),
    #[error("VIDEO_PROBE_LAUNCH_FAILED: {0}")]
    LaunchFailed(String),
    #[error("VIDEO_PROBE_TIMEOUT: exceeded 20 seconds")]
    Timeout,
    #[error("VIDEO_PROBE_PROCESS_FAILED: {0}")]
    ProcessFailed(String),
    #[error("VIDEO_PROBE_OUTPUT_TOO_LARGE: metadata exceeded 8 MiB")]
    OutputTooLarge,
    #[error("VIDEO_PROBE_INVALID_JSON: {0}")]
    InvalidJson(String),
    #[error("VIDEO_PROBE_NO_VIDEO_STREAM")]
    NoVideoStream,
    #[error("VIDEO_PROBE_INVALID_VIDEO_DIMENSIONS")]
    InvalidVideoDimensions,
    #[error("VIDEO_PROBE_INVALID_DURATION")]
    InvalidDuration,
}

fn string(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(str::to_owned)
}

fn u32_value(value: &Value, key: &str) -> Option<u32> {
    let value = value.get(key)?;
    value
        .as_u64()
        .and_then(|number| u32::try_from(number).ok())
        .or_else(|| value.as_str()?.parse().ok())
}

fn u64_string(value: &Value, key: &str) -> Option<u64> {
    let value = value.get(key)?;
    value.as_u64().or_else(|| value.as_str()?.parse().ok())
}

fn disposition(value: &Value, key: &str) -> bool {
    value
        .get("disposition")
        .and_then(|item| item.get(key))
        .and_then(Value::as_u64)
        == Some(1)
}

fn language(value: &Value) -> Option<String> {
    value
        .get("tags")
        .and_then(|tags| tags.get("language"))
        .and_then(Value::as_str)
        .filter(|language| !language.eq_ignore_ascii_case("und"))
        .map(str::to_owned)
}

fn parse_rate(value: Option<&str>) -> Option<f64> {
    let value = value?;
    let (numerator, denominator) = value.split_once('/')?;
    let numerator: f64 = numerator.parse().ok()?;
    let denominator: f64 = denominator.parse().ok()?;
    (denominator != 0.0).then_some(numerator / denominator)
}

fn rotation(value: &Value) -> i32 {
    let raw = value
        .get("side_data_list")
        .and_then(Value::as_array)
        .and_then(|items| items.iter().find_map(|item| item.get("rotation")))
        .and_then(|item| item.as_i64().or_else(|| item.as_str()?.parse().ok()))
        .or_else(|| {
            value
                .get("tags")
                .and_then(|tags| tags.get("rotate"))
                .and_then(|item| item.as_i64().or_else(|| item.as_str()?.parse().ok()))
        })
        .unwrap_or(0);
    i32::try_from(raw.rem_euclid(360)).unwrap_or(0)
}

fn parse_duration_ms(root: &Value) -> Result<u64, VideoProbeError> {
    let seconds = root
        .get("format")
        .and_then(|format| format.get("duration"))
        .and_then(|value| {
            value
                .as_str()
                .and_then(|text| text.parse::<f64>().ok())
                .or_else(|| value.as_f64())
        })
        .filter(|seconds| seconds.is_finite() && *seconds > 0.0)
        .ok_or(VideoProbeError::InvalidDuration)?;
    Ok((seconds * 1_000.0).round() as u64)
}

fn parse_probe_report(
    source: &Path,
    input_bytes: u64,
    root: Value,
) -> Result<VideoProbeReport, VideoProbeError> {
    let streams = root
        .get("streams")
        .and_then(Value::as_array)
        .ok_or(VideoProbeError::NoVideoStream)?;
    let video_streams: Vec<&Value> = streams
        .iter()
        .filter(|stream| {
            stream.get("codec_type").and_then(Value::as_str) == Some("video")
                && !disposition(stream, "attached_pic")
        })
        .collect();
    let primary = video_streams
        .iter()
        .copied()
        .find(|stream| disposition(stream, "default"))
        .or_else(|| video_streams.first().copied())
        .ok_or(VideoProbeError::NoVideoStream)?;

    let encoded_width = u32_value(primary, "width")
        .filter(|value| *value > 0)
        .ok_or(VideoProbeError::InvalidVideoDimensions)?;
    let encoded_height = u32_value(primary, "height")
        .filter(|value| *value > 0)
        .ok_or(VideoProbeError::InvalidVideoDimensions)?;
    let rotation_degrees = rotation(primary);
    let (visible_width, visible_height) = if matches!(rotation_degrees, 90 | 270) {
        (encoded_height, encoded_width)
    } else {
        (encoded_width, encoded_height)
    };
    let nominal_frame_rate = string(primary, "r_frame_rate");
    let average_frame_rate = string(primary, "avg_frame_rate");
    let nominal_rate = parse_rate(nominal_frame_rate.as_deref());
    let average_rate = parse_rate(average_frame_rate.as_deref());
    let frame_rate_mode = match (nominal_rate, average_rate) {
        (Some(nominal), Some(average)) if (nominal - average).abs() > 0.001 => {
            VideoFrameRateMode::Variable
        }
        _ => VideoFrameRateMode::ConstantOrUndetermined,
    };
    let color_transfer = string(primary, "color_transfer");
    let hdr = matches!(
        color_transfer.as_deref(),
        Some("smpte2084" | "arib-std-b67")
    );
    let average_frame_rate_milli = average_rate
        .filter(|rate| rate.is_finite() && *rate >= 0.0)
        .map(|rate| (rate * 1_000.0).round() as u32);

    let audio_streams = streams
        .iter()
        .filter(|stream| stream.get("codec_type").and_then(Value::as_str) == Some("audio"))
        .map(|stream| AudioStreamFacts {
            index: u32_value(stream, "index").unwrap_or(0),
            codec: string(stream, "codec_name"),
            channels: u32_value(stream, "channels"),
            sample_rate: u32_value(stream, "sample_rate"),
            bit_rate: u64_string(stream, "bit_rate"),
            language: language(stream),
            default: disposition(stream, "default"),
        })
        .collect::<Vec<_>>();
    let subtitle_streams = streams
        .iter()
        .filter(|stream| stream.get("codec_type").and_then(Value::as_str) == Some("subtitle"))
        .map(|stream| SubtitleStreamFacts {
            index: u32_value(stream, "index").unwrap_or(0),
            codec: string(stream, "codec_name"),
            language: language(stream),
            default: disposition(stream, "default"),
            forced: disposition(stream, "forced"),
        })
        .collect::<Vec<_>>();
    let attached_picture_count = streams
        .iter()
        .filter(|stream| {
            stream.get("codec_type").and_then(Value::as_str) == Some("video")
                && disposition(stream, "attached_pic")
        })
        .count();
    let chapter_count = root
        .get("chapters")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);

    let mut warnings = Vec::new();
    if video_streams.len() > 1 {
        warnings.push("VIDEO_PROBE_MULTIPLE_VIDEO_STREAMS: only the default or first video stream will be encoded".to_string());
    }
    if audio_streams.len() > 1 {
        warnings.push("VIDEO_PROBE_ADDITIONAL_AUDIO_WILL_BE_DROPPED: only the default or first audio stream will be converted to AAC".to_string());
    }
    if !subtitle_streams.is_empty() {
        warnings.push("VIDEO_PROBE_SUBTITLES_WILL_BE_DROPPED: explicit confirmation is required before encoding".to_string());
    }
    if chapter_count > 0 {
        warnings.push("VIDEO_PROBE_CHAPTERS_WILL_BE_DROPPED: explicit confirmation is required before encoding".to_string());
    }
    if attached_picture_count > 0 {
        warnings.push("VIDEO_PROBE_ATTACHED_PICTURES_WILL_BE_DROPPED: explicit confirmation is required before encoding".to_string());
    }
    let blocking_reasons = if hdr {
        vec!["VIDEO_PROBE_HDR_UNSUPPORTED: the first release has no audited HDR preservation or tone-mapping policy".to_string()]
    } else {
        Vec::new()
    };

    Ok(VideoProbeReport {
        source: source.to_path_buf(),
        input_bytes,
        container: root
            .get("format")
            .and_then(|format| string(format, "format_name")),
        duration_ms: parse_duration_ms(&root)?,
        overall_bit_rate: root
            .get("format")
            .and_then(|format| u64_string(format, "bit_rate")),
        primary_video: VideoStreamFacts {
            index: u32_value(primary, "index").unwrap_or(0),
            codec: string(primary, "codec_name"),
            profile: string(primary, "profile"),
            encoded_width,
            encoded_height,
            visible_width,
            visible_height,
            rotation_degrees,
            pixel_format: string(primary, "pix_fmt"),
            color_transfer,
            hdr,
            nominal_frame_rate,
            average_frame_rate,
            average_frame_rate_milli,
            frame_rate_mode,
            bit_rate: u64_string(primary, "bit_rate"),
            default: disposition(primary, "default"),
        },
        video_stream_count: u32::try_from(video_streams.len()).unwrap_or(u32::MAX),
        audio_streams,
        subtitle_streams,
        chapter_count: u32::try_from(chapter_count).unwrap_or(u32::MAX),
        attached_picture_count: u32::try_from(attached_picture_count).unwrap_or(u32::MAX),
        policy: VideoFirstReleasePolicy::default(),
        warnings,
        blocking_reasons,
    })
}

fn bounded_error_detail(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .trim()
        .chars()
        .take(MAX_ERROR_DETAIL_CHARS)
        .collect()
}

pub async fn probe_video_file(
    ffprobe: &Path,
    source: &Path,
) -> Result<VideoProbeReport, VideoProbeError> {
    let metadata = std::fs::metadata(source)
        .map_err(|_| VideoProbeError::SourceMissing(source.display().to_string()))?;
    if !metadata.is_file() {
        return Err(VideoProbeError::SourceNotFile(source.display().to_string()));
    }
    if metadata.len() == 0 {
        return Err(VideoProbeError::SourceEmpty(source.display().to_string()));
    }

    let mut command = Command::new(ffprobe);
    command
        .args([
            "-v",
            "error",
            "-show_format",
            "-show_streams",
            "-show_chapters",
            "-of",
            "json",
            "-i",
        ])
        .arg(source)
        .stdin(Stdio::null())
        .kill_on_drop(true);
    let output = tokio::time::timeout(PROBE_TIMEOUT, command.output())
        .await
        .map_err(|_| VideoProbeError::Timeout)?
        .map_err(|error| VideoProbeError::LaunchFailed(error.to_string()))?;
    if !output.status.success() {
        return Err(VideoProbeError::ProcessFailed(bounded_error_detail(
            &output.stderr,
        )));
    }
    if output.stdout.len() > MAX_PROBE_OUTPUT_BYTES {
        return Err(VideoProbeError::OutputTooLarge);
    }
    let root: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| VideoProbeError::InvalidJson(error.to_string()))?;
    parse_probe_report(source, metadata.len(), root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resource_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn ffprobe() -> PathBuf {
        resource_root().join("resources/video-engine/ffprobe.exe")
    }

    fn fixture(name: &str) -> PathBuf {
        resource_root()
            .join("../tests/fixtures/media/videos")
            .join(name)
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn probes_vfr_rotation_audio_and_subtitle_from_frozen_input() {
        let report = probe_video_file(
            &ffprobe(),
            &fixture("h264-vfr-audio-rotation-subtitles.mp4"),
        )
        .await
        .expect("frozen H.264 fixture must probe");
        assert_eq!(report.input_bytes, 22_769);
        assert_eq!(report.duration_ms, 1_000);
        assert_eq!(report.primary_video.codec.as_deref(), Some("h264"));
        assert_eq!(
            (
                report.primary_video.visible_width,
                report.primary_video.visible_height
            ),
            (360, 640)
        );
        assert_eq!(report.primary_video.rotation_degrees, 90);
        assert_eq!(
            report.primary_video.frame_rate_mode,
            VideoFrameRateMode::Variable
        );
        assert_eq!(report.audio_streams.len(), 1);
        assert_eq!(report.audio_streams[0].codec.as_deref(), Some("aac"));
        assert_eq!(report.subtitle_streams.len(), 1);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.starts_with("VIDEO_PROBE_SUBTITLES_WILL_BE_DROPPED")));
        assert!(report.blocking_reasons.is_empty());
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn probes_no_audio_input_without_inventing_an_audio_stream() {
        let report = probe_video_file(&ffprobe(), &fixture("h265.mp4"))
            .await
            .expect("frozen HEVC fixture must probe");
        assert_eq!(report.input_bytes, 61_329);
        assert_eq!(report.primary_video.codec.as_deref(), Some("hevc"));
        assert_eq!(
            report.primary_video.frame_rate_mode,
            VideoFrameRateMode::ConstantOrUndetermined
        );
        assert!(report.audio_streams.is_empty());
        assert!(report.subtitle_streams.is_empty());
    }

    #[tokio::test]
    async fn refuses_empty_input_before_launching_a_process() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("empty.mp4");
        std::fs::File::create(&source).expect("empty source");
        let error = probe_video_file(Path::new("missing-ffprobe"), &source)
            .await
            .unwrap_err();
        assert!(error.to_string().starts_with("VIDEO_PROBE_SOURCE_EMPTY:"));
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn classifies_corrupt_input_with_a_stable_error_prefix() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("corrupt.mp4");
        std::fs::write(&source, b"not a media container").expect("corrupt source");
        let error = probe_video_file(&ffprobe(), &source).await.unwrap_err();
        assert!(error.to_string().starts_with("VIDEO_PROBE_PROCESS_FAILED:"));
    }

    #[test]
    fn classifies_hdr_and_all_lossy_stream_changes_before_encoding() {
        let root = serde_json::json!({
            "streams": [
                { "index": 0, "codec_type": "video", "codec_name": "hevc", "width": 3840, "height": 2160,
                  "r_frame_rate": "30/1", "avg_frame_rate": "30/1", "color_transfer": "smpte2084",
                  "disposition": { "default": 1, "attached_pic": 0 } },
                { "index": 1, "codec_type": "video", "codec_name": "h264", "width": 320, "height": 180,
                  "disposition": { "default": 0, "attached_pic": 0 } },
                { "index": 2, "codec_type": "video", "codec_name": "mjpeg", "width": 600, "height": 600,
                  "disposition": { "default": 0, "attached_pic": 1 } },
                { "index": 3, "codec_type": "audio", "codec_name": "aac", "disposition": { "default": 1 } },
                { "index": 4, "codec_type": "audio", "codec_name": "ac3", "disposition": { "default": 0 } },
                { "index": 5, "codec_type": "subtitle", "codec_name": "subrip", "disposition": { "default": 1, "forced": 0 } }
            ],
            "chapters": [{ "id": 0 }],
            "format": { "format_name": "matroska,webm", "duration": "60.000", "bit_rate": "9000000" }
        });
        let report =
            parse_probe_report(Path::new("hdr.mkv"), 1_000_000, root).expect("synthetic report");
        assert!(report.primary_video.hdr);
        assert_eq!(report.video_stream_count, 2);
        assert_eq!(report.audio_streams.len(), 2);
        assert_eq!(report.subtitle_streams.len(), 1);
        assert_eq!(report.chapter_count, 1);
        assert_eq!(report.attached_picture_count, 1);
        assert_eq!(report.warnings.len(), 5);
        assert_eq!(report.blocking_reasons.len(), 1);
        assert!(report.blocking_reasons[0].starts_with("VIDEO_PROBE_HDR_UNSUPPORTED:"));
    }
}
