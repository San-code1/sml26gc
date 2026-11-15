//! Following the game's memory while a button script plays: which bytes mark a game mode,
//! what changed, what an instruction writes or reads, pokes, and where the player is.
use crate::console::{apply_pokes, load, probed, ram_snapshot, run_script};
use cart_model::ram_map::{GAME_MODE, LIVES};
use cart_model::GameDriver;
use gb_core::script::{buttons_at, parse_script};
use host::png_out::save_png;
use std::path::Path;

/// Snapshot memory at `frames` (label:frame list, e.g. "t:450,m:720,l:1000") and print
/// addresses whose value is the same at every frame of one label and differs between labels.
pub fn modes(rom: &Path, script: &str, frames: &str) {
    let Some(mut gb) = load(rom) else { return };
    let sc = parse_script(script);
    let want: Vec<(String, u32)> = frames
        .split(',')
        .filter_map(|p| p.split_once(':').map(|(l, f)| (l.to_string(), f.parse().unwrap_or(0))))
        .collect();
    let last = want.iter().map(|w| w.1).max().unwrap_or(0);
    let addrs = crate::console::ram_addresses();
    let mut snaps: Vec<(String, Vec<u8>)> = Vec::new();
    for f in 0..=last {
        run_script(&mut gb, &sc, f);
        for (l, wf) in &want {
            if *wf == f {
                snaps.push((l.clone(), addrs.iter().map(|&a| gb.read(a)).collect()));
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
    let Some(mut gb) = load(rom) else { return };
    let sc = parse_script(script);
    let addrs = crate::console::ram_addresses();
    let mut a = Vec::new();
    for f in 0..=fb {
        run_script(&mut gb, &sc, f);
        if f == fa {
            a = addrs.iter().map(|&x| gb.read(x)).collect();
        }
    }
    for (i, &x) in addrs.iter().enumerate() {
        let v = gb.read(x);
        if a[i] == from_v && v == to_v {
            println!("{x:04x}");
        }
    }
}

/// Run the script to frame `at`, write `pokes` ("a299=00,ff9b=14"), keep running to
/// `until`, printing the mode byte and lives each 20 frames and saving a picture at the end.
pub fn poke(rom: &Path, script: &str, at: u32, pokes: &str, until: u32, png: &Path) {
    let Some(mut gb) = load(rom) else { return };
    let sc = parse_script(script);
    for f in 0..until {
        if f == at {
            apply_pokes(&mut gb, pokes);
        }
        run_script(&mut gb, &sc, f);
        if f >= at && (f - at) % 20 == 0 {
            println!("f{f} mode={:02x} lives={:02x}", gb.read(GAME_MODE), gb.read(LIVES));
        }
    }
    save_png(&gb, png);
}

/// Try the menu's "back to map" (at `at`) or "file select" (kind "file"), then keep playing
/// the script, printing mode and lives every 20 frames.
pub fn leave(rom: &Path, script: &str, at: u32, kind: &str, until: u32, png: &Path) {
    let Some(mut gb) = load(rom) else { return };
    let sc = parse_script(script);
    let pokes = std::env::var("POKES").unwrap_or_default();
    for f in 0..until {
        if f == at + 40 {
            apply_pokes(&mut gb, &pokes);
        }
        if f == at {
            if kind == "file" {
                gb.back_to_file_select();
            } else {
                gb.back_to_map();
            }
            println!("after: mode={:02x}", gb.mode().byte());
        }
        run_script(&mut gb, &sc, f);
        if f >= at && (f - at) % 20 == 0 {
            println!("f{f} mode={:02x} lives={:02x}", gb.mode().byte(), gb.read(LIVES));
        }
    }
    save_png(&gb, png);
}

/// Compare memory: run A (natural death) to `fa`; run B (back_to_map at `at`) to `fb`.
pub fn cmp_map(rom: &Path, sa: &str, fa: u32, sb: &str, at: u32, fb: u32) {
    let Some(mut a) = load(rom) else { return };
    let sca = parse_script(sa);
    for f in 0..fa {
        run_script(&mut a, &sca, f);
    }
    let Some(mut b) = load(rom) else { return };
    let scb = parse_script(sb);
    for f in 0..fb {
        if f == at {
            b.back_to_map();
        }
        run_script(&mut b, &scb, f);
    }
    let (ma, mb) = (ram_snapshot(&mut a), ram_snapshot(&mut b));
    let d: Vec<String> = ma.iter().zip(&mb).filter(|(x, y)| x.1 != y.1 && x.0 < 0xC000 || (x.0 >= 0xFF80 && x.1 != y.1)).map(|(x, y)| format!("{:04x}:{:02x}/{:02x}", x.0, x.1, y.1)).collect();
    println!("{} differ (natural/ours):\n{}", d.len(), d.join(" "));
}

/// Log data reads from ROM bank `bank` addresses lo..=hi between frames, summarised by
/// reading instruction: how many reads and the address range each one touched.
/// Env READS_POKE="FRAME:ff9b=09,a299=00" writes memory at that frame (to clear a level).
pub fn reads(rom: &Path, script: &str, bank: usize, lo: u16, hi: u16, from: u32, to: u32) {
    let Some(mut gb) = probed(rom) else { return };
    let sc = parse_script(script);
    let poke = std::env::var("READS_POKE").ok().and_then(|v| v.split_once(':').map(|(f, p)| (f.parse::<u32>().unwrap_or(0), p.to_string())));
    let mut by_pc: std::collections::BTreeMap<u32, (usize, u16, u16, Vec<u32>)> = Default::default();
    for f in 0..to {
        if f == from {
            gb.observer_mut().read_watch = Some((lo, hi, bank));
        }
        if let Some((_, pokes)) = poke.as_ref().filter(|p| p.0 == f) {
            for (a, v) in pokes.split(',').filter_map(|p| p.split_once('=')) {
                gb.write(u16::from_str_radix(a, 16).unwrap_or(0), u8::from_str_radix(v, 16).unwrap_or(0));
            }
        }
        let before = gb.read(GAME_MODE);
        run_script(&mut gb, &sc, f);
        if std::env::var("READS_MODES").is_ok() && gb.read(GAME_MODE) != before {
            println!("f{f} mode {:02x}", gb.read(GAME_MODE));
        }
        for (pc, a, b) in gb.observer_mut().read_log.drain(..) {
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

/// Play the self-test script and print RAM bytes that move with the player.
pub fn find_player(rom: &Path, script: &str) {
    let Some(mut gb) = load(rom) else { return };
    let sc = parse_script(script);
    let mut frames: Vec<(Vec<u8>, i32, i32)> = Vec::new(); // wram+hram, sprite x+scroll, sprite y
    let mut world = 0i32;
    let mut prev = (0i32, 0u8);
    for f in 0..1230u32 {
        run_script(&mut gb, &sc, f);
        let x = gb.video().oam()[1] as i32;
        let scx = gb.video().scx();
        world += (x - prev.0) + (scx.wrapping_sub(prev.1) as i8) as i32;
        prev = (x, scx);
        if f >= 940 {
            let mut m = gb.wram().to_vec();
            for a in 0xFF80..=0xFFFEu16 {
                m.push(gb.read(a));
            }
            frames.push((m, world, gb.video().oam()[0] as i32));
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
    let Some(mut gb) = probed(rom) else { return };
    let sc = parse_script(script);
    for f in 0..to {
        if f == from {
            gb.observer_mut().watch = addrs.to_vec();
        }
        run_script(&mut gb, &sc, f);
        for (pc, bank, a, v) in gb.observer_mut().watch_log.drain(..) {
            println!("f{f} {bank:02x}:{pc:04x} [{a:04x}]={v:02x}");
        }
    }
}

/// Print chosen RAM bytes every frame of the script between `from` and `to`.
pub fn trace(rom: &Path, script: &str, addrs: &[u16], from: u32, to: u32) {
    let Some(mut gb) = load(rom) else { return };
    let sc = parse_script(script);
    for f in 0..to {
        run_script(&mut gb, &sc, f);
        if f >= from {
            let v: Vec<String> = addrs.iter().map(|&a| format!("{:02x}", gb.read(a))).collect();
            println!("f{f} btn={:02x} {}", buttons_at(&sc, f).bits(), v.join(" "));
        }
    }
}

/// Cartridge and work RAM on the world map with no save and with the player's battery save:
/// the addresses that differ (where the game keeps its progress).
pub fn save_diff(rom: &Path) {
    let snap = |save: bool| {
        crate::console::use_battery_save(save);
        let mut gb = load(rom)?;
        gb.boot_to_map();
        Some(crate::console::ram_snapshot(&mut gb))
    };
    let (Some(a), Some(b)) = (snap(false), snap(true)) else { return };
    for (x, y) in a.iter().zip(&b) {
        if x.1 != y.1 && x.0 < 0xC000 {
            println!("{:04x}: new {:02x}  saved {:02x}", x.0, x.1, y.1);
        }
    }
}
