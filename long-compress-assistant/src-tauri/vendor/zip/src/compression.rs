//! Possible ZIP compression methods.

use core::fmt;
use std::{fmt::Debug, io};
#[cfg(any(feature = "lzma", feature = "ppmd", feature = "xz"))]
use std::io::Read;

#[allow(deprecated)]
/// Identifies the storage format used to compress a file within a ZIP archive.
///
/// Each file's compression method is stored alongside it, allowing the
/// contents to be read without context.
///
/// When creating ZIP files, you may choose the method to use with
/// [`crate::write::FileOptions::compression_method`]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "_arbitrary", derive(arbitrary::Arbitrary))]
#[non_exhaustive]
pub enum CompressionMethod {
    /// Store the file as is
    Stored,
    /// Compress the file using Deflate
    #[cfg(feature = "_deflate-any")]
    Deflated,
    /// Compress the file using Deflate64.
    /// Decoding deflate64 is supported but encoding deflate64 is not supported.
    #[cfg(feature = "deflate64")]
    Deflate64,
    /// Compress the file using BZIP2
    #[cfg(feature = "_bzip2_any")]
    Bzip2,
    /// Encrypted using AES.
    ///
    /// The actual compression method has to be taken from the AES extra data field
    /// or from `ZipFileData`.
    #[cfg(feature = "aes-crypto")]
    Aes,
    /// Compress the file using `ZStandard`
    #[cfg(feature = "zstd")]
    Zstd,
    /// Compress the file using LZMA
    #[cfg(feature = "lzma")]
    Lzma,
    #[cfg(feature = "legacy-zip")]
    /// Method 1 Shrink
    Shrink,
    #[cfg(feature = "legacy-zip")]
    /// Reduce (Method 2-5)
    Reduce(u8),
    #[cfg(feature = "legacy-zip")]
    /// Method 6 Implode/explode
    Implode,
    /// Compress the file using XZ
    #[cfg(feature = "xz")]
    Xz,
    /// Compress the file using `PPMd`
    #[cfg(feature = "ppmd")]
    Ppmd,
    /// Unsupported compression method
    #[cfg_attr(
        not(any(fuzzing, feature = "_arbitrary")),
        deprecated(since = "0.5.7", note = "use the constants instead")
    )]
    Unsupported(u16),
}
#[allow(deprecated, missing_docs)]
/// All compression methods defined for the ZIP format
impl CompressionMethod {
    pub const STORE: Self = CompressionMethod::Stored;
    #[cfg(feature = "legacy-zip")]
    pub const SHRINK: Self = CompressionMethod::Shrink;
    #[cfg(not(feature = "legacy-zip"))]
    /// Legacy compression method (enable feature `legacy-zip` to get support)
    pub const SHRINK: Self = CompressionMethod::Unsupported(1);
    #[cfg(feature = "legacy-zip")]
    /// Legacy compression method
    pub const REDUCE_1: Self = CompressionMethod::Reduce(1);
    #[cfg(not(feature = "legacy-zip"))]
    /// Legacy compression method (enable feature `legacy-zip` to get support)
    pub const REDUCE_1: Self = CompressionMethod::Unsupported(2);
    #[cfg(feature = "legacy-zip")]
    /// Legacy compression method
    pub const REDUCE_2: Self = CompressionMethod::Reduce(2);
    #[cfg(not(feature = "legacy-zip"))]
    /// Legacy compression method (enable feature `legacy-zip` to get support)
    pub const REDUCE_2: Self = CompressionMethod::Unsupported(3);
    #[cfg(feature = "legacy-zip")]
    /// Legacy compression method
    pub const REDUCE_3: Self = CompressionMethod::Reduce(3);
    #[cfg(not(feature = "legacy-zip"))]
    /// Legacy compression method (enable feature `legacy-zip` to get support)
    pub const REDUCE_3: Self = CompressionMethod::Unsupported(4);
    #[cfg(feature = "legacy-zip")]
    /// Legacy compression method
    pub const REDUCE_4: Self = CompressionMethod::Reduce(4);
    #[cfg(not(feature = "legacy-zip"))]
    /// Legacy compression method (enable feature `legacy-zip` to get support)
    pub const REDUCE_4: Self = CompressionMethod::Unsupported(5);
    #[cfg(feature = "legacy-zip")]
    /// Legacy compression method
    pub const IMPLODE: Self = CompressionMethod::Implode;
    #[cfg(not(feature = "legacy-zip"))]
    /// Legacy compression method (enable feature `legacy-zip` to get support)
    pub const IMPLODE: Self = CompressionMethod::Unsupported(6);
    #[cfg(feature = "_deflate-any")]
    pub const DEFLATE: Self = CompressionMethod::Deflated;
    #[cfg(not(feature = "_deflate-any"))]
    pub const DEFLATE: Self = CompressionMethod::Unsupported(8);
    #[cfg(feature = "deflate64")]
    pub const DEFLATE64: Self = CompressionMethod::Deflate64;
    #[cfg(not(feature = "deflate64"))]
    pub const DEFLATE64: Self = CompressionMethod::Unsupported(9);
    pub const PKWARE_IMPLODE: Self = CompressionMethod::Unsupported(10);
    #[cfg(feature = "_bzip2_any")]
    pub const BZIP2: Self = CompressionMethod::Bzip2;
    #[cfg(not(feature = "_bzip2_any"))]
    pub const BZIP2: Self = CompressionMethod::Unsupported(12);
    #[cfg(not(feature = "lzma"))]
    pub const LZMA: Self = CompressionMethod::Unsupported(14);
    #[cfg(feature = "lzma")]
    pub const LZMA: Self = CompressionMethod::Lzma;
    pub const IBM_ZOS_CMPSC: Self = CompressionMethod::Unsupported(16);
    pub const IBM_TERSE: Self = CompressionMethod::Unsupported(18);
    pub const ZSTD_DEPRECATED: Self = CompressionMethod::Unsupported(20);
    #[cfg(feature = "zstd")]
    pub const ZSTD: Self = CompressionMethod::Zstd;
    #[cfg(not(feature = "zstd"))]
    pub const ZSTD: Self = CompressionMethod::Unsupported(93);
    pub const MP3: Self = CompressionMethod::Unsupported(94);
    #[cfg(feature = "xz")]
    pub const XZ: Self = CompressionMethod::Xz;
    #[cfg(not(feature = "xz"))]
    pub const XZ: Self = CompressionMethod::Unsupported(95);
    pub const JPEG: Self = CompressionMethod::Unsupported(96);
    pub const WAVPACK: Self = CompressionMethod::Unsupported(97);
    #[cfg(feature = "ppmd")]
    pub const PPMD: Self = CompressionMethod::Ppmd;
    #[cfg(not(feature = "ppmd"))]
    pub const PPMD: Self = CompressionMethod::Unsupported(98);
    #[cfg(feature = "aes-crypto")]
    pub const AES: Self = CompressionMethod::Aes;

    // E.4 The compression method field (section 4.4.5) is set to 99
    // to indicate a file has been encrypted using this method.
    #[cfg(not(feature = "aes-crypto"))]
    pub const AES: Self = CompressionMethod::Unsupported(99);

    #[cfg(feature = "_deflate-any")]
    pub const DEFAULT: Self = CompressionMethod::Deflated;

    #[cfg(all(not(feature = "_deflate-any"), feature = "_bzip2_any"))]
    pub const DEFAULT: Self = CompressionMethod::Bzip2;

    #[cfg(all(not(feature = "_deflate-any"), not(feature = "_bzip2_any")))]
    pub const DEFAULT: Self = CompressionMethod::Stored;
}
impl CompressionMethod {
    /// Convert the compression method (as `u16`) to a name
    #[must_use]
    pub const fn name_from_u16(val: u16) -> &'static str {
        match val {
            0 => "Stored",
            1 => "Shrink",
            2 => "Reduce(1)",
            3 => "Reduce(2)",
            4 => "Reduce(3)",
            5 => "Reduce(4)",
            6 => "Implode",
            8 => "Deflated",
            9 => "Deflate64",
            12 => "Bzip2",
            14 => "Lzma",
            95 => "Xz",
            93 => "Zstd",
            98 => "Ppmd",
            99 => "Aes",
            _ => "Unknown",
        }
    }

    pub(crate) const fn parse_from_u16(val: u16) -> Self {
        match val {
            0 => CompressionMethod::Stored,
            #[cfg(feature = "legacy-zip")]
            1 => CompressionMethod::Shrink,
            #[cfg(feature = "legacy-zip")]
            2 => CompressionMethod::Reduce(1),
            #[cfg(feature = "legacy-zip")]
            3 => CompressionMethod::Reduce(2),
            #[cfg(feature = "legacy-zip")]
            4 => CompressionMethod::Reduce(3),
            #[cfg(feature = "legacy-zip")]
            5 => CompressionMethod::Reduce(4),
            #[cfg(feature = "legacy-zip")]
            6 => CompressionMethod::Implode,
            #[cfg(feature = "_deflate-any")]
            8 => CompressionMethod::Deflated,
            #[cfg(feature = "deflate64")]
            9 => CompressionMethod::Deflate64,
            #[cfg(feature = "_bzip2_any")]
            12 => CompressionMethod::Bzip2,
            #[cfg(feature = "lzma")]
            14 => CompressionMethod::Lzma,
            #[cfg(feature = "xz")]
            95 => CompressionMethod::Xz,
            #[cfg(feature = "zstd")]
            93 => CompressionMethod::Zstd,
            #[cfg(feature = "ppmd")]
            98 => CompressionMethod::Ppmd,
            #[cfg(feature = "aes-crypto")]
            99 => CompressionMethod::Aes,
            #[allow(deprecated)]
            v => CompressionMethod::Unsupported(v),
        }
    }

    /// Converts a u16 to its corresponding `CompressionMethod`
    #[deprecated(
        since = "0.5.7",
        note = "use a constant to construct a compression method"
    )]
    #[must_use]
    pub const fn from_u16(val: u16) -> CompressionMethod {
        Self::parse_from_u16(val)
    }

    pub(crate) const fn serialize_to_u16(self) -> u16 {
        match self {
            CompressionMethod::Stored => 0,
            #[cfg(feature = "legacy-zip")]
            CompressionMethod::Shrink => 1,
            #[cfg(feature = "legacy-zip")]
            CompressionMethod::Reduce(n) => 1 + n as u16,
            #[cfg(feature = "legacy-zip")]
            CompressionMethod::Implode => 6,

            #[cfg(feature = "_deflate-any")]
            CompressionMethod::Deflated => 8,
            #[cfg(feature = "deflate64")]
            CompressionMethod::Deflate64 => 9,
            #[cfg(feature = "_bzip2_any")]
            CompressionMethod::Bzip2 => 12,
            #[cfg(feature = "zstd")]
            CompressionMethod::Zstd => 93,
            #[cfg(feature = "lzma")]
            CompressionMethod::Lzma => 14,
            #[cfg(feature = "xz")]
            CompressionMethod::Xz => 95,
            #[cfg(feature = "ppmd")]
            CompressionMethod::Ppmd => 98,
            // E.4 The compression method field (section 4.4.5) is set to 99
            // to indicate a file has been encrypted using this method.
            #[cfg(feature = "aes-crypto")]
            CompressionMethod::Aes => 99,
            #[allow(deprecated)]
            CompressionMethod::Unsupported(v) => v,
        }
    }

    /// Converts a `CompressionMethod` to a u16
    #[deprecated(
        since = "0.5.7",
        note = "to match on other compression methods, use a constant"
    )]
    #[must_use]
    pub const fn to_u16(self) -> u16 {
        self.serialize_to_u16()
    }
}

impl Default for CompressionMethod {
    #[cfg(feature = "_deflate-any")]
    fn default() -> Self {
        CompressionMethod::Deflated
    }

    #[cfg(not(feature = "_deflate-any"))]
    fn default() -> Self {
        CompressionMethod::Stored
    }
}

impl fmt::Display for CompressionMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Just duplicate what the Debug format looks like, i.e, the enum key:
        let method_u16 = self.serialize_to_u16();
        let name = Self::name_from_u16(method_u16);
        write!(f, "{name}")
    }
}

/// The compression methods which have been implemented.
pub const SUPPORTED_COMPRESSION_METHODS: &[CompressionMethod] = &[
    CompressionMethod::Stored,
    #[cfg(feature = "_deflate-any")]
    CompressionMethod::Deflated,
    #[cfg(feature = "deflate64")]
    CompressionMethod::Deflate64,
    #[cfg(feature = "_bzip2_any")]
    CompressionMethod::Bzip2,
    #[cfg(feature = "zstd")]
    CompressionMethod::Zstd,
    #[cfg(feature = "xz")]
    CompressionMethod::Xz,
    #[cfg(feature = "ppmd")]
    CompressionMethod::Ppmd,
];

pub(crate) enum Decompressor<R: io::BufRead> {
    Stored(R),
    #[cfg(feature = "deflate-flate2")]
    Deflated(flate2::bufread::DeflateDecoder<R>),
    #[cfg(feature = "deflate64")]
    Deflate64(deflate64::Deflate64Decoder<R>),
    #[cfg(feature = "_bzip2_any")]
    Bzip2(bzip2::bufread::BzDecoder<R>),
    #[cfg(feature = "zstd")]
    Zstd(zstd::Decoder<'static, R>),
    #[cfg(feature = "lzma")]
    Lzma(Lzma<R>),
    #[cfg(feature = "legacy-zip")]
    Shrink(crate::legacy::shrink::ShrinkDecoder<R>),
    #[cfg(feature = "legacy-zip")]
    Reduce(crate::legacy::reduce::ReduceDecoder<R>),
    #[cfg(feature = "legacy-zip")]
    Implode(crate::legacy::implode::ImplodeDecoder<R>),
    #[cfg(feature = "xz")]
    Xz { decoder: Box<lzma_rust2::XzReader<R>>, remaining: u64, finished: bool },
    #[cfg(feature = "ppmd")]
    Ppmd(Ppmd<R>),
}

impl<R: io::BufRead> Debug for Decompressor<R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Stored(_) => write!(f, "StoredDecompressor"),
            #[cfg(feature = "deflate-flate2")]
            Self::Deflated(_) => write!(f, "DeflatedDecompressor"),
            #[cfg(feature = "deflate64")]
            Self::Deflate64(_) => write!(f, "Deflate64Decompressor"),
            #[cfg(feature = "_bzip2_any")]
            Self::Bzip2(_) => write!(f, "Bzip2Decompressor"),
            #[cfg(feature = "zstd")]
            Self::Zstd(_) => write!(f, "ZstdDecompressor"),
            #[cfg(feature = "lzma")]
            Self::Lzma(_) => write!(f, "LzmaDecompressor"),
            #[cfg(feature = "legacy-zip")]
            Self::Shrink(_) => write!(f, "ShrinkDecompressor"),
            #[cfg(feature = "legacy-zip")]
            Self::Reduce(_) => write!(f, "ReduceDecompressor"),
            #[cfg(feature = "legacy-zip")]
            Self::Implode(_) => write!(f, "ImplodeDecompressor"),
            #[cfg(feature = "xz")]
            Self::Xz { .. } => write!(f, "XzDecompressor"),
            #[cfg(feature = "ppmd")]
            Self::Ppmd(_) => write!(f, "PpmdDecompressor"),
        }
    }
}

#[cfg(feature = "lzma")]
pub(crate) enum Lzma<R: io::BufRead> {
    Uninitialized {
        reader: Option<R>,
        uncompressed_size: u64,
        require_eos: bool,
    },
    Initialized { decoder: Box<lzma_rust2::LzmaReader<R>>, remaining: u64 },
}

#[cfg(feature = "ppmd")]
pub(crate) enum Ppmd<R: io::BufRead> {
    Uninitialized { reader: Option<R>, uncompressed_size: u64 },
    Initialized { decoder: Box<ppmd_rust::Ppmd8Decoder<PpmdInput<R>>>, remaining: u64, finished: bool },
}

// ppmd-rust treats UnexpectedEof as a sized-stream ending. ZIP method 98
// requires the actual end marker, so make physical exhaustion a hard error.
#[cfg(feature = "ppmd")]
pub(crate) struct PpmdInput<R> { inner: R }
#[cfg(feature = "ppmd")]
impl<R: io::Read> io::Read for PpmdInput<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() { return Ok(0); }
        match self.inner.read(buf) {
            Ok(0) => Err(io::Error::new(io::ErrorKind::InvalidData, "ZIP PPMd 未完整结束；请补齐或重新获取来源后重试")),
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => Err(io::Error::new(io::ErrorKind::InvalidData, format!("ZIP PPMd 未完整结束：{error}"))),
            result => result,
        }
    }
}
#[cfg(feature = "ppmd")]
fn read_zip_ppmd<R: io::BufRead>(decoder: &mut ppmd_rust::Ppmd8Decoder<PpmdInput<R>>, remaining: &mut u64, finished: &mut bool, buf: &mut [u8]) -> io::Result<usize> {
    if buf.is_empty() || *finished { return Ok(0); }
    if *remaining == 0 {
        if decoder.read(&mut [0; 1])? != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "ZIP PPMd 内容超出目录记录大小，条目未通过完整性检查"));
        }
        // Read through the crypto wrapper as well: no trailing payload, and any
        // final AES authentication error must be observed before reporting EOF.
        if decoder.get_mut().inner.read(&mut [0; 1])? != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "ZIP PPMd 结束标记后存在多余压缩数据，条目未通过完整性检查"));
        }
        *finished = true;
        return Ok(0);
    }
    let limit = (buf.len() as u64).min(*remaining) as usize;
    let n = decoder.read(&mut buf[..limit])?;
    if n == 0 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "ZIP PPMd 内容小于目录记录大小，条目未完整读取"));
    }
    *remaining -= n as u64;
    Ok(n)
}

// Keep method 95's existing single-XZ-stream decoder, but require exact
// output size and payload exhaustion, as ZIP receivers do for this method.
#[cfg(feature = "xz")]
fn read_zip_xz<R: io::BufRead>(decoder: &mut lzma_rust2::XzReader<R>, remaining: &mut u64, finished: &mut bool, buf: &mut [u8]) -> io::Result<usize> {
    if buf.is_empty() || *finished { return Ok(0); }
    if *remaining == 0 {
        if decoder.read(&mut [0; 1])? != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "ZIPX（XZ）内容超出目录记录大小，条目未通过完整性检查"));
        }
        if decoder.inner_mut().read(&mut [0; 1])? != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "ZIPX（XZ）结束后存在额外数据、填充或后续流；请用原工具重新导出无附加数据的单个 XZ 流条目"));
        }
        *finished = true;
        return Ok(0);
    }
    let limit = (buf.len() as u64).min(*remaining) as usize;
    let n = decoder.read(&mut buf[..limit])?;
    if n == 0 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "ZIPX（XZ）内容小于目录记录大小，条目未完整读取"));
    }
    *remaining -= n as u64;
    Ok(n)
}

// Always check the exact decoded size. For EOS streams the final one-byte
// probe consumes the marker (or detects a truncated/longer stream) before EOF.
#[cfg(feature = "lzma")]
fn read_zip_lzma<R: io::BufRead>(decoder: &mut lzma_rust2::LzmaReader<R>, remaining: &mut u64, buf: &mut [u8]) -> io::Result<usize> {
    if buf.is_empty() { return Ok(0); }
    if *remaining == 0 {
        if decoder.read(&mut [0; 1])? != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "ZIP LZMA 内容超出目录记录大小，条目未通过完整性检查"));
        }
        return Ok(0);
    }
    let limit = (buf.len() as u64).min(*remaining) as usize;
    let n = decoder.read(&mut buf[..limit])?;
    if n == 0 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "ZIP LZMA 未完整结束，内容小于目录记录大小"));
    }
    *remaining -= n as u64;
    Ok(n)
}

impl<R: io::BufRead> io::Read for Decompressor<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Decompressor::Stored(r) => r.read(buf),
            #[cfg(feature = "deflate-flate2")]
            Decompressor::Deflated(r) => r.read(buf),
            #[cfg(feature = "deflate64")]
            Decompressor::Deflate64(r) => r.read(buf),
            #[cfg(feature = "_bzip2_any")]
            Decompressor::Bzip2(r) => r.read(buf),
            #[cfg(feature = "zstd")]
            Decompressor::Zstd(r) => {
                if buf.is_empty() { return Ok(0); }
                r.read(buf).map_err(|error| {
                    let detail = error.to_string();
                    let reason = if detail.contains("Dictionary") {
                        "条目需要外部字典或字典不匹配，当前无法提供外部字典；请用原工具重新导出不依赖字典的 ZIP"
                    } else if detail.contains("Window size too large") || detail.contains("Frame requires too much memory") {
                        "条目窗口超过当前解码器支持范围；请用原工具减小窗口后重新导出"
                    } else if error.kind() == io::ErrorKind::UnexpectedEof {
                        "条目未完整结束；请补齐来源后重试"
                    } else {
                        "内容无法完整解码；请检查来源完整性和密码"
                    };
                    io::Error::new(error.kind(), format!("ZIP Zstandard：{reason}（{detail}）"))
                })
            },
            #[cfg(feature = "lzma")]
            Decompressor::Lzma(r) => match r {
                Lzma::Uninitialized {
                    reader,
                    uncompressed_size,
                    require_eos,
                } => {
                    if buf.is_empty() { return Ok(0); }
                    let mut reader = reader.take().ok_or_else(|| {
                        io::Error::other("Reader was not set while reading LZMA data")
                    })?;

                    // 5.8.8.1 LZMA Version Information & 5.8.8.2 LZMA Properties Size
                    let mut header = [0; 4];
                    reader.read_exact(&mut header)?;
                    let _version_information =
                        u16::from_le_bytes(header[0..2].try_into().map_err(|e| {
                            std::io::Error::other(format!("Cannot transform header to u16: {e}"))
                        })?);
                    let properties_size =
                        u16::from_le_bytes(header[2..4].try_into().map_err(|e| {
                            std::io::Error::other(format!("Cannot transform header to u16: {e}"))
                        })?);
                    if properties_size != 5 {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("unexpected LZMA properties size of {properties_size}"),
                        ));
                    }

                    let mut props_data = [0; 5];
                    reader.read_exact(&mut props_data)?;
                    let props = props_data[0];
                    let dict_size =
                        u32::from_le_bytes(props_data[1..5].try_into().map_err(|e| {
                            std::io::Error::other(format!("Cannot transform header to u32: {e}"))
                        })?);

                    // Bit 1 promises EOS. Decode to that marker rather than silently
                    // accepting a matching prefix at the central directory's size.
                    // Preserve the old small-file dictionary clamp when size is unknown
                    // to the decoder, and validate the original properties before it.
                    lzma_rust2::lzma_get_memory_usage_by_props(dict_size, props)?;
                    let effective_dict = u64::from(dict_size).min(*uncompressed_size).max(4096) as u32;
                    let mut decompressor = lzma_rust2::LzmaReader::new_with_props(
                        reader,
                        if *require_eos { u64::MAX } else { *uncompressed_size },
                        props,
                        effective_dict,
                        None,
                    )?;
                    let mut remaining = *uncompressed_size;
                    let read = read_zip_lzma(&mut decompressor, &mut remaining, buf)?;
                    *r = Lzma::Initialized { decoder: Box::new(decompressor), remaining };
                    Ok(read)
                }
                Lzma::Initialized { decoder, remaining } => read_zip_lzma(decoder, remaining, buf),
            },
            #[cfg(feature = "xz")]
            Decompressor::Xz { decoder, remaining, finished } => read_zip_xz(decoder, remaining, finished, buf),
            #[cfg(feature = "ppmd")]
            Decompressor::Ppmd(r) => match r {
                Ppmd::Uninitialized { reader, uncompressed_size } => {
                    if buf.is_empty() { return Ok(0); }
                    let mut reader = reader.take().ok_or_else(|| {
                        io::Error::other("Reader was not set while reading PPMd data")
                    })?;

                    let mut buffer = [0; 2];
                    reader.read_exact(&mut buffer)?;
                    let parameters = u16::from_le_bytes(buffer);

                    let order = u32::from((parameters & 0x0F) + 1);
                    let memory_size = 1024 * 1024 * u32::from(((parameters >> 4) & 0xFF) + 1);
                    let restoration_method = (parameters >> 12) & 0x0F;

                    if order < 2 || restoration_method > 1 {
                        return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("ZIP PPMd 参数暂不支持（阶数 {order}，恢复方式 {restoration_method}）；请用原工具重新导出 Restart/CutOff 格式")));
                    }
                    let mut decompressor = ppmd_rust::Ppmd8Decoder::new(
                        PpmdInput { inner: reader },
                        order,
                        memory_size,
                        restoration_method.into(),
                    )
                    .map_err(|error| match error {
                        ppmd_rust::Error::RangeDecoderInitialization => io::Error::new(
                            io::ErrorKind::InvalidData,
                            "ZIP PPMd 范围解码器初始化失败；请检查来源完整性",
                        ),
                        ppmd_rust::Error::InvalidParameter => {
                            io::Error::new(io::ErrorKind::InvalidInput, "ZIP PPMd 参数暂不支持；请用原工具重新导出")
                        }
                        ppmd_rust::Error::IoError(io_error) => io_error,
                        ppmd_rust::Error::MemoryAllocation => {
                            io::Error::new(io::ErrorKind::OutOfMemory, format!("ZIP PPMd 需要 {} MiB 解码内存，当前分配失败；请释放内存或用原工具减小内存参数后重新导出", memory_size / 1024 / 1024))
                        }
                    })?;

                    let mut remaining = *uncompressed_size;
                    let mut finished = false;
                    let read = read_zip_ppmd(&mut decompressor, &mut remaining, &mut finished, buf)?;
                    *r = Ppmd::Initialized { decoder: Box::new(decompressor), remaining, finished };
                    Ok(read)
                }
                Ppmd::Initialized { decoder, remaining, finished } => read_zip_ppmd(decoder, remaining, finished, buf),
            },
            #[cfg(feature = "legacy-zip")]
            Decompressor::Shrink(r) => r.read(buf),
            #[cfg(feature = "legacy-zip")]
            Decompressor::Reduce(r) => r.read(buf),
            #[cfg(feature = "legacy-zip")]
            Decompressor::Implode(r) => r.read(buf),
        }
    }
}

impl<R: io::BufRead> Decompressor<R> {
    pub fn new(
        reader: R,
        compression_method: CompressionMethod,
        #[cfg_attr(not(any(feature = "lzma", feature = "ppmd", feature = "xz", feature = "legacy-zip")), allow(unused))]
        uncompressed_size: u64,
        #[cfg_attr(not(any(feature = "legacy-zip", feature = "lzma")), allow(unused))] flags: u16,
    ) -> crate::result::ZipResult<Self> {
        Ok(match compression_method {
            CompressionMethod::Stored => Decompressor::Stored(reader),
            #[cfg(feature = "deflate-flate2")]
            CompressionMethod::Deflated => {
                Decompressor::Deflated(flate2::bufread::DeflateDecoder::new(reader))
            }
            #[cfg(feature = "deflate64")]
            CompressionMethod::Deflate64 => {
                Decompressor::Deflate64(deflate64::Deflate64Decoder::with_buffer(reader))
            }
            #[cfg(feature = "_bzip2_any")]
            CompressionMethod::Bzip2 => Decompressor::Bzip2(bzip2::bufread::BzDecoder::new(reader)),
            #[cfg(feature = "zstd")]
            CompressionMethod::Zstd => Decompressor::Zstd(zstd::Decoder::with_buffer(reader)?),
            #[cfg(feature = "lzma")]
            CompressionMethod::Lzma => Decompressor::Lzma(Lzma::Uninitialized {
                reader: Some(reader),
                uncompressed_size,
                require_eos: flags & (1 << 1) != 0,
            }),
            #[cfg(feature = "xz")]
            CompressionMethod::Xz => {
                Decompressor::Xz { decoder: Box::new(lzma_rust2::XzReader::new(reader, false)), remaining: uncompressed_size, finished: false }
            }
            #[cfg(feature = "ppmd")]
            CompressionMethod::Ppmd => Decompressor::Ppmd(Ppmd::Uninitialized { reader: Some(reader), uncompressed_size }),
            #[cfg(feature = "legacy-zip")]
            CompressionMethod::Shrink => Decompressor::Shrink(
                crate::legacy::shrink::ShrinkDecoder::new(reader, uncompressed_size),
            ),
            #[cfg(feature = "legacy-zip")]
            CompressionMethod::Reduce(n) => Decompressor::Reduce(
                crate::legacy::reduce::ReduceDecoder::new(reader, uncompressed_size, n),
            ),
            #[cfg(feature = "legacy-zip")]
            CompressionMethod::Implode => Decompressor::Implode(
                crate::legacy::implode::ImplodeDecoder::new(reader, uncompressed_size, flags),
            ),
            method => {
                let method = method.serialize_to_u16();
                return Err(crate::result::ZipError::CompressionMethodNotSupported(
                    method,
                ));
            }
        })
    }

    /// Consumes this decoder, returning the underlying reader.
    #[allow(clippy::infallible_destructuring_match)]
    pub fn into_inner(self) -> io::Result<R> {
        let inner = match self {
            Decompressor::Stored(r) => r,
            #[cfg(feature = "deflate-flate2")]
            Decompressor::Deflated(r) => r.into_inner(),
            #[cfg(feature = "deflate64")]
            Decompressor::Deflate64(r) => r.into_inner(),
            #[cfg(feature = "_bzip2_any")]
            Decompressor::Bzip2(r) => r.into_inner(),
            #[cfg(feature = "zstd")]
            Decompressor::Zstd(r) => r.finish(),
            #[cfg(feature = "lzma")]
            Decompressor::Lzma(r) => match r {
                Lzma::Uninitialized { mut reader, .. } => reader
                    .take()
                    .ok_or_else(|| io::Error::other("Reader was not set"))?,
                Lzma::Initialized { decoder, .. } => decoder.into_inner(),
            },
            #[cfg(feature = "legacy-zip")]
            Decompressor::Shrink(r) => r.into_inner(),
            #[cfg(feature = "legacy-zip")]
            Decompressor::Reduce(r) => r.into_inner(),
            #[cfg(feature = "legacy-zip")]
            Decompressor::Implode(r) => r.into_inner(),
            #[cfg(feature = "xz")]
            Decompressor::Xz { decoder, .. } => decoder.into_inner(),
            #[cfg(feature = "ppmd")]
            Decompressor::Ppmd(r) => match r {
                Ppmd::Uninitialized { mut reader, .. } => reader
                    .take()
                    .ok_or_else(|| io::Error::other("Reader was not set"))?,
                Ppmd::Initialized { decoder, .. } => decoder.into_inner().inner,
            },
        };
        Ok(inner)
    }
}

#[cfg(test)]
mod tests {
    use super::{CompressionMethod, SUPPORTED_COMPRESSION_METHODS};

    #[test]
    fn from_eq_to() {
        for v in 0..(u32::from(u16::MAX) + 1) {
            let from = CompressionMethod::parse_from_u16(v as u16);
            let to = u32::from(from.serialize_to_u16());
            assert_eq!(v, to);
        }
    }

    #[test]
    fn to_eq_from() {
        fn check_match(method: CompressionMethod) {
            let to = method.serialize_to_u16();
            let from = CompressionMethod::parse_from_u16(to);
            let back = from.serialize_to_u16();
            assert_eq!(to, back);
        }

        for &method in SUPPORTED_COMPRESSION_METHODS {
            check_match(method);
        }
    }

    #[test]
    fn to_display_fmt() {
        fn check_match(method: CompressionMethod) {
            let debug_str = format!("{method:?}");
            let display_str = format!("{method}");
            assert_eq!(debug_str, display_str);
        }

        for &method in SUPPORTED_COMPRESSION_METHODS {
            check_match(method);
        }
    }
}
