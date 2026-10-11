use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum PublishError {
    #[error("output publication was cancelled")]
    Cancelled,
    #[error("verified staged output is missing or is not a regular file: {0}")]
    InvalidStagedOutput(PathBuf),
    #[error("staged output must be created beside its final destination")]
    CrossDirectoryPublish,
    #[error("output appeared while processing was running; it was not overwritten: {0}")]
    TargetAppeared(PathBuf),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub(crate) fn staged_output_path(final_output: &Path, namespace: &str) -> Result<PathBuf, PublishError> {
    let parent = final_output.parent().unwrap_or_else(|| Path::new("."));
    let file_name = final_output
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| PublishError::InvalidStagedOutput(final_output.to_path_buf()))?;
    let safe_namespace: String = namespace
        .chars()
        .filter(|value| value.is_ascii_alphanumeric() || *value == '-')
        .collect();
    let safe_namespace = if safe_namespace.is_empty() {
        "publish"
    } else {
        &safe_namespace
    };

    Ok(parent.join(format!(
        ".{}-{}.{}",
        safe_namespace,
        uuid::Uuid::new_v4(),
        file_name
    )))
}

fn transaction_family_prefix(staged_output: &Path) -> Option<String> {
    let file_name = staged_output.file_name()?.to_str()?;
    let (transaction_id, _) = file_name.strip_prefix('.')?.split_once('.')?;
    let uuid_start = transaction_id.len().checked_sub(36)?;
    uuid::Uuid::parse_str(transaction_id.get(uuid_start..)?).ok()?;
    Some(format!(".{}.", transaction_id))
}

fn staged_output_belongs_to_destination(staged_output: &Path, final_output: &Path) -> bool {
    let Some(final_name) = final_output.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let Some(staged_name) = staged_output.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    transaction_family_prefix(staged_output)
        .and_then(|prefix| staged_name.strip_prefix(&prefix))
        .is_some_and(|staged_final_name| staged_final_name == final_name)
}

pub(crate) fn cleanup_staged_output_family(staged_output: &Path) {
    let Some(family_prefix) = transaction_family_prefix(staged_output) else {
        return;
    };
    let _ = std::fs::remove_file(staged_output);

    let Some(parent) = staged_output.parent() else {
        return;
    };
    let Some(file_name) = staged_output.file_name().and_then(|name| name.to_str()) else {
        return;
    };
    if let Ok(entries) = std::fs::read_dir(parent) {
        for entry in entries.flatten() {
            let candidate = entry.path();
            let name = candidate
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            let belongs_to_transaction = name.starts_with(file_name)
                || name.starts_with(&family_prefix);
            if belongs_to_transaction {
                let _ = std::fs::remove_file(candidate);
            }
        }
    }
}

pub(crate) fn publish_verified_file(
    staged_output: &Path,
    final_output: &Path,
    is_cancelled: impl FnOnce() -> bool,
) -> Result<(), PublishError> {
    let result = (|| {
        if is_cancelled() {
            return Err(PublishError::Cancelled);
        }
        if staged_output.parent() != final_output.parent() {
            return Err(PublishError::CrossDirectoryPublish);
        }
        if !staged_output_belongs_to_destination(staged_output, final_output) {
            return Err(PublishError::InvalidStagedOutput(staged_output.to_path_buf()));
        }
        let metadata = std::fs::symlink_metadata(staged_output)
            .map_err(|_| PublishError::InvalidStagedOutput(staged_output.to_path_buf()))?;
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return Err(PublishError::InvalidStagedOutput(staged_output.to_path_buf()));
        }
        if final_output.exists() {
            return Err(PublishError::TargetAppeared(final_output.to_path_buf()));
        }

        std::fs::rename(staged_output, final_output)?;
        Ok(())
    })();

    if result.is_err() {
        cleanup_staged_output_family(staged_output);
    }
    result
}

/// Publish a numeric volume set without overwriting any destination. This is
/// rollback-capable, not an atomic multi-file filesystem transaction.
pub(crate) fn publish_verified_volumes(
    staged: &Path,
    destination: &Path,
    mut is_cancelled: impl FnMut() -> bool,
) -> Result<(), PublishError> {
    let mut published = Vec::new();
    let result = (|| {
        if staged.parent() != destination.parent() {
            return Err(PublishError::CrossDirectoryPublish);
        }
        if !staged_output_belongs_to_destination(staged, destination) {
            return Err(PublishError::InvalidStagedOutput(staged.to_path_buf()));
        }
        let parent = staged.parent().unwrap_or_else(|| Path::new("."));
        let prefix = format!("{}.", staged.file_name().unwrap().to_string_lossy());
        let final_prefix = format!("{}.", destination.file_name().unwrap().to_string_lossy());
        let mut parts = Vec::new();
        for entry in std::fs::read_dir(parent)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name == destination.file_name().unwrap().to_string_lossy()
                || name.strip_prefix(&final_prefix).is_some_and(|suffix|
                    !suffix.is_empty() && suffix.bytes().all(|value| value.is_ascii_digit())) {
                return Err(PublishError::TargetAppeared(entry.path()));
            }
            if let Some(suffix) = name.strip_prefix(&prefix) {
                let index = suffix.parse::<usize>().map_err(|_| PublishError::InvalidStagedOutput(entry.path()))?;
                let metadata = std::fs::symlink_metadata(entry.path())?;
                if !metadata.file_type().is_file() || metadata.len() == 0 || suffix != format!("{index:03}") {
                    return Err(PublishError::InvalidStagedOutput(entry.path()));
                }
                parts.push((index, entry.path(), parent.join(format!("{final_prefix}{suffix}"))));
            }
        }
        parts.sort_by_key(|part| part.0);
        if parts.is_empty() || parts.iter().enumerate().any(|(index, part)| part.0 != index + 1) {
            return Err(PublishError::InvalidStagedOutput(staged.to_path_buf()));
        }
        for (_, source, target) in &parts {
            if is_cancelled() { return Err(PublishError::Cancelled); }
            // Explicitly omit REPLACE_EXISTING (std::fs::rename replaces files).
            // Moving also supports FAT/exFAT without requiring hard links.
            #[cfg(windows)]
            let transfer = {
                use std::os::windows::ffi::OsStrExt;
                let source_wide: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
                let target_wide: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
                let moved = unsafe {
                    windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                        source_wide.as_ptr(), target_wide.as_ptr(), 0,
                    )
                };
                if moved == 0 { Err(std::io::Error::last_os_error()) } else { Ok(()) }
            };
            #[cfg(not(windows))]
            let transfer = std::fs::hard_link(source, target);
            transfer.map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    PublishError::TargetAppeared(target.clone())
                } else { PublishError::Io(error) }
            })?;
            published.push(target.clone());
        }
        if is_cancelled() { return Err(PublishError::Cancelled); }
        Ok(())
    })();
    if result.is_err() {
        for path in published { let _ = std::fs::remove_file(path); }
    }
    cleanup_staged_output_family(staged);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_publication_handles_success_conflict_cancellation_and_gaps() {
        for scenario in ["success", "conflict", "cancel", "gap", "late-conflict"] {
            let temp = tempfile::tempdir().unwrap();
            let target = temp.path().join("archive.zip");
            let staged = staged_output_path(&target, "compress").unwrap();
            let part = |base: &Path, index: usize| PathBuf::from(format!("{}.{index:03}", base.display()));
            for index in 1..=3 {
                if scenario != "gap" || index != 2 {
                    std::fs::write(part(&staged, index), vec![index as u8; 10]).unwrap();
                }
            }
            if scenario == "conflict" { std::fs::write(part(&target, 2), b"user data").unwrap(); }
            let mut checks = 0;
            let result = publish_verified_volumes(&staged, &target, || {
                checks += 1;
                if scenario == "late-conflict" && checks == 2 {
                    std::fs::write(part(&target, 2), b"user data").unwrap();
                }
                scenario == "cancel" && checks == 2
            });
            assert_eq!(result.is_ok(), scenario == "success", "{scenario}");
            for index in 1..=3 {
                assert!(!part(&staged, index).exists());
                if scenario == "success" {
                    assert_eq!(std::fs::read(part(&target, index)).unwrap(), vec![index as u8; 10]);
                } else if (scenario == "conflict" || scenario == "late-conflict") && index == 2 {
                    assert_eq!(std::fs::read(part(&target, index)).unwrap(), b"user data");
                } else { assert!(!part(&target, index).exists(), "{scenario}: {index}"); }
            }
        }
    }

    #[test]
    fn real_file_publish_preserves_bytes_and_removes_staging() {
        let temp = tempfile::tempdir().expect("temp directory");
        let final_output = temp.path().join("result.bin");
        let staged_output = staged_output_path(&final_output, "media-publish").expect("staging path");
        let expected = (0..=255).cycle().take(1024 * 1024 + 37).collect::<Vec<u8>>();
        std::fs::write(&staged_output, &expected).expect("write staged bytes");

        publish_verified_file(&staged_output, &final_output, || false).expect("publish file");

        assert_eq!(std::fs::read(&final_output).expect("read final bytes"), expected);
        assert!(!staged_output.exists());
    }

    #[test]
    fn target_race_preserves_existing_file_and_cleans_staged_family() {
        let temp = tempfile::tempdir().expect("temp directory");
        let final_output = temp.path().join("result.bin");
        let staged_output = staged_output_path(&final_output, "media-publish").expect("staging path");
        let sidecar = staged_output.with_file_name(format!(
            "{}.001",
            staged_output.file_name().unwrap().to_string_lossy()
        ));
        std::fs::write(&staged_output, b"new bytes").expect("write staged bytes");
        std::fs::write(&sidecar, b"sidecar").expect("write sidecar");
        std::fs::write(&final_output, b"existing bytes").expect("write existing output");

        let error = publish_verified_file(&staged_output, &final_output, || false)
            .expect_err("target race must fail");

        assert!(matches!(error, PublishError::TargetAppeared(_)));
        assert_eq!(std::fs::read(&final_output).unwrap(), b"existing bytes");
        assert!(!staged_output.exists());
        assert!(!sidecar.exists());
    }

    #[test]
    fn cancellation_keeps_destination_absent_and_cleans_staging() {
        let temp = tempfile::tempdir().expect("temp directory");
        let final_output = temp.path().join("result.bin");
        let staged_output = staged_output_path(&final_output, "media-publish").expect("staging path");
        std::fs::write(&staged_output, b"verified but cancelled").expect("write staged bytes");

        let error = publish_verified_file(&staged_output, &final_output, || true)
            .expect_err("cancelled publish must fail");

        assert!(matches!(error, PublishError::Cancelled));
        assert!(!staged_output.exists());
        assert!(!final_output.exists());
    }

    #[test]
    fn missing_staged_file_never_creates_destination() {
        let temp = tempfile::tempdir().expect("temp directory");
        let final_output = temp.path().join("result.bin");
        let staged_output = staged_output_path(&final_output, "media-publish").expect("staging path");

        let error = publish_verified_file(&staged_output, &final_output, || false)
            .expect_err("missing staged output must fail");

        assert!(matches!(error, PublishError::InvalidStagedOutput(_)));
        assert!(!final_output.exists());
    }

    #[test]
    fn arbitrary_neighbor_cannot_be_published_or_family_cleaned() {
        let temp = tempfile::tempdir().expect("temp directory");
        let final_output = temp.path().join("result.bin");
        let arbitrary = temp.path().join("user-file.bin");
        let neighbor = temp.path().join("user-file.bin.backup");
        std::fs::write(&arbitrary, b"not transaction owned").expect("write arbitrary file");
        std::fs::write(&neighbor, b"must survive").expect("write neighbor");

        let error = publish_verified_file(&arbitrary, &final_output, || false)
            .expect_err("arbitrary file must not publish");

        assert!(matches!(error, PublishError::InvalidStagedOutput(_)));
        assert_eq!(std::fs::read(&arbitrary).unwrap(), b"not transaction owned");
        assert_eq!(std::fs::read(&neighbor).unwrap(), b"must survive");
        assert!(!final_output.exists());
    }
}
