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

#[cfg(test)]
fn compressed_sample_size(sample: &[u8], format: &str, level: u32) -> Result<usize> {
    compressed_sample_size_with_cancel(sample, format, level, &AtomicBool::new(false))
}

fn compressed_sample_size_with_cancel(sample: &[u8], format: &str, level: u32, cancelled: &AtomicBool) -> Result<usize> {
    if format == "zip-ppmd" || format == "zip-lzma" {
        // Use the selected real ZIP writer, including method parameters and stream finalization.
        let mut archive = zip_aes::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        check_cancelled(cancelled)?;
        archive.start_file("sample", crate::services::zip_method::file_options(false, if format == "zip-lzma" { crate::services::zip_method::ZipMethod::Lzma } else { crate::services::zip_method::ZipMethod::Ppmd }, level))?;
        for chunk in sample.chunks(SAMPLE_CHUNK_BYTES) {
            check_cancelled(cancelled)?;
            archive.write_all(chunk)?;
        }
        check_cancelled(cancelled)?;
        let output = archive.finish()?;
        check_cancelled(cancelled)?;
        return Ok(output.into_inner().len());
    }
    let format = format.strip_suffix(".aes").unwrap_or(format);
    if format == "tar" || (format == "wim" && level == 0) {
        return Ok(sample.len());
    }
    let level = level.min(9);
    if format.contains("bz2") {
        let mut encoder = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::new(level.clamp(1, 9)));
        encoder.write_all(sample)?;
        return Ok(encoder.finish()?.len());
    }
    if format.ends_with("lz4") {
        let mut encoder = lz4_flex::frame::FrameEncoder::new(Vec::new());
        encoder.write_all(sample)?;
        return Ok(encoder.finish()?.len());
    }
    if matches!(format, "br" | "tar.br") {
        let mut output = Vec::new();
        brotli::BrotliCompress(&mut std::io::Cursor::new(sample), &mut output,
            &brotli::enc::BrotliEncoderParams { quality: level as i32, ..Default::default() })?;
        return Ok(output.len());
    }
    if matches!(format, "lz" | "tar.lz") {
        let mut settings = lzma_rust2::LzipOptions::with_preset(level);
        settings.lzma_options.dict_size = settings.lzma_options.dict_size
            .min(sample.len().clamp(4096, 512 * 1024 * 1024).next_power_of_two() as u32);
        let mut encoder = lzma_rust2::LzipWriter::new(Vec::new(), settings);
        encoder.write_all(sample)?;
        return Ok(encoder.finish()?.len());
    }
    if matches!(format, "lzma" | "tar.lzma") {
        let options = xz2::stream::LzmaOptions::new_preset(level.clamp(1, 9))?;
        let stream = xz2::stream::Stream::new_lzma_encoder(&options)?;
        let mut encoder = xz2::write::XzEncoder::new_stream(Vec::new(), stream);
        encoder.write_all(sample)?;
        return Ok(encoder.finish()?.len());
    }
    if format.contains("xz") || format == "7z" {
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
    let format = format.strip_suffix(".aes").unwrap_or(format);
    let base_mib_per_second = match format {
        "7z" | "xz" | "tar.xz" | "lzma" | "tar.lzma" | "zip-lzma" | "lz" | "tar.lz" => 45.0,
        "bz2" | "tar.bz2" => 35.0,
        "zip-ppmd" => 12.0,
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
        compressed_sample_size_with_cancel(&sample, format, level, cancelled)? as f64 / sample.len() as f64
    };
    check_cancelled(cancelled)?;
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

#[cfg(test)]
mod aes1521_analysis_tests {
    #[test]
    fn aes1521_analysis_samples_the_actual_codec_under_the_wrapper() {
        let sample = "真正按内层格式抽样。".repeat(512);
        for codec in ["lz4", "br", "lz", "tar.lz4", "tar.br", "tar.lz"] {
            assert_eq!(super::compressed_sample_size(sample.as_bytes(), codec, 1).unwrap(),
                super::compressed_sample_size(sample.as_bytes(), &format!("{codec}.aes"), 1).unwrap());
        }
    }
}

#[cfg(test)]
#[test]
fn lzma1523_analysis_uses_alone_framing_from_independent_fixture() {
    use std::io::Read;
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/lzma-1523/说明.txt.lzma");
    let mut bytes = vec![];
    super::codec_stream::decoder(&path, "lzma").unwrap().read_to_end(&mut bytes).unwrap();
    assert_eq!(compressed_sample_size(&bytes, "lzma", 1).unwrap(), std::fs::metadata(path).unwrap().len() as usize);
}

#[cfg(test)]
mod ppmd_1535_tests {
    use super::*;
    #[test]
    fn zip1535_analysis_uses_the_real_ppmd_writer_and_honors_cancellation() {
        let data="PPMd 文本样本与真实归档参数。\n".repeat(500);
        let mut writer=zip_aes::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        writer.start_file("sample",crate::services::zip_method::file_options(false,crate::services::zip_method::ZipMethod::Ppmd,0)).unwrap();
        writer.write_all(data.as_bytes()).unwrap();let expected=writer.finish().unwrap().into_inner();
        assert_eq!(compressed_sample_size(data.as_bytes(),"zip-ppmd",0).unwrap(),expected.len());
        assert!(compressed_sample_size_with_cancel(data.as_bytes(),"zip-ppmd",9,&AtomicBool::new(true)).is_err());
    }
    #[test]
    fn zip1538_analysis_uses_the_real_lzma_writer_and_honors_cancellation() {
        let data="LZMA 文本样本与真实归档参数。\n".repeat(500);
        let mut writer=zip_aes::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        writer.start_file("sample",crate::services::zip_method::file_options(false,crate::services::zip_method::ZipMethod::Lzma,0)).unwrap();
        writer.write_all(data.as_bytes()).unwrap();let expected=writer.finish().unwrap().into_inner();
        assert_eq!(compressed_sample_size(data.as_bytes(),"zip-lzma",0).unwrap(),expected.len());
        assert!(compressed_sample_size_with_cancel(data.as_bytes(),"zip-lzma",9,&AtomicBool::new(true)).is_err());
    }
}
