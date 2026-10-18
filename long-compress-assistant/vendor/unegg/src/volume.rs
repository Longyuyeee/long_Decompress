use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::error::{EggError, EggResult};

const SIG_EGG_HEADER: u32 = 0x41474745;
const SIG_SPLIT_INFO: u32 = 0x24F5A262;
const SIG_END_MARKER: u32 = 0x08E28222;

struct Segment {
    file: File,
    phys_start: u64,
    logical_start: u64,
    data_size: u64,
}

/// Reader that chains multiple EGG volumes into one contiguous stream.
/// Continuation volume headers are transparently skipped.
pub struct MultiVolumeReader {
    segments: Vec<Segment>,
    current: usize,
    logical_pos: u64,
    total_size: u64,
}

impl MultiVolumeReader {
    /// Open a multi-volume archive starting from the first volume.
    /// Returns None if the archive is not split.
    pub fn try_open(first_path: &Path) -> EggResult<Option<Self>> {
        let mut file = File::open(first_path).map_err(EggError::CantOpenFile)?;
        let file_size = file.metadata().map_err(EggError::Io)?.len();

        let split = read_split_info(&mut file)?;
        let next_id = match split {
            Some((_, next)) if next != 0 => next,
            _ => return Ok(None),
        };

        file.seek(SeekFrom::Start(0)).map_err(EggError::Io)?;

        let mut segments = vec![Segment {
            file,
            phys_start: 0,
            logical_start: 0,
            data_size: file_size,
        }];

        // `Path::parent()` of a bare filename is `Some("")`, not `None`, so
        // treat an empty parent as the current directory.
        let dir = match first_path.parent() {
            Some(p) if !p.as_os_str().is_empty() => p,
            _ => Path::new("."),
        };

        // Reject cycles (incl. self-loops) and cap the chain length so a crafted
        // volume set can't spin forever, exhausting file descriptors and memory.
        const MAX_VOLUMES: usize = 10_000;
        let mut seen = std::collections::HashSet::new();
        if let Some(first_id) = quick_header_id(first_path) {
            seen.insert(first_id);
        }
        let mut next = next_id;

        while next != 0 {
            if !seen.insert(next) {
                return Err(EggError::CorruptedFile);
            }
            if segments.len() >= MAX_VOLUMES {
                return Err(EggError::CorruptedFile);
            }
            let vol_path = find_volume_by_id(dir, next)?;
            let mut vol_file = File::open(&vol_path).map_err(EggError::CantOpenFile)?;
            let vol_size = vol_file.metadata().map_err(EggError::Io)?.len();

            let data_start = skip_continuation_header(&mut vol_file)?;

            // Read this volume's split info for the next pointer
            vol_file.seek(SeekFrom::Start(0)).map_err(EggError::Io)?;
            let vol_split = read_split_info(&mut vol_file)?;
            next = vol_split.map_or(0, |(_, n)| n);

            let logical_start = segments.last().map_or(0, |s| s.logical_start + s.data_size);
            let data_size = vol_size
                .checked_sub(data_start)
                .ok_or(EggError::CorruptedFile)?;

            segments.push(Segment {
                file: vol_file,
                phys_start: data_start,
                logical_start,
                data_size,
            });
        }

        let total_size = segments.last().map_or(0, |s| s.logical_start + s.data_size);

        Ok(Some(MultiVolumeReader {
            segments,
            current: 0,
            logical_pos: 0,
            total_size,
        }))
    }

    pub fn volume_count(&self) -> usize {
        self.segments.len()
    }
}

impl Read for MultiVolumeReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            if self.current >= self.segments.len() {
                return Ok(0);
            }

            let seg = &self.segments[self.current];
            let seg_end = seg.logical_start + seg.data_size;

            if self.logical_pos >= seg_end {
                self.current += 1;
                if self.current >= self.segments.len() {
                    return Ok(0);
                }
                let next_seg = &mut self.segments[self.current];
                next_seg.file.seek(SeekFrom::Start(next_seg.phys_start))?;
                continue;
            }

            let seg = &mut self.segments[self.current];
            // Clamp in u64 before narrowing so a >4 GiB span cannot truncate to 0
            // (a premature EOF) on a 32-bit target.
            let remaining = seg.logical_start + seg.data_size - self.logical_pos;
            let to_read = remaining.min(buf.len() as u64) as usize;
            let n = seg.file.read(&mut buf[..to_read])?;
            self.logical_pos += n as u64;
            return Ok(n);
        }
    }
}

impl Seek for MultiVolumeReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let target = match pos {
            SeekFrom::Start(p) => p as i64,
            SeekFrom::Current(delta) => self.logical_pos as i64 + delta,
            SeekFrom::End(delta) => self.total_size as i64 + delta,
        };

        if target < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek before start",
            ));
        }
        let target = target as u64;
        self.logical_pos = target;

        let idx = self
            .segments
            .iter()
            .position(|s| target < s.logical_start + s.data_size)
            .unwrap_or(self.segments.len().saturating_sub(1));

        self.current = idx;
        let seg = &mut self.segments[idx];
        let offset_in_seg = target.saturating_sub(seg.logical_start);
        seg.file
            .seek(SeekFrom::Start(seg.phys_start + offset_in_seg))?;

        Ok(self.logical_pos)
    }
}

/// Read header_id from an EGG file (first 10 bytes).
fn quick_header_id(path: &Path) -> Option<u32> {
    let mut file = File::open(path).ok()?;
    let mut buf = [0u8; 10];
    file.read_exact(&mut buf).ok()?;
    let sig = u32::from_le_bytes(buf[0..4].try_into().ok()?);
    if sig != SIG_EGG_HEADER {
        return None;
    }
    Some(u32::from_le_bytes(buf[6..10].try_into().ok()?))
}

/// Scan directory for an EGG file with the given header_id.
fn find_volume_by_id(dir: &Path, target_id: u32) -> EggResult<PathBuf> {
    for entry in fs::read_dir(dir).map_err(EggError::Io)? {
        let entry = entry.map_err(EggError::Io)?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(id) = quick_header_id(&path)
            && id == target_id
        {
            return Ok(path);
        }
    }

    Err(EggError::CantOpenFile(io::Error::new(
        io::ErrorKind::NotFound,
        format!("split volume with ID {target_id:#010x} not found"),
    )))
}

/// Read split info (prev_id, next_id) from an EGG file's prefix section.
fn read_split_info(file: &mut File) -> EggResult<Option<(u32, u32)>> {
    file.seek(SeekFrom::Start(0)).map_err(EggError::Io)?;
    let mut sig = [0u8; 4];
    if file.read_exact(&mut sig).is_err() || u32::from_le_bytes(sig) != SIG_EGG_HEADER {
        return Ok(None);
    }

    file.seek(SeekFrom::Start(14)).map_err(EggError::Io)?;

    loop {
        let sig = read_u32_raw(file)?;

        if sig == SIG_END_MARKER {
            return Ok(None);
        }

        let (_, size) = read_extra_field_raw(file)?;

        if sig == SIG_SPLIT_INFO {
            let prev_id = read_u32_raw(file)?;
            let next_id = read_u32_raw(file)?;
            return Ok(Some((prev_id, next_id)));
        }

        file.seek(SeekFrom::Current(size as i64))
            .map_err(EggError::Io)?;
    }
}

/// Skip past a continuation volume's EGG Header + prefix section.
/// Returns the byte offset where the data stream continues.
fn skip_continuation_header(file: &mut File) -> EggResult<u64> {
    file.seek(SeekFrom::Start(0)).map_err(EggError::Io)?;

    let sig = read_u32_raw(file)?;
    if sig != SIG_EGG_HEADER {
        return Err(EggError::NotEggFile);
    }
    // Skip version(2) + header_id(4) + reserved(4) = 10 bytes
    file.seek(SeekFrom::Current(10)).map_err(EggError::Io)?;

    // Parse prefix section until End Marker
    loop {
        let sig = read_u32_raw(file)?;
        if sig == SIG_END_MARKER {
            break;
        }
        let (_, size) = read_extra_field_raw(file)?;
        file.seek(SeekFrom::Current(size as i64))
            .map_err(EggError::Io)?;
    }

    file.stream_position().map_err(EggError::Io)
}

fn read_u32_raw(r: &mut impl Read) -> EggResult<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf).map_err(EggError::Io)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_extra_field_raw(r: &mut (impl Read + Seek)) -> EggResult<(u8, u32)> {
    let mut flags = [0u8; 1];
    r.read_exact(&mut flags).map_err(EggError::Io)?;
    let size = if flags[0] & 0x01 != 0 {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf).map_err(EggError::Io)?;
        u32::from_le_bytes(buf)
    } else {
        let mut buf = [0u8; 2];
        r.read_exact(&mut buf).map_err(EggError::Io)?;
        u16::from_le_bytes(buf) as u32
    };
    Ok((flags[0], size))
}
