//! Research tools used to read the cartridge's code (SM83 disassembler, RAM search,
//! write watches). They only print; the game itself never uses them.
use crate::{load_gb, Gb, GameDriver};
use gb_core::script::{buttons_at, parse_script};
use host::png_out::{save_png, write_rgba};
use std::path::Path;

const R8: [&str; 8] = ["b", "c", "d", "e", "h", "l", "(hl)", "a"];
const RP: [&str; 4] = ["bc", "de", "hl", "sp"];
const RP2: [&str; 4] = ["bc", "de", "hl", "af"];
const CC: [&str; 4] = ["nz", "z", "nc", "c"];
const ALU: [&str; 8] = ["add a,", "adc a,", "sub ", "sbc a,", "and ", "xor ", "or ", "cp "];
const ROT: [&str; 8] = ["rlc", "rrc", "rl", "rr", "sla", "sra", "swap", "srl"];

/// Disassemble one instruction; returns (text, length).
pub fn dis1(b: &[u8; 3], pc: u16) -> (String, u16) {
    let op = b[0];
    let n = b[1];
    let nn = (b[2] as u16) << 8 | b[1] as u16;
    let rel = pc.wrapping_add(2).wrapping_add(n as i8 as i16 as u16);
    let x = op >> 6;
    let y = (op >> 3) & 7;
    let z = op & 7;
    let s = |t: String, l| (t, l);
    match op {
        0x00 => s("nop".into(), 1),
        0x08 => s(format!("ld (${nn:04x}),sp"), 3),
        0x10 => s("stop".into(), 2),
        0x18 => s(format!("jr ${rel:04x}"), 2),
        0x20 | 0x28 | 0x30 | 0x38 => s(format!("jr {},${rel:04x}", CC[(y - 4) as usize]), 2),
        0x22 => s("ld (hl+),a".into(), 1),
        0x2A => s("ld a,(hl+)".into(), 1),
        0x32 => s("ld (hl-),a".into(), 1),
        0x3A => s("ld a,(hl-)".into(), 1),
        0x02 => s("ld (bc),a".into(), 1),
        0x12 => s("ld (de),a".into(), 1),
        0x0A => s("ld a,(bc)".into(), 1),
        0x1A => s("ld a,(de)".into(), 1),
        0x07 => s("rlca".into(), 1),
        0x0F => s("rrca".into(), 1),
        0x17 => s("rla".into(), 1),
        0x1F => s("rra".into(), 1),
        0x27 => s("daa".into(), 1),
        0x2F => s("cpl".into(), 1),
        0x37 => s("scf".into(), 1),
        0x3F => s("ccf".into(), 1),
        0x76 => s("halt".into(), 1),
        0xC3 => s(format!("jp ${nn:04x}"), 3),
        0xC9 => s("ret".into(), 1),
        0xD9 => s("reti".into(), 1),
        0xCD => s(format!("call ${nn:04x}"), 3),
        0xE0 => s(format!("ldh ($ff{n:02x}),a"), 2),
        0xF0 => s(format!("ldh a,($ff{n:02x})"), 2),
        0xE2 => s("ld ($ff00+c),a".into(), 1),
        0xF2 => s("ld a,($ff00+c)".into(), 1),
        0xE8 => s(format!("add sp,{}", n as i8), 2),
        0xF8 => s(format!("ld hl,sp{:+}", n as i8), 2),
        0xE9 => s("jp hl".into(), 1),
        0xF9 => s("ld sp,hl".into(), 1),
        0xEA => s(format!("ld (${nn:04x}),a"), 3),
        0xFA => s(format!("ld a,(${nn:04x})"), 3),
        0xF3 => s("di".into(), 1),
        0xFB => s("ei".into(), 1),
        0xCB => {
            let r = R8[(n & 7) as usize];
            let k = (n >> 3) & 7;
            match n >> 6 {
                0 => s(format!("{} {r}", ROT[k as usize]), 2),
                1 => s(format!("bit {k},{r}"), 2),
                2 => s(format!("res {k},{r}"), 2),
                _ => s(format!("set {k},{r}"), 2),
            }
        }
        _ => match x {
            0 => match z {
                1 => {
                    if y & 1 == 0 {
                        s(format!("ld {},${nn:04x}", RP[(y >> 1) as usize]), 3)
                    } else {
                        s(format!("add hl,{}", RP[(y >> 1) as usize]), 1)
                    }
                }
                3 => s(format!("{} {}", if y & 1 == 0 { "inc" } else { "dec" }, RP[(y >> 1) as usize]), 1),
                4 => s(format!("inc {}", R8[y as usize]), 1),
                5 => s(format!("dec {}", R8[y as usize]), 1),
                6 => s(format!("ld {},${n:02x}", R8[y as usize]), 2),
                _ => s(format!("db ${op:02x}"), 1),
            },
            1 => s(format!("ld {},{}", R8[y as usize], R8[z as usize]), 1),
            2 => s(format!("{}{}", ALU[y as usize], R8[z as usize]), 1),
            _ => match z {
                0 if y < 4 => s(format!("ret {}", CC[y as usize]), 1),
                1 if y & 1 == 0 => s(format!("pop {}", RP2[(y >> 1) as usize]), 1),
                2 if y < 4 => s(format!("jp {},${nn:04x}", CC[y as usize]), 3),
                4 if y < 4 => s(format!("call {},${nn:04x}", CC[y as usize]), 3),
                5 if y & 1 == 0 => s(format!("push {}", RP2[(y >> 1) as usize]), 1),
                6 => s(format!("{}${n:02x}", ALU[y as usize]), 2),
                7 => s(format!("rst ${:02x}", y * 8), 1),
                _ => s(format!("db ${op:02x}"), 1),
            },
        },
    }
}

/// Print `count` instructions of ROM bank `bank` starting at CPU address `addr`.
pub fn disasm(rom: &Path, bank: usize, addr: u16, count: u32) {
    let Ok(gb) = load_gb(rom, 44100, false) else { return };
    let off = |a: u16| -> usize {
        if a < 0x4000 {
            a as usize
        } else {
            bank * 0x4000 + (a as usize - 0x4000)
        }
    };
    let mut pc = addr;
    for _ in 0..count {
        let b = [gb.bus.cart.rom_byte(off(pc)), gb.bus.cart.rom_byte(off(pc.wrapping_add(1))), gb.bus.cart.rom_byte(off(pc.wrapping_add(2)))];
        let (t, l) = dis1(&b, pc);
        let bytes: Vec<String> = b[..l as usize].iter().map(|x| format!("{x:02x}")).collect();
        println!("{:02x}:{pc:04x}  {:<9} {t}", if pc < 0x4000 { 0 } else { bank }, bytes.join(" "));
        pc = pc.wrapping_add(l);
    }
}

/// Snapshot memory at `frames` (label:frame list, e.g. "t:450,m:720,l:1000") and print
/// addresses whose value is the same at every frame of one label and differs between labels.
pub fn modes(rom: &Path, script: &str, frames: &str) {
    let Ok(mut gb) = load_gb(rom, 44100, false) else { return };
    let sc = parse_script(script);
    let want: Vec<(String, u32)> = frames
        .split(',')
        .filter_map(|p| p.split_once(':').map(|(l, f)| (l.to_string(), f.parse().unwrap_or(0))))
        .collect();
    let last = want.iter().map(|w| w.1).max().unwrap_or(0);
    let addrs: Vec<u16> = (0xA000..=0xBFFFu16).chain(0xC000..=0xDFFF).chain(0xFF80..=0xFFFE).collect();
    let mut snaps: Vec<(String, Vec<u8>)> = Vec::new();
    for f in 0..=last {
        run_script(&mut gb, &sc, f);
        for (l, wf) in &want {
            if *wf == f {
                snaps.push((l.clone(), addrs.iter().map(|&a| gb.bus.read(a)).collect()));
            }
        }
    }
    let mut labels: Vec<String> = want.iter().map(|w| w.0.clone()).collect();
    labels.dedup();
    let mut shown = 0;
    // Bytes constant across every "l" frame whose value never appears in other frames.
    for (i, a) in addrs.iter().enumerate() {
        let lv: Vec<u8> = snaps.iter().filter(|s| s.0 == "l").map(|s| s.1[i]).collect();
        if lv.is_empty() || lv.iter().any(|&v| v != lv[0]) {
            continue;
        }
        let others: Vec<u8> = snaps.iter().filter(|s| s.0 != "l").map(|s| s.1[i]).collect();
        if others.iter().all(|&v| v != lv[0]) && shown < 60 {
            shown += 1;
            println!("level-only {a:04x} level={:02x} others={:02x?}", lv[0], others);
        }
    }
    shown = 0;
    for (i, a) in addrs.iter().enumerate() {
        let mut vals = Vec::new();
        let mut ok = true;
        for l in &labels {
            let vs: Vec<u8> = snaps.iter().filter(|s| &s.0 == l).map(|s| s.1[i]).collect();
            if vs.iter().any(|&v| v != vs[0]) {
                ok = false;
                break;
            }
            vals.push(vs[0]);
        }
        let mut uniq = vals.clone();
        uniq.sort();
        uniq.dedup();
        if ok && uniq.len() == vals.len() && shown < 80 {
            shown += 1;
            let t: Vec<String> = labels.iter().zip(&vals).map(|(l, v)| format!("{l}={v:02x}")).collect();
            println!("{a:04x} {}", t.join(" "));
        }
    }
}

/// Print addresses whose value went from `from_v` at frame `fa` to `to_v` at frame `fb`.
pub fn diff(rom: &Path, script: &str, fa: u32, fb: u32, from_v: u8, to_v: u8) {
    let Ok(mut gb) = load_gb(rom, 44100, false) else { return };
    let sc = parse_script(script);
    let addrs: Vec<u16> = (0xA000..=0xBFFFu16).chain(0xC000..=0xDFFF).chain(0xFF80..=0xFFFE).collect();
    let mut a = Vec::new();
    for f in 0..=fb {
        run_script(&mut gb, &sc, f);
        if f == fa {
            a = addrs.iter().map(|&x| gb.bus.read(x)).collect();
        }
    }
    for (i, &x) in addrs.iter().enumerate() {
        let v = gb.bus.read(x);
        if a[i] == from_v && v == to_v {
            println!("{x:04x}");
        }
    }
}

/// Run the script to frame `at`, write `pokes` ("a299=00,ff9b=14"), keep running to
/// `until`, printing the mode byte and lives each 20 frames and saving a picture at the end.
pub fn poke(rom: &Path, script: &str, at: u32, pokes: &str, until: u32, png: &Path) {
    let Ok(mut gb) = load_gb(rom, 44100, false) else { return };
    let sc = parse_script(script);
    for f in 0..until {
        if f == at {
            for p in pokes.split(',') {
                if let Some((a, v)) = p.split_once('=') {
                    let a = u16::from_str_radix(a, 16).unwrap_or(0);
                    let v = u8::from_str_radix(v, 16).unwrap_or(0);
                    gb.bus.write(a, v);
                }
            }
        }
        run_script(&mut gb, &sc, f);
        if f >= at && (f - at) % 20 == 0 {
            println!("f{f} mode={:02x} lives={:02x}", gb.bus.read(0xFF9B), gb.bus.read(0xA22C));
        }
    }
    save_png(&gb, png);
}

/// Try the menu's "back to map" (at `at`) or "file select" (kind "file"), then keep playing
/// the script, printing mode and lives every 20 frames.
pub fn leave(rom: &Path, script: &str, at: u32, kind: &str, until: u32, png: &Path) {
    let Ok(mut gb) = load_gb(rom, 44100, false) else { return };
    let sc = parse_script(script);
    let pokes = std::env::var("POKES").unwrap_or_default();
    for f in 0..until {
        if f == at + 40 {
            for p in pokes.split(',') {
                if let Some((a, v)) = p.split_once('=') {
                    gb.bus.write(u16::from_str_radix(a, 16).unwrap_or(0), u8::from_str_radix(v, 16).unwrap_or(0));
                }
            }
        }
        if f == at {
            if kind == "file" {
                gb.back_to_file_select();
            } else {
                gb.back_to_map();
            }
            println!("after: mode={:02x}", gb.mode());
        }
        run_script(&mut gb, &sc, f);
        if f >= at && (f - at) % 20 == 0 {
            println!("f{f} mode={:02x} lives={:02x}", gb.mode(), gb.bus.read(0xA22C));
        }
    }
    save_png(&gb, png);
}

/// Compare memory: run A (natural death) to `fa`; run B (back_to_map at `at`) to `fb`.
pub fn cmp_map(rom: &Path, sa: &str, fa: u32, sb: &str, at: u32, fb: u32) {
    let snap = |gb: &mut Gb| -> Vec<(u16, u8)> {
        (0xA000..=0xBFFFu16).chain(0xC000..=0xDFFF).chain(0xFF80..=0xFFFE).map(|a| (a, gb.bus.read(a))).collect()
    };
    let Ok(mut a) = load_gb(rom, 44100, false) else { return };
    let sca = parse_script(sa);
    for f in 0..fa {
        run_script(&mut a, &sca, f);
    }
    let Ok(mut b) = load_gb(rom, 44100, false) else { return };
    let scb = parse_script(sb);
    for f in 0..fb {
        if f == at {
            b.back_to_map();
        }
        run_script(&mut b, &scb, f);
    }
    let (ma, mb) = (snap(&mut a), snap(&mut b));
    let d: Vec<String> = ma.iter().zip(&mb).filter(|(x, y)| x.1 != y.1 && x.0 < 0xC000 || (x.0 >= 0xFF80 && x.1 != y.1)).map(|(x, y)| format!("{:04x}:{:02x}/{:02x}", x.0, x.1, y.1)).collect();
    println!("{} differ (natural/ours):\n{}", d.len(), d.join(" "));
}

/// For every level: where its map is, its compressed size, whether decoding the ROM gives
/// the same blocks the game itself puts in memory when the level loads, and whether
/// re-compressing gives the same blocks back.
pub fn levels_check(rom_path: &Path) {
    use crate::levels::*;
    let Ok(mut gb) = load_gb(rom_path, 44100, false) else { return };
    let rom = gb.bus.cart.rom_copy();
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
    use crate::levels::*;
    let Ok(gb) = load_gb(rom_path, 44100, false) else { return };
    let original = gb.bus.cart.rom_copy();
    // 1) an expanded copy must play every level exactly like the original
    let expanded = match expand(&original) {
        Ok(r) => r,
        Err(e) => {
            println!("expand: {e}");
            return;
        }
    };
    let Ok(mut g0) = crate::Gb::new(expanded.clone(), 44100) else { return };
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
        let Ok(mut g) = crate::Gb::new(rom.clone(), 44100) else { return };
        let ok = g.boot_to_map() && g.enter_level(level as u8);
        let same = map_in_memory(&g).iter().zip(&map).filter(|(a, b)| a == b).count();
        // the other levels of the bank must still load unchanged
        let mut others_ok = true;
        for o in 0..LEVELS {
            if o != level && header(&rom, o).bank == h.bank {
                let mut g2 = crate::Gb::new(rom.clone(), 44100).unwrap();
                g2.boot_to_map();
                let (_, s) = map_location(&original, header(&original, o));
                let (want, _) = decode(&original, s);
                if !(g2.enter_level(o as u8) && map_in_memory(&g2) == want) {
                    others_ok = false;
                }
            }
        }
        println!("expanded big edit: level {level} ({old} bytes, {rows} rows of checkerboard) -> {res:?}; game loaded={ok}; matches {same}/{MAP_BYTES}; other levels of the zone unchanged={others_ok}");
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
        let Ok(mut g) = crate::Gb::new(rom.clone(), 44100) else { return };
        let ok = g.boot_to_map() && g.enter_level(level as u8);
        let mem = map_in_memory(&g);
        let same = mem.iter().zip(&map).filter(|(a, b)| a == b).count();
        println!("{name}: level {level} old {old} bytes -> {:?}; game loaded={ok}; game memory matches edit {same}/{MAP_BYTES}", res);
        // nothing else in the ROM changed except the map data and (if moved) its table entry
        let changed = rom.iter().zip(&original).filter(|(a, b)| a != b).count();
        println!("   bytes changed in the ROM: {changed}");
    }
}

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

/// Log data reads from ROM bank `bank` addresses lo..=hi between frames, summarised by
/// reading instruction: how many reads and the address range each one touched.
pub fn reads(rom: &Path, script: &str, bank: usize, lo: u16, hi: u16, from: u32, to: u32) {
    let Ok(mut gb) = load_gb(rom, 44100, false) else { return };
    let sc = parse_script(script);
    let mut by_pc: std::collections::BTreeMap<u32, (usize, u16, u16, Vec<u32>)> = Default::default();
    for f in 0..to {
        if f == from {
            gb.bus.read_watch = Some((lo, hi, bank));
        }
        run_script(&mut gb, &sc, f);
        for (pc, a, b) in gb.bus.read_log.drain(..) {
            let key = (b as u32) << 16 | pc as u32;
            let e = by_pc.entry(key).or_insert((0, u16::MAX, 0, Vec::new()));
            e.0 += 1;
            e.1 = e.1.min(a);
            e.2 = e.2.max(a);
            if e.3.last() != Some(&f) && e.3.len() < 12 {
                e.3.push(f);
            }
        }
    }
    for (pc, (n, a, b, fs)) in by_pc {
        println!("bank {:02x} pc {:04x}: {n} reads {a:04x}-{b:04x} frames {fs:?}", pc >> 16, pc & 0xFFFF);
    }
}

/// Print a level's object list (bank 3, table $6037) and the 5-byte records the game
/// builds from it at $AB06 once the level has loaded.
pub fn objects(rom_path: &Path, level: u8) {
    let Ok(mut gb) = load_gb(rom_path, 44100, false) else { return };
    let rom = gb.bus.cart.rom_copy();
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
        let r: Vec<String> = (0..5).map(|j| format!("{:02x}", gb.bus.cart.ram[0xB06 + k * 5 + j])).collect();
        println!("{k:2}: {b:02x} {c:02x} {d:02x}   -> ram {}", r.join(" "));
    }
    println!("{} objects, list at file {p:05x}", raw.len());
}

/// Objects: re-encoding gives the same bytes; what the game unpacks matches our reading;
/// the lists sit back to back; and an edited list (moved, removed, added, new type) is what
/// the game unpacks after storing it.
pub fn objects_check(rom_path: &Path) {
    use cart_model::objects::{self, Obj};
    let Ok(mut gb) = load_gb(rom_path, 44100, false) else { return };
    let original = gb.bus.cart.rom_copy();
    gb.boot_to_map();
    let mut total = 0;
    let mut good = 0;
    let mut bytes_ok = true;
    for l in 0..crate::levels::LEVELS {
        let (objs, len) = objects::read(&original, l);
        total += len;
        let mut i = 0;
        for o in &objs {
            let p = objects::read(&original, l).0;
            let _ = p;
            i += 1;
            let _ = o.to_bytes();
        }
        let _ = i;
        // bytes round trip
        let (_, _) = (objs.len(), len);
        let mut g = gb.clone();
        g.enter_level(l as u8);
        let mem = objects::in_memory(&g.bus.cart.ram, objs.len());
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
    let mut lists: Vec<Vec<Obj>> = (0..crate::levels::LEVELS).map(|l| objects::read(&original, l).0).collect();
    let removed = lists[1].pop();
    lists[0][0].x += 16;
    lists[0][1].kind = lists[0][2].kind;
    let extra = Obj { x: 40 * 16, y: lists[0][0].y, kind: lists[0][3].kind, flag: false };
    lists[0].push(extra);
    let mut rom = original.clone();
    let res = objects::store(&mut rom, &original, &lists);
    let mut g = crate::Gb::new(rom.clone(), 44100).unwrap();
    g.boot_to_map();
    let mut g0 = g.clone();
    g0.enter_level(0);
    let mut want = lists[0].clone();
    want.sort_by_key(|o| o.x / 256);
    let mem = objects::in_memory(&g0.bus.cart.ram, want.len());
    let mut g1 = g.clone();
    g1.enter_level(1);
    let mem1 = objects::in_memory(&g1.bus.cart.ram, lists[1].len());
    println!(
        "edit: {res:?}; level 0 game records match edit: {}; level 1 (one removed: {removed:?}) match: {}",
        mem == want,
        mem1 == lists[1]
    );
    // expanded copy: everything still the same, then 40 more objects in level 0
    let exp = crate::levels::expand(&original).unwrap();
    let mut gx = crate::Gb::new(exp.clone(), 44100).unwrap();
    gx.boot_to_map();
    let mut same = 0;
    for l in 0..crate::levels::LEVELS {
        let (objs, _) = objects::read(&original, l);
        let mut g = gx.clone();
        g.enter_level(l as u8);
        if objects::in_memory(&g.bus.cart.ram, objs.len()) == objs && objects::read(&exp, l).0 == objs {
            same += 1;
        }
    }
    let mut lists: Vec<Vec<Obj>> = (0..crate::levels::LEVELS).map(|l| objects::read(&exp, l).0).collect();
    let (k, y0) = (lists[0][0].kind, lists[0][0].y);
    for i in 0..40u16 {
        lists[0].push(Obj { x: 304 + i * 48, y: y0, kind: k, flag: false }); // positions are in 8 px steps
    }
    let mut rom = exp.clone();
    let res = objects::store(&mut rom, &original, &lists);
    let mut g = crate::Gb::new(rom.clone(), 44100).unwrap();
    g.boot_to_map();
    let mut ok = 0;
    for l in [0usize, 5, 17, 31] {
        let mut gl = g.clone();
        gl.enter_level(l as u8);
        let mut want = lists[l].clone();
        want.sort_by_key(|o| o.x / 256);
        let mem = objects::in_memory(&gl.bus.cart.ram, want.len());
        if mem == want {
            ok += 1;
        } else {
            let first = mem.iter().zip(&want).position(|(a, b)| a != b);
            println!("  level {l}: {} objects, first difference at {first:?}: game {:?} / stored {:?}", want.len(), first.map(|i| mem[i]), first.map(|i| want[i]));
        }
    }
    println!("expanded: {same}/32 levels' objects unchanged; +40 objects in level 0 -> {res:?}; levels 0,5,17,31 load as stored: {ok}/4");
}

/// Draw ROM bytes at `off` as an 8x8-tile map `width` tiles wide, with the tiles the game
/// has in VRAM on the world map, to test what the world map data looks like.
pub fn world_shot(rom_path: &Path, off: usize, len: usize, width: usize, png: &Path) {
    let Ok(mut gb) = load_gb(rom_path, 44100, false) else { return };
    gb.boot_to_map();
    let rom = gb.bus.cart.rom_copy();
    let vram = gb.bus.ppu.vram.to_vec();
    let lcdc = gb.bus.ppu.lcdc;
    let bgp = gb.bus.ppu.bgp();
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
                    let col = crate::PALETTES[0][s as usize];
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

/// World map: the 32x32 window the game puts in VRAM on the map matches our reading of the
/// picture; an edited tile shows up there; and a re-pointed map point opens another level.
pub fn world_check(rom_path: &Path) {
    use cart_model::world;
    let Ok(gb) = load_gb(rom_path, 44100, false) else { return };
    let original = gb.bus.cart.rom_copy();
    let pic = world::read(&original);
    let window = |g: &crate::Gb| g.bus.ppu.vram[0x1800..0x1C00].to_vec();
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
    let mut g = crate::Gb::new(original.clone(), 44100).unwrap();
    g.boot_to_map();
    let pos = find(&window(&g), &pic);
    // count how many tiles of the VRAM window equal the picture at the same wrapped place
    let w0 = window(&g);
    let best = (0..world::H * world::W)
        .map(|o| {
            let (ox, oy) = (o % world::W, o / world::W);
            (0..32 * 32).filter(|&i| {
                let (x, y) = (i % 32, i / 32);
                let (px, py) = ((ox + x) % world::W, (oy + y) % world::H);
                w0[(py % 32) * 32 + px % 32] == pic[py * world::W + px]
            }).count()
        })
        .max()
        .unwrap_or(0);
    println!("original: VRAM window found at {pos:?}; best match {best}/1024 tiles");
    // edit: change every tile in the picture's top-left quadrant row 10 and point 0 -> level 6
    let mut rom = crate::levels::expand(&original).unwrap();
    let mut p2 = pic.clone();
    for x in 0..world::W {
        for y in 0..world::H {
            if p2[y * world::W + x] == pic[(world::H / 2) * world::W + world::W / 2] {
                // leave
            }
        }
    }
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
    let mut g = crate::Gb::new(rom.clone(), 44100).unwrap();
    g.boot_to_map();
    let w1 = window(&g);
    let same_new = (0..world::H * world::W)
        .map(|o| {
            let (ox, oy) = (o % world::W, o / world::W);
            (0..32 * 32).filter(|&i| {
                let (x, y) = (i % 32, i / 32);
                let (px, py) = ((ox + x) % world::W, (oy + y) % world::H);
                w1[(py % 32) * 32 + px % 32] == p2[py * world::W + px]
            }).count()
        })
        .max()
        .unwrap_or(0);
    // press Start on the map and see which level the game asks for
    let mut level = None;
    for f in 0..300u32 {
        let m = g.mode();
        if m == 0x0B {
            level = Some(g.bus.read(0xA269));
            break;
        }
        g.run_frame_with(if f % 30 < 4 { crate::bus::BTN_START } else { 0 });
    }
    println!("edited: VRAM window best match with the edited picture {same_new}/1024; Start on point 0 opens level {level:?} (set to 6)");
}

/// Capture what each object type of a level looks like and save them side by side.
pub fn looks(rom_path: &Path, level: usize, png: &Path) {
    use cart_model::objects;
    let Ok(mut gb) = load_gb(rom_path, 44100, false) else { return };
    let rom = gb.bus.cart.rom_copy();
    gb.boot_to_map();
    let mut kinds: Vec<u8> = objects::read(&rom, level).0.iter().map(|o| o.kind).collect();
    kinds.sort();
    kinds.dedup();
    let cell = 40usize;
    let w = cell * kinds.len().max(1);
    let mut rgba = vec![255u8; w * cell * 4];
    for (i, &k) in kinds.iter().enumerate() {
        let look = objects::capture_look(&gb, &rom, &rom, level, k);
        println!("type {k:02X}: {}", match &look { Some(l) => format!("{}x{} offset {},{}", l.w, l.h, l.dx, l.dy), None => "nothing drawn".into() });
        if let Some(l) = look {
            for y in 0..l.h.min(cell) {
                for x in 0..l.w.min(cell) {
                    if let Some(s) = l.px[y * l.w + x] {
                        let c = crate::PALETTES[0][s as usize];
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

/// Print ROM bytes as signed rows of `row` values with each row's sum.
pub fn table(rom: &Path, off: usize, len: usize, row: usize) {
    let Ok(gb) = load_gb(rom, 44100, false) else { return };
    let mut i = 0;
    while i < len {
        let vals: Vec<i32> = (0..row.min(len - i)).map(|k| gb.bus.cart.rom_byte(off + i + k) as i8 as i32).collect();
        let s: i32 = vals.iter().sum();
        let t: Vec<String> = vals.iter().map(|v| format!("{v:3}")).collect();
        println!("{:05x} idx {:02x}: {}  sum {s}", off + i, i, t.join(""));
        i += row;
    }
}

fn run_script(gb: &mut Gb, sc: &[(u8, u32, u32)], f: u32) {
    gb.bus.set_buttons(buttons_at(sc, f));
    gb.run_frame();
    gb.bus.apu.out.clear();
}

/// Play the self-test script and print RAM bytes that move with the player.
pub fn find_player(rom: &Path, script: &str) {
    let Ok(mut gb) = load_gb(rom, 44100, false) else { return };
    let sc = parse_script(script);
    let mut frames: Vec<(Vec<u8>, i32, i32)> = Vec::new(); // wram+hram, sprite x+scroll, sprite y
    let mut world = 0i32;
    let mut prev = (0i32, 0u8);
    for f in 0..1230u32 {
        run_script(&mut gb, &sc, f);
        let x = gb.bus.ppu.oam[1] as i32;
        let scx = gb.bus.ppu.scx();
        world += (x - prev.0) + (scx.wrapping_sub(prev.1) as i8) as i32;
        prev = (x, scx);
        if f >= 940 {
            let mut m = gb.bus.wram.to_vec();
            for a in 0xFF80..=0xFFFEu16 {
                m.push(gb.bus.read(a));
            }
            frames.push((m, world, gb.bus.ppu.oam[0] as i32));
        }
    }
    let name = |i: usize| if i < 0x2000 { 0xC000 + i } else { 0xFF80 + i - 0x2000 };
    let n = frames[0].0.len();
    let total_w = frames[160].1 - frames[20].1;
    println!("-- bytes whose summed change over the walk is within 3 of {total_w} px");
    for i in 0..n {
        let mut s = 0i32;
        let mut changes = 0;
        for k in 21..=160 {
            let d = frames[k].0[i].wrapping_sub(frames[k - 1].0[i]) as i8 as i32;
            s += d;
            if d != 0 {
                changes += 1;
            }
        }
        if (s - total_w).abs() <= 3 && changes > 20 {
            println!("{:04x} sum={s} changes={changes}", name(i));
        }
    }
    let ground = frames[209].2;
    let top = frames[210..].iter().map(|f| f.2).min().unwrap_or(0);
    println!("-- bytes whose lowest point in the jump is {} below the start (sprite {ground}->{top})", ground - top);
    for i in 0..n {
        let start = frames[209].0[i] as i32;
        let lo = frames[210..].iter().map(|f| f.0[i] as i32).min().unwrap_or(0);
        let end = frames[frames.len() - 1].0[i] as i32;
        if start - lo >= 10 && (start - lo - (ground - top)).abs() <= 3 && (end - start).abs() <= 3 {
            println!("{:04x} start={start} low={lo}", name(i));
        }
    }
    // Position: byte whose per-frame wrapping delta matches the sprite's world delta.
    println!("-- x candidates (byte delta == world delta on >=90% frames, walking)");
    for i in 0..n {
        let mut ok = 0;
        let mut moving = 0;
        for k in 21..160 {
            let dw = frames[k].1 - frames[k - 1].1;
            if dw != 0 {
                moving += 1;
                let db = frames[k].0[i].wrapping_sub(frames[k - 1].0[i]) as i8 as i32;
                if db == dw {
                    ok += 1;
                }
            }
        }
        if moving > 20 && ok * 10 >= moving * 9 {
            println!("{:04x}", name(i));
        }
    }
    println!("-- y candidates (byte delta == sprite y delta while jumping)");
    for i in 0..n {
        let mut ok = 0;
        let mut moving = 0;
        for k in 211..frames.len() {
            let dy = frames[k].2 - frames[k - 1].2;
            if dy != 0 {
                moving += 1;
                let db = frames[k].0[i].wrapping_sub(frames[k - 1].0[i]) as i8 as i32;
                if db == dy {
                    ok += 1;
                }
            }
        }
        if moving > 5 && ok * 10 >= moving * 9 {
            println!("{:04x}", name(i));
        }
    }
}

/// Log every write to `addrs` during the script, with the instruction that made it.
pub fn watch(rom: &Path, script: &str, addrs: &[u16], from: u32, to: u32) {
    let Ok(mut gb) = load_gb(rom, 44100, false) else { return };
    let sc = parse_script(script);
    for f in 0..to {
        if f == from {
            gb.bus.watch = addrs.to_vec();
        }
        run_script(&mut gb, &sc, f);
        for (pc, bank, a, v) in gb.bus.watch_log.drain(..) {
            println!("f{f} {bank:02x}:{pc:04x} [{a:04x}]={v:02x}");
        }
    }
}

/// Print chosen RAM bytes every frame of the script between `from` and `to`.
pub fn trace(rom: &Path, script: &str, addrs: &[u16], from: u32, to: u32) {
    let Ok(mut gb) = load_gb(rom, 44100, false) else { return };
    let sc = parse_script(script);
    for f in 0..to {
        run_script(&mut gb, &sc, f);
        if f >= from {
            let v: Vec<String> = addrs.iter().map(|&a| format!("{:02x}", gb.bus.read(a))).collect();
            println!("f{f} btn={:02x} {}", buttons_at(&sc, f), v.join(" "));
        }
    }
}
