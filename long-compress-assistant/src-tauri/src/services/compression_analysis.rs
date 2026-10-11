use anyhow::{Context, Result};
use serde::Serialize;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

const MAX_ENTRIES: usize = 100_000;
const MAX_SAMPLE_FILES: usize = 16;
const SAMPLE_CHUNK_BYTES: usize = 64 * 1024;
const MAX_SAMPLE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionAnalysisResult {
    pub total_size: u64,
    pub file_count: usize,
    pub sampled_files: usize,
    pub sampled_bytes: u64,
    pub estimated_size: u64,
    pub estimated_ratio: f64,
    pub estimated_seconds_low: u64,
    pub estimated_seconds_high: u64,
    pub confidence: String,
    pub recommended_format: String,
    pub recommended_level: u32,
    pub recommended_solid: bool,
    pub low_value_bytes: u64,
    pub low_value_file_count: usize,
    pub reasons: Vec<String>,
}

#[derive(Debug)]
struct SourceFile {
    path: PathBuf,
    size: u64,
    score: u64,
    low_value: bool,
}

fn check_cancelled(cancelled: &AtomicBool) -> Result<()> {
    if cancelled.load(Ordering::SeqCst) {
        anyhow::bail!("Compression analysis cancelled");
    }
    Ok(())
}

fn path_score(path: &Path) -> u64 {
    let hash = blake3::hash(path.to_string_lossy().as_bytes());
    u64::from_le_bytes(hash.as_bytes()[..8].try_into().expect("hash prefix"))
}

fn is_low_value_extension(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "7z" | "zip"
            | "zipx"
            | "rar"
            | "gz"
            | "bz2"
            | "xz"
            | "zst"
            | "jpg"
            | "jpeg"
            | "png"
            | "webp"
            | "avif"
            | "mp3"
            | "aac"
            | "flac"
            | "mp4"
            | "mkv"
            | "avi"
            | "mov"
            | "webm"
            | "pdf"
            | "docx"
            | "xlsx"
            | "pptx"
            | "apk"
            | "msi"
    )
}

fn collect_sources(paths: &[String], cancelled: &AtomicBool) -> Result<Vec<SourceFile>> {
    let mut files = Vec::new();
    for source in paths {
        check_cancelled(cancelled)?;
        let source = Path::new(source);
        let metadata = std::fs::symlink_metadata(source)
            .with_context(|| format!("Unable to inspect source: {}", source.display()))?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_file() {
            if files.len() >= MAX_ENTRIES {
                anyhow::bail!("Compression analysis source contains more than {MAX_ENTRIES} files");
            }
            files.push(SourceFile {
                path: source.to_path_buf(),
                size: metadata.len(),
                score: path_score(source),
                low_value: is_low_value_extension(source),
            });
            continue;
        }
        if !metadata.is_dir() {
            continue;
        }
        for entry in walkdir::WalkDir::new(source).follow_links(false) {
            check_cancelled(cancelled)?;
            let entry = entry?;
            if entry.file_type().is_symlink() || !entry.file_type().is_file() {
                continue;
            }
            if files.len() >= MAX_ENTRIES {
                anyhow::bail!("Compression analysis source contains more than {MAX_ENTRIES} files");
            }
            let size = entry.metadata()?.len();
            files.push(SourceFile {
                path: entry.path().to_path_buf(),
                size,
                score: path_score(entry.path()),
                low_value: is_low_value_extension(entry.path()),
            });
        }
    }
    if files.is_empty() {
        anyhow::bail!("Compression analysis found no readable files");
    }
    Ok(files)
}

fn read_chunk(file: &mut File, offset: u64, length: usize, target: &mut Vec<u8>) -> Result<()> {
    if target.len() >= MAX_SAMPLE_BYTES || length == 0 {
        return Ok(());
    }
    file.seek(SeekFrom::Start(offset))?;
    let remaining = MAX_SAMPLE_BYTES - target.len();
    let mut chunk = vec![0u8; length.min(remaining)];
    let read = file.read(&mut chunk)?;
    target.extend_from_slice(&chunk[..read]);
    Ok(())
}

fn sample_files(files: &[SourceFile], cancelled: &AtomicBool) -> Result<(Vec<u8>, usize)> {
    let mut candidates: Vec<&SourceFile> = files.iter().filter(|file| file.size > 0).collect();
    candidates.sort_by_key(|file| file.score);
    candidates.truncate(MAX_SAMPLE_FILES);
    let mut sample = Vec::new();
    let mut sampled_files = 0;
    for source in candidates {
        check_cancelled(cancelled)?;
        let mut file = File::open(&source.path)?;
        let chunk = SAMPLE_CHUNK_BYTES.min(source.size as usize);
        read_chunk(&mut file, 0, chunk, &mut sample)?;
        if source.size > (SAMPLE_CHUNK_BYTES * 2) as u64 {
            read_chunk(&mut file, source.size / 2, chunk, &mut sample)?;
        }
        if source.size > (SAMPLE_CHUNK_BYTES * 3) as u64 {
            read_chunk(
                &mut file,
                source.size.saturating_sub(chunk as u64),
                chunk,
                &mut sample,
            )?;
        }
        sampled_files += 1;
        if sample.len() >= MAX_SAMPLE_BYTES {
            break;
        }
    }
    Ok((sample, sampled_files))
}

fn compressed_sample_size(sample: &[u8], format: &str, level: u32) -> Result<usize> {
    if format == "tar" || (format == "wim" && level == 0) {
        return Ok(sample.len());
    }
    let level = level.min(9);
    if format.contains("bz2") {
        let mut encoder = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::new(level));
        encoder.write_all(sample)?;
        return Ok(encoder.finish()?.len());
    }
    if format.contains("xz") || format == "7z" || format == "lzma" {
        let mut encoder = xz2::write::XzEncoder::new(Vec::new(), level);
        encoder.write_all(sample)?;
        return Ok(encoder.finish()?.len());
    }
    if format.contains("zst") || format == "zstd" {
        return Ok(zstd::stream::encode_all(sample, (level as i32).clamp(1, 9))?.len());
    }
    let mut encoder =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(level));
    encoder.write_all(sample)?;
    Ok(encoder.finish()?.len())
}

fn duration_range(total_size: u64, format: &str, level: u32, file_count: usize) -> (u64, u64) {
    let base_mib_per_second = match format {
        "7z" | "xz" | "tar.xz" | "lzma" => 45.0,
        "bz2" | "tar.bz2" => 35.0,
        "zst" | "zstd" | "tar.zst" => 180.0,
        "tar" => 350.0,
        _ => 110.0,
    };
    let level_factor = 1.0 + level.saturating_sub(1) as f64 * 0.16;
    let seconds = total_size as f64 / (1024.0 * 1024.0) / base_mib_per_second * level_factor
        + file_count as f64 * 0.0008;
    (
        (seconds * 0.65).ceil().max(1.0) as u64,
        (seconds * 2.0 + 2.0).ceil().max(2.0) as u64,
    )
}

pub fn analyze_compression(
    paths: &[String],
    format: &str,
    level: u32,
    cancelled: &AtomicBool,
) -> Result<CompressionAnalysisResult> {
    let mut files = collect_sources(paths, cancelled)?;
    let total_size = files
        .iter()
        .try_fold(0u64, |total, file| total.checked_add(file.size))
        .context("Compression analysis source size overflowed")?;
    let file_count = files.len();
    let low_value_bytes = files
        .iter()
        .filter(|file| file.low_value)
        .map(|file| file.size)
        .sum();
    let low_value_file_count = files.iter().filter(|file| file.low_value).count();
    files.sort_by_key(|file| file.score);
    let (sample, sampled_files) = sample_files(&files, cancelled)?;
    check_cancelled(cancelled)?;
    let sample_ratio = if sample.is_empty() {
        1.0
    } else {
        compressed_sample_size(&sample, format, level)? as f64 / sample.len() as f64
    };
    let low_value_share = if total_size == 0 {
        0.0
    } else {
        low_value_bytes as f64 / total_size as f64
    };
    let adjusted_ratio =
        (sample_ratio * (1.0 - low_value_share) + 0.98 * low_value_share).clamp(0.02, 1.08);
    let overhead = 1024u64.saturating_add(file_count as u64 * 96);
    let estimated_size = ((total_size as f64 * adjusted_ratio) as u64).saturating_add(overhead);
    let confidence = if sample.len() >= 1024 * 1024 && sampled_files >= 8 {
        "high"
    } else if sample.len() >= 256 * 1024 && sampled_files >= 3 {
        "medium"
    } else {
        "low"
    };
    let (recommended_format, recommended_level, recommended_solid, mut reasons) =
        if adjusted_ratio >= 0.9 {
            (
                "zip",
                1,
                false,
                vec!["多数内容已经压缩，继续追求高压缩率收益很低".to_string()],
            )
        } else if adjusted_ratio <= 0.5 && total_size >= 8 * 1024 * 1024 {
            (
                "7z",
                7,
                file_count > 1,
                vec!["抽样内容具有较高可压缩性，7Z 更适合减小归档体积".to_string()],
            )
        } else {
            (
                "zip",
                6,
                false,
                vec!["压缩收益与兼容性较均衡，建议使用 ZIP 平衡模式".to_string()],
            )
        };
    if low_value_share >= 0.5 {
        reasons.push(format!(
            "约 {:.0}% 的数据属于图片、视频、文档或既有压缩包，高等级压缩收益有限",
            low_value_share * 100.0
        ));
    }
    reasons.push("耗时为典型设备区间估算，实际速度取决于磁盘、CPU 与文件数量".to_string());
    let (estimated_seconds_low, estimated_seconds_high) =
        duration_range(total_size, format, level, file_count);
    Ok(CompressionAnalysisResult {
        total_size,
        file_count,
        sampled_files,
        sampled_bytes: sample.len() as u64,
        estimated_size,
        estimated_ratio: adjusted_ratio,
        estimated_seconds_low,
        estimated_seconds_high,
        confidence: confidence.to_string(),
        recommended_format: recommended_format.to_string(),
        recommended_level,
        recommended_solid,
        low_value_bytes,
        low_value_file_count,
        reasons,
    })
}

#[cfg(test)]
mod tests {
    use super::analyze_compression;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn real_text_and_precompressed_samples_produce_explainable_different_advice() {
        let temp = tempfile::tempdir().unwrap();
        let text = temp.path().join("large.txt");
        std::fs::write(&text, "Long解压 predictable text\n".repeat(400_000)).unwrap();
        let text_result = analyze_compression(
            &[text.to_string_lossy().to_string()],
            "zip",
            6,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(text_result.estimated_ratio < 0.5);
        assert_eq!(text_result.recommended_format, "7z");
        assert!(text_result.sampled_bytes <= super::MAX_SAMPLE_BYTES as u64);

        let media = temp.path().join("photo.jpg");
        let pseudo_random: Vec<u8> = (0u64..900_000)
            .map(|index| blake3::hash(&index.to_le_bytes()).as_bytes()[0])
            .collect();
        std::fs::write(&media, pseudo_random).unwrap();
        let media_result = analyze_compression(
            &[media.to_string_lossy().to_string()],
            "zip",
            9,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(media_result.recommended_level, 1);
        assert!(media_result.low_value_bytes > 0);
    }

    #[test]
    fn cancellation_stops_before_reading_sources() {
        let result = analyze_compression(
            &["missing.txt".to_string()],
            "zip",
            6,
            &AtomicBool::new(true),
        );
        assert!(result.unwrap_err().to_string().contains("cancelled"));
    }

    #[test]
    fn mixed_directory_reports_low_value_share_with_bounded_sampling() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("mixed");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(
            source.join("notes.txt"),
            "repeatable project notes\n".repeat(200_000),
        )
        .unwrap();
        let media: Vec<u8> = (0u64..6_500_000)
            .map(|index| blake3::hash(&index.to_le_bytes()).as_bytes()[0])
            .collect();
        std::fs::write(source.join("preview.mp4"), media).unwrap();

        let result = analyze_compression(
            &[source.to_string_lossy().to_string()],
            "zip",
            6,
            &AtomicBool::new(false),
        )
        .unwrap();

        assert_eq!(result.file_count, 2);
        assert_eq!(result.low_value_file_count, 1);
        assert!(result.low_value_bytes >= 6_500_000);
        assert!(result.sampled_bytes <= super::MAX_SAMPLE_BYTES as u64);
        assert!(result
            .reasons
            .iter()
            .any(|reason| reason.contains("收益有限")));
    }
}
