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
