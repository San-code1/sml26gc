//! The port's own world map screen (used when the editor made a map for this cartridge):
//! the cartridge's world picture with the person's points and paths, Mario walking between
//! them, and Start / A entering the level of the point he stands on.
use crate::menu::Canvas;
use crate::ppu::{H, W};
use gb_port::custom_map::CustomMap;
use gb_port::machine::Gb;
use gb_port::world;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub struct MapView {
    pub map: CustomMap,
    pub cur: usize,
    walk: Option<(usize, usize, f32)>, // from, to, progress 0..1
    picture: Vec<u8>,                  // 512 x 512 shades
    pub cleared: HashSet<u8>,
    progress_file: PathBuf,
    blink: u32,
    pub enabled: bool,
}

const PW: usize = world::W * 8;
const PH: usize = world::H * 8;

impl MapView {
    pub fn load(rom: &Path) -> Option<MapView> {
        let map = CustomMap::load(rom)?;
        let progress_file = gb_port::machine::save_path(rom).with_extension("progress.txt");
        let cleared = std::fs::read_to_string(&progress_file)
            .map(|t| t.split_whitespace().filter_map(|w| w.parse().ok()).collect())
            .unwrap_or_default();
        Some(MapView { cur: map.start, map, walk: None, picture: Vec::new(), cleared, progress_file, blink: 0, enabled: true })
    }

    pub fn mark_cleared(&mut self, level: u8) {
        if self.cleared.insert(level) {
            let mut v: Vec<u8> = self.cleared.iter().copied().collect();
            v.sort();
            let t: Vec<String> = v.iter().map(|l| l.to_string()).collect();
            if let Some(d) = self.progress_file.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            let _ = std::fs::write(&self.progress_file, t.join(" "));
        }
    }

    /// Draw the world picture once with the map tiles the game has just loaded.
    pub fn prepare(&mut self, gb: &Gb) {
        if !self.picture.is_empty() {
            return;
        }
        let rom = gb.bus.cart.rom_copy();
        let tiles = world::read(&rom);
        let art = world::WorldArt::capture(gb);
        self.picture = vec![0; PW * PH];
        for ty in 0..world::H {
            for tx in 0..world::W {
                let t = tiles[ty * world::W + tx];
                for y in 0..8 {
                    for x in 0..8 {
                        self.picture[(ty * 8 + y) * PW + tx * 8 + x] = art.pixel(t, x, y);
                    }
                }
            }
        }
    }

    fn pos(&self) -> (f32, f32) {
        let p = |i: usize| self.map.points.get(i).map(|p| (p.x as f32, p.y as f32)).unwrap_or((256.0, 256.0));
        match self.walk {
            Some((a, b, t)) => {
                let (pa, pb) = (p(a), p(b));
                (pa.0 + (pb.0 - pa.0) * t, pa.1 + (pb.1 - pa.1) * t)
            }
            None => p(self.cur),
        }
    }

    /// One frame of input: d-pad walks along a path, A or Start returns the level to enter.
    pub fn update(&mut self, left: bool, right: bool, up: bool, down: bool, enter: bool) -> Option<u8> {
        self.blink = self.blink.wrapping_add(1);
        if let Some((a, b, t)) = self.walk {
            let pa = self.map.points[a];
            let pb = self.map.points[b];
            let dist = (((pb.x - pa.x).pow(2) + (pb.y - pa.y).pow(2)) as f32).sqrt().max(1.0);
            let t = t + 1.5 / dist; // 1.5 px per frame
            if t >= 1.0 {
                self.cur = b;
                self.walk = None;
            } else {
                self.walk = Some((a, b, t));
            }
            return None;
        }
        let (dx, dy) = if left {
            (-1, 0)
        } else if right {
            (1, 0)
        } else if up {
            (0, -1)
        } else if down {
            (0, 1)
        } else {
            (0, 0)
        };
        if (dx, dy) != (0, 0) {
            if let Some(n) = self.map.neighbour(self.cur, dx, dy) {
                self.walk = Some((self.cur, n, 0.0));
            }
            return None;
        }
        if enter {
            return self.map.points.get(self.cur).and_then(|p| p.level);
        }
        None
    }

    pub fn draw(&self, out: &mut [u8], lang: usize) {
        let (mx, my) = self.pos();
        let cam_x = (mx as i32 - W as i32 / 2).clamp(0, (PW - W) as i32);
        let cam_y = (my as i32 - H as i32 / 2).clamp(0, (PH - H) as i32);
        if !self.picture.is_empty() {
            for y in 0..H {
                let row = (cam_y as usize + y) * PW + cam_x as usize;
                out[y * W..y * W + W].copy_from_slice(&self.picture[row..row + W]);
            }
        }
        let mut c = Canvas { px: out };
        let sx = |x: i32| x - cam_x;
        let sy = |y: i32| y - cam_y;
        // paths: dotted
        for &(a, b) in &self.map.paths {
            let (pa, pb) = (self.map.points[a], self.map.points[b]);
            let len = ((((pb.x - pa.x).pow(2) + (pb.y - pa.y).pow(2)) as f32).sqrt() as i32).max(1);
            let mut d = 0;
            while d <= len {
                let x = pa.x + (pb.x - pa.x) * d / len;
                let y = pa.y + (pb.y - pa.y) * d / len;
                c.rect(sx(x) - 1, sy(y) - 1, 2, 2, 3);
                d += 4;
            }
        }
        // points
        for p in &self.map.points {
            let (x, y) = (sx(p.x), sy(p.y));
            match p.level {
                Some(l) => {
                    c.rect(x - 5, y - 5, 10, 10, 3);
                    let fill = if self.cleared.contains(&l) { 2 } else { 0 };
                    c.rect(x - 4, y - 4, 8, 8, fill);
                    c.rect(x - 1, y - 1, 2, 2, 3);
                }
                None => c.rect(x - 2, y - 2, 4, 4, 3),
            }
        }
        // Mario: a small figure (cap, face, body) that bobs while standing
        let bob = if self.walk.is_none() && (self.blink / 20) % 2 == 0 { 1 } else { 0 };
        let (x, y) = (sx(mx as i32), sy(my as i32) - 15 - bob);
        c.rect(x - 5, y - 1, 11, 14, 0); // light outline so he stands out on any ground
        c.rect(x - 4, y, 9, 3, 3); // cap
        c.rect(x - 3, y + 3, 7, 3, 1); // face
        c.rect(x - 2, y + 4, 1, 1, 3); // eyes
        c.rect(x + 2, y + 4, 1, 1, 3);
        c.rect(x - 4, y + 6, 9, 4, 3); // body
        c.rect(x - 2, y + 7, 1, 1, 0); // buttons
        c.rect(x + 2, y + 7, 1, 1, 0);
        c.rect(x - 4, y + 10, 3, 2, 3); // feet
        c.rect(x + 2, y + 10, 3, 2, 3);
        // the level of the point Mario stands on
        if self.walk.is_none() {
            if let Some(l) = self.map.points.get(self.cur).and_then(|p| p.level) {
                let word = match lang {
                    crate::menu::LANG_ES => "NIVEL",
                    crate::menu::LANG_JA => "コース",
                    _ => "LEVEL",
                };
                let done = if self.cleared.contains(&l) { " *" } else { "" };
                let t = format!("{word} {l:02}{done}");
                let w = crate::menu::text_width(&t) + 8;
                c.rect(2, 2, w, 11, 3);
                c.text(6, 4, &t, 0);
            }
        }
    }
}
