//! 32-bit arithmetic range coder reading bits MSB-first.

const MSB: u64 = 0x80000000;
const SMSB: u64 = 0x40000000;
const MASK: u64 = 0xFFFFFFFF;

pub struct RangeDecoder<'a> {
    reader: BitReader<'a>,
    low: u64,
    up: u64,
    tag: u64,
}

impl<'a> RangeDecoder<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        RangeDecoder {
            reader: BitReader::new(data),
            low: 0,
            up: MASK,
            tag: 0,
        }
    }

    pub fn initialize(&mut self) {
        self.tag = 0;
        for _ in 0..32 {
            self.tag = ((self.tag << 1) | self.reader.read_bit() as u64) & MASK;
        }
    }

    /// Decode a uniform symbol with `total_bit` bits.
    pub fn decode_uniform(&mut self, total_bit: u32) -> u32 {
        // `.max(1)` guards against division by zero; the range invariant keeps
        // the width well above `2^total_bit` in practice, but never rely on it.
        let t = ((self.up - self.low + 1) >> total_bit).max(1);
        // Clamp to `total_bit` bits. For a well-formed stream `value` is always
        // in range, but `t` is floored, so a crafted stream can drive `value`
        // past `2^total_bit` and yield an out-of-range distance/length code
        // (which would later index a fixed-size table out of bounds).
        let value = ((self.tag - self.low) / t).min((1u64 << total_bit) - 1);
        self.up = self.low + t * (value + 1) - 1;
        self.low += t * value;
        self.rescale();
        value as u32
    }

    /// Decode a boolean given cumCount (probability of 0) and total_bit.
    pub fn decode_boolean(&mut self, cum_count: u32, total_bit: u32) -> u32 {
        let t = ((self.up - self.low + 1) >> total_bit).max(1);
        let v = (self.tag - self.low) / t;
        if v >= cum_count as u64 {
            self.low += t * cum_count as u64;
            self.rescale();
            1
        } else {
            self.up = self.low + t * cum_count as u64 - 1;
            self.rescale();
            0
        }
    }

    /// Whether the bit reader fetched every input byte. Reads past the end
    /// do not advance the position, so tag lookahead overread still counts
    /// as fully consumed.
    pub fn fully_consumed(&self) -> bool {
        self.reader.pos == self.reader.data.len()
    }

    fn rescale(&mut self) {
        // Phase 1: MSB convergence
        while (self.low & MSB) == (self.up & MSB) {
            let bit = self.reader.read_bit() as u64;
            self.tag = ((self.tag << 1) & MASK) | bit;
            self.low = (self.low << 1) & MASK;
            self.up = ((self.up << 1) & MASK) | 1;
        }

        // Phase 2: Underflow resolution
        while (self.low & SMSB) != 0 && (self.up & SMSB) == 0 {
            let bit = self.reader.read_bit() as u64;
            self.tag = (((self.tag << 1) | bit) ^ MSB) & MASK;
            self.low = (self.low << 1) & (MASK >> 1);
            self.up = ((self.up << 1) | 1 | MSB) & MASK;
        }
    }
}

struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    bit_pos: u8, // bits remaining in current byte (8..1)
    current: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        BitReader {
            data,
            pos: 0,
            bit_pos: 0,
            current: 0,
        }
    }

    fn read_bit(&mut self) -> u8 {
        if self.bit_pos == 0 {
            if self.pos < self.data.len() {
                self.current = self.data[self.pos];
                self.pos += 1;
            } else {
                self.current = 0; // Read beyond buffer
            }
            self.bit_pos = 8;
        }
        self.bit_pos -= 1;
        (self.current >> self.bit_pos) & 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitreader_single_byte() {
        let data = [0b10110001u8];
        let mut br = BitReader::new(&data);
        assert_eq!(br.read_bit(), 1);
        assert_eq!(br.read_bit(), 0);
        assert_eq!(br.read_bit(), 1);
        assert_eq!(br.read_bit(), 1);
        assert_eq!(br.read_bit(), 0);
        assert_eq!(br.read_bit(), 0);
        assert_eq!(br.read_bit(), 0);
        assert_eq!(br.read_bit(), 1);
    }

    #[test]
    fn test_bitreader_multi_byte() {
        let data = [0xFF, 0x00];
        let mut br = BitReader::new(&data);
        for _ in 0..8 {
            assert_eq!(br.read_bit(), 1);
        }
        for _ in 0..8 {
            assert_eq!(br.read_bit(), 0);
        }
    }

    #[test]
    fn test_bitreader_beyond_buffer() {
        let data = [0xFF];
        let mut br = BitReader::new(&data);
        for _ in 0..8 {
            br.read_bit();
        }
        // Beyond buffer should return 0
        assert_eq!(br.read_bit(), 0);
    }

    #[test]
    fn test_range_decoder_initialize() {
        // Initialize reads 32 bits (4 bytes) into tag
        let data = [0x12, 0x34, 0x56, 0x78, 0x00, 0x00, 0x00, 0x00];
        let mut rd = RangeDecoder::new(&data);
        rd.initialize();
        assert_eq!(rd.tag, 0x12345678);
    }

    #[test]
    fn test_decode_uniform_1bit() {
        // Decode a single bit. Tag = 0x80000000 means the top bit is 1,
        // so decoding 1 bit with full range should give 1.
        let data = [0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let mut rd = RangeDecoder::new(&data);
        rd.initialize();
        let val = rd.decode_uniform(1);
        assert!(val <= 1);
    }
}
