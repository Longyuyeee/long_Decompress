//! ALZ archive parsing: signatures, headers, and the file entry list.

use std::io::{Read, Seek, SeekFrom};

use crate::crypto::ENCR_HEADER_LEN;
use crate::encoding::cp949_to_utf8;
use crate::error::{AlzError, AlzResult};
use crate::multivolume::MultiVolumeReader;

// ALZ signatures (little-endian u32)
const SIG_ALZ_FILE_HEADER: u32 = 0x015a4c41; // "ALZ\x01"
const SIG_LOCAL_FILE_HEADER: u32 = 0x015a4c42; // "BLZ\x01"
const SIG_CENTRAL_DIRECTORY: u32 = 0x015a4c43; // "CLZ\x01"
const SIG_END_OF_CENTRAL_DIR: u32 = 0x025a4c43; // "CLZ\x02"
const SIG_COMMENT: u32 = 0x015a4c45; // "ELZ\x01"
const SIG_SPLIT_MARKER: u32 = 0x035a4c43; // "CLZ\x03"

// File descriptor flags
const DESC_ENCRYPTED: u8 = 0x01;
const DESC_DATA_DESCR: u8 = 0x08;

// Entry attribute flags
pub const ATTR_READONLY: u8 = 0x01;
pub const ATTR_HIDDEN: u8 = 0x02;
pub const ATTR_SYSTEM: u8 = 0x04;
pub const ATTR_DIRECTORY: u8 = 0x10;
pub const ATTR_ARCHIVE: u8 = 0x20;

/// Compression method of an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionMethod {
    /// No compression (method byte 0).
    Store,
    /// ALZ-modified bzip2 (method byte 1).
    Bzip2,
    /// Raw DEFLATE (method byte 2).
    Deflate,
    /// Unrecognized method byte.
    Unknown(u8),
}

impl CompressionMethod {
    fn from_byte(b: u8) -> Self {
        match b {
            0 => Self::Store,
            1 => Self::Bzip2,
            2 => Self::Deflate,
            n => Self::Unknown(n),
        }
    }
}

impl std::fmt::Display for CompressionMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store => write!(f, "Store"),
            Self::Bzip2 => write!(f, "BZip2"),
            Self::Deflate => write!(f, "Deflate"),
            Self::Unknown(n) => write!(f, "Unknown({n})"),
        }
    }
}

/// One file entry from the archive's local file headers.
#[derive(Debug, Clone)]
pub struct AlzFileEntry {
    /// Long adapter: preserve the original bytes for an explicit source codepage.
    pub file_name_raw: Vec<u8>,
    /// Entry path, decoded from CP949 to UTF-8.
    pub file_name: String,
    /// `ATTR_*` attribute bits.
    pub file_attribute: u8,
    /// DOS packed modification date/time.
    pub file_time_date: u32,
    /// Descriptor bits (encryption, data descriptor).
    pub file_descriptor: u8,
    pub compression_method: CompressionMethod,
    /// CRC-32 of the uncompressed data.
    pub file_crc: u32,
    /// Size of the stored (compressed) data in bytes.
    pub compressed_size: u64,
    /// Declared size of the uncompressed data in bytes.
    pub uncompressed_size: u64,
    /// Offset of the entry data in the (virtual, joined) archive stream.
    pub data_pos: u64,
    /// Encryption check header, present on encrypted entries.
    pub enc_check: Option<[u8; ENCR_HEADER_LEN]>,
}

impl AlzFileEntry {
    pub fn is_encrypted(&self) -> bool {
        self.file_descriptor & DESC_ENCRYPTED != 0
    }

    pub fn is_directory(&self) -> bool {
        self.file_attribute & ATTR_DIRECTORY != 0
    }

    pub fn has_data_descriptor(&self) -> bool {
        self.file_descriptor & DESC_DATA_DESCR != 0
    }
}

/// A parsed ALZ archive: the volume reader plus the entry list.
pub struct AlzArchive {
    /// Underlying reader over all volumes.
    pub reader: MultiVolumeReader,
    /// File entries in archive order.
    pub entries: Vec<AlzFileEntry>,
    /// Whether any entry is encrypted.
    pub is_encrypted: bool,
    /// Whether any entry uses a data descriptor.
    pub is_data_descr: bool,
    /// Whether the index ended on a truncated header (e.g. a missing split
    /// volume) rather than a proper end-of-central-directory record.
    pub truncated: bool,
}

impl AlzArchive {
    /// Long adapter: parse an explicitly opened, prevalidated volume view.
    /// Keeps I/O errors and caller volume identity instead of probing names.
    pub fn from_reader(reader: MultiVolumeReader) -> AlzResult<Self> {
        let mut archive = Self { reader, entries: Vec::new(), is_encrypted: false,
            is_data_descr: false, truncated: false };
        archive.parse()?;
        Ok(archive)
    }

    /// Open and parse an archive from a path, discovering split volumes
    /// automatically.
    pub fn open(path: &str) -> AlzResult<Self> {
        let reader = MultiVolumeReader::open(path)?;
        let mut archive = AlzArchive {
            reader,
            entries: Vec::new(),
            is_encrypted: false,
            is_data_descr: false,
            truncated: false,
        };
        archive.parse()?;
        Ok(archive)
    }

    /// Parse an archive already loaded into memory.
    pub fn from_bytes(data: Vec<u8>) -> AlzResult<Self> {
        let reader = MultiVolumeReader::from_bytes(data);
        let mut archive = AlzArchive {
            reader,
            entries: Vec::new(),
            is_encrypted: false,
            is_data_descr: false,
            truncated: false,
        };
        archive.parse()?;
        Ok(archive)
    }

    fn parse(&mut self) -> AlzResult<()> {
        let mut seen_alz_header = false;
        let mut seen_central_dir = false;

        // Parse endInfos from the 16-byte file tail.
        let tail = *self.reader.tail();
        let comment_section_size = u32::from_le_bytes([tail[4], tail[5], tail[6], tail[7]]) as u64;

        // Bound the entry count so a stream of tiny local-file headers can't
        // inflate memory (each ~14-byte on-disk header becomes a much larger
        // heap struct). See max_entries_for.
        let max_entries = max_entries_for(self.reader.total_size());

        loop {
            let sig = match self.read_u32_le() {
                Ok(sig) => sig,
                // A clean (or partial) end-of-data terminates parsing; any
                // other I/O error is real and must propagate.
                Err(AlzError::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    // Reaching EOF here is a clean end once the central directory
                    // has been read; before that it means the index was cut off.
                    if !seen_central_dir {
                        self.truncated = true;
                    }
                    break;
                }
                Err(e) => return Err(e),
            };
            match sig {
                SIG_ALZ_FILE_HEADER => {
                    self.read_alz_header()?;
                    seen_alz_header = true;
                }
                SIG_LOCAL_FILE_HEADER => {
                    if self.entries.len() as u64 >= max_entries {
                        return Err(AlzError::TooManyEntries);
                    }
                    // A header cut short by truncation (missing split volume)
                    // ends the index cleanly, keeping the entries read so far;
                    // extraction then recovers those complete files.
                    match self.read_local_file_header() {
                        Ok(()) => {}
                        Err(AlzError::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                            self.truncated = true;
                            break;
                        }
                        Err(e) => return Err(e),
                    }
                }
                SIG_CENTRAL_DIRECTORY => {
                    self.read_central_directory()?;
                    seen_central_dir = true;
                }
                SIG_END_OF_CENTRAL_DIR => {
                    break;
                }
                SIG_COMMENT => {
                    self.skip_comment_section(comment_section_size)?;
                }
                SIG_SPLIT_MARKER => {}
                _ => {
                    if seen_central_dir {
                        break;
                    }
                    if seen_alz_header {
                        return Err(AlzError::CorruptedFile);
                    } else {
                        return Err(AlzError::NotAlzFile);
                    }
                }
            }
        }

        Ok(())
    }

    fn read_alz_header(&mut self) -> AlzResult<()> {
        let mut buf = [0u8; 4];
        self.reader.read_exact(&mut buf)?;
        Ok(())
    }

    fn read_local_file_header(&mut self) -> AlzResult<()> {
        let mut head = [0u8; 9];
        self.reader.read_exact(&mut head)?;

        let file_name_length = u16::from_le_bytes([head[0], head[1]]) as usize;
        let file_attribute = head[2];
        let file_time_date = u32::from_le_bytes([head[3], head[4], head[5], head[6]]);
        let file_descriptor = head[7];
        let _unknown2 = head[8];

        if file_descriptor & DESC_ENCRYPTED != 0 {
            self.is_encrypted = true;
        }
        if file_descriptor & DESC_DATA_DESCR != 0 {
            self.is_data_descr = true;
        }

        // Size field width from descriptor bits 4-7
        let byte_len = match file_descriptor & 0xF0 {
            0x00 => 0,
            0x10 => 1,
            0x20 => 2,
            0x40 => 4,
            0x80 => 8,
            _ => return Err(AlzError::InvalidSizeFieldWidth(file_descriptor & 0xF0)),
        };

        let mut compression_method = CompressionMethod::Store;
        let mut file_crc: u32 = 0;
        let mut compressed_size: u64 = 0;
        let mut uncompressed_size: u64 = 0;

        if byte_len > 0 {
            let mut cm = [0u8; 1];
            self.reader.read_exact(&mut cm)?;
            compression_method = CompressionMethod::from_byte(cm[0]);

            let mut unk = [0u8; 1];
            self.reader.read_exact(&mut unk)?;

            let mut crc_buf = [0u8; 4];
            self.reader.read_exact(&mut crc_buf)?;
            file_crc = u32::from_le_bytes(crc_buf);

            compressed_size = self.read_var_int(byte_len)?;
            uncompressed_size = self.read_var_int(byte_len)?;
        }

        if file_name_length == 0 || file_name_length > 4096 {
            return Err(AlzError::InvalidFilenameLength);
        }
        let mut name_buf = vec![0u8; file_name_length];
        self.reader.read_exact(&mut name_buf)?;
        let file_name = cp949_to_utf8(&name_buf);

        let enc_check = if file_descriptor & DESC_ENCRYPTED != 0 {
            let mut buf = [0u8; ENCR_HEADER_LEN];
            self.reader.read_exact(&mut buf)?;
            Some(buf)
        } else {
            None
        };

        let data_pos = self.reader.stream_position()?;
        let skip: i64 = compressed_size
            .try_into()
            .map_err(|_| AlzError::CorruptedFile)?;
        self.reader.seek(SeekFrom::Current(skip))?;

        self.entries.push(AlzFileEntry {
            file_name_raw: name_buf,
            file_name,
            file_attribute,
            file_time_date,
            file_descriptor,
            compression_method,
            file_crc,
            compressed_size,
            uncompressed_size,
            data_pos,
            enc_check,
        });

        Ok(())
    }

    fn read_central_directory(&mut self) -> AlzResult<()> {
        let mut buf = [0u8; 12];
        self.reader.read_exact(&mut buf)?;
        Ok(())
    }

    fn skip_comment_section(&mut self, total_size: u64) -> AlzResult<()> {
        // total_size includes the 4-byte signature we already read.
        if total_size > 4 {
            let skip: i64 = (total_size - 4)
                .try_into()
                .map_err(|_| AlzError::CorruptedFile)?;
            self.reader.seek(SeekFrom::Current(skip))?;
        }
        Ok(())
    }

    fn read_u32_le(&mut self) -> AlzResult<u32> {
        let mut buf = [0u8; 4];
        self.reader.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    /// Read a variable-width little-endian integer (1, 2, 4, or 8 bytes).
    fn read_var_int(&mut self, byte_len: usize) -> AlzResult<u64> {
        let mut buf = [0u8; 8];
        self.reader.read_exact(&mut buf[..byte_len])?;
        Ok(u64::from_le_bytes(buf))
    }
}

/// Maximum number of file entries to accept, as a function of the archive's
/// total data size. Two bounds combine: a physical one (each entry needs at
/// least 14 bytes on disk) and an absolute sanity ceiling that caps memory on
/// very large inputs, mirroring the MAX_VOLUMES guard.
pub fn max_entries_for(total_size: u64) -> u64 {
    const MIN_ENTRY_ON_DISK: u64 = 14;
    const MAX_ENTRIES: u64 = 1_000_000;
    (total_size / MIN_ENTRY_ON_DISK + 16).min(MAX_ENTRIES)
}

/// Sum the uncompressed/compressed sizes and file count of a set of entries
/// for display. Sizes are attacker-controlled, so accumulate with saturating
/// arithmetic: overflow-checks would otherwise turn a forged size into a panic.
pub fn archive_totals(entries: &[AlzFileEntry]) -> (u64, u64, u32) {
    let mut total_uncompressed: u64 = 0;
    let mut total_compressed: u64 = 0;
    let mut count: u32 = 0;
    for entry in entries {
        total_uncompressed = total_uncompressed.saturating_add(entry.uncompressed_size);
        total_compressed = total_compressed.saturating_add(entry.compressed_size);
        count = count.saturating_add(1);
    }
    (total_uncompressed, total_compressed, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(uncompressed: u64, compressed: u64) -> AlzFileEntry {
        AlzFileEntry {
            file_name_raw: Vec::new(),
            file_name: String::new(),
            file_attribute: 0,
            file_time_date: 0,
            file_descriptor: 0,
            compression_method: CompressionMethod::Store,
            file_crc: 0,
            compressed_size: compressed,
            uncompressed_size: uncompressed,
            data_pos: 0,
            enc_check: None,
        }
    }

    #[test]
    fn totals_saturate_instead_of_overflowing() {
        // Two u64::MAX entries must not panic under overflow-checks.
        let entries = [entry(u64::MAX, u64::MAX), entry(u64::MAX, u64::MAX)];
        let (u, c, n) = archive_totals(&entries);
        assert_eq!(u, u64::MAX);
        assert_eq!(c, u64::MAX);
        assert_eq!(n, 2);
    }

    #[test]
    fn totals_normal() {
        let entries = [entry(100, 40), entry(200, 60)];
        assert_eq!(archive_totals(&entries), (300, 100, 2));
    }

    #[test]
    fn entry_cap_bounds() {
        // Tiny archive: the physical bound dominates.
        assert_eq!(max_entries_for(0), 16);
        assert_eq!(max_entries_for(1400), 1400 / 14 + 16);
        // Huge archive: the absolute ceiling dominates.
        assert_eq!(max_entries_for(u64::MAX), 1_000_000);
        assert_eq!(max_entries_for(1_000_000_000), 1_000_000);
    }
}
