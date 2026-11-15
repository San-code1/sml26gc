//! Where the game's own map points are drawn: on the world map or on one of the zone maps,
//! and where on its picture. The game keeps this in its map code (bank $0F), not in a
//! table, so it was measured: `research mapscan` leaves a level on every point and reads
//! where the game draws Mario (his sprite; the camera on the world map). Points the game
//! never puts Mario on by itself (path corners, hidden points) are not here.
//!
//! Positions are pixels on the map's picture (512 x 512 for the world map, 160 x 144 for a
//! zone map): the middle of Mario's feet.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnMap {
    World,
    /// Index into `zone_maps::ZONES`.
    Zone(usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub point: u8,
    pub map: OnMap,
    pub x: i32,
    pub y: i32,
}

const fn w(point: u8, x: i32, y: i32) -> Spot {
    Spot { point, map: OnMap::World, x, y }
}

const fn z(zone: usize, point: u8, x: i32, y: i32) -> Spot {
    Spot { point, map: OnMap::Zone(zone), x, y }
}

const TREE: usize = 0;
const PUMPKIN: usize = 1;
const MACRO: usize = 2;
const SPACE: usize = 3;
const TURTLE: usize = 4;
const MARIO: usize = 5;
const CASTLE: usize = 6;
const BONUS: usize = 7;

pub const SPOTS: &[Spot] = &[
    // the world map
    w(0x00, 204, 394),
    w(0x01, 140, 330),
    w(0x03, 140, 258),
    w(0x04, 168, 258),
    w(0x05, 96, 122),
    w(0x07, 296, 154),
    w(0x08, 340, 130),
    w(0x0A, 356, 282),
    w(0x0E, 284, 354),
    w(0x0F, 356, 370),
    w(0x10, 252, 298),
    w(0x12, 204, 354),
    w(0x16, 244, 354),
    w(0x18, 212, 154),
    w(0x19, 92, 314),
    w(0x1B, 212, 146),
    w(0x1C, 384, 130),
    w(0x1D, 404, 274),
    w(0x66, 472, 422),
    w(0x78, 424, 150),
    w(0x7A, 328, 90),
    // Tree Zone
    z(TREE, 0x1F, 76, 54),
    z(TREE, 0x20, 76, 30),
    z(TREE, 0x21, 112, 62),
    z(TREE, 0x22, 88, 106),
    z(TREE, 0x23, 48, 106),
    z(TREE, 0x24, 44, 62),
    z(TREE, 0x26, 148, 114),
    // Pumpkin Zone
    z(PUMPKIN, 0x28, 83, 85),
    z(PUMPKIN, 0x29, 99, 65),
    z(PUMPKIN, 0x2A, 55, 65),
    z(PUMPKIN, 0x2B, 83, 53),
    // Macro Zone
    z(MACRO, 0x32, 18, 116),
    z(MACRO, 0x33, 34, 100),
    z(MACRO, 0x34, 86, 100),
    z(MACRO, 0x35, 94, 60),
    z(MACRO, 0x6A, 18, 60),
    // Space Zone
    z(SPACE, 0x3C, 75, 76),
    z(SPACE, 0x3D, 59, 28),
    z(SPACE, 0x3E, 75, 68),
    z(SPACE, 0x44, 147, 28),
    // Turtle Zone
    z(TURTLE, 0x46, 74, 30),
    z(TURTLE, 0x48, 114, 86),
    z(TURTLE, 0x49, 90, 110),
    // Mario Zone
    z(MARIO, 0x50, 60, 108),
    z(MARIO, 0x51, 80, 88),
    z(MARIO, 0x52, 80, 68),
    z(MARIO, 0x53, 80, 12),
    // Wario's castle
    z(CASTLE, 0x5C, 81, 133),
    // the bonus game
    z(BONUS, 0x6E, 40, 127),
    z(BONUS, 0x6F, 64, 127),
    z(BONUS, 0x70, 88, 127),
    z(BONUS, 0x71, 112, 127),
];

/// The points drawn on `map`.
pub fn on(map: OnMap) -> impl Iterator<Item = &'static Spot> {
    SPOTS.iter().filter(move |s| s.map == map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::POINT_COUNT;
    use crate::zone_maps::ZONES;

    #[test]
    fn every_spot_is_a_point_on_its_picture_once() {
        for (i, s) in SPOTS.iter().enumerate() {
            assert!((s.point as usize) < POINT_COUNT);
            assert!(!SPOTS[..i].iter().any(|t| t.point == s.point), "point {:02x} twice", s.point);
            let (w, h) = match s.map {
                OnMap::World => (512, 512),
                OnMap::Zone(z) => {
                    assert!(z < ZONES.len());
                    (160, 144)
                }
            };
            assert!((0..w).contains(&s.x) && (0..h).contains(&s.y), "point {:02x}", s.point);
        }
    }

    #[test]
    fn each_zone_has_its_viewing_point() {
        for (i, zm) in ZONES.iter().enumerate() {
            assert!(on(OnMap::Zone(i)).any(|s| s.point == zm.point), "{}", zm.name);
        }
    }
}
