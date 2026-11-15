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
    let Ok(mut g0) = Gb::new(expanded.clone(), 44100) else {
        return;
    };
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
        let Ok(mut g) = Gb::new(rom.clone(), 44100) else {
            return;
        };
        let ok = g.boot_to_map() && g.enter_level(level as u8);
        let same = map_in_memory(&g)
            .iter()
            .zip(&map)
            .filter(|(a, b)| a == b)
            .count();
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
    for (name, level, big) in [
        ("small edit", 0usize, false),
        ("too big edit", 25usize, true),
        ("grow one, simplify another", 26usize, false),
    ] {
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
        let Ok(mut g) = Gb::new(rom.clone(), 44100) else {
            return;
        };
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
            println!(
                "level {l}: {same}/{} objects match the game's records",
                objs.len()
            );
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
    let mut lists: Vec<Vec<Obj>> = (0..cart_model::levels::LEVELS)
        .map(|l| objects::read(&original, l).0)
        .collect();
    let removed = lists[1].pop();
    lists[0][0].x += 16;
    lists[0][1].kind = lists[0][2].kind;
    let extra = Obj {
        x: 40 * 16,
        y: lists[0][0].y,
        kind: lists[0][3].kind,
        flag: false,
    };
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
        if cart_model::live::objects_in_memory(&g, objs.len()) == objs
            && objects::read(&exp, l).0 == objs
        {
            same += 1;
        }
    }
    let mut lists: Vec<Vec<Obj>> = (0..cart_model::levels::LEVELS)
        .map(|l| objects::read(&exp, l).0)
        .collect();
    let (k, y0) = (lists[0][0].kind, lists[0][0].y);
    for i in 0..40u16 {
        lists[0].push(Obj {
            x: 304 + i * 48,
            y: y0,
            kind: k,
            flag: false,
        }); // positions are in 8 px steps
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
            println!(
                "  level {l}: {} objects, first difference at {first:?}: game {:?} / stored {:?}",
                want.len(),
                first.map(|i| mem[i]),
                first.map(|i| want[i])
            );
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
                let ok = (0..32).all(|y| {
                    (0..32).all(|x| {
                        let (px, py) = ((ox + x) % world::W, (oy + y) % world::H);
                        // VRAM holds the tile at (px % 32, py % 32)
                        win[(py % 32) * 32 + px % 32] == pic[py * world::W + px]
                    })
                });
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
        g.run_frame_with(if f % 30 < 4 {
            Buttons::START
        } else {
            Buttons::NONE
        });
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
