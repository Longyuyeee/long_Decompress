use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::models::compression::DecompressOptions;

use super::compression_service::CompressionError;

#[cfg(windows)]
const FILE_ATTRIBUTE_HIDDEN: u32 = 0x0000_0002;
#[cfg(windows)]
const FILE_ATTRIBUTE_NOT_CONTENT_INDEXED: u32 = 0x0000_2000;
#[cfg(windows)]
const INVALID_FILE_ATTRIBUTES: u32 = u32::MAX;
#[cfg(windows)]
const SHCNE_UPDATEDIR: i32 = 0x0000_1000;
#[cfg(windows)]
const SHCNF_PATHW: u32 = 0x0005;
#[cfg(windows)]
const SHCNF_FLUSHNOWAIT: u32 = 0x2000;

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetFileAttributesW(file_name: *const u16) -> u32;
    fn SetFileAttributesW(file_name: *const u16, attributes: u32) -> i32;
}

#[cfg(windows)]
#[link(name = "shell32")]
extern "system" {
    fn SHChangeNotify(
        event_id: i32,
        flags: u32,
        item1: *const std::ffi::c_void,
        item2: *const std::ffi::c_void,
    );
}

#[cfg(windows)]
fn wide_path(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

#[cfg(windows)]
fn conceal_staging_directory(path: &Path) -> std::io::Result<()> {
    let path = wide_path(path);
    // SAFETY: `path` is a NUL-terminated UTF-16 buffer that remains alive for
    // both calls. SetFileAttributesW does not retain the pointer.
    let attributes = unsafe { GetFileAttributesW(path.as_ptr()) };
    if attributes == INVALID_FILE_ATTRIBUTES {
        return Err(std::io::Error::last_os_error());
    }
    let updated = attributes | FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_NOT_CONTENT_INDEXED;
    if unsafe { SetFileAttributesW(path.as_ptr(), updated) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(windows))]
fn conceal_staging_directory(_: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(windows)]
fn notify_explorer_directory_changed(path: &Path) {
    let path = wide_path(path);
    // SAFETY: SHCNF_PATHW requires a NUL-terminated UTF-16 path in item1. The
    // buffer is alive for the duration of the synchronous notification call.
    unsafe {
        SHChangeNotify(
            SHCNE_UPDATEDIR,
            SHCNF_PATHW | SHCNF_FLUSHNOWAIT,
            path.as_ptr().cast(),
            std::ptr::null(),
        );
    }
}

#[cfg(not(windows))]
fn notify_explorer_directory_changed(_: &Path) {}

pub(crate) const MAX_EXTRACTED_ENTRIES: usize = 250_000;
pub(crate) const MAX_EXTRACTED_BYTES: u64 = 512 * 1024 * 1024 * 1024;
pub(crate) const MAX_EXPANSION_RATIO: u64 = 10_000;
pub(crate) use super::storage_preflight::DISK_SAFETY_RESERVE;

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
            // A Windows volume root (for example H:\\) has no parent. The
            // staging directory must still live on that volume; falling back
            // to the process directory makes the final atomic rename cross a
            // volume and fails with os error 17.
            .unwrap_or_else(|| if output.is_absolute() { output } else { Path::new(".") });
        std::fs::create_dir_all(parent)?;
        let path = parent.join(format!(".long-extract-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path)?;
        if let Err(error) = conceal_staging_directory(&path) {
            log::warn!(
                "Unable to hide extraction staging directory {}: {}",
                path.display(),
                error
            );
        }
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
                if let Some(parent) = self.path.parent() {
                    notify_explorer_directory_changed(parent);
                }
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.cleaned = true;
                if let Some(parent) = self.path.parent() {
                    notify_explorer_directory_changed(parent);
                }
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

pub(crate) fn validate_disk_capacity(path: &Path, expanded_bytes: u64) -> Result<()> {
    if super::storage_preflight::available_disk_space(path)
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

fn normalized_selected_entry(value: &str) -> Option<String> {
    let value = value.trim().replace('\\', "/");
    if value.is_empty() || value.starts_with('/') || value.contains('\0') {
        return None;
    }
    let mut parts = Vec::new();
    for part in value.split('/') {
        match part {
            "" | "." => {}
            ".." => return None,
            part => parts.push(part),
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

pub(crate) fn matches_selected_entries(path: &Path, selected_entries: &[String]) -> bool {
    if selected_entries.is_empty() {
        return true;
    }
    let candidate = path.to_string_lossy().replace('\\', "/");
    let Some(candidate) = normalized_selected_entry(&candidate) else {
        return false;
    };
    selected_entries.iter().any(|selected| {
        normalized_selected_entry(selected).is_some_and(|selected| {
            candidate == selected || candidate.starts_with(&format!("{selected}/"))
        })
    })
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
    let mut retained_file_count = 0usize;
    let mut expanded_bytes = 0u64;
    for source in files {
        let relative = source.strip_prefix(staging)?;
        if !matches_compiled_file_filter(relative, &file_filter)
            || !matches_selected_entries(relative, &options.selected_entries)
        {
            std::fs::remove_file(&source)?;
            continue;
        }
        let source_size = std::fs::metadata(&source)?.len();
        retained_file_count = retained_file_count.saturating_add(1);
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
    if !options.selected_entries.is_empty() && retained_file_count == 0 {
        return Err(CompressionError::ExtractionFailed(
            "None of the selected archive entries matched the extracted file paths".to_string(),
        )
        .into());
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
    on_conflict: impl FnMut(FileConflict),
) -> Result<()> {
    commit_staged_extraction_with_resolutions(
        source_archive,
        staging,
        output,
        options,
        &HashMap::new(),
        None,
        on_conflict,
    )
}

pub(crate) fn commit_staged_extraction_with_resolutions(
    source_archive: &str,
    staging: &Path,
    output: &Path,
    options: &DecompressOptions,
    resolutions: &HashMap<String, String>,
    fallback_action: Option<&str>,
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
        let mut unresolved = false;
        for relative in files.iter().filter(|relative| {
            let destination = output.join(relative);
            destination.exists()
                && !(options.extract_only_newer
                    && staged_file_is_not_newer(&staging.join(relative), &destination))
        }) {
            let destination = output.join(relative);
            let destination_key = destination.to_string_lossy().into_owned();
            if resolutions.contains_key(&destination_key) || fallback_action.is_some() {
                continue;
            }
            unresolved = true;
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
        }
        if unresolved {
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

            let requested_key = requested.to_string_lossy().into_owned();
            let conflict_action = resolutions
                .get(&requested_key)
                .map(String::as_str)
                .or(fallback_action)
                .unwrap_or(options.conflict_policy.as_str());

            if options.extract_only_newer
                && requested.exists()
                && staged_file_is_not_newer(&source, &requested)
            {
                continue;
            }
            if requested.exists() && conflict_action == "skip" {
                continue;
            }
            let destination = if requested.exists() && conflict_action == "rename" {
                let mut rename_options = options.clone();
                rename_options.overwrite_existing = false;
                rename_options.conflict_policy = "rename".to_string();
                resolve_extract_path(&requested, &rename_options)?
            } else if requested.exists()
                && conflict_action != "overwrite"
                && !options.overwrite_existing
            {
                return Err(CompressionError::ExtractionFailed(format!(
                    "File conflict has no valid resolution: {}",
                    requested.display()
                ))
                .into());
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

    #[cfg(windows)]
    #[test]
    fn volume_root_staging_stays_on_the_selected_volume() {
        let root = std::env::current_dir()
            .expect("current directory")
            .ancestors()
            .last()
            .expect("volume root")
            .to_path_buf();
        let staging = ExtractionStaging::create_for(&root).expect("root staging");
        assert_eq!(staging.path().parent(), Some(root.as_path()));
    }

    #[cfg(windows)]
    #[test]
    fn staging_directory_is_hidden_from_explorer() {
        let temp = tempfile::tempdir().expect("temp dir");
        let output = temp.path().join("output");
        let staging = ExtractionStaging::create_for(&output).expect("staging");
        let wide = wide_path(staging.path());

        // SAFETY: `wide` is a live, NUL-terminated UTF-16 path buffer.
        let attributes = unsafe { GetFileAttributesW(wide.as_ptr()) };
        assert_ne!(attributes, INVALID_FILE_ATTRIBUTES);
        assert_ne!(attributes & FILE_ATTRIBUTE_HIDDEN, 0);
        assert_ne!(attributes & FILE_ATTRIBUTE_NOT_CONTENT_INDEXED, 0);
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
        for name in ["same.txt", "second.txt"] {
            std::fs::write(staging.join(name), b"new").expect("staged");
            std::fs::write(output.join(name), b"old").expect("destination");
        }
        let options = DecompressOptions {
            conflict_policy: "ask".to_string(),
            ..Default::default()
        };
        let mut conflicts = Vec::new();

        assert!(
            commit_staged_extraction("archive.zip", &staging, &output, &options, |value| {
                conflicts.push(value)
            },)
            .is_err()
        );
        assert_eq!(std::fs::read(output.join("same.txt")).unwrap(), b"old");
        assert_eq!(std::fs::read(output.join("second.txt")).unwrap(), b"old");
        assert_eq!(conflicts.len(), 2);
        assert!(conflicts.iter().any(|item| item.file_name == "same.txt"));
        assert!(conflicts.iter().any(|item| item.file_name == "second.txt"));
    }

    #[test]
    fn resolved_conflicts_commit_once_with_mixed_actions() {
        let temp = tempfile::tempdir().expect("temp dir");
        let staging = temp.path().join("staging");
        let output = temp.path().join("output");
        std::fs::create_dir_all(&staging).expect("staging");
        std::fs::create_dir_all(&output).expect("output");
        for name in ["overwrite.txt", "skip.txt", "rename.txt"] {
            std::fs::write(staging.join(name), format!("new-{name}"))
                .expect("staged file");
            std::fs::write(output.join(name), format!("old-{name}"))
                .expect("destination file");
        }
        let options = DecompressOptions {
            conflict_policy: "ask".to_string(),
            ..Default::default()
        };
        let resolutions = HashMap::from([
            (output.join("overwrite.txt").to_string_lossy().into_owned(), "overwrite".to_string()),
            (output.join("skip.txt").to_string_lossy().into_owned(), "skip".to_string()),
            (output.join("rename.txt").to_string_lossy().into_owned(), "rename".to_string()),
        ]);

        commit_staged_extraction_with_resolutions(
            "archive.zip", &staging, &output, &options, &resolutions, None,
            |_| panic!("every conflict has an explicit resolution"),
        ).expect("resolved commit");

        assert_eq!(std::fs::read_to_string(output.join("overwrite.txt")).unwrap(), "new-overwrite.txt");
        assert_eq!(std::fs::read_to_string(output.join("skip.txt")).unwrap(), "old-skip.txt");
        assert_eq!(std::fs::read_to_string(output.join("rename.txt")).unwrap(), "old-rename.txt");
        assert_eq!(std::fs::read_to_string(output.join("rename (1).txt")).unwrap(), "new-rename.txt");
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

    #[cfg(windows)]
    #[test]
    fn rollback_restores_the_original_mark_of_web_stream() {
        use crate::services::mark_of_web;

        let temp = tempfile::tempdir().expect("temp dir");
        let archive = temp.path().join("download.zip");
        std::fs::write(&archive, b"archive").expect("archive fixture");
        let internet_zone = b"[ZoneTransfer]\r\nZoneId=3\r\n";
        std::fs::write(
            format!("{}:Zone.Identifier", archive.display()),
            internet_zone,
        )
        .expect("archive zone stream");
        let mark = mark_of_web::read_from(&archive)
            .expect("read archive mark")
            .expect("archive is marked");

        let staging = temp.path().join("staging");
        let output = temp.path().join("output");
        std::fs::create_dir_all(staging.join("z")).expect("staging tree");
        std::fs::create_dir_all(&output).expect("output tree");
        std::fs::write(staging.join("a.txt"), b"new").expect("staged overwrite");
        std::fs::write(staging.join("z/child.txt"), b"child").expect("later staged file");
        mark_of_web::propagate_to_tree(&staging, &mark, || false).expect("propagate mark");

        let destination = output.join("a.txt");
        let original_zone = b"[ZoneTransfer]\r\nZoneId=2\r\n";
        std::fs::write(&destination, b"old").expect("old destination");
        std::fs::write(
            format!("{}:Zone.Identifier", destination.display()),
            original_zone,
        )
        .expect("original destination zone stream");
        std::fs::write(output.join("z"), b"blocks directory creation").expect("blocking file");

        let result = commit_staged_extraction(
            "archive.zip",
            &staging,
            &output,
            &DecompressOptions {
                overwrite_existing: true,
                conflict_policy: "overwrite".to_string(),
                ..Default::default()
            },
            |_| {},
        );

        assert!(result.is_err());
        assert_eq!(std::fs::read(&destination).unwrap(), b"old");
        assert_eq!(
            std::fs::read(format!("{}:Zone.Identifier", destination.display())).unwrap(),
            original_zone
        );
        assert!(!output.join("z/child.txt").exists());
    }

    #[test]
    fn extract_only_newer_handles_older_equal_and_newer_staged_files() {
        let scenarios = [
            ("older", 1_000, 2_000, b"destination".as_slice()),
            ("equal", 2_000, 2_000, b"destination".as_slice()),
            ("newer", 3_000, 2_000, b"staged".as_slice()),
        ];

        for (name, staged_seconds, destination_seconds, expected) in scenarios {
            let temp = tempfile::tempdir().expect("temp dir");
            let staging = temp.path().join("staging");
            let output = temp.path().join("output");
            std::fs::create_dir_all(&staging).expect("staging");
            std::fs::create_dir_all(&output).expect("output");
            let staged = staging.join("same.txt");
            let destination = output.join("same.txt");
            std::fs::write(&staged, b"staged").expect("staged fixture");
            std::fs::write(&destination, b"destination").expect("destination fixture");
            filetime::set_file_mtime(
                &staged,
                filetime::FileTime::from_unix_time(staged_seconds, 0),
            )
            .expect("staged timestamp");
            filetime::set_file_mtime(
                &destination,
                filetime::FileTime::from_unix_time(destination_seconds, 0),
            )
            .expect("destination timestamp");

            commit_staged_extraction(
                "archive.7z",
                &staging,
                &output,
                &DecompressOptions {
                    extract_only_newer: true,
                    overwrite_existing: true,
                    conflict_policy: "overwrite".to_string(),
                    ..Default::default()
                },
                |_| panic!("extract-only-newer must not request conflict resolution"),
            )
            .unwrap_or_else(|error| panic!("{name} scenario failed: {error}"));

            assert_eq!(
                std::fs::read(&destination).expect("committed destination"),
                expected,
                "{name} scenario"
            );
        }
    }

    #[test]
    fn compiled_file_filter_preserves_multi_pattern_matching() {
        let filter = compile_file_filter(Some("*.txt; assets/*.PNG"));

        assert!(matches_compiled_file_filter(
            Path::new("notes/readme.TXT"),
            &filter,
        ));
        assert!(matches_compiled_file_filter(
            Path::new("assets/logo.png"),
            &filter,
        ));
        assert!(!matches_compiled_file_filter(
            Path::new("assets/logo.svg"),
            &filter,
        ));
        assert!(matches_compiled_file_filter(Path::new("anything.bin"), &[]));
    }

    #[test]
    fn selected_entries_are_exact_safe_archive_paths() {
        let selected = vec!["docs/readme.txt".to_string(), "images".to_string()];
        assert!(matches_selected_entries(
            Path::new("docs/readme.txt"),
            &selected
        ));
        assert!(matches_selected_entries(
            Path::new("images/icon.png"),
            &selected
        ));
        assert!(!matches_selected_entries(
            Path::new("docs/readme.txt.bak"),
            &selected
        ));
        assert!(!matches_selected_entries(
            Path::new("other/icon.png"),
            &selected
        ));
        assert!(!matches_selected_entries(
            Path::new("../docs/readme.txt"),
            &selected
        ));
    }

    #[test]
    fn staging_layout_keeps_only_exact_selected_files() {
        let temp = tempfile::tempdir().expect("temp dir");
        let staging = temp.path().join("staging");
        std::fs::create_dir_all(staging.join("docs")).unwrap();
        std::fs::create_dir_all(staging.join("images")).unwrap();
        std::fs::write(staging.join("docs/readme.txt"), b"keep").unwrap();
        std::fs::write(staging.join("docs/notes.txt"), b"remove").unwrap();
        std::fs::write(staging.join("images/icon.png"), b"remove").unwrap();
        let archive = temp.path().join("fixture.zip");
        std::fs::write(&archive, b"fixture").unwrap();

        prepare_staging_layout(
            &archive,
            &staging,
            &DecompressOptions {
                preserve_paths: true,
                selected_entries: vec!["docs/readme.txt".to_string()],
                ..Default::default()
            },
        )
        .unwrap();

        assert!(staging.join("docs/readme.txt").is_file());
        assert!(!staging.join("docs/notes.txt").exists());
        assert!(!staging.join("images/icon.png").exists());
    }

    #[test]
    fn staging_layout_rejects_empty_explicit_selection_results() {
        let temp = tempfile::tempdir().expect("temp dir");
        let staging = temp.path().join("staging");
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::write(staging.join("actual.txt"), b"payload").unwrap();
        let archive = temp.path().join("fixture.zip");
        std::fs::write(&archive, b"fixture").unwrap();

        let error = prepare_staging_layout(
            &archive,
            &staging,
            &DecompressOptions {
                preserve_paths: true,
                selected_entries: vec!["missing.txt".to_string()],
                ..Default::default()
            },
        )
        .expect_err("an explicit selection must never report success with no output");

        assert!(error.to_string().contains("selected archive entries"));
    }
}
