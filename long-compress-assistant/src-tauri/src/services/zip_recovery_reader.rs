//! Recover only trusted, declared local-record boundaries. Never search for
//! arbitrary PK bytes inside compressed payloads or guess an unknown length.
use super::{extraction_transaction, numeric_volume_reader::ZipInputReader};
use anyhow::{ensure, Context, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    io::{Read, Seek, SeekFrom},
    path::Path,
};

struct LocalRecord {
    offset: u64,
}
pub(crate) struct RecoveryReader {
    archive: Option<zip_aes::ZipArchive<ZipInputReader>>,
    local: Option<(ZipInputReader, Vec<LocalRecord>)>,
    pub names: Vec<String>,
    pub expanded_bytes: u64,
    pub encrypted: bool,
    pub directory_rebuilt: bool,
    pub notes: Vec<String>,
}

fn word(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
}
fn dword(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}
pub(crate) fn can_scan_local(path: &Path) -> bool {
    // Probe metadata boundaries, not just the first (possibly plain directory)
    // header. A later encrypted entry must not advertise password repair.
    RecoveryReader::open(path, &AtomicBool::new(false)).is_ok_and(|reader| {
        reader.directory_rebuilt && reader.names.iter().any(|name| !name.ends_with(['/', '\\']))
    })
}

impl RecoveryReader {
    pub fn open(path: &Path, cancelled: &AtomicBool) -> Result<Self> {
        if let Ok(mut archive) = zip_aes::ZipArchive::new(ZipInputReader::open(path)?) {
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
            return Ok(Self {
                archive: Some(archive),
                local: None,
                names,
                expanded_bytes,
                encrypted,
                directory_rebuilt: false,
                notes: Vec::new(),
            });
        }
        let mut reader = ZipInputReader::open(path)?;
        let length = reader.seek(SeekFrom::End(0))?;
        let mut position = 0u64;
        let mut records = Vec::new();
        let mut names = Vec::new();
        let mut expanded_bytes = 0u64;
        let mut metadata_bytes = 0usize;
        let mut notes = Vec::new();
        while position < length {
            ensure!(!cancelled.load(Ordering::SeqCst), "ZIP recovery cancelled");
            reader.seek(SeekFrom::Start(position))?;
            if length - position < 4 {
                notes.push(format!("尾部在偏移 {position} 被截断，后续内容无法确认"));
                break;
            }
            let mut signature = [0; 4];
            reader.read_exact(&mut signature)?;
            if &signature == b"PK\x01\x02" {
                break;
            }
            if &signature != b"PK\x03\x04" {
                notes.push(format!(
                    "偏移 {position} 不是可信本地文件头，未扫描后续内容"
                ));
                break;
            }
            ensure!(
                records.len() < extraction_transaction::MAX_EXTRACTED_ENTRIES,
                "ZIP 可恢复条目数超过读取上限"
            );
            if length - position < 30 {
                notes.push(format!("偏移 {position} 的文件头被截断"));
                break;
            }
            let mut header = [0; 30];
            header[..4].copy_from_slice(&signature);
            reader.read_exact(&mut header[4..])?;
            let name_length = word(&header, 26) as usize;
            let extra_length = word(&header, 28) as usize;
            let header_end = position
                .checked_add(30 + name_length as u64 + extra_length as u64)
                .context("ZIP 本地头大小溢出")?;
            if header_end > length {
                notes.push(format!("偏移 {position} 的名称或扩展信息被截断"));
                break;
            }
            metadata_bytes = metadata_bytes
                .checked_add(30 + name_length + extra_length)
                .context("ZIP 元数据大小溢出")?;
            ensure!(
                metadata_bytes <= 256 * 1024 * 1024,
                "ZIP 本地元数据超过读取上限"
            );
            let mut name = vec![0; name_length];
            reader.read_exact(&mut name)?;
            let name = String::from_utf8_lossy(&name).into_owned();
            let flags = word(&header, 6);
            ensure!(flags & 1 == 0, "暂不支持密码 ZIP 重建；原压缩包未修改");
            if flags & 8 != 0 {
                notes.push(format!(
                    "{name}: 使用数据描述符，缺少目录后无法可靠确定条目边界；未扫描后续内容"
                ));
                break;
            }
            let sizes = (|| -> Result<(u64, u64)> {
                let mut expanded = dword(&header, 22) as u64;
                let mut compressed = dword(&header, 18) as u64;
                let mut extra = vec![0; extra_length];
                reader.read_exact(&mut extra)?;
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
                                expanded = u64::from_le_bytes(
                                    extra[cursor..cursor + 8].try_into().unwrap(),
                                );
                                cursor += 8;
                            }
                            if compressed == u32::MAX as u64 {
                                ensure!(cursor + 8 <= end, "ZIP64 压缩体积缺失");
                                compressed = u64::from_le_bytes(
                                    extra[cursor..cursor + 8].try_into().unwrap(),
                                );
                            }
                            found = true;
                            break;
                        }
                        at = end;
                    }
                    ensure!(found, "ZIP64 本地体积信息缺失，不能猜测条目边界");
                }
                Ok((expanded, compressed))
            })();
            let (expanded, compressed) = match sizes {
                Ok(sizes) => sizes,
                Err(error) => {
                    notes.push(format!("{name}: {error}；未扫描后续内容"));
                    break;
                }
            };
            let Some(end) = header_end.checked_add(compressed) else {
                notes.push(format!("{name}: 本地条目范围溢出，未扫描后续内容"));
                break;
            };
            if end > length {
                notes.push(format!("{name}: 内容被截断，未恢复此条目和后续内容"));
                break;
            }
            records.push(LocalRecord { offset: position });
            names.push(name);
            expanded_bytes = expanded_bytes.saturating_add(expanded);
            position = end;
        }
        ensure!(
            !records.is_empty(),
            "未找到具有可靠边界的完整 ZIP 文件条目；未创建新文件"
        );
        Ok(Self {
            archive: None,
            local: Some((reader, records)),
            names,
            expanded_bytes,
            encrypted: false,
            directory_rebuilt: true,
            notes,
        })
    }
    pub fn entry(
        &mut self,
        index: usize,
        password: Option<&str>,
    ) -> zip_aes::result::ZipResult<zip_aes::read::ZipFile<'_, ZipInputReader>> {
        if let Some(archive) = self.archive.as_mut() {
            return match password {
                Some(password) => super::zip_password::open_entry(archive, index, password),
                None => archive.by_index(index),
            };
        }
        let (reader, records) = self.local.as_mut().unwrap();
        reader.seek(SeekFrom::Start(records[index].offset))?;
        zip_aes::read::read_zipfile_from_stream(reader)?
            .ok_or_else(|| zip_aes::result::ZipError::InvalidArchive("本地条目缺失".into()))
    }
}
