use super::CompressionRuntime;
use crate::models::compression::CompressionOptions;
use crate::services::compression_entries;
use crate::services::compression_service::CompressionError;
use anyhow::Result;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use tauri::Window;

const PROGRESS_EMIT_INTERVAL_BYTES: u64 = 4 * 1024 * 1024;

struct CancellableProgressReader<R, F> {
    inner: R,
    cancellation_flag: Arc<AtomicBool>,
    on_read: F,
}

impl<R, F> CancellableProgressReader<R, F> {
    fn new(inner: R, cancellation_flag: Arc<AtomicBool>, on_read: F) -> Self {
        Self {
            inner,
            cancellation_flag,
            on_read,
        }
    }
}

impl<R: Read, F: FnMut(u64)> Read for CancellableProgressReader<R, F> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        crate::services::task_control::wait_if_paused(&self.cancellation_flag);
        if self.cancellation_flag.load(Ordering::Relaxed) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "compression cancelled",
            ));
        }
        let read = self.inner.read(buffer)?;
        if read > 0 {
            (self.on_read)(read as u64);
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
) -> Result<()> {
    if !output.to_ascii_lowercase().ends_with(".7z") {
        return Err(CompressionError::CompressionFailed(
            "7z compression output path must end with .7z".to_string(),
        )
        .into());
    }

    if let Some(parent) = Path::new(output).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let preserve_paths = options.preserve_paths.unwrap_or(true);
    let entries = compression_entries::collect(sources, preserve_paths, true)?;
    let total_bytes = entries
        .iter()
        .filter(|entry| !entry.is_dir)
        .try_fold(0u64, |total, entry| {
            Ok::<_, std::io::Error>(total.saturating_add(entry.path.metadata()?.len()))
        })?
        .max(1);
    let mut processed_bytes = 0u64;
    let mut writer = sevenz_rust::SevenZWriter::create(output)
        .map_err(|error| CompressionError::CompressionFailed(error.to_string()))?;

    let level = options.level.clamp(1, 9);
    let lzma_options = sevenz_rust::lzma::LZMA2Options::with_preset(level);
    let mut methods = Vec::new();
    if let Some(password) = options
        .password
        .as_deref()
        .filter(|password| !password.is_empty())
    {
        methods.push(
            sevenz_rust::AesEncoderOptions::new(sevenz_rust::Password::from(password)).into(),
        );
    }
    methods.push(lzma_options.into());
    writer.set_content_methods(methods);

    if options.create_solid_archive {
        let processed = Arc::new(AtomicU64::new(0));
        let last_emitted = Arc::new(AtomicU64::new(0));
        let mut solid_entries = Vec::new();
        let mut solid_readers = Vec::new();

        for entry in &entries {
            runtime.check_cancellation()?;
            let archive_entry = sevenz_rust::SevenZArchiveEntry::from_path(
                &entry.path,
                entry.archive_name.clone(),
            );
            if entry.is_dir {
                writer
                    .push_archive_entry::<&[u8]>(archive_entry, None)
                    .map_err(|error| CompressionError::CompressionFailed(error.to_string()))?;
                continue;
            }

            let processed = processed.clone();
            let last_emitted = last_emitted.clone();
            let current_name = entry.archive_name.clone();
            let progress_reader = CancellableProgressReader::new(
                File::open(&entry.path)?,
                runtime.cancellation_flag(),
                move |read| {
                    let current = processed.fetch_add(read, Ordering::Relaxed) + read;
                    let previous = last_emitted.load(Ordering::Relaxed);
                    if current.saturating_sub(previous) >= PROGRESS_EMIT_INTERVAL_BYTES
                        || current >= total_bytes
                    {
                        last_emitted.store(current, Ordering::Relaxed);
                        if let Some(window) = window {
                            runtime.emit_progress(
                                window,
                                task_id,
                                current as f32 / total_bytes as f32,
                                Some(current_name.clone()),
                                current,
                                total_bytes,
                            );
                        }
                    }
                },
            );
            solid_entries.push(archive_entry);
            solid_readers.push(sevenz_rust::SourceReader::new(progress_reader));
        }

        if !solid_entries.is_empty() {
            if let Err(error) = writer.push_archive_entries(
                solid_entries,
                sevenz_rust::SeqReader::new(solid_readers),
            ) {
                if runtime.cancellation_flag().load(Ordering::Relaxed) {
                    return Err(CompressionError::Cancelled.into());
                }
                return Err(CompressionError::CompressionFailed(error.to_string()).into());
            }
        }
    } else {
        for (index, entry) in entries.iter().enumerate() {
            runtime.check_cancellation()?;
            let archive_entry =
                sevenz_rust::SevenZArchiveEntry::from_path(&entry.path, entry.archive_name.clone());
            if entry.is_dir {
                writer
                    .push_archive_entry::<&[u8]>(archive_entry, None)
                    .map_err(|error| CompressionError::CompressionFailed(error.to_string()))?;
            } else {
                let file = File::open(&entry.path)?;
                let current_name = entry.archive_name.clone();
                let mut last_emitted = processed_bytes;
                let progress_reader =
                    CancellableProgressReader::new(file, runtime.cancellation_flag(), |read| {
                        processed_bytes = processed_bytes.saturating_add(read);
                        if processed_bytes.saturating_sub(last_emitted) >= PROGRESS_EMIT_INTERVAL_BYTES
                            || processed_bytes >= total_bytes
                        {
                            if let Some(window) = window {
                                runtime.emit_progress(
                                    window,
                                    task_id,
                                    processed_bytes as f32 / total_bytes as f32,
                                    Some(current_name.clone()),
                                    processed_bytes,
                                    total_bytes,
                                );
                            }
                            last_emitted = processed_bytes;
                        }
                    });
                if let Err(error) = writer.push_archive_entry(archive_entry, Some(progress_reader)) {
                    if runtime.cancellation_flag().load(Ordering::Relaxed) {
                        return Err(CompressionError::Cancelled.into());
                    }
                    return Err(CompressionError::CompressionFailed(error.to_string()).into());
                }
            }
            if processed_bytes == 0 {
                if let Some(window) = window {
                    runtime.emit_progress(
                        window,
                        task_id,
                        (index + 1) as f32 / entries.len().max(1) as f32,
                        Some(entry.archive_name.clone()),
                        0,
                        0,
                    );
                }
            }
        }
    }

    writer
        .finish()
        .map_err(|error| CompressionError::CompressionFailed(error.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::compression::TaskLogSeverity;

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
    fn plain_seven_zip_round_trip_preserves_payload() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("payload.txt");
        let archive = temp.path().join("payload.7z");
        let extracted = temp.path().join("extracted");
        std::fs::write(&source, b"native seven zip payload").expect("write source");

        compress(
            &TestRuntime::default(),
            None,
            "7z-task",
            &[source.to_string_lossy().to_string()],
            archive.to_string_lossy().as_ref(),
            CompressionOptions::default(),
        )
        .expect("create 7z");
        sevenz_rust::decompress_file(&archive, &extracted).expect("extract 7z");

        assert_eq!(
            std::fs::read(extracted.join("payload.txt")).expect("read extracted payload"),
            b"native seven zip payload"
        );
    }

    #[test]
    fn encrypted_seven_zip_requires_the_requested_password() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("secret.txt");
        let archive = temp.path().join("secret.7z");
        let wrong_password_output = temp.path().join("wrong-password");
        let extracted = temp.path().join("extracted");
        std::fs::write(&source, b"encrypted seven zip payload").expect("write source");
        let options = CompressionOptions {
            password: Some("correct-password".to_string()),
            ..CompressionOptions::default()
        };

        compress(
            &TestRuntime::default(),
            None,
            "encrypted-7z-task",
            &[source.to_string_lossy().to_string()],
            archive.to_string_lossy().as_ref(),
            options,
        )
        .expect("create encrypted 7z");
        assert!(
            sevenz_rust::decompress_file_with_password(
                &archive,
                &wrong_password_output,
                sevenz_rust::Password::from("wrong-password"),
            )
            .is_err(),
            "wrong password must not decrypt the archive"
        );
        sevenz_rust::decompress_file_with_password(
            &archive,
            &extracted,
            sevenz_rust::Password::from("correct-password"),
        )
        .expect("extract encrypted 7z");

        assert_eq!(
            std::fs::read(extracted.join("secret.txt")).expect("read extracted secret"),
            b"encrypted seven zip payload"
        );
    }

    #[test]
    fn solid_option_places_multiple_files_in_one_compression_folder() {
        let temp = tempfile::tempdir().expect("temp dir");
        let first = temp.path().join("first.txt");
        let second = temp.path().join("second.txt");
        let archive = temp.path().join("solid.7z");
        std::fs::write(&first, b"shared-prefix-first").expect("write first");
        std::fs::write(&second, b"shared-prefix-second").expect("write second");

        compress(
            &TestRuntime::default(),
            None,
            "solid-7z-task",
            &[
                first.to_string_lossy().to_string(),
                second.to_string_lossy().to_string(),
            ],
            archive.to_string_lossy().as_ref(),
            CompressionOptions {
                password: Some("solid-password".to_string()),
                create_solid_archive: true,
                ..CompressionOptions::default()
            },
        )
        .expect("create solid 7z");

        let mut archive_file = File::open(&archive).expect("open solid archive");
        let archive_len = archive_file.metadata().expect("archive metadata").len();
        let password = sevenz_rust::Password::from("solid-password");
        let metadata = sevenz_rust::Archive::read(
            &mut archive_file,
            archive_len,
            password.as_slice(),
        )
            .expect("read solid metadata");

        assert_eq!(metadata.folders.len(), 1);
        assert_eq!(metadata.files.iter().filter(|entry| entry.has_stream).count(), 2);
        let extracted = temp.path().join("solid-output");
        sevenz_rust::decompress_file_with_password(&archive, &extracted, password)
            .expect("extract encrypted solid archive");
        assert_eq!(std::fs::read(extracted.join("first.txt")).expect("first output"), b"shared-prefix-first");
        assert_eq!(std::fs::read(extracted.join("second.txt")).expect("second output"), b"shared-prefix-second");
    }

    #[test]
    fn cancellation_is_preserved_as_cancelled() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("payload.txt");
        let archive = temp.path().join("cancelled.7z");
        std::fs::write(&source, b"cancelled payload").expect("write source");
        let runtime = TestRuntime::default();
        runtime.cancelled.store(true, Ordering::Relaxed);

        let error = compress(
            &runtime,
            None,
            "cancelled-7z-task",
            &[source.to_string_lossy().to_string()],
            archive.to_string_lossy().as_ref(),
            CompressionOptions::default(),
        )
        .expect_err("cancelled 7z must fail");

        assert!(matches!(
            error.downcast_ref::<CompressionError>(),
            Some(CompressionError::Cancelled)
        ));
    }
}
