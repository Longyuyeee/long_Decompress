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
    fn originals_158() -> (std::path::PathBuf, serde_json::Value) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sfx-158");
        let manifest = serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap();
        (root, manifest)
    }
    #[test]
    fn original_158_gui_modules_are_exact_loaders() {
        let (root, manifest) = originals_158();
        let mut failures = Vec::new();
        for c in manifest["cases"].as_array().unwrap().iter().filter(|c| c["kind"] == "7z" && c["layout"] == "external") {
            let exe = root.join(c["name"].as_str().unwrap()).join(c["archive"].as_str().unwrap());
            if super::super::sfx_volumes::seven_zip_loader_data(&exe).unwrap().is_none() { failures.push(c["name"].clone()); }
        }
        assert!(failures.is_empty(), "Original loaders not recognized: {failures:?}");
    }
    #[tokio::test]
    async fn original_158_every_member_browses_and_keeps_sources() {
        use sha2::{Digest, Sha256};
        let (root, manifest) = originals_158();
        for c in manifest["cases"].as_array().unwrap() {
            let dir = root.join(c["name"].as_str().unwrap());
            for member in c["members"].as_array().unwrap() {
                let input = dir.join(member["name"].as_str().unwrap());
                let browse = super::super::archive_browser::browse_archive(&input, c["password"].as_str()).await
                    .unwrap_or_else(|e| panic!("{}: {e:#}", input.display()));
                assert_eq!(browse.total_files, 3);
                assert_eq!(browse.total_uncompressed_size, 275544);
                assert_eq!(format!("{:x}", Sha256::digest(std::fs::read(&input).unwrap())), member["sha256"].as_str().unwrap());
            }
        }
    }
    #[tokio::test]
    async fn original_158_missing_data_volumes_precede_password() {
        let (root, manifest) = originals_158();
        for c in manifest["cases"].as_array().unwrap().iter().filter(|c| c["layout"] == "external") {
            let dir = root.join(c["name"].as_str().unwrap()); let tmp = tempfile::tempdir().unwrap();
            for member in c["members"].as_array().unwrap() { let name = member["name"].as_str().unwrap(); std::fs::copy(dir.join(name), tmp.path().join(name)).unwrap(); }
            let selected = tmp.path().join(c["archive"].as_str().unwrap());
            let group = super::super::split_archive_detector::SplitArchiveDetector::detect_split_archive(&selected).unwrap().unwrap();
            for index in [0, 2, group.parts.len()-1] {
                let part = &group.parts[index]; let bytes = std::fs::read(part).unwrap(); std::fs::remove_file(part).unwrap();
                let input = if index == 0 && c["kind"] == "rar" { &group.parts[1] } else { &selected };
                if index != group.parts.len()-1 || c["mode"] != "headers" || c["kind"] == "7z" {
                    for key in [None, Some("wrong-key"), c["password"].as_str()] {
                        let e = super::super::archive_browser::browse_archive(input, key).await.unwrap_err();
                        assert!(format!("{e:#}").contains("incomplete"), "{} index{index}: {e:#}", c["name"]);
                    }
                }
                std::fs::write(part, bytes).unwrap();
            }
        }
    }
    #[test]
    fn original_158_changed_loader_with_same_length_is_rejected() {
        let (root, manifest) = originals_158();
        for m in manifest["modules"].as_array().unwrap().iter().filter(|m| m["kind"] == "7z-gui") {
            let tmp = tempfile::tempdir().unwrap(); let input = tmp.path().join("changed.exe");
            let mut bytes = std::fs::read(root.join(m["module"].as_str().unwrap())).unwrap(); bytes[128] ^= 1; std::fs::write(&input, bytes).unwrap();
            std::fs::write(tmp.path().join("changed.7z.001"), b"unrelated data").unwrap();
            assert!(super::super::sfx_volumes::seven_zip_loader_data(&input).unwrap().is_none());
            assert!(super::super::archive_browser::detect_format(&input).is_err());
        }
    }
    #[test]
    fn genuine_external_sfx_originals_group_every_member() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sfx-155");
        let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap();
        for case in manifest["cases"].as_array().unwrap() {
            if case["kind"] == "rar4-single" { continue; }
            let archive = root.join(case["archive"].as_str().unwrap());
            let dir = archive.parent().unwrap();
            let seven = case["kind"] == "7z-volumes";
            let first = if seven { dir.join("用户资料.7z.001") } else { archive.clone() };
            for member in case["members"].as_array().unwrap() {
                let member = dir.join(member["name"].as_str().unwrap());
                let group = super::super::split_archive_detector::SplitArchiveDetector::detect_split_archive(&member).unwrap().unwrap_or_else(|| panic!("SFX group missing: {}", member.display()));
                assert_eq!(group.first_part, first);
                assert!(group.is_complete);
                assert_eq!(group.parts.len(), if seven {5} else {6});
            }
        }
    }
    #[tokio::test]
    async fn genuine_external_sfx_browse_and_password_candidates_every_member() {
        use super::super::{archive_browser, compression_service::CompressionService};
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sfx-155");
        let m: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let service = CompressionService::new(Default::default(), std::sync::Arc::new(super::super::io_buffer_pool::IOBufferPool::default()), std::sync::Arc::new(super::super::rar_support::RarSupportService::new()), std::sync::Arc::new(super::super::universal_engine::UniversalCliEngine::new()), std::sync::Arc::new(super::super::encrypted_password_service::EncryptedPasswordService::new(scratch.path())));
        for c in m["cases"].as_array().unwrap() {
            let source = root.join(c["archive"].as_str().unwrap());
            for member in c["members"].as_array().unwrap() {
                let member = source.parent().unwrap().join(member["name"].as_str().unwrap());
                let b = archive_browser::browse_archive(&member, c["password"].as_str()).await.unwrap_or_else(|e| panic!("{}: {e:#}", member.display()));
                assert_eq!(b.total_files, 3, "{}", member.display());
                assert_eq!(b.total_uncompressed_size, 275544);
                if c["password"].is_string() && member == source {
                    assert!(service.verify_archive_password_candidate(member.to_str().unwrap(), c["password"].as_str().unwrap()).await.unwrap());
                    assert!(!service.verify_archive_password_candidate(member.to_str().unwrap(), "wrong-sfx-password").await.unwrap_or(false), "wrong key accepted: {}", member.display());
                }
            }
        }
    }
    #[tokio::test]
    async fn genuine_external_sfx_missing_and_damaged_volumes_precede_keys() {
        use super::super::{archive_browser, split_archive_detector::SplitArchiveDetector};
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sfx-155");
        let m: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap();
        for c in m["cases"].as_array().unwrap().iter().filter(|c| c["kind"] != "rar4-single") {
            let source = root.join(c["archive"].as_str().unwrap()); let tmp = tempfile::tempdir().unwrap();
            for member in c["members"].as_array().unwrap() { let name = member["name"].as_str().unwrap(); std::fs::copy(source.parent().unwrap().join(name), tmp.path().join(name)).unwrap(); }
            let selected = tmp.path().join(source.file_name().unwrap());
            let group = SplitArchiveDetector::detect_split_archive(&selected).unwrap().unwrap();
            for index in [0, 2, group.parts.len()-1] {
                let part = &group.parts[index]; let bytes = std::fs::read(part).unwrap(); std::fs::remove_file(part).unwrap();
                let input = if index == 0 { &group.parts[1] } else { &selected };
                // Encrypted RAR headers conceal the last index. In that case
                // final completeness is established only during keyed decode.
                if index != group.parts.len()-1 || c["mode"] != "headers" || c["kind"] == "7z-volumes" {
                    let error = archive_browser::browse_archive(input, Some("wrong-key")).await.unwrap_err();
                    assert!(format!("{error:#}").contains("[archive-inspection:incomplete]"), "{} index{index}: {error:#}", c["name"]);
                }
                std::fs::write(part, bytes).unwrap();
            }
            if c["kind"] != "7z-volumes" {
                let part = &group.parts[1]; let mut bytes = std::fs::read(part).unwrap(); bytes[8] ^= 1; std::fs::write(part, bytes).unwrap();
                let error = SplitArchiveDetector::detect_split_archive(&selected).unwrap_err().to_string(); assert!(error.contains("[archive-inspection:damaged]"), "{error}");
            }
        }
    }
    #[test]
    fn unknown_exe_is_not_a_loader_just_because_matching_data_exists() {
        let tmp = tempfile::tempdir().unwrap(); let exe = tmp.path().join("ordinary.exe");
        std::fs::write(&exe, b"MZ this program is not an archive").unwrap(); std::fs::write(tmp.path().join("ordinary.7z.001"), b"not data").unwrap();
        assert!(super::super::sfx_volumes::seven_zip_loader_data(&exe).unwrap().is_none());
        assert!(super::super::archive_browser::detect_format(&exe).is_err());
    }
    #[test]
    fn native_sfx_cancel_unwinds_and_releases_partial_output_handle() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sfx-155/rar5-modern-plain/用户资料.part1.exe");
        let tmp = tempfile::tempdir().unwrap(); let output = tmp.path().join("private-output"); std::fs::create_dir(&output).unwrap();
        let mut archive = unrar::Archive::new(&source).open_for_processing().unwrap();
        loop {
            let header = archive.read_header().unwrap().unwrap();
            if header.entry().unpacked_size == 0 { archive = header.skip().unwrap(); continue; }
            let file = output.join("partial.bin");
            assert!(header.extract_to_with_control(&file, &|| false).is_err());
            // This failed with MSVC's original missing /EHsc: the local C++
            // output File destructor was skipped by the cancellation throw.
            std::fs::remove_dir_all(&output).unwrap(); assert!(!output.exists()); break;
        }
    }
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
