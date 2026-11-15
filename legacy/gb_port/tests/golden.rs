//! Golden regression fingerprint of the legacy port.
//!
//! Runs the emulator and the cartridge-knowledge code over the person's own ROM and writes
//! a list of `key = value` lines (hashes and small values) to `regression/golden.txt` at the
//! repository root. The refactored crates must reproduce the very same lines.
//!
//!   cargo test --release --test golden                  compare against the stored file
//!   $env:GOLDEN_UPDATE=1; cargo test --release --test golden   (re)write the stored file
//!
//! The ROM is taken from the `SML2_ROM` environment variable, else found the way the game
//! finds it. Without a ROM the test is skipped (it prints why). Nothing from the ROM is
//! written to the golden file except hashes, sizes and offsets.
use gb_port::bus::*;
use gb_port::custom_map::CustomMap;
use gb_port::machine::{self, Gb};
use gb_port::{levels, objects, world};
use std::path::PathBuf;

const SAMPLE_RATE: u32 = 44_100;

/// FNV-1a, 64 bit: tiny, dependency-free and stable across versions.
fn fnv(data: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

fn fnv_f32(data: &[f32]) -> String {
    let bytes: Vec<u8> = data.iter().flat_map(|s| s.to_bits().to_le_bytes()).collect();
    fnv(&bytes)
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../regression/golden.txt")
}

fn rom_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("SML2_ROM") {
        return Some(PathBuf::from(p));
    }
    machine::find_rom(None)
}

struct Out(Vec<String>);
impl Out {
    fn put(&mut self, key: impl AsRef<str>, value: impl std::fmt::Display) {
        self.0.push(format!("{} = {}", key.as_ref(), value));
    }
}

/// Buttons for frame `f` of the scripted run: Start pulses through the title and file
/// select, then a fixed cycle of walking, running and jumping (whatever the game does with
/// it is deterministic, which is all a fingerprint needs).
fn script(f: u32) -> u8 {
    if f < 200 {
        0
    } else if f < 1000 {
        if f % 50 < 5 { BTN_START } else { 0 }
    } else {
        const CYCLE: [u8; 8] = [
            BTN_RIGHT,
            BTN_RIGHT | BTN_B,
            BTN_RIGHT | BTN_A,
            BTN_RIGHT | BTN_B | BTN_A,
            0,
            BTN_LEFT,
            BTN_RIGHT,
            BTN_A,
        ];
        let b = CYCLE[((f / 30) % 8) as usize];
        // an occasional Start, to cover the map -> level transition too
        if f % 600 < 4 { b | BTN_START } else { b }
    }
}

/// 1. The raw emulator: power-on, scripted buttons, picture and sound fingerprints.
fn emulation(rom: &[u8], out: &mut Out) {
    let mut gb = Gb::new(rom.to_vec(), SAMPLE_RATE).expect("cartridge");
    let mut audio: Vec<f32> = Vec::new();
    for f in 0..3600u32 {
        gb.bus.set_buttons(script(f));
        gb.run_frame();
        audio.extend_from_slice(&gb.bus.apu.out);
        gb.bus.apu.out.clear();
        if f % 60 == 59 {
            out.put(format!("emu.f{f:04}.screen"), fnv(&gb.bus.ppu.fb));
            out.put(format!("emu.f{f:04}.sound"), format!("{} {}", audio.len(), fnv_f32(&audio)));
            audio.clear();
        }
    }
    out.put("emu.end.mode", format!("{:02x}", gb.mode()));
    out.put("emu.end.wram", fnv(&gb.bus.wram));
    out.put("emu.end.cart_ram", fnv(&gb.bus.cart.ram));
    out.put("emu.end.vram", fnv(&gb.bus.ppu.vram));
    out.put("emu.end.oam", fnv(&gb.bus.ppu.oam));
}

/// 2. The game-specific use cases: boot to the map and enter each level.
fn use_cases(rom: &[u8], out: &mut Out) {
    let mut map = Gb::new(rom.to_vec(), SAMPLE_RATE).expect("cartridge");
    out.put("uc.boot_to_map", map.boot_to_map());
    out.put("uc.map.screen", fnv(&map.bus.ppu.fb));
    let art = world::WorldArt::capture(&map);
    out.put("uc.map.world_art", fnv(&[art.vram.as_slice(), &[art.lcdc, art.bgp]].concat()));

    for level in 0..levels::LEVELS {
        let mut g = map.clone();
        let ok = g.enter_level_anywhere(level as u8);
        let la = levels::LevelArt::capture(&g);
        out.put(format!("uc.level{level:02}.entered"), format!("{ok} mode={:02x}", g.mode()));
        out.put(format!("uc.level{level:02}.screen"), fnv(&g.bus.ppu.fb));
        out.put(format!("uc.level{level:02}.map_in_memory"), fnv(&levels::map_in_memory(&g)));
        out.put(
            format!("uc.level{level:02}.art"),
            fnv(&[la.metatiles.as_slice(), &la.vram, &[la.lcdc, la.bgp]].concat()),
        );
        for _ in 0..120 {
            g.run_frame_with(BTN_RIGHT | BTN_B);
        }
        out.put(format!("uc.level{level:02}.after_run.screen"), fnv(&g.bus.ppu.fb));
        let mut back = g.clone();
        back.back_to_map();
        for _ in 0..240 {
            back.run_frame_with(0);
        }
        out.put(format!("uc.level{level:02}.back_to_map.mode"), format!("{:02x}", back.mode()));
    }
}

/// 3. The cartridge knowledge: levels, objects, world map, ROM expansion and re-packing.
fn rom_data(rom: &[u8], out: &mut Out) {
    out.put("rom.size", rom.len());
    out.put("rom.can_expand", levels::can_expand(rom));

    let mut lists = Vec::new();
    for level in 0..levels::LEVELS {
        let h = levels::header(rom, level);
        let (entry, start) = levels::map_location(rom, h);
        let (map, used) = levels::decode(rom, start);
        let enc = levels::encode(&map);
        let (again, _) = levels::decode(&enc, 0);
        out.put(
            format!("rom.level{level:02}.header"),
            format!("at={:05x} bank={:02x} map={} entry={:05x} start={:05x}", h.at, h.bank, h.map, entry, start),
        );
        out.put(format!("rom.level{level:02}.map"), format!("{} used={used}", fnv(&map)));
        out.put(
            format!("rom.level{level:02}.encode"),
            format!("len={} roundtrip={} same_as_rom={}", enc.len(), again == map, enc[..] == rom[start..start + used]),
        );
        out.put(format!("rom.level{level:02}.sharing"), format!("{:?}", levels::sharing(rom, level)));

        let (objs, bytes) = objects::read(rom, level);
        let packed: Vec<u8> = objs.iter().flat_map(|o| o.to_bytes()).collect();
        let desc: Vec<String> = objs.iter().map(|o| format!("{},{},{:02x},{}", o.x, o.y, o.kind, o.flag as u8)).collect();
        out.put(format!("rom.level{level:02}.objects"), format!("n={} bytes={bytes} {}", objs.len(), fnv(desc.join(";").as_bytes())));
        out.put(format!("rom.level{level:02}.objects_packed"), fnv(&packed));
        lists.push(objs);
    }
    out.put("rom.objects.region", format!("{:?}", objects::region(rom)));

    out.put("rom.world.picture", fnv(&world::read(rom)));
    out.put("rom.world.point_levels", fnv(&world::point_levels(rom)));

    let mut banks: Vec<u8> = (0..levels::LEVELS).map(|l| levels::header(rom, l).bank).collect();
    banks.sort();
    banks.dedup();
    for &b in &banks {
        out.put(format!("rom.bank{b:02x}.maps"), format!("{:?}", levels::bank_maps(rom, b)));
        out.put(format!("rom.bank{b:02x}.capacity"), format!("{:?}", levels::bank_capacity(rom, rom, b)));
    }

    match levels::expand(rom) {
        Ok(mut big) => {
            out.put("exp.rom", fnv(&big));
            out.put("exp.flags", format!("expanded={} objects_patched={}", levels::is_expanded(&big), levels::objects_patched(&big)));
            out.put("exp.objects.region", format!("{:?}", objects::region_for(&big, rom)));
            out.put("exp.objects.space", format!("{:?}", objects::space(&big, rom, &lists)));
            // Re-pack every bank and the objects unchanged, then edit one level and re-pack.
            for &b in &banks {
                let r = levels::store_bank(&mut big, rom, b, &[]);
                out.put(format!("exp.bank{b:02x}.store"), format!("{r:?}"));
            }
            out.put("exp.objects.store", format!("{:?}", objects::store(&mut big, rom, &lists)));
            out.put("exp.after_store.rom", fnv(&big));

            let h = levels::header(&big, 0);
            let (_, start) = levels::map_location(&big, h);
            let (mut map, _) = levels::decode(&big, start);
            for x in 0..levels::COLS {
                map[x] = (x % 7) as u8; // a recognisable edit in the top row
            }
            out.put("exp.edit.space", format!("{:?}", levels::bank_space(&big, rom, h.bank, &[(h.map, &map[..])])));
            out.put("exp.edit.store", format!("{:?}", levels::store_bank(&mut big, rom, h.bank, &[(h.map, &map[..])])));
            let (_, start) = levels::map_location(&big, levels::header(&big, 0));
            out.put("exp.edit.reread_ok", levels::decode(&big, start).0 == map);

            world::write(&mut big, &world::read(rom));
            let mut pts = world::point_levels(rom);
            pts.reverse();
            world::write_point_levels(&mut big, &pts);
            out.put("exp.after_edit.rom", fnv(&big));

            // The edited copy must still boot and play the edited level.
            let mut g = Gb::new(big, SAMPLE_RATE).expect("expanded cartridge");
            let booted = g.boot_to_map();
            let entered = g.enter_level_anywhere(0);
            out.put("exp.play.level00", format!("boot={booted} entered={entered} {}", fnv(&levels::map_in_memory(&g))));
        }
        Err(e) => out.put("exp.error", e),
    }
}

/// 4. The editor's own world map file format.
fn custom_map(out: &mut Out) {
    let text = "# sample\npoint 10 20 0\npoint 100 20 -\npoint 100 200 31\npoint 300 40 40\n\
                path 0 1\npath 1 2\npath 2 2\npath 1 9\npath 1 3\nstart 1\n";
    let mut m = CustomMap::parse(text);
    out.put("cmap.parse", fnv(m.to_text().as_bytes()));
    out.put("cmap.linked", format!("{} {} {}", m.linked(0, 1), m.linked(1, 0), m.linked(0, 2)));
    out.put("cmap.neighbour", format!("{:?} {:?} {:?}", m.neighbour(1, 1, 0), m.neighbour(1, 0, 1), m.neighbour(1, -1, 0)));
    m.toggle_path(0, 2);
    m.toggle_path(0, 1);
    out.put("cmap.toggle", fnv(m.to_text().as_bytes()));
    m.remove_point(1);
    out.put("cmap.remove", fnv(m.to_text().as_bytes()));
}

#[test]
fn golden() {
    let Some(path) = rom_path() else {
        eprintln!("golden: no ROM found (set SML2_ROM); skipped");
        return;
    };
    let rom = std::fs::read(&path).expect("read ROM");
    let mut out = Out(Vec::new());
    out.put("rom.fingerprint", fnv(&rom));
    rom_data(&rom, &mut out);
    custom_map(&mut out);
    emulation(&rom, &mut out);
    use_cases(&rom, &mut out);
    let text = out.0.join("\n") + "\n";

    let file = golden_path();
    if std::env::var_os("GOLDEN_UPDATE").is_some() || !file.exists() {
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, &text).unwrap();
        eprintln!("golden: wrote {} lines to {}", out.0.len(), file.display());
        return;
    }
    let want = std::fs::read_to_string(&file).unwrap().replace("\r\n", "\n");
    let mut diffs = Vec::new();
    let (got, exp): (Vec<&str>, Vec<&str>) = (text.lines().collect(), want.lines().collect());
    for i in 0..got.len().max(exp.len()) {
        let (g, e) = (got.get(i).copied().unwrap_or("<missing>"), exp.get(i).copied().unwrap_or("<missing>"));
        if g != e {
            diffs.push(format!("  expected: {e}\n       got: {g}"));
        }
    }
    assert!(diffs.is_empty(), "{} golden lines differ:\n{}", diffs.len(), diffs.iter().take(20).cloned().collect::<Vec<_>>().join("\n"));
}
