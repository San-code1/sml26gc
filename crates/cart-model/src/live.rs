//! Reading the running game's memory: what it has loaded for the level being played.
use crate::levels::MAP_BYTES;
use crate::objects::Obj;
use crate::ram_map::{cart_ram_offset, LEVEL_MAP, OBJECT_RECORDS};
use gb_core::{BusObserver, GameBoy, Gb};

/// The level map as the game holds it after loading ($B000-$BFFF in cartridge RAM,
/// $C000-$DFFF in work RAM).
pub fn map_in_memory<O: BusObserver>(gb: &GameBoy<O>) -> Vec<u8> {
    let from = cart_ram_offset(LEVEL_MAP);
    let mut m = gb.cart().ram()[from..from + 0x1000].to_vec();
    m.extend_from_slice(&gb.wram()[..]);
    debug_assert_eq!(m.len(), MAP_BYTES);
    m
}

/// The 6-byte records the game built at $AB06 for a loaded level, decoded back.
pub fn objects_in_memory(gb: &Gb, count: usize) -> Vec<Obj> {
    let ram = gb.cart().ram();
    let base = cart_ram_offset(OBJECT_RECORDS);
    (0..count)
        .map(|k| {
            let r = &ram[base + k * 6..base + k * 6 + 6];
            Obj { x: r[0] as u16 * 256 + r[1] as u16, flag: r[2] != 0, y: r[3] as u16 * 256 + r[4] as u16, kind: r[5] & 0x7F }
        })
        .collect()
}

/// The player's position in the level (pixels).
pub fn player_position(gb: &mut Gb) -> (u16, u16) {
    use crate::ram_map::{PLAYER_X, PLAYER_Y};
    let x = (gb.read(PLAYER_X + 1) as u16) << 8 | gb.read(PLAYER_X) as u16;
    let y = (gb.read(PLAYER_Y + 1) as u16) << 8 | gb.read(PLAYER_Y) as u16;
    (x, y)
}
