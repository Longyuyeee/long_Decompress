//! Virtual reader spanning multi-volume archives (.alz, .a00, .a01, ...).

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use crate::error::{AlzError, AlzResult};

trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

const MAX_VOLUMES: usize = 1000;
const VOLUME_HEADER_SIZE: u64 = 8;
const VOLUME_TRAILER_SIZE: u64 = 16;

struct Volume {
    file: Box<dyn ReadSeek>,
    file_size: u64,
    header_size: u64,
    tail_size: u64,
}

impl Volume {
    fn data_size(&self) -> u64 {
        // Saturating: a corrupt/truncated volume smaller than its framing
        // must not underflow (would wrap to ~u64::MAX in release builds).
        self.file_size
            .saturating_sub(self.header_size)
            .saturating_sub(self.tail_size)
    }
}

/// Virtual reader over multi-volume ALZ archives (.alz, .a00, .a01, ...).
/// Transparently handles seeking and reading across volume boundaries.
pub struct MultiVolumeReader {
    volumes: Vec<Volume>,
    cur_volume: usize,
    virtual_pos: u64,
    tail: [u8; 16],
}

impl MultiVolumeReader {
    /// Long adapter: read exactly the prevalidated native members. Never hide
    /// an access/device error as the end of the archive or probe unrelated files.
    pub fn from_paths(paths: &[std::path::PathBuf]) -> AlzResult<Self> {
        if paths.is_empty() || paths.len() > MAX_VOLUMES { return Err(AlzError::CorruptedFile); }
        let mut volumes = Vec::new();
        for (index, path) in paths.iter().enumerate() {
            let file = File::open(path)?;
            let file_size = file.metadata()?.len();
            let header_size = if index == 0 { 0 } else { VOLUME_HEADER_SIZE };
            let tail_size = if index + 1 == paths.len() { 0 } else { VOLUME_TRAILER_SIZE };
            if file_size < header_size + tail_size { return Err(AlzError::CorruptedFile); }
            volumes.push(Volume { file: Box::new(file), file_size, header_size, tail_size });
        }
        let last = volumes.last_mut().unwrap();
        let mut tail = [0; 16];
        last.file.seek(SeekFrom::End(-16))?;
        last.file.read_exact(&mut tail)?;
        let mut reader = Self { volumes, cur_volume: 0, virtual_pos: 0, tail };
        reader.seek_to_virtual(0)?;
        Ok(reader)
    }

    /// Open a multi-volume archive starting from the given .alz path.
    /// Discovers .a00, .a01, ... .a99, .b00, ... automatically.
    pub fn open<P: AsRef<Path>>(path: P) -> AlzResult<Self> {
        let path = path.as_ref();
        let path_str = path.to_string_lossy().to_string();

        // Multi-volume probing rewrites the trailing 3-char extension
        // (.alz -> .a00, .a01, ...). Only treat the path as a multi-volume
        // base when that 3-byte cut lands on a char boundary; otherwise
        // (short names, or a cut inside a multi-byte character) open it as a
        // single volume. Byte-slicing a str off a char boundary would panic.
        let cut = path_str.len().wrapping_sub(3);
        let multivol_prefix = if path_str.len() >= 4 && path_str.is_char_boundary(cut) {
            Some(&path_str[..cut])
        } else {
            None
        };
        let mut volumes = Vec::new();

        for i in 0..MAX_VOLUMES {
            let vol_path = if i == 0 {
                path_str.clone()
            } else {
                match multivol_prefix {
                    Some(prefix) => {
                        let letter = (b'a' + ((i - 1) / 100) as u8) as char;
                        let num = (i - 1) % 100;
                        format!("{prefix}{letter}{num:02}")
                    }
                    // Not a multi-volume base name: single volume only.
                    None => break,
                }
            };

            let file = match File::open(&vol_path) {
                Ok(f) => f,
                Err(_) => break,
            };

            let file_size = file.metadata()?.len();
            let header_size = if i == 0 { 0 } else { VOLUME_HEADER_SIZE };
            let tail_size = VOLUME_TRAILER_SIZE; // corrected for last volume below

            volumes.push(Volume {
                file: Box::new(file),
                file_size,
                header_size,
                tail_size,
            });
        }

        if volumes.is_empty() {
            return Err(AlzError::CantOpenFile(io::Error::new(
                io::ErrorKind::NotFound,
                format!("can't open: {path_str}"),
            )));
        }

        // Last volume has no tail.
        if let Some(last) = volumes.last_mut() {
            last.tail_size = 0;
        }

        // Fail fast on a corrupt set: every volume must be large enough to
        // hold its own framing (header + trailer).
        for vol in &volumes {
            if vol.file_size < vol.header_size + vol.tail_size {
                return Err(AlzError::CorruptedFile);
            }
        }

        // The end-of-archive record sits at the end of the LAST volume; every
        // earlier volume ends in its own split trailer instead.
        let mut tail = [0u8; 16];
        let last = volumes.last_mut().expect("volumes is non-empty");
        if last.file_size >= 16 {
            last.file.seek(SeekFrom::Start(last.file_size - 16))?;
            last.file.read_exact(&mut tail)?;
        }

        let mut reader = MultiVolumeReader {
            volumes,
            cur_volume: 0,
            virtual_pos: 0,
            tail,
        };
        // Position at the data start of volume 0.
        reader.seek_to_virtual(0)?;
        Ok(reader)
    }

    /// Create a single-volume reader from in-memory data (e.g. stdin).
    pub fn from_bytes(data: Vec<u8>) -> Self {
        let len = data.len() as u64;
        let mut tail = [0u8; 16];
        if data.len() >= 16 {
            tail.copy_from_slice(&data[data.len() - 16..]);
        }
        MultiVolumeReader {
            volumes: vec![Volume {
                file: Box::new(io::Cursor::new(data)),
                file_size: len,
                header_size: 0,
                tail_size: 0,
            }],
            cur_volume: 0,
            virtual_pos: 0,
            tail,
        }
    }

    /// The 16-byte end-of-archive record from the last volume.
    pub fn tail(&self) -> &[u8; 16] {
        &self.tail
    }

    /// Total virtual data size across all volumes.
    pub fn total_size(&self) -> u64 {
        self.volumes.iter().map(|v| v.data_size()).sum()
    }

    fn seek_to_virtual(&mut self, offset: u64) -> AlzResult<()> {
        self.virtual_pos = offset;
        let mut remain = offset;

        for (i, vol) in self.volumes.iter_mut().enumerate() {
            let data_size = vol.data_size();
            if remain <= data_size {
                let phys_pos = remain + vol.header_size;
                vol.file.seek(SeekFrom::Start(phys_pos))?;
                self.cur_volume = i;
                return Ok(());
            }
            remain -= data_size;
        }

        // Past end of available volumes -- park at EOF so reads return 0.
        let last = self.volumes.len() - 1;
        let vol = &mut self.volumes[last];
        let end = vol.file_size.saturating_sub(vol.tail_size);
        vol.file.seek(SeekFrom::Start(end))?;
        self.cur_volume = last;
        Ok(())
    }
}

impl Read for MultiVolumeReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() || self.cur_volume >= self.volumes.len() {
            return Ok(0);
        }

        let mut total_read = 0;

        while total_read < buf.len() && self.cur_volume < self.volumes.len() {
            let vol = &mut self.volumes[self.cur_volume];
            let phys_pos = vol.file.stream_position()?;
            let data_end = vol.file_size.saturating_sub(vol.tail_size);
            // Clamp in u64 before narrowing so a >4 GiB span cannot truncate to 0
            // (which would skip a volume) on a 32-bit target.
            let want = (buf.len() - total_read) as u64;
            let to_read = data_end.saturating_sub(phys_pos).min(want) as usize;

            if to_read == 0 {
                // Move to next volume.
                self.cur_volume += 1;
                if self.cur_volume >= self.volumes.len() {
                    break;
                }
                let next_vol = &mut self.volumes[self.cur_volume];
                next_vol.file.seek(SeekFrom::Start(next_vol.header_size))?;
                continue;
            }

            let n = vol.file.read(&mut buf[total_read..total_read + to_read])?;
            if n == 0 {
                break;
            }
            total_read += n;
            self.virtual_pos += n as u64;
        }

        Ok(total_read)
    }
}

impl Seek for MultiVolumeReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let out_of_range =
            || io::Error::new(io::ErrorKind::InvalidInput, "seek position out of range");
        let new_pos = match pos {
            SeekFrom::Start(p) => p,
            SeekFrom::Current(delta) => self
                .virtual_pos
                .checked_add_signed(delta)
                .ok_or_else(out_of_range)?,
            SeekFrom::End(delta) => self
                .total_size()
                .checked_add_signed(delta)
                .ok_or_else(out_of_range)?,
        };

        self.seek_to_virtual(new_pos)
            .map_err(|e| io::Error::other(e.to_string()))?;
        Ok(self.virtual_pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// A non-last volume smaller than its framing (header + 16-byte trailer)
    /// must error, not panic/underflow. Regression for the size-math bug.
    #[test]
    fn truncated_volume_errors_instead_of_panicking() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("unalz_mv_test_{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        let alz = dir.join("x.alz");
        std::fs::write(&alz, b"ALZ\x01").unwrap(); // 4 bytes < framing
        std::fs::write(dir.join("x.a00"), [0u8; 4]).unwrap();

        let result = MultiVolumeReader::open(&alz);
        assert!(matches!(result, Err(AlzError::CorruptedFile)));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Opening an archive whose name's last 3 bytes split a multi-byte UTF-8
    /// character must not panic on the volume-prefix slice.
    #[test]
    fn non_char_boundary_name_does_not_panic() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("unalz_mv_boundary_{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        // "рх" is 4 bytes (D1 80 D1 85); len-3 = 1 lands inside 'р'.
        let path = dir.join("\u{0440}\u{0445}");
        std::fs::write(&path, [0u8; 32]).unwrap();

        // Must return a Result (single-volume open), never panic.
        let _ = MultiVolumeReader::open(&path);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
