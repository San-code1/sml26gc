//! Custom enemy sets: a level's enemies chosen from several game levels of its object
//! catalogue (`level_graphics`), as long as their tiles in the enemy sprite area ($B0-$E7,
//! 56 tiles, loaded by 0:0709 from the level's 0:362A entry) do not overlap. A set's block is
//! the level's own enemy sprites with each chosen enemy's tiles copied from its own level's
//! (research `enemymix`: drawn as in their own levels).
//!
//! Where the copy keeps them: the low copies of the map banks' compressed maps, which an
//! expanded copy never reads (maps are read from the twin banks, `layout`); the graphics loader
//! sets only the low bank bits, so a block must sit in banks $00-$1F. Every map bank's maps
//! start at $4500; the first room ($09:$4500) starts with a directory: "ENS1", then per level with a set (its number, game or new), its enemies
//! (count, then level and type each) and where its block is (bank, address); $00 ends it.
//! A game level with a set has its 0:362A entry pointed at its block (bank 0 and its twin); a
//! new level's entry is written while it is played (`extra_levels::writes_for`).
use crate::error::CartError;
use crate::layout::bank_maps;
use crate::patch::{is_expanded, TWIN};
use crate::rom::{offset, word};

/// A block of enemy sprites ($380 bytes, 56 tiles).
pub const BLOCK: usize = 0x380;
/// The enemy sprite area's tiles.
pub const AREA: std::ops::RangeInclusive<u8> = 0xB0..=0xE7;
/// Enemies a set can have.
pub const MOST: usize = 8;
const ENTRIES: u16 = 0x362A;
const MAGIC: &[u8; 4] = b"ENS1";
/// Bytes kept for the directory before the first block.
const DIRECTORY: usize = 0x300;
/// Banks that hold the game's level maps.
const MAP_BANKS: [u8; 8] = [0x09, 0x0A, 0x0B, 0x12, 0x13, 0x14, 0x15, 0x17];

/// An enemy of the game: the level its tiles come from and its type there.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Enemy {
    pub level: u8,
    pub kind: u8,
}

/// Level `level`'s enemy sprites as the game loads them (bank, address).
pub fn own_block(rom: &[u8], level: usize) -> (u8, u16) {
    let e = offset(0, ENTRIES) + level * 4;
    (rom[e], word(rom, e + 2))
}

fn block_at(rom: &[u8], (bank, addr): (u8, u16)) -> &[u8] {
    let at = offset(bank as usize, addr);
    &rom[at..at + BLOCK]
}

/// The tiles of `tiles` that are in the enemy area.
pub fn area_tiles(tiles: &[u8]) -> Vec<u8> {
    tiles.iter().copied().filter(|t| AREA.contains(t)).collect()
}

/// Do these two enemies' tiles overlap in the enemy area?
pub fn clash(a: &[u8], b: &[u8]) -> bool {
    area_tiles(a).iter().any(|t| b.contains(t))
}

/// The block for game level `base` with `enemies` (each with the tiles it shows): `base`'s own
/// enemy sprites, each enemy's area tiles copied from its own level's (in `original`).
pub fn build(original: &[u8], base: usize, enemies: &[(Enemy, Vec<u8>)]) -> Vec<u8> {
    let mut block = block_at(original, own_block(original, base)).to_vec();
    for (e, tiles) in enemies {
        let from = block_at(original, own_block(original, e.level as usize));
        for t in area_tiles(tiles) {
            let i = (t - AREA.start()) as usize * 16;
            block[i..i + 16].copy_from_slice(&from[i..i + 16]);
        }
    }
    block
}

/// Where blocks can go (bank, address), in order: the low copies of the map banks' maps
/// (first map's start to last map's end in `original`), the first room's start kept for the
/// directory.
pub fn slots(original: &[u8]) -> Vec<(u8, u16)> {
    let mut out = Vec::new();
    for (i, &bank) in MAP_BANKS.iter().enumerate() {
        let maps = bank_maps(original, bank);
        let (Some(start), Some(end)) = (maps.iter().map(|m| m.2).min(), maps.iter().map(|m| m.2 + m.3).max()) else { continue };
        let mut at = start + if i == 0 { DIRECTORY } else { 0 };
        while at + BLOCK <= end {
            out.push((bank, 0x4000 + (at & 0x3FFF) as u16));
            at += BLOCK;
        }
    }
    out
}

/// Where the directory is: the start of bank $09's maps.
const DIRECTORY_AT: (usize, u16) = (0x09, 0x4500);

fn directory(rom: &[u8]) -> Option<usize> {
    is_expanded(rom).then(|| offset(DIRECTORY_AT.0, DIRECTORY_AT.1))
}

/// A level's set as the copy keeps it: its number (game or new), its enemies, its block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stored {
    pub level: usize,
    pub enemies: Vec<Enemy>,
    pub block: (u8, u16),
}

/// The sets kept in the copy.
pub fn read(rom: &[u8]) -> Vec<Stored> {
    let Some(d) = directory(rom) else { return Vec::new() };
    if &rom[d..d + 4] != MAGIC {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut at = d + 4;
    while at < d + DIRECTORY && rom[at] != 0 {
        let level = rom[at] as usize - 1;
        let n = (rom[at + 1] as usize).min(MOST);
        let enemies = (0..n).map(|i| Enemy { level: rom[at + 2 + i * 2], kind: rom[at + 3 + i * 2] }).collect();
        let b = at + 2 + n * 2;
        out.push(Stored { level, enemies, block: (rom[b], word(rom, b + 1)) });
        at = b + 3;
    }
    out
}

/// The writes that point game level `level`'s 0:362A entry at `block` (bank 0 and its twin).
pub fn entry_writes(rom: &[u8], level: usize, (bank, addr): (u8, u16)) -> Vec<(usize, Vec<u8>)> {
    let e = offset(0, ENTRIES) + level * 4;
    let bytes = vec![bank, 0, addr as u8, (addr >> 8) as u8];
    let mut w = vec![(e, bytes.clone())];
    if rom.len() > TWIN {
        w.push((e + TWIN, bytes));
    }
    w
}

/// Keep `sets` (level, enemies, block bytes) in the copy, replacing the ones it had: blocks in
/// their slots, the directory, and the game levels' entries (those without a set get the
/// game's back).
pub fn store(rom: &mut [u8], original: &[u8], sets: &[(usize, Vec<Enemy>, Vec<u8>)]) -> Result<(), CartError> {
    let Some(d) = directory(rom) else { return Err(CartError::UnknownVersion) };
    let slots = slots(original);
    if sets.len() > slots.len() || 4 + sets.iter().map(|s| 5 + 2 * s.1.len().min(MOST)).sum::<usize>() >= DIRECTORY {
        return Err(CartError::EnemySetsFull { need: sets.len(), capacity: slots.len() });
    }
    rom[d..d + DIRECTORY].fill(0);
    rom[d..d + 4].copy_from_slice(MAGIC);
    let mut at = d + 4;
    for l in 0..crate::levels::LEVELS {
        for (pos, b) in entry_writes(rom, l, own_block(original, l)) {
            rom[pos..pos + b.len()].copy_from_slice(&b);
        }
    }
    for ((level, enemies, block), &slot) in sets.iter().zip(&slots) {
        let o = offset(slot.0 as usize, slot.1);
        rom[o..o + BLOCK].copy_from_slice(&block[..BLOCK]);
        let n = enemies.len().min(MOST);
        rom[at] = *level as u8 + 1;
        rom[at + 1] = n as u8;
        for (i, e) in enemies.iter().take(n).enumerate() {
            rom[at + 2 + i * 2] = e.level;
            rom[at + 3 + i * 2] = e.kind;
        }
        let b = at + 2 + n * 2;
        rom[b] = slot.0;
        rom[b + 1..b + 3].copy_from_slice(&slot.1.to_le_bytes());
        at = b + 3;
        if *level < crate::levels::LEVELS {
            for (pos, b) in entry_writes(rom, *level, slot) {
                rom[pos..pos + b.len()].copy_from_slice(&b);
            }
        }
    }
    Ok(())
}

/// Where level `level`'s block is, if it has a set.
pub fn block_of(rom: &[u8], level: usize) -> Option<(u8, u16)> {
    read(rom).into_iter().find(|s| s.level == level).map(|s| s.block)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_directory_is_where_bank_09s_maps_start() {
        let Some(path) = std::env::var_os("RESEARCH_ROM") else { return };
        let original = std::fs::read(path).unwrap();
        assert_eq!(bank_maps(&original, MAP_BANKS[0]).iter().map(|m| m.2).min(), Some(offset(DIRECTORY_AT.0, DIRECTORY_AT.1)));
    }

    #[test]
    fn clashes_are_area_tiles_in_common() {
        assert!(clash(&[0xB0, 0xB1, 0x0A], &[0xB1]));
        assert!(!clash(&[0xB0, 0x0A], &[0x0A, 0xC0]), "tiles outside the area do not count");
        assert_eq!(area_tiles(&[0x0A, 0xB0, 0xE7, 0xE8]), vec![0xB0, 0xE7]);
    }
}
