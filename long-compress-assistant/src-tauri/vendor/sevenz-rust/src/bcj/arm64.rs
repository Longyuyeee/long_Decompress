// SPDX-License-Identifier: 0BSD
// Rust adaptation of XZ src/liblzma/simple/arm64.c (BL and ADRP conversion).
// Upstream authors: Lasse Collin, Jia Tan, Igor Pavlov.
// https://github.com/tukaani-project/xz/blob/master/src/liblzma/simple/arm64.c
// License: ARM64-0BSD.txt. Existing other architecture kernels are unchanged.
use super::BCJFilter;

impl BCJFilter {
    pub fn new_arm64(start_pos: usize, encoder: bool) -> Self {
        Self { is_encoder: encoder, pos: start_pos, prev_mask: 0, filter: Self::arm64_code }
    }

    fn arm64_code(&mut self, bytes: &mut [u8]) -> usize {
        let processed = bytes.len() & !3;
        for offset in (0..processed).step_by(4) {
            let pc = (self.pos as u32).wrapping_add(offset as u32);
            let mut instruction = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
            if instruction >> 26 == 0x25 {
                let relative = pc >> 2;
                let relative = if self.is_encoder { relative } else { 0u32.wrapping_sub(relative) };
                instruction = 0x94000000 | (instruction.wrapping_add(relative) & 0x03ffffff);
            } else if instruction & 0x9f000000 == 0x90000000 {
                let source = ((instruction >> 29) & 3) | ((instruction >> 3) & 0x001ffffc);
                if source.wrapping_add(0x00020000) & 0x001c0000 != 0 { continue; }
                let relative = pc >> 12;
                let relative = if self.is_encoder { relative } else { 0u32.wrapping_sub(relative) };
                let destination = source.wrapping_add(relative);
                instruction &= 0x9000001f;
                instruction |= (destination & 3) << 29;
                instruction |= (destination & 0x0003fffc) << 3;
                instruction |= 0u32.wrapping_sub(destination & 0x00020000) & 0x00e00000;
            } else { continue; }
            bytes[offset..offset + 4].copy_from_slice(&instruction.to_le_bytes());
        }
        self.pos = self.pos.wrapping_add(processed);
        processed
    }
}
