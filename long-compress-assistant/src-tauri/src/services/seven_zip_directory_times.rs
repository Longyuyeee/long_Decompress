//! Explicit directory dates captured in the existing 7Z extraction callbacks.
use super::{lzop::selected::DirectoryTimes, tar_recovery_time::ModificationTime};
use crate::models::compression::DecompressOptions;
use std::{
    collections::HashSet,
    path::{Component, Path},
};

#[derive(Default)]
pub(crate) struct Collector {
    times: DirectoryTimes,
    seen: HashSet<String>,
}

impl Collector {
    pub(crate) fn observe(
        &mut self,
        entry: &sevenz_rust::SevenZArchiveEntry,
        relative: &Path,
        options: &DecompressOptions,
    ) {
        if !options.preserve_paths
            || !options.preserve_timestamps
            || !entry.is_directory()
            || entry.is_anti_item
            || !super::extraction_transaction::matches_selected_entries(
                Path::new(entry.name()),
                &options.selected_entries,
            )
            || relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return;
        }
        let name = relative.to_string_lossy().replace('\\', "/");
        let key = name.to_lowercase();
        if !self.seen.insert(key.clone()) {
            self.times.retain(|(name, _)| name.to_lowercase() != key);
            return;
        }
        if entry.has_last_modified_date {
            let time: std::time::SystemTime = entry.last_modified_date().into();
            let time = filetime::FileTime::from_system_time(time);
            self.times.push((
                name,
                ModificationTime {
                    seconds: time.unix_seconds(),
                    nanoseconds: time.nanoseconds(),
                },
            ));
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
