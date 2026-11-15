//! Looking at ROM data: free space per bank, a level's object list, the world map data as
//! a picture, and what each object type looks like.
use crate::console::load;
use cart_model::GameDriver;
use gb_core::palette::PALETTES;
use host::png_out::write_rgba;
use std::path::Path;

/// Longest run of $FF and of $00 in each bank (possible free space).
pub fn free_space(rom_path: &Path) {
    let Ok(rom) = std::fs::read(rom_path) else { return };
    for bank in 0..rom.len() / 0x4000 {
        let b = &rom[bank * 0x4000..(bank + 1) * 0x4000];
        let longest = |v: u8| {
            let (mut best, mut at, mut run) = (0, 0, 0);
            for (i, &x) in b.iter().enumerate() {
                if x == v {
                    run += 1;
                    if run > best {
                        best = run;
                        at = i + 1 - run;
                    }
                } else {
                    run = 0;
                }
            }
            (best, at)
        };
        let (ff, ffa) = longest(0xFF);
        let (zz, za) = longest(0x00);
        println!("bank {bank:02x}: FF run {ff} at {:04x}, 00 run {zz} at {:04x}", ffa + if bank == 0 { 0 } else { 0x4000 }, za + if bank == 0 { 0 } else { 0x4000 });
    }
}

/// Print a level's object list (bank 3, table $6037) and the 5-byte records the game
/// builds from it at $AB06 once the level has loaded.
pub fn objects(rom_path: &Path, level: u8) {
    let Some(mut gb) = load(rom_path) else { return };
    let rom = gb.cart().rom().to_vec();
    let t = 3 * 0x4000 + 0x2037 + level as usize * 2;
    let p = (rom[t] as usize | (rom[t + 1] as usize) << 8) - 0x4000 + 3 * 0x4000;
    let mut i = p;
    let mut raw = Vec::new();
    while rom[i] != 0xFF && raw.len() < 200 {
        raw.push((rom[i], rom[i + 1], rom[i + 2]));
        i += 3;
    }
    gb.boot_to_map();
    gb.enter_level(level);
    for (k, (b, c, d)) in raw.iter().enumerate() {
        let r: Vec<String> = (0..5).map(|j| format!("{:02x}", gb.cart().ram()[0xB06 + k * 5 + j])).collect();
        println!("{k:2}: {b:02x} {c:02x} {d:02x}   -> ram {}", r.join(" "));
    }
    println!("{} objects, list at file {p:05x}", raw.len());
}

/// Draw ROM bytes at `off` as an 8x8-tile map `width` tiles wide, with the tiles the game
/// has in VRAM on the world map, to test what the world map data looks like.
pub fn world_shot(rom_path: &Path, off: usize, len: usize, width: usize, png: &Path) {
    let Some(mut gb) = load(rom_path) else { return };
    gb.boot_to_map();
    let rom = gb.cart().rom().to_vec();
    let vram = gb.video().vram().to_vec();
    let lcdc = gb.video().lcdc();
    let bgp = gb.video().bgp();
    let h = len / width;
    let (w, hh) = (width * 8, h * 8);
    let mut rgba = vec![0u8; w * hh * 4];
    for ty in 0..h {
        for tx in 0..width {
            // width 64: the data is four 32x32 quadrants (top-left, top-right, bottom-left, bottom-right)
            let t = if width == 64 {
                let q = (ty / 32) * 2 + tx / 32;
                rom[off + q * 1024 + (ty % 32) * 32 + tx % 32]
            } else {
                rom[off + ty * width + tx]
            };
            let base = if lcdc & 0x10 != 0 { t as usize * 16 } else { (0x1000 + (t as i8 as i32) * 16) as usize };
            for y in 0..8 {
                for x in 0..8 {
                    let a = base + y * 2;
                    let bit = 7 - x;
                    let c = ((vram[a + 1] >> bit) & 1) << 1 | ((vram[a] >> bit) & 1);
                    let s = (bgp >> (c * 2)) & 3;
                    let col = PALETTES[0][s as usize];
                    let p = ((ty * 8 + y) * w + tx * 8 + x) * 4;
                    rgba[p] = (col >> 16) as u8;
                    rgba[p + 1] = (col >> 8) as u8;
                    rgba[p + 2] = col as u8;
                    rgba[p + 3] = 255;
                }
            }
        }
    }
    if let Err(e) = write_rgba(png, w, hh, &rgba) {
        eprintln!("{e}");
    }
}

/// Capture what each object type of a level looks like and save them side by side.
pub fn looks(rom_path: &Path, level: usize, png: &Path) {
    use cart_model::objects;
    let Some(mut gb) = load(rom_path) else { return };
    let rom = gb.cart().rom().to_vec();
    gb.boot_to_map();
    let mut kinds: Vec<u8> = objects::read(&rom, level).0.iter().map(|o| o.kind).collect();
    kinds.sort();
    kinds.dedup();
    let cell = 40usize;
    let w = cell * kinds.len().max(1);
    let mut rgba = vec![255u8; w * cell * 4];
    for (i, &k) in kinds.iter().enumerate() {
        let look = cart_model::art::capture_look(&gb, &rom, &rom, level, k);
        println!("type {k:02X}: {}", match &look { Some(l) => format!("{}x{} offset {},{}", l.w, l.h, l.dx, l.dy), None => "nothing drawn".into() });
        if let Some(l) = look {
            for y in 0..l.h.min(cell) {
                for x in 0..l.w.min(cell) {
                    if let Some(s) = l.px[y * l.w + x] {
                        let c = PALETTES[0][s as usize];
                        let p = (y * w + i * cell + x) * 4;
                        rgba[p] = (c >> 16) as u8;
                        rgba[p + 1] = (c >> 8) as u8;
                        rgba[p + 2] = c as u8;
                    }
                }
            }
        }
    }
    if let Err(e) = write_rgba(png, w, cell, &rgba) {
        eprintln!("{e}");
    }
}

/// Enter `level` from the point that opens it, leave it as if cleared, and picture the
/// map the game comes back to (for finding the zone maps).
pub fn map_after(rom_path: &Path, level: u8, png: &Path, at_point: Option<u8>) {
    use cart_model::ram_map::{GAME_MODE, MAP_POINT};
    let Some(mut gb) = load(rom_path) else { return };
    gb.boot_to_map();
    let entered = gb.enter_level_anywhere(level);
    let point = gb.read(MAP_POINT);
    for _ in 0..60 {
        gb.run_frame();
    }
    if let Some(p) = at_point {
        gb.write(MAP_POINT, p);
    }
    gb.back_to_map();
    let mut modes = Vec::new();
    for f in 0..600 {
        gb.run_frame();
        let m = gb.read(GAME_MODE);
        if modes.last().map(|&(_, x)| x != m).unwrap_or(true) {
            modes.push((f, m));
        }
    }
    println!(
        "level {level:02}: entered={entered} from point {point:02x}; back on point {:02x} at ({:02x},{:02x}); modes {:02x?}",
        gb.read(MAP_POINT),
        gb.read(0xA842),
        gb.read(0xA843),
        modes
    );
    host::png_out::save_png(&gb, png);
}

/// For every map point: leave a level as if standing on it and note where the game puts
/// Mario and which map it shows (maps told apart by their background tile map; one
/// picture per map in `dir`).
pub fn map_scan(rom_path: &Path, dir: &Path) {
    use cart_model::ram_map::{GAME_MODE, MAP_POINT};
    use cart_model::world::POINT_COUNT;
    let Some(mut base) = load(rom_path) else { return };
    base.boot_to_map();
    base.enter_level_anywhere(1);
    for _ in 0..60 {
        base.run_frame();
    }
    let mut maps: Vec<Vec<u8>> = Vec::new();
    for p in 0..POINT_COUNT as u8 {
        let mut gb = base.clone();
        gb.write(MAP_POINT, p);
        gb.back_to_map();
        let mut trail = Vec::new();
        for _ in 0..600 {
            gb.run_frame();
            gb.clear_audio();
            let m = gb.read(GAME_MODE);
            if trail.last() != Some(&m) {
                trail.push(m);
            }
        }
        let bg = gb.video().vram()[0x1800..0x2000].to_vec();
        let id = maps.iter().position(|m| *m == bg).unwrap_or_else(|| {
            maps.push(bg);
            host::png_out::save_png(&gb, &dir.join(format!("map{}.png", maps.len() - 1)));
            find_rows(gb.cart().rom(), &maps[maps.len() - 1], maps.len() - 1);
            maps.len() - 1
        });
        let world = on_world_picture(&mut gb);
        // Mario's sprite (tile $0A first) -> the middle of his feet on the screen
        let o = gb.video().oam().to_vec();
        let mario = (0..40).find(|&i| o[i * 4 + 2] == 0x0A && o[i * 4] != 0).map(|i| (o[i * 4 + 1] as i32, o[i * 4] as i32 - 4));
        let spot = match (mario, world) {
            (Some((x, y)), Some((cx, cy))) => Some(((cx - 80 + x).rem_euclid(512), (cy - 72 + y).rem_euclid(512))),
            (m, None) => m,
            _ => None,
        };
        println!("spot {p:02x} map {id} {spot:?}");
        if spot.is_none() {
            let v: Vec<(u8, u8, u8)> = (0..40).map(|i| (o[i * 4 + 1], o[i * 4], o[i * 4 + 2])).filter(|s| s.1 != 0 && s.1 < 160).collect();
            println!("  oam {v:02x?}");
        }
        println!(
            "point {p:02x}: map {id} point {:02x} at ({:02x},{:02x}) scroll ({},{}) world {:?} modes {:02x?}",
            gb.read(MAP_POINT),
            gb.read(0xA842),
            gb.read(0xA843),
            gb.read(0xFF43),
            gb.read(0xFF42),
            world,
            trail
        );
    }
}

/// Where in the ROM the rows of a background tile map appear as plain bytes (12-byte
/// pieces of each row; pieces made of one repeated tile are skipped).
fn find_rows(rom: &[u8], bg: &[u8], id: usize) {
    let mut hits = Vec::new();
    for row in 0..32 {
        for col in [0usize, 10, 20] {
            let piece = &bg[row * 32 + col..row * 32 + col + 12];
            if piece.iter().all(|&b| b == piece[0]) {
                continue;
            }
            if let Some(at) = rom.windows(12).position(|w| w == piece) {
                hits.push(format!("r{row}c{col}@{:02x}:{:04x}", at / 0x4000, 0x4000 + at % 0x4000));
            }
        }
    }
    println!("map {id}: {} plain rows {}", hits.len(), hits.iter().take(8).cloned().collect::<Vec<_>>().join(" "));
}

/// Where the middle of the screen is on the 512x512 world picture (pixels), when the
/// screen shows the world picture (the background matches it around the middle).
fn on_world_picture(gb: &mut gb_core::Gb) -> Option<(i32, i32)> {
    use cart_model::world;
    let pic = world::read(gb.cart().rom());
    let bg = gb.video().vram()[0x1800..0x2000].to_vec();
    let (bx, by) = ((gb.read(0xFF43) as usize + 80) % 256, (gb.read(0xFF42) as usize + 72) % 256);
    let (vc, vr) = (bx / 8, by / 8);
    let mut best = (0, 0, 0);
    for a in 0..2 {
        for b in 0..2 {
            let (x, y) = (vc + 32 * b, vr + 32 * a);
            let mut same = 0;
            for dy in 0..9usize {
                for dx in 0..9usize {
                    let v = bg[((vr + 28 + dy) % 32) * 32 + (vc + 28 + dx) % 32];
                    if v == pic[((y + 60 + dy) % 64) * 64 + (x + 60 + dx) % 64] {
                        same += 1;
                    }
                }
            }
            if same > best.0 {
                best = (same, x, y);
            }
        }
    }
    (best.0 >= 70).then(|| ((best.1 * 8 + bx % 8) as i32, (best.2 * 8 + by % 8) as i32))
}
