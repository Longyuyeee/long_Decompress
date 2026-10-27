//! File modification times for prefix recovery. Content success is independent
//! of the destination filesystem's timestamp range and precision.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModificationTime {
    pub seconds: i64,
    pub nanoseconds: u32,
}
impl ModificationTime {
    pub(crate) fn pax(value: &str) -> Option<Self> {
        let negative = value.starts_with('-');
        let value = value.strip_prefix(['-', '+']).unwrap_or(value);
        let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
        if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit())
            || !fraction.bytes().all(|b| b.is_ascii_digit()) { return None; }
        let whole: i64 = whole.parse().ok()?;
        if whole > 1_000_000_000_000 { return None; }
        // PAX decimal fractions are exact; do not round through f64. More than
        // nine meaningful digits cannot be represented without losing time.
        if fraction.get(9..).is_some_and(|tail| tail.bytes().any(|b| b != b'0')) { return None; }
        let mut nanos = 0;
        for i in 0..9 { nanos = nanos * 10 + fraction.as_bytes().get(i).map_or(0, |b| (b - b'0') as u32); }
        if negative && nanos != 0 { Some(Self { seconds: -whole - 1, nanoseconds: 1_000_000_000 - nanos }) }
        else { Some(Self { seconds: if negative { -whole } else { whole }, nanoseconds: nanos }) }
    }
    pub(crate) fn header(header: &tar::Header) -> Option<Self> {
        let raw = &header.as_old().mtime;
        let seconds = if raw[0] & 0x80 != 0 {
            // GNU base-256 includes a marker bit and a signed two's-complement
            // value. tar-rs's public mtime accessor only returns unsigned u64.
            let negative = raw[0] & 0x40 != 0;
            let mut value = [if negative { 0xff } else { 0 }; 16];
            value[16-raw.len()..].copy_from_slice(raw);
            if !negative { value[16-raw.len()] &= 0x7f; }
            i64::try_from(i128::from_be_bytes(value)).ok()?
        } else { i64::try_from(header.mtime().ok()?).ok()? };
        if !(-1_000_000_000_000..=1_000_000_000_000).contains(&seconds) { return None; }
        Some(Self { seconds, nanoseconds: 0 })
    }
    pub(crate) fn apply(&self, file: &std::fs::File) -> Result<(), &'static str> {
        #[cfg(windows)]
        if self.seconds < -11_644_473_600 {
            return Err("原修改时间早于 Windows 支持范围，文件内容已取回");
        }
        let target = filetime::FileTime::from_unix_time(self.seconds, self.nanoseconds);
        filetime::set_file_handle_times(file, None, Some(target))
            .map_err(|_| "保存位置无法写入原修改时间，文件内容已取回")?;
        let actual = file.metadata().map_err(|_| "无法确认保存后的修改时间，文件内容已取回")?;
        if filetime::FileTime::from_last_modification_time(&actual) != target {
            return Err("保存位置的时间精度或范围有限，原修改时间未完整保留，文件内容已取回");
        }
        Ok(())
    }
    pub(crate) fn apply_directory(&self, path: &std::path::Path) -> Result<(), &'static str> {
        let mut options = std::fs::OpenOptions::new();
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            // Directory handles need backup semantics; request only read/write
            // attributes, not write access to the directory's child entries.
            options.access_mode(0x80 | 0x100).custom_flags(0x02000000);
        }
        #[cfg(not(windows))]
        options.read(true);
        let file = options.open(path).map_err(|_| "保存位置无法打开目录以保留修改时间，已取回内容仍保留")?;
        self.apply(&file).map_err(|message| match message {
            "原修改时间早于 Windows 支持范围，文件内容已取回" => "目录原修改时间早于 Windows 支持范围，已取回内容仍保留",
            "保存位置无法写入原修改时间，文件内容已取回" => "保存位置无法写入目录原修改时间，已取回内容仍保留",
            "无法确认保存后的修改时间，文件内容已取回" => "无法确认保存后的目录修改时间，已取回内容仍保留",
            _ => "保存位置的时间精度或范围有限，目录原修改时间未完整保留，已取回内容仍保留",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimePreservation {
    pub preserved_files: usize,
    pub unpreserved_files: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preserved_directories: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unpreserved_directories: Option<usize>,
}
impl TimePreservation {
    pub(crate) fn valid_for(&self, files: usize, directories: usize) -> bool {
        let directory_counts_valid = match (self.preserved_directories, self.unpreserved_directories) {
            (None, None) => true,
            (Some(preserved), Some(unpreserved)) => preserved <= directories && unpreserved == directories - preserved,
            _ => false,
        };
        self.preserved_files <= files && self.unpreserved_files == files - self.preserved_files && directory_counts_valid
    }
}
