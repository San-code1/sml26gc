//! A level's start and ways out, tried in the emulator: where Mario begins (header $00-$07),
//! the goal, the secret exit and the bonus bell.
use crate::console::load;
use cart_model::{levels, GameDriver};
use gb_core::{Buttons, Gb};
use std::path::Path;

fn word(g: &mut Gb, a: u16) -> u16 {
    g.read(a) as u16 | (g.read(a + 1) as u16) << 8
}

/// Mario's y and x ($FFC0, $FFC2), the camera's y and x ($FFC8, $FFCA), SCY and SCX.
fn where_now(g: &mut Gb) -> [u16; 6] {
    [word(g, 0xFFC0), word(g, 0xFFC2), word(g, 0xFFC8), word(g, 0xFFCA), g.read(0xFF42) as u16, g.read(0xFF43) as u16]
}

/// The sprites on the screen: (y, x, tile, attributes).
fn sprites(g: &Gb) -> Vec<[u8; 4]> {
    g.video().oam().chunks(4).filter(|s| s[0] != 0 && s[0] < 160).map(|s| [s[0], s[1], s[2], s[3]]).collect()
}

/// Every level's header start, where Mario and the camera are once it plays, and the
/// sprites then.
pub fn start_probe(rom_path: &Path) {
    let Some(mut g) = load(rom_path) else { return };
    g.boot_to_map();
    for l in 0..levels::LEVELS as u8 {
        let at = levels::header(g.cart().rom(), l as usize).at;
        let h: Vec<u8> = g.cart().rom()[at..at + 8].to_vec();
        let mut t = g.clone();
        if !t.enter_level_anywhere(l) {
            println!("level {l:2}: not entered");
            continue;
        }
        let first = where_now(&mut t);
        for _ in 0..20 {
            t.run_frame_with(Buttons::NONE);
        }
        let later = where_now(&mut t);
        println!("level {l:2}: header {h:02x?} at once {first:04x?} 20 frames {later:04x?}");
        println!("          sprites {:02x?}", sprites(&t));
    }
}

/// Levels entered from moved starts (header bytes $00-$07 written as the editor will):
/// Mario where the start says, on the screen at the place the camera puts him, and still
/// playing a second later.
pub fn start_check(rom_path: &Path) {
    use cart_model::extra_levels::{apply, based_on, store, writes_for};
    use cart_model::level_start::{Start, SPRITE_TOP};
    let Some(g0) = load(rom_path) else { return };
    // the copy as the editor saves it: the 32 levels' starts in their headers, a new level's
    // in its own header
    let Ok(mut rom) = cart_model::patch::expand(g0.cart().rom()) else { return };
    let cases = [(0u8, 0x400u16, 0x1F0u16, false), (3, 0x800, 0x1F0, false), (14, 0x200, 0x260, false), (24, 0x500, 0x300, false), (10, 0xF00, 0x300, false), (0, 0xF60, 0x1F0, false), (14, 0x900, 0x40, false), (12, 0x600, 0x1F0, true)];
    let mut extra = based_on(&rom, 12);
    for &(l, x, feet, new) in &cases {
        let s = Start::standing(x, feet);
        if new {
            s.write(&mut extra.header);
        } else {
            let at = levels::header(&rom, l as usize).at;
            s.write(&mut rom[at..at + 8]);
        }
    }
    if store(&mut rom, std::slice::from_ref(&extra)).is_err() {
        println!("startcheck FAILED: the new level does not fit");
        return;
    }
    let Ok(mut g) = Gb::new(rom.clone(), 44100) else { return };
    g.boot_to_map();
    let mut ok = true;
    let mut seen = std::collections::HashSet::new();
    for &(l, x, feet, new) in &cases {
        if !seen.insert((l, new)) {
            continue; // one start per level in the copy: the last one written
        }
        let s = Start::standing(x, feet);
        let s = if new { s } else { Start::read(&rom[levels::header(&rom, l as usize).at..]) };
        let mut t = g.clone();
        if new {
            apply(t.cart_mut().rom_mut(), &writes_for(&rom, &extra));
        }
        if !t.enter_level_anywhere(l) {
            println!("level {l:2}: FAIL not entered");
            ok = false;
            continue;
        }
        let now = where_now(&mut t);
        let screen = (s.mario_x as i32 - (s.camera_x as i32 - 0x50), (s.mario_y + SPRITE_TOP) as i32 - (s.camera_y as i32 - 0x48));
        let left_top = sprites(&t).iter().map(|sp| (sp[1] as i32 - 8, sp[0] as i32 - 16)).min();
        // the game has run a few frames since: Mario may have fallen a pixel or been pushed
        let near = |a: i32, b: i32| (a - b).abs() <= 4;
        let placed = now[..4].iter().zip([s.mario_y, s.mario_x, s.camera_y, s.camera_x]).all(|(&a, b)| near(a as i32, b as i32));
        let shown = left_top.is_some_and(|(x, y)| near(x, screen.0) && near(y, screen.1)) && (0..160).contains(&screen.0) && (0..144).contains(&screen.1);
        for _ in 0..60 {
            t.run_frame_with(Buttons::NONE);
        }
        let playing = t.mode() == cart_model::GameMode::Playing;
        let fine = placed && shown && playing;
        ok &= fine;
        println!("level {l:2}{}: {} start {s:04x?} now {now:04x?} sprite {left_top:?} expected {screen:?} playing {playing} y after 1 s {:04x}", if new { " (new)" } else { "" }, if fine { "ok" } else { "FAIL" }, word(&mut t, 0xFFC0));
    }
    println!("{}", if ok { "startcheck ok" } else { "startcheck FAILED" });
}

/// Start level `level` at level pixel (x, feet), hold Right (and A now and then when
/// `jump`) and print every change of the game mode with Mario's place, plus the cartridge RAM
/// bytes that changed by the time the mode first changed.
pub fn goal_probe(rom_path: &Path, level: u8, x: u16, feet: u16, how: &str) {
    use cart_model::level_start::Start;
    let Some(mut g) = load(rom_path) else { return };
    g.boot_to_map();
    let s = Start::standing(x, feet);
    let at = levels::header(g.cart().rom(), level as usize).at;
    s.write(&mut g.cart_mut().rom_mut()[at..at + 8]);
    if !g.enter_level_anywhere(level) {
        println!("not entered");
        return;
    }
    if how == "move" {
        // the decoded map is at $B000, 256 blocks a row: the goal block moved to column 6
        let goal = 0xB000 + 30 * 256;
        g.write(goal + 254, 0x60);
        g.write(goal + 6, 0x4A);
    }
    let before: Vec<u8> = (0xA000..0xA900u16).map(|a| g.read(a)).collect();
    let mut mode = g.read(0xFF9B);
    let mut diffed = false;
    for f in 0..1500u32 {
        let b = match how {
            "jump" if f % 40 < 12 => Buttons::RIGHT | Buttons::A,
            "up" if f < 60 => if f < 30 { Buttons::A } else { Buttons::NONE },
            "runup" if f < 60 => if f < 30 { Buttons::A | Buttons::B } else { Buttons::NONE },
            _ if g.read(0xFF9B) == 0x04 => Buttons::RIGHT,
            _ => Buttons::NONE,
        };
        g.run_frame_with(b);
        g.clear_audio();
        let m = g.read(0xFF9B);
        if how == "a266" && m == 0x18 && mode != 0x18 {
            g.write(0xA266, 0);
        }
        if m != mode {
            let w = where_now(&mut g);
            println!("frame {f}: mode {mode:02x} -> {m:02x}, mario y {:04x} x {:04x}", w[0], w[1]);
            if !diffed {
                diffed = true;
                let after: Vec<u8> = (0xA000..0xA900u16).map(|a| g.read(a)).collect();
                let d: Vec<String> = before.iter().zip(&after).enumerate().filter(|(_, (a, b))| a != b).map(|(i, (a, b))| format!("{:04x}:{a:02x}>{b:02x}", 0xA000 + i)).collect();
                println!("  changed: {}", d.join(" "));
            }
            mode = m;
        }
        if f % 100 == 0 {
            let w = where_now(&mut g);
            println!("  f{f} y {:04x} x {:04x}", w[0], w[1]);
        }
    }
}

/// Where block `b` is in level `level`'s map (column, row), the first one.
fn block_at(rom: &[u8], level: usize, b: u8) -> Option<(u16, u16)> {
    let (_, start) = levels::map_location(rom, levels::header(rom, level));
    let map = levels::decode(rom, start).0;
    map.iter().position(|&v| v == b).map(|i| ((i % levels::COLS) as u16, (i / levels::COLS) as u16))
}

/// From point `point`'s map, play level `level` starting right on its block `b` (the goal
/// $4A or the secret exit $4B), hold Right, and come back to the map. Returns the game then
/// (or None when it did not get back).
fn through_block(g: &Gb, point: u8, level: u8, b: u8) -> Option<Gb> {
    use cart_model::level_start::Start;
    let mut t = g.clone();
    let (c, r) = block_at(t.cart().rom(), level as usize, b)?;
    let s = Start::standing(c * 16 - 4, (r + 1) * 16);
    let at = levels::header(t.cart().rom(), level as usize).at;
    s.write(&mut t.cart_mut().rom_mut()[at..at + 8]);
    if !t.show_map_at(point) || !t.enter_level(level) {
        return None;
    }
    let mut on_map = 0;
    for f in 0..3000u32 {
        let mode = t.read(0xFF9B);
        let b = if mode == 0x04 { Buttons::RIGHT } else if f % 120 < 3 && mode != 0x0C { Buttons::A } else { Buttons::NONE };
        t.run_frame_with(b);
        t.clear_audio();
        on_map = if t.read(0xFF9B) == 0x0C { on_map + 1 } else { 0 };
        if on_map > 240 {
            let rom = g.cart().rom().to_vec();
            t.cart_mut().rom_mut().copy_from_slice(&rom);
            return Some(t);
        }
    }
    None
}

/// The game's secret exits: each level with a $4B block, played from its own point through
/// its goal and through its secret exit; where Mario is after each and which cleared flags
/// changed.
pub fn secret_probe(rom_path: &Path) {
    let Some(mut g) = load(rom_path) else { return };
    g.boot_to_map();
    let table = cart_model::world::point_levels(g.cart().rom());
    let flags = |t: &mut Gb| (0xA848..0xA880u16).map(|a| t.read(a)).collect::<Vec<u8>>();
    let before = flags(&mut g);
    for l in 0..32u8 {
        if block_at(g.cart().rom(), l as usize, 0x4B).is_none() {
            continue;
        }
        for &p in table.iter().enumerate().filter(|(_, &v)| v == l).map(|(p, _)| p as u8).collect::<Vec<_>>().iter() {
            for b in [0x4A, 0x4B] {
                match through_block(&g, p, l, b) {
                    Some(mut t) => {
                        let after = flags(&mut t);
                        let d: Vec<String> = before.iter().zip(&after).enumerate().filter(|(_, (a, b))| a != b).map(|(i, (a, b))| format!("{:04x}:{a:02x}>{b:02x}", 0xA848 + i)).collect();
                        println!("level {l:2} point {p:02x} via {b:02x}: now on point {:02x} zone {:02x}, flags {}", t.read(0xA840), t.read(0xA68B), d.join(" "));
                    }
                    None => println!("level {l:2} point {p:02x} via {b:02x}: did not get back to the map"),
                }
            }
        }
    }
}

/// Levels cleared through moved finishes (as the editor saves them: the level's map in its
/// bank, a new level's in its own room): from the level's point, Mario starts four blocks
/// left of the finish and holds Right; the level ends, Mario is back on the map and a
/// cleared flag is set.
pub fn finish_check(rom_path: &Path) {
    use cart_model::extra_levels::{apply, based_on, store, writes_for};
    use cart_model::level_goals::{self, Goal};
    use cart_model::level_start::Start;
    let Some(g0) = load(rom_path) else { return };
    let original = g0.cart().rom().to_vec();
    let Ok(mut rom) = cart_model::patch::expand(&original) else { return };
    // the finish four blocks right of Mario's start, at his height
    let near_start = |header: &[u8]| {
        let s = Start::read(header);
        ((s.mario_x / 16 + 4) as usize, ((s.mario_y + cart_model::level_start::FEET) / 16 - 1) as usize)
    };
    let mut ok = true;
    let mut cases: Vec<(u8, bool)> = Vec::new();
    for l in [1u8, 25] {
        let h = levels::header(&rom, l as usize);
        let mut map = levels::decode(&rom, levels::map_location(&rom, h).1).0;
        let to = near_start(&rom[h.at..h.at + 8]);
        level_goals::place(&mut map, &mut [], Goal::Finish, to, &mut Vec::new());
        match cart_model::layout::store_bank(&mut rom, &original, h.bank, &[(h.map, map.as_slice())]) {
            Ok(_) => cases.push((l, false)),
            Err(e) => {
                println!("level {l:2}: FAIL not stored ({e:?})");
                ok = false;
            }
        }
    }
    let mut extra = based_on(&rom, 12);
    let to = near_start(&extra.header);
    level_goals::place(&mut extra.map, &mut extra.objs, Goal::Finish, to, &mut Vec::new());
    if store(&mut rom, std::slice::from_ref(&extra)).is_err() {
        println!("finishcheck FAILED: the new level does not fit");
        return;
    }
    cases.push((12, true));
    let Ok(mut g) = Gb::new(rom.clone(), 44100) else { return };
    g.boot_to_map();
    let table = cart_model::world::point_levels(&rom);
    let flags = |t: &mut Gb| (0xA848..0xA880u16).map(|a| t.read(a)).collect::<Vec<u8>>();
    for (l, new) in cases {
        let Some(p) = table.iter().position(|&v| v == l).map(|p| p as u8) else {
            println!("level {l:2}: no point opens it");
            continue;
        };
        let mut t = g.clone();
        if new {
            apply(t.cart_mut().rom_mut(), &writes_for(&rom, &extra));
        }
        let before = flags(&mut t);
        if !t.show_map_at(p) || !t.enter_level(l) {
            println!("level {l:2}: FAIL not entered");
            ok = false;
            continue;
        }
        let (mut ended, mut on_map) = (false, 0);
        for _ in 0..3000u32 {
            let mode = t.read(0xFF9B);
            ended |= mode == 0x18;
            t.run_frame_with(if mode == 0x04 { Buttons::RIGHT } else { Buttons::NONE });
            t.clear_audio();
            on_map = if t.read(0xFF9B) == 0x0C { on_map + 1 } else { 0 };
            if on_map > 240 {
                break;
            }
        }
        let cleared = flags(&mut t) != before;
        let fine = ended && on_map > 240 && cleared;
        ok &= fine;
        println!("level {l:2}{} from point {p:02x}: {} goal touched {ended}, back on the map {}, cleared {cleared}", if new { " (new)" } else { "" }, if fine { "ok" } else { "FAIL" }, on_map > 240);
    }
    println!("{}", if ok { "finishcheck ok" } else { "finishcheck FAILED" });
}

/// Secret exits sent elsewhere (`secret_exits`): each case plays its level from its point
/// through its $4B block (one placed by the check in level 1) and finds Mario on the target,
/// the level's point not cleared, and staying there (the game walks Mario on after its own
/// levels in a zone); level 8 keeps the game's own way.
pub fn secret_check(rom_path: &Path) {
    use cart_model::level_goals::{self, Goal};
    use cart_model::secret_exits::{self, SecretTo};
    let Some(g0) = load(rom_path) else { return };
    let original = g0.cart().rom().to_vec();
    let Ok(mut rom) = cart_model::patch::expand(&original) else { return };
    // level 1 gets a secret exit four blocks right of its start
    let h = levels::header(&rom, 1);
    let mut map = levels::decode(&rom, levels::map_location(&rom, h).1).0;
    let s = cart_model::level_start::Start::read(&rom[h.at..h.at + 8]);
    let to = ((s.mario_x / 16 + 4) as usize, ((s.mario_y + cart_model::level_start::FEET) / 16 - 1) as usize);
    level_goals::place(&mut map, &mut [], Goal::Secret, to, &mut Vec::new());
    if cart_model::layout::store_bank(&mut rom, &original, h.bank, &[(h.map, map.as_slice())]).is_err() {
        println!("secretcheck FAILED: level 1 not stored");
        return;
    }
    let targets = [(7, SecretTo::Point(0x07)), (17, SecretTo::Point(0x34)), (1, SecretTo::Point(0x0A))];
    if let Err(e) = secret_exits::write(&mut rom, &targets, &[]) {
        println!("secretcheck FAILED: {e}");
        return;
    }
    println!("read back the same: {}", secret_exits::read(&rom) == targets);
    let Ok(mut g) = Gb::new(rom.clone(), 44100) else { return };
    g.boot_to_map();
    let flags = |t: &mut Gb| (0xA848..0xA880u16).map(|a| t.read(a)).collect::<Vec<u8>>();
    let before = flags(&mut g);
    let mut ok = true;
    for (l, p, want) in [(7u8, 0x29u8, 0x07u8), (17, 0x05, 0x34), (17, 0x1A, 0x34), (1, 0x23, 0x0A), (8, 0x2A, 0x7A)] {
        match through_block(&g, p, l, 0x4B) {
            Some(mut t) => {
                let after = flags(&mut t);
                let flag = cart_model::rom::offset(0x0F, 0x650E) + p as usize;
                let own = 0xA848 - 0xA848 + rom[flag] as usize;
                let cleared = after.get(own).is_some_and(|&f| f & 0x80 != 0) && before.get(own).is_some_and(|&f| f & 0x80 == 0);
                let on = t.read(0xA840);
                let fine = on == want && !cleared;
                if !fine && std::env::var("SECRET_TRACE").is_ok() {
                    trace_modes(&g, p, l);
                }
                ok &= fine;
                println!("level {l:2} point {p:02x}: {} on point {on:02x} map {:02x} (want {want:02x}), level point cleared {cleared}", if fine { "ok" } else { "FAIL" }, t.read(0xA68B));
            }
            None => {
                ok = false;
                println!("level {l:2} point {p:02x}: FAIL did not get back to the map");
                if std::env::var("SECRET_TRACE").is_ok() {
                    trace_modes(&g, p, l);
                }
            }
        }
    }
    // the walk on after a level in a zone is the game's: its own level from $33 ends on $34
    let walked = through_block(&g, 0x33, 21, 0x4A).map(|mut t| t.read(0xA840));
    println!("level 21 cleared from point 33 (the game's walk on): on {walked:02x?}");
    println!("{}", if ok { "secretcheck ok" } else { "secretcheck FAILED" });
}

fn trace_modes(g: &Gb, point: u8, level: u8) {
    use cart_model::level_start::Start;
    let mut t = g.clone();
    let Some((c, r)) = block_at(t.cart().rom(), level as usize, 0x4B) else { return };
    let s = Start::standing(c * 16 - 4, (r + 1) * 16);
    let at = levels::header(t.cart().rom(), level as usize).at;
    s.write(&mut t.cart_mut().rom_mut()[at..at + 8]);
    if !t.show_map_at(point) || !t.enter_level(level) {
        return;
    }
    let mut last = (0xFFu8, 0u8, 0u8);
    for f in 0..3000u32 {
        let mode = t.read(0xFF9B);
        let b = if mode == 0x04 { Buttons::RIGHT } else { Buttons::NONE };
        t.run_frame_with(b);
        t.clear_audio();
        let now = (t.read(0xFF9B), t.read(0xA840), t.read(0xA68B));
        if now != last {
            println!("  f{f}: mode {:02x} point {:02x} map {:02x} pc {:04x}", now.0, now.1, now.2, t.pc());
            last = now;
        }
    }
}

/// Mario walking on after arriving in a zone: level 17's secret exit sent to the Pumpkin
/// Zone's $28 (or, with `own`, the zone's level from $28 cleared); the RAM that changes from
/// the frame the map is up to the frame Mario starts to move.
pub fn walk_probe(rom_path: &Path, own: bool) {
    use cart_model::secret_exits::{self, SecretTo};
    let Some(g0) = load(rom_path) else { return };
    let Ok(mut rom) = cart_model::patch::expand(g0.cart().rom()) else { return };
    if secret_exits::write(&mut rom, &[(17, SecretTo::Point(0x28))], &[]).is_err() {
        return;
    }
    let Ok(mut g) = Gb::new(rom, 44100) else { return };
    g.boot_to_map();
    let (level, point, b) = if own { (6u8, 0x28u8, 0x4Au8) } else { (17, 0x05, 0x4B) };
    let Some((c, r)) = block_at(g.cart().rom(), level as usize, b) else { return };
    let s = cart_model::level_start::Start::standing(c * 16 - 4, (r + 1) * 16);
    let at = levels::header(g.cart().rom(), level as usize).at;
    s.write(&mut g.cart_mut().rom_mut()[at..at + 8]);
    if !g.show_map_at(point) || !g.enter_level(level) {
        println!("not entered");
        return;
    }
    let ram = |g: &mut Gb| (0xA200..0xA900u16).chain(0xC000..0xC100).chain(0xFF80..0xFFFF).map(|a| (a, g.read(a))).collect::<Vec<_>>();
    let mut on_map: Option<Vec<(u16, u8)>> = None;
    let mut place = (0, 0);
    for f in 0..2500u32 {
        let mode = g.read(0xFF9B);
        g.run_frame_with(if mode == 0x04 { Buttons::RIGHT } else { Buttons::NONE });
        g.clear_audio();
        if g.read(0xFF9B) == 0x0C && on_map.is_none() {
            on_map = Some(ram(&mut g));
            place = (g.read(0xA842), g.read(0xA843));
            println!("f{f}: map up, point {:02x} place {place:02x?} a844 {:02x}", g.read(0xA840), g.read(0xA844));
        } else if let Some(before) = &on_map {
            let now = (g.read(0xA842), g.read(0xA843));
            if now != place {
                let after = ram(&mut g);
                let d: Vec<String> = before.iter().zip(&after).filter(|(a, b)| a.1 != b.1).map(|(a, b)| format!("{:04x}:{:02x}>{:02x}", a.0, a.1, b.1)).collect();
                println!("f{f}: moves to {now:02x?}; changed since map up: {}", d.join(" "));
                return;
            }
        }
    }
    println!("did not move");
}
