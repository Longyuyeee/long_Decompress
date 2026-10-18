use std::io::{Read, Write};

use crate::crypto::Decryptor;
use crate::error::EggResult;

const BUF_SIZE: usize = 8192;

pub fn extract_store<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    size: u64,
    crypto: Option<&mut dyn Decryptor>,
) -> EggResult<u32> {
    let mut hasher = crc32fast::Hasher::new();
    let mut buf = [0u8; BUF_SIZE];
    let mut remaining = size;
    let mut crypto = crypto;

    while remaining > 0 {
        let to_read = (remaining as usize).min(BUF_SIZE);
        reader.read_exact(&mut buf[..to_read])?;
        if let Some(ref mut c) = crypto {
            c.decrypt(&mut buf[..to_read]);
        }
        hasher.update(&buf[..to_read]);
        writer.write_all(&buf[..to_read])?;
        remaining -= to_read as u64;
    }

    Ok(hasher.finalize())
}
