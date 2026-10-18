//! CRC-32 checksum (polynomial 0xEDB88320).

const TABLE: [u32; 256] = build_table();

const fn build_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}

pub(crate) struct Crc32 {
    value: u32,
}

impl Crc32 {
    pub fn new() -> Self {
        Crc32 { value: 0xFFFF_FFFF }
    }

    pub fn update(&mut self, data: &[u8]) {
        let mut c = self.value;
        for &b in data {
            c = TABLE[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
        }
        self.value = c;
    }

    pub fn finalize(self) -> u32 {
        self.value ^ 0xFFFF_FFFF
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_check_vector() {
        // The canonical CRC-32 check value for "123456789".
        let mut c = Crc32::new();
        c.update(b"123456789");
        assert_eq!(c.finalize(), 0xCBF4_3926);
    }

    #[test]
    fn chunked_matches_oneshot() {
        let data = b"Hello, AZO!";
        let mut one = Crc32::new();
        one.update(data);
        let mut split = Crc32::new();
        split.update(&data[..4]);
        split.update(&data[4..]);
        assert_eq!(one.finalize(), split.finalize());
    }
}
