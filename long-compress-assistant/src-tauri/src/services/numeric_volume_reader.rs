//! Seekable numeric ZIP/7Z volumes, with only one open volume handle at a time.
use std::{fs::File, io::{self, Read, Seek, SeekFrom}, path::{Path, PathBuf}};

/// Archive consumers must read the complete numeric volume set, not just its first file.
pub(crate) enum ZipInputReader {
    File(File),
    SevenZipSfx { file: File, start: u64, length: u64, position: u64 },
    Volumes(NumericVolumeReader),
    StandardZip(super::standard_zip_reader::StandardZipReader),
}

impl ZipInputReader {
    /// Recovery must read original disks even when their central directory is lost.
    pub(crate) fn open_recovery(path: &Path) -> anyhow::Result<Self> {
        if let Some(group) = super::split_archive_detector::SplitArchiveDetector::detect_split_archive(path)?
            .filter(|group| group.format == super::split_archive_detector::SplitFormat::ZipSplit)
        {
            anyhow::ensure!(group.is_complete, "缺少 ZIP 分卷：{:?}", group.missing_parts);
            return Ok(Self::Volumes(NumericVolumeReader::from_paths(group.parts)?));
        }
        Self::open(path)
    }

    pub(crate) fn open_seven_zip(path: &Path) -> anyhow::Result<Self> {
        if let Some(data) = super::sfx_volumes::seven_zip_loader_data(path)? {
            let source = super::split_archive_detector::SplitArchiveDetector::complete_source(&data)?;
            anyhow::ensure!(source.is_file(), "[archive-inspection:incomplete] 缺少 7Z 外置数据首卷：{}", source.display());
            return Self::open_seven_zip(&source);
        }
        if super::sfx_archive::is_executable_path(path) {
            if let Some((super::archive_format::ArchiveFormat::SevenZip, start)) = super::sfx_archive::detect(path)? {
                let mut file = File::open(path)?;
                let length = file.metadata()?.len().checked_sub(start)
                    .ok_or_else(|| anyhow::anyhow!("[archive-inspection:incomplete] 自解压文件在读取期间被截断，请确认下载完成后重试"))?;
                file.seek(SeekFrom::Start(start))?;
                return Ok(Self::SevenZipSfx { file, start, length, position: 0 });
            }
        }
        if Self::is_numeric_first(path) {
            Ok(Self::Volumes(NumericVolumeReader::open(path).map_err(|error|
                if super::archive_io::kind(&error).is_some() {
                    error.context("[archive-inspection:io] 无法读取 7Z 卷组，请检查原始读写原因")
                } else { error.context("[archive-inspection:incomplete] 无法读取完整 7Z 卷组") })?))
        } else {
            Self::open(path)
        }
    }

    pub(crate) fn is_numeric_first(path: &Path) -> bool {
        path.extension().and_then(|value| value.to_str()).is_some_and(|value|
            value.len() >= 3 && value.bytes().all(|byte| byte.is_ascii_digit()) && value.parse::<usize>().ok() == Some(1))
    }

    pub(crate) fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(group) = super::zpaq_multipart::inspect(path)? { return Ok(Self::Volumes(NumericVolumeReader::from_paths(group.parts)?)); }
        if super::split_archive_detector::SplitArchiveDetector::is_haozip_seven_zip(path) {
            let group = super::split_archive_detector::SplitArchiveDetector::detect_split_archive(path)?
                .ok_or_else(|| anyhow::anyhow!("[archive-inspection:incomplete] 未找到好压 7Z 论坛卷组"))?;
            anyhow::ensure!(group.is_complete,
                "[archive-inspection:incomplete] 缺少好压 7Z 论坛分卷：{}", group.missing_part_names());
            let mut reader = NumericVolumeReader::from_paths(group.parts)?;
            let length = reader.seek(SeekFrom::End(0))?;
            super::native_extraction::seven_zip::validate_archive_extent(&mut reader, length)?;
            reader.seek(SeekFrom::Start(0))?;
            Ok(Self::Volumes(reader))
        } else if Self::is_numeric_first(path) {
            Ok(Self::Volumes(NumericVolumeReader::open(path)?))
        } else {
            if let Some(reader) = super::standard_zip_reader::StandardZipReader::try_open(path)? {
                return Ok(Self::StandardZip(reader));
            }
            Ok(Self::File(File::open(path)?))
        }
    }
}

impl Read for ZipInputReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if let Self::SevenZipSfx { file, length, position, .. } = self {
            let limit = length.saturating_sub(*position).min(buffer.len() as u64) as usize;
            let read = file.read(&mut buffer[..limit])?;
            *position += read as u64;
            return Ok(read);
        }
        match self { Self::File(reader) => reader.read(buffer), Self::Volumes(reader) => reader.read(buffer), Self::StandardZip(reader) => reader.read(buffer), Self::SevenZipSfx { .. } => unreachable!() }
    }
}

impl Seek for ZipInputReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        if let Self::SevenZipSfx { file, start, length, position: current } = self {
            let target = match position { SeekFrom::Start(value) => value as i128, SeekFrom::End(delta) => *length as i128 + delta as i128, SeekFrom::Current(delta) => *current as i128 + delta as i128 };
            let target = u64::try_from(target).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid SFX seek"))?;
            let physical = start.checked_add(target).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "SFX seek overflow"))?;
            file.seek(SeekFrom::Start(physical))?;
            *current = target; return Ok(target);
        }
        match self { Self::File(reader) => reader.seek(position), Self::Volumes(reader) => reader.seek(position), Self::StandardZip(reader) => reader.seek(position), Self::SevenZipSfx { .. } => unreachable!() }
    }
}

pub(crate) struct NumericVolumeReader {
    parts: Vec<(PathBuf, u64, u64)>,
    length: u64,
    position: u64,
    current: Option<(usize, File)>,
}

impl NumericVolumeReader {
    pub(crate) fn open(first: &Path) -> anyhow::Result<Self> {
        anyhow::ensure!(ZipInputReader::is_numeric_first(first), "Expected first numeric volume (number 1)");
        let format = "archive";
        let info = super::split_archive_detector::SplitArchiveDetector::detect_split_archive(first)?
            .ok_or_else(|| anyhow::anyhow!("Numeric {format} volumes not found"))?;
        anyhow::ensure!(info.missing_parts.is_empty(), "Missing {format} volumes: {:?}", info.missing_parts);
        Self::from_paths(info.parts)
    }

    pub(crate) fn from_paths(paths: Vec<PathBuf>) -> anyhow::Result<Self> {
        let mut length = 0u64;
        let mut parts = Vec::new();
        for path in paths {
            let metadata = path.metadata()?;
            anyhow::ensure!(metadata.is_file() && metadata.len() > 0, "Invalid archive volume: {}", path.display());
            let start = length;
            length = length.checked_add(metadata.len()).ok_or_else(|| anyhow::anyhow!("Archive size overflow"))?;
            parts.push((path, start, length));
        }
        anyhow::ensure!(!parts.is_empty(), "No archive volumes found");
        Ok(Self { parts, length, position: 0, current: None })
    }
}

impl Read for NumericVolumeReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() || self.position >= self.length { return Ok(0); }
        let index = self.parts.partition_point(|(_, _, end)| *end <= self.position);
        let (path, start, end) = &self.parts[index];
        if self.current.as_ref().map(|(i, _)| *i) != Some(index) {
            self.current = None;
            self.current = Some((index, File::open(path)?));
        }
        let file = &mut self.current.as_mut().unwrap().1;
        file.seek(SeekFrom::Start(self.position - start))?;
        let limit = (end - self.position).min(buffer.len() as u64) as usize;
        let read = file.read(&mut buffer[..limit])?;
        if read == 0 { return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Archive volume changed or truncated")); }
        self.position += read as u64;
        Ok(read)
    }
}

impl Seek for NumericVolumeReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let value = match position {
            SeekFrom::Start(value) => value as i128,
            SeekFrom::End(delta) => self.length as i128 + delta as i128,
            SeekFrom::Current(delta) => self.position as i128 + delta as i128,
        };
        self.position = u64::try_from(value).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid volume seek"))?;
        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn genuine_haozip_forum_seven_zip_rechecks_gaps_tail_and_header_before_password() {
        use super::super::split_archive_detector::SplitArchiveDetector;
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/archives/haozip-forum-7z-1417");
        for stem in ["论坛7Z内容密码", "论坛7Z文件名密码"] {
            let temp = tempfile::tempdir().unwrap();
            let parts: Vec<_> = (1..=3).map(|index| {
                let name = format!("{stem}.haozip{index:02}.7z");
                let to = temp.path().join(&name);
                std::fs::copy(fixtures.join(name), &to).unwrap(); to
            }).collect();
            let original: Vec<_> = parts.iter().map(|part| std::fs::read(part).unwrap()).collect();
            for member in &parts {
                let info = SplitArchiveDetector::detect_split_archive(member).unwrap().unwrap();
                assert!(info.is_complete);
                assert_eq!(info.base_name, stem);
                assert_eq!(info.parts, parts);
                let mut reader = ZipInputReader::open_seven_zip(member).unwrap();
                let mut bytes = Vec::new(); reader.read_to_end(&mut bytes).unwrap();
                assert_eq!(bytes, original.concat());
                reader.seek(SeekFrom::Start(65_534)).unwrap();
                let mut boundary = [0; 6]; reader.read_exact(&mut boundary).unwrap();
                assert_eq!(&boundary, &bytes[65_534..65_540]);
            }
            for missing in 0..3 {
                std::fs::remove_file(&parts[missing]).unwrap();
                let selected = &parts[if missing == 0 { 2 } else { 0 }];
                if missing < 2 {
                    let info = SplitArchiveDetector::detect_split_archive(selected).unwrap().unwrap();
                    assert!(!info.is_complete);
                    assert_eq!(info.missing_parts, vec![parts[missing].clone()]);
                }
                let error = ZipInputReader::open_seven_zip(selected).err().unwrap().to_string();
                assert!(error.contains("[archive-inspection:incomplete]"), "{error}");
                assert!(error.contains(if missing < 2 { parts[missing].file_name().unwrap().to_str().unwrap() } else { "可能缺少尾卷" }), "{error}");
                std::fs::write(&parts[missing], &original[missing]).unwrap();
                assert!(ZipInputReader::open_seven_zip(selected).is_ok());
            }
            std::fs::write(&parts[2], &original[2][..original[2].len() - 1]).unwrap();
            assert!(ZipInputReader::open_seven_zip(&parts[1]).err().unwrap().to_string().contains("[archive-inspection:incomplete]"));
            std::fs::write(&parts[2], &original[2]).unwrap();
            let mut damaged = original[0].clone(); damaged[12] ^= 1;
            std::fs::write(&parts[0], damaged).unwrap();
            assert!(ZipInputReader::open_seven_zip(&parts[2]).err().unwrap().to_string().contains("[archive-inspection:damaged]"));
            std::fs::write(&parts[0], &original[0]).unwrap();
            let duplicate = temp.path().join(format!("{stem}.haozip001.7z"));
            std::fs::copy(&parts[0], &duplicate).unwrap();
            assert!(SplitArchiveDetector::detect_split_archive(&parts[1]).unwrap_err().to_string().contains("重复"));
        }
    }
    #[test]
    fn generic_four_digit_volumes_preserve_padding_and_cross_tiny_headers() {
        let temp = tempfile::tempdir().unwrap();
        for (index, bytes) in [b"ab", b"cd", b"ef"].iter().enumerate() {
            std::fs::write(temp.path().join(format!("原包.zipx.{:04}", index + 1)), bytes).unwrap();
        }
        let first = temp.path().join("原包.zipx.0001");
        let mut reader = ZipInputReader::open(&first).unwrap();
        let mut bytes = Vec::new(); reader.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"abcdef"); drop(reader);
        std::fs::remove_file(temp.path().join("原包.zipx.0002")).unwrap();
        assert!(ZipInputReader::open(&first).err().unwrap().to_string().contains("原包.zipx.0002"));
    }
    #[test]
    fn seven_zip_volume_reader_seeks_and_reports_missing_middle_volume() {
        let temp = tempfile::tempdir().unwrap();
        for (index, bytes) in [b"abc", b"def", b"ghi"].iter().enumerate() {
            std::fs::write(temp.path().join(format!("sample.7z.{:03}", index + 1)), bytes).unwrap();
        }
        let first = temp.path().join("sample.7z.001");
        let mut reader = ZipInputReader::open_seven_zip(&first).unwrap();
        reader.seek(SeekFrom::Start(2)).unwrap();
        let mut bytes = [0; 5]; reader.read_exact(&mut bytes).unwrap();
        assert_eq!(&bytes, b"cdefg");
        drop(reader);
        std::fs::remove_file(temp.path().join("sample.7z.002")).unwrap();
        let error = ZipInputReader::open_seven_zip(&first).err().unwrap().to_string();
        assert!(error.contains("[archive-inspection:incomplete]"), "{error}");
        assert!(error.contains("7Z"), "{error}");
        assert!(!error.contains("ZIP"), "{error}");
    }

    #[test]
    fn numeric_zip_seeks_across_boundaries_and_rejects_missing_volumes() {
        let temp = tempfile::tempdir().unwrap();
        for (index, bytes) in [b"abc", b"def", b"ghi"].iter().enumerate() {
            std::fs::write(temp.path().join(format!("sample.zip.{:03}", index + 1)), bytes).unwrap();
        }
        let first = temp.path().join("sample.zip.001");
        let mut reader = NumericVolumeReader::open(&first).unwrap();
        reader.seek(SeekFrom::Start(2)).unwrap();
        let mut bytes = [0; 5]; reader.read_exact(&mut bytes).unwrap();
        assert_eq!(&bytes, b"cdefg");
        reader.seek(SeekFrom::End(-3)).unwrap();
        let mut tail = String::new(); reader.read_to_string(&mut tail).unwrap();
        assert_eq!(tail, "ghi");
        reader.seek(SeekFrom::Start(0)).unwrap();
        assert!(reader.seek(SeekFrom::Current(-1)).is_err());
        drop(reader);
        std::fs::remove_file(temp.path().join("sample.zip.002")).unwrap();
        assert!(NumericVolumeReader::open(&first).is_err());
    }
}
