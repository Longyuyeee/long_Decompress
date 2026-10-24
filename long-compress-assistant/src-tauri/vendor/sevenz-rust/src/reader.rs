use std::{
    fs::File,
    io::{ErrorKind, Read, Seek, SeekFrom},
    path::Path,
};

use bit_set::BitSet;
use crc::Crc;

use crate::{archive::*, decoders::add_decoder, error::Error, folder::*, password::Password};
pub(crate) const CRC32: Crc<u32> = Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);
const MAX_MEM_LIMIT_KB: usize = usize::MAX / 1024;

pub(crate) trait SeedRead: Read + Seek {}

pub struct BoundedReader<R: Read> {
    inner: R,
    remain: usize,
}

impl<R: Read> BoundedReader<R> {
    pub fn new(inner: R, max_size: usize) -> Self {
        Self {
            inner,
            remain: max_size,
        }
    }
}

impl<R: Read> Read for BoundedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.remain == 0 {
            return Ok(0);
        }
        let remain = self.remain;
        let buf2 = if buf.len() < remain {
            buf
        } else {
            &mut buf[..remain]
        };
        match self.inner.read(buf2) {
            Ok(size) => {
                if self.remain < size {
                    self.remain = 0;
                } else {
                    self.remain -= size;
                }
                Ok(size)
            }
            Err(e) => Err(e),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct SeekableBoundedReader<R: Read + Seek> {
    inner: R,
    cur: u64,
    bounds: (u64, u64),
}

impl<R: Read + Seek> SeedRead for SeekableBoundedReader<R> {}

impl<R: Read + Seek> Seek for SeekableBoundedReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(pos) => self.bounds.0 as i64 + pos as i64,
            SeekFrom::End(pos) => self.bounds.1 as i64 + pos,
            SeekFrom::Current(pos) => self.cur as i64 + pos,
        };
        if new_pos < 0 {
            return Err(std::io::Error::new(ErrorKind::Other, "SeekBeforeStart"));
        }
        self.cur = new_pos as u64;
        self.inner.seek(SeekFrom::Start(self.cur))
    }
}

impl<R: Read + Seek> Read for SeekableBoundedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.cur >= self.bounds.1 {
            return Ok(0);
        }
        if self.stream_position()? != self.cur {
            self.inner.seek(SeekFrom::Start(self.cur))?;
        }
        let buf2 = if buf.len() < (self.bounds.1 - self.cur) as usize {
            buf
        } else {
            &mut buf[..(self.bounds.1 - self.cur) as usize]
        };
        let size = self.inner.read(buf2)?;
        self.cur += size as u64;
        Ok(size)
    }
}

impl<R: Read + Seek> SeekableBoundedReader<R> {
    pub fn new(inner: R, bounds: (u64, u64)) -> Self {
        Self {
            inner,
            cur: bounds.0,
            bounds,
        }
    }
}

struct Crc32VerifyingReader<R> {
    inner: R,
    crc_digest: crc::Digest<'static, u32>,
    expected_value: u64,
    remaining: i64,
}

// A file/CRC size wrapper would otherwise hide a malformed LZMA2 terminal
// chunk after all declared bytes have been returned. Check it before handing
// the last bytes to the complete-operation callback. Prefix readers do not
// use this stack and deliberately leave their unread tail unverified.
struct Lzma2EndReader<R> { inner: R, remaining: u64 }
impl<R: Read> Read for Lzma2EndReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if buffer.is_empty() { return Ok(0); }
        if self.remaining == 0 { return Ok(0); }
        let capacity = self.remaining.min(buffer.len() as u64) as usize;
        let n = self.inner.read(&mut buffer[..capacity])?;
        if n == 0 { return Err(std::io::Error::new(ErrorKind::UnexpectedEof,"LZMA2 output ended before declared size")); }
        self.remaining -= n as u64;
        if self.remaining == 0 && self.inner.read(&mut [0u8;1])? != 0 {
            return Err(std::io::Error::new(ErrorKind::InvalidData,"LZMA2 output exceeds declared size"));
        }
        Ok(n)
    }
}

impl<R: Read> Crc32VerifyingReader<R> {
    fn new(inner: R, remaining: usize, expected_value: u64) -> Self {
        Self {
            inner,
            crc_digest: CRC32.digest(),
            expected_value,
            remaining: remaining as i64,
        }
    }
}

impl<R: Read> Read for Crc32VerifyingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.remaining <= 0 || buf.is_empty() {
            return Ok(0);
        }
        let size = self.inner.read(buf)?;
        if size == 0 {
            return Err(std::io::Error::new(
                ErrorKind::UnexpectedEof,
                Error::ChecksumVerificationFailed,
            ));
        }
        if size > 0 {
            self.remaining -= size as i64;
            self.crc_digest.update(&buf[..size]);
        }
        if self.remaining <= 0 {
            let d = std::mem::replace(&mut self.crc_digest, CRC32.digest()).finalize();
            if d as u64 != self.expected_value {
                return Err(std::io::Error::new(
                    ErrorKind::Other,
                    Error::ChecksumVerificationFailed,
                ));
            }
        }
        Ok(size)
    }
}

#[cfg(test)]
mod long_crc_tests {
    use super::*;

    #[test]
    fn premature_decoder_eof_must_not_pass_crc_verification() {
        for bytes in [b"".as_slice(), b"a".as_slice()] {
            let mut reader = Crc32VerifyingReader::new(bytes, 3, CRC32.checksum(b"abc") as u64);
            let mut output = Vec::new();
            assert!(reader.read_to_end(&mut output).is_err(), "A short decoded stream must fail even if its decoder reports normal EOF");
        }
    }

    #[test]
    fn complete_and_empty_buffer_reads_keep_standard_read_semantics() {
        let mut reader = Crc32VerifyingReader::new(b"abc".as_slice(), 3, CRC32.checksum(b"abc") as u64);
        assert_eq!(reader.read(&mut []).unwrap(), 0);
        let mut output = Vec::new();
        reader.read_to_end(&mut output).unwrap();
        assert_eq!(output, b"abc");
        let mut empty = Crc32VerifyingReader::new(b"".as_slice(), 0, CRC32.checksum(b"") as u64);
        assert_eq!(empty.read(&mut [0; 1]).unwrap(), 0);
    }
}

impl Archive {
    /// Open 7z file under specified `path`.
    #[inline]
    pub fn open(path: impl AsRef<Path>) -> Result<Archive, Error> {
        Self::open_with_password(path, &Password::empty())
    }

    /// Open an encrypted 7z file under specified `path` with  `password`.
    #[inline]
    pub fn open_with_password(
        path: impl AsRef<Path>,
        password: &Password,
    ) -> Result<Archive, Error> {
        let mut file = std::fs::File::open(path)?;
        let len = file.metadata()?.len();
        Self::read(&mut file, len, password.as_ref())
    }
    /// Read 7z file archive info use the specified `reader`.
    /// -[`reader_len`] the `reader` stream length
    /// -[`password`] Archive password encoded in utf16 little endian.
    ///
    /// # Examples
    /// ```no_run
    /// use std::io::{Read,Seek};
    /// use std::fs::File;
    /// use sevenz_rust::*;
    /// let mut reader = File::open("example.7z").unwrap();
    /// let len = reader.metadata().unwrap().len();
    ///
    /// let password = Password::from("the password");
    /// let archive = Archive::read(&mut reader,len, password.as_ref()).unwrap();
    ///
    /// //let archive = Archive::read(&mut reader,len, &[]);// for unencrypted file
    /// for entry in &archive.files {
    ///     println!("{}", entry.name());
    /// }
    /// ```
    pub fn read<R: Read + Seek>(
        reader: &mut R,
        reader_len: u64,
        password: &[u8],
    ) -> Result<Archive, Error> {
        let mut signature = [0; 6];
        reader.read_exact(&mut signature).map_err(Error::io)?;
        if signature != SEVEN_Z_SIGNATURE {
            return Err(Error::BadSignature(signature));
        }
        let mut versions = [0; 2];
        reader.read_exact(&mut versions).map_err(Error::io)?;
        let version_major = versions[0];
        let version_minor = versions[1];
        if version_major != 0 {
            return Err(Error::UnsupportedVersion {
                major: version_major,
                minor: version_minor,
            });
        }

        let start_header_crc = read_u32(reader)?;

        let header_valid = if start_header_crc == 0 {
            let current_position = reader.stream_position().map_err(Error::io)?;
            let mut buf = [0; 20];
            reader.read_exact(&mut buf).map_err(Error::io)?;
            reader
                .seek(std::io::SeekFrom::Start(current_position))
                .map_err(Error::io)?;
            buf.iter().any(|a| *a != 0)
        } else {
            true
        };
        if header_valid {
            let start_header = Self::read_start_header(reader, start_header_crc)?;
            Self::init_archive(reader, start_header, password, true)
        } else {
            Self::try_to_locale_end_header(reader, reader_len, password)
        }
    }

    fn read_start_header<R: Read>(
        reader: &mut R,
        start_header_crc: u32,
    ) -> Result<StartHeader, Error> {
        let mut buf = [0; 20];
        reader.read_exact(&mut buf).map_err(Error::io)?;
        let value = crc32_cksum(&buf);
        if value != start_header_crc {
            return Err(Error::ChecksumVerificationFailed);
        }
        let mut buf_read = buf.as_slice();
        let offset = read_u64le(&mut buf_read)?;

        let size = read_u64le(&mut buf_read)?;
        let crc = read_u32(&mut buf_read)?;
        Ok(StartHeader {
            next_header_offset: offset,
            next_header_size: size,
            next_header_crc: crc as u64,
        })
    }

    fn read_header<R: Read + Seek>(header: &mut R, archive: &mut Archive) -> Result<(), Error> {
        let mut nid = read_u8(header)?;
        if nid == K_ARCHIVE_PROPERTIES {
            Self::read_archive_properties(header)?;
            nid = read_u8(header)?;
        }

        if nid == K_ADDITIONAL_STREAMS_INFO {
            return Err(Error::other("Additional streams unsupported"));
        }
        if nid == K_MAIN_STREAMS_INFO {
            Self::read_streams_info(header, archive)?;
            nid = read_u8(header)?;
        }
        if nid == K_FILES_INFO {
            Self::read_files_info(header, archive)?;
            nid = read_u8(header)?;
        }
        if nid != K_END {
            return Err(Error::BadTerminatedheader(nid));
        }

        Ok(())
    }

    fn read_archive_properties<R: Read + Seek>(header: &mut R) -> Result<(), Error> {
        let mut nid = read_u8(header)?;
        while nid != K_END {
            let property_size = read_usize(header, "propertySize")?;
            header
                .seek(SeekFrom::Current(property_size as i64))
                .map_err(Error::io)?;
            nid = read_u8(header)?;
        }
        Ok(())
    }

    fn try_to_locale_end_header<R: Read + Seek>(
        reader: &mut R,
        reader_len: u64,
        password: &[u8],
    ) -> Result<Self, Error> {
        let search_limit = 1024 * 1024;
        let prev_data_size = reader.stream_position().map_err(Error::io)? + 20;
        let size = reader_len;
        let min_pos = if reader.stream_position().map_err(Error::io)? + search_limit > size {
            reader.stream_position().map_err(Error::io)?
        } else {
            size - search_limit
        };
        let mut pos = reader_len - 1;
        while pos > min_pos {
            pos -= 1;

            reader
                .seek(std::io::SeekFrom::Start(pos))
                .map_err(Error::io)?;
            let nid = read_u8(reader)?;
            if nid == K_ENCODED_HEADER || nid == K_HEADER {
                let start_header = StartHeader {
                    next_header_offset: pos - prev_data_size,
                    next_header_size: reader_len - pos,
                    next_header_crc: 0,
                };
                let result = Self::init_archive(reader, start_header, password, false)?;

                if !result.files.is_empty() {
                    return Ok(result);
                }
            }
        }
        Err(Error::other(
            "Start header corrupt and unable to guess end header",
        ))
    }

    fn init_archive<R: Read + Seek>(
        reader: &mut R,
        start_header: StartHeader,
        password: &[u8],
        verify_crc: bool,
    ) -> Result<Self, Error> {
        crate::preview::size(start_header.next_header_size)?;
        if start_header.next_header_size > usize::MAX as u64 {
            return Err(Error::other(format!(
                "Cannot handle next_header_size {}",
                start_header.next_header_size
            )));
        }

        let next_header_size_int = start_header.next_header_size as usize;

        reader
            .seek(SeekFrom::Start(
                SIGNATURE_HEADER_SIZE + start_header.next_header_offset,
            ))
            .map_err(Error::io)?;

        let mut buf = vec![0; next_header_size_int];
        reader.read_exact(&mut buf).map_err(Error::io)?;
        if verify_crc && crc32_cksum(&buf) as u64 != start_header.next_header_crc {
            return Err(Error::NextHeaderCrcMismatch);
        }

        let mut archive = Archive::default();
        let mut buf_reader = buf.as_slice();
        let mut nid = read_u8(&mut buf_reader)?;
        let mut header = if nid == K_ENCODED_HEADER {
            let (mut out_reader, buf_size) =
                Self::read_encoded_header(&mut buf_reader, reader, &mut archive, password)?;
            buf.clear();
            buf.resize(buf_size, 0);
            out_reader
                .read_exact(&mut buf)
                .map_err(|e| Error::bad_password(e, !password.is_empty()))?;
            if crate::preview::active() {
                crate::preview::check()?;
                let mut extra=[0];
                if out_reader.read(&mut extra).map_err(|e|Error::bad_password(e,!password.is_empty()))?!=0 {
                    return Err(Error::other("Encoded header exceeds declared size"));
                }
                let folder=&archive.folders[0];
                if folder.has_crc && CRC32.checksum(&buf) as u64!=folder.crc {return Err(Error::ChecksumVerificationFailed);}
            }
            archive = Archive::default();
            buf_reader = buf.as_slice();
            nid = read_u8(&mut buf_reader)?;
            buf_reader
        } else {
            buf_reader
        };
        let mut header = std::io::Cursor::new(&mut header);
        if nid == K_HEADER {
            Self::read_header(&mut header, &mut archive)?;
        } else {
            return Err(Error::other("Broken or unsupported archive: no Header"));
        }
        Ok(archive)
    }

    fn read_encoded_header<'r, R: Read, RI: 'r + Read + Seek>(
        header: &mut R,
        reader: &'r mut RI,
        archive: &mut Archive,
        password: &[u8],
    ) -> Result<(Box<dyn Read + 'r>, usize), Error> {
        Self::read_streams_info(header, archive)?;
        let folder = archive
            .folders
            .first()
            .ok_or(Error::other("no folders, can't read encoded header"))?;
        let first_pack_stream_index = 0;
        let folder_offset = SIGNATURE_HEADER_SIZE + archive.pack_pos;
        if archive.pack_sizes.is_empty() {
            return Err(Error::other("no packed streams, can't read encoded header"));
        }

        reader
            .seek(SeekFrom::Start(folder_offset))
            .map_err(Error::io)?;
        let coder_len = folder.coders.len();
        let unpack_size = folder.get_unpack_size() as usize;
        let pack_size = archive.pack_sizes[first_pack_stream_index] as usize;
        if crate::preview::active() {
            crate::preview::size(unpack_size as u64)?;crate::preview::size(pack_size as u64)?;
            if archive.folders.len()!=1 || archive.pack_sizes.len()!=1 || folder.packed_streams.len()!=1 {return Err(Error::unsupported("7Z 预览文件头共享范围无法确认，请先提取检查"));}
        }
        let input_reader =
            SeekableBoundedReader::new(reader, (folder_offset, folder_offset + pack_size as u64));
        let mut decoder: Box<dyn Read> = Box::new(input_reader);
        let mut decoder = if coder_len > 0 {
            for (index, coder) in folder.ordered_coder_iter() {
                if coder.num_in_streams != 1 || coder.num_out_streams != 1 {
                    return Err(Error::other(
                        "Multi input/output stream coders are not yet supported",
                    ));
                }
                let next = crate::decoders::add_decoder(
                    decoder,
                    folder.get_unpack_size_at_index(index) as usize,
                    coder,
                    password,
                    MAX_MEM_LIMIT_KB,
                )?;
                decoder = Box::new(next);
            }
            decoder
        } else {
            decoder
        };
        if folder.has_crc && !crate::preview::active() {
            decoder = Box::new(Crc32VerifyingReader::new(decoder, unpack_size, folder.crc));
        }

        Ok((decoder, unpack_size))
    }

    fn read_streams_info<R: Read>(header: &mut R, archive: &mut Archive) -> Result<(), Error> {
        let mut nid = read_u8(header)?;
        if nid == K_PACK_INFO {
            Self::read_pack_info(header, archive)?;
            nid = read_u8(header)?;
        }

        if nid == K_UNPACK_INFO {
            Self::read_unpack_info(header, archive)?;
            nid = read_u8(header)?;
        } else {
            archive.folders.clear();
        }
        if nid == K_SUB_STREAMS_INFO {
            Self::read_sub_streams_info(header, archive)?;
            nid = read_u8(header)?;
        }
        if nid != K_END {
            return Err(Error::BadTerminatedStreamsInfo(nid));
        }

        Ok(())
    }

    fn read_files_info<R: Read + Seek>(header: &mut R, archive: &mut Archive) -> Result<(), Error> {
        let num_files = read_usize(header, "num files")?;
        let mut files: Vec<SevenZArchiveEntry> = vec![Default::default(); num_files];

        let mut is_empty_stream: Option<BitSet> = None;
        let mut is_empty_file: Option<BitSet> = None;
        let mut is_anti: Option<BitSet> = None;
        loop {
            let prop_type = read_u8(header)?;
            if prop_type == 0 {
                break;
            }
            let size = read_u64(header)?;
            match prop_type {
                K_EMPTY_STREAM => {
                    is_empty_stream = Some(read_bits(header, num_files)?);
                }
                K_EMPTY_FILE => {
                    let n = if let Some(s) = &is_empty_stream {
                        s.len()
                    } else {
                        return Err(Error::other(
                            "Header format error: kEmptyStream must appear before kEmptyFile",
                        ));
                    };
                    is_empty_file = Some(read_bits(header, n)?);
                }
                K_ANTI => {
                    let n = if let Some(s) = is_empty_stream.as_ref() {
                        s.len()
                    } else {
                        return Err(Error::other(
                            "Header format error: kEmptyStream must appear before kEmptyFile",
                        ));
                    };
                    is_anti = Some(read_bits(header, n)?);
                }
                K_NAME => {
                    let external = read_u8(header)?;
                    if external != 0 {
                        return Err(Error::other("Not implemented:external != 0"));
                    }
                    if (size - 1) & 1 != 0 {
                        return Err(Error::other("file names length invalid"));
                    }

                    let size = assert_usize(size, "file names length")?;
                    // let mut names = vec![0u8; size - 1];
                    // header.read_exact(&mut names).map_err(Error::io)?;
                    let names_reader = NamesReader::new(header, size - 1);

                    let mut next_file = 0;
                    for s in names_reader {
                        files[next_file].name = s?;
                        next_file += 1;
                    }

                    if next_file != files.len() {
                        return Err(Error::other("Error parsing file names"));
                    }
                }
                K_C_TIME => {
                    let times_defined = read_all_or_bits(header, num_files)?;
                    let external = read_u8(header)?;
                    if external != 0 {
                        return Err(Error::other(format!(
                            "kCTime Unimplemented:external={}",
                            external
                        )));
                    }
                    for i in 0..num_files {
                        files[i].has_creation_date = times_defined.contains(i);
                        if files[i].has_creation_date {
                            files[i].creation_date = read_u64le(header)?.into();
                        }
                    }
                }
                K_A_TIME => {
                    let times_defined = read_all_or_bits(header, num_files)?;
                    let external = read_u8(header)?;
                    if external != 0 {
                        return Err(Error::other(format!(
                            "kATime Unimplemented:external={}",
                            external
                        )));
                    }
                    for i in 0..num_files {
                        files[i].has_access_date = times_defined.contains(i);
                        if files[i].has_access_date {
                            files[i].access_date = read_u64le(header)?.into();
                        }
                    }
                }
                K_M_TIME => {
                    let times_defined = read_all_or_bits(header, num_files)?;
                    let external = read_u8(header)?;
                    if external != 0 {
                        return Err(Error::other(format!(
                            "kMTime Unimplemented:external={}",
                            external
                        )));
                    }
                    for i in 0..num_files {
                        files[i].has_last_modified_date = times_defined.contains(i);
                        if files[i].has_last_modified_date {
                            files[i].last_modified_date = read_u64le(header)?.into();
                        }
                    }
                }
                K_WIN_ATTRIBUTES => {
                    let times_defined = read_all_or_bits(header, num_files)?;
                    let external = read_u8(header)?;
                    if external != 0 {
                        return Err(Error::other(format!(
                            "kWinAttributes Unimplemented:external={}",
                            external
                        )));
                    }
                    for i in 0..num_files {
                        files[i].has_windows_attributes = times_defined.contains(i);
                        if files[i].has_windows_attributes {
                            files[i].windows_attributes = read_u32(header)?;
                        }
                    }
                }
                K_START_POS => return Err(Error::other("kStartPos is unsupported, please report")),
                K_DUMMY => {
                    header
                        .seek(SeekFrom::Current(size as i64))
                        .map_err(Error::io)?;
                }
                _ => {
                    header
                        .seek(SeekFrom::Current(size as i64))
                        .map_err(Error::io)?;
                }
            };
        }

        let mut non_empty_file_counter = 0;
        let mut empty_file_counter = 0;
        for i in 0..files.len() {
            let file = &mut files[i];
            file.has_stream = is_empty_stream
                .as_ref()
                .map(|s| !s.contains(i))
                .unwrap_or(true);
            if file.has_stream {
                let sub_stream_info = if let Some(s) = archive.sub_streams_info.as_ref() {
                    s
                } else {
                    return Err(Error::other(
                        "Archive contains file with streams but no subStreamsInfo",
                    ));
                };
                file.is_directory = false;
                file.is_anti_item = false;
                file.has_crc = sub_stream_info.has_crc.contains(non_empty_file_counter);
                file.crc = sub_stream_info.crcs[non_empty_file_counter];
                file.size = sub_stream_info.unpack_sizes[non_empty_file_counter];
                non_empty_file_counter += 1;
            } else {
                file.is_directory = if let Some(s) = &is_empty_file {
                    !s.contains(empty_file_counter)
                } else {
                    true
                };
                file.is_anti_item = is_anti
                    .as_ref()
                    .map(|s| s.contains(empty_file_counter))
                    .unwrap_or(false);
                file.has_crc = false;
                file.size = 0;
                empty_file_counter += 1;
            }
        }
        archive.files = files;

        Self::calculate_stream_map(archive)?;
        Ok(())
    }

    fn calculate_stream_map(archive: &mut Archive) -> Result<(), Error> {
        let mut stream_map = StreamMap::default();

        let mut next_folder_pack_stream_index = 0;
        let num_folders = archive.folders.len();
        stream_map.folder_first_pack_stream_index = vec![0; num_folders];
        for i in 0..num_folders {
            stream_map.folder_first_pack_stream_index[i] = next_folder_pack_stream_index;
            next_folder_pack_stream_index += archive.folders[i].packed_streams.len();
        }

        let mut next_pack_stream_offset = 0;
        let num_pack_sizes = archive.pack_sizes.len();
        stream_map.pack_stream_offsets = vec![0; num_pack_sizes];
        for i in 0..num_pack_sizes {
            stream_map.pack_stream_offsets[i] = next_pack_stream_offset;
            next_pack_stream_offset += archive.pack_sizes[i];
        }

        stream_map.folder_first_file_index = vec![0; num_folders];
        stream_map.file_folder_index = vec![None; archive.files.len()];
        let mut next_folder_index = 0;
        let mut next_folder_unpack_stream_index = 0;
        for i in 0..archive.files.len() {
            if !archive.files[i].has_stream && next_folder_unpack_stream_index == 0 {
                stream_map.file_folder_index[i] = None;
                continue;
            }
            if next_folder_unpack_stream_index == 0 {
                while next_folder_index < archive.folders.len() {
                    stream_map.folder_first_file_index[next_folder_index] = i;
                    if archive.folders[next_folder_index].num_unpack_sub_streams > 0 {
                        break;
                    }
                    next_folder_index += 1;
                }
                if next_folder_index >= archive.folders.len() {
                    return Err(Error::other("Too few folders in archive"));
                }
            }
            stream_map.file_folder_index[i] = Some(next_folder_index);
            if !archive.files[i].has_stream {
                continue;
            }

            //set `compressed_size` of first file in block
            if stream_map.folder_first_file_index[next_folder_index] == i {
                let first_pack_stream_index =
                    stream_map.folder_first_pack_stream_index[next_folder_index];
                let pack_size = archive.pack_sizes[first_pack_stream_index];

                archive.files[i].compressed_size = pack_size;
            }

            next_folder_unpack_stream_index += 1;
            if next_folder_unpack_stream_index
                >= archive.folders[next_folder_index].num_unpack_sub_streams
            {
                next_folder_index += 1;
                next_folder_unpack_stream_index = 0;
            }
        }

        archive.stream_map = stream_map;
        Ok(())
    }

    fn read_pack_info<R: Read>(header: &mut R, archive: &mut Archive) -> Result<(), Error> {
        archive.pack_pos = read_u64(header)?;
        let num_pack_streams = read_usize(header, "num pack streams")?;
        let mut nid = read_u8(header)?;
        if nid == K_SIZE {
            archive.pack_sizes = vec![0u64; num_pack_streams];
            for i in 0..archive.pack_sizes.len() {
                archive.pack_sizes[i] = read_u64(header)?;
            }
            nid = read_u8(header)?;
        }

        if nid == K_CRC {
            archive.pack_crcs_defined = read_all_or_bits(header, num_pack_streams)?;
            archive.pack_crcs = vec![0; num_pack_streams];
            for i in 0..num_pack_streams {
                if archive.pack_crcs_defined.contains(i) {
                    archive.pack_crcs[i] = read_u32(header)? as u64;
                }
            }
            nid = read_u8(header)?;
        }

        if nid != K_END {
            return Err(Error::BadTerminatedPackInfo(nid));
        }

        Ok(())
    }
    fn read_unpack_info<R: Read>(header: &mut R, archive: &mut Archive) -> Result<(), Error> {
        let nid = read_u8(header)?;
        if nid != K_FOLDER {
            return Err(Error::other(format!("Expected kFolder, got {}", nid)));
        }
        let num_folders = read_usize(header, "num folders")?;

        archive.folders.reserve_exact(num_folders);
        let external = read_u8(header)?;
        if external != 0 {
            return Err(Error::ExternalUnsupported);
        }

        for _ in 0..num_folders {
            archive.folders.push(Self::read_folder(header)?);
        }

        let nid = read_u8(header)?;
        if nid != K_CODERS_UNPACK_SIZE {
            return Err(Error::other(format!(
                "Expected kCodersUnpackSize, got {}",
                nid
            )));
        }

        for folder in archive.folders.iter_mut() {
            let tos = folder.total_output_streams;
            folder.unpack_sizes.reserve_exact(tos);
            for _ in 0..tos {
                folder.unpack_sizes.push(read_u64(header)?);
            }
        }

        let mut nid = read_u8(header)?;
        if nid == K_CRC {
            let crcs_defined = read_all_or_bits(header, num_folders)?;
            for i in 0..num_folders {
                if crcs_defined.contains(i) {
                    archive.folders[i].has_crc = true;
                    archive.folders[i].crc = read_u32(header)? as u64;
                } else {
                    archive.folders[i].has_crc = false;
                }
            }
            nid = read_u8(header)?;
        }
        if nid != K_END {
            return Err(Error::BadTerminatedUnpackInfo);
        }

        Ok(())
    }

    fn read_sub_streams_info<R: Read>(header: &mut R, archive: &mut Archive) -> Result<(), Error> {
        for folder in archive.folders.iter_mut() {
            folder.num_unpack_sub_streams = 1;
        }
        let mut total_unpack_streams = archive.folders.len();

        let mut nid = read_u8(header)?;
        if nid == K_NUM_UNPACK_STREAM {
            total_unpack_streams = 0;
            for folder in archive.folders.iter_mut() {
                let num_streams = read_usize(header, "numStreams")?;
                folder.num_unpack_sub_streams = num_streams;
                total_unpack_streams += num_streams;
            }
            nid = read_u8(header)?;
        }

        let mut sub_streams_info = SubStreamsInfo::default();
        sub_streams_info
            .unpack_sizes
            .resize(total_unpack_streams, Default::default());
        sub_streams_info
            .has_crc
            .reserve_len_exact(total_unpack_streams);
        sub_streams_info.crcs = vec![0; total_unpack_streams];

        let mut next_unpack_stream = 0;
        for folder in archive.folders.iter() {
            if folder.num_unpack_sub_streams == 0 {
                continue;
            }
            let mut sum = 0;
            if nid == K_SIZE {
                for _i in 0..folder.num_unpack_sub_streams - 1 {
                    let size = read_u64(header)?;
                    sub_streams_info.unpack_sizes[next_unpack_stream] = size;
                    next_unpack_stream += 1;
                    sum += size;
                }
            }
            if sum > folder.get_unpack_size() {
                return Err(Error::other(
                    "sum of unpack sizes of folder exceeds total unpack size",
                ));
            }
            sub_streams_info.unpack_sizes[next_unpack_stream] = folder.get_unpack_size() - sum;
            next_unpack_stream += 1;
        }
        if nid == K_SIZE {
            nid = read_u8(header)?;
        }

        let mut num_digests = 0;
        for folder in archive.folders.iter() {
            if folder.num_unpack_sub_streams != 1 || !folder.has_crc {
                num_digests += folder.num_unpack_sub_streams;
            }
        }

        if nid == K_CRC {
            let has_missing_crc = read_all_or_bits(header, num_digests)?;
            let mut missing_crcs = vec![0; num_digests];
            for i in 0..num_digests {
                if has_missing_crc.contains(i) {
                    missing_crcs[i] = read_u32(header)? as u64;
                }
            }
            let mut next_crc = 0;
            let mut next_missing_crc = 0;
            for folder in archive.folders.iter() {
                if folder.num_unpack_sub_streams == 1 && folder.has_crc {
                    sub_streams_info.has_crc.insert(next_crc);
                    sub_streams_info.crcs[next_crc] = folder.crc;
                    next_crc += 1;
                } else {
                    for _i in 0..folder.num_unpack_sub_streams {
                        if has_missing_crc.contains(next_missing_crc) {
                            sub_streams_info.has_crc.insert(next_crc);
                        } else {
                            sub_streams_info.has_crc.remove(next_crc);
                        }
                        sub_streams_info.crcs[next_crc] = missing_crcs[next_missing_crc];
                        next_crc += 1;
                        next_missing_crc += 1;
                    }
                }
            }

            nid = read_u8(header)?;
        }

        if nid != K_END {
            return Err(Error::BadTerminatedSubStreamsInfo);
        }

        archive.sub_streams_info = Some(sub_streams_info);
        Ok(())
    }

    fn read_folder<R: Read>(header: &mut R) -> Result<Folder, Error> {
        let mut folder = Folder::default();

        let num_coders = read_usize(header, "num coders")?;
        let mut coders = Vec::with_capacity(num_coders);
        let mut total_in_streams = 0;
        let mut total_out_streams = 0;
        for _i in 0..num_coders {
            let mut coder = Coder::default();
            let bits = read_u8(header)?;
            let id_size = bits & 0xf;
            let is_simple = (bits & 0x10) == 0;
            let has_attributes = (bits & 0x20) != 0;
            let more_alternative_methods = (bits & 0x80) != 0;

            coder.id_size = id_size as usize;

            header
                .read(coder.decompression_method_id_mut())
                .map_err(Error::io)?;
            if is_simple {
                coder.num_in_streams = 1;
                coder.num_out_streams = 1;
            } else {
                coder.num_in_streams = read_u64(header)?;
                coder.num_out_streams = read_u64(header)?;
            }
            total_in_streams += coder.num_in_streams;
            total_out_streams += coder.num_out_streams;
            if has_attributes {
                let properties_size = read_usize(header, "properties size")?;
                let mut props = vec![0u8; properties_size];
                header.read(&mut props).map_err(Error::io)?;
                coder.properties = props;
            }
            coders.push(coder);
            // would need to keep looping as above:
            if more_alternative_methods {
                return Err(Error::other("Alternative methods are unsupported, please report. The reference implementation doesn't support them either."));
            }
        }
        folder.coders = coders;
        let total_in_streams = assert_usize(total_in_streams, "totalInStreams")?;
        let total_out_streams = assert_usize(total_out_streams, "totalOutStreams")?;
        folder.total_input_streams = total_in_streams;
        folder.total_output_streams = total_out_streams;

        if total_out_streams == 0 {
            return Err(Error::other("Total output streams can't be 0"));
        }
        let num_bind_pairs = total_out_streams - 1;
        let mut bind_pairs = Vec::with_capacity(num_bind_pairs);
        for _ in 0..num_bind_pairs {
            let bp = BindPair {
                in_index: read_u64(header)?,
                out_index: read_u64(header)?,
            };
            bind_pairs.push(bp);
        }
        folder.bind_pairs = bind_pairs;

        if total_in_streams < num_bind_pairs {
            return Err(Error::other(
                "Total input streams can't be less than the number of bind pairs",
            ));
        }
        let num_packed_streams = total_in_streams - num_bind_pairs;
        let mut packed_streams = vec![0; num_packed_streams];
        if num_packed_streams == 1 {
            let mut index = u64::MAX;
            for i in 0..total_in_streams {
                if folder.find_bind_pair_for_in_stream(i).is_none() {
                    index = i as u64;
                    break;
                }
            }
            if index == u64::MAX {
                return Err(Error::other("Couldn't find stream's bind pair index"));
            }
            packed_streams[0] = index;
        } else {
            for i in 0..num_packed_streams {
                packed_streams[i] = read_u64(header)?;
            }
        }
        folder.packed_streams = packed_streams;

        Ok(folder)
    }
}

#[inline]
fn crc32_cksum(data: &[u8]) -> u32 {
    CRC32.checksum(data)
}

#[inline]
fn read_usize<R: Read>(reader: &mut R, field: &str) -> Result<usize, Error> {
    let size = read_u64(reader)?;
    assert_usize(size, field)
}

#[inline]
fn assert_usize(size: u64, field: &str) -> Result<usize, Error> {
    if size > usize::MAX as u64 {
        return Err(Error::other(format!("Cannot handle {} {}", field, size)));
    }
    Ok(size as usize)
}

#[inline]
fn read_u64le<R: Read>(reader: &mut R) -> Result<u64, Error> {
    let mut buf = [0; 8];
    reader.read_exact(&mut buf).map_err(Error::io)?;
    Ok(u64::from_le_bytes(buf))
}

fn read_u64<R: Read>(reader: &mut R) -> Result<u64, Error> {
    let first = read_u8(reader)? as u64;
    let mut mask = 0x80_u64;
    let mut value = 0;
    for i in 0..8 {
        if (first & mask) == 0 {
            return Ok(value | ((first & (mask - 1)) << (8 * i)));
        }
        let b = read_u8(reader)? as u64;
        value |= b << (8 * i);
        mask >>= 1;
    }
    Ok(value)
}

#[inline(always)]
fn read_u32<R: Read>(reader: &mut R) -> Result<u32, Error> {
    let mut buf = [0; 4];
    reader.read_exact(&mut buf).map_err(Error::io)?;
    Ok(u32::from_le_bytes(buf))
}

#[inline(always)]
fn read_u8<R: Read>(reader: &mut R) -> Result<u8, Error> {
    let mut buf = [0];
    reader.read_exact(&mut buf).map_err(Error::io)?;
    Ok(buf[0])
}

fn read_all_or_bits<R: Read>(header: &mut R, size: usize) -> Result<BitSet, Error> {
    let all = read_u8(header)?;
    if all != 0 {
        let mut bits = BitSet::with_capacity(size);
        for i in 0..size {
            bits.insert(i);
        }
        Ok(bits)
    } else {
        read_bits(header, size)
    }
}

fn read_bits<R: Read>(header: &mut R, size: usize) -> Result<BitSet, Error> {
    let mut bits = BitSet::with_capacity(size);
    let mut mask = 0u32;
    let mut cache = 0u32;
    for i in 0..size {
        if mask == 0 {
            mask = 0x80;
            cache = read_u8(header)? as u32;
        }
        if (cache & mask) != 0 {
            bits.insert(i);
        }
        mask >>= 1;
    }
    Ok(bits)
}

struct NamesReader<'a, R: Read> {
    max_bytes: usize,
    read_bytes: usize,
    cache: Vec<u16>,
    reader: &'a mut R,
}

impl<'a, R: Read> NamesReader<'a, R> {
    fn new(reader: &'a mut R, max_bytes: usize) -> Self {
        Self {
            max_bytes,
            reader,
            read_bytes: 0,
            cache: Vec::with_capacity(16),
        }
    }
}

impl<'a, R: Read> Iterator for NamesReader<'a, R> {
    type Item = Result<String, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.max_bytes <= self.read_bytes {
            return None;
        }
        self.cache.clear();
        let mut buf = [0; 2];
        while self.read_bytes < self.max_bytes {
            let r = self.reader.read_exact(&mut buf).map_err(Error::io);
            self.read_bytes += 2;
            if let Err(e) = r {
                return Some(Err(e));
            }
            let u = u16::from_le_bytes(buf);
            if u == 0 {
                break;
            }
            self.cache.push(u);
        }

        Some(String::from_utf16(&self.cache).map_err(|e| Error::other(e.to_string())))
    }
}

pub struct SevenZReader<R: Read + Seek> {
    source: R,
    archive: Archive,
    password: Vec<u8>,
}

#[cfg(not(target_arch = "wasm32"))]
impl SevenZReader<File> {
    #[inline]
    pub fn open(path: impl AsRef<std::path::Path>, password: Password) -> Result<Self, Error> {
        let file = std::fs::File::open(path.as_ref())
            .map_err(|e| Error::file_open(e, path.as_ref().to_string_lossy().to_string()))?;
        let len = file.metadata().map(|m| m.len()).map_err(Error::io)?;
        Self::new(file, len, password)
    }
}

impl<R: Read + Seek> SevenZReader<R> {
    #[inline]
    pub fn new(mut source: R, reader_len: u64, password: Password) -> Result<Self, Error> {
        let password = password.to_vec();
        let archive = Archive::read(&mut source, reader_len, &password)?;
        Ok(Self {
            source,
            archive,
            password,
        })
    }

    #[inline]
    pub fn from_archive(archive: Archive, source: R, password: Password) -> Self {
        Self {
            source,
            archive,
            password: password.to_vec(),
        }
    }

    #[inline]
    pub fn archive(&self) -> &Archive {
        &self.archive
    }

    fn build_decode_stack<'r>(
        source: &'r mut R,
        archive: &Archive,
        folder_index: usize,
        password: &[u8],
    ) -> Result<(Box<dyn Read + 'r>, usize), Error> {
        let folder = &archive.folders[folder_index];
        if folder.total_input_streams > folder.total_output_streams {
            return Self::build_decode_stack2(source, archive, folder_index, password);
        }
        let first_pack_stream_index =
            archive.stream_map.folder_first_pack_stream_index[folder_index];
        let folder_offset = SIGNATURE_HEADER_SIZE
            + archive.pack_pos
            + archive.stream_map.pack_stream_offsets[first_pack_stream_index];

        source
            .seek(SeekFrom::Start(folder_offset))
            .map_err(Error::io)?;
        let pack_size = archive.pack_sizes[first_pack_stream_index] as usize;

        let mut decoder: Box<dyn Read> = Box::new(BoundedReader::new(source, pack_size));
        let folder = &archive.folders[folder_index];
        for (index, coder) in folder.ordered_coder_iter() {
            if coder.num_in_streams != 1 || coder.num_out_streams != 1 {
                return Err(Error::unsupported(
                    "Multi input/output stream coders are not yet supported",
                ));
            }
            let next = crate::decoders::add_decoder(
                decoder,
                folder.get_unpack_size_at_index(index) as usize,
                coder,
                password,
                MAX_MEM_LIMIT_KB,
            )?;
            decoder = Box::new(next);
        }
        let methods: Vec<_> = folder.ordered_coder_iter().map(|(_,c)|c.decompression_method_id()).collect();
        if methods.as_slice() == [SevenZMethod::ID_LZMA2]
            || methods.as_slice() == [SevenZMethod::ID_AES256SHA256,SevenZMethod::ID_LZMA2]
        {
            decoder = Box::new(Lzma2EndReader { inner: decoder, remaining: folder.get_unpack_size() });
        }
        if folder.has_crc {
            decoder = Box::new(Crc32VerifyingReader::new(
                decoder,
                folder.get_unpack_size() as usize,
                folder.crc,
            ));
        }

        Ok((decoder, pack_size))
    }

    fn build_decode_stack2<'r>(
        source: &'r mut R,
        archive: &Archive,
        folder_index: usize,
        password: &[u8],
    ) -> Result<(Box<dyn Read + 'r>, usize), Error> {
        const MAX_CODER_COUNT: usize = 32;
        let folder = &archive.folders[folder_index];
        if folder.coders.len() > MAX_CODER_COUNT {
            return Err(Error::unsupported(format!(
                "Too many coders: {}",
                folder.coders.len()
            )));
        }

        assert!(folder.total_input_streams > folder.total_output_streams);
        let source = ReaderPtr::new(source);
        let first_pack_stream_index =
            archive.stream_map.folder_first_pack_stream_index[folder_index];
        let start_pos = SIGNATURE_HEADER_SIZE + archive.pack_pos;
        let offsets = &archive.stream_map.pack_stream_offsets[first_pack_stream_index..];

        let mut sources = Vec::with_capacity(folder.packed_streams.len());
        for i in 0..folder.packed_streams.len() {
            let pack_pos = start_pos + offsets[i];
            let pack_size = archive.pack_sizes[first_pack_stream_index + i];
            let pack_reader =
                SeekableBoundedReader::new(source.clone(), (pack_pos, pack_pos + pack_size));
            sources.push(pack_reader);
        }

        let mut coder_to_stream_map = [usize::MAX; MAX_CODER_COUNT];

        let mut si = 0;
        for i in 0..folder.coders.len() {
            coder_to_stream_map[i] = si;
            si += folder.coders[i].num_in_streams as usize;
        }

        let main_coder_index = {
            let mut coder_used = [false; MAX_CODER_COUNT];
            for bp in folder.bind_pairs.iter() {
                coder_used[bp.out_index as usize] = true;
            }
            let mut mci = 0;
            for i in 0..folder.coders.len() {
                if !coder_used[i] {
                    mci = i;
                    break;
                }
            }
            mci
        };

        let id = folder.coders[main_coder_index].decompression_method_id();
        if id != SevenZMethod::ID_BCJ2 {
            return Err(Error::unsupported(format!("Unsupported method: {:?}", id)));
        }

        let num_in_streams = folder.coders[main_coder_index].num_in_streams as usize;
        let mut inputs: Vec<Box<dyn Read>> = Vec::with_capacity(num_in_streams);
        let start_i = coder_to_stream_map[main_coder_index];
        for i in start_i..num_in_streams + start_i {
            inputs.push(Self::get_in_stream(
                folder,
                &sources,
                &coder_to_stream_map,
                password,
                i,
            )?);
        }
        let mut decoder: Box<dyn Read> = Box::new(crate::bcj2::BCJ2Reader::new(
            inputs,
            folder.get_unpack_size(),
        ));
        if folder.has_crc {
            decoder = Box::new(Crc32VerifyingReader::new(
                decoder,
                folder.get_unpack_size() as usize,
                folder.crc,
            ));
        }
        Ok((
            decoder,
            archive.pack_sizes[first_pack_stream_index] as usize,
        ))
    }

    fn get_in_stream<'r>(
        folder: &Folder,
        sources: &[SeekableBoundedReader<ReaderPtr<R>>],
        coder_to_stream_map: &[usize],
        password: &[u8],

        in_stream_index: usize,
    ) -> Result<Box<dyn Read + 'r>, Error>
    where
        R: 'r,
    {
        let index = folder
            .packed_streams
            .iter()
            .position(|&i| i == in_stream_index as u64);
        if let Some(index) = index {
            return Ok(Box::new(sources[index].clone()));
        }

        let bp = folder
            .find_bind_pair_for_in_stream(in_stream_index)
            .ok_or_else(|| {
                Error::other(format!(
                    "Couldn't find bind pair for stream {}",
                    in_stream_index
                ))
            })?;
        let index = folder.bind_pairs[bp].out_index as usize;

        Self::get_in_stream2(folder, sources, coder_to_stream_map, password, index)
    }

    fn get_in_stream2<'r>(
        folder: &Folder,
        sources: &[SeekableBoundedReader<ReaderPtr<R>>],
        coder_to_stream_map: &[usize],
        password: &[u8],
        in_stream_index: usize,
    ) -> Result<Box<dyn Read + 'r>, Error>
    where
        R: 'r,
    {
        let coder = &folder.coders[in_stream_index];
        let start_index = coder_to_stream_map[in_stream_index];
        if start_index == usize::MAX {
            return Err(Error::other("in_stream_index out of range"));
        }
        let uncompressed_len = folder.unpack_sizes[in_stream_index] as usize;
        if coder.num_in_streams == 1 {
            let input =
                Self::get_in_stream(folder, sources, coder_to_stream_map, password, start_index)?;

            let decoder = add_decoder(input, uncompressed_len, coder, password, MAX_MEM_LIMIT_KB)?;
            return Ok(Box::new(decoder));
        }
        Err(Error::unsupported(
            "Multi input stream coders are not yet supported",
        ))
    }

    /// Takes a closure to decode each files in the archive.
    ///
    /// Attention about solid archive:
    /// When decoding a solid archive, the data to be decompressed depends on the data in front of it,
    /// you cannot simply skip the previous data and only decompress the data in the back.
    ///
    /// See [ChecksumVerificationFailed](https://github.com/dyz1990/sevenz-rust/issues/31).
    ///
    /// To speed up decompression, you can check this example [examples/forder_dec.rs](https://github.com/dyz1990/sevenz-rust/blob/main/examples/forder_dec.rs).
    /// And this example [mt_decompress.rs](https://github.com/dyz1990/sevenz-rust/blob/main/examples/mt_decompress.rs) if you want use multi-thread.
    ///
    pub fn for_each_entries<F: FnMut(&SevenZArchiveEntry, &mut dyn Read) -> Result<bool, Error>>(
        &mut self,
        mut each: F,
    ) -> Result<(), Error> {
        let folder_count = self.archive.folders.len();
        for folder_index in 0..folder_count {
            let forder_dec = BlockDecoder::new(
                folder_index,
                &self.archive,
                &self.password,
                &mut self.source,
            );
            forder_dec.for_each_entries(&mut each)?;
        }
        // decode empty files
        for file_index in 0..self.archive.files.len() {
            let folder_index = self.archive.stream_map.file_folder_index[file_index];
            if folder_index.is_none() {
                let file = &self.archive.files[file_index];
                let empty_reader: &mut dyn Read = &mut ([0u8; 0].as_slice());
                if !each(file, empty_reader)? {
                    return Ok(());
                }
            }
        }
        Ok(())
    }
}

/// Alias for ['BlockDecoder'], used for compatibility purposes.
#[allow(unused)]
#[deprecated]
pub type FolderDecoder<'a, R> = BlockDecoder<'a, R>;

pub struct BlockDecoder<'a, R: Read + Seek> {
    folder_index: usize,
    archive: &'a Archive,
    password: &'a [u8],
    source: &'a mut R,
}

impl<'a, R: Read + Seek> BlockDecoder<'a, R> {
    /// Complete a bounded PPMd7 shared block, optionally preceded by AES.
    /// Check each mapped stream, block/pack CRCs and the size-defined finish.
    #[cfg(feature="additional-codecs")]
    pub fn read_solid_ppmd_entry(self, selected:usize, max_selected:usize, max_shared:u64, mut check:impl FnMut()->Result<(),Error>) -> Result<Vec<u8>,Error> {
        let a=self.archive;let f=a.folders.get(self.folder_index).ok_or_else(||Error::other("Missing shared PPMd block"))?;
        let files:Vec<_>=a.files.iter().enumerate().filter(|(i,e)|e.has_stream && a.stream_map.file_folder_index.get(*i)==Some(&Some(self.folder_index))).collect();
        let e=files.iter().find(|(i,_)|*i==selected).ok_or_else(||Error::other("Selected file is outside shared PPMd block"))?.1;
        let total=files.iter().try_fold(0u64,|n,(_,e)|n.checked_add(e.size)).ok_or_else(||Error::other("Shared PPMd size overflow"))?;
        if max_selected==0 || max_selected>8*1024*1024 || max_shared==0 || max_shared>64*1024*1024
            || e.size>max_selected as u64 || total>max_shared || total!=f.get_unpack_size() || files.len()!=f.num_unpack_sub_streams
            || files.iter().any(|(_,e)|e.is_directory || e.is_anti_item || (!e.has_crc && !f.has_crc))
            || !(1..=2).contains(&f.coders.len()) || f.packed_streams.len()!=1
        {return Err(Error::unsupported("7Z PPMd 固实共享映射或大小无法确认，请先提取后打开"));}
        let coders:Vec<_>=f.ordered_coder_iter().collect();
        if coders.len()!=f.coders.len() || f.coders.iter().any(|c|c.num_in_streams!=1 || c.num_out_streams!=1)
            || !(coders.len()==1 && coders[0].1.decompression_method_id()==SevenZMethod::ID_PPMD
                || coders.len()==2 && coders[0].1.decompression_method_id()==SevenZMethod::ID_AES256SHA256 && coders[1].1.decompression_method_id()==SevenZMethod::ID_PPMD)
        {return Err(Error::unsupported("7Z PPMd 固实预览方法或顺序无法确认，请先提取后打开"));}
        let encrypted=coders.len()==2;let c=coders.last().unwrap().1;let p=&c.properties;
        if p.len()!=5 {
            return Err(Error::unsupported("7Z PPMd 固实预览方法或参数无法确认，请先提取后打开"));
        }
        let order=p[0] as u32;let memory=u32::from_le_bytes(p[1..5].try_into().unwrap());
        if !(2..=64).contains(&order) || !(2048..=64*1024*1024).contains(&memory) {return Err(Error::unsupported("7Z PPMd 固实预览阶数或字典超出范围，请先提取后打开"));}
        crate::preview::coder(c,total as usize)?;check()?;
        let index=*a.stream_map.folder_first_pack_stream_index.get(self.folder_index).ok_or_else(||Error::other("Missing shared PPMd pack mapping"))?;
        let size=*a.pack_sizes.get(index).ok_or_else(||Error::other("Missing shared PPMd packed size"))?;
        if size>max_shared {return Err(Error::unsupported("7Z PPMd 固实压缩数据超过 64 MiB，请先提取后打开"));}
        let plain=if encrypted {f.get_unpack_size_at_index(coders[0].0)} else {size};
        if plain==0 || plain>max_shared {return Err(Error::unsupported("7Z PPMd 固实预览输入长度超出范围，请先提取后打开"));}
        if encrypted && plain.checked_add(15).map(|n|n/16*16)!=Some(size) {return Err(Error::other("Shared PPMd AES packed size mismatch"));}
        let start=32u64.checked_add(a.pack_pos).and_then(|n|a.stream_map.pack_stream_offsets.get(index).and_then(|o|n.checked_add(*o))).ok_or_else(||Error::other("Shared PPMd pack offset overflow"))?;
        self.source.seek(SeekFrom::Start(start)).map_err(Error::io)?;
        let mut packed=PreviewPack{source:self.source,start,size,read:0,crc:CRC32.digest()};
        let mut stream:Box<dyn Read+'_>=if encrypted {
            #[cfg(feature="aes256")]
            {crate::preview::coder(coders[0].1,plain as usize)?;Box::new(crate::aes256sha256::PreviewAes::new(&mut packed,&coders[0].1.properties,self.password,plain as usize)?)}
            #[cfg(not(feature="aes256"))]
            {return Err(Error::unsupported("AES is unavailable"));}
        } else {Box::new(&mut packed)};
        let mut limited=(&mut stream).take(plain);
        let mut decoder=ppmd_rust::Ppmd7Decoder::new(&mut limited,order,memory).map_err(|e|Error::other(format!("Shared PPMd decoder: {e}")))?;
        let mut output=Vec::with_capacity(e.size as usize);let mut buffer=[0;32*1024];let mut block_crc=CRC32.digest();
        for(i,file)in files {
            let mut remaining=file.size;let mut file_crc=CRC32.digest();
            while remaining>0 {
                check()?;let capacity=remaining.min(buffer.len() as u64) as usize;let n=decoder.read(&mut buffer[..capacity]).map_err(Error::io)?;
                if n==0 {return Err(Error::other("Shared PPMd ended before declared file size"));}
                let chunk=&buffer[..n];file_crc.update(chunk);block_crc.update(chunk);if i==selected {output.extend_from_slice(chunk);}remaining-=n as u64;
            }
            if file.has_crc && file_crc.finalize() as u64!=file.crc {return Err(Error::ChecksumVerificationFailed);}
        }
        // Standard 7z PPMd has no EOS: finish the declared total, not another symbol.
        decoder.finish_7z().map_err(Error::io)?;
        if limited.limit()!=0 {return Err(Error::other("Shared PPMd compressed plaintext was not completely consumed"));}
        drop(limited);
        // Drain only raw/decrypted EOF to finalize AES padding and its receipt.
        let mut extra=[0];if stream.read(&mut extra).map_err(Error::io)?!=0 {return Err(Error::other("Shared PPMd extra plaintext"));}
        drop(stream);
        if packed.read!=size {return Err(Error::other("Shared PPMd packed stream was not completely consumed"));}
        if f.has_crc && block_crc.finalize() as u64!=f.crc || a.pack_crcs_defined.contains(index) && a.pack_crcs.get(index).copied()!=Some(packed.crc.finalize() as u64) {return Err(Error::ChecksumVerificationFailed);}
        check()?;Ok(output)
    }

    /// Complete one non-solid PPMd7 stream, optionally preceded by AES.
    /// Size-delimited PPMd verifies range finish without an extra symbol;
    /// AES still requires complete plaintext/ciphertext and valid padding.
    #[cfg(feature="additional-codecs")]
    pub fn read_ppmd_entry(self, selected:usize, max_bytes:usize, mut check:impl FnMut()->Result<(),Error>) -> Result<Vec<u8>,Error> {
        let a=self.archive;let f=a.folders.get(self.folder_index).ok_or_else(||Error::other("Missing PPMd block"))?;
        let e=a.files.get(selected).ok_or_else(||Error::other("Missing PPMd entry"))?;
        if max_bytes==0 || max_bytes>8*1024*1024 || e.size>max_bytes as u64 || !e.has_stream || e.is_directory || e.is_anti_item
            || a.stream_map.file_folder_index.get(selected)!=Some(&Some(self.folder_index))
            || a.files.iter().enumerate().filter(|(i,e)|e.has_stream && a.stream_map.file_folder_index.get(*i)==Some(&Some(self.folder_index))).count()!=1
            || f.num_unpack_sub_streams!=1 || f.get_unpack_size()!=e.size || !(1..=2).contains(&f.coders.len()) || f.packed_streams.len()!=1
            || (!e.has_crc && !f.has_crc)
        {return Err(Error::unsupported("7Z PPMd 预览仅支持有完整 CRC 的非固实小文本或图片，请先提取后打开"));}
        let coders:Vec<_>=f.ordered_coder_iter().collect();
        if coders.len()!=f.coders.len() || f.coders.iter().any(|c|c.num_in_streams!=1 || c.num_out_streams!=1)
            || !(coders.len()==1 && coders[0].1.decompression_method_id()==SevenZMethod::ID_PPMD
                || coders.len()==2 && coders[0].1.decompression_method_id()==SevenZMethod::ID_AES256SHA256 && coders[1].1.decompression_method_id()==SevenZMethod::ID_PPMD)
        {return Err(Error::unsupported("7Z PPMd 预览方法或顺序无法确认，请先提取后打开"));}
        let encrypted=coders.len()==2;
        let c=coders.last().unwrap().1;let p=&c.properties;
        if p.len()!=5 {return Err(Error::unsupported("7Z PPMd 预览参数无法确认，请先提取后打开"));}
        let order=p[0] as u32;let memory=u32::from_le_bytes(p[1..5].try_into().unwrap());
        if !(2..=64).contains(&order) || !(2048..=64*1024*1024).contains(&memory) {return Err(Error::unsupported("7Z PPMd 预览阶数或字典超出范围，请先提取后打开"));}
        crate::preview::coder(c,e.size as usize)?;check()?;
        let index=*a.stream_map.folder_first_pack_stream_index.get(self.folder_index).ok_or_else(||Error::other("Missing PPMd pack mapping"))?;
        let size=*a.pack_sizes.get(index).ok_or_else(||Error::other("Missing PPMd pack size"))?;
        if size>64*1024*1024 {return Err(Error::unsupported("7Z PPMd 预览压缩数据超过 64 MiB，请先提取后打开"));}
        let plain=if encrypted {f.get_unpack_size_at_index(coders[0].0)} else {size};
        if plain==0 || plain>64*1024*1024 {return Err(Error::unsupported("7Z PPMd 预览输入长度超出范围，请先提取后打开"));}
        if encrypted && plain.checked_add(15).map(|n|n/16*16)!=Some(size) {return Err(Error::other("PPMd AES packed size mismatch"));}
        let start=32u64.checked_add(a.pack_pos).and_then(|n|a.stream_map.pack_stream_offsets.get(index).and_then(|o|n.checked_add(*o))).ok_or_else(||Error::other("PPMd pack offset overflow"))?;
        self.source.seek(SeekFrom::Start(start)).map_err(Error::io)?;
        let mut packed=PreviewPack{source:self.source,start,size,read:0,crc:CRC32.digest()};
        let mut stream:Box<dyn Read+'_>=if encrypted {
            #[cfg(feature="aes256")]
            {crate::preview::coder(coders[0].1,plain as usize)?;Box::new(crate::aes256sha256::PreviewAes::new(&mut packed,&coders[0].1.properties,self.password,plain as usize)?)}
            #[cfg(not(feature="aes256"))]
            {return Err(Error::unsupported("AES is unavailable"));}
        } else {Box::new(&mut packed)};
        let mut limited=(&mut stream).take(plain);
        let mut decoder=ppmd_rust::Ppmd7Decoder::new(&mut limited,order,memory).map_err(|e|Error::other(format!("PPMd decoder: {e}")))?;
        let mut bytes=Vec::with_capacity(e.size as usize);let mut buffer=[0;32*1024];
        while bytes.len()<e.size as usize {
            check()?;let capacity=buffer.len().min(e.size as usize-bytes.len());let n=decoder.read(&mut buffer[..capacity]).map_err(Error::io)?;
            if n==0 {return Err(Error::other("PPMd output ended before declared size"));}bytes.extend_from_slice(&buffer[..n]);
        }
        decoder.finish_7z().map_err(Error::io)?;
        if limited.limit()!=0 {return Err(Error::other("PPMd compressed plaintext was not completely consumed"));}
        drop(limited);
        // Check the raw/decrypted EOF, never decode an additional PPMd symbol.
        let mut extra=[0];if stream.read(&mut extra).map_err(Error::io)?!=0 {return Err(Error::other("PPMd extra plaintext"));}
        drop(stream);
        if packed.read!=size {return Err(Error::other("PPMd compressed stream was not completely consumed"));}
        let crc=CRC32.checksum(&bytes) as u64;
        if e.has_crc && e.crc!=crc || f.has_crc && f.crc!=crc || a.pack_crcs_defined.contains(index) && a.pack_crcs.get(index).copied()!=Some(packed.crc.finalize() as u64) {return Err(Error::ChecksumVerificationFailed);}
        check()?;Ok(bytes)
    }

    /// Explicit partial LZMA/LZMA2/PPMd shared-block read, optionally preceded by AES.
    /// Decode/discard only the
    /// bounded dependencies before this exact entry, then retain its prefix.
    /// This never verifies preceding CRCs, unread tail or complete codec end.
    /// The complete shared reader below remains a separate verification path.
    pub fn read_solid_entry_prefix(self, selected: usize, max_bytes: usize, max_decoded: u64, mut check: impl FnMut() -> Result<(), Error>) -> Result<Vec<u8>, Error> {
        let archive=self.archive;
        let folder=archive.folders.get(self.folder_index).ok_or_else(||Error::other("Missing shared prefix block"))?;
        let files:Vec<_>=archive.files.iter().enumerate().filter(|(i,e)|e.has_stream && archive.stream_map.file_folder_index.get(*i)==Some(&Some(self.folder_index))).collect();
        let position=files.iter().position(|(i,_)|*i==selected).ok_or_else(||Error::other("Selected prefix is outside shared block"))?;
        let total=files.iter().try_fold(0u64,|n,(_,e)|n.checked_add(e.size)).ok_or_else(||Error::other("Shared prefix size overflow"))?;
        let offset=files[..position].iter().try_fold(0u64,|n,(_,e)|n.checked_add(e.size)).ok_or_else(||Error::other("Shared prefix offset overflow"))?;
        if max_bytes==0 || max_bytes>1024*1024 || max_decoded>64*1024*1024 || files[position].1.size<=max_bytes as u64
            || files.len()!=folder.num_unpack_sub_streams || total!=folder.get_unpack_size()
            || files.iter().any(|(_,e)|e.is_directory || e.is_anti_item)
            || !(1..=2).contains(&folder.coders.len()) || folder.packed_streams.len()!=1
            || folder.coders.iter().any(|c|c.num_in_streams!=1 || c.num_out_streams!=1)
        {return Err(Error::unsupported("7Z 固实预览映射或方法无法确认，请先提取后打开"));}
        let coders:Vec<_>=folder.ordered_coder_iter().collect();
        let codec=|id:&[u8]|id==SevenZMethod::ID_LZMA || id==SevenZMethod::ID_LZMA2 || id==SevenZMethod::ID_PPMD;
        if coders.len()!=folder.coders.len() || !(coders.len()==1 && codec(coders[0].1.decompression_method_id()) || coders.len()==2 && coders[0].1.decompression_method_id()==SevenZMethod::ID_AES256SHA256 && codec(coders[1].1.decompression_method_id())) {
            return Err(Error::unsupported("Shared prefix requires LZMA/LZMA2/PPMd, optionally preceded by AES"));
        }
        if offset.checked_add(max_bytes as u64).is_none_or(|n|n>max_decoded) {
            return Err(Error::unsupported("7Z 固实预览前置内容与开头合计超过 64 MiB，请先提取后打开"));
        }
        check()?;
        let pack_index=*archive.stream_map.folder_first_pack_stream_index.get(self.folder_index).ok_or_else(||Error::other("Missing shared prefix pack mapping"))?;
        let packed=*archive.pack_sizes.get(pack_index).ok_or_else(||Error::other("Missing shared prefix pack size"))?;
        let start=32u64.checked_add(archive.pack_pos).and_then(|n|archive.stream_map.pack_stream_offsets.get(pack_index).and_then(|offset|n.checked_add(*offset))).ok_or_else(||Error::other("Shared prefix pack offset overflow"))?;
        let unpacked=usize::try_from(total).map_err(|_|Error::unsupported("Shared prefix declared size exceeds platform range"))?;
        self.source.seek(SeekFrom::Start(start)).map_err(Error::io)?;
        let packed_input=self.source.take(packed);
        let stream:Box<dyn Read+'_>=if coders.len()==2 {
            #[cfg(feature="aes256")]
            {
                let (index,coder)=coders[0];crate::preview::coder(coder,0)?;
                let plain=usize::try_from(folder.get_unpack_size_at_index(index)).map_err(|_|Error::unsupported("Shared AES prefix size exceeds platform range"))?;
                if plain.checked_add(15).map(|n|(n/16*16) as u64)!=Some(packed){return Err(Error::other("Shared AES prefix packed size mismatch"));}
                // Intentional partial payload only. Header/full-entry AES
                // streams still register and require complete consumption.
                Box::new(crate::aes256sha256::PreviewAes::new_prefix(packed_input,&coder.properties,self.password,plain)?)
            }
            #[cfg(not(feature="aes256"))]
            {return Err(Error::unsupported("AES is unavailable"));}
        } else {Box::new(packed_input)};
        let mut decoder=crate::decoders::add_prefix_decoder(stream,unpacked,coders.last().unwrap().1,MAX_MEM_LIMIT_KB)?;
        let mut bytes=Vec::with_capacity(max_bytes);let mut buffer=[0;32*1024];
        for (keep,mut remaining) in [(false,offset),(true,max_bytes as u64)] {
            while remaining>0 {
                check()?;let capacity=remaining.min(buffer.len() as u64) as usize;
                let n=decoder.read(&mut buffer[..capacity]).map_err(Error::io)?;
                if n==0 {return Err(Error::other("7Z 固实预览在所选开头前结束，数据与声明大小不符，请重新下载或先提取检查"));}
                if keep {bytes.extend_from_slice(&buffer[..n]);}remaining-=n as u64;
            }
        }
        check()?;Ok(bytes)
    }

    /// Read a selected non-solid LZMA/LZMA2/PPMd prefix, optionally preceded by AES.
    /// The encrypted payload uses a distinct partial constructor; metadata and
    /// complete readers retain their full AES consumption receipts.
    /// This deliberately does not verify the unread tail, CRCs or codec end.
    /// Complete entry readers retain their independent full verification path.
    pub fn read_single_entry_prefix(self, selected: usize, max_bytes: usize, mut check: impl FnMut() -> Result<(), Error>) -> Result<Vec<u8>, Error> {
        let archive = self.archive;
        let folder = archive.folders.get(self.folder_index).ok_or_else(|| Error::other("Missing prefix block"))?;
        let entry = archive.files.get(selected).ok_or_else(|| Error::other("Missing prefix file"))?;
        if max_bytes == 0 || max_bytes > 1024 * 1024 || entry.size <= max_bytes as u64
            || !entry.has_stream || entry.is_directory || entry.is_anti_item
            || archive.stream_map.file_folder_index.get(selected) != Some(&Some(self.folder_index))
            || folder.num_unpack_sub_streams != 1 || folder.get_unpack_size() != entry.size
            || !(1..=2).contains(&folder.coders.len()) || folder.packed_streams.len() != 1
            || folder.coders.iter().any(|c|c.num_in_streams!=1 || c.num_out_streams!=1)
            || archive.files.iter().enumerate().filter(|(i,e)|e.has_stream && archive.stream_map.file_folder_index.get(*i)==Some(&Some(self.folder_index))).count() != 1
        { return Err(Error::unsupported("Prefix requires one complete mapped non-solid stream")); }
        let coders:Vec<_>=folder.ordered_coder_iter().collect();
        let codec=|id:&[u8]|id==SevenZMethod::ID_LZMA || id==SevenZMethod::ID_LZMA2 || id==SevenZMethod::ID_PPMD;
        if coders.len()!=folder.coders.len() || !(coders.len()==1 && codec(coders[0].1.decompression_method_id()) || coders.len()==2 && coders[0].1.decompression_method_id()==SevenZMethod::ID_AES256SHA256 && codec(coders[1].1.decompression_method_id())) {
            return Err(Error::unsupported("Prefix requires LZMA/LZMA2/PPMd, optionally preceded by AES"));
        }
        check()?;
        let pack_index = *archive.stream_map.folder_first_pack_stream_index.get(self.folder_index).ok_or_else(|| Error::other("Missing prefix pack mapping"))?;
        let size = *archive.pack_sizes.get(pack_index).ok_or_else(|| Error::other("Missing prefix pack size"))?;
        let start = 32u64.checked_add(archive.pack_pos).and_then(|n| archive.stream_map.pack_stream_offsets.get(pack_index).and_then(|offset|n.checked_add(*offset))).ok_or_else(|| Error::other("Prefix pack offset overflow"))?;
        let unpacked = usize::try_from(entry.size).map_err(|_|Error::unsupported("Prefix declared size exceeds platform range"))?;
        self.source.seek(SeekFrom::Start(start)).map_err(Error::io)?;
        let packed=self.source.take(size);
        let stream:Box<dyn Read+'_>=if coders.len()==2 {
            #[cfg(feature="aes256")]
            {
                let (index,coder)=coders[0];
                crate::preview::coder(coder,0)?;
                let plain=usize::try_from(folder.get_unpack_size_at_index(index)).map_err(|_|Error::unsupported("AES prefix size exceeds platform range"))?;
                if plain.checked_add(15).map(|n|(n/16*16) as u64)!=Some(size){return Err(Error::other("AES prefix packed size mismatch"));}
                Box::new(crate::aes256sha256::PreviewAes::new_prefix(packed,&coder.properties,self.password,plain)?)
            }
            #[cfg(not(feature="aes256"))]
            {return Err(Error::unsupported("AES is unavailable"));}
        } else {Box::new(packed)};
        let mut decoder = crate::decoders::add_prefix_decoder(stream, unpacked, coders.last().unwrap().1, MAX_MEM_LIMIT_KB)?;
        let mut bytes = Vec::with_capacity(max_bytes);
        let mut buffer = [0u8;32*1024];
        while bytes.len() < max_bytes {
            check()?;
            let capacity = buffer.len().min(max_bytes-bytes.len());
            let n = decoder.read(&mut buffer[..capacity]).map_err(Error::io)?;
            if n == 0 { return Err(Error::other("7Z 文本开头在 1 MiB 前结束，数据与声明大小不符，请重新下载或先提取检查")); }
            bytes.extend_from_slice(&buffer[..n]);
        }
        check()?;
        Ok(bytes)
    }

    /// Explicit non-solid Deflate64 prefix, optionally preceded by partial AES.
    /// Stop at the requested output cap. Unread tail, CRCs, codec completion
    /// and payload AES completion are deliberately not claimed as verified.
    #[cfg(feature="additional-codecs")]
    pub fn read_deflate64_entry_prefix(self, selected:usize, max_bytes:usize, mut check:impl FnMut()->Result<(),Error>)->Result<Vec<u8>,Error>{
        let a=self.archive;
        let f=a.folders.get(self.folder_index).ok_or_else(||Error::other("Missing Deflate64 prefix block"))?;
        let e=a.files.get(selected).ok_or_else(||Error::other("Missing Deflate64 prefix file"))?;
        if max_bytes==0 || max_bytes>1024*1024 || e.size<=max_bytes as u64 || !e.has_stream || e.is_directory || e.is_anti_item
            || a.stream_map.file_folder_index.get(selected)!=Some(&Some(self.folder_index))
            || a.files.iter().enumerate().filter(|(i,e)|e.has_stream && a.stream_map.file_folder_index.get(*i)==Some(&Some(self.folder_index))).count()!=1
            || !(1..=2).contains(&f.coders.len()) || f.unpack_sizes.len()!=f.coders.len()
            || f.total_input_streams!=f.coders.len() || f.total_output_streams!=f.coders.len()
            || f.bind_pairs.len()!=f.coders.len()-1 || f.packed_streams.len()!=1
            || f.coders.iter().any(|c|c.num_in_streams!=1 || c.num_out_streams!=1)
            || f.num_unpack_sub_streams!=1 || f.get_unpack_size()!=e.size
        {return Err(Error::unsupported("7Z Deflate64 大文本预览需要唯一非固实条目和明确映射，请先提取后打开"));}
        let coders:Vec<_>=f.ordered_coder_iter().take(f.coders.len()+1).collect();
        if coders.len()!=f.coders.len() || !(coders.len()==1 && coders[0].1.decompression_method_id()==SevenZMethod::ID_DEFLATE64
            || coders.len()==2 && coders[0].1.decompression_method_id()==SevenZMethod::ID_AES256SHA256 && coders[1].1.decompression_method_id()==SevenZMethod::ID_DEFLATE64)
            || !coders.last().unwrap().1.properties.is_empty()
        {return Err(Error::unsupported("7Z Deflate64 大文本预览方法或顺序无法确认，请先提取后打开"));}
        let pack=*a.stream_map.folder_first_pack_stream_index.get(self.folder_index).ok_or_else(||Error::other("Missing Deflate64 prefix pack mapping"))?;
        let size=*a.pack_sizes.get(pack).ok_or_else(||Error::other("Missing Deflate64 prefix pack size"))?;
        let encrypted=coders.len()==2;
        let plain=if encrypted{f.get_unpack_size_at_index(coders[0].0)}else{size};
        if plain==0{return Err(Error::other("Deflate64 prefix has no compressed plaintext"));}
        if encrypted && plain.checked_add(15).map(|n|n/16*16)!=Some(size){return Err(Error::other("Deflate64 prefix AES packed size mismatch"));}
        let start=32u64.checked_add(a.pack_pos).and_then(|n|a.stream_map.pack_stream_offsets.get(pack).and_then(|o|n.checked_add(*o))).ok_or_else(||Error::other("Deflate64 prefix offset overflow"))?;
        check()?;self.source.seek(SeekFrom::Start(start)).map_err(Error::io)?;
        let packed=self.source.take(size);
        let mut stream:Box<dyn Read+'_>=if encrypted{
            #[cfg(feature="aes256")]
            {crate::preview::coder(coders[0].1,0)?;let n=usize::try_from(plain).map_err(|_|Error::unsupported("Deflate64 prefix size exceeds platform range"))?;Box::new(crate::aes256sha256::PreviewAes::new_prefix(packed,&coders[0].1.properties,self.password,n)?)}
            #[cfg(not(feature="aes256"))]
            {return Err(Error::unsupported("AES is unavailable"));}
        }else{Box::new(packed)};
        let mut codec=Box::new(deflate64::InflaterManaged::new());let mut bytes=Vec::with_capacity(max_bytes);
        let mut input=[0;32*1024];let mut output=[0;32*1024];let(mut offset,mut end,mut read_plain)=(0,0,0u64);
        while bytes.len()<max_bytes{
            check()?;
            // Drain already decoded output before requesting another chunk;
            // short raw/AES reads must not pull in an unrequested tail.
            if offset==end && codec.available_output()==0{
                offset=0;end=0;
                while end<input.len() && read_plain<plain{
                    check()?;let capacity=((input.len()-end) as u64).min(plain-read_plain) as usize;
                    let n=stream.read(&mut input[end..end+capacity]).map_err(Error::io)?;
                    if n==0{return Err(Error::other("7Z Deflate64 文本开头前数据结束，大小与声明不符，请重新下载或先提取检查"));}
                    end+=n;read_plain+=n as u64;
                }
            }
            let capacity=output.len().min(max_bytes-bytes.len());
            let result=codec.inflate(&input[offset..end],&mut output[..capacity]);
            if result.data_error || codec.errored(){return Err(Error::other("Deflate64 prefix invalid stream"));}
            offset+=result.bytes_consumed;
            // A real terminal may already be known while the fixed window
            // still holds output. Reject known size conflicts without draining
            // or certifying the unrequested tail.
            let observed=bytes.len() as u64+result.bytes_written as u64+codec.available_output() as u64;
            if codec.input_finished() && observed!=e.size{return Err(Error::other("7Z Deflate64 文本已提前结束，大小与声明不符，请先提取检查"));}
            if result.bytes_consumed==0 && result.bytes_written==0{return Err(Error::other("7Z Deflate64 文本开头前数据不足，请先提取检查"));}
            bytes.extend_from_slice(&output[..result.bytes_written]);
        }
        check()?;Ok(bytes)
    }

    /// Explicit non-solid Deflate prefix, optionally preceded by partial AES.
    /// Stop at the requested output cap. Unread tail, CRCs, codec completion
    /// and payload AES completion are deliberately not claimed as verified.
    #[cfg(feature="additional-codecs")]
    pub fn read_deflate_entry_prefix(self, selected:usize, max_bytes:usize, mut check:impl FnMut()->Result<(),Error>)->Result<Vec<u8>,Error>{
        let a=self.archive;
        let f=a.folders.get(self.folder_index).ok_or_else(||Error::other("Missing Deflate prefix block"))?;
        let e=a.files.get(selected).ok_or_else(||Error::other("Missing Deflate prefix file"))?;
        if max_bytes==0 || max_bytes>1024*1024 || e.size<=max_bytes as u64 || !e.has_stream || e.is_directory || e.is_anti_item
            || a.stream_map.file_folder_index.get(selected)!=Some(&Some(self.folder_index))
            || a.files.iter().enumerate().filter(|(i,e)|e.has_stream && a.stream_map.file_folder_index.get(*i)==Some(&Some(self.folder_index))).count()!=1
            || !(1..=2).contains(&f.coders.len()) || f.unpack_sizes.len()!=f.coders.len()
            || f.total_input_streams!=f.coders.len() || f.total_output_streams!=f.coders.len()
            || f.bind_pairs.len()!=f.coders.len()-1 || f.packed_streams.len()!=1
            || f.coders.iter().any(|c|c.num_in_streams!=1 || c.num_out_streams!=1)
            || f.num_unpack_sub_streams!=1 || f.get_unpack_size()!=e.size
        {return Err(Error::unsupported("7Z Deflate 大文本预览需要唯一非固实条目和明确映射，请先提取后打开"));}
        let coders:Vec<_>=f.ordered_coder_iter().take(f.coders.len()+1).collect();
        if coders.len()!=f.coders.len() || !(coders.len()==1 && coders[0].1.decompression_method_id()==SevenZMethod::ID_DEFLATE
            || coders.len()==2 && coders[0].1.decompression_method_id()==SevenZMethod::ID_AES256SHA256 && coders[1].1.decompression_method_id()==SevenZMethod::ID_DEFLATE)
            || !coders.last().unwrap().1.properties.is_empty()
        {return Err(Error::unsupported("7Z Deflate 大文本预览方法或顺序无法确认，请先提取后打开"));}
        let pack=*a.stream_map.folder_first_pack_stream_index.get(self.folder_index).ok_or_else(||Error::other("Missing Deflate prefix pack mapping"))?;
        let size=*a.pack_sizes.get(pack).ok_or_else(||Error::other("Missing Deflate prefix pack size"))?;
        let encrypted=coders.len()==2;
        let plain=if encrypted{f.get_unpack_size_at_index(coders[0].0)}else{size};
        if plain==0{return Err(Error::other("Deflate prefix has no compressed plaintext"));}
        if encrypted && plain.checked_add(15).map(|n|n/16*16)!=Some(size){return Err(Error::other("Deflate prefix AES packed size mismatch"));}
        let start=32u64.checked_add(a.pack_pos).and_then(|n|a.stream_map.pack_stream_offsets.get(pack).and_then(|o|n.checked_add(*o))).ok_or_else(||Error::other("Deflate prefix offset overflow"))?;
        check()?;self.source.seek(SeekFrom::Start(start)).map_err(Error::io)?;
        let packed=self.source.take(size);
        let mut stream:Box<dyn Read+'_>=if encrypted{
            #[cfg(feature="aes256")]
            {crate::preview::coder(coders[0].1,0)?;let n=usize::try_from(plain).map_err(|_|Error::unsupported("Deflate prefix size exceeds platform range"))?;Box::new(crate::aes256sha256::PreviewAes::new_prefix(packed,&coders[0].1.properties,self.password,n)?)}
            #[cfg(not(feature="aes256"))]
            {return Err(Error::unsupported("AES is unavailable"));}
        }else{Box::new(packed)};
        let mut codec=flate2::Decompress::new(false);let mut bytes=Vec::with_capacity(max_bytes);
        let mut input=[0;32*1024];let mut output=[0;32*1024];let(mut offset,mut end,mut read_plain)=(0,0,0u64);
        while bytes.len()<max_bytes{
            check()?;
            if offset==end{
                offset=0;end=0;
                while end<input.len() && read_plain<plain{
                    check()?;let capacity=((input.len()-end) as u64).min(plain-read_plain) as usize;
                    let n=stream.read(&mut input[end..end+capacity]).map_err(Error::io)?;
                    if n==0{return Err(Error::other("7Z Deflate 文本开头前数据结束，大小与声明不符，请重新下载或先提取检查"));}
                    end+=n;read_plain+=n as u64;
                }
            }
            let before_in=codec.total_in();let before_out=codec.total_out();let capacity=output.len().min(max_bytes-bytes.len());
            let status=codec.decompress(&input[offset..end],&mut output[..capacity],flate2::FlushDecompress::None).map_err(|e|Error::other(format!("Deflate prefix: {e}")))?;
            let consumed=(codec.total_in()-before_in) as usize;let produced=(codec.total_out()-before_out) as usize;offset+=consumed;
            // An observed early end is a known short file, not an unread tail.
            if status==flate2::Status::StreamEnd{return Err(Error::other("7Z Deflate 文本在所选开头结束，大小与声明不符，请先提取检查"));}
            if consumed==0 && produced==0{return Err(Error::other("7Z Deflate 文本开头前数据不足，请先提取检查"));}
            bytes.extend_from_slice(&output[..produced]);
        }
        check()?;Ok(bytes)
    }

    /// Explicit shared Deflate64 prefix: discard bounded dependencies, retain
    /// only the selected prefix, and make no complete-block/CRC claim.
    #[cfg(feature="additional-codecs")]
    pub fn read_solid_deflate64_entry_prefix(self, selected:usize, max_bytes:usize, max_decoded:u64, mut check:impl FnMut()->Result<(),Error>)->Result<Vec<u8>,Error>{
        let a=self.archive;
        let f=a.folders.get(self.folder_index).ok_or_else(||Error::other("Missing Deflate64 prefix block"))?;
        let files:Vec<_>=a.files.iter().enumerate().filter(|(i,e)|e.has_stream && a.stream_map.file_folder_index.get(*i)==Some(&Some(self.folder_index))).collect();
        let position=files.iter().position(|(i,_)|*i==selected).ok_or_else(||Error::other("Selected prefix is outside shared Deflate64 block"))?;
        let e=files[position].1;
        let total=files.iter().try_fold(0u64,|n,(_,e)|n.checked_add(e.size)).ok_or_else(||Error::other("Shared Deflate64 prefix size overflow"))?;
        let skip=files[..position].iter().try_fold(0u64,|n,(_,e)|n.checked_add(e.size)).ok_or_else(||Error::other("Shared Deflate64 prefix offset overflow"))?;
        if max_bytes==0 || max_bytes>1024*1024 || e.size<=max_bytes as u64 || max_decoded==0 || max_decoded>64*1024*1024
            || files.iter().any(|(_,e)|e.is_directory || e.is_anti_item)
            || !(1..=2).contains(&f.coders.len()) || f.unpack_sizes.len()!=f.coders.len()
            || f.total_input_streams!=f.coders.len() || f.total_output_streams!=f.coders.len()
            || f.bind_pairs.len()!=f.coders.len()-1 || f.packed_streams.len()!=1
            || f.coders.iter().any(|c|c.num_in_streams!=1 || c.num_out_streams!=1)
            || files.len()!=f.num_unpack_sub_streams || f.get_unpack_size()!=total
        {return Err(Error::unsupported("7Z Deflate64 固实开头预览的共享映射或大小无法确认，请先提取后打开"));}
        let target=skip.checked_add(max_bytes as u64).filter(|n|*n<=max_decoded).ok_or_else(||Error::unsupported("7Z Deflate64 固实预览前置内容与开头合计超过 64 MiB，请先提取后打开"))?;
        let coders:Vec<_>=f.ordered_coder_iter().take(f.coders.len()+1).collect();
        if coders.len()!=f.coders.len() || !(coders.len()==1 && coders[0].1.decompression_method_id()==SevenZMethod::ID_DEFLATE64
            || coders.len()==2 && coders[0].1.decompression_method_id()==SevenZMethod::ID_AES256SHA256 && coders[1].1.decompression_method_id()==SevenZMethod::ID_DEFLATE64)
            || !coders.last().unwrap().1.properties.is_empty()
        {return Err(Error::unsupported("7Z Deflate64 大文本预览方法或顺序无法确认，请先提取后打开"));}
        let pack=*a.stream_map.folder_first_pack_stream_index.get(self.folder_index).ok_or_else(||Error::other("Missing Deflate64 prefix pack mapping"))?;
        let size=*a.pack_sizes.get(pack).ok_or_else(||Error::other("Missing Deflate64 prefix pack size"))?;
        let encrypted=coders.len()==2;
        let plain=if encrypted{f.get_unpack_size_at_index(coders[0].0)}else{size};
        if plain==0{return Err(Error::other("Deflate64 prefix has no compressed plaintext"));}
        if encrypted && plain.checked_add(15).map(|n|n/16*16)!=Some(size){return Err(Error::other("Deflate64 prefix AES packed size mismatch"));}
        let start=32u64.checked_add(a.pack_pos).and_then(|n|a.stream_map.pack_stream_offsets.get(pack).and_then(|o|n.checked_add(*o))).ok_or_else(||Error::other("Deflate64 prefix offset overflow"))?;
        check()?;self.source.seek(SeekFrom::Start(start)).map_err(Error::io)?;
        let packed=self.source.take(size);
        let mut stream:Box<dyn Read+'_>=if encrypted{
            #[cfg(feature="aes256")]
            {crate::preview::coder(coders[0].1,0)?;let n=usize::try_from(plain).map_err(|_|Error::unsupported("Deflate64 prefix size exceeds platform range"))?;Box::new(crate::aes256sha256::PreviewAes::new_prefix(packed,&coders[0].1.properties,self.password,n)?)}
            #[cfg(not(feature="aes256"))]
            {return Err(Error::unsupported("AES is unavailable"));}
        }else{Box::new(packed)};
        let mut codec=Box::new(deflate64::InflaterManaged::new());let mut bytes=Vec::with_capacity(max_bytes);
        let mut input=[0;32*1024];let mut output=[0;32*1024];let(mut offset,mut end,mut read_plain,mut decoded)=(0,0,0u64,0u64);
        while decoded<target{
            check()?;
            // Drain buffered output before pulling more compressed input.
            if offset==end && codec.available_output()==0{
                offset=0;end=0;
                while end<input.len() && read_plain<plain{
                    check()?;let capacity=((input.len()-end) as u64).min(plain-read_plain) as usize;
                    let n=stream.read(&mut input[end..end+capacity]).map_err(Error::io)?;
                    if n==0{return Err(Error::other("7Z Deflate64 文本开头前数据结束，大小与声明不符，请重新下载或先提取检查"));}
                    end+=n;read_plain+=n as u64;
                }
            }
            let before_out=decoded;let capacity=(output.len() as u64).min(target-decoded) as usize;
            let result=codec.inflate(&input[offset..end],&mut output[..capacity]);
            if result.data_error || codec.errored(){return Err(Error::other("Deflate64 shared prefix invalid stream"));}
            let produced=result.bytes_written;offset+=result.bytes_consumed;decoded+=produced as u64;
            // A known terminal includes pending output, even when we stop at
            // the selected prefix; it cannot hide a conflicting shared size.
            if codec.input_finished() && decoded+codec.available_output() as u64!=total{return Err(Error::other("7Z Deflate64 共享内容已提前结束，大小与声明不符，请先提取检查"));}
            if result.bytes_consumed==0 && produced==0{return Err(Error::other("7Z Deflate64 文本开头前数据不足，请先提取检查"));}
            // Discard dependencies immediately, including a chunk crossing the
            // selected boundary; only the selected prefix enters the buffer.
            if decoded>skip {let first=skip.saturating_sub(before_out) as usize;bytes.extend_from_slice(&output[first..produced]);}
        }
        if bytes.len()!=max_bytes{return Err(Error::other("Shared Deflate64 prefix size mismatch"));}
        check()?;Ok(bytes)
    }

    /// Explicit single/shared Bzip2 prefix: discard bounded dependencies, retain
    /// only the selected prefix, and make no complete-block/CRC claim.
    #[cfg(feature="bzip2")]
    pub fn read_bzip2_entry_prefix(self, selected:usize, max_bytes:usize, max_decoded:u64, mut check:impl FnMut()->Result<(),Error>)->Result<Vec<u8>,Error>{
        let a=self.archive;
        let f=a.folders.get(self.folder_index).ok_or_else(||Error::other("Missing Bzip2 prefix block"))?;
        let files:Vec<_>=a.files.iter().enumerate().filter(|(i,e)|e.has_stream && a.stream_map.file_folder_index.get(*i)==Some(&Some(self.folder_index))).collect();
        let position=files.iter().position(|(i,_)|*i==selected).ok_or_else(||Error::other("Selected prefix is outside shared Bzip2 block"))?;
        let e=files[position].1;
        let total=files.iter().try_fold(0u64,|n,(_,e)|n.checked_add(e.size)).ok_or_else(||Error::other("Shared Bzip2 prefix size overflow"))?;
        let skip=files[..position].iter().try_fold(0u64,|n,(_,e)|n.checked_add(e.size)).ok_or_else(||Error::other("Shared Bzip2 prefix offset overflow"))?;
        if max_bytes==0 || max_bytes>1024*1024 || e.size<=max_bytes as u64 || max_decoded==0 || max_decoded>64*1024*1024
            || files.iter().any(|(_,e)|e.is_directory || e.is_anti_item)
            || !(1..=2).contains(&f.coders.len()) || f.unpack_sizes.len()!=f.coders.len()
            || f.total_input_streams!=f.coders.len() || f.total_output_streams!=f.coders.len()
            || f.bind_pairs.len()!=f.coders.len()-1 || f.packed_streams.len()!=1
            || f.coders.iter().any(|c|c.num_in_streams!=1 || c.num_out_streams!=1)
            || files.len()!=f.num_unpack_sub_streams || f.get_unpack_size()!=total
        {return Err(Error::unsupported("7Z Bzip2 固实开头预览的共享映射或大小无法确认，请先提取后打开"));}
        let target=skip.checked_add(max_bytes as u64).filter(|n|*n<=max_decoded).ok_or_else(||Error::unsupported("7Z Bzip2 固实预览前置内容与开头合计超过 64 MiB，请先提取后打开"))?;
        let coders:Vec<_>=f.ordered_coder_iter().take(f.coders.len()+1).collect();
        if coders.len()!=f.coders.len() || !(coders.len()==1 && coders[0].1.decompression_method_id()==SevenZMethod::ID_BZIP2
            || coders.len()==2 && coders[0].1.decompression_method_id()==SevenZMethod::ID_AES256SHA256 && coders[1].1.decompression_method_id()==SevenZMethod::ID_BZIP2)
            || !coders.last().unwrap().1.properties.is_empty()
        {return Err(Error::unsupported("7Z Bzip2 大文本预览方法或顺序无法确认，请先提取后打开"));}
        let pack=*a.stream_map.folder_first_pack_stream_index.get(self.folder_index).ok_or_else(||Error::other("Missing Bzip2 prefix pack mapping"))?;
        let size=*a.pack_sizes.get(pack).ok_or_else(||Error::other("Missing Bzip2 prefix pack size"))?;
        let encrypted=coders.len()==2;
        let plain=if encrypted{f.get_unpack_size_at_index(coders[0].0)}else{size};
        if plain==0{return Err(Error::other("Bzip2 prefix has no compressed plaintext"));}
        if encrypted && plain.checked_add(15).map(|n|n/16*16)!=Some(size){return Err(Error::other("Bzip2 prefix AES packed size mismatch"));}
        let start=32u64.checked_add(a.pack_pos).and_then(|n|a.stream_map.pack_stream_offsets.get(pack).and_then(|o|n.checked_add(*o))).ok_or_else(||Error::other("Bzip2 prefix offset overflow"))?;
        check()?;self.source.seek(SeekFrom::Start(start)).map_err(Error::io)?;
        let packed=self.source.take(size);
        let mut stream:Box<dyn Read+'_>=if encrypted{
            #[cfg(feature="aes256")]
            {crate::preview::coder(coders[0].1,0)?;let n=usize::try_from(plain).map_err(|_|Error::unsupported("Bzip2 prefix size exceeds platform range"))?;Box::new(crate::aes256sha256::PreviewAes::new_prefix(packed,&coders[0].1.properties,self.password,n)?)}
            #[cfg(not(feature="aes256"))]
            {return Err(Error::unsupported("AES is unavailable"));}
        }else{Box::new(packed)};
        let mut codec=bzip2::Decompress::new(false);let mut bytes=Vec::with_capacity(max_bytes);
        let mut input=[0;32*1024];let mut output=[0;32*1024];let(mut offset,mut end,mut read_plain)=(0,0,0u64);
        while codec.total_out()<target{
            check()?;
            let before_in=codec.total_in();let before_out=codec.total_out();let capacity=(output.len() as u64).min(target-before_out) as usize;
            let status=codec.decompress(&input[offset..end],&mut output[..capacity]).map_err(|e|Error::other(format!("Bzip2 prefix: {e}")))?;
            let consumed=(codec.total_in()-before_in) as usize;let produced=(codec.total_out()-before_out) as usize;offset+=consumed;
            // An observed early end is a known short file, not an unread tail.
            if status==bzip2::Status::StreamEnd{return Err(Error::other("7Z Bzip2 文本在所选开头结束，大小与声明不符，请先提取检查"));}
            if status==bzip2::Status::MemNeeded{return Err(Error::other("Bzip2 prefix decoder memory unavailable"));}
            // Try buffered output before asking for more input. Short reads
            // are valid; never fill a speculative chunk past a usable prefix.
            if consumed==0 && produced==0{
                if offset!=end || read_plain>=plain{return Err(Error::other("7Z Bzip2 文本开头前数据不足，请先提取检查"));}
                check()?;let capacity=(input.len() as u64).min(plain-read_plain) as usize;
                offset=0;end=stream.read(&mut input[..capacity]).map_err(Error::io)?;
                if end==0{return Err(Error::other("7Z Bzip2 文本开头前数据结束，大小与声明不符，请先提取检查"));}
                read_plain+=end as u64;continue;
            }
            // Discard dependencies immediately, including a chunk crossing the
            // selected boundary; only the selected prefix enters the buffer.
            if codec.total_out()>skip {let first=skip.saturating_sub(before_out) as usize;bytes.extend_from_slice(&output[first..produced]);}
        }
        if bytes.len()!=max_bytes{return Err(Error::other("Shared Bzip2 prefix size mismatch"));}
        check()?;Ok(bytes)
    }

    /// Explicit shared Deflate prefix: discard bounded dependencies, retain
    /// only the selected prefix, and make no complete-block/CRC claim.
    #[cfg(feature="additional-codecs")]
    pub fn read_solid_deflate_entry_prefix(self, selected:usize, max_bytes:usize, max_decoded:u64, mut check:impl FnMut()->Result<(),Error>)->Result<Vec<u8>,Error>{
        let a=self.archive;
        let f=a.folders.get(self.folder_index).ok_or_else(||Error::other("Missing Deflate prefix block"))?;
        let files:Vec<_>=a.files.iter().enumerate().filter(|(i,e)|e.has_stream && a.stream_map.file_folder_index.get(*i)==Some(&Some(self.folder_index))).collect();
        let position=files.iter().position(|(i,_)|*i==selected).ok_or_else(||Error::other("Selected prefix is outside shared Deflate block"))?;
        let e=files[position].1;
        let total=files.iter().try_fold(0u64,|n,(_,e)|n.checked_add(e.size)).ok_or_else(||Error::other("Shared Deflate prefix size overflow"))?;
        let skip=files[..position].iter().try_fold(0u64,|n,(_,e)|n.checked_add(e.size)).ok_or_else(||Error::other("Shared Deflate prefix offset overflow"))?;
        if max_bytes==0 || max_bytes>1024*1024 || e.size<=max_bytes as u64 || max_decoded==0 || max_decoded>64*1024*1024
            || files.iter().any(|(_,e)|e.is_directory || e.is_anti_item)
            || !(1..=2).contains(&f.coders.len()) || f.unpack_sizes.len()!=f.coders.len()
            || f.total_input_streams!=f.coders.len() || f.total_output_streams!=f.coders.len()
            || f.bind_pairs.len()!=f.coders.len()-1 || f.packed_streams.len()!=1
            || f.coders.iter().any(|c|c.num_in_streams!=1 || c.num_out_streams!=1)
            || files.len()!=f.num_unpack_sub_streams || f.get_unpack_size()!=total
        {return Err(Error::unsupported("7Z Deflate 固实开头预览的共享映射或大小无法确认，请先提取后打开"));}
        let target=skip.checked_add(max_bytes as u64).filter(|n|*n<=max_decoded).ok_or_else(||Error::unsupported("7Z Deflate 固实预览前置内容与开头合计超过 64 MiB，请先提取后打开"))?;
        let coders:Vec<_>=f.ordered_coder_iter().take(f.coders.len()+1).collect();
        if coders.len()!=f.coders.len() || !(coders.len()==1 && coders[0].1.decompression_method_id()==SevenZMethod::ID_DEFLATE
            || coders.len()==2 && coders[0].1.decompression_method_id()==SevenZMethod::ID_AES256SHA256 && coders[1].1.decompression_method_id()==SevenZMethod::ID_DEFLATE)
            || !coders.last().unwrap().1.properties.is_empty()
        {return Err(Error::unsupported("7Z Deflate 大文本预览方法或顺序无法确认，请先提取后打开"));}
        let pack=*a.stream_map.folder_first_pack_stream_index.get(self.folder_index).ok_or_else(||Error::other("Missing Deflate prefix pack mapping"))?;
        let size=*a.pack_sizes.get(pack).ok_or_else(||Error::other("Missing Deflate prefix pack size"))?;
        let encrypted=coders.len()==2;
        let plain=if encrypted{f.get_unpack_size_at_index(coders[0].0)}else{size};
        if plain==0{return Err(Error::other("Deflate prefix has no compressed plaintext"));}
        if encrypted && plain.checked_add(15).map(|n|n/16*16)!=Some(size){return Err(Error::other("Deflate prefix AES packed size mismatch"));}
        let start=32u64.checked_add(a.pack_pos).and_then(|n|a.stream_map.pack_stream_offsets.get(pack).and_then(|o|n.checked_add(*o))).ok_or_else(||Error::other("Deflate prefix offset overflow"))?;
        check()?;self.source.seek(SeekFrom::Start(start)).map_err(Error::io)?;
        let packed=self.source.take(size);
        let mut stream:Box<dyn Read+'_>=if encrypted{
            #[cfg(feature="aes256")]
            {crate::preview::coder(coders[0].1,0)?;let n=usize::try_from(plain).map_err(|_|Error::unsupported("Deflate prefix size exceeds platform range"))?;Box::new(crate::aes256sha256::PreviewAes::new_prefix(packed,&coders[0].1.properties,self.password,n)?)}
            #[cfg(not(feature="aes256"))]
            {return Err(Error::unsupported("AES is unavailable"));}
        }else{Box::new(packed)};
        let mut codec=flate2::Decompress::new(false);let mut bytes=Vec::with_capacity(max_bytes);
        let mut input=[0;32*1024];let mut output=[0;32*1024];let(mut offset,mut end,mut read_plain)=(0,0,0u64);
        while codec.total_out()<target{
            check()?;
            if offset==end{
                offset=0;end=0;
                while end<input.len() && read_plain<plain{
                    check()?;let capacity=((input.len()-end) as u64).min(plain-read_plain) as usize;
                    let n=stream.read(&mut input[end..end+capacity]).map_err(Error::io)?;
                    if n==0{return Err(Error::other("7Z Deflate 文本开头前数据结束，大小与声明不符，请重新下载或先提取检查"));}
                    end+=n;read_plain+=n as u64;
                }
            }
            let before_in=codec.total_in();let before_out=codec.total_out();let capacity=(output.len() as u64).min(target-before_out) as usize;
            let status=codec.decompress(&input[offset..end],&mut output[..capacity],flate2::FlushDecompress::None).map_err(|e|Error::other(format!("Deflate prefix: {e}")))?;
            let consumed=(codec.total_in()-before_in) as usize;let produced=(codec.total_out()-before_out) as usize;offset+=consumed;
            // An observed early end is a known short file, not an unread tail.
            if status==flate2::Status::StreamEnd{return Err(Error::other("7Z Deflate 文本在所选开头结束，大小与声明不符，请先提取检查"));}
            if consumed==0 && produced==0{return Err(Error::other("7Z Deflate 文本开头前数据不足，请先提取检查"));}
            // Discard dependencies immediately, including a chunk crossing the
            // selected boundary; only the selected prefix enters the buffer.
            if codec.total_out()>skip {let first=skip.saturating_sub(before_out) as usize;bytes.extend_from_slice(&output[first..produced]);}
        }
        if bytes.len()!=max_bytes{return Err(Error::other("Shared Deflate prefix size mismatch"));}
        check()?;Ok(bytes)
    }

    /// Complete Bzip2 (single or shared), optionally preceded by full AES.
    /// Keep only the selected bytes; require the real single-stream terminal,
    /// precise input/output coverage, all available CRCs and full AES receipts.
    #[cfg(feature="bzip2")]
    pub fn read_bzip2_entry(self, selected: usize, max_bytes: usize, max_shared: u64, mut check: impl FnMut() -> Result<(), Error>) -> Result<Vec<u8>, Error> {
        let a=self.archive;
        let f=a.folders.get(self.folder_index).ok_or_else(||Error::other("Missing Bzip2 block"))?;
        let files:Vec<_>=a.files.iter().enumerate().filter(|(i,e)|e.has_stream && a.stream_map.file_folder_index.get(*i)==Some(&Some(self.folder_index))).collect();
        let e=files.iter().find(|(i,_)|*i==selected).ok_or_else(||Error::other("Selected file is outside shared Bzip2 block"))?.1;
        let total=files.iter().try_fold(0u64,|n,(_,e)|n.checked_add(e.size)).ok_or_else(||Error::other("Shared Bzip2 size overflow"))?;
        if max_bytes==0 || max_bytes>8*1024*1024 || e.size>max_bytes as u64 || max_shared==0 || max_shared>64*1024*1024 || total>max_shared
            || !(1..=2).contains(&f.coders.len()) || f.unpack_sizes.len()!=f.coders.len()
            || f.total_input_streams!=f.coders.len() || f.total_output_streams!=f.coders.len()
            || f.bind_pairs.len()!=f.coders.len()-1 || f.packed_streams.len()!=1
            || f.coders.iter().any(|c|c.num_in_streams!=1 || c.num_out_streams!=1)
            || files.len()!=f.num_unpack_sub_streams || f.get_unpack_size()!=total
            || files.iter().any(|(_,e)|e.is_directory || e.is_anti_item || (!e.has_crc && !f.has_crc))
        {return Err(Error::unsupported("7Z Bzip2 固实共享映射或大小无法确认（共享内容须 ≤64 MiB），请先提取后打开"));}
        let coders:Vec<_>=f.ordered_coder_iter().take(f.coders.len()+1).collect();
        if coders.len()!=f.coders.len() || !(coders.len()==1 && coders[0].1.decompression_method_id()==SevenZMethod::ID_BZIP2
            || coders.len()==2 && coders[0].1.decompression_method_id()==SevenZMethod::ID_AES256SHA256 && coders[1].1.decompression_method_id()==SevenZMethod::ID_BZIP2)
            || !coders.last().unwrap().1.properties.is_empty()
        {return Err(Error::unsupported("7Z Bzip2 预览方法或顺序无法确认，请先提取后打开"));}
        let pack=*a.stream_map.folder_first_pack_stream_index.get(self.folder_index).ok_or_else(||Error::other("Missing Bzip2 pack mapping"))?;
        let size=*a.pack_sizes.get(pack).ok_or_else(||Error::other("Missing Bzip2 pack size"))?;
        if size>max_shared {return Err(Error::unsupported("7Z Bzip2 压缩内容超过 64 MiB，请先提取后打开"));}
        let encrypted=coders.len()==2;
        let plain=if encrypted {f.get_unpack_size_at_index(coders[0].0)} else {size};
        if plain==0 || plain>max_shared {return Err(Error::unsupported("7Z Bzip2 预览输入长度超出范围，请先提取后打开"));}
        if encrypted && plain.checked_add(15).map(|n|n/16*16)!=Some(size) {return Err(Error::other("Bzip2 AES packed size mismatch"));}
        let start=32u64.checked_add(a.pack_pos).and_then(|n|a.stream_map.pack_stream_offsets.get(pack).and_then(|o|n.checked_add(*o))).ok_or_else(||Error::other("Bzip2 pack offset overflow"))?;
        check()?;self.source.seek(SeekFrom::Start(start)).map_err(Error::io)?;
        let mut packed=PreviewPack{source:self.source,start,size,read:0,crc:CRC32.digest()};
        let mut stream:Box<dyn Read+'_>=if encrypted {
            #[cfg(feature="aes256")]
            {crate::preview::coder(coders[0].1,plain as usize)?;Box::new(crate::aes256sha256::PreviewAes::new(&mut packed,&coders[0].1.properties,self.password,plain as usize)?)}
            #[cfg(not(feature="aes256"))]
            {return Err(Error::unsupported("AES is unavailable"));}
        } else {Box::new(&mut packed)};
        let mut codec=bzip2::Decompress::new(false);
        let mut input=[0u8;32*1024];let mut output=[0u8;32*1024];let(mut offset,mut end)=(0,0);
        let(mut consumed,mut decoded,mut read_plain)=(0u64,0u64,0u64);let mut bytes=Vec::with_capacity(e.size as usize);
        let mut block_crc=CRC32.digest();let mut file_crcs:Vec<_>=files.iter().map(|_|CRC32.digest()).collect();
        let(mut file_pos,mut in_file)=(0,0u64);
        loop {
            check()?;
            if offset==end {
                offset=0;end=0;
                while end<input.len() && read_plain<plain {
                    check()?;let capacity=(input.len()-end).min((plain-read_plain) as usize);let n=stream.read(&mut input[end..end+capacity]).map_err(Error::io)?;
                    if n==0 {return Err(Error::other("Bzip2 packed stream ended early"));}end+=n;read_plain+=n as u64;
                }
            }
            let before_in=codec.total_in();let before_out=codec.total_out();
            let status=codec.decompress(&input[offset..end],&mut output).map_err(|e|Error::other(format!("Bzip2 invalid stream: {e}")))?;
            if status==bzip2::Status::MemNeeded {return Err(Error::other("Bzip2 decoder memory unavailable"));}
            let used=(codec.total_in()-before_in) as usize;let produced=(codec.total_out()-before_out) as usize;
            offset+=used;consumed+=used as u64;decoded+=produced as u64;
            if decoded>total {return Err(Error::other("Bzip2 output exceeds declared shared size"));}
            block_crc.update(&output[..produced]);let mut position=0;
            while position<produced {
                check()?;
                while file_pos<files.len() && in_file==files[file_pos].1.size {file_pos+=1;in_file=0;}
                if file_pos==files.len() {return Err(Error::other("Shared Bzip2 output exceeds mapped files"));}
                let n=(files[file_pos].1.size-in_file).min((produced-position) as u64) as usize;
                let chunk=&output[position..position+n];file_crcs[file_pos].update(chunk);
                if files[file_pos].0==selected {bytes.extend_from_slice(chunk);}
                in_file+=n as u64;position+=n;
            }
            if status==bzip2::Status::StreamEnd {
                if codec.total_in()!=plain || consumed!=plain || read_plain!=plain || decoded!=total {return Err(Error::other("Bzip2 end or compressed coverage mismatch"));}break;
            }
            if used==0 && produced==0 {return Err(Error::other("Bzip2 stream did not finish"));}
        }
        // A complete raw terminal cannot waive full AES EOF/padding or receipts.
        let mut extra=[0];if stream.read(&mut extra).map_err(Error::io)?!=0 {return Err(Error::other("Bzip2 extra plaintext"));}
        drop(stream);
        if packed.read!=size {return Err(Error::other("Bzip2 packed stream was not completely consumed"));}
        for ((_,file),crc) in files.iter().zip(file_crcs) {
            if file.has_crc && file.crc!=crc.finalize() as u64 {return Err(Error::ChecksumVerificationFailed);}
        }
        if bytes.len() as u64!=e.size {return Err(Error::other("Shared Bzip2 selected output size mismatch"));}
        if f.has_crc && f.crc!=block_crc.finalize() as u64 || a.pack_crcs_defined.contains(pack) && a.pack_crcs.get(pack).copied()!=Some(packed.crc.finalize() as u64) {return Err(Error::ChecksumVerificationFailed);}
        check()?;Ok(bytes)
    }

    /// Complete shared Deflate64, optionally preceded by full AES: stream dependencies,
    /// retain only the selected file, and verify every available CRC plus the
    /// real terminal extent, excluding whole Huffman-prefetched bytes.
    #[cfg(feature="additional-codecs")]
    pub fn read_solid_deflate64_entry(self, selected: usize, max_bytes: usize, max_shared: u64, mut check: impl FnMut() -> Result<(), Error>) -> Result<Vec<u8>, Error> {
        let a=self.archive;
        let f=a.folders.get(self.folder_index).ok_or_else(||Error::other("Missing Deflate64 block"))?;
        let files:Vec<_>=a.files.iter().enumerate().filter(|(i,e)|e.has_stream && a.stream_map.file_folder_index.get(*i)==Some(&Some(self.folder_index))).collect();
        let e=files.iter().find(|(i,_)|*i==selected).ok_or_else(||Error::other("Selected file is outside shared Deflate64 block"))?.1;
        let total=files.iter().try_fold(0u64,|n,(_,e)|n.checked_add(e.size)).ok_or_else(||Error::other("Shared Deflate64 size overflow"))?;
        if max_bytes==0 || max_bytes>8*1024*1024 || e.size>max_bytes as u64 || max_shared==0 || max_shared>64*1024*1024 || total>max_shared
            || !(1..=2).contains(&f.coders.len()) || f.unpack_sizes.len()!=f.coders.len()
            || f.total_input_streams!=f.coders.len() || f.total_output_streams!=f.coders.len()
            || f.bind_pairs.len()!=f.coders.len()-1 || f.packed_streams.len()!=1
            || f.coders.iter().any(|c|c.num_in_streams!=1 || c.num_out_streams!=1)
            || files.len()!=f.num_unpack_sub_streams || f.get_unpack_size()!=total
            || files.iter().any(|(_,e)|e.is_directory || e.is_anti_item || (!e.has_crc && !f.has_crc))
        {return Err(Error::unsupported("7Z Deflate64 固实共享映射或大小无法确认（共享内容须 ≤64 MiB），请先提取后打开"));}
        let coders:Vec<_>=f.ordered_coder_iter().take(f.coders.len()+1).collect();
        if coders.len()!=f.coders.len() || !(coders.len()==1 && coders[0].1.decompression_method_id()==SevenZMethod::ID_DEFLATE64
            || coders.len()==2 && coders[0].1.decompression_method_id()==SevenZMethod::ID_AES256SHA256 && coders[1].1.decompression_method_id()==SevenZMethod::ID_DEFLATE64)
            || !coders.last().unwrap().1.properties.is_empty()
        {return Err(Error::unsupported("7Z Deflate64 预览方法或顺序无法确认，请先提取后打开"));}
        let pack=*a.stream_map.folder_first_pack_stream_index.get(self.folder_index).ok_or_else(||Error::other("Missing Deflate64 pack mapping"))?;
        let size=*a.pack_sizes.get(pack).ok_or_else(||Error::other("Missing Deflate64 pack size"))?;
        if size>max_shared {return Err(Error::unsupported("7Z Deflate64 压缩内容超过 64 MiB，请先提取后打开"));}
        let encrypted=coders.len()==2;
        let plain=if encrypted {f.get_unpack_size_at_index(coders[0].0)} else {size};
        if plain==0 || plain>max_shared {return Err(Error::unsupported("7Z Deflate64 预览输入长度超出范围，请先提取后打开"));}
        if encrypted && plain.checked_add(15).map(|n|n/16*16)!=Some(size) {return Err(Error::other("Deflate64 AES packed size mismatch"));}
        let start=32u64.checked_add(a.pack_pos).and_then(|n|a.stream_map.pack_stream_offsets.get(pack).and_then(|o|n.checked_add(*o))).ok_or_else(||Error::other("Deflate64 pack offset overflow"))?;
        check()?;self.source.seek(SeekFrom::Start(start)).map_err(Error::io)?;
        let mut packed=PreviewPack{source:self.source,start,size,read:0,crc:CRC32.digest()};
        let mut stream:Box<dyn Read+'_>=if encrypted {
            #[cfg(feature="aes256")]
            {crate::preview::coder(coders[0].1,plain as usize)?;Box::new(crate::aes256sha256::PreviewAes::new(&mut packed,&coders[0].1.properties,self.password,plain as usize)?)}
            #[cfg(not(feature="aes256"))]
            {return Err(Error::unsupported("AES is unavailable"));}
        } else {Box::new(&mut packed)};
        let mut codec=Box::new(deflate64::InflaterManaged::new());
        let mut input=[0u8;32*1024];let mut output=[0u8;32*1024];let(mut offset,mut end)=(0,0);
        let(mut consumed,mut decoded,mut read_plain)=(0u64,0u64,0u64);let mut bytes=Vec::with_capacity(e.size as usize);
        let mut block_crc=CRC32.digest();let mut file_crcs:Vec<_>=files.iter().map(|_|CRC32.digest()).collect();
        let(mut file_pos,mut in_file)=(0,0u64);
        loop {
            check()?;
            if offset==end {
                offset=0;end=0;
                while end<input.len() && read_plain<plain {
                    check()?;let capacity=(input.len()-end).min((plain-read_plain) as usize);let n=stream.read(&mut input[end..end+capacity]).map_err(Error::io)?;
                    if n==0 {return Err(Error::other("Deflate64 packed stream ended early"));}end+=n;read_plain+=n as u64;
                }
            }
            let result=codec.inflate(&input[offset..end],&mut output);
            if result.data_error || codec.errored() {return Err(Error::other("Deflate64 invalid stream"));}
            offset+=result.bytes_consumed;consumed+=result.bytes_consumed as u64;decoded+=result.bytes_written as u64;
            if decoded>total {return Err(Error::other("Deflate64 output exceeds declared shared size"));}
            let produced=result.bytes_written;block_crc.update(&output[..produced]);let mut position=0;
            while position<produced {
                check()?;
                while file_pos<files.len() && in_file==files[file_pos].1.size {file_pos+=1;in_file=0;}
                if file_pos==files.len() {return Err(Error::other("Shared Deflate64 output exceeds mapped files"));}
                let n=(files[file_pos].1.size-in_file).min((produced-position) as u64) as usize;
                let chunk=&output[position..position+n];file_crcs[file_pos].update(chunk);
                if files[file_pos].0==selected {bytes.extend_from_slice(chunk);}
                in_file+=n as u64;position+=n;
            }
            if codec.finished() {
                if codec.finished_input_bytes()!=Some(plain) || consumed!=plain || read_plain!=plain || decoded!=total {return Err(Error::other("Deflate64 end or compressed coverage mismatch"));}break;
            }
            if result.bytes_consumed==0 && produced==0 {return Err(Error::other("Deflate64 stream did not finish"));}
        }
        // A complete raw terminal cannot waive full AES EOF/padding or receipts.
        let mut extra=[0];if stream.read(&mut extra).map_err(Error::io)?!=0 {return Err(Error::other("Deflate64 extra plaintext"));}
        drop(stream);
        if packed.read!=size {return Err(Error::other("Deflate64 packed stream was not completely consumed"));}
        for ((_,file),crc) in files.iter().zip(file_crcs) {
            if file.has_crc && file.crc!=crc.finalize() as u64 {return Err(Error::ChecksumVerificationFailed);}
        }
        if bytes.len() as u64!=e.size {return Err(Error::other("Shared Deflate64 selected output size mismatch"));}
        if f.has_crc && f.crc!=block_crc.finalize() as u64 || a.pack_crcs_defined.contains(pack) && a.pack_crcs.get(pack).copied()!=Some(packed.crc.finalize() as u64) {return Err(Error::ChecksumVerificationFailed);}
        check()?;Ok(bytes)
    }

    /// Complete non-solid Deflate64, optionally preceded by full AES. Observe
    /// BFINAL/EOB, drain output, exclude Huffman-prefetched whole bytes and
    /// complete AES EOF/padding; declared output must never stand in for finish.
    #[cfg(feature="additional-codecs")]
    pub fn read_deflate64_entry(self, selected: usize, max_bytes: usize, mut check: impl FnMut() -> Result<(), Error>) -> Result<Vec<u8>, Error> {
        let a=self.archive;
        let f=a.folders.get(self.folder_index).ok_or_else(||Error::other("Missing Deflate64 block"))?;
        let e=a.files.get(selected).ok_or_else(||Error::other("Missing Deflate64 file"))?;
        let mapped=a.files.iter().enumerate().filter(|(i,e)|e.has_stream && a.stream_map.file_folder_index.get(*i)==Some(&Some(self.folder_index))).count();
        if max_bytes>8*1024*1024 || e.size>max_bytes as u64 || !e.has_stream || e.is_directory || e.is_anti_item
            || a.stream_map.file_folder_index.get(selected)!=Some(&Some(self.folder_index)) || mapped!=1
            || !(1..=2).contains(&f.coders.len()) || f.unpack_sizes.len()!=f.coders.len()
            || f.total_input_streams!=f.coders.len() || f.total_output_streams!=f.coders.len()
            || f.bind_pairs.len()!=f.coders.len()-1 || f.packed_streams.len()!=1
            || f.coders.iter().any(|c|c.num_in_streams!=1 || c.num_out_streams!=1)
            || f.num_unpack_sub_streams!=1 || f.get_unpack_size()!=e.size || !(e.has_crc || f.has_crc)
        {return Err(Error::unsupported("7Z Deflate64 预览映射或方法无法确认，请先提取后打开"));}
        let coders:Vec<_>=f.ordered_coder_iter().take(f.coders.len()+1).collect();
        if coders.len()!=f.coders.len() || !(coders.len()==1 && coders[0].1.decompression_method_id()==SevenZMethod::ID_DEFLATE64
            || coders.len()==2 && coders[0].1.decompression_method_id()==SevenZMethod::ID_AES256SHA256 && coders[1].1.decompression_method_id()==SevenZMethod::ID_DEFLATE64)
            || !coders.last().unwrap().1.properties.is_empty()
        {return Err(Error::unsupported("7Z Deflate64 预览方法或顺序无法确认，请先提取后打开"));}
        let pack=*a.stream_map.folder_first_pack_stream_index.get(self.folder_index).ok_or_else(||Error::other("Missing Deflate64 pack mapping"))?;
        let size=*a.pack_sizes.get(pack).ok_or_else(||Error::other("Missing Deflate64 pack size"))?;
        if size>64*1024*1024 {return Err(Error::unsupported("7Z Deflate64 压缩内容超过 64 MiB，请先提取后打开"));}
        let encrypted=coders.len()==2;
        let plain=if encrypted {f.get_unpack_size_at_index(coders[0].0)} else {size};
        if plain==0 || plain>64*1024*1024 {return Err(Error::unsupported("7Z Deflate64 预览输入长度超出范围，请先提取后打开"));}
        if encrypted && plain.checked_add(15).map(|n|n/16*16)!=Some(size) {return Err(Error::other("Deflate64 AES packed size mismatch"));}
        let start=32u64.checked_add(a.pack_pos).and_then(|n|a.stream_map.pack_stream_offsets.get(pack).and_then(|o|n.checked_add(*o))).ok_or_else(||Error::other("Deflate64 pack offset overflow"))?;
        check()?;self.source.seek(SeekFrom::Start(start)).map_err(Error::io)?;
        let mut packed=PreviewPack{source:self.source,start,size,read:0,crc:CRC32.digest()};
        let mut stream:Box<dyn Read+'_>=if encrypted {
            #[cfg(feature="aes256")]
            {crate::preview::coder(coders[0].1,plain as usize)?;Box::new(crate::aes256sha256::PreviewAes::new(&mut packed,&coders[0].1.properties,self.password,plain as usize)?)}
            #[cfg(not(feature="aes256"))]
            {return Err(Error::unsupported("AES is unavailable"));}
        } else {Box::new(&mut packed)};
        let mut codec=Box::new(deflate64::InflaterManaged::new());
        let mut input=[0u8;32*1024];let mut output=[0u8;32*1024];let(mut offset,mut end)=(0,0);
        let mut read_plain=0u64;let mut consumed=0u64;let mut bytes=Vec::with_capacity(e.size as usize);
        loop {
            check()?;
            if offset==end {
                offset=0;end=0;
                while end<input.len() && read_plain<plain {
                    check()?;let capacity=(input.len()-end).min((plain-read_plain) as usize);let n=stream.read(&mut input[end..end+capacity]).map_err(Error::io)?;
                    if n==0 {return Err(Error::other("Deflate64 packed stream ended early"));}end+=n;read_plain+=n as u64;
                }
            }
            let result=codec.inflate(&input[offset..end],&mut output);
            if result.data_error || codec.errored() {return Err(Error::other("Deflate64 invalid stream"));}
            offset+=result.bytes_consumed;consumed+=result.bytes_consumed as u64;
            if bytes.len()+result.bytes_written>e.size as usize {return Err(Error::other("Deflate64 output exceeds declared size"));}
            bytes.extend_from_slice(&output[..result.bytes_written]);
            if codec.finished() {
                if codec.finished_input_bytes()!=Some(plain) || consumed!=plain || read_plain!=plain || bytes.len() as u64!=e.size {return Err(Error::other("Deflate64 end or compressed coverage mismatch"));}break;
            }
            if result.bytes_consumed==0 && result.bytes_written==0 {return Err(Error::other("Deflate64 stream did not finish"));}
        }
        // A valid raw terminal cannot waive the complete AES EOF/padding receipt.
        let mut extra=[0];if stream.read(&mut extra).map_err(Error::io)?!=0 {return Err(Error::other("Deflate64 extra plaintext"));}
        drop(stream);
        if packed.read!=size {return Err(Error::other("Deflate64 packed stream was not completely consumed"));}
        let crc=CRC32.checksum(&bytes) as u64;
        if e.has_crc && e.crc!=crc || f.has_crc && f.crc!=crc || a.pack_crcs_defined.contains(pack) && a.pack_crcs.get(pack).copied()!=Some(packed.crc.finalize() as u64) {return Err(Error::ChecksumVerificationFailed);}
        check()?;Ok(bytes)
    }

    /// Complete non-solid Deflate preview, optionally preceded by full AES.
    /// Observe raw StreamEnd and exact
    /// compressed consumption rather than treating a Read EOF as stream finish.
    /// Full AES padding/consumption receipts remain mandatory; shared blocks
    /// and other codecs keep their independent paths.
    #[cfg(feature="additional-codecs")]
    pub fn read_deflate_entry(self, selected: usize, max_bytes: usize, mut check: impl FnMut() -> Result<(), Error>) -> Result<Vec<u8>, Error> {
        let a=self.archive;
        let f=a.folders.get(self.folder_index).ok_or_else(||Error::other("Missing Deflate block"))?;
        let e=a.files.get(selected).ok_or_else(||Error::other("Missing Deflate file"))?;
        let mapped=a.files.iter().enumerate().filter(|(i,e)|e.has_stream && a.stream_map.file_folder_index.get(*i)==Some(&Some(self.folder_index))).count();
        if max_bytes>8*1024*1024 || e.size>max_bytes as u64 || !e.has_stream || e.is_directory || e.is_anti_item
            || a.stream_map.file_folder_index.get(selected)!=Some(&Some(self.folder_index)) || mapped!=1
            || !(1..=2).contains(&f.coders.len()) || f.unpack_sizes.len()!=f.coders.len()
            || f.total_input_streams!=f.coders.len() || f.total_output_streams!=f.coders.len()
            || f.bind_pairs.len()!=f.coders.len()-1 || f.packed_streams.len()!=1
            || f.coders.iter().any(|c|c.num_in_streams!=1 || c.num_out_streams!=1)
            || f.num_unpack_sub_streams!=1 || f.get_unpack_size()!=e.size || !(e.has_crc || f.has_crc)
        {return Err(Error::unsupported("7Z Deflate 预览映射或方法无法确认，请先提取后打开"));}
        let coders:Vec<_>=f.ordered_coder_iter().take(f.coders.len()+1).collect();
        if coders.len()!=f.coders.len() || !(coders.len()==1 && coders[0].1.decompression_method_id()==SevenZMethod::ID_DEFLATE
            || coders.len()==2 && coders[0].1.decompression_method_id()==SevenZMethod::ID_AES256SHA256 && coders[1].1.decompression_method_id()==SevenZMethod::ID_DEFLATE)
            || !coders.last().unwrap().1.properties.is_empty()
        {return Err(Error::unsupported("7Z Deflate 预览方法或顺序无法确认，请先提取后打开"));}
        let pack=*a.stream_map.folder_first_pack_stream_index.get(self.folder_index).ok_or_else(||Error::other("Missing Deflate pack mapping"))?;
        let size=*a.pack_sizes.get(pack).ok_or_else(||Error::other("Missing Deflate pack size"))?;
        if size>64*1024*1024 {return Err(Error::unsupported("7Z Deflate 压缩内容超过 64 MiB，请先提取后打开"));}
        let encrypted=coders.len()==2;
        let plain=if encrypted {f.get_unpack_size_at_index(coders[0].0)} else {size};
        if plain==0 || plain>64*1024*1024 {return Err(Error::unsupported("7Z Deflate 预览输入长度超出范围，请先提取后打开"));}
        if encrypted && plain.checked_add(15).map(|n|n/16*16)!=Some(size) {return Err(Error::other("Deflate AES packed size mismatch"));}
        let start=32u64.checked_add(a.pack_pos).and_then(|n|a.stream_map.pack_stream_offsets.get(pack).and_then(|o|n.checked_add(*o))).ok_or_else(||Error::other("Deflate pack offset overflow"))?;
        check()?;self.source.seek(SeekFrom::Start(start)).map_err(Error::io)?;
        let mut packed=PreviewPack{source:self.source,start,size,read:0,crc:CRC32.digest()};
        let mut stream:Box<dyn Read+'_>=if encrypted {
            #[cfg(feature="aes256")]
            {crate::preview::coder(coders[0].1,plain as usize)?;Box::new(crate::aes256sha256::PreviewAes::new(&mut packed,&coders[0].1.properties,self.password,plain as usize)?)}
            #[cfg(not(feature="aes256"))]
            {return Err(Error::unsupported("AES is unavailable"));}
        } else {Box::new(&mut packed)};
        let mut read_plain=0;
        let mut codec=flate2::Decompress::new(false);
        let mut input=[0u8;32*1024];let mut output=[0u8;32*1024];let(mut offset,mut end)=(0,0);
        let mut bytes=Vec::with_capacity(e.size as usize);
        loop {
            check()?;
            if offset==end {
                offset=0;end=0;
                // Fill a bounded chunk even when the underlying source short-reads.
                while end<input.len() && read_plain<plain {
                    check()?;let capacity=(input.len()-end).min((plain-read_plain) as usize);
                    let n=stream.read(&mut input[end..end+capacity]).map_err(Error::io)?;
                    if n==0 {return Err(Error::other("Deflate packed stream ended early"));}end+=n;read_plain+=n as u64;
                }
            }
            let before_in=codec.total_in();let before_out=codec.total_out();
            let status=codec.decompress(&input[offset..end],&mut output,flate2::FlushDecompress::None).map_err(|e|Error::other(format!("Deflate stream: {e}")))?;
            let consumed=(codec.total_in()-before_in) as usize;let produced=(codec.total_out()-before_out) as usize;offset+=consumed;
            if codec.total_out()>e.size {return Err(Error::other("Deflate output exceeds declared size"));}
            bytes.extend_from_slice(&output[..produced]);
            if status==flate2::Status::StreamEnd {
                if codec.total_in()!=plain || read_plain!=plain || codec.total_out()!=e.size {return Err(Error::other("Deflate end or compressed coverage mismatch"));}break;
            }
            if consumed==0 && produced==0 {return Err(Error::other("Deflate stream did not finish"));}
        }
        // Complete the raw/decrypted EOF, including AES zero padding and its
        // registered receipt. A valid Deflate end cannot waive this step.
        let mut extra=[0];if stream.read(&mut extra).map_err(Error::io)?!=0 {return Err(Error::other("Deflate extra plaintext"));}
        drop(stream);
        if packed.read!=size {return Err(Error::other("Deflate packed stream was not completely consumed"));}
        let crc=CRC32.checksum(&bytes) as u64;
        if e.has_crc && e.crc!=crc || f.has_crc && f.crc!=crc || a.pack_crcs_defined.contains(pack) && a.pack_crcs.get(pack).copied()!=Some(packed.crc.finalize() as u64) {return Err(Error::ChecksumVerificationFailed);}
        check()?;Ok(bytes)
    }

    /// Complete a bounded shared Deflate block (optionally full AES), checking
    /// every file and terminal stream while retaining only the selected bytes.
    #[cfg(feature="additional-codecs")]
    pub fn read_solid_deflate_entry(self, selected: usize, max_bytes: usize, max_shared: u64, mut check: impl FnMut() -> Result<(), Error>) -> Result<Vec<u8>, Error> {
        let a=self.archive;
        let f=a.folders.get(self.folder_index).ok_or_else(||Error::other("Missing Deflate block"))?;
        let files:Vec<_>=a.files.iter().enumerate().filter(|(i,e)|e.has_stream && a.stream_map.file_folder_index.get(*i)==Some(&Some(self.folder_index))).collect();
        let e=files.iter().find(|(i,_)|*i==selected).ok_or_else(||Error::other("Selected file is outside shared Deflate block"))?.1;
        let total=files.iter().try_fold(0u64,|n,(_,e)|n.checked_add(e.size)).ok_or_else(||Error::other("Shared Deflate size overflow"))?;
        if max_bytes==0 || max_bytes>8*1024*1024 || e.size>max_bytes as u64 || max_shared==0 || max_shared>64*1024*1024 || total>max_shared
            || !(1..=2).contains(&f.coders.len()) || f.unpack_sizes.len()!=f.coders.len()
            || f.total_input_streams!=f.coders.len() || f.total_output_streams!=f.coders.len()
            || f.bind_pairs.len()!=f.coders.len()-1 || f.packed_streams.len()!=1
            || f.coders.iter().any(|c|c.num_in_streams!=1 || c.num_out_streams!=1)
            || files.len()!=f.num_unpack_sub_streams || f.get_unpack_size()!=total
            || files.iter().any(|(_,e)|e.is_directory || e.is_anti_item || (!e.has_crc && !f.has_crc))
        {return Err(Error::unsupported("7Z Deflate 固实共享映射或大小无法确认（共享内容须 ≤64 MiB），请先提取后打开"));}
        let coders:Vec<_>=f.ordered_coder_iter().take(f.coders.len()+1).collect();
        if coders.len()!=f.coders.len() || !(coders.len()==1 && coders[0].1.decompression_method_id()==SevenZMethod::ID_DEFLATE
            || coders.len()==2 && coders[0].1.decompression_method_id()==SevenZMethod::ID_AES256SHA256 && coders[1].1.decompression_method_id()==SevenZMethod::ID_DEFLATE)
            || !coders.last().unwrap().1.properties.is_empty()
        {return Err(Error::unsupported("7Z Deflate 预览方法或顺序无法确认，请先提取后打开"));}
        let pack=*a.stream_map.folder_first_pack_stream_index.get(self.folder_index).ok_or_else(||Error::other("Missing Deflate pack mapping"))?;
        let size=*a.pack_sizes.get(pack).ok_or_else(||Error::other("Missing Deflate pack size"))?;
        if size>max_shared {return Err(Error::unsupported("7Z Deflate 压缩内容超过 64 MiB，请先提取后打开"));}
        let encrypted=coders.len()==2;
        let plain=if encrypted {f.get_unpack_size_at_index(coders[0].0)} else {size};
        if plain==0 || plain>max_shared {return Err(Error::unsupported("7Z Deflate 预览输入长度超出范围，请先提取后打开"));}
        if encrypted && plain.checked_add(15).map(|n|n/16*16)!=Some(size) {return Err(Error::other("Deflate AES packed size mismatch"));}
        let start=32u64.checked_add(a.pack_pos).and_then(|n|a.stream_map.pack_stream_offsets.get(pack).and_then(|o|n.checked_add(*o))).ok_or_else(||Error::other("Deflate pack offset overflow"))?;
        check()?;self.source.seek(SeekFrom::Start(start)).map_err(Error::io)?;
        let mut packed=PreviewPack{source:self.source,start,size,read:0,crc:CRC32.digest()};
        let mut stream:Box<dyn Read+'_>=if encrypted {
            #[cfg(feature="aes256")]
            {crate::preview::coder(coders[0].1,plain as usize)?;Box::new(crate::aes256sha256::PreviewAes::new(&mut packed,&coders[0].1.properties,self.password,plain as usize)?)}
            #[cfg(not(feature="aes256"))]
            {return Err(Error::unsupported("AES is unavailable"));}
        } else {Box::new(&mut packed)};
        let mut read_plain=0;
        let mut codec=flate2::Decompress::new(false);
        let mut input=[0u8;32*1024];let mut output=[0u8;32*1024];let(mut offset,mut end)=(0,0);
        let mut bytes=Vec::with_capacity(e.size as usize);
        let mut block_crc=CRC32.digest();let mut file_crcs:Vec<_>=files.iter().map(|_|CRC32.digest()).collect();
        let(mut file_pos,mut in_file)=(0,0u64);
        loop {
            check()?;
            if offset==end {
                offset=0;end=0;
                // Fill a bounded chunk even when the underlying source short-reads.
                while end<input.len() && read_plain<plain {
                    check()?;let capacity=(input.len()-end).min((plain-read_plain) as usize);
                    let n=stream.read(&mut input[end..end+capacity]).map_err(Error::io)?;
                    if n==0 {return Err(Error::other("Deflate packed stream ended early"));}end+=n;read_plain+=n as u64;
                }
            }
            let before_in=codec.total_in();let before_out=codec.total_out();
            let status=codec.decompress(&input[offset..end],&mut output,flate2::FlushDecompress::None).map_err(|e|Error::other(format!("Deflate stream: {e}")))?;
            let consumed=(codec.total_in()-before_in) as usize;let produced=(codec.total_out()-before_out) as usize;offset+=consumed;
            if codec.total_out()>total {return Err(Error::other("Deflate output exceeds declared size"));}
            block_crc.update(&output[..produced]);
            let mut position=0;
            while position<produced {
                while file_pos<files.len() && in_file==files[file_pos].1.size {file_pos+=1;in_file=0;}
                if file_pos==files.len() {return Err(Error::other("Shared Deflate output exceeds mapped files"));}
                let n=(files[file_pos].1.size-in_file).min((produced-position) as u64) as usize;
                let chunk=&output[position..position+n];file_crcs[file_pos].update(chunk);
                if files[file_pos].0==selected {bytes.extend_from_slice(chunk);}
                in_file+=n as u64;position+=n;
            }
            if status==flate2::Status::StreamEnd {
                if codec.total_in()!=plain || read_plain!=plain || codec.total_out()!=total {return Err(Error::other("Deflate end or compressed coverage mismatch"));}break;
            }
            if consumed==0 && produced==0 {return Err(Error::other("Deflate stream did not finish"));}
        }
        // Complete the raw/decrypted EOF, including AES zero padding and its
        // registered receipt. A valid Deflate end cannot waive this step.
        let mut extra=[0];if stream.read(&mut extra).map_err(Error::io)?!=0 {return Err(Error::other("Deflate extra plaintext"));}
        drop(stream);
        if packed.read!=size {return Err(Error::other("Deflate packed stream was not completely consumed"));}
        for ((_,file),crc) in files.iter().zip(file_crcs) {
            if file.has_crc && file.crc!=crc.finalize() as u64 {return Err(Error::ChecksumVerificationFailed);}
        }
        if bytes.len() as u64!=e.size {return Err(Error::other("Shared Deflate selected output size mismatch"));}
        if f.has_crc && f.crc!=block_crc.finalize() as u64 || a.pack_crcs_defined.contains(pack) && a.pack_crcs.get(pack).copied()!=Some(packed.crc.finalize() as u64) {return Err(Error::ChecksumVerificationFailed);}
        check()?;Ok(bytes)
    }


    /// Read one file from a complete bounded shared block. Stream and verify
    /// every dependency, retain only the requested file, and read the codec end.
    pub fn read_solid_entry(
        self, selected: usize, max_selected: usize, max_shared: u64,
        mut check: impl FnMut() -> Result<(), Error>,
    ) -> Result<Vec<u8>, Error> {
        let archive = self.archive;
        let folder = archive.folders.get(self.folder_index)
            .ok_or_else(|| Error::other("Missing shared block"))?;
        let files: Vec<_> = archive.files.iter().enumerate().filter(|(i, e)|
            e.has_stream && archive.stream_map.file_folder_index.get(*i) == Some(&Some(self.folder_index))
        ).collect();
        let total = files.iter().try_fold(0u64, |sum, (_, e)| sum.checked_add(e.size))
            .ok_or_else(|| Error::other("Shared block size overflow"))?;
        let entry = files.iter().find(|(i, _)| *i == selected)
            .ok_or_else(|| Error::other("Selected file is outside shared block"))?.1;
        if files.len() != folder.num_unpack_sub_streams || total != folder.get_unpack_size()
            || total > max_shared || entry.size > max_selected as u64
            || files.iter().any(|(_, e)| e.is_directory || e.is_anti_item || (!e.has_crc && !folder.has_crc))
            || folder.packed_streams.len() != 1
        { return Err(Error::unsupported("Unconfirmed or oversized shared block")); }
        let pack_index = *archive.stream_map.folder_first_pack_stream_index.get(self.folder_index)
            .ok_or_else(|| Error::other("Missing packed stream mapping"))?;
        let pack_size = *archive.pack_sizes.get(pack_index).ok_or_else(|| Error::other("Missing packed size"))?;
        if pack_size > max_shared { return Err(Error::unsupported("Packed shared block exceeds limit")); }
        let start = 32u64.checked_add(archive.pack_pos)
            .and_then(|v| archive.stream_map.pack_stream_offsets.get(pack_index).and_then(|n| v.checked_add(*n)))
            .ok_or_else(|| Error::other("Packed stream offset overflow"))?;
        let mut packed = PreviewPack { source: self.source, start, size: pack_size, read: 0, crc: CRC32.digest() };
        let mut metadata = archive.clone();
        metadata.folders[self.folder_index].has_crc = false;
        let (mut decoder, _) = SevenZReader::build_decode_stack(&mut packed, &metadata, self.folder_index, self.password)?;
        let mut output = Vec::with_capacity(entry.size as usize);
        let mut buffer = [0u8; 32 * 1024];
        let mut block_crc = CRC32.digest();
        for (i, file) in files {
            let mut remaining = file.size;
            let mut file_crc = CRC32.digest();
            while remaining > 0 {
                check()?;
                let capacity = remaining.min(buffer.len() as u64) as usize;
                let count = decoder.read(&mut buffer[..capacity]).map_err(Error::io)?;
                if count == 0 { return Err(Error::other("Shared block ended before declared file size")); }
                let chunk = &buffer[..count];
                file_crc.update(chunk); block_crc.update(chunk);
                if i == selected { output.extend_from_slice(chunk); }
                remaining -= count as u64;
            }
            if file.has_crc && file_crc.finalize() as u64 != file.crc {
                return Err(Error::ChecksumVerificationFailed);
            }
        }
        check()?;
        if decoder.read(&mut buffer[..1]).map_err(Error::io)? != 0 {
            return Err(Error::other("Shared block exceeds declared unpacked size"));
        }
        drop(decoder);
        if packed.read != pack_size { return Err(Error::other("Shared packed stream was not completely consumed")); }
        if (folder.has_crc && block_crc.finalize() as u64 != folder.crc)
            || (archive.pack_crcs_defined.contains(pack_index) &&
                archive.pack_crcs.get(pack_index).copied() != Some(packed.crc.finalize() as u64))
        { return Err(Error::ChecksumVerificationFailed); }
        check()?;
        Ok(output)
    }

    /// Complete one non-solid entry without a file-size wrapper masking the
    /// decoder's terminal read. The caller retains only its bounded output.
    pub fn read_single_entry(self, max_bytes: usize) -> Result<Vec<u8>, Error> {
        if self.entry_count() != 1 { return Err(Error::unsupported("Shared block requires extraction")); }
        let entry = self.entries()[0].clone();
        let folder = &self.archive.folders[self.folder_index];
        if entry.size > max_bytes as u64 || folder.get_unpack_size() != entry.size {
            return Err(Error::unsupported("Selected text exceeds output limit"));
        }
        // CRC wrappers stop at the declared size. Read the underlying codec to
        // its own end, then verify both declared sizes and CRCs ourselves.
        let mut metadata = self.archive.clone();
        metadata.folders[self.folder_index].has_crc = false;
        let (decoder, _) = SevenZReader::build_decode_stack(self.source, &metadata, self.folder_index, self.password)?;
        let mut bytes = Vec::with_capacity(entry.size as usize);
        decoder.take(max_bytes as u64 + 1).read_to_end(&mut bytes).map_err(Error::io)?;
        if bytes.len() as u64 != entry.size { return Err(Error::other("Selected text size mismatch")); }
        let crc = CRC32.checksum(&bytes) as u64;
        if (entry.has_crc && crc != entry.crc) || (folder.has_crc && crc != folder.crc) {
            return Err(Error::ChecksumVerificationFailed);
        }
        Ok(bytes)
    }

    pub fn new(
        folder_index: usize,
        archive: &'a Archive,
        password: &'a [u8],
        source: &'a mut R,
    ) -> Self {
        Self {
            folder_index,
            archive,
            password,
            source,
        }
    }

    pub fn entries(&self) -> &[SevenZArchiveEntry] {
        let start = self.archive.stream_map.folder_first_file_index[self.folder_index];
        let file_count = self.archive.folders[self.folder_index].num_unpack_sub_streams;
        &self.archive.files[start..(file_count + start)]
    }

    pub fn entry_count(&self) -> usize {
        self.archive.folders[self.folder_index].num_unpack_sub_streams
    }

    /// Takes a closure to decode each files in this block.
    ///
    /// When decoding files in a block, the data to be decompressed depends on the data in front of it,
    /// you cannot simply skip the previous data and only decompress the data in the back.
    ///
    /// See [ChecksumVerificationFailed](https://github.com/dyz1990/sevenz-rust/issues/31).
    ///
    /// To speed up decompression, you can check this example [examples/forder_dec.rs](https://github.com/dyz1990/sevenz-rust/blob/main/examples/forder_dec.rs).
    /// And this example [mt_decompress.rs](https://github.com/dyz1990/sevenz-rust/blob/main/examples/mt_decompress.rs) if you want use multi-thread.
    ///
    pub fn for_each_entries<F: FnMut(&SevenZArchiveEntry, &mut dyn Read) -> Result<bool, Error>>(
        self,
        each: &mut F,
    ) -> Result<bool, Error> {
        let Self {
            folder_index,
            archive,
            password,
            source,
        } = self;
        let (mut folder_reader, _size) =
            SevenZReader::build_decode_stack(source, archive, folder_index, password)?;
        let start = archive.stream_map.folder_first_file_index[folder_index];
        let file_count = archive.folders[folder_index].num_unpack_sub_streams;

        for file_index in start..(file_count + start) {
            let file = &archive.files[file_index];
            if file.has_stream && file.size > 0 {
                let mut decoder: Box<dyn Read> =
                    Box::new(BoundedReader::new(&mut folder_reader, file.size as usize));
                if file.has_crc {
                    decoder = Box::new(Crc32VerifyingReader::new(
                        decoder,
                        file.size as usize,
                        file.crc,
                    ));
                }
                if !each(file, &mut decoder)
                    .map_err(|e| e.maybe_bad_password(!self.password.is_empty()))?
                {
                    return Ok(false);
                }
            } else {
                let empty_reader: &mut dyn Read = &mut ([0u8; 0].as_slice());
                if !each(file, empty_reader)? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}

// The shared preview stack seeks to its packed stream once, then reads it
// sequentially. Track compressed coverage and CRC without retaining the block.
struct PreviewPack<'a, R> {
    source: &'a mut R, start: u64, size: u64, read: u64,
    crc: crc::Digest<'static, u32>,
}
impl<R: Read> Read for PreviewPack<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let capacity = buffer.len().min(self.size.saturating_sub(self.read) as usize);
        let n = self.source.read(&mut buffer[..capacity])?;
        self.read += n as u64; self.crc.update(&buffer[..n]); Ok(n)
    }
}
impl<R: Seek> Seek for PreviewPack<'_, R> {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        if self.read != 0 || !matches!(position, SeekFrom::Start(n) if n == self.start) {
            return Err(std::io::Error::other("Unexpected shared preview seek"));
        }
        self.source.seek(position)
    }
}

#[derive(Debug, Copy)]
struct ReaderPtr<R> {
    reader: *mut R,
}

impl<R> Clone for ReaderPtr<R> {
    fn clone(&self) -> Self {
        Self {
            reader: self.reader,
        }
    }
}

impl<R> ReaderPtr<R> {
    fn new(reader: &mut R) -> Self {
        Self {
            reader: reader as *mut R,
        }
    }
}

impl<R: Read> Read for ReaderPtr<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        unsafe { (*self.reader).read(buf) }
    }
}

impl<R: Seek> Seek for ReaderPtr<R> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        unsafe { (*self.reader).seek(pos) }
    }
}
