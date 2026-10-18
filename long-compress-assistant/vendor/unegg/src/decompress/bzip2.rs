use std::io::{Read, Write};

use crate::crypto::Decryptor;
use crate::error::{EggError, EggResult};

const BUF_SIZE: usize = 8192;

pub fn extract_bzip2<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    compressed_size: u64,
    crypto: Option<&mut dyn Decryptor>,
) -> EggResult<u32> {
    // Bzip2 needs the whole compressed stream. If encrypted, decrypt first.
    let bounded = reader.take(compressed_size);
    let source: Box<dyn Read> = if let Some(c) = crypto {
        Box::new(DecryptingReader::new(bounded, c))
    } else {
        Box::new(bounded)
    };

    let mut decoder = bzip2::read::BzDecoder::new(source);

    let mut hasher = crc32fast::Hasher::new();
    let mut buf = [0u8; BUF_SIZE];

    loop {
        let n = decoder
            .read(&mut buf)
            .map_err(|e| EggError::Bzip2Failed(e.to_string()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        writer.write_all(&buf[..n])?;
    }

    if decoder.total_in() != compressed_size {
        return Err(EggError::Bzip2Failed("trailing data after Bzip2 end".into()));
    }
    Ok(hasher.finalize())
}

/// Adapter that decrypts data on the fly as it's read.
struct DecryptingReader<'a, R: Read> {
    inner: R,
    crypto: &'a mut dyn Decryptor,
}

impl<'a, R: Read> DecryptingReader<'a, R> {
    fn new(inner: R, crypto: &'a mut dyn Decryptor) -> Self {
        Self { inner, crypto }
    }
}

impl<R: Read> Read for DecryptingReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n > 0 {
            self.crypto.decrypt(&mut buf[..n]);
        }
        Ok(n)
    }
}
