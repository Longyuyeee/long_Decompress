use crate::services::compression_service::CompressionError;
use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(crate) struct CompressionEntry {
    pub(crate) path: PathBuf,
    pub(crate) archive_name: String,
    pub(crate) is_dir: bool,
}

fn unique_archive_name(used_archive_names: &mut HashSet<String>, raw_name: String) -> String {
    let normalized = raw_name.replace('\\', "/");
    if used_archive_names.insert(normalized.clone()) {
        return normalized;
    }

    let path = Path::new(&normalized);
    let parent = path.parent().filter(|value| !value.as_os_str().is_empty());
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("file");
    let extension = path.extension().and_then(|name| name.to_str());

    for index in 1..10_000 {
        let file_name = match extension {
            Some(ext) if !ext.is_empty() => format!("{stem} ({index}).{ext}"),
            _ => format!("{stem} ({index})"),
        };
        let candidate = parent
            .map(|dir| dir.join(&file_name))
            .unwrap_or_else(|| PathBuf::from(&file_name))
            .to_string_lossy()
            .replace('\\', "/");
        if used_archive_names.insert(candidate.clone()) {
            return candidate;
        }
    }

    normalized
}

pub(crate) fn collect(
    sources: &[String],
    preserve_paths: bool,
    include_dirs: bool,
) -> Result<Vec<CompressionEntry>> {
    let mut used_archive_names = HashSet::new();
    let mut entries = Vec::new();

    for source in sources {
        let path = Path::new(source);
        if path.is_file() {
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| {
                    CompressionError::CompressionFailed(format!("Invalid file name: {source}"))
                })?;
            entries.push(CompressionEntry {
                path: path.to_path_buf(),
                archive_name: unique_archive_name(&mut used_archive_names, file_name.to_string()),
                is_dir: false,
            });
        } else if path.is_dir() {
            let root_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("folder")
                .to_string();

            for entry in walkdir::WalkDir::new(path) {
                let entry = entry.map_err(|error| {
                    CompressionError::CompressionFailed(format!(
                        "Unable to read source tree {}: {error}",
                        path.display()
                    ))
                })?;
                let entry_path = entry.path();
                let is_dir = entry.file_type().is_dir();
                if is_dir && !include_dirs {
                    continue;
                }
                if !is_dir && !entry.file_type().is_file() {
                    continue;
                }

                let relative = entry_path
                    .strip_prefix(path)
                    .map_err(|error| CompressionError::CompressionFailed(error.to_string()))?;
                if relative.as_os_str().is_empty() {
                    continue;
                }

                let archive_name = if preserve_paths {
                    Path::new(&root_name)
                        .join(relative)
                        .to_string_lossy()
                        .replace('\\', "/")
                } else {
                    entry_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(if is_dir { "folder" } else { "file" })
                        .to_string()
                };
                entries.push(CompressionEntry {
                    path: entry_path.to_path_buf(),
                    archive_name: unique_archive_name(&mut used_archive_names, archive_name),
                    is_dir,
                });
            }
        }
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flattened_duplicate_names_remain_unique() {
        let temp = tempfile::tempdir().expect("temp dir");
        let first = temp.path().join("one");
        let second = temp.path().join("two");
        std::fs::create_dir_all(&first).expect("first folder");
        std::fs::create_dir_all(&second).expect("second folder");
        std::fs::write(first.join("same.txt"), b"one").expect("first source");
        std::fs::write(second.join("same.txt"), b"two").expect("second source");

        let entries = collect(
            &[
                first.to_string_lossy().to_string(),
                second.to_string_lossy().to_string(),
            ],
            false,
            false,
        )
        .expect("collect entries");
        let names: Vec<_> = entries
            .into_iter()
            .map(|entry| entry.archive_name)
            .collect();

        assert_eq!(names, ["same.txt", "same (1).txt"]);
    }
}
