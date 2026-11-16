//! Pictures captured from the running game: a level's blocks, the world map's tiles, and
//! what each object type looks like. The editor draws with these.
use crate::driver::GameDriver;
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
#[derive(Clone)]
pub struct Look {
    pub w: usize,
    pub h: usize,
    pub px: Vec<Option<u8>>,
    /// Where the picture's top-left sits relative to the object's position.
    pub dx: i32,
    pub dy: i32,
}

fn sprites(g: &Gb) -> Vec<[u8; 4]> {
    let oam = g.video().oam();
    (0..40).map(|i| [oam[i * 4], oam[i * 4 + 1], oam[i * 4 + 2], oam[i * 4 + 3]]).filter(|s| s[0] != 0 && s[0] < 160).collect()
}

/// Let the game show one object of type `kind` alone beside Mario at the start of `level`,
/// and cut out the sprites it draws (compared with the same level with no objects).
/// `map_state` is a machine standing on the world map; `rom`/`original` the cartridge images.
pub fn capture_look(map_state: &Gb, rom: &[u8], original: &[u8], level: usize, kind: u8) -> Option<Look> {
    let lists: Vec<Vec<Obj>> = (0..LEVELS).map(|l| read(rom, l).0).collect();
    // Mario's start: let the game load the level once with no objects
    let mut empty = lists.clone();
    empty[level].clear();
    let mut r0 = rom.to_vec();
    store(&mut r0, original, &empty).ok()?;
    let mut g0 = map_state.clone();
    g0.cart_mut().set_rom(&r0);
    if !g0.enter_level(level as u8) {
        return None;
    }
    let (mx, my) = player_position(&mut g0);
    // just past the right edge of the screen: the game brings objects in as the view
    // scrolls, so Mario then walks right a little in both runs
    let obj = Obj { x: ((mx + 216) / 8) * 8, y: (my / 8) * 8, kind, flag: false };
    let mut one = lists;
    one[level] = vec![obj];
    let mut r1 = rom.to_vec();
    store(&mut r1, original, &one).ok()?;
    let mut g1 = map_state.clone();
    g1.cart_mut().set_rom(&r1);
    if !g1.enter_level(level as u8) {
        return None;
    }
    for f in 0..150 {
        let b = if f < 130 { Buttons::RIGHT } else { Buttons::NONE };
        g0.run_frame_with(b);
        g1.run_frame_with(b);
    }
    let base = sprites(&g0);
    let new: Vec<[u8; 4]> = sprites(&g1).into_iter().filter(|s| !base.contains(s)).collect();
    if std::env::var("LOOK_DEBUG").is_ok() {
        eprintln!("mario {mx},{my} obj {:?} rec {:02x?} base {:?} one {:?}", obj, &g1.cart().ram()[0xB00..0xB12], base, sprites(&g1));
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
    Some(Look { w, h, px, dx: dx.clamp(-32, 32), dy: dy.clamp(-32, 32) })
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
    Some(Look { w, h, px, dx, dy })
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
