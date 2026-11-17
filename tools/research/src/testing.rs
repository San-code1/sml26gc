//! Research for the testing tools: the midway bell (checkpoint), the level header's
//! settings and the object limits.
use crate::probe::probed;
use cart_model::ram_map::{GAME_MODE, LEVEL_NUMBER, PLAYER_X, PLAYER_Y};
use cart_model::{objects, GameDriver};
use gb_core::Buttons;
use std::path::Path;

/// The restart table: 8 bytes per level (Mario's y, x, the camera's y, x).
const RESTARTS: usize = 0x377B;

fn word(b: &[u8], at: usize) -> u16 {
    b[at] as u16 | (b[at + 1] as u16) << 8
}

/// Per level: the objects of type $15 and the restart table entry.
pub fn checkpoint_list(rom_path: &Path) {
    let Ok(rom) = std::fs::read(rom_path) else { return };
    for l in 0..32usize {
        let bells: Vec<String> = objects::read(&rom, l).0.iter().filter(|o| o.kind == 0x15).map(|o| format!("({}, {})", o.x, o.y)).collect();
        let e = RESTARTS + 8 * l;
        println!("level {l:2}: restart x {:5} y {:4} cam x {:5} y {:4}  bells $15 {}", word(&rom, e + 2), word(&rom, e), word(&rom, e + 6), word(&rom, e + 4), bells.join(" "));
    }
    println!("after the table: {:02x?}", &rom[RESTARTS + 8 * 31..RESTARTS + 8 * 31 + 16]);
}

/// In each level, for each object type its list uses: a slot filled as the game's spawner
/// fills one (02:4C8D), 4 blocks right of Mario, then its class byte (+$12) after 8 frames.
/// The midway bell is the one that becomes $87 (touching it gives kind 7).
pub fn classes_probe(rom_path: &Path, only: Option<u8>) {
    let Ok(rom) = std::fs::read(rom_path) else { return };
    let Some(mut base) = probed(rom_path) else { return };
    if !base.boot_to_map() {
        return;
    }
    for level in 0..32u8 {
        if only.is_some_and(|o| o != level) {
            continue;
        }
        let mut lvl = base.clone();
        if !lvl.enter_level_anywhere(level) {
            continue;
        }
        for _ in 0..30 {
            lvl.run_frame_with(Buttons::NONE);
        }
        let mut types: Vec<u8> = objects::read(&rom, level as usize).0.iter().map(|o| o.kind).collect();
        types.push(0x17);
        types.sort();
        types.dedup();
        let mut out = Vec::new();
        for t in types {
            let mut g = lvl.clone();
            let Some(slot) = (0..16u16).map(|s| 0xAD00 + s * 0x20).find(|&s| g.read(s + 8) == 0) else { continue };
            let end = (0..).map(|n| 0xAB06 + n * 6).find(|&r: &u16| g.read(r) == 0xFF || r >= 0xACFA).unwrap_or(0xACFA);
            let x = (g.read(PLAYER_X) as u16 | (g.read(PLAYER_X + 1) as u16) << 8) + 0x40;
            let y = g.read(PLAYER_Y) as u16 | (g.read(PLAYER_Y + 1) as u16) << 8;
            let mut b = [0u8; 0x20];
            [b[0], b[1]] = x.to_be_bytes();
            [b[3], b[4]] = y.to_be_bytes();
            b[5] = t;
            [b[6], b[7]] = (end + 2).to_be_bytes();
            b[8] = 1;
            for (k, v) in (0..).zip(b) {
                g.write(slot + k, v);
            }
            for _ in 0..8 {
                g.run_frame_with(Buttons::NONE);
                g.clear_audio();
            }
            out.push(format!("{t:02x}:{:02x}", g.read(slot + 0x12)));
        }
        println!("level {level:2} (mode {:02x}): {}", lvl.read(GAME_MODE), out.join(" "));
    }
    let _ = LEVEL_NUMBER;
}

/// Level `level` with its midway bell ($17) moved to (`x`, `feet` - 64) and its restart entry
/// at 01:377B set to Mario standing at `x` - 8 on `feet`; Mario starts 3 blocks left of the
/// bell, walks and jumps into it, loses a life, enters the level again: where he starts.
pub fn checkpoint_probe(rom_path: &Path, level: u8, x: u16, feet: u16) {
    use cart_model::level_start::Start;
    let Ok(original) = std::fs::read(rom_path) else { return };
    let Ok(mut rom) = cart_model::patch::expand(&original) else { return };
    let mut lists: Vec<Vec<objects::Obj>> = (0..32).map(|l| objects::read(&rom, l).0).collect();
    let h0 = cart_model::levels::header(&rom, level as usize);
    let own = Start::read(&rom[h0.at..h0.at + 8]);
    let (x, feet) = if feet == 0 { (own.mario_x + 240, own.mario_y + cart_model::level_start::FEET) } else { (x, feet) };
    println!("own bells: {:?}", lists[level as usize].iter().filter(|o| o.kind == 0x17).collect::<Vec<_>>());
    if std::env::var("SAME").is_err() {
        lists[level as usize].retain(|o| o.kind != 0x17);
        lists[level as usize].push(objects::Obj { x, y: feet - 32, kind: 0x17, flag: false });
        lists[level as usize].sort_by_key(|o| o.x);
    }
    if std::env::var("KEEP").is_err() { if let Err(e) = objects::store(&mut rom, &original, &lists) {
        println!("not stored: {e:?}");
        return;
    } }
    let restart = Start::standing(x - 8, feet);
    let e = RESTARTS + 8 * level as usize;
    restart.write(&mut rom[e..e + 8]);
    let h = cart_model::levels::header(&rom, level as usize);
    Start::standing(x - 200, feet).write(&mut rom[h.at..h.at + 8]);
    let point = cart_model::world::point_levels(&rom).iter().position(|&l| l == level).unwrap_or(0) as u8;
    let Ok(mut g) = gb_core::GameBoy::with_observer(rom, 44100, crate::probe::Probe::default()) else { return };
    if !g.boot_to_map() || !g.show_map_at(point) || !g.enter_level(level) {
        println!("could not enter level {level}");
        return;
    }
    let pos = |g: &mut gb_core::GameBoy<crate::probe::Probe>| (g.read(PLAYER_X) as u16 | (g.read(PLAYER_X + 1) as u16) << 8, g.read(PLAYER_Y) as u16 | (g.read(PLAYER_Y + 1) as u16) << 8);
    println!("level {level} start {:?}, restart wanted {:?}", pos(&mut g), (restart.mario_x, restart.mario_y));
    let mut rung = false;
    for f in 0..400u32 {
        g.run_frame_with(if f % 30 < 12 { Buttons::RIGHT | Buttons::A } else { Buttons::RIGHT });
        g.clear_audio();
        rung |= g.read(0xA2A0) == 0xFF;
        if f % 40 == 0 {
            let slots: Vec<u16> = (0..16u16).map(|k| 0xAD00 + k * 0x20).filter(|&s| g.read(s + 8) != 0).collect();
            let used: Vec<String> = slots.into_iter().map(|s| format!("{:02x}/{:02x}@{:02x}{:02x},{:02x}{:02x}", g.read(s + 5), g.read(s + 0x12), g.read(s), g.read(s + 1), g.read(s + 3), g.read(s + 4))).collect();
            println!("  frame {f} mario {:?} slots {}", pos(&mut g), used.join(" "));
        }
    }
    println!("bell rung {rung} at {:?}", pos(&mut g));
    g.write(0xA224, 0x80);
    for _ in 0..600 {
        g.run_frame_with(Buttons::NONE);
        g.clear_audio();
        if g.read(GAME_MODE) == 0x0C {
            break;
        }
    }
    for _ in 0..120 {
        g.run_frame_with(Buttons::NONE);
    }
    if !g.enter_level(level) {
        println!("could not enter again (mode {:02x}, level {:02x})", g.read(GAME_MODE), g.read(LEVEL_NUMBER));
        return;
    }
    println!("entered again at {:?}", pos(&mut g));
}

/// Enter level `level` watching writes to `watch` while it loads: who writes what.
pub fn load_watch(rom_path: &Path, level: u8, watch: &[u16]) {
    let Some(mut g) = probed(rom_path) else { return };
    if !g.boot_to_map() {
        return;
    }
    g.observer_mut().watch = watch.to_vec();
    if !g.enter_level_anywhere(level) {
        return;
    }
    for _ in 0..5 {
        g.run_frame_with(Buttons::NONE);
    }
    for (pc, bank, a, v) in &g.observer().watch_log {
        println!("  write {a:04x} = {v:02x} by {bank:02x}:{pc:04x}");
    }
}

/// Enter level `level` watching reads of the header copy at $A800-$A813: which code reads
/// each byte.
pub fn header_reads(rom_path: &Path, level: u8) {
    let Some(mut g) = probed(rom_path) else { return };
    if !g.boot_to_map() {
        return;
    }
    g.observer_mut().read_watch = Some((0xA800, 0xA813, 0xFF));
    if !g.enter_level_anywhere(level) {
        return;
    }
    for _ in 0..30 {
        g.run_frame_with(Buttons::NONE);
    }
    let mut seen: Vec<(u16, u16, u8)> = g.observer().read_log.clone();
    seen.sort();
    seen.dedup();
    for (pc, a, bank) in seen {
        println!("  {a:04x} (+{:02x}) read by {bank:02x}:{pc:04x}", a - 0xA800);
    }
}

/// Level 1 with `n` Goombas (type $1F) added in one screen 20 blocks right of the start, on
/// the start's ground: the most active object slots in use at once while Mario walks there
/// (invincible), and how many of the added ones came to life.
pub fn crowd_probe(rom_path: &Path, n: u16) {
    use cart_model::level_start::Start;
    let Ok(original) = std::fs::read(rom_path) else { return };
    let Ok(mut rom) = cart_model::patch::expand(&original) else { return };
    let level = 1usize;
    let h = cart_model::levels::header(&rom, level);
    let s = Start::read(&rom[h.at..h.at + 8]);
    let mut lists: Vec<Vec<objects::Obj>> = (0..32).map(|l| objects::read(&rom, l).0).collect();
    let x0 = s.mario_x + 320;
    lists[level].retain(|o| o.x < x0 - 200 || o.x > x0 + 400);
    for k in 0..n {
        let step: u16 = std::env::var("STEP").ok().and_then(|v| v.parse().ok()).unwrap_or(16);
        lists[level].push(objects::Obj { x: x0 / 16 * 16 + k * step, y: s.mario_y, kind: 0x1F, flag: false });
    }
    lists[level].sort_by_key(|o| o.x);
    if let Err(e) = objects::store(&mut rom, &original, &lists) {
        println!("not stored: {e:?}");
        return;
    }
    cart_model::cheats::set_invincible(&mut rom, true);
    let Ok(mut g) = gb_core::GameBoy::with_observer(rom, 44100, crate::probe::Probe::default()) else { return };
    if !g.boot_to_map() || !g.enter_level_anywhere(level as u8) {
        return;
    }
    let (mut most, mut goombas) = (0, 0);
    let mut ever: Vec<u16> = Vec::new();
    for _ in 0..600 {
        g.run_frame_with(Buttons::RIGHT);
        g.clear_audio();
        let used: Vec<u16> = (0..16u16).map(|k| 0xAD00 + k * 0x20).filter(|&a| g.read(a + 8) != 0).collect();
        most = most.max(used.len());
        let mut here: Vec<u16> = Vec::new();
        for &a in &used {
            if g.read(a + 5) == 0x1F {
                here.push((g.read(a + 6) as u16) << 8 | g.read(a + 7) as u16);
            }
        }
        goombas = goombas.max(here.len());
        for r in here {
            if !ever.contains(&r) {
                ever.push(r);
            }
        }
    }
    println!("{n} goombas in one screen: at most {most} slots in use, {goombas} goombas alive at once, {} came to life", ever.len());
}

/// The most objects the game's own levels put within one screen width (160 px), and the
/// longest object list.
pub fn crowd_list(rom_path: &Path) {
    let Ok(rom) = std::fs::read(rom_path) else { return };
    let mut worst = (0, 0, 0);
    let mut longest = (0, 0);
    for l in 0..32usize {
        let objs = objects::read(&rom, l).0;
        longest = longest.max((objs.len(), l));
        for o in &objs {
            let n = objs.iter().filter(|p| p.x >= o.x && p.x < o.x + 160).count();
            if n > worst.0 {
                worst = (n, l, o.x);
            }
        }
    }
    println!("most in one screen: {} (level {}, from x {}); longest list: {} objects (level {})", worst.0, worst.1, worst.2, longest.0, longest.1);
}

/// The point flags ($A848-$A87F) that change when level `level` is finished, and when it is
/// left through its secret exit.
pub fn flags_probe(rom_path: &Path, level: u8) {
    let Some(base) = crate::console::load(rom_path) else { return };
    for secret in [false, true] {
        let mut g = base.clone();
        g.boot_to_map();
        let point = cart_model::world::point_levels(g.cart().rom()).iter().position(|&l| l == level).unwrap_or(0) as u8;
        g.show_map_at(point);
        let before: Vec<u8> = (0xA848..0xA880u16).map(|a| g.read(a)).collect();
        g.enter_level(level);
        for _ in 0..30 {
            g.run_frame_with(Buttons::NONE);
        }
        let done = if secret { cart_model::cheats::take_secret_exit(&mut g) } else { cart_model::cheats::finish_level(&mut g) };
        let mut map = 0;
        for f in 0..3000u32 {
            g.run_frame_with(if !secret && map > 0 && map < 400 && f % 60 < 4 { Buttons::A } else { Buttons::NONE });
            g.clear_audio();
            map = if g.read(GAME_MODE) == 0x0C { map + 1 } else { 0 };
            if map > 1500 {
                break;
            }
        }
        let after: Vec<u8> = (0xA848..0xA880u16).map(|a| g.read(a)).collect();
        let d: Vec<String> = before.iter().zip(&after).enumerate().filter(|(_, (a, b))| a != b).map(|(i, (a, b))| format!("flag {i:02x}: {a:02x}>{b:02x}")).collect();
        println!("level {level} {} ({done}): {}", if secret { "secret exit" } else { "finish" }, d.join(", "));
    }
}

/// From the world map's first point, with every point flag given bit 7 (and bit 0 where
/// `secret`): pressing the four directions in turn for `frames`, the world points reached.
pub fn open_probe(rom_path: &Path, open: bool, frames: u32) {
    let Some(mut g) = crate::console::load(rom_path) else { return };
    g.boot_to_map();
    if open {
        for a in 0xA848..0xA878u16 {
            let v = g.read(a) | 0x81;
            g.write(a, v);
        }
    }
    g.show_map_at(0x07);
    let mut seen: Vec<u8> = Vec::new();
    let dirs = [Buttons::RIGHT, Buttons::UP, Buttons::LEFT, Buttons::DOWN];
    let mut seed: u32 = 12345;
    for f in 0..frames {
        if f % 20 == 0 {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        }
        let b = if f % 20 < 4 { dirs[(seed >> 16) as usize % 4] } else { Buttons::NONE };
        g.run_frame_with(b);
        g.clear_audio();
        let p = g.read(0xA840);
        if g.read(GAME_MODE) == 0x0C && !seen.contains(&p) {
            seen.push(p);
        }
    }
    seen.sort();
    println!("{} points reached: {:02x?}", seen.len(), seen);
}

/// Every code address that reads the point flags ($A848-$A87F) while the game boots, shows
/// the maps, plays and finishes level 2 through its secret exit and level 1, and goes back
/// to the file select.
pub fn flag_readers(rom_path: &Path) {
    let Some(mut g) = probed(rom_path) else { return };
    g.observer_mut().read_watch = Some((0xA848, 0xA87F, 0xFF));
    g.observer_mut().watch = (0xA848..0xA880).collect();
    g.boot_to_map();
    for (level, secret) in [(2u8, true), (1, false), (7, true)] {
        let point = cart_model::world::point_levels(g.cart().rom()).iter().position(|&l| l == level).unwrap_or(0) as u8;
        g.show_map_at(point);
        g.enter_level(level);
        for _ in 0..30 {
            g.run_frame_with(Buttons::NONE);
        }
        if secret {
            cart_model::cheats::take_secret_exit(&mut g);
        } else {
            cart_model::cheats::finish_level(&mut g);
        }
        for f in 0..2400u32 {
            g.run_frame_with(if !secret && f % 60 < 4 { Buttons::A } else { Buttons::NONE });
            g.clear_audio();
        }
    }
    for p in [0x28u8, 0x3C, 0x50, 0x5C] {
        g.show_map_at(p);
    }
    g.back_to_file_select();
    for _ in 0..200 {
        g.run_frame_with(Buttons::NONE);
    }
    let mut pcs: Vec<(u8, u16)> = g.observer().read_log.iter().map(|&(pc, _, bank)| (bank, pc)).collect();
    pcs.sort();
    pcs.dedup();
    for (bank, pc) in pcs {
        println!("read by {bank:02x}:{pc:04x}");
    }
    let mut ws: Vec<(usize, u16)> = g.observer().watch_log.iter().map(|&(pc, bank, _, _)| (bank, pc)).collect();
    ws.sort();
    ws.dedup();
    for (bank, pc) in ws {
        println!("written by {bank:02x}:{pc:04x}");
    }
}

/// Whether anything runs or reads bank 0 from `lo` to `hi` while the game boots, plays
/// levels 1 to 3 (walking, jumping, dying, finishing) and shows the maps.
pub fn room_probe(rom_path: &Path, lo: u16, hi: u16) {
    let Some(mut g) = probed(rom_path) else { return };
    g.observer_mut().run_watch = Some((lo, hi));
    g.observer_mut().read_watch = Some((lo, hi, 0));
    g.boot_to_map();
    for level in [1u8, 2, 3, 9, 14] {
        let point = cart_model::world::point_levels(g.cart().rom()).iter().position(|&l| l == level).unwrap_or(0) as u8;
        g.show_map_at(point);
        g.enter_level(level);
        for f in 0..2400u32 {
            g.run_frame_with(match f % 50 { 0..=20 => Buttons::RIGHT, 21..=30 => Buttons::A | Buttons::RIGHT, _ => Buttons::RIGHT | Buttons::B });
            g.clear_audio();
        }
        cart_model::cheats::finish_level(&mut g);
        for f in 0..2400u32 {
            g.run_frame_with(if f % 60 < 4 { Buttons::A } else { Buttons::NONE });
            g.clear_audio();
        }
    }
    for p in [0x28u8, 0x3C, 0x50, 0x5C, 0x07] {
        g.show_map_at(p);
    }
    let o = g.observer();
    println!("{:04x}-{:04x}: ran {} times {:04x?}, read {} times {:04x?}", lo, hi, o.run_log.len(), &o.run_log[..o.run_log.len().min(5)], o.read_log.len(), o.read_log.iter().take(5).collect::<Vec<_>>());
}

/// Level 1 in an expanded copy with the several-bells hook and two bells at walking height
/// (x 1700 and x 2600) with their restart entries: Mario starts at `from`, walks right
/// for `frames` ringing what he passes, loses a life, enters again: where he starts.
pub fn bells_probe(rom_path: &Path, from: u16, frames: u32) {
    use cart_model::checkpoint;
    use cart_model::level_start::Start;
    let Ok(original) = std::fs::read(rom_path) else { return };
    let Ok(mut rom) = cart_model::patch::expand(&original) else { return };
    let level = 1usize;
    let feet = 496;
    let mut lists: Vec<Vec<objects::Obj>> = (0..32).map(|l| objects::read(&rom, l).0).collect();
    lists[level].retain(|o| o.kind != checkpoint::BELL);
    for x in [1700u16, 2600] {
        lists[level].push(objects::Obj { x, y: feet - 32, kind: checkpoint::BELL, flag: false });
    }
    lists[level].sort_by_key(|o| o.x);
    if let Err(e) = objects::store(&mut rom, &original, &lists) {
        println!("not stored: {e:?}");
        return;
    }
    println!("installed: {}", checkpoint::install(&mut rom));
    let mut e = [0u8; 8];
    for (bell, x) in [(0usize, 1700u16), (1, 2600)] {
        Start::standing(x - 8, feet).write(&mut e);
        checkpoint::set_bell_entry(&mut rom, level, bell, &e);
    }
    let h = cart_model::levels::header(&rom, level);
    Start::standing(from, feet).write(&mut rom[h.at..h.at + 8]);
    let point = cart_model::world::point_levels(&rom).iter().position(|&l| l == level as u8).unwrap_or(0) as u8;
    let Ok(mut g) = gb_core::GameBoy::with_observer(rom, 44100, crate::probe::Probe::default()) else { return };
    if !g.boot_to_map() || !g.show_map_at(point) || !g.enter_level(level as u8) {
        println!("could not enter level {level}");
        return;
    }
    let x = |g: &mut gb_core::GameBoy<crate::probe::Probe>| g.read(PLAYER_X) as u16 | (g.read(PLAYER_X + 1) as u16) << 8;
    for f in 0..frames {
        g.run_frame_with(if f % 30 < 12 { Buttons::RIGHT | Buttons::A } else { Buttons::RIGHT });
        g.clear_audio();
    }
    println!("walked to x {}, bell rung {:02x}, bell number {}", x(&mut g), g.read(0xA2A0), g.read(0xA300));
    for round in 0..2 {
        g.write(0xA224, 0x80);
        for _ in 0..600 {
            g.run_frame_with(Buttons::NONE);
            g.clear_audio();
            if g.read(GAME_MODE) == 0x0C {
                break;
            }
        }
        for _ in 0..120 {
            g.run_frame_with(Buttons::NONE);
        }
        let flag = g.read(0xA848 + 1);
        if !g.enter_level(level as u8) {
            println!("could not enter again");
            return;
        }
        println!("death {}: point flag {flag:02x}, started again at x {}", round + 1, x(&mut g));
        if std::env::var("WALK_AGAIN").is_ok() {
            for f in 0..1600u32 {
                g.run_frame_with(if f % 30 < 12 { Buttons::RIGHT | Buttons::A } else { Buttons::RIGHT });
                g.clear_audio();
            }
            println!("  walked on to x {}, bell number {}", x(&mut g), g.read(0xA300));
        }
    }
}

/// From the map, enter `level` with Mario put at `from` (on `feet`), walk right for `frames`
/// jumping now and then, lose a life, enter again: the bell number kept and where Mario
/// starts again. `writes` are made in the console's ROM first (a new level in place).
fn restart_after(rom: &[u8], writes: &[(usize, Vec<u8>)], level: u8, from: u16, feet: u16, frames: u32) -> Option<(u8, u16)> {
    use cart_model::level_start::Start;
    let mut rom = rom.to_vec();
    cart_model::extra_levels::apply(&mut rom, writes);
    let h = cart_model::levels::header(&rom, level as usize);
    Start::standing(from, feet).write(&mut rom[h.at..h.at + 8]);
    let point = cart_model::world::point_levels(&rom).iter().position(|&l| l == level).unwrap_or(0) as u8;
    let mut g = gb_core::Gb::new(rom, 44100).ok()?;
    if !g.boot_to_map() || !g.show_map_at(point) || !g.enter_level(level) {
        return None;
    }
    for f in 0..frames {
        g.run_frame_with(if f % 30 < 12 { Buttons::RIGHT | Buttons::A } else { Buttons::RIGHT });
        g.clear_audio();
    }
    let rung = g.read(0xA300);
    if std::env::var("SHOW").is_ok() {
        println!("  reached x {}, bell rung {:02x}, mode {:02x}", g.read(PLAYER_X) as u16 | (g.read(PLAYER_X + 1) as u16) << 8, g.read(0xA2A0), g.read(GAME_MODE));
    }
    g.write(0xA224, 0x80);
    for _ in 0..600 {
        g.run_frame_with(Buttons::NONE);
        g.clear_audio();
        if g.read(GAME_MODE) == 0x0C {
            break;
        }
    }
    for _ in 0..120 {
        g.run_frame_with(Buttons::NONE);
    }
    if !g.enter_level(level) {
        return None;
    }
    Some((rung, g.read(PLAYER_X) as u16 | (g.read(PLAYER_X + 1) as u16) << 8))
}

/// Checkpoints as the editor saves them (`cart_model::checkpoint`): a moved bell in level 1
/// (no hook), two bells in level 1 (hook), two bells in a new level based on level 1 (its
/// entries put in place while it is played, level 1's own kept).
pub fn checkpoint_check(rom_path: &Path) {
    use cart_model::{checkpoint, extra_levels};
    let Ok(original) = std::fs::read(rom_path) else { return };
    let Ok(base) = cart_model::patch::expand(&original) else { return };
    let feet = 496;
    let mut ok = true;
    let mut report = |name: &str, got: Option<(u8, u16)>, want: u16| {
        let fine = got.is_some_and(|(_, x)| x == want);
        ok &= fine;
        println!("{name}: {} (got {got:?}, want x {want})", if fine { "ok" } else { "FAIL" });
    };
    let with_bells = |rom: &mut Vec<u8>, xs: &[u16]| -> Vec<objects::Obj> {
        let mut lists: Vec<Vec<objects::Obj>> = (0..32).map(|l| objects::read(rom, l).0).collect();
        lists[1].retain(|o| o.kind != checkpoint::BELL);
        for &x in xs {
            checkpoint::insert_bell(&mut lists[1], x, feet - 32);
        }
        objects::store(rom, &original, &lists).ok();
        lists[1].clone()
    };
    let map1 = |rom: &[u8]| cart_model::levels::decode(rom, cart_model::levels::map_location(rom, cart_model::levels::header(rom, 1)).1).0;
    {
        let mut rom = base.clone();
        let objs = with_bells(&mut rom, &[1700]);
        let map = map1(&rom);
        cart_model::extra_levels::apply(&mut rom, &checkpoint::writes(1, &objs, &map));
        println!("one bell: hook installed {}", checkpoint::installed(&rom));
        let want = checkpoint::restart_for(&map, &objs[checkpoint::bells(&objs)[0]]).mario_x;
        report("one moved bell", restart_after(&rom, &[], 1, 1500, feet, 400), want);
    }
    {
        let mut rom = base.clone();
        let objs = with_bells(&mut rom, &[1700, 2600]);
        let map = map1(&rom);
        checkpoint::install(&mut rom);
        cart_model::extra_levels::apply(&mut rom, &checkpoint::writes(1, &objs, &map));
        let b = checkpoint::bells(&objs);
        let first = checkpoint::restart_for(&map, &objs[b[0]]).mario_x;
        let second = checkpoint::restart_for(&map, &objs[b[1]]).mario_x;
        report("two bells, the first rung", restart_after(&rom, &[], 1, 1500, feet, 400), first);
        report("two bells, both rung", restart_after(&rom, &[], 1, 1500, feet, 1400), second);
        report("two bells, the second rung", restart_after(&rom, &[], 1, 2400, feet, 300), second);
    }
    {
        let mut rom = base.clone();
        checkpoint::install(&mut rom);
        let mut e = extra_levels::based_on(&rom, 1);
        e.objs.retain(|o| o.kind != checkpoint::BELL);
        for x in [1700, 2600] {
            checkpoint::insert_bell(&mut e.objs, x, feet - 32);
        }
        if extra_levels::store(&mut rom, std::slice::from_ref(&e)).is_err() {
            println!("checkpointcheck FAILED: the new level does not fit");
            return;
        }
        let own = checkpoint::bell_entry(&rom, 1, 0);
        let writes = extra_levels::writes_for(&rom, &e);
        let b = checkpoint::bells(&e.objs);
        let second = checkpoint::restart_for(&e.map, &e.objs[b[1]]).mario_x;
        report("new level, both bells rung", restart_after(&rom, &writes, 1, 1500, feet, 1400), second);
        let kept = checkpoint::bell_entry(&rom, 1, 0) == own;
        ok &= kept;
        println!("level 1's own entry kept in the copy: {kept}");
    }
    println!("{}", if ok { "checkpointcheck ok" } else { "checkpointcheck FAILED" });
}

/// What the map does from point `p` with every point cleared (none with CLOSED set): A
/// (or PRESS: up, down, left, right, none) pressed there (`level` None),
/// or `level` finished as its goal; prints each change of ($A840 point, $A68B map, mode).
pub fn door_probe(rom_path: &Path, p: u8, level: Option<u8>) {
    let Some(mut g) = crate::console::load(rom_path) else { return };
    g.boot_to_map();
    let open = std::env::var("CLOSED").is_err();
    for a in (0xA848..0xA878u16).filter(|_| open) {
        let v = g.read(a) | 0x81;
        g.write(a, v);
    }
    g.show_map_at(p);
    let o = g.video().oam().to_vec();
    let mario = (0..40).find(|&i| o[i * 4 + 2] == 0x0A && o[i * 4] != 0).map(|i| (o[i * 4 + 1] as i32, o[i * 4] as i32 - 4));
    let at = match (mario, crate::rom_views::on_world_picture(&mut g)) {
        (Some((x, y)), Some((cx, cy))) => Some(((cx - 80 + x).rem_euclid(512), (cy - 72 + y).rem_euclid(512))),
        (m, _) => m,
    };
    println!("shown: point {:02x} map {:02x} mario at {at:?}", g.read(0xA840), g.read(0xA68B));
    match level {
        Some(l) => {
            println!("enter {l}: {}", cart_model::cheats::go_to_level(&mut g, l));
            for _ in 0..120 {
                g.run_frame_with(Buttons::NONE);
            }
            println!("finish: {}", cart_model::cheats::finish_level(&mut g));
        }
        None => {
            for _ in 0..30 {
                g.run_frame_with(Buttons::NONE);
            }
            let press = match std::env::var("PRESS").as_deref() {
                Ok("up") => Buttons::UP,
                Ok("down") => Buttons::DOWN,
                Ok("left") => Buttons::LEFT,
                Ok("right") => Buttons::RIGHT,
                Ok("none") => Buttons::NONE,
                _ => Buttons::A,
            };
            for _ in 0..6 {
                g.run_frame_with(press);
            }
        }
    }
    let mut last = (0xFFu8, 0xFFu8, 0xFFu8);
    for f in 0..3000 {
        g.run_frame_with(if level.is_some() && f % 60 < 3 { Buttons::A } else { Buttons::NONE });
        g.clear_audio();
        let now = (g.read(0xA840), g.read(0xA68B), g.read(GAME_MODE));
        if now != last {
            println!("frame {f:4}: point {:02x} map {:02x} mode {:02x}", now.0, now.1, now.2);
            last = now;
        }
    }
}

/// The ways (right, left, up, down) out of each point given, as the map's route tables have them.
pub fn ways(rom_path: &Path, points: &[u16]) {
    let Ok(rom) = std::fs::read(rom_path) else { return };
    let routes = cart_model::map_routes::read(&rom);
    for &p in points {
        let to: Vec<String> = routes.ways[p as usize].iter().map(|w| w.as_ref().map_or("-".into(), |w| format!("{:02x}", w.to))).collect();
        println!("{p:02x}: R {} L {} U {} D {}", to[0], to[1], to[2], to[3]);
    }
}

/// The game's own spots (`original_points`) Mario never stands on: no way in or out in the
/// route tables, and not where a zone's door or exit puts him.
pub fn spot_check(rom_path: &Path) {
    use cart_model::{doors, map_routes, original_points::SPOTS};
    let Ok(rom) = std::fs::read(rom_path) else { return };
    let routes = map_routes::read(&rom);
    let d = doors::read(&rom);
    for s in SPOTS {
        let p = s.point;
        let out = routes.ways[p as usize].iter().flatten().count();
        let into = routes.ways.iter().take(map_routes::POINTS).flatten().flatten().filter(|w| w.to == p).count();
        let door = d.door_of(p).is_some();
        let exit = d.exit_on(p).is_some();
        let twin = map_routes::twin(p).is_some();
        if out == 0 || into == 0 {
            println!("{p:02x} {:?}: out {out} in {into} door {door} exit {exit} twin {twin}", s.map);
        }
    }
}

/// Can level `a` show level `b`'s objects? For each object type `b` uses: its picture in `b`,
/// in `a` as it is, and in `a` with `b`'s enemy sprite set (0:362A entry) and, with
/// `with_common`, `b`'s common sets byte (0:360A) too.
pub fn combo_probe(rom_path: &Path, a: usize, b: usize, with_common: bool) {
    use cart_model::{art, objects, rom::offset};
    let Some(mut g) = crate::console::load(rom_path) else { return };
    let rom = g.cart().rom().to_vec();
    g.boot_to_map();
    let mut patched = rom.clone();
    let set = offset(0, 0x362A);
    patched.copy_within(set + 4 * b..set + 4 * b + 4, set + 4 * a);
    if with_common {
        let common = offset(0, 0x360A);
        patched[common + a] = rom[common + b];
    }
    let mut kinds: Vec<u8> = objects::read(&rom, b).0.iter().map(|o| o.kind).collect();
    kinds.sort();
    kinds.dedup();
    let (mut same_before, mut same_after, mut drawn) = (0, 0, 0);
    for &k in &kinds {
        let own = art::capture_look(&g, &rom, &rom, b, k);
        let before = art::capture_look(&g, &rom, &rom, a, k);
        let after = art::capture_look(&g, &patched, &rom, a, k);
        drawn += own.is_some() as usize;
        same_before += (own.is_some() && before == own) as usize;
        same_after += (own.is_some() && after == own) as usize;
        println!("type {k:02x}: own {} | in {a} as is {} | with {b}'s set {}", own.is_some(), before == own, after == own);
        if let (Some(o), Some(t)) = (&own, &after) {
            if o != t {
                let shades = |l: &art::Look| { let mut c = [0usize; 4]; l.px.iter().flatten().for_each(|&s| c[s as usize] += 1); c };
                println!("    own {}x{} at {},{} shades {:?} | there {}x{} at {},{} shades {:?}", o.w, o.h, o.dx, o.dy, shades(o), t.w, t.h, t.dx, t.dy, shades(t));
            }
        }
    }
    println!("level {a} with level {b}'s objects: {drawn} types drawn; same picture as is {same_before}, with the set {same_after}");
}

/// Level `level` with each value of the physics table (0:1F91, read into $A287): how high
/// Mario jumps holding A, how fast he falls back, and how far he runs right in 60 frames.
pub fn physics_probe(rom_path: &Path, level: u8) {
    use cart_model::rom::offset;
    let Some(base) = crate::console::load(rom_path) else { return };
    for v in [0u8, 1, 8] {
        let mut g = base.clone();
        let mut rom = g.cart().rom().to_vec();
        rom[offset(0, 0x1F91) + level as usize] = v;
        g.cart_mut().set_rom(&rom);
        g.boot_to_map();
        if !g.enter_level(level) {
            println!("{v}: level not entered");
            continue;
        }
        for _ in 0..90 {
            g.run_frame_with(Buttons::NONE);
        }
        let y = |g: &mut gb_core::Gb| g.read(PLAYER_Y) as i32 | (g.read(PLAYER_Y + 1) as i32) << 8;
        let x = |g: &mut gb_core::Gb| g.read(PLAYER_X) as i32 | (g.read(PLAYER_X + 1) as i32) << 8;
        let ground = y(&mut g);
        let (mut top, mut up_frames, mut down_frames) = (ground, 0, 0);
        for f in 0..240 {
            g.run_frame_with(if f < 40 { Buttons::A } else { Buttons::NONE });
            let now = y(&mut g);
            if now < top {
                top = now;
                up_frames = f;
            }
            if now >= ground && f > 2 && down_frames == 0 && top < ground {
                down_frames = f - up_frames;
            }
        }
        let x0 = x(&mut g);
        for _ in 0..60 {
            g.run_frame_with(Buttons::RIGHT);
        }
        println!("physics {v}: jump {} px in {up_frames} frames, back down in {down_frames}; ran {} px; $A287 = {:02x}", ground - top, x(&mut g) - x0, g.read(0xA287));
    }
}

/// Level `level` with byte `level` of the per-level table at bank 0 `table` set to `value`:
/// Mario, the camera ($FFC8 y, $FFCA x) and the game mode every 30 frames with no input.
pub fn table_probe(rom_path: &Path, table: u16, level: u8, value: u8) {
    use cart_model::rom::offset;
    let Some(mut g) = crate::console::load(rom_path) else { return };
    let mut rom = g.cart().rom().to_vec();
    rom[offset(0, table) + level as usize] = value;
    g.cart_mut().set_rom(&rom);
    g.boot_to_map();
    if !g.enter_level(level) {
        println!("level not entered");
        return;
    }
    for f in 0..600 {
        g.run_frame_with(Buttons::NONE);
        g.clear_audio();
        if f % 30 == 0 {
            let w = |g: &mut gb_core::Gb, a: u16| g.read(a) as u16 | (g.read(a + 1) as u16) << 8;
            println!("frame {f:3}: mario {},{} camera {},{} mode {:02x}", w(&mut g, PLAYER_X), w(&mut g, PLAYER_Y), w(&mut g, 0xFFCA), w(&mut g, 0xFFC8), g.read(GAME_MODE));
        }
    }
}

/// The camera margin (0:1F51) of `level` at 0 and at 8: Mario jumps while walking right, and
/// for each value the heights on the screen ($FF99) at which the camera moved up and down.
pub fn camera_probe(rom_path: &Path, level: u8) {
    use cart_model::rom::offset;
    let Some(base) = crate::console::load(rom_path) else { return };
    for v in [0u8, 8] {
        let mut g = base.clone();
        let mut rom = g.cart().rom().to_vec();
        rom[offset(0, 0x1F51) + level as usize] = v;
        g.cart_mut().set_rom(&rom);
        g.boot_to_map();
        if !g.enter_level(level) {
            println!("{v}: level not entered");
            continue;
        }
        let cam = |g: &mut gb_core::Gb| g.read(0xFFC8) as i32 | (g.read(0xFFC9) as i32) << 8;
        let (mut up, mut down) = (Vec::new(), Vec::new());
        let mut last = cam(&mut g);
        for f in 0..1500 {
            let b = if f % 70 < 30 { Buttons::A | Buttons::RIGHT } else { Buttons::RIGHT };
            g.run_frame_with(b);
            g.clear_audio();
            let now = cam(&mut g);
            let screen = g.read(0xFF99);
            if now < last && !up.contains(&screen) {
                up.push(screen);
            }
            if now > last && !down.contains(&screen) {
                down.push(screen);
            }
            last = now;
        }
        up.sort();
        down.sort();
        println!("margin {v}: camera up with Mario at screen y {:?}; down at {:?}", up.iter().take(3).chain(up.iter().rev().take(1)).collect::<Vec<_>>(), down.iter().take(1).chain(down.iter().rev().take(3)).collect::<Vec<_>>());
    }
}

/// Level `level` with the auto-scroll table (0:1F71) at `value`, from its start or (`near`)
/// `near` pixels left of its finish block; with `walk` Mario walks right and jumps now and
/// then. Prints Mario, the camera and the game mode when the mode changes and every 60 frames.
pub fn scroll_probe(rom_path: &Path, level: u8, value: u8, near: u16, walk: bool) {
    use cart_model::{level_goals::{self, Goal}, level_start::Start, levels, rom::offset};
    let Some(mut g) = crate::console::load(rom_path) else { return };
    let mut rom = g.cart().rom().to_vec();
    rom[offset(0, 0x1F71) + level as usize] = value;
    if near > 0 {
        let map = levels::decode(&rom, levels::map_location(&rom, levels::header(&rom, level as usize)).1).0;
        let Some(&(fx, fy)) = level_goals::cells(&map, Goal::Finish).first() else {
            println!("no finish");
            return;
        };
        let h = levels::header(&rom, level as usize).at;
        Start::standing((fx * 16) as u16 - near, (fy * 16) as u16).write(&mut rom[h..h + 8]);
        println!("finish block at {},{}", fx * 16, fy * 16);
    }
    g.cart_mut().set_rom(&rom);
    g.boot_to_map();
    if !g.enter_level(level) {
        println!("level not entered");
        return;
    }
    let w = |g: &mut gb_core::Gb, a: u16| g.read(a) as u16 | (g.read(a + 1) as u16) << 8;
    let mut last_mode = 0xFF;
    for f in 0..3600 {
        let b = if walk { if f % 50 < 12 { Buttons::RIGHT | Buttons::A } else { Buttons::RIGHT } } else { Buttons::NONE };
        g.run_frame_with(b);
        g.clear_audio();
        let mode = g.read(GAME_MODE);
        if mode != last_mode || f % 60 == 0 {
            println!("frame {f:4}: mario {},{} camera {} mode {mode:02x}", w(&mut g, PLAYER_X), w(&mut g, PLAYER_Y), w(&mut g, 0xFFCA));
            last_mode = mode;
        }
        if mode == 0x0C {
            break;
        }
    }
}

/// Level `level`'s animated tiles (0:35EA) as they are and turned off ($FF): over 120 frames,
/// how many times the first four background tiles ($9200-$923F) change, and whether the rest
/// of the background tiles end the same in both runs.
pub fn animation_probe(rom_path: &Path, level: u8) {
    use cart_model::rom::offset;
    let Some(base) = crate::console::load(rom_path) else { return };
    let mut ends = Vec::new();
    for off in [false, true] {
        let mut g = base.clone();
        let mut rom = g.cart().rom().to_vec();
        if off {
            rom[offset(0, 0x35EA) + level as usize] = 0xFF;
        }
        g.cart_mut().set_rom(&rom);
        g.boot_to_map();
        if !g.enter_level(level) {
            println!("level not entered");
            return;
        }
        let mut changes = 0;
        let mut last = g.video().vram()[0x1200..0x1240].to_vec();
        for _ in 0..120 {
            g.run_frame_with(Buttons::NONE);
            g.clear_audio();
            let now = g.video().vram()[0x1200..0x1240].to_vec();
            changes += (now != last) as usize;
            last = now;
        }
        println!("table {:02x}: first four tiles changed {changes} times", if off { 0xFF } else { rom[offset(0, 0x35EA) + level as usize] });
        ends.push(g.video().vram()[0x1240..0x1800].to_vec());
    }
    println!("rest of the background tiles the same: {}", ends[0] == ends[1]);
}

/// Level `level` played as it is and with the boss list (02:4DFF, `cp $05` first) giving it
/// index `index` (its number written into that `cp`): $AFC0-$AFCA, $A267, the music request,
/// the timer and how much of the screen differs after `frames` frames standing still.
pub fn boss_probe(rom_path: &Path, level: u8, index: usize, frames: u32) {
    use cart_model::rom::offset;
    const CPS: [u16; 6] = [0x4E03, 0x4E08, 0x4E0D, 0x4E12, 0x4E17, 0x4E1C];
    let Some(base) = crate::console::load(rom_path) else { return };
    let mut screens = Vec::new();
    for patched in [false, true] {
        let mut g = base.clone();
        let mut rom = g.cart().rom().to_vec();
        if patched {
            rom[offset(2, CPS[index])] = level;
        }
        // Mario a little left of the level's last object (a boss level's boss)
        let objs = cart_model::objects::read(&rom, level as usize).0;
        if let Some(last) = objs.iter().max_by_key(|o| o.x) {
            let h = cart_model::levels::header(&rom, level as usize).at;
            cart_model::level_start::Start::standing(last.x.saturating_sub(120), last.y).write(&mut rom[h..h + 8]);
            if !patched {
                println!("last object: type {:02x} at {},{}", last.kind, last.x, last.y);
            }
        }
        g.cart_mut().set_rom(&rom);
        g.boot_to_map();
        if !g.enter_level(level) {
            println!("level not entered");
            return;
        }
        for _ in 0..frames {
            g.run_frame_with(Buttons::NONE);
            g.clear_audio();
        }
        let r = |g: &mut gb_core::Gb, a: u16| g.read(a);
        println!("{}: AFC0 {:02x} AFC1 {:02x} AFC5 {:02x} AFCA {:02x} A267 {:02x} music {:02x} timer {:02x}{:02x} mode {:02x}", if patched { "patched" } else { "as is  " }, r(&mut g, 0xAFC0), r(&mut g, 0xAFC1), r(&mut g, 0xAFC5), r(&mut g, 0xAFCA), r(&mut g, 0xA267), r(&mut g, 0xA468), r(&mut g, 0xA255), r(&mut g, 0xA254), r(&mut g, GAME_MODE));
        screens.push(g.screen().to_vec());
    }
    let diff = screens[0].iter().zip(&screens[1]).filter(|(a, b)| a != b).count();
    println!("screen pixels that differ: {diff}");
}

/// The per-level tables and blank levels in the emulator: level 01's tables written into a copy
/// and read by the game; a new level with space gravity played over level 01, which keeps its
/// own; blank levels made from levels 01 and 18, entered with Mario standing on their floor.
pub fn tables_check(rom_path: &Path) {
    use cart_model::extra_levels::{apply, based_on, writes_for};
    use cart_model::level_start::Start;
    use cart_model::level_tables::Tables;
    use cart_model::{blank_level, patch};
    let Some(gb) = crate::console::load(rom_path) else { return };
    let Ok(rom) = patch::expand(gb.cart().rom()) else { return };
    let Ok(mut g0) = gb_core::Gb::new(rom.clone(), 44100) else { return };
    g0.boot_to_map();
    let game = |g: &mut gb_core::Gb| [g.read(0xA287), g.read(0xA2C8), g.read(0xA2D9), g.read(0xA299)];
    // a game level's own tables
    let mut copy = rom.clone();
    let t = Tables { gravity: 8, scroll: 1, camera: 8, animation: 0xFF };
    t.write(&mut copy, 1);
    let mut g = g0.clone();
    g.cart_mut().set_rom(&copy);
    let entered = g.enter_level(1);
    let got = game(&mut g);
    println!("level 01's tables read by the game: {}", if entered && got == [8, 1, 8, 0] { "ok".to_string() } else { format!("FAILED {got:02x?}") });
    // a new level with space gravity over level 01
    let mut e = based_on(&rom, 1);
    e.tables.set_space_gravity(true, &Tables::read(&rom, 1));
    let mut g = g0.clone();
    let undo = apply(g.cart_mut().rom_mut(), &writes_for(&rom, &e));
    let entered = g.enter_level(1);
    let new_gravity = g.read(0xA287);
    g.back_to_map();
    for _ in 0..300 {
        g.run_frame_with(Buttons::NONE);
    }
    apply(g.cart_mut().rom_mut(), &undo);
    let base_entered = g.enter_level(1);
    let base_gravity = g.read(0xA287);
    println!("new level with space gravity: {}", if entered && base_entered && new_gravity == 8 && base_gravity == 0 { "ok".to_string() } else { format!("FAILED new {new_gravity:02x} base {base_gravity:02x}") });
    // blank levels
    for base in [1u8, 18] {
        let mut e = based_on(&rom, base);
        let (map, start) = blank_level::blank(&e.map, &Start::read(&e.header));
        e.map = map;
        e.objs.clear();
        start.write(&mut e.header);
        let mut g = g0.clone();
        apply(g.cart_mut().rom_mut(), &writes_for(&rom, &e));
        let entered = g.enter_level(base);
        let y0 = g.read(PLAYER_Y) as u16 | (g.read(PLAYER_Y + 1) as u16) << 8;
        for _ in 0..180 {
            g.run_frame_with(Buttons::NONE);
        }
        let y1 = g.read(PLAYER_Y) as u16 | (g.read(PLAYER_Y + 1) as u16) << 8;
        let playing = g.read(GAME_MODE) == 0x04;
        println!("blank level from {base:02}: {}", if entered && playing && y0 == y1 && y0 == start.mario_y { "ok".to_string() } else { format!("FAILED entered {entered} playing {playing} y {y0} -> {y1} (start {})", start.mario_y) });
    }
}

/// For each object type: in how many enemy-sprite groups (`level_graphics`) of each catalogue
/// the game's levels use it.
pub fn type_groups(rom_path: &Path) {
    use cart_model::{level_graphics, objects};
    let Ok(rom) = std::fs::read(rom_path) else { return };
    let mut groups: Vec<([u8; 4], bool)> = Vec::new();
    let mut uses: Vec<Vec<usize>> = vec![Vec::new(); 128];
    for l in 0..32 {
        let g = level_graphics::object_group(&rom, l);
        let gi = groups.iter().position(|x| *x == g).unwrap_or_else(|| { groups.push(g); groups.len() - 1 });
        for o in objects::read(&rom, l).0 {
            if !uses[o.kind as usize].contains(&gi) {
                uses[o.kind as usize].push(gi);
            }
        }
    }
    let per_cat = |cat: bool| groups.iter().filter(|g| g.1 == cat).count();
    println!("groups: {} in levels 0-9, {} in 10-31", per_cat(false), per_cat(true));
    for (k, u) in uses.iter().enumerate().filter(|(_, u)| !u.is_empty()) {
        let (a, b) = (u.iter().filter(|&&g| !groups[g].1).count(), u.iter().filter(|&&g| groups[g].1).count());
        println!("type {k:02x}: {a} / {b}");
    }
}

/// Objects near Mario's start: a blank level made from `base` with one object of `kind`
/// `dx` pixels right of Mario at his height; the objects alive after the level begins, after
/// walking right a little, and after walking back.
pub fn start_objects_probe(rom_path: &Path, base: u8, kind: u8, dx: u16) {
    use cart_model::extra_levels::{apply, based_on, writes_for};
    use cart_model::level_start::Start;
    use cart_model::objects::Obj;
    use cart_model::{blank_level, cheats, patch};
    let Some(gb) = crate::console::load(rom_path) else { return };
    let Ok(rom) = patch::expand(gb.cart().rom()) else { return };
    let Ok(mut g) = gb_core::Gb::new(rom.clone(), 44100) else { return };
    g.boot_to_map();
    let mut e = based_on(&rom, base);
    let (map, start) = blank_level::blank(&e.map, &Start::read(&e.header));
    e.map = map;
    let feet = start.mario_y + cart_model::level_start::FEET;
    e.objs = vec![Obj { x: (start.mario_x + dx) / 8 * 8, y: (feet - 16) / 8 * 8, kind, flag: false }];
    start.write(&mut e.header);
    apply(g.cart_mut().rom_mut(), &writes_for(&rom, &e));
    g.enter_level(base);
    let report = |g: &mut gb_core::Gb, when: &str| {
        let a = cheats::active_objects(g);
        println!("{when}: {} alive {:?}", a.len(), a.iter().map(|o| (o.kind, o.x, o.y)).collect::<Vec<_>>());
    };
    for _ in 0..30 {
        g.run_frame_with(Buttons::NONE);
    }
    report(&mut g, "after the start");
    for _ in 0..40 {
        g.run_frame_with(Buttons::RIGHT);
    }
    report(&mut g, "walked right");
    for _ in 0..120 {
        g.run_frame_with(Buttons::RIGHT);
    }
    report(&mut g, "walked further");
}

/// The game's levels' objects that are on the first screen (the camera of the level's start).
pub fn first_screen_objects(rom_path: &Path) {
    use cart_model::{level_start::Start, levels, objects};
    let Ok(rom) = std::fs::read(rom_path) else { return };
    for l in 0..32 {
        let h = levels::header(&rom, l).at;
        let s = Start::read(&rom[h..h + 8]);
        let (left, right) = (s.camera_x as i32 - 0x50, s.camera_x as i32 + 0x50);
        let (top, bottom) = (s.camera_y as i32 - 0x48, s.camera_y as i32 + 0x48);
        let on: Vec<(u8, u16, u16)> = objects::read(&rom, l).0.iter().filter(|o| (left..right).contains(&(o.x as i32)) && (top..bottom).contains(&(o.y as i32))).map(|o| (o.kind, o.x, o.y)).collect();
        if !on.is_empty() {
            println!("level {l:02}: screen x {left}-{right} y {top}-{bottom}: {on:?}");
        }
    }
}

/// Every type the game's levels use: the sprite tiles it shows over 2 and over 10 seconds
/// alone beside Mario in its own level, and whether the short run missed any.
pub fn object_tiles(rom_path: &Path) {
    use cart_model::{art, objects};
    let Some(g) = crate::console::load(rom_path) else { return };
    let original = g.cart().rom().to_vec();
    // the captures play each level as a blank one, which needs the expanded copy
    let Ok(mut rom) = cart_model::patch::expand(&original) else { return };
    cart_model::patch::upgrade(&mut rom);
    let Ok(mut g) = gb_core::Gb::new(rom.clone(), 44100) else { return };
    g.boot_to_map();
    let (mut kinds, mut missed) = (0, 0);
    for l in 0..32 {
        let mut ks: Vec<u8> = objects::read(&rom, l).0.iter().map(|o| o.kind).collect();
        ks.sort_unstable();
        ks.dedup();
        for k in ks {
            let short = art::object_tiles(&g, &rom, &original, l, k, 120);
            let long = art::object_tiles(&g, &rom, &original, l, k, 600);
            kinds += 1;
            let more: Vec<u8> = long.iter().copied().filter(|t| !short.contains(t)).collect();
            missed += !more.is_empty() as usize;
            println!("level {l:02} type {k:02x}: {:02x?}{}", long, if more.is_empty() { String::new() } else { format!("  (2 s missed {more:02x?})") });
        }
    }
    println!("{kinds} level types, {missed} with tiles only the long run saw");
}

/// The game's objects once each, per catalogue (levels 0-9, 10-31): the (level, type) pairs
/// whose captured pictures have the same pixels form one object. Prints each with its index,
/// type numbers, levels and sprite sets, and writes `DIR/catalogue0.png` and `catalogue1.png`:
/// the pictures in index order at twice their size, 8 per row, 80 pixels a cell.
pub fn object_list(rom_path: &Path, dir: &Path) {
    use cart_model::{art, level_graphics, objects};
    use gb_core::palette::PALETTES;
    let Some(g) = crate::console::load(rom_path) else { return };
    let original = g.cart().rom().to_vec();
    // the captures play each level as a blank one, which needs the expanded copy
    let Ok(mut rom) = cart_model::patch::expand(&original) else { return };
    cart_model::patch::upgrade(&mut rom);
    let Ok(mut g) = gb_core::Gb::new(rom.clone(), 44100) else { return };
    g.boot_to_map();
    for cat in [false, true] {
        // (picture, [(level, type)])
        let mut found: Vec<(Option<art::Look>, Vec<(usize, u8)>)> = Vec::new();
        for l in (0..32).filter(|&l| level_graphics::object_group(&rom, l).1 == cat) {
            let mut ks: Vec<u8> = objects::read(&rom, l).0.iter().map(|o| o.kind).collect();
            ks.sort_unstable();
            ks.dedup();
            for k in ks {
                let look = art::capture_look(&g, &rom, &original, l, k);
                let same = |o: &(Option<art::Look>, Vec<(usize, u8)>)| match (&o.0, &look) {
                    (Some(a), Some(b)) => (a.w, a.h, &a.px) == (b.w, b.h, &b.px),
                    (None, None) => o.1.iter().any(|&(_, t)| t == k),
                    _ => false,
                };
                match found.iter_mut().find(|o| same(o)) {
                    Some(o) => o.1.push((l, k)),
                    None => found.push((look, vec![(l, k)])),
                }
            }
        }
        println!("== catalogue {} ({} objects)", if cat { "10-31" } else { "0-9" }, found.len());
        for (i, (look, uses)) in found.iter().enumerate() {
            let mut types: Vec<u8> = uses.iter().map(|u| u.1).collect();
            types.sort_unstable();
            types.dedup();
            let levels: Vec<usize> = uses.iter().map(|u| u.0).collect();
            let what = look.as_ref().map_or("draws nothing".to_string(), |lk| format!("{}x{} sets {}", lk.w, lk.h, lk.sets));
            println!("{i:3}: types {types:02x?} levels {levels:?} {what}");
        }
        let (cell, per_row, zoom) = (80usize, 8usize, 2usize);
        let rows = found.len().div_ceil(per_row).max(1);
        let (w, h) = (per_row * cell, rows * cell);
        let mut rgba = vec![255u8; w * h * 4];
        for (i, (look, _)) in found.iter().enumerate() {
            let Some(l) = look else { continue };
            let (ox, oy) = ((i % per_row) * cell, (i / per_row) * cell);
            for y in 0..(l.h * zoom).min(cell - 2) {
                for x in 0..(l.w * zoom).min(cell - 2) {
                    if let Some(s) = l.px[(y / zoom) * l.w + x / zoom] {
                        let c = PALETTES[0][s as usize];
                        let p = ((oy + y) * w + ox + x) * 4;
                        rgba[p] = (c >> 16) as u8;
                        rgba[p + 1] = (c >> 8) as u8;
                        rgba[p + 2] = c as u8;
                    }
                }
            }
            // a grey line under and right of each cell, to count them
            for k in 0..cell {
                for (x, y) in [(ox + k, oy + cell - 1), (ox + cell - 1, oy + k)] {
                    let p = (y * w + x) * 4;
                    rgba[p..p + 3].copy_from_slice(&[200, 200, 200]);
                }
            }
        }
        let out = dir.join(format!("catalogue{}.png", cat as u8));
        if let Err(e) = host::png_out::write_rgba(&out, w, h, &rgba) {
            eprintln!("{e}");
        }
    }
}

/// Level `level` entered and RIGHT held for 150 frames (as the object capture does): Mario and
/// the camera every 30 frames.
pub fn walk_probe(rom_path: &Path, level: u8) {
    let Some(mut g) = crate::console::load(rom_path) else { return };
    g.boot_to_map();
    if !g.enter_level(level) {
        println!("not entered");
        return;
    }
    let w = |g: &mut gb_core::Gb, a: u16| g.read(a) as u16 | (g.read(a + 1) as u16) << 8;
    for f in 0..150 {
        if f % 30 == 0 {
            println!("frame {f:3}: mario {},{} camera {} mode {:02x}", w(&mut g, PLAYER_X), w(&mut g, PLAYER_Y), w(&mut g, 0xFFCA), g.read(GAME_MODE));
        }
        g.run_frame_with(Buttons::RIGHT);
    }
}

/// Can level `base` show its own enemies and `kind` of level `other` (same catalogue) at once?
/// Builds a $380-byte enemy sprite block: `base`'s, with `kind`'s enemy-area tiles taken from
/// `other`'s; puts it where the low copy of `base`'s compressed map was (an expanded copy
/// reads maps from the twin banks), points `base`'s 0:362A entry at it, and compares each
/// picture with the one in its own level.
pub fn enemy_mix(rom_path: &Path, base: usize, other: usize, kind: u8) {
    use cart_model::{art, levels, objects, patch, rom::{offset, word}};
    let Some(g) = crate::console::load(rom_path) else { return };
    let original = g.cart().rom().to_vec();
    let Ok(mut rom) = patch::expand(&original) else { return };
    patch::upgrade(&mut rom);
    let Ok(mut g) = gb_core::Gb::new(rom.clone(), 44100) else { return };
    g.boot_to_map();
    let enemy = |t: &u8| (0xB0..=0xE7).contains(t);
    let mine: Vec<u8> = {
        let mut k: Vec<u8> = objects::read(&rom, base).0.iter().map(|o| o.kind).collect();
        k.sort_unstable();
        k.dedup();
        k
    };
    let mut used = Vec::new();
    for &k in &mine {
        let t: Vec<u8> = art::object_tiles(&g, &rom, &original, base, k, 600).into_iter().filter(enemy).collect();
        println!("level {base:02} type {k:02x}: enemy tiles {t:02x?}");
        used.extend(t);
    }
    let theirs: Vec<u8> = art::object_tiles(&g, &rom, &original, other, kind, 600).into_iter().filter(enemy).collect();
    let clash: Vec<u8> = theirs.iter().copied().filter(|t| used.contains(t)).collect();
    println!("level {other:02} type {kind:02x}: enemy tiles {theirs:02x?}; clashes with level {base:02}'s: {clash:02x?}");
    // the two sets as the game loads them: bank, -, address
    let set = |l: usize| { let e = offset(0, 0x362A) + l * 4; (rom[e] as usize, word(&rom, e + 2)) };
    let (bb, ba) = set(base);
    let (ob, oa) = set(other);
    let mut block = rom[offset(bb, ba)..offset(bb, ba) + 0x380].to_vec();
    for &t in &theirs {
        let i = (t - 0xB0) as usize * 16;
        block[i..i + 16].copy_from_slice(&rom[offset(ob, oa) + i..offset(ob, oa) + i + 16]);
    }
    let h = levels::header(&rom, base);
    let at = word(&rom, levels::map_location(&rom, h).0);
    let free = offset(h.bank as usize, at);
    let mut mixed = rom.clone();
    mixed[free..free + 0x380].copy_from_slice(&block);
    let e = offset(0, 0x362A) + base * 4;
    mixed[e] = h.bank;
    mixed[e + 2..e + 4].copy_from_slice(&at.to_le_bytes());
    for (pos, w) in [(e, h.bank), (e + 2, at as u8), (e + 3, (at >> 8) as u8)] {
        mixed[pos + patch::TWIN] = w;
    }
    println!("block at {:02x}:{at:04x}", h.bank);
    for &k in &mine {
        let own = art::capture_look(&g, &rom, &original, base, k);
        let now = art::capture_look(&g, &mixed, &original, base, k);
        println!("level {base:02} type {k:02x} with the mix: {}", if own == now { "same picture" } else { "DIFFERENT" });
    }
    let own = art::capture_look(&g, &rom, &original, other, kind);
    let now = art::capture_look(&g, &mixed, &original, base, kind);
    let pixels = |l: &Option<art::Look>| l.as_ref().map(|l| (l.w, l.h, l.px.clone()));
    println!("level {other:02} type {kind:02x} in level {base:02} with the mix: {}", if own.is_some() && pixels(&own) == pixels(&now) { "same picture as in its own level" } else { "DIFFERENT" });
}

/// `enemy_sets` on a saved copy: level 01 with its hedgehog and bee and level 00's turtle,
/// stored with `enemy_sets::store`; each picture compared with its own level's, and the set
/// read back.
pub fn enemy_set_check(rom_path: &Path) {
    use cart_model::enemy_sets::{self, Enemy};
    use cart_model::{art, patch};
    let Some(g) = crate::console::load(rom_path) else { return };
    let original = g.cart().rom().to_vec();
    let Ok(mut rom) = patch::expand(&original) else { return };
    patch::upgrade(&mut rom);
    let Ok(mut g) = gb_core::Gb::new(rom.clone(), 44100) else { return };
    g.boot_to_map();
    let members = [Enemy { level: 1, kind: 0x21 }, Enemy { level: 1, kind: 0x22 }, Enemy { level: 0, kind: 0x01 }];
    let with_tiles: Vec<(Enemy, Vec<u8>)> = members.iter().map(|&e| (e, art::object_tiles(&g, &rom, &original, e.level as usize, e.kind, 600))).collect();
    for (i, (a, ta)) in with_tiles.iter().enumerate() {
        for (b, tb) in &with_tiles[i + 1..] {
            if enemy_sets::clash(ta, tb) {
                println!("clash: {a:?} and {b:?}");
            }
        }
    }
    let block = enemy_sets::build(&original, 1, &with_tiles);
    let mut copy = rom.clone();
    if let Err(e) = enemy_sets::store(&mut copy, &original, &[(1, members.to_vec(), block)]) {
        println!("store: {e}");
        return;
    }
    let back = enemy_sets::read(&copy);
    println!("read back: {}", if back.len() == 1 && back[0].level == 1 && back[0].enemies == members { "ok".to_string() } else { format!("FAILED {back:?}") });
    println!("slots: {}", enemy_sets::slots(&original).len());
    let pixels = |l: &Option<art::Look>| l.as_ref().map(|l| (l.w, l.h, l.px.clone()));
    for e in members {
        let own = art::capture_look(&g, &rom, &original, e.level as usize, e.kind);
        let now = art::capture_look(&g, &copy, &original, 1, e.kind);
        println!("{e:?} in level 01: {}", if own.is_some() && pixels(&own) == pixels(&now) { "as in its own level" } else { "DIFFERENT" });
    }
    let mut none = copy.clone();
    let _ = enemy_sets::store(&mut none, &original, &[]);
    println!("no sets: level 01's entry back to the game's: {}", enemy_sets::own_block(&none, 1) == enemy_sets::own_block(&original, 1));
}

/// What one object capture costs: entering a level, a frame, a whole `capture_look` and
/// `object_tiles`, timed in an expanded copy.
pub fn capture_bench(rom_path: &Path) {
    use cart_model::{art, patch};
    use std::time::Instant;
    let Some(g) = crate::console::load(rom_path) else { return };
    let original = g.cart().rom().to_vec();
    let Ok(mut rom) = patch::expand(&original) else { return };
    patch::upgrade(&mut rom);
    let Ok(mut g) = gb_core::Gb::new(rom.clone(), 44100) else { return };
    g.boot_to_map();
    let t = Instant::now();
    let mut e = g.clone();
    e.enter_level(1);
    println!("enter a level: {:?}", t.elapsed());
    let t = Instant::now();
    for _ in 0..600 {
        e.run_frame_with(Buttons::NONE);
        e.clear_audio();
    }
    println!("600 frames: {:?} ({:?} each)", t.elapsed(), t.elapsed() / 600);
    for rate in [8000u32, 1000] {
        let Ok(mut q) = gb_core::Gb::new(rom.clone(), rate) else { return };
        q.boot_to_map();
        q.enter_level(1);
        let t = Instant::now();
        for _ in 0..600 {
            q.run_frame_with(Buttons::NONE);
            q.clear_audio();
        }
        println!("600 frames with sound at {rate} Hz: {:?}", t.elapsed());
    }
    let t = Instant::now();
    for k in [0x1F, 0x21, 0x22, 0x1B, 0x40] {
        art::capture_look(&g, &rom, &original, 1, k);
    }
    println!("capture_look: {:?} each", t.elapsed() / 5);
    let t = Instant::now();
    art::object_tiles(&g, &rom, &original, 1, 0x21, 600);
    println!("object_tiles (600 frames): {:?}", t.elapsed());
}

/// Remembers every byte written, and at each read of $AF06 from one of the four sites that
/// pick the object catalogue, what the current object's copy in $FFD0-$FFDF says and which
/// object slot ($AD00 + $20 n) it matches.
#[derive(Clone)]
pub struct Shadow {
    mem: Vec<u8>,
    /// (site pc, $FFD5, $FFDB, the slot whose first bytes match $FFD0, that slot's type, the
    /// slot $AF28/$AF29 points at)
    pub log: Vec<(u16, u8, u8, Option<u16>, u8, u16)>,
}

/// The `ld a,($AF06)` sites (pc of the instruction), bank 0 addresses as the CPU sees them.
const CATALOGUE_SITES: [u16; 4] = [0x4036, 0x424D, 0x41D2, 0x400B];

impl gb_core::BusObserver for Shadow {
    const ENABLED: bool = true;

    fn on_read(&mut self, pc: u16, _bank: usize, addr: u16) {
        if addr != 0xAF06 || !CATALOGUE_SITES.contains(&pc) || self.log.len() > 200_000 {
            return;
        }
        let m = &self.mem;
        let copy = &m[0xFFD0..0xFFD6];
        let slot = (0..16u16).map(|n| 0xAD00 + n * 0x20).find(|&s| m[s as usize + 8] != 0 && &m[s as usize..s as usize + 6] == copy);
        let kind = slot.map_or(0, |s| m[s as usize + 5]);
        let current = (m[0xAF28] as u16) << 8 | (m[0xAF29] & 0xF0) as u16;
        self.log.push((pc, m[0xFFD5], m[0xFFDB], slot, kind, current));
    }

    fn on_write(&mut self, _pc: u16, _bank: usize, addr: u16, value: u8) {
        self.mem[addr as usize] = value;
    }
}

/// Level `level` played for `frames` frames (walking right, jumping now and then) with a
/// `Shadow` watching: at each of the four reads of $AF06, whether $FFD5 is the type of the
/// object slot being handled.
pub fn catalogue_reads(rom_path: &Path, level: u8, frames: u32) {
    let Ok(data) = std::fs::read(rom_path) else { return };
    let Ok(mut g) = gb_core::GameBoy::with_observer(data, 1000, Shadow { mem: vec![0; 0x10000], log: Vec::new() }) else { return };
    g.boot_to_map();
    g.enter_level(level);
    for f in 0..frames {
        g.run_frame_with(if f % 60 < 10 { Buttons::RIGHT | Buttons::A } else { Buttons::RIGHT });
        g.clear_audio();
    }
    let log = &g.observer().log;
    for site in CATALOGUE_SITES {
        let at: Vec<_> = log.iter().filter(|e| e.0 == site).collect();
        let found = at.iter().filter(|e| e.3.is_some()).count();
        let same = at.iter().filter(|e| e.3.is_some() && e.1 == e.4).count();
        let kinds: std::collections::BTreeSet<u8> = at.iter().map(|e| e.1).collect();
        println!("site {site:04x}: {} reads, {found} with a matching slot, {same} with $FFD5 = its type; types seen {kinds:02x?}", at.len());
        let pointed = at.iter().filter(|e| e.3 == Some(e.5)).count();
        println!("    $AF28 points at the matching slot: {pointed} of {found}");
        let lost: std::collections::BTreeMap<u8, usize> = at.iter().filter(|e| e.3.is_none()).fold(Default::default(), |mut m, e| {
            *m.entry(e.1).or_default() += 1;
            m
        });
        if !lost.is_empty() {
            println!("    no matching slot, by $FFD5: {lost:02x?}");
        }
    }
}

/// The catalogue hook by hand: `ld a,($AF06)` at the four sites becomes `call HOOK`, and HOOK
/// returns, for a type whose byte in RAM at $A380 is set, that byte minus one (its own
/// catalogue, whatever $AF06 says), else $AF06.
const MIX_HOOK_AT: u16 = 0x1967;
const MIX_TABLE: u16 = 0xA380;
const MIX_SITES: [(usize, u16); 4] = [(0x16, 0x4036), (0x02, 0x424D), (0x02, 0x41D2), (0x03, 0x400B)];

fn mix_hook(rom: &mut [u8]) {
    use cart_model::{patch::TWIN, rom::offset};
    // push hl / ldh a,($FFD5) / or $80 / ld l,a / ld h,$A3 / ld a,(hl) / pop hl / and a / jr z,+2 / dec a / ret / ld a,($AF06) / ret
    let code = [0xE5, 0xF0, 0xD5, 0xF6, 0x80, 0x6F, 0x26, (MIX_TABLE >> 8) as u8, 0x7E, 0xE1, 0xA7, 0x28, 0x02, 0x3D, 0xC9, 0xFA, 0x06, 0xAF, 0xC9];
    for base in [0, TWIN] {
        let at = base + MIX_HOOK_AT as usize;
        rom[at..at + code.len()].copy_from_slice(&code);
    }
    for (bank, addr) in MIX_SITES {
        let at = offset(bank, addr);
        assert_eq!(rom[at..at + 3], [0xFA, 0x06, 0xAF], "site {bank:02x}:{addr:04x}");
        rom[at..at + 3].copy_from_slice(&[0xCD, MIX_HOOK_AT as u8, (MIX_HOOK_AT >> 8) as u8]);
    }
}

/// One object of type `kind` on `level`'s capture start (`art::capture_rom`) in `rom`, Mario
/// walking at it and jumping now and then: each frame, the object's place from Mario and
/// whether it is alive, and the game mode.
fn mix_trace(map_state: &gb_core::Gb, rom: &[u8], original: &[u8], level: usize, kind: u8, mark: u8) -> Vec<(Option<(i32, i32)>, u8)> {
    use cart_model::{art, cheats, objects::Obj};
    let base = art::capture_rom(rom, original, level, Vec::new()).unwrap();
    let mut probe = map_state.clone();
    probe.cart_mut().set_rom(&base);
    probe.enter_level(level as u8);
    let mx = probe.read(PLAYER_X) as u16 | (probe.read(PLAYER_X + 1) as u16) << 8;
    let my = probe.read(PLAYER_Y) as u16 | (probe.read(PLAYER_Y + 1) as u16) << 8;
    let r = art::capture_rom(rom, original, level, vec![Obj { x: (mx + 216) / 8 * 8, y: my / 8 * 8, kind, flag: false }]).unwrap();
    let mut g = map_state.clone();
    g.cart_mut().set_rom(&r);
    g.write(MIX_TABLE + kind as u16, mark);
    g.enter_level(level as u8);
    let mut out = Vec::new();
    for f in 0..600 {
        g.run_frame_with(if f % 45 < 12 { Buttons::RIGHT | Buttons::A } else { Buttons::RIGHT });
        g.clear_audio();
        let mario = (g.read(PLAYER_X) as i32 | (g.read(PLAYER_X + 1) as i32) << 8, g.read(PLAYER_Y) as i32 | (g.read(PLAYER_Y + 1) as i32) << 8);
        let cam = (g.read(0xFFCA) as i32 | (g.read(0xFFCB) as i32) << 8, g.read(0xFFC8) as i32 | (g.read(0xFFC9) as i32) << 8);
        let it = cheats::active_objects(&mut g).into_iter().find(|o| o.kind == kind).map(|o| (o.x + cam.0 - 0x50 - mario.0, o.y + cam.1 - 0x48 - mario.1));
        out.push((it, g.read(GAME_MODE)));
    }
    out
}

/// Can level `into` (the other object catalogue) use type `kind` of level `from`? The hook by
/// hand, `into` given `kind`'s tiles through an enemy set (its own enemies that clash dropped),
/// `kind` marked foreign; the picture and a played trace compared with `from`'s.
pub fn mix_probe(rom_path: &Path, from: usize, kind: u8, into: usize) {
    use cart_model::enemy_sets::{self, Enemy};
    use cart_model::{art, objects, patch};
    let Some(g) = crate::console::load(rom_path) else { return };
    let original = g.cart().rom().to_vec();
    let Ok(mut rom) = patch::expand(&original) else { return };
    patch::upgrade(&mut rom);
    let Ok(mut g) = gb_core::Gb::new(rom.clone(), 1000) else { return };
    g.boot_to_map();
    let theirs = art::object_tiles(&g, &rom, &original, from, kind, 600);
    let mut kinds: Vec<u8> = objects::read(&original, into).0.iter().map(|o| o.kind).collect();
    kinds.sort_unstable();
    kinds.dedup();
    let mut members = Vec::new();
    for k in kinds {
        let t = art::object_tiles(&g, &rom, &original, into, k, 600);
        if !enemy_sets::area_tiles(&t).is_empty() && !enemy_sets::clash(&t, &theirs) && k != kind {
            members.push((Enemy { level: into as u8, kind: k }, t));
        }
    }
    let kept: Vec<String> = members.iter().map(|m| format!("{:02x}", m.0.kind)).collect();
    members.push((Enemy { level: from as u8, kind }, theirs.clone()));
    let block = enemy_sets::build(&original, into, &members);
    let mut mixed = rom.clone();
    enemy_sets::store(&mut mixed, &original, &[(into, members.iter().map(|m| m.0).collect(), block)]).unwrap();
    mix_hook(&mut mixed);
    println!("level {into:02}'s own enemies kept: {kept:?}; {:02x}'s tiles {theirs:02x?}", kind);
    let mut gm = g.clone();
    gm.cart_mut().set_rom(&mixed);
    gm.write(MIX_TABLE + kind as u16, 1 + (from >= 10) as u8);
    let pixels = |l: &Option<art::Look>| l.as_ref().map(|l| (l.w, l.h, l.px.clone()));
    let own = art::capture_look(&g, &rom, &original, from, kind);
    let there = art::capture_look(&gm, &mixed, &original, into, kind);
    let plain = art::capture_look(&g, &rom, &original, into, kind);
    println!("picture in level {into:02} with the hook: {}; without it (level {into:02}'s own ${kind:02x}): {}", if own.is_some() && pixels(&own) == pixels(&there) { "as in its own level" } else { "DIFFERENT" }, if pixels(&plain) == pixels(&own) { "the same" } else { "another object" });
    let a = mix_trace(&g, &rom, &original, from, kind, 0);
    let b = mix_trace(&gm, &mixed, &original, into, kind, 1 + (from >= 10) as u8);
    let first = a.iter().zip(&b).position(|(x, y)| x != y);
    let alive = |t: &[(Option<(i32, i32)>, u8)]| t.iter().filter(|e| e.0.is_some()).count();
    let lost = |t: &[(Option<(i32, i32)>, u8)]| t.iter().position(|e| e.1 == 0x09);
    println!("trace over 600 frames: {}; alive {} / {} frames; Mario hurt at {:?} / {:?}", match first { None => "the same".to_string(), Some(f) => format!("differs from frame {f}: {:?} / {:?}", a[f], b[f]) }, alive(&a), alive(&b), lost(&a), lost(&b));
    // the level's own enemies with the hook in: as without it
    for m in members.iter().filter(|m| m.0.level as usize == into) {
        let k = m.0.kind;
        let plain = mix_trace(&g, &rom, &original, into, k, 0);
        let hooked = mix_trace(&gm, &mixed, &original, into, k, 0);
        println!("level {into:02}'s own ${k:02x} with the hook: {}", if plain == hooked { "the same" } else { "DIFFERENT" });
    }
}

/// Is bank 0 `lo`-`hi` ever run or read? Boot, the map, and every level entered and walked
/// for a while, with a probe on the range.
pub fn unused_check(rom_path: &Path, lo: u16, hi: u16, bank: Option<usize>) {
    let Some(mut g) = probed(rom_path) else { return };
    g.observer_mut().run_watch = Some((lo, hi));
    g.observer_mut().run_bank = bank;
    g.observer_mut().read_watch = Some((lo, hi, bank.unwrap_or(0xFF)));
    g.boot_to_map();
    for l in 0..32u8 {
        g.enter_level(l);
        for f in 0..400 {
            g.run_frame_with(if f % 50 < 10 { Buttons::RIGHT | Buttons::A } else { Buttons::RIGHT });
            g.clear_audio();
        }
        g.back_to_map();
        for _ in 0..200 {
            g.run_frame_with(Buttons::NONE);
            g.clear_audio();
        }
    }
    let o = g.observer();
    println!("{lo:04x}-{hi:04x}: run {} times, read {} times {:04x?}; reads (pc, addr) {:04x?}", o.run_log.len(), o.read_log.len(), o.run_log.iter().take(5).collect::<Vec<_>>(), o.read_log.iter().take(5).map(|r| (r.0, r.1)).collect::<Vec<_>>());
}

/// How an object of one catalogue does in a level of the other (`mix_check`).
#[derive(Clone, Debug)]
pub struct MixResult {
    pub from: usize,
    pub kind: u8,
    pub into: usize,
    pub picture: bool,
    pub trace: bool,
    pub dropped: Vec<u8>,
    pub note: String,
}

/// `mix_probe` without printing: type `kind` of level `from` in level `into` with the hook,
/// its picture and its played trace against its own level's.
pub fn mix_check(g: &gb_core::Gb, rom: &[u8], original: &[u8], from: usize, kind: u8, into: usize) -> MixResult {
    use cart_model::enemy_sets::{self, Enemy};
    use cart_model::{art, objects};
    let theirs = art::object_tiles(g, rom, original, from, kind, 600);
    let mut kinds: Vec<u8> = objects::read(original, into).0.iter().map(|o| o.kind).collect();
    kinds.sort_unstable();
    kinds.dedup();
    let (mut members, mut dropped) = (Vec::new(), Vec::new());
    for k in kinds {
        let t = art::object_tiles(g, rom, original, into, k, 600);
        if enemy_sets::area_tiles(&t).is_empty() {
            continue;
        }
        if enemy_sets::clash(&t, &theirs) || k == kind {
            dropped.push(k);
        } else {
            members.push((Enemy { level: into as u8, kind: k }, t));
        }
    }
    members.push((Enemy { level: from as u8, kind }, theirs));
    let block = enemy_sets::build(original, into, &members);
    let mut mixed = rom.to_vec();
    let _ = enemy_sets::store(&mut mixed, original, &[(into, members.iter().map(|m| m.0).collect(), block)]);
    mix_hook(&mut mixed);
    let mut gm = g.clone();
    gm.cart_mut().set_rom(&mixed);
    gm.write(MIX_TABLE + kind as u16, 1 + (from >= 10) as u8);
    let pixels = |l: &Option<art::Look>| l.as_ref().map(|l| (l.w, l.h, l.px.clone()));
    let own = art::capture_look(g, rom, original, from, kind);
    let there = art::capture_look(&gm, &mixed, original, into, kind);
    let a = mix_trace(g, rom, original, from, kind, 0);
    let b = mix_trace(&gm, &mixed, original, into, kind, 1 + (from >= 10) as u8);
    let note = match a.iter().zip(&b).position(|(x, y)| x != y) {
        None => String::new(),
        Some(f) => format!("differs from frame {f}"),
    };
    MixResult { from, kind, into, picture: pixels(&own) == pixels(&there), trace: a == b, dropped, note }
}

/// Every type the game's levels use, tried in a level of the other catalogue (types of levels
/// 0-9 in `into_second`, of 10-31 in `into_first`), once per type and picture, on all cores.
pub fn mix_all(rom_path: &Path, into_first: usize, into_second: usize) {
    use cart_model::{art, level_graphics, objects, patch};
    let Some(g) = crate::console::load(rom_path) else { return };
    let original = g.cart().rom().to_vec();
    let Ok(mut rom) = patch::expand(&original) else { return };
    patch::upgrade(&mut rom);
    let Ok(mut g) = gb_core::Gb::new(rom.clone(), 1000) else { return };
    g.boot_to_map();
    // one (level, type) per catalogue, type and picture
    let mut jobs: Vec<(usize, u8)> = Vec::new();
    let mut seen: Vec<(bool, u8, Option<(usize, usize, Vec<Option<u8>>)>)> = Vec::new();
    for l in 0..32 {
        let second = level_graphics::object_group(&original, l).1;
        let mut ks: Vec<u8> = objects::read(&original, l).0.iter().map(|o| o.kind).collect();
        ks.sort_unstable();
        ks.dedup();
        for k in ks {
            let look = art::capture_look(&g, &rom, &original, l, k).map(|lk| (lk.w, lk.h, lk.px));
            if look.is_none() {
                continue; // markers and objects that draw nothing alone
            }
            if !seen.iter().any(|s| s.0 == second && s.1 == k && s.2 == look) {
                seen.push((second, k, look));
                jobs.push((l, k));
            }
        }
    }
    println!("{} objects to try", jobs.len());
    let jobs = std::sync::Arc::new(std::sync::Mutex::new(jobs));
    let (tx, rx) = std::sync::mpsc::channel();
    let (rom, original) = (std::sync::Arc::new(rom), std::sync::Arc::new(original));
    for _ in 0..12 {
        let (jobs, tx, rom, original, g) = (jobs.clone(), tx.clone(), rom.clone(), original.clone(), g.clone());
        std::thread::spawn(move || loop {
            let Some((l, k)) = jobs.lock().ok().and_then(|mut j| j.pop()) else { return };
            let into = if l >= 10 { into_first } else { into_second };
            if tx.send(mix_check(&g, &rom, &original, l, k, into)).is_err() {
                return;
            }
        });
    }
    drop(tx);
    let mut all: Vec<MixResult> = rx.into_iter().collect();
    all.sort_by_key(|r| (r.from >= 10, r.kind, r.from));
    for r in &all {
        println!("{} ${:02x} of level {:02} in level {:02}: picture {}, trace {} {} (dropped {:02x?})", if r.picture && r.trace { "OK  " } else { "FAIL" }, r.kind, r.from, r.into, r.picture, r.trace, r.note, r.dropped);
    }
    let ok = all.iter().filter(|r| r.picture && r.trace).count();
    println!("{ok} of {} work as in their own level", all.len());
}



/// The types that appear in object slots while one object of type `kind` plays on `level`'s
/// capture start (`mix_trace`'s setup, three ways of walking at it), besides `kind` itself.
fn spawned_by(map_state: &gb_core::Gb, rom: &[u8], original: &[u8], level: usize, kind: u8) -> std::collections::BTreeSet<u8> {
    use cart_model::{art, cheats, objects::Obj};
    let mut out = std::collections::BTreeSet::new();
    let Some(base) = art::capture_rom(rom, original, level, Vec::new()) else { return out };
    let mut probe = map_state.clone();
    probe.cart_mut().set_rom(&base);
    probe.enter_level(level as u8);
    let mx = probe.read(PLAYER_X) as u16 | (probe.read(PLAYER_X + 1) as u16) << 8;
    let my = probe.read(PLAYER_Y) as u16 | (probe.read(PLAYER_Y + 1) as u16) << 8;
    for (ahead, jump_every) in [(216u16, 45u32), (120, 30), (300, 60)] {
        let Some(r) = art::capture_rom(rom, original, level, vec![Obj { x: (mx + ahead) / 8 * 8, y: my / 8 * 8, kind, flag: false }]) else { continue };
        let mut g = map_state.clone();
        g.cart_mut().set_rom(&r);
        g.enter_level(level as u8);
        for f in 0..600 {
            g.run_frame_with(if f % jump_every < 12 { Buttons::RIGHT | Buttons::A } else { Buttons::RIGHT });
            g.clear_audio();
            for o in cheats::active_objects(&mut g) {
                if o.kind != kind {
                    out.insert(o.kind);
                }
            }
        }
    }
    out
}

/// For every object of the game's levels (one per catalogue, type and picture, as `mix_all`):
/// the other types it brings into play (defeated bodies, shots, pieces), on all cores.
pub fn spawns(rom_path: &Path) {
    use cart_model::{art, level_graphics, objects, patch};
    let Some(g) = crate::console::load(rom_path) else { return };
    let original = g.cart().rom().to_vec();
    let Ok(mut rom) = patch::expand(&original) else { return };
    patch::upgrade(&mut rom);
    let Ok(mut g) = gb_core::Gb::new(rom.clone(), 1000) else { return };
    g.boot_to_map();
    let mut jobs: Vec<(usize, u8)> = Vec::new();
    let mut seen: Vec<(bool, u8)> = Vec::new();
    for l in 0..32 {
        let second = level_graphics::object_group(&original, l).1;
        let mut ks: Vec<u8> = objects::read(&original, l).0.iter().map(|o| o.kind).collect();
        ks.sort_unstable();
        ks.dedup();
        for k in ks {
            if !seen.contains(&(second, k)) && art::capture_look(&g, &rom, &original, l, k).is_some() {
                seen.push((second, k));
                jobs.push((l, k));
            }
        }
    }
    let jobs = std::sync::Arc::new(std::sync::Mutex::new(jobs));
    let (tx, rx) = std::sync::mpsc::channel();
    let (rom, original) = (std::sync::Arc::new(rom), std::sync::Arc::new(original));
    for _ in 0..12 {
        let (jobs, tx, rom, original, g) = (jobs.clone(), tx.clone(), rom.clone(), original.clone(), g.clone());
        std::thread::spawn(move || loop {
            let Some((l, k)) = jobs.lock().ok().and_then(|mut j| j.pop()) else { return };
            if tx.send((l, k, spawned_by(&g, &rom, &original, l, k))).is_err() {
                return;
            }
        });
    }
    drop(tx);
    let mut all: Vec<_> = rx.into_iter().collect();
    all.sort_by_key(|r| (r.0 >= 10, r.1, r.0));
    for (l, k, s) in &all {
        if !s.is_empty() {
            println!("${k:02x} of level {l:02} ({}): {s:02x?}", if *l >= 10 { "10-31" } else { "0-9" });
        }
    }
}

/// Writes to the object slots' type bytes ($AD05 + $20 n): (pc, bank, slot, old, new).
#[derive(Clone, Default)]
pub struct TypeWrites {
    types: [u8; 16],
    pub log: Vec<(u16, usize, u8, u8, u8)>,
    /// Instructions that read $AF06 (pc, bank).
    pub flag_reads: std::collections::BTreeSet<(u16, usize)>,
}

impl gb_core::BusObserver for TypeWrites {
    const ENABLED: bool = true;

    fn on_read(&mut self, pc: u16, bank: usize, addr: u16) {
        if addr == 0xAF06 {
            self.flag_reads.insert((pc, if pc < 0x4000 { 0 } else { bank }));
        }
    }

    fn on_write(&mut self, pc: u16, bank: usize, addr: u16, value: u8) {
        if (0xAD00..0xAF00).contains(&addr) && (addr - 0xAD00) % 0x20 == 5 {
            let n = ((addr - 0xAD00) / 0x20) as usize;
            if self.types[n] != value && self.log.len() < 10_000 {
                self.log.push((pc, bank, n as u8, self.types[n], value));
            }
            self.types[n] = value;
        }
    }
}

/// Type `kind` of `level` on its capture start, Mario jumping at it at many moments: every
/// change of a slot's type away from `kind` (what it becomes, and the code that does it).
pub fn stomp(rom_path: &Path, level: usize, kind: u8) {
    use cart_model::{art, objects::Obj, patch};
    let Some(g) = crate::console::load(rom_path) else { return };
    let original = g.cart().rom().to_vec();
    let Ok(mut rom) = patch::expand(&original) else { return };
    patch::upgrade(&mut rom);
    let Some(base) = art::capture_rom(&rom, &original, level, Vec::new()) else { return };
    let Ok(mut probe) = gb_core::Gb::new(base, 1000) else { return };
    probe.boot_to_map();
    probe.enter_level(level as u8);
    let mx = probe.read(PLAYER_X) as u16 | (probe.read(PLAYER_X + 1) as u16) << 8;
    let my = probe.read(PLAYER_Y) as u16 | (probe.read(PLAYER_Y + 1) as u16) << 8;
    let mut found = std::collections::BTreeMap::new();
    let mut reads = std::collections::BTreeSet::new();
    for ahead in [216u16, 260] {
        let Some(r) = art::capture_rom(&rom, &original, level, vec![Obj { x: (mx + ahead) / 8 * 8, y: my / 8 * 8, kind, flag: false }]) else { continue };
        for at in (20..240).step_by(4) {
            let mut gm = match gb_core::GameBoy::with_observer(rom.clone(), 1000, TypeWrites::default()) {
                Ok(g) => g,
                Err(e) => return eprintln!("{e}"),
            };
            gm.boot_to_map();
            gm.cart_mut().set_rom(&r);
            if !gm.enter_level(level as u8) {
                eprintln!("level {level} not entered");
            }
            gm.observer_mut().log.clear();
            for f in 0..360 {
                let jump = f >= at && f < at + 14;
                gm.run_frame_with(if jump { Buttons::RIGHT | Buttons::A } else if f < at { Buttons::NONE } else { Buttons::RIGHT });
                gm.clear_audio();
            }
            for &(pc, bank, _, old, new) in &gm.observer().log {
                *found.entry((old, new, bank, pc)).or_insert(0) += 1;
            }
            reads.extend(gm.observer().flag_reads.iter().copied());
        }
    }
    for ((old, new, bank, pc), n) in found {
        println!("${old:02x} -> ${new:02x} by {bank:02x}:{pc:04x} ({n} times)");
    }
    println!("reads of $AF06: {:x?}", reads.iter().map(|(pc, b)| format!("{b:02x}:{pc:04x}")).collect::<Vec<_>>());
}



/// Type `kind` of `level` ahead of Mario, jumping at many moments: for the first run where Mario
/// touches it without being hurt (it ends close to him), its slot's 32 bytes frame by frame
/// around that moment, and every other slot that appears then.
pub fn slot_watch(rom_path: &Path, level: usize, kind: u8, rom_override: Option<&str>) {
    use cart_model::{art, objects::Obj, patch};
    let Some(g) = crate::console::load(rom_path) else { return };
    let original = g.cart().rom().to_vec();
    let Ok(mut rom) = patch::expand(&original) else { return };
    patch::upgrade(&mut rom);
    if let Some(p) = rom_override {
        let Ok(r) = std::fs::read(p) else { return };
        rom = r;
    }
    let Ok(mut map) = gb_core::Gb::new(rom.clone(), 1000) else { return };
    map.boot_to_map();
    let Some(base) = art::capture_rom(&rom, &original, level, Vec::new()) else { return };
    let mut probe = map.clone();
    probe.cart_mut().set_rom(&base);
    probe.enter_level(level as u8);
    let mx = probe.read(PLAYER_X) as u16 | (probe.read(PLAYER_X + 1) as u16) << 8;
    let my = probe.read(PLAYER_Y) as u16 | (probe.read(PLAYER_Y + 1) as u16) << 8;
    for ahead in [216u16, 260] {
        let Some(r) = art::capture_rom(&rom, &original, level, vec![Obj { x: (mx + ahead) / 8 * 8, y: my / 8 * 8, kind, flag: false }]) else { continue };
        for at in (20..240).step_by(2) {
            let mut gm = map.clone();
            gm.cart_mut().set_rom(&r);
            gm.enter_level(level as u8);
            let mut frames: Vec<(u32, Vec<[u8; 32]>, u8)> = Vec::new();
            for f in 0..360 {
                gm.run_frame_with(if f >= at && f < at + 14 { Buttons::RIGHT | Buttons::A } else if f < at { Buttons::NONE } else { Buttons::RIGHT });
                gm.clear_audio();
                let slots = (0..16u16).map(|n| std::array::from_fn(|i| gm.read(0xAD00 + n * 0x20 + i as u16))).collect();
                frames.push((f, slots, gm.read(GAME_MODE)));
            }
            // the frame where a used slot of `kind` changes its byte $0F or more than its place
            let hurt = frames.iter().any(|f| f.2 == 0x09);
            let change = frames.windows(2).find_map(|w| (0..16).find(|&n| {
                let (a, b) = (&w[0].1[n], &w[1].1[n]);
                a[8] != 0 && a[5] == kind && (b[5] != kind || b[8] == 0 || a[9..] != b[9..] && a[0x0F] != b[0x0F])
            }).map(|n| (w[0].0, n)));
            let Some((f, n)) = change else { continue };
            if hurt {
                continue;
            }
            println!("ahead {ahead} jump {at}: slot {n} changes at frame {f}");
            for (ff, slots, mode) in frames.iter().filter(|x| x.0 + 3 >= f && x.0 <= f + 12) {
                let used: Vec<String> = slots.iter().enumerate().filter(|(_, s)| s[8] != 0).map(|(i, s)| format!("{i}:{:02x}", s[5])).collect();
                println!("f{ff} mode {mode:02x} slot {n}: {:02x?} | used {used:?}", &slots[n][..20]);
            }
            return;
        }
    }
    println!("no run where Mario touches it unhurt");
}




/// Memory as written, plus: slot type and used-flag writes, and at each read of $AF06 at the
/// four sites what $FFD5, $FFDB and $AF28/$AF29 hold.
#[derive(Clone)]
pub struct KillWatch {
    mem: Vec<u8>,
    pub frame: u32,
    pub events: Vec<String>,
}

impl gb_core::BusObserver for KillWatch {
    const ENABLED: bool = true;

    fn on_read(&mut self, pc: u16, _bank: usize, addr: u16) {
        if addr == 0xAF06 && CATALOGUE_SITES.contains(&pc) && self.events.len() < 200_000 {
            let m = &self.mem;
            let slot = ((m[0xAF28] as u16) << 8 | (m[0xAF29] & 0xF0) as u16).wrapping_sub(0xAD00) / 0x20;
            self.events.push(format!("f{} read at {pc:04x}: $FFD5 {:02x} $FFDB {:02x} slot {slot}", self.frame, m[0xFFD5], m[0xFFDB]));
        }
    }

    fn on_write(&mut self, pc: u16, bank: usize, addr: u16, value: u8) {
        if (0xAD00..0xAF00).contains(&addr) && matches!((addr - 0xAD00) % 0x20, 5 | 8) && self.mem[addr as usize] != value && self.events.len() < 200_000 {
            self.events.push(format!("f{} {bank:02x}:{pc:04x} slot {} byte {} {:02x} -> {value:02x}", self.frame, (addr - 0xAD00) / 0x20, (addr - 0xAD00) % 0x20, self.mem[addr as usize]));
        }
        self.mem[addr as usize] = value;
    }
}

/// `stomp`'s runs (type `kind` of `level`, in `rom_override` when given, else the expanded
/// copy): the first where a slot of `kind` becomes type $00 while Mario is not hurt, with
/// every event `KillWatch` saw from 4 frames before to 30 after.
pub fn kill_watch(rom_path: &Path, level: usize, kind: u8, foreign_mark: u8) {
    use cart_model::{art, objects::Obj, patch};
    let Some(g) = crate::console::load(rom_path) else { return };
    let original = g.cart().rom().to_vec();
    let Ok(mut rom) = patch::expand(&original) else { return };
    patch::upgrade(&mut rom);
    let Some(base) = art::capture_rom(&rom, &original, level, Vec::new()) else { return };
    let Ok(mut probe) = gb_core::Gb::new(rom.clone(), 1000) else { return };
    probe.boot_to_map();
    probe.cart_mut().set_rom(&base);
    probe.enter_level(level as u8);
    let mx = probe.read(PLAYER_X) as u16 | (probe.read(PLAYER_X + 1) as u16) << 8;
    let my = probe.read(PLAYER_Y) as u16 | (probe.read(PLAYER_Y + 1) as u16) << 8;
    let _ = foreign_mark;
    for ahead in [216u16, 260] {
        let Some(r) = art::capture_rom(&rom, &original, level, vec![Obj { x: (mx + ahead) / 8 * 8, y: my / 8 * 8, kind, flag: false }]) else { continue };
        for at in (20..240).step_by(4) {
            let Ok(mut gm) = gb_core::GameBoy::with_observer(rom.clone(), 1000, KillWatch { mem: vec![0; 0x10000], frame: 0, events: Vec::new() }) else { return };
            gm.boot_to_map();
            gm.cart_mut().set_rom(&r);
            gm.enter_level(level as u8);
            gm.observer_mut().events.clear();
            let mut hurt = false;
            for f in 0..360 {
                gm.observer_mut().frame = f;
                gm.run_frame_with(if f >= at && f < at + 14 { Buttons::RIGHT | Buttons::A } else if f < at { Buttons::NONE } else { Buttons::RIGHT });
                gm.clear_audio();
                hurt |= gm.read(GAME_MODE) != 0x04;
            }
            let ev = &gm.observer().events;
            let killed = ev.iter().find(|e| e.contains(&format!("byte 5 {kind:02x} -> 00")));
            let Some(k) = killed else { continue };
            let f: u32 = k[1..k.find(' ').unwrap_or(1)].parse().unwrap_or(0);
            println!("ahead {ahead} jump {at}: {k} (Mario hurt or dead in the run: {hurt})");
            for e in ev.iter().filter(|e| e[1..e.find(' ').unwrap_or(1)].parse::<u32>().is_ok_and(|x| x + 4 >= f && x <= f + 30)) {
                println!("  {e}");
            }
            return;
        }
    }
    println!("no run turns it into type $00");
}




/// Type `kind` of `level` on its capture start (walking right, jumping now and then): every
/// write that gives a slot a type other than `kind`, with the code, the slot $AF28/$AF29 points
/// at then (the object being handled) and that slot's type.
pub fn spawn_code(rom_path: &Path, level: usize, kind: u8) {
    use cart_model::{art, objects::Obj, patch};
    let Some(g) = crate::console::load(rom_path) else { return };
    let original = g.cart().rom().to_vec();
    let Ok(mut rom) = patch::expand(&original) else { return };
    patch::upgrade(&mut rom);
    let Some(base) = art::capture_rom(&rom, &original, level, Vec::new()) else { return };
    let Ok(mut probe) = gb_core::Gb::new(rom.clone(), 1000) else { return };
    probe.boot_to_map();
    probe.cart_mut().set_rom(&base);
    probe.enter_level(level as u8);
    let mx = probe.read(PLAYER_X) as u16 | (probe.read(PLAYER_X + 1) as u16) << 8;
    let my = probe.read(PLAYER_Y) as u16 | (probe.read(PLAYER_Y + 1) as u16) << 8;
    let Some(r) = art::capture_rom(&rom, &original, level, vec![Obj { x: (mx + 216) / 8 * 8, y: my / 8 * 8, kind, flag: false }]) else { return };
    let Ok(mut gm) = gb_core::GameBoy::with_observer(rom.clone(), 1000, SpawnWatch { mem: vec![0; 0x10000], log: Vec::new() }) else { return };
    gm.boot_to_map();
    gm.cart_mut().set_rom(&r);
    gm.enter_level(level as u8);
    gm.observer_mut().log.clear();
    for f in 0..600 {
        gm.run_frame_with(if f % 45 < 12 { Buttons::RIGHT | Buttons::A } else { Buttons::RIGHT });
        gm.clear_audio();
    }
    let mut seen = std::collections::BTreeSet::new();
    for l in &gm.observer().log {
        if seen.insert(l.clone()) {
            println!("{l}");
        }
    }
}

/// Writes of a slot's type byte ($AD05 + $20 n) to a nonzero value from 0: the code, the slot
/// written, and the slot $AF28/$AF29 points at with its type.
#[derive(Clone)]
pub struct SpawnWatch {
    mem: Vec<u8>,
    pub log: Vec<String>,
}

impl gb_core::BusObserver for SpawnWatch {
    const ENABLED: bool = true;

    fn on_write(&mut self, pc: u16, bank: usize, addr: u16, value: u8) {
        if (0xAD00..0xAF00).contains(&addr) && (addr - 0xAD00) % 0x20 == 5 && value != 0 && self.mem[addr as usize] == 0 && self.log.len() < 10_000 {
            let m = &self.mem;
            let cur = (m[0xAF28] as u16) << 8 | (m[0xAF29] & 0xF0) as u16;
            let cur_kind = if (0xAD00..0xAF00).contains(&cur) { m[cur as usize + 5] } else { 0xFF };
            self.log.push(format!("{bank:02x}:{pc:04x} gives slot {:04x} type {value:02x}; handling slot {cur:04x} (type {cur_kind:02x}), $FFD5 {:02x}", addr - 5, m[0xFFD5]));
        }
        self.mem[addr as usize] = value;
    }
}




const TILE_RUNS: [(u32, u32); 5] = [(1000, 0), (45, 12), (30, 8), (60, 20), (24, 4)];

/// The sprite tiles shown while Mario plays `rom` from `entered` with jumps every `every`
/// frames held `hold` (none when `every` is 1000) for `frames` frames (until he dies).
fn shown_tiles(entered: &gb_core::Gb, every: u32, hold: u32, frames: u32) -> std::collections::BTreeSet<u8> {
    let mut g = entered.clone();
    let mut out = std::collections::BTreeSet::new();
    for f in 0..frames {
        g.run_frame_with(if every < 1000 && f % every < hold { Buttons::RIGHT | Buttons::A } else { Buttons::RIGHT });
        g.clear_audio();
        if g.read(GAME_MODE) != 0x04 {
            break;
        }
        let tall = g.video().lcdc() & 0x04 != 0;
        let oam = g.video().oam().to_vec();
        for s in oam.chunks(4) {
            if s[0] == 0 || s[0] >= 160 || s[1] == 0 || s[1] >= 168 {
                continue;
            }
            let t = if tall { s[2] & 0xFE } else { s[2] };
            out.insert(t);
            if tall {
                out.insert(t | 1);
            }
        }
    }
    out
}

/// Every object of the game's levels (as `mix_all`): the tiles it shows played many ways in its
/// own level, Mario invincible (Mario's own tiles, from the same runs without it, left out) against the tiles
/// captured (`art::object_tiles`); then, for each level of its catalogue it does not draw right,
/// whether its tiles outside the enemy area are in sets both levels load the same.
pub fn full_tiles(rom_path: &Path) {
    use cart_model::{art, enemy_sets, level_graphics, objects, patch};
    let Some(g) = crate::console::load(rom_path) else { return };
    let original = g.cart().rom().to_vec();
    let Ok(mut rom) = patch::expand(&original) else { return };
    patch::upgrade(&mut rom);
    let Ok(mut g) = gb_core::Gb::new(rom.clone(), 1000) else { return };
    g.boot_to_map();
    let mut jobs: Vec<(usize, u8)> = Vec::new();
    let mut seen: Vec<(bool, u8, Option<(usize, usize, Vec<Option<u8>>)>)> = Vec::new();
    for l in 0..32 {
        let second = level_graphics::object_group(&original, l).1;
        let mut ks: Vec<u8> = objects::read(&original, l).0.iter().map(|o| o.kind).collect();
        ks.sort_unstable();
        ks.dedup();
        for k in ks {
            let look = art::capture_look(&g, &rom, &original, l, k).map(|lk| (lk.w, lk.h, lk.px));
            if look.is_some() && !seen.iter().any(|s| s.0 == second && s.1 == k && s.2 == look) {
                seen.push((second, k, look));
                jobs.push((l, k));
            }
        }
    }
    println!("{} objects", jobs.len());
    let jobs = std::sync::Arc::new(std::sync::Mutex::new(jobs));
    let (tx, rx) = std::sync::mpsc::channel();
    let (rom, original) = (std::sync::Arc::new(rom), std::sync::Arc::new(original));
    for _ in 0..12 {
        let (jobs, tx, rom, original, g) = (jobs.clone(), tx.clone(), rom.clone(), original.clone(), g.clone());
        std::thread::spawn(move || loop {
            use cart_model::objects::Obj;
            let Some((l, k)) = jobs.lock().ok().and_then(|mut j| j.pop()) else { return };
            let captured = art::object_tiles(&g, &rom, &original, l, k, 600);
            let mut safe = rom.to_vec();
            cart_model::cheats::set_invincible(&mut safe, true);
            let Some(base) = art::capture_rom(&safe, &original, l, Vec::new()) else { continue };
            let mut probe = g.clone();
            probe.cart_mut().set_rom(&base);
            probe.enter_level(l as u8);
            let mx = probe.read(PLAYER_X) as u16 | (probe.read(PLAYER_X + 1) as u16) << 8;
            let my = probe.read(PLAYER_Y) as u16 | (probe.read(PLAYER_Y + 1) as u16) << 8;
            let mut played = std::collections::BTreeSet::new();
            for ahead in [120u16, 216] {
                let Some(r) = art::capture_rom(&safe, &original, l, vec![Obj { x: (mx + ahead) / 8 * 8, y: my / 8 * 8, kind: k, flag: false }]) else { continue };
                for (every, hold) in TILE_RUNS {
                    let mut empty = g.clone();
                    empty.cart_mut().set_rom(&base);
                    empty.enter_level(l as u8);
                    let mario = shown_tiles(&empty, every, hold, 900);
                    let mut with = g.clone();
                    with.cart_mut().set_rom(&r);
                    with.enter_level(l as u8);
                    played.extend(shown_tiles(&with, every, hold, 900).difference(&mario).copied());
                }
            }
            played.extend(captured.iter().copied());
            let missed: Vec<u8> = played.iter().copied().filter(|t| !captured.contains(t)).collect();
            let outside = played.iter().filter(|t| !enemy_sets::AREA.contains(t)).fold(0u8, |m, &t| m | level_graphics::set_of(t));
            // the levels of its catalogue that do not use it: whether its outside tiles agree
            let second = l >= 10;
            let (mut kept, mut lost) = (Vec::new(), Vec::new());
            for to in (0..32usize).filter(|&to| (to >= 10) == second && !objects::read(&original, to).0.iter().any(|o| o.kind == k)) {
                let agrees = level_graphics::same_sets(&original, to, l, outside);
                let was = level_graphics::same_sets(&original, to, l, captured.iter().filter(|t| !enemy_sets::AREA.contains(t)).fold(0u8, |m, &t| m | level_graphics::set_of(t)));
                if agrees {
                    kept.push(to);
                } else if was {
                    lost.push(to);
                }
            }
            let _ = tx.send((l, k, captured, missed, outside, kept, lost));
        });
    }
    drop(tx);
    let mut all: Vec<_> = rx.into_iter().collect();
    all.sort_by_key(|r| (r.0 >= 10, r.1, r.0));
    let (mut with_missed, mut missed_outside) = (0, 0);
    for (l, k, captured, missed, outside, kept, lost) in &all {
        let out_missed = missed.iter().filter(|t| !enemy_sets::AREA.contains(t)).count();
        with_missed += !missed.is_empty() as u32;
        missed_outside += (out_missed > 0) as u32;
        println!("${k:02x} of level {l:02}: captured {} tiles, missed {missed:02x?} ({out_missed} outside the enemy area); outside sets {outside:03b}; fine in {} levels{}", captured.len(), kept.len(), if lost.is_empty() { String::new() } else { format!(", now refused in {lost:02?}") });
    }
    println!("{} objects; {with_missed} show tiles the capture missed, {missed_outside} of them outside the enemy area", all.len());
}

/// Which OAM entries hold sprites while Mario plays level `level`'s capture start with nothing
/// else (walking, jumping, then with each power-up), and their tiles.
pub fn mario_oam(rom_path: &Path, level: usize) {
    use cart_model::{art, cheats, patch};
    let Some(g) = crate::console::load(rom_path) else { return };
    let original = g.cart().rom().to_vec();
    let Ok(mut rom) = patch::expand(&original) else { return };
    patch::upgrade(&mut rom);
    let Ok(mut g) = gb_core::Gb::new(rom.clone(), 1000) else { return };
    g.boot_to_map();
    let Some(base) = art::capture_rom(&rom, &original, level, Vec::new()) else { return };
    g.cart_mut().set_rom(&base);
    g.enter_level(level as u8);
    for (n, item) in [None, Some(cheats::Item::Mushroom), Some(cheats::Item::Carrot)].into_iter().enumerate() {
        if let Some(i) = item {
            cheats::summon(&mut g, i);
        }
        let mut used: std::collections::BTreeMap<usize, std::collections::BTreeSet<u8>> = Default::default();
        for f in 0..300 {
            g.run_frame_with(if f % 40 < 12 { Buttons::RIGHT | Buttons::A } else { Buttons::RIGHT });
            g.clear_audio();
            let oam = g.video().oam().to_vec();
            for (i, s) in oam.chunks(4).enumerate() {
                if s[0] != 0 && s[0] < 160 {
                    used.entry(i).or_default().insert(s[2]);
                }
            }
        }
        println!("run {n}: {:?}", used.iter().map(|(i, t)| format!("{i}:{t:02x?}")).collect::<Vec<_>>());
    }
}

/// Add type `kind` of game level `from` to level `level`'s enemy set in the edited copy `copy`
/// (with the members it has, or the level's own enemies), as the editor's catalogue does.
pub fn add_member(rom_path: &Path, copy: &str, level: usize, from: usize, kind: u8) {
    use cart_model::enemy_sets::{self, Enemy};
    use cart_model::{art, extra_levels, objects};
    let Some(g) = crate::console::load(rom_path) else { return };
    let original = g.cart().rom().to_vec();
    let Ok(mut rom) = std::fs::read(copy) else { return };
    let Ok(mut map) = gb_core::Gb::new(rom.clone(), 1000) else { return };
    map.boot_to_map();
    let base = if level < 32 { level } else { extra_levels::read(&rom)[level - 32].base as usize };
    let sets = enemy_sets::read(&rom);
    let mut members: Vec<Enemy> = sets.iter().find(|s| s.level == level).map(|s| s.enemies.clone()).unwrap_or_else(|| {
        let mut ks: Vec<u8> = objects::read(&original, base).0.iter().map(|o| o.kind).collect();
        ks.sort_unstable();
        ks.dedup();
        ks.into_iter().map(|k| Enemy { level: base as u8, kind: k }).collect()
    });
    members.push(Enemy { level: from as u8, kind });
    let with: Vec<(Enemy, Vec<u8>)> = members.iter().map(|e| (*e, art::object_tiles(&map, &rom, &original, e.level as usize, e.kind, 600))).filter(|m| !enemy_sets::area_tiles(&m.1).is_empty()).collect();
    let block = enemy_sets::build(&original, base, &with);
    let mut all: Vec<(usize, Vec<Enemy>, Vec<u8>)> = sets.iter().filter(|s| s.level != level).map(|s| {
        let at = cart_model::rom::offset(s.block.0 as usize, s.block.1);
        (s.level, s.enemies.clone(), rom[at..at + enemy_sets::BLOCK].to_vec())
    }).collect();
    all.push((level, with.iter().map(|m| m.0).collect(), block));
    enemy_sets::store(&mut rom, &original, &all).unwrap();
    let _ = std::fs::write(copy, &rom);
    println!("level {level}'s set: {:?}", with.iter().map(|m| m.0).collect::<Vec<_>>());
}
