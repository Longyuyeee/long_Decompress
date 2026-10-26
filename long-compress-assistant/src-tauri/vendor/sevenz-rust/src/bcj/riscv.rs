// SPDX-License-Identifier: 0BSD
// Rust adaptation of XZ src/liblzma/simple/riscv.c.
// Upstream authors: Lasse Collin, Jia Tan; filter design with Igor Pavlov.
// https://github.com/tukaani-project/xz/blob/master/src/liblzma/simple/riscv.c
// License: RISCV-0BSD.txt. Keep the bijective AUIPC fake-pair conversion.
use super::BCJFilter;

impl BCJFilter {
    pub fn new_riscv(start_pos: usize, encoder: bool) -> Self {
        Self { is_encoder: encoder, pos: start_pos, prev_mask: 0, filter: Self::riscv_code }
    }

    fn riscv_code(&mut self, bytes: &mut [u8]) -> usize {
        // Scan at two-byte boundaries, but retain up to seven bytes. Even JAL
        // needs eight available bytes: changing that rule desynchronizes pairs.
        if bytes.len() < 8 { return 0; }
        let mut i = 0;
        while i <= bytes.len() - 8 {
            let first = bytes[i] as u32;
            let pc = (self.pos as u32).wrapping_add(i as u32);
            if first == 0xef {
                let b1 = bytes[i + 1] as u32;
                if b1 & 0x0d != 0 { i += 2; continue; }
                let b2 = bytes[i + 2] as u32;
                let b3 = bytes[i + 3] as u32;
                if self.is_encoder {
                    let addr = (((b1 & 0xf0) << 8) | ((b2 & 0x0f) << 16)
                        | ((b2 & 0x10) << 7) | ((b2 & 0xe0) >> 4)
                        | ((b3 & 0x7f) << 4) | ((b3 & 0x80) << 13)).wrapping_add(pc);
                    bytes[i + 1] = ((b1 & 0x0f) | ((addr >> 13) & 0xf0)) as u8;
                    bytes[i + 2] = (addr >> 9) as u8;
                    bytes[i + 3] = (addr >> 1) as u8;
                } else {
                    let addr = (((b1 & 0xf0) << 13) | (b2 << 9) | (b3 << 1)).wrapping_sub(pc);
                    bytes[i + 1] = ((b1 & 0x0f) | ((addr >> 8) & 0xf0)) as u8;
                    bytes[i + 2] = (((addr >> 16) & 0x0f) | ((addr >> 7) & 0x10) | ((addr << 4) & 0xe0)) as u8;
                    bytes[i + 3] = (((addr >> 4) & 0x7f) | ((addr >> 13) & 0x80)) as u8;
                }
                i += 4;
            } else if first & 0x7f == 0x17 {
                let inst = u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
                let (out1, out2);
                if inst & 0xe80 != 0 {
                    let inst2 = u32::from_le_bytes(bytes[i + 4..i + 8].try_into().unwrap());
                    if ((inst << 8) ^ inst2.wrapping_sub(3)) & 0xf8003 != 0 {
                        // Skip six bytes so a later match cannot create a pair
                        // the other direction would recognize at this position.
                        i += 6; continue;
                    }
                    out1 = 0x117 | (inst2 << 12);
                    if self.is_encoder {
                        let addr = (inst & 0xfffff000)
                            .wrapping_add((inst2 >> 20).wrapping_sub((inst2 >> 19) & 0x1000))
                            .wrapping_add(pc);
                        bytes[i..i + 4].copy_from_slice(&out1.to_le_bytes());
                        bytes[i + 4..i + 8].copy_from_slice(&addr.to_be_bytes());
                        i += 8; continue;
                    }
                    // Undo the encoder's fake decode, without PC/sign extension.
                    out2 = (inst & 0xfffff000).wrapping_add(inst2 >> 20);
                } else {
                    let rs1 = inst >> 27;
                    if inst.wrapping_sub(0x3117) << 18 >= rs1 & 0x1d {
                        i += 4; continue;
                    }
                    if self.is_encoder {
                        // Arbitrary input may already resemble a packed pair.
                        // Fake decoding makes the transformation reversible.
                        let addr = u32::from_le_bytes(bytes[i + 4..i + 8].try_into().unwrap());
                        out1 = 0x17 | (rs1 << 7) | (addr & 0xfffff000);
                        out2 = (inst >> 12) | (addr << 20);
                    } else {
                        let addr = u32::from_be_bytes(bytes[i + 4..i + 8].try_into().unwrap()).wrapping_sub(pc);
                        out1 = 0x17 | (rs1 << 7) | (addr.wrapping_add(0x800) & 0xfffff000);
                        out2 = (inst >> 12) | (addr << 20);
                    }
                }
                bytes[i..i + 4].copy_from_slice(&out1.to_le_bytes());
                bytes[i + 4..i + 8].copy_from_slice(&out2.to_le_bytes());
                i += 8;
            } else { i += 2; }
        }
        self.pos = self.pos.wrapping_add(i);
        i
    }
}
