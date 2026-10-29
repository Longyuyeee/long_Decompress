//! Source modification time: confirmed UTC seconds plus the legacy DOS calendar.
use chrono::{Datelike, Timelike};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) struct SourceTime {
    pub date: zip_aes::DateTime,
    pub clamped: bool,
    pub utc_seconds: Option<u32>,
}
impl SourceTime {
    pub(crate) fn file_options<'k>(
        &self,
        options: zip_aes::write::FileOptions<'k, ()>,
    ) -> zip_aes::result::ZipResult<zip_aes::write::FullFileOptions<'k>> {
        let mut options = options.last_modified_time(self.date).into_full_options();
        if let Some(seconds) = self.utc_seconds {
            let mut value = [0u8; 5];
            value[0] = 1;
            value[1..].copy_from_slice(&seconds.to_le_bytes());
            // A non-central-only field is written to both headers by the writer.
            options.add_extra_data(0x5455, value, false)?;
        }
        Ok(options)
    }
}
pub(crate) fn from_system_time(time: SystemTime) -> SourceTime {
    let utc_seconds = time
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|time| time.as_secs())
        .filter(|seconds| *seconds <= i32::MAX as u64)
        .map(|seconds| seconds as u32);
    let local: chrono::DateTime<chrono::Local> = time.into();
    let (fields, clamped) = if local.year() < 1980 {
        ((1980, 1, 1, 0, 0, 0), true)
    } else if local.year() > 2107 {
        ((2107, 12, 31, 23, 59, 58), true)
    } else {
        (
            (
                local.year() as u16,
                local.month() as u8,
                local.day() as u8,
                local.hour() as u8,
                local.minute() as u8,
                (local.second().min(59) as u8) & !1,
            ),
            false,
        )
    };
    let (year, month, day, hour, minute, second) = fields;
    SourceTime {
        date: zip_aes::DateTime::from_date_and_time(year, month, day, hour, minute, second)
            .expect("valid local calendar within DOS date range"),
        clamped,
        utc_seconds,
    }
}
