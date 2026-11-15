//! Self-checks of what `cart-model` knows, against what the game itself does: levels decode
//! and store, objects, the world map. Each prints a summary line per check.
use crate::console::{load, shown};
use cart_model::GameDriver;
use gb_core::{Buttons, Gb};
use std::path::Path;

/// For every level: where its map is, its compressed size, whether decoding the ROM gives
/// the same blocks the game itself puts in memory when the level loads, and whether
/// re-compressing gives the same blocks back.
pub fn levels_check(rom_path: &Path) {
    use cart_model::levels::*;
    use cart_model::live::*;
    let Some(mut gb) = load(rom_path) else { return };
    let rom = gb.cart().rom().to_vec();
    if !gb.boot_to_map() {
        println!("could not reach the map");
        return;
    }
    let mut ok = 0;
    for l in 0..LEVELS {
        let h = header(&rom, l);
        let (entry, start) = map_location(&rom, h);
        let (map, used) = decode(&rom, start);
        let enc = encode(&map);
        let (back, _) = decode(&enc, 0);
        let mut g = gb.clone();
        let entered = g.enter_level(l as u8);
        let mem = map_in_memory(&g);
        let same = mem.iter().zip(&map).filter(|(a, b)| a == b).count();
        let first_diff = mem.iter().zip(&map).position(|(a, b)| a != b);
        if same == MAP_BYTES {
            ok += 1;
        }
        println!(
            "level {l:2}: header {:05x} bank {:02x} map {} table {entry:05x} data {start:05x} {used} bytes, re-encoded {} bytes roundtrip={} entered={entered} matches game memory {same}/{MAP_BYTES} first diff {first_diff:?}",
            h.at, h.bank, h.map, enc.len(), back == map
        );
    }
    println!("{ok}/{LEVELS} levels decode exactly as the game loads them");
}

/// Edit a level two ways (a small change that fits in place, and a big one that must move
/// to free space), store it, then let the game load it from the edited ROM and compare.
pub fn store_check(rom_path: &Path) {
    use cart_model::levels::*;
    use cart_model::live::*;
    use cart_model::{layout::*, patch::*};
    let Some(gb) = load(rom_path) else { return };
    let original = gb.cart().rom().to_vec();
    // 1) an expanded copy must play every level exactly like the original
    let expanded = match expand(&original) {
        Ok(r) => r,
        Err(e) => {
            println!("expand: {e}");
            return;
        }
    };
    let Ok(mut g0) = Gb::new(expanded.clone(), 44100) else { return };
    let booted = g0.boot_to_map();
    let mut same_levels = 0;
    for l in 0..LEVELS {
        let mut g = g0.clone();
        let (_, s) = map_location(&original, header(&original, l));
        let (want, _) = decode(&original, s);
        if g.enter_level(l as u8) && map_in_memory(&g) == want {
            same_levels += 1;
        }
    }
    println!("expanded copy: boots={booted}, {same_levels}/{LEVELS} levels load exactly as in the original");
    // 2) big edits in the expanded copy
    for (level, rows) in [(0usize, 10usize), (25, 30)] {
        let mut rom = expanded.clone();
        let h = header(&rom, level);
        let (_, start) = map_location(&rom, h);
        let (mut map, old) = decode(&rom, start);
        for r in 5..5 + rows {
            for c in 0..200 {
                map[r * COLS + c] = if (r + c) % 2 == 0 { 0x01 } else { 0x00 };
            }
        }
        let res = store_bank(&mut rom, &original, h.bank, &[(h.map, &map)]);
        let Ok(mut g) = Gb::new(rom.clone(), 44100) else { return };
        let ok = g.boot_to_map() && g.enter_level(level as u8);
        let same = map_in_memory(&g).iter().zip(&map).filter(|(a, b)| a == b).count();
        // the other levels of the bank must still load unchanged
        let mut others_ok = true;
        for o in 0..LEVELS {
            if o != level && header(&rom, o).bank == h.bank {
                let mut g2 = Gb::new(rom.clone(), 44100).unwrap();
                g2.boot_to_map();
                let (_, s) = map_location(&original, header(&original, o));
                let (want, _) = decode(&original, s);
                if !(g2.enter_level(o as u8) && map_in_memory(&g2) == want) {
                    others_ok = false;
                }
            }
        }
        println!("expanded big edit: level {level} ({old} bytes, {rows} rows of checkerboard) -> {}; game loaded={ok}; matches {same}/{MAP_BYTES}; other levels of the zone unchanged={others_ok}", shown(&res));
    }
    for (name, level, big) in [("small edit", 0usize, false), ("too big edit", 25usize, true), ("grow one, simplify another", 26usize, false)] {
        let mut rom = original.clone();
        let h = header(&rom, level);
        let (_, start) = map_location(&rom, h);
        let (mut map, old) = decode(&rom, start);
        let mut edits: Vec<(u8, Vec<u8>)> = Vec::new();
        if big {
            // a checkerboard band: many short runs, far bigger than the original data
            for r in 20..30 {
                for c in 0..200 {
                    map[r * COLS + c] = if (r + c) % 2 == 0 { 0x01 } else { 0x00 };
                }
            }
        } else if level == 26 {
            // add 300 bytes of detail here, and empty level 27 (same bank) to make room
            for c in 0..150 {
                map[3 * COLS + c] = if c % 2 == 0 { 0x01 } else { 0x00 };
            }
            let h27 = header(&rom, 27);
            let (_, s27) = map_location(&rom, h27);
            let (mut m27, _) = decode(&rom, s27);
            for r in 0..10 {
                for c in 0..COLS {
                    m27[r * COLS + c] = m27[10 * COLS];
                }
            }
            edits.push((h27.map, m27));
        } else {
            map[10 * COLS + 12] = 0x01;
        }
        edits.push((h.map, map.clone()));
        let refs: Vec<(u8, &[u8])> = edits.iter().map(|(m, d)| (*m, d.as_slice())).collect();
        let res = store_bank(&mut rom, &original, h.bank, &refs);
        let Ok(mut g) = Gb::new(rom.clone(), 44100) else { return };
        let ok = g.boot_to_map() && g.enter_level(level as u8);
        let mem = map_in_memory(&g);
        let same = mem.iter().zip(&map).filter(|(a, b)| a == b).count();
        println!("{name}: level {level} old {old} bytes -> {}; game loaded={ok}; game memory matches edit {same}/{MAP_BYTES}", shown(&res));
        // nothing else in the ROM changed except the map data and (if moved) its table entry
        let changed = rom.iter().zip(&original).filter(|(a, b)| a != b).count();
        println!("   bytes changed in the ROM: {changed}");
    }
}

/// Objects: re-encoding gives the same bytes; what the game unpacks matches our reading;
/// the lists sit back to back; and an edited list (moved, removed, added, new type) is what
/// the game unpacks after storing it.
pub fn objects_check(rom_path: &Path) {
    use cart_model::objects::{self, Obj};
    let Some(mut gb) = load(rom_path) else { return };
    let original = gb.cart().rom().to_vec();
    gb.boot_to_map();
    let mut total = 0;
    let mut good = 0;
    let mut bytes_ok = true;
    for l in 0..cart_model::levels::LEVELS {
        let (objs, len) = objects::read(&original, l);
        total += len;
        let mut g = gb.clone();
        g.enter_level(l as u8);
        let mem = cart_model::live::objects_in_memory(&g, objs.len());
        let same = mem.iter().zip(&objs).filter(|(a, b)| a == b).count();
        if same == objs.len() {
            good += 1;
        } else {
            println!("level {l}: {same}/{} objects match the game's records", objs.len());
        }
        for o in &objs {
            let b = o.to_bytes();
            if Obj::from_bytes(b[0], b[1], b[2]) != *o {
                bytes_ok = false;
            }
        }
    }
    let (start, cap) = objects::region(&original);
    println!("{good}/32 levels: the game's unpacked objects match; round trip ok={bytes_ok}; lists total {total} bytes, region {start:05x} {cap} bytes");
    // edit level 0
    let mut lists: Vec<Vec<Obj>> = (0..cart_model::levels::LEVELS).map(|l| objects::read(&original, l).0).collect();
    let removed = lists[1].pop();
    lists[0][0].x += 16;
    lists[0][1].kind = lists[0][2].kind;
    let extra = Obj { x: 40 * 16, y: lists[0][0].y, kind: lists[0][3].kind, flag: false };
    lists[0].push(extra);
    let mut rom = original.clone();
    let res = objects::store(&mut rom, &original, &lists);
    let mut g = Gb::new(rom.clone(), 44100).unwrap();
    g.boot_to_map();
    let mut g0 = g.clone();
    g0.enter_level(0);
    let mut want = lists[0].clone();
    want.sort_by_key(|o| o.x / 256);
    let mem = cart_model::live::objects_in_memory(&g0, want.len());
    let mut g1 = g.clone();
    g1.enter_level(1);
    let mem1 = cart_model::live::objects_in_memory(&g1, lists[1].len());
    println!(
        "edit: {}; level 0 game records match edit: {}; level 1 (one removed: {removed:?}) match: {}",
        shown(&res),
        mem == want,
        mem1 == lists[1]
    );
    // expanded copy: everything still the same, then 40 more objects in level 0
    let exp = cart_model::patch::expand(&original).unwrap();
    let mut gx = Gb::new(exp.clone(), 44100).unwrap();
    gx.boot_to_map();
    let mut same = 0;
    for l in 0..cart_model::levels::LEVELS {
        let (objs, _) = objects::read(&original, l);
        let mut g = gx.clone();
        g.enter_level(l as u8);
        if cart_model::live::objects_in_memory(&g, objs.len()) == objs && objects::read(&exp, l).0 == objs {
            same += 1;
        }
    }
    let mut lists: Vec<Vec<Obj>> = (0..cart_model::levels::LEVELS).map(|l| objects::read(&exp, l).0).collect();
    let (k, y0) = (lists[0][0].kind, lists[0][0].y);
    for i in 0..40u16 {
        lists[0].push(Obj { x: 304 + i * 48, y: y0, kind: k, flag: false }); // positions are in 8 px steps
    }
    let mut rom = exp.clone();
    let res = objects::store(&mut rom, &original, &lists);
    let mut g = Gb::new(rom.clone(), 44100).unwrap();
    g.boot_to_map();
    let mut ok = 0;
    for l in [0usize, 5, 17, 31] {
        let mut gl = g.clone();
        gl.enter_level(l as u8);
        let mut want = lists[l].clone();
        want.sort_by_key(|o| o.x / 256);
        let mem = cart_model::live::objects_in_memory(&gl, want.len());
        if mem == want {
            ok += 1;
        } else {
            let first = mem.iter().zip(&want).position(|(a, b)| a != b);
            println!("  level {l}: {} objects, first difference at {first:?}: game {:?} / stored {:?}", want.len(), first.map(|i| mem[i]), first.map(|i| want[i]));
        }
    }
    println!("expanded: {same}/32 levels' objects unchanged; +40 objects in level 0 -> {}; levels 0,5,17,31 load as stored: {ok}/4", shown(&res));
}

/// World map: the 32x32 window the game puts in VRAM on the map matches our reading of the
/// picture; an edited tile shows up there; and a re-pointed map point opens another level.
pub fn world_check(rom_path: &Path) {
    use cart_model::world;
    let Some(gb) = load(rom_path) else { return };
    let original = gb.cart().rom().to_vec();
    let pic = world::read(&original);
    let window = |g: &Gb| g.video().vram()[0x1800..0x1C00].to_vec();
    // find where the VRAM window sits in the picture (the BG map wraps every 32 tiles)
    let find = |win: &[u8], pic: &[u8]| -> Option<(usize, usize)> {
        for oy in 0..world::H {
            for ox in 0..world::W {
                let ok = (0..32).all(|y| (0..32).all(|x| {
                    let (px, py) = ((ox + x) % world::W, (oy + y) % world::H);
                    // VRAM holds the tile at (px % 32, py % 32)
                    win[(py % 32) * 32 + px % 32] == pic[py * world::W + px]
                }));
                if ok {
                    return Some((ox, oy));
                }
            }
        }
        None
    };
    let mut g = Gb::new(original.clone(), 44100).unwrap();
    g.boot_to_map();
    let pos = find(&window(&g), &pic);
    // count how many tiles of the VRAM window equal the picture at the same wrapped place
    let best = best_window_match(&window(&g), &pic);
    println!("original: VRAM window found at {pos:?}; best match {best}/1024 tiles");
    // edit: change every tile in the picture's top-left quadrant row 10 and point 0 -> level 6
    let mut rom = cart_model::patch::expand(&original).unwrap();
    let mut p2 = pic.clone();
    let target = pic[0]; // the sea tile
    let mut changed = 0;
    for y in 0..world::H {
        for x in 0..world::W {
            if p2[y * world::W + x] != target && changed < 1 && y > 20 && x > 20 {
                p2[y * world::W + x] = target;
                changed += 1;
            }
        }
    }
    world::write(&mut rom, &p2);
    let mut pts = world::point_levels(&rom);
    pts[0] = 6;
    world::write_point_levels(&mut rom, &pts);
    let mut g = Gb::new(rom.clone(), 44100).unwrap();
    g.boot_to_map();
    let same_new = best_window_match(&window(&g), &p2);
    // press Start on the map and see which level the game asks for
    let mut level = None;
    for f in 0..300u32 {
        let m = g.mode();
        if m == cart_model::GameMode::LevelHeader {
            level = Some(g.read(cart_model::ram_map::LEVEL_NUMBER));
            break;
        }
        g.run_frame_with(if f % 30 < 4 { Buttons::START } else { Buttons::NONE });
    }
    println!("edited: VRAM window best match with the edited picture {same_new}/1024; Start on point 0 opens level {level:?} (set to 6)");
}

/// The most tiles of the 32x32 VRAM window that equal the world picture, over every place
/// the window could sit (the BG map wraps every 32 tiles).
fn best_window_match(win: &[u8], pic: &[u8]) -> usize {
    use cart_model::world;
    (0..world::H * world::W)
        .map(|o| {
            let (ox, oy) = (o % world::W, o / world::W);
            (0..32 * 32)
                .filter(|&i| {
                    let (x, y) = (i % 32, i / 32);
                    let (px, py) = ((ox + x) % world::W, (oy + y) % world::H);
                    win[(py % 32) * 32 + px % 32] == pic[py * world::W + px]
                })
                .count()
        })
        .max()
        .unwrap_or(0)
}

/// New levels (beyond the 32): store two in an expanded copy, play each in place of its
/// base level and compare what the game loaded; then the base level must load as before.
pub fn extra_check(rom_path: &Path) {
    use cart_model::extra_levels::{apply, based_on, read, store, writes_for};
    use cart_model::levels::{decode, header, map_location, COLS};
    use cart_model::live::{map_in_memory, objects_in_memory};
    use cart_model::objects::Obj;
    let Some(gb) = load(rom_path) else { return };
    let Ok(mut rom) = cart_model::patch::expand(gb.cart().rom()) else { return };
    let mut a = based_on(&rom, 0);
    for r in 20..30 {
        for c in 10..120 {
            a.map[r * COLS + c] = if (r + c) % 3 == 0 { 0x01 } else { 0x00 };
        }
    }
    a.objs.truncate(3);
    a.objs.push(Obj { x: 200, y: 296, kind: 5, flag: false });
    a.objs.sort_by_key(|o| o.x); // stored left to right, as the game keeps them
    let mut b = based_on(&rom, 25);
    b.map.iter_mut().take(COLS * 4).for_each(|v| *v = 0);
    b.objs.clear();
    println!("store: {}", shown(&store(&mut rom, &[a.clone(), b.clone()]).map(|n| format!("{n} bytes"))));
    println!("read back the same: {}", read(&rom) == vec![a.clone(), b.clone()]);
    if let Ok(out) = std::env::var("EXTRA_OUT") {
        let _ = std::fs::write(out, &rom); // the copy with the two new levels, to look at
    }
    let Ok(mut g0) = Gb::new(rom.clone(), 44100) else { return };
    println!("boots: {}", g0.boot_to_map());
    for (n, e) in [(32, &a), (33, &b)] {
        let mut g = g0.clone();
        let undo = apply(g.cart_mut().rom_mut(), &writes_for(&rom, e));
        let entered = g.enter_level(e.base);
        let mut want = e.objs.clone();
        want.sort_by_key(|o| o.x);
        let map_ok = map_in_memory(&g) == e.map;
        let objs_ok = objects_in_memory(&g, want.len()) == want;
        println!("level {n} (base {}): entered={entered} map={map_ok} objects={objs_ok}", e.base);
        if !objs_ok {
            println!("  want {want:?}
  got  {:?}", objects_in_memory(&g, want.len() + 1));
        }
        // back to the map: the base level is itself again
        g.back_to_map();
        for _ in 0..600 {
            g.run_frame_with(Buttons::NONE);
            if g.mode() == cart_model::GameMode::Map {
                break;
            }
        }
        apply(g.cart_mut().rom_mut(), &undo);
        let same_rom = g.cart().rom() == &rom[..];
        let (_, s) = map_location(&rom, header(&rom, e.base as usize));
        let again = g.enter_level_anywhere(e.base) && map_in_memory(&g) == decode(&rom, s).0;
        println!("  then base level {} loads as before: {again} (ROM restored: {same_rom})", e.base);
    }
}

/// The map's paths moved to bank $36: read back the same, and the game walks a changed way
/// to a new point, stands on it and enters its level. ROUTE_OUT writes the changed copy.
pub fn route_check(rom_path: &Path) {
    use cart_model::map_routes::{read, write, Place, Way, FREE_POINTS};
    use cart_model::ram_map::{GAME_MODE, LEVEL_NUMBER, MAP_POINT};
    use cart_model::world;
    let Some(gb) = load(rom_path) else { return };
    let Ok(mut rom) = cart_model::patch::expand(gb.cart().rom()) else { return };
    let r = read(&rom);
    println!("move: {}", shown(&write(&mut rom, &r).map(|n| format!("{n} bytes added"))));
    println!("read back the same: {}", read(&rom) == r);
    let same = rom.clone();
    // a new point 4 steps above point 04, way up from 04 and back down, opening level 7
    let p = FREE_POINTS[0];
    let mut c = r.clone();
    let (x, y) = c.places[4].on_world();
    c.ways[4][2] = Some(Way::through(&[(0, -32)], p));
    c.ways[p as usize][3] = Some(Way::through(&[(0, 32)], 4));
    c.places[p as usize] = Place::at_world(x, y - 32);
    c.added.push(p);
    println!("change: {}", shown(&write(&mut rom, &c).map(|n| format!("{n} bytes added"))));
    println!("read back the same: {}", read(&rom) == c);
    let mut lv = world::point_levels(&rom);
    lv[p as usize] = 7;
    world::write_point_levels(&mut rom, &lv);
    if let Ok(out) = std::env::var("ROUTE_OUT") {
        let _ = std::fs::write(out, &rom);
    }
    for (name, image) in [("moved", same), ("changed", rom)] {
        let Ok(mut g) = Gb::new(image, 44100) else { return };
        if !g.boot_to_map() {
            println!("{name}: could not reach the map");
            continue;
        }
        g.show_map_at(4);
        print!("{name}: on {:02x}; ", g.read(MAP_POINT));
        let walk = |g: &mut Gb, b: Buttons| {
            for f in 0..240u32 {
                g.run_frame_with(if f < 6 { b } else { Buttons::NONE });
                g.clear_audio();
            }
            g.read(MAP_POINT)
        };
        let down = walk(&mut g.clone(), Buttons::LEFT);
        let up = walk(&mut g, Buttons::UP);
        print!("from 04 left to {down:02x}, up to {up:02x}");
        if up == p {
            let back = walk(&mut g.clone(), Buttons::DOWN);
            let mut modes = Vec::new();
            for f in 0..300u32 {
                g.run_frame_with(if f < 6 { Buttons::A } else { Buttons::NONE });
                g.clear_audio();
                let m = g.read(GAME_MODE);
                if modes.last() != Some(&m) {
                    modes.push(m);
                }
            }
            print!(", back down to {back:02x}, A: modes {modes:02x?} level {}", g.read(LEVEL_NUMBER));
            g.back_to_map();
            for _ in 0..300 {
                g.run_frame_with(Buttons::NONE);
                g.clear_audio();
            }
            let cam = (g.read(0xFF43), g.read(0xFF42));
            let want = c.places[p as usize];
            print!(
                "; back on the map at {:02x}, Mario ({:02x},{:02x}) scroll {cam:?} (place {:?})",
                g.read(MAP_POINT),
                g.read(0xA842),
                g.read(0xA843),
                want
            );
            print!(", then down to {:02x}", walk(&mut g, Buttons::DOWN));
        }
        println!();
    }
}

/// Leaving a zone on an edited copy: the zone exit point follows its entrance's paths.
/// From zone point `from` press `first` (out of the zone), then `then` on the world map.
pub fn exit_check(rom_path: &Path, from: u8, first: &str, then: &str) {
    use cart_model::map_routes::{read, write};
    use cart_model::ram_map::MAP_POINT;
    let Some(gb) = load(rom_path) else { return };
    let mut rom = gb.cart().rom().to_vec();
    let mut r = read(&rom);
    println!("exits follow their entrance: changed {}", r.sync_exits());
    if cart_model::map_routes::can_write(&rom) {
        println!("write: {}", shown(&write(&mut rom, &r).map(|n| format!("{n} bytes"))));
    }
    let Ok(mut g) = Gb::new(rom, 44100) else { return };
    if !g.boot_to_map() {
        println!("could not reach the map");
        return;
    }
    g.show_map_at(from);
    let pad = |s: &str| match s { "R" => Buttons::RIGHT, "L" => Buttons::LEFT, "U" => Buttons::UP, _ => Buttons::DOWN };
    let mut trail = vec![g.read(MAP_POINT)];
    for b in [pad(first), pad(then)] {
        for f in 0..400u32 {
            g.run_frame_with(if f < 6 { b } else { Buttons::NONE });
            g.clear_audio();
            let a = g.read(MAP_POINT);
            if trail.last() != Some(&a) {
                trail.push(a);
            }
        }
    }
    println!("points: {}", trail.iter().map(|t| format!("{t:02x}")).collect::<Vec<_>>().join(" "));
}

/// Try a map change: patch the ROM (`patches` "BB:AAAA=hexbytes,..."), show the map at
/// `point`, press `pads` one after another (R L U D A), and say where Mario ends up: the
/// point, the map ($A68B, 0 = world) and a picture `png`.
pub fn door_try(rom_path: &Path, patches: &str, point: u8, pads: &str, png: &Path) {
    use cart_model::ram_map::{GAME_MODE, MAP_POINT};
    let Some(gb) = load(rom_path) else { return };
    let mut rom = gb.cart().rom().to_vec();
    for p in patches.split(',').filter(|s| !s.is_empty()) {
        let (at, bytes) = p.split_once('=').unwrap();
        let (b, a) = at.split_once(':').unwrap();
        let o = cart_model::rom::offset(usize::from_str_radix(b, 16).unwrap(), u16::from_str_radix(a, 16).unwrap());
        for (i, k) in (0..bytes.len()).step_by(2).enumerate() {
            rom[o + i] = u8::from_str_radix(&bytes[k..k + 2], 16).unwrap();
        }
    }
    let Ok(mut g) = Gb::new(rom, 44100) else { return };
    if !g.boot_to_map() {
        println!("could not reach the map");
        return;
    }
    g.show_map_at(point);
    let mut trail = vec![(g.read(MAP_POINT), g.read(0xA68B), g.read(GAME_MODE))];
    for c in pads.chars() {
        let b = match c { 'R' => Buttons::RIGHT, 'L' => Buttons::LEFT, 'U' => Buttons::UP, 'D' => Buttons::DOWN, _ => Buttons::A };
        for f in 0..500u32 {
            g.run_frame_with(if f < 6 { b } else { Buttons::NONE });
            g.clear_audio();
            let now = (g.read(MAP_POINT), g.read(0xA68B), g.read(GAME_MODE));
            if trail.last() != Some(&now) {
                trail.push(now);
            }
        }
    }
    println!("(point, map, mode): {}", trail.iter().map(|t| format!("({:02x},{},{:02x})", t.0, t.1, t.2)).collect::<Vec<_>>().join(" "));
    println!("Mario ({:02x},{:02x})", g.read(0xA842), g.read(0xA843));
    host::png_out::save_png(&g, png);
}

/// Show the map at `point`, press `pads` (R L U D A), return (point, zone $A68B) as they change.
fn pads_trail(rom: Vec<u8>, point: u8, pads: &str) -> Vec<(u8, u8)> {
    use cart_model::ram_map::MAP_POINT;
    let Ok(mut g) = Gb::new(rom, 44100) else { return vec![] };
    if !g.boot_to_map() {
        return vec![];
    }
    g.show_map_at(point);
    let mut trail = vec![(g.read(MAP_POINT), g.read(0xA68B))];
    for c in pads.chars() {
        let b = match c { 'R' => Buttons::RIGHT, 'L' => Buttons::LEFT, 'U' => Buttons::UP, 'D' => Buttons::DOWN, _ => Buttons::A };
        for f in 0..500u32 {
            g.run_frame_with(if f < 6 { b } else { Buttons::NONE });
            g.clear_audio();
            let now = (g.read(MAP_POINT), g.read(0xA68B));
            if trail.last() != Some(&now) {
                trail.push(now);
            }
        }
    }
    trail
}

/// Doors changed through `cart_model::doors`, tried in the game.
pub fn door_check(rom_path: &Path) {
    use cart_model::doors::{read, write, Leads};
    let Some(gb) = load(rom_path) else { return };
    let rom = gb.cart().rom().to_vec();
    let d = read(&rom);
    println!("known: {}  into {:02x?}", cart_model::doors::known(&rom), d.into);
    println!("exits {:02x?}", d.exits);
    let show = |t: Vec<(u8, u8)>| t.iter().map(|(p, z)| format!("{p:02x}/{z}")).collect::<Vec<_>>().join(" ");
    let tries: Vec<(&str, Box<dyn Fn(&mut cart_model::doors::Doors)>, u8, &str)> = vec![
        ("as the game is: Tree Zone out", Box::new(|_| {}), 0x23, "D"),
        ("Tree Zone out into Macro Zone", Box::new(|d| d.exits[0].1 = Leads::Zone(2)), 0x23, "D"),
        ("Macro Zone out into Tree Zone", Box::new(|d| d.exits[4].1 = Leads::Zone(0)), 0x32, "L"),
        ("Tree Zone out to world point 07", Box::new(|d| d.exits[0].1 = Leads::World(0x07)), 0x23, "D"),
        ("07 opens Pumpkin Zone", Box::new(|d| d.set_door(0x07, Some(1))), 0x07, "A"),
        ("02 no longer opens Tree Zone", Box::new(|d| d.set_door(0x02, None)), 0x02, "A"),
        ("Tree Zone exit moved to 22", Box::new(|d| d.exits[0].0 = 0x22), 0x22, "D"),
        ("Space Zone out into Mario Zone", Box::new(|d| d.exits[5].1 = Leads::Zone(4)), 0x3C, "D"),
    ];
    for (name, change, point, pads) in tries {
        let mut r = rom.clone();
        let mut dd = d.clone();
        change(&mut dd);
        write(&mut r, &dd);
        println!("{name}: {}", show(pads_trail(r, point, pads)));
    }
}
