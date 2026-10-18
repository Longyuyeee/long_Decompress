//! Raw DEFLATE extraction via flate2.

use std::io::{Read, Write};

use flate2::{Decompress, FlushDecompress, Status};

use crate::crypto::ZipCrypto;
use crate::error::{AlzError, AlzResult};

const IN_BUF_SIZE: usize = 32768;
const OUT_BUF_SIZE: usize = 32768;

/// Extract a raw DEFLATE stream (no zlib/gzip header), erroring if more than
/// `max_output` bytes would be produced. Returns the CRC32.
pub fn extract_deflate<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    compressed_size: u64,
    max_output: u64,
    mut crypto: Option<&mut ZipCrypto>,
) -> AlzResult<u32> {
    let mut hasher = crc32fast::Hasher::new();
    let mut in_buf = [0u8; IN_BUF_SIZE];
    let mut out_buf = [0u8; OUT_BUF_SIZE];
    let mut decompressor = Decompress::new(false); // raw deflate (no header)
    let mut remaining = compressed_size;
    let mut in_avail = 0usize; // unconsumed bytes at front of in_buf
    let mut produced_total: u64 = 0;

    loop {
        // Refill once the input buffer is drained.
        if in_avail == 0 && remaining > 0 {
            // Clamp before narrowing to usize (32-bit truncation guard).
            let to_read = remaining.min(IN_BUF_SIZE as u64) as usize;
            reader.read_exact(&mut in_buf[..to_read])?;
            if let Some(ref mut c) = crypto {
                c.decrypt(&mut in_buf[..to_read]);
            }
            remaining -= to_read as u64;
            in_avail = to_read;
        }

        let before_in = decompressor.total_in();
        let before_out = decompressor.total_out();

        let status = decompressor
            .decompress(&in_buf[..in_avail], &mut out_buf, FlushDecompress::Sync)
            .map_err(|e| AlzError::InflateFailed(e.to_string()))?;

        let consumed = (decompressor.total_in() - before_in) as usize;
        let produced = (decompressor.total_out() - before_out) as usize;

        // Shift unconsumed input to front of buffer.
        if consumed < in_avail {
            in_buf.copy_within(consumed..in_avail, 0);
        }
        in_avail -= consumed;

        if produced > 0 {
            produced_total += produced as u64;
            if produced_total > max_output {
                return Err(AlzError::UncompressedSizeExceeded { limit: max_output });
            }
            hasher.update(&out_buf[..produced]);
            writer
                .write_all(&out_buf[..produced])
                .map_err(AlzError::CantOpenDestFile)?;
        }

        if status == Status::StreamEnd {
            let trailing = remaining + in_avail as u64;
            if trailing != 0 {
                return Err(AlzError::TrailingCompressedData { remaining: trailing });
            }
            break;
        }

        if in_avail == 0 && remaining == 0 && produced == 0 {
            // Long adapter: CRC/size alone cannot prove a terminated stream.
            return Err(AlzError::Io(std::io::Error::from(std::io::ErrorKind::UnexpectedEof)));
        }
    }

    Ok(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Cursor};
    fn packed(bytes:&[u8])->Vec<u8>{let mut e=flate2::write::DeflateEncoder::new(Vec::new(),flate2::Compression::best());e.write_all(bytes).unwrap();e.finish().unwrap()}
    #[test]
    fn exact_streams_include_empty_output_and_multiple_output_chunks(){
        for bytes in [Vec::new(),b"Original ALZ stream. ".repeat(10000)] {let b=packed(&bytes);let mut out=Vec::new();assert_eq!(extract_deflate(&mut Cursor::new(&b),&mut out,b.len() as u64,bytes.len() as u64,None).unwrap(),crc32fast::hash(&bytes));assert_eq!(out,bytes);}
    }
    #[test]
    fn rejects_buffered_and_not_yet_read_trailing_bytes(){
        let bytes=b"Deflate end marker is not the declared input end. ".repeat(100);let b=packed(&bytes);
        for tail in [1,5,65536] {let mut c=b.clone();c.resize(c.len()+tail,0xa5);let mut out=Vec::new();let e=extract_deflate(&mut Cursor::new(&c),&mut out,c.len() as u64,bytes.len() as u64,None).unwrap_err();assert!(matches!(e,AlzError::TrailingCompressedData{remaining} if remaining==tail as u64));assert_eq!(out,bytes);}
    }
    #[test]
    fn rejects_declared_truncation_even_if_crc_might_match(){let bytes=b"Incomplete terminator. ".repeat(100);let mut b=packed(&bytes);b.pop();assert!(extract_deflate(&mut Cursor::new(&b),&mut Vec::new(),b.len() as u64,bytes.len() as u64,None).is_err());}
    #[test]
    fn does_not_read_past_declared_entry_into_the_next_header(){let bytes=b"Exact boundary.".repeat(100);let b=packed(&bytes);let mut input=Cursor::new([b.clone(),b"BLZ\x01next header".to_vec()].concat());extract_deflate(&mut input,&mut Vec::new(),b.len() as u64,bytes.len() as u64,None).unwrap();assert_eq!(input.position(),b.len() as u64);}
    #[test]
    fn output_size_limit_is_not_relaxed(){let bytes=b"Size limit.".repeat(500);let b=packed(&bytes);assert!(matches!(extract_deflate(&mut Cursor::new(&b),&mut Vec::new(),b.len() as u64,1,None),Err(AlzError::UncompressedSizeExceeded{..})));}
    struct FailedRead;
    impl Read for FailedRead{fn read(&mut self,_:&mut[u8])->io::Result<usize>{Err(io::Error::from(io::ErrorKind::PermissionDenied))}}
    struct FailedWrite;
    impl Write for FailedWrite{fn write(&mut self,_:&[u8])->io::Result<usize>{Err(io::Error::from(io::ErrorKind::BrokenPipe))}fn flush(&mut self)->io::Result<()>{Ok(())}}
    #[test]
    fn preserves_read_io_kind(){assert!(matches!(extract_deflate(&mut FailedRead,&mut Vec::new(),10,10,None),Err(AlzError::Io(e)) if e.kind()==io::ErrorKind::PermissionDenied));}
    #[test]
    fn preserves_output_broken_pipe(){let b=packed(b"Output error.");let e=extract_deflate(&mut Cursor::new(&b),&mut FailedWrite,b.len() as u64,100,None).unwrap_err();assert!(e.is_broken_pipe());}
}
