//! The world map, as the cartridge stores it (read from the code that shows it):
//!
//! * Picture: 64 x 64 tiles of 8 x 8 pixels (512 x 512), uncompressed, in bank $0D at
//!   $7000-$7FFF as four 32 x 32 quadrants (top-left, top-right, bottom-left, bottom-right).
//!   0F:6AE5 copies it through 0:3E22 into cartridge RAM as the map scrolls; the tile
//!   graphics come from bank $0D $4000-$57FF.
//! * Which level each map point opens: byte table at 0F:4218 indexed by the point number
//!   ($A840); $FE = a point that opens no level. 0F:41A6-0F:41B5 copies the entry to $A269.
use crate::patch::TWIN;
use crate::{added_points, map_routes};
use crate::rom::offset;

pub const W: usize = 64;
pub const H: usize = 64;
const PICTURE: usize = offset(0x0D, 0x7000);
const POINTS: usize = offset(0x0F, 0x4218);
/// Point numbers with a level (`point_levels`).
pub const POINT_COUNT: usize = map_routes::ALL;
/// The game's points ($00-$7F) in the table.
const GAME_POINTS: usize = map_routes::POINTS;
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
    let twin = rom.len() >= 2 * TWIN;
    for (i, &t) in tiles.iter().enumerate() {
        let a = at(i % W, i / W);
        rom[a] = t;
        if twin {
            rom[a + TWIN] = t;
        }
    }
}

/// The level of every point number ($00-$FF; an added point's from `added_points`, those of
/// the map in place).
pub fn point_levels(rom: &[u8]) -> Vec<u8> {
    let mut v = rom[POINTS..POINTS + GAME_POINTS].to_vec();
    v.extend(added_points::levels(rom));
    v.resize(map_routes::ALL, NO_LEVEL);
    v
}

/// Write the levels back (the added points' too, if the copy has their hooks).
pub fn write_point_levels(rom: &mut [u8], v: &[u8]) {
    let twin = rom.len() >= 2 * TWIN;
    for (i, &l) in v.iter().enumerate().take(GAME_POINTS) {
        rom[POINTS + i] = l;
        if twin {
            rom[POINTS + i + TWIN] = l;
        }
    }
    if added_points::installed(rom) && v.len() > GAME_POINTS {
        let (shuts, rep) = (added_points::shuts(rom), added_points::rep(rom));
        added_points::write(rom, &v[GAME_POINTS..], &shuts, rep);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picture_quadrants_round_trip_and_reach_the_twin() {
        let mut rom = vec![0u8; 2 * TWIN];
        let tiles: Vec<u8> = (0..W * H).map(|i| (i * 7 % 251) as u8).collect();
        write(&mut rom, &tiles);
        assert_eq!(read(&rom), tiles);
        assert_eq!(&rom[PICTURE + TWIN..PICTURE + TWIN + W * H], &rom[PICTURE..PICTURE + W * H]);
        // the top-right quadrant starts 1024 bytes after the top-left one
        assert_eq!(rom[PICTURE + 1024], tiles[32]);
    }
}
