use crate::services::compression_service::CompressionError;
use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(crate) struct CompressionEntry {
    pub(crate) path: PathBuf,
    pub(crate) archive_name: String,
    pub(crate) is_dir: bool,
}

/// ZIP can encode the selected root itself. Keep the legacy TAR/7Z and flat
/// layouts unchanged; reserve each ZIP root once before remapping its children.
pub(crate) fn collect_zip(sources:&[String],preserve_paths:bool)->Result<Vec<CompressionEntry>>{
    use anyhow::Context;
    if !preserve_paths{return collect(sources,false,true);}
    let mut roots=HashSet::new();let mut seen=HashSet::new();let mut result=Vec::new();
    for source in sources{
        let path=Path::new(source);let identity=path.canonicalize().with_context(||format!("无法读取压缩来源：{source}"))?;
        if !seen.insert(identity){continue;}
        let is_dir=path.is_dir();let original=path.file_name().and_then(|n|n.to_str()).or(if is_dir{Some("folder")}else{None}).ok_or_else(||anyhow::anyhow!("压缩来源没有有效名称：{source}"))?;
        let mut name=original.to_owned();let mut suffix=1;
        while !roots.insert(name.to_lowercase()){
            anyhow::ensure!(suffix<10_000,"同名压缩来源过多：{source}");
            name=if is_dir{format!("{original} ({suffix})")}else{
                let stem=Path::new(original).file_stem().and_then(|n|n.to_str()).unwrap_or(original);
                match Path::new(original).extension().and_then(|n|n.to_str()).filter(|v|!v.is_empty()){
                    Some(extension)=>format!("{stem} ({suffix}).{extension}"),None=>format!("{stem} ({suffix})")
                }
            };suffix+=1;
        }
        if is_dir{result.push(CompressionEntry{path:path.into(),archive_name:name.clone(),is_dir:true});}
        for mut entry in collect(std::slice::from_ref(source),true,true)?{
            if entry.path==path&&is_dir{continue;} // Empty root was already added.
            entry.archive_name=if let Some(relative)=entry.archive_name.strip_prefix(&format!("{original}/")){format!("{name}/{relative}")}else{name.clone()};
            result.push(entry);
        }
    }
    Ok(result)
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
            let mut has_descendants = false;

            for entry in walkdir::WalkDir::new(path) {
                let entry = entry.map_err(|error| {
                    CompressionError::CompressionFailed(format!(
                        "Unable to read source tree {}: {error}",
                        path.display()
                    ))
                })?;
                has_descendants |= entry.depth() > 0;
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
            // A conversion passes top-level directories as separate sources.
            // An empty source still represents a directory the user selected.
            // Nonempty roots retain the existing relative-path layout.
            if include_dirs && !has_descendants {
                entries.push(CompressionEntry {
                    path: path.to_path_buf(),
                    archive_name: unique_archive_name(&mut used_archive_names, root_name),
                    is_dir: true,
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
    fn explicitly_selected_empty_roots_survive_mixed_inputs_without_renaming_children() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("one/空目录");
        let second = temp.path().join("two/空目录");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        let file = temp.path().join("说明.txt");
        std::fs::write(&file, b"keep every selected input").unwrap();
        let sources = [first.to_string_lossy().into_owned(), file.to_string_lossy().into_owned(), second.to_string_lossy().into_owned()];
        for preserve in [true, false] {
            let entries = collect(&sources, preserve, true).unwrap();
            assert_eq!(entries.iter().map(|e| e.archive_name.as_str()).collect::<Vec<_>>(), ["空目录", "说明.txt", "空目录 (1)"]);
            assert_eq!(entries.iter().map(|e| e.is_dir).collect::<Vec<_>>(), [true, false, true]);
        }
        assert_eq!(collect(&sources, true, false).unwrap().len(), 1);
    }

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
