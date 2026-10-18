//! Adaptive probability models.
use crate::range::RangeDecoder;

const BOOL_TOTAL_BIT: u32 = 12;
const BOOL_TOTAL_COUNT: u32 = 4096;
const BOOL_SHIFT_BIT: u32 = 6;

pub struct BoolState {
    state: usize,
    prob: Vec<u32>,
    n_bits: u32,
}

impl BoolState {
    pub fn new(n_bits: u32) -> Self {
        let size = 1 << n_bits;
        BoolState {
            state: 0,
            prob: vec![BOOL_TOTAL_COUNT / 2; size],
            n_bits,
        }
    }

    pub fn decode(&mut self, entropy: &mut RangeDecoder) -> u32 {
        let bit = entropy.decode_boolean(self.prob[self.state], BOOL_TOTAL_BIT);
        if bit == 0 {
            self.prob[self.state] += (BOOL_TOTAL_COUNT - self.prob[self.state]) >> BOOL_SHIFT_BIT;
        } else {
            self.prob[self.state] -= self.prob[self.state] >> BOOL_SHIFT_BIT;
        }
        let mask = (1 << self.n_bits) - 1;
        self.state = ((self.state << 1) | bit as usize) & mask;
        bit
    }
}

const EBP_TOTAL_BIT: u32 = 10;
const EBP_TOTAL_COUNT: u32 = 1024;
const EBP_SHIFT_BIT: u32 = 4;

pub struct EntropyBitProb {
    prob: Vec<u32>,
    bit_n: u32,
}

impl EntropyBitProb {
    pub fn new(n: usize) -> Self {
        let bit_n = ceil_log2(n);
        let array_n = 1 << bit_n;
        EntropyBitProb {
            prob: vec![EBP_TOTAL_COUNT / 2; array_n],
            bit_n,
        }
    }

    pub fn decode(&mut self, entropy: &mut RangeDecoder) -> u32 {
        let mut value = 0u32;
        let mut pre = 1usize;

        for i in (0..self.bit_n).rev() {
            let bit = entropy.decode_boolean(self.prob[pre], EBP_TOTAL_BIT);
            self.update_prob(pre, bit);
            if bit != 0 {
                value |= 1 << i;
            }
            pre = (pre << 1) | bit as usize;
        }

        value
    }

    /// Update probabilities for a known value without decoding.
    pub fn update(&mut self, value: u32) {
        let mut pre = 1usize;
        for i in (0..self.bit_n).rev() {
            let v = (value >> i) & 1;
            self.update_prob(pre, v);
            pre = (pre << 1) | v as usize;
        }
    }

    fn update_prob(&mut self, idx: usize, bit: u32) {
        if bit == 0 {
            self.prob[idx] += (EBP_TOTAL_COUNT - self.prob[idx]) >> EBP_SHIFT_BIT;
        } else {
            self.prob[idx] -= self.prob[idx] >> EBP_SHIFT_BIT;
        }
    }

    /// Compare which model better predicted the value.
    pub fn compare_with(m1: &EntropyBitProb, m2: &EntropyBitProb, value: u32) -> i32 {
        let mut prod1 = 1u32;
        let mut prod2 = 1u32;
        let mut pre = 1usize;
        let overflow_mask = (EBP_TOTAL_COUNT - 1) << (32 - EBP_TOTAL_BIT);

        for i in (0..m1.bit_n).rev() {
            let p1 = m1.prob[pre];
            let p2 = m2.prob[pre];
            let v = (value >> i) & 1;

            if (prod1 | prod2) & overflow_mask != 0 {
                prod1 >>= EBP_TOTAL_BIT;
                prod2 >>= EBP_TOTAL_BIT;
            }

            if v != 0 {
                prod1 = prod1.wrapping_mul(EBP_TOTAL_COUNT - p1);
                prod2 = prod2.wrapping_mul(EBP_TOTAL_COUNT - p2);
            } else {
                prod1 = prod1.wrapping_mul(p1);
                prod2 = prod2.wrapping_mul(p2);
            }

            pre = (pre << 1) | v as usize;
        }

        if prod1 > prod2 {
            1
        } else if prod1 < prod2 {
            -1
        } else {
            0
        }
    }
}

pub struct PredictProb {
    prob1: Vec<EntropyBitProb>,
    prob2: Vec<EntropyBitProb>,
    lucky: Vec<i32>,
    shift: u32,
}

impl PredictProb {
    pub fn new(key: usize, n: usize, shift: u32) -> Self {
        let coarse_size = key >> shift;
        PredictProb {
            prob1: (0..key).map(|_| EntropyBitProb::new(n)).collect(),
            prob2: (0..coarse_size).map(|_| EntropyBitProb::new(n)).collect(),
            lucky: vec![0; key],
            shift,
        }
    }

    pub fn decode(&mut self, entropy: &mut RangeDecoder, context: usize) -> u32 {
        let coarse = context >> self.shift;
        let value = if self.lucky[context] >= 0 {
            let v = self.prob1[context].decode(entropy);
            self.prob2[coarse].update(v);
            v
        } else {
            let v = self.prob2[coarse].decode(entropy);
            self.prob1[context].update(v);
            v
        };

        let r = EntropyBitProb::compare_with(&self.prob1[context], &self.prob2[coarse], value);
        if r > 0 {
            self.lucky[context] += 1;
        }
        if r < 0 {
            self.lucky[context] -= 1;
        }

        value
    }
}

fn ceil_log2(n: usize) -> u32 {
    if n <= 1 {
        return 0;
    }
    usize::BITS - (n - 1).leading_zeros()
}
