//! LZOP member framing and checked block decoding. The container is distinct from raw LZO blocks.
//! Framing follows lzop 1.04 src/lzop.c and src/p_lzo.c; no external executable.
use std::io::{self, Read};
use std::path::Path;

pub(crate) const MAGIC: &[u8] = b"\x89LZO\x00\x0d\x0a\x1a\x0a";
const MAX_BLOCK: u32 = 64 * 1024 * 1024; // LZOP format limit.
const ADLER_D: u32 = 1;
const ADLER_C: u32 = 2;
const CRC_D: u32 = 0x100;
const CRC_C: u32 = 0x200;
const HEADER_CRC: u32 = 0x1000;

fn damaged(message: impl std::fmt::Display) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("[archive-inspection:damaged] LZOP {message}，请重新获取完整原文件"),
    )
}
fn unsupported(message: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        format!("[archive-inspection:unsupported] LZOP {message}"),
    )
}
fn exact(input: &mut impl Read, bytes: &mut [u8]) -> io::Result<()> {
    input.read_exact(bytes).map_err(|e| {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            damaged("数据不完整")
        } else {
            e
        }
    })
}
fn read_u32(input: &mut impl Read) -> io::Result<u32> {
    let mut b = [0; 4];
    exact(input, &mut b)?;
    Ok(u32::from_be_bytes(b))
}
fn adler(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for chunk in bytes.chunks(5552) {
        for &byte in chunk {
            a += byte as u32;
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    b << 16 | a
}
fn checksum(bytes: &[u8], crc: bool) -> u32 {
    if crc {
        crc32fast::hash(bytes)
    } else {
        adler(bytes)
    }
}

struct Header<'a, R> {
    input: &'a mut R,
    bytes: Vec<u8>,
}
impl<R: Read> Header<'_, R> {
    fn take(&mut self, n: usize) -> io::Result<&[u8]> {
        let start = self.bytes.len();
        self.bytes.resize(start + n, 0);
        exact(self.input, &mut self.bytes[start..])?;
        Ok(&self.bytes[start..])
    }
    fn byte(&mut self) -> io::Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> io::Result<u16> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
}

struct Block {
    unpacked: u32,
    packed: u32,
    expected_adler: Option<u32>,
    expected_crc: Option<u32>,
    packed_adler: Option<u32>,
    packed_crc: Option<u32>,
}

pub(crate) struct LzopReader<R> {
    input: R,
    flags: u32,
    block: Vec<u8>,
    offset: usize,
    finished: bool,
    failed: Option<(io::ErrorKind, String)>,
    numeric: bool,
    member_mode: bool,
    name: Vec<u8>,
}
impl<R: Read> LzopReader<R> {
    pub(crate) fn new(input: R) -> io::Result<Self> {
        Self::with_mode(input, false)
    }
    fn with_mode(mut input: R, member_mode: bool) -> io::Result<Self> {
        let mut magic = [0; 9];
        exact(&mut input, &mut magic)?;
        if magic != MAGIC {
            return Err(damaged("文件头无效，文件不是 LZOP 容器"));
        }
        let mut header = Header {
            input: &mut input,
            bytes: Vec::with_capacity(300),
        };
        let version = header.u16()?;
        header.u16()?; // producer and LZO library versions
        if version < 0x900 {
            return Err(unsupported("版本过旧，请用 lzop 重新打包"));
        }
        let required = if version >= 0x940 {
            header.u16()?
        } else {
            0x900
        };
        let method = header.byte()?;
        let level = if version >= 0x940 { header.byte()? } else { 0 };
        let flags = header.u32()?;
        let filter = if flags & 0x800 != 0 { header.u32()? } else { 0 };
        header.u32()?;
        header.u32()?; // mode and low mtime, never output paths/permissions
        if version >= 0x940 {
            header.u32()?;
        }
        let name_len = header.byte()? as usize;
        let name = header.take(name_len)?.to_vec();
        let expected = checksum(&header.bytes, flags & HEADER_CRC != 0);
        if read_u32(header.input)? != expected {
            return Err(damaged("文件头校验失败"));
        }
        if required > 0x1040 || required < 0x900 {
            return Err(unsupported("所需版本暂不支持，请用 lzop 解压后再导入"));
        }
        if !(1..=3).contains(&method) || level > 9 {
            return Err(unsupported(
                "压缩方法或等级暂不支持，请用 lzop 解压后再导入",
            ));
        }
        if flags & !(0xfff03fffu32) != 0 {
            return Err(unsupported(
                "文件头扩展标记暂不支持，请用 lzop 解压后再导入",
            ));
        }
        if flags & 0x400 != 0 && !member_mode {
            return Err(unsupported(
                "原生多部分文件暂不支持，请先使用 lzop 合并解压",
            ));
        }
        if filter != 0 {
            return Err(unsupported("附加过滤器暂不支持，请先用 lzop 解压"));
        }
        if flags & ADLER_C != 0 && flags & ADLER_D == 0 || flags & CRC_C != 0 && flags & CRC_D == 0
        {
            return Err(damaged("块校验标记冲突"));
        }
        if flags & 0x40 != 0 {
            // Extra checksum includes its length. Bound it before allocation.
            let len = read_u32(&mut input)?;
            if len > 1024 * 1024 {
                return Err(unsupported("附加元数据超过 1 MiB，请先用 lzop 解压"));
            }
            let mut bytes = vec![0; len as usize + 4];
            bytes[..4].copy_from_slice(&len.to_be_bytes());
            exact(&mut input, &mut bytes[4..])?;
            if read_u32(&mut input)? != checksum(&bytes, flags & HEADER_CRC != 0) {
                return Err(damaged("附加元数据校验失败"));
            }
        }
        Ok(Self {
            input,
            flags,
            block: Vec::new(),
            offset: 0,
            finished: false,
            failed: None,
            numeric: false,
            member_mode,
            name,
        })
    }
    fn block_header(&mut self) -> io::Result<Option<Block>> {
        let unpacked = read_u32(&mut self.input)?;
        if unpacked == 0 {
            let mut tail = Vec::new();
            if !self.member_mode {
                self.input.by_ref().take(9).read_to_end(&mut tail)?;
                if tail == MAGIC {
                    return Err(unsupported(
                        "当前按单文件读取；此文件包含多个成员，请先用 lzop 分别解压",
                    ));
                }
                if !tail.is_empty() {
                    return Err(damaged("结束标记后仍有额外数据"));
                }
            }
            self.finished = true;
            self.block.clear();
            return Ok(None);
        }
        if unpacked == u32::MAX {
            return Err(unsupported("原生分段标记暂不支持，请先使用 lzop 合并解压"));
        }
        if unpacked > MAX_BLOCK {
            return Err(damaged("数据块大小超过格式上限"));
        }
        let packed = read_u32(&mut self.input)?;
        if packed == 0 || packed > unpacked {
            return Err(damaged("数据块长度无效"));
        }
        let expected_adler = if self.flags & ADLER_D != 0 {
            Some(read_u32(&mut self.input)?)
        } else {
            None
        };
        let expected_crc = if self.flags & CRC_D != 0 {
            Some(read_u32(&mut self.input)?)
        } else {
            None
        };
        let packed_adler = if self.flags & ADLER_C != 0 && packed < unpacked {
            Some(read_u32(&mut self.input)?)
        } else {
            None
        };
        let packed_crc = if self.flags & CRC_C != 0 && packed < unpacked {
            Some(read_u32(&mut self.input)?)
        } else {
            None
        };
        Ok(Some(Block {
            unpacked,
            packed,
            expected_adler,
            expected_crc,
            packed_adler,
            packed_crc,
        }))
    }
    fn next_block(&mut self) -> io::Result<()> {
        let Some(Block {
            unpacked,
            packed,
            expected_adler,
            expected_crc,
            packed_adler,
            packed_crc,
        }) = self.block_header()?
        else {
            return Ok(());
        };
        let mut data = vec![0; packed as usize];
        exact(&mut self.input, &mut data)?;
        if packed_adler.is_some_and(|v| v != adler(&data))
            || packed_crc.is_some_and(|v| v != crc32fast::hash(&data))
        {
            return Err(damaged("压缩块校验失败"));
        }
        if packed == unpacked {
            self.block = data;
        } else {
            self.block.resize(unpacked as usize, 0);
            let n = lzokay::decompress::decompress(&data, &mut self.block)
                .map_err(|e| damaged(format!("LZO 数据解码失败：{e:?}")))?;
            if n != unpacked as usize {
                return Err(damaged("数据块展开长度不符"));
            }
        }
        if expected_adler.is_some_and(|v| v != adler(&self.block))
            || expected_crc.is_some_and(|v| v != crc32fast::hash(&self.block))
        {
            return Err(damaged("展开块校验失败"));
        }
        self.offset = 0;
        Ok(())
    }
}
impl<R: Read> Read for LzopReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if let Some((kind, message)) = &self.failed {
            return Err(io::Error::new(*kind, message.clone()));
        }
        if self.finished {
            return Ok(0);
        }
        if self.offset == self.block.len() {
            if let Err(e) = self.next_block() {
                let e = numeric_error(e, self.numeric);
                self.failed = Some((e.kind(), e.to_string()));
                return Err(e);
            }
            if self.finished {
                return Ok(0);
            }
        }
        let n = buf.len().min(self.block.len() - self.offset);
        buf[..n].copy_from_slice(&self.block[self.offset..self.offset + n]);
        self.offset += n;
        Ok(n)
    }
}

// A numeric suffix describes byte slices of one member, not LZOP native multipart.
fn numeric_error(error: io::Error, numeric: bool) -> io::Error {
    if numeric && error.to_string().contains("LZOP 数据不完整") {
        io::Error::new(error.kind(), format!("{error}。数字卷数据未完整结束，可能缺少后续卷或卷已截断；总卷数无法确认，请补齐原始分卷后重试"))
    } else {
        error
    }
}

fn source_input(
    path: &Path,
) -> anyhow::Result<(super::numeric_volume_reader::ZipInputReader, bool)> {
    use super::{
        numeric_volume_reader::ZipInputReader,
        split_archive_detector::{SplitArchiveDetector, SplitFormat},
    };
    if let Some(group) = SplitArchiveDetector::detect_split_archive(path)? {
        if group.format == SplitFormat::GenericNumeric {
            let mut numbers = std::collections::HashSet::new();
            for part in &group.parts {
                let index = part
                    .extension()
                    .and_then(|s| s.to_str())
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0);
                anyhow::ensure!(index > 0 && numbers.insert(index), "[archive-inspection:incomplete] LZOP 数字分卷编号为 0 或重复，请检查同目录文件");
            }
        }
    }
    let source = SplitArchiveDetector::complete_source(path)?;
    let numeric = ZipInputReader::is_numeric_first(&source);
    let input = ZipInputReader::open(&source)?;
    Ok((input, numeric))
}
pub(crate) fn open(
    path: &Path,
) -> anyhow::Result<LzopReader<super::numeric_volume_reader::ZipInputReader>> {
    let (input, numeric) = source_input(path)?;
    let mut reader = LzopReader::new(input).map_err(|e| numeric_error(e, numeric))?;
    reader.numeric = numeric;
    Ok(reader)
}

pub(crate) fn verify(path: &Path, mut cancelled: impl FnMut() -> bool) -> anyhow::Result<()> {
    let members = member_index(path, || {
        anyhow::ensure!(!cancelled(), "操作已取消");
        Ok(())
    })?;
    let mut bytes = [0; 64 * 1024];
    for member in &members {
        let mut reader = member_reader(path, member)?;
        loop {
            anyhow::ensure!(!cancelled(), "操作已取消");
            if reader.read(&mut bytes)? == 0 {
                break;
            }
        }
    }
    Ok(())
}

#[path = "lzop_members.rs"]
mod members;
pub(crate) use members::{extract_members, member_index, member_reader, multi_index};
