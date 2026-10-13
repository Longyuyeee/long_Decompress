//! Seekable numeric ZIP/7Z volumes, with only one open volume handle at a time.
use std::{fs::File, io::{self, Read, Seek, SeekFrom}, path::{Path, PathBuf}};

/// Archive consumers must read the complete numeric volume set, not just its first file.
pub(crate) enum ZipInputReader {
    File(File),
    Volumes(NumericVolumeReader),
    StandardZip(super::standard_zip_reader::StandardZipReader),
}

impl ZipInputReader {
    pub(crate) fn open_seven_zip(path: &Path) -> anyhow::Result<Self> {
        if Self::is_numeric_first(path) {
            Ok(Self::Volumes(NumericVolumeReader::open(path).map_err(|error|
                anyhow::anyhow!("[archive-inspection:incomplete] 无法读取完整 7Z 卷组：{error}"))?))
        } else {
            Self::open(path)
        }
    }

    pub(crate) fn is_numeric_first(path: &Path) -> bool {
        path.extension().and_then(|value| value.to_str()).is_some_and(|value|
            value.len() >= 3 && value.bytes().all(|byte| byte.is_ascii_digit()) && value.parse::<usize>().ok() == Some(1))
    }

    pub(crate) fn open(path: &Path) -> anyhow::Result<Self> {
        if Self::is_numeric_first(path) {
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
        match self { Self::File(reader) => reader.read(buffer), Self::Volumes(reader) => reader.read(buffer), Self::StandardZip(reader) => reader.read(buffer) }
    }
}

impl Seek for ZipInputReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        match self { Self::File(reader) => reader.seek(position), Self::Volumes(reader) => reader.seek(position), Self::StandardZip(reader) => reader.seek(position) }
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
