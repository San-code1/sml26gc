//! Finding things in the ROM file: banks of 16 KB, little-endian words.

pub const BANK_SIZE: usize = 0x4000;

/// File offset of `addr` as seen with `bank` mapped (bank 0 for $0000-$3FFF).
pub const fn offset(bank: usize, addr: u16) -> usize {
    bank * BANK_SIZE + (addr as usize & 0x3FFF)
}

/// The little-endian word at file offset `at`.
pub fn word(rom: &[u8], at: usize) -> u16 {
    rom[at] as u16 | (rom[at + 1] as u16) << 8
}

/// Write a little-endian word at file offset `at`.
pub fn set_word(rom: &mut [u8], at: usize, v: u16) {
    rom[at] = v as u8;
    rom[at + 1] = (v >> 8) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_and_words() {
        assert_eq!(offset(1, 0x55CB), 0x55CB);
        assert_eq!(offset(0x0D, 0x7000), 0x37000);
        assert_eq!(offset(0, 0x0150), 0x0150);
        let mut r = vec![0u8; 4];
        set_word(&mut r, 1, 0xBEEF);
        assert_eq!(r, [0, 0xEF, 0xBE, 0]);
        assert_eq!(word(&r, 1), 0xBEEF);
    }
}
