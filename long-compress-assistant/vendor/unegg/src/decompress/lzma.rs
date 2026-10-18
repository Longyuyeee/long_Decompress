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

    if cursor.position() != cursor.get_ref().len() as u64 {
        return Err(EggError::LzmaFailed("trailing data after LZMA end".into()));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, io, rc::Rc};

    fn sample() -> (Vec<u8>, Vec<u8>) {
        let mut state = 7u32;
        let data: Vec<u8> = (0..256 * 1024).map(|_| { state ^= state << 13; state ^= state >> 17; state ^= state << 5; state as u8 }).collect();
        let mut compressed = Vec::new();
        lzma_rs::lzma_compress(&mut Cursor::new(&data), &mut compressed).unwrap();
        // The literal-only test encoder does not emit backward distances.
        // Small dictionary forces an observable output flush before input EOF.
        compressed[1..5].copy_from_slice(&4096u32.to_le_bytes());
        let mut egg = vec![0; 4]; egg.extend_from_slice(&compressed[..5]); egg.extend_from_slice(&compressed[13..]);
        (data, egg)
    }
    struct Observed { input: Cursor<Vec<u8>>, read: Rc<Cell<usize>>, chunk: usize }
    impl Read for Observed {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> { let n=b.len().min(self.chunk); let n=self.input.read(&mut b[..n])?; self.read.set(self.read.get()+n); Ok(n) }
    }
    struct FirstOutput { output: Vec<u8>, read: Rc<Cell<usize>>, first: Option<usize> }
    impl Write for FirstOutput {
        fn write(&mut self,b:&[u8])->io::Result<usize>{self.first.get_or_insert(self.read.get()); self.output.extend_from_slice(b); Ok(b.len())}
        fn flush(&mut self)->io::Result<()>{Ok(())}
    }
    #[test]
    fn output_starts_before_whole_compressed_input_is_read() {
        let(data,egg)=sample();let size=egg.len();let read=Rc::new(Cell::new(0));
        let mut input=Observed{input:Cursor::new(egg),read:read.clone(),chunk:777};
        let mut output=FirstOutput{output:vec![],read,first:None};
        let crc=extract_lzma(&mut input,&mut output,size as u64,data.len() as u64,None).unwrap();
        assert_eq!(crc,crc32fast::hash(&data));assert_eq!(output.output,data);
        assert!(output.first.unwrap()<size/2,"first output consumed all {size} input bytes");
    }
    #[test]
    fn short_reads_and_zipcrypto_keep_cipher_state_across_header_and_chunks() {
        let(data,mut egg)=sample();crate::crypto::ZipCrypto::new(b"public-test-key").encrypt(&mut egg);let size=egg.len();
        let mut input=Observed{input:Cursor::new(egg),read:Rc::new(Cell::new(0)),chunk:7};
        let mut key=crate::crypto::ZipCrypto::new(b"public-test-key");let mut output=vec![];
        assert_eq!(extract_lzma(&mut input,&mut output,size as u64,data.len() as u64,Some(&mut key)).unwrap(),crc32fast::hash(&data));assert_eq!(output,data);
    }
    #[test]
    fn extra_input_and_real_truncation_are_not_success() {
        let(data,egg)=sample();let size=egg.len();
        for n in [1,4,9,size/2] { let mut out=vec![];assert!(extract_lzma(&mut Cursor::new(&egg[..size-n]),&mut out,size as u64,data.len() as u64,None).is_err()); }
        let mut extra=egg.clone();extra.extend_from_slice(b"unused tail");let n=extra.len();
        assert!(extract_lzma(&mut Cursor::new(extra),&mut io::sink(),n as u64,data.len() as u64,None).is_err());
    }
    #[test]
    fn output_io_kind_is_preserved_for_final_classification() {
        struct Denied;impl Write for Denied {fn write(&mut self,_:&[u8])->io::Result<usize>{Err(io::Error::from(io::ErrorKind::PermissionDenied))}fn flush(&mut self)->io::Result<()>{Ok(())}}
        let(data,egg)=sample();let n=egg.len();let err=extract_lzma(&mut Cursor::new(egg),&mut Denied,n as u64,data.len() as u64,None).unwrap_err();
        assert!(matches!(err,EggError::Io(ref e) if e.kind()==io::ErrorKind::PermissionDenied),"{err:?}");
    }
}
