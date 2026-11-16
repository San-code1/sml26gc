//! Cheats and debug tools, tried in the emulator: Mario's power-up state, what hurts him,
//! items, the end of a level and the counters.
use crate::probe::{probed, Probe};
use cart_model::ram_map::{self, GAME_MODE, LEVEL_NUMBER, PLAYER_X, PLAYER_Y};
use cart_model::GameDriver;
use gb_core::{Buttons, GameBoy};
use std::path::Path;

type Pg = GameBoy<Probe>;

/// High RAM and the cartridge RAM the game keeps its state in ($A000-$A8FF).
fn state(g: &mut Pg) -> Vec<(u16, u8)> {
    (0xA000u16..0xA900).chain(0xFF80..0xFFFF).map(|a| (a, g.read(a))).collect()
}

fn word(g: &mut Pg, a: u16) -> u16 {
    g.read(a) as u16 | (g.read(a + 1) as u16) << 8
}

/// In level `level`, hold Right (jumping every `jump` frames when not 0) until Mario loses a
/// life or `frames` pass; print each write to the game mode with the instruction, and the
/// state bytes that changed in the last 8 frames before the mode left $04.
pub fn damage_probe(rom: &Path, level: u8, jump: u32, frames: u32, watch: &[u16]) {
    let Some(mut g) = probed(rom) else { return };
    if !g.boot_to_map() || !g.enter_level_anywhere(level) {
        println!("could not enter level {level}");
        return;
    }
    println!("level {} playing at x {:04x} y {:04x}", g.read(LEVEL_NUMBER), word(&mut g, PLAYER_X), word(&mut g, PLAYER_Y));
    let start = state(&mut g);
    g.observer_mut().watch = [&[GAME_MODE][..], watch].concat();
    let mut ring: Vec<Vec<(u16, u8)>> = Vec::new();
    for f in 0..frames {
        let b = if jump > 0 && f % jump < 14 { Buttons::RIGHT | Buttons::A } else { Buttons::RIGHT };
        g.run_frame_with(b);
        g.clear_audio();
        ring.push(state(&mut g));
        if ring.len() > 9 {
            ring.remove(0);
        }
        if g.read(GAME_MODE) != 0x04 {
            println!("frame {f}: mode {:02x} at x {:04x} y {:04x}", g.read(GAME_MODE), word(&mut g, PLAYER_X), word(&mut g, PLAYER_Y));
            break;
        }
    }
    for (pc, bank, a, v) in &g.observer().watch_log {
        println!("  write {a:04x} = {v:02x} by {bank:02x}:{pc:04x} ");
    }
    let (first, last) = (&ring[0], &ring[ring.len() - 1]);
    let moved: Vec<String> = first.iter().zip(last).zip(&start).filter(|((a, b), _)| a.1 != b.1).map(|(((a, x), (_, y)), (_, s))| format!("{a:04x}:{s:02x}/{x:02x}>{y:02x}")).collect();
    println!("changed in the last 8 frames (start/before>after): {}", moved.join(" "));
}

/// In level `level`, write `value` to `addr` once Mario plays, then for each of a few frames
/// print the shadow sprites ($A100: y, x, tile, attributes) near Mario and the byte.
pub fn poke_probe(rom: &Path, level: u8, addr: u16, value: u8) {
    let Some(mut g) = probed(rom) else { return };
    if !g.boot_to_map() || !g.enter_level_anywhere(level) {
        println!("could not enter level {level}");
        return;
    }
    let show = |g: &mut Pg, tag: &str| {
        let s: Vec<String> = (0..10u16).map(|i| (0..4).map(|k| g.read(0xA100 + i * 4 + k)).collect::<Vec<u8>>()).filter(|s| s[0] != 0).map(|s| format!("{:02x},{:02x}:{:02x}/{:02x}", s[0], s[1], s[2], s[3])).collect();
        println!("{tag}: {addr:04x}={:02x} mode {:02x} sprites {}", g.read(addr), g.read(GAME_MODE), s.join(" "));
    };
    for _ in 0..30 {
        g.run_frame_with(Buttons::NONE);
    }
    show(&mut g, "before");
    g.write(addr, value);
    for f in 0..120u32 {
        g.run_frame_with(if f > 60 && f < 70 { Buttons::RIGHT } else { Buttons::NONE });
        g.clear_audio();
        if f % 40 == 0 || f == 119 {
            show(&mut g, &format!("after {f}"));
        }
    }
}

/// Mario's sprites: the shadow sprites in use among the first eight (he is drawn first).
fn mario_tiles(g: &mut Pg) -> Vec<u8> {
    let s: Vec<[u8; 3]> = (0..8u16).map(|i| [g.read(0xA100 + i * 4), g.read(0xA100 + i * 4 + 2), 0]).collect();
    s.iter().filter(|s| s[0] != 0).map(|s| s[1]).collect()
}

/// In level `level`, hold Right and jump every `jump` frames; each time Mario's tiles change
/// in number or the game mode changes, print the frame, the tiles and the state bytes that
/// changed since 4 frames before (sprite buffer left out).
pub fn grow_probe(rom: &Path, level: u8, jump: u32, frames: u32) {
    let Some(mut g) = probed(rom) else { return };
    if !g.boot_to_map() || !g.enter_level_anywhere(level) {
        println!("could not enter level {level}");
        return;
    }
    let mut back: Vec<Vec<(u16, u8)>> = vec![state(&mut g)];
    let (mut tiles, mut mode) = (mario_tiles(&mut g), g.read(GAME_MODE));
    println!("start tiles {tiles:02x?}");
    for f in 0..frames {
        let b = if jump > 0 && f % jump < 14 { Buttons::RIGHT | Buttons::A } else { Buttons::RIGHT };
        g.run_frame_with(b);
        g.clear_audio();
        let now = state(&mut g);
        let (t, m) = (mario_tiles(&mut g), g.read(GAME_MODE));
        if t.len() != tiles.len() || m != mode || (t.first() == Some(&0x4E) && tiles.first() != Some(&0x4E)) {
            let old = &back[0];
            let ch: Vec<String> = old.iter().zip(&now).filter(|(a, b)| a.1 != b.1 && !(0xA100..0xA1A0).contains(&a.0)).map(|(a, b)| format!("{:04x}:{:02x}>{:02x}", a.0, a.1, b.1)).collect();
            println!("frame {f}: mode {m:02x} x {:04x} tiles {t:02x?}\n   {}", word(&mut g, PLAYER_X), ch.join(" "));
            tiles = t;
            mode = m;
        }
        back.push(now);
        if back.len() > 4 {
            back.remove(0);
        }
        if m != 0x04 && m != 0x08 {
            break;
        }
    }
}

/// In level `level`, hold Right (jumping every `jump` frames) and print the active object
/// slots ($AD00, $20 bytes each) that are in use at frames `at`.
pub fn slots_probe(rom: &Path, level: u8, jump: u32, at: &[u16]) {
    let Some(mut g) = probed(rom) else { return };
    if !g.boot_to_map() || !g.enter_level_anywhere(level) {
        println!("could not enter level {level}");
        return;
    }
    let last = at.iter().copied().max().unwrap_or(0) as u32;
    for f in 0..=last {
        if at.contains(&(f as u16)) {
            println!("frame {f}: mario x {:04x} y {:04x} a216 {:02x}", word(&mut g, PLAYER_X), word(&mut g, PLAYER_Y), g.read(0xA216));
            for s in 0..8u16 {
                let b: Vec<u8> = (0..0x20).map(|k| g.read(0xAD00 + s * 0x20 + k)).collect();
                if b.iter().any(|&v| v != 0) {
                    println!("  slot {s}: {}", b.iter().map(|v| format!("{v:02x}")).collect::<Vec<_>>().join(" "));
                }
            }
        }
        let b = if jump > 0 && f % jump < 14 { Buttons::RIGHT | Buttons::A } else { Buttons::RIGHT };
        g.run_frame_with(b);
        g.clear_audio();
    }
}

/// In each level, hold Right and jump every `jump` frames; print the first frame an active
/// object slot holds an item (property byte +$12 with bit 7 set) and the whole slot.
pub fn item_probe(rom: &Path, jump: u32, frames: u32) {
    let Some(mut base) = probed(rom) else { return };
    if !base.boot_to_map() {
        return;
    }
    for level in 0..32u8 {
        let mut g = base.clone();
        if !g.enter_level_anywhere(level) {
            continue;
        }
        let mut seen = Vec::new();
        for f in 0..frames {
            let b = if jump > 0 && f % jump < 14 { Buttons::RIGHT | Buttons::A } else { Buttons::RIGHT };
            g.run_frame_with(b);
            g.clear_audio();
            if g.read(GAME_MODE) != 0x04 {
                break;
            }
            for s in 0..8u16 {
                let p = g.read(0xAD00 + s * 0x20 + 0x12);
                if p & 0x80 != 0 && p != 0x80 && !seen.contains(&(p, g.read(0xAD00 + s * 0x20 + 5))) {
                    seen.push((p, g.read(0xAD00 + s * 0x20 + 5)));
                    let b: Vec<u8> = (0..0x20).map(|k| g.read(0xAD00 + s * 0x20 + k)).collect();
                    println!("level {level} frame {f} slot {s}: {}", b.iter().map(|v| format!("{v:02x}")).collect::<Vec<_>>().join(" "));
                }
            }
        }
    }
    let _ = &mut base;
}

/// In level `level`, fill a free active object slot with an item of each type in `types`
/// (copied from one that came out of a block) two blocks right of Mario, walk into it and
/// print what Mario got: power-up state, star, lives, coins and what the slot became.
pub fn summon_probe(rom: &Path, level: u8, types: &[u16]) {
    let Some(mut base) = probed(rom) else { return };
    if !base.boot_to_map() || !base.enter_level_anywhere(level) {
        println!("could not enter level {level}");
        return;
    }
    for _ in 0..30 {
        base.run_frame_with(Buttons::NONE);
    }
    const TEMPLATE: [u8; 0x20] = [0x01, 0x48, 0x10, 0x01, 0x97, 0x11, 0xab, 0x08, 0x02, 0x35, 0x46, 0x19, 0x80, 0, 0, 0x01, 0, 0, 0x81, 0xf3, 0xfd, 0xfa, 0x06, 0, 0x01, 0, 0, 0, 0, 0, 0, 0];
    for &t in types {
        let mut g = base.clone();
        let Some(s) = (0..8u16).find(|&s| g.read(0xAD00 + s * 0x20) == 0) else { return };
        let (x, y) = (word(&mut g, PLAYER_X) + 0x20, word(&mut g, PLAYER_Y));
        let mut b = TEMPLATE;
        b[0] = (x >> 8) as u8;
        b[1] = x as u8;
        b[3] = (y >> 8) as u8;
        b[4] = y as u8;
        b[5] = (t >> 8) as u8;
        b[0x12] = t as u8;
        for (k, v) in b.into_iter().enumerate() {
            g.write(0xAD00 + s * 0x20 + k as u16, v);
        }
        let objs = |g: &mut Pg| (0xAB00..0xAB60u16).map(|a| g.read(a)).collect::<Vec<u8>>();
        let ob = objs(&mut g);
        let before = (g.read(0xA216), g.read(0xA271), g.read(0xA22C), g.read(0xA262));
        let mut got = Vec::new();
        for f in 0..400u32 {
            let away = std::env::var("AWAY").is_ok();
            g.run_frame_with(if f > 20 { if away { Buttons::LEFT } else { Buttons::RIGHT } } else { Buttons::NONE });
            g.clear_audio();
            let k = g.read(0xA26C);
            if k != 0 {
                got.push(k);
            }
            if f == 2 || f == 20 {
                let sl: Vec<u8> = (0..0x14).map(|k| g.read(0xAD00 + s * 0x20 + k)).collect();
                println!("  type {t:02x} frame {f}: {}", sl.iter().map(|v| format!("{v:02x}")).collect::<Vec<_>>().join(" "));
            }
        }
        let oa = objs(&mut g);
        let d: Vec<String> = ob.iter().zip(&oa).enumerate().filter(|(_, (x, y))| x != y).map(|(i, (x, y))| format!("{:04x}:{x:02x}>{y:02x}", 0xAB00 + i)).collect();
        println!("  object records changed: {}", d.join(" "));
        let after = (g.read(0xA216), g.read(0xA271), g.read(0xA22C), g.read(0xA262));
        println!("type {t:02x}: state/star/lives/coins {before:02x?} > {after:02x?}, touched {got:02x?}, mode {:02x}", g.read(GAME_MODE));
    }
}

/// In level `level`, put a mature item (type $11, property `prop`) with sprite byte +$0B =
/// each of `looks` two blocks right of Mario, wait 20 frames and save the screen as
/// `<dir>/look_<b>.png` (scaled 3x, side by side in one picture).
pub fn look_probe(rom: &Path, level: u8, prop: u8, looks: &[u16], dir: &Path) {
    let Some(mut base) = probed(rom) else { return };
    if !base.boot_to_map() || !base.enter_level_anywhere(level) {
        return;
    }
    for _ in 0..30 {
        base.run_frame_with(Buttons::NONE);
    }
    let mut row: Vec<Vec<u8>> = Vec::new();
    for &l in looks {
        let mut g = base.clone();
        let s = (0..8u16).find(|&s| g.read(0xAD00 + s * 0x20) == 0).unwrap_or(7);
        let (x, y) = (word(&mut g, PLAYER_X) + 0x20, word(&mut g, PLAYER_Y));
        let mut b = [0x01u8, 0x48, 0x10, 0x01, 0x97, 0x11, 0xab, 0x08, 0x02, 0x35, 0x46, 0x19, 0x80, 0, 0, 0x01, 0, 0, 0x81, 0xf3, 0xfd, 0xfa, 0x06, 0, 0x01, 0, 0, 0, 0, 0, 0, 0];
        b[0] = (x >> 8) as u8;
        b[1] = x as u8;
        b[3] = (y >> 8) as u8;
        b[4] = y as u8;
        b[0x0b] = l as u8;
        b[0x12] = prop;
        for (k, v) in b.into_iter().enumerate() {
            g.write(0xAD00 + s * 0x20 + k as u16, v);
        }
        for _ in 0..20 {
            g.run_frame_with(Buttons::NONE);
            g.clear_audio();
        }
        let base_oam: Vec<[u8; 4]> = (0..40u16).map(|i| [0, 1, 2, 3].map(|k| base.read(0xFE00 + i * 4 + k))).collect();
        let new: Vec<String> = (0..40u16).map(|i| [0, 1, 2, 3].map(|k| g.read(0xFE00 + i * 4 + k))).filter(|e| e[0] != 0 && !base_oam.iter().any(|b| b[2] == e[2] && b[3] == e[3])).map(|e| format!("y{:02x} x{:02x} t{:02x} f{:02x}", e[0], e[1], e[2], e[3])).collect();
        println!("look {l:02x}: {}", new.join(" | "));
        row.push(g.screen().to_vec());
    }
    let (w, h) = (gb_core::ppu::W, gb_core::ppu::H);
    let mut all = vec![0u8; w * h * row.len()];
    for (i, sc) in row.iter().enumerate() {
        for y in 0..h {
            all[(y * row.len() + i) * w..(y * row.len() + i) * w + w].copy_from_slice(&sc[y * w..y * w + w]);
        }
    }
    let mut rgba = vec![0u8; all.len() * 4];
    gb_core::palette::fb_to_rgba(&all, &gb_core::palette::PALETTES[0], &mut rgba);
    let p = dir.join("looks.png");
    if let Err(e) = host::png_out::write_rgba(&p, w * row.len(), h, &rgba) {
        println!("{e}");
    }
    println!("wrote {}", p.display());
}

/// In level `level`, standing still: the state bytes whose value at frame 600 is below
/// their value at frame 60 (the timer counts down).
pub fn timer_probe(rom: &Path, level: u8) {
    let Some(mut g) = probed(rom) else { return };
    if !g.boot_to_map() || !g.enter_level_anywhere(level) {
        return;
    }
    let mut at = Vec::new();
    for f in 0..=600u32 {
        if f == 60 || f == 600 {
            at.push(state(&mut g));
        }
        g.run_frame_with(Buttons::NONE);
        g.clear_audio();
    }
    let down: Vec<String> = at[0].iter().zip(&at[1]).filter(|(a, b)| b.1 < a.1 && !(0xA100..0xA1A0).contains(&a.0)).map(|(a, b)| format!("{:04x}:{:02x}>{:02x}", a.0, a.1, b.1)).collect();
    println!("{}", down.join(" "));
}

/// In level `level`, after 60 frames write `pokes` ("a24f=0c,ff9b=18"), then for 3000 frames
/// print every change of the game mode with the level number ($A269), pressing A now and
/// then on screens that wait for it.
pub fn finish_probe(rom: &Path, level: u8, pokes: &str) {
    let Some(mut g) = probed(rom) else { return };
    if !g.boot_to_map() || !g.enter_level_anywhere(level) {
        return;
    }
    for _ in 0..60 {
        g.run_frame_with(Buttons::NONE);
    }
    crate::console::apply_pokes(&mut g, pokes);
    let mut mode = 0xFF;
    let mut shots: Vec<Vec<u8>> = Vec::new();
    for f in 0..3000u32 {
        if f >= 200 && f <= 1100 && f % 100 == 0 {
            shots.push(g.screen().to_vec());
        }
        g.run_frame_with(if f % 120 < 3 && f > 400 { Buttons::A } else { Buttons::NONE });
        g.clear_audio();
        let m = g.read(GAME_MODE);
        if m != mode {
            println!("frame {f}: mode {m:02x} level {:02x} a2a0 {:02x} a24f {:02x}", g.read(LEVEL_NUMBER), g.read(0xA2A0), g.read(0xA24F));
            mode = m;
        }
    }
    let (w, h) = (gb_core::ppu::W, gb_core::ppu::H);
    let mut all = vec![0u8; w * h * shots.len()];
    for (i, sc) in shots.iter().enumerate() {
        for y in 0..h {
            all[(y * shots.len() + i) * w..(y * shots.len() + i) * w + w].copy_from_slice(&sc[y * w..y * w + w]);
        }
    }
    let mut rgba = vec![0u8; all.len() * 4];
    gb_core::palette::fb_to_rgba(&all, &gb_core::palette::PALETTES[0], &mut rgba);
    let _ = host::png_out::write_rgba(Path::new("finish.png"), w * shots.len(), h, &rgba);
}

/// Level `level` started on its bell (Mario put where the bell object is, so he takes it),
/// then Right held into the goal: every change of the game mode with the level number,
/// $A2A0 and $A24F, and the active slot that holds the bell when Mario starts.
pub fn bell_probe(rom_path: &Path, level: u8) {
    use cart_model::level_start::Start;
    use cart_model::{level_goals, levels, objects};
    let Ok(original) = std::fs::read(rom_path) else { return };
    let mut rom = original.clone();
    let objs = objects::read(&rom, level as usize).0;
    let Some(bell) = objs.iter().find(|o| o.kind == level_goals::BELL) else {
        println!("level {level} has no bell");
        return;
    };
    let h = levels::header(&rom, level as usize);
    let s = Start::standing(bell.x, bell.y + 8);
    s.write(&mut rom[h.at..h.at + 8]);
    println!("bell at ({}, {}), Mario {:?}", bell.x, bell.y, s);
    let Ok(mut g) = GameBoy::with_observer(rom, 44100, Probe::default()) else { return };
    if !g.boot_to_map() || !g.enter_level_anywhere(level) {
        return;
    }
    let mut mode = 0xFF;
    for f in 0..4000u32 {
        let m = g.read(GAME_MODE);
        g.run_frame_with(if m == 0x04 { Buttons::RIGHT } else if f % 120 < 3 { Buttons::A } else { Buttons::NONE });
        g.clear_audio();
        if f < 3 {
            for sl in 0..8u16 {
                if g.read(0xAD00 + sl * 0x20) != 0 {
                    let b: Vec<u8> = (0..0x14).map(|k| g.read(0xAD00 + sl * 0x20 + k)).collect();
                    println!("  frame {f} slot {sl}: {}", b.iter().map(|v| format!("{v:02x}")).collect::<Vec<_>>().join(" "));
                }
            }
        }
        let m = g.read(GAME_MODE);
        if m != mode {
            println!("frame {f}: mode {m:02x} level {:02x} a2a0 {:02x} a24f {:02x} x {:04x}", g.read(LEVEL_NUMBER), g.read(0xA2A0), g.read(0xA24F), word(&mut g, PLAYER_X));
            mode = m;
        }
    }
}

/// Hash of the item sprite tiles ($90-$A4 at $8000) in each level after 30 frames.
pub fn tiles_probe(rom: &Path) {
    let Some(mut base) = probed(rom) else { return };
    if !base.boot_to_map() {
        return;
    }
    for level in 0..32u8 {
        let mut g = base.clone();
        if !g.enter_level_anywhere(level) {
            continue;
        }
        for _ in 0..30 {
            g.run_frame_with(Buttons::NONE);
        }
        let tiles: Vec<String> = [0x90u16, 0x91, 0x92, 0x93, 0x94, 0x97, 0x98, 0xa0, 0xa1, 0xa2, 0xa3, 0xa4].iter().map(|&t| {
            let mut h: u16 = 0;
            for a in 0x8000 + t * 16..0x8000 + t * 16 + 16 {
                h = h.wrapping_mul(31).wrapping_add(g.read(a) as u16);
            }
            format!("{t:02x}:{h:04x}")
        }).collect();
        println!("level {level}: {}", tiles.join(" "));
    }
}

/// In level `level`, with the program bytes `patch` ("2e95=c9,1c58=18") swapped in bank 0,
/// hold Right (jumping every `jump` frames) for `frames`: print each change of the power-up
/// state, $A224, lives and the game mode.
pub fn patch_probe(rom: &Path, level: u8, jump: u32, frames: u32, patch: &str) {
    let Some(mut g) = probed(rom) else { return };
    for p in patch.split(',').filter(|p| !p.is_empty()) {
        if let Some((a, v)) = p.split_once('=') {
            let a = usize::from_str_radix(a, 16).unwrap_or(0);
            g.cart_mut().rom_mut()[a] = u8::from_str_radix(v, 16).unwrap_or(0);
        }
    }
    if !g.boot_to_map() || !g.enter_level_anywhere(level) {
        return;
    }
    g.observer_mut().watch = vec![0xA224];
    let mut last = [0u8; 4];
    for f in 0..frames {
        let b = if jump > 0 && f % jump < 14 { Buttons::RIGHT | Buttons::A } else { Buttons::RIGHT };
        g.run_frame_with(b);
        g.clear_audio();
        let now = [g.read(0xA216), g.read(0xA224), g.read(0xA22C), g.read(GAME_MODE)];
        if now != last {
            println!("frame {f}: state {:02x} a224 {:02x} lives {:02x} mode {:02x} x {:04x}", now[0], now[1], now[2], now[3], word(&mut g, PLAYER_X));
            for (pc, bank, _, v) in g.observer_mut().watch_log.drain(..) {
                println!("  a224 = {v:02x} by {bank:02x}:{pc:04x}");
            }
            last = now;
        }
    }
}

/// The object records ($AB00-$ACFF) of level `level` 30 frames after it starts.
pub fn records_probe(rom: &Path, level: u8) {
    let Some(mut g) = probed(rom) else { return };
    if !g.boot_to_map() || !g.enter_level_anywhere(level) {
        return;
    }
    for _ in 0..30 {
        g.run_frame_with(Buttons::NONE);
    }
    println!("objects in the level: {}", cart_model::objects::read(g.cart().rom(), level as usize).0.len());
    for r in 0..0x200u16 / 16 {
        let b: Vec<String> = (0..16).map(|k| format!("{:02x}", g.read(0xAB00 + r * 16 + k))).collect();
        println!("{:04x}: {}", 0xAB00 + r * 16, b.join(" "));
    }
}

/// In level `level`: state bytes that differ between Mario standing after walking right and
/// standing after walking left (his facing).
pub fn facing_probe(rom: &Path, level: u8) {
    let Some(mut base) = probed(rom) else { return };
    if !base.boot_to_map() || !base.enter_level_anywhere(level) {
        return;
    }
    let look = |dir: Buttons, base: &Pg| {
        let mut g = base.clone();
        for f in 0..60 {
            g.run_frame_with(if f < 20 { dir } else { Buttons::NONE });
        }
        state(&mut g)
    };
    let (r, l, r2) = (look(Buttons::RIGHT, &base), look(Buttons::LEFT, &base), look(Buttons::RIGHT | Buttons::B, &base));
    let d: Vec<String> = r.iter().zip(&l).zip(&r2).filter(|((a, b), c)| a.1 != b.1 && a.1 == c.1 && !(0xA100..0xA1A0).contains(&a.0)).map(|((a, b), _)| format!("{:04x}:{:02x}/{:02x}", a.0, a.1, b.1)).collect();
    println!("right/left: {}", d.join(" "));
}

/// Wario's castle, the level behind the door at point $5C.
const WARIO_CASTLE: u8 = 0x18;

/// Every cheat of `cart_model::cheats` played in the emulator, each reported ok or FAIL.
pub fn cheat_check(rom_path: &Path) {
    use cart_model::cheats::{self, Item};
    let Some(mut base) = crate::console::load(rom_path) else { return };
    if !base.boot_to_map() {
        println!("cheatcheck FAILED: no map");
        return;
    }
    let mut ok = true;
    let mut report = |name: &str, fine: bool, how: String| {
        ok &= fine;
        println!("{name}: {} {how}", if fine { "ok" } else { "FAIL" });
    };
    let play = |g: &mut gb_core::Gb, frames: u32, b: Buttons| {
        for _ in 0..frames {
            g.run_frame_with(b);
            g.clear_audio();
        }
    };
    let mut lvl = base.clone();
    lvl.enter_level_anywhere(0);
    play(&mut lvl, 30, Buttons::NONE);
    for on in [true, false] {
        let mut g = lvl.clone();
        let patched = cheats::set_invincible(g.cart_mut().rom_mut(), on);
        let lives = g.read(ram_map::LIVES);
        let mut hurt = false;
        for _ in 0..900 {
            g.run_frame_with(Buttons::RIGHT);
            g.clear_audio();
            hurt |= g.read(0xA224) != 0;
        }
        let lost = g.read(ram_map::LIVES) != lives || g.read(GAME_MODE) != 0x04;
        report(&format!("invincible {}", if on { "on" } else { "off" }), patched && hurt != on && lost != on, format!("(hurt {hurt}, life lost {lost})"));
    }
    {
        let mut g = base.clone();
        let flags = |t: &mut gb_core::Gb| (0xA848..0xA880u16).map(|a| t.read(a)).collect::<Vec<u8>>();
        let point = cart_model::world::point_levels(g.cart().rom()).iter().position(|&l| l == 1).unwrap_or(0) as u8;
        g.show_map_at(point);
        let before = flags(&mut g);
        g.enter_level(1);
        play(&mut g, 30, Buttons::NONE);
        let finished = cheats::finish_level(&mut g);
        let (mut bell, mut bonus, mut map) = (false, false, 0);
        for f in 0..3000u32 {
            let m = g.read(GAME_MODE);
            bell |= m == 0x14 && g.read(0xA24F) == 0x0C;
            g.run_frame_with(if bonus && map < 400 && f % 60 < 4 { Buttons::A } else { Buttons::NONE });
            g.clear_audio();
            bonus |= bell && g.read(GAME_MODE) == 0x0C && g.read(0xA24F) == 0;
            map = if g.read(GAME_MODE) == 0x0C { map + 1 } else { 0 };
        }
        let cleared = flags(&mut g) != before;
        report("finish level 1", finished && bell && bonus && map > 600 && cleared, format!("(bonus game {bonus}, on the map {}, cleared {cleared})", map > 600));
        let mut m = base.clone();
        report("finish refused on the map", !cheats::finish_level(&mut m) && m.read(GAME_MODE) == 0x0C, String::new());
    }
    {
        let table = cart_model::world::point_levels(base.cart().rom());
        let mut found = 0;
        for level in 0..32u8 {
            let Some(point) = table.iter().position(|&l| l == level).map(|p| p as u8) else { continue };
            let mut g = base.clone();
            if !g.show_map_at(point) || !g.enter_level(level) {
                continue;
            }
            play(&mut g, 30, Buttons::NONE);
            if !cheats::has_secret_exit(&mut g) {
                let refused = !cheats::take_secret_exit(&mut g) && g.read(GAME_MODE) == 0x04;
                if !refused {
                    report(&format!("secret exit refused in level {level}"), false, String::new());
                }
                continue;
            }
            found += 1;
            let ends = |mut t: gb_core::Gb, secret: bool| {
                if secret {
                    cheats::take_secret_exit(&mut t);
                } else {
                    cheats::finish_level(&mut t);
                }
                let mut map = 0;
                for f in 0..3000u32 {
                    t.run_frame_with(if !secret && map > 0 && map < 400 && f % 60 < 4 { Buttons::A } else { Buttons::NONE });
                    t.clear_audio();
                    map = if t.read(GAME_MODE) == 0x0C { map + 1 } else { 0 };
                    if map > 600 {
                        break;
                    }
                }
                (map > 600, t.read(0xA840))
            };
            let (secret_map, secret_point) = ends(g.clone(), true);
            let (_, finish_point) = ends(g, false);
            report(&format!("secret exit of level {level}"), secret_map && secret_point != finish_point, format!("(on the map {secret_map}, point {secret_point:02x}, the finish leads to {finish_point:02x})"));
        }
        report("levels with a secret exit", found > 0, format!("({found})"));
    }
    for item in Item::ALL {
        for left in [false, true] {
            let mut g = lvl.clone();
            if left {
                play(&mut g, 40, Buttons::RIGHT);
                play(&mut g, 8, Buttons::LEFT);
                play(&mut g, 20, Buttons::NONE);
            }
            let records = |t: &mut gb_core::Gb| (0..64u16).map(|n| t.read(0xAB08 + n * 6)).collect::<Vec<u8>>();
            let rec = records(&mut g);
            let before = (g.read(0xA216), g.read(0xA271), cheats::lives(&g));
            let mx = g.read(ram_map::PLAYER_X) as u16 | (g.read(ram_map::PLAYER_X + 1) as u16) << 8;
            let summoned = cheats::summon(&mut g, item);
            play(&mut g, 2, Buttons::NONE);
            let ix = (0..16u16).map(|s| 0xAD00 + s * 0x20).find(|&s| g.read(s + 0x12) == 0x80 | item.kind()).map(|s| (g.read(s) as u16) << 8 | g.read(s + 1) as u16);
            let ahead = ix.is_some_and(|x| if left { x < mx } else { x > mx });
            play(&mut g, 60, if left { Buttons::LEFT } else { Buttons::RIGHT });
            let after = (g.read(0xA216), g.read(0xA271), cheats::lives(&g));
            let took = match item {
                Item::Mushroom => after.0 == 1,
                Item::Flower => after.0 == 3,
                Item::Carrot => after.0 == 2,
                Item::Star => after.1 == 4,
                Item::Heart => after.2 == before.2 + 1,
            };
            let untouched = records(&mut g).iter().zip(&rec).all(|(a, b)| a == b || *a != 0x02);
            report(&format!("summon {item:?}{}", if left { " facing left" } else { "" }), summoned && ahead && took && untouched, format!("(ahead {ahead}, taken {took}, records untouched {untouched})"));
        }
    }
    {
        let mut g = lvl.clone();
        let t = cheats::timer(&g);
        for _ in 0..600 {
            g.run_frame_with(Buttons::RIGHT);
            g.clear_audio();
            cheats::set_timer(&mut g, t);
        }
        let mut free = lvl.clone();
        play(&mut free, 600, Buttons::RIGHT);
        report("timer frozen", cheats::timer(&g) == t && cheats::timer(&free) != t, format!("({:02x?}, {:02x?} without the cheat)", cheats::timer(&g), cheats::timer(&free)));
    }
    {
        let mut g = lvl.clone();
        cheats::max_lives(&mut g);
        cheats::max_coins(&mut g);
        play(&mut g, 60, Buttons::NONE);
        report("99 lives, 999 coins", cheats::lives(&g) == 99 && cheats::coins(&g) == 999, format!("({} lives, {} coins)", cheats::lives(&g), cheats::coins(&g)));
    }
    {
        let mut g = base.clone();
        cheats::golden_coins(&mut g);
        let coins = cheats::has_golden_coins(&g);
        g.show_map_at(0x5C);
        let mut door = false;
        for f in 0..900u32 {
            g.run_frame_with(if f % 60 < 4 { Buttons::A } else { Buttons::NONE });
            g.clear_audio();
            door |= g.read(GAME_MODE) == 0x04 && g.read(LEVEL_NUMBER) == WARIO_CASTLE;
        }
        let mut plain = base.clone();
        plain.show_map_at(0x5C);
        let mut shut = true;
        for f in 0..900u32 {
            plain.run_frame_with(if f % 60 < 4 { Buttons::A } else { Buttons::NONE });
            plain.clear_audio();
            shut &= !(plain.read(GAME_MODE) == 0x04 && plain.read(LEVEL_NUMBER) == WARIO_CASTLE);
        }
        report("six golden coins", coins && door && shut, format!("(Wario's castle entered with the coins {door}, door shut without them {shut})"));
    }
    {
        let mut g = lvl.clone();
        let a = cheats::info(&mut g);
        play(&mut g, 60, Buttons::RIGHT);
        let b = cheats::info(&mut g);
        report("info", a.level == 0 && a.mode == 0x04 && b.x > a.x, format!("({a:?} > x {:04x})", b.x));
    }
    {
        let mut g = lvl.clone();
        play(&mut g, 300, Buttons::RIGHT);
        let seen = cheats::active_objects(&mut g);
        let on_screen = seen.iter().filter(|a| (0..160).contains(&a.x) && (0..144).contains(&a.y)).count();
        report("active objects", !seen.is_empty() && on_screen > 0 && cheats::info(&mut g).objects as usize == seen.len(), format!("({seen:?})"));
    }
    {
        let mut g = base.clone();
        g.enter_level_anywhere(1);
        play(&mut g, 30, Buttons::NONE);
        let from_level = cheats::go_to_level(&mut g, 12) && g.read(LEVEL_NUMBER) == 12;
        let lives = cheats::lives(&g);
        let mut m = base.clone();
        let from_map = cheats::go_to_level(&mut m, 5) && m.read(LEVEL_NUMBER) == 5;
        report("go to level", from_level && from_map, format!("(from level 1 to 12 {from_level}, from the map to 5 {from_map}, lives {lives})"));
    }
    {
        let reached = |open: bool| {
            let mut g = base.clone();
            if open {
                cheats::open_map(&mut g);
            }
            g.show_map_at(0x07);
            let mut seen: Vec<u8> = Vec::new();
            let dirs = [Buttons::RIGHT, Buttons::UP, Buttons::LEFT, Buttons::DOWN];
            let mut seed: u32 = 12345;
            for f in 0..20000u32 {
                if f % 20 == 0 {
                    seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
                }
                g.run_frame_with(if f % 20 < 4 { dirs[(seed >> 16) as usize % 4] } else { Buttons::NONE });
                g.clear_audio();
                let p = g.read(0xA840);
                if g.read(GAME_MODE) == 0x0C && !seen.contains(&p) {
                    seen.push(p);
                }
            }
            seen
        };
        let (shut, open) = (reached(false), reached(true));
        let more: Vec<u8> = open.iter().copied().filter(|p| !shut.contains(p)).collect();
        report("open the whole map", !more.is_empty(), format!("(points only reached with it open: {more:02x?})"));
    }
    println!("{}", if ok { "cheatcheck ok" } else { "cheatcheck FAILED" });
}

/// From map point `point`, with the golden coins given or not, press A now and then: every
/// change of the game mode, the level number and the point.
pub fn door_probe(rom: &Path, point: u8, coins: bool) {
    let Some(mut g) = probed(rom) else { return };
    if !g.boot_to_map() {
        return;
    }
    if coins {
        cart_model::cheats::golden_coins(&mut g);
    }
    println!("shown {}", g.show_map_at(point));
    let mut last = (0xFF, 0xFF, 0xFF);
    for f in 0..1500u32 {
        g.run_frame_with(if f % 60 < 4 { Buttons::A } else { Buttons::NONE });
        g.clear_audio();
        let now = (g.read(GAME_MODE), g.read(LEVEL_NUMBER), g.read(0xA840));
        if now != last {
            println!("frame {f}: mode {:02x} level {:02x} point {:02x} 99c8 {:02x}", now.0, now.1, now.2, g.read(0x99C8));
            last = now;
        }
    }
}
