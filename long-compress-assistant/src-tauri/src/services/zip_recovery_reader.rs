//! A read-only ZIP view rebuilt from declared local-record boundaries.
//! Payloads stay in the source; only a synthetic directory is held in memory.
use super::{extraction_transaction, numeric_volume_reader::ZipInputReader};
use super::zip_recovery_outcome::{RecoveryEntryResult, RecoveryStatus};
use anyhow::{ensure, Context, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{io::{self, Read, Seek, SeekFrom}, path::Path};

pub(crate) struct RecoveryZipInput {
    source: ZipInputReader,
    prefix_len: u64,
    directory: Vec<u8>,
    position: u64,
}

#[cfg(test)]
mod streamed_tests {
    use super::*;
    use std::io::Write;

    fn streamed(zip64: bool, signed: bool) -> (Vec<u8>, Vec<u8>) {
        let content = b"streamed payload with PK\x07\x08 inside the content".repeat(300);
        let mut encoder = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&content).unwrap();
        let compressed = encoder.finish().unwrap();
        let mut header = [0u8; 30];
        header[..4].copy_from_slice(b"PK\x03\x04");
        put_word(&mut header, 4, if zip64 { 45 } else { 20 });
        put_word(&mut header, 6, 8);
        put_word(&mut header, 8, 8);
        put_word(&mut header, 26, 8);
        if zip64 { put_dword(&mut header, 18, u32::MAX); put_dword(&mut header, 22, u32::MAX); put_word(&mut header, 28, 20); }
        let mut bytes = header.to_vec();
        bytes.extend_from_slice(b"file.txt");
        if zip64 { bytes.extend_from_slice(&1u16.to_le_bytes()); bytes.extend_from_slice(&16u16.to_le_bytes()); bytes.extend_from_slice(&[0; 16]); }
        bytes.extend_from_slice(&compressed);
        if signed { bytes.extend_from_slice(b"PK\x07\x08"); }
        bytes.extend_from_slice(&crc32fast::hash(&content).to_le_bytes());
        if zip64 { bytes.extend_from_slice(&(compressed.len() as u64).to_le_bytes()); bytes.extend_from_slice(&(content.len() as u64).to_le_bytes()); }
        else { bytes.extend_from_slice(&(compressed.len() as u32).to_le_bytes()); bytes.extend_from_slice(&(content.len() as u32).to_le_bytes()); }
        (bytes, content)
    }

    #[test]
    fn streamed_deflate_descriptors_recover_without_searching_for_payload_signatures() {
        let dir = tempfile::tempdir().unwrap();
        for zip64 in [false, true] { for signed in [false, true] {
            let (bytes, content) = streamed(zip64, signed);
            let path = dir.path().join("stream.zip");
            std::fs::write(&path, &bytes).unwrap();
            let mut reader = RecoveryReader::open(&path, &AtomicBool::new(false)).unwrap();
            let mut restored = Vec::new(); reader.entry(0, None).unwrap().read_to_end(&mut restored).unwrap();
            assert_eq!(restored, content);
            assert!(reader.directory_rebuilt);
            let mut truncated = bytes.clone(); truncated.pop(); std::fs::write(&path, truncated).unwrap();
            assert!(RecoveryReader::open(&path, &AtomicBool::new(false)).is_err());
            let mut wrong_crc = bytes; let at = wrong_crc.len() - if zip64 { 20 } else { 12 }; wrong_crc[at] ^= 1;
            std::fs::write(&path, wrong_crc).unwrap();
            assert!(RecoveryReader::open(&path, &AtomicBool::new(false)).is_err());
        } }
    }

    #[test]
    fn bziprescue1733_decoder_obeys_the_output_limit_and_cancellation() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/zip-bzip2-rescue-1733");
        let records: serde_json::Value = serde_json::from_slice(&std::fs::read(base.join("producer.json")).unwrap()).unwrap();
        let start = records["originals"][0]["entries"][1]["data"].as_u64().unwrap();
        let mut reader = ZipInputReader::open(&base.join("stream32.zip")).unwrap();
        assert!(bzip2_boundary(&mut reader, start, &AtomicBool::new(false), 1024).unwrap_err().to_string().contains("读取上限"));
        assert!(bzip2_boundary(&mut reader, start, &AtomicBool::new(true), u64::MAX).unwrap_err().to_string().contains("cancelled"));
    }

    #[test]
    fn lzmarescue1734_decoder_obeys_output_limit_cancellation_and_eos_declaration() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/zip-lzma-rescue-1734");
        let records: serde_json::Value = serde_json::from_slice(&std::fs::read(base.join("producer.json")).unwrap()).unwrap();
        let start = records["originals"][0]["entries"][1]["data"].as_u64().unwrap();
        let mut reader = ZipInputReader::open(&base.join("stream32.zip")).unwrap();
        assert!(lzma_boundary(&mut reader, start, 2, &AtomicBool::new(false), 1024).unwrap_err().to_string().contains("读取上限"));
        assert!(lzma_boundary(&mut reader, start, 2, &AtomicBool::new(true), u64::MAX).unwrap_err().to_string().contains("cancelled"));
        assert!(lzma_boundary(&mut reader, start, 0, &AtomicBool::new(false), u64::MAX).unwrap_err().to_string().contains("未声明结束标记"));
    }
}
impl RecoveryZipInput {
    fn original(path: &Path) -> Result<Self> {
        Self::from_source(ZipInputReader::open(path)?)
    }
    fn from_source(mut source: ZipInputReader) -> Result<Self> {
        let prefix_len = source.seek(SeekFrom::End(0))?;
        source.rewind()?;
        Ok(Self { source, prefix_len, directory: Vec::new(), position: 0 })
    }
}
impl Read for RecoveryZipInput {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() { return Ok(0); }
        let read = if self.position < self.prefix_len {
            self.source.seek(SeekFrom::Start(self.position))?;
            let limit = (self.prefix_len - self.position).min(buffer.len() as u64) as usize;
            let read = self.source.read(&mut buffer[..limit])?;
            if read == 0 { return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "ZIP source changed during recovery")); }
            read
        } else {
            let at = self.position - self.prefix_len;
            if at >= self.directory.len() as u64 { return Ok(0); }
            let at = at as usize;
            let read = (self.directory.len() - at).min(buffer.len());
            buffer[..read].copy_from_slice(&self.directory[at..at + read]);
            read
        };
        self.position += read as u64;
        Ok(read)
    }
}
impl Seek for RecoveryZipInput {
    fn seek(&mut self, seek: SeekFrom) -> io::Result<u64> {
        let value = match seek {
            SeekFrom::Start(value) => value as i128,
            SeekFrom::End(delta) => self.prefix_len as i128 + self.directory.len() as i128 + delta as i128,
            SeekFrom::Current(delta) => self.position as i128 + delta as i128,
        };
        self.position = u64::try_from(value).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid recovery seek"))?;
        Ok(self.position)
    }
}

pub(crate) struct RecoveryReader {
    archive: zip_aes::ZipArchive<RecoveryZipInput>,
    pub names: Vec<String>,
    pub expanded_bytes: u64,
    pub encrypted: bool,
    pub directory_rebuilt: bool,
    pub notes: Vec<String>,
    pub entry_results: Vec<RecoveryEntryResult>,
}
fn word(bytes: &[u8], at: usize) -> u16 { u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap()) }
fn dword(bytes: &[u8], at: usize) -> u32 { u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) }
fn put_word(bytes: &mut [u8], at: usize, value: u16) { bytes[at..at + 2].copy_from_slice(&value.to_le_bytes()); }
fn put_dword(bytes: &mut [u8], at: usize, value: u32) { bytes[at..at + 4].copy_from_slice(&value.to_le_bytes()); }

fn deflate_boundary(reader: &mut ZipInputReader, start: u64, cancelled: &AtomicBool) -> Result<(u64, u64, u32)> {
    let mut decoder = flate2::Decompress::new(false);
    let mut crc = crc32fast::Hasher::new();
    let mut input = [0u8; 65536];
    let mut output = [0u8; 65536];
    loop {
        ensure!(!cancelled.load(Ordering::SeqCst), "ZIP recovery cancelled");
        reader.seek(SeekFrom::Start(start.checked_add(decoder.total_in()).context("ZIP offset overflow")?))?;
        let length = reader.read(&mut input)?;
        ensure!(length > 0, "Deflate 数据被截断");
        let mut at = 0;
        loop {
            ensure!(!cancelled.load(Ordering::SeqCst), "ZIP recovery cancelled");
            let before_in = decoder.total_in();
            let before_out = decoder.total_out();
            let status = decoder.decompress(&input[at..length], &mut output, flate2::FlushDecompress::None)?;
            let used = (decoder.total_in() - before_in) as usize;
            let produced = (decoder.total_out() - before_out) as usize;
            crc.update(&output[..produced]);
            ensure!(decoder.total_out() <= extraction_transaction::MAX_EXTRACTED_BYTES, "ZIP 条目展开体积超过读取上限");
            at += used;
            if status == flate2::Status::StreamEnd { return Ok((decoder.total_in(), decoder.total_out(), crc.finalize())); }
            ensure!(used > 0 || produced > 0, "Deflate 数据无效或被截断");
            if at == length { break; }
        }
    }
}

fn bzip2_boundary(reader: &mut ZipInputReader, start: u64, cancelled: &AtomicBool, limit: u64) -> Result<(u64, u64, u32)> {
    let mut decoder = bzip2::Decompress::new(false);
    let mut crc = crc32fast::Hasher::new();
    let mut input = [0u8; 65536];
    let mut output = [0u8; 65536];
    let mut length = 0;
    let mut at = 0;
    loop {
        ensure!(!cancelled.load(Ordering::SeqCst), "ZIP recovery cancelled");
        if at == length {
            reader.seek(SeekFrom::Start(start.checked_add(decoder.total_in()).context("ZIP offset overflow")?))?;
            length = reader.read(&mut input)?;
            at = 0;
        }
        let before_in = decoder.total_in();
        let before_out = decoder.total_out();
        let status = decoder.decompress(&input[at..length], &mut output)
            .context("BZip2 数据无效或内容校验失败")?;
        ensure!(status != bzip2::Status::MemNeeded, "BZip2 解码内存不足");
        let used = (decoder.total_in() - before_in) as usize;
        let produced = (decoder.total_out() - before_out) as usize;
        ensure!(decoder.total_out() <= limit, "ZIP 条目展开体积超过读取上限");
        crc.update(&output[..produced]);
        at += used;
        if status == bzip2::Status::StreamEnd { return Ok((decoder.total_in(), decoder.total_out(), crc.finalize())); }
        // A decoder may emit buffered output without consuming more input.
        // At EOF keep draining it, but never accept a stream without its end marker.
        ensure!(used > 0 || produced > 0, "BZip2 数据无效或被截断");
    }
}

fn lzma_boundary(reader: &mut ZipInputReader, start: u64, flags: u16, cancelled: &AtomicBool, limit: u64) -> Result<(u64, u64, u32)> {
    ensure!(!cancelled.load(Ordering::SeqCst), "ZIP recovery cancelled");
    ensure!(flags & 2 != 0, "LZMA 未声明结束标记，无法确认未知尺寸文件的边界");
    reader.seek(SeekFrom::Start(start))?;
    let mut header = [0u8; 9];
    reader.read_exact(&mut header).context("ZIP LZMA 属性头被截断")?;
    ensure!(word(&header, 2) == 5, "ZIP LZMA 属性长度无效");
    let dictionary = dword(&header, 5);
    let memory = lzma_rust2::lzma_get_memory_usage_by_props(dictionary, header[4])
        .context("ZIP LZMA 属性无效")?;
    // With no trusted size, the normal ZIP reader's small-file dictionary
    // clamp is unavailable. Bound this recovery decoder before allocating it.
    ensure!(memory <= 256 * 1024, "LZMA 未知尺寸恢复所需内存超过 256 MiB，未读取此条目");
    let mut decoder = lzma_rust2::LzmaReader::new_with_props(&mut *reader, u64::MAX, header[4], dictionary, None)?;
    let mut output = [0u8; 65536];
    let mut expanded = 0u64;
    let mut crc = crc32fast::Hasher::new();
    loop {
        ensure!(!cancelled.load(Ordering::SeqCst), "ZIP recovery cancelled");
        let read = decoder.read(&mut output).context("LZMA 数据无效、结束标记缺失或内容被截断")?;
        if read == 0 { break; }
        expanded = expanded.checked_add(read as u64).context("ZIP 展开体积溢出")?;
        ensure!(expanded <= limit, "ZIP 条目展开体积超过读取上限");
        crc.update(&output[..read]);
    }
    // LZMA reads ahead into the descriptor/next member. Only consumed bytes
    // belong to this stream; the ZIP descriptor is matched at that exact point.
    let (input, unused) = decoder.into_parts();
    let compressed = input.stream_position()?.checked_sub(unused.len() as u64)
        .and_then(|end| end.checked_sub(start)).context("ZIP LZMA 流边界无效")?;
    Ok((compressed, expanded, crc.finalize()))
}

// Match a descriptor against a declared size or a decoded stream boundary.
// Never search payload bytes for a magic signature to guess where a file ends.
fn descriptor(reader: &mut ZipInputReader, at: u64, compressed: u64, expanded: Option<u64>, crc: Option<u32>, zip64: bool) -> Result<(u64, u64, u32)> {
    reader.seek(SeekFrom::Start(at))?;
    let mut bytes = [0u8; 24];
    let mut length = 0;
    while length < bytes.len() {
        let read = reader.read(&mut bytes[length..])?;
        if read == 0 { break; }
        length += read;
    }
    for prefix in [4usize, 0] {
        if prefix == 4 && (length < 4 || &bytes[..4] != b"PK\x07\x08") { continue; }
        let width = if zip64 { 8 } else { 4 };
        let needed = prefix + 4 + width * 2;
        if needed > length { continue; }
        let actual_crc = dword(&bytes, prefix);
        let size = |offset| if width == 8 { u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap()) } else { dword(&bytes, offset) as u64 };
        let actual_compressed = size(prefix + 4);
        let actual_expanded = size(prefix + 4 + width);
        if actual_compressed == compressed && expanded.is_none_or(|value| value == actual_expanded) && crc.is_none_or(|value| value == actual_crc) {
            return Ok((at + needed as u64, actual_expanded, actual_crc));
        }
    }
    anyhow::bail!("数据描述符与文件内容或声明体积不一致，未恢复此条目")
}

// Keep AES/Unicode/time metadata, but rebuild ZIP64 fields for the central
// header's size/offset layout rather than copying the local layout verbatim.
fn central_extra(extra: &[u8], expanded: u64, compressed: u64, offset: u64) -> Result<Vec<u8>> {
    let mut result = Vec::new();
    let mut at = 0;
    while at < extra.len() {
        ensure!(at + 4 <= extra.len(), "ZIP 本地扩展字段被截断");
        let end = at + 4 + word(extra, at + 2) as usize;
        ensure!(end <= extra.len(), "ZIP 本地扩展字段被截断");
        // Local UT may include access/creation timestamps. Its layout cannot
        // be copied as a central UT; normalize a single confirmed mtime below.
        if !matches!(word(extra, at), 1 | 0x5455) { result.extend_from_slice(&extra[at..end]); }
        at = end;
    }
    if let Some(time) = super::zip_read_time::local_utc_modified(extra) {
        let seconds = time.duration_since(std::time::UNIX_EPOCH)?.as_secs() as u32;
        result.extend_from_slice(&[0x55, 0x54, 5, 0, 1]);
        result.extend_from_slice(&seconds.to_le_bytes());
    }
    let mut zip64 = Vec::new();
    for value in [expanded, compressed, offset] {
        if value >= u32::MAX as u64 { zip64.extend_from_slice(&value.to_le_bytes()); }
    }
    if !zip64.is_empty() {
        result.extend_from_slice(&1u16.to_le_bytes());
        result.extend_from_slice(&(zip64.len() as u16).to_le_bytes());
        result.extend_from_slice(&zip64);
    }
    ensure!(result.len() <= u16::MAX as usize, "ZIP 重建扩展字段过长");
    Ok(result)
}
fn append_central(directory: &mut Vec<u8>, header: &[u8; 30], name: &[u8], extra: &[u8], offset: u64, expanded: u64, compressed: u64) -> Result<()> {
    let extra = central_extra(extra, expanded, compressed, offset)?;
    let mut central = [0u8; 46];
    central[..4].copy_from_slice(b"PK\x01\x02");
    put_word(&mut central, 4, 45); // DOS attributes; no unavailable permissions guessed.
    central[6..28].copy_from_slice(&header[4..26]);
    if [expanded, compressed, offset].iter().any(|value| *value >= u32::MAX as u64) {
        put_word(&mut central, 6, word(header, 4).max(45));
    }
    put_dword(&mut central, 20, compressed.min(u32::MAX as u64) as u32);
    put_dword(&mut central, 24, expanded.min(u32::MAX as u64) as u32);
    put_word(&mut central, 28, name.len() as u16);
    put_word(&mut central, 30, extra.len() as u16);
    if name.ends_with(b"/") || name.ends_with(b"\\") { put_dword(&mut central, 38, 0x10); }
    put_dword(&mut central, 42, offset.min(u32::MAX as u64) as u32);
    directory.extend_from_slice(&central);
    directory.extend_from_slice(name);
    directory.extend_from_slice(&extra);
    Ok(())
}
fn append_end(directory: &mut Vec<u8>, offset: u64, count: usize) {
    let size = directory.len() as u64;
    let zip64 = count >= u16::MAX as usize || offset >= u32::MAX as u64 || size >= u32::MAX as u64;
    if zip64 {
        let end_offset = offset + size;
        directory.extend_from_slice(b"PK\x06\x06");
        directory.extend_from_slice(&44u64.to_le_bytes());
        directory.extend_from_slice(&45u16.to_le_bytes());
        directory.extend_from_slice(&45u16.to_le_bytes());
        directory.extend_from_slice(&0u32.to_le_bytes());
        directory.extend_from_slice(&0u32.to_le_bytes());
        directory.extend_from_slice(&(count as u64).to_le_bytes());
        directory.extend_from_slice(&(count as u64).to_le_bytes());
        directory.extend_from_slice(&size.to_le_bytes());
        directory.extend_from_slice(&offset.to_le_bytes());
        directory.extend_from_slice(b"PK\x06\x07");
        directory.extend_from_slice(&0u32.to_le_bytes());
        directory.extend_from_slice(&end_offset.to_le_bytes());
        directory.extend_from_slice(&1u32.to_le_bytes());
    }
    let mut end = [0u8; 22];
    end[..4].copy_from_slice(b"PK\x05\x06");
    put_word(&mut end, 8, count.min(u16::MAX as usize) as u16);
    put_word(&mut end, 10, count.min(u16::MAX as usize) as u16);
    put_dword(&mut end, 12, size.min(u32::MAX as u64) as u32);
    put_dword(&mut end, 16, offset.min(u32::MAX as u64) as u32);
    directory.extend_from_slice(&end);
}

pub(crate) fn can_scan_local(path: &Path) -> bool {
    RecoveryReader::open(path, &AtomicBool::new(false)).is_ok_and(|reader| {
        reader.directory_rebuilt && !reader.encrypted && reader.names.iter().any(|name| !name.ends_with(['/', '\\']))
    })
}

impl RecoveryReader {
    fn from_archive(mut archive: zip_aes::ZipArchive<RecoveryZipInput>, directory_rebuilt: bool, notes: Vec<String>, cancelled: &AtomicBool) -> Result<Self> {
        let mut names = Vec::new();
        let mut expanded_bytes = 0u64;
        let mut encrypted = false;
        for index in 0..archive.len() {
            ensure!(!cancelled.load(Ordering::SeqCst), "ZIP recovery cancelled");
            let raw = archive.by_index_raw(index)?;
            encrypted |= raw.encrypted();
            names.push(raw.name().to_string());
            expanded_bytes = expanded_bytes.saturating_add(raw.size());
        }
        let entry_results = notes.iter().map(|reason| RecoveryEntryResult::new(None, RecoveryStatus::Unknown, reason.clone())).collect();
        Ok(Self { archive, names, expanded_bytes, encrypted, directory_rebuilt, notes, entry_results })
    }
    pub fn open(path: &Path, cancelled: &AtomicBool) -> Result<Self> {
        if super::split_archive_detector::SplitArchiveDetector::detect_split_archive(path)?
            .is_some_and(|group| !group.is_complete)
        {
            let mut partial = super::standard_zip_reader::StandardZipReader::try_open_recovery(path, cancelled)?
                .context("此卷型暂不提供缺卷救援，请补齐原始分卷后重试")?;
            let notes = std::mem::take(&mut partial.recovery_notes);
            let entries = std::mem::take(&mut partial.recovery_entries);
            let encrypted = partial.recovery_encrypted;
            let input = RecoveryZipInput::from_source(ZipInputReader::StandardZip(partial))?;
            let mut result = Self::from_archive(zip_aes::ZipArchive::new(input)?, false, notes, cancelled)?;
            result.encrypted |= encrypted;
            result.entry_results = entries;
            return Ok(result);
        }
        if let Ok(input) = RecoveryZipInput::original(path) {
            if let Ok(archive) = zip_aes::ZipArchive::new(input) {
                return Self::from_archive(archive, false, Vec::new(), cancelled);
            }
        }
        let mut reader = ZipInputReader::open_recovery(path)?;
        let length = reader.seek(SeekFrom::End(0))?;
        let mut position = 0u64;
        let mut count = 0usize;
        let mut directory = Vec::new();
        let mut seen_names = std::collections::HashSet::new();
        let mut metadata_bytes = 0usize;
        let mut notes = Vec::new();
        let mut unknown_entries = Vec::new();
        macro_rules! unknown {
            ($name:expr, $reason:expr) => {{
                let reason = $reason;
                notes.push(reason.clone());
                unknown_entries.push(RecoveryEntryResult::new($name, RecoveryStatus::Unknown, reason));
            }};
        }
        let mut observed_encrypted = false;
        while position < length {
            ensure!(!cancelled.load(Ordering::SeqCst), "ZIP recovery cancelled");
            reader.seek(SeekFrom::Start(position))?;
            if length - position < 4 { unknown!(None, format!("尾部在偏移 {position} 被截断，后续内容无法确认")); break; }
            let mut signature = [0; 4];
            reader.read_exact(&mut signature)?;
            // APPNOTE spanning marker belongs to the volume set, not a file.
            if position == 0 && &signature == b"PK\x07\x08" { position = 4; continue; }
            if &signature == b"PK\x01\x02" { break; }
            if &signature != b"PK\x03\x04" { unknown!(None, format!("偏移 {position} 不是可信本地文件头，未扫描后续内容")); break; }
            ensure!(count < extraction_transaction::MAX_EXTRACTED_ENTRIES, "ZIP 可恢复条目数超过读取上限");
            if length - position < 30 { unknown!(None, format!("偏移 {position} 的文件头被截断")); break; }
            let mut header = [0; 30];
            header[..4].copy_from_slice(&signature);
            reader.read_exact(&mut header[4..])?;
            let name_length = word(&header, 26) as usize;
            let extra_length = word(&header, 28) as usize;
            let header_end = position.checked_add(30 + name_length as u64 + extra_length as u64).context("ZIP 本地头大小溢出")?;
            if header_end > length { unknown!(None, format!("偏移 {position} 的名称或扩展信息被截断")); break; }
            metadata_bytes = metadata_bytes.checked_add(30 + name_length + extra_length).context("ZIP 元数据大小溢出")?;
            ensure!(metadata_bytes <= 256 * 1024 * 1024, "ZIP 本地元数据超过读取上限");
            let mut name = vec![0; name_length];
            reader.read_exact(&mut name)?;
            let label = String::from_utf8_lossy(&name);
            let flags = word(&header, 6);
            observed_encrypted |= flags & 1 != 0;
            if flags & (0x40 | 0x2000) != 0 {
                unknown!(Some(label.to_string()), format!("{label}: 当前不支持此加密或隐藏元数据方式，未扫描后续内容")); break;
            }
            let mut extra = vec![0; extra_length];
            reader.read_exact(&mut extra)?;
            let sizes = (|| -> Result<(u64, u64)> {
                let mut expanded = dword(&header, 22) as u64;
                let mut compressed = dword(&header, 18) as u64;
                if expanded == u32::MAX as u64 || compressed == u32::MAX as u64 {
                    let mut at = 0;
                    let mut found = false;
                    while at + 4 <= extra.len() {
                        let end = at + 4 + word(&extra, at + 2) as usize;
                        ensure!(end <= extra.len(), "ZIP64 本地扩展字段被截断");
                        if word(&extra, at) == 1 {
                            let mut cursor = at + 4;
                            if expanded == u32::MAX as u64 {
                                ensure!(cursor + 8 <= end, "ZIP64 原始体积缺失");
                                expanded = u64::from_le_bytes(extra[cursor..cursor + 8].try_into().unwrap()); cursor += 8;
                            }
                            if compressed == u32::MAX as u64 {
                                ensure!(cursor + 8 <= end, "ZIP64 压缩体积缺失");
                                compressed = u64::from_le_bytes(extra[cursor..cursor + 8].try_into().unwrap());
                            }
                            found = true; break;
                        }
                        at = end;
                    }
                    ensure!(found, "ZIP64 本地体积信息缺失，不能猜测条目边界");
                }
                Ok((expanded, compressed))
            })();
            let (mut expanded, mut compressed) = match sizes {
                Ok(sizes) => sizes,
                Err(error) => { unknown!(Some(label.to_string()), format!("{label}: {error}；未扫描后续内容")); break; }
            };
            let mut decoded_crc = None;
            if flags & 8 != 0 && compressed == 0 {
                let method = word(&header, 8);
                if flags & 1 == 0 && matches!(method, 8 | 12 | 14) {
                    let boundary = match method {
                        14 => lzma_boundary(&mut reader, header_end, flags, cancelled, extraction_transaction::MAX_EXTRACTED_BYTES),
                        12 => bzip2_boundary(&mut reader, header_end, cancelled, extraction_transaction::MAX_EXTRACTED_BYTES),
                        _ => deflate_boundary(&mut reader, header_end, cancelled),
                    };
                    match boundary {
                        Ok((input, output, crc)) => {
                            if matches!(method, 12 | 14) && expanded != 0 && expanded != output {
                                unknown!(Some(label.to_string()), format!("{label}: 本地声明体积与内容不一致，未扫描后续内容")); break;
                            }
                            compressed = input; expanded = output; decoded_crc = Some(crc);
                        }
                        Err(error) if cancelled.load(Ordering::SeqCst) => return Err(error),
                        Err(error) => { unknown!(Some(label.to_string()), format!("{label}: {error}；未扫描后续内容")); break; }
                    }
                } else if !name.ends_with(b"/") {
                    unknown!(Some(label.to_string()), format!("{label}: 数据描述符缺少可确定的文件边界，未扫描后续内容")); break;
                }
            }
            let Some(mut end) = header_end.checked_add(compressed) else { unknown!(Some(label.to_string()), format!("{label}: 本地条目范围溢出，未扫描后续内容")); break; };
            if end > length { unknown!(Some(label.to_string()), format!("{label}: 内容被截断，未恢复此条目和后续内容")); break; }
            if flags & 8 != 0 {
                let known_crc = decoded_crc.or_else(|| (dword(&header, 14) != 0).then(|| dword(&header, 14)));
                let zip64 = dword(&header, 18) == u32::MAX || dword(&header, 22) == u32::MAX;
                let expected_expanded = (expanded != 0 || decoded_crc.is_some() || name.ends_with(b"/")).then_some(expanded);
                match descriptor(&mut reader, end, compressed, expected_expanded, known_crc, zip64) {
                    Ok((next, size, crc)) => { end = next; expanded = size; put_dword(&mut header, 14, crc); }
                    Err(error) => { unknown!(Some(label.to_string()), format!("{label}: {error}；未扫描后续内容")); break; }
                }
            }
            if !seen_names.insert(name.clone()) {
                unknown!(Some(label.to_string()), format!("{label}: 包内名称重复，未恢复此条目"));
                position = end;
                continue;
            }
            if let Err(error) = append_central(&mut directory, &header, &name, &extra, position, expanded, compressed) {
                unknown!(Some(label.to_string()), format!("{label}: {error}；未扫描后续内容")); break;
            }
            count += 1;
            position = end;
        }
        ensure!(count > 0, "未找到具有可靠边界的完整 ZIP 文件条目；未创建新文件");
        append_end(&mut directory, position, count);
        let input = RecoveryZipInput { source: reader, prefix_len: position, directory, position: 0 };
        let archive = zip_aes::ZipArchive::new(input).context("无法读取具有明确边界的 ZIP 条目")?;
        let mut result = Self::from_archive(archive, true, notes, cancelled)?;
        // With no complete directory the total file list is not knowable, even
        // when every bounded local record was read successfully.
        unknown_entries.push(RecoveryEntryResult::new(None, RecoveryStatus::Unknown, "原目录不完整，无法确认是否还有其他文件；仅列出已确认的本地条目"));
        result.entry_results = unknown_entries;
        result.encrypted |= observed_encrypted;
        Ok(result)
    }
    // Only check password headers here. Full content/authentication validation
    // happens once while rescuing each file, before any output is published.
    #[cfg(test)]
    pub fn check_password_headers(&mut self, password: &str) -> zip_aes::result::ZipResult<()> {
        self.check_password_headers_with_encoding(password, None)
    }

    pub fn check_password_headers_with_encoding(&mut self, password: &str,
        encoding: Option<super::zip_password::ZipPasswordEncoding>) -> zip_aes::result::ZipResult<()> {
        let mut invalid = false;
        let mut other = None;
        for index in 0..self.archive.len() {
            let raw = self.archive.by_index_raw(index)?;
            let encrypted_file = raw.encrypted() && !raw.is_dir();
            drop(raw);
            if !encrypted_file { continue; }
            match super::zip_password::open_entry_with_encoding(&mut self.archive, index, password, encoding) {
                Ok(_) => return Ok(()),
                Err(zip_aes::result::ZipError::InvalidPassword) => invalid = true,
                Err(error) => other = Some(error),
            }
        }
        if let Some(error) = other { Err(error) }
        else if invalid { Err(zip_aes::result::ZipError::InvalidPassword) }
        else { Err(zip_aes::result::ZipError::UnsupportedArchive("未找到具有明确边界的完整加密文件".into())) }
    }
    #[cfg(test)]
    pub fn entry(&mut self, index: usize, password: Option<&str>) -> zip_aes::result::ZipResult<zip_aes::read::ZipFile<'_, RecoveryZipInput>> {
        self.entry_with_encoding(index, password, None)
    }

    pub fn entry_with_encoding(&mut self, index: usize, password: Option<&str>,
        encoding: Option<super::zip_password::ZipPasswordEncoding>) -> zip_aes::result::ZipResult<zip_aes::read::ZipFile<'_, RecoveryZipInput>> {
        match password {
            Some(password) => super::zip_password::open_entry_with_encoding(&mut self.archive, index, password, encoding),
            None => self.archive.by_index(index),
        }
    }
}
