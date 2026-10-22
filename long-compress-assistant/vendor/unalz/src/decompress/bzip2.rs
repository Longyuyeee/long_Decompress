//! ALZ-modified bzip2 extraction.
//!
//! ALZ bzip2 differs from a standard bzip2 stream:
//!
//! - the `BZh9` stream header is absent (block size is hardcoded to 9);
//! - each block is introduced by the 4-byte marker `DLZ\x01` instead of the
//!   48-bit `0x314159265359`, and the stream ends with `DLZ\x02` in place of
//!   `0x177245385090` plus the combined CRC;
//! - the per-block CRC and the randomised bit are omitted;
//! - the block payload (origPtr + Huffman/MTF/BWT data) is otherwise identical.
//!
//! Each block is rewrapped as a standalone single-block standard stream
//! (`BZh9` + block magic + faked zero CRC + randomised = 0 + payload + EOS +
//! faked combined CRC) and decoded with the `bzip2` crate.
//!
//! Block boundaries are found by decoding rather than by scanning. The 4-byte
//! `DLZ` marker can occur by coincidence inside a block's compressed payload,
//! so a naive scan for the next marker mis-splits multi-block streams. A
//! candidate cut before a block's true end yields a truncated block that
//! produces no output (bzip2 only emits after the whole block's inverse-BWT),
//! so the first candidate cut that produces output is the real boundary — the
//! same end-of-block signal the reference decoder uses.

use std::io::{self, Read, Write};

use crate::crypto::ZipCrypto;
use crate::error::{AlzError, AlzResult};

/// ALZ bzip2 block header "DLZ\x01" and end-of-stream "DLZ\x02", as big-endian u32.
const ALZ_BLOCK_MAGIC_U32: u32 = 0x444C5A01;
const ALZ_EOS_MAGIC_U32: u32 = 0x444C5A02;

/// Standard bzip2 stream header: "BZh9"
const BZ_STREAM_HEADER: [u8; 4] = *b"BZh9";
/// Standard bzip2 block magic (48 bits, big-endian): pi digits 0x314159265359
const BZ_BLOCK_MAGIC: [u8; 6] = [0x31, 0x41, 0x59, 0x26, 0x53, 0x59];
/// Standard bzip2 end-of-stream magic (48 bits): sqrt(pi) digits 0x177245385090
const BZ_EOS_MAGIC: [u8; 6] = [0x17, 0x72, 0x45, 0x38, 0x50, 0x90];

/// Build a standalone standard single-block bzip2 stream from one ALZ block's
/// payload bits, inserting `randomised = 0` before origPtr (which shifts the
/// subsequent block bits).
fn build_block_probe(payload: &[u8], start_bit: usize, end_bit: usize) -> Vec<u8> {
    let mut w = BitWriter::new();
    w.write_bytes(&BZ_STREAM_HEADER);
    for &b in &BZ_BLOCK_MAGIC {
        w.write_bits(b as u32, 8);
    }
    w.write_bits(0, 32); // fake block CRC
    w.write_bits(0, 1); // randomised = 0

    let mut r = BitReader::at(payload, start_bit);
    let mut remaining = end_bit - start_bit;
    while remaining > 0 {
        let n = remaining.min(24);
        w.write_bits(r.read_bits(n).unwrap_or(0), n);
        remaining -= n;
    }

    for &b in &BZ_EOS_MAGIC {
        w.write_bits(b as u32, 8);
    }
    w.write_bits(0, 32); // fake combined CRC
    w.flush();
    w.into_bytes()
}

/// Decode one standalone standard bzip2 stream into `out`, capped at `cap`
/// bytes. Returns the number of bytes produced (0 if the block is truncated /
/// could not be completed).
fn decode_probe(std_stream: &[u8], out: &mut Vec<u8>, cap: u64) -> AlzResult<u64> {
    let mut decompressor = bzip2::Decompress::new(false);
    let mut input_pos = 0;
    let mut tmp = [0u8; 32768];
    let mut produced_total: u64 = 0;

    loop {
        let before_in = decompressor.total_in();
        let before_out = decompressor.total_out();
        let result = decompressor.decompress(&std_stream[input_pos..], &mut tmp);
        let consumed = (decompressor.total_in() - before_in) as usize;
        let produced = (decompressor.total_out() - before_out) as usize;
        input_pos += consumed;

        if produced > 0 {
            produced_total += produced as u64;
            if produced_total > cap {
                return Err(AlzError::UncompressedSizeExceeded { limit: cap });
            }
            out.extend_from_slice(&tmp[..produced]);
        }

        match result {
            Ok(bzip2::Status::StreamEnd) => break,
            // The faked zero block CRC makes the stock decoder return a data
            // error once a block is fully decoded and emitted; that is our
            // signal the block completed.
            Err(_) => break,
            // Any in-progress status: stop on no progress (input exhausted with
            // nothing more to emit), guarding against an infinite spin.
            Ok(_) => {
                if consumed == 0 && produced == 0 {
                    break;
                }
            }
        }
    }

    Ok(produced_total)
}

// A BZh9 block contains at most 900,000 RLE/MTF symbols, each using at
// most 20 Huffman bits, plus selectors and tables. 4 MiB leaves ample room
// for that encoded block and lookahead, independent of the entry length.
const MAX_BLOCK_INPUT: usize = 4 * 1024 * 1024;
const INPUT_CHUNK: usize = 32 * 1024;

fn read_chunk<R: Read>(input: &mut io::Take<R>, bytes: &mut [u8]) -> AlzResult<usize> {
    loop {
        match input.read(bytes) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(AlzError::Io(error)),
            Ok(0) if input.limit() != 0 => return Err(AlzError::Io(io::ErrorKind::UnexpectedEof.into())),
            Ok(size) => return Ok(size),
        }
    }
}

fn fill_window<R: Read>(input: &mut io::Take<R>, data: &mut Vec<u8>, crypto: &mut Option<&mut ZipCrypto>) -> AlzResult<bool> {
    if input.limit() == 0 { return Ok(false); }
    let room = MAX_BLOCK_INPUT.saturating_sub(data.len()).min(INPUT_CHUNK);
    if room == 0 { return Err(AlzError::Bzip2Failed("bzip2 block exceeds bounded input window".into())); }
    let mut chunk = [0u8; INPUT_CHUNK];
    let size = read_chunk(input, &mut chunk[..room])?;
    if let Some(cipher) = crypto { cipher.decrypt(&mut chunk[..size]); }
    data.extend_from_slice(&chunk[..size]);
    Ok(size != 0)
}

/// Extract using one bounded compressed block window, retaining the partial
/// byte at each boundary. Cipher state continues across reads. Success requires
/// the actual EOS and declared input boundary, and returns the output CRC32.
pub fn extract_bzip2<R: Read, W: Write>(
    reader: &mut R, writer: &mut W, compressed_size: u64, max_output: u64,
    mut crypto: Option<&mut ZipCrypto>,
) -> AlzResult<u32> {
    const MAX_CANDIDATES_PER_BLOCK: u32 = 4096;
    let mut input = reader.take(compressed_size);
    let mut data = Vec::with_capacity(INPUT_CHUNK * 2);
    let mut hasher = crc32fast::Hasher::new();
    let mut produced_total = 0u64;
    let mut pos = 0usize;
    loop {
        let marker = loop {
            if let Some(marker) = peek32(&data, pos) { break marker; }
            if !fill_window(&mut input, &mut data, &mut crypto)? { return Err(AlzError::Bzip2MissingEnd); }
        };
        pos += 32;
        if marker == ALZ_EOS_MAGIC_U32 {
            let remaining = ((data.len() * 8 - pos) / 8) as u64 + input.limit();
            // Drain without retaining the tail: real short reads / I/O errors
            // remain distinguishable. Never read beyond this entry.
            let mut tail = [0u8; INPUT_CHUNK];
            while input.limit() != 0 {
                let size = read_chunk(&mut input, &mut tail)?;
                if let Some(cipher) = &mut crypto { cipher.decrypt(&mut tail[..size]); }
            }
            if remaining != 0 { return Err(AlzError::TrailingCompressedData { remaining }); }
            return Ok(hasher.finalize());
        }
        if marker != ALZ_BLOCK_MAGIC_U32 {
            return Err(AlzError::Bzip2Failed(format!("expected ALZ block header, got {marker:08x}")));
        }
        let block_start = pos;
        let mut search_from = block_start;
        let mut attempts = 0u32;
        let mut probe_bytes = 0u64;
        let mut block_out = Vec::new();
        let block_end = loop {
            let cand = if let Some(cand) = next_marker(&data, search_from) { cand }
            else {
                let old_bits = data.len() * 8;
                if fill_window(&mut input, &mut data, &mut crypto)? {
                    search_from = search_from.max(old_bits.saturating_sub(31));
                    continue;
                }
                old_bits
            };
            attempts += 1;
            probe_bytes += (cand - block_start).div_ceil(8) as u64;
            // Budget each block against actual buffered input, not a large
            // untrusted declaration. False markers cannot buy unlimited work.
            if attempts > MAX_CANDIDATES_PER_BLOCK || probe_bytes > (data.len() as u64 * 32).max(1 << 20) {
                return Err(AlzError::Bzip2Failed("bzip2 boundary search exceeded work budget".into()));
            }
            block_out.clear();
            let produced = decode_probe(&build_block_probe(&data, block_start, cand), &mut block_out, max_output.saturating_sub(produced_total))?;
            if produced > 0 { break cand; }
            if cand >= data.len() * 8 { return Err(AlzError::Bzip2Failed("could not decode bzip2 block".into())); }
            search_from = cand + 1;
        };
        produced_total += block_out.len() as u64;
        hasher.update(&block_out);
        writer.write_all(&block_out).map_err(AlzError::CantOpenDestFile)?;
        data.drain(..block_end / 8);
        pos = block_end % 8;
    }
}

/// Read the 32 bits starting at `bit` as a big-endian u32, or None if fewer
/// than 32 bits remain.
fn peek32(data: &[u8], bit: usize) -> Option<u32> {
    if bit + 32 > data.len() * 8 {
        return None;
    }
    let mut v = 0u32;
    for i in 0..32 {
        let b = bit + i;
        v = (v << 1) | ((data[b >> 3] >> (7 - (b & 7))) & 1) as u32;
    }
    Some(v)
}

/// First bit offset >= `from` where a 32-bit DLZ block or EOS marker begins.
fn next_marker(data: &[u8], from: usize) -> Option<usize> {
    let nbits = data.len() * 8;
    if nbits < 32 {
        return None;
    }
    let mut window = peek32(data, from)?;
    if window == ALZ_BLOCK_MAGIC_U32 || window == ALZ_EOS_MAGIC_U32 {
        return Some(from);
    }
    for start in (from + 1)..=(nbits - 32) {
        let last = start + 31;
        window = (window << 1) | ((data[last >> 3] >> (7 - (last & 7))) & 1) as u32;
        if window == ALZ_BLOCK_MAGIC_U32 || window == ALZ_EOS_MAGIC_U32 {
            return Some(start);
        }
    }
    None
}

/// MSB-first bit reader.
struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8, // 0-7, 0 = MSB
}

impl<'a> BitReader<'a> {
    fn at(data: &'a [u8], bit: usize) -> Self {
        Self {
            data,
            byte_pos: bit >> 3,
            bit_pos: (bit & 7) as u8,
        }
    }

    fn bits_remaining(&self) -> usize {
        if self.byte_pos >= self.data.len() {
            return 0;
        }
        (self.data.len() - self.byte_pos) * 8 - self.bit_pos as usize
    }

    fn read_bits(&mut self, n: usize) -> AlzResult<u32> {
        if n > 32 || self.bits_remaining() < n {
            return Err(AlzError::Bzip2Failed("unexpected end of bzip2 data".into()));
        }
        let mut val: u32 = 0;
        for _ in 0..n {
            val = (val << 1) | self.read_bit() as u32;
        }
        Ok(val)
    }

    fn read_bit(&mut self) -> u8 {
        let bit = (self.data[self.byte_pos] >> (7 - self.bit_pos)) & 1;
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
        bit
    }
}

/// MSB-first bit writer.
struct BitWriter {
    data: Vec<u8>,
    current: u8,
    bit_pos: u8, // 0-7, 0 = MSB (next bit to write)
}

impl BitWriter {
    fn new() -> Self {
        Self {
            data: Vec::new(),
            current: 0,
            bit_pos: 0,
        }
    }

    fn write_bits(&mut self, val: u32, n: usize) {
        for i in (0..n).rev() {
            let bit = (val >> i) & 1;
            self.current |= (bit as u8) << (7 - self.bit_pos);
            self.bit_pos += 1;
            if self.bit_pos == 8 {
                self.data.push(self.current);
                self.current = 0;
                self.bit_pos = 0;
            }
        }
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_bits(b as u32, 8);
        }
    }

    fn flush(&mut self) {
        if self.bit_pos > 0 {
            self.data.push(self.current);
            self.current = 0;
            self.bit_pos = 0;
        }
    }

    fn into_bytes(self) -> Vec<u8> {
        self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // Byte-preserved upstream CC0 ALZip original, already tracked by v1.5.2.
    fn original() -> Vec<u8> {
        include_bytes!("../../../../tests/fixtures/archives/alz-152/bzip2.alz")[36..159].to_vec()
    }
    fn expected() -> Vec<u8> { b"ABCDEFG the quick brown fox 0123456789\n".repeat(60) }
    fn append_bits(writer: &mut BitWriter, data: &[u8], count: usize) {
        let mut reader = BitReader::at(data, 0);
        let mut left = count;
        while left > 0 {
            let n = left.min(24);
            writer.write_bits(reader.read_bits(n).unwrap(), n);
            left -= n;
        }
    }

    #[test]
    fn bzip2_rejects_missing_end_and_trailing_bytes_after_complete_content() {
        let bytes = original();
        let end = next_marker(&bytes, 32).unwrap();
        assert_eq!(peek32(&bytes, end), Some(ALZ_EOS_MAGIC_U32));
        let mut missing = BitWriter::new();
        append_bits(&mut missing, &bytes, end);
        missing.flush();
        let mut faults = vec![missing.into_bytes()];
        for tail in [1, 5, 65536] {
            let mut trailing = bytes.clone();
            trailing.resize(trailing.len() + tail, 0xa5);
            faults.push(trailing);
        }
        for fault in faults {
            let mut out = Vec::new();
            let result = extract_bzip2(&mut Cursor::new(&fault), &mut out, fault.len() as u64, expected().len() as u64, None);
            assert_eq!(out, expected(), "The defect is after complete content, not a bad CRC");
            assert!(result.is_err(), "A missing end marker or trailing data must not verify successfully");
        }
    }

    #[test]
    fn bzip2_reads_only_declared_input_and_reports_real_short_reads() {
        let bytes = original();
        let mut with_next_entry = bytes.clone();
        with_next_entry.extend_from_slice(b"next archive entry must remain unread");
        let mut input = Cursor::new(with_next_entry);
        let mut out = Vec::new();
        extract_bzip2(&mut input, &mut out, bytes.len() as u64, expected().len() as u64, None).unwrap();
        assert_eq!(input.position(), bytes.len() as u64);
        assert_eq!(out, expected());
        let error = extract_bzip2(&mut Cursor::new(&bytes[..bytes.len()-1]), &mut Vec::new(), bytes.len() as u64, 2340, None).unwrap_err();
        assert!(matches!(error, AlzError::Io(e) if e.kind() == io::ErrorKind::UnexpectedEof));
    }

    #[test]
    fn bzip2_original_multiblock_empty_and_alignment_remain_readable() {
        let bytes = original();
        let end = next_marker(&bytes, 32).unwrap();
        let mut joined = BitWriter::new();
        append_bits(&mut joined, &bytes, end);
        append_bits(&mut joined, &bytes, end);
        joined.write_bits(ALZ_EOS_MAGIC_U32, 32);
        joined.flush();
        for (compressed, plain) in [
            (bytes, expected()),
            (joined.into_bytes(), expected().repeat(2)),
            (ALZ_EOS_MAGIC_U32.to_be_bytes().to_vec(), Vec::new()),
        ] {
            let mut out = Vec::new();
            let crc = extract_bzip2(&mut Cursor::new(&compressed), &mut out, compressed.len() as u64, plain.len() as u64, None).unwrap();
            assert_eq!(out, plain);
            assert_eq!(crc, crc32fast::hash(&plain));
        }
    }

    #[test]
    fn bzip2_outputs_before_reading_whole_entry_and_bounds_each_read() {
        use std::{cell::Cell, rc::Rc};
        let bytes = original();
        let end = next_marker(&bytes, 32).unwrap();
        let mut stream = BitWriter::new();
        for _ in 0..1024 { append_bits(&mut stream, &bytes, end); }
        stream.write_bits(ALZ_EOS_MAGIC_U32, 32); stream.flush();
        let compressed = stream.into_bytes();
        let read = Rc::new(Cell::new(0usize));
        struct Measured { cursor: Cursor<Vec<u8>>, read: Rc<Cell<usize>> }
        impl Read for Measured {
            fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
                assert!(bytes.len() <= INPUT_CHUNK);
                let n = self.cursor.read(bytes)?; self.read.set(self.read.get()+n); Ok(n)
            }
        }
        struct Output { read: Rc<Cell<usize>>, first: Option<usize>, crc: crc32fast::Hasher, size: u64 }
        impl Write for Output {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.first.get_or_insert(self.read.get());
                self.crc.update(bytes); self.size += bytes.len() as u64; Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> { Ok(()) }
        }
        let length = compressed.len();
        let mut source = Measured { cursor: Cursor::new(compressed), read: read.clone() };
        let mut out = Output { read, first: None, crc: crc32fast::Hasher::new(), size: 0 };
        let crc = extract_bzip2(&mut source, &mut out, length as u64, 2340*1024, None).unwrap();
        assert_eq!(out.size, 2340*1024);
        assert_eq!(crc, crc32fast::hash(&expected().repeat(1024)));
        assert_eq!(crc, out.crc.finalize());
        assert!(out.first.unwrap() <= INPUT_CHUNK, "Output must precede reading the whole entry");
        assert!(length > INPUT_CHUNK*3);
        let mut invalid = Cursor::new([ALZ_BLOCK_MAGIC_U32.to_be_bytes().to_vec(), vec![0; MAX_BLOCK_INPUT+1]].concat());
        assert!(matches!(extract_bzip2(&mut invalid, &mut io::sink(), u64::MAX, u64::MAX, None), Err(AlzError::Bzip2Failed(_))));
        assert_eq!(invalid.position(), MAX_BLOCK_INPUT as u64);
    }

    #[test]
    fn bzip2_short_chunks_and_io_errors_preserve_the_original_cause() {
        struct Short(Cursor<Vec<u8>>);
        impl Read for Short {
            fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
                let size = bytes.len().min(2);
                self.0.read(&mut bytes[..size])
            }
        }
        struct InputFailure;
        impl Read for InputFailure {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> { Err(io::ErrorKind::PermissionDenied.into()) }
        }
        struct OutputFailure;
        impl Write for OutputFailure {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> { Err(io::ErrorKind::BrokenPipe.into()) }
            fn flush(&mut self) -> io::Result<()> { Ok(()) }
        }
        let bytes = original();
        let mut out = Vec::new();
        extract_bzip2(&mut Short(Cursor::new(bytes.clone())), &mut out, bytes.len() as u64, 2340, None).unwrap();
        assert_eq!(out, expected());
        assert!(matches!(extract_bzip2(&mut InputFailure, &mut Vec::new(), 123, 2340, None), Err(AlzError::Io(e)) if e.kind() == io::ErrorKind::PermissionDenied));
        assert!(matches!(extract_bzip2(&mut Cursor::new(&bytes), &mut OutputFailure, 123, 2340, None), Err(AlzError::CantOpenDestFile(e)) if e.kind() == io::ErrorKind::BrokenPipe));
        assert!(matches!(extract_bzip2(&mut Cursor::new(&bytes), &mut Vec::new(), 123, 2339, None), Err(AlzError::UncompressedSizeExceeded{limit:2339})));
    }
}
