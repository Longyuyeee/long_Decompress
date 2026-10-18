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

/// Extract ALZ-modified bzip2 data, streaming decoded output to `writer` and
/// stopping if more than `max_output` bytes would be produced.
/// Returns the CRC32 of the decompressed data.
pub fn extract_bzip2<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    compressed_size: u64,
    max_output: u64,
    mut crypto: Option<&mut ZipCrypto>,
) -> AlzResult<u32> {
    // Bound the in-memory compressed copy. 256 MiB keeps `len * 8` (below)
    // within a 32-bit usize.
    const MAX_BZ2_COMPRESSED: u64 = 256 * 1024 * 1024;
    // Guard against a payload packed with spurious DLZ markers forcing many
    // decode attempts for one block.
    const MAX_CANDIDATES_PER_BLOCK: u32 = 4096;

    let mut alz_data = Vec::new();
    let read = reader
        .take(MAX_BZ2_COMPRESSED + 1)
        .read_to_end(&mut alz_data)?;
    if read as u64 > MAX_BZ2_COMPRESSED {
        return Err(AlzError::Bzip2Failed(
            "compressed size exceeds limit".into(),
        ));
    }
    // read_to_end never errors on a short read, so a truncated entry surfaces as
    // fewer bytes than the header declared. Report it as EOF so the truncation
    // recovery path (which store and deflate reach via read_exact) also applies.
    if compressed_size > 0 && (read as u64) < compressed_size {
        return Err(AlzError::Io(io::Error::from(io::ErrorKind::UnexpectedEof)));
    }
    if let Some(ref mut c) = crypto {
        c.decrypt(&mut alz_data);
    }

    let total_bits = alz_data.len() * 8;
    // Each probe re-copies the block up to the candidate cut, so false DLZ
    // markers make the search quadratic. Cap the total bytes copied; a valid
    // archive stays far under the cap.
    let probe_budget = (alz_data.len() as u64).saturating_mul(32).max(1 << 20);
    let mut probe_bytes: u64 = 0;
    let mut hasher = crc32fast::Hasher::new();
    let mut produced_total: u64 = 0;
    let mut pos = 0usize; // bit position

    // Read the 4-byte marker at the current bit position each iteration; a
    // missing marker (fewer than 32 bits left) ends the stream.
    while let Some(marker) = peek32(&alz_data, pos) {
        pos += 32;
        if marker == ALZ_EOS_MAGIC_U32 {
            break;
        }
        if marker != ALZ_BLOCK_MAGIC_U32 {
            return Err(AlzError::Bzip2Failed(format!(
                "expected ALZ block header, got {marker:08x}"
            )));
        }

        // Find the true end of this block by decoding candidate cuts.
        let block_start = pos;
        let mut search_from = block_start;
        let mut attempts = 0u32;
        let mut block_out = Vec::new();
        let block_end = loop {
            attempts += 1;
            if attempts > MAX_CANDIDATES_PER_BLOCK {
                return Err(AlzError::Bzip2Failed("block boundary not found".into()));
            }
            let cand = next_marker(&alz_data, search_from).unwrap_or(total_bits);
            probe_bytes += ((cand - block_start) / 8) as u64;
            if probe_bytes > probe_budget {
                return Err(AlzError::Bzip2Failed(
                    "bzip2 boundary search exceeded work budget".into(),
                ));
            }
            block_out.clear();
            let produced = decode_probe(
                &build_block_probe(&alz_data, block_start, cand),
                &mut block_out,
                max_output.saturating_sub(produced_total),
            )?;
            if produced > 0 {
                break cand; // real boundary: a complete block emitted output
            }
            // Truncated (false marker before the true end): try the next one.
            if cand >= total_bits {
                return Err(AlzError::Bzip2Failed("could not decode bzip2 block".into()));
            }
            search_from = cand + 1;
        };

        produced_total += block_out.len() as u64;
        if produced_total > max_output {
            return Err(AlzError::UncompressedSizeExceeded { limit: max_output });
        }
        hasher.update(&block_out);
        writer
            .write_all(&block_out)
            .map_err(AlzError::CantOpenDestFile)?;

        pos = block_end;
    }

    Ok(hasher.finalize())
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
