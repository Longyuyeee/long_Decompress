//! One ZIP selection for password checks, resource estimates and extraction.
use crate::models::compression::DecompressOptions;
use crate::services::extraction_transaction;
use anyhow::Result;
use std::io::{Read, Seek};
use std::path::Path;
use zip_aes::ZipArchive;

#[derive(Debug)]
pub(crate) struct ZipSelection {
    pub indices: Vec<usize>,
    pub expanded_bytes: u64,
    pub password_required: bool,
}

pub(crate) fn has_selection(options: &DecompressOptions) -> bool {
    !options.selected_entries.is_empty()
        || options.file_filter.as_deref().is_some_and(|filter| !filter.trim().is_empty())
}

impl ZipSelection {
    pub(crate) fn read<R: Read + Seek>(
        archive: &mut ZipArchive<R>,
        options: &DecompressOptions,
        mut check_control: impl FnMut() -> Result<()>,
    ) -> Result<Self> {
        let filter = extraction_transaction::compile_file_filter(options.file_filter.as_deref());
        let mut selection = Self { indices: Vec::new(), expanded_bytes: 0, password_required: false };
        for index in 0..archive.len() {
            check_control()?;
            // Raw metadata does not initialize a codec or ask for the password
            // of an entry outside the requested selection.
            let entry = archive.by_index_raw(index)?;
            let normalized = entry.name().replace('\\', "/");
            let path = Path::new(&normalized);
            if !extraction_transaction::matches_selected_entries(path, &options.selected_entries)
                || !extraction_transaction::matches_compiled_file_filter(path, &filter)
            {
                continue;
            }
            #[allow(deprecated)]
            if matches!(entry.compression(), zip_aes::CompressionMethod::Unsupported(_)) {
                anyhow::bail!("[archive-inspection:unknown] ZIP 条目「{}」的压缩方法 {:?} 当前不受支持；不能据此判定文件损坏或密码错误", entry.name(), entry.compression());
            }
            selection.indices.push(index);
            selection.password_required |= entry.encrypted();
            selection.expanded_bytes = selection.expanded_bytes.checked_add(entry.size())
                .ok_or_else(|| anyhow::anyhow!("ZIP 所选内容的解压大小超出支持范围"))?;
        }
        anyhow::ensure!(!selection.indices.is_empty() || !has_selection(options),
            "所选范围内没有可解压的条目，请重新选择文件或修改筛选条件");
        Ok(selection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::{compression_service::CompressionService, native_extraction::zip, universal_engine::UniversalCliEngine};
    use std::io::{Cursor, Write};

    fn fixture() -> Vec<u8> {
        let mut writer = zip_aes::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, password, payload) in [
            ("资料/说明.txt", None, "公开"),
            ("资料/甲方/合同.bin", Some("alpha"), "甲方的合同"),
            ("资料/乙方/合同.bin", Some("beta"), "乙方的合同"),
        ] {
            let options = zip_aes::write::SimpleFileOptions::default();
            let options = password.map_or(options, |password| options.with_aes_encryption(zip_aes::AesMode::Aes256, password));
            writer.start_file(name, options).unwrap();
            writer.write_all(payload.as_bytes()).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn selected_zip_metadata_counts_only_requested_original_paths() {
        let mut archive = ZipArchive::new(Cursor::new(fixture())).unwrap();
        let options = DecompressOptions { selected_entries: vec!["资料/甲方".into()], preserve_paths: false, ..Default::default() };
        let plan = ZipSelection::read(&mut archive, &options, || Ok(())).unwrap();
        assert_eq!(plan.indices, vec![1]);
        assert_eq!(plan.expanded_bytes, "甲方的合同".len() as u64);
        assert!(plan.password_required);
        let options = DecompressOptions { file_filter: Some("*.txt".into()), ..Default::default() };
        let plan = ZipSelection::read(&mut archive, &options, || Ok(())).unwrap();
        assert_eq!(plan.indices, vec![0]);
        assert_eq!(plan.expanded_bytes, "公开".len() as u64);
        assert!(!plan.password_required);
    }

    #[test]
    fn selected_zip_filter_and_selection_intersect_and_missing_range_is_explicit() {
        let mut archive = ZipArchive::new(Cursor::new(fixture())).unwrap();
        for options in [
            DecompressOptions { selected_entries: vec!["不存在.txt".into()], ..Default::default() },
            DecompressOptions { selected_entries: vec!["资料/甲方".into()], file_filter: Some("*.txt".into()), ..Default::default() },
        ] {
            assert!(ZipSelection::read(&mut archive, &options, || Ok(())).unwrap_err().to_string().contains("没有可解压"));
        }
    }

    #[test]
    fn selected_zip_cancellation_is_not_a_password_error() {
        let mut archive = ZipArchive::new(Cursor::new(fixture())).unwrap();
        let error = ZipSelection::read(&mut archive, &DecompressOptions::default(), || anyhow::bail!("user-cancelled")).err().unwrap();
        assert_eq!(error.to_string(), "user-cancelled");
    }

    #[test]
    fn selected_zip_aes_credentials_do_not_claim_to_unlock_the_whole_package() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("混合.zipx");
        std::fs::write(&file, fixture()).unwrap();
        let selected = DecompressOptions { selected_entries: vec!["资料/甲方".into()], ..Default::default() };
        assert!(UniversalCliEngine::try_zip_password_for_selection_cancellable(&file, "alpha", &selected, || Ok(())).unwrap());
        assert!(!UniversalCliEngine::try_zip_password_for_selection_cancellable(&file, "beta", &selected, || Ok(())).unwrap());
        assert!(!UniversalCliEngine::try_zip_password(&file, "alpha").unwrap());
        let public = DecompressOptions { file_filter: Some("*.txt".into()), ..Default::default() };
        assert!(!UniversalCliEngine::zip_requires_password_for_selection(&file, &public).unwrap());
        assert!(!UniversalCliEngine::try_zip_password_for_selection_cancellable(&file, "anything", &public, || Ok(())).unwrap());
    }

    #[test]
    fn selected_zip_aes_reads_original_paths_then_flattens_once() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("混合.zip");
        std::fs::write(&file, fixture()).unwrap();
        let output = temp.path().join("out");
        let options = DecompressOptions { selected_entries: vec!["资料/甲方".into()], preserve_paths: false, ..Default::default() };
        zip::extract_with_optional_window(&CompressionService::for_testing(), None, "selected-aes", file.to_str().unwrap(), output.to_str().unwrap(), Some("alpha"), &options).unwrap();
        extraction_transaction::prepare_staging_layout(&file, &output, &options).unwrap();
        assert_eq!(std::fs::read_to_string(output.join("合同.bin")).unwrap(), "甲方的合同");
        assert_eq!(std::fs::read_dir(output).unwrap().count(), 1);
    }

    #[test]
    fn selected_zip_unknown_method_outside_selection_does_not_trigger_password_attempts() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("未知方法.zip");
        let mut bytes = fixture();
        // Replace the actual compression method in the AES extra field, leaving
        // ZIP's outer method 99 intact. This is an unsupported feature, not a
        // damaged stream or an invalid password.
        let mut at = 0;
        while at + 46 <= bytes.len() {
            if bytes[at..].starts_with(b"PK\x03\x04") {
                let n = u16::from_le_bytes(bytes[at+26..at+28].try_into().unwrap()) as usize;
                if std::str::from_utf8(&bytes[at+30..at+30+n]).unwrap().ends_with("合同.bin") {
                    let length = u16::from_le_bytes(bytes[at+28..at+30].try_into().unwrap()) as usize;
                    replace_aes_method(&mut bytes[at+30+n..at+30+n+length]);
                }
            } else if bytes[at..].starts_with(b"PK\x01\x02") {
                let n = u16::from_le_bytes(bytes[at+28..at+30].try_into().unwrap()) as usize;
                if std::str::from_utf8(&bytes[at+46..at+46+n]).unwrap().ends_with("合同.bin") {
                    let length = u16::from_le_bytes(bytes[at+30..at+32].try_into().unwrap()) as usize;
                    replace_aes_method(&mut bytes[at+46+n..at+46+n+length]);
                }
            }
            at += 1;
        }
        std::fs::write(&file, bytes).unwrap();
        let public = DecompressOptions { file_filter: Some("*.txt".into()), ..Default::default() };
        assert!(!UniversalCliEngine::zip_requires_password_for_selection(&file, &public).unwrap());
        let error = UniversalCliEngine::zip_requires_password(&file).unwrap_err().to_string();
        assert!(error.contains("不受支持") && error.contains("不能据此判定"), "{error}");
    }

    fn replace_aes_method(extra: &mut [u8]) {
        let mut at = 0;
        while at + 4 <= extra.len() {
            let tag = u16::from_le_bytes(extra[at..at+2].try_into().unwrap());
            let length = u16::from_le_bytes(extra[at+2..at+4].try_into().unwrap()) as usize;
            if tag == 0x9901 {
                assert_eq!(length, 7);
                extra[at+9..at+11].copy_from_slice(&60000u16.to_le_bytes());
                return;
            }
            at += 4 + length;
        }
        panic!("fixture must expose the actual AES method");
    }

    #[test]
    fn selected_zip_zstd_aes_numeric_and_standard_volumes_roundtrip() {
        use rand::RngCore;
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("原包.zip");
        let mut payload = vec![0; 180_000];
        rand::thread_rng().fill_bytes(&mut payload);
        let mut writer = zip_aes::ZipWriter::new(std::fs::File::create(&file).unwrap());
        writer.start_file("公开说明.txt", zip_aes::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(b"not selected").unwrap();
        writer.start_file("资料/合同.bin", zip_aes::write::SimpleFileOptions::default()
            .compression_method(zip_aes::CompressionMethod::Zstd)
            .with_aes_encryption(zip_aes::AesMode::Aes256, "中文密码 + !")).unwrap();
        writer.write_all(&payload).unwrap();
        writer.finish().unwrap();
        let bytes = std::fs::read(&file).unwrap();
        for (index, chunk) in bytes.chunks(24_000).enumerate() {
            std::fs::write(temp.path().join(format!("数字.zip.{:03}", index+1)), chunk).unwrap();
        }
        let standard = temp.path().join("标准.zip");
        crate::services::standard_zip_writer::split(&file, &standard, 65_536, || Ok(()), |_, _| {}).unwrap();
        assert!(temp.path().join("标准.z01").exists());
        let options = DecompressOptions { selected_entries: vec!["资料".into()], ..Default::default() };
        for (index, source) in [file, temp.path().join("数字.zip.001"), standard].into_iter().enumerate() {
            assert!(UniversalCliEngine::try_zip_password_for_selection_cancellable(&source, "中文密码 + !", &options, || Ok(())).unwrap());
            assert!(!UniversalCliEngine::try_zip_password_for_selection_cancellable(&source, "wrong", &options, || Ok(())).unwrap());
            let output = temp.path().join(format!("out-{index}"));
            zip::extract_with_optional_window(&CompressionService::for_testing(), None, "selected-volumes", source.to_str().unwrap(), output.to_str().unwrap(), Some("中文密码 + !"), &options).unwrap();
            assert_eq!(std::fs::read(output.join("资料/合同.bin")).unwrap(), payload);
            assert!(!output.join("公开说明.txt").exists());
        }
    }

    #[test]
    #[ignore = "exports real encrypted Zstandard ZIP fixtures for independent-tool and desktop checks"]
    fn export_zip_1410_compatibility_fixtures() {
        use rand::RngCore;
        use zip_aes::unstable::write::FileOptionsExt;
        let root = std::path::PathBuf::from(std::env::var("LONG_ZIP1410_FIXTURE_ROOT").expect("isolated fixture output required"));
        assert!(root.is_absolute() && !root.exists(), "use a new absolute fixture directory");
        std::fs::create_dir_all(&root).unwrap();
        let mut payload = vec![0; 180_000]; rand::thread_rng().fill_bytes(&mut payload);
        std::fs::write(root.join("expected.bin"), &payload).unwrap();
        let mut cases = Vec::new();
        for mode in ["AES256", "ZipCrypto"] {
            let file = root.join(format!("Zstandard-{mode}.zip"));
            let plain = zip_aes::write::SimpleFileOptions::default().compression_method(zip_aes::CompressionMethod::Zstd);
            let encrypted = if mode == "AES256" { plain.with_aes_encryption(zip_aes::AesMode::Aes256, "zstd-password + !") }
                else { plain.with_deprecated_encryption(b"zstd-password + !").unwrap() };
            let mut writer = zip_aes::ZipWriter::new(std::fs::File::create(&file).unwrap());
            writer.start_file("资料/内容.bin", encrypted).unwrap(); writer.write_all(&payload).unwrap(); writer.finish().unwrap();
            let standard = root.join(format!("Zstandard-{mode}-standard.zip"));
            crate::services::standard_zip_writer::split(&file, &standard, 65_536, || Ok(()), |_, _| {}).unwrap();
            let bytes = std::fs::read(&file).unwrap();
            let numeric = root.join(format!("Zstandard-{mode}-numeric.zip.001"));
            for (index, chunk) in bytes.chunks(65_536).enumerate() {
                std::fs::write(root.join(format!("Zstandard-{mode}-numeric.zip.{:03}", index+1)), chunk).unwrap();
            }
            for (layout, source) in [("single", file), ("numeric", numeric), ("standard", standard)] {
                cases.push(serde_json::json!({"encryption":mode, "layout":layout, "file":source, "password":"zstd-password + !", "entry":"资料/内容.bin"}));
            }
        }
        std::fs::write(root.join("manifest.json"), serde_json::to_vec_pretty(&serde_json::json!({"producer":"zip-rs 8.6 fixture writer; independent verification required", "cases":cases})).unwrap()).unwrap();
        println!("ZIP1410_FIXTURES={}", root.display());
    }
}
