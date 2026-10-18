#![allow(missing_docs)]

use super::*;
use std::error;
use std::ffi;
use std::fmt;
use std::result::Result;


#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[repr(i32)]
pub enum Code {
    Success = native::ERAR_SUCCESS,
    EndArchive = native::ERAR_END_ARCHIVE,
    NoMemory = native::ERAR_NO_MEMORY,
    BadData = native::ERAR_BAD_DATA,
    BadArchive = native::ERAR_BAD_ARCHIVE,
    UnknownFormat = native::ERAR_UNKNOWN_FORMAT,
    EOpen = native::ERAR_EOPEN,
    ECreate = native::ERAR_ECREATE,
    EClose = native::ERAR_ECLOSE,
    ERead = native::ERAR_EREAD,
    EWrite = native::ERAR_EWRITE,
    SmallBuf = native::ERAR_SMALL_BUF,
    Unknown = native::ERAR_UNKNOWN,
    MissingPassword = native::ERAR_MISSING_PASSWORD,
    // From the UnRARDLL docs:
    // When attempting to unpack a reference record (see RAR -oi switch),
    // source file for this reference was not found.
    // Entire archive needs to be unpacked to properly create file references.
    // This error is returned when attempting to unpack the reference
    // record without its source file.
    EReference = native::ERAR_EREFERENCE,
    BadPassword = native::ERAR_BAD_PASSWORD,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum When {
    Open,
    Read,
    Process,
}

impl Code {
    pub fn from(code: i32) -> Option<Self> {
        use Code::*;
        match code {
            native::ERAR_SUCCESS => Some(Success),
            native::ERAR_END_ARCHIVE => Some(EndArchive),
            native::ERAR_NO_MEMORY => Some(NoMemory),
            native::ERAR_BAD_DATA => Some(BadData),
            native::ERAR_BAD_ARCHIVE => Some(BadArchive),
            native::ERAR_UNKNOWN_FORMAT => Some(UnknownFormat),
            native::ERAR_EOPEN => Some(EOpen),
            native::ERAR_ECREATE => Some(ECreate),
            native::ERAR_ECLOSE => Some(EClose),
            native::ERAR_EREAD => Some(ERead),
            native::ERAR_EWRITE => Some(EWrite),
            native::ERAR_SMALL_BUF => Some(SmallBuf),
            native::ERAR_UNKNOWN => Some(Unknown),
            native::ERAR_MISSING_PASSWORD => Some(MissingPassword),
            native::ERAR_EREFERENCE => Some(EReference),
            native::ERAR_BAD_PASSWORD => Some(BadPassword),
            _ => None,
        }
    }
}

#[derive(PartialEq)]
pub struct UnrarError {
    pub code: Code,
    pub when: When,
    /// Last native volume notification, or an actual failed open request.
    pub volume: Option<VolumeContext>,
}

#[derive(Debug, PartialEq)]
pub struct VolumeContext {
    pub path: std::path::PathBuf,
    pub requested: bool,
    /// Snapshot at failure time; restoring a volume cannot rewrite history.
    pub unavailable_kind: Option<std::io::ErrorKind>,
}

impl std::error::Error for UnrarError {}

impl fmt::Debug for UnrarError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}@{:?}", self.code, self.when)?;
        write!(f, " ({})", self)
    }
}

impl fmt::Display for UnrarError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Code::*;
        use self::When::*;
        if let Some(message) = self.volume_failure_message() { return write!(f, "{message}"); }
        match (self.code, self.when) {
            (BadData, Open) => write!(f, "Archive header damaged"),
            (BadData, Read) => write!(f, "File header damaged"),
            (BadData, Process) => write!(f, "File CRC error"),
            (UnknownFormat, Open) => write!(f, "Unknown encryption"),
            (EOpen, Process) => write!(f, "Could not open next volume"),
            (UnknownFormat, _) => write!(f, "Unknown archive format"),
            (EOpen, _) => write!(f, "Could not open archive"),
            (NoMemory, _) => write!(f, "Not enough memory"),
            (BadArchive, _) => write!(f, "Not a RAR archive"),
            (ECreate, _) => write!(f, "Could not create file"),
            (EClose, _) => write!(f, "Could not close file"),
            (ERead, _) => write!(f, "Read error"),
            (EWrite, _) => write!(f, "Write error"),
            (SmallBuf, _) => write!(f, "Archive comment was truncated to fit to buffer"),
            (MissingPassword, _) => write!(f, "Password for encrypted archive not specified"),
            (EReference, _) => write!(f, "Cannot open file source for reference record"),
            (BadPassword, _) => write!(f, "Wrong password was specified"),
            (Unknown, _) => write!(f, "Unknown error"),
            (EndArchive, _) => write!(f, "Archive end"),
            (Success, _) => write!(f, "Success"),
        }?;
        if let Some(volume) = &self.volume { write!(f, "; 分卷：{}", volume.path.display())?; }
        Ok(())
    }
}

impl UnrarError {
    pub fn from(code: Code, when: When) -> Self {
        UnrarError { code, when, volume: None }
    }

    pub fn with_volume(mut self, mut volume: Option<VolumeContext>) -> Self {
        if let Some(v) = volume.as_mut().filter(|v| v.requested) {
            v.unavailable_kind = match std::fs::metadata(&v.path) {
                Err(error) => Some(error.kind()),
                Ok(metadata) if !metadata.is_file() => Some(std::io::ErrorKind::InvalidInput),
                Ok(_) => None,
            };
        }
        self.volume = volume;
        self
    }

    pub fn volume_failure_message(&self) -> Option<String> {
        let volume = self.volume.as_ref().filter(|v| v.requested)?;
        if volume.unavailable_kind == Some(std::io::ErrorKind::NotFound) {
            let name = volume.path.file_name().unwrap_or(volume.path.as_os_str()).to_string_lossy();
            let parent = volume.path.parent().unwrap_or(std::path::Path::new("."));
            Some(format!("[archive-inspection:missing-volume] 缺少 RAR 分卷：{name}。请补回同组原卷到 {}，保持原文件名，再创建新任务重试", parent.display()))
        } else {
            let category = if volume.unavailable_kind == Some(std::io::ErrorKind::PermissionDenied) { "permission" } else { "io" };
            Some(format!("[archive-inspection:{category}] 无法打开 RAR 请求的分卷：{}；请检查文件是否可读、是否仍在下载及是否属于同一原始卷组（{:?}）", volume.path.display(), volume.unavailable_kind))
        }
    }
}

pub type UnrarResult<T> = Result<T, UnrarError>;

#[derive(Debug)]
pub struct NulError(usize);

impl fmt::Display for NulError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "nul value found at position: {}", self.0)
    }
}

impl error::Error for NulError {
    fn description(&self) -> &str {
        "nul value found"
    }
}

impl<C> From<widestring::error::ContainsNul<C>> for NulError {
    fn from(e: widestring::error::ContainsNul<C>) -> NulError {
        NulError(e.nul_position())
    }
}

impl From<ffi::NulError> for NulError {
    fn from(e: ffi::NulError) -> NulError {
        NulError(e.nul_position())
    }
}
