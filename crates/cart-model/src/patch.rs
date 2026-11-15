//! Room for bigger levels and object lists: the 1 MB edited copy.
//!
//! The cartridge has no free space in its level banks, so an edited copy is made 1 MB (the
//! MBC1 chip addresses up to 2 MB): banks $20-$3F are copies of $00-$3F, and a tiny patch
//! makes the level decoder (0:0361, which runs with the screen off and no interrupts) read
//! map data from bank $20 + the level's bank, then switch back. Each zone gets a whole
//! 16 KB bank for its maps instead of the ~6 KB it had.
//!   0:037C  ld a,$00 -> ld a,$01   upper bank bit set while decoding (0:037B writes it to $4000)
//!   0:03A9  ld a,h / cp $e0        -> jp $39BD (end-of-map test moved to the stub)
//!   0:39BD  stub in the unused $FF filler:
//!           ld a,h / cp $e0 / jp nz,$0386 / xor a / ld ($4000),a / ret
//! Bank $20 holds the same patched copy of bank 0, because with the upper bit set the
//! MBC1 maps bank $20 at $0000-$3FFF.
//!
//! Objects the same way: the object loader 03:6BBB (called once, at 0:0581, screen off) is
//! run with the upper bank bit set, so it reads its table and lists from bank $23, whose
//! $4000-$6036 the loader never uses and so can hold bigger lists.
//!   0:0581  call $6BBB -> call $39C8
//!   0:39C8  ld a,1 / ld ($4000),a / call $6BBB / xor a / ld ($4000),a / ret
use crate::error::CartError;

const ORIGINAL_SIZE: usize = 0x8_0000;
pub const EXPANDED_SIZE: usize = 0x10_0000;
/// Banks $20-$3F of the expanded copy mirror $00-$1F.
pub const TWIN: usize = 0x8_0000;
const STUB_AT: usize = 0x39BD;
const STUB: [u8; 11] = [
    0x7C, 0xFE, 0xE0, 0xC2, 0x86, 0x03, 0xAF, 0xEA, 0x00, 0x40, 0xC9,
];
pub const OBJ_STUB_AT: usize = 0x39C8;
const OBJ_STUB: [u8; 13] = [
    0x3E, 0x01, 0xEA, 0x00, 0x40, 0xCD, 0xBB, 0x6B, 0xAF, 0xEA, 0x00, 0x40, 0xC9,
];

pub fn is_expanded(rom: &[u8]) -> bool {
    rom.len() == EXPANDED_SIZE && rom[0x037C] == 0x01 && rom[STUB_AT..STUB_AT + STUB.len()] == STUB
}

pub fn objects_patched(rom: &[u8]) -> bool {
    is_expanded(rom)
        && rom[OBJ_STUB_AT..OBJ_STUB_AT + OBJ_STUB.len()] == OBJ_STUB
        && rom[0x0582] == OBJ_STUB_AT as u8
}

/// The bank the decoder reads map data from: the level's own bank, or in an expanded
/// copy its twin $20 banks higher.
pub fn data_bank(rom: &[u8], bank: u8) -> u8 {
    if is_expanded(rom) {
        bank | 0x20
    } else {
        bank
    }
}

/// Can this cartridge be expanded safely? (the patched bytes must be what we expect)
pub fn can_expand(original: &[u8]) -> bool {
    original.len() == ORIGINAL_SIZE
        && original[0x037B..0x037D] == [0x3E, 0x00]
        && original[0x03A9..0x03AF] == [0x7C, 0xFE, 0xE0, 0x20, 0xD8, 0xC9]
        && original[STUB_AT..OBJ_STUB_AT + OBJ_STUB.len()]
            .iter()
            .all(|&b| b == 0xFF)
        && original[0x0581..0x0584] == [0xCD, 0xBB, 0x6B]
        && original[0x0147] == 0x03
}

fn write_object_patch(rom: &mut [u8], base: usize) {
    rom[base + 0x0581..base + 0x0584].copy_from_slice(&[
        0xCD,
        OBJ_STUB_AT as u8,
        (OBJ_STUB_AT >> 8) as u8,
    ]);
    rom[base + OBJ_STUB_AT..base + OBJ_STUB_AT + OBJ_STUB.len()].copy_from_slice(&OBJ_STUB);
}

/// Add the object patch to a copy expanded before it existed (its bank $23 is still a plain
/// copy of bank 3, so the game reads the same lists until they're saved again).
pub fn upgrade(rom: &mut [u8]) {
    if is_expanded(rom)
        && !objects_patched(rom)
        && rom[OBJ_STUB_AT..OBJ_STUB_AT + OBJ_STUB.len()]
            .iter()
            .all(|&b| b == 0xFF)
    {
        for base in [0, TWIN] {
            write_object_patch(rom, base);
        }
    }
}

/// The 1 MB edited copy of an original cartridge (an already expanded copy is returned
/// as it is, upgraded if needed).
pub fn expand(original: &[u8]) -> Result<Vec<u8>, CartError> {
    if is_expanded(original) {
        let mut r = original.to_vec();
        upgrade(&mut r);
        return Ok(r);
    }
    if !can_expand(original) {
        return Err(CartError::UnknownVersion);
    }
    let mut rom = original.to_vec();
    rom.extend_from_slice(original);
    for base in [0, TWIN] {
        rom[base + 0x037C] = 0x01;
        rom[base + 0x03A9..base + 0x03AC].copy_from_slice(&[
            0xC3,
            STUB_AT as u8,
            (STUB_AT >> 8) as u8,
        ]);
        rom[base + STUB_AT..base + STUB_AT + STUB.len()].copy_from_slice(&STUB);
        write_object_patch(&mut rom, base);
    }
    // header: ROM size 1 MB, and its checksum
    rom[0x0148] = 0x05;
    let mut x: u8 = 0;
    for &b in &rom[0x0134..=0x014C] {
        x = x.wrapping_sub(b).wrapping_sub(1);
    }
    rom[0x014D] = x;
    Ok(rom)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake 512 KB cartridge with exactly the bytes the patches look for.
    fn fake_original() -> Vec<u8> {
        let mut r = vec![0u8; ORIGINAL_SIZE];
        r[0x037B..0x037D].copy_from_slice(&[0x3E, 0x00]);
        r[0x03A9..0x03AF].copy_from_slice(&[0x7C, 0xFE, 0xE0, 0x20, 0xD8, 0xC9]);
        r[STUB_AT..OBJ_STUB_AT + OBJ_STUB.len()].fill(0xFF);
        r[0x0581..0x0584].copy_from_slice(&[0xCD, 0xBB, 0x6B]);
        r[0x0147] = 0x03;
        r
    }

    #[test]
    fn expands_once_and_patches_both_halves() {
        let o = fake_original();
        assert!(can_expand(&o) && !is_expanded(&o));
        let e = expand(&o).unwrap();
        assert_eq!(e.len(), EXPANDED_SIZE);
        assert!(is_expanded(&e) && objects_patched(&e));
        assert_eq!(e[0x037C], e[TWIN + 0x037C]);
        assert_eq!(data_bank(&e, 0x09), 0x29);
        assert_eq!(data_bank(&o, 0x09), 0x09);
        // expanding again changes nothing
        assert_eq!(expand(&e).unwrap(), e);
    }

    #[test]
    fn refuses_an_unknown_cartridge() {
        let mut o = fake_original();
        o[0x0581] = 0x00;
        assert_eq!(expand(&o), Err(CartError::UnknownVersion));
    }
}
