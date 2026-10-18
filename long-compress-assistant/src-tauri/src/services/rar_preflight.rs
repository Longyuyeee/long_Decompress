//! Metadata inspection runs on a blocking worker. Control checks surround SDK
//! calls; they cannot interrupt a single blocking SDK / filesystem operation.
use super::compression_service::CompressionError;
use anyhow::Result;
use std::path::Path;

pub(crate) fn stats(
    path: &Path,
    password: Option<&str>,
    check_control: impl FnMut() -> Result<()>,
) -> Result<(usize, u64)> {
    stats_with_options(path, password, &crate::models::compression::DecompressOptions::default(), check_control)
}

pub(crate) fn stats_with_options(
    path: &Path,
    password: Option<&str>,
    options: &crate::models::compression::DecompressOptions,
    mut check_control: impl FnMut() -> Result<()>,
) -> Result<(usize, u64)> {
    check_control()?;
    let path = path.to_str().ok_or_else(|| {
        CompressionError::ExtractionFailed("RAR path is not valid Unicode".to_string())
    })?;
    let archive = if let Some(password) = password {
        unrar::Archive::with_password(path, password)
    } else {
        unrar::Archive::new(path)
    };
    let opened = archive.open_for_processing();
    check_control()?;
    let mut archive = opened.map_err(|error| {
        CompressionError::ExtractionFailed(format!(
            "Unable to inspect RAR metadata safely: {:?}", error
        ))
    })?;
    let mut entries = 0usize;
    let mut expanded = 0u64;
    let encrypted_headers = archive.has_encrypted_headers();
    let mut password_proven = false;
    loop {
        check_control()?;
        let header = archive.read_header();
        // Prefer a requested cancellation to a coincident SDK failure.
        check_control()?;
        let Some(header) = header.map_err(|error| {
            let detail = super::rar_support::RarSupportService::native_failure_message(&error, password_proven)
                .unwrap_or_else(|| format!("{error:?}"));
            CompressionError::ExtractionFailed(format!("Unable to inspect RAR entry metadata: {detail}"))
        })? else { break };
        password_proven |= encrypted_headers;
        let entry = header.entry();
        // RAR_SKIP exposes each physical continuation header in processing
        // mode. Its unpacked_size repeats the entire logical file size. Do not
        // deduplicate by name: separate logical entries may share a name.
        let included = !entry.is_split_before()
            && super::rar_support::RarSupportService::normalized_archive_path(&entry.filename, options.preserve_paths)
                .is_some_and(|path| super::rar_support::RarSupportService::matches_file_filter(&path, options.file_filter.as_deref()));
        if included {
            entries = entries.saturating_add(1);
            expanded = expanded.checked_add(entry.unpacked_size).ok_or_else(|| {
                CompressionError::ExtractionFailed(
                    "RAR expanded size overflowed the supported range".to_string(),
                )
            })?;
        }
        check_control()?;
        let skipped = header.skip();
        check_control()?;
        archive = skipped.map_err(|error| {
            let detail = super::rar_support::RarSupportService::native_failure_message(&error, password_proven)
                .unwrap_or_else(|| format!("{error:?}"));
            CompressionError::ExtractionFailed(format!("Unable to continue RAR metadata inspection: {detail}"))
        })?;
    }
    Ok((entries, expanded))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genuine_rar4_rar5_volume_continuations_are_not_duplicate_files() {
        let base=Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/archives/sfx-155");
        let manifest:serde_json::Value=serde_json::from_slice(&std::fs::read(base.join("manifest.json")).unwrap()).unwrap();
        for c in manifest["cases"].as_array().unwrap().iter().filter(|c|c["kind"].as_str().unwrap().starts_with("rar")) {
            let actual=stats(&base.join(c["archive"].as_str().unwrap()),c["password"].as_str(),||Ok(())).unwrap();
            assert_eq!(actual,(5,275544),"{}",c["name"]);
            let options=crate::models::compression::DecompressOptions { preserve_paths:true,file_filter:Some("*.txt".into()),..Default::default() };
            assert_eq!(stats_with_options(&base.join(c["archive"].as_str().unwrap()),c["password"].as_str(),&options,||Ok(())).unwrap(),(2,13400));
        }
    }

    fn fixture() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/archives/rar-preflight-1415/stored.rar")
    }

    #[test]
    fn real_rar_metadata_remains_complete_without_extracting() {
        let bytes = std::fs::read(fixture()).unwrap();
        assert_eq!(stats(&fixture(), None, || Ok(())).unwrap(), (1, 29));
        assert_eq!(std::fs::read(fixture()).unwrap(), bytes);
    }

    #[test]
    fn cancellation_precedes_opening_a_missing_source() {
        let error = stats(Path::new("not-opened.rar"), None, || Err(CompressionError::Cancelled.into())).unwrap_err();
        assert!(matches!(error.downcast_ref::<CompressionError>(), Some(CompressionError::Cancelled)));
    }

    #[test]
    fn cancellation_after_a_real_header_is_not_metadata_failure() {
        let mut checks = 0;
        let error = stats(&fixture(), None, || {
            checks += 1;
            if checks == 4 { Err(CompressionError::Cancelled.into()) } else { Ok(()) }
        }).unwrap_err();
        assert_eq!(checks, 4, "stop immediately after the first real SDK header read");
        assert!(matches!(error.downcast_ref::<CompressionError>(), Some(CompressionError::Cancelled)));
    }

    #[test]
    fn malformed_source_still_reports_the_sdk_failure() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("damaged.rar");
        std::fs::write(&path, b"not an archive").unwrap();
        let error = stats(&path, None, || Ok(())).unwrap_err();
        assert!(matches!(error.downcast_ref::<CompressionError>(), Some(CompressionError::ExtractionFailed(_))));
        assert!(error.to_string().contains("Unable to inspect RAR"), "{error:?}");
    }
}
