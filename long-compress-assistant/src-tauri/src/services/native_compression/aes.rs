use super::{single_stream, tar, CompressionRuntime};
use crate::models::compression::{CompressionOptions, TaskLogSeverity};
use crate::services::aes_wrapper::AesWrapper;
use crate::services::compression_service::CompressionError;
use crate::services::tar_aes_engine::TarAesEngine;
use anyhow::Result;
use std::path::{Path, PathBuf};
use tauri::Window;

#[derive(Clone, Copy)]
pub(crate) enum AesCompressionKind {
    Tar,
    TarGzip,
    TarBzip2,
    TarXz,
    TarZstd,
    Gzip,
    Bzip2,
    Xz,
    Zstd,
}

impl AesCompressionKind {
    fn label(self) -> &'static str {
        match self {
            Self::Tar => "TAR.AES",
            Self::TarGzip => "TAR.GZ.AES",
            Self::TarBzip2 => "TAR.BZ2.AES",
            Self::TarXz => "TAR.XZ.AES",
            Self::TarZstd => "TAR.ZST.AES",
            Self::Gzip => "GZ.AES",
            Self::Bzip2 => "BZ2.AES",
            Self::Xz => "XZ.AES",
            Self::Zstd => "ZST.AES",
        }
    }

    fn temporary_extension(self) -> Option<&'static str> {
        match self {
            Self::Tar => None,
            Self::TarGzip => Some("tar.gz"),
            Self::TarBzip2 => Some("tar.bz2"),
            Self::TarXz => Some("tar.xz"),
            Self::TarZstd => Some("tar.zst"),
            Self::Gzip => Some("gz"),
            Self::Bzip2 => Some("bz2"),
            Self::Xz => Some("xz"),
            Self::Zstd => Some("zst"),
        }
    }
}

struct TemporaryAesInput(PathBuf);

impl TemporaryAesInput {
    fn new(extension: &str) -> Self {
        Self(std::env::temp_dir().join(format!(
            "long-compress-aes-input-{}.{}",
            uuid::Uuid::new_v4(),
            extension,
        )))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TemporaryAesInput {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn encrypt_file<R: CompressionRuntime>(
    runtime: &R,
    input: &Path,
    output: &Path,
    password: &str,
) -> Result<()> {
    AesWrapper::encrypt_file_cancellable(input, output, password, || runtime.check_cancellation())
        .map_err(|error| {
            if matches!(
                error.downcast_ref::<CompressionError>(),
                Some(CompressionError::Cancelled)
            ) {
                CompressionError::Cancelled
            } else {
                CompressionError::CompressionFailed(format!("加密失败: {error}"))
            }
        })
        .map_err(Into::into)
}

pub(crate) fn compress<R: CompressionRuntime>(
    runtime: &R,
    window: Option<&Window>,
    task_id: &str,
    sources: &[String],
    output: &str,
    options: CompressionOptions,
    kind: AesCompressionKind,
) -> Result<()> {
    let label = kind.label();
    if let Some(window) = window {
        runtime.emit_log(
            window,
            task_id,
            &format!("使用 {label} 格式压缩"),
            TaskLogSeverity::Info,
        );
    }
    let password = options
        .password
        .as_deref()
        .ok_or_else(|| CompressionError::CompressionFailed(format!("{label} 格式需要密码")))?;
    if matches!(kind, AesCompressionKind::Tar) && password.is_empty() {
        return Err(CompressionError::CompressionFailed("密码不能为空".to_string()).into());
    }

    if matches!(kind, AesCompressionKind::Tar) {
        let source_paths: Vec<PathBuf> = sources.iter().map(PathBuf::from).collect();
        let base_dir = if sources.len() == 1 {
            Path::new(&sources[0]).parent()
        } else {
            None
        };
        let result = TarAesEngine::compress_tar_aes_cancellable(
            &source_paths,
            Path::new(output),
            password,
            base_dir,
            || runtime.check_cancellation(),
        );
        if let Err(error) = result {
            if matches!(
                error.downcast_ref::<CompressionError>(),
                Some(CompressionError::Cancelled)
            ) {
                return Err(CompressionError::Cancelled.into());
            }
            if let Some(window) = window {
                runtime.emit_log(
                    window,
                    task_id,
                    &format!("TAR.AES 压缩失败: {error}"),
                    TaskLogSeverity::Error,
                );
            }
            return Err(
                CompressionError::CompressionFailed(format!("TAR.AES 压缩失败: {error}")).into(),
            );
        }
    } else {
        let password = password.to_string();
        let temporary = TemporaryAesInput::new(
            kind.temporary_extension()
                .expect("wrapped AES kind has a temporary extension"),
        );
        let plain_options = CompressionOptions {
            password: None,
            ..options
        };
        let temporary_output = temporary.path().to_string_lossy();
        match kind {
            AesCompressionKind::TarGzip => tar::compress_gzip(
                runtime,
                window,
                task_id,
                sources,
                &temporary_output,
                plain_options,
            )?,
            AesCompressionKind::TarBzip2 => tar::compress_bzip2(
                runtime,
                window,
                task_id,
                sources,
                &temporary_output,
                plain_options,
            )?,
            AesCompressionKind::TarXz => tar::compress_xz(
                runtime,
                window,
                task_id,
                sources,
                &temporary_output,
                plain_options,
            )?,
            AesCompressionKind::TarZstd => tar::compress_zstd(
                runtime,
                window,
                task_id,
                sources,
                &temporary_output,
                plain_options,
            )?,
            AesCompressionKind::Gzip => single_stream::compress_gzip(
                runtime,
                window,
                task_id,
                sources,
                &temporary_output,
                plain_options,
            )?,
            AesCompressionKind::Bzip2 => single_stream::compress_bzip2(
                runtime,
                window,
                task_id,
                sources,
                &temporary_output,
                plain_options,
            )?,
            AesCompressionKind::Xz => single_stream::compress_xz(
                runtime,
                window,
                task_id,
                sources,
                &temporary_output,
                plain_options,
            )?,
            AesCompressionKind::Zstd => single_stream::compress_zstd(
                runtime,
                window,
                task_id,
                sources,
                &temporary_output,
                plain_options,
            )?,
            AesCompressionKind::Tar => unreachable!("direct TAR.AES handled above"),
        }
        encrypt_file(runtime, temporary.path(), Path::new(output), &password)?;
    }

    if let Some(window) = window {
        runtime.emit_log(
            window,
            task_id,
            &format!("{label} 压缩完成"),
            TaskLogSeverity::Success,
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    #[derive(Default)]
    struct TestRuntime {
        cancelled: Arc<AtomicBool>,
    }

    impl CompressionRuntime for TestRuntime {
        fn check_cancellation(&self) -> Result<()> {
            if self.cancelled.load(Ordering::Relaxed) {
                Err(CompressionError::Cancelled.into())
            } else {
                Ok(())
            }
        }
        fn cancellation_flag(&self) -> Arc<AtomicBool> {
            self.cancelled.clone()
        }
        fn copy_buffer_size(&self) -> usize {
            256 * 1024
        }
        fn emit_log(
            &self,
            _window: &Window,
            _task_id: &str,
            _message: &str,
            _severity: TaskLogSeverity,
        ) {
        }
        fn emit_progress(
            &self,
            _window: &Window,
            _task_id: &str,
            _progress: f32,
            _current_file: Option<String>,
            _processed_bytes: u64,
            _total_bytes: u64,
        ) {
        }
    }

    #[test]
    fn wrapped_gzip_aes_round_trip_preserves_payload() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("payload.txt");
        let encrypted = temp.path().join("payload.gz.aes");
        let decrypted = temp.path().join("payload.gz");
        std::fs::write(&source, b"wrapped AES payload").expect("write source");

        compress(
            &TestRuntime::default(),
            None,
            "gzip-aes-task",
            &[source.to_string_lossy().to_string()],
            encrypted.to_string_lossy().as_ref(),
            CompressionOptions {
                password: Some("correct-password".to_string()),
                ..CompressionOptions::default()
            },
            AesCompressionKind::Gzip,
        )
        .expect("create gzip AES");
        AesWrapper::decrypt_file(&encrypted, &decrypted, "correct-password")
            .expect("decrypt gzip AES");
        let mut decoder =
            flate2::read::GzDecoder::new(std::fs::File::open(decrypted).expect("open gzip"));
        let mut payload = Vec::new();
        decoder.read_to_end(&mut payload).expect("decode gzip");
        assert_eq!(payload, b"wrapped AES payload");
    }

    #[test]
    fn pre_cancelled_aes_cleans_output() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("payload.txt");
        let output = temp.path().join("payload.gz.aes");
        std::fs::write(&source, vec![3u8; 2 * 1024 * 1024]).expect("write source");
        let runtime = TestRuntime::default();
        runtime.cancelled.store(true, Ordering::Relaxed);

        let error = compress(
            &runtime,
            None,
            "cancelled-aes-task",
            &[source.to_string_lossy().to_string()],
            output.to_string_lossy().as_ref(),
            CompressionOptions {
                password: Some("password".to_string()),
                ..CompressionOptions::default()
            },
            AesCompressionKind::Gzip,
        )
        .expect_err("pre-cancelled AES task must fail");
        assert!(matches!(
            error.downcast_ref::<CompressionError>(),
            Some(CompressionError::Cancelled)
        ));
        assert!(!output.exists());
    }

    #[test]
    fn temporary_input_is_removed_on_drop() {
        let path;
        {
            let input = TemporaryAesInput::new("bin");
            path = input.path().to_path_buf();
            std::fs::write(&path, b"temporary compressed payload").expect("write temp input");
            assert!(path.exists());
        }
        assert!(!path.exists());
    }
}
