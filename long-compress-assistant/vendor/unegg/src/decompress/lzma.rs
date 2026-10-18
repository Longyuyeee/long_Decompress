use std::io::{Cursor, Read, Write};

use crate::crypto::Decryptor;
use crate::error::{EggError, EggResult};

pub fn extract_lzma<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    compressed_size: u64,
    max_output: u64,
    mut crypto: Option<&mut dyn Decryptor>,
) -> EggResult<u32> {
    if compressed_size < 9 {
        return Err(EggError::LzmaFailed(
            "block too small for LZMA header".into(),
        ));
    }

    // Read EGG's 9-byte LZMA header
    let mut header = [0u8; 9];
    reader.read_exact(&mut header)?;
    if let Some(ref mut c) = crypto {
        c.decrypt(&mut header);
    }

    // Bytes 0..4: reserved (discard)
    // Bytes 4..9: LZMA properties
    let lzma_props = &header[4..9];

    // Read remaining compressed data, growing from the bytes actually present
    // so a bogus compressed_size can't trigger a huge up-front allocation.
    let data_size = compressed_size - 9;
    let mut compressed_data = Vec::new();
    if (&mut *reader)
        .take(data_size)
        .read_to_end(&mut compressed_data)? as u64
        != data_size
    {
        return Err(EggError::LzmaFailed("truncated LZMA block".into()));
    }
    if let Some(ref mut c) = crypto {
        c.decrypt(&mut compressed_data);
    }

    // Build standard LZMA header: 5 props + 8 bytes uncompressed size (-1 = unknown)
    let mut full_stream = Vec::with_capacity(13 + compressed_data.len());
    full_stream.extend_from_slice(lzma_props);
    full_stream.extend_from_slice(&u64::MAX.to_le_bytes());
    full_stream.extend_from_slice(&compressed_data);

    // Cap lzma-rs's buffer to the declared size; its default memlimit is
    // unlimited, so a crafted stream could otherwise buffer hundreds of MiB
    // before the caller's LimitWriter sees the first byte.
    let memlimit = (max_output.saturating_add(1 << 16)).min(usize::MAX as u64) as usize;
    let options = lzma_rs::decompress::Options {
        unpacked_size: lzma_rs::decompress::UnpackedSize::ReadFromHeader,
        memlimit: Some(memlimit),
        allow_incomplete: false,
    };

    let mut cursor = Cursor::new(full_stream);
    let mut hasher = crc32fast::Hasher::new();
    {
        let mut sink = HashingWriter {
            inner: writer,
            hasher: &mut hasher,
        };
        lzma_rs::lzma_decompress_with_options(&mut cursor, &mut sink, &options)
            .map_err(|e| EggError::LzmaFailed(e.to_string()))?;
    }

    Ok(hasher.finalize())
}

/// Writer adapter that CRC32-hashes bytes as they pass through to `inner`.
struct HashingWriter<'a, W: Write> {
    inner: &'a mut W,
    hasher: &'a mut crc32fast::Hasher,
}

impl<W: Write> Write for HashingWriter<'_, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
