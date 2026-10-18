use std::io::{Read, Write};

use crate::crypto::Decryptor;
use crate::error::{EggError, EggResult};

pub fn extract_azo<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    compressed_size: u64,
    max_output: u64,
    crypto: Option<&mut dyn Decryptor>,
) -> EggResult<u32> {
    let mut decrypt_fn;
    let callback: Option<libazo::DecryptFn<'_>> = match crypto {
        Some(c) => {
            decrypt_fn = |data: &mut [u8]| c.decrypt(data);
            Some(&mut decrypt_fn)
        }
        None => None,
    };
    // max_output lets azo reject an over-large declared block up front; the
    // LimitWriter only catches output once it has already been produced.
    libazo::extract_azo(reader, writer, compressed_size, Some(max_output), callback)
        .map_err(|e| match e {libazo::AzoError::Io(e)=>EggError::Io(e),other=>EggError::AzoFailed(other.to_string())})
}
