use std::fs;
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};

use hmac::{Hmac, KeyInit, Mac};
use sha1::Sha1;

use crate::aes_ctr::AesCtrDecryptor;
use crate::archive::{CompressionMethod, EggArchive, EggBlock, EggFileEntry, EncryptionMethod};
use crate::crypto::{Decryptor, ZipCrypto};
use crate::decompress;
use crate::error::{EggError, EggResult};
use crate::lea::LeaCtrDecryptor;

/// A file's decryptor plus, for AES/LEA, the material needed to verify the
/// WinZip AE-2 HMAC-SHA1 authentication footer.
struct FileCrypto {
    decryptor: Box<dyn Decryptor>,
    auth: Option<AuthInfo>,
}

struct AuthInfo {
    /// PBKDF2-derived HMAC key.
    key: Vec<u8>,
    /// Stored 10-byte authentication footer (truncated HMAC-SHA1 tag).
    tag: Vec<u8>,
}

/// Verify the AE-2 authentication footer: HMAC-SHA1(auth_key, ciphertext) over
/// all of the file's encrypted block bytes, truncated to the stored 10 bytes.
fn verify_hmac<R: Read + Seek>(
    reader: &mut R,
    entry: &EggFileEntry,
    auth: &AuthInfo,
) -> EggResult<()> {
    let mut mac = <Hmac<Sha1>>::new_from_slice(&auth.key).map_err(|_| EggError::CorruptedFile)?;
    let mut buf = [0u8; 8192];
    for block in &entry.blocks {
        reader.seek(SeekFrom::Start(block.data_pos))?;
        let mut remaining = block.compressed_size as usize;
        while remaining > 0 {
            let n = remaining.min(buf.len());
            reader.read_exact(&mut buf[..n])?;
            mac.update(&buf[..n]);
            remaining -= n;
        }
    }
    let computed = mac.finalize().into_bytes();
    if auth.tag.len() != 10 || computed.len() < 10 || computed[..10] != auth.tag[..] {
        return Err(EggError::AuthenticationFailed);
    }
    Ok(())
}

/// Join an archive entry name onto `dest` safely, rejecting any attempt to
/// escape the destination directory (`..`, absolute paths, Windows drive/UNC
/// prefixes). The path is rebuilt from `Normal` components only, so the result
/// is always contained within `dest`.
fn safe_join(dest: &Path, name: &str) -> EggResult<PathBuf> {
    let mut out = dest.to_path_buf();
    for comp in Path::new(name).components() {
        match comp {
            Component::Normal(c) => out.push(c),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(EggError::PathTraversal(name.to_string()));
            }
        }
    }
    // An entry with no usable name would resolve to the destination directory
    // itself, and writing there would clobber it.
    if out == dest {
        return Err(EggError::CorruptedFile);
    }
    Ok(out)
}

/// The 10-byte AE-2 authentication footer follows salt + 2-byte verifier in the
/// Encrypt Info data. Returns None if the archive omitted it (older layout).
fn auth_footer(data: &[u8], salt_size: usize) -> Option<Vec<u8>> {
    let start = salt_size + 2;
    data.get(start..start + 10).map(<[u8]>::to_vec)
}

fn setup_decryptor(entry: &EggFileEntry, password: Option<&str>) -> EggResult<Option<FileCrypto>> {
    let ei = match entry.encrypt_info {
        Some(ref ei) => ei,
        None => return Ok(None),
    };
    let pwd = password.ok_or(EggError::PasswordNotSet)?;

    let build_aes = |salt_size: usize, mode: u8| -> EggResult<FileCrypto> {
        if ei.data.len() < salt_size + 2 {
            return Err(EggError::CorruptedFile);
        }
        let salt = &ei.data[..salt_size];
        let verifier: [u8; 2] = ei.data[salt_size..salt_size + 2].try_into().unwrap();
        let dec = AesCtrDecryptor::new(mode, pwd, salt, &verifier)?;
        let auth = auth_footer(&ei.data, salt_size).map(|tag| AuthInfo {
            key: dec.auth_key().to_vec(),
            tag,
        });
        Ok(FileCrypto {
            decryptor: Box::new(dec),
            auth,
        })
    };

    let build_lea = |salt_size: usize, mode: u8| -> EggResult<FileCrypto> {
        if ei.data.len() < salt_size + 2 {
            return Err(EggError::CorruptedFile);
        }
        let salt = &ei.data[..salt_size];
        let verifier: [u8; 2] = ei.data[salt_size..salt_size + 2].try_into().unwrap();
        let dec = LeaCtrDecryptor::new(mode, pwd, salt, &verifier)?;
        let auth = auth_footer(&ei.data, salt_size).map(|tag| AuthInfo {
            key: dec.auth_key().to_vec(),
            tag,
        });
        Ok(FileCrypto {
            decryptor: Box::new(dec),
            auth,
        })
    };

    match ei.method {
        EncryptionMethod::ZipCrypto => {
            let mut zc = ZipCrypto::new(pwd.as_bytes());
            if ei.data.len() < 16 {
                return Err(EggError::CorruptedFile);
            }
            let verify: [u8; 12] = ei.data[..12].try_into().unwrap();
            let stored_crc = u32::from_le_bytes(ei.data[12..16].try_into().unwrap());
            if !zc.check_password(&verify, stored_crc) {
                return Err(EggError::InvalidPassword);
            }
            Ok(Some(FileCrypto {
                decryptor: Box::new(zc),
                auth: None,
            }))
        }
        EncryptionMethod::Aes128 => Ok(Some(build_aes(8, 1)?)),
        EncryptionMethod::Aes256 => Ok(Some(build_aes(16, 3)?)),
        EncryptionMethod::Lea128 => Ok(Some(build_lea(8, 1)?)),
        EncryptionMethod::Lea256 => Ok(Some(build_lea(16, 3)?)),
        EncryptionMethod::Unknown(n) => Err(EggError::UnsupportedEncryption(n)),
    }
}

/// Check `password` against the archive's encryption by verifying it on the
/// first encrypted entry. Returns `Ok(false)` for a wrong password, `Ok(true)`
/// if it matches or nothing is encrypted. Used to validate an interactively
/// prompted password before extraction begins.
pub fn verify_password(entries: &[EggFileEntry], password: &str) -> EggResult<bool> {
    for entry in entries {
        if entry.encrypt_info.is_some() {
            return match setup_decryptor(entry, Some(password)) {
                Ok(_) => Ok(true),
                Err(EggError::InvalidPassword) => Ok(false),
                Err(e) => Err(e),
            };
        }
    }
    Ok(true)
}

pub fn extract_entry<R: Read + Seek>(
    archive: &mut EggArchive<R>,
    entry: &EggFileEntry,
    dest_dir: &Path,
    password: Option<&str>,
    pipe_mode: bool,
) -> EggResult<()> {
    let target = safe_join(dest_dir, &entry.file_name)?;

    if entry.is_directory() {
        if !pipe_mode {
            fs::create_dir_all(&target).map_err(EggError::CantOpenDestFile)?;
        }
        return Ok(());
    }

    if entry.blocks.is_empty() {
        // No blocks with a zero size is a legitimate empty file; a nonzero size
        // means the block headers were lost, so refuse rather than write 0 bytes.
        if entry.uncompressed_size != 0 {
            return Err(EggError::CorruptedFile);
        }
        if !pipe_mode {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(EggError::CantOpenDestFile)?;
            }
            fs::File::create(&target).map_err(EggError::CantOpenDestFile)?;
        }
        return Ok(());
    }

    let mut crypto = setup_decryptor(entry, password)?;

    // Verify the AE-2 authentication footer before creating any output, so a
    // tampered encrypted file fails without leaving a file on disk.
    if let Some(fc) = &crypto
        && let Some(auth) = &fc.auth
    {
        verify_hmac(&mut archive.reader, entry, auth)?;
    }

    let mut writer: Box<dyn Write> = if pipe_mode {
        Box::new(io::stdout())
    } else {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(EggError::CantOpenDestFile)?;
        }
        Box::new(fs::File::create(&target).map_err(EggError::CantOpenDestFile)?)
    };

    let result = (|| {
        for block in &entry.blocks {
            archive.reader.seek(SeekFrom::Start(block.data_pos))?;
            let mut bounded = (&mut archive.reader).take(block.compressed_size as u64);

            let crypto_ref = crypto
                .as_mut()
                .map(|c| &mut *c.decryptor as &mut dyn Decryptor);
            let crc = decompress_block(&mut bounded, &mut writer, block, crypto_ref)?;

            if crc != block.crc32 {
                return Err(EggError::InvalidFileCrc {
                    expected: block.crc32,
                    got: crc,
                });
            }
        }
        Ok(())
    })();

    drop(writer);

    if let Err(e) = result {
        // Don't leave a partial / unverified file on disk.
        if !pipe_mode {
            let _ = fs::remove_file(&target);
        }
        return Err(e);
    }

    if !pipe_mode && let Some(ft) = entry.file_time {
        set_file_time(&target, ft);
    }

    Ok(())
}

pub fn extract_all<R: Read + Seek>(
    archive: &mut EggArchive<R>,
    dest_dir: &Path,
    password: Option<&str>,
    pipe_mode: bool,
) -> EggResult<()> {
    let entries: Vec<EggFileEntry> = archive.entries.clone();
    if archive.is_solid {
        extract_all_solid(archive, &entries, dest_dir, password, pipe_mode, None)
    } else {
        for entry in &entries {
            extract_entry(archive, entry, dest_dir, password, pipe_mode)?;
        }
        Ok(())
    }
}

pub fn extract_files<R: Read + Seek>(
    archive: &mut EggArchive<R>,
    dest_dir: &Path,
    password: Option<&str>,
    pipe_mode: bool,
    files: &[String],
) -> EggResult<()> {
    let entries: Vec<EggFileEntry> = archive.entries.clone();
    // Names the caller asked for that correspond to no entry at all.
    let unmatched: Option<String> = files
        .iter()
        .find(|f| !entries.iter().any(|e| matches_filter(&e.file_name, f)))
        .cloned();

    if archive.is_solid {
        extract_all_solid(
            archive,
            &entries,
            dest_dir,
            password,
            pipe_mode,
            Some(files),
        )?;
    } else {
        for entry in &entries {
            if should_extract(entry, Some(files)) {
                extract_entry(archive, entry, dest_dir, password, pipe_mode)?;
            }
        }
    }
    match unmatched {
        Some(name) => Err(EggError::FileNotFound(name)),
        None => Ok(()),
    }
}

/// Solid archive extraction. The blocks form one continuous compressed stream
/// (decompressor state carries across block boundaries), so it is decompressed
/// as a unit and streamed through `SolidSink`, which routes each block's bytes
/// to its file and verifies per-block CRC as bytes arrive. The decompressed
/// output is not held in RAM; the compressed input is.
fn extract_all_solid<R: Read + Seek>(
    archive: &mut EggArchive<R>,
    entries: &[EggFileEntry],
    dest_dir: &Path,
    password: Option<&str>,
    pipe_mode: bool,
    filter: Option<&[String]>,
) -> EggResult<()> {
    struct BlockRead {
        file_idx: usize,
        compressed_size: u32,
        data_pos: u64,
    }

    // The solid stream is tiled two ways: output by entry (each file takes its
    // own uncompressed_size) and integrity by block (each block's own size + CRC,
    // one block spanning the whole group). Conflating them drops files that own
    // no block.
    let mut file_spans: Vec<(usize, u64)> = Vec::new();
    let mut crc_spans: Vec<(u64, u32)> = Vec::new();
    let mut block_reads: Vec<BlockRead> = Vec::new();

    for (fi, entry) in entries.iter().enumerate() {
        if entry.is_directory() {
            // Only materialize directory entries that pass the filter; file
            // parents are created on demand by the sink.
            if !pipe_mode && should_extract(entry, filter) {
                let dir_path = safe_join(dest_dir, &entry.file_name)?;
                fs::create_dir_all(&dir_path).map_err(EggError::CantOpenDestFile)?;
            }
            continue;
        }

        // Validate the destination path up front so a path-traversal entry fails
        // cleanly before any decompression happens.
        let _ = safe_join(dest_dir, &entry.file_name)?;

        file_spans.push((fi, entry.uncompressed_size));
        for block in &entry.blocks {
            crc_spans.push((block.uncompressed_size as u64, block.crc32));
            block_reads.push(BlockRead {
                file_idx: fi,
                compressed_size: block.compressed_size,
                data_pos: block.data_pos,
            });
        }
    }

    if !block_reads.is_empty() {
        // Read and decrypt every block into one compressed buffer (bounded by the
        // archive's own size), maintaining per-file decryptor state and verifying
        // each encrypted file's AE-2 authentication footer.
        let mut all_compressed = Vec::new();
        let mut current_file_idx = usize::MAX;
        let mut crypto: Option<FileCrypto> = None;

        for br in &block_reads {
            if br.file_idx != current_file_idx {
                current_file_idx = br.file_idx;
                crypto = setup_decryptor(&entries[br.file_idx], password)?;
                if let Some(fc) = &crypto
                    && let Some(auth) = &fc.auth
                {
                    verify_hmac(&mut archive.reader, &entries[br.file_idx], auth)?;
                }
            }

            archive.reader.seek(SeekFrom::Start(br.data_pos))?;
            let want = br.compressed_size as u64;
            let mut buf = Vec::new();
            if (&mut archive.reader).take(want).read_to_end(&mut buf)? as u64 != want {
                return Err(EggError::CorruptedFile);
            }
            if let Some(fc) = crypto.as_mut() {
                fc.decryptor.decrypt(&mut buf);
            }
            all_compressed.extend_from_slice(&buf);
        }

        let method = entries
            .iter()
            .flat_map(|e| e.blocks.iter())
            .next()
            .map(|b| b.compression_method)
            .unwrap_or(CompressionMethod::Store);

        let total_uncompressed: u64 = crc_spans.iter().map(|(size, _)| *size).sum();

        let mut sink = SolidSink::new(
            &file_spans,
            &crc_spans,
            entries,
            dest_dir,
            filter,
            pipe_mode,
        );

        let mut cursor = Cursor::new(&all_compressed);
        let len = all_compressed.len() as u64;
        let outcome = (|| -> EggResult<()> {
            match method {
                CompressionMethod::Store => {
                    io::copy(&mut cursor, &mut sink).map_err(EggError::Io)?;
                }
                CompressionMethod::Deflate => {
                    decompress::deflate::extract_deflate(&mut cursor, &mut sink, len, None)?;
                }
                CompressionMethod::Bzip2 => {
                    decompress::bzip2::extract_bzip2(&mut cursor, &mut sink, len, None)?;
                }
                CompressionMethod::Lzma => {
                    decompress::lzma::extract_lzma(
                        &mut cursor,
                        &mut sink,
                        len,
                        total_uncompressed,
                        None,
                    )?;
                }
                CompressionMethod::Azo => {
                    decompress::azo::extract_azo(
                        &mut cursor,
                        &mut sink,
                        len,
                        total_uncompressed,
                        None,
                    )?;
                }
                CompressionMethod::Unknown(n) => return Err(EggError::UnknownCompressionMethod(n)),
            }
            sink.finish()
        })();
        if let Err(e) = outcome {
            if !pipe_mode {
                sink.cleanup();
            }
            return Err(e);
        }
    } else if !pipe_mode {
        // A solid group with no data blocks at all: every regular entry must be
        // empty, so a nonzero declared size means the blocks were lost.
        if file_spans.iter().any(|&(_, size)| size != 0) {
            return Err(EggError::CorruptedFile);
        }
        for &(fi, _) in &file_spans {
            let entry = &entries[fi];
            if !should_extract(entry, filter) {
                continue;
            }
            let path = safe_join(dest_dir, &entry.file_name)?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(EggError::CantOpenDestFile)?;
            }
            fs::File::create(&path).map_err(EggError::CantOpenDestFile)?;
        }
    }

    // Apply modification times (the sink already created every file).
    if !pipe_mode {
        for entry in entries {
            if entry.is_directory() || !should_extract(entry, filter) {
                continue;
            }
            if let Some(ft) = entry.file_time {
                let path = safe_join(dest_dir, &entry.file_name)?;
                set_file_time(&path, ft);
            }
        }
    }

    Ok(())
}

/// Streaming sink for solid decompression: splits one decompressed stream back
/// into individual files by block boundary, verifying each block's CRC32 as it
/// completes and rejecting any output beyond the declared total (bomb guard).
struct SolidSink<'a> {
    file_spans: &'a [(usize, u64)], // (file_idx, uncompressed_size), output tiling
    crc_spans: &'a [(u64, u32)],    // (uncompressed_size, crc32), integrity tiling
    entries: &'a [EggFileEntry],
    dest_dir: &'a Path,
    filter: Option<&'a [String]>,
    pipe_mode: bool,
    file_idx: usize,
    file_written: u64,
    file_open: bool,
    writer: Option<Box<dyn Write>>,
    crc_idx: usize,
    crc_written: u64,
    hasher: crc32fast::Hasher,
    created: Vec<PathBuf>,
}

impl<'a> SolidSink<'a> {
    fn new(
        file_spans: &'a [(usize, u64)],
        crc_spans: &'a [(u64, u32)],
        entries: &'a [EggFileEntry],
        dest_dir: &'a Path,
        filter: Option<&'a [String]>,
        pipe_mode: bool,
    ) -> Self {
        SolidSink {
            file_spans,
            crc_spans,
            entries,
            dest_dir,
            filter,
            pipe_mode,
            file_idx: 0,
            file_written: 0,
            file_open: false,
            writer: None,
            crc_idx: 0,
            crc_written: 0,
            hasher: crc32fast::Hasher::new(),
            created: Vec::new(),
        }
    }

    /// Remove every file this sink created, so a mid-stream failure (a bad CRC,
    /// a truncated stream) leaves nothing partial on disk.
    fn cleanup(&self) {
        for path in &self.created {
            let _ = fs::remove_file(path);
        }
    }

    /// Create the current file span's output (once), so even zero-length files
    /// are materialized.
    fn ensure_file_open(&mut self) -> io::Result<()> {
        if self.file_open {
            return Ok(());
        }
        self.file_open = true;
        let (fi, _) = self.file_spans[self.file_idx];
        let entry = &self.entries[fi];
        if should_extract(entry, self.filter) {
            self.writer = if self.pipe_mode {
                Some(Box::new(io::stdout()))
            } else {
                let path = safe_join(self.dest_dir, &entry.file_name)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let file = fs::File::create(&path)?;
                self.created.push(path);
                Some(Box::new(file))
            };
        }
        Ok(())
    }

    /// Advance past any file spans that are already full (including zero-length
    /// ones, which are created and closed without any bytes flowing through).
    fn advance_full_files(&mut self) -> io::Result<()> {
        while self.file_idx < self.file_spans.len() {
            let (_, size) = self.file_spans[self.file_idx];
            if self.file_written < size {
                break;
            }
            self.ensure_file_open()?;
            self.writer = None; // flush/close
            self.file_idx += 1;
            self.file_written = 0;
            self.file_open = false;
        }
        Ok(())
    }

    /// Ensure the whole declared stream was produced (no truncation), creating
    /// any trailing zero-length files.
    fn finish(&mut self) -> EggResult<()> {
        self.advance_full_files().map_err(EggError::Io)?;
        if self.file_idx != self.file_spans.len()
            || self.crc_idx != self.crc_spans.len()
            || self.crc_written != 0
        {
            return Err(EggError::CorruptedFile);
        }
        Ok(())
    }
}

impl Write for SolidSink<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let total = buf.len();
        let mut rest = buf;
        while !rest.is_empty() {
            self.advance_full_files()?;
            let Some(&(_, file_size)) = self.file_spans.get(self.file_idx) else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "solid output exceeds declared uncompressed size",
                ));
            };
            let Some(&(crc_size, crc32)) = self.crc_spans.get(self.crc_idx) else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "solid output exceeds declared block size",
                ));
            };
            self.ensure_file_open()?;

            // Write the largest run bounded by both the current file span and the
            // current CRC span, so file and integrity boundaries advance together.
            // Clamp in u64 before narrowing so a >4 GiB span cannot truncate to 0
            // (which would stall the loop) on a 32-bit target.
            let rest_len = rest.len() as u64;
            let file_need = (file_size - self.file_written).min(rest_len) as usize;
            let crc_need = (crc_size - self.crc_written).min(rest_len) as usize;
            let take = rest.len().min(file_need).min(crc_need);
            let chunk = &rest[..take];
            self.hasher.update(chunk);
            if let Some(w) = self.writer.as_mut() {
                w.write_all(chunk)?;
            }
            self.file_written += take as u64;
            self.crc_written += take as u64;
            rest = &rest[take..];

            if self.crc_written == crc_size {
                let crc = std::mem::replace(&mut self.hasher, crc32fast::Hasher::new()).finalize();
                if crc != crc32 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "solid block CRC mismatch",
                    ));
                }
                self.crc_idx += 1;
                self.crc_written = 0;
            }
        }
        Ok(total)
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.writer.as_mut() {
            Some(w) => w.flush(),
            None => Ok(()),
        }
    }
}

fn should_extract(entry: &EggFileEntry, filter: Option<&[String]>) -> bool {
    match filter {
        None => true,
        Some(files) => files.iter().any(|f| matches_filter(&entry.file_name, f)),
    }
}

/// A filter matches an entry by exact name or as a parent directory, rather
/// than by substring (which would also pull in unintended files).
fn matches_filter(name: &str, pat: &str) -> bool {
    let pat = pat.trim_end_matches('/');
    name == pat
        || name
            .strip_prefix(pat)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn decompress_block<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    block: &EggBlock,
    crypto: Option<&mut dyn Decryptor>,
) -> EggResult<u32> {
    // Cap output at the block's declared uncompressed size to stop bombs.
    let cs = block.compressed_size as u64;
    let mut w = decompress::LimitWriter::new(writer, block.uncompressed_size as u64);
    match block.compression_method {
        CompressionMethod::Store => decompress::store::extract_store(reader, &mut w, cs, crypto),
        CompressionMethod::Deflate => {
            decompress::deflate::extract_deflate(reader, &mut w, cs, crypto)
        }
        CompressionMethod::Bzip2 => decompress::bzip2::extract_bzip2(reader, &mut w, cs, crypto),
        CompressionMethod::Lzma => decompress::lzma::extract_lzma(
            reader,
            &mut w,
            cs,
            block.uncompressed_size as u64,
            crypto,
        ),
        CompressionMethod::Azo => {
            decompress::azo::extract_azo(reader, &mut w, cs, block.uncompressed_size as u64, crypto)
        }
        CompressionMethod::Unknown(n) => Err(EggError::UnknownCompressionMethod(n)),
    }
}

/// Convert Windows FILETIME to Unix timestamp and set file mtime.
fn set_file_time(path: &Path, filetime_val: u64) {
    const EPOCH_DIFF: u64 = 11644473600;
    const TICKS_PER_SEC: u64 = 10_000_000;

    if filetime_val < EPOCH_DIFF * TICKS_PER_SEC {
        return;
    }

    let unix_secs = (filetime_val / TICKS_PER_SEC).saturating_sub(EPOCH_DIFF);
    let nanos = ((filetime_val % TICKS_PER_SEC) * 100) as u32;
    let ft = filetime::FileTime::from_unix_time(unix_secs as i64, nanos);
    let _ = filetime::set_file_mtime(path, ft);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_join_keeps_normal_paths() {
        let dest = Path::new("/out");
        assert_eq!(
            safe_join(dest, "a/b/c.txt").unwrap(),
            Path::new("/out/a/b/c.txt")
        );
        // CurDir components are ignored.
        assert_eq!(safe_join(dest, "./a/./b").unwrap(), Path::new("/out/a/b"));
    }

    #[test]
    fn safe_join_rejects_nameless_entry() {
        let dest = Path::new("/out");
        for name in ["", ".", "./."] {
            assert!(
                matches!(safe_join(dest, name), Err(EggError::CorruptedFile)),
                "should reject {name:?}"
            );
        }
    }

    #[test]
    fn safe_join_rejects_traversal() {
        let dest = Path::new("/out");
        assert!(matches!(
            safe_join(dest, "../etc/passwd"),
            Err(EggError::PathTraversal(_))
        ));
        assert!(matches!(
            safe_join(dest, "a/../../etc/passwd"),
            Err(EggError::PathTraversal(_))
        ));
        // A bare ".." that the old substring check missed.
        assert!(matches!(
            safe_join(dest, ".."),
            Err(EggError::PathTraversal(_))
        ));
        assert!(matches!(
            safe_join(dest, "a/.."),
            Err(EggError::PathTraversal(_))
        ));
    }

    #[test]
    fn safe_join_rejects_absolute() {
        let dest = Path::new("/out");
        assert!(matches!(
            safe_join(dest, "/etc/passwd"),
            Err(EggError::PathTraversal(_))
        ));
    }

    #[test]
    fn filter_matches_exact_and_directory_only() {
        assert!(matches_filter("dir/file.txt", "dir/file.txt")); // exact
        assert!(matches_filter("dir/file.txt", "dir")); // parent dir
        assert!(matches_filter("dir/file.txt", "dir/")); // trailing slash
        assert!(!matches_filter("mydir/file.txt", "dir")); // not a substring match
        assert!(!matches_filter("dir/file.txt.bak", "dir/file.txt")); // not a prefix match
    }

    #[test]
    fn limit_writer_caps_output() {
        use std::io::Write;
        let mut sink = Vec::new();
        let mut w = decompress::LimitWriter::new(&mut sink, 4);
        assert!(w.write_all(b"abcd").is_ok());
        let mut sink2 = Vec::new();
        let mut w2 = decompress::LimitWriter::new(&mut sink2, 4);
        // Writing more than the declared size fails instead of growing unbounded.
        assert!(w2.write_all(b"abcde").is_err());
    }
}
