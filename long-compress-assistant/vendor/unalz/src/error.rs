//! Error type returned across the crate.

use std::fmt;

/// An error from opening, parsing, or extracting an ALZ archive.
#[derive(Debug)]
pub enum AlzError {
    /// The input does not start with an ALZ signature.
    NotAlzFile,
    /// A header or field is malformed or inconsistent.
    CorruptedFile,
    /// The archive file (or a volume) could not be opened.
    CantOpenFile(std::io::Error),
    /// The destination file could not be created or written.
    CantOpenDestFile(std::io::Error),
    /// A local file header declares an out-of-range filename length.
    InvalidFilenameLength,
    /// DEFLATE decompression failed.
    InflateFailed(String),
    /// bzip2 decompression failed.
    Bzip2Failed(String),
    /// The decompressed data does not match the stored CRC32.
    InvalidFileCrc {
        /// CRC32 stored in the archive.
        expected: u32,
        /// CRC32 of the decompressed data.
        got: u32,
    },
    /// The size-field width bits in a file descriptor are not 0/1/2/4/8 bytes.
    InvalidSizeFieldWidth(u8),
    /// The entry uses a compression method this extractor does not support.
    UnknownCompressionMethod(u8),
    /// Decompression produced more than the declared uncompressed size.
    UncompressedSizeExceeded {
        /// The declared uncompressed size that was exceeded.
        limit: u64,
    },
    /// The entry count exceeds what the archive size could hold.
    TooManyEntries,
    /// An encrypted entry was reached without a password.
    PasswordNotSet,
    /// The password failed the encryption header check.
    InvalidPassword,
    /// An entry name would escape the destination directory.
    PathTraversal(String),
    /// An underlying I/O error.
    Io(std::io::Error),
}

impl fmt::Display for AlzError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAlzFile => write!(f, "not an ALZ file"),
            Self::CorruptedFile => write!(f, "corrupted file"),
            Self::CantOpenFile(e) => write!(f, "can't open archive file: {e}"),
            Self::CantOpenDestFile(e) => write!(f, "can't open dest file: {e}"),
            Self::InvalidFilenameLength => write!(f, "invalid filename length"),
            Self::InflateFailed(s) => write!(f, "inflate failed: {s}"),
            Self::Bzip2Failed(s) => write!(f, "bzip2 decompress failed: {s}"),
            Self::InvalidFileCrc { expected, got } => {
                write!(
                    f,
                    "invalid file CRC: expected {expected:08x}, got {got:08x}"
                )
            }
            Self::InvalidSizeFieldWidth(v) => {
                write!(f, "invalid size field width: 0x{v:02x}")
            }
            Self::UnknownCompressionMethod(m) => write!(f, "unknown compression method: {m}"),
            Self::UncompressedSizeExceeded { limit } => {
                write!(
                    f,
                    "decompressed output exceeds declared size ({limit} bytes)"
                )
            }
            Self::TooManyEntries => write!(f, "too many file entries for archive size"),
            Self::PasswordNotSet => write!(f, "password was not set"),
            Self::InvalidPassword => write!(f, "invalid password"),
            Self::PathTraversal(p) => write!(f, "path traversal blocked: {p}"),
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

impl AlzError {
    /// True when this ultimately came from writing to a closed pipe, whichever
    /// I/O-carrying variant wrapped it.
    pub fn is_broken_pipe(&self) -> bool {
        matches!(
            self,
            Self::Io(e) | Self::CantOpenFile(e) | Self::CantOpenDestFile(e)
            if e.kind() == std::io::ErrorKind::BrokenPipe
        )
    }
}

impl std::error::Error for AlzError {}

impl From<std::io::Error> for AlzError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

pub type AlzResult<T> = Result<T, AlzError>;
