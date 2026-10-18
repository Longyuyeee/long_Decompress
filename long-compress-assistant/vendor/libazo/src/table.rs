//! Code tables for AZO distance and length coding.

const MATCH_MIN_LENGTH: u32 = 2;
const MATCH_MIN_DIST: u32 = 1;
const MATCH_LENGTH_SGAP: u32 = 32;
const MATCH_LENGTH_GAP: u32 = 8;
const MATCH_DIST_SGAP: u32 = 16;
const MATCH_DIST_GAP: u32 = 4;
pub(crate) const CODE_SIZE: usize = 128;

pub(crate) struct CodeTable {
    pub(crate) base: [u32; CODE_SIZE],
    pub(crate) extra_bits: [u32; CODE_SIZE],
}

impl CodeTable {
    pub fn build_length_table() -> Self {
        let mut base = [0u32; CODE_SIZE];
        let mut extra_bits = [0u32; CODE_SIZE];

        base[0] = MATCH_MIN_LENGTH;
        for i in 1..CODE_SIZE {
            let eb = if (i as u32) < MATCH_LENGTH_SGAP {
                0
            } else {
                ((i as u32) - MATCH_LENGTH_SGAP) / MATCH_LENGTH_GAP
            };
            extra_bits[i] = eb;
            base[i] = base[i - 1] + (1 << extra_bits[i - 1]);
        }
        // extra_bits[0] is already 0

        CodeTable { base, extra_bits }
    }

    pub fn build_dist_table() -> Self {
        let mut base = [0u32; CODE_SIZE];
        let mut extra_bits = [0u32; CODE_SIZE];

        base[0] = MATCH_MIN_DIST;
        for i in 1..CODE_SIZE {
            let eb = if (i as u32) < MATCH_DIST_SGAP {
                0
            } else {
                ((i as u32) - MATCH_DIST_SGAP) / MATCH_DIST_GAP
            };
            extra_bits[i] = eb;
            base[i] = base[i - 1] + (1 << extra_bits[i - 1]);
        }

        CodeTable { base, extra_bits }
    }

    /// Code index whose bucket contains `value`: the last entry of `base` that
    /// does not exceed it (`base` is strictly increasing).
    pub fn code_for(&self, value: u32) -> usize {
        self.base.partition_point(|&b| b <= value).saturating_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_length_table_first_entries() {
        let t = CodeTable::build_length_table();
        assert_eq!(t.base[0], 2);
        assert_eq!(t.base[1], 3);
        assert_eq!(t.base[31], 33);
        assert_eq!(t.base[32], 34);
        assert_eq!(t.base[39], 41);
        assert_eq!(t.extra_bits[39], 0);
        assert_eq!(t.extra_bits[40], 1);
        assert_eq!(t.base[40], 42);
    }

    #[test]
    fn test_dist_table_first_entries() {
        let t = CodeTable::build_dist_table();
        assert_eq!(t.base[0], 1);
        assert_eq!(t.base[1], 2);
        assert_eq!(t.base[15], 16);
        assert_eq!(t.base[16], 17);
        assert_eq!(t.base[19], 20);
        assert_eq!(t.extra_bits[19], 0);
        assert_eq!(t.extra_bits[20], 1);
        assert_eq!(t.base[20], 21);
    }

    #[test]
    fn test_code_for_roundtrip() {
        let t = CodeTable::build_dist_table();
        for i in 0..CODE_SIZE {
            assert_eq!(t.code_for(t.base[i]), i);
        }
    }
}
