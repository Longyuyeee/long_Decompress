//! Extractor for the EGG archive format.
//!
//! Opens EGG archives (Store, Deflate, Bzip2, LZMA, and AZO entries, with
//! optional ZipCrypto/AES/LEA encryption, solid and split archives) and
//! extracts them to disk. Open an archive with [`archive::EggArchive`] and
//! write entries out with [`extract`].

pub mod archive;
pub mod crypto;
pub mod encoding;
pub mod error;

pub mod decompress;
pub mod aes_ctr;
pub mod lea;
