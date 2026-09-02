use crate::services::video_compression_plan::VideoCompressionPlan;
use serde::Serialize;
use std::ffi::OsString;
use std::path::Path;
#[cfg(windows)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(windows)]
use std::sync::Arc;
#[cfg(windows)]
use std::time::{Duration, Instant};
use thiserror::Error;

#[cfg(windows)]
const MAX_FFMPEG_ERROR_BYTES: usize = 64 * 1024;
#[cfg(windows)]
const PROGRESS_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug, Error, PartialEq, Eq)]
pub enum VideoEncodingError {
    #[error("VIDEO_ENCODING_PLAN_BLOCKED")]
    PlanBlocked,
    #[error("VIDEO_ENCODING_OUTPUT_MUST_BE_MP4")]
    OutputMustBeMp4,
    #[error("VIDEO_ENCODING_PROGRESS_INVALID: {0}")]
    InvalidProgress(String),
    #[error("VIDEO_ENCODING_DESTINATION_EXISTS: {0}")]
    DestinationExists(String),
    #[error("VIDEO_ENCODING_SOURCE_DESTINATION_CONFLICT")]
    SourceDestinationConflict,
    #[error("VIDEO_ENCODING_RESOURCE_PREFLIGHT_FAILED: {0}")]
    ResourcePreflightFailed(String),
    #[error("VIDEO_ENCODING_LAUNCH_FAILED: {0}")]
    LaunchFailed(String),
    #[error("VIDEO_ENCODING_PROCESS_FAILED: {0}")]
    ProcessFailed(String),
    #[error("VIDEO_ENCODING_PROGRESS_INCOMPLETE")]
    ProgressIncomplete,
    #[error("VIDEO_ENCODING_OUTPUT_EMPTY")]
    OutputEmpty,
    #[error("VIDEO_ENCODING_CANCELLED")]
    Cancelled,
    #[cfg(windows)]
    #[error("VIDEO_ENCODING_JOB_OBJECT_FAILED: {0}")]
    JobObjectFailed(String),
}

/// Builds the exact product FFmpeg argv. Paths remain individual OS strings and
/// are never interpolated into a shell command.
pub fn build_ffmpeg_arguments(
    plan: &VideoCompressionPlan,
    staged_output: &Path,
) -> Result<Vec<OsString>, VideoEncodingError> {
    if !plan.can_encode {
        return Err(VideoEncodingError::PlanBlocked);
    }
    if staged_output
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case("mp4"))
    {
        return Err(VideoEncodingError::OutputMustBeMp4);
    }

    let mut arguments = vec![
        OsString::from("-hide_banner"),
        OsString::from("-nostdin"),
        OsString::from("-nostats"),
        OsString::from("-loglevel"),
        OsString::from("error"),
        OsString::from("-n"),
        OsString::from("-i"),
        plan.probe.source.as_os_str().to_owned(),
        OsString::from("-map"),
        OsString::from(format!("0:{}", plan.probe.primary_video.index)),
    ];
    if let Some(audio) = plan.probe.audio_streams.first() {
        arguments.extend([
            OsString::from("-map"),
            OsString::from(format!("0:{}", audio.index)),
        ]);
    }
    arguments.extend([
        OsString::from("-vf"),
        OsString::from(format!(
            "scale={}:{},format=nv12",
            plan.output_width, plan.output_height
        )),
        OsString::from("-c:v"),
        OsString::from("h264_mf"),
        OsString::from("-hw_encoding"),
        OsString::from("0"),
        OsString::from("-rate_control"),
        OsString::from("cbr"),
        OsString::from("-b:v"),
        OsString::from(plan.target_video_bit_rate.to_string()),
        OsString::from("-fps_mode"),
        OsString::from("passthrough"),
    ]);
    if let Some(audio_bit_rate) = plan.target_audio_bit_rate {
        arguments.extend([
            OsString::from("-c:a"),
            OsString::from("aac"),
            OsString::from("-b:a"),
            OsString::from(audio_bit_rate.to_string()),
        ]);
    }
    arguments.extend([
        OsString::from("-sn"),
        OsString::from("-dn"),
        OsString::from("-map_metadata"),
        OsString::from("-1"),
        OsString::from("-map_chapters"),
        OsString::from("-1"),
        OsString::from("-movflags"),
        OsString::from("+faststart"),
        OsString::from("-progress"),
        OsString::from("pipe:1"),
        OsString::from("-f"),
        OsString::from("mp4"),
        staged_output.as_os_str().to_owned(),
    ]);
    Ok(arguments)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoProgressSnapshot {
    pub current_time_ms: u64,
    pub progress_percent: f32,
    pub speed_multiple: Option<f64>,
    pub output_bytes: Option<u64>,
    pub output_bytes_provisional: bool,
    pub eta_seconds: Option<u64>,
    pub output_to_input_ratio: Option<f64>,
    pub finished: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum VideoEncodingEvent {
    Progress { snapshot: VideoProgressSnapshot },
    Heartbeat { seconds_since_progress: u64 },
}

/// Owns an unpublished video output. Dropping it removes the whole staging
/// family, so a caller cannot accidentally leak or publish an unverified file.
#[derive(Debug)]
#[cfg(windows)]
pub struct StagedVideoOutput {
    path: std::path::PathBuf,
    input_bytes: u64,
    output_bytes: u64,
}

#[cfg(windows)]
impl StagedVideoOutput {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn input_bytes(&self) -> u64 {
        self.input_bytes
    }

    pub fn output_bytes(&self) -> u64 {
        self.output_bytes
    }
}

#[cfg(windows)]
impl Drop for StagedVideoOutput {
    fn drop(&mut self) {
        crate::services::output_publish_transaction::cleanup_staged_output_family(&self.path);
    }
}

#[cfg(windows)]
struct StagedOutputCleanup {
    path: std::path::PathBuf,
    armed: bool,
}

#[cfg(windows)]
impl StagedOutputCleanup {
    fn new(path: std::path::PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

#[cfg(windows)]
impl Drop for StagedOutputCleanup {
    fn drop(&mut self) {
        if self.armed {
            crate::services::output_publish_transaction::cleanup_staged_output_family(&self.path);
        }
    }
}

#[derive(Debug)]
pub struct VideoProgressParser {
    duration_us: u64,
    input_bytes: u64,
    out_time_us: Option<u64>,
    total_size: Option<u64>,
    speed_multiple: Option<f64>,
    valid_timeline_samples: u32,
    previous_sample_time_us: Option<u64>,
}

impl VideoProgressParser {
    pub fn new(duration_ms: u64, input_bytes: u64) -> Result<Self, VideoEncodingError> {
        let duration_us = duration_ms
            .checked_mul(1_000)
            .filter(|value| *value > 0)
            .ok_or_else(|| VideoEncodingError::InvalidProgress("duration".to_string()))?;
        Ok(Self {
            duration_us,
            input_bytes,
            out_time_us: None,
            total_size: None,
            speed_multiple: None,
            valid_timeline_samples: 0,
            previous_sample_time_us: None,
        })
    }

    pub fn push_line(
        &mut self,
        line: &str,
    ) -> Result<Option<VideoProgressSnapshot>, VideoEncodingError> {
        let line = line.trim_end_matches(['\r', '\n']);
        let Some((key, value)) = line.split_once('=') else {
            return Ok(None);
        };
        match key {
            "out_time_us" => {
                self.out_time_us = parse_optional_timestamp_us(value, key)?;
            }
            "total_size" => {
                self.total_size = parse_optional_u64(value, key)?;
            }
            "speed" => {
                self.speed_multiple = parse_speed(value);
            }
            "progress" if value == "continue" || value == "end" => {
                let finished = value == "end";
                let out_time_us = self.out_time_us.unwrap_or(0).min(self.duration_us);
                if self
                    .previous_sample_time_us
                    .is_none_or(|previous| out_time_us > previous)
                {
                    self.valid_timeline_samples = self.valid_timeline_samples.saturating_add(1);
                    self.previous_sample_time_us = Some(out_time_us);
                }
                let speed = self.speed_multiple.filter(|value| *value > 0.0);
                let eta_seconds = (self.valid_timeline_samples >= 2)
                    .then_some(speed)
                    .flatten()
                    .map(|speed| {
                        ((self.duration_us.saturating_sub(out_time_us) as f64 / 1_000_000.0)
                            / speed)
                            .ceil() as u64
                    });
                let output_to_input_ratio = self
                    .total_size
                    .filter(|_| self.input_bytes > 0)
                    .map(|size| size as f64 / self.input_bytes as f64);
                return Ok(Some(VideoProgressSnapshot {
                    current_time_ms: out_time_us / 1_000,
                    progress_percent: if finished {
                        100.0
                    } else {
                        (out_time_us as f64 * 100.0 / self.duration_us as f64).clamp(0.0, 99.9)
                            as f32
                    },
                    speed_multiple: speed,
                    output_bytes: self.total_size,
                    output_bytes_provisional: self.total_size.is_some(),
                    eta_seconds,
                    output_to_input_ratio,
                    finished,
                }));
            }
            "progress" => {
                return Err(VideoEncodingError::InvalidProgress(format!(
                    "progress={value}"
                )));
            }
            _ => {}
        }
        Ok(None)
    }
}

fn parse_optional_timestamp_us(value: &str, key: &str) -> Result<Option<u64>, VideoEncodingError> {
    if value == "N/A" {
        return Ok(None);
    }
    let value = value
        .parse::<i64>()
        .map_err(|_| VideoEncodingError::InvalidProgress(format!("{key}={value}")))?;
    Ok(Some(value.max(0) as u64))
}

fn parse_optional_u64(value: &str, key: &str) -> Result<Option<u64>, VideoEncodingError> {
    if value == "N/A" {
        return Ok(None);
    }
    value
        .parse()
        .map(Some)
        .map_err(|_| VideoEncodingError::InvalidProgress(format!("{key}={value}")))
}

fn parse_speed(value: &str) -> Option<f64> {
    value
        .strip_suffix('x')?
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

#[cfg(windows)]
pub struct WindowsJobObject {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

// Windows kernel handles may be transferred between threads; ownership remains
// unique because this guard is not Clone and closes the handle exactly once.
#[cfg(windows)]
unsafe impl Send for WindowsJobObject {}

#[cfg(windows)]
impl WindowsJobObject {
    pub fn create() -> Result<Self, VideoEncodingError> {
        use windows_sys::Win32::System::JobObjects::{
            CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return Err(last_job_error("create"));
        }
        let mut information: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        information.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let configured = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                std::ptr::addr_of!(information).cast(),
                std::mem::size_of_val(&information) as u32,
            )
        };
        if configured == 0 {
            unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) };
            return Err(last_job_error("configure"));
        }
        Ok(Self { handle })
    }

    pub fn assign(&self, child: &std::process::Child) -> Result<(), VideoEncodingError> {
        use std::os::windows::io::AsRawHandle;
        self.assign_raw_handle(child.as_raw_handle())
    }

    pub fn assign_raw_handle(
        &self,
        raw_handle: std::os::windows::io::RawHandle,
    ) -> Result<(), VideoEncodingError> {
        use windows_sys::Win32::System::JobObjects::AssignProcessToJobObject;
        let assigned = unsafe { AssignProcessToJobObject(self.handle, raw_handle as _) };
        if assigned == 0 {
            return Err(last_job_error("assign"));
        }
        Ok(())
    }

    pub fn terminate(&self) -> Result<(), VideoEncodingError> {
        use windows_sys::Win32::System::JobObjects::TerminateJobObject;
        if unsafe { TerminateJobObject(self.handle, 1) } == 0 {
            return Err(last_job_error("terminate"));
        }
        Ok(())
    }
}

#[cfg(windows)]
fn last_job_error(operation: &str) -> VideoEncodingError {
    VideoEncodingError::JobObjectFailed(format!("{operation}: win32={}", unsafe {
        windows_sys::Win32::Foundation::GetLastError()
    }))
}

#[cfg(windows)]
impl Drop for WindowsJobObject {
    fn drop(&mut self) {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.handle) };
    }
}

#[cfg(windows)]
pub async fn encode_video_to_staging<F>(
    ffmpeg: &Path,
    plan: &VideoCompressionPlan,
    final_output: &Path,
    cancelled: Arc<AtomicBool>,
    observer: F,
) -> Result<StagedVideoOutput, VideoEncodingError>
where
    F: FnMut(VideoEncodingEvent),
{
    encode_video_to_staging_with_heartbeat(
        ffmpeg,
        plan,
        final_output,
        cancelled,
        PROGRESS_HEARTBEAT_INTERVAL,
        observer,
    )
    .await
}

#[cfg(windows)]
async fn encode_video_to_staging_with_heartbeat<F>(
    ffmpeg: &Path,
    plan: &VideoCompressionPlan,
    final_output: &Path,
    cancelled: Arc<AtomicBool>,
    heartbeat_interval: Duration,
    mut observer: F,
) -> Result<StagedVideoOutput, VideoEncodingError>
where
    F: FnMut(VideoEncodingEvent),
{
    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

    if cancelled.load(Ordering::SeqCst) {
        return Err(VideoEncodingError::Cancelled);
    }
    if final_output.exists() {
        return Err(VideoEncodingError::DestinationExists(
            final_output.to_string_lossy().into_owned(),
        ));
    }
    if plan.probe.source == final_output {
        return Err(VideoEncodingError::SourceDestinationConflict);
    }

    let source = plan.probe.source.to_string_lossy().into_owned();
    let destination = final_output.to_string_lossy().into_owned();
    let preflight = crate::services::storage_preflight::preflight_operation_resources(
        "compression",
        &destination,
        &[source],
        None,
        Some(plan.estimated_output.high_bytes),
        false,
    )
    .await
    .map_err(|error| VideoEncodingError::ResourcePreflightFailed(error.to_string()))?;
    if !preflight.can_start {
        return Err(VideoEncodingError::ResourcePreflightFailed(
            preflight.summary,
        ));
    }

    let staged_path = crate::services::output_publish_transaction::staged_output_path(
        final_output,
        "video-encode",
    )
    .map_err(|error| VideoEncodingError::LaunchFailed(error.to_string()))?;
    let mut staged_cleanup = StagedOutputCleanup::new(staged_path.clone());
    let arguments = build_ffmpeg_arguments(plan, &staged_path)?;
    let job = WindowsJobObject::create()?;
    let mut command = crate::utils::process::async_command(ffmpeg);
    command
        .args(arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|error| VideoEncodingError::LaunchFailed(error.to_string()))?;
    let raw_handle = child
        .raw_handle()
        .ok_or_else(|| VideoEncodingError::LaunchFailed("missing process handle".to_string()))?;
    if let Err(error) = job.assign_raw_handle(raw_handle) {
        let _ = child.kill().await;
        return Err(error);
    }

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| VideoEncodingError::LaunchFailed("missing progress pipe".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| VideoEncodingError::LaunchFailed("missing error pipe".to_string()))?;
    let mut stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr);
        let mut retained = Vec::new();
        let mut buffer = [0_u8; 8 * 1024];
        loop {
            let read = reader.read(&mut buffer).await?;
            if read == 0 {
                break;
            }
            let remaining = MAX_FFMPEG_ERROR_BYTES.saturating_sub(retained.len());
            retained.extend_from_slice(&buffer[..read.min(remaining)]);
        }
        std::io::Result::Ok(retained)
    });

    let mut lines = BufReader::new(stdout).lines();
    let mut parser = VideoProgressParser::new(plan.probe.duration_ms, plan.probe.input_bytes)?;
    let mut heartbeat = tokio::time::interval(heartbeat_interval);
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    heartbeat.tick().await;
    let mut cancellation_poll = tokio::time::interval(Duration::from_millis(50));
    cancellation_poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    cancellation_poll.tick().await;
    let mut last_progress_at = Instant::now();
    let mut progress_finished = false;
    let mut progress_pipe_open = true;

    let status = loop {
        tokio::select! {
            biased;
            _ = cancellation_poll.tick() => {
                if cancelled.load(Ordering::SeqCst) {
                    if job.terminate().is_err() {
                        let _ = child.kill().await;
                    }
                    let _ = child.wait().await;
                    let _ = (&mut stderr_task).await;
                    return Err(VideoEncodingError::Cancelled);
                }
            }
            line = lines.next_line(), if progress_pipe_open => {
                match line.map_err(|error| VideoEncodingError::ProcessFailed(error.to_string()))? {
                    Some(line) => {
                        if let Some(snapshot) = parser.push_line(&line)? {
                            progress_finished |= snapshot.finished;
                            last_progress_at = Instant::now();
                            observer(VideoEncodingEvent::Progress { snapshot });
                        }
                    }
                    None => progress_pipe_open = false,
                }
            }
            _ = heartbeat.tick() => {
                observer(VideoEncodingEvent::Heartbeat {
                    seconds_since_progress: last_progress_at.elapsed().as_secs(),
                });
            }
            status = child.wait() => {
                break status.map_err(|error| VideoEncodingError::ProcessFailed(error.to_string()))?;
            }
        }
    };

    let stderr = (&mut stderr_task)
        .await
        .map_err(|error| VideoEncodingError::ProcessFailed(error.to_string()))?
        .map_err(|error| VideoEncodingError::ProcessFailed(error.to_string()))?;
    if cancelled.load(Ordering::SeqCst) {
        return Err(VideoEncodingError::Cancelled);
    }
    if !status.success() {
        return Err(VideoEncodingError::ProcessFailed(
            String::from_utf8_lossy(&stderr).trim().to_string(),
        ));
    }
    if !progress_finished {
        return Err(VideoEncodingError::ProgressIncomplete);
    }
    let output_bytes = std::fs::metadata(&staged_path)
        .map_err(|_| VideoEncodingError::OutputEmpty)?
        .len();
    if output_bytes == 0 {
        return Err(VideoEncodingError::OutputEmpty);
    }

    staged_cleanup.disarm();
    Ok(StagedVideoOutput {
        path: staged_path,
        input_bytes: plan.probe.input_bytes,
        output_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::video_compression_plan::{
        build_video_compression_plan, VideoCompressionPlanRequest, VideoCompressionPreset,
    };
    use crate::services::video_probe::{
        VideoFirstReleasePolicy, VideoFrameRateMode, VideoProbeReport, VideoStreamFacts,
    };
    use std::path::PathBuf;

    fn plan(source: &str) -> VideoCompressionPlan {
        let probe = VideoProbeReport {
            source: PathBuf::from(source),
            input_bytes: 10_000,
            container: Some("mov,mp4".to_string()),
            duration_ms: 10_000,
            overall_bit_rate: None,
            primary_video: VideoStreamFacts {
                index: 2,
                codec: Some("h264".to_string()),
                profile: None,
                encoded_width: 640,
                encoded_height: 360,
                visible_width: 640,
                visible_height: 360,
                rotation_degrees: 0,
                pixel_format: Some("yuv420p".to_string()),
                color_transfer: None,
                hdr: false,
                nominal_frame_rate: Some("30/1".to_string()),
                average_frame_rate: Some("30/1".to_string()),
                average_frame_rate_milli: Some(30_000),
                frame_rate_mode: VideoFrameRateMode::ConstantOrUndetermined,
                bit_rate: None,
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
        };
        build_video_compression_plan(
            probe,
            &VideoCompressionPlanRequest {
                path: source.to_string(),
                preset: VideoCompressionPreset::Balanced,
                quality: 76,
                max_width: None,
                max_height: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn argv_keeps_special_paths_as_single_os_arguments_and_uses_progress_pipe() {
        let source = r"C:\视频 文件\a&b (最终).mp4";
        let output = Path::new(r"D:\输出 目录\.video-123.a&b (最终).mp4");
        let arguments = build_ffmpeg_arguments(&plan(source), output).unwrap();
        assert!(arguments.iter().any(|value| value == source));
        assert!(arguments.iter().any(|value| value == output.as_os_str()));
        assert!(arguments
            .windows(2)
            .any(|pair| pair == ["-progress", "pipe:1"]));
        assert!(arguments
            .windows(2)
            .any(|pair| pair == ["-hw_encoding", "0"]));
        assert!(arguments
            .windows(2)
            .any(|pair| pair == ["-rate_control", "cbr"]));
        assert!(arguments
            .windows(2)
            .any(|pair| pair == ["-fps_mode", "passthrough"]));
        assert!(!arguments
            .iter()
            .any(|value| value == "cmd.exe" || value == "/C"));
    }

    #[test]
    fn progress_uses_machine_fields_and_waits_for_two_samples_before_eta() {
        let mut parser = VideoProgressParser::new(10_000, 20_000).unwrap();
        for line in ["out_time_us=2000000", "total_size=5000", "speed=2.0x"] {
            assert!(parser.push_line(line).unwrap().is_none());
        }
        let first = parser.push_line("progress=continue").unwrap().unwrap();
        assert_eq!(first.current_time_ms, 2_000);
        assert_eq!(first.progress_percent, 20.0);
        assert_eq!(first.output_bytes, Some(5_000));
        assert_eq!(first.output_to_input_ratio, Some(0.25));
        assert_eq!(first.eta_seconds, None);

        for line in ["out_time_us=6000000", "total_size=8000", "speed=2.0x"] {
            assert!(parser.push_line(line).unwrap().is_none());
        }
        let second = parser.push_line("progress=continue").unwrap().unwrap();
        assert_eq!(second.eta_seconds, Some(2));
        assert!(second.output_bytes_provisional);
        let end = parser.push_line("progress=end").unwrap().unwrap();
        assert_eq!(end.progress_percent, 100.0);
        assert!(end.finished);
    }

    #[test]
    fn malformed_authoritative_values_are_rejected_not_fabricated() {
        let mut parser = VideoProgressParser::new(1_000, 1).unwrap();
        assert!(matches!(
            parser.push_line("out_time_us=not-a-number"),
            Err(VideoEncodingError::InvalidProgress(_))
        ));
        assert!(matches!(
            parser.push_line("progress=localized"),
            Err(VideoEncodingError::InvalidProgress(_))
        ));
    }

    #[test]
    fn official_unknown_and_negative_start_values_remain_zero_or_absent() {
        let mut parser = VideoProgressParser::new(1_000, 1_000).unwrap();
        for line in ["out_time_us=N/A", "total_size=N/A", "speed=N/A"] {
            assert!(parser.push_line(line).unwrap().is_none());
        }
        let unknown = parser.push_line("progress=continue").unwrap().unwrap();
        assert_eq!(unknown.current_time_ms, 0);
        assert_eq!(unknown.output_bytes, None);
        assert_eq!(unknown.speed_multiple, None);
        assert_eq!(unknown.eta_seconds, None);

        assert!(parser.push_line("out_time_us=-21333").unwrap().is_none());
        let preroll = parser.push_line("progress=continue").unwrap().unwrap();
        assert_eq!(preroll.current_time_ms, 0);
        assert_eq!(preroll.progress_percent, 0.0);

        assert!(matches!(
            parser.push_line("total_size=-1"),
            Err(VideoEncodingError::InvalidProgress(_))
        ));
    }

    #[test]
    #[cfg(windows)]
    fn job_object_terminates_an_assigned_real_process() {
        let job = WindowsJobObject::create().expect("create job");
        let mut child = crate::utils::process::command("ping")
            .args(["127.0.0.1", "-n", "60"])
            .spawn()
            .expect("spawn ping");
        job.assign(&child).expect("assign ping");
        job.terminate().expect("terminate job");
        let status = child.wait().expect("wait ping");
        assert!(!status.success());
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn product_ffmpeg_encodes_a_real_special_character_path_and_emits_machine_progress() {
        use crate::services::video_probe::probe_video_file;
        use std::process::Stdio;

        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ffmpeg = manifest.join("resources/video-engine/ffmpeg.exe");
        let ffprobe = manifest.join("resources/video-engine/ffprobe.exe");
        let fixture =
            manifest.join("../tests/fixtures/media/videos/h264-vfr-audio-rotation-subtitles.mp4");
        let temporary = tempfile::tempdir().expect("temporary directory");
        let special_directory = temporary.path().join("中文 空格 & (C03)");
        std::fs::create_dir(&special_directory).expect("create special directory");
        let source = special_directory.join("输入 & source (1).mp4");
        std::fs::copy(fixture, &source).expect("copy real input");
        let staged = special_directory.join(".video-test.输出 & staged (1).mp4");

        let report = probe_video_file(&ffprobe, &source)
            .await
            .expect("probe real input");
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
        .expect("plan real input");
        let arguments = build_ffmpeg_arguments(&plan, &staged).expect("build argv");
        let job = WindowsJobObject::create().expect("create job");
        let mut command = crate::utils::process::command(&ffmpeg);
        command
            .args(arguments)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let child = command.spawn().expect("spawn product ffmpeg");
        job.assign(&child).expect("assign product ffmpeg");
        let output = child.wait_with_output().expect("wait product ffmpeg");
        assert!(
            output.status.success(),
            "product ffmpeg failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let mut parser = VideoProgressParser::new(plan.probe.duration_ms, plan.probe.input_bytes)
            .expect("create progress parser");
        let snapshots = String::from_utf8(output.stdout)
            .expect("progress is UTF-8 machine data")
            .lines()
            .filter_map(|line| parser.push_line(line).expect("parse progress"))
            .collect::<Vec<_>>();
        assert!(snapshots.last().is_some_and(|snapshot| snapshot.finished));
        assert!(snapshots
            .iter()
            .any(|snapshot| snapshot.output_bytes.is_some()));
        assert!(std::fs::metadata(staged).expect("staged output").len() > 0);
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn internal_executor_returns_only_owned_staging_and_drop_cleans_it() {
        use crate::services::video_probe::probe_video_file;

        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ffmpeg = manifest.join("resources/video-engine/ffmpeg.exe");
        let ffprobe = manifest.join("resources/video-engine/ffprobe.exe");
        let fixture =
            manifest.join("../tests/fixtures/media/videos/h264-vfr-audio-rotation-subtitles.mp4");
        let temporary = tempfile::tempdir().expect("temporary directory");
        let source = temporary.path().join("输入 & executor (1).mp4");
        std::fs::copy(fixture, &source).expect("copy real input");
        let final_output = temporary.path().join("最终 & unpublished (1).mp4");
        let report = probe_video_file(&ffprobe, &source)
            .await
            .expect("probe real input");
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
        .expect("plan real input");
        let mut events = Vec::new();
        let staged = encode_video_to_staging(
            &ffmpeg,
            &plan,
            &final_output,
            Arc::new(AtomicBool::new(false)),
            |event| events.push(event),
        )
        .await
        .expect("encode to staging");

        assert!(!final_output.exists(), "C-03 must not publish final output");
        assert!(staged.path().exists());
        assert_eq!(staged.input_bytes(), plan.probe.input_bytes);
        assert_eq!(
            staged.output_bytes(),
            std::fs::metadata(staged.path()).unwrap().len()
        );
        assert!(events.iter().any(|event| matches!(
            event,
            VideoEncodingEvent::Progress { snapshot } if snapshot.finished
        )));
        let staged_path = staged.path().to_path_buf();
        drop(staged);
        assert!(
            !staged_path.exists(),
            "unverified staging must be owned and cleaned"
        );
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn heartbeat_cancellation_terminates_long_encode_and_cleans_staging_family() {
        use crate::services::video_probe::probe_video_file;

        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ffmpeg = manifest.join("resources/video-engine/ffmpeg.exe");
        let ffprobe = manifest.join("resources/video-engine/ffprobe.exe");
        let fixture =
            manifest.join("../tests/fixtures/media/videos/h264-vfr-audio-rotation-subtitles.mp4");
        let temporary = tempfile::tempdir().expect("temporary directory");
        let source = temporary.path().join("十分钟 输入 & cancel (1).mp4");
        let generated = crate::utils::process::command(&ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-stream_loop",
                "499",
                "-i",
            ])
            .arg(&fixture)
            .args([
                "-map", "0:v:0", "-map", "0:a:0?", "-c", "copy", "-sn", "-t", "600", "-f", "mp4",
            ])
            .arg(&source)
            .status()
            .expect("generate long real input");
        assert!(generated.success());
        let report = probe_video_file(&ffprobe, &source)
            .await
            .expect("probe long real input");
        assert!(report.duration_ms >= 500_000);
        let plan = build_video_compression_plan(
            report,
            &VideoCompressionPlanRequest {
                path: source.to_string_lossy().into_owned(),
                preset: VideoCompressionPreset::Clear,
                quality: 92,
                max_width: None,
                max_height: None,
            },
        )
        .expect("plan long input");
        let final_output = temporary.path().join("不得发布 & cancelled.mp4");
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancel_from_heartbeat = cancelled.clone();
        let mut saw_heartbeat = false;
        let started = Instant::now();
        let result = encode_video_to_staging_with_heartbeat(
            &ffmpeg,
            &plan,
            &final_output,
            cancelled,
            Duration::from_millis(10),
            |event| {
                if matches!(event, VideoEncodingEvent::Heartbeat { .. }) {
                    saw_heartbeat = true;
                    cancel_from_heartbeat.store(true, Ordering::SeqCst);
                }
            },
        )
        .await;

        assert!(matches!(result, Err(VideoEncodingError::Cancelled)));
        assert!(saw_heartbeat);
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(!final_output.exists());
        let leaked = std::fs::read_dir(temporary.path())
            .unwrap()
            .flatten()
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".video-encode-")
            });
        assert!(!leaked, "cancel must remove staged output and sidecars");
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn real_ffmpeg_nonzero_exit_publishes_nothing_and_leaves_no_staging() {
        use crate::services::video_probe::probe_video_file;

        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ffmpeg = manifest.join("resources/video-engine/ffmpeg.exe");
        let ffprobe = manifest.join("resources/video-engine/ffprobe.exe");
        let fixture =
            manifest.join("../tests/fixtures/media/videos/h264-vfr-audio-rotation-subtitles.mp4");
        let temporary = tempfile::tempdir().expect("temporary directory");
        let source = temporary.path().join("becomes-corrupt.mp4");
        std::fs::copy(fixture, &source).expect("copy valid source");
        let report = probe_video_file(&ffprobe, &source)
            .await
            .expect("probe valid source");
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
        .expect("plan source");
        std::fs::write(&source, b"not a media container after planning")
            .expect("replace source with corrupt bytes");
        let final_output = temporary.path().join("must-not-publish.mp4");

        let result = encode_video_to_staging(
            &ffmpeg,
            &plan,
            &final_output,
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .await;
        assert!(matches!(result, Err(VideoEncodingError::ProcessFailed(_))));
        assert!(!final_output.exists());
        let leaked = std::fs::read_dir(temporary.path())
            .unwrap()
            .flatten()
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".video-encode-")
            });
        assert!(!leaked);
    }
}
