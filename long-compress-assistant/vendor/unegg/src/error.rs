use std::fmt;

#[derive(Debug)]
pub enum EggError {
    UnsupportedFeature(String),
    NotEggFile,
    CorruptedFile,
    CantOpenFile(std::io::Error),
    CantOpenDestFile(std::io::Error),
    InvalidFileCrc {
        expected: u32,
        got: u32,
    },
    UnknownCompressionMethod(u8),
    UnsupportedEncryption(u8),
    PasswordNotSet,
    InvalidPassword,
    AuthenticationFailed,
    PathTraversal(String),
    Bzip2Failed(String),
    InflateFailed(String),
    LzmaFailed(String),
    AzoFailed(String),
    /// A name requested on the command line matched no entry.
    FileNotFound(String),
    Io(std::io::Error),
}

pub type EggResult<T> = Result<T, EggError>;

impl fmt::Display for EggError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFeature(s) => write!(f, "unsupported EGG feature: {s}"),
            Self::NotEggFile => write!(f, "not an EGG file"),
            Self::CorruptedFile => write!(f, "corrupted file"),
            Self::CantOpenFile(e) => write!(f, "can't open file: {e}"),
            Self::CantOpenDestFile(e) => write!(f, "can't open destination file: {e}"),
            Self::InvalidFileCrc { expected, got } => {
                write!(f, "CRC mismatch: expected {expected:08x}, got {got:08x}")
            }
            Self::UnknownCompressionMethod(m) => {
                write!(f, "unknown compression method: {m}")
            }
            Self::UnsupportedEncryption(m) => {
                write!(f, "unsupported encryption method: {m}")
            }
            Self::PasswordNotSet => write!(f, "password not set"),
            Self::InvalidPassword => write!(f, "invalid password"),
            Self::AuthenticationFailed => write!(f, "authentication failed (HMAC mismatch)"),
            Self::PathTraversal(p) => write!(f, "path traversal: {p}"),
            Self::Bzip2Failed(e) => write!(f, "bzip2 failed: {e}"),
            Self::InflateFailed(e) => write!(f, "inflate failed: {e}"),
            Self::LzmaFailed(e) => write!(f, "LZMA failed: {e}"),
            Self::AzoFailed(e) => write!(f, "AZO failed: {e}"),
            Self::FileNotFound(name) => write!(f, "file not found in archive: {name}"),
            Self::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl EggError {
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

impl std::error::Error for EggError {}

impl From<std::io::Error> for EggError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
