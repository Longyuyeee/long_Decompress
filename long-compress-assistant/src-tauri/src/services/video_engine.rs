use anyhow::{bail, Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

const RUNTIME_DIRECTORY: &str = "video-engine";

#[derive(Clone, Copy)]
struct ExpectedResource {
    relative_path: &'static str,
    bytes: u64,
    sha256: &'static str,
}

const EXPECTED_RESOURCES: [ExpectedResource; 8] = [
    ExpectedResource {
        relative_path: "ffmpeg.exe",
        bytes: 12_349_440,
        sha256: "35c3c8bb7d9371825ba3ee8ee6f6b39205877c5d1172e4a4e925c2d6368672eb",
    },
    ExpectedResource {
        relative_path: "ffprobe.exe",
        bytes: 12_131_840,
        sha256: "2c1df07c649e9499eddd40b445c8721f07b95b8a85524a5e8645a86fb2ba1d98",
    },
    ExpectedResource {
        relative_path: "SOURCE.txt",
        bytes: 785,
        sha256: "7f769fd860605044474cee1669cf5e6a7d7a93cbd67c40c15856832d0735546b",
    },
    ExpectedResource {
        relative_path: "BUILD-CONFIGURATION.txt",
        bytes: 1_564,
        sha256: "c4df007d0aa655a9dee97a24edeca00fe555430e083006aaf9e5c73e93e11973",
    },
    ExpectedResource {
        relative_path: "licenses/COPYING.LGPLv2.1",
        bytes: 26_517,
        sha256: "246041b6ecf9bc32d718a62c57877c78b5eb397b6467e74ed7ae2626ab189c30",
    },
    ExpectedResource {
        relative_path: "licenses/COPYING.LGPLv3",
        bytes: 7_651,
        sha256: "da7eabb7bafdf7d3ae5e9f223aa5bdc1eece45ac569dc21b3b037520b4464768",
    },
    ExpectedResource {
        relative_path: "licenses/GCC-MinGW-runtime-copyright.txt",
        bytes: 75_729,
        sha256: "a481f772f7a53335f13b32c6c54eb1c8577ce97704edd3757ab7ed4287a8e96a",
    },
    ExpectedResource {
        relative_path: "licenses/MinGW-w64-copyright.txt",
        bytes: 37_808,
        sha256: "af23297b7d17e8e31817a5d58088de6fdafe34705bb5a7d1d330f08711b31314",
    },
];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VideoEngineFileIdentity {
    pub relative_path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VideoEngineStatus {
    pub version: String,
    pub license: String,
    pub video_encoder: String,
    pub audio_encoder: String,
    pub hardware_encoding: bool,
    pub enabled_filters: Vec<String>,
    pub files: Vec<VideoEngineFileIdentity>,
}

fn hash_file(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .with_context(|| format!("cannot read {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn inspect_resources(resource_root: &Path) -> Result<(PathBuf, Vec<VideoEngineFileIdentity>)> {
    let runtime_root = resource_root.join(RUNTIME_DIRECTORY);
    let mut identities = Vec::with_capacity(EXPECTED_RESOURCES.len());
    for expected in EXPECTED_RESOURCES {
        let path = runtime_root.join(expected.relative_path);
        let metadata = std::fs::metadata(&path)
            .with_context(|| format!("VIDEO_ENGINE_RESOURCE_MISSING: {}", path.display()))?;
        if !metadata.is_file() {
            bail!("VIDEO_ENGINE_RESOURCE_NOT_FILE: {}", path.display());
        }
        if metadata.len() != expected.bytes {
            bail!(
                "VIDEO_ENGINE_RESOURCE_SIZE_MISMATCH: {} expected={} actual={}",
                path.display(),
                expected.bytes,
                metadata.len()
            );
        }
        let sha256 = hash_file(&path)?;
        if sha256 != expected.sha256 {
            bail!(
                "VIDEO_ENGINE_RESOURCE_HASH_MISMATCH: {} expected={} actual={}",
                path.display(),
                expected.sha256,
                sha256
            );
        }
        identities.push(VideoEngineFileIdentity {
            relative_path: expected.relative_path.replace('/', "\\"),
            bytes: metadata.len(),
            sha256,
        });
    }
    Ok((runtime_root, identities))
}

fn run_probe(executable: &Path, arguments: &[&str], label: &str) -> Result<String> {
    let output = Command::new(executable)
        .args(arguments)
        .output()
        .with_context(|| {
            format!(
                "VIDEO_ENGINE_LAUNCH_FAILED: {label}: {}",
                executable.display()
            )
        })?;
    if !output.status.success() {
        bail!(
            "VIDEO_ENGINE_PROBE_FAILED: {label}: exit={:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

pub fn validate_video_engine(resource_root: &Path) -> Result<VideoEngineStatus> {
    let (runtime_root, files) = inspect_resources(resource_root)?;
    let ffmpeg = runtime_root.join("ffmpeg.exe");
    let ffprobe = runtime_root.join("ffprobe.exe");

    let version = run_probe(&ffmpeg, &["-version"], "ffmpeg-version")?;
    if !version.contains("ffmpeg version 9.0.1")
        || !version.contains("--disable-everything")
        || !version.contains("--disable-hwaccels")
        || version.contains("--enable-gpl")
        || version.contains("--enable-nonfree")
        || version.contains("libx264")
        || version.contains("libx265")
    {
        bail!("VIDEO_ENGINE_VERSION_POLICY_MISMATCH");
    }

    let probe_version = run_probe(&ffprobe, &["-version"], "ffprobe-version")?;
    if !probe_version.contains("ffprobe version 9.0.1") {
        bail!("VIDEO_ENGINE_FFPROBE_VERSION_MISMATCH");
    }

    let encoders = run_probe(&ffmpeg, &["-hide_banner", "-encoders"], "ffmpeg-encoders")?;
    if !encoders.contains("h264_mf") || !encoders.contains(" AAC (Advanced Audio Coding)") {
        bail!("VIDEO_ENGINE_ENCODER_MISSING");
    }
    if encoders.contains("libx264")
        || encoders.contains("libx265")
        || encoders.contains("libopenh264")
    {
        bail!("VIDEO_ENGINE_FORBIDDEN_ENCODER");
    }

    let encoder_help = run_probe(
        &ffmpeg,
        &["-hide_banner", "-h", "encoder=h264_mf"],
        "h264-mf-options",
    )?;
    if !encoder_help.contains("hw_encoding") || !encoder_help.contains("default false") {
        bail!("VIDEO_ENGINE_SOFTWARE_DEFAULT_MISSING");
    }

    let filters = run_probe(&ffmpeg, &["-hide_banner", "-filters"], "ffmpeg-filters")?;
    let required_filters = ["scale", "format", "fps", "transpose", "aresample"];
    for filter in required_filters {
        if !filters
            .lines()
            .any(|line| line.split_whitespace().any(|item| item == filter))
        {
            bail!("VIDEO_ENGINE_FILTER_MISSING: {filter}");
        }
    }

    Ok(VideoEngineStatus {
        version: "9.0.1".to_string(),
        license: "LGPL-2.1-or-later".to_string(),
        video_encoder: "h264_mf".to_string(),
        audio_encoder: "aac".to_string(),
        hardware_encoding: false,
        enabled_filters: required_filters
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repository_resource_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources")
    }

    #[cfg(windows)]
    #[test]
    fn bundled_candidate_has_the_frozen_identity_and_real_capabilities() {
        let status = validate_video_engine(&repository_resource_root()).unwrap();
        assert_eq!(status.version, "9.0.1");
        assert_eq!(status.video_encoder, "h264_mf");
        assert_eq!(status.audio_encoder, "aac");
        assert!(!status.hardware_encoding);
        assert_eq!(status.files.len(), EXPECTED_RESOURCES.len());
    }

    #[test]
    fn missing_runtime_is_refused_before_process_launch() {
        let directory = tempfile::tempdir().unwrap();
        let error = validate_video_engine(directory.path())
            .unwrap_err()
            .to_string();
        assert!(error.contains("VIDEO_ENGINE_RESOURCE_MISSING"));
    }

    #[test]
    fn replaced_executable_is_refused_by_byte_identity() {
        let directory = tempfile::tempdir().unwrap();
        let target_root = directory.path().join(RUNTIME_DIRECTORY);
        std::fs::create_dir_all(target_root.join("licenses")).unwrap();
        for expected in EXPECTED_RESOURCES {
            let source = repository_resource_root()
                .join(RUNTIME_DIRECTORY)
                .join(expected.relative_path);
            let target = target_root.join(expected.relative_path);
            std::fs::copy(source, target).unwrap();
        }
        let ffmpeg = target_root.join("ffmpeg.exe");
        let mut bytes = std::fs::read(&ffmpeg).unwrap();
        bytes[0] ^= 0xff;
        std::fs::write(&ffmpeg, bytes).unwrap();

        let error = validate_video_engine(directory.path())
            .unwrap_err()
            .to_string();
        assert!(error.contains("VIDEO_ENGINE_RESOURCE_HASH_MISMATCH"));
    }
}
