//! Standard .z01/.../.zip disks exposed as a read-only, single-disk ZIP view.
//! Payload stays on the original disks. Only directory records are rebuilt in
//! memory, translating disk-relative offsets to 64-bit positions for native
//! ZIP metadata, passwords, previews and extraction. No merged payload copy.
use super::{
    numeric_volume_reader::NumericVolumeReader,
    split_archive_detector::{SplitArchiveDetector, SplitFormat},
};
use anyhow::{ensure, Context, Result};
use super::zip_recovery_outcome::{RecoveryEntryResult, RecoveryStatus};
use std::{
    io::{self, Read, Seek, SeekFrom},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

pub(crate) struct StandardZipReader {
    source: NumericVolumeReader,
    directory_start: u64,
    directory: Vec<u8>,
    position: u64,
    pub(crate) recovery_notes: Vec<String>,
    pub(crate) recovery_encrypted: bool,
    pub(crate) recovery_entries: Vec<RecoveryEntryResult>,
}

// Compact positions are private to this view. Missing disk lengths are never
// inferred: every candidate must fit within consecutive, physically present disks.
struct DiskLayout {
    starts: Vec<u64>,
    indices: Vec<Option<usize>>,
}
impl DiskLayout {
    fn absolute(&self, disk: u32, relative: u64) -> Result<u64> {
        let index = self.indices.get(disk as usize).copied().flatten()
            .context("文件涉及缺失分卷")?;
        absolute(&self.starts, index as u32, relative)
    }
    fn contiguous_end(&self, disk: u32) -> Result<u64> {
        let mut index = self.indices.get(disk as usize).copied().flatten()
            .context("文件涉及缺失分卷")?;
        for next in self.indices.iter().skip(disk as usize + 1) {
            if *next != Some(index + 1) { break; }
            index += 1;
        }
        Ok(self.starts[index + 1])
    }
}

fn u16_at(bytes: &[u8], index: usize) -> u16 {
    u16::from_le_bytes(bytes[index..index + 2].try_into().unwrap())
}
fn u32_at(bytes: &[u8], index: usize) -> u32 {
    u32::from_le_bytes(bytes[index..index + 4].try_into().unwrap())
}
fn u64_at(bytes: &[u8], index: usize) -> u64 {
    u64::from_le_bytes(bytes[index..index + 8].try_into().unwrap())
}
fn read_at(reader: &mut NumericVolumeReader, offset: u64, size: usize) -> Result<Vec<u8>> {
    reader.seek(SeekFrom::Start(offset))?;
    let mut bytes = vec![0; size];
    reader.read_exact(&mut bytes)?;
    Ok(bytes)
}
fn absolute(starts: &[u64], disk: u32, relative: u64) -> Result<u64> {
    let start = *starts
        .get(disk as usize)
        .context("ZIP references a missing disk")?;
    let end = *starts
        .get(disk as usize + 1)
        .context("ZIP disk boundary missing")?;
    ensure!(relative <= end - start, "ZIP offset is outside its disk");
    start.checked_add(relative).context("ZIP offset overflow")
}

impl StandardZipReader {
    pub(crate) fn try_open(path: &Path) -> Result<Option<Self>> {
        Self::open(path, false, None)
    }

    pub(crate) fn try_open_recovery(path: &Path, cancelled: &AtomicBool) -> Result<Option<Self>> {
        Self::open(path, true, Some(cancelled))
    }

    fn open(path: &Path, allow_missing: bool, cancelled: Option<&AtomicBool>) -> Result<Option<Self>> {
        if !path.file_name().and_then(|name| name.to_str())
            .is_some_and(|name| SplitArchiveDetector::zip_disk_family(name).is_some()) {
            return Ok(None);
        }
        let Some(group) = SplitArchiveDetector::detect_split_archive(path)?
            .filter(|group| group.format == SplitFormat::ZipSplit)
        else {
            return Ok(None);
        };
        ensure!(
            group.is_complete || allow_missing,
            "[archive-inspection:incomplete] 缺少 ZIP 分卷：{}",
            group.missing_part_names()
        );
        let partial = !group.is_complete;
        let forum = SplitArchiveDetector::forum_zip_disk(path).is_some();
        let mut indices = vec![None; group.total_parts];
        if partial && !forum {
            ensure!(group.first_part.is_file()
                && group.first_part.file_stem().and_then(|name| name.to_str())
                    .is_some_and(|name| name.eq_ignore_ascii_case(&group.base_name)),
                "缺卷救援需要可读终卷目录，请补齐终卷后重试");
        }
        let mut starts = vec![0u64];
        for (index, part) in group.parts.iter().enumerate() {
            let disk = if forum {
                SplitArchiveDetector::forum_zip_disk(part).context("无法确定论坛 ZIP 分卷编号")?
            } else if !partial { index } else if part == &group.first_part {
                group.total_parts - 1
            } else {
                part.extension().and_then(|extension| extension.to_str())
                    .map(str::to_ascii_lowercase)
                    .and_then(|extension| extension.strip_prefix("zx").or_else(|| extension.strip_prefix('z'))
                        .and_then(|number| number.parse::<usize>().ok()))
                    .and_then(|number| number.checked_sub(1)).context("无法确定 ZIP 分卷编号")?
            };
            ensure!(disk < indices.len() && indices[disk].is_none(), "ZIP 分卷编号重复或越界");
            indices[disk] = Some(index);
            starts.push(
                starts
                    .last()
                    .unwrap()
                    .checked_add(part.metadata()?.len())
                    .context("ZIP size overflow")?,
            );
        }
        let length = *starts.last().unwrap();
        let layout = DiskLayout { starts, indices };
        let missing_names = group.missing_part_names();
        let terminal_start = layout.absolute((group.total_parts - 1) as u32, 0)?;
        let mut source = NumericVolumeReader::from_paths(group.parts)?;
        let tail_start = length.saturating_sub(65_557).max(if partial { terminal_start } else { 0 });
        let tail = read_at(&mut source, tail_start, (length - tail_start) as usize)?;
        let eocd = (0..tail.len().saturating_sub(21))
            .rev()
            .find(|&index| {
                &tail[index..index + 4] == b"PK\x05\x06"
                    && index + 22 + u16_at(&tail, index + 20) as usize == tail.len()
            })
            .context("[archive-inspection:damaged] ZIP 终卷目录损坏或不完整")?;
        let eocd_position = tail_start + eocd as u64;
        let footer = &tail[eocd..];
        let mut last_disk = u16_at(footer, 4) as u32;
        let mut cd_disk = u16_at(footer, 6) as u32;
        let mut count = u16_at(footer, 10) as u64;
        let mut cd_size = u32_at(footer, 12) as u64;
        let mut cd_relative = u32_at(footer, 16) as u64;
        // ZIP64 locator can exist even when the small ZIP fields aren't saturated.
        if eocd_position >= 20 && (!partial || eocd_position - 20 >= terminal_start) {
            let locator = read_at(&mut source, eocd_position - 20, 20)?;
            if &locator[..4] == b"PK\x06\x07" {
                ensure!(
                    u32_at(&locator, 16) as usize == layout.indices.len(),
                    "[archive-inspection:incomplete] ZIP64 卷数与实际文件不符"
                );
                let record_disk = u32_at(&locator, 4);
                let record_at = layout.absolute(record_disk, u64_at(&locator, 8))?;
                ensure!(record_at.checked_add(56).is_some_and(|end| end <= layout.contiguous_end(record_disk).unwrap_or(0)),
                    "ZIP64 目录信息涉及缺失分卷，请补齐后重试");
                let record = read_at(
                    &mut source,
                    record_at,
                    56,
                )?;
                ensure!(
                    &record[..4] == b"PK\x06\x06" && u64_at(&record, 4) >= 44,
                    "Invalid ZIP64 end record"
                );
                last_disk = u32_at(&record, 16);
                cd_disk = u32_at(&record, 20);
                count = u64_at(&record, 32);
                cd_size = u64_at(&record, 40);
                cd_relative = u64_at(&record, 48);
            }
        }
        ensure!(
            last_disk as usize + 1 == layout.indices.len(),
            "[archive-inspection:incomplete] ZIP 终卷声明的卷数与实际文件不符"
        );
        let directory_start = layout.absolute(cd_disk, cd_relative)?;
        ensure!(
            directory_start
                .checked_add(cd_size)
                .is_some_and(|end| end <= eocd_position),
            "Invalid ZIP directory range"
        );
        ensure!(directory_start.checked_add(cd_size).is_some_and(|end| end <= layout.contiguous_end(cd_disk).unwrap_or(0)),
            "ZIP 目录涉及缺失分卷，无法确定完整文件清单；请补齐分卷后重试");
        ensure!(
            cd_size <= 256 * 1024 * 1024,
            "ZIP directory exceeds the metadata reading limit"
        );
        let original = read_at(&mut source, directory_start, cd_size as usize)?;
        let mut directory = Vec::with_capacity(original.len());
        let mut cursor = 0usize;
        let mut retained = 0u64;
        let mut recovery_notes = Vec::new();
        let mut recovery_encrypted = false;
        let mut recovery_entries = Vec::new();
        if partial {
            recovery_notes.push(format!("缺少：{missing_names}。本次仅救回现有连续分卷中内容完整的文件，原卷保持不变。"));
        }
        for _ in 0..count {
            ensure!(!cancelled.is_some_and(|flag| flag.load(Ordering::SeqCst)), "ZIP recovery cancelled");
            ensure!(
                cursor + 46 <= original.len() && &original[cursor..cursor + 4] == b"PK\x01\x02",
                "Invalid ZIP central entry"
            );
            let mut header = original[cursor..cursor + 46].to_vec();
            let name_len = u16_at(&header, 28) as usize;
            let extra_len = u16_at(&header, 30) as usize;
            let comment_len = u16_at(&header, 32) as usize;
            let entry_end = cursor + 46 + name_len + extra_len + comment_len;
            ensure!(entry_end <= original.len(), "Truncated ZIP central entry");
            let original_extra =
                &original[cursor + 46 + name_len..cursor + 46 + name_len + extra_len];
            let mut offset = u32_at(&header, 42) as u64;
            let mut disk = u16_at(&header, 34) as u32;
            let mut compressed = u32_at(&header, 20) as u64;
            recovery_encrypted |= u16_at(&header, 8) & 1 != 0;
            let large_size = u32_at(&header, 24) == u32::MAX;
            let large_compressed = u32_at(&header, 20) == u32::MAX;
            let large_offset = offset == u32::MAX as u64;
            let large_disk = disk == u16::MAX as u32;
            let mut sizes = Vec::new();
            let mut extra = Vec::new();
            let mut extra_cursor = 0;
            let mut found_zip64 = false;
            while extra_cursor < original_extra.len() {
                ensure!(
                    extra_cursor + 4 <= original_extra.len(),
                    "Truncated ZIP extra field"
                );
                let tag = u16_at(original_extra, extra_cursor);
                let end = extra_cursor + 4 + u16_at(original_extra, extra_cursor + 2) as usize;
                ensure!(end <= original_extra.len(), "Truncated ZIP extra payload");
                if tag == 1 {
                    ensure!(!found_zip64, "Duplicate ZIP64 extra field");
                    found_zip64 = true;
                    let data = &original_extra[extra_cursor + 4..end];
                    let size_bytes = (large_size as usize + large_compressed as usize) * 8;
                    ensure!(
                        data.len()
                            >= size_bytes + large_offset as usize * 8 + large_disk as usize * 4,
                        "Missing ZIP64 values"
                    );
                    sizes.extend_from_slice(&data[..size_bytes]);
                    if large_compressed { compressed = u64_at(data, large_size as usize * 8); }
                    if large_offset {
                        offset = u64_at(data, size_bytes);
                    }
                    if large_disk {
                        disk = u32_at(data, size_bytes + large_offset as usize * 8);
                    }
                } else {
                    extra.extend_from_slice(&original_extra[extra_cursor..end]);
                }
                extra_cursor = end;
            }
            ensure!(
                !(large_size || large_compressed || large_offset || large_disk) || found_zip64,
                "Missing ZIP64 extra field"
            );
            let entry_offset = (|| -> Result<u64> {
                let offset = layout.absolute(disk, offset)?;
                ensure!(offset < directory_start, "ZIP entry points outside payload");
                if partial {
                    let available = layout.contiguous_end(disk)?.min(directory_start);
                    ensure!(offset.checked_add(30).is_some_and(|end| end <= available), "文件头涉及缺失分卷或被截断");
                    let local = read_at(&mut source, offset, 30)?;
                    ensure!(&local[..4] == b"PK\x03\x04", "文件头损坏");
                    ensure!(u16_at(&local, 6) == u16_at(&header, 8)
                        && u16_at(&local, 8) == u16_at(&header, 10), "文件头与目录记录不一致");
                    let name_size = u16_at(&local, 26) as u64;
                    let data_start = offset + 30 + name_size + u16_at(&local, 28) as u64;
                    ensure!(data_start.checked_add(compressed).is_some_and(|end| end <= available),
                        "内容涉及缺失分卷或被截断，请补齐原卷后重试");
                    let name = read_at(&mut source, offset + 30, name_size as usize)?;
                    ensure!(name == original[cursor + 46..cursor + 46 + name_len], "文件名与目录记录不一致");
                }
                Ok(offset)
            })();
            let offset = match entry_offset {
                Ok(offset) => offset,
                Err(error) if partial => {
                    let name = String::from_utf8_lossy(&original[cursor + 46..cursor + 46 + name_len]);
                    recovery_notes.push(format!("{name}：{error}，未恢复"));
                    // Only name a missing-volume cause when a referenced disk,
                    // header or declared payload actually crosses a known gap.
                    let missing = layout.indices.get(disk as usize).is_some_and(Option::is_none)
                        || layout.absolute(disk, offset).ok().is_some_and(|start| {
                            let available = layout.contiguous_end(disk).unwrap_or(0);
                            available < directory_start && (start.checked_add(30).is_some_and(|end| end > available)
                                || read_at(&mut source, start, 30).ok().is_some_and(|local| {
                                &local[..4] == b"PK\x03\x04" && start.checked_add(30 + u16_at(&local, 26) as u64 + u16_at(&local, 28) as u64)
                                    .and_then(|data| data.checked_add(compressed)).is_some_and(|end| end > available)
                            }))
                        });
                    if !name.ends_with(['/', '\\']) {
                        recovery_entries.push(RecoveryEntryResult::new(Some(name.into_owned()),
                            if missing { RecoveryStatus::Missing } else { RecoveryStatus::Unknown }, error.to_string()));
                    }
                    cursor = entry_end;
                    continue;
                }
                Err(error) => return Err(error),
            };
            // Use ZIP64 offsets for every entry, including >4 GiB aggregate disk sets.
            header[34..36].copy_from_slice(&0u16.to_le_bytes());
            header[42..46].copy_from_slice(&u32::MAX.to_le_bytes());
            let version_needed = u16_at(&header, 6).max(45);
            header[6..8].copy_from_slice(&version_needed.to_le_bytes());
            extra.extend_from_slice(&1u16.to_le_bytes());
            extra.extend_from_slice(&((sizes.len() + 8) as u16).to_le_bytes());
            extra.extend_from_slice(&sizes);
            extra.extend_from_slice(&offset.to_le_bytes());
            header[30..32].copy_from_slice(
                &u16::try_from(extra.len())
                    .context("ZIP extra field too long")?
                    .to_le_bytes(),
            );
            directory.extend_from_slice(&header);
            directory.extend_from_slice(&original[cursor + 46..cursor + 46 + name_len]);
            directory.extend_from_slice(&extra);
            directory.extend_from_slice(&original[entry_end - comment_len..entry_end]);
            cursor = entry_end;
            retained += 1;
        }
        ensure!(!partial || retained > 0, "现有卷中未找到可完整读取的条目，请补齐所缺分卷后重试");
        let count = retained;
        let size = directory.len() as u64;
        let zip64_position = directory_start + size;
        directory.extend_from_slice(b"PK\x06\x06");
        directory.extend_from_slice(&44u64.to_le_bytes());
        directory.extend_from_slice(&45u16.to_le_bytes());
        directory.extend_from_slice(&45u16.to_le_bytes());
        directory.extend_from_slice(&0u32.to_le_bytes());
        directory.extend_from_slice(&0u32.to_le_bytes());
        directory.extend_from_slice(&count.to_le_bytes());
        directory.extend_from_slice(&count.to_le_bytes());
        directory.extend_from_slice(&size.to_le_bytes());
        directory.extend_from_slice(&directory_start.to_le_bytes());
        directory.extend_from_slice(b"PK\x06\x07");
        directory.extend_from_slice(&0u32.to_le_bytes());
        directory.extend_from_slice(&zip64_position.to_le_bytes());
        directory.extend_from_slice(&1u32.to_le_bytes());
        directory.extend_from_slice(b"PK\x05\x06");
        directory.extend_from_slice(&0u32.to_le_bytes());
        directory.extend_from_slice(&u16::MAX.to_le_bytes());
        directory.extend_from_slice(&u16::MAX.to_le_bytes());
        directory.extend_from_slice(&u32::MAX.to_le_bytes());
        directory.extend_from_slice(&u32::MAX.to_le_bytes());
        directory.extend_from_slice(&footer[20..]);
        Ok(Some(Self {
            source,
            directory_start,
            directory,
            position: 0,
            recovery_notes,
            recovery_encrypted,
            recovery_entries,
        }))
    }
}

impl Read for StandardZipReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let length = self.directory_start + self.directory.len() as u64;
        if buffer.is_empty() || self.position >= length {
            return Ok(0);
        }
        let count = if self.position < self.directory_start {
            self.source.seek(SeekFrom::Start(self.position))?;
            let limit = (self.directory_start - self.position).min(buffer.len() as u64) as usize;
            self.source.read(&mut buffer[..limit])?
        } else {
            let index = (self.position - self.directory_start) as usize;
            let count = buffer.len().min(self.directory.len() - index);
            buffer[..count].copy_from_slice(&self.directory[index..index + count]);
            count
        };
        self.position += count as u64;
        Ok(count)
    }
}
impl Seek for StandardZipReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let length = self.directory_start + self.directory.len() as u64;
        let value = match position {
            SeekFrom::Start(value) => value as i128,
            SeekFrom::End(delta) => length as i128 + delta as i128,
            SeekFrom::Current(delta) => self.position as i128 + delta as i128,
        };
        self.position = u64::try_from(value)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid ZIP seek"))?;
        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn zip64_1515_corrupt_end_record_is_a_final_structural_cause() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.zip");
        let output = temp.path().join("用户收到的大包.zip");
        let mut writer = zip_aes::ZipWriter::new(std::fs::File::create(&source).unwrap());
        writer.start_file("内容.bin", zip_aes::write::SimpleFileOptions::default()
            .compression_method(zip_aes::CompressionMethod::Stored)).unwrap();
        writer.write_all(&vec![73; 170_003]).unwrap();
        writer.finish().unwrap();
        super::super::standard_zip_writer::split(&source, &output, 65536, || Ok(()), |_, _| {}).unwrap();
        let mut bytes = std::fs::read(&output).unwrap();
        let at = bytes.windows(4).position(|b| b == b"PK\x06\x06").unwrap();
        bytes[at] ^= 1;
        std::fs::write(&output, bytes).unwrap();
        let error = match super::StandardZipReader::try_open(&output) {
            Err(error) => error,
            Ok(_) => panic!("A changed ZIP64 end signature must not be accepted"),
        };
        assert!(error.to_string().contains("[archive-inspection:damaged]"), "{error:#}");
        assert!(error.to_string().contains("ZIP64"), "{error:#}");
    }

    use super::*;
    use crate::services::{
        archive_browser::browse_archive, archive_engine::ArchiveEngine,
        archive_preview::preview_archive_text, universal_engine::UniversalCliEngine,
    };
    use std::sync::{atomic::AtomicBool, Arc};
    use std::{fs::File, io::Write};
    use zip_aes::unstable::write::FileOptionsExt;

    // A standards-based disk fixture, independently read by bundled 7-Zip.
    // Central directory begins on disk 1 and crosses into the terminal disk.
    fn split(source: &Path, main: &Path, zip64: bool) -> Result<()> {
        let bytes = std::fs::read(source)?;
        let eocd = bytes.len() - 22;
        let central = u32_at(&bytes, eocd + 16) as usize;
        let cut = 8192;
        let second_cut = central + 55; // split central metadata across disks
        let mut joined = b"PK\x07\x08".to_vec();
        joined.extend_from_slice(&bytes[..eocd]);
        let boundaries = [0, cut + 4, second_cut + 4];
        let mut cursor = central + 4;
        while cursor < eocd + 4 {
            let offset = u32_at(&joined, cursor + 42) as usize + 4;
            let disk = boundaries.partition_point(|start| *start <= offset) - 1;
            joined[cursor + 34..cursor + 36].copy_from_slice(&(disk as u16).to_le_bytes());
            joined[cursor + 42..cursor + 46]
                .copy_from_slice(&((offset - boundaries[disk]) as u32).to_le_bytes());
            cursor += 46
                + u16_at(&joined, cursor + 28) as usize
                + u16_at(&joined, cursor + 30) as usize
                + u16_at(&joined, cursor + 32) as usize;
        }
        let cd = central + 4 - boundaries[1];
        if zip64 {
            let zip64_position = joined.len() - boundaries[2];
            joined.extend_from_slice(b"PK\x06\x06");
            joined.extend_from_slice(&44u64.to_le_bytes());
            joined.extend_from_slice(&45u16.to_le_bytes());
            joined.extend_from_slice(&45u16.to_le_bytes());
            joined.extend_from_slice(&2u32.to_le_bytes());
            joined.extend_from_slice(&1u32.to_le_bytes());
            joined.extend_from_slice(&1u64.to_le_bytes());
            joined.extend_from_slice(&(u16_at(&bytes, eocd + 10) as u64).to_le_bytes());
            joined.extend_from_slice(&(u32_at(&bytes, eocd + 12) as u64).to_le_bytes());
            joined.extend_from_slice(&(cd as u64).to_le_bytes());
            joined.extend_from_slice(b"PK\x06\x07");
            joined.extend_from_slice(&2u32.to_le_bytes());
            joined.extend_from_slice(&(zip64_position as u64).to_le_bytes());
            joined.extend_from_slice(&3u32.to_le_bytes());
        }
        let mut footer = bytes[eocd..].to_vec();
        footer[4..6].copy_from_slice(&2u16.to_le_bytes());
        footer[6..8].copy_from_slice(&1u16.to_le_bytes());
        footer[8..10].copy_from_slice(&1u16.to_le_bytes());
        footer[16..20].copy_from_slice(&(cd as u32).to_le_bytes());
        if zip64 {
            footer[10..12].copy_from_slice(&u16::MAX.to_le_bytes());
            footer[12..20].fill(255);
        }
        joined.extend_from_slice(&footer);
        std::fs::write(main.with_extension("z01"), &joined[..boundaries[1]])?;
        std::fs::write(
            main.with_extension("z02"),
            &joined[boundaries[1]..boundaries[2]],
        )?;
        std::fs::write(main, &joined[boundaries[2]..])?;
        Ok(())
    }

    // Five files: one before the hole, one crossing it, one inside it,
    // one after it, and one spanning two consecutive surviving disks.
    fn partial_fixture(root: &Path, mode: &str) -> (std::path::PathBuf, Vec<Vec<u8>>) {
        let source = root.join("original.zip");
        let main = root.join("待救资料.zip");
        let payloads = (0..5).map(|index| format!("中文文件 {index} 的完整内容\n").repeat(100).into_bytes()).collect::<Vec<_>>();
        let mut writer = zip_aes::ZipWriter::new(File::create(&source).unwrap());
        let options = zip_aes::write::SimpleFileOptions::default().compression_method(zip_aes::CompressionMethod::Stored);
        let options = match mode {
            "aes" => options.with_aes_encryption(zip_aes::AesMode::Aes256, "rescue-password"),
            "zipcrypto" => options.with_deprecated_encryption(b"rescue-password").unwrap(),
            _ => options,
        };
        for (index, data) in payloads.iter().enumerate() {
            writer.start_file(format!("中文目录/文件{index}.txt"), options).unwrap();
            writer.write_all(data).unwrap();
        }
        writer.add_directory("中文目录/空目录/", zip_aes::write::SimpleFileOptions::default()).unwrap();
        writer.finish().unwrap();
        let bytes = std::fs::read(source).unwrap();
        let eocd = bytes.len() - 22;
        let central = u32_at(&bytes, eocd + 16) as usize;
        let mut offsets = Vec::new();
        let mut cursor = central;
        while cursor < eocd {
            offsets.push(u32_at(&bytes, cursor + 42) as usize + 4);
            cursor += 46 + u16_at(&bytes, cursor + 28) as usize + u16_at(&bytes, cursor + 30) as usize + u16_at(&bytes, cursor + 32) as usize;
        }
        let boundaries = [0, offsets[1] + 100, offsets[3], offsets[4] + 100];
        let mut joined = b"PK\x07\x08".to_vec();
        joined.extend_from_slice(&bytes[..eocd]);
        cursor = central + 4;
        for offset in offsets {
            let disk = boundaries.partition_point(|start| *start <= offset) - 1;
            joined[cursor + 34..cursor + 36].copy_from_slice(&(disk as u16).to_le_bytes());
            joined[cursor + 42..cursor + 46].copy_from_slice(&((offset - boundaries[disk]) as u32).to_le_bytes());
            cursor += 46 + u16_at(&joined, cursor + 28) as usize + u16_at(&joined, cursor + 30) as usize + u16_at(&joined, cursor + 32) as usize;
        }
        let cd = central + 4 - boundaries[3];
        if mode == "zip64" {
            let record = joined.len() - boundaries[3];
            joined.extend_from_slice(b"PK\x06\x06"); joined.extend_from_slice(&44u64.to_le_bytes());
            joined.extend_from_slice(&45u16.to_le_bytes()); joined.extend_from_slice(&45u16.to_le_bytes());
            joined.extend_from_slice(&3u32.to_le_bytes()); joined.extend_from_slice(&3u32.to_le_bytes());
            joined.extend_from_slice(&6u64.to_le_bytes()); joined.extend_from_slice(&6u64.to_le_bytes());
            joined.extend_from_slice(&(u32_at(&bytes, eocd + 12) as u64).to_le_bytes()); joined.extend_from_slice(&(cd as u64).to_le_bytes());
            joined.extend_from_slice(b"PK\x06\x07"); joined.extend_from_slice(&3u32.to_le_bytes());
            joined.extend_from_slice(&(record as u64).to_le_bytes()); joined.extend_from_slice(&4u32.to_le_bytes());
        }
        let mut footer = bytes[eocd..].to_vec();
        footer[4..6].copy_from_slice(&3u16.to_le_bytes()); footer[6..8].copy_from_slice(&3u16.to_le_bytes());
        footer[16..20].copy_from_slice(&(cd as u32).to_le_bytes());
        if mode == "zip64" { footer[10..20].fill(255); }
        joined.extend_from_slice(&footer);
        for index in 0..3 { std::fs::write(main.with_extension(format!("z{:02}", index + 1)), &joined[boundaries[index]..boundaries[index + 1]]).unwrap(); }
        std::fs::write(&main, &joined[boundaries[3]..]).unwrap();
        (main, payloads)
    }

    #[tokio::test]
    async fn partial_zip_recovery_preserves_files_on_both_sides_of_missing_disks_and_passwords() {
        use crate::services::archive_diagnostics::{diagnose_archive, repair_zip_with_password_to_new};
        for mode in ["plain", "aes", "zipcrypto", "zip64"] {
            for missing in ["z01", "z02"] {
                let root = tempfile::tempdir().unwrap();
                let (main, payloads) = partial_fixture(root.path(), mode);
                std::fs::remove_file(main.with_extension(missing)).unwrap();
                let before = std::fs::read(&main).unwrap();
                let password = matches!(mode, "aes" | "zipcrypto").then_some("rescue-password");
                assert!(StandardZipReader::try_open(&main).is_err(), "normal extraction must still require the full set");
                let report = diagnose_archive(&main.with_extension("z03"), password, Arc::new(AtomicBool::new(false))).await.unwrap();
                assert_eq!(report.status, "partial_recovery", "{mode} {missing}: {report:?}");
                assert!(report.can_repair && !report.integrity_tested);
                assert_eq!(report.total_files, 3);
                if password.is_some() {
                    let required = diagnose_archive(&main, None, Arc::new(AtomicBool::new(false))).await.unwrap();
                    assert_eq!(required.status, "password_required"); assert!(!required.can_repair);
                    let wrong = root.path().join("wrong.zip");
                    assert!(repair_zip_with_password_to_new(&main, &wrong, Some("wrong"), &AtomicBool::new(false)).is_err());
                    assert!(!wrong.exists());
                }
                let output = root.path().join("recovered.zip");
                let result = repair_zip_with_password_to_new(&main, &output, password, &AtomicBool::new(false)).unwrap();
                assert!(result.partial && result.verified && !result.directory_rebuilt);
                assert_eq!(result.output_encrypted, password.is_some()); assert_eq!(result.recovered_files, 3);
                assert!(result.skipped_entries.iter().any(|note| note.contains("文件1.txt")));
                let mut zip = zip_aes::ZipArchive::new(File::open(output).unwrap()).unwrap();
                for index in if missing == "z02" { vec![0, 3, 4] } else { vec![2, 3, 4] } {
                    let name = format!("中文目录/文件{index}.txt");
                    let mut data = Vec::new();
                    match password {
                        Some(password) => zip.by_name_decrypt(&name, password.as_bytes()).unwrap().read_to_end(&mut data).unwrap(),
                        None => zip.by_name(&name).unwrap().read_to_end(&mut data).unwrap(),
                    };
                    assert_eq!(data, payloads[index]);
                }
                assert_eq!(std::fs::read(&main).unwrap(), before);
            }
        }
    }

    #[tokio::test]
    async fn partial_zip_recovery_rejects_missing_directory_and_skips_corrupt_content() {
        use crate::services::archive_diagnostics::{diagnose_archive, repair_zip_with_password_to_new};
        let root = tempfile::tempdir().unwrap();
        let (main, _) = partial_fixture(root.path(), "plain");
        std::fs::remove_file(main.with_extension("z02")).unwrap();
        let part = main.with_extension("z03");
        let mut bytes = std::fs::read(&part).unwrap(); bytes[100] ^= 0x40; std::fs::write(&part, bytes).unwrap();
        let result = repair_zip_with_password_to_new(&main, &root.path().join("partial.zip"), None, &AtomicBool::new(false)).unwrap();
        assert_eq!(result.recovered_files, 2);
        assert!(result.skipped_entries.iter().any(|note| note.contains("文件3.txt") && note.contains("校验")));
        assert!(result.entry_results.iter().any(|entry| entry.name.as_deref() == Some("中文目录/文件3.txt")
            && entry.status == RecoveryStatus::ChecksumFailed));
        assert!(result.entry_results.iter().any(|entry| entry.name.as_deref() == Some("中文目录/文件1.txt")
            && entry.status == RecoveryStatus::Missing));
        std::fs::remove_file(&main).unwrap();
        let report = diagnose_archive(&part, None, Arc::new(AtomicBool::new(false))).await.unwrap();
        assert_eq!(report.status, "missing_volume"); assert!(!report.can_repair);
        assert!(repair_zip_with_password_to_new(&part, &root.path().join("unavailable.zip"), None, &AtomicBool::new(false)).is_err());
        assert!(!root.path().join("unavailable.zip").exists());
    }

    #[tokio::test]
    async fn standard_zip_disks_browse_preview_passwords_and_zip64_roundtrip() {
        let temp = tempfile::tempdir().unwrap();
        let text = "收到几个分卷，我要查看说明然后解压。\n".repeat(700);
        for mode in ["plain", "zipcrypto", "aes", "zip64"] {
            let original = temp.path().join(format!("original-{mode}.zip"));
            let main = temp.path().join(format!("下载资料-{mode}.zip"));
            let mut writer = zip_aes::ZipWriter::new(File::create(&original).unwrap());
            let options = zip_aes::write::SimpleFileOptions::default()
                .compression_method(zip_aes::CompressionMethod::Stored);
            let options = match mode {
                "zipcrypto" => options
                    .with_deprecated_encryption(b"sample-password")
                    .unwrap(),
                "aes" => options.with_aes_encryption(zip_aes::AesMode::Aes256, "sample-password"),
                _ => options,
            };
            writer.start_file("中文目录/说明.txt", options).unwrap();
            writer.write_all(text.as_bytes()).unwrap();
            writer
                .add_directory(
                    "中文目录/空目录/",
                    zip_aes::write::SimpleFileOptions::default(),
                )
                .unwrap();
            writer.finish().unwrap();
            split(&original, &main, mode == "zip64").unwrap();
            let password = matches!(mode, "aes" | "zipcrypto").then_some("sample-password");
            for selected in [
                &main,
                &main.with_extension("z01"),
                &main.with_extension("z02"),
            ] {
                let metadata = browse_archive(selected, password).await.unwrap();
                assert_eq!(metadata.total_files, 1);
                assert_eq!(metadata.total_directories, 1);
                assert_eq!(metadata.encrypted, password.is_some());
                let preview = preview_archive_text(selected, "中文目录/说明.txt", password)
                    .await
                    .unwrap();
                assert_eq!(preview.content, text);
                assert_eq!(
                    UniversalCliEngine::zip_requires_password(selected).unwrap(),
                    password.is_some()
                );
                if password.is_some() {
                    assert!(
                        UniversalCliEngine::try_zip_password(selected, "sample-password").unwrap()
                    );
                    assert!(!UniversalCliEngine::try_zip_password(selected, "wrong").unwrap());
                }
            }
            let output = temp.path().join(format!("output-{mode}"));
            // Read through the same native ZIP password route even for plain fixtures.
            UniversalCliEngine::new()
                .extract_with_progress(
                    &main,
                    &output,
                    Some(password.unwrap_or("unused")),
                    false,
                    Arc::new(|_| {}),
                    Arc::new(|_, _| {}),
                    Arc::new(AtomicBool::new(false)),
                )
                .await
                .unwrap();
            assert_eq!(
                std::fs::read_to_string(output.join("中文目录/说明.txt")).unwrap(),
                text
            );
            assert!(output.join("中文目录/空目录").is_dir());
            // Removing the last .zNN must be detected even though all earlier names are continuous.
            std::fs::remove_file(main.with_extension("z02")).unwrap();
            let group = SplitArchiveDetector::detect_split_archive(&main)
                .unwrap()
                .unwrap();
            assert!(!group.is_complete);
            assert!(group.missing_parts.contains(&main.with_extension("z02")));
            assert!(StandardZipReader::try_open(&main)
                .err()
                .unwrap()
                .to_string()
                .contains("缺少 ZIP 分卷"));
        }
    }

    #[tokio::test]
    async fn forum_zip_layout_regression_preserves_passwords_and_native_offsets() {
        // Renamed standards-based fixtures test the shared reader only. They
        // are deliberately NOT evidence of HaoZip-generated encrypted output.
        let temp = tempfile::tempdir().unwrap();
        for mode in ["plain", "zipcrypto", "aes", "zip64"] {
            let source = temp.path().join(format!("source-{mode}.zip"));
            let main = temp.path().join(format!("layout-{mode}.zip"));
            let mut writer = zip_aes::ZipWriter::new(File::create(&source).unwrap());
            let options = zip_aes::write::SimpleFileOptions::default().compression_method(zip_aes::CompressionMethod::Stored);
            let options = match mode {
                "zipcrypto" => options.with_deprecated_encryption(b"forum-password").unwrap(),
                "aes" => options.with_aes_encryption(zip_aes::AesMode::Aes256, "forum-password"),
                _ => options,
            };
            let text = "请从任意一卷打开，再把资料完整解压。\n".repeat(700);
            writer.start_file("中文说明.txt", options).unwrap();
            writer.write_all(text.as_bytes()).unwrap(); writer.finish().unwrap();
            split(&source, &main, mode == "zip64").unwrap();
            let members = [main.with_extension("z01"), main.with_extension("z02"), main.clone()];
            let forum = (1..=3).map(|index| temp.path().join(format!("资料-{mode}.haozip{index:02}.zip"))).collect::<Vec<_>>();
            for (old, new) in members.iter().zip(&forum) { std::fs::rename(old, new).unwrap(); }
            let password = matches!(mode, "aes" | "zipcrypto").then_some("forum-password");
            for member in &forum {
                let result = browse_archive(member, password).await.unwrap();
                assert_eq!(result.total_files, 1);
                assert_eq!(preview_archive_text(member, "中文说明.txt", password).await.unwrap().content, text);
                assert_eq!(UniversalCliEngine::zip_requires_password(member).unwrap(), password.is_some());
                if password.is_some() { assert!(!UniversalCliEngine::try_zip_password(member, "wrong").unwrap()); }
            }
            let output = temp.path().join(format!("output-{mode}"));
            UniversalCliEngine::new().extract_with_progress(&forum[2], &output, Some(password.unwrap_or("unused")), false,
                Arc::new(|_| {}), Arc::new(|_, _| {}), Arc::new(AtomicBool::new(false))).await.unwrap();
            assert_eq!(std::fs::read_to_string(output.join("中文说明.txt")).unwrap(), text);
        }
    }

    #[tokio::test]
    #[ignore = "Requires the genuine public HaoZip fixture; see v1.4.11 audit source URLs"]
    async fn forum_zip_public_fixture_reads_outer_epub_not_embedded_directory() {
        use sha2::{Digest, Sha256};
        let root = std::path::PathBuf::from(std::env::var("LONG_HAOZIP_FIXTURES").unwrap());
        let temp = tempfile::tempdir().unwrap();
        for index in 1..=2 {
            let member = root.join(format!("图解计算机基础（小林）.haozip{index:02}.zip"));
            let before = std::fs::read(&member).unwrap();
            let metadata = browse_archive(&member, None).await.unwrap();
            assert_eq!(metadata.total_files, 1, "must read outer archive, not 1635 embedded EPUB entries");
            assert!(!metadata.encrypted);
            let output = temp.path().join(index.to_string());
            UniversalCliEngine::new().extract_with_progress(&member, &output, Some("unused"), false,
                Arc::new(|_| {}), Arc::new(|_, _| {}), Arc::new(AtomicBool::new(false))).await.unwrap();
            let data = std::fs::read(output.join("图解计算机基础（小林）.epub")).unwrap();
            assert_eq!(data.len(), 32_607_587);
            assert_eq!(format!("{:x}", Sha256::digest(data)), "151a4929baf7f7d0e0626c3d865e26882ee45de330d868b6f26422df0c505900");
            assert_eq!(std::fs::read(member).unwrap(), before);
        }
    }

    #[tokio::test]
    #[ignore = "Requires HaoZipC.exe via LONG_HAOZIP_C; native standard ZIP disks, not forum-mode evidence"]
    async fn haozip_cli_zipcrypto_disks_preserve_passwords_and_payloads() {
        // Let the external encoder create its own headers, disk offsets and
        // encryption. Do not rename its standard .zNN/.zip output to forum names.
        let encoder = std::path::PathBuf::from(std::env::var("LONG_HAOZIP_C").unwrap());
        assert!(encoder.is_file(), "LONG_HAOZIP_C must name the actual HaoZip CLI");
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("原始资料");
        std::fs::create_dir_all(source.join("中文目录/空目录")).unwrap();
        let text = "收到分卷后，先输错密码，再纠正并完整解压。\n";
        let payload = (0..163_871).map(|index| (index % 251) as u8).collect::<Vec<_>>();
        std::fs::write(source.join("说明.txt"), text).unwrap();
        std::fs::write(source.join("中文目录/原文件.bin"), &payload).unwrap();
        let main = temp.path().join("好压原生密码卷.zip");
        let password = "HaoZip sample + !";
        let mut command = std::process::Command::new(encoder);
        command.current_dir(&source).args(["a", "-tzip", "-mx=0", "-v64k", "-mem=ZipCrypto", "-y"])
            .arg(format!("-p{password}")).arg(&main).arg("*");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let encoded = command.output().unwrap();
        assert!(encoded.status.success(), "HaoZip must finish before its files are accepted: {:?}", encoded.status);
        if let Some(validator) = std::env::var_os("LONG_BANDIZIP_C") {
            let mut verify = std::process::Command::new(validator);
            verify.args(["t", &format!("-p:{password}")]).arg(&main);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                verify.creation_flags(0x08000000);
            }
            let verified = verify.output().unwrap();
            assert!(verified.status.success(), "Independent Bandizip check failed: {:?}", verified.status);
            eprintln!("Independent Bandizip verified all generated payloads");
        }
        let members = [main.with_extension("z01"), main.with_extension("z02"), main.clone()];
        let original = members.iter().map(std::fs::read).collect::<std::io::Result<Vec<_>>>().unwrap();
        for (index, member) in members.iter().enumerate() {
            let group = SplitArchiveDetector::detect_split_archive(member).unwrap().unwrap();
            assert!(group.is_complete);
            assert_eq!(group.total_parts, 3);
            assert_eq!(group.parts, members);
            assert!(UniversalCliEngine::zip_requires_password(member).unwrap());
            assert!(!UniversalCliEngine::try_zip_password(member, "wrong password").is_ok_and(|accepted| accepted),
                "A short header collision must not accept a wrong password");
            assert!(UniversalCliEngine::try_zip_password(member, password).unwrap());
            let metadata = browse_archive(member, Some(password)).await.unwrap();
            assert_eq!(metadata.total_files, 2);
            assert!(metadata.encrypted);
            assert_eq!(preview_archive_text(member, "说明.txt", Some(password)).await.unwrap().content, text);
            let output = temp.path().join(format!("从第{}卷解压", index + 1));
            UniversalCliEngine::new().extract_with_progress(member, &output, Some(password), false,
                Arc::new(|_| {}), Arc::new(|_, _| {}), Arc::new(AtomicBool::new(false))).await.unwrap();
            assert_eq!(std::fs::read_to_string(output.join("说明.txt")).unwrap(), text);
            assert_eq!(std::fs::read(output.join("中文目录/原文件.bin")).unwrap(), payload);
            assert!(output.join("中文目录/空目录").is_dir());
        }
        for (member, before) in members.iter().zip(original) {
            assert_eq!(std::fs::read(member).unwrap(), before);
        }
    }

    #[tokio::test]
    #[ignore = "Requires externally generated standard ZIP disks"]
    async fn standard_zip_external_winrar_disks_read_exact_payloads() {
        let root = std::path::PathBuf::from(std::env::var("LONG_STANDARD_ZIP_FIXTURES").unwrap());
        let temp = tempfile::tempdir().unwrap();
        for mode in ["plain", "zipcrypto", "aes", "zip64"] {
            let main = root.join(format!("下载资料-{mode}.zip"));
            let password = matches!(mode, "zipcrypto" | "aes").then_some("ZIP sample + !");
            for selected in [&main, &main.with_extension("z01")] {
                let result = browse_archive(selected, password).await.unwrap();
                assert_eq!(result.total_files, 3);
                let preview = preview_archive_text(selected, "说明.txt", password)
                    .await
                    .unwrap();
                assert_eq!(
                    preview.content,
                    std::fs::read_to_string(root.join("source/说明.txt")).unwrap()
                );
                let image = crate::services::archive_preview::preview_archive_image(
                    selected,
                    "像素.png",
                    password,
                )
                .await
                .unwrap();
                assert_eq!((image.width, image.height), (1, 1));
                assert_eq!(
                    UniversalCliEngine::new()
                        .requires_password(selected)
                        .await
                        .unwrap(),
                    password.is_some()
                );
            }
            let output = temp.path().join(mode);
            UniversalCliEngine::new()
                .extract_with_progress(
                    &main,
                    &output,
                    password,
                    false,
                    Arc::new(|_| {}),
                    Arc::new(|_, _| {}),
                    Arc::new(AtomicBool::new(false)),
                )
                .await
                .unwrap();
            assert_eq!(
                std::fs::read(output.join("中文目录/原文件.bin")).unwrap(),
                std::fs::read(root.join("source/中文目录/原文件.bin")).unwrap()
            );
            assert!(output.join("中文目录/空目录").is_dir());
        }
    }
}
