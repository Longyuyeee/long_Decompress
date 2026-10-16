//! Metadata inspection runs on a blocking worker. Control checks surround SDK
//! calls; they cannot interrupt a single blocking SDK / filesystem operation.
use super::compression_service::CompressionError;
use anyhow::Result;
use std::path::Path;

pub(crate) fn stats(
    path: &Path,
    password: Option<&str>,
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
    loop {
        check_control()?;
        let header = archive.read_header();
        // Prefer a requested cancellation to a coincident SDK failure.
        check_control()?;
        let Some(header) = header.map_err(|error| {
            CompressionError::ExtractionFailed(format!(
                "Unable to inspect RAR entry metadata: {:?}", error
            ))
        })? else { break };
        entries = entries.saturating_add(1);
        expanded = expanded.checked_add(header.entry().unpacked_size).ok_or_else(|| {
            CompressionError::ExtractionFailed(
                "RAR expanded size overflowed the supported range".to_string(),
            )
        })?;
        check_control()?;
        let skipped = header.skip();
        check_control()?;
        archive = skipped.map_err(|error| {
            CompressionError::ExtractionFailed(format!(
                "Unable to continue RAR metadata inspection: {:?}", error
            ))
        })?;
    }
    Ok((entries, expanded))
}

#[cfg(test)]
mod tests {
    use super::*;

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
