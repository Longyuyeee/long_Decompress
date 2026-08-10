use crate::services::aes_wrapper::AesWrapper;
use crate::services::compression_format::CompressionRoute;
use crate::services::compression_service::CompressionError;
use crate::services::tar_aes_engine::TarAesEngine;
use anyhow::Result;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    fn create(prefix: &str) -> Result<Self> {
        let path = std::env::temp_dir().join(format!("{prefix}{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn check_cancelled(is_cancelled: &impl Fn() -> bool) -> Result<()> {
    if is_cancelled() {
        Err(CompressionError::Cancelled.into())
    } else {
        Ok(())
    }
}

fn verify_plain_zip(path: &Path, is_cancelled: &impl Fn() -> bool) -> Result<()> {
    let file = File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut buffer = vec![0u8; 256 * 1024];
    for index in 0..archive.len() {
        check_cancelled(is_cancelled)?;
        let mut entry = archive.by_index(index)?;
        while !entry.is_dir() {
            check_cancelled(is_cancelled)?;
            let read = entry.read(&mut buffer)?;
            if read == 0 {
                break;
            }
        }
    }
    Ok(())
}

fn verify_encrypted_zip(
    path: &Path,
    password: &str,
    is_cancelled: &impl Fn() -> bool,
) -> Result<()> {
    let file = File::open(path)?;
    let mut archive = zip_aes::ZipArchive::new(file)?;
    let mut buffer = vec![0u8; 256 * 1024];
    for index in 0..archive.len() {
        check_cancelled(is_cancelled)?;
        let mut entry = archive
            .by_index_decrypt(index, password.as_bytes())
            .map_err(|_| {
                CompressionError::CompressionFailed(
                    "The newly created ZIP could not be opened with its requested password"
                        .to_string(),
                )
            })?;
        while !entry.is_dir() {
            check_cancelled(is_cancelled)?;
            let read = entry.read(&mut buffer)?;
            if read == 0 {
                break;
            }
        }
    }
    Ok(())
}

fn verify_seven_zip(
    path: &Path,
    password: Option<&str>,
    is_cancelled: &impl Fn() -> bool,
) -> Result<()> {
    let scratch = TemporaryDirectory::create("long-compress-verify-7z-")?;
    let mut verify_entry = |_entry: &sevenz_rust::SevenZArchiveEntry,
                            reader: &mut dyn Read,
                            _destination: &PathBuf|
     -> Result<bool, sevenz_rust::Error> {
        let mut buffer = [0u8; 256 * 1024];
        loop {
            if is_cancelled() {
                return Err(sevenz_rust::Error::other("Archive verification cancelled"));
            }
            let read = reader.read(&mut buffer).map_err(sevenz_rust::Error::io)?;
            if read == 0 {
                break;
            }
        }
        Ok(true)
    };
    let result = sevenz_rust::decompress_with_extract_fn_and_password(
        File::open(path)?,
        scratch.path(),
        sevenz_rust::Password::from(password.unwrap_or_default()),
        &mut verify_entry,
    );
    if is_cancelled() {
        return Err(CompressionError::Cancelled.into());
    }
    result.map_err(|error| {
        CompressionError::CompressionFailed(format!(
            "Newly created 7Z archive failed verification: {error}"
        ))
        .into()
    })
}

fn drain_reader(mut reader: impl Read, is_cancelled: &impl Fn() -> bool) -> Result<()> {
    let mut buffer = [0u8; 256 * 1024];
    loop {
        check_cancelled(is_cancelled)?;
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            return Ok(());
        }
    }
}

fn verify_tar_reader(reader: impl Read, is_cancelled: &impl Fn() -> bool) -> Result<()> {
    let mut archive = tar::Archive::new(reader);
    for entry in archive.entries()? {
        check_cancelled(is_cancelled)?;
        drain_reader(entry?, is_cancelled)?;
    }
    Ok(())
}

fn verify_wrapped_aes(
    route: CompressionRoute,
    path: &Path,
    password: &str,
    is_cancelled: &impl Fn() -> bool,
) -> Result<()> {
    let temporary = TemporaryDirectory::create("long-compress-verify-aes-")?;
    let decrypted = temporary.path().join("decrypted-payload");
    AesWrapper::decrypt_file_cancellable(path, &decrypted, password, || {
        check_cancelled(is_cancelled)
    })?;
    check_cancelled(is_cancelled)?;
    match route {
        CompressionRoute::TarGzipAes => verify_tar_reader(
            flate2::read::GzDecoder::new(File::open(&decrypted)?),
            is_cancelled,
        ),
        CompressionRoute::TarBzip2Aes => verify_tar_reader(
            bzip2::read::BzDecoder::new(File::open(&decrypted)?),
            is_cancelled,
        ),
        CompressionRoute::TarXzAes => verify_tar_reader(
            xz2::read::XzDecoder::new(File::open(&decrypted)?),
            is_cancelled,
        ),
        CompressionRoute::TarZstdAes => verify_tar_reader(
            zstd::stream::read::Decoder::new(File::open(&decrypted)?)?,
            is_cancelled,
        ),
        CompressionRoute::GzipAes => drain_reader(
            flate2::read::GzDecoder::new(File::open(&decrypted)?),
            is_cancelled,
        ),
        CompressionRoute::Bzip2Aes => drain_reader(
            bzip2::read::BzDecoder::new(File::open(&decrypted)?),
            is_cancelled,
        ),
        CompressionRoute::XzAes => drain_reader(
            xz2::read::XzDecoder::new(File::open(&decrypted)?),
            is_cancelled,
        ),
        CompressionRoute::ZstdAes => drain_reader(
            zstd::stream::read::Decoder::new(File::open(&decrypted)?)?,
            is_cancelled,
        ),
        _ => unreachable!("only wrapped AES routes reach this verifier"),
    }
}

pub(crate) fn verify_native(
    route: CompressionRoute,
    path: &Path,
    password: Option<&str>,
    is_cancelled: impl Fn() -> bool,
) -> Result<bool> {
    check_cancelled(&is_cancelled)?;
    match route {
        CompressionRoute::Zip => {
            if let Some(password) = password.filter(|value| !value.is_empty()) {
                verify_encrypted_zip(path, password, &is_cancelled)?;
            } else {
                verify_plain_zip(path, &is_cancelled)?;
            }
            Ok(true)
        }
        CompressionRoute::SevenZip => {
            verify_seven_zip(path, password, &is_cancelled)?;
            Ok(true)
        }
        CompressionRoute::Tar => {
            verify_tar_reader(File::open(path)?, &is_cancelled)?;
            Ok(true)
        }
        CompressionRoute::TarGzip => {
            verify_tar_reader(
                flate2::read::GzDecoder::new(File::open(path)?),
                &is_cancelled,
            )?;
            Ok(true)
        }
        CompressionRoute::TarBzip2 => {
            verify_tar_reader(
                bzip2::read::BzDecoder::new(File::open(path)?),
                &is_cancelled,
            )?;
            Ok(true)
        }
        CompressionRoute::TarXz => {
            verify_tar_reader(xz2::read::XzDecoder::new(File::open(path)?), &is_cancelled)?;
            Ok(true)
        }
        CompressionRoute::TarZstd => {
            verify_tar_reader(
                zstd::stream::read::Decoder::new(File::open(path)?)?,
                &is_cancelled,
            )?;
            Ok(true)
        }
        CompressionRoute::Gzip => {
            drain_reader(
                flate2::read::GzDecoder::new(File::open(path)?),
                &is_cancelled,
            )?;
            Ok(true)
        }
        CompressionRoute::Bzip2 => {
            drain_reader(
                bzip2::read::BzDecoder::new(File::open(path)?),
                &is_cancelled,
            )?;
            Ok(true)
        }
        CompressionRoute::Xz => {
            drain_reader(xz2::read::XzDecoder::new(File::open(path)?), &is_cancelled)?;
            Ok(true)
        }
        CompressionRoute::Zstd => {
            drain_reader(
                zstd::stream::read::Decoder::new(File::open(path)?)?,
                &is_cancelled,
            )?;
            Ok(true)
        }
        CompressionRoute::TarAes => {
            let password = password.ok_or_else(|| {
                CompressionError::CompressionFailed(
                    "TAR.AES verification requires the requested password".to_string(),
                )
            })?;
            let temporary = TemporaryDirectory::create("long-compress-verify-tar-aes-")?;
            TarAesEngine::decompress_tar_aes_cancellable(path, temporary.path(), password, || {
                check_cancelled(&is_cancelled)
            })?;
            Ok(true)
        }
        CompressionRoute::TarGzipAes
        | CompressionRoute::TarBzip2Aes
        | CompressionRoute::TarXzAes
        | CompressionRoute::TarZstdAes
        | CompressionRoute::GzipAes
        | CompressionRoute::Bzip2Aes
        | CompressionRoute::XzAes
        | CompressionRoute::ZstdAes => {
            let password = password.ok_or_else(|| {
                CompressionError::CompressionFailed(
                    "AES container verification requires the requested password".to_string(),
                )
            })?;
            verify_wrapped_aes(route, path, password, &is_cancelled)?;
            Ok(true)
        }
        CompressionRoute::Rar | CompressionRoute::Wim | CompressionRoute::Lzma => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::compression::CompressionOptions;
    use crate::models::compression::TaskLogSeverity;
    use crate::services::native_compression::{self, CompressionRuntime};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use tauri::Window;

    #[derive(Default)]
    struct TestRuntime {
        cancelled: Arc<AtomicBool>,
    }

    impl CompressionRuntime for TestRuntime {
        fn check_cancellation(&self) -> Result<()> {
            check_cancelled(&|| self.cancelled.load(Ordering::Relaxed))
        }

        fn cancellation_flag(&self) -> Arc<AtomicBool> {
            self.cancelled.clone()
        }

        fn copy_buffer_size(&self) -> usize {
            256 * 1024
        }

        fn emit_log(&self, _: &Window, _: &str, _: &str, _: TaskLogSeverity) {}

        fn emit_progress(&self, _: &Window, _: &str, _: f32, _: Option<String>, _: u64, _: u64) {}
    }

    #[test]
    fn real_zip_verification_reads_every_entry_and_rejects_corruption() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("payload.txt");
        let archive = temp.path().join("payload.zip");
        std::fs::write(&source, vec![b'x'; 128 * 1024]).unwrap();
        native_compression::zip::compress(
            &TestRuntime::default(),
            None,
            "verify-zip",
            &[source.to_string_lossy().to_string()],
            archive.to_string_lossy().as_ref(),
            CompressionOptions::default(),
        )
        .unwrap();
        assert!(verify_native(CompressionRoute::Zip, &archive, None, || false).unwrap());

        let mut bytes = std::fs::read(&archive).unwrap();
        let middle = bytes.len() / 2;
        bytes[middle] ^= 0xff;
        std::fs::write(&archive, bytes).unwrap();
        assert!(verify_native(CompressionRoute::Zip, &archive, None, || false).is_err());
    }

    #[test]
    fn cancellation_interrupts_verification() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("payload.txt");
        let archive = temp.path().join("payload.zip");
        std::fs::write(&source, b"payload").unwrap();
        native_compression::zip::compress(
            &TestRuntime::default(),
            None,
            "verify-cancel",
            &[source.to_string_lossy().to_string()],
            archive.to_string_lossy().as_ref(),
            CompressionOptions::default(),
        )
        .unwrap();

        let error = verify_native(CompressionRoute::Zip, &archive, None, || true).unwrap_err();
        assert!(matches!(
            error.downcast_ref::<CompressionError>(),
            Some(CompressionError::Cancelled)
        ));
    }

    #[test]
    fn encrypted_zip_and_seven_zip_require_the_creation_password() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("secret.txt");
        std::fs::write(&source, b"classified payload").unwrap();
        let sources = [source.to_string_lossy().to_string()];
        let runtime = TestRuntime::default();

        let zip_path = temp.path().join("secret.zip");
        native_compression::zip::compress(
            &runtime,
            None,
            "verify-encrypted-zip",
            &sources,
            zip_path.to_string_lossy().as_ref(),
            CompressionOptions {
                password: Some("correct-password".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(verify_native(
            CompressionRoute::Zip,
            &zip_path,
            Some("correct-password"),
            || false,
        )
        .unwrap());
        assert!(verify_native(
            CompressionRoute::Zip,
            &zip_path,
            Some("wrong-password"),
            || false,
        )
        .is_err());

        let seven_zip_path = temp.path().join("secret.7z");
        native_compression::seven_zip::compress(
            &runtime,
            None,
            "verify-encrypted-7z",
            &sources,
            seven_zip_path.to_string_lossy().as_ref(),
            CompressionOptions {
                password: Some("correct-password".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(verify_native(
            CompressionRoute::SevenZip,
            &seven_zip_path,
            Some("correct-password"),
            || false,
        )
        .unwrap());
        assert!(verify_native(
            CompressionRoute::SevenZip,
            &seven_zip_path,
            Some("wrong-password"),
            || false,
        )
        .is_err());
    }

    #[test]
    fn aes_container_authentication_detects_tampering() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("payload.txt");
        let archive = temp.path().join("payload.gz.aes");
        std::fs::write(&source, vec![b'a'; 64 * 1024]).unwrap();
        native_compression::aes::compress(
            &TestRuntime::default(),
            None,
            "verify-aes",
            &[source.to_string_lossy().to_string()],
            archive.to_string_lossy().as_ref(),
            CompressionOptions {
                password: Some("correct-password".to_string()),
                ..Default::default()
            },
            native_compression::aes::AesCompressionKind::Gzip,
        )
        .unwrap();
        assert!(verify_native(
            CompressionRoute::GzipAes,
            &archive,
            Some("correct-password"),
            || false,
        )
        .unwrap());

        let mut bytes = std::fs::read(&archive).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;
        std::fs::write(&archive, bytes).unwrap();
        assert!(verify_native(
            CompressionRoute::GzipAes,
            &archive,
            Some("correct-password"),
            || false,
        )
        .is_err());

        let invalid_inner = temp.path().join("invalid-inner.bin");
        let authenticated_but_invalid = temp.path().join("invalid.gz.aes");
        std::fs::write(&invalid_inner, b"not a gzip stream").unwrap();
        AesWrapper::encrypt_file(
            &invalid_inner,
            &authenticated_but_invalid,
            "correct-password",
        )
        .unwrap();
        assert!(verify_native(
            CompressionRoute::GzipAes,
            &authenticated_but_invalid,
            Some("correct-password"),
            || false,
        )
        .is_err());
    }
}
