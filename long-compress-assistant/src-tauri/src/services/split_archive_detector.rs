use anyhow::{anyhow, Result};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SplitArchiveDetector;

#[derive(Debug, Clone, PartialEq)]
pub enum SplitFormat {
    ZipSplit,
    RarSplit,
    SevenZipSplit,
    GenericNumeric,
    GenericPart,
}

#[derive(Debug, Clone)]
pub struct SplitArchiveInfo {
    pub format: SplitFormat,
    pub base_name: String,
    pub parts: Vec<PathBuf>,
    pub first_part: PathBuf,
    pub total_parts: usize,
    pub total_size: u64,
    pub is_complete: bool,
    pub missing_parts: Vec<PathBuf>,
}

impl SplitArchiveDetector {
    fn file_name(path: &Path) -> Result<&str> {
        path.file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("无效或无法转换为 Unicode 的文件名: {}", path.display()))
    }

    pub fn is_split_archive(path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };
        Self::is_zip_split(name)
            || Self::is_rar_split(name)
            || Self::is_7z_split(name)
            || Self::is_generic_numeric_split(name)
            || Self::is_generic_part_split(name)
    }

    pub fn detect_split_archive(path: &Path) -> Result<Option<SplitArchiveInfo>> {
        if !Self::is_split_archive(path) {
            return Ok(None);
        }
        let name = Self::file_name(path)?;
        let parent = path.parent().ok_or_else(|| anyhow!("无法获取父目录"))?;
        if Self::is_zip_split(name) {
            Self::detect_zip_split(path, parent)
        } else if Self::is_rar_split(name) {
            Self::detect_rar_split(path, parent)
        } else if Self::is_7z_split(name) {
            Self::detect_7z_split(path, parent)
        } else if Self::is_generic_numeric_split(name) {
            Self::detect_generic_numeric_split(path, parent)
        } else {
            Self::detect_generic_part_split(path, parent)
        }
    }

    fn collect_indexed(parent: &Path, matcher: &Regex) -> Result<Vec<(usize, PathBuf)>> {
        let mut found = Vec::new();
        for entry in fs::read_dir(parent)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let Some(captures) = matcher.captures(name) else {
                continue;
            };
            let Some(index) = captures
                .get(1)
                .and_then(|value| value.as_str().parse().ok())
            else {
                continue;
            };
            found.push((index, entry.path()));
        }
        found.sort_by_key(|(index, _)| *index);
        Ok(found)
    }

    fn actual_case_insensitive(parent: &Path, expected_name: &str) -> Result<Option<PathBuf>> {
        for entry in fs::read_dir(parent)? {
            let entry = entry?;
            if entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(expected_name)
            {
                return Ok(Some(entry.path()));
            }
        }
        Ok(None)
    }

    fn finalize(
        format: SplitFormat,
        base_name: String,
        parts: Vec<PathBuf>,
        first_part: PathBuf,
        total_parts: usize,
        missing_parts: Vec<PathBuf>,
    ) -> SplitArchiveInfo {
        let total_size = parts
            .iter()
            .filter_map(|part| fs::metadata(part).ok())
            .map(|metadata| metadata.len())
            .sum();
        SplitArchiveInfo {
            format,
            base_name,
            parts,
            first_part,
            total_parts,
            total_size,
            is_complete: missing_parts.is_empty(),
            missing_parts,
        }
    }

    fn is_zip_split(name: &str) -> bool {
        name.to_lowercase().ends_with(".zip")
            || Regex::new(r"(?i)\.z\d{2,3}$").unwrap().is_match(name)
    }

    fn detect_zip_split(path: &Path, parent: &Path) -> Result<Option<SplitArchiveInfo>> {
        let name = Self::file_name(path)?;
        let base = Regex::new(r"(?i)\.(?:zip|z\d{2,3})$")
            .unwrap()
            .replace(name, "")
            .to_string();
        let matcher = Regex::new(&format!(r"(?i)^{}\.z(\d{{2,3}})$", regex::escape(&base)))?;
        let indexed = Self::collect_indexed(parent, &matcher)?;
        if indexed.is_empty() {
            return Ok(None);
        }
        let max = indexed.last().map(|item| item.0).unwrap_or(0);
        let expected_main = parent.join(format!("{base}.zip"));
        let actual_main = Self::actual_case_insensitive(parent, &format!("{base}.zip"))?;
        let mut parts: Vec<PathBuf> = indexed.iter().map(|(_, part)| part.clone()).collect();
        if let Some(main) = &actual_main {
            parts.push(main.clone());
        }
        let mut missing = Vec::new();
        for index in 1..=max {
            if !indexed.iter().any(|(found, _)| *found == index) {
                missing.push(parent.join(format!("{base}.z{index:02}")));
            }
        }
        if actual_main.is_none() {
            missing.push(expected_main.clone());
        }
        Ok(Some(Self::finalize(
            SplitFormat::ZipSplit,
            base,
            parts,
            actual_main.unwrap_or(expected_main),
            max + 1,
            missing,
        )))
    }

    fn is_rar_split(name: &str) -> bool {
        Regex::new(r"(?i)(?:\.rar|\.r\d{2,3}|\.part\d+\.rar)$")
            .unwrap()
            .is_match(name)
    }

    fn detect_rar_split(path: &Path, parent: &Path) -> Result<Option<SplitArchiveInfo>> {
        let name = Self::file_name(path)?;
        if let Some(captures) = Regex::new(r"(?i)^(.+)\.part\d+\.rar$")
            .unwrap()
            .captures(name)
        {
            let base = captures[1].to_string();
            let matcher = Regex::new(&format!(r"(?i)^{}\.part(\d+)\.rar$", regex::escape(&base)))?;
            let indexed = Self::collect_indexed(parent, &matcher)?;
            return Self::indexed_info(
                SplitFormat::RarSplit,
                base,
                parent,
                indexed,
                1,
                |base, index| format!("{base}.part{index}.rar"),
            );
        }
        let base = Regex::new(r"(?i)\.(?:rar|r\d{2,3})$")
            .unwrap()
            .replace(name, "")
            .to_string();
        let matcher = Regex::new(&format!(r"(?i)^{}\.r(\d{{2,3}})$", regex::escape(&base)))?;
        let indexed = Self::collect_indexed(parent, &matcher)?;
        if indexed.is_empty() {
            return Ok(None);
        }
        let max = indexed.last().map(|item| item.0).unwrap_or(0);
        let expected_main = parent.join(format!("{base}.rar"));
        let actual_main = Self::actual_case_insensitive(parent, &format!("{base}.rar"))?;
        let mut parts = Vec::new();
        if let Some(main) = &actual_main {
            parts.push(main.clone());
        }
        parts.extend(indexed.iter().map(|(_, part)| part.clone()));
        let mut missing = Vec::new();
        if actual_main.is_none() {
            missing.push(expected_main.clone());
        }
        for index in 0..=max {
            if !indexed.iter().any(|(found, _)| *found == index) {
                missing.push(parent.join(format!("{base}.r{index:02}")));
            }
        }
        Ok(Some(Self::finalize(
            SplitFormat::RarSplit,
            base,
            parts,
            actual_main.unwrap_or(expected_main),
            max + 2,
            missing,
        )))
    }

    fn is_7z_split(name: &str) -> bool {
        Regex::new(r"(?i)\.7z\.\d{3,}$").unwrap().is_match(name)
    }
    fn detect_7z_split(path: &Path, parent: &Path) -> Result<Option<SplitArchiveInfo>> {
        let name = Self::file_name(path)?;
        let base = Regex::new(r"(?i)\.7z\.\d{3,}$")
            .unwrap()
            .replace(name, "")
            .to_string();
        let matcher = Regex::new(&format!(r"(?i)^{}\.7z\.(\d{{3,}})$", regex::escape(&base)))?;
        Self::indexed_info(
            SplitFormat::SevenZipSplit,
            base,
            parent,
            Self::collect_indexed(parent, &matcher)?,
            1,
            |base, index| format!("{base}.7z.{index:03}"),
        )
    }

    fn is_generic_numeric_split(name: &str) -> bool {
        Regex::new(r"\.\d{3,}$").unwrap().is_match(name)
    }
    fn detect_generic_numeric_split(
        path: &Path,
        parent: &Path,
    ) -> Result<Option<SplitArchiveInfo>> {
        let name = Self::file_name(path)?;
        let base = Regex::new(r"\.\d{3,}$")
            .unwrap()
            .replace(name, "")
            .to_string();
        let matcher = Regex::new(&format!(r"(?i)^{}\.(\d{{3,}})$", regex::escape(&base)))?;
        Self::indexed_info(
            SplitFormat::GenericNumeric,
            base,
            parent,
            Self::collect_indexed(parent, &matcher)?,
            1,
            |base, index| format!("{base}.{index:03}"),
        )
    }

    fn is_generic_part_split(name: &str) -> bool {
        Regex::new(r"(?i)\.part\d+$").unwrap().is_match(name)
    }
    fn detect_generic_part_split(path: &Path, parent: &Path) -> Result<Option<SplitArchiveInfo>> {
        let name = Self::file_name(path)?;
        let base = Regex::new(r"(?i)\.part\d+$")
            .unwrap()
            .replace(name, "")
            .to_string();
        let matcher = Regex::new(&format!(r"(?i)^{}\.part(\d+)$", regex::escape(&base)))?;
        Self::indexed_info(
            SplitFormat::GenericPart,
            base,
            parent,
            Self::collect_indexed(parent, &matcher)?,
            1,
            |base, index| format!("{base}.part{index}"),
        )
    }

    fn indexed_info<F>(
        format: SplitFormat,
        base: String,
        parent: &Path,
        indexed: Vec<(usize, PathBuf)>,
        start: usize,
        file_name: F,
    ) -> Result<Option<SplitArchiveInfo>>
    where
        F: Fn(&str, usize) -> String,
    {
        let Some(max) = indexed.last().map(|item| item.0) else {
            return Ok(None);
        };
        let mut missing = Vec::new();
        for index in start..=max {
            if !indexed.iter().any(|(found, _)| *found == index) {
                missing.push(parent.join(file_name(&base, index)));
            }
        }
        let expected_first = parent.join(file_name(&base, start));
        let first = indexed
            .iter()
            .find(|(index, _)| *index == start)
            .map(|(_, path)| path.clone())
            .unwrap_or(expected_first);
        let parts = indexed
            .into_iter()
            .map(|(_, path)| path)
            .collect::<Vec<_>>();
        Ok(Some(Self::finalize(
            format,
            base,
            parts,
            first,
            max - start + 1,
            missing,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    fn touch(path: &Path) {
        fs::write(path, b"part").unwrap();
    }

    #[test]
    fn plain_zip_and_rar_are_not_reported_as_split() {
        let dir = tempdir().unwrap();
        let zip = dir.path().join("plain.zip");
        let rar = dir.path().join("plain.rar");
        touch(&zip);
        touch(&rar);
        assert!(SplitArchiveDetector::detect_split_archive(&zip)
            .unwrap()
            .is_none());
        assert!(SplitArchiveDetector::detect_split_archive(&rar)
            .unwrap()
            .is_none());
    }

    #[test]
    fn detects_numeric_group_from_a_middle_volume() {
        let dir = tempdir().unwrap();
        for index in 1..=5 {
            touch(&dir.path().join(format!("project.zip.{index:03}")));
        }
        let info = SplitArchiveDetector::detect_split_archive(&dir.path().join("project.zip.004"))
            .unwrap()
            .unwrap();
        assert!(info.is_complete);
        assert_eq!(info.total_parts, 5);
        assert_eq!(info.parts.len(), 5);
        assert!(info.first_part.ends_with("project.zip.001"));
    }

    #[test]
    fn reports_gaps_instead_of_silently_truncating_the_group() {
        let dir = tempdir().unwrap();
        touch(&dir.path().join("project.001"));
        touch(&dir.path().join("project.003"));
        let info = SplitArchiveDetector::detect_split_archive(&dir.path().join("project.003"))
            .unwrap()
            .unwrap();
        assert!(!info.is_complete);
        assert_eq!(info.total_parts, 3);
        assert_eq!(info.missing_parts.len(), 1);
        assert!(info.missing_parts[0].ends_with("project.002"));
    }

    #[test]
    fn handles_zip_rar_and_seven_zip_naming_conventions() {
        let dir = tempdir().unwrap();
        for name in [
            "photos.z01",
            "photos.z02",
            "photos.zip",
            "backup.part1.rar",
            "backup.part2.rar",
            "legacy.rar",
            "legacy.r00",
            "legacy.r01",
            "bundle.7z.001",
            "bundle.7z.002",
        ] {
            touch(&dir.path().join(name));
        }

        let zip = SplitArchiveDetector::detect_split_archive(&dir.path().join("photos.z02"))
            .unwrap()
            .unwrap();
        assert!(zip.is_complete);
        assert!(zip.first_part.ends_with("photos.zip"));

        let rar = SplitArchiveDetector::detect_split_archive(&dir.path().join("backup.part2.rar"))
            .unwrap()
            .unwrap();
        assert!(rar.is_complete);
        assert!(rar.first_part.ends_with("backup.part1.rar"));

        let legacy = SplitArchiveDetector::detect_split_archive(&dir.path().join("legacy.r01"))
            .unwrap()
            .unwrap();
        assert!(legacy.is_complete);
        assert!(legacy.first_part.ends_with("legacy.rar"));

        let seven_zip =
            SplitArchiveDetector::detect_split_archive(&dir.path().join("bundle.7z.002"))
                .unwrap()
                .unwrap();
        assert!(seven_zip.is_complete);
        assert!(seven_zip.first_part.ends_with("bundle.7z.001"));
    }

    #[test]
    fn invalid_file_names_return_an_error_instead_of_panicking() {
        assert!(SplitArchiveDetector::file_name(Path::new("/")).is_err());
    }
}
