//! Explicit directory dates captured during the existing TAR preflight scan.
use super::{
    lzop::selected::DirectoryTimes, tar_compatibility::LogicalEntry,
    tar_recovery_time::ModificationTime,
};
use crate::models::compression::DecompressOptions;
use std::{
    collections::HashSet,
    io::Read,
    path::{Component, Path},
};

#[derive(Default)]
pub(crate) struct Collector {
    times: DirectoryTimes,
    seen: HashSet<String>,
}
impl Collector {
    pub(crate) fn observe<R: Read>(
        &mut self,
        entry: &LogicalEntry<'_, R>,
        options: &DecompressOptions,
    ) {
        if !options.preserve_paths
            || !options.preserve_timestamps
            || !entry.header().entry_type().is_dir()
            || !super::extraction_transaction::matches_selected_entries(
                entry.path(),
                &options.selected_entries,
            )
        {
            return;
        }
        // Use the same normal components as the native output path, never an
        // archive's absolute path when applying dates after publication.
        let path = entry
            .path()
            .components()
            .filter_map(|c| {
                if let Component::Normal(n) = c {
                    Some(n)
                } else {
                    None
                }
            })
            .collect::<std::path::PathBuf>();
        if path.as_os_str().is_empty() {
            return;
        }
        let name = path.to_string_lossy().replace('\\', "/");
        let key = name.to_lowercase();
        if !self.seen.insert(key.clone()) {
            self.times.retain(|(name, _)| name.to_lowercase() != key);
            return;
        }
        let time = entry
            .modified()
            .map(|t| {
                let t = filetime::FileTime::from_system_time(t);
                ModificationTime {
                    seconds: t.unix_seconds(),
                    nanoseconds: t.nanoseconds(),
                }
            })
            .or_else(|| ModificationTime::header(entry.header()));
        if let Some(time) = time {
            self.times.push((name, time));
        }
    }
    pub(crate) fn finish(self) -> DirectoryTimes {
        self.times
    }
}
pub(crate) fn retained(
    times: &DirectoryTimes,
    staging: &Path,
    options: &DecompressOptions,
) -> DirectoryTimes {
    if !options.preserve_paths || !options.preserve_timestamps {
        return Vec::new();
    }
    times
        .iter()
        .filter(|(name, _)| staging.join(name).is_dir())
        .cloned()
        .collect()
}
