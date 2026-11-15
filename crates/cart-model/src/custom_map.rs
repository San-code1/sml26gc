//! The port's own world map, made in the editor: points anywhere on the world picture, the
//! paths between them and which level each point opens. The game uses it instead of the
//! cartridge's map (whose points and paths are fixed inside its code).
//!
//! Saved as plain text next to the cartridge copy (`<name>.points.txt`):
//!   point <x> <y> <level or ->      one per line, numbered from 0 in order
//!   path <a> <b>                    two point numbers
//!   start <n>                       where Mario starts
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    /// Pixels on the 512 x 512 world picture.
    pub x: i32,
    pub y: i32,
    pub level: Option<u8>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CustomMap {
    pub points: Vec<Point>,
    pub paths: Vec<(usize, usize)>,
    pub start: usize,
}

impl CustomMap {
    /// The map file that goes with a cartridge file.
    pub fn file_for(rom: &Path) -> PathBuf {
        rom.with_extension("points.txt")
    }

    pub fn parse(text: &str) -> CustomMap {
        let mut m = CustomMap::default();
        for line in text.lines() {
            let w: Vec<&str> = line.split_whitespace().collect();
            match w.as_slice() {
                ["point", x, y, l] => m.points.push(Point {
                    x: x.parse().unwrap_or(0),
                    y: y.parse().unwrap_or(0),
                    level: l.parse::<u8>().ok().filter(|&n| n < 32),
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
            s += &format!("point {} {} {}\n", p.x, p.y, l);
        }
        for (a, b) in &self.paths {
            s += &format!("path {a} {b}\n");
        }
        s += &format!("start {}\n", self.start);
        s
    }

    pub fn load(rom: &Path) -> Option<CustomMap> {
        let t = std::fs::read_to_string(Self::file_for(rom)).ok()?;
        let m = Self::parse(&t);
        if m.points.is_empty() {
            None
        } else {
            Some(m)
        }
    }

    pub fn save(&self, rom: &Path) -> std::io::Result<()> {
        std::fs::write(Self::file_for(rom), self.to_text())
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

    /// The point joined to `from` that lies most in direction (dx, dy).
    pub fn neighbour(&self, from: usize, dx: i32, dy: i32) -> Option<usize> {
        let p = self.points.get(from)?;
        let mut best: Option<(usize, f32)> = None;
        for &(a, b) in &self.paths {
            let o = if a == from { b } else if b == from { a } else { continue };
            let q = self.points[o];
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
