use anyhow::{ensure, Result};
use std::path::{Path, PathBuf};

pub(crate) fn recovery_volume_paths(output: &Path) -> Result<Vec<PathBuf>> {
    if !output.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("rar")) { return Ok(Vec::new()); }
    let matcher = regex::Regex::new(&format!(r"(?i)^{}\.part(\d+)\.rev$", regex::escape(&output.file_stem().unwrap().to_string_lossy())))?;
    let mut files = Vec::new();
    for entry in std::fs::read_dir(output.parent().unwrap_or(Path::new(".")))? {
        let entry = entry?;
        if let Some(captures) = matcher.captures(&entry.file_name().to_string_lossy()) {
            ensure!(entry.file_type()?.is_file() && entry.metadata()?.len() > 0, "恢复卷不是有效文件：{}", entry.path().display());
            files.push((captures[1].parse::<usize>()?, entry.path()));
        }
    }
    files.sort_by_key(|(index, _)| *index);
    Ok(files.into_iter().map(|(_, path)| path).collect())
}

/// Actual volume names, shared by publication, progress and result collection.
pub(crate) fn volume_paths(output: &Path) -> Result<Vec<(usize, PathBuf)>> {
    let name = output
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid archive name"))?;
    let rar = output
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("rar"));
    let pattern = if rar {
        format!(
            r"(?i)^{}\.part(\d+)\.rar$",
            regex::escape(output.file_stem().unwrap().to_str().unwrap())
        )
    } else {
        format!(r"(?i)^{}\.(\d{{3,}})$", regex::escape(name))
    };
    let matcher = regex::Regex::new(&pattern)?;
    let standard_matcher = output.extension().filter(|ext| ext.eq_ignore_ascii_case("zip"))
        .map(|_| regex::Regex::new(&format!(r"(?i)^{}\.z(\d{{2,5}})$", regex::escape(output.file_stem().unwrap().to_str().unwrap())))).transpose()?;
    let mut standard = Vec::new();
    let mut parts = Vec::new();
    for entry in std::fs::read_dir(output.parent().unwrap_or(Path::new(".")))? {
        let entry = entry?;
        let name = entry.file_name();
        if let Some(captures) = standard_matcher.as_ref().and_then(|matcher| matcher.captures(name.to_str().unwrap_or(""))) {
            ensure!(entry.file_type()?.is_file(), "ZIP 卷不是普通文件：{}", entry.path().display());
            let index = captures[1].parse::<usize>()?;
            ensure!(index > 0, "Invalid ZIP disk number");
            standard.push((index, entry.path()));
            continue;
        }
        let Some(captures) = matcher.captures(name.to_str().unwrap_or("")) else {
            continue;
        };
        ensure!(
            entry.file_type()?.is_file(),
            "Volume is not a regular file: {}",
            entry.path().display()
        );
        let index = captures[1].parse::<usize>()?;
        ensure!(index > 0, "Invalid volume number");
        parts.push((index, entry.path()));
    }
    parts.sort_by_key(|part| part.0);
    if !standard.is_empty() || (standard_matcher.is_some() && output.is_file() && parts.is_empty()) {
        standard.sort_by_key(|part| part.0);
        if output.is_file() { standard.push((standard.last().map(|part| part.0 + 1).unwrap_or(1), output.to_path_buf())); }
        return Ok(standard);
    }
    Ok(parts)
}

pub(crate) fn complete_volume_paths(output: &Path) -> Result<Vec<PathBuf>> {
    let parts = volume_paths(output)?;
    ensure!(!parts.is_empty(), "没有找到压缩产物的分卷");
    if is_standard_zip_set(output, &parts) {
        let selected = if output.is_file() { output } else { &parts[0].1 };
        if let Some(group) = super::split_archive_detector::SplitArchiveDetector::detect_split_archive(selected)? {
            ensure!(group.is_complete, "缺少 ZIP 分卷：{:?}", group.missing_parts);
        } else { ensure!(parts.len() == 1 && output.is_file(), "缺少 ZIP 终卷"); }
        ensure!(output.is_file(), "缺少 ZIP 终卷");
    }
    for (index, (number, path)) in parts.iter().enumerate() {
        ensure!(*number == index + 1, "分卷编号不连续：{}", path.display());
        ensure!(path.metadata()?.len() > 0, "分卷为空：{}", path.display());
    }
    Ok(parts.into_iter().map(|(_, path)| path).collect())
}

fn is_standard_zip_set(output: &Path, parts: &[(usize, PathBuf)]) -> bool {
    output.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
        && parts.iter().any(|(_, path)| path == output || path.extension().is_some_and(|ext| ext.to_string_lossy().to_ascii_lowercase().starts_with('z')))
}

pub(crate) fn first_volume_path(output: &Path) -> PathBuf {
    let paths = volume_paths(output).ok();
    if paths.as_ref().is_some_and(|parts| is_standard_zip_set(output, parts)) { return output.to_path_buf(); }
    paths
        .and_then(|parts| parts.first().map(|(_, path)| path.clone()))
        .unwrap_or_else(|| {
            if output
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("rar"))
            {
                output.with_file_name(format!(
                    "{}.part1.rar",
                    output.file_stem().unwrap().to_string_lossy()
                ))
            } else {
                PathBuf::from(format!("{}.001", output.display()))
            }
        })
}
