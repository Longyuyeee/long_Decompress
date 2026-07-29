use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::models::compression::DecompressOptions;

use super::compression_service::CompressionError;

pub(crate) const MAX_EXTRACTED_ENTRIES: usize = 250_000;
pub(crate) const MAX_EXTRACTED_BYTES: u64 = 512 * 1024 * 1024 * 1024;
pub(crate) const MAX_EXPANSION_RATIO: u64 = 10_000;
pub(crate) const DISK_SAFETY_RESERVE: u64 = 128 * 1024 * 1024;

/// Owns a sibling extraction directory and removes it on every exit path.
pub(crate) struct ExtractionStaging {
    path: PathBuf,
    cleaned: bool,
}

impl ExtractionStaging {
    pub(crate) fn create_for(output: &Path) -> Result<Self> {
        let parent = output
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;
        let path = parent.join(format!(".long-extract-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path)?;
        Ok(Self {
            path,
            cleaned: false,
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn cleanup(&mut self) -> std::io::Result<()> {
        if self.cleaned {
            return Ok(());
        }
        match std::fs::remove_dir_all(&self.path) {
            Ok(()) => {
                self.cleaned = true;
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.cleaned = true;
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
}

impl Drop for ExtractionStaging {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileConflict {
    pub file_name: String,
    pub source_path: String,
    pub dest_path: String,
    pub source_size: u64,
    pub dest_size: u64,
    pub source_modified: u64,
    pub dest_modified: u64,
}

pub(crate) fn validate_resource_limits(
    archive_path: &Path,
    entry_count: usize,
    expanded_bytes: u64,
) -> Result<()> {
    if entry_count > MAX_EXTRACTED_ENTRIES {
        return Err(CompressionError::ExtractionFailed(format!(
            "Archive contains too many entries ({} > {})",
            entry_count, MAX_EXTRACTED_ENTRIES
        ))
        .into());
    }
    if expanded_bytes > MAX_EXTRACTED_BYTES {
        return Err(CompressionError::ExtractionFailed(format!(
            "Archive expands beyond the safety limit ({} bytes)",
            MAX_EXTRACTED_BYTES
        ))
        .into());
    }
    let compressed_bytes = std::fs::metadata(archive_path)?.len().max(1);
    if expanded_bytes >= 1024 * 1024 * 1024
        && expanded_bytes / compressed_bytes > MAX_EXPANSION_RATIO
    {
        return Err(CompressionError::ExtractionFailed(format!(
            "Archive expansion ratio exceeds the safety limit ({}:1)",
            MAX_EXPANSION_RATIO
        ))
        .into());
    }
    Ok(())
}

fn available_disk_space(path: &Path) -> Option<u64> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .filter(|disk| path.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().components().count())
        .map(|disk| disk.available_space())
}

pub(crate) fn validate_disk_capacity(path: &Path, expanded_bytes: u64) -> Result<()> {
    if available_disk_space(path)
        .is_some_and(|available| available < expanded_bytes.saturating_add(DISK_SAFETY_RESERVE))
    {
        return Err(CompressionError::DiskFull.into());
    }
    Ok(())
}

pub(crate) fn validate_staging_disk_reserve(staging: &Path) -> Result<()> {
    validate_disk_capacity(staging, 0)
}

#[cfg(windows)]
fn is_link_or_reparse(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_link_or_reparse(metadata: &std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

pub(crate) fn ensure_no_link_ancestors(path: &Path) -> Result<()> {
    let mut current = Some(path);
    while let Some(candidate) = current {
        if let Ok(metadata) = std::fs::symlink_metadata(candidate) {
            if is_link_or_reparse(&metadata) {
                return Err(CompressionError::ExtractionFailed(format!(
                    "Extraction path contains a symbolic link or reparse point: {}",
                    candidate.display()
                ))
                .into());
            }
        }
        current = candidate.parent();
    }
    Ok(())
}

fn inspect_staged_resources(staging: &Path) -> Result<(usize, u64)> {
    let mut entry_count = 0usize;
    let mut expanded_bytes = 0u64;
    for entry in walkdir::WalkDir::new(staging).follow_links(false) {
        let entry = entry.map_err(|error| {
            CompressionError::ExtractionFailed(format!(
                "Unable to inspect extracted output: {}",
                error
            ))
        })?;
        if entry.path() == staging {
            continue;
        }
        let metadata = std::fs::symlink_metadata(entry.path())?;
        if is_link_or_reparse(&metadata) || (!metadata.is_file() && !metadata.is_dir()) {
            return Err(CompressionError::ExtractionFailed(format!(
                "Unsafe link or special entry was rejected: {}",
                entry.path().display()
            ))
            .into());
        }
        entry_count = entry_count.saturating_add(1);
        if metadata.is_file() {
            expanded_bytes = expanded_bytes.checked_add(metadata.len()).ok_or_else(|| {
                CompressionError::ExtractionFailed(
                    "Extracted size overflowed the supported range".to_string(),
                )
            })?;
        }
    }
    Ok((entry_count, expanded_bytes))
}

pub(crate) fn validate_staged_resources(archive_path: &Path, staging: &Path) -> Result<()> {
    let (entry_count, expanded_bytes) = inspect_staged_resources(staging)?;
    validate_resource_limits(archive_path, entry_count, expanded_bytes)?;
    validate_staging_disk_reserve(staging)
}

pub(crate) fn compile_file_filter(filter: Option<&str>) -> Vec<regex::Regex> {
    let Some(filter) = filter.map(str::trim).filter(|value| !value.is_empty()) else {
        return Vec::new();
    };
    filter
        .split([',', ';'])
        .map(str::trim)
        .filter(|pattern| !pattern.is_empty())
        .filter_map(|pattern| {
            let escaped = regex::escape(pattern)
                .replace("\\*", ".*")
                .replace("\\?", ".");
            regex::Regex::new(&format!("(?i)^{}$", escaped)).ok()
        })
        .collect()
}

pub(crate) fn matches_compiled_file_filter(path: &Path, filter: &[regex::Regex]) -> bool {
    if filter.is_empty() {
        return true;
    }
    let normalized = path.to_string_lossy().replace('\\', "/");
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    filter
        .iter()
        .any(|pattern| pattern.is_match(&normalized) || pattern.is_match(file_name))
}

pub(crate) fn resolve_extract_path(target: &Path, options: &DecompressOptions) -> Result<PathBuf> {
    if options.overwrite_existing || !target.exists() {
        return Ok(target.to_path_buf());
    }

    let parent = target.parent().unwrap_or_else(|| Path::new(""));
    let stem = target
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("file");
    let extension = target.extension().and_then(|name| name.to_str());

    for index in 1..10_000 {
        let file_name = match extension {
            Some(ext) if !ext.is_empty() => format!("{} ({}).{}", stem, index, ext),
            _ => format!("{} ({})", stem, index),
        };
        let candidate = parent.join(file_name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(CompressionError::ExtractionFailed(format!(
        "Unable to find available output name for {}",
        target.display()
    ))
    .into())
}

pub(crate) fn prepare_staging_layout(
    archive_path: &Path,
    staging: &Path,
    options: &DecompressOptions,
) -> Result<()> {
    let file_filter = compile_file_filter(options.file_filter.as_deref());
    let mut files = Vec::new();
    let mut directories = Vec::new();
    for entry in walkdir::WalkDir::new(staging).follow_links(false) {
        let entry = entry.map_err(|error| {
            CompressionError::ExtractionFailed(format!(
                "Unable to normalize extracted output: {}",
                error
            ))
        })?;
        if entry.path() == staging {
            continue;
        }
        let metadata = std::fs::symlink_metadata(entry.path())?;
        if is_link_or_reparse(&metadata) || (!metadata.is_file() && !metadata.is_dir()) {
            return Err(CompressionError::ExtractionFailed(format!(
                "Unsafe link or special entry was rejected: {}",
                entry.path().display()
            ))
            .into());
        }
        if metadata.is_dir() {
            directories.push(entry.path().to_path_buf());
        } else {
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort();

    let mut entry_count = directories.len();
    let mut expanded_bytes = 0u64;
    for source in files {
        let relative = source.strip_prefix(staging)?;
        if !matches_compiled_file_filter(relative, &file_filter) {
            std::fs::remove_file(&source)?;
            continue;
        }
        let source_size = std::fs::metadata(&source)?.len();
        entry_count = entry_count.saturating_add(1);
        expanded_bytes = expanded_bytes.checked_add(source_size).ok_or_else(|| {
            CompressionError::ExtractionFailed(
                "Extracted size overflowed the supported range".to_string(),
            )
        })?;
        if options.preserve_paths || source.parent() == Some(staging) {
            continue;
        }
        let file_name = source.file_name().ok_or_else(|| {
            CompressionError::ExtractionFailed("Extracted file has no valid name".to_string())
        })?;
        let requested = staging.join(file_name);
        let destination = if requested.exists() {
            let stem = requested
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("file");
            let extension = requested.extension().and_then(|value| value.to_str());
            let mut available = None;
            for index in 1..10_000 {
                let name = match extension {
                    Some(extension) if !extension.is_empty() => {
                        format!("{} ({}).{}", stem, index, extension)
                    }
                    _ => format!("{} ({})", stem, index),
                };
                let candidate = staging.join(name);
                if !candidate.exists() {
                    available = Some(candidate);
                    break;
                }
            }
            available.ok_or_else(|| {
                CompressionError::ExtractionFailed(format!(
                    "Unable to flatten duplicate archive entry: {}",
                    relative.display()
                ))
            })?
        } else {
            requested
        };
        std::fs::rename(&source, destination)?;
    }

    directories.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for directory in directories {
        let _ = std::fs::remove_dir(&directory);
    }
    validate_resource_limits(archive_path, entry_count, expanded_bytes)?;
    validate_staging_disk_reserve(staging)
}

fn ensure_commit_target_safe(root: &Path, target: &Path) -> Result<()> {
    ensure_no_link_ancestors(root)?;
    let relative = target.strip_prefix(root).map_err(|_| {
        CompressionError::ExtractionFailed(format!(
            "Extraction target escaped the output directory: {}",
            target.display()
        ))
    })?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        if let Ok(metadata) = std::fs::symlink_metadata(&current) {
            if is_link_or_reparse(&metadata) {
                return Err(CompressionError::ExtractionFailed(format!(
                    "Extraction target contains a symbolic link or reparse point: {}",
                    current.display()
                ))
                .into());
            }
        }
    }
    Ok(())
}

fn rollback_extraction_commit(
    created_files: &[PathBuf],
    created_dirs: &[PathBuf],
    backups: &[(PathBuf, PathBuf)],
) -> std::result::Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for path in created_files.iter().rev() {
        if let Err(error) = std::fs::remove_file(path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                errors.push(format!("remove {}: {}", path.display(), error));
            }
        }
    }
    for (destination, backup) in backups.iter().rev() {
        if let Err(error) = std::fs::remove_file(destination) {
            if error.kind() != std::io::ErrorKind::NotFound {
                errors.push(format!("remove {}: {}", destination.display(), error));
            }
        }
        if let Err(error) = std::fs::rename(backup, destination) {
            errors.push(format!(
                "restore {} from {}: {}",
                destination.display(),
                backup.display(),
                error
            ));
        }
    }
    for path in created_dirs.iter().rev() {
        if let Err(error) = std::fs::remove_dir(path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                errors.push(format!("remove directory {}: {}", path.display(), error));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub(crate) fn staged_file_is_not_newer(source: &Path, destination: &Path) -> bool {
    let source_modified = std::fs::metadata(source).and_then(|value| value.modified());
    let destination_modified = std::fs::metadata(destination).and_then(|value| value.modified());
    matches!((source_modified, destination_modified), (Ok(source), Ok(destination)) if source <= destination)
}

pub(crate) fn commit_staged_extraction(
    source_archive: &str,
    staging: &Path,
    output: &Path,
    options: &DecompressOptions,
    mut on_conflict: impl FnMut(FileConflict),
) -> Result<()> {
    ensure_no_link_ancestors(output)?;
    let mut directories = Vec::new();
    let mut files = Vec::new();
    for item in walkdir::WalkDir::new(staging).follow_links(false) {
        let item = item.map_err(|error| {
            CompressionError::ExtractionFailed(format!(
                "Unable to inspect staged output: {}",
                error
            ))
        })?;
        if item.path() == staging {
            continue;
        }
        let metadata = std::fs::symlink_metadata(item.path())?;
        if is_link_or_reparse(&metadata) || (!metadata.is_file() && !metadata.is_dir()) {
            return Err(CompressionError::ExtractionFailed(format!(
                "Unsafe link or special entry was rejected: {}",
                item.path().display()
            ))
            .into());
        }
        let relative = item.path().strip_prefix(staging)?.to_path_buf();
        if metadata.is_dir() {
            directories.push(relative);
        } else {
            files.push(relative);
        }
    }
    directories.sort_by_key(|path| path.components().count());
    files.sort();

    if options.conflict_policy == "ask" && !options.overwrite_existing {
        if let Some(relative) = files.iter().find(|relative| {
            let destination = output.join(relative);
            destination.exists()
                && !(options.extract_only_newer
                    && staged_file_is_not_newer(&staging.join(relative), &destination))
        }) {
            let destination = output.join(relative);
            let metadata = std::fs::metadata(&destination).ok();
            on_conflict(FileConflict {
                file_name: destination
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("file")
                    .to_string(),
                source_path: source_archive.to_string(),
                dest_path: destination.to_string_lossy().into_owned(),
                source_size: std::fs::metadata(staging.join(relative))
                    .map(|value| value.len())
                    .unwrap_or(0),
                dest_size: metadata.as_ref().map(|value| value.len()).unwrap_or(0),
                source_modified: 0,
                dest_modified: metadata
                    .and_then(|value| value.modified().ok())
                    .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|value| value.as_millis() as u64)
                    .unwrap_or(0),
            });
            return Err(CompressionError::ExtractionFailed(
                "File conflict requires resolution".to_string(),
            )
            .into());
        }
    }

    let rollback_root = staging.join(".rollback");
    let mut created_files = Vec::new();
    let mut created_dirs = Vec::new();
    let mut backups = Vec::new();
    let commit_result = (|| -> Result<()> {
        if !output.exists() {
            std::fs::create_dir_all(output)?;
            created_dirs.push(output.to_path_buf());
        }
        for relative in directories {
            let destination = output.join(&relative);
            ensure_commit_target_safe(output, &destination)?;
            if !destination.exists() {
                std::fs::create_dir_all(&destination)?;
                created_dirs.push(destination);
            }
        }
        for relative in files {
            let source = staging.join(&relative);
            let requested = output.join(&relative);
            ensure_commit_target_safe(output, &requested)?;

            if options.extract_only_newer
                && requested.exists()
                && staged_file_is_not_newer(&source, &requested)
            {
                continue;
            }
            if requested.exists() && options.conflict_policy == "skip" {
                continue;
            }
            let destination = if requested.exists()
                && !options.overwrite_existing
                && options.conflict_policy != "overwrite"
            {
                resolve_extract_path(&requested, options)?
            } else {
                requested
            };
            ensure_commit_target_safe(output, &destination)?;
            if let Some(parent) = destination.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent)?;
                    created_dirs.push(parent.to_path_buf());
                }
            }

            if destination.exists() {
                std::fs::create_dir_all(&rollback_root)?;
                let backup = rollback_root.join(format!("{}.bak", uuid::Uuid::new_v4()));
                std::fs::rename(&destination, &backup)?;
                backups.push((destination.clone(), backup));
            } else {
                created_files.push(destination.clone());
            }
            std::fs::rename(&source, &destination)?;
        }
        Ok(())
    })();

    if let Err(error) = commit_result {
        if let Err(rollback_errors) =
            rollback_extraction_commit(&created_files, &created_dirs, &backups)
        {
            return Err(CompressionError::ExtractionFailed(format!(
                "Extraction commit failed: {}. Rollback was incomplete: {}",
                error,
                rollback_errors.join("; ")
            ))
            .into());
        }
        return Err(error);
    }
    let _ = std::fs::remove_dir_all(&rollback_root);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staging_guard_removes_output_on_drop() {
        let temp = tempfile::tempdir().expect("temp dir");
        let output = temp.path().join("output");
        let path;
        {
            let staging = ExtractionStaging::create_for(&output).expect("staging");
            path = staging.path().to_path_buf();
            std::fs::write(path.join("partial.txt"), b"partial").expect("partial");
        }
        assert!(!path.exists());
    }

    #[test]
    fn staged_resource_scan_counts_directories_and_files() {
        let temp = tempfile::tempdir().expect("temp dir");
        let staging = temp.path().join("staging");
        std::fs::create_dir_all(staging.join("a/b")).expect("directories");
        std::fs::write(staging.join("a/b/file.txt"), b"payload").expect("file");

        assert_eq!(inspect_staged_resources(&staging).unwrap(), (3, 7));
    }

    #[test]
    fn ask_conflict_reports_without_mutating_destination() {
        let temp = tempfile::tempdir().expect("temp dir");
        let staging = temp.path().join("staging");
        let output = temp.path().join("output");
        std::fs::create_dir_all(&staging).expect("staging");
        std::fs::create_dir_all(&output).expect("output");
        std::fs::write(staging.join("same.txt"), b"new").expect("staged");
        std::fs::write(output.join("same.txt"), b"old").expect("destination");
        let options = DecompressOptions {
            conflict_policy: "ask".to_string(),
            ..Default::default()
        };
        let mut conflict = None;

        assert!(
            commit_staged_extraction("archive.zip", &staging, &output, &options, |value| {
                conflict = Some(value)
            },)
            .is_err()
        );
        assert_eq!(std::fs::read(output.join("same.txt")).unwrap(), b"old");
        assert_eq!(conflict.unwrap().file_name, "same.txt");
    }

    #[test]
    fn rollback_failures_are_reported_instead_of_silently_ignored() {
        let temp = tempfile::tempdir().expect("temp dir");
        let destination = temp.path().join("destination.txt");
        let missing_backup = temp.path().join("missing.bak");

        let errors =
            rollback_extraction_commit(&[], &[], &[(destination.clone(), missing_backup.clone())])
                .expect_err("missing backup must be reported");
        assert!(errors.iter().any(|error| {
            error.contains(&destination.to_string_lossy().to_string())
                && error.contains(&missing_backup.to_string_lossy().to_string())
        }));
    }
}
