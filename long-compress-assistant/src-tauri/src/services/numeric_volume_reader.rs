//! Seekable numeric ZIP volumes, with only one open volume handle at a time.
use std::{fs::File, io::{self, Read, Seek, SeekFrom}, path::{Path, PathBuf}};

pub(crate) struct NumericVolumeReader {
    parts: Vec<(PathBuf, u64, u64)>,
    length: u64,
    position: u64,
    current: Option<(usize, File)>,
}

impl NumericVolumeReader {
    pub(crate) fn open(first: &Path) -> anyhow::Result<Self> {
        anyhow::ensure!(first.to_string_lossy().to_ascii_lowercase().ends_with(".zip.001"), "Expected first numeric ZIP volume (.zip.001)");
        let info = super::split_archive_detector::SplitArchiveDetector::detect_split_archive(first)?
            .ok_or_else(|| anyhow::anyhow!("Numeric ZIP volumes not found"))?;
        anyhow::ensure!(info.missing_parts.is_empty(), "Missing ZIP volumes: {:?}", info.missing_parts);
        let mut length = 0u64;
        let mut parts = Vec::new();
        for path in info.parts {
            let metadata = path.metadata()?;
            anyhow::ensure!(metadata.is_file() && metadata.len() > 0, "Invalid ZIP volume: {}", path.display());
            let start = length;
            length = length.checked_add(metadata.len()).ok_or_else(|| anyhow::anyhow!("ZIP size overflow"))?;
            parts.push((path, start, length));
        }
        anyhow::ensure!(!parts.is_empty(), "No ZIP volumes found");
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
        if read == 0 { return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "ZIP volume changed or truncated")); }
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
