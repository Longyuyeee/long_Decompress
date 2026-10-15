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
        if Self::is_haozip_split(name) {
            Self::detect_haozip_split(path, parent)
        } else if Self::is_zip_split(name) {
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

    fn number_width(indexed: &[(usize, PathBuf)], matcher: &Regex, fallback: usize) -> usize {
        indexed.iter().filter_map(|(_, path)| {
            let name = path.file_name()?.to_str()?;
            Some(matcher.captures(name)?.get(1)?.as_str().len())
        }).min().unwrap_or(fallback)
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
            || Regex::new(r"(?i)\.z\d{2,5}$").unwrap().is_match(name)
    }

    // HaoZip's forum ZIPs are native ZIP disks, not independent ZIP files or
    // numeric byte slices. Their terminal EOCD declares the actual disk count.
    fn is_haozip_split(name: &str) -> bool {
        Regex::new(r"(?i)^(?:.*\.)?haozip\d{2,5}\.zip$").unwrap().is_match(name)
    }

    fn detect_haozip_split(path: &Path, parent: &Path) -> Result<Option<SplitArchiveInfo>> {
        use std::io::{Read, Seek, SeekFrom};
        let name = Self::file_name(path)?;
        let captures = Regex::new(r"(?i)^(.*?)(haozip)(\d{2,5})\.zip$").unwrap()
            .captures(name).ok_or_else(|| anyhow!("无效的好压论坛分卷名称"))?;
        let prefix = captures[1].to_string();
        let marker = captures[2].to_string();
        let base = prefix.trim_end_matches('.').to_string();
        let matcher = Regex::new(&format!(r"(?i)^{}haozip(\d{{2,5}})\.zip$", regex::escape(&prefix)))?;
        let indexed = Self::collect_indexed(parent, &matcher)?;
        let width = Self::number_width(&indexed, &matcher, 2);
        anyhow::ensure!(indexed.iter().all(|(index, _)| *index > 0)
            && indexed.windows(2).all(|pair| pair[0].0 != pair[1].0),
            "[archive-inspection:incomplete] 好压论坛分卷编号为 0 或重复，请检查同目录文件");
        let (highest, last) = indexed.last().ok_or_else(|| anyhow!("没有找到好压论坛分卷"))?;
        let mut file = fs::File::open(last)?;
        let size = file.metadata()?.len();
        let tail_size = size.min(65_557) as usize;
        file.seek(SeekFrom::End(-(tail_size as i64)))?;
        let mut tail = vec![0; tail_size];
        file.read_exact(&mut tail)?;
        let end = (0..tail.len().saturating_sub(21)).rev().find(|&index|
            &tail[index..index + 4] == b"PK\x05\x06"
                && index + 22 + u16::from_le_bytes(tail[index + 20..index + 22].try_into().unwrap()) as usize == tail.len());
        let missing_name = |index| parent.join(format!("{prefix}{marker}{index:0width$}.zip"));
        let mut total = *highest;
        if let Some(end) = end {
            let last_disk = u16::from_le_bytes(tail[end + 4..end + 6].try_into().unwrap()) as usize;
            total = if end >= 20 && &tail[end - 20..end - 16] == b"PK\x06\x07" {
                u32::from_le_bytes(tail[end - 4..end].try_into().unwrap()) as usize
            } else { last_disk + 1 };
            anyhow::ensure!(total >= *highest && total <= 65_536,
                "[archive-inspection:damaged] 好压论坛分卷终卷编号与 ZIP 目录不符，请检查是否混入其他卷组");
        }
        let missing = (1..=total).filter(|index| !indexed.iter().any(|(found, _)| found == index))
            .map(missing_name).collect::<Vec<_>>();
        // A continuous list of names is not proof that the terminal disk exists.
        // Report known gaps first; never invent a terminal filename/count.
        anyhow::ensure!(end.is_some() || !missing.is_empty(),
            "[archive-inspection:incomplete] 好压论坛分卷未找到可读的终卷目录，可能缺少后续卷或终卷已损坏；总卷数无法确认，请补齐原始分卷后重试");
        let first = indexed.iter().find(|(index, _)| *index == 1)
            .map(|(_, path)| path.clone()).unwrap_or_else(|| missing_name(1));
        Ok(Some(Self::finalize(SplitFormat::ZipSplit,
            if base.is_empty() { "archive".into() } else { base },
            indexed.into_iter().map(|(_, path)| path).collect(), first, total, missing)))
    }

    fn detect_zip_split(path: &Path, parent: &Path) -> Result<Option<SplitArchiveInfo>> {
        let name = Self::file_name(path)?;
        let base = Regex::new(r"(?i)\.(?:zip|z\d{2,5})$")
            .unwrap()
            .replace(name, "")
            .to_string();
        let matcher = Regex::new(&format!(r"(?i)^{}\.z(\d{{2,5}})$", regex::escape(&base)))?;
        let indexed = Self::collect_indexed(parent, &matcher)?;
        let width = Self::number_width(&indexed, &matcher, 2);
        let mut max = indexed.last().map(|item| item.0).unwrap_or(0);
        let expected_main = parent.join(format!("{base}.zip"));
        let actual_main = Self::actual_case_insensitive(parent, &format!("{base}.zip"))?;
        // The final directory declares the disk count. A missing highest .zNN
        // member would otherwise look complete when only existing names count.
        if let Some(main) = &actual_main {
            use std::io::{Read, Seek, SeekFrom};
            let mut file = fs::File::open(main)?;
            let size = file.metadata()?.len();
            let tail_size = size.min(65_557) as usize;
            file.seek(SeekFrom::End(-(tail_size as i64)))?;
            let mut tail = vec![0; tail_size]; file.read_exact(&mut tail)?;
            if let Some(index) = (0..tail.len().saturating_sub(21)).rev().find(|&index|
                &tail[index..index + 4] == b"PK\x05\x06"
                    && index + 22 + u16::from_le_bytes(tail[index + 20..index + 22].try_into().unwrap()) as usize == tail.len()) {
                let disk = u16::from_le_bytes(tail[index + 4..index + 6].try_into().unwrap()) as usize;
                let declared = if index >= 20 && &tail[index - 20..index - 16] == b"PK\x06\x07" {
                    (u32::from_le_bytes(tail[index - 4..index].try_into().unwrap()) as usize).saturating_sub(1)
                } else { disk };
                anyhow::ensure!(declared <= 65_535, "ZIP 卷数超过可读取范围");
                max = max.max(declared);
            }
        }
        if max == 0 && indexed.is_empty() { return Ok(None); }
        let mut parts: Vec<PathBuf> = indexed.iter().map(|(_, part)| part.clone()).collect();
        if let Some(main) = &actual_main {
            parts.push(main.clone());
        }
        let mut missing = Vec::new();
        for index in 1..=max {
            if !indexed.iter().any(|(found, _)| *found == index) {
                missing.push(parent.join(format!("{base}.z{index:0width$}")));
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
        if let Some(captures) = Regex::new(r"(?i)^(.+)\.part(\d+)\.rar$")
            .unwrap()
            .captures(name)
        {
            let base = captures[1].to_string();

            let matcher = Regex::new(&format!(r"(?i)^{}\.part(\d+)\.rar$", regex::escape(&base)))?;
            let indexed = Self::collect_indexed(parent, &matcher)?;
        let width = Self::number_width(&indexed, &matcher, 2);
            return Self::indexed_info(
                SplitFormat::RarSplit,
                base,
                parent,
                indexed,
                1,
                |base, index| format!("{base}.part{index:0width$}.rar"),
            );
        }
        let base = Regex::new(r"(?i)\.(?:rar|r\d{2,3})$")
            .unwrap()
            .replace(name, "")
            .to_string();
        let matcher = Regex::new(&format!(r"(?i)^{}\.r(\d{{2,3}})$", regex::escape(&base)))?;
        let indexed = Self::collect_indexed(parent, &matcher)?;
        let width = Self::number_width(&indexed, &matcher, 2);
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
                missing.push(parent.join(format!("{base}.r{index:0width$}")));
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
        let indexed = Self::collect_indexed(parent, &matcher)?;
        let width = Self::number_width(&indexed, &matcher, name.rsplit('.').next().unwrap().len());
        Self::indexed_info(
            SplitFormat::SevenZipSplit,
            base,
            parent,
            indexed,
            1,
            |base, index| format!("{base}.7z.{index:0width$}"),
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
        let indexed = Self::collect_indexed(parent, &matcher)?;
        let width = Self::number_width(&indexed, &matcher, name.rsplit('.').next().unwrap().len());
        Self::indexed_info(
            SplitFormat::GenericNumeric,
            base,
            parent,
            indexed,
            1,
            |base, index| format!("{base}.{index:0width$}"),
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

    fn forum_end(path: &Path, last_disk: u16) {
        let mut bytes = b"PK\x05\x06".to_vec();
        bytes.extend_from_slice(&last_disk.to_le_bytes());
        bytes.extend_from_slice(&last_disk.to_le_bytes());
        bytes.extend_from_slice(&[0; 14]);
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn forum_zip_any_member_uses_first_volume_and_exact_missing_names() {
        let dir = tempdir().unwrap();
        let first = dir.path().join("下载资料.HaoZip01.ZIP");
        let middle = dir.path().join("下载资料.haozip02.zip");
        let last = dir.path().join("下载资料.haozip03.zip");
        touch(&first); touch(&middle); forum_end(&last, 2);
        for selected in [&first, &middle, &last] {
            let info = SplitArchiveDetector::detect_split_archive(selected).unwrap().unwrap();
            assert_eq!(info.format, SplitFormat::ZipSplit);
            assert_eq!(info.first_part, first);
            assert_eq!(info.base_name, "下载资料");
            assert_eq!(info.parts, vec![first.clone(), middle.clone(), last.clone()]);
            assert!(info.is_complete); assert_eq!(info.total_parts, 3);
        }
        fs::remove_file(&middle).unwrap();
        let info = SplitArchiveDetector::detect_split_archive(&last).unwrap().unwrap();
        assert!(!info.is_complete);
        assert_eq!(info.missing_parts, vec![middle]);
        fs::remove_file(&first).unwrap();
        let info = SplitArchiveDetector::detect_split_archive(&last).unwrap().unwrap();
        assert_eq!(info.missing_parts.len(), 2);
        assert!(info.missing_parts[0].ends_with("下载资料.haozip01.zip"));
    }

    #[test]
    fn forum_zip_continuous_names_do_not_prove_the_final_volume_exists() {
        let dir = tempdir().unwrap();
        let first = dir.path().join("haozip01.zip"); touch(&first);
        let error = SplitArchiveDetector::detect_split_archive(&first).unwrap_err().to_string();
        assert!(error.contains("终卷目录"), "{error}");
        assert!(error.contains("总卷数无法确认"), "{error}");
        let last = dir.path().join("haozip02.zip"); forum_end(&last, 1);
        assert!(SplitArchiveDetector::detect_split_archive(&first).unwrap().unwrap().is_complete);
        // A similarly named ordinary ZIP must not be joined to this group.
        let plain = dir.path().join("下载资料.zip"); forum_end(&plain, 0);
        assert!(SplitArchiveDetector::detect_split_archive(&plain).unwrap().is_none());
        forum_end(&last, 0);
        assert!(SplitArchiveDetector::detect_split_archive(&last).unwrap_err().to_string().contains("编号"));
    }

    #[test]
    fn forum_zip_rejects_duplicate_numeric_members() {
        let dir = tempdir().unwrap();
        let first = dir.path().join("资料.haozip01.zip"); touch(&first);
        touch(&dir.path().join("资料.haozip001.zip"));
        assert!(SplitArchiveDetector::detect_split_archive(&first).unwrap_err().to_string().contains("重复"));
    }

    #[test]
    fn forum_zip_output_preview_removes_the_volume_marker() {
        use crate::services::compression_service::CompressionService;
        for source in ["资料.haozip01.zip", "资料.HAOZIP02.ZIP"] {
            assert_eq!(CompressionService::extraction_output_path(Path::new(source), Some("output"), true), Path::new("output").join("资料"));
        }
    }
}
