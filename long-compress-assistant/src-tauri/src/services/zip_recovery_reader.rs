//! A read-only ZIP view rebuilt from declared local-record boundaries.
//! Payloads stay in the source; only a synthetic directory is held in memory.
use super::{extraction_transaction, numeric_volume_reader::ZipInputReader};
use anyhow::{ensure, Context, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{io::{self, Read, Seek, SeekFrom}, path::Path};

pub(crate) struct RecoveryZipInput {
    source: ZipInputReader,
    prefix_len: u64,
    directory: Vec<u8>,
    position: u64,
}
impl RecoveryZipInput {
    fn original(path: &Path) -> Result<Self> {
        let mut source = ZipInputReader::open(path)?;
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
}
fn word(bytes: &[u8], at: usize) -> u16 { u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap()) }
fn dword(bytes: &[u8], at: usize) -> u32 { u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) }
fn put_word(bytes: &mut [u8], at: usize, value: u16) { bytes[at..at + 2].copy_from_slice(&value.to_le_bytes()); }
fn put_dword(bytes: &mut [u8], at: usize, value: u32) { bytes[at..at + 4].copy_from_slice(&value.to_le_bytes()); }

// Keep AES/Unicode/time metadata, but rebuild ZIP64 fields for the central
// header's size/offset layout rather than copying the local layout verbatim.
fn central_extra(extra: &[u8], expanded: u64, compressed: u64, offset: u64) -> Result<Vec<u8>> {
    let mut result = Vec::new();
    let mut at = 0;
    while at < extra.len() {
        ensure!(at + 4 <= extra.len(), "ZIP 本地扩展字段被截断");
        let end = at + 4 + word(extra, at + 2) as usize;
        ensure!(end <= extra.len(), "ZIP 本地扩展字段被截断");
        if word(extra, at) != 1 { result.extend_from_slice(&extra[at..end]); }
        at = end;
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
        Ok(Self { archive, names, expanded_bytes, encrypted, directory_rebuilt, notes })
    }
    pub fn open(path: &Path, cancelled: &AtomicBool) -> Result<Self> {
        if let Ok(archive) = zip_aes::ZipArchive::new(RecoveryZipInput::original(path)?) {
            return Self::from_archive(archive, false, Vec::new(), cancelled);
        }
        let mut reader = ZipInputReader::open(path)?;
        // The standard multi-disk view needs its original central directory.
        ensure!(!matches!(&reader, ZipInputReader::StandardZip(_)), "标准 ZIP 分卷丢目录重建暂不支持，请保留原卷组");
        let length = reader.seek(SeekFrom::End(0))?;
        let mut position = 0u64;
        let mut count = 0usize;
        let mut directory = Vec::new();
        let mut seen_names = std::collections::HashSet::new();
        let mut metadata_bytes = 0usize;
        let mut notes = Vec::new();
        let mut observed_encrypted = false;
        while position < length {
            ensure!(!cancelled.load(Ordering::SeqCst), "ZIP recovery cancelled");
            reader.seek(SeekFrom::Start(position))?;
            if length - position < 4 { notes.push(format!("尾部在偏移 {position} 被截断，后续内容无法确认")); break; }
            let mut signature = [0; 4];
            reader.read_exact(&mut signature)?;
            if &signature == b"PK\x01\x02" { break; }
            if &signature != b"PK\x03\x04" { notes.push(format!("偏移 {position} 不是可信本地文件头，未扫描后续内容")); break; }
            ensure!(count < extraction_transaction::MAX_EXTRACTED_ENTRIES, "ZIP 可恢复条目数超过读取上限");
            if length - position < 30 { notes.push(format!("偏移 {position} 的文件头被截断")); break; }
            let mut header = [0; 30];
            header[..4].copy_from_slice(&signature);
            reader.read_exact(&mut header[4..])?;
            let name_length = word(&header, 26) as usize;
            let extra_length = word(&header, 28) as usize;
            let header_end = position.checked_add(30 + name_length as u64 + extra_length as u64).context("ZIP 本地头大小溢出")?;
            if header_end > length { notes.push(format!("偏移 {position} 的名称或扩展信息被截断")); break; }
            metadata_bytes = metadata_bytes.checked_add(30 + name_length + extra_length).context("ZIP 元数据大小溢出")?;
            ensure!(metadata_bytes <= 256 * 1024 * 1024, "ZIP 本地元数据超过读取上限");
            let mut name = vec![0; name_length];
            reader.read_exact(&mut name)?;
            let label = String::from_utf8_lossy(&name);
            let flags = word(&header, 6);
            observed_encrypted |= flags & 1 != 0;
            if flags & 8 != 0 {
                notes.push(format!("{label}: 使用数据描述符，缺少目录后无法可靠确定条目边界；未扫描后续内容")); break;
            }
            if flags & (0x40 | 0x2000) != 0 {
                notes.push(format!("{label}: 当前不支持此加密或隐藏元数据方式，未扫描后续内容")); break;
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
            let (expanded, compressed) = match sizes {
                Ok(sizes) => sizes,
                Err(error) => { notes.push(format!("{label}: {error}；未扫描后续内容")); break; }
            };
            let Some(end) = header_end.checked_add(compressed) else { notes.push(format!("{label}: 本地条目范围溢出，未扫描后续内容")); break; };
            if end > length { notes.push(format!("{label}: 内容被截断，未恢复此条目和后续内容")); break; }
            if !seen_names.insert(name.clone()) {
                notes.push(format!("{label}: 包内名称重复，未恢复此条目"));
                position = end;
                continue;
            }
            if let Err(error) = append_central(&mut directory, &header, &name, &extra, position, expanded, compressed) {
                notes.push(format!("{label}: {error}；未扫描后续内容")); break;
            }
            count += 1;
            position = end;
        }
        ensure!(count > 0, "未找到具有可靠边界的完整 ZIP 文件条目；未创建新文件");
        append_end(&mut directory, position, count);
        let input = RecoveryZipInput { source: reader, prefix_len: position, directory, position: 0 };
        let archive = zip_aes::ZipArchive::new(input).context("无法读取具有明确边界的 ZIP 条目")?;
        let mut result = Self::from_archive(archive, true, notes, cancelled)?;
        result.encrypted |= observed_encrypted;
        Ok(result)
    }
    // Only check password headers here. Full content/authentication validation
    // happens once while rescuing each file, before any output is published.
    pub fn check_password_headers(&mut self, password: &str) -> zip_aes::result::ZipResult<()> {
        let mut invalid = false;
        let mut other = None;
        for index in 0..self.archive.len() {
            let raw = self.archive.by_index_raw(index)?;
            let encrypted_file = raw.encrypted() && !raw.is_dir();
            drop(raw);
            if !encrypted_file { continue; }
            match super::zip_password::open_entry(&mut self.archive, index, password) {
                Ok(_) => return Ok(()),
                Err(zip_aes::result::ZipError::InvalidPassword) => invalid = true,
                Err(error) => other = Some(error),
            }
        }
        if let Some(error) = other { Err(error) }
        else if invalid { Err(zip_aes::result::ZipError::InvalidPassword) }
        else { Err(zip_aes::result::ZipError::UnsupportedArchive("未找到具有明确边界的完整加密文件".into())) }
    }
    pub fn entry(&mut self, index: usize, password: Option<&str>) -> zip_aes::result::ZipResult<zip_aes::read::ZipFile<'_, RecoveryZipInput>> {
        match password {
            Some(password) => super::zip_password::open_entry(&mut self.archive, index, password),
            None => self.archive.by_index(index),
        }
    }
}
