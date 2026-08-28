use std::path::PathBuf;

use anyhow::Result;

pub(crate) fn move_paths_to_system_recycle_bin(paths: &[PathBuf]) -> Result<usize> {
    if paths.is_empty() {
        return Ok(0);
    }
    for path in paths {
        if !path.exists() {
            anyhow::bail!("source no longer exists: {}", path.display());
        }
    }

    trash::delete_all(paths).map_err(|error| {
        anyhow::anyhow!("Windows system Recycle Bin rejected the source: {}", error)
    })?;
    Ok(paths.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_source_set_is_a_noop() {
        assert_eq!(move_paths_to_system_recycle_bin(&[]).unwrap(), 0);
    }

    #[test]
    fn missing_source_is_rejected_before_recycle_bin_request() {
        let temp = tempfile::tempdir().expect("temp directory");
        let missing = temp.path().join("missing.bin");

        let error = move_paths_to_system_recycle_bin(std::slice::from_ref(&missing))
            .expect_err("missing source must fail");

        assert!(error.to_string().contains(&missing.display().to_string()));
    }
}
