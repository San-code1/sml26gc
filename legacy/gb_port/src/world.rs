//! The world map, as the cartridge stores it (read from the code that shows it):
//!
//! * Picture: 64 x 64 tiles of 8 x 8 pixels (512 x 512), uncompressed, in bank $0D at
//!   $7000-$7FFF as four 32 x 32 quadrants (top-left, top-right, bottom-left, bottom-right).
//!   0F:6AE5 copies it through 0:3E22 into cartridge RAM as the map scrolls; the tile
//!   graphics come from bank $0D $4000-$57FF.
//! * Which level each map point opens: byte table at 0F:4218 indexed by the point number
//!   ($A840); $FE = a point that opens no level. 0F:41A6-0F:41B5 copies the entry to $A269.
use crate::machine::Gb;

pub const W: usize = 64;
pub const H: usize = 64;
const PICTURE: usize = 0x0D * 0x4000 + 0x3000;
const POINTS: usize = 0x0F * 0x4000 + 0x0218;
pub const POINT_COUNT: usize = 0x82;
pub const NO_LEVEL: u8 = 0xFE;

fn at(x: usize, y: usize) -> usize {
    PICTURE + ((y / 32) * 2 + x / 32) * 1024 + (y % 32) * 32 + x % 32
}

/// The picture as 64 x 64 tile numbers, row by row.
pub fn read(rom: &[u8]) -> Vec<u8> {
    (0..W * H).map(|i| rom[at(i % W, i / W)]).collect()
}

/// Write the picture back (same size, same place; the twin bank of an expanded copy too).
pub fn write(rom: &mut [u8], tiles: &[u8]) {
    let twin = rom.len() >= 0x10_0000;
    for (i, &t) in tiles.iter().enumerate() {
        let a = at(i % W, i / W);
        rom[a] = t;
        if twin {
            rom[a + 0x8_0000] = t;
        }
    }
}

pub fn point_levels(rom: &[u8]) -> Vec<u8> {
    rom[POINTS..POINTS + POINT_COUNT].to_vec()
}

pub fn write_point_levels(rom: &mut [u8], v: &[u8]) {
    let twin = rom.len() >= 0x10_0000;
    for (i, &l) in v.iter().enumerate().take(POINT_COUNT) {
        rom[POINTS + i] = l;
        if twin {
            rom[POINTS + i + 0x8_0000] = l;
        }
    }
}

/// The tile graphics the game shows the map with (captured from the running game).
#[derive(Clone)]
pub struct WorldArt {
    pub vram: Vec<u8>,
    pub lcdc: u8,
    pub bgp: u8,
}

impl WorldArt {
    pub fn capture(gb: &Gb) -> WorldArt {
        WorldArt { vram: gb.bus.ppu.vram.to_vec(), lcdc: gb.bus.ppu.lcdc, bgp: gb.bus.ppu.bgp() }
    }
    pub fn pixel(&self, tile: u8, x: usize, y: usize) -> u8 {
        let base = if self.lcdc & 0x10 != 0 { tile as usize * 16 } else { (0x1000 + (tile as i8 as i32) * 16) as usize };
        let a = base + y * 2;
        let bit = 7 - x;
        let c = ((self.vram[a + 1] >> bit) & 1) << 1 | ((self.vram[a] >> bit) & 1);
        (self.bgp >> (c * 2)) & 3
    }
}
