use super::native_compression::{tar, CompressionRuntime};
use crate::models::compression::CompressionOptions;
use anyhow::{ensure, Context, Result};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use tauri::Window;

struct TarPreparation<'a, R>(&'a R);
impl<R: CompressionRuntime> CompressionRuntime for TarPreparation<'_, R> {
    fn check_cancellation(&self) -> Result<()> {
        self.0.check_cancellation()
    }
    fn cancellation_flag(&self) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
        self.0.cancellation_flag()
    }
    fn copy_buffer_size(&self) -> usize {
        self.0.copy_buffer_size()
    }
    fn emit_log(
        &self,
        window: &Window,
        task_id: &str,
        message: &str,
        severity: crate::models::compression::TaskLogSeverity,
    ) {
        self.0.emit_log(window, task_id, message, severity);
    }
    fn emit_progress(
        &self,
        window: &Window,
        task_id: &str,
        progress: f32,
        file: Option<String>,
        processed: u64,
        total: u64,
    ) {
        self.0
            .emit_progress(window, task_id, progress * 0.35, file, processed, total);
    }
}

struct CheckedReader<'a, R: CompressionRuntime, T: Read> {
    runtime: &'a R,
    reader: T,
    window: Option<&'a Window>,
    task_id: &'a str,
    processed: u64,
    total: u64,
    progress_start: f32,
}
impl<R: CompressionRuntime, T: Read> Read for CheckedReader<'_, R, T> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.runtime
            .check_cancellation()
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        let read = self.reader.read(buffer)?;
        self.processed += read as u64;
        if let Some(window) = self.window {
            let ratio = if self.total == 0 {
                0.0
            } else {
                self.processed as f32 / self.total as f32
            };
            self.runtime.emit_progress(
                window,
                self.task_id,
                self.progress_start + ratio * (1.0 - self.progress_start),
                None,
                self.processed,
                self.total,
            );
        }
        Ok(read)
    }
}

pub(crate) fn compress<R: CompressionRuntime>(
    runtime: &R,
    window: Option<&Window>,
    task_id: &str,
    sources: &[String],
    output: &str,
    options: CompressionOptions,
    format: &str,
) -> Result<()> {
    runtime.check_cancellation()?;
    let path = Path::new(output);
    let parent = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let tar_source = if format.starts_with("tar.") {
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        let mut builder = ::tar::Builder::new(file.as_file_mut());
        tar::write_entries(
            &TarPreparation(runtime),
            window,
            task_id,
            sources,
            &options,
            &mut builder,
        )?;
        builder.finish()?;
        drop(builder);
        Some(file)
    } else {
        ensure!(
            sources.len() == 1 && Path::new(&sources[0]).is_file(),
            "此格式只支持一个普通文件，请使用 TAR 组合处理目录或多个文件"
        );
        None
    };
    let mut source = if let Some(file) = &tar_source {
        file.reopen()?
    } else {
        File::open(&sources[0])?
    };
    use std::io::{Seek, SeekFrom};
    source.seek(SeekFrom::Start(0))?;
    let total = source.metadata()?.len();
    let mut input = CheckedReader {
        runtime,
        reader: source,
        window,
        task_id,
        processed: 0,
        total,
        progress_start: if tar_source.is_some() { 0.35 } else { 0.0 },
    };
    let destination = File::create(path)?;
    if format.ends_with("lz4") {
        let mut info = lz4_flex::frame::FrameInfo::new();
        info.content_checksum = true;
        info.content_size = Some(total);
        let mut encoder = lz4_flex::frame::FrameEncoder::with_frame_info(info, destination);
        std::io::copy(&mut input, &mut encoder)?;
        encoder.finish()?.sync_all()?;
    } else if format.ends_with(".lz") || format == "lz" {
        let mut settings = lzma_rust2::LzipOptions::with_preset(options.level.clamp(0, 9) as u32);
        // A small input should not allocate the preset's full dictionary.
        settings.lzma_options.dict_size = settings.lzma_options.dict_size
            .min(total.clamp(4096, 512 * 1024 * 1024).next_power_of_two() as u32);
        let mut encoder = lzma_rust2::LzipWriter::new(destination, settings);
        std::io::copy(&mut input, &mut encoder)?;
        runtime.check_cancellation()?;
        encoder.finish()?.sync_all()?;
    } else {
        let mut destination = destination;
        let params = brotli::enc::BrotliEncoderParams {
            quality: options.level.clamp(0, 9) as i32,
            ..Default::default()
        };
        brotli::BrotliCompress(&mut input, &mut destination, &params)?;
        destination.flush()?;
        destination.sync_all()?;
    }
    if let Some(size) = options.split_size.filter(|size| *size > 0) {
        super::split_compression::SplitCompressionService::split_archive_file_with_progress(
            path,
            path,
            size,
            || runtime.check_cancellation(),
            |processed, total| {
                if let Some(window) = window {
                    runtime.emit_split_progress(window, task_id, processed, total);
                }
            },
        )?;
        std::fs::remove_file(path)?;
    }
    Ok(())
}

struct LzipReader<R>(lzma_rust2::LzipReader<R>);
impl<R: Read> Read for LzipReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buffer).map_err(|error| std::io::Error::new(error.kind(), format!("LZIP 读取失败：{error}")))
    }
}

pub(crate) fn decoder(path: &Path, format: &str) -> Result<Box<dyn Read + Send>> {
    let mut input = super::numeric_volume_reader::ZipInputReader::open(path)?;
    Ok(if format.ends_with("lz4") {
        Box::new(lz4_flex::frame::FrameDecoder::new(input))
    } else if format == "lz" || format.ends_with(".lz") {
        // A real .lz contains at least one member; reject empty input before
        // the decoder can interpret it as a clean end of stream.
        use std::io::{Seek, SeekFrom};
        let mut header = [0u8; 6];
        input.read_exact(&mut header).context("LZIP 文件头不完整")?;
        ensure!(header.starts_with(b"LZIP"), "文件不是有效的 LZIP 压缩流");
        input.seek(SeekFrom::Start(0))?;
        Box::new(LzipReader(lzma_rust2::LzipReader::new(input)))
    } else {
        Box::new(brotli::Decompressor::new(input, 64 * 1024))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::compression::{DecompressOptions, TaskLogSeverity};
    use crate::services::io_buffer_pool::IOBufferPool;
    use crate::services::{
        compression_format::CompressionRoute, compression_verification, native_extraction,
    };
    use std::path::PathBuf;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    };

    #[derive(Default)]
    struct Runtime {
        reads: AtomicUsize,
        cancel_after: usize,
        pool: IOBufferPool,
    }
    impl CompressionRuntime for Runtime {
        fn check_cancellation(&self) -> Result<()> {
            if self.cancel_after > 0
                && self.reads.fetch_add(1, Ordering::Relaxed) >= self.cancel_after
            {
                anyhow::bail!("Task cancelled");
            }
            Ok(())
        }
        fn cancellation_flag(&self) -> Arc<AtomicBool> {
            Arc::new(AtomicBool::new(false))
        }
        fn copy_buffer_size(&self) -> usize {
            64 * 1024
        }
        fn emit_log(&self, _: &Window, _: &str, _: &str, _: TaskLogSeverity) {}
        fn emit_progress(&self, _: &Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {}
    }
    impl native_extraction::ExtractionRuntime for Runtime {
        fn check_cancellation(&self) -> Result<()> {
            CompressionRuntime::check_cancellation(self)
        }
        fn buffer_pool(&self) -> &IOBufferPool {
            &self.pool
        }
        fn copy_buffer_size(&self) -> usize {
            64 * 1024
        }
        fn normalized_archive_path(&self, path: &Path, preserve: bool) -> Option<PathBuf> {
            Some(if preserve {
                path.to_path_buf()
            } else {
                PathBuf::from(path.file_name()?)
            })
        }
        fn emit_log(&self, _: &Window, _: &str, _: &str, _: TaskLogSeverity) {}
        fn emit_progress(&self, _: &Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {}
    }

    #[tokio::test]
    async fn reads_fixed_gnu_lzip_members_and_tar_alias() {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/lzip-1412");
        let temp = tempfile::tempdir().unwrap();
        let text = std::fs::read(fixtures.join("原文件/说明.txt")).unwrap();
        for name in ["说明.txt.lz", "资料.tar.lz", "资料别名.tlz", "多成员.txt.lz"] {
            let input = fixtures.join(name);
            let is_tar = name.ends_with(".tar.lz") || name.ends_with(".tlz");
            let browse = crate::services::archive_browser::browse_archive(&input, None).await.unwrap();
            assert_eq!(browse.format, if is_tar { "TAR.LZIP" } else { "LZIP" });
            let route = if is_tar { CompressionRoute::TarLzip } else { CompressionRoute::Lzip };
            assert!(compression_verification::verify_native(route, &input, None, || false).unwrap());
            let output = temp.path().join(name);
            std::fs::create_dir(&output).unwrap();
            if is_tar {
                native_extraction::tar::extract_inner(&Runtime::default(), None, "reference", &input.to_string_lossy(), &output, Some(decoder(&input, "lz").unwrap()), &DecompressOptions { preserve_paths: true, ..Default::default() }).unwrap();
                for payload in ["说明.txt", "中文目录/数据.bin", "空文件.txt"] {
                    assert_eq!(std::fs::read(output.join(payload)).unwrap(), std::fs::read(fixtures.join("原文件").join(payload)).unwrap());
                }
                assert!(output.join("中文目录/空目录").is_dir());
            } else {
                let mut expected = text.clone();
                if name.starts_with("多成员") { expected.extend_from_slice("第二成员：两个成员还原成同一个文件。\n".as_bytes()); }
                let output_name = native_extraction::single_stream::output_name(&input, &[".lz"]);
                native_extraction::single_stream::extract_inner(&Runtime::default(), None, "reference", decoder(&input, "lz").unwrap(), &output, output_name.clone(), &DecompressOptions::default()).unwrap();
                assert_eq!(std::fs::read(output.join(output_name)).unwrap(), expected);
            }
        }
    }

    #[tokio::test]
    async fn lzip_rejects_bad_trailers_even_after_tar_entries_have_ended() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("说明.txt");
        std::fs::write(&source, "完整内容需要正确的 LZIP 校验尾部。".repeat(200)).unwrap();
        for (format, route) in [("lz", CompressionRoute::Lzip), ("tar.lz", CompressionRoute::TarLzip)] {
            let good = temp.path().join(format!("正常.{format}"));
            compress(&Runtime::default(), None, "lzip", &[source.to_string_lossy().into_owned()], &good.to_string_lossy(), CompressionOptions { level: 1, ..Default::default() }, format).unwrap();
            let bytes = std::fs::read(&good).unwrap();
            for mutation in ["crc", "data-size", "member-size", "truncated", "next-header"] {
                let mut bad = bytes.clone();
                let len = bad.len();
                match mutation {
                    "crc" => bad[len - 20] ^= 1,
                    "data-size" => bad[len - 16] ^= 1,
                    "member-size" => bad[len - 8] ^= 1,
                    "truncated" => { bad.truncate(len - 1); },
                    _ => bad.extend_from_slice(b"LZIP\x01"),
                }
                let path = temp.path().join(format!("{mutation}.{format}"));
                std::fs::write(&path, bad).unwrap();
                assert!(compression_verification::verify_native(route, &path, None, || false).is_err(), "{format} {mutation}");
                assert!(crate::services::archive_browser::browse_archive(&path, None).await.is_err(), "{format} {mutation} browse");
            }
        }
        let empty = temp.path().join("空文件.lz");
        std::fs::write(&empty, []).unwrap();
        assert!(decoder(&empty, "lz").is_err());
    }

    #[tokio::test]
    async fn lzip_multimember_and_tlz_alias_preserve_content() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("说明.txt");
        let mut members = Vec::new();
        let mut expected = Vec::new();
        for content in ["第一段中文内容。", "Second member, same logical file."] {
            std::fs::write(&source, content).unwrap();
            let member = temp.path().join("成员.lz");
            compress(&Runtime::default(), None, "member", &[source.to_string_lossy().into_owned()], &member.to_string_lossy(), CompressionOptions { level: 1, ..Default::default() }, "lz").unwrap();
            members.extend(std::fs::read(member).unwrap());
            expected.extend_from_slice(content.as_bytes());
        }
        let joined = temp.path().join("说明.txt.lz");
        std::fs::write(&joined, members).unwrap();
        let mut decoded = Vec::new();
        decoder(&joined, "lz").unwrap().read_to_end(&mut decoded).unwrap();
        assert_eq!(decoded, expected);
        let browse = crate::services::archive_browser::browse_archive(&joined, None).await.unwrap();
        assert_eq!(browse.format, "LZIP");
        assert_eq!(browse.total_uncompressed_size, expected.len() as u64);

        let alias = temp.path().join("中文资料.TLZ");
        compress(&Runtime::default(), None, "alias", &[source.to_string_lossy().into_owned()], &alias.to_string_lossy(), CompressionOptions { level: 1, ..Default::default() }, "tar.lz").unwrap();
        let browse = crate::services::archive_browser::browse_archive(&alias, None).await.unwrap();
        assert_eq!(browse.format, "TAR.LZIP");
        let preview = crate::services::archive_preview::preview_archive_text(&alias, "说明.txt", None).await.unwrap();
        assert_eq!(preview.content, "Second member, same logical file.");
    }

    #[tokio::test]
    async fn tar_codec_numeric_volumes_preview_text_without_extracting_to_disk() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("说明.txt");
        std::fs::write(&source, "中文内容，可以先预览再解压。").unwrap();
        for format in ["tar.lz4", "tar.br"] {
            let output = temp.path().join(format!("资料.{format}"));
            compress(&Runtime::default(), None, "preview", &[source.to_string_lossy().into_owned()], &output.to_string_lossy(), CompressionOptions { split_size: Some(19), ..Default::default() }, format).unwrap();
            let first = crate::services::archive_volumes::first_volume_path(&output);
            let preview = crate::services::archive_preview::preview_archive_text(&first, "说明.txt", None).await.unwrap();
            assert_eq!(preview.content, "中文内容，可以先预览再解压。");
            assert!(!preview.truncated);
        }
    }

    #[tokio::test]
    async fn native_codecs_roundtrip_split_publish_browse_and_preserve_names() {
        use rand::{RngCore, SeedableRng};
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("中文资料");
        std::fs::create_dir_all(source.join("空目录")).unwrap();
        let mut bytes = vec![0u8; 96 * 1024 + 37];
        rand::rngs::StdRng::seed_from_u64(91).fill_bytes(&mut bytes);
        let payload = source.join("原始.txt");
        std::fs::write(&payload, &bytes).unwrap();
        for format in ["lz4", "br", "tar.lz4", "tar.br", "lz", "tar.lz"] {
            for split in [false, true] {
                if split && matches!(format, "lz" | "tar.lz") { continue; }
                let target = temp.path().join(format!("原始.txt-{split}.{format}"));
                let staged = crate::services::compression_service::CompressionService::temporary_compression_output(&target).unwrap();
                let sources = vec![if format.starts_with("tar.") {
                    source.clone()
                } else {
                    payload.clone()
                }
                .to_string_lossy()
                .into_owned()];
                compress(
                    &Runtime::default(),
                    None,
                    "codec",
                    &sources,
                    &staged.to_string_lossy(),
                    CompressionOptions {
                        level: 2,
                        split_size: split.then_some(8192),
                        ..Default::default()
                    },
                    format,
                )
                .unwrap();
                let first = if split {
                    crate::services::archive_volumes::first_volume_path(&staged)
                } else {
                    staged.clone()
                };
                let route = match format {
                    "lz4" => CompressionRoute::Lz4,
                    "br" => CompressionRoute::Brotli,
                    "tar.lz4" => CompressionRoute::TarLz4,
                    "lz" => CompressionRoute::Lzip,
                    "tar.lz" => CompressionRoute::TarLzip,
                    _ => CompressionRoute::TarBrotli,
                };
                assert!(
                    compression_verification::verify_native(route, &first, None, || false).unwrap()
                );
                crate::services::compression_service::CompressionService::finalize_compression_output(Ok(()), &staged, &target, split, false).unwrap();
                let first = if split {
                    crate::services::archive_volumes::first_volume_path(&target)
                } else {
                    target.clone()
                };
                let output = temp.path().join(format!("还原-{format}-{split}"));
                std::fs::create_dir(&output).unwrap();
                let decoded = decoder(&first, format).unwrap();
                if format.starts_with("tar.") {
                    native_extraction::tar::extract_inner(
                        &Runtime::default(),
                        None,
                        "extract",
                        &first.to_string_lossy(),
                        &output,
                        Some(decoded),
                        &DecompressOptions {
                            preserve_paths: true,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                    assert_eq!(
                        std::fs::read(output.join("中文资料/原始.txt")).unwrap(),
                        bytes
                    );
                    assert!(output.join("中文资料/空目录").is_dir());
                } else {
                    let name = native_extraction::single_stream::output_name(
                        &first,
                        &[&format!(".{format}")],
                    );
                    assert_eq!(name, format!("原始.txt-{split}"));
                    native_extraction::single_stream::extract_inner(
                        &Runtime::default(),
                        None,
                        "extract",
                        decoded,
                        &output,
                        name.clone(),
                        &DecompressOptions::default(),
                    )
                    .unwrap();
                    assert_eq!(std::fs::read(output.join(name)).unwrap(), bytes);
                }
                let browse = crate::services::archive_browser::browse_archive(&first, None)
                    .await
                    .unwrap();
                assert_eq!(browse.total_files, 1);
                assert_eq!(browse.total_uncompressed_size, bytes.len() as u64);
                if split {
                    assert!(
                        crate::services::archive_volumes::complete_volume_paths(&target)
                            .unwrap()
                            .len()
                            > 1
                    );
                }
            }
        }
    }

    #[test]
    fn tar_and_single_stream_numeric_volumes_roundtrip_and_reject_truncated_footer() {
        use crate::services::native_compression::single_stream;
        type Encoder =
            fn(&Runtime, Option<&Window>, &str, &[String], &str, CompressionOptions) -> Result<()>;
        let formats: [(&str, CompressionRoute, Encoder); 9] = [
            ("tar", CompressionRoute::Tar, tar::compress_tar),
            ("tar.gz", CompressionRoute::TarGzip, tar::compress_gzip),
            ("tar.bz2", CompressionRoute::TarBzip2, tar::compress_bzip2),
            ("tar.xz", CompressionRoute::TarXz, tar::compress_xz),
            ("tar.zst", CompressionRoute::TarZstd, tar::compress_zstd),
            ("gz", CompressionRoute::Gzip, single_stream::compress_gzip),
            (
                "bz2",
                CompressionRoute::Bzip2,
                single_stream::compress_bzip2,
            ),
            ("xz", CompressionRoute::Xz, single_stream::compress_xz),
            ("zst", CompressionRoute::Zstd, single_stream::compress_zstd),
        ];
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("原文件.txt");
        let bytes = (0..20_039)
            .map(|index| ((index * 17 + index / 13) % 251) as u8)
            .collect::<Vec<_>>();
        std::fs::write(&source, &bytes).unwrap();
        for (format, route, encode) in formats {
            let target = temp.path().join(format!("原文件.txt.{format}"));
            let staged = crate::services::compression_service::CompressionService::temporary_compression_output(&target).unwrap();
            encode(
                &Runtime::default(),
                None,
                "plain",
                &[source.to_string_lossy().into_owned()],
                &staged.to_string_lossy(),
                CompressionOptions::default(),
            )
            .unwrap();
            super::super::split_compression::SplitCompressionService::split_archive_file(
                &staged,
                &staged,
                19,
                || Ok(()),
            )
            .unwrap();
            std::fs::remove_file(&staged).unwrap();
            let first = crate::services::archive_volumes::first_volume_path(&staged);
            assert!(
                compression_verification::verify_native(route, &first, None, || false).unwrap(),
                "{format}"
            );
            crate::services::compression_service::CompressionService::finalize_compression_output(
                Ok(()),
                &staged,
                &target,
                true,
                false,
            )
            .unwrap();
            let first = crate::services::archive_volumes::first_volume_path(&target);
            let input =
                crate::services::numeric_volume_reader::ZipInputReader::open(&first).unwrap();
            let reader: Box<dyn Read + Send> = match format {
                "tar" => Box::new(input),
                "tar.gz" | "gz" => Box::new(flate2::read::GzDecoder::new(input)),
                "tar.bz2" | "bz2" => Box::new(bzip2::read::BzDecoder::new(input)),
                "tar.xz" | "xz" => Box::new(xz2::read::XzDecoder::new(input)),
                _ => Box::new(zstd::stream::read::Decoder::new(input).unwrap()),
            };
            let output = temp.path().join(format!("还原-{format}"));
            std::fs::create_dir(&output).unwrap();
            if format.starts_with("tar") {
                native_extraction::tar::extract_inner(
                    &Runtime::default(),
                    None,
                    "extract",
                    &first.to_string_lossy(),
                    &output,
                    Some(reader),
                    &DecompressOptions::default(),
                )
                .unwrap();
            } else {
                let name =
                    native_extraction::single_stream::output_name(&first, &[&format!(".{format}")]);
                assert_eq!(name, "原文件.txt");
                native_extraction::single_stream::extract_inner(
                    &Runtime::default(),
                    None,
                    "extract",
                    reader,
                    &output,
                    name,
                    &DecompressOptions::default(),
                )
                .unwrap();
            }
            assert_eq!(
                std::fs::read(output.join("原文件.txt")).unwrap(),
                bytes,
                "{format}"
            );
        }
        let gzip = temp.path().join("truncated.tar.gz");
        tar::compress_gzip(
            &Runtime::default(),
            None,
            "gzip",
            &[source.to_string_lossy().into_owned()],
            &gzip.to_string_lossy(),
            CompressionOptions::default(),
        )
        .unwrap();
        let mut bytes = std::fs::read(&gzip).unwrap();
        bytes.truncate(bytes.len() - 2);
        std::fs::write(&gzip, &bytes).unwrap();
        assert!(compression_verification::verify_native(
            CompressionRoute::TarGzip,
            &gzip,
            None,
            || false
        )
        .is_err());
        let output = temp.path().join("损坏还原");
        std::fs::create_dir(&output).unwrap();
        let decoder = flate2::read::GzDecoder::new(File::open(&gzip).unwrap());
        assert!(native_extraction::tar::extract_inner(
            &Runtime::default(),
            None,
            "extract",
            &gzip.to_string_lossy(),
            &output,
            Some(Box::new(decoder)),
            &DecompressOptions::default()
        )
        .is_err());
    }

    #[test]
    fn native_codec_truncation_corruption_and_cancellation_are_reported() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("payload.bin");
        std::fs::write(&source, vec![b'x'; 1024 * 1024]).unwrap();
        for format in ["lz4", "br", "tar.lz4", "tar.br"] {
            let output = temp.path().join(format!("sample.{format}"));
            compress(
                &Runtime::default(),
                None,
                "codec",
                &[source.to_string_lossy().into_owned()],
                &output.to_string_lossy(),
                CompressionOptions::default(),
                format,
            )
            .unwrap();
            let bytes = std::fs::read(&output).unwrap();
            std::fs::write(&output, &bytes[..bytes.len() - 1]).unwrap();
            assert!(
                decoder(&output, format)
                    .unwrap()
                    .read_to_end(&mut Vec::new())
                    .is_err(),
                "truncated {format}"
            );
            std::fs::write(&output, &bytes).unwrap();
            if format.starts_with("tar.") {
                let extract_output = temp.path().join(format!("取消-{format}"));
                std::fs::create_dir(&extract_output).unwrap();
                let failure = native_extraction::tar::extract_inner(
                    &Runtime {
                        cancel_after: 5,
                        ..Default::default()
                    },
                    None,
                    "extract",
                    &output.to_string_lossy(),
                    &extract_output,
                    Some(decoder(&output, format).unwrap()),
                    &DecompressOptions::default(),
                )
                .unwrap_err();
                assert!(failure.to_string().contains("cancelled"), "{failure}");
            }
            if format.ends_with("lz4") {
                let mut corrupt = bytes.clone();
                let last = corrupt.len() - 1;
                corrupt[last] ^= 0xff;
                std::fs::write(&output, corrupt).unwrap();
                assert!(
                    decoder(&output, format)
                        .unwrap()
                        .read_to_end(&mut Vec::new())
                        .is_err(),
                    "checksum {format}"
                );
            }
            let failure = compress(
                &Runtime {
                    cancel_after: 3,
                    ..Default::default()
                },
                None,
                "codec",
                &[source.to_string_lossy().into_owned()],
                &output.to_string_lossy(),
                CompressionOptions::default(),
                format,
            )
            .unwrap_err();
            assert!(failure.to_string().contains("cancelled"), "{failure}");
        }
    }
}
