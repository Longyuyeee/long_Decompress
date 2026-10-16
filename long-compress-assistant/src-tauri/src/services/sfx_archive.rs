//! Read SFX payloads as data. This module never launches an input executable.
use super::archive_format::ArchiveFormat;
use anyhow::Result;
use std::{fs::File, io::{Read, Seek, SeekFrom}, path::Path};

const MAX_MODULE_SCAN: u64 = 16 * 1024 * 1024;

fn valid_rar_header(bytes: &[u8], signature_size: usize) -> bool {
    let header = &bytes[signature_size..];
    if signature_size == 7 {
        if header.len() < 7 || header[2] != 0x73 { return false; }
        let size = u16::from_le_bytes([header[5], header[6]]) as usize;
        return size >= 7 && size <= header.len()
            && crc32fast::hash(&header[2..size]) as u16 == u16::from_le_bytes([header[0], header[1]]);
    }
    if header.len() < 6 { return false; }
    let mut size = 0u64;
    for index in 4..header.len().min(13) {
        let value = header[index];
        size |= u64::from(value & 0x7f) << ((index - 4) * 7);
        if value & 0x80 == 0 {
            let start = index + 1;
            let Ok(size) = usize::try_from(size) else { return false; };
            let Some(end) = start.checked_add(size).filter(|end| *end <= header.len()) else { return false; };
            return size > 0 && matches!(header[start], 1 | 4)
                && crc32fast::hash(&header[4..end]) == u32::from_le_bytes(header[..4].try_into().unwrap());
        }
    }
    false
}

/// A bare .exe suffix is not a format. Verify a native header or ZIP directory,
/// ignoring signature literals inside the executable's own code/resources.
pub(crate) fn detect(path: &Path) -> Result<Option<(ArchiveFormat, u64)>> {
    let mut file = File::open(path)?;
    let mut magic = [0; 2];
    if file.read(&mut magic)? != 2 || &magic != b"MZ" { return Ok(None); }
    file.seek(SeekFrom::Start(0))?;
    let mut prefix = Vec::new();
    (&mut file).take(MAX_MODULE_SCAN).read_to_end(&mut prefix)?;
    for index in 2..prefix.len().saturating_sub(7) {
        let bytes = &prefix[index..];
        if bytes.starts_with(b"7z\xbc\xaf\x27\x1c") && bytes.len() >= 32
            && crc32fast::hash(&bytes[12..32]) == u32::from_le_bytes(bytes[8..12].try_into().unwrap()) {
            return Ok(Some((ArchiveFormat::SevenZip, index as u64)));
        }
        let signature_size = if bytes.starts_with(b"Rar!\x1a\x07\x01\0") { 8 }
            else if bytes.starts_with(b"Rar!\x1a\x07\0") { 7 } else { continue; };
        if valid_rar_header(bytes, signature_size) { return Ok(Some((ArchiveFormat::Rar, index as u64))); }
    }
    file.seek(SeekFrom::Start(0))?;
    if let Ok(archive) = zip_aes::ZipArchive::new(file) {
        return Ok(Some((ArchiveFormat::Zip, archive.offset())));
    }
    Ok(None)
}

pub(crate) fn is_executable_path(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordinary_executable_and_fake_signature_are_not_archives() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("program.exe");
        std::fs::write(&path, b"MZ ordinary program 7z\xbc\xaf\x27\x1c with no valid archive header").unwrap();
        assert!(detect(&path).unwrap().is_none());
    }

    #[test]
    fn seven_zip_sfx_reader_uses_relative_offsets_and_preserves_input() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("说明.txt");
        std::fs::write(&source, b"actual payload").unwrap();
        let archive = dir.path().join("plain.7z");
        sevenz_rust::compress_to_path(&source, &archive).unwrap();
        let mut bytes = b"MZ offset regression fixture".to_vec();
        bytes.extend(std::fs::read(archive).unwrap());
        let sfx = dir.path().join("资料.exe"); std::fs::write(&sfx, &bytes).unwrap();
        assert_eq!(detect(&sfx).unwrap(), Some((ArchiveFormat::SevenZip, 28)));
        let mut reader = super::super::numeric_volume_reader::ZipInputReader::open_seven_zip(&sfx).unwrap();
        let size = reader.seek(SeekFrom::End(0)).unwrap();
        reader.seek(SeekFrom::Start(0)).unwrap();
        let metadata = sevenz_rust::Archive::read(&mut reader, size, &[]).unwrap();
        assert_eq!(metadata.files[0].size, 14);
        assert_eq!(std::fs::read(sfx).unwrap(), bytes);
    }
}
