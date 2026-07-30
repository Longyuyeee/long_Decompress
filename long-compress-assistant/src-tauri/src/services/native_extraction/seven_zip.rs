use super::ExtractionRuntime;
use crate::models::compression::{DecompressOptions, TaskLogSeverity};
use crate::services::compression_service::CompressionError;
use crate::services::extraction_transaction;
use anyhow::Result;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::Window;

const PROGRESS_EMIT_INTERVAL_BYTES: u64 = 4 * 1024 * 1024;

struct IncompleteExtractedFile {
    path: PathBuf,
    complete: bool,
}

impl IncompleteExtractedFile {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            complete: false,
        }
    }

    fn complete(&mut self) {
        self.complete = true;
    }
}

impl Drop for IncompleteExtractedFile {
    fn drop(&mut self) {
        if !self.complete {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

pub(crate) fn requires_password(file_path: &Path) -> Result<bool> {
    let mut archive_file = File::open(file_path)?;
    let len = archive_file.metadata()?.len();
    match sevenz_rust::Archive::read(&mut archive_file, len, &[]) {
        Ok(archive) => Ok(archive.folders.iter().any(|folder| {
            folder.coders.iter().any(|coder| {
                coder.decompression_method_id() == sevenz_rust::SevenZMethod::ID_AES256SHA256
            })
        })),
        Err(error) => {
            let message = error.to_string().to_ascii_lowercase();
            if message.contains("password")
                || message.contains("encrypted")
                || message.contains("aes")
            {
                Ok(true)
            } else {
                Err(CompressionError::ExtractionFailed(format!(
                    "Unable to inspect 7z encryption metadata: {error}"
                ))
                .into())
            }
        }
    }
}

fn classify_error(
    error: sevenz_rust::Error,
    archive_encrypted: bool,
    password_provided: bool,
) -> CompressionError {
    if matches!(
        &error,
        sevenz_rust::Error::Io(io_error, _)
            if io_error.kind() == std::io::ErrorKind::StorageFull
    ) {
        return CompressionError::DiskFull;
    }

    if archive_encrypted {
        if !password_provided {
            return CompressionError::PasswordRequired;
        }

        let error_message = error.to_string().to_ascii_lowercase();
        let invalid_decoder_input = matches!(
            &error,
            sevenz_rust::Error::Io(io_error, _)
                if io_error.kind() == std::io::ErrorKind::InvalidInput
        );
        let password_failure = matches!(
            &error,
            sevenz_rust::Error::PasswordRequired | sevenz_rust::Error::ChecksumVerificationFailed
        ) || invalid_decoder_input
            || error_message.contains("checksumverificationfailed")
            || error_message.contains("corrupted input data");
        if password_failure {
            return CompressionError::InvalidPassword;
        }
    }

    CompressionError::ExtractionFailed(error.to_string())
}

fn into_sevenz_error(error: anyhow::Error) -> sevenz_rust::Error {
    if let Some(io_error) = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<std::io::Error>())
    {
        return sevenz_rust::Error::io(std::io::Error::new(
            io_error.kind(),
            error.to_string(),
        ));
    }
    sevenz_rust::Error::other(error.to_string())
}

fn is_data_corruption(error: &sevenz_rust::Error) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    matches!(
        error,
        sevenz_rust::Error::ChecksumVerificationFailed | sevenz_rust::Error::NextHeaderCrcMismatch
    ) || matches!(
        error,
        sevenz_rust::Error::Io(io_error, _)
            if io_error.kind() == std::io::ErrorKind::InvalidInput
    ) || message.contains("checksumverificationfailed")
        || message.contains("corrupted input data")
        || message.contains("dist overflow")
        || message.contains("crc mismatch")
}

fn apply_entry_mtime(
    target: &Path,
    entry: &sevenz_rust::SevenZArchiveEntry,
    options: &DecompressOptions,
) -> Result<()> {
    if (options.preserve_timestamps || options.extract_only_newer) && entry.has_last_modified_date {
        let modified: std::time::SystemTime = entry.last_modified_date().into();
        filetime::set_file_mtime(target, filetime::FileTime::from_system_time(modified))?;
    }
    Ok(())
}

fn copy_with_progress<Rt, Rd, Wr, F>(
    runtime: &Rt,
    reader: &mut Rd,
    writer: &mut Wr,
    buffer: &mut [u8],
    mut on_chunk: F,
) -> Result<u64>
where
    Rt: ExtractionRuntime,
    Rd: Read + ?Sized,
    Wr: Write,
    F: FnMut(u64),
{
    let mut copied = 0u64;
    loop {
        runtime.check_cancellation()?;
        let read = reader.read(buffer)?;
        if read == 0 {
            break;
        }
        runtime.write_extracted_chunk(writer, &buffer[..read])?;
        let read = read as u64;
        copied = copied.checked_add(read).ok_or_else(|| {
            CompressionError::ExtractionFailed(
                "Copied byte count overflowed the supported range".to_string(),
            )
        })?;
        on_chunk(read);
    }
    Ok(copied)
}

pub(crate) fn extract<R: ExtractionRuntime>(
    runtime: &R,
    window: Option<&Window>,
    task_id: &str,
    file: &str,
    output: &str,
    password: Option<&str>,
    options: &DecompressOptions,
) -> Result<()> {
    let archive_encrypted = requires_password(Path::new(file))?;
    let output_root = PathBuf::from(output);
    let opts = options.clone();
    let file_filter = extraction_transaction::compile_file_filter(opts.file_filter.as_deref());
    let mut copy_buffer = vec![0u8; runtime.copy_buffer_size()];
    let mut processed = 0usize;
    let total_uncompressed_bytes = {
        let archive_file = File::open(file);
        archive_file
            .and_then(|mut archive_file| {
                let len = archive_file.metadata()?.len();
                let archive = if let Some(password) = password {
                    let password = sevenz_rust::Password::from(password);
                    sevenz_rust::Archive::read(&mut archive_file, len, password.as_slice())
                        .map_err(|error| std::io::Error::other(error.to_string()))
                } else {
                    sevenz_rust::Archive::read(&mut archive_file, len, &[])
                        .map_err(|error| std::io::Error::other(error.to_string()))
                }?;
                Ok(archive
                    .files
                    .iter()
                    .filter(|entry| !entry.is_directory())
                    .fold(0u64, |total, entry| total.saturating_add(entry.size()))
                    .max(1))
            })
            .unwrap_or(1)
    };
    let mut processed_bytes = 0u64;
    let mut last_emitted_bytes = 0u64;

    let mut extract_entry = |entry: &sevenz_rust::SevenZArchiveEntry,
                             reader: &mut dyn Read,
                             _default_dest: &PathBuf|
     -> Result<bool, sevenz_rust::Error> {
        runtime
            .check_cancellation()
            .map_err(|error| sevenz_rust::Error::other(error.to_string()))?;

        let relative =
            match runtime.normalized_archive_path(Path::new(entry.name()), opts.preserve_paths) {
                Some(path) => path,
                None => {
                    copy_with_progress(
                        runtime,
                        reader,
                        &mut std::io::sink(),
                        &mut copy_buffer,
                        |read| {
                            processed_bytes = processed_bytes.saturating_add(read);
                            if processed_bytes.saturating_sub(last_emitted_bytes)
                                >= PROGRESS_EMIT_INTERVAL_BYTES
                                || processed_bytes >= total_uncompressed_bytes
                            {
                                if let Some(window) = window {
                                    runtime.emit_progress(
                                        window,
                                        task_id,
                                        processed_bytes as f32 / total_uncompressed_bytes as f32,
                                        Some(entry.name().to_string()),
                                        processed_bytes,
                                        total_uncompressed_bytes,
                                    );
                                }
                                last_emitted_bytes = processed_bytes;
                            }
                        },
                    )
                    .map_err(into_sevenz_error)?;
                    return Ok(true);
                }
            };

        if !extraction_transaction::matches_compiled_file_filter(&relative, &file_filter) {
            let current_file = relative.to_string_lossy().to_string();
            copy_with_progress(
                runtime,
                reader,
                &mut std::io::sink(),
                &mut copy_buffer,
                |read| {
                    processed_bytes = processed_bytes.saturating_add(read);
                    if processed_bytes.saturating_sub(last_emitted_bytes)
                        >= PROGRESS_EMIT_INTERVAL_BYTES
                        || processed_bytes >= total_uncompressed_bytes
                    {
                        if let Some(window) = window {
                            runtime.emit_progress(
                                window,
                                task_id,
                                processed_bytes as f32 / total_uncompressed_bytes as f32,
                                Some(current_file.clone()),
                                processed_bytes,
                                total_uncompressed_bytes,
                            );
                        }
                        last_emitted_bytes = processed_bytes;
                    }
                },
            )
            .map_err(into_sevenz_error)?;
            return Ok(true);
        }

        let entry_result = (|| -> Result<()> {
            let target = output_root.join(&relative);
            if entry.is_directory() {
                std::fs::create_dir_all(&target)?;
                return Ok(());
            }

            let target = extraction_transaction::resolve_extract_path(&target, &opts)?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let mut incomplete_file = IncompleteExtractedFile::new(target.clone());
            let mut outfile = File::create(&target)?;
            let current_file = relative.to_string_lossy().to_string();
            copy_with_progress(runtime, reader, &mut outfile, &mut copy_buffer, |read| {
                processed_bytes = processed_bytes.saturating_add(read);
                if processed_bytes.saturating_sub(last_emitted_bytes)
                    >= PROGRESS_EMIT_INTERVAL_BYTES
                    || processed_bytes >= total_uncompressed_bytes
                {
                    if let Some(window) = window {
                        runtime.emit_progress(
                            window,
                            task_id,
                            processed_bytes as f32 / total_uncompressed_bytes as f32,
                            Some(current_file.clone()),
                            processed_bytes,
                            total_uncompressed_bytes,
                        );
                    }
                    last_emitted_bytes = processed_bytes;
                }
            })?;
            outfile.flush()?;
            drop(outfile);
            apply_entry_mtime(&target, entry, &opts)?;
            incomplete_file.complete();
            processed += 1;
            Ok(())
        })();

        if let Err(error) = entry_result {
            copy_with_progress(
                runtime,
                reader,
                &mut std::io::sink(),
                &mut copy_buffer,
                |_| {},
            )
            .map_err(into_sevenz_error)?;
            if error.chain().any(|cause| {
                cause
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|io_error| io_error.kind() == std::io::ErrorKind::StorageFull)
            }) {
                return Err(into_sevenz_error(error));
            }
            if opts.skip_corrupted {
                if let Some(window) = window {
                    runtime.emit_log(
                        window,
                        task_id,
                        &format!("Skipped 7z entry {}: {}", entry.name(), error),
                        TaskLogSeverity::Warning,
                    );
                }
                return Ok(true);
            }
            return Err(into_sevenz_error(error));
        }

        Ok(true)
    };

    let result = if let Some(password) = password {
        sevenz_rust::decompress_with_extract_fn_and_password(
            File::open(file)?,
            output,
            sevenz_rust::Password::from(password),
            &mut extract_entry,
        )
    } else {
        sevenz_rust::decompress_file_with_extract_fn(file, output, &mut extract_entry)
    };

    if let Err(error) = result {
        if runtime.check_cancellation().is_err() {
            return Err(CompressionError::Cancelled.into());
        }
        if opts.skip_corrupted && !archive_encrypted && is_data_corruption(&error) {
            if let Some(window) = window {
                runtime.emit_log(
                    window,
                    task_id,
                    &format!("Skipped corrupted 7z data stream: {error}"),
                    TaskLogSeverity::Warning,
                );
            }
        } else {
            return Err(classify_error(error, archive_encrypted, password.is_some()).into());
        }
    }

    if processed == 0 {
        if let Some(window) = window {
            runtime.emit_log(
                window,
                task_id,
                "No 7z entries matched the current extraction options.",
                TaskLogSeverity::Warning,
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::extraction_transaction::ExtractionStaging;
    use crate::services::io_buffer_pool::IOBufferPool;
    use std::path::Component;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    struct TestRuntime {
        cancelled: AtomicBool,
        buffer_pool: IOBufferPool,
        storage_bytes_remaining: Mutex<Option<usize>>,
    }

    impl Default for TestRuntime {
        fn default() -> Self {
            Self {
                cancelled: AtomicBool::new(false),
                buffer_pool: IOBufferPool::default(),
                storage_bytes_remaining: Mutex::new(None),
            }
        }
    }

    impl TestRuntime {
        fn cancel(&self) {
            self.cancelled.store(true, Ordering::SeqCst);
        }

        fn storage_full_after(bytes: usize) -> Self {
            Self {
                storage_bytes_remaining: Mutex::new(Some(bytes)),
                ..Self::default()
            }
        }
    }

    impl ExtractionRuntime for TestRuntime {
        fn check_cancellation(&self) -> Result<()> {
            if self.cancelled.load(Ordering::Relaxed) {
                Err(CompressionError::Cancelled.into())
            } else {
                Ok(())
            }
        }

        fn write_extracted_chunk(&self, writer: &mut dyn Write, bytes: &[u8]) -> Result<()> {
            let mut remaining = self
                .storage_bytes_remaining
                .lock()
                .expect("storage fault lock");
            let Some(available) = *remaining else {
                writer.write_all(bytes)?;
                return Ok(());
            };
            let writable = available.min(bytes.len());
            if writable > 0 {
                writer.write_all(&bytes[..writable])?;
            }
            *remaining = Some(available.saturating_sub(writable));
            if writable < bytes.len() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::StorageFull,
                    "injected extraction storage exhaustion",
                )
                .into());
            }
            Ok(())
        }

        fn buffer_pool(&self) -> &IOBufferPool {
            &self.buffer_pool
        }

        fn copy_buffer_size(&self) -> usize {
            256 * 1024
        }

        fn normalized_archive_path(&self, path: &Path, preserve_paths: bool) -> Option<PathBuf> {
            let source = if preserve_paths {
                path.to_path_buf()
            } else {
                path.file_name().map(PathBuf::from)?
            };
            let mut safe_path = PathBuf::new();
            for component in source.components() {
                if let Component::Normal(part) = component {
                    safe_path.push(part);
                }
            }
            (!safe_path.as_os_str().is_empty()).then_some(safe_path)
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

    fn create_fixture(archive: &Path, entries: &[(&Path, &str)]) {
        let mut writer = sevenz_rust::SevenZWriter::create(archive).expect("create 7z writer");
        for (source, name) in entries {
            let entry = sevenz_rust::SevenZArchiveEntry::from_path(source, (*name).to_string());
            writer
                .push_archive_entry(entry, Some(File::open(source).expect("open fixture entry")))
                .expect("write fixture entry");
        }
        writer.finish().expect("finish 7z fixture");
    }

    fn corrupt_first_packed_stream(archive: &Path) {
        let mut archive_file = File::open(archive).expect("open 7z fixture");
        let archive_len = archive_file.metadata().expect("archive metadata").len();
        let metadata = sevenz_rust::Archive::read(&mut archive_file, archive_len, &[])
            .expect("read archive metadata");
        let packed_size = *metadata.pack_sizes.first().expect("packed stream");
        let corrupt_offset = 32 + metadata.pack_pos + packed_size / 2;
        let mut bytes = std::fs::read(archive).expect("read archive bytes");
        bytes[corrupt_offset as usize] ^= 0x5a;
        std::fs::write(archive, bytes).expect("corrupt packed payload");
    }

    #[test]
    fn encryption_detection_reads_metadata_without_testing_payload() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("payload.bin");
        let plain = temp.path().join("plain.7z");
        let encrypted = temp.path().join("encrypted.7z");
        std::fs::write(&source, vec![5u8; 64 * 1024]).expect("write fixture");
        sevenz_rust::compress_to_path(&source, &plain).expect("plain 7z");
        sevenz_rust::compress_to_path_encrypted(
            &source,
            &encrypted,
            sevenz_rust::Password::from("correct-password"),
        )
        .expect("encrypted 7z");

        assert!(!requires_password(&plain).expect("inspect plain 7z"));
        assert!(requires_password(&encrypted).expect("inspect encrypted 7z"));
    }

    #[test]
    fn corrupt_plain_crc_is_not_classified_as_a_password_failure() {
        let errors = [
            sevenz_rust::Error::ChecksumVerificationFailed,
            sevenz_rust::Error::NextHeaderCrcMismatch,
            sevenz_rust::Error::other("CRC mismatch"),
        ];
        for error in errors {
            assert!(matches!(
                classify_error(error, false, false),
                CompressionError::ExtractionFailed(_)
            ));
        }
    }

    #[test]
    fn real_corrupt_plain_archive_does_not_enter_password_flow() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("plain-payload.bin");
        let archive = temp.path().join("corrupt-plain.7z");
        let payload: Vec<u8> = (0..256 * 1024)
            .map(|index| ((index * 31 + index / 251) % 256) as u8)
            .collect();
        std::fs::write(&source, payload).expect("write source");
        sevenz_rust::compress_to_path(&source, &archive).expect("create plain 7z");
        assert!(!requires_password(&archive).expect("plain archive encryption state"));
        corrupt_first_packed_stream(&archive);

        let mut drain_entry = |_entry: &sevenz_rust::SevenZArchiveEntry,
                               reader: &mut dyn Read,
                               _destination: &PathBuf|
         -> Result<bool, sevenz_rust::Error> {
            std::io::copy(reader, &mut std::io::sink()).map_err(sevenz_rust::Error::io)?;
            Ok(true)
        };
        let error = sevenz_rust::decompress_file_with_extract_fn(
            &archive,
            temp.path().join("corrupt-output"),
            &mut drain_entry,
        )
        .expect_err("corrupt plain archive must fail");
        assert!(matches!(
            classify_error(error, false, false),
            CompressionError::ExtractionFailed(_)
        ));
    }

    #[test]
    fn encrypted_password_failures_are_classified_from_archive_metadata() {
        assert!(matches!(
            classify_error(sevenz_rust::Error::PasswordRequired, true, false),
            CompressionError::PasswordRequired
        ));
        assert!(matches!(
            classify_error(sevenz_rust::Error::ChecksumVerificationFailed, true, true,),
            CompressionError::InvalidPassword
        ));
        assert!(matches!(
            classify_error(
                sevenz_rust::Error::io(std::io::Error::other(
                    sevenz_rust::Error::ChecksumVerificationFailed,
                )),
                true,
                true,
            ),
            CompressionError::InvalidPassword
        ));
    }

    #[test]
    fn real_encrypted_archive_covers_missing_wrong_and_correct_passwords() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("secret.txt");
        let archive = temp.path().join("secret.7z");
        std::fs::write(&source, b"real encrypted seven zip fixture").expect("write source");
        sevenz_rust::compress_to_path_encrypted(
            &source,
            &archive,
            sevenz_rust::Password::from("correct-password"),
        )
        .expect("create encrypted 7z");
        assert!(requires_password(&archive).expect("encrypted archive state"));

        let drain = || {
            |_entry: &sevenz_rust::SevenZArchiveEntry,
             reader: &mut dyn Read,
             _destination: &PathBuf|
             -> Result<bool, sevenz_rust::Error> {
                std::io::copy(reader, &mut std::io::sink()).map_err(sevenz_rust::Error::io)?;
                Ok(true)
            }
        };
        let mut missing_entry = drain();
        let missing = sevenz_rust::decompress_file_with_extract_fn(
            &archive,
            temp.path().join("missing-password"),
            &mut missing_entry,
        )
        .expect_err("missing password must fail");
        assert!(matches!(
            classify_error(missing, true, false),
            CompressionError::PasswordRequired
        ));

        let mut wrong_entry = drain();
        let wrong = sevenz_rust::decompress_with_extract_fn_and_password(
            File::open(&archive).expect("open encrypted fixture"),
            temp.path().join("wrong-password"),
            sevenz_rust::Password::from("wrong-password"),
            &mut wrong_entry,
        )
        .expect_err("wrong password must fail");
        let wrong_debug = format!("{wrong:?}");
        assert!(
            matches!(
                classify_error(wrong, true, true),
                CompressionError::InvalidPassword
            ),
            "unexpected wrong-password error: {wrong_debug}"
        );

        let mut correct_entry = drain();
        sevenz_rust::decompress_with_extract_fn_and_password(
            File::open(&archive).expect("open encrypted fixture"),
            temp.path().join("correct-password"),
            sevenz_rust::Password::from("correct-password"),
            &mut correct_entry,
        )
        .expect("correct password must extract");
    }

    #[test]
    fn entry_timestamp_is_restored_for_preserve_and_newer_modes() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("timestamp-source.txt");
        let archive = temp.path().join("timestamp.7z");
        let output = temp.path().join("timestamp-output.txt");
        let expected = filetime::FileTime::from_unix_time(1_700_000_000, 0);
        std::fs::write(&source, b"timestamp fixture").expect("write source");
        filetime::set_file_mtime(&source, expected).expect("set source timestamp");
        sevenz_rust::compress_to_path(&source, &archive).expect("create 7z fixture");

        let mut archive_file = File::open(&archive).expect("open 7z fixture");
        let archive_len = archive_file.metadata().expect("archive metadata").len();
        let metadata = sevenz_rust::Archive::read(&mut archive_file, archive_len, &[])
            .expect("read 7z metadata");
        let entry = metadata
            .files
            .iter()
            .find(|entry| !entry.is_directory())
            .expect("file entry");
        std::fs::write(&output, b"output").expect("write output");

        apply_entry_mtime(
            &output,
            entry,
            &DecompressOptions {
                preserve_timestamps: true,
                ..Default::default()
            },
        )
        .expect("restore preserved timestamp");
        let restored = filetime::FileTime::from_last_modification_time(
            &std::fs::metadata(&output).expect("output metadata"),
        );
        assert_eq!(restored.unix_seconds(), expected.unix_seconds());

        filetime::set_file_mtime(&output, filetime::FileTime::from_unix_time(2_000, 0))
            .expect("reset output timestamp");
        apply_entry_mtime(
            &output,
            entry,
            &DecompressOptions {
                extract_only_newer: true,
                ..Default::default()
            },
        )
        .expect("restore timestamp for newer comparison");
        let restored_for_newer = filetime::FileTime::from_last_modification_time(
            &std::fs::metadata(&output).expect("output metadata"),
        );
        assert_eq!(restored_for_newer.unix_seconds(), expected.unix_seconds());
    }

    #[test]
    fn timestamp_is_unchanged_when_timestamp_options_are_disabled() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("source.txt");
        let output = temp.path().join("output.txt");
        std::fs::write(&source, b"source").expect("write source");
        std::fs::write(&output, b"output").expect("write output");
        filetime::set_file_mtime(
            &source,
            filetime::FileTime::from_unix_time(1_700_000_000, 0),
        )
        .expect("source timestamp");
        let unchanged = filetime::FileTime::from_unix_time(2_000, 0);
        filetime::set_file_mtime(&output, unchanged).expect("output timestamp");
        let entry = sevenz_rust::SevenZArchiveEntry::from_path(&source, "source.txt".to_string());

        apply_entry_mtime(&output, &entry, &DecompressOptions::default())
            .expect("leave timestamp unchanged");
        let actual = filetime::FileTime::from_last_modification_time(
            &std::fs::metadata(&output).expect("output metadata"),
        );
        assert_eq!(actual.unix_seconds(), unchanged.unix_seconds());
    }

    #[test]
    fn real_filter_extracts_only_matching_entries() {
        let temp = tempfile::tempdir().expect("temp dir");
        let keep = temp.path().join("keep.txt");
        let drop = temp.path().join("drop.bin");
        let archive = temp.path().join("filter.7z");
        let output = temp.path().join("filter-output");
        std::fs::write(&keep, b"keep").expect("write matching entry");
        std::fs::write(&drop, b"drop").expect("write filtered entry");
        create_fixture(
            &archive,
            &[(&keep, "nested/keep.txt"), (&drop, "nested/drop.bin")],
        );

        extract(
            &TestRuntime::default(),
            None,
            "filter-task",
            archive.to_string_lossy().as_ref(),
            output.to_string_lossy().as_ref(),
            None,
            &DecompressOptions {
                preserve_paths: true,
                file_filter: Some("*.txt".to_string()),
                ..Default::default()
            },
        )
        .expect("filtered extraction");
        assert_eq!(
            std::fs::read(output.join("nested/keep.txt")).expect("matching output"),
            b"keep"
        );
        assert!(!output.join("nested/drop.bin").exists());
    }

    #[test]
    fn real_cancellation_stays_cancelled_and_writes_no_file() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("large.bin");
        let archive = temp.path().join("cancel.7z");
        let output = temp.path().join("cancel-output");
        std::fs::write(&source, vec![7u8; 2 * 1024 * 1024]).expect("write source");
        sevenz_rust::compress_to_path(&source, &archive).expect("create 7z fixture");
        let runtime = TestRuntime::default();
        runtime.cancel();

        let error = extract(
            &runtime,
            None,
            "cancel-task",
            archive.to_string_lossy().as_ref(),
            output.to_string_lossy().as_ref(),
            None,
            &DecompressOptions::default(),
        )
        .expect_err("cancelled extraction must fail");
        assert!(matches!(
            error.downcast_ref::<CompressionError>(),
            Some(CompressionError::Cancelled)
        ));
        assert!(!output.join("large.bin").exists());
    }

    #[test]
    fn real_storage_full_is_classified_and_staging_is_removed() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("disk-full.bin");
        let archive = temp.path().join("disk-full.7z");
        let destination = temp.path().join("disk-full-output");
        std::fs::write(&source, vec![13u8; 2 * 1024 * 1024]).expect("write source");
        sevenz_rust::compress_to_path(&source, &archive).expect("create 7z fixture");
        std::fs::create_dir_all(&destination).expect("create destination");
        std::fs::write(destination.join("existing.txt"), b"original")
            .expect("write destination fixture");

        let staging_path;
        {
            let staging = ExtractionStaging::create_for(&destination).expect("create staging");
            staging_path = staging.path().to_path_buf();
            let error = extract(
                &TestRuntime::storage_full_after(64 * 1024),
                None,
                "disk-full-task",
                archive.to_string_lossy().as_ref(),
                staging.path().to_string_lossy().as_ref(),
                None,
                &DecompressOptions::default(),
            )
            .expect_err("injected storage exhaustion must fail");
            assert!(matches!(
                error.downcast_ref::<CompressionError>(),
                Some(CompressionError::DiskFull)
            ));
            assert!(!staging.path().join("disk-full.bin").exists());
        }

        assert!(!staging_path.exists());
        assert_eq!(
            std::fs::read(destination.join("existing.txt")).expect("original destination"),
            b"original"
        );
    }

    #[test]
    fn corrupt_staging_is_removed_without_mutating_destination() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("payload.bin");
        let archive = temp.path().join("rollback.7z");
        let output = temp.path().join("rollback-output");
        std::fs::write(&source, vec![3u8; 256 * 1024]).expect("write source");
        sevenz_rust::compress_to_path(&source, &archive).expect("create 7z fixture");
        corrupt_first_packed_stream(&archive);
        std::fs::create_dir_all(&output).expect("create destination");
        std::fs::write(output.join("existing.txt"), b"original").expect("destination fixture");
        let staging_path;
        {
            let staging = ExtractionStaging::create_for(&output).expect("create staging");
            staging_path = staging.path().to_path_buf();
            extract(
                &TestRuntime::default(),
                None,
                "rollback-task",
                archive.to_string_lossy().as_ref(),
                staging.path().to_string_lossy().as_ref(),
                None,
                &DecompressOptions::default(),
            )
            .expect_err("corrupt extraction must fail");
        }
        assert!(!staging_path.exists());
        assert_eq!(
            std::fs::read(output.join("existing.txt")).expect("original destination"),
            b"original"
        );
    }

    #[test]
    fn real_corrupt_archive_can_be_skipped_without_partial_output() {
        let temp = tempfile::tempdir().expect("temp dir");
        let source = temp.path().join("payload.bin");
        let archive = temp.path().join("skip-corrupt.7z");
        let output = temp.path().join("skip-corrupt-output");
        std::fs::write(&source, vec![11u8; 256 * 1024]).expect("write source");
        sevenz_rust::compress_to_path(&source, &archive).expect("create 7z fixture");
        corrupt_first_packed_stream(&archive);

        extract(
            &TestRuntime::default(),
            None,
            "skip-corrupt-task",
            archive.to_string_lossy().as_ref(),
            output.to_string_lossy().as_ref(),
            None,
            &DecompressOptions {
                skip_corrupted: true,
                ..Default::default()
            },
        )
        .expect("skip corrupted entry");
        assert!(!output.join("payload.bin").exists());
    }
}
