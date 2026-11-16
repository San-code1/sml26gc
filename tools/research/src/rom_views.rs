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
pub(crate) fn on_world_picture(gb: &mut gb_core::Gb) -> Option<(i32, i32)> {
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

/// From every map point, press each direction once and follow Mario: the points ($A840)
/// he goes through and where he stops. `pokes` (hex "a2xx=ff,...") are written on the point,
/// e.g. to open paths a new game keeps shut.
pub fn path_scan(rom_path: &Path, pokes: &str) {
    use cart_model::ram_map::MAP_POINT;
    use cart_model::world::POINT_COUNT;
    use gb_core::Buttons;
    let Some(mut base) = load(rom_path) else { return };
    if std::env::var("POKE_FIRST").is_ok() {
        crate::console::apply_pokes(&mut base, pokes);
    }
    base.boot_to_map();
    if std::env::var("CLEAR_ALL").is_ok() {
        clear_all(&mut base, &[0x09]);
    }
    crate::console::apply_pokes(&mut base, pokes);
    for p in 0..POINT_COUNT as u8 {
        let mut g = base.clone();
        g.show_map_at(p);
        if g.read(MAP_POINT) != p {
            println!("point {p:02x}: not reachable (shown at {:02x})", g.read(MAP_POINT));
            continue;
        }
        println!("start {p:02x} at {:?}", mario_spot(&mut g));
        for (name, b) in [("R", Buttons::RIGHT), ("L", Buttons::LEFT), ("U", Buttons::UP), ("D", Buttons::DOWN)] {
            let mut h = g.clone();
            let mut trail = vec![p];
            let mut track = Vec::new();
            let mut still = 0;
            let mut last_oam = (0u8, 0u8, 0u8, 0u8);
            for f in 0..600u32 {
                h.run_frame_with(if f < 6 { b } else { Buttons::NONE });
                h.clear_audio();
                // Mario's track, for the bends of the paths
                if std::env::var("TRACK").is_ok() {
                    if let Some(t) = mario_spot(&mut h) {
                        if track.last() != Some(&t) {
                            println!("  t {:?} {} {}", t.0, t.1, t.2);
                            track.push(t);
                        }
                    }
                }
                let a = h.read(MAP_POINT);
                if *trail.last().unwrap() != a {
                    trail.push(a);
                    println!("  passing {a:02x} at {:?}", mario_spot(&mut h));
                    if std::env::var("OAM_DEBUG").is_ok() {
                        let o = h.video().oam();
                        let v: Vec<(u8, u8, u8)> = (0..12).map(|i| (o[i * 4 + 1], o[i * 4], o[i * 4 + 2])).collect();
                        println!("    oam {v:02x?}");
                    }
                }
                // Mario's sprite and the scroll (on the world map the camera follows him)
                let (o0, o1) = (h.video().oam()[0], h.video().oam()[1]);
                let now = (o0, o1, h.read(0xFF42), h.read(0xFF43));
                still = if now == last_oam { still + 1 } else { 0 };
                last_oam = now;
                if f > 20 && still > 40 {
                    break;
                }
            }
            if trail.len() > 1 {
                println!("point {p:02x} {name}: {}", trail.iter().map(|t| format!("{t:02x}")).collect::<Vec<_>>().join(" "));
                println!("  stops on {:02x} at {:?}", h.read(MAP_POINT), mario_spot(&mut h));
            }
        }
    }
}

/// Which single RAM byte, set to $FF, lets Mario leave map point `point` (for finding where
/// the game keeps which paths are open).
pub fn unlock(rom_path: &Path, point: u8, lo: u16, hi: u16) {
    use cart_model::ram_map::MAP_POINT;
    use gb_core::Buttons;
    let Some(mut base) = load(rom_path) else { return };
    base.boot_to_map();
    base.show_map_at(point);
    for a in lo..=hi {
        if (0xA840..=0xA84F).contains(&a) {
            continue;
        }
        for b in [Buttons::RIGHT, Buttons::LEFT, Buttons::UP, Buttons::DOWN] {
            let mut h = base.clone();
            h.write(a, 0xFF);
            for f in 0..90u32 {
                h.run_frame_with(if f < 6 { b } else { Buttons::NONE });
                h.clear_audio();
            }
            if h.read(MAP_POINT) != point {
                println!("{a:04x}=ff: {:?} moves to {:02x}", b, h.read(MAP_POINT));
            }
        }
    }
}

/// Hold `dir` (R/L/U/D) for `hold` frames on the map after booting (optionally after
/// `show_map_at(point)`), printing $A840 and Mario's sprite as they change.
pub fn walk_test(rom_path: &Path, point: Option<u8>, dir: &str, hold: u32) {
    use cart_model::ram_map::{GAME_MODE, MAP_POINT};
    use gb_core::Buttons;
    let Some(mut g) = load(rom_path) else { return };
    g.boot_to_map();
    if let Some(p) = point {
        g.show_map_at(p);
    }
    let b = match dir { "R" => Buttons::RIGHT, "L" => Buttons::LEFT, "U" => Buttons::UP, _ => Buttons::DOWN };
    let mut last = (0, 0, 0, 0);
    for f in 0..400u32 {
        g.run_frame_with(if f < hold { b } else { Buttons::NONE });
        g.clear_audio();
        let (sx, sy) = (g.video().oam()[1], g.video().oam()[0]);
        let now = (g.read(MAP_POINT), g.read(GAME_MODE), sx, sy);
        if now != last {
            println!("f{f}: point {:02x} mode {:02x} sprite ({},{})", now.0, now.1, now.2, now.3);
            last = now;
        }
    }
}

/// In level 0, set the game mode byte to each value the code writes, let the game run back
/// to the map, and see whether Mario can then leave point 0 (which mode ends a level as
/// cleared).
pub fn clear_scan(rom_path: &Path) {
    use cart_model::ram_map::{GAME_MODE, MAP_POINT};
    use gb_core::Buttons;
    let Some(mut base) = load(rom_path) else { return };
    base.boot_to_map();
    base.enter_level(0);
    for _ in 0..60 {
        base.run_frame_with(Buttons::NONE);
    }
    for m in [0x05u8, 0x06, 0x07, 0x08, 0x09, 0x11, 0x12, 0x15, 0x16, 0x18, 0x19, 0x1b, 0x1d, 0x23] {
        let mut g = base.clone();
        g.write(GAME_MODE, m);
        let mut trail = vec![m];
        let mut on_map = 0;
        for f in 0..3000u32 {
            // tap A now and then to get through any message
            g.run_frame_with(if f % 90 < 3 && on_map == 0 { Buttons::A } else { Buttons::NONE });
            g.clear_audio();
            let now = g.read(GAME_MODE);
            if trail.last() != Some(&now) {
                trail.push(now);
            }
            on_map = if now == 0x0C { on_map + 1 } else { 0 };
            if on_map > 200 {
                break;
            }
        }
        let mut moved = Vec::new();
        for b in [Buttons::UP, Buttons::RIGHT, Buttons::LEFT, Buttons::DOWN] {
            let mut h = g.clone();
            for f in 0..120u32 {
                h.run_frame_with(if f < 20 { b } else { Buttons::NONE });
            }
            moved.push(h.read(MAP_POINT));
        }
        println!("mode {m:02x}: modes {trail:02x?} point {:02x} after U/R/L/D {moved:02x?}", g.read(MAP_POINT));
    }
}

/// Clear every level as the game does at a goal: inside the level the game mode is set to
/// the first of `modes` that brings the game back to the map (the goal, a boss...). Ends on
/// the map.
pub fn clear_all(g: &mut gb_core::Gb, modes: &[u8]) {
    use cart_model::ram_map::GAME_MODE;
    use gb_core::Buttons;
    for l in 0..32u8 {
        // stand on the point that opens level l, then press Start there
        let table = cart_model::world::point_levels(g.cart().rom());
        if let Some(p) = table.iter().position(|&t| t == l) {
            g.write(cart_model::ram_map::MAP_POINT, p as u8);
        }
        let mut inside = g.clone();
        if !inside.enter_level(l) {
            eprintln!("level {l}: not entered");
            continue;
        }
        for _ in 0..30 {
            inside.run_frame_with(Buttons::NONE);
        }
        let mut done = None;
        for &m in modes {
            let mut t = inside.clone();
            t.write(GAME_MODE, m);
            let mut on_map = 0;
            let mut trail = vec![m];
            for f in 0..6000u32 {
                // tap A or Start now and then to get through a zone's ending
                let tap = match f % 240 { 0..=2 => Buttons::A, 120..=122 => Buttons::START, _ => Buttons::NONE };
                let mode = t.read(GAME_MODE);
                t.run_frame_with(if mode != 0x0C && mode != 0x14 { tap } else { Buttons::NONE });
                t.clear_audio();
                let now = t.read(GAME_MODE);
                if trail.last() != Some(&now) && trail.len() < 20 {
                    trail.push(now);
                }
                on_map = if now == 0x0C { on_map + 1 } else { 0 };
                if on_map > 600 {
                    break;
                }
            }
            if std::env::var("CLEAR_DEBUG").is_ok() {
                eprintln!("  level {l} mode {m:02x}: {trail:02x?}");
            }
            if on_map > 600 {
                done = Some((m, t));
                break;
            }
        }
        match done {
            Some((m, t)) => {
                *g = t;
                if std::env::var("CLEAR_DEBUG").is_ok() {
                    eprintln!("level {l}: mode {m:02x}, back on point {:02x}", g.read(0xA840));
                }
            }
            None => eprintln!("level {l}: no mode brought the game back to the map"),
        }
    }
}

/// RAM bytes ($A000-$BFFF, $C000-$DFFF) that differ after clearing level `level` with the
/// goal mode, compared with standing on the map before (where progress is kept).
pub fn clear_diff(rom_path: &Path, level: u8) {
    use cart_model::ram_map::{GAME_MODE, MAP_POINT};
    use gb_core::Buttons;
    let Some(mut g) = load(rom_path) else { return };
    g.boot_to_map();
    let before = crate::console::ram_snapshot(&mut g);
    let table = cart_model::world::point_levels(g.cart().rom());
    if let Some(p) = table.iter().position(|&t| t == level) {
        g.write(MAP_POINT, p as u8);
    }
    g.enter_level(level);
    for _ in 0..30 {
        g.run_frame_with(Buttons::NONE);
    }
    if std::env::var("CLEAR_EXIT").is_ok() {
        g.back_to_map();
    } else {
        g.write(GAME_MODE, 0x09);
    }
    let mut on_map = 0;
    for _ in 0..6000u32 {
        g.run_frame_with(Buttons::NONE);
        on_map = if g.read(GAME_MODE) == 0x0C { on_map + 1 } else { 0 };
        if on_map > 600 {
            break;
        }
    }
    let after = crate::console::ram_snapshot(&mut g);
    for (a, b) in before.iter().zip(&after) {
        if a.1 != b.1 && a.0 < 0xC000 {
            println!("{:04x}: {:02x} -> {:02x}", a.0, a.1, b.1);
        }
    }
}

/// Level `level` (entered from its point), game mode set to `mode`, then `buttons` held
/// (R/L/U/D/A/B letters): the modes the game goes through.
pub fn mode_try(rom_path: &Path, level: u8, mode: u8, buttons: &str) {
    use cart_model::ram_map::{GAME_MODE, MAP_POINT};
    use gb_core::Buttons;
    let Some(mut g) = load(rom_path) else { return };
    g.boot_to_map();
    let table = cart_model::world::point_levels(g.cart().rom());
    if let Some(p) = table.iter().position(|&t| t == level) {
        g.write(MAP_POINT, p as u8);
    }
    println!("entered {}", g.enter_level(level));
    for _ in 0..30 {
        g.run_frame_with(Buttons::NONE);
    }
    g.write(GAME_MODE, mode);
    let mut b = Buttons::NONE;
    for c in buttons.chars() {
        b |= match c { 'R' => Buttons::RIGHT, 'L' => Buttons::LEFT, 'U' => Buttons::UP, 'D' => Buttons::DOWN, 'A' => Buttons::A, 'B' => Buttons::B, _ => Buttons::NONE };
    }
    let mut last = 0xFFu8;
    for f in 0..6000u32 {
        g.run_frame_with(if f % 60 < 40 { b } else { Buttons::NONE });
        g.clear_audio();
        let m = g.read(GAME_MODE);
        if m != last {
            println!("f{f}: mode {m:02x} point {:02x}", g.read(MAP_POINT));
            last = m;
        }
    }
}

/// Which map picture is on screen and where Mario stands on it (the middle of his feet, in
/// the picture's pixels): the world map is found by matching the background with its
/// picture around the middle of the screen, a zone map by matching its 20 x 18 tiles.
pub fn mario_spot(gb: &mut gb_core::Gb) -> Option<(cart_model::original_points::OnMap, i32, i32)> {
    use cart_model::original_points::OnMap;
    use cart_model::zone_maps;
    let o = gb.video().oam().to_vec();
    // Mario is the first four sprites (16 x 16, the first one top left); in the Macro Zone
    // he is one 8 x 8 sprite and the other three are blank (tile $7F)
    if o[0] == 0 {
        return None;
    }
    let (x, y) = if o[6] == 0x7F { (o[1] as i32 - 4, o[0] as i32 - 10) } else { (o[1] as i32, o[0] as i32 - 4) };
    if let Some((cx, cy)) = on_world_picture(gb) {
        return Some((OnMap::World, (cx - 80 + x).rem_euclid(512), (cy - 72 + y).rem_euclid(512)));
    }
    let rom = gb.cart().rom().to_vec();
    let bg = gb.video().vram()[0x1800..0x2000].to_vec();
    let same = |z: usize| {
        let t = zone_maps::read(&rom, z);
        (0..zone_maps::W * zone_maps::H).filter(|&i| t[i] == bg[(i / zone_maps::W) * 32 + i % zone_maps::W]).count()
    };
    // most of the screen must match (some zones animate a few tiles), not a screen half drawn while maps change
    let z = (0..zone_maps::ZONES.len()).find(|&z| same(z) * 4 >= zone_maps::W * zone_maps::H * 3)?;
    Some((OnMap::Zone(z), x, y))
}
