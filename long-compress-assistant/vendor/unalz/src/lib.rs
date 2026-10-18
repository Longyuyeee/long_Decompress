//! Extractor for the ALZ archive format.
//!
//! Opens ALZ archives — store, DEFLATE, and ALZ-modified bzip2 entries, with
//! optional traditional ZIP encryption and multi-volume splits — and extracts
//! them to disk. Open an archive with [`archive::AlzArchive`] and write entries
//! out with [`extract`].

pub mod archive;
pub mod crypto;
pub mod decompress;
pub mod dostime;
pub mod encoding;
pub mod error;
pub mod extract;
pub mod multivolume;
