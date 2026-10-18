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
            break;
        }

        if in_avail == 0 && remaining == 0 && produced == 0 {
            // Long adapter: CRC/size alone cannot prove a terminated stream.
            return Err(AlzError::Io(std::io::Error::from(std::io::ErrorKind::UnexpectedEof)));
        }
    }

    Ok(hasher.finalize())
}
