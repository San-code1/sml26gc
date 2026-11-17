//! What a level's objects look and act as, chosen by its number when it loads (0:0669-0:0762,
//! research `comboprobe`): its sprite tiles and its object catalogue (03:6C55: levels 10 and up
//! set $AF06, and the object code then reads other tables, so a type number is another object
//! there). The sprite tiles come in three sets:
//! - $00-$7F ($8000-$87FF): bank 6 $4000, $4800 or $5000 by the low nibble of 0:360A (0, 1,
//!   other);
//! - $80-$AF and $E8-$FF ($8800-$8AFF, $8E80-$8FFF): bank $1B $7000 + n x $300 and bank 7 from
//!   the table at 0:0763, both by the high nibble n of 0:360A;
//! - $B0-$E7 ($8B00-$8E7F): the enemy sprites, 0:362A (four bytes per level: bank, -,
//!   address; $380 bytes).
//! An object looks the same in two levels that load the same graphics into the sets it uses.
use crate::levels::LEVELS;
use crate::objects;
use crate::rom::offset;

const ENEMY_SETS: u16 = 0x362A;
const COMMON_SETS: u16 = 0x360A;
/// `set_of` bits.
pub const LOW_SET: u8 = 1;
pub const HIGH_SET: u8 = 2;
pub const ENEMY_SET: u8 = 4;

/// The set sprite tile `tile` is in.
pub fn set_of(tile: u8) -> u8 {
    match tile {
        0x00..=0x7F => LOW_SET,
        0xB0..=0xE7 => ENEMY_SET,
        _ => HIGH_SET,
    }
}

/// What level `level` loads into each set: the low set's choice, the high set's, the enemy
/// sprites' entry.
fn loads(rom: &[u8], level: usize) -> (u8, u8, [u8; 4]) {
    let c = rom[offset(0, COMMON_SETS) + level];
    let low = match c & 0x0F {
        0 => 0,
        1 => 1,
        _ => 2,
    };
    (low, c >> 4, object_group(rom, level).0)
}

/// Do levels `a` and `b` load the same graphics into the sets `sets` (`set_of` bits)?
pub fn same_sets(rom: &[u8], a: usize, b: usize, sets: u8) -> bool {
    let (x, y) = (loads(rom, a), loads(rom, b));
    (sets & LOW_SET == 0 || x.0 == y.0) && (sets & HIGH_SET == 0 || x.1 == y.1) && (sets & ENEMY_SET == 0 || x.2 == y.2)
}

/// The game's levels of level `level`'s catalogue that use object type `kind`.
pub fn levels_using(original: &[u8], level: usize, kind: u8) -> Vec<usize> {
    let cat = object_group(original, level).1;
    (0..LEVELS).filter(|&l| object_group(original, l).1 == cat && objects::read(original, l).0.iter().any(|o| o.kind == kind)).collect()
}

/// The first level of the second object catalogue.
pub const SECOND_CATALOGUE: usize = 10;

/// What decides how level `level`'s objects look and act.
pub fn object_group(rom: &[u8], level: usize) -> ([u8; 4], bool) {
    let at = offset(0, ENEMY_SETS) + level * 4;
    (rom[at..at + 4].try_into().unwrap_or([0; 4]), level >= SECOND_CATALOGUE)
}

/// The game's levels (in `original`) whose objects look and act as level `level`'s.
pub fn same_objects(original: &[u8], level: usize) -> Vec<usize> {
    let g = object_group(original, level);
    (0..LEVELS).filter(|&l| object_group(original, l) == g).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_group_is_the_same_sprites_and_catalogue() {
        let mut rom = vec![0u8; 0x8000];
        let set = offset(0, ENEMY_SETS);
        for l in 0..LEVELS {
            rom[set + l * 4..set + l * 4 + 4].copy_from_slice(&[6, 0, 0, 0x58 + (l % 3) as u8]);
        }
        assert_eq!(same_objects(&rom, 0), vec![0, 3, 6, 9]);
        assert_eq!(same_objects(&rom, 12), vec![12, 15, 18, 21, 24, 27, 30], "the second catalogue apart");
    }

    #[test]
    fn sets_compared_only_where_used() {
        let mut rom = vec![0u8; 0x8000];
        let common = offset(0, COMMON_SETS);
        rom[common] = 0x10;
        rom[common + 1] = 0x00;
        rom[common + 2] = 0x05;
        let set = offset(0, ENEMY_SETS);
        rom[set + 4] = 9;
        assert_eq!((set_of(0x20), set_of(0x90), set_of(0xC0), set_of(0xF0)), (LOW_SET, HIGH_SET, ENEMY_SET, HIGH_SET));
        assert!(same_sets(&rom, 0, 1, LOW_SET) && !same_sets(&rom, 0, 1, HIGH_SET));
        assert!(!same_sets(&rom, 0, 1, ENEMY_SET), "other enemy sprites");
        assert!(same_sets(&rom, 1, 2, HIGH_SET) && !same_sets(&rom, 1, 2, LOW_SET), "5 is neither 0 nor 1");
    }
}
