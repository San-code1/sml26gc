//! The port's own world map, made in the editor: points anywhere on the world picture or on
//! a zone map's picture, the paths between them and which level each point opens. The game
//! uses it instead of the cartridge's map (whose points and paths are fixed inside its code).
//! A path between points on two different pictures is a door: on its end without a level,
//! A or Start takes Mario to the other picture.
//!
//! Kept as plain text (`host` saves it next to the cartridge copy as `<name>.points.txt`):
//!   point <x> <y> <level or -> [map]   one per line, numbered from 0 in order; map 0 (or
//!                                      none) = the world map, 1.. = zone map 1..
//!   path <a> <b>                       two point numbers
//!   start <n>                          where Mario starts
use crate::original_points::{OnMap, SPOTS};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    /// Pixels on its map's picture (512 x 512 for the world map, 160 x 144 for a zone map).
    pub x: i32,
    pub y: i32,
    pub level: Option<u8>,
    pub map: OnMap,
}

impl Point {
    /// A point on the world map.
    pub fn new(x: i32, y: i32, level: Option<u8>) -> Point {
        Point { x, y, level, map: OnMap::World }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CustomMap {
    pub points: Vec<Point>,
    pub paths: Vec<(usize, usize)>,
    pub start: usize,
}

impl CustomMap {
    pub fn parse(text: &str) -> CustomMap {
        let mut m = CustomMap::default();
        for line in text.lines() {
            let w: Vec<&str> = line.split_whitespace().collect();
            match w.as_slice() {
                ["point", x, y, l, rest @ ..] if rest.len() <= 1 => m.points.push(Point {
                    x: x.parse().unwrap_or(0),
                    y: y.parse().unwrap_or(0),
                    level: l.parse::<u8>().ok().filter(|&n| n < 32),
                    map: OnMap::from_number(rest.first().and_then(|v| v.parse().ok()).unwrap_or(0)),
                }),
                ["path", a, b] => {
                    if let (Ok(a), Ok(b)) = (a.parse(), b.parse()) {
                        m.paths.push((a, b));
                    }
                }
                ["start", n] => m.start = n.parse().unwrap_or(0),
                _ => {}
            }
        }
        let n = m.points.len();
        m.paths.retain(|&(a, b)| a < n && b < n && a != b);
        if m.start >= n {
            m.start = 0;
        }
        m
    }

    pub fn to_text(&self) -> String {
        let mut s = String::from("# world map made with the level editor\n");
        for p in &self.points {
            let l = p.level.map(|l| l.to_string()).unwrap_or_else(|| "-".into());
            match p.map {
                OnMap::World => s += &format!("point {} {} {}\n", p.x, p.y, l),
                m => s += &format!("point {} {} {} {}\n", p.x, p.y, l, m.number()),
            }
        }
        for (a, b) in &self.paths {
            s += &format!("path {a} {b}\n");
        }
        s += &format!("start {}\n", self.start);
        s
    }

    /// The game's own points where it draws them (`original_points`), each opening the level
    /// `levels` (the point-to-level table) gives it; no paths. Mario starts on point $00.
    pub fn from_original(levels: &[u8]) -> CustomMap {
        let points = SPOTS.iter().map(|s| Point { x: s.x, y: s.y, level: levels.get(s.point as usize).copied().filter(|&l| l < 32), map: s.map }).collect();
        let start = SPOTS.iter().position(|s| s.point == 0).unwrap_or(0);
        CustomMap { points, paths: Vec::new(), start }
    }

    /// Where a door at point `i` leads: the first point joined to it on another picture.
    pub fn door(&self, i: usize) -> Option<usize> {
        let map = self.points.get(i)?.map;
        self.paths.iter().filter_map(|&(a, b)| if a == i { Some(b) } else if b == i { Some(a) } else { None }).find(|&o| self.points[o].map != map)
    }

    pub fn linked(&self, a: usize, b: usize) -> bool {
        self.paths.iter().any(|&(x, y)| (x, y) == (a, b) || (x, y) == (b, a))
    }

    pub fn toggle_path(&mut self, a: usize, b: usize) {
        if a == b {
            return;
        }
        if self.linked(a, b) {
            self.paths.retain(|&(x, y)| !((x, y) == (a, b) || (x, y) == (b, a)));
        } else {
            self.paths.push((a, b));
        }
    }

    pub fn remove_point(&mut self, i: usize) {
        if i >= self.points.len() {
            return;
        }
        self.points.remove(i);
        self.paths.retain(|&(a, b)| a != i && b != i);
        for p in &mut self.paths {
            if p.0 > i {
                p.0 -= 1;
            }
            if p.1 > i {
                p.1 -= 1;
            }
        }
        if self.start > i || self.start >= self.points.len() {
            self.start = self.start.saturating_sub(1);
        }
    }

    /// The point joined to `from` on the same picture that lies most in direction (dx, dy).
    pub fn neighbour(&self, from: usize, dx: i32, dy: i32) -> Option<usize> {
        let p = self.points.get(from)?;
        let mut best: Option<(usize, f32)> = None;
        for &(a, b) in &self.paths {
            let o = if a == from { b } else if b == from { a } else { continue };
            let q = self.points[o];
            if q.map != p.map {
                continue;
            }
            let (vx, vy) = ((q.x - p.x) as f32, (q.y - p.y) as f32);
            let len = (vx * vx + vy * vy).sqrt().max(1.0);
            let score = (vx * dx as f32 + vy * dy as f32) / len;
            if score > 0.4 && best.map(|b| score > b.1).unwrap_or(true) {
                best = Some((o, score));
            }
        }
        best.map(|b| b.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_round_trip_and_cleanup() {
        let m = CustomMap::parse("# x\npoint 1 2 3\npoint 4 5 -\npoint 6 7 99\npath 0 1\npath 1 1\npath 0 7\nstart 9\n");
        assert_eq!(m.points.len(), 3);
        assert_eq!(m.points[2].level, None, "levels over 31 are dropped");
        assert_eq!(m.paths, vec![(0, 1)], "self and out-of-range paths are dropped");
        assert_eq!(m.start, 0, "a start past the points goes back to 0");
        assert_eq!(CustomMap::parse(&m.to_text()), m);
    }

    #[test]
    fn zone_points_doors_and_the_original_points() {
        let m = CustomMap::parse("point 10 20 3\npoint 30 40 - 2\npoint 50 60 4 2\npath 0 1\npath 1 2\n");
        assert_eq!(m.points[1].map, OnMap::Zone(1));
        assert_eq!(CustomMap::parse(&m.to_text()), m);
        assert_eq!(m.door(0), Some(1));
        assert_eq!(m.door(1), Some(0));
        assert_eq!(m.door(2), None, "a path on the same picture is no door");
        assert_eq!(m.neighbour(1, 1, 1), Some(2));
        assert_eq!(m.neighbour(0, 1, 1), None, "walking never crosses pictures");

        let mut levels = vec![crate::world::NO_LEVEL; crate::world::POINT_COUNT];
        levels[0] = 0;
        levels[0x20] = 5;
        let o = CustomMap::from_original(&levels);
        assert_eq!(o.points.len(), SPOTS.len());
        assert_eq!(o.points[o.start].level, Some(0));
        let i = SPOTS.iter().position(|s| s.point == 0x20).unwrap();
        assert_eq!((o.points[i].level, o.points[i].map), (Some(5), OnMap::Zone(0)));
    }
}
