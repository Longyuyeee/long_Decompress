//! Entry extraction: path safety, decompression dispatch, and CRC verification.

use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path};

use crate::archive::{AlzArchive, AlzFileEntry, CompressionMethod};
use crate::crypto::ZipCrypto;
use crate::decompress::{bzip2, deflate, raw};
use crate::dostime::dos_datetime_to_systime;
use crate::encoding::password_to_cp949;
use crate::error::{AlzError, AlzResult};

/// Reject paths that could escape the destination directory: parent-dir
/// (`..`) components, absolute roots, or Windows drive prefixes. Backslashes
/// are normalized to `/` by callers before this check.
fn has_unsafe_components(path: &Path) -> bool {
    path.components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    })
}

/// Extract a single file entry from the archive.
pub fn extract_entry(
    archive: &mut AlzArchive,
    entry: &AlzFileEntry,
    dest_dir: &Path,
    password: Option<&str>,
    pipe_mode: bool,
) -> AlzResult<()> {
    let mut crypto = if entry.is_encrypted() {
        let pwd = password.ok_or(AlzError::PasswordNotSet)?;
        let enc_chk = entry.enc_check.as_ref().ok_or(AlzError::PasswordNotSet)?;
        // ZIP keys are derived from the CP949 bytes of the password.
        let pwd_bytes = password_to_cp949(pwd);
        let mut c = ZipCrypto::new(&pwd_bytes);
        if !c.check_header(
            enc_chk,
            entry.file_crc,
            entry.file_time_date,
            entry.has_data_descriptor(),
        ) {
            return Err(AlzError::InvalidPassword);
        }
        // The verify step above consumed the cipher's key state, so start over
        // and re-run the 12-byte header to reach the keystream for the payload.
        let mut c = ZipCrypto::new(&pwd_bytes);
        let mut hdr_copy = *enc_chk;
        c.decrypt(&mut hdr_copy);
        Some(c)
    } else {
        None
    };

    let file_name = entry.file_name.replace('\\', "/");

    // Security: reject parent-dir (`..`) traversal and absolute paths up
    // front, before touching the filesystem.
    if has_unsafe_components(Path::new(&file_name)) {
        return Err(AlzError::PathTraversal(file_name));
    }

    let dest_path = dest_dir.join(&file_name);

    // Defense in depth: confirm the resolved path stays inside dest_dir,
    // catching escapes via a symlinked destination directory.
    if !pipe_mode {
        // Create the destination up front so `-d newdir` works and canonicalize
        // (which requires an existing path) has something to resolve.
        fs::create_dir_all(dest_dir)?;
        let canonical_dest = fs::canonicalize(dest_dir)?;
        // dest_path may not exist yet; resolve via its parent directory.
        let resolved = if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
            fs::canonicalize(parent)?.join(dest_path.file_name().unwrap_or_default())
        } else {
            dest_path.clone()
        };
        if !resolved.starts_with(&canonical_dest) {
            return Err(AlzError::PathTraversal(file_name));
        }
    }

    if entry.is_directory() {
        if !pipe_mode {
            fs::create_dir_all(&dest_path)?;
        }
        return Ok(());
    }

    archive.reader.seek(SeekFrom::Start(entry.data_pos))?;
    let mut limited = (&mut archive.reader).take(entry.compressed_size);

    let crc = if pipe_mode {
        let stdout = io::stdout();
        let mut out = stdout.lock();
        decompress_to(
            &mut limited,
            &mut out,
            entry,
            entry.uncompressed_size,
            crypto.as_mut(),
        )?
    } else {
        let mut file = fs::File::create(&dest_path).map_err(AlzError::CantOpenDestFile)?;
        // On any decompression failure, remove the partial output file so a
        // rejected (e.g. bomb-capped or corrupt) entry leaves nothing behind.
        let crc = match decompress_to(
            &mut limited,
            &mut file,
            entry,
            entry.uncompressed_size,
            crypto.as_mut(),
        ) {
            Ok(crc) => crc,
            Err(e) => {
                drop(file);
                let _ = fs::remove_file(&dest_path);
                return Err(e);
            }
        };
        file.flush().map_err(AlzError::CantOpenDestFile)?;
        if let Some(systime) = dos_datetime_to_systime(entry.file_time_date) {
            let _ = file.set_modified(systime);
        }
        drop(file);

        crc
    };

    if crc != entry.file_crc {
        if !pipe_mode {
            let _ = fs::remove_file(&dest_path);
        }
        return Err(AlzError::InvalidFileCrc {
            expected: entry.file_crc,
            got: crc,
        });
    }

    Ok(())
}

fn decompress_to<R: io::Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    entry: &AlzFileEntry,
    max_output: u64,
    crypto: Option<&mut ZipCrypto>,
) -> AlzResult<u32> {
    match entry.compression_method {
        CompressionMethod::Store => {
            raw::extract_raw(reader, writer, entry.compressed_size, max_output, crypto)
        }
        CompressionMethod::Deflate => {
            deflate::extract_deflate(reader, writer, entry.compressed_size, max_output, crypto)
        }
        CompressionMethod::Bzip2 => {
            bzip2::extract_bzip2(reader, writer, entry.compressed_size, max_output, crypto)
        }
        CompressionMethod::Unknown(n) => Err(AlzError::UnknownCompressionMethod(n)),
    }
}

/// A premature end of data (missing volume or partial download), not a format
/// error; every entry before the cut is intact.
fn is_truncation(e: &AlzError) -> bool {
    matches!(e, AlzError::Io(io) if io.kind() == io::ErrorKind::UnexpectedEof)
}

/// Extract all entries. Returns `Ok(true)` when every entry was written, or
/// `Ok(false)` when the archive was truncated: the complete files before the
/// cut are kept and a warning is printed.
pub fn extract_all(
    archive: &mut AlzArchive,
    dest_dir: &Path,
    password: Option<&str>,
    pipe_mode: bool,
    quiet: bool,
) -> AlzResult<bool> {
    let entries: Vec<AlzFileEntry> = archive.entries.clone();
    let total = entries.len();
    for (done, entry) in entries.iter().enumerate() {
        if !quiet && !pipe_mode {
            eprint!(
                "\nextracting : {} ({}bytes) ",
                entry.file_name, entry.uncompressed_size
            );
        }
        match extract_entry(archive, entry, dest_dir, password, pipe_mode) {
            Ok(()) => {
                if !quiet && !pipe_mode {
                    eprint!(".. ok");
                }
            }
            Err(ref e) if is_truncation(e) => {
                eprintln!(
                    "\nwarning: archive truncated after {done} of {total} files \
                     (missing split volume?); kept the complete files"
                );
                return Ok(false);
            }
            Err(e) => return Err(e),
        }
    }
    // The index itself may have ended on a truncated header, so there is no cut
    // mid-entry to report above yet the archive is still incomplete.
    if archive.truncated {
        eprintln!(
            "\nwarning: archive truncated (missing split volume?); \
             kept the {total} complete files"
        );
        return Ok(false);
    }
    Ok(true)
}

/// Extract specific files by name.
pub fn extract_files(
    archive: &mut AlzArchive,
    dest_dir: &Path,
    file_names: &[String],
    password: Option<&str>,
    pipe_mode: bool,
    quiet: bool,
) -> AlzResult<bool> {
    let entries: Vec<AlzFileEntry> = archive.entries.clone();
    let mut all_matched = true;
    for name in file_names {
        if let Some(entry) = entries.iter().find(|e| e.file_name == *name) {
            if !quiet && !pipe_mode {
                eprint!(
                    "\nextracting : {} ({}bytes) ",
                    entry.file_name, entry.uncompressed_size
                );
            }
            match extract_entry(archive, entry, dest_dir, password, pipe_mode) {
                Ok(()) => {
                    if !quiet && !pipe_mode {
                        eprint!(".. ok");
                    }
                }
                Err(ref e) if is_truncation(e) => {
                    eprintln!(
                        "\nwarning: {name} is truncated (missing split volume?); \
                         its data is incomplete"
                    );
                    return Ok(false);
                }
                Err(e) => return Err(e),
            }
        } else {
            all_matched = false;
            if !quiet && !pipe_mode {
                eprintln!("\nfilename not matched : {name}");
            }
        }
    }
    Ok(all_matched)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsafe_components_are_rejected() {
        // Callers normalize '\\' to '/' before this check.
        for p in ["../etc", "a/../../b", "..", "a/..", "/etc/passwd"] {
            assert!(has_unsafe_components(Path::new(p)), "should reject {p}");
        }
        for p in ["a/b/c.txt", "한글.txt", "deep/dir/file", "a.b/c"] {
            assert!(!has_unsafe_components(Path::new(p)), "should allow {p}");
        }
    }
}
