//! The port's own world map screen (used when the editor made a map for this cartridge):
//! the cartridge's world picture and zone map pictures with the person's points and paths,
//! Mario walking between them, Start / A entering the level of the point he stands on, and
//! on a door (a point joined to one on another picture) taking him to that picture. Where
//! the map and the cleared levels are kept is the caller's business.
use super::i18n::texts;
use super::overlay::{text_width, Canvas};
use cart_model::custom_map::CustomMap;
use cart_model::original_points::OnMap;
use cart_model::zone_maps::{self, ZONES};
use cart_model::{world, GameDriver};
use gb_core::ppu::{H, W};
use gb_core::Gb;
use std::collections::HashSet;

/// A map picture in shades, `w` x `h` pixels.
struct Picture {
    w: usize,
    h: usize,
    px: Vec<u8>,
}

pub struct MapView {
    pub map: CustomMap,
    pub cur: usize,
    walk: Option<(usize, usize, f32)>, // from, to, progress 0..1
    /// One per picture (`OnMap::number`), drawn when first needed.
    pictures: Vec<Option<Picture>>,
    pub cleared: HashSet<u8>,
    blink: u32,
    pub enabled: bool,
}

impl MapView {
    pub fn new(map: CustomMap, cleared: HashSet<u8>) -> MapView {
        MapView { cur: map.start, map, walk: None, pictures: (0..=ZONES.len()).map(|_| None).collect(), cleared, blink: 0, enabled: true }
    }

    /// Returns true when the level was not cleared before (so the progress needs saving).
    pub fn mark_cleared(&mut self, level: u8) -> bool {
        self.cleared.insert(level)
    }

    /// The picture Mario is on.
    fn here(&self) -> OnMap {
        self.map.points.get(self.cur).map(|p| p.map).unwrap_or(OnMap::World)
    }

    /// Draw the picture Mario is on, once: the world map with the map tiles the game has
    /// just loaded; a zone map with tiles captured by letting a copy of the console show it.
    pub fn prepare(&mut self, gb: &Gb) {
        let on = self.here();
        if self.pictures[on.number()].is_some() {
            return;
        }
        let rom = gb.cart().rom().to_vec();
        let (tiles, tw, th, art) = match on {
            OnMap::World => (world::read(&rom), world::W, world::H, cart_model::art::WorldArt::capture(gb)),
            OnMap::Zone(z) => {
                let mut copy = gb.clone();
                copy.show_map_at(ZONES[z].point);
                (zone_maps::read(&rom, z), zone_maps::W, zone_maps::H, cart_model::art::WorldArt::capture(&copy))
            }
        };
        let (w, h) = (tw * 8, th * 8);
        let mut px = vec![0; w * h];
        for ty in 0..th {
            for tx in 0..tw {
                let t = tiles[ty * tw + tx];
                for y in 0..8 {
                    for x in 0..8 {
                        px[(ty * 8 + y) * w + tx * 8 + x] = art.pixel(t, x, y);
                    }
                }
            }
        }
        self.pictures[on.number()] = Some(Picture { w, h, px });
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

    /// One frame of input: d-pad walks along a path, A or Start returns the level to enter
    /// (or, on a door with no level, goes through it).
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
            let level = self.map.points.get(self.cur).and_then(|p| p.level);
            if level.is_none() {
                if let Some(to) = self.map.door(self.cur) {
                    self.cur = to;
                }
            }
            return level;
        }
        None
    }

    pub fn draw(&self, out: &mut [u8], lang: usize) {
        let on = self.here();
        let Some(pic) = &self.pictures[on.number()] else { return };
        let (mx, my) = self.pos();
        let cam_x = (mx as i32 - W as i32 / 2).clamp(0, pic.w.saturating_sub(W) as i32);
        let cam_y = (my as i32 - H as i32 / 2).clamp(0, pic.h.saturating_sub(H) as i32);
        for y in 0..H.min(pic.h) {
            let row = (cam_y as usize + y) * pic.w + cam_x as usize;
            let n = W.min(pic.w);
            out[y * W..y * W + n].copy_from_slice(&pic.px[row..row + n]);
        }
        let mut c = Canvas { px: out };
        let sx = |x: i32| x - cam_x;
        let sy = |y: i32| y - cam_y;
        // paths on this picture: dotted
        for &(a, b) in &self.map.paths {
            let (pa, pb) = (self.map.points[a], self.map.points[b]);
            if pa.map != on || pb.map != on {
                continue;
            }
            let len = ((((pb.x - pa.x).pow(2) + (pb.y - pa.y).pow(2)) as f32).sqrt() as i32).max(1);
            let mut d = 0;
            while d <= len {
                let x = pa.x + (pb.x - pa.x) * d / len;
                let y = pa.y + (pb.y - pa.y) * d / len;
                c.rect(sx(x) - 1, sy(y) - 1, 2, 2, 3);
                d += 4;
            }
        }
        // points on this picture
        for (i, p) in self.map.points.iter().enumerate() {
            if p.map != on {
                continue;
            }
            let (x, y) = (sx(p.x), sy(p.y));
            match p.level {
                Some(l) => {
                    c.rect(x - 5, y - 5, 10, 10, 3);
                    let fill = if self.cleared.contains(&l) { 2 } else { 0 };
                    c.rect(x - 4, y - 4, 8, 8, fill);
                    c.rect(x - 1, y - 1, 2, 2, 3);
                }
                None if self.map.door(i).is_some() => {
                    // a door: a hollow ring
                    c.rect(x - 4, y - 4, 8, 8, 3);
                    c.rect(x - 2, y - 2, 4, 4, 0);
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
                let word = texts(lang).level;
                let done = if self.cleared.contains(&l) { " *" } else { "" };
                let t = format!("{word} {l:02}{done}");
                let w = text_width(&t) + 8;
                c.rect(2, 2, w, 11, 3);
                c.text(6, 4, &t, 0);
            }
        }
    }
}
