//! One authenticated inner file for the existing native consumers.
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub(crate) struct AuthenticatedInner {
    pub(crate) path: PathBuf,
    _directory: tempfile::TempDir,
}

pub(crate) fn logical_path(path: &Path) -> Result<PathBuf> {
    let source = super::split_archive_detector::SplitArchiveDetector::complete_source(path)?;
    Ok(if super::numeric_volume_reader::ZipInputReader::is_numeric_first(&source) {
        source.with_file_name(source.file_stem().context("AES 卷组名称缺失")?)
    } else { source })
}

impl AuthenticatedInner {
    pub(crate) fn open(path: &Path, password: Option<&str>, mut check: impl FnMut() -> Result<()>) -> Result<Self> {
        check()?;
        let password = password.filter(|p| !p.is_empty())
            .ok_or(crate::services::compression_service::CompressionError::PasswordRequired)?;
        let logical = logical_path(path)?;
        let name = logical.file_stem().context("AES 容器名称缺失")?;
        let directory = tempfile::Builder::new().prefix("long-aes-auth-").tempdir()?;
        let tar = super::tar_aes_engine::TarAesEngine::is_tar_aes(path)?;
        // TARAES carries its own kind; a misleading filename cannot override it.
        let output = directory.path().join(if tar { std::ffi::OsStr::new("payload.tar") } else { name });
        if tar {
            super::tar_aes_engine::TarAesEngine::decrypt_to_file(path, &output, password, &mut check)?;
        } else {
            anyhow::ensure!(super::aes_wrapper::AesWrapper::is_aes_encrypted(path)?,
                "[archive-inspection:unsupported] 不是 Long AES 容器");
            let format = super::compression_format::infer_compression_format(&logical.to_string_lossy(), None);
            anyhow::ensure!(matches!(format.as_str(), "tar.gz.aes" | "tar.bz2.aes" | "tar.xz.aes" | "tar.zst.aes" | "gz.aes" | "bz2.aes" | "xz.aes" | "zst.aes"),
                "[archive-inspection:unsupported] AES 容器名称缺少可识别的内层格式，请保留原始文件名，例如 .tar.gz.aes");
            super::aes_wrapper::AesWrapper::decrypt_file_cancellable(path, &output, password, &mut check)?;
        }
        check()?;
        anyhow::ensure!(super::archive_browser::detect_format(&output)? != super::archive_format::ArchiveFormat::AesEncrypted,
            "[archive-inspection:unsupported] 不支持嵌套的 Long AES 容器");
        Ok(Self { path: output, _directory: directory })
    }
}
