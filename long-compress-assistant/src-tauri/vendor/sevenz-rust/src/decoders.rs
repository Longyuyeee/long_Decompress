use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};
#[cfg(feature = "bzip2")]
use bzip2::read::BzDecoder;

#[cfg(feature = "aes256")]
use crate::aes256sha256::Aes256Sha256Decoder;
use crate::{
    archive::SevenZMethod,
    bcj::SimpleReader,
    delta::DeltaReader,
    error::Error,
    folder::Coder,
    lzma::{lzma2_get_memery_usage, LZMA2Reader, LZMAReader},
};

pub enum Decoder<R: Read> {
    COPY(R),
    LZMA(LZMAReader<R>),
    LZMA2(LZMA2Reader<R>),
    BCJ(SimpleReader<R>),
    Delta(DeltaReader<R>),
    #[cfg(feature = "additional-codecs")]
    Deflate(flate2::read::DeflateDecoder<R>),
    #[cfg(feature = "additional-codecs")]
    Deflate64(deflate64::Deflate64Decoder<std::io::BufReader<R>>),
    #[cfg(feature = "additional-codecs")]
    Ppmd(ppmd_rust::Ppmd7Decoder<R>),
    #[cfg(feature = "zstd")]
    ZSTD(zstd::Decoder<'static, std::io::BufReader<R>>),
    #[cfg(feature = "bzip2")]
    BZip2(BzDecoder<R>),
    #[cfg(feature = "aes256")]
    AES256SHA256(Aes256Sha256Decoder<R>),
    #[cfg(feature = "aes256")]
    PreviewAES(crate::aes256sha256::PreviewAes<R>),
}

// impl<R: Read> Decoder<R> {
//     pub fn num_streams(&self) -> usize {
//         match self {
//             Self::BCJ(_) => 4,
//             _ => 1,
//         }
//     }
// }

impl<R: Read> Read for Decoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(feature = "zstd")]
            Decoder::ZSTD(r) => r.read(buf),
            Decoder::COPY(r) => r.read(buf),
            Decoder::LZMA(r) => r.read(buf),
            Decoder::LZMA2(r) => r.read(buf),
            Decoder::BCJ(r) => r.read(buf),
            Decoder::Delta(r) => r.read(buf),
            #[cfg(feature = "additional-codecs")]
            Decoder::Deflate(r) => r.read(buf),
            #[cfg(feature = "additional-codecs")]
            Decoder::Deflate64(r) => r.read(buf),
            #[cfg(feature = "additional-codecs")]
            Decoder::Ppmd(r) => r.read(buf),
            #[cfg(feature = "bzip2")]
            Decoder::BZip2(r) => r.read(buf),
            #[cfg(feature = "aes256")]
            Decoder::AES256SHA256(r) => r.read(buf),
            #[cfg(feature = "aes256")]
            Decoder::PreviewAES(r) => r.read(buf),
        }
    }
}

pub fn add_decoder<I: Read>(
    input: I,
    uncompressed_len: usize,
    coder: &Coder,
    #[allow(unused)] password: &[u8],
    max_mem_limit_kb: usize,
) -> Result<Decoder<I>, Error> {
    crate::preview::coder(coder, uncompressed_len)?;
    add_decoder_inner(input, uncompressed_len, coder, password, max_mem_limit_kb)
}

// Prefix output is capped by the caller, so declared total size is not an
// allocation. Keep the scoped dictionary/shape checks, and allow only a bare
// LZMA/LZMA2/PPMd codec after any explicit partial AES stage. Complete decoder
// defaults remain unchanged.
pub(crate) fn add_prefix_decoder<I: Read>(input: I, uncompressed_len: usize, coder: &Coder, max_mem_limit_kb: usize) -> Result<Decoder<I>, Error> {
    let id = coder.decompression_method_id();
    if id != SevenZMethod::ID_LZMA && id != SevenZMethod::ID_LZMA2 && id != SevenZMethod::ID_PPMD {
        return Err(Error::unsupported("Prefix requires LZMA/LZMA2/PPMd"));
    }
    if id==SevenZMethod::ID_PPMD {
        let p=&coder.properties;
        if p.len()!=5 || !(2..=64).contains(&p[0]) || !(2048..=64*1024*1024).contains(&u32::from_le_bytes(p[1..5].try_into().unwrap())) {
            return Err(Error::unsupported("7Z PPMd 预览阶数或字典超出范围，请先提取后打开"));
        }
    }
    crate::preview::coder(coder, 0)?;
    add_decoder_inner(input, uncompressed_len, coder, &[], max_mem_limit_kb)
}

fn add_decoder_inner<I: Read>(input: I, uncompressed_len: usize, coder: &Coder, #[allow(unused)] password: &[u8], max_mem_limit_kb: usize) -> Result<Decoder<I>, Error> {
    let method = SevenZMethod::by_id(coder.decompression_method_id());
    let method = if let Some(m) = method {
        m
    } else {
        return Err(Error::UnsupportedCompressionMethod(format!(
            "{:?}",
            coder.decompression_method_id()
        )));
    };
    match method.id() {
        SevenZMethod::ID_COPY => Ok(Decoder::COPY(input)),
        #[cfg(feature = "additional-codecs")]
        SevenZMethod::ID_DEFLATE => Ok(Decoder::Deflate(flate2::read::DeflateDecoder::new(input))),
        #[cfg(feature = "additional-codecs")]
        SevenZMethod::ID_DEFLATE64 => Ok(Decoder::Deflate64(deflate64::Deflate64Decoder::new(input))),
        #[cfg(feature = "additional-codecs")]
        SevenZMethod::ID_PPMD => {
            // 7z PPMdH properties: one order byte and a LE u32 arena size.
            // Keep the caller's memory limit and reject properties before allocation.
            if coder.properties.len() != 5 {
                return Err(Error::other("Invalid PPMd properties length"));
            }
            let order = coder.properties[0] as u32;
            let memory = u32::from_le_bytes(coder.properties[1..5].try_into().unwrap());
            let memory_kb = (memory as usize).div_ceil(1024);
            if memory_kb > max_mem_limit_kb {
                return Err(Error::MaxMemLimited { max_kb: max_mem_limit_kb, actaul_kb: memory_kb });
            }
            let decoder = ppmd_rust::Ppmd7Decoder::new(input, order, memory)
                .map_err(|error| Error::other(format!("PPMd decoder: {error}")))?;
            Ok(Decoder::Ppmd(decoder))
        }
        #[cfg(feature = "zstd")]
        SevenZMethod::ID_ZSTD => {
            let zs = zstd::Decoder::new(input).unwrap();
            Ok(Decoder::ZSTD(zs))
        }
        SevenZMethod::ID_LZMA => {
            let dict_size = get_lzma_dic_size(coder)?;
            if coder.properties.is_empty() {
                return Err(Error::Other("LZMA properties too short".into()));
            }
            let props = coder.properties[0];
            let lz =
                LZMAReader::new_with_props(input, uncompressed_len as _, props, dict_size, None)
                    .map_err(|e| Error::bad_password(e, !password.is_empty()))?;
            Ok(Decoder::LZMA(lz))
        }
        SevenZMethod::ID_LZMA2 => {
            let dic_size = get_lzma2_dic_size(coder)?;
            let mem_size = lzma2_get_memery_usage(dic_size) as usize;
            if mem_size > max_mem_limit_kb {
                return Err(Error::MaxMemLimited {
                    max_kb: max_mem_limit_kb,
                    actaul_kb: mem_size,
                });
            }
            let lz = LZMA2Reader::new(input, dic_size, None);
            Ok(Decoder::LZMA2(lz))
        }
        SevenZMethod::ID_BCJ_X86 => {
            let de = SimpleReader::new_x86(input);
            Ok(Decoder::BCJ(de))
        }
        SevenZMethod::ID_BCJ_ARM => {
            let de = SimpleReader::new_arm(input);
            Ok(Decoder::BCJ(de))
        }
        SevenZMethod::ID_BCJ_ARM_THUMB => {
            let de = SimpleReader::new_arm_thumb(input);
            Ok(Decoder::BCJ(de))
        }
        SevenZMethod::ID_BCJ_PPC => {
            let de = SimpleReader::new_ppc(input);
            Ok(Decoder::BCJ(de))
        }
        SevenZMethod::ID_BCJ_SPARC => {
            let de = SimpleReader::new_sparc(input);
            Ok(Decoder::BCJ(de))
        }
        SevenZMethod::ID_DELTA => {
            let d = if coder.properties.is_empty() {
                1
            } else {
                coder.properties[0].wrapping_add(1)
            };
            let de = DeltaReader::new(input, d as usize);
            Ok(Decoder::Delta(de))
        }
        #[cfg(feature = "bzip2")]
        SevenZMethod::ID_BZIP2 => {
            let de = BzDecoder::new(input);
            Ok(Decoder::BZip2(de))
        }
        #[cfg(feature = "aes256")]
        SevenZMethod::ID_AES256SHA256 => {
            crate::preview::aes(&coder.properties,password)?;
            if password.is_empty() {
                return Err(Error::PasswordRequired);
            }
            if crate::preview::active() {
                return Ok(Decoder::PreviewAES(crate::aes256sha256::PreviewAes::new(input,&coder.properties,password,uncompressed_len)?));
            }
            let de = Aes256Sha256Decoder::new(input, &coder.properties, password)?;
            Ok(Decoder::AES256SHA256(de))
        }
        _ => Err(Error::UnsupportedCompressionMethod(
            method.name().to_string(),
        )),
    }
}

#[inline]
fn get_lzma2_dic_size(coder: &Coder) -> Result<u32, Error> {
    if coder.properties.is_empty() {
        return Err(Error::other("LZMA2 properties too short"));
    }
    let dict_size_bits = 0xff & coder.properties[0] as u32;
    if (dict_size_bits & (!0x3f)) != 0 {
        return Err(Error::other("Unsupported LZMA2 property bits"));
    }
    if dict_size_bits > 40 {
        return Err(Error::other("Dictionary larger than 4GiB maximum size"));
    }
    if dict_size_bits == 40 {
        return Ok(0xFFFFFFFF);
    }
    let size = (2 | (dict_size_bits & 0x1)) << (dict_size_bits / 2 + 11);
    Ok(size)
}

#[inline]
fn get_lzma_dic_size(coder: &Coder) -> Result<u32, Error> {
    let mut props = &coder.properties[1..5];
    props.read_u32::<LittleEndian>().map_err(Error::io)
}

#[cfg(all(test, feature = "additional-codecs"))]
mod additional_codec_tests {
    use super::*;

    fn ppmd(properties: Vec<u8>) -> Coder {
        let mut coder = Coder::default();
        coder.id_size = 3;
        coder.properties = properties;
        coder.decompression_method_id_mut().copy_from_slice(SevenZMethod::ID_PPMD);
        coder
    }

    #[test]
    fn ppmd_bad_property_lengths_do_not_panic_or_allocate() {
        for size in [0, 1, 4, 6] {
            assert!(matches!(add_decoder(&b""[..], 10, &ppmd(vec![0; size]), &[], 1024), Err(Error::Other(_))));
        }
    }

    #[test]
    fn ppmd_memory_limit_is_checked_before_decoder_initialization() {
        let coder = ppmd(vec![6, 0, 0, 32, 0]);
        assert!(matches!(add_decoder(&b""[..], 10, &coder, &[], 1024),
            Err(Error::MaxMemLimited { max_kb: 1024, actaul_kb: 2048 })));
    }

    #[test]
    fn ppmd_invalid_order_and_arena_are_errors() {
        for props in [vec![0, 0, 0, 16, 0], vec![6, 0, 0, 0, 0]] {
            assert!(matches!(add_decoder(&b""[..], 10, &ppmd(props), &[], 32768), Err(Error::Other(_))));
        }
    }
}
