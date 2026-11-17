//! The object catalogue patch some edited copies may carry: it let a level use objects of the
//! other object catalogue (levels 0-9 or 10-31), and was taken out because many of those
//! objects need graphics their new level does not load (research `mixall`, `defeats`). `remove`
//! gives such a copy the game's bytes back.
//!
//! What it changed: the four reads of the catalogue flag ($AF06: 16:4036, 02:424D, 02:41D2,
//! 03:400B) and 16:738C became calls; the object loader stub's call (`patch::OBJ_STUB_AT` + 6)
//! went to a routine in bank $23; code went into bank 0 at 0:1967 and 0:1AFF (and its twin), and
//! into bank 16 at 16:7F1F.
use crate::patch::{OBJ_STUB_AT, TWIN};
use crate::rom::offset;

/// The object catalogue of game level `level` (0 or 1).
pub fn catalogue(level: usize) -> u8 {
    (level >= crate::level_graphics::SECOND_CATALOGUE) as u8
}

/// (bank, address, length) of every byte the patch changed outside bank $23's object lists.
const CHANGED: [(usize, u16, usize); 8] = [
    (0x16, 0x4036, 3),
    (0x02, 0x424D, 3),
    (0x02, 0x41D2, 3),
    (0x03, 0x400B, 3),
    (0x16, 0x738C, 3),
    (0x16, 0x7F1F, 40),
    (0x00, 0x1967, 41),
    (0x00, 0x1AFF, 19),
];
const GAME_LOADER: [u8; 2] = [0xBB, 0x6B];

/// Give the game's bytes (from `original`) back where the patch was; true if anything changed.
pub fn remove(rom: &mut [u8], original: &[u8]) -> bool {
    if rom.len() <= TWIN || original.len() < TWIN {
        return false;
    }
    let before = rom.to_vec();
    for (bank, addr, len) in CHANGED {
        let at = if bank == 0 { addr as usize } else { offset(bank, addr) };
        rom[at..at + len].copy_from_slice(&original[at..at + len]);
        if bank == 0 {
            rom[TWIN + at..TWIN + at + len].copy_from_slice(&original[at..at + len]);
        }
    }
    for base in [0, TWIN] {
        let call = base + OBJ_STUB_AT + 6;
        rom[call..call + 2].copy_from_slice(&GAME_LOADER);
    }
    *rom != before[..]
}
