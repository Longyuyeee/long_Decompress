//! Turn a complete ZIP into real .z01/.../.zip disks. Local/central headers
//! stay on one disk; payload streams across disks. Encryption is not changed.
use super::split_compression::SplitCompressionResult;
use anyhow::{ensure, Context, Result};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ZipSplitMode {
    #[default]
    Numeric,
    Standard,
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
}
fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}
fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap())
}
fn read_at(file: &mut File, at: u64, len: usize) -> Result<Vec<u8>> {
    file.seek(SeekFrom::Start(at))?;
    let mut bytes = vec![0; len];
    file.read_exact(&mut bytes)?;
    Ok(bytes)
}

struct Directory {
    start: u64,
    records: Vec<Vec<u8>>,
    comment: Vec<u8>,
}
fn directory(file: &mut File) -> Result<Directory> {
    let length = file.metadata()?.len();
    let tail_start = length.saturating_sub(65_557);
    let tail = read_at(file, tail_start, (length - tail_start) as usize)?;
    let end = (0..tail.len().saturating_sub(21))
        .rev()
        .find(|&at| {
            &tail[at..at + 4] == b"PK\x05\x06"
                && at + 22 + u16_at(&tail, at + 20) as usize == tail.len()
        })
        .context("ZIP 目录不完整，不能创建标准分卷")?;
    ensure!(
        u16_at(&tail, end + 4) == 0 && u16_at(&tail, end + 6) == 0,
        "标准分卷创建需要一个完整的单盘 ZIP"
    );
    let mut count = u16_at(&tail, end + 10) as u64;
    let mut size = u32_at(&tail, end + 12) as u64;
    let mut start = u32_at(&tail, end + 16) as u64;
    let end_absolute = tail_start + end as u64;
    if end_absolute >= 20 {
        let locator = read_at(file, end_absolute - 20, 20)?;
        if &locator[..4] == b"PK\x06\x07" {
            ensure!(
                u32_at(&locator, 4) == 0 && u32_at(&locator, 16) == 1,
                "ZIP64 输入不是单盘归档"
            );
            let record = read_at(file, u64_at(&locator, 8), 56)?;
            ensure!(
                &record[..4] == b"PK\x06\x06" && u64_at(&record, 4) >= 44,
                "ZIP64 目录记录损坏"
            );
            count = u64_at(&record, 32);
            size = u64_at(&record, 40);
            start = u64_at(&record, 48);
        }
    }
    ensure!(
        size <= 256 * 1024 * 1024
            && start
                .checked_add(size)
                .is_some_and(|value| value <= end_absolute),
        "ZIP 目录范围无效或超过元数据读取上限"
    );
    let bytes = read_at(file, start, size as usize)?;
    let mut cursor = 0;
    let mut records = Vec::new();
    for _ in 0..count {
        ensure!(
            cursor + 46 <= bytes.len() && &bytes[cursor..cursor + 4] == b"PK\x01\x02",
            "ZIP 中央目录条目损坏"
        );
        let len = 46
            + u16_at(&bytes, cursor + 28) as usize
            + u16_at(&bytes, cursor + 30) as usize
            + u16_at(&bytes, cursor + 32) as usize;
        ensure!(cursor + len <= bytes.len(), "ZIP 中央目录条目被截断");
        records.push(bytes[cursor..cursor + len].to_vec());
        cursor += len;
    }
    ensure!(cursor == bytes.len(), "ZIP 目录含有无法重新分卷的额外记录");
    Ok(Directory {
        start,
        records,
        comment: tail[end + 22..].to_vec(),
    })
}

// Preserve size values and every non-ZIP64 extra (including AES metadata).
fn extra_parts(record: &[u8]) -> Result<(Vec<u8>, Vec<u8>, u64)> {
    let name = u16_at(record, 28) as usize;
    let len = u16_at(record, 30) as usize;
    let bytes = &record[46 + name..46 + name + len];
    let sizes = (usize::from(u32_at(record, 24) == u32::MAX)
        + usize::from(u32_at(record, 20) == u32::MAX))
        * 8;
    let large_offset = u32_at(record, 42) == u32::MAX;
    let mut offset = u32_at(record, 42) as u64;
    let mut preserved = Vec::new();
    let mut size_values = Vec::new();
    let mut found = false;
    let mut cursor = 0;
    while cursor < bytes.len() {
        ensure!(cursor + 4 <= bytes.len(), "ZIP 扩展字段被截断");
        let end = cursor + 4 + u16_at(bytes, cursor + 2) as usize;
        ensure!(end <= bytes.len(), "ZIP 扩展字段长度无效");
        if u16_at(bytes, cursor) == 1 {
            ensure!(
                !found && end - cursor >= 4 + sizes + usize::from(large_offset) * 8,
                "ZIP64 扩展字段无效"
            );
            found = true;
            size_values.extend_from_slice(&bytes[cursor + 4..cursor + 4 + sizes]);
            if large_offset {
                offset = u64_at(bytes, cursor + 4 + sizes);
            }
        } else {
            preserved.extend_from_slice(&bytes[cursor..end]);
        }
        cursor = end;
    }
    ensure!((sizes == 0 && !large_offset) || found, "ZIP64 扩展字段缺失");
    Ok((preserved, size_values, offset))
}
fn relocate(record: &[u8], disk: u32, offset: u64) -> Result<Vec<u8>> {
    let (mut extra, mut large, _) = extra_parts(record)?;
    let mut header = record[..46].to_vec();
    header[34..36].copy_from_slice(&(disk.min(u16::MAX as u32) as u16).to_le_bytes());
    header[42..46].copy_from_slice(&(offset.min(u32::MAX as u64) as u32).to_le_bytes());
    if offset >= u32::MAX as u64 {
        large.extend_from_slice(&offset.to_le_bytes());
    }
    if disk >= u16::MAX as u32 {
        large.extend_from_slice(&disk.to_le_bytes());
    }
    if !large.is_empty() {
        extra.extend_from_slice(&1u16.to_le_bytes());
        extra.extend_from_slice(&(large.len() as u16).to_le_bytes());
        extra.extend_from_slice(&large);
        header[6..8].copy_from_slice(&u16_at(record, 6).max(45).to_le_bytes());
    }
    header[30..32].copy_from_slice(
        &u16::try_from(extra.len())
            .context("ZIP 扩展字段过长")?
            .to_le_bytes(),
    );
    let name_len = u16_at(record, 28) as usize;
    let comment = 46 + name_len + u16_at(record, 30) as usize;
    header.extend_from_slice(&record[46..46 + name_len]);
    header.extend_from_slice(&extra);
    header.extend_from_slice(&record[comment..]);
    Ok(header)
}

struct Disks {
    output: PathBuf,
    limit: u64,
    position: u64,
    current: Option<File>,
    paths: Vec<PathBuf>,
    committed: bool,
}
impl Drop for Disks {
    fn drop(&mut self) {
        self.current.take();
        if !self.committed {
            for path in &self.paths {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}
impl Disks {
    fn disk(&self) -> u32 {
        (self.paths.len() - 1) as u32
    }
    fn next(&mut self) -> Result<()> {
        if let Some(mut file) = self.current.take() {
            file.flush()?;
        }
        ensure!(
            self.paths.len() < 65_536,
            "标准 ZIP 分卷数量超过当前读取范围，请增大每卷大小"
        );
        let marker = if self.output.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("zipx")) { "zx" } else { "z" };
        let path = self.output.with_extension(format!("{marker}{:02}", self.paths.len() + 1));
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        self.paths.push(path);
        self.current = Some(file);
        self.position = 0;
        Ok(())
    }
    fn reserve(&mut self, len: usize) -> Result<()> {
        ensure!(
            len as u64 <= self.limit,
            "分卷大小不足以容纳 ZIP 头记录，请增大每卷大小"
        );
        if self.current.is_none() || self.position + len as u64 > self.limit {
            self.next()?;
        }
        Ok(())
    }
    fn record(&mut self, bytes: &[u8]) -> Result<(u32, u64)> {
        self.reserve(bytes.len())?;
        let location = (self.disk(), self.position);
        self.current.as_mut().unwrap().write_all(bytes)?;
        self.position += bytes.len() as u64;
        Ok(location)
    }
    fn payload(&mut self, mut bytes: &[u8]) -> Result<()> {
        while !bytes.is_empty() {
            if self.position == self.limit {
                self.next()?;
            }
            let len = (self.limit - self.position).min(bytes.len() as u64) as usize;
            self.current.as_mut().unwrap().write_all(&bytes[..len])?;
            self.position += len as u64;
            bytes = &bytes[len..];
        }
        Ok(())
    }
    fn finish(mut self) -> Result<SplitCompressionResult> {
        self.current.take().unwrap().sync_all()?;
        let last = self.paths.last().unwrap().clone();
        super::output_publish_transaction::transfer_no_replace(&last, &self.output)?;
        self.paths.push(self.output.clone());
        #[cfg(not(windows))]
        std::fs::remove_file(&last)?;
        let mut parts = self.paths[..self.paths.len() - 2].to_vec();
        parts.push(self.output.clone());
        let total_size = parts.iter().try_fold(0u64, |sum, path| -> Result<u64> {
            sum.checked_add(path.metadata()?.len())
                .context("ZIP 分卷大小溢出")
        })?;
        self.committed = true;
        Ok(SplitCompressionResult {
            part_count: parts.len(),
            part_files: parts,
            total_size,
        })
    }
}

pub(crate) fn split(
    source: &Path,
    output: &Path,
    limit: u64,
    mut check: impl FnMut() -> Result<()>,
    mut progress: impl FnMut(u64, u64),
) -> Result<SplitCompressionResult> {
    ensure!(
        (65_536..=u32::MAX as u64).contains(&limit),
        "标准 ZIP 每卷大小需要在 64 KiB 至 4 GiB（不含）之间"
    );
    ensure!(
        !output.exists() && super::archive_volumes::volume_paths(output)?.is_empty(),
        "ZIP 分卷输出已存在"
    );
    check()?;
    let mut file = File::open(source)?;
    let length = file.metadata()?.len();
    let required = length
        .checked_add(super::storage_preflight::DISK_SAFETY_RESERVE)
        .context("ZIP 分卷空间计算溢出")?;
    if let Some(free) =
        super::storage_preflight::available_disk_space(output.parent().unwrap_or(Path::new(".")))
    {
        ensure!(free >= required, "分卷前空间不足，尚未创建标准 ZIP 分卷");
    }
    let dir = directory(&mut file)?;
    // A small archive remains one ordinary ZIP, with no unnecessary z01 file.
    if length <= limit {
        let mut writer = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)?;
        let result = (|| -> Result<()> {
            file.seek(SeekFrom::Start(0))?;
            let mut copied = 0;
            let mut buffer = vec![0; 256 * 1024];
            loop {
                check()?;
                let read = file.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                writer.write_all(&buffer[..read])?;
                copied += read as u64;
                progress(copied, length);
            }
            writer.sync_all()?;
            check()?;
            Ok(())
        })();
        drop(writer);
        if let Err(error) = result {
            let _ = std::fs::remove_file(output);
            return Err(error);
        }
        return Ok(SplitCompressionResult {
            part_files: vec![output.to_path_buf()],
            total_size: length,
            part_count: 1,
        });
    }
    let mut disks = Disks {
        output: output.to_path_buf(),
        limit,
        position: 0,
        current: None,
        paths: Vec::new(),
        committed: false,
    };
    disks.record(b"PK\x07\x08")?;
    let mut order = dir
        .records
        .iter()
        .enumerate()
        .map(|(index, record)| Ok((extra_parts(record)?.2, index)))
        .collect::<Result<Vec<_>>>()?;
    order.sort_by_key(|entry| entry.0);
    ensure!(
        order.first().is_some_and(|entry| entry.0 == 0),
        "ZIP 含有自解压前缀或没有可分卷的本地条目"
    );
    let mut locations = vec![(0, 0); dir.records.len()];
    let mut buffer = vec![0; 256 * 1024];
    progress(0, length);
    for (position, &(offset, index)) in order.iter().enumerate() {
        check()?;
        let next = order
            .get(position + 1)
            .map(|entry| entry.0)
            .unwrap_or(dir.start);
        let header = read_at(&mut file, offset, 30)?;
        ensure!(&header[..4] == b"PK\x03\x04", "ZIP 本地头记录损坏");
        let header_len = 30 + u16_at(&header, 26) as usize + u16_at(&header, 28) as usize;
        ensure!(
            offset
                .checked_add(header_len as u64)
                .is_some_and(|end| end <= next),
            "ZIP 本地记录范围无效"
        );
        locations[index] = disks.record(&read_at(&mut file, offset, header_len)?)?;
        let mut cursor = offset + header_len as u64;
        while cursor < next {
            check()?;
            let amount = (next - cursor).min(buffer.len() as u64) as usize;
            file.read_exact(&mut buffer[..amount])?;
            disks.payload(&buffer[..amount])?;
            cursor += amount as u64;
            progress(cursor, length);
        }
    }
    let mut cd_location = None;
    let mut cd_size = 0u64;
    let mut counts = HashMap::<u32, u64>::new();
    for (index, record) in dir.records.iter().enumerate() {
        check()?;
        let (disk, offset) = locations[index];
        let record = relocate(record, disk, offset)?;
        let location = disks.record(&record)?;
        cd_location.get_or_insert(location);
        *counts.entry(location.0).or_default() += 1;
        cd_size += record.len() as u64;
    }
    disks.reserve(98 + dir.comment.len())?;
    let (cd_disk, cd_offset) = cd_location.context("ZIP 中央目录为空")?;
    let disk = disks.disk();
    let zip64_offset = disks.position;
    let count = dir.records.len() as u64;
    let disk_count = counts.get(&disk).copied().unwrap_or(0);
    let mut end = vec![0u8; 98];
    end[..4].copy_from_slice(b"PK\x06\x06");
    end[4..12].copy_from_slice(&44u64.to_le_bytes());
    end[12..14].copy_from_slice(&45u16.to_le_bytes());
    end[14..16].copy_from_slice(&45u16.to_le_bytes());
    end[16..20].copy_from_slice(&disk.to_le_bytes());
    end[20..24].copy_from_slice(&cd_disk.to_le_bytes());
    end[24..32].copy_from_slice(&disk_count.to_le_bytes());
    end[32..40].copy_from_slice(&count.to_le_bytes());
    end[40..48].copy_from_slice(&cd_size.to_le_bytes());
    end[48..56].copy_from_slice(&cd_offset.to_le_bytes());
    end[56..60].copy_from_slice(b"PK\x06\x07");
    end[60..64].copy_from_slice(&disk.to_le_bytes());
    end[64..72].copy_from_slice(&zip64_offset.to_le_bytes());
    end[72..76].copy_from_slice(&(disk + 1).to_le_bytes());
    end[76..80].copy_from_slice(b"PK\x05\x06");
    end[80..82].copy_from_slice(&(disk.min(u16::MAX as u32) as u16).to_le_bytes());
    end[82..84].copy_from_slice(&(cd_disk.min(u16::MAX as u32) as u16).to_le_bytes());
    end[84..86].copy_from_slice(&(disk_count.min(u16::MAX as u64) as u16).to_le_bytes());
    end[86..88].copy_from_slice(&(count.min(u16::MAX as u64) as u16).to_le_bytes());
    end[88..92].copy_from_slice(&(cd_size.min(u32::MAX as u64) as u32).to_le_bytes());
    end[92..96].copy_from_slice(&(cd_offset.min(u32::MAX as u64) as u32).to_le_bytes());
    end[96..98].copy_from_slice(&(dir.comment.len() as u16).to_le_bytes());
    end.extend_from_slice(&dir.comment);
    disks.record(&end)?;
    check()?;
    progress(length, length);
    disks.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires the genuine streaming >4 GiB original and the completed main matrix"]
    fn zip64_1515_streamed_descriptor_crosses_a_volume_boundary() {
        use sha2::{Digest, Sha256};
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-results/zip64-1515");
        let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("source.json")).unwrap()).unwrap();
        let source = root.join("external-plain-stream.zip");
        let mut archive = zip_aes::ZipArchive::new(File::open(&source).unwrap()).unwrap();
        let big = archive.by_index(0).unwrap();
        assert!(big.size() > u32::MAX as u64);
        let descriptor = big.data_start().expect("Original local header must be readable") + big.compressed_size();
        drop(big); drop(archive);
        let mut file = File::open(&source).unwrap();
        let local = read_at(&mut file, 0, 30).unwrap();
        assert_ne!(u16_at(&local, 6) & 8, 0, "Original must actually use a data descriptor");
        assert_eq!(read_at(&mut file, descriptor, 4).unwrap(), b"PK\x07\x08");
        let directory = root.join("streamed-descriptor-standard");
        std::fs::create_dir_all(&directory).unwrap();
        let output = directory.join("流式ZIP64跨卷.zip");
        let result = split(&source, &output, descriptor + 4 + 10, || Ok(()), |_, _| {}).unwrap();
        assert!(descriptor + 4 < result.part_files[0].metadata().unwrap().len());
        assert!(descriptor + 4 + 24 > result.part_files[0].metadata().unwrap().len());
        let reader = super::super::numeric_volume_reader::ZipInputReader::open(&output).unwrap();
        let mut archive = zip_aes::ZipArchive::new(reader).unwrap();
        let mut checked = Vec::new();
        for expected in manifest["files"].as_array().unwrap() {
            let name = expected["name"].as_str().unwrap();
            let mut entry = archive.by_name(name).unwrap();
            let mut h = Sha256::new(); let mut bytes = 0u64; let mut buffer = vec![0; 1024 * 1024];
            loop { let n = entry.read(&mut buffer).unwrap(); if n == 0 { break; } h.update(&buffer[..n]); bytes += n as u64; }
            let digest = format!("{:x}", h.finalize());
            assert_eq!(bytes, expected["bytes"].as_u64().unwrap());
            assert_eq!(digest, expected["sha256"].as_str().unwrap());
            checked.push(serde_json::json!({"name":name,"bytes":bytes,"sha256":digest}));
        }
        assert!(archive.by_name("空目录/").unwrap().is_dir());
        let mut cases: Vec<serde_json::Value> = serde_json::from_slice(&std::fs::read(root.join("native-results.json")).unwrap()).unwrap();
        assert_eq!(cases.len(), 9, "Run after the completed main matrix, once");
        cases.push(serde_json::json!({"mode":"plain-stream","layout":"standard-descriptor","source":output,
            "parts":result.part_files,"files":checked,"descriptorCrossesDisk":true,"descriptorVirtualOffset":descriptor+4}));
        std::fs::write(root.join("native-results.json"), serde_json::to_vec_pretty(&cases).unwrap()).unwrap();
    }

    const PASSWORD: &str = "volume-password + !";
    fn fixture(path: &Path, mode: u8, many: bool) -> Vec<u8> {
        let mut writer = zip_aes::ZipWriter::new(File::create(path).unwrap());
        let plain = zip_aes::write::SimpleFileOptions::default()
            .compression_method(zip_aes::CompressionMethod::Stored);
        let options = match mode {
            1 => plain.with_aes_encryption(zip_aes::AesMode::Aes256, PASSWORD),
            2 => {
                use zip_aes::unstable::write::FileOptionsExt;
                plain
                    .with_deprecated_encryption(PASSWORD.as_bytes())
                    .unwrap()
            }
            _ => plain,
        };
        let payload = (0..170_003)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        writer.add_directory("中文/空目录/", plain).unwrap();
        writer.start_file("中文/资料.bin", options).unwrap();
        writer.write_all(&payload).unwrap();
        if many {
            for index in 0..1000 {
                writer
                    .start_file(format!("中文/条目-{index:04}-空文件.txt"), options)
                    .unwrap();
            }
        }
        writer.finish().unwrap();
        if mode == 3 {
            // Force a valid small ZIP64 end/locator, not fake oversized local
            // headers. The source itself must pass the independent verifier.
            let mut bytes = std::fs::read(path).unwrap();
            let offset = bytes.len() - 22;
            assert_eq!(&bytes[offset..offset + 4], b"PK\x05\x06");
            let mut end = vec![0u8; 76];
            end[..4].copy_from_slice(b"PK\x06\x06");
            end[4..12].copy_from_slice(&44u64.to_le_bytes());
            end[12..14].copy_from_slice(&45u16.to_le_bytes());
            end[14..16].copy_from_slice(&45u16.to_le_bytes());
            end[24..32].copy_from_slice(&(u16_at(&bytes, offset + 8) as u64).to_le_bytes());
            end[32..40].copy_from_slice(&(u16_at(&bytes, offset + 10) as u64).to_le_bytes());
            end[40..48].copy_from_slice(&(u32_at(&bytes, offset + 12) as u64).to_le_bytes());
            end[48..56].copy_from_slice(&(u32_at(&bytes, offset + 16) as u64).to_le_bytes());
            end[56..60].copy_from_slice(b"PK\x06\x07");
            end[64..72].copy_from_slice(&(offset as u64).to_le_bytes());
            end[72..76].copy_from_slice(&1u32.to_le_bytes());
            let mut footer = bytes.split_off(offset);
            footer[8..20].fill(255);
            bytes.extend_from_slice(&end);
            bytes.extend_from_slice(&footer);
            std::fs::write(path, bytes).unwrap();
        }
        payload
    }
    #[test]
    fn plain_aes_zipcrypto_zip64_and_cross_disk_directory_roundtrip() {
        for mode in 0..4 {
            let dir = tempfile::tempdir().unwrap();
            let source = dir.path().join("原包.zip");
            let output = dir.path().join("发给朋友.zip");
            let payload = fixture(&source, mode, true);
            let before = std::fs::read(&source).unwrap();
            let result = split(&source, &output, 65_536, || Ok(()), |_, _| {}).unwrap();
            assert!(result.part_count > 2);
            assert_eq!(result.part_files.last().unwrap(), &output);
            assert!(result
                .part_files
                .iter()
                .all(|part| part.metadata().unwrap().len() <= 65_536));
            let reader =
                super::super::numeric_volume_reader::ZipInputReader::open(&output).unwrap();
            let mut archive = zip_aes::ZipArchive::new(reader).unwrap();
            assert_eq!(archive.len(), 1002);
            let mut entry = if mode == 1 || mode == 2 {
                archive
                    .by_name_decrypt("中文/资料.bin", PASSWORD.as_bytes())
                    .unwrap()
            } else {
                archive.by_name("中文/资料.bin").unwrap()
            };
            let mut restored = Vec::new();
            entry.read_to_end(&mut restored).unwrap();
            assert_eq!(restored, payload);
            drop(entry);
            assert!(archive.by_name("中文/空目录/").unwrap().is_dir());
            assert_eq!(std::fs::read(&source).unwrap(), before);
            super::super::compression_verification::verify_native(
                super::super::compression_format::CompressionRoute::Zip,
                &output,
                if mode == 1 || mode == 2 {
                    Some(PASSWORD)
                } else {
                    None
                },
                || false,
            )
            .unwrap();
            super::super::compression_verification::verify_numeric_zip(
                &output,
                if mode == 1 || mode == 2 {
                    Some(PASSWORD)
                } else {
                    None
                },
                || false,
            )
            .unwrap();
            let group =
                super::super::split_archive_detector::SplitArchiveDetector::detect_split_archive(
                    &result.part_files[1],
                )
                .unwrap()
                .unwrap();
            assert!(group.is_complete);
            assert_eq!(group.first_part, output);
        }
    }
    #[test]
    fn small_archive_is_one_zip_and_legacy_mode_is_the_default() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("原包.zip");
        let output = dir.path().join("小包.zip");
        let mut writer = zip::ZipWriter::new(File::create(&source).unwrap());
        writer
            .start_file("说明.txt", zip::write::FileOptions::default())
            .unwrap();
        writer.write_all(b"hello").unwrap();
        writer.finish().unwrap();
        let result = split(&source, &output, 65_536, || Ok(()), |_, _| {}).unwrap();
        assert_eq!(result.part_count, 1);
        assert_eq!(
            std::fs::read(source).unwrap(),
            std::fs::read(&output).unwrap()
        );
        assert_eq!(
            super::super::archive_volumes::first_volume_path(&output),
            output
        );
        let options: crate::models::compression::CompressionOptions = serde_json::from_value(
            serde_json::json!({"level":6,"password":null,"split_size":null}),
        )
        .unwrap();
        assert_eq!(options.zip_split_mode, ZipSplitMode::Numeric);
    }
    #[test]
    fn cancellation_size_limits_and_existing_outputs_leave_only_the_original_files() {
        for scenario in ["cancel", "existing", "tiny"] {
            let dir = tempfile::tempdir().unwrap();
            let source = dir.path().join("原包.zip");
            let output = dir.path().join("分享.zip");
            fixture(&source, 0, false);
            let before = std::fs::read(&source).unwrap();
            if scenario == "existing" {
                std::fs::write(output.with_extension("z01"), b"existing").unwrap();
            }
            let mut checks = 0;
            let result = split(
                &source,
                &output,
                if scenario == "tiny" { 37 } else { 65_536 },
                || {
                    checks += 1;
                    if scenario == "cancel" && checks == 4 {
                        anyhow::bail!("cancelled");
                    }
                    Ok(())
                },
                |_, _| {},
            );
            assert!(result.is_err());
            assert_eq!(std::fs::read(&source).unwrap(), before);
            assert!(!output.exists());
            if scenario == "existing" {
                assert_eq!(
                    std::fs::read(output.with_extension("z01")).unwrap(),
                    b"existing"
                );
            } else {
                assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
            }
        }
    }
    #[test]
    fn standard_zip_publication_and_primary_result_preserve_volume_sets() {
        for scenario in ["success", "conflict", "cancel", "gap"] {
            let dir = tempfile::tempdir().unwrap();
            let source = dir.path().join("原包.zip");
            fixture(&source, 1, false);
            let output = dir.path().join("分享.zip");
            let staged =
                super::super::output_publish_transaction::staged_output_path(&output, "test")
                    .unwrap();
            let result = split(&source, &staged, 65_536, || Ok(()), |_, _| {}).unwrap();
            if scenario == "gap" {
                std::fs::remove_file(&result.part_files[1]).unwrap();
            }
            if scenario == "conflict" {
                std::fs::write(output.with_extension("z01"), b"existing").unwrap();
            }
            let mut checks = 0;
            let result = super::super::output_publish_transaction::publish_verified_volumes(
                &staged,
                &output,
                || {
                    checks += 1;
                    scenario == "cancel" && checks == 2
                },
            );
            assert_eq!(result.is_ok(), scenario == "success");
            if scenario == "success" {
                assert_eq!(
                    super::super::archive_volumes::first_volume_path(&output),
                    output
                );
                assert_eq!(
                    super::super::archive_volumes::complete_volume_paths(&output)
                        .unwrap()
                        .last()
                        .unwrap(),
                    &output
                );
            } else {
                assert!(!output.exists());
                assert_eq!(
                    std::fs::read_dir(dir.path()).unwrap().count(),
                    if scenario == "conflict" { 2 } else { 1 }
                );
            }
            assert!(!staged.exists());
        }
    }
    #[test]
    #[ignore = "requires independent 7-Zip executable"]
    fn independent_engine_opens_all_standard_zip_modes() {
        let executable = std::env::var("LONG_SEVENZIP_BINARY")
            .ok()
            .or_else(crate::utils::archive_tools::find_7z_command)
            .expect("7-Zip executable required");
        for mode in 0..4 {
            let dir = tempfile::tempdir().unwrap();
            let source = dir.path().join("source.zip");
            fixture(&source, mode, true);
            let original = crate::utils::process::command(&executable)
                .arg("t")
                .arg(&source)
                .arg(format!("-p{PASSWORD}"))
                .arg("-y")
                .output()
                .unwrap();
            assert!(
                original.status.success(),
                "Source fixture mode {mode} must independently validate: {} {}",
                String::from_utf8_lossy(&original.stdout),
                String::from_utf8_lossy(&original.stderr)
            );
            let output = dir.path().join("result.zip");
            split(&source, &output, 65_536, || Ok(()), |_, _| {}).unwrap();
            let mut command = crate::utils::process::command(&executable);
            command.arg("t").arg(&output).arg("-y");
            if mode == 1 || mode == 2 {
                command.arg(format!("-p{PASSWORD}"));
            }
            let result = command.output().unwrap();
            if !result.status.success() {
                let retained = dir.keep();
                panic!(
                    "mode {mode}; retained {}: {} {}",
                    retained.display(),
                    String::from_utf8_lossy(&result.stdout),
                    String::from_utf8_lossy(&result.stderr)
                );
            }
        }
    }

    struct Runtime;
    impl super::super::native_compression::CompressionRuntime for Runtime {
        fn check_cancellation(&self) -> Result<()> {
            Ok(())
        }
        fn cancellation_flag(&self) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false))
        }
        fn copy_buffer_size(&self) -> usize {
            256 * 1024
        }
        fn emit_log(
            &self,
            _: &tauri::Window,
            _: &str,
            _: &str,
            _: crate::models::compression::TaskLogSeverity,
        ) {
        }
        fn emit_progress(
            &self,
            _: &tauri::Window,
            _: &str,
            _: f32,
            _: Option<String>,
            _: u64,
            _: u64,
        ) {
        }
    }
    #[test]
    #[ignore = "creates and verifies a real >4 GiB ZIP64 archive"]
    fn real_zip64_file_and_large_offsets_create_standard_disks() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("大文件.bin");
        let note = dir.path().join("说明.txt");
        let output = dir.path().join("真实ZIP64.zip");
        let length = (1u64 << 32) + 12345;
        File::create(&input).unwrap().set_len(length).unwrap();
        std::fs::write(&note, "大文件之后的条目不能丢失").unwrap();
        println!("Creating real ZIP64 input with {length} bytes; bounded-memory native creation and standard disks");
        super::super::native_compression::zip::compress(
            &Runtime,
            None,
            "real-zip64",
            &[
                input.to_string_lossy().into_owned(),
                note.to_string_lossy().into_owned(),
            ],
            &output.to_string_lossy(),
            crate::models::compression::CompressionOptions {
                level: 0,
                split_size: Some(1 << 30),
                zip_split_mode: ZipSplitMode::Standard,
                ..Default::default()
            },
        )
        .unwrap();
        let parts = super::super::archive_volumes::complete_volume_paths(&output).unwrap();
        let archive_bytes: u64 = parts
            .iter()
            .map(|part| part.metadata().unwrap().len())
            .sum();
        assert!(archive_bytes > u32::MAX as u64);
        assert_eq!(parts.len(), 5);
        super::super::extraction_transaction::validate_resource_limits(&output, 2, length + 128).unwrap();
        let mut archive = zip_aes::ZipArchive::new(
            super::super::numeric_volume_reader::ZipInputReader::open(&output).unwrap(),
        )
        .unwrap();
        let mut entry = archive.by_name("大文件.bin").unwrap();
        assert_eq!(entry.size(), length);
        assert_eq!(
            std::io::copy(&mut entry, &mut std::io::sink()).unwrap(),
            length
        );
        drop(entry);
        let mut text = String::new();
        archive
            .by_name("说明.txt")
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        assert_eq!(text, "大文件之后的条目不能丢失");
        let executable = std::env::var("LONG_SEVENZIP_BINARY")
            .ok()
            .or_else(crate::utils::archive_tools::find_7z_command)
            .expect("7-Zip executable required");
        let independent = crate::utils::process::command(&executable)
            .arg("t")
            .arg(&output)
            .arg("-y")
            .output()
            .unwrap();
        assert!(
            independent.status.success(),
            "{} {}",
            String::from_utf8_lossy(&independent.stdout),
            String::from_utf8_lossy(&independent.stderr)
        );
        assert_eq!(input.metadata().unwrap().len(), length);
        let evidence = serde_json::json!({ "kind": "real-native-rust-zip64-creation-and-read", "inputBytes": length, "archiveBytes": archive_bytes,
            "volumes": parts.len(), "nativeFullCrcRead": true, "largeOffsetFollowingEntryPreserved": true, "independentSevenZipVerified": true, "aggregateVolumeResourcePreflight": true, "passwordVariantsTestedAtThisSize": false });
        if let Ok(path) = std::env::var("LONG_ZIP64_EVIDENCE") {
            std::fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
        }
        println!("REAL_ZIP64_EVIDENCE={evidence}");
    }
}
