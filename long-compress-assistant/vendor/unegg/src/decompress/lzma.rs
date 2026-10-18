use std::io::{BufReader, Cursor, Read, Take, Write};

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
    let dictionary = u32::from_le_bytes(header[5..9].try_into().unwrap()).max(4096);
    // Bound decoder history, not the total length of an otherwise valid stream.
    // Large input must never create an input-sized Vec or a second full copy.
    const MAX_DICTIONARY: u32 = 256 * 1024 * 1024;
    if dictionary > MAX_DICTIONARY {
        return Err(EggError::UnsupportedFeature("EGG LZMA 字典超过 256 MiB".into()));
    }
    let mut standard_header = [0u8; 13];
    standard_header[..5].copy_from_slice(&header[4..9]);
    standard_header[5..].copy_from_slice(&u64::MAX.to_le_bytes());
    let payload = DecryptedInput { inner: reader.take(compressed_size - 9), crypto, error: None };
    let mut input = BufReader::with_capacity(64 * 1024, Cursor::new(standard_header).chain(payload));
    let memlimit = max_output.saturating_add(1 << 16).min(MAX_DICTIONARY as u64) as usize;
    let options = lzma_rs::decompress::Options {
        unpacked_size: lzma_rs::decompress::UnpackedSize::ReadFromHeader,
        memlimit: Some(memlimit),
        allow_incomplete: false,
    };

    let mut hasher = crc32fast::Hasher::new();
    {
        let mut sink = HashingWriter {
            inner: writer,
            hasher: &mut hasher,
        };
        let mut limited = super::LimitWriter::new(&mut sink, max_output);
        let result = lzma_rs::lzma_decompress_with_options(&mut input, &mut limited, &options);
        // lzma-rs turns range-decoder input errors into LzmaError strings.
        // Keep the actual error at the input boundary, not a text heuristic.
        if let Some(error) = input.get_mut().get_mut().1.error.take() { return Err(EggError::Io(error)); }
        result.map_err(|e| match e {
                lzma_rs::error::Error::IoError(e) | lzma_rs::error::Error::HeaderTooShort(e) => EggError::Io(e),
                e => EggError::LzmaFailed(e.to_string()),
            })?;
    }

    // BufRead may prefetch past the end marker. Count unread buffered bytes too;
    // Take reaching zero alone is not evidence that the decoder consumed them.
    if !input.buffer().is_empty() || input.get_ref().get_ref().1.inner.limit() != 0 {
        return Err(EggError::LzmaFailed("unused or truncated data after LZMA end".into()));
    }
    Ok(hasher.finalize())
}

struct DecryptedInput<'a, R: Read> {
    inner: Take<R>,
    crypto: Option<&'a mut dyn Decryptor>,
    error: Option<std::io::Error>,
}
impl<R: Read> Read for DecryptedInput<'_, R> {
    fn read(&mut self, data: &mut [u8]) -> std::io::Result<usize> {
        let size = data.len().min(64 * 1024);
        let n = match self.inner.read(&mut data[..size]) {
            Ok(n) => n,
            Err(error) => { let forwarded = std::io::Error::new(error.kind(), error.to_string()); self.error = Some(error); return Err(forwarded); }
        };
        if let Some(cipher) = self.crypto.as_mut() { cipher.decrypt(&mut data[..n]); }
        Ok(n)
    }
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
    #[test]
    fn dictionary_and_output_limits_remain_bounded() {
        let(data,mut egg)=sample();let n=egg.len();
        assert!(extract_lzma(&mut Cursor::new(&egg),&mut io::sink(),n as u64,1,None).is_err());
        egg[5..9].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(extract_lzma(&mut Cursor::new(egg),&mut io::sink(),n as u64,data.len() as u64,None),Err(EggError::UnsupportedFeature(_))));
    }
    #[test]
    fn input_io_kind_is_preserved_for_final_classification() {
        struct DeniedAfterHeader { input: Cursor<Vec<u8>> }
        impl Read for DeniedAfterHeader { fn read(&mut self,b:&mut[u8])->io::Result<usize>{if self.input.position()>=9 {Err(io::Error::from(io::ErrorKind::PermissionDenied))}else{let n=b.len().min(9-self.input.position() as usize);self.input.read(&mut b[..n])}} }
        let(data,egg)=sample();let n=egg.len();let err=extract_lzma(&mut DeniedAfterHeader{input:Cursor::new(egg)},&mut io::sink(),n as u64,data.len() as u64,None).unwrap_err();
        assert!(matches!(err,EggError::Io(ref e) if e.kind()==io::ErrorKind::PermissionDenied),"{err:?}");
    }
}
