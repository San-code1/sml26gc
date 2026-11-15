//! The zone maps: the one-screen maps Mario walks on inside a zone (Tree, Pumpkin...), as
//! the cartridge stores them (found by leaving a level on each zone's points and matching
//! the screen's background with the ROM; `research mapscan`):
//!
//! * Picture: 20 x 18 visible tiles, uncompressed, rows 32 tiles apart (the background's
//!   width), copied whole to $9800 by the zone's own set-up code in bank $0F (e.g. the Tree
//!   Zone's at 0F:511E: `ld a,$0E / ld hl,$7240 / ld de,$9800 / call $2BFB`).
//! * Tile graphics: loaded by the same code from the zone's own bank, so the editor draws
//!   them with tiles captured from the running game (`GameDriver::show_map_at`).
use crate::patch::TWIN;
use crate::rom::offset;

/// Visible size in tiles (the screen).
pub const W: usize = 20;
pub const H: usize = 18;
/// Bytes from one stored row to the next.
const STRIDE: usize = 32;

pub struct ZoneMap {
    pub name: &'static str,
    pub name_es: &'static str,
    bank: usize,
    addr: u16,
    /// A map point on this zone map: the game shows the map when Mario is put there.
    pub point: u8,
}

pub const ZONES: [ZoneMap; 8] = [
    ZoneMap { name: "Tree Zone", name_es: "Zona Arbol", bank: 0x0E, addr: 0x7240, point: 0x20 },
    ZoneMap { name: "Pumpkin Zone", name_es: "Zona Calabaza", bank: 0x0E, addr: 0x7000, point: 0x28 },
    ZoneMap { name: "Macro Zone", name_es: "Zona Macro", bank: 0x0E, addr: 0x76C0, point: 0x32 },
    ZoneMap { name: "Space Zone", name_es: "Zona Espacial", bank: 0x0E, addr: 0x7480, point: 0x3C },
    ZoneMap { name: "Turtle Zone", name_es: "Zona Tortuga", bank: 0x0E, addr: 0x7900, point: 0x46 },
    ZoneMap { name: "Mario Zone", name_es: "Zona Mario", bank: 0x0E, addr: 0x7B40, point: 0x50 },
    ZoneMap { name: "Wario's Castle", name_es: "Castillo de Wario", bank: 0x0E, addr: 0x7D80, point: 0x5C },
    ZoneMap { name: "Bonus game", name_es: "Juego de bonus", bank: 0x10, addr: 0x7000, point: 0x6E },
];

fn at(z: &ZoneMap, x: usize, y: usize) -> usize {
    offset(z.bank, z.addr) + y * STRIDE + x
}

/// Zone `z`'s picture as 20 x 18 tile numbers, row by row.
pub fn read(rom: &[u8], z: usize) -> Vec<u8> {
    (0..W * H).map(|i| rom[at(&ZONES[z], i % W, i / W)]).collect()
}

/// Write zone `z`'s picture back (same place; the twin bank of an expanded copy too).
pub fn write(rom: &mut [u8], z: usize, tiles: &[u8]) {
    let twin = rom.len() >= 2 * TWIN;
    for (i, &t) in tiles.iter().enumerate().take(W * H) {
        let a = at(&ZONES[z], i % W, i / W);
        rom[a] = t;
        if twin {
            rom[a + TWIN] = t;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_32_apart_and_reach_the_twin() {
        let mut rom = vec![0u8; 2 * TWIN];
        let tiles: Vec<u8> = (0..W * H).map(|i| (i % 251) as u8).collect();
        write(&mut rom, 0, &tiles);
        assert_eq!(read(&rom, 0), tiles);
        let base = offset(0x0E, 0x7240);
        assert_eq!(rom[base + 32], tiles[W], "row 1 starts 32 bytes on");
        assert_eq!(rom[base + 20], 0, "the hidden columns are left alone");
        assert_eq!(rom[base + TWIN + 33], tiles[W + 1]);
    }

    #[test]
    fn zone_pictures_do_not_overlap() {
        for (i, a) in ZONES.iter().enumerate() {
            for b in &ZONES[i + 1..] {
                let (s, t) = (offset(a.bank, a.addr), offset(b.bank, b.addr));
                assert!(s + STRIDE * H <= t || t + STRIDE * H <= s, "{} / {}", a.name, b.name);
            }
        }
    }
}
