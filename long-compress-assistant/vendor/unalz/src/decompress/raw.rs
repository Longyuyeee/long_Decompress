//! Store (uncompressed) extraction.

use std::io::{Read, Write};

use crate::crypto::ZipCrypto;
use crate::error::{AlzError, AlzResult};

const BUF_SIZE: usize = 32768;

/// Copy `size` stored bytes to `writer`, optionally decrypting, erroring if
/// more than `max_output` bytes would be produced. Returns the CRC32.
pub fn extract_raw<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    size: u64,
    max_output: u64,
    mut crypto: Option<&mut ZipCrypto>,
) -> AlzResult<u32> {
    let mut hasher = crc32fast::Hasher::new();
    let mut buf = [0u8; BUF_SIZE];
    let mut remaining = size;
    let mut produced: u64 = 0;

    while remaining > 0 {
        // Clamp against a u64 bound before narrowing to usize; a bare
        // `remaining as usize` would truncate on 32-bit targets and could
        // yield 0 (infinite loop) when the low 32 bits are zero.
        let to_read = remaining.min(BUF_SIZE as u64) as usize;
        reader.read_exact(&mut buf[..to_read])?;

        produced += to_read as u64;
        if produced > max_output {
            return Err(AlzError::UncompressedSizeExceeded { limit: max_output });
        }

        let data = &mut buf[..to_read];
        if let Some(ref mut c) = crypto {
            c.decrypt(data);
        }

        hasher.update(data);
        writer.write_all(data).map_err(AlzError::CantOpenDestFile)?;
        remaining -= to_read as u64;
    }

    Ok(hasher.finalize())
}
