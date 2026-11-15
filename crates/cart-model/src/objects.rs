//! Enemies and other objects placed in levels, as the cartridge stores them (read from
//! routine 03:6BC7, which the game calls when a level starts):
//!
//! * Bank 3, word table at $6037, one entry per level number ($A269) → a list of 3-byte
//!   objects ended by $FF.
//! * Object bytes b, c, d:
//!   - X = (b & $0F) · 256 + (c & $1F) · 8 pixels
//!   - Y = ((d >> 5) & 3) · 256 + (d & $1F) · 8 pixels
//!   - d bit 7: a flag the game keeps with the object
//!   - type (0-127) = b.4→bit6, b.7→bit5, b.6→bit4, b.5→bit3, c bits 5-7 → bits 0-2
//!   The game unpacks each one into 6 bytes at $AB06 (0, X page, X, flag, Y page, Y, type).
use crate::driver::GameDriver;
use crate::levels::LEVELS;

/// The bank the object loader reads: 3, or its twin $23 in an expanded copy
/// (see `levels::expand`).
pub fn bank(rom: &[u8]) -> usize {
    if crate::levels::objects_patched(rom) {
        0x23
    } else {
        3
    }
}

fn table(rom: &[u8]) -> usize {
    bank(rom) * 0x4000 + (0x6037 - 0x4000)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Obj {
    pub x: u16,
    pub y: u16,
    pub kind: u8,
    pub flag: bool,
}

impl Obj {
    pub fn from_bytes(b: u8, c: u8, d: u8) -> Obj {
        let kind = ((b >> 4) & 1) << 6 | ((b >> 7) & 1) << 5 | ((b >> 6) & 1) << 4 | ((b >> 5) & 1) << 3 | (c >> 5);
        Obj {
            x: (b as u16 & 0x0F) * 256 + (c as u16 & 0x1F) * 8,
            y: ((d as u16 >> 5) & 3) * 256 + (d as u16 & 0x1F) * 8,
            kind,
            flag: d & 0x80 != 0,
        }
    }

    pub fn to_bytes(&self) -> [u8; 3] {
        let x = self.x.min(4095);
        let y = self.y.min(1023);
        let k = self.kind & 0x7F;
        let b = (x / 256) as u8
            | ((k >> 6) & 1) << 4
            | ((k >> 3) & 1) << 5
            | ((k >> 4) & 1) << 6
            | ((k >> 5) & 1) << 7;
        let c = ((x % 256) / 8) as u8 | (k & 7) << 5;
        let d = ((y % 256) / 8) as u8 | (((y / 256) as u8) & 3) << 5 | (self.flag as u8) << 7;
        [b, c, d]
    }
}

fn ptr(rom: &[u8], level: usize) -> usize {
    let t = table(rom) + level * 2;
    bank(rom) * 0x4000 + ((rom[t] as usize | (rom[t + 1] as usize) << 8) & 0x3FFF)
}

/// A level's objects and the bytes its list takes (with the $FF end mark).
pub fn read(rom: &[u8], level: usize) -> (Vec<Obj>, usize) {
    let mut i = ptr(rom, level);
    let start = i;
    let mut v = Vec::new();
    while i + 2 < rom.len() && rom[i] != 0xFF && v.len() < 400 {
        v.push(Obj::from_bytes(rom[i], rom[i + 1], rom[i + 2]));
        i += 3;
    }
    (v, i + 1 - start)
}

/// Where the lists may go: in an expanded copy, $4000-$6036 of bank $23 (which the loader
/// doesn't use); otherwise the original lists' own space (they follow each other).
pub fn region_for(rom: &[u8], original: &[u8]) -> (usize, usize) {
    if crate::levels::objects_patched(rom) {
        return (0x23 * 0x4000, 0x6037 - 0x4000);
    }
    region(original)
}

/// Where the original lists sit (they follow each other): first start, total bytes.
pub fn region(original: &[u8]) -> (usize, usize) {
    let mut starts: Vec<usize> = (0..LEVELS).map(|l| ptr(original, l)).collect();
    starts.sort();
    starts.dedup();
    let start = starts[0];
    let end = starts.iter().map(|&s| {
        let mut i = s;
        while original[i] != 0xFF {
            i += 3;
        }
        i + 1
    }).max().unwrap_or(start);
    (start, end - start)
}

/// Bytes the lists need (levels that shared one list keep sharing it).
pub fn space(rom: &[u8], original: &[u8], lists: &[Vec<Obj>]) -> (usize, usize) {
    let mut seen: Vec<usize> = Vec::new();
    let mut need = 0;
    for l in 0..LEVELS {
        let p = ptr(rom, l);
        if !seen.contains(&p) {
            seen.push(p);
            need += lists[l].len() * 3 + 1;
        }
    }
    (need, region_for(rom, original).1)
}

/// Write every level's list back, one after another in the original lists' space, sorted
/// left to right (the order the game keeps them in), and point the table at them.
pub fn store(rom: &mut [u8], original: &[u8], lists: &[Vec<Obj>]) -> Result<String, String> {
    let (start, cap) = region_for(rom, original);
    let (need, _) = space(rom, original, lists);
    let b = bank(rom);
    let tbl = table(rom);
    if need > cap {
        return Err(format!("the objects need {need} bytes but the cartridge has {cap} for them; remove {} objects somewhere", (need - cap + 2) / 3));
    }
    let old: Vec<usize> = (0..LEVELS).map(|l| ptr(rom, l)).collect();
    let mut placed: Vec<(usize, u16)> = Vec::new(); // (old pointer, new address)
    let mut at = start;
    for l in 0..LEVELS {
        let addr = if let Some(&(_, a)) = placed.iter().find(|p| p.0 == old[l]) {
            a
        } else {
            let a = 0x4000 + (at - b * 0x4000) as u16;
            let mut objs = lists[l].clone();
            // The game's lists go page by page (256 px); keep their own order inside a page.
            objs.sort_by_key(|o| o.x / 256);
            for o in &objs {
                rom[at..at + 3].copy_from_slice(&o.to_bytes());
                at += 3;
            }
            rom[at] = 0xFF;
            at += 1;
            placed.push((old[l], a));
            a
        };
        let t = tbl + l * 2;
        rom[t] = addr as u8;
        rom[t + 1] = (addr >> 8) as u8;
    }
    Ok(format!("objects {need}/{cap} bytes"))
}

/// What an object type looks like in a level: shades 0-3, None = transparent.
#[derive(Clone)]
pub struct Look {
    pub w: usize,
    pub h: usize,
    pub px: Vec<Option<u8>>,
    /// Where the picture's top-left sits relative to the object's position.
    pub dx: i32,
    pub dy: i32,
}

fn sprites(g: &gb_core::Gb) -> Vec<[u8; 4]> {
    (0..40).map(|i| [g.bus.ppu.oam[i * 4], g.bus.ppu.oam[i * 4 + 1], g.bus.ppu.oam[i * 4 + 2], g.bus.ppu.oam[i * 4 + 3]]).filter(|s| s[0] != 0 && s[0] < 160).collect()
}

/// Let the game show one object of type `kind` alone beside Mario at the start of `level`,
/// and cut out the sprites it draws (compared with the same level with no objects).
/// `map_state` is a machine standing on the world map; `rom`/`original` the cartridge images.
pub fn capture_look(map_state: &gb_core::Gb, rom: &[u8], original: &[u8], level: usize, kind: u8) -> Option<Look> {
    let lists: Vec<Vec<Obj>> = (0..LEVELS).map(|l| read(rom, l).0).collect();
    // Mario's start: let the game load the level once with no objects
    let mut empty = lists.clone();
    empty[level].clear();
    let mut r0 = rom.to_vec();
    store(&mut r0, original, &empty).ok()?;
    let mut g0 = map_state.clone();
    g0.bus.cart.set_rom(&r0);
    if !g0.enter_level(level as u8) {
        return None;
    }
    let mx = (g0.bus.read(0xFFC3) as u16) << 8 | g0.bus.read(0xFFC2) as u16;
    let my = (g0.bus.read(0xFFC1) as u16) << 8 | g0.bus.read(0xFFC0) as u16;
    // just past the right edge of the screen: the game brings objects in as the view
    // scrolls, so Mario then walks right a little in both runs
    let obj = Obj { x: ((mx + 216) / 8) * 8, y: (my / 8) * 8, kind, flag: false };
    let mut one = lists;
    one[level] = vec![obj];
    let mut r1 = rom.to_vec();
    store(&mut r1, original, &one).ok()?;
    let mut g1 = map_state.clone();
    g1.bus.cart.set_rom(&r1);
    if !g1.enter_level(level as u8) {
        return None;
    }
    for f in 0..150 {
        let b = if f < 130 { gb_core::bus::BTN_RIGHT } else { 0 };
        g0.run_frame_with(b);
        g1.run_frame_with(b);
    }
    let base = sprites(&g0);
    let new: Vec<[u8; 4]> = sprites(&g1).into_iter().filter(|s| !base.contains(s)).collect();
    if std::env::var("LOOK_DEBUG").is_ok() {
        eprintln!(
            "mario {mx},{my} obj {:?} rec {:02x?} base {:?} one {:?}",
            obj,
            &g1.bus.cart.ram[0xB00..0xB12],
            base,
            sprites(&g1)
        );
    }
    if new.is_empty() {
        return None;
    }
    let tall = g1.bus.ppu.lcdc & 0x04 != 0;
    let sh = if tall { 16 } else { 8 };
    let x0 = new.iter().map(|s| s[1] as i32).min()?;
    let y0 = new.iter().map(|s| s[0] as i32).min()?;
    let x1 = new.iter().map(|s| s[1] as i32 + 8).max()?;
    let y1 = new.iter().map(|s| s[0] as i32 + sh).max()?;
    let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
    if w > 64 || h > 64 {
        return None;
    }
    let mut px = vec![None; w * h];
    for s in &new {
        let mut tile = s[2] as usize;
        if tall {
            tile &= 0xFE;
        }
        let pal = g1.bus.ppu.obp((s[3] >> 4) & 1);
        for row in 0..sh as usize {
            let r = if s[3] & 0x40 != 0 { sh as usize - 1 - row } else { row };
            let a = tile * 16 + r * 2;
            for col in 0..8usize {
                let bit = if s[3] & 0x20 != 0 { col } else { 7 - col };
                let c = ((g1.bus.ppu.vram[a + 1] >> bit) & 1) << 1 | ((g1.bus.ppu.vram[a] >> bit) & 1);
                if c == 0 {
                    continue;
                }
                let x = (s[1] as i32 - x0) as usize + col;
                let y = (s[0] as i32 - y0) as usize + row;
                px[y * w + x] = Some((pal >> (c * 2)) & 3);
            }
        }
    }
    // screen position of the object = its world position minus the camera; the camera is
    // where Mario's world X/Y minus his sprite's screen place puts it (OAM is screen + 8/16)
    let cx = ((g1.bus.read(0xFFC3) as i32) << 8) | g1.bus.read(0xFFC2) as i32;
    let cy = ((g1.bus.read(0xFFC1) as i32) << 8) | g1.bus.read(0xFFC0) as i32;
    let cam_x = cx - (g1.bus.ppu.oam[1] as i32 - 8);
    let cam_y = cy - (g1.bus.ppu.oam[0] as i32 - 16);
    let dx = (x0 - 8 + cam_x) - obj.x as i32;
    let dy = (y0 - 16 + cam_y) - obj.y as i32;
    Some(Look { w, h, px, dx: dx.clamp(-32, 32), dy: dy.clamp(-32, 32) })
}

/// The 6-byte records the game built at $AB06 for a loaded level, decoded back.
pub fn in_memory(cart_ram: &[u8], count: usize) -> Vec<Obj> {
    (0..count)
        .map(|k| {
            let r = &cart_ram[0xB06 + k * 6..0xB06 + k * 6 + 6];
            Obj { x: r[0] as u16 * 256 + r[1] as u16, flag: r[2] != 0, y: r[3] as u16 * 256 + r[4] as u16, kind: r[5] & 0x7F }
        })
        .collect()
}
