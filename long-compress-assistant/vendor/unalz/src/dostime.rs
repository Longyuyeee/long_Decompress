//! DOS timestamp conversion.

use std::time::SystemTime;

use chrono::{Local, TimeZone};

/// Convert DOS date/time (as stored in ALZ headers) to SystemTime.
/// DOS time format:
///   bits 0-4:  seconds/2 (0-29)
///   bits 5-10: minutes (0-59)
///   bits 11-15: hours (0-23)
///   bits 16-20: day (1-31)
///   bits 21-24: month (1-12)
///   bits 25-31: year offset from 1980
///
/// DOS time carries no zone; it is local wall-clock time.
pub fn dos_datetime_to_systime(dostime: u32) -> Option<SystemTime> {
    let (year, month, day, hour, min, sec) = split_fields(dostime)?;
    // A DST gap makes the stamp nonexistent locally; a DST overlap makes it
    // ambiguous, where the earlier reading is the conventional choice.
    let local = Local
        .with_ymd_and_hms(year as i32, month, day, hour, min, sec)
        .earliest()?;
    let secs = u64::try_from(local.timestamp()).ok()?;
    Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs))
}

fn split_fields(dostime: u32) -> Option<(u32, u32, u32, u32, u32, u32)> {
    let sec = (dostime & 0x1f) << 1;
    let min = (dostime >> 5) & 0x3f;
    let hour = (dostime >> 11) & 0x1f;
    let day = (dostime >> 16) & 0x1f;
    let month = (dostime >> 21) & 0x0f;
    let year = ((dostime >> 25) & 0x7f) + 1980;

    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some((year, month, day, hour, min, sec))
}

/// Format DOS datetime for display (YYYY-MM-DD HH:MM:SS).
pub fn dos_datetime_to_string(dostime: u32) -> String {
    let sec = (dostime & 0x1f) << 1;
    let min = (dostime >> 5) & 0x3f;
    let hour = (dostime >> 11) & 0x1f;
    let day = (dostime >> 16) & 0x1f;
    let month = (dostime >> 21) & 0x0f;
    let year = ((dostime >> 25) & 0x7f) + 1980;

    format!("{year:04}-{month:02}-{day:02} {hour:02}:{min:02}:{sec:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dos_datetime_to_string() {
        // 2019-04-12 04:16:18
        assert_eq!(dos_datetime_to_string(0x4E8C2209), "2019-04-12 04:16:18");
        // 1980-01-01 00:00:00 (minimum DOS date)
        assert_eq!(dos_datetime_to_string(0x00210000), "1980-01-01 00:00:00");
    }

    #[test]
    fn test_dos_datetime_to_systime() {
        // 2019-04-12 04:16:18 local time, whatever the host timezone is.
        use chrono::{DateTime, Datelike, Timelike};

        let t = dos_datetime_to_systime(0x4E8C2209).unwrap();
        let dt: DateTime<Local> = t.into();
        assert_eq!(
            (
                dt.year(),
                dt.month(),
                dt.day(),
                dt.hour(),
                dt.minute(),
                dt.second()
            ),
            (2019, 4, 12, 4, 16, 18)
        );
    }

    #[test]
    fn test_invalid_date() {
        // Month 0 is invalid
        assert!(dos_datetime_to_systime(0).is_none());
    }
}
