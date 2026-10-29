//! One timestamp policy for native ZIP metadata, files and explicit directories.
//!
//! Use a single well-formed central UT modification field. No local-header
//! scan is needed, and the same choice reaches sequential and parallel copies.
use std::{
    io::Read,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub(crate) fn utc_modified(extra: Option<&[u8]>) -> Option<SystemTime> {
    let mut extra = extra?;
    let mut timestamp = None;
    let mut seen = false;
    while !extra.is_empty() {
        let header = extra.get(..4)?;
        let kind = u16::from_le_bytes([header[0], header[1]]);
        let len = u16::from_le_bytes([header[2], header[3]]) as usize;
        let value = extra.get(4..4 + len)?;
        extra = &extra[4 + len..];
        if kind != 0x5455 {
            continue;
        }
        // Repeated fields have no single authoritative modification time.
        if seen {
            return None;
        }
        seen = true;
        // Central UT contains mtime only; the other low bits describe the
        // local field, so flags 3/7 with a five-byte central value are valid.
        if value.len() != 5 || value[0] & 1 == 0 || value[0] & !7 != 0 {
            return None;
        }
        let seconds = u32::from_le_bytes(value[1..5].try_into().ok()?);
        // The specification uses signed seconds, while some producers use
        // unsigned high-bit values. Do not guess between pre-1970 and 2038+.
        let seconds = i32::try_from(seconds).ok()? as u64;
        timestamp = UNIX_EPOCH.checked_add(Duration::from_secs(seconds));
    }
    timestamp
}

pub(crate) fn modified<R: Read + ?Sized>(
    entry: &zip_aes::read::ZipFile<'_, R>,
) -> Option<SystemTime> {
    utc_modified(entry.extra_data()).or_else(|| {
        entry.last_modified().and_then(|t| {
            super::native_extraction::zip::system_time(
                t.year(),
                t.month(),
                t.day(),
                t.hour(),
                t.minute(),
                t.second(),
            )
        })
    })
}

pub(crate) fn display<R: Read + ?Sized>(entry: &zip_aes::read::ZipFile<'_, R>) -> Option<String> {
    if let Some(time) = utc_modified(entry.extra_data()) {
        return Some(chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339());
    }
    entry.last_modified().map(|t| {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            t.year(),
            t.month(),
            t.day(),
            t.hour(),
            t.minute(),
            t.second()
        )
    })
}

/// Comparison rows use the same local calendar format as filesystem rows.
pub(crate) fn local_display<R: Read + ?Sized>(
    entry: &zip_aes::read::ZipFile<'_, R>,
) -> Option<String> {
    if let Some(time) = utc_modified(entry.extra_data()) {
        return Some(
            chrono::DateTime::<chrono::Local>::from(time)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
        );
    }
    display(entry)
}
