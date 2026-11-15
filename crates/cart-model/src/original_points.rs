//! The game's own map as it plays: where each map point is drawn (on the world map or on a
//! zone map, and where on its picture) and which points Mario walks between. The game keeps
//! all this in its map code (bank $0F), not in tables, so it was measured with the research
//! tool on a game with every level cleared:
//!
//! * `research mapscan`: leave a level on every point and read where the game draws Mario
//!   (his sprite; the camera on the world map).
//! * `research pathscan` (CLEAR_ALL=1): from every point press each direction and follow
//!   Mario: the points ($A840) he passes and where. A walk counts only if it started where
//!   the point really is; corners and zone entrances come from what such walks pass.
//!
//! A point the game uses on two maps (a zone's point seen at the zone's entrance on the
//! world map) is two spots; the link between them is a door. Leaving a zone, Mario walks
//! off its picture: that exit is a spot at the edge, linked by a door to where he comes out.
//! Spots drawn on the same place of one map were merged (the map's own point kept).
//!
//! Positions are pixels on the map's picture (512 x 512 for the world map, 160 x 144 for a
//! zone map): the middle of Mario's feet.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnMap {
    World,
    /// Index into `zone_maps::ZONES`.
    Zone(usize),
}

impl OnMap {
    /// 0 = the world map, 1.. = zone map 1.. (as kept in text files).
    pub fn number(self) -> usize {
        match self {
            OnMap::World => 0,
            OnMap::Zone(z) => z + 1,
        }
    }

    /// Back from `number`; numbers past the last zone are the world map.
    pub fn from_number(n: usize) -> OnMap {
        if n == 0 || n > crate::zone_maps::ZONES.len() {
            OnMap::World
        } else {
            OnMap::Zone(n - 1)
        }
    }
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
    w(0x00, 204, 394),
    w(0x01, 140, 330),
    w(0x02, 92, 314),
    w(0x03, 140, 258),
    w(0x04, 168, 258),
    w(0x05, 96, 122),
    w(0x06, 212, 148),
    w(0x07, 296, 154),
    w(0x08, 340, 130),
    w(0x09, 384, 130),
    w(0x0A, 356, 282),
    w(0x0B, 404, 274),
    w(0x0C, 356, 386),
    w(0x0D, 440, 380),
    w(0x0E, 284, 354),
    w(0x0F, 356, 370),
    w(0x10, 252, 298),
    w(0x11, 280, 226),
    w(0x12, 204, 354),
    w(0x15, 280, 348),
    w(0x16, 244, 354),
    w(0x18, 212, 154),
    w(0x66, 472, 422),
    w(0x78, 424, 150),
    w(0x7A, 328, 90),
    z(TREE, 0x19, 48, 130),
    z(TREE, 0x1F, 76, 54),
    z(TREE, 0x20, 76, 30),
    z(TREE, 0x21, 112, 62),
    z(TREE, 0x22, 88, 106),
    z(TREE, 0x23, 48, 106),
    z(TREE, 0x24, 44, 62),
    z(TREE, 0x26, 148, 114),
    z(PUMPKIN, 0x1C, 83, 109),
    z(PUMPKIN, 0x28, 83, 85),
    z(PUMPKIN, 0x29, 99, 65),
    z(PUMPKIN, 0x2A, 55, 65),
    z(PUMPKIN, 0x2B, 83, 53),
    z(MACRO, 0x1B, 6, 116),
    z(MACRO, 0x32, 18, 116),
    z(MACRO, 0x33, 34, 100),
    z(MACRO, 0x34, 86, 100),
    z(MACRO, 0x35, 94, 60),
    z(MACRO, 0x6A, 18, 60),
    z(SPACE, 0x1A, 59, 116),
    z(SPACE, 0x3C, 75, 76),
    z(SPACE, 0x3D, 59, 28),
    z(SPACE, 0x3E, 75, 68),
    z(SPACE, 0x40, 59, 92),
    z(SPACE, 0x44, 147, 28),
    z(TURTLE, 0x1E, 74, 6),
    z(TURTLE, 0x46, 74, 30),
    z(TURTLE, 0x48, 114, 86),
    z(TURTLE, 0x49, 90, 110),
    z(MARIO, 0x1D, 36, 108),
    z(MARIO, 0x50, 60, 108),
    z(MARIO, 0x51, 80, 88),
    z(MARIO, 0x52, 80, 68),
    z(MARIO, 0x53, 80, 12),
    z(CASTLE, 0x5C, 81, 133),
    z(CASTLE, 0x64, 105, 133),
    z(BONUS, 0x17, 6, 127),
    z(BONUS, 0x6E, 40, 127),
    z(BONUS, 0x6F, 64, 127),
    z(BONUS, 0x70, 88, 127),
    z(BONUS, 0x71, 112, 127),
];

/// Which spots Mario walks between (indices into `SPOTS`) and the corners of the road
/// between them (pixels on their picture; none where it is straight or was not measured);
/// between two maps, a door.
pub const LINKS: &[(usize, usize, &[(i32, i32)])] = &[
    (1, 2, &[(92, 330)]),
    (1, 3, &[]),
    (1, 18, &[(156, 330), (156, 354)]),
    (2, 25, &[]),
    (2, 30, &[]),
    (3, 4, &[]),
    (3, 5, &[(76, 258), (76, 170), (60, 170), (60, 122)]),
    (3, 21, &[(140, 154), (156, 154), (153, 148), (176, 148), (181, 156)]),
    (5, 44, &[]),
    (6, 21, &[]),
    (6, 38, &[]),
    (6, 39, &[]),
    (7, 8, &[(293, 140), (300, 140), (300, 124)]),
    (7, 21, &[(276, 154), (271, 148), (240, 148), (243, 156)]),
    (8, 9, &[]),
    (8, 10, &[(340, 250), (356, 250)]),
    (9, 33, &[]),
    (9, 34, &[]),
    (10, 11, &[(404, 282)]),
    (10, 15, &[]),
    (11, 54, &[]),
    (11, 55, &[]),
    (12, 13, &[(372, 386), (368, 380)]),
    (12, 15, &[]),
    (12, 50, &[]),
    (13, 51, &[]),
    (15, 20, &[(244, 370)]),
    (17, 18, &[(276, 224), (276, 256), (248, 256), (252, 338), (204, 338)]),
    (18, 20, &[]),
    (18, 60, &[]),
    (19, 20, &[]),
    (20, 61, &[]),
    (25, 30, &[]),
    (26, 28, &[(84, 62)]),
    (26, 29, &[(72, 53), (72, 100)]),
    (26, 31, &[(68, 62)]),
    (27, 28, &[]),
    (27, 31, &[]),
    (29, 30, &[]),
    (33, 34, &[]),
    (34, 35, &[]),
    (35, 36, &[]),
    (36, 37, &[(63, 53)]),
    (38, 39, &[]),
    (39, 40, &[]),
    (39, 43, &[]),
    (40, 41, &[(50, 100), (47, 92), (70, 92), (75, 100)]),
    (41, 42, &[(118, 100), (115, 92), (122, 92), (122, 52), (114, 52), (117, 60)]),
    (42, 43, &[(78, 60), (73, 52), (50, 52), (53, 60)]),
    (44, 48, &[]),
    (45, 46, &[(75, 44)]),
    (45, 48, &[(75, 92)]),
    (50, 51, &[]),
    (51, 52, &[(70, 80)]),
    (52, 53, &[]),
    (54, 55, &[]),
    (55, 56, &[(56, 90), (76, 90), (76, 83)]),
    (56, 57, &[]),
    (57, 58, &[]),
    (59, 60, &[]),
    (61, 62, &[(11, 127), (8, 121), (23, 121)]),
    (62, 63, &[]),
    (63, 64, &[]),
    (64, 65, &[]),
];

/// The links drawn on `map` (both ends on it), each as the road from one spot to the other.
pub fn links_on(map: OnMap) -> impl Iterator<Item = Vec<(i32, i32)>> {
    LINKS.iter().filter(move |&&(a, b, _)| SPOTS[a].map == map && SPOTS[b].map == map).map(|&(a, b, bends)| {
        let mut road = vec![(SPOTS[a].x, SPOTS[a].y)];
        road.extend_from_slice(bends);
        road.push((SPOTS[b].x, SPOTS[b].y));
        road
    })
}

/// Where spot `i` is a door to: the spot at the other end of a link to another map.
pub fn door(i: usize) -> Option<&'static Spot> {
    LINKS.iter().filter_map(|&(a, b, _)| if a == i { Some(b) } else if b == i { Some(a) } else { None }).map(|o| &SPOTS[o]).find(|o| o.map != SPOTS[i].map)
}

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
    fn every_spot_is_a_point_on_its_picture_once_per_map() {
        for (i, s) in SPOTS.iter().enumerate() {
            assert!((s.point as usize) < POINT_COUNT);
            assert!(!SPOTS[..i].iter().any(|t| (t.point, t.map) == (s.point, s.map)), "point {:02x} twice on one map", s.point);
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
    fn links_join_spots_and_every_zone_has_a_door_in_and_out() {
        for &(a, b, bends) in LINKS {
            assert!(a < SPOTS.len() && b < SPOTS.len() && a != b);
            assert!(bends.is_empty() || SPOTS[a].map == SPOTS[b].map, "a door has no road");
        }
        for z in 0..ZONES.len() {
            let doors = LINKS.iter().filter(|&&(a, b, _)| (SPOTS[a].map == OnMap::Zone(z)) != (SPOTS[b].map == OnMap::Zone(z))).count();
            // the bonus game and Wario's castle have one way in and out; the others two
            assert!(doors >= 1, "{}", ZONES[z].name);
        }
    }

    #[test]
    fn each_zone_has_its_viewing_point() {
        for (i, zm) in ZONES.iter().enumerate() {
            assert!(on(OnMap::Zone(i)).any(|s| s.point == zm.point), "{}", zm.name);
        }
    }
}
