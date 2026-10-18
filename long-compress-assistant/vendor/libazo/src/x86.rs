/// x86 CALL/JMP filter for AZO decompression.
/// Converts absolute addresses back to relative in E8 (CALL) / E9 (JMP) instructions.
/// Applied as a post-processing step after LZ77 decompression.
pub fn x86_filter(buf: &mut [u8]) {
    let size = buf.len();
    if size < 5 {
        return;
    }

    let mut i = 0;
    while i < size - 4 {
        if buf[i] == 0xE8 || buf[i] == 0xE9 {
            if buf[i + 4] == 0x00 || buf[i + 4] == 0xFF {
                let mut addr = u32::from_le_bytes([buf[i + 1], buf[i + 2], buf[i + 3], buf[i + 4]]);
                addr = addr.wrapping_sub(i as u32);
                if (addr >> 24) & 1 != 0 {
                    addr |= 0xFF000000;
                } else {
                    addr &= 0x00FFFFFF;
                }
                let bytes = addr.to_le_bytes();
                buf[i + 1] = bytes[0];
                buf[i + 2] = bytes[1];
                buf[i + 3] = bytes[2];
                buf[i + 4] = bytes[3];
            }
            i += 5;
        } else {
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_e8_e9_unchanged() {
        let mut buf = vec![0x90, 0x90, 0x90, 0x90, 0x90, 0x90];
        let original = buf.clone();
        x86_filter(&mut buf);
        assert_eq!(buf, original);
    }

    #[test]
    fn test_too_short() {
        let mut buf = vec![0xE8, 0x01, 0x02, 0x03];
        let original = buf.clone();
        x86_filter(&mut buf);
        assert_eq!(buf, original);
    }

    #[test]
    fn test_empty() {
        let mut buf: Vec<u8> = vec![];
        x86_filter(&mut buf);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_e8_at_position_zero() {
        // E8 at pos 0, absolute addr 0x00000005 -> relative = 0x05 - 0 = 0x05
        // But the filter converts absolute TO relative by subtracting position.
        // Input: E8 [abs_addr as LE u32] where high byte is 0x00 or 0xFF
        // addr = 0x00000005, pos = 0, result = 0x00000005 - 0 = 0x00000005
        // high bit check: (0x05 >> 24) & 1 == 0, so addr &= 0x00FFFFFF -> 0x00000005
        let mut buf = vec![0xE8, 0x05, 0x00, 0x00, 0x00, 0x90];
        x86_filter(&mut buf);
        assert_eq!(buf, vec![0xE8, 0x05, 0x00, 0x00, 0x00, 0x90]);
    }

    #[test]
    fn test_e8_at_nonzero_position() {
        // E8 at pos 5: addr = 0x0000000A, subtract pos 5 -> 0x00000005
        // high bit check: 0 -> mask to 0x00FFFFFF -> 0x00000005
        let mut buf = vec![0x90, 0x90, 0x90, 0x90, 0x90, 0xE8, 0x0A, 0x00, 0x00, 0x00];
        x86_filter(&mut buf);
        assert_eq!(
            buf,
            vec![0x90, 0x90, 0x90, 0x90, 0x90, 0xE8, 0x05, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn test_e9_jmp() {
        // E9 should be handled the same as E8
        let mut buf = vec![0x90, 0x90, 0x90, 0x90, 0x90, 0xE9, 0x0A, 0x00, 0x00, 0x00];
        x86_filter(&mut buf);
        assert_eq!(
            buf,
            vec![0x90, 0x90, 0x90, 0x90, 0x90, 0xE9, 0x05, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn test_e8_high_byte_not_00_or_ff_skipped() {
        // E8 where 4th byte is not 0x00 or 0xFF -> not filtered
        let mut buf = vec![0xE8, 0x05, 0x00, 0x00, 0x01, 0x90];
        let original = buf.clone();
        x86_filter(&mut buf);
        assert_eq!(buf, original);
    }

    #[test]
    fn test_negative_address() {
        // E8 at pos 0, addr = 0xFF000010 (negative/backward call)
        // subtract 0 -> 0xFF000010
        // (0xFF000010 >> 24) & 1 == 1, so addr |= 0xFF000000 -> 0xFF000010
        let mut buf = vec![0xE8, 0x10, 0x00, 0x00, 0xFF, 0x90];
        x86_filter(&mut buf);
        assert_eq!(buf, vec![0xE8, 0x10, 0x00, 0x00, 0xFF, 0x90]);
    }
}
