//! Block contents: objects of a level's list that sit on a block and come out of it when Mario
//! hits it from below. Each candidate is tried with Mario standing under its block (the
//! header's start moved there, as Play from here does), small and big, jumping into it.
use crate::probe::{probed, Probe};
use cart_model::level_start::{ground, Start};
use cart_model::levels::{self, COLS, LEVELS, ROWS};
use cart_model::live::map_in_memory;
use cart_model::ram_map::{GAME_MODE, LEVEL_MAP, PLAYER_X};
use cart_model::{objects, GameDriver};
use gb_core::{Buttons, GameBoy};
use std::path::Path;

/// Object types tried: the invisible ones of $10-$1F (the bells and the visible power-ups
/// left out).
const CANDIDATES: [u8; 10] = [0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x18, 0x19, 0x1A, 0x1F];
const SLOTS: u16 = 16;
const SLOT: u16 = 0xAD00;
const POWER: u16 = 0xA216;
const COINS: u16 = 0xA262;
const LIVES: u16 = 0xA22C;
/// Furthest Mario's feet may be below the block's bottom, in blocks, for a jump to reach it.
const REACH: usize = 4;
/// How far left of the block Mario starts, in pixels.
const WALK: u16 = 12 * 16;

type Gb = GameBoy<Probe>;

/// The map's cell in memory (cartridge RAM, then work RAM, `live::map_in_memory`).
fn cell_address(cell: usize) -> u16 {
    if cell < 0x1000 {
        LEVEL_MAP + cell as u16
    } else {
        0xC000 + (cell - 0x1000) as u16
    }
}

/// What came out of the block: the active object type and item byte (+$05, +$12) of a new
/// item, and the coins and lives before and after.
struct Outcome {
    item: Option<(u8, u8)>,
    block_after: u8,
    coins: (u8, u8),
    lives: (u8, u8),
    writer: Option<(usize, u16)>,
    /// How far Mario stood from the block's left when he jumped.
    off: i32,
}

fn mario_x(g: &mut Gb) -> u16 {
    (g.read(PLAYER_X + 1) as u16) << 8 | g.read(PLAYER_X) as u16
}

fn items(g: &mut Gb) -> Vec<(u8, u8)> {
    let slots: Vec<(u8, u8, u8)> = (0..SLOTS).map(|s| SLOT + s * 0x20).map(|s| (g.read(s + 8), g.read(s + 5), g.read(s + 0x12))).collect();
    slots.into_iter().filter(|&(used, _, item)| used != 0 && item & 0x80 != 0 && item != 0x80).map(|(_, kind, item)| (kind, item)).collect()
}

/// Mario (big when `big`) under cell `cell` of `level`, on the ground `feet`, the block there
/// made `block` when given; he jumps.
fn try_hit(base: &Gb, level: u8, cell: usize, feet: u16, big: bool, block: Option<u8>) -> Option<Outcome> {
    let mut g = base.clone();
    let x = (cell % COLS) as u16 * 16;
    let h = levels::header(g.cart().rom(), level as usize).at;
    // a screen to the left, so the object comes into view as the game spawns objects
    Start::standing(x.saturating_sub(WALK), feet).write(&mut g.cart_mut().rom_mut()[h..h + 8]);
    if !g.enter_level_anywhere(level) {
        return None;
    }
    for _ in 0..20 {
        g.run_frame_with(Buttons::NONE);
        g.clear_audio();
    }
    let (mut last, mut stuck) = (0, 0);
    for _ in 0..600 {
        let now = mario_x(&mut g);
        if now + 10 >= x {
            break;
        }
        stuck = if now == last { stuck + 1 } else { 0 };
        last = now;
        g.run_frame_with(if (8..24).contains(&stuck) { Buttons::RIGHT | Buttons::A } else { Buttons::RIGHT });
        g.clear_audio();
        if stuck >= 40 {
            stuck = 0;
        }
    }
    for _ in 0..30 {
        g.run_frame_with(Buttons::NONE);
        g.clear_audio();
    }
    let at = mario_x(&mut g);
    if big {
        g.write(POWER, 1);
    }
    if let Some(b) = block {
        g.write(cell_address(cell), b);
    }
    let before = items(&mut g);
    let (coins, lives) = (g.read(COINS), g.read(LIVES));
    g.observer_mut().watch = (0..SLOTS).map(|s| SLOT + s * 0x20 + 0x12).collect();
    let mut item = None;
    let mut came = 0;
    for f in 0..90 {
        g.run_frame_with(if f < 18 { Buttons::A } else { Buttons::NONE });
        g.clear_audio();
        if g.read(GAME_MODE) != 0x04 {
            break;
        }
        if item.is_none() {
            item = items(&mut g).into_iter().find(|i| !before.contains(i));
            if item.is_some() {
                came = f;
            }
        }
        if item.is_some() && f >= came + 25 {
            break;
        }
    }
    if let (Some((_, k)), Ok(dir)) = (item, std::env::var("BLOCKSHOT")) {
        let mut rgba = vec![0u8; g.screen().len() * 4];
        gb_core::palette::fb_to_rgba(g.screen(), &gb_core::palette::PALETTES[0], &mut rgba);
        let _ = host::png_out::write_rgba(&Path::new(&dir).join(format!("level{level:02}_item{:02x}_{}.png", k & 0x7F, cell)), gb_core::ppu::W, gb_core::ppu::H, &rgba);
    }
    let writer = g.observer().watch_log.iter().find(|w| w.3 & 0x80 != 0 && w.3 != 0x80).map(|w| (w.1, w.0));
    let block_after = map_in_memory(&g)[cell];
    Some(Outcome { item, block_after, coins: (coins, g.read(COINS)), lives: (lives, g.read(LIVES)), writer, off: at as i32 - x as i32 })
}

/// The ground under cell `cell` (feet y), when a jump from it reaches the block.
fn ground_under(map: &[u8], cell: usize) -> Option<u16> {
    let (col, row) = (cell % COLS, cell / COLS);
    let g = (row + 2..ROWS.min(row + 2 + REACH)).find(|&r| ground(map[r * COLS + col]) && ground(map[r * COLS + col + 1.min(COLS - 1 - col)]))?;
    Some(g as u16 * 16)
}

fn shown(o: &Option<Outcome>) -> String {
    match o {
        None => "could not enter".into(),
        Some(o) => {
            let item = o.item.map_or("nothing".to_string(), |(t, k)| format!("type {t:02x} item {:02x}", k & 0x7F));
            let writer = o.writer.map_or(String::new(), |(b, pc)| format!(" by {b:02x}:{pc:04x}"));
            format!("{item}{writer} (Mario {:+} px), block after {:02x}, coins {:02x}>{:02x}, lives {:02x}>{:02x}", o.off, o.block_after, o.coins.0, o.coins.1, o.lives.0, o.lives.1)
        }
    }
}

/// Levels `only` (all when empty): each candidate object, the block under it, and what
/// hitting that block gives Mario small and big; with `blocks`, also with the block made each
/// of them.
pub fn block_probe(rom_path: &Path, only: &[u16], blocks: &[u16]) {
    let Ok(rom) = std::fs::read(rom_path) else { return };
    let Some(mut base) = probed(rom_path) else { return };
    if !base.boot_to_map() {
        return;
    }
    let levels: Vec<u8> = if only.is_empty() { (0..LEVELS as u8).collect() } else { only.iter().map(|&l| l as u8).collect() };
    for level in levels {
        let mut seen = base.clone();
        if !seen.enter_level_anywhere(level) {
            println!("level {level:02}: could not enter");
            continue;
        }
        let map = map_in_memory(&seen);
        for o in objects::read(&rom, level as usize).0.into_iter().filter(|o| CANDIDATES.contains(&o.kind)) {
            // the object is half a block above its block, where the item comes out
            let cell = ((o.y as usize + 8) / 16) * COLS + o.x as usize / 16;
            let under = map.get(cell).copied().unwrap_or(0xFF);
            let Some(feet) = ground_under(&map, cell) else {
                println!("level {level:02} type {:02x} at {},{}: block {under:02x}, no ground to jump from", o.kind, o.x, o.y);
                continue;
            };
            for big in [false, true] {
                println!("level {level:02} type {:02x} at {},{}: block {under:02x}, {}: {}", o.kind, o.x, o.y, if big { "big" } else { "small" }, shown(&try_hit(&base, level, cell, feet, big, None)));
            }
            for &b in blocks {
                println!("    as block {b:02x}: {}", shown(&try_hit(&base, level, cell, feet, false, Some(b as u8))));
            }
        }
    }
}

/// The blocks around (x, y) of `level` as the game holds them (rows above and below).
pub fn block_view(rom_path: &Path, level: u8, x: u16, y: u16) {
    let Some(mut g) = probed(rom_path) else { return };
    if !g.boot_to_map() || !g.enter_level_anywhere(level) {
        return;
    }
    let map = map_in_memory(&g);
    let (c, r) = (x as usize / 16, (y as usize + 8) / 16);
    for row in r.saturating_sub(4)..(r + 5).min(ROWS) {
        let cells: Vec<String> = (c.saturating_sub(4)..(c + 5).min(COLS)).map(|col| format!("{}{:02x}", if (col, row) == (c, r) { '*' } else { ' ' }, map[row * COLS + col])).collect();
        println!("row {row:2}: {}", cells.join(""));
    }
}
