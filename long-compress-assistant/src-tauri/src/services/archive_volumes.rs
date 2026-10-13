use anyhow::{ensure, Result};
use std::path::{Path, PathBuf};

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
    let mut parts = Vec::new();
    for entry in std::fs::read_dir(output.parent().unwrap_or(Path::new(".")))? {
        let entry = entry?;
        let name = entry.file_name();
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
    Ok(parts)
}

pub(crate) fn complete_volume_paths(output: &Path) -> Result<Vec<PathBuf>> {
    let parts = volume_paths(output)?;
    ensure!(!parts.is_empty(), "没有找到压缩产物的分卷");
    for (index, (number, path)) in parts.iter().enumerate() {
        ensure!(*number == index + 1, "分卷编号不连续：{}", path.display());
        ensure!(path.metadata()?.len() > 0, "分卷为空：{}", path.display());
    }
    Ok(parts.into_iter().map(|(_, path)| path).collect())
}

pub(crate) fn first_volume_path(output: &Path) -> PathBuf {
    volume_paths(output)
        .ok()
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
