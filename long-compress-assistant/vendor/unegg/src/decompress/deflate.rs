use std::io::{Read, Write};

use flate2::{Decompress, FlushDecompress, Status};

use crate::crypto::Decryptor;
use crate::error::{EggError, EggResult};

const IN_BUF_SIZE: usize = 4096;
const OUT_BUF_SIZE: usize = 4096;

pub fn extract_deflate<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    compressed_size: u64,
    crypto: Option<&mut dyn Decryptor>,
) -> EggResult<u32> {
    let mut hasher = crc32fast::Hasher::new();
    let mut in_buf = [0u8; IN_BUF_SIZE];
    let mut out_buf = [0u8; OUT_BUF_SIZE];
    let mut decompressor = Decompress::new(false);
    let mut remaining = compressed_size;
    let mut in_avail = 0usize;
    let mut crypto = crypto;

    loop {
        if in_avail == 0 && remaining > 0 {
            let to_read = (remaining as usize).min(IN_BUF_SIZE);
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
            .map_err(|e| EggError::InflateFailed(e.to_string()))?;

        let consumed = (decompressor.total_in() - before_in) as usize;
        let produced = (decompressor.total_out() - before_out) as usize;

        if consumed < in_avail {
            in_buf.copy_within(consumed..in_avail, 0);
        }
        in_avail -= consumed;

        if produced > 0 {
            hasher.update(&out_buf[..produced]);
            writer.write_all(&out_buf[..produced])?;
        }

        if status == Status::StreamEnd {
            if remaining != 0 || in_avail != 0 {
                return Err(EggError::InflateFailed("trailing data after Deflate end".into()));
            }
            break;
        }

        if in_avail == 0 && remaining == 0 && produced == 0 {
            return Err(EggError::InflateFailed("truncated Deflate stream (no StreamEnd)".into()));
        }
        if consumed == 0 && produced == 0 {
            return Err(EggError::InflateFailed("Deflate decoder made no progress".into()));
        }
    }

    Ok(hasher.finalize())
}
