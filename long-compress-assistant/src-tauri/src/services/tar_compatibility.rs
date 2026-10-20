//! Shared logical view of existing tar::Entry, including GNU PAX sparse files.
//! Layouts follow GNU tar's public PAX 0.0/0.1/1.0 specification. No external
//! extraction fallback and no change to the tar crate or archive dependencies.
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

const MAX_SPARSE_BLOCKS: usize = 65_536;
const MAX_MAP_BYTES: u64 = 8 * 1024 * 1024;

/// Consume every enclosing codec member and allow only TAR padding after EOF.
/// A second complete TAR is not one TAR split over several compression streams.
pub(crate) fn finish_stream(mut reader: impl Read, mut check: impl FnMut() -> anyhow::Result<()>) -> anyhow::Result<()> {
    let mut bytes = [0u8; 64 * 1024];
    loop {
        check()?;
        let count = reader.read(&mut bytes)?;
        if count == 0 { return Ok(()); }
        if bytes[..count].iter().any(|byte| *byte != 0) {
            anyhow::bail!("[archive-inspection:unsupported] TAR 结束标记后仍有其他数据，可能包含多个拼接的 TAR；本次没有将其当作完整单包处理，请分别提供原包或重新打包");
        }
    }
}

pub(crate) fn integrity_route(
    path: &Path,
    format: &super::archive_format::ArchiveFormat,
) -> Option<super::compression_format::CompressionRoute> {
    use super::compression_format::{
        compression_route, infer_compression_format, CompressionRoute as Route,
    };
    if *format == super::archive_format::ArchiveFormat::Tar {
        return Some(Route::Tar);
    }
    let logical = if super::numeric_volume_reader::ZipInputReader::is_numeric_first(path) {
        Path::new(path.file_stem()?)
    } else {
        path
    };
    let route = compression_route(&infer_compression_format(&logical.to_string_lossy(), None))?;
    // A suffix describes the inner container, never overrides known magic.
    // Raw TAR was handled above, including a TAR with a misleading suffix.
    use super::archive_format::ArchiveFormat as F;
    matches!(
        (route, format),
        (Route::TarGzip, F::Gzip)
            | (Route::TarLzma, F::Lzma)
            | (Route::TarBzip2, F::Bzip2)
            | (Route::TarXz, F::Xz)
            | (Route::TarZstd, F::Zstd)
            | (Route::TarLz4, F::Lz4)
            | (Route::TarBrotli, F::Brotli)
            | (Route::TarLzip, F::Lzip)
    )
    .then_some(route)
}
pub(crate) fn inspection_error(error: anyhow::Error) -> anyhow::Error {
    if error.to_string().contains("[archive-inspection:damaged] 压缩流") {
        return error.context("[archive-inspection:damaged] TAR 压缩流数据或尾部无效，请重新获取完整原包，不必反复尝试密码");
    }
    if error.to_string().contains("[archive-inspection:") {
        return error;
    }
    if error
        .chain()
        .filter_map(|cause| cause.downcast_ref::<io::Error>())
        .any(|e| {
            matches!(
                e.kind(),
                io::ErrorKind::InvalidData | io::ErrorKind::UnexpectedEof
            )
        })
    {
        error.context("[archive-inspection:damaged] TAR 内容或压缩流尾部不完整，请重新获取完整原包，不必反复尝试密码")
    } else {
        error
    }
}

fn damaged(message: impl std::fmt::Display) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("[archive-inspection:damaged] TAR 稀疏布局无效：{message}"),
    )
}
fn unsupported(message: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, format!("[archive-inspection:unsupported] TAR {message}；请向提供者索取普通 TAR/ZIP，不必反复尝试密码"))
}
fn number(bytes: &[u8]) -> io::Result<u64> {
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return Err(damaged("需要无符号十进制数"));
    }
    std::str::from_utf8(bytes)
        .unwrap()
        .parse()
        .map_err(|_| damaged("数值溢出"))
}
type Blocks = Vec<(u64, u64)>;
type Pax = Vec<(Vec<u8>, Vec<u8>)>;
fn field<'a>(pax: &'a Pax, key: &[u8]) -> Option<&'a [u8]> {
    pax.iter()
        .rev()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_slice())
}
fn required(pax: &Pax, key: &[u8]) -> io::Result<u64> {
    number(
        field(pax, key).ok_or_else(|| damaged(format!("缺少 {}", String::from_utf8_lossy(key))))?,
    )
}
fn validate(blocks: &Blocks, logical: u64, stored: u64) -> io::Result<()> {
    let (mut previous, mut total) = (0u64, 0u64);
    for &(offset, length) in blocks {
        let end = offset
            .checked_add(length)
            .ok_or_else(|| damaged("块范围溢出"))?;
        if offset < previous || end > logical {
            return Err(damaged("块重叠、乱序或超出逻辑大小"));
        }
        total = total
            .checked_add(length)
            .ok_or_else(|| damaged("块大小溢出"))?;
        previous = end;
    }
    if total != stored {
        return Err(damaged("稀疏映射的数据总量与归档条目不一致"));
    }
    if logical > super::extraction_transaction::MAX_EXTRACTED_BYTES {
        return Err(unsupported("逻辑展开大小超出支持范围"));
    }
    Ok(())
}
fn line_number(reader: &mut impl Read, consumed: &mut u64) -> io::Result<u64> {
    let mut digits = Vec::with_capacity(20);
    loop {
        let mut byte = [0];
        reader.read_exact(&mut byte).map_err(|e| {
            if e.kind() == io::ErrorKind::UnexpectedEof {
                damaged("稀疏映射被截断")
            } else {
                e
            }
        })?;
        *consumed += 1;
        if *consumed > MAX_MAP_BYTES {
            return Err(unsupported("稀疏映射过大"));
        }
        if byte[0] == b'\n' {
            return number(&digits);
        }
        if digits.len() == 20 {
            return Err(damaged("稀疏映射数值过长"));
        }
        digits.push(byte[0]);
    }
}

pub(crate) struct LogicalEntry<'a, R: Read> {
    entry: tar::Entry<'a, R>,
    path: PathBuf,
    size: u64,
    payload_size: u64,
    blocks: Option<Blocks>,
    block: usize,
    position: u64,
}
impl<'a, R: Read> LogicalEntry<'a, R> {
    pub(crate) fn new(mut entry: tar::Entry<'a, R>) -> io::Result<Self> {
        let mut path = entry.path()?.into_owned();
        let mut size = entry.size();
        let mut payload_size = if entry.header().entry_type().is_gnu_sparse() {
            entry.header().entry_size()?
        } else {
            size
        };
        let mut pax = Vec::new();
        if let Some(extensions) = entry.pax_extensions()? {
            for extension in extensions {
                let e = extension?;
                pax.push((e.key_bytes().to_vec(), e.value_bytes().to_vec()));
            }
        }
        let sparse = pax.iter().any(|(k, _)| k.starts_with(b"GNU.sparse."));
        if field(&pax, b"SCHILY.realsize").is_some()
            || field(&pax, b"SCHILY.filetype") == Some(&b"sparse"[..])
        {
            return Err(unsupported("star 稀疏布局尚未通过原包验证"));
        }
        let mut blocks = None;
        if sparse {
            if !entry.header().entry_type().is_file() {
                return Err(damaged("PAX 稀疏元数据附在非普通文件上"));
            }
            if let Some(name) = field(&pax, b"GNU.sparse.name") {
                let name =
                    std::str::from_utf8(name).map_err(|_| damaged("GNU.sparse.name 不是 UTF-8"))?;
                if name.is_empty() || name.contains('\0') {
                    return Err(damaged("稀疏文件名无效"));
                }
                path = PathBuf::from(name);
            }
            let mut consumed = 0u64;
            let map = if field(&pax, b"GNU.sparse.major").is_some()
                || field(&pax, b"GNU.sparse.minor").is_some()
            {
                let major = required(&pax, b"GNU.sparse.major")?;
                let minor = required(&pax, b"GNU.sparse.minor")?;
                if (major, minor) != (1, 0) {
                    return Err(unsupported(format!(
                        "PAX 稀疏版本 {major}.{minor} 暂不支持"
                    )));
                }
                size = required(&pax, b"GNU.sparse.realsize")?;
                let count = line_number(&mut entry, &mut consumed)?;
                if count > MAX_SPARSE_BLOCKS as u64 {
                    return Err(unsupported("稀疏映射条目过多"));
                }
                let mut map = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    map.push((
                        line_number(&mut entry, &mut consumed)?,
                        line_number(&mut entry, &mut consumed)?,
                    ));
                }
                let padding = (512 - consumed % 512) % 512;
                for _ in 0..padding {
                    let mut byte = [0];
                    entry.read_exact(&mut byte)?;
                    if byte[0] != 0 {
                        return Err(damaged("稀疏映射填充不是零"));
                    }
                }
                consumed += padding;
                map
            } else if let Some(bytes) = field(&pax, b"GNU.sparse.map") {
                size = required(&pax, b"GNU.sparse.size")?;
                if bytes.len() as u64 > MAX_MAP_BYTES {
                    return Err(unsupported("稀疏映射过大"));
                }
                let mut map = Vec::new();
                if !bytes.is_empty() {
                    let mut values = bytes.split(|b| *b == b',');
                    while let Some(offset) = values.next() {
                        if map.len() == MAX_SPARSE_BLOCKS {
                            return Err(unsupported("稀疏映射条目过多"));
                        }
                        let length = values
                            .next()
                            .ok_or_else(|| damaged("稀疏映射缺少偏移或长度"))?;
                        map.push((number(offset)?, number(length)?));
                    }
                }
                if required(&pax, b"GNU.sparse.numblocks")? != map.len() as u64 {
                    return Err(damaged("稀疏块数不一致"));
                }
                map
            } else {
                size = required(&pax, b"GNU.sparse.size")?;
                let count = required(&pax, b"GNU.sparse.numblocks")?;
                if count > MAX_SPARSE_BLOCKS as u64 {
                    return Err(unsupported("稀疏映射条目过多"));
                }
                let mut map = Vec::with_capacity(count as usize);
                let mut offset = None;
                for (key, value) in &pax {
                    if key == b"GNU.sparse.offset" {
                        if offset.is_some() {
                            return Err(damaged("稀疏偏移缺少配对长度"));
                        }
                        offset = Some(number(value)?);
                    } else if key == b"GNU.sparse.numbytes" {
                        map.push((
                            offset.take().ok_or_else(|| damaged("稀疏长度缺少偏移"))?,
                            number(value)?,
                        ));
                        if map.len() > MAX_SPARSE_BLOCKS {
                            return Err(unsupported("稀疏映射条目过多"));
                        }
                    }
                }
                if offset.is_some() || map.len() as u64 != count {
                    return Err(damaged("稀疏块数或偏移长度不完整"));
                }
                map
            };
            let stored = entry
                .size()
                .checked_sub(consumed)
                .ok_or_else(|| damaged("稀疏映射超出声明条目大小"))?;
            validate(&map, size, stored)?;
            payload_size = stored;
            blocks = Some(map);
        }
        // GNU tar commonly prefixes entries with ./; keep traversal/root
        // components intact for the extraction path checks, remove only CurDir.
        path = path
            .components()
            .filter(|c| !matches!(c, Component::CurDir))
            .collect();
        Ok(Self {
            entry,
            path,
            size,
            payload_size,
            blocks,
            block: 0,
            position: 0,
        })
    }
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
    pub(crate) fn size(&self) -> u64 {
        self.size
    }
    pub(crate) fn payload_size(&self) -> u64 {
        self.payload_size
    }
    pub(crate) fn header(&self) -> &tar::Header {
        self.entry.header()
    }
    pub(crate) fn is_file(&self) -> bool {
        let t = self.header().entry_type();
        t.is_file() || t.is_gnu_sparse() || t.is_contiguous()
    }
    pub(crate) fn require_file_or_dir(&self) -> io::Result<()> {
        if self.is_file() || self.header().entry_type().is_dir() {
            return Ok(());
        }
        let kind = self.header().entry_type();
        let label = if kind.is_symlink() {
            "符号链接"
        } else if kind.is_hard_link() {
            "硬链接"
        } else if kind.is_fifo() {
            "管道"
        } else {
            "链接、设备或非普通条目"
        };
        Err(unsupported(format!(
            "条目「{}」属于{label}，本次不提取；可在归档浏览器中仅选择普通文件",
            self.path.display()
        )))
    }
}

#[derive(Debug)]
pub(crate) struct Plan {
    pub entries: usize,
    pub logical_bytes: u64,
    pub payload_bytes: u64,
}

pub(crate) fn plan(
    path: &Path,
    route: super::compression_format::CompressionRoute,
    options: &crate::models::compression::DecompressOptions,
    check: impl Fn() -> anyhow::Result<()>,
) -> anyhow::Result<Plan> {
    use super::compression_format::CompressionRoute as Route;
    let result = (|| -> anyhow::Result<Plan> {
        let reader: Box<dyn Read + Send> = match route {
            Route::Tar => Box::new(super::numeric_volume_reader::ZipInputReader::open(path)?),
            Route::TarGzip => super::codec_stream::decoder(path, "gz")?,
            Route::TarBzip2 => super::codec_stream::decoder(path, "bz2")?,
            Route::TarXz => super::codec_stream::decoder(path, "xz")?,
            Route::TarZstd => super::codec_stream::decoder(path, "zst")?,
            Route::TarLz4 => super::codec_stream::decoder(path, "lz4")?,
            Route::TarBrotli => super::codec_stream::decoder(path, "br")?,
            Route::TarLzip => super::codec_stream::decoder(path, "lz")?,
            Route::TarLzma => super::codec_stream::decoder(path, "lzma")?,
            _ => anyhow::bail!("Not a native TAR route"),
        };
        struct Cancellable<'a> {
            reader: Box<dyn Read + Send>,
            check: &'a dyn Fn() -> anyhow::Result<()>,
        }
        impl Read for Cancellable<'_> {
            fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
                (self.check)().map_err(io::Error::other)?;
                self.reader.read(b)
            }
        }
        let mut archive = tar::Archive::new(Cancellable {
            reader,
            check: &check,
        });
        let filter =
            super::extraction_transaction::compile_file_filter(options.file_filter.as_deref());
        let mut stats = Plan {
            entries: 0,
            logical_bytes: 0,
            payload_bytes: 0,
        };
        for entry in archive.entries()? {
            check()?;
            let entry = LogicalEntry::new(entry?)?;
            if entry.path().as_os_str().is_empty()
                || !super::extraction_transaction::matches_selected_entries(
                    entry.path(),
                    &options.selected_entries,
                )
                || !super::extraction_transaction::matches_compiled_file_filter(
                    entry.path(),
                    &filter,
                )
            {
                continue;
            }
            stats.entries = stats
                .entries
                .checked_add(1)
                .ok_or_else(|| damaged("条目数溢出"))?;
            if !entry.header().entry_type().is_dir() {
                stats.logical_bytes = stats
                    .logical_bytes
                    .checked_add(entry.size())
                    .ok_or_else(|| damaged("逻辑总量溢出"))?;
                stats.payload_bytes = stats
                    .payload_bytes
                    .checked_add(entry.payload_size())
                    .ok_or_else(|| damaged("数据总量溢出"))?;
            }
        }
        finish_stream(archive.into_inner(), &check)?;
        Ok(stats)
    })();
    check()?;
    result.map_err(inspection_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    #[test]
    fn tar_suffix_must_not_override_a_known_non_tar_signature() {
        use super::super::archive_format::ArchiveFormat as F;
        for name in ["用户下载.tar", "用户下载.tar.gz", "用户下载.tar.xz"] {
            for format in [F::Zip, F::SevenZip, F::Rar, F::AesEncrypted] {
                assert!(
                    integrity_route(Path::new(name), &format).is_none(),
                    "{name}: {format:?}"
                );
            }
        }
    }
    fn decode(pax: &[(&str, &[u8])], payload: &[u8]) -> io::Result<Vec<u8>> {
        let mut writer = tar::Builder::new(Vec::new());
        writer.append_pax_extensions(pax.iter().copied())?;
        let mut header = tar::Header::new_ustar();
        header.set_size(payload.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        writer.append_data(&mut header, "placeholder", payload)?;
        let bytes = writer.into_inner()?;
        let mut archive = tar::Archive::new(Cursor::new(bytes));
        let mut entries = archive.entries()?;
        let mut entry = LogicalEntry::new(entries.next().unwrap()?)?;
        let mut result = Vec::new();
        entry.read_to_end(&mut result)?;
        Ok(result)
    }
    #[test]
    fn decimal_is_strict_and_overflow_checked() {
        for s in [b"".as_slice(), b"-1", b"1x", b"18446744073709551616"] {
            assert_eq!(number(s).unwrap_err().kind(), io::ErrorKind::InvalidData);
        }
    }
    #[test]
    fn sparse_ranges_reject_overlap_order_overflow_size_and_stored_mismatch() {
        for (map, size, stored) in [
            (vec![(3, 2), (4, 1)], 8, 3),
            (vec![(u64::MAX, 2)], 8, 2),
            (vec![(5, 4)], 8, 4),
            (vec![(0, 2)], 8, 3),
        ] {
            assert_eq!(
                validate(&map, size, stored).unwrap_err().kind(),
                io::ErrorKind::InvalidData
            );
        }
    }
    #[test]
    fn empty_map_represents_all_holes_without_allocating_logical_content() {
        assert_eq!(
            decode(
                &[
                    ("GNU.sparse.size", b"16"),
                    ("GNU.sparse.numblocks", b"0"),
                    ("GNU.sparse.map", b"")
                ],
                b""
            )
            .unwrap(),
            vec![0; 16]
        );
    }
    #[test]
    fn pax00_retains_repeated_pairs_and_trailing_holes() {
        assert_eq!(
            decode(
                &[
                    ("GNU.sparse.size", b"8"),
                    ("GNU.sparse.numblocks", b"2"),
                    ("GNU.sparse.offset", b"1"),
                    ("GNU.sparse.numbytes", b"2"),
                    ("GNU.sparse.offset", b"5"),
                    ("GNU.sparse.numbytes", b"1")
                ],
                b"abc"
            )
            .unwrap(),
            b"\0ab\0\0c\0\0"
        );
    }
    #[test]
    fn pax00_rejects_missing_or_out_of_order_pairs() {
        for p in [
            vec![
                ("GNU.sparse.size", &b"8"[..]),
                ("GNU.sparse.numblocks", b"1"),
                ("GNU.sparse.numbytes", b"1"),
            ],
            vec![
                ("GNU.sparse.size", b"8"),
                ("GNU.sparse.numblocks", b"1"),
                ("GNU.sparse.offset", b"1"),
            ],
        ] {
            assert_eq!(
                decode(&p, b"a").unwrap_err().kind(),
                io::ErrorKind::InvalidData
            );
        }
    }
    #[test]
    fn pax01_rejects_odd_maps_and_count_mismatch() {
        for (map, count) in [(&b"1,2,3"[..], &b"1"[..]), (&b"1,2"[..], &b"2"[..])] {
            assert_eq!(
                decode(
                    &[
                        ("GNU.sparse.size", b"8"),
                        ("GNU.sparse.numblocks", count),
                        ("GNU.sparse.map", map)
                    ],
                    b"ab"
                )
                .unwrap_err()
                .kind(),
                io::ErrorKind::InvalidData
            );
        }
    }
    #[test]
    fn pax10_checks_map_padding_and_truncation() {
        for data in [b"1\n0\n1\n".as_slice(), b"1\n0\n1\nX".as_slice()] {
            assert!(decode(
                &[
                    ("GNU.sparse.major", b"1"),
                    ("GNU.sparse.minor", b"0"),
                    ("GNU.sparse.realsize", b"1")
                ],
                data
            )
            .is_err());
        }
    }
    #[test]
    fn unknown_version_is_unsupported_not_a_password_question() {
        let e = decode(
            &[
                ("GNU.sparse.major", b"9"),
                ("GNU.sparse.minor", b"0"),
                ("GNU.sparse.realsize", b"1"),
            ],
            b"",
        )
        .unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::Unsupported);
        assert!(e.to_string().contains("不必反复尝试密码"));
    }
    #[test]
    fn mapped_name_must_be_nonempty_utf8_without_nul() {
        for name in [b"".as_slice(), b"\xff", b"a\0b"] {
            assert!(decode(
                &[
                    ("GNU.sparse.name", name),
                    ("GNU.sparse.size", b"1"),
                    ("GNU.sparse.numblocks", b"0")
                ],
                b""
            )
            .is_err());
        }
    }
    #[test]
    fn source_read_io_kind_is_preserved() {
        struct Failing {
            bytes: Cursor<Vec<u8>>,
        }
        impl Read for Failing {
            fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
                if self.bytes.position() >= 512 {
                    return Err(io::Error::from(io::ErrorKind::PermissionDenied));
                }
                let limit = b.len().min((512 - self.bytes.position()) as usize);
                self.bytes.read(&mut b[..limit])
            }
        }
        let mut writer = tar::Builder::new(Vec::new());
        let mut h = tar::Header::new_ustar();
        h.set_size(1);
        h.set_mode(0o644);
        h.set_cksum();
        writer.append_data(&mut h, "file", &b"a"[..]).unwrap();
        let mut archive = tar::Archive::new(Failing {
            bytes: Cursor::new(writer.into_inner().unwrap()),
        });
        let mut entries = archive.entries().unwrap();
        let mut entry = LogicalEntry::new(entries.next().unwrap().unwrap()).unwrap();
        assert_eq!(
            entry.read(&mut [0]).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
    }
    #[test]
    fn declared_regular_size_does_not_turn_truncation_into_success() {
        let mut writer = tar::Builder::new(Vec::new());
        let mut h = tar::Header::new_ustar();
        h.set_size(4);
        h.set_mode(0o644);
        h.set_cksum();
        writer.append_data(&mut h, "file", &b"abcd"[..]).unwrap();
        let mut bytes = writer.into_inner().unwrap();
        bytes.truncate(514);
        let mut archive = tar::Archive::new(Cursor::new(bytes));
        let mut entries = archive.entries().unwrap();
        let mut entry = LogicalEntry::new(entries.next().unwrap().unwrap()).unwrap();
        let mut output = Vec::new();
        assert_eq!(
            entry.read_to_end(&mut output).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
    }
}
impl<R: Read> Read for LogicalEntry<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() || self.position == self.size {
            return Ok(0);
        }
        let count = buffer
            .len()
            .min((self.size - self.position).min(usize::MAX as u64) as usize);
        let target = if let Some(blocks) = &self.blocks {
            while self.block < blocks.len()
                && self.position >= blocks[self.block].0 + blocks[self.block].1
            {
                self.block += 1;
            }
            let (offset, length) = blocks.get(self.block).copied().unwrap_or((self.size, 0));
            if self.position < offset {
                let n = count.min((offset - self.position) as usize);
                buffer[..n].fill(0);
                self.position += n as u64;
                return Ok(n);
            }
            count.min((offset + length - self.position) as usize)
        } else {
            count
        };
        let n = self.entry.read(&mut buffer[..target])?;
        if n == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!(
                    "[archive-inspection:damaged] TAR 条目「{}」内容被截断，未达到声明逻辑大小",
                    self.path.display()
                ),
            ));
        }
        self.position += n as u64;
        Ok(n)
    }
}
