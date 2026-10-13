//! Standard .z01/.../.zip disks exposed as a read-only, single-disk ZIP view.
//! Payload stays on the original disks. Only directory records are rebuilt in
//! memory, translating disk-relative offsets to 64-bit positions for native
//! ZIP metadata, passwords, previews and extraction. No merged payload copy.
use super::{
    numeric_volume_reader::NumericVolumeReader,
    split_archive_detector::{SplitArchiveDetector, SplitFormat},
};
use anyhow::{ensure, Context, Result};
use std::{
    io::{self, Read, Seek, SeekFrom},
    path::Path,
};

pub(crate) struct StandardZipReader {
    source: NumericVolumeReader,
    directory_start: u64,
    directory: Vec<u8>,
    position: u64,
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
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if extension != "zip"
            && !(extension.starts_with('z')
                && extension[1..].bytes().all(|byte| byte.is_ascii_digit()))
        {
            return Ok(None);
        }
        let Some(group) = SplitArchiveDetector::detect_split_archive(path)?
            .filter(|group| group.format == SplitFormat::ZipSplit)
        else {
            return Ok(None);
        };
        ensure!(
            group.is_complete,
            "[archive-inspection:incomplete] 缺少 ZIP 分卷：{:?}",
            group.missing_parts
        );
        let mut starts = vec![0u64];
        for part in &group.parts {
            starts.push(
                starts
                    .last()
                    .unwrap()
                    .checked_add(part.metadata()?.len())
                    .context("ZIP size overflow")?,
            );
        }
        let length = *starts.last().unwrap();
        let mut source = NumericVolumeReader::from_paths(group.parts)?;
        let tail_start = length.saturating_sub(65_557);
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
        if eocd_position >= 20 {
            let locator = read_at(&mut source, eocd_position - 20, 20)?;
            if &locator[..4] == b"PK\x06\x07" {
                ensure!(
                    u32_at(&locator, 16) as usize == starts.len() - 1,
                    "[archive-inspection:incomplete] ZIP64 卷数与实际文件不符"
                );
                let record = read_at(
                    &mut source,
                    absolute(&starts, u32_at(&locator, 4), u64_at(&locator, 8))?,
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
            last_disk as usize + 1 == starts.len() - 1,
            "[archive-inspection:incomplete] ZIP 终卷声明的卷数与实际文件不符"
        );
        let directory_start = absolute(&starts, cd_disk, cd_relative)?;
        ensure!(
            directory_start
                .checked_add(cd_size)
                .is_some_and(|end| end <= eocd_position),
            "Invalid ZIP directory range"
        );
        ensure!(
            cd_size <= 256 * 1024 * 1024,
            "ZIP directory exceeds the metadata reading limit"
        );
        let original = read_at(&mut source, directory_start, cd_size as usize)?;
        let mut directory = Vec::with_capacity(original.len());
        let mut cursor = 0usize;
        for _ in 0..count {
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
            let offset = absolute(&starts, disk, offset)?;
            ensure!(offset < directory_start, "ZIP entry points outside payload");
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
        }
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
