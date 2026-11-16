//! Pictures captured from the running game: a level's blocks, the world map's tiles, and
//! what each object type looks like. The editor draws with these.
use crate::driver::GameDriver;
use crate::level_start::Start;
use crate::patch::is_expanded;
use crate::{blank_level, extra_levels};
use crate::levels::LEVELS;
use crate::live::player_position;
use crate::objects::{read, store, Obj};
use crate::ram_map::{cart_ram_offset, BLOCK_DEFS, BLOCK_DEFS_LEN};
use gb_core::{Buttons, Gb};

/// Shade (0-3) of pixel (x, y) of an 8x8 background tile, with the game's tile addressing.
fn tile_pixel(vram: &[u8], lcdc: u8, bgp: u8, tile: u8, x: usize, y: usize) -> u8 {
    let base = if lcdc & 0x10 != 0 { tile as usize * 16 } else { (0x1000 + (tile as i8 as i32) * 16) as usize };
    let a = base + y * 2;
    let bit = 7 - x;
    let c = ((vram[a + 1] >> bit) & 1) << 1 | ((vram[a] >> bit) & 1);
    (bgp >> (c * 2)) & 3
}

/// Everything needed to draw a level: its blocks, the block definitions and the tile
/// graphics the game loads for it, captured by letting the game itself load the level.
#[derive(Clone)]
pub struct LevelArt {
    pub metatiles: Vec<u8>, // 128 * 4 tile numbers
    pub vram: Vec<u8>,
    pub lcdc: u8,
    pub bgp: u8,
}

impl LevelArt {
    pub fn capture(gb: &Gb) -> LevelArt {
        let from = cart_ram_offset(BLOCK_DEFS);
        LevelArt {
            metatiles: gb.cart().ram()[from..from + BLOCK_DEFS_LEN].to_vec(),
            vram: gb.video().vram().to_vec(),
            lcdc: gb.video().lcdc(),
            bgp: gb.video().bgp(),
        }
    }

    /// Shade (0-3) of pixel (x, y) of block `id`.
    pub fn block_pixel(&self, id: u8, x: usize, y: usize) -> u8 {
        let q = (y / 8) * 2 + x / 8;
        let tile = self.metatiles[(id as usize & 0x7F) * 4 + q];
        tile_pixel(&self.vram, self.lcdc, self.bgp, tile, x % 8, y % 8)
    }
}

/// The tile graphics the game shows the world map with (captured from the running game).
#[derive(Clone)]
pub struct WorldArt {
    pub vram: Vec<u8>,
    pub lcdc: u8,
    pub bgp: u8,
}

impl WorldArt {
    pub fn capture(gb: &Gb) -> WorldArt {
        WorldArt { vram: gb.video().vram().to_vec(), lcdc: gb.video().lcdc(), bgp: gb.video().bgp() }
    }

    pub fn pixel(&self, tile: u8, x: usize, y: usize) -> u8 {
        tile_pixel(&self.vram, self.lcdc, self.bgp, tile, x, y)
    }
}

/// What an object type looks like in a level: shades 0-3, None = transparent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Look {
    pub w: usize,
    pub h: usize,
    pub px: Vec<Option<u8>>,
    /// Where the picture's top-left sits relative to the object's position.
    pub dx: i32,
    pub dy: i32,
    /// The sprite tile sets it uses (`level_graphics::SETS` bits): it looks the same in
    /// levels that load the same graphics into those.
    pub sets: u8,
}

fn sprites(g: &Gb) -> Vec<[u8; 4]> {
    let oam = g.video().oam();
    (0..40).map(|i| [oam[i * 4], oam[i * 4 + 1], oam[i * 4 + 2], oam[i * 4 + 3]]).filter(|s| s[0] != 0 && s[0] < 160).collect()
}

/// The start of `level` with no objects, Mario having walked right a little, and the sprites
/// over the next frames: what every capture in that level compares with, the same for every
/// object type, so it is played once. In an expanded copy the level is played as a blank level
/// made from it (`blank_level`: its graphics, catalogue and tables, a flat floor), as Mario
/// cannot walk on at the start of every level (a wall in level 18). `map_state` is a machine
/// standing on the world map; `rom`/`original` the cartridge images.
pub struct Baseline {
    /// The sprites at the end of the walk and on each frame after it.
    sprites: Vec<Vec<[u8; 4]>>,
    /// Where an object goes: just past the right edge of the screen (the game brings objects
    /// in as the view scrolls), at Mario's height.
    at: (u16, u16),
}

/// The level's map and start as captures play it (see `Baseline`), with these objects.
fn capture_rom(rom: &[u8], original: &[u8], level: usize, objs: Vec<Obj>) -> Option<Vec<u8>> {
    if is_expanded(rom) {
        let mut e = extra_levels::based_on(rom, level as u8);
        let (map, start) = blank_level::blank(&e.map, &Start::read(&e.header));
        e.map = map;
        start.write(&mut e.header);
        e.objs = objs;
        let mut r = rom.to_vec();
        extra_levels::apply(&mut r, &extra_levels::writes_for(rom, &e));
        Some(r)
    } else {
        let mut lists: Vec<Vec<Obj>> = (0..LEVELS).map(|l| read(rom, l).0).collect();
        lists[level] = objs;
        let mut r = rom.to_vec();
        store(&mut r, original, &lists).ok()?;
        Some(r)
    }
}

fn walk(map_state: &Gb, r: &[u8], level: usize) -> Option<Gb> {
    let mut g = map_state.clone();
    g.cart_mut().set_rom(r);
    if !g.enter_level(level as u8) {
        return None;
    }
    for f in 0..150 {
        g.run_frame_with(if f < 130 { Buttons::RIGHT } else { Buttons::NONE });
        g.clear_audio();
    }
    Some(g)
}

/// Play `level`'s capture start with no objects and keep its sprites for `frames` frames more.
pub fn baseline(map_state: &Gb, rom: &[u8], original: &[u8], level: usize, frames: u32) -> Option<Baseline> {
    let r = capture_rom(rom, original, level, Vec::new())?;
    let mut g = map_state.clone();
    g.cart_mut().set_rom(&r);
    if !g.enter_level(level as u8) {
        return None;
    }
    let (mx, my) = player_position(&mut g);
    let mut g = walk(map_state, &r, level)?;
    let mut sprites = vec![self::sprites(&g)];
    for _ in 0..frames {
        g.run_frame_with(Buttons::NONE);
        g.clear_audio();
        sprites.push(self::sprites(&g));
    }
    Some(Baseline { sprites, at: (((mx + 216) / 8) * 8, (my / 8) * 8) })
}

/// The same start as `base` with one object of type `kind`.
fn with_object(map_state: &Gb, rom: &[u8], original: &[u8], level: usize, kind: u8, base: &Baseline) -> Option<(Gb, Obj)> {
    let obj = Obj { x: base.at.0, y: base.at.1, kind, flag: false };
    let r = capture_rom(rom, original, level, vec![obj])?;
    Some((walk(map_state, &r, level)?, obj))
}

/// Every sprite tile an object of type `kind` shows in `level` over `frames` frames after it
/// came in (see `Baseline`), sorted. Mario is invincible, so an enemy reaching him does not
/// change his sprites.
pub fn object_tiles(map_state: &Gb, rom: &[u8], original: &[u8], level: usize, kind: u8, frames: u32) -> Vec<u8> {
    match baseline(map_state, rom, original, level, frames) {
        Some(base) => object_tiles_with(map_state, rom, original, level, kind, &base),
        None => Vec::new(),
    }
}

/// `object_tiles` against a baseline already played (as many frames as it has).
pub fn object_tiles_with(map_state: &Gb, rom: &[u8], original: &[u8], level: usize, kind: u8, base: &Baseline) -> Vec<u8> {
    let mut safe = rom.to_vec();
    crate::cheats::set_invincible(&mut safe, true);
    let Some((mut g, _)) = with_object(map_state, &safe, original, level, kind, base) else { return Vec::new() };
    let mut tiles = Vec::new();
    for still in &base.sprites {
        let tall = g.video().lcdc() & 0x04 != 0;
        for s in sprites(&g).into_iter().filter(|s| !still.contains(s)) {
            let t = if tall { s[2] & 0xFE } else { s[2] };
            for t in if tall { vec![t, t | 1] } else { vec![t] } {
                if !tiles.contains(&t) {
                    tiles.push(t);
                }
            }
        }
        g.run_frame_with(Buttons::NONE);
        g.clear_audio();
    }
    tiles.sort_unstable();
    tiles
}

/// Let the game show one object of type `kind` alone beside Mario at the start of `level`,
/// and cut out the sprites it draws (compared with the same level with no objects).
pub fn capture_look(map_state: &Gb, rom: &[u8], original: &[u8], level: usize, kind: u8) -> Option<Look> {
    capture_look_with(map_state, rom, original, level, kind, &baseline(map_state, rom, original, level, 0)?)
}

/// `capture_look` against a baseline already played.
pub fn capture_look_with(map_state: &Gb, rom: &[u8], original: &[u8], level: usize, kind: u8, base: &Baseline) -> Option<Look> {
    let (mut g1, obj) = with_object(map_state, rom, original, level, kind, base)?;
    let still = &base.sprites[0];
    let new: Vec<[u8; 4]> = sprites(&g1).into_iter().filter(|s| !still.contains(s)).collect();
    if std::env::var("LOOK_DEBUG").is_ok() {
        eprintln!("obj {:?} rec {:02x?} base {:?} one {:?}", obj, &g1.cart().ram()[0xB00..0xB12], still, sprites(&g1));
    }
    if new.is_empty() {
        return None;
    }
    let (w, h, px, x0, y0) = cut(&g1, &new)?;
    // screen position of the object = its world position minus the camera; the camera is
    // where Mario's world X/Y minus his sprite's screen place puts it (OAM is screen + 8/16)
    let (cx, cy) = player_position(&mut g1);
    let oam = g1.video().oam();
    let cam_x = cx as i32 - (oam[1] as i32 - 8);
    let cam_y = cy as i32 - (oam[0] as i32 - 16);
    let dx = (x0 - 8 + cam_x) - obj.x as i32;
    let dy = (y0 - 16 + cam_y) - obj.y as i32;
    let tall = g1.video().lcdc() & 0x04 != 0;
    let sets = new.iter().fold(0, |m, s| m | crate::level_graphics::set_of(s[2]) | if tall { crate::level_graphics::set_of(s[2] | 1) } else { 0 });
    Some(Look { w, h, px, dx: dx.clamp(-32, 32), dy: dy.clamp(-32, 32), sets })
}

/// Mario as he stands at the start of level 0 (all the sprites then are his); `dx`/`dy`
/// from his position ($FFC2, $FFC0) to the picture's top-left.
pub fn capture_mario(map_state: &Gb) -> Option<Look> {
    let mut g = map_state.clone();
    if !g.enter_level(0) {
        return None;
    }
    let all = sprites(&g);
    let (w, h, px, x0, y0) = cut(&g, &all)?;
    let at = |g: &mut Gb, a: u16| g.read(a) as i32 | (g.read(a + 1) as i32) << 8;
    // the screen's top-left in level pixels: the camera ($FFCA, $FFC8) is its middle
    let (sx, sy) = (at(&mut g, 0xFFCA) - 0x50, at(&mut g, 0xFFC8) - 0x48);
    let dx = x0 - 8 + sx - at(&mut g, 0xFFC2);
    let dy = y0 - 16 + sy - at(&mut g, 0xFFC0);
    Some(Look { w, h, px, dx, dy, sets: 0 })
}

/// The picture the sprites `new` draw: width, height, shades, and its top-left in OAM terms.
fn cut(g: &Gb, new: &[[u8; 4]]) -> Option<(usize, usize, Vec<Option<u8>>, i32, i32)> {
    let tall = g.video().lcdc() & 0x04 != 0;
    let sh = if tall { 16 } else { 8 };
    let x0 = new.iter().map(|s| s[1] as i32).min()?;
    let y0 = new.iter().map(|s| s[0] as i32).min()?;
    let x1 = new.iter().map(|s| s[1] as i32 + 8).max()?;
    let y1 = new.iter().map(|s| s[0] as i32 + sh).max()?;
    let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
    if w > 64 || h > 64 {
        return None;
    }
    let vram = g.video().vram();
    let mut px = vec![None; w * h];
    for s in new {
        let mut tile = s[2] as usize;
        if tall {
            tile &= 0xFE;
        }
        let pal = g.video().obp((s[3] >> 4) & 1);
        for row in 0..sh as usize {
            let r = if s[3] & 0x40 != 0 { sh as usize - 1 - row } else { row };
            let a = tile * 16 + r * 2;
            for col in 0..8usize {
                let bit = if s[3] & 0x20 != 0 { col } else { 7 - col };
                let c = ((vram[a + 1] >> bit) & 1) << 1 | ((vram[a] >> bit) & 1);
                if c == 0 {
                    continue;
                }
                let x = (s[1] as i32 - x0) as usize + col;
                let y = (s[0] as i32 - y0) as usize + row;
                px[y * w + x] = Some((pal >> (c * 2)) & 3);
            }
        }
    }
    Some((w, h, px, x0, y0))
}
